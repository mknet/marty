//! Shared **demo logic** for Marty CGI examples and cloud alternatives.
//!
//! Hosts (CGI, Lambda, Azure, Workers, Scaleway, …) stay different; this crate holds the bits that
//! should stay identical so the pitch is “same app logic, different runtime.”
//!
//! | Module | Used by |
//! |--------|---------|
//! | [`basic`] | [`01_basic`](../01_basic/), AWS/Azure/Cloudflare/Scaleway `01_basic` |
//! | [`visit`] | [`05_sqlite`](../05_sqlite/), [`07_mysql`](../07_mysql/), AWS `02_dynamodb` / `03_mysql` |

#![forbid(unsafe_code)]

/// Smallest demo: fixed plain-text body (Marty `01_basic` and cloud twins).
pub mod basic {
    /// Response body for the “hello world” sample.
    pub const BODY: &str = "88 miles per hour!";

    #[inline]
    pub fn body() -> &'static str {
        BODY
    }
}

/// “Visit” demo: insert one user, list names (SQLite / MySQL / DynamoDB twins).
pub mod visit {
    /// Demo user name inserted on each visit.
    pub const NAME: &str = "Bob";

    /// Demo user age inserted on each visit.
    pub const AGE: i32 = 42;

    /// Age as decimal string (DynamoDB `N`, etc.).
    pub const AGE_STR: &str = "42";

    /// Format `"{label}: Name1 Name2 …\n"` — same shape as Marty `05_sqlite` / `07_mysql`.
    pub fn format_names(label: &str, names: impl IntoIterator<Item = impl AsRef<str>>) -> String {
        let mut out = String::from(label);
        out.push_str(": ");
        for name in names {
            out.push_str(name.as_ref());
            out.push(' ');
        }
        out.push('\n');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_body() {
        assert_eq!(basic::body(), "88 miles per hour!");
    }

    #[test]
    fn visit_line() {
        assert_eq!(
            visit::format_names("05_sqlite", ["Bob", "Bob"]),
            "05_sqlite: Bob Bob \n"
        );
    }
}
