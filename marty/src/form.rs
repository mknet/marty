//! Query string and `application/x-www-form-urlencoded` helpers (no serde).

use crate::cgi::CgiRequest;
use crate::MetaVariableKind;
use std::collections::HashMap;

/// Decode `application/x-www-form-urlencoded` / query bytes into owned key/value pairs.
///
/// Duplicate keys: last value wins. Values are percent-decoded; invalid UTF-8 becomes lossy.
pub fn parse_urlencoded(input: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for pair in input.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        map.insert(percent_decode(key), percent_decode(value));
    }
    map
}

/// Percent-decode a query/form component (`+` → space, `%XX` → byte).
pub fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                if let (Some(hi), Some(lo)) = (from_hex(bytes[i + 1]), from_hex(bytes[i + 2])) {
                    out.push((hi << 4) | lo);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn from_hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

impl CgiRequest {
    /// `CONTENT_TYPE` meta-variable (may be empty).
    pub fn content_type(&self) -> &str {
        self.content_type.as_str()
    }

    /// Single query parameter from `QUERY_STRING` (last value if duplicated).
    pub fn query(&self, name: &str) -> Option<String> {
        self.query_map().get(name).cloned()
    }

    /// All query parameters (parsed lazily per call — fine for CGI).
    pub fn query_map(&self) -> HashMap<String, String> {
        parse_urlencoded(&self.query_string)
    }

    /// Parse body as `application/x-www-form-urlencoded` when Content-Type matches.
    ///
    /// Returns `None` if the content type is missing/other, or the body is not UTF-8.
    pub fn form(&self) -> Option<HashMap<String, String>> {
        let ct = self.content_type.to_ascii_lowercase();
        if !ct.starts_with("application/x-www-form-urlencoded") {
            return None;
        }
        let s = std::str::from_utf8(&self.body).ok()?;
        Some(parse_urlencoded(s))
    }

    /// Form field, or `None` if not a urlencoded form body / missing key.
    pub fn form_value(&self, name: &str) -> Option<String> {
        self.form()?.get(name).cloned()
    }
}

/// Read `CONTENT_TYPE` from the CGI environment (used while building [`CgiRequest`]).
pub(crate) fn content_type_from_env() -> String {
    MetaVariableKind::ContentType
        .from_env()
        .and_then(|v| v.as_str().ok().map(str::to_string))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_query() {
        let m = parse_urlencoded("a=1&b=hello+world&c=%2F");
        assert_eq!(m.get("a").map(String::as_str), Some("1"));
        assert_eq!(m.get("b").map(String::as_str), Some("hello world"));
        assert_eq!(m.get("c").map(String::as_str), Some("/"));
    }
}
