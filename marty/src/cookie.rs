//! Cookie parsing and `Set-Cookie` helpers for sync CGI (no tower-cookies).

use crate::MetaVariableKind;
use crate::cgi::{CgiRequest, CgiResponse};
use std::collections::HashMap;
use std::fmt::Write as _;

/// A cookie to send with `Set-Cookie` or a name/value pair from `Cookie`.
#[derive(Debug, Clone)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub path: Option<String>,
    pub domain: Option<String>,
    pub max_age: Option<u64>,
    pub http_only: bool,
    pub secure: bool,
    pub same_site: Option<SameSite>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SameSite {
    Strict,
    Lax,
    None,
}

impl Cookie {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            path: Some("/".into()),
            domain: None,
            max_age: None,
            http_only: false,
            secure: false,
            same_site: None,
        }
    }

    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn http_only(mut self, on: bool) -> Self {
        self.http_only = on;
        self
    }

    pub fn secure(mut self, on: bool) -> Self {
        self.secure = on;
        self
    }

    pub fn max_age(mut self, secs: u64) -> Self {
        self.max_age = Some(secs);
        self
    }

    pub fn same_site(mut self, ss: SameSite) -> Self {
        self.same_site = Some(ss);
        self
    }

    /// Format as a single `Set-Cookie` header value.
    pub fn encode(&self) -> String {
        let mut s = format!("{}={}", self.name, self.value);
        if let Some(path) = &self.path {
            let _ = write!(s, "; Path={path}");
        }
        if let Some(domain) = &self.domain {
            let _ = write!(s, "; Domain={domain}");
        }
        if let Some(max_age) = self.max_age {
            let _ = write!(s, "; Max-Age={max_age}");
        }
        if self.http_only {
            s.push_str("; HttpOnly");
        }
        if self.secure {
            s.push_str("; Secure");
        }
        if let Some(ss) = self.same_site {
            s.push_str(match ss {
                SameSite::Strict => "; SameSite=Strict",
                SameSite::Lax => "; SameSite=Lax",
                SameSite::None => "; SameSite=None",
            });
        }
        s
    }
}

/// Cookies from the request `Cookie` header.
#[derive(Debug, Clone, Default)]
pub struct Cookies {
    map: HashMap<String, String>,
}

impl Cookies {
    pub fn from_header(header: &str) -> Self {
        let mut map = HashMap::new();
        for part in header.split(';') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let (name, value) = part.split_once('=').unwrap_or((part, ""));
            map.insert(name.trim().to_string(), value.trim().to_string());
        }
        Self { map }
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.map.get(name).map(String::as_str)
    }
}

impl CgiRequest {
    /// Raw `HTTP_COOKIE` meta-variable.
    pub fn cookie_header(&self) -> &str {
        self.cookie_header.as_str()
    }

    /// Parsed request cookies.
    pub fn cookies(&self) -> Cookies {
        Cookies::from_header(&self.cookie_header)
    }
}

impl CgiResponse {
    /// Append a `Set-Cookie` header (disables the JSON fast-write path).
    pub fn set_cookie(self, cookie: Cookie) -> Self {
        self.header("Set-Cookie", cookie.encode())
    }
}

pub(crate) fn cookie_header_from_env() -> String {
    MetaVariableKind::HttpCookie
        .from_env()
        .and_then(|v| v.as_str().ok().map(str::to_string))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cookie_header() {
        let c = Cookies::from_header("a=1; b=two");
        assert_eq!(c.get("a"), Some("1"));
        assert_eq!(c.get("b"), Some("two"));
    }

    #[test]
    fn encode_set_cookie() {
        let c = Cookie::new("marty_04", "x")
            .http_only(true)
            .max_age(60)
            .same_site(SameSite::Lax);
        let s = c.encode();
        assert!(s.contains("marty_04=x"));
        assert!(s.contains("HttpOnly"));
        assert!(s.contains("Max-Age=60"));
        assert!(s.contains("SameSite=Lax"));
    }
}
