//! Signed cookie sessions for sync CGI (survives process-per-request).
//!
//! Enable Cargo feature **`session`**. Values are stringly; keep payloads small.
//!
//! The cookie value is `v1.<exp>.<payload>.<sig>`: the HMAC covers the expiry, so a copied
//! cookie stops verifying after [`SessionStore::max_age`] even if the client ignores
//! `Max-Age`. `exp = 0` means no expiry (`max_age(None)`).
//!
//! Build the store with [`SessionStore::from_config`] so a missing secret fails loudly
//! instead of falling back to a public default:
//!
//! ```rust,no_run
//! let cfg = marty::Config::load()?;
//! let store = marty::SessionStore::from_config(&cfg)?;
//! # Ok::<(), marty::ConfigError>(())
//! ```

use crate::cgi::{CgiRequest, CgiResponse};
use crate::config::{Config, ConfigError};
use crate::cookie::{Cookie, SameSite};
use crate::form::parse_urlencoded;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

/// Key (env var and config file) read by [`SessionStore::from_config`].
pub const SECRET_KEY: &str = "MARTY_SESSION_SECRET";
/// Minimum secret length (bytes) accepted by [`SessionStore::from_config`].
pub const MIN_SECRET_LEN: usize = 32;

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

    /// Store keyed by `MARTY_SESSION_SECRET` (env var or config file).
    ///
    /// Errors if the secret is missing or shorter than [`MIN_SECRET_LEN`] bytes.
    /// Generate one with e.g. `openssl rand -hex 32`.
    pub fn from_config(cfg: &Config) -> Result<Self, ConfigError> {
        let secret = cfg.require(SECRET_KEY)?;
        if secret.len() < MIN_SECRET_LEN {
            return Err(ConfigError::Invalid {
                key: SECRET_KEY.into(),
                reason: format!(
                    "{} bytes, need at least {MIN_SECRET_LEN} (e.g. `openssl rand -hex 32`)",
                    secret.len()
                ),
            });
        }
        Ok(Self::new(secret))
    }

    pub fn cookie_name(mut self, name: impl Into<String>) -> Self {
        self.cookie_name = name.into();
        self
    }

    /// Lifetime in seconds (default 600), written as `Max-Age` **and** as the signed expiry
    /// checked by [`Self::load`]. `None`: browser-session cookie without a signed expiry, so a
    /// copied value stays valid until the secret changes.
    pub fn max_age(mut self, secs: Option<u64>) -> Self {
        self.max_age = secs;
        self
    }

    pub fn secure(mut self, on: bool) -> Self {
        self.secure = on;
        self
    }

    /// Load session from the request cookie (empty session if missing, invalid, or expired).
    pub fn load(&self, req: &CgiRequest) -> Session {
        let cookies = req.cookies();
        let Some(raw) = cookies.get(&self.cookie_name) else {
            return Session::default();
        };
        match verify_and_decode(&self.secret, raw, unix_now()) {
            Some(data) => Session { data, dirty: false },
            None => Session::default(),
        }
    }

    /// Attach a `Set-Cookie` when the session was mutated. The signed expiry restarts at
    /// every save; a session that is only read keeps its old expiry.
    pub fn save(&self, session: &Session, resp: CgiResponse) -> CgiResponse {
        if !session.dirty {
            return resp;
        }
        let exp = self
            .max_age
            .map_or(0, |secs| unix_now().saturating_add(secs));
        let value = sign_and_encode(&self.secret, &session.data, exp);
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

const FORMAT: &str = "v1";

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// `v1.<exp>.<payload>.<sig>`; the signature covers everything before the last `.`.
fn sign_and_encode(secret: &[u8], data: &HashMap<String, String>, exp: u64) -> String {
    let signed = format!("{FORMAT}.{exp}.{}", encode_map(data));
    let sig = hmac_hex(secret, signed.as_bytes());
    format!("{signed}.{sig}")
}

fn verify_and_decode(secret: &[u8], raw: &str, now: u64) -> Option<HashMap<String, String>> {
    let (signed, sig) = raw.rsplit_once('.')?;
    let expected = hmac_hex(secret, signed.as_bytes());
    if !constant_time_eq(sig.as_bytes(), expected.as_bytes()) {
        return None;
    }
    let rest = signed.strip_prefix(FORMAT)?.strip_prefix('.')?;
    let (exp, payload) = rest.split_once('.')?;
    let exp: u64 = exp.parse().ok()?;
    if exp != 0 && now >= exp {
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
        let raw = sign_and_encode(secret, &map, 2_000);
        assert!(raw.starts_with("v1.2000.count=3."), "{raw}");
        let back = verify_and_decode(secret, &raw, 1_000).unwrap();
        assert_eq!(back.get("count").map(String::as_str), Some("3"));
        assert!(verify_and_decode(secret, &(raw.clone() + "x"), 1_000).is_none());
        assert!(verify_and_decode(b"other-secret", &raw, 1_000).is_none());
    }

    #[test]
    fn from_config_requires_long_secret() {
        let short = Config::parse("MARTY_SESSION_SECRET=too-short\n").unwrap();
        let err = SessionStore::from_config(&short).unwrap_err().to_string();
        assert!(err.contains("need at least 32"), "{err}");

        let ok = Config::parse("MARTY_SESSION_SECRET=0123456789abcdef0123456789abcdef\n").unwrap();
        assert!(SessionStore::from_config(&ok).is_ok());

        let missing = Config::parse("").unwrap();
        let err = SessionStore::from_config(&missing).unwrap_err().to_string();
        assert!(err.contains(SECRET_KEY), "{err}");
    }

    #[test]
    fn expiry_is_signed_and_enforced() {
        let secret = b"test-secret-key-please-rotate";
        let mut map = HashMap::new();
        map.insert("user".into(), "42".into());
        let raw = sign_and_encode(secret, &map, 1_000);
        assert!(verify_and_decode(secret, &raw, 999).is_some());
        assert!(verify_and_decode(secret, &raw, 1_000).is_none());
        assert!(verify_and_decode(secret, &raw, 5_000).is_none());

        // Pushing the expiry forward breaks the signature.
        let forged = raw.replacen("v1.1000.", "v1.9999999999.", 1);
        assert!(verify_and_decode(secret, &forged, 5_000).is_none());

        // exp = 0: no expiry.
        let forever = sign_and_encode(secret, &map, 0);
        assert!(verify_and_decode(secret, &forever, u64::MAX).is_some());
    }

    #[test]
    fn pre_v1_cookies_are_rejected() {
        // 0.2.0 format: `<payload>.<hmac(payload)>`.
        let secret = b"test-secret-key-please-rotate";
        let payload = "count=3";
        let old = format!("{payload}.{}", hmac_hex(secret, payload.as_bytes()));
        assert!(verify_and_decode(secret, &old, 1).is_none());
    }

    #[test]
    fn values_with_dots_survive() {
        let mut map = HashMap::new();
        map.insert("next".into(), "/a.b/c.d".into());
        let raw = sign_and_encode(b"s", &map, 0);
        let back = verify_and_decode(b"s", &raw, 1).unwrap();
        assert_eq!(back.get("next").map(String::as_str), Some("/a.b/c.d"));
    }

    #[test]
    fn percent_roundtrip_in_map() {
        let mut map = HashMap::new();
        map.insert("k".into(), "a/b c".into());
        let raw = sign_and_encode(b"s", &map, 0);
        let back = verify_and_decode(b"s", &raw, 1).unwrap();
        assert_eq!(back.get("k").map(String::as_str), Some("a/b c"));
    }
}
