//! Example **05_sqlite**: `rusqlite` with Axum behind `marty::serve_cgi`.
//!
//! One GET creates a small table (if needed), inserts a row, and returns names read back — all in
//! a single request. Database file: `./marty_05_sqlite.db` (relative to the CGI process cwd).

use axum::{Router, routing::get};
use marty::serve_cgi;
use rusqlite::Connection;

#[tokio::main]
async fn main() {
    let app = Router::new().route(
        "/cgi-bin/marty-05-sqlite",
        get(|| async move {
            let conn = Connection::open("./marty_05_sqlite.db").unwrap();
            conn.execute(
                "CREATE TABLE IF NOT EXISTS user (
                    id INTEGER PRIMARY KEY,
                    name TEXT NOT NULL,
                    age INTEGER
                )",
                [],
            )
            .unwrap();

            let mut stmt = conn
                .prepare("INSERT INTO user (name, age) VALUES (?,?)")
                .unwrap();
            stmt.execute(["Bob", "42"]).unwrap();

            let mut stmt = conn.prepare("SELECT name FROM user ORDER BY id").unwrap();
            let mut out = String::from("05_sqlite: ");
            let mut rows = stmt.query([]).unwrap();
            while let Some(row) = rows.next().unwrap() {
                let name: String = row.get(0).unwrap();
                out.push_str(&name);
                out.push(' ');
            }
            out.push('\n');
            out
        }),
    );

    if let Err(e) = serve_cgi(app).await {
        eprintln!("Error while serving CGI request: {e}");
        std::process::exit(1);
    }
}
