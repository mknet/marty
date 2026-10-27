//! Alternative to Marty [`examples/07_mysql`](../../../../examples/07_mysql/):
//! same visit pattern (CREATE TABLE IF NOT EXISTS, insert Bob/42, list names) against MySQL.
//!
//! On shared hosting, MySQL is usually in the flat fee and reachable at `localhost`.
//! On Lambda you need a network-reachable engine (typically **RDS** or **Aurora**) plus VPC
//! or public access — a higher cost floor than DynamoDB. Prefer
//! [`02_dynamodb`](../02_dynamodb/) when a few rows are enough (Marty
//! [`examples/05_sqlite`](../../../../examples/05_sqlite/) analogy).

use lambda_http::{run, service_fn, tracing, Body, Error, Request, Response};
use marty_demo_logic::visit;
use mysql_async::prelude::*;
use mysql_async::{Opts, OptsBuilder, Pool};
use std::env;

async fn function_handler(pool: &Pool, _event: Request) -> Result<Response<Body>, Error> {
    let mut conn = pool.get_conn().await.map_err(|e| format!("get_conn: {e}"))?;

    conn.query_drop(
        "CREATE TABLE IF NOT EXISTS marty_user (
            id INT AUTO_INCREMENT PRIMARY KEY,
            name VARCHAR(64) NOT NULL,
            age INT NULL
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4",
    )
    .await
    .map_err(|e| format!("create table: {e}"))?;

    conn.exec_drop(
        "INSERT INTO marty_user (name, age) VALUES (?, ?)",
        (visit::NAME, visit::AGE),
    )
    .await
    .map_err(|e| format!("insert: {e}"))?;

    let names: Vec<String> = conn
        .query_map("SELECT name FROM marty_user ORDER BY id", |name: String| name)
        .await
        .map_err(|e| format!("select: {e}"))?;

    let out = visit::format_names("03_mysql", names);

    let resp = Response::builder()
        .status(200)
        .header("content-type", "text/plain; charset=utf-8")
        .body(Body::from(out))
        .map_err(Box::new)?;
    Ok(resp)
}

fn db_opts() -> Result<Opts, String> {
    let host = env::var("MYSQL_HOST").map_err(|_| "set MYSQL_HOST".to_string())?;
    let user = env::var("MYSQL_USER").map_err(|_| "set MYSQL_USER".to_string())?;
    let pass = env::var("MYSQL_PASSWORD").unwrap_or_default();
    let db = env::var("MYSQL_DATABASE").map_err(|_| "set MYSQL_DATABASE".to_string())?;
    let port: u16 = env::var("MYSQL_PORT")
        .unwrap_or_else(|_| "3306".into())
        .parse()
        .map_err(|_| "invalid MYSQL_PORT".to_string())?;

    Ok(OptsBuilder::default()
        .ip_or_hostname(host)
        .tcp_port(port)
        .user(Some(user))
        .pass(Some(pass))
        .db_name(Some(db))
        .into())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    let opts = db_opts().map_err(|e| format!("db config: {e}"))?;
    let pool = Pool::new(opts);

    run(service_fn(move |event| {
        let pool = pool.clone();
        async move { function_handler(&pool, event).await }
    }))
    .await
}
