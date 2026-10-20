//! Per-request salt so workloads cannot return a cached constant result.

pub const SALT_MOD_PRIME: usize = 5_000;
pub const SALT_MOD_MATRIX: usize = 16;

/// Explicit `?salt=` from the query string, or high-resolution time (unique per request).
pub fn request_salt(explicit: Option<u64>) -> u64 {
    explicit.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or_else(|_| std::process::id() as u64)
    })
}

pub fn effective_prime_limit(base: usize, salt: u64, max: usize) -> usize {
    (base + (salt as usize % SALT_MOD_PRIME)).min(max)
}

pub fn effective_matrix_size(base: usize, salt: u64, max: usize) -> usize {
    (base + (salt as usize % SALT_MOD_MATRIX)).min(max)
}
