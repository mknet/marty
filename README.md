# Marty

> *“Roads? Where we’re going, we don’t need roads.”*  
> — Doc Brown, *Back to the Future* (and yet: sometimes all you need is **one process per request**—and we call that **CGI**.)

**Marty** is the new name on [crates.io](https://crates.io) for Rust helpers around the [Common Gateway Interface](https://en.wikipedia.org/wiki/Common_Gateway_Interface) (RFC 3875). The line starts with [**cbgbt/cgi-rs**](https://github.com/cbgbt/cgi-rs) (the original project), continues in the [**mknet/cgi-rs**](https://github.com/mknet/cgi-rs) fork, and lands here—with more *Huey Lewis*, more *88 mph*, and the firm belief that your next HTTP stack does not need a DeLorean, only solid types.

This repository: [**github.com/mknet/marty**](https://github.com/mknet/marty).

## Lineage (who did what, no time paradox)

| Stage | Repository |
|--------|------------|
| **Original `cgi-rs`** | [**github.com/cbgbt/cgi-rs**](https://github.com/cbgbt/cgi-rs) |
| **Maintained fork** | [**github.com/mknet/cgi-rs**](https://github.com/mknet/cgi-rs) |
| **Crates.io & this tree** | [**github.com/mknet/marty**](https://github.com/mknet/marty) |

---

## Great Scott — what is this?

A CGI server **launches** (classically) a script as its **own process** for each incoming request. Think of it as a trip to another timeline: clean isolation, gone when the work is done, and the next request gets a fresh *timeline*.

For typical HTTP services, CGI’s role has long been superseded by models that keep **long-lived** worker processes around (*“where we’re going, we keep the server running”*). That buys you latency and throughput. CGI is still useful when you want **very low idle footprint** and are willing to trade some throughput for a **simple resource story** (one request, one process)—*low power mode* for Hill Valley instead of constant fire from the Mr. Fusion equivalent.

---

## Crates in the ecosystem (status: evolving from cgi-rs)

The [**cbgbt/cgi-rs**](https://github.com/cbgbt/cgi-rs) / [**mknet/cgi-rs**](https://github.com/mknet/cgi-rs) workspace is split into several crates. The story continues on crates.io under **Marty**; the exact layout in [**mknet/marty**](https://github.com/mknet/marty) follows the port.

| Theme | Idea |
|--------|------|
| **Core** | CGI environment → `hyper::Request` / `hyper::Response` |
| **Tower** | Use `tower::Service`—e.g. with **Axum**—and “serve” it as a CGI script |
| **Examples** | Sample scripts and optionally a small HTTP-CGI server to try things out |

If you return from the future and suddenly need **Windows**: upstream still listed that as a limitation—check the current README in the repo before you bet the farm on 1955.

---

## Tower / Axum: *“I guess you guys aren’t ready for that yet”*

This pattern comes straight from the cgi-rs README ([**cbgbt/cgi-rs**](https://github.com/cbgbt/cgi-rs) original; same text in the [**mknet/cgi-rs**](https://github.com/mknet/cgi-rs) fork): a **Tower** service (here `tower_cgi`; after a rename, possibly its own crate in the Marty workspace) forwards requests into your Axum app:

```rust ignore
// From the cgi-rs README; requires `tower-cgi` + Axum in your workspace.
use axum::{routing::get, Router};
use tower_cgi::serve_cgi;

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(|| async { "Hello, World!" }));
    serve_cgi(app).await.unwrap();
}
```

**Operation:** Like the *Enchantment Under the Sea* dance—steady tempo, one route at a time, and no panicking before the first note.

---

## Core: CGI → Hyper (meta-variables & request)

The core crate (historically **`cgi_rs`**, on crates.io as **`marty`**) maps the CGI environment to [`hyper::Request`](https://docs.rs/hyper/latest/hyper/struct.Request.html) / [`hyper::Response`](https://docs.rs/hyper/latest/hyper/struct.Response.html) so you can keep building with the rest of the **Hyper** ecosystem—without your code unraveling like *alternate 1985*.

### Current limitations (per upstream docs: [**cbgbt/cgi-rs**](https://github.com/cbgbt/cgi-rs), [**mknet/cgi-rs**](https://github.com/mknet/cgi-rs))

- Focused on **utilities for CGI scripts**, not a full CGI server in the crate sense.
- Only **“Document”-style** responses.
- Only a **subset** of CGI environment variables are hoisted into requests.
- **No Windows** (in the referenced upstream state).

### Example: parse an HTTP request from the CGI environment

```rust ignore
// Target API on crates.io; until the port lands you may still see `cgi_rs::…`.
use hyper::{Request, Body};
use marty::CGIRequest;

fn main() {
    // In a real CGI environment the server sets these variables (and more).
    std::env::set_var("REQUEST_METHOD", "GET");
    std::env::set_var("CONTENT_LENGTH", "0");
    std::env::set_var("REQUEST_URI", "/");

    let cgi_request: Request<Body> = CGIRequest::from_env()
        .and_then(Request::try_from)
        .unwrap();
}
```

> **Note:** While the public API is still being ported 1:1 from `cgi_rs`, your checkout may temporarily still show `cgi_rs::…`. The goal is a single **`marty::`** path on crates.io.

### Example: meta-variables (RFC 3875)

You can always read environment variables raw—the core also offers a **structured** way to fetch and parse CGI *meta-variables*:

```rust ignore
fn main() {
    let method = marty::MetaVariableKind::RequestMethod
        .try_from_env()
        .unwrap();
    assert_eq!(method.as_str().unwrap(), "GET");
}
```

If `REQUEST_METHOD` is missing, that is the wrong almanac in the suitcase: **double-check your server configuration** before you fix the timeline.

---

## License

Apache License, Version 2.0 (see `LICENSE-APACHE` in the repository once it is synced with upstream **cgi-rs**: [cbgbt/cgi-rs](https://github.com/cbgbt/cgi-rs) (original) and [mknet/cgi-rs](https://github.com/mknet/cgi-rs) (fork)).

---

## Acknowledgements / *Easter eggs*

- **Doc** would yell *“1.21 gigawatts!”*—here you often need fewer watts, but you do need solid **types**.
- **Marty** might forget to `git revert` *Johnny B. Goode*—you should still ship **versions** and a **changelog**.
- If something will not compile: `cargo clean`, then try again—like the **second attempt** at the flux capacitor wiring harness.

---

*“If you put your mind to it, you can accomplish anything.”* — George McFly (and a halfway decent `README.md`).
