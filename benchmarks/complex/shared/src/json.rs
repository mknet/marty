//! Fixed-shape JSON for hosting benches (no serde, no profile fields).

use std::io::Write;

pub fn write_compute_u64(out: &mut Vec<u8>, route: &str, salt: u64, result: u64) {
    out.clear();
    out.extend_from_slice(b"{\"route\":\"");
    out.extend_from_slice(route.as_bytes());
    out.extend_from_slice(b"\",\"salt\":");
    let _ = write!(out, "{salt}");
    out.extend_from_slice(b",\"result\":");
    let _ = write!(out, "{result}");
    out.push(b'}');
}

pub fn write_compute_matrix(out: &mut Vec<u8>, route: &str, salt: u64, size: usize, checksum: f64) {
    out.clear();
    out.extend_from_slice(b"{\"route\":\"");
    out.extend_from_slice(route.as_bytes());
    out.extend_from_slice(b"\",\"salt\":");
    let _ = write!(out, "{salt}");
    out.extend_from_slice(b",\"result\":{\"size\":");
    let _ = write!(out, "{size}");
    out.extend_from_slice(b",\"checksum\":");
    let _ = write!(out, "{checksum}");
    out.extend_from_slice(b"}}");
}
