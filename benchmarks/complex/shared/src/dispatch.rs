//! HTTP path dispatch for complex CPU routes (hosting CGI + cloud twins).
//!
//! Paths (optional `/api` prefix for Azure-style front doors):
//! - `GET /primes/{limit}?salt=`
//! - `GET /fibonacci/{n}?salt=`
//! - `GET /matrix/{size}?salt=`

use crate::compute::{fibonacci_work, matrix_checksum, prime_count};
use crate::json::{write_compute_matrix, write_compute_u64};
use crate::salt::{effective_matrix_size, effective_prime_limit, request_salt};
use crate::{
    DEFAULT_MATRIX_SIZE, DEFAULT_PRIME_LIMIT, MAX_MATRIX_SIZE, MAX_PRIME_LIMIT, parse_salt,
    parse_u64_ascii,
};

/// Strip query, trailing slash, and optional `/api` prefix used by Azure Functions.
pub fn normalize_path(path: &str) -> &str {
    let path = path.split('?').next().unwrap_or(path);
    let path = path.trim_end_matches('/');
    let path = if path.is_empty() { "/" } else { path };
    if let Some(rest) = path.strip_prefix("/api") {
        if rest.is_empty() { "/" } else { rest }
    } else {
        path
    }
}

/// Run a complex route. Returns JSON body bytes, or `None` if the path does not match.
pub fn run_complex_route(path: &str, query: &str) -> Option<Vec<u8>> {
    let path = normalize_path(path);
    let salt = request_salt(parse_salt(query));

    if let Some(limit) = path.strip_prefix("/primes/") {
        let base = parse_u64_ascii(limit)
            .and_then(|n| usize::try_from(n).ok())
            .unwrap_or(DEFAULT_PRIME_LIMIT)
            .min(MAX_PRIME_LIMIT);
        let effective = effective_prime_limit(base, salt, MAX_PRIME_LIMIT);
        let result = prime_count(effective);
        let mut buf = Vec::with_capacity(128);
        write_compute_u64(&mut buf, "primes", salt, result);
        return Some(buf);
    }

    if let Some(_n) = path.strip_prefix("/fibonacci/") {
        let result = fibonacci_work(salt);
        let mut buf = Vec::with_capacity(128);
        write_compute_u64(&mut buf, "fibonacci", salt, result);
        return Some(buf);
    }

    if let Some(size) = path.strip_prefix("/matrix/") {
        let base = parse_u64_ascii(size)
            .and_then(|n| usize::try_from(n).ok())
            .unwrap_or(DEFAULT_MATRIX_SIZE)
            .min(MAX_MATRIX_SIZE);
        let n = effective_matrix_size(base, salt, MAX_MATRIX_SIZE);
        let checksum = matrix_checksum(n);
        let mut buf = Vec::with_capacity(192);
        write_compute_matrix(&mut buf, "matrix", salt, n, checksum);
        return Some(buf);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_api() {
        assert_eq!(normalize_path("/api/primes/10"), "/primes/10");
        assert_eq!(normalize_path("/primes/10"), "/primes/10");
    }

    #[test]
    fn primes_salt_zero() {
        let body = run_complex_route("/primes/400000", "salt=0").expect("primes");
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("\"route\":\"primes\""));
        assert!(s.contains("\"salt\":0"));
        assert!(s.contains("\"result\":33860"));
    }

    #[test]
    fn matrix_size_in_json() {
        let body = run_complex_route("/api/matrix/128", "salt=0").expect("matrix");
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("\"size\":128"));
    }
}
