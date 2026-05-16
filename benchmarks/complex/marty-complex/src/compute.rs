//! Shared CPU workloads (same algorithms in go/python/php under benchmarks/complex/).

/// Sieve of Eratosthenes — count primes strictly below `limit`.
/// Inspired by classic language benchmark suites (e.g. prime counting in HsBenchMarkSuite).
pub fn prime_count(limit: usize) -> u64 {
    if limit <= 2 {
        return 0;
    }
    let mut sieve = vec![true; limit];
    sieve[0] = false;
    sieve[1] = false;
    let mut count = 0u64;
    let mut i = 2usize;
    while i < limit {
        if sieve[i] {
            count += 1;
            let mut j = i.saturating_mul(i);
            while j < limit {
                sieve[j] = false;
                j = j.saturating_add(i);
            }
        }
        i += 1;
    }
    count
}

pub const FIB_SEED: u32 = 42;
pub const FIB_REPEAT_BASE: u32 = 3_000;
pub const FIB_REPEAT_SALT_MOD: u32 = 7_000;

/// Iterative Fibonacci.
pub fn fibonacci(n: u32) -> u64 {
    if n == 0 {
        return 0;
    }
    let (mut a, mut b) = (0u64, 1u64);
    for _ in 1..n {
        let next = a.saturating_add(b);
        a = b;
        b = next;
    }
    b
}

/// Salt-dependent CPU load: `repeats = FIB_REPEAT_BASE + (salt % FIB_REPEAT_SALT_MOD)` calls to `fibonacci(FIB_SEED)`.
pub fn fibonacci_work(salt: u64) -> u64 {
    let repeats = FIB_REPEAT_BASE + (salt as u32 % FIB_REPEAT_SALT_MOD);
    let mut acc = 0u64;
    for _ in 0..repeats {
        acc = acc.wrapping_add(fibonacci(FIB_SEED));
    }
    acc
}

/// Naive dense matrix multiply — checksum prevents dead-code elimination.
pub fn matrix_checksum(n: usize) -> f64 {
    if n == 0 {
        return 0.0;
    }
    let size = n * n;
    let mut a = vec![0.0f64; size];
    let mut b = vec![0.0f64; size];
    let mut c = vec![0.0f64; size];
    for i in 0..size {
        a[i] = i as f64 * 0.001;
        b[i] = i as f64 * 0.002;
    }
    for i in 0..n {
        for j in 0..n {
            let mut sum = 0.0;
            for k in 0..n {
                sum += a[i * n + k] * b[k * n + j];
            }
            c[i * n + j] = sum;
        }
    }
    c.iter().sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fib_42() {
        assert_eq!(fibonacci(42), 267_914_296);
    }

    #[test]
    fn fib_work_salt_zero() {
        assert_eq!(fibonacci_work(0), 803_742_888_000);
    }

    #[test]
    fn primes_small() {
        assert_eq!(prime_count(10), 4);
    }
}
