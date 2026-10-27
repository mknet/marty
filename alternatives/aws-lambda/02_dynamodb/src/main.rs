//! Alternative to Marty [`examples/05_sqlite`](../../../../examples/05_sqlite/):
//! same visit pattern (insert demo user, list names) — but **DynamoDB**, not a local file.
//!
//! Name/age/format from [`marty_demo_logic::visit`]. Lambda has no durable local SQLite.

use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::Client;
use lambda_http::{run, service_fn, tracing, Body, Error, Request, Response};
use marty_demo_logic::visit;
use std::env;
use uuid::Uuid;

async fn function_handler(client: &Client, table: &str, _event: Request) -> Result<Response<Body>, Error> {
    client
        .put_item()
        .table_name(table)
        .item("id", AttributeValue::S(Uuid::new_v4().to_string()))
        .item("name", AttributeValue::S(visit::NAME.into()))
        .item("age", AttributeValue::N(visit::AGE_STR.into()))
        .send()
        .await?;

    let scan = client.scan().table_name(table).send().await?;
    let mut names: Vec<String> = scan
        .items()
        .iter()
        .filter_map(|item| item.get("name").and_then(|v| v.as_s().ok()).cloned())
        .collect();
    names.sort();

    let out = visit::format_names("02_dynamodb", names);

    let resp = Response::builder()
        .status(200)
        .header("content-type", "text/plain; charset=utf-8")
        .body(Body::from(out))
        .map_err(Box::new)?;
    Ok(resp)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    let table = env::var("TABLE_NAME").unwrap_or_else(|_| "alt-aws-lambda-02-user".into());
    let config = aws_config::load_from_env().await;
    let client = Client::new(&config);

    run(service_fn(|event| {
        let client = client.clone();
        let table = table.clone();
        async move { function_handler(&client, &table, event).await }
    }))
    .await
}
