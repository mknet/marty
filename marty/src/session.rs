//! Signed cookie sessions for sync CGI (survives process-per-request).
//!
//! Enable Cargo feature **`session`**. Values are stringly; keep payloads small.

use crate::cgi::{CgiRequest, CgiResponse};
use crate::cookie::{Cookie, SameSite};
use crate::form::parse_urlencoded;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::HashMap;
use std::fmt::Write as _;

type HmacSha256 = Hmac<Sha256>;

/// HMAC-signed cookie session store.
#[derive(Debug, Clone)]
pub struct SessionStore {
    secret: Vec<u8>,
    cookie_name: String,
    max_age: Option<u64>,
    secure: bool,
}

impl SessionStore {
    /// `secret` should be a long random string (env / deploy secret).
    pub fn new(secret: impl AsRef<[u8]>) -> Self {
        Self {
            secret: secret.as_ref().to_vec(),
            cookie_name: "marty_session".into(),
            max_age: Some(600),
            secure: false,
        }
    }

    pub fn cookie_name(mut self, name: impl Into<String>) -> Self {
        self.cookie_name = name.into();
        self
    }

    pub fn max_age(mut self, secs: Option<u64>) -> Self {
        self.max_age = secs;
        self
    }

    pub fn secure(mut self, on: bool) -> Self {
        self.secure = on;
        self
    }

    /// Load session from the request cookie (empty session if missing/invalid).
    pub fn load(&self, req: &CgiRequest) -> Session {
        let cookies = req.cookies();
        let Some(raw) = cookies.get(&self.cookie_name) else {
            return Session::default();
        };
        match verify_and_decode(&self.secret, raw) {
            Some(data) => Session {
                data,
                dirty: false,
            },
            None => Session::default(),
        }
    }

    /// Attach a `Set-Cookie` when the session was mutated (or always if `force`).
    pub fn save(&self, session: &Session, resp: CgiResponse) -> CgiResponse {
        if !session.dirty {
            return resp;
        }
        let value = sign_and_encode(&self.secret, &session.data);
        let mut cookie = Cookie::new(&self.cookie_name, value)
            .http_only(true)
            .same_site(SameSite::Lax)
            .secure(self.secure);
        if let Some(secs) = self.max_age {
            cookie = cookie.max_age(secs);
        }
        resp.set_cookie(cookie)
    }
}

/// In-memory session map for one request.
#[derive(Debug, Clone, Default)]
pub struct Session {
    data: HashMap<String, String>,
    dirty: bool,
}

impl Session {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.data.get(key).map(String::as_str)
    }

    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.data.insert(key.into(), value.into());
        self.dirty = true;
    }

    pub fn remove(&mut self, key: &str) {
        if self.data.remove(key).is_some() {
            self.dirty = true;
        }
    }
}

fn sign_and_encode(secret: &[u8], data: &HashMap<String, String>) -> String {
    let payload = encode_map(data);
    let sig = hmac_hex(secret, payload.as_bytes());
    format!("{payload}.{sig}")
}

fn verify_and_decode(secret: &[u8], raw: &str) -> Option<HashMap<String, String>> {
    let (payload, sig) = raw.rsplit_once('.')?;
    let expected = hmac_hex(secret, payload.as_bytes());
    if !constant_time_eq(sig.as_bytes(), expected.as_bytes()) {
        return None;
    }
    Some(parse_urlencoded(payload))
}

fn encode_map(data: &HashMap<String, String>) -> String {
    let mut keys: Vec<&String> = data.keys().collect();
    keys.sort();
    let mut out = String::new();
    for (i, k) in keys.iter().enumerate() {
        if i > 0 {
            out.push('&');
        }
        let _ = write!(
            out,
            "{}={}",
            percent_encode(k),
            percent_encode(data.get(*k).map(String::as_str).unwrap_or(""))
        );
    }
    out
}

fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => {
                let _ = write!(out, "%{b:02X}");
            }
        }
    }
    out
}

fn hmac_hex(secret: &[u8], msg: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC key");
    mac.update(msg);
    let bytes = mac.finalize().into_bytes();
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_signed_payload() {
        let secret = b"test-secret-key-please-rotate";
        let mut map = HashMap::new();
        map.insert("count".into(), "3".into());
        let raw = sign_and_encode(secret, &map);
        let back = verify_and_decode(secret, &raw).unwrap();
        assert_eq!(back.get("count").map(String::as_str), Some("3"));
        assert!(verify_and_decode(secret, &(raw + "x")).is_none());
    }

    #[test]
    fn percent_roundtrip_in_map() {
        let mut map = HashMap::new();
        map.insert("k".into(), "a/b c".into());
        let raw = sign_and_encode(b"s", &map);
        let back = verify_and_decode(b"s", &raw).unwrap();
        assert_eq!(back.get("k").map(String::as_str), Some("a/b c"));
    }
}
