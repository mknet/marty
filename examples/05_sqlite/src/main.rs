//! Example **05_sqlite**: `rusqlite` behind sync [`marty::Router`].
//!
//! One GET creates a small table (if needed), inserts a row, and returns names read back.
//! Insert/format via [`marty_demo_logic::visit`] (same as MySQL / DynamoDB twins).
//! Database file: `./marty_05_sqlite.db` (relative to the CGI process cwd).
//!
//! `.htaccess` / deploy: [`../02_routing/README.md`](../02_routing/README.md).

use marty::Router;
use marty_demo_logic::visit;
use rusqlite::Connection;

fn main() {
    let mut router = Router::new();
    router.get("/", |_| {
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
        stmt.execute([visit::NAME, visit::AGE_STR]).unwrap();

        let mut stmt = conn.prepare("SELECT name FROM user ORDER BY id").unwrap();
        let mut names = Vec::new();
        let mut rows = stmt.query([]).unwrap();
        while let Some(row) = rows.next().unwrap() {
            let name: String = row.get(0).unwrap();
            names.push(name);
        }
        visit::format_names("05_sqlite", names)
    });
    router.fallback(|_| (404u16, "Not found\n"));

    if let Err(e) = router.serve() {
        eprintln!("Error while serving CGI request: {e}");
        std::process::exit(1);
    }
}
