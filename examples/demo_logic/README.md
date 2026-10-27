# `marty-demo-logic`

Shared demo **business** logic for Marty [`examples/`](../) and cloud [`alternatives/`](../../alternatives/). Runtimes differ (CGI / Lambda / Azure); strings and visit formatting stay one crate so the comparison stays honest.

| API | Consumers |
|-----|-----------|
| `basic::BODY` | `01_basic`, AWS/Azure `01_basic` |
| `visit::{NAME, AGE, format_names}` | `05_sqlite`, `07_mysql`, AWS `02_dynamodb`, `03_mysql` |

```rust
use marty_demo_logic::{basic, visit};

assert_eq!(basic::body(), "88 miles per hour!");
let line = visit::format_names("05_sqlite", [visit::NAME]);
```
