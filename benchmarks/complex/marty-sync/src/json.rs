//! Fixed-shape JSON for bench responses (no serde on the hot path).

use crate::timing::PhaseProfile;
use std::io::Write;

pub fn write_compute_u64(
    out: &mut Vec<u8>,
    route: &str,
    salt: u64,
    result: u64,
    profile: Option<&PhaseProfile>,
) {
    out.clear();
    append_compute_u64(out, route, salt, result, profile);
}

pub fn append_compute_u64(
    out: &mut Vec<u8>,
    route: &str,
    salt: u64,
    result: u64,
    profile: Option<&PhaseProfile>,
) {
    out.extend_from_slice(b"{\"route\":\"");
    out.extend_from_slice(route.as_bytes());
    out.extend_from_slice(b"\",\"salt\":");
    append_u64(out, salt);
    out.extend_from_slice(b",\"result\":");
    append_u64(out, result);
    if let Some(p) = profile {
        append_profile(out, p);
    }
    out.push(b'}');
}

pub fn write_compute_matrix(
    out: &mut Vec<u8>,
    route: &str,
    salt: u64,
    size: usize,
    checksum: f64,
    profile: Option<&PhaseProfile>,
) {
    out.clear();
    append_compute_matrix(out, route, salt, size, checksum, profile);
}

pub fn append_compute_matrix(
    out: &mut Vec<u8>,
    route: &str,
    salt: u64,
    size: usize,
    checksum: f64,
    profile: Option<&PhaseProfile>,
) {
    out.extend_from_slice(b"{\"route\":\"");
    out.extend_from_slice(route.as_bytes());
    out.extend_from_slice(b"\",\"salt\":");
    append_u64(out, salt);
    out.extend_from_slice(b",\"result\":{\"size\":");
    append_usize(out, size);
    out.extend_from_slice(b",\"checksum\":");
    append_f64(out, checksum);
    out.extend_from_slice(b"}");
    if let Some(p) = profile {
        append_profile(out, p);
    }
    out.extend_from_slice(b"}");
}

fn append_f64(out: &mut Vec<u8>, v: f64) {
    let _ = write!(out, "{v}");
}

fn append_profile(out: &mut Vec<u8>, p: &PhaseProfile) {
    out.extend_from_slice(b",\"profile\":{");
    let mut first = true;
    if let Some(v) = p.until_process_us {
        append_field_u128(out, &mut first, "until_process_us", v);
    }
    if let Some(v) = p.until_compute_us {
        append_field_u128(out, &mut first, "until_compute_us", v);
    }
    append_field_u128(out, &mut first, "startup_us", p.startup_us);
    append_field_u128(out, &mut first, "handler_setup_us", p.handler_setup_us);
    append_field_u128(out, &mut first, "compute_us", p.compute_us);
    append_field_u128(out, &mut first, "post_compute_us", p.post_compute_us);
    append_field_u128(out, &mut first, "total_us", p.total_us);
    out.push(b'}');
}

fn append_field_u128(out: &mut Vec<u8>, first: &mut bool, key: &str, v: u128) {
    if !*first {
        out.push(b',');
    }
    *first = false;
    out.extend_from_slice(b"\"");
    out.extend_from_slice(key.as_bytes());
    out.extend_from_slice(b"\":");
    append_u128(out, v);
}

fn append_u64(out: &mut Vec<u8>, n: u64) {
    let _ = write!(out, "{n}");
}

fn append_u128(out: &mut Vec<u8>, n: u128) {
    let _ = write!(out, "{n}");
}

fn append_usize(out: &mut Vec<u8>, n: usize) {
    append_u64(out, n as u64);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u64_json_valid() {
        let mut buf = Vec::new();
        write_compute_u64(&mut buf, "primes", 0, 3, None);
        let s = std::str::from_utf8(&buf).unwrap();
        assert_eq!(s, "{\"route\":\"primes\",\"salt\":0,\"result\":3}");
    }

    #[test]
    fn matrix_json_valid() {
        let mut buf = Vec::new();
        write_compute_matrix(&mut buf, "matrix", 0, 128, 1.5, None);
        let s = std::str::from_utf8(&buf).unwrap();
        assert!(s.ends_with("}}"), "got {s}");
        assert_eq!(s.matches('}').count(), 2);
    }
}
