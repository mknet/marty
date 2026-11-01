//! Example **07_mysql**: MySQL/MariaDB read+write behind sync [`marty::Router`].
//!
//! Aimed at **all-inkl–style** shared hosting: one musl CGI binary, DB on `localhost`.
//! Credentials come from `MARTY_MYSQL_*` env vars or the same keys in the config file
//! outside the docroot (suexec does **not** pass `.htaccess` `SetEnv` — see README).
//!
//! One GET: `CREATE TABLE IF NOT EXISTS`, insert a row, list names.
//! Insert/format via [`marty_demo_logic::visit`] (same as SQLite / cloud MySQL twins).
//!
//! `.htaccess` / deploy: [`../02_routing/README.md`](../02_routing/README.md).

use marty::{CgiResponse, Config, Router};
use marty_demo_logic::visit;
use mysql::prelude::*;
use mysql::{Opts, OptsBuilder, Pool};

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

/// Build [`Opts`] from `MARTY_MYSQL_*` (env vars, then the config file; see [`Config::load`]).
fn db_opts() -> Result<Opts, String> {
    let cfg = Config::load().map_err(|e| e.to_string())?;
    let host = cfg
        .get("MARTY_MYSQL_HOST")
        .unwrap_or_else(|| "localhost".into());
    let user = cfg.require("MARTY_MYSQL_USER").map_err(|e| e.to_string())?;
    let pass = cfg.get("MARTY_MYSQL_PASSWORD").unwrap_or_default();
    let db = cfg
        .require("MARTY_MYSQL_DATABASE")
        .map_err(|e| e.to_string())?;
    let port: u16 = cfg
        .get("MARTY_MYSQL_PORT")
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
