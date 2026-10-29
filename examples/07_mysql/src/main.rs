//! Example **07_mysql**: MySQL/MariaDB read+write behind sync [`marty::Router`].
//!
//! Aimed at **all-inkl–style** shared hosting: one musl CGI binary, DB on `localhost`.
//! Credentials come from env and/or `marty_mysql.conf` (suexec does **not** pass
//! `.htaccess` `SetEnv` — see README).
//!
//! One GET: `CREATE TABLE IF NOT EXISTS`, insert a row, list names.
//! Insert/format via [`marty_demo_logic::visit`] (same as SQLite / cloud MySQL twins).
//!
//! `.htaccess` / deploy: [`../02_routing/README.md`](../02_routing/README.md).

use marty::{CgiResponse, Router};
use marty_demo_logic::visit;
use mysql::prelude::*;
use mysql::{Opts, OptsBuilder, Pool};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

fn main() {
    let mut router = Router::new();
    router.get("/", |_| match handle_visit() {
        Ok(body) => CgiResponse::text(body),
        Err(e) => CgiResponse::new(503, format!("07_mysql: {e}\n")),
    });
    router.fallback(|_| (404u16, "Not found\n"));

    if let Err(e) = router.serve() {
        eprintln!("Error while serving CGI request: {e}");
        std::process::exit(1);
    }
}

fn handle_visit() -> Result<String, String> {
    let opts = db_opts()?;
    let pool = Pool::new(opts).map_err(|e| format!("connect: {e}"))?;
    let mut conn = pool.get_conn().map_err(|e| format!("get_conn: {e}"))?;

    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS marty_user (
            id INT AUTO_INCREMENT PRIMARY KEY,
            name VARCHAR(64) NOT NULL,
            age INT NULL
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4",
    )
    .map_err(|e| format!("create table: {e}"))?;

    conn.exec_drop(
        "INSERT INTO marty_user (name, age) VALUES (?, ?)",
        (visit::NAME, visit::AGE),
    )
    .map_err(|e| format!("insert: {e}"))?;

    let names: Vec<String> = conn
        .query("SELECT name FROM marty_user ORDER BY id")
        .map_err(|e| format!("select: {e}"))?;

    Ok(visit::format_names("07_mysql", names))
}

/// Build [`Opts`] from `MARTY_MYSQL_*` env and/or `marty_mysql.conf` (cwd or next to binary).
fn db_opts() -> Result<Opts, String> {
    let mut paths = vec![Path::new("marty_mysql.conf").to_path_buf()];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            paths.push(dir.join("marty_mysql.conf"));
        }
    }
    let file = load_conf_first(&paths);
    let host = env_or_file("MARTY_MYSQL_HOST", "host", &file)
        .unwrap_or_else(|| "localhost".into());
    let user = env_or_file("MARTY_MYSQL_USER", "user", &file)
        .ok_or("missing user (set MARTY_MYSQL_USER or marty_mysql.conf)")?;
    let pass = env_or_file("MARTY_MYSQL_PASSWORD", "password", &file).unwrap_or_default();
    let db = env_or_file("MARTY_MYSQL_DATABASE", "database", &file)
        .ok_or("missing database (set MARTY_MYSQL_DATABASE or marty_mysql.conf)")?;
    let port: u16 = env_or_file("MARTY_MYSQL_PORT", "port", &file)
        .unwrap_or_else(|| "3306".into())
        .parse()
        .map_err(|_| "invalid port".to_string())?;

    Ok(OptsBuilder::new()
        .ip_or_hostname(Some(host))
        .tcp_port(port)
        .user(Some(user))
        .pass(Some(pass))
        .db_name(Some(db))
        .into())
}

fn env_or_file(env_key: &str, file_key: &str, file: &HashMap<String, String>) -> Option<String> {
    std::env::var(env_key)
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| file.get(file_key).cloned())
}

fn load_conf_first(paths: &[std::path::PathBuf]) -> HashMap<String, String> {
    for path in paths {
        let map = load_conf_file(path);
        if !map.is_empty() {
            return map;
        }
    }
    HashMap::new()
}

fn load_conf_file(path: &Path) -> HashMap<String, String> {
    let Ok(text) = fs::read_to_string(path) else {
        return HashMap::new();
    };
    let mut map = HashMap::new();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        map.insert(k.trim().to_string(), v.trim().to_string());
    }
    map
}
