//! Shared primes / fibonacci / matrix algorithms for complex CGI benches.

pub mod compute;
pub mod json;
pub mod salt;

pub const DEFAULT_PRIME_LIMIT: usize = 400_000;
pub const DEFAULT_MATRIX_SIZE: usize = 128;
pub const MAX_PRIME_LIMIT: usize = 1_000_000;
pub const MAX_MATRIX_SIZE: usize = 400;

pub fn query_param<'a>(query_string: &'a str, name: &str) -> Option<&'a str> {
    query_string.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        (key == name).then_some(value)
    })
}

pub fn parse_salt(query_string: &str) -> Option<u64> {
    query_param(query_string, "salt").and_then(parse_u64_ascii)
}

pub fn parse_u64_ascii(s: &str) -> Option<u64> {
    if s.is_empty() {
        return None;
    }
    let mut n = 0u64;
    for b in s.bytes() {
        if !b.is_ascii_digit() {
            return None;
        }
        n = n.checked_mul(10)?.checked_add(u64::from(b - b'0'))?;
    }
    Some(n)
}

pub fn parse_usize_arg(arg: Option<&str>, default: usize, max: usize) -> usize {
    arg.and_then(parse_u64_ascii)
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(default)
        .min(max)
}
