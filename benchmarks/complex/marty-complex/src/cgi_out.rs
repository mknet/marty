//! Minimal CGI document response (no framework deps).

use std::io::{self, Write};

const STATUS_200_OK: &[u8] = b"Status: 200 OK\n";
const JSON_NO_CACHE_HEADERS: &[u8] =
    b"Cache-Control: no-store, no-cache, must-revalidate\nPragma: no-cache\nContent-Type: application/json\n\n";

pub fn write_json_200(body: &[u8]) -> io::Result<()> {
    let mut out = io::stdout();
    out.write_all(STATUS_200_OK)?;
    out.write_all(JSON_NO_CACHE_HEADERS)?;
    out.write_all(body)?;
    Ok(())
}

pub fn write_status(status: u16, reason: &str, body: &[u8]) -> io::Result<()> {
    let mut out = io::stdout();
    writeln!(out, "Status: {status} {reason}")?;
    writeln!(out, "Cache-Control: no-store")?;
    writeln!(out)?;
    out.write_all(body)?;
    Ok(())
}
