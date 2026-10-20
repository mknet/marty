use crate::Result;
use crate::cgi::{CgiRequest, CgiResponse, IntoResponse, relative_path};
use std::collections::HashMap;

type BoxedHandler = Box<dyn Fn(&CgiRequest) -> CgiResponse + Send + Sync>;

/// A synchronous CGI router with method-based routing and path parameters.
///
/// Routes are matched in registration order. Path patterns support:
/// - `"/"` — root
/// - `"/hello/{name}"` — one path segment → `req.param("name")`
/// - `"/static/{*path}"` — catch-all (must be last) → remaining segments joined by `/`
///
/// Compose routers with [`Router::nest`] (prefix) and [`Router::merge`] (append).
///
/// The Router automatically strips `SCRIPT_NAME` from the request path so that
/// routes are relative to the CGI script's mount point.
///
/// # Example
///
/// ```rust,ignore
/// use marty::Router;
///
/// let mut api = Router::new();
/// api.get("/users/{id}", |req| format!("user {}\n", req.param("id").unwrap()));
///
/// let mut router = Router::new();
/// router.nest("/api", api);
/// router.get("/static/{*path}", |req| {
///     format!("file {}\n", req.param("path").unwrap_or(""))
/// });
/// router.fallback(|_| (404u16, "Not found\n"));
/// router.serve().unwrap();
/// ```
pub struct Router {
    routes: Vec<Route>,
    /// Fallbacks installed by [`Router::nest`], tried (longest prefix first) before [`Self::fallback`].
    scoped_fallbacks: Vec<(String, BoxedHandler)>,
    fallback: Option<BoxedHandler>,
}

struct Route {
    method: String,
    pattern: PathPattern,
    handler: BoxedHandler,
}

impl Router {
    /// Create a new empty Router.
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            scoped_fallbacks: Vec::new(),
            fallback: None,
        }
    }

    /// Register a GET handler for the given path pattern.
    ///
    /// Path patterns support `{param}` and a trailing `{*catch_all}`. Examples:
    /// - `"/"` → root
    /// - `"/hello/{name}"` → one path parameter
    /// - `"/assets/{*path}"` → catch-all remainder
    pub fn get<H, R>(&mut self, pattern: &str, handler: H) -> &mut Self
    where
        H: Fn(&CgiRequest) -> R + Send + Sync + 'static,
        R: IntoResponse + 'static,
    {
        self.add_route("GET", pattern, handler)
    }

    /// Register a POST handler.
    pub fn post<H, R>(&mut self, pattern: &str, handler: H) -> &mut Self
    where
        H: Fn(&CgiRequest) -> R + Send + Sync + 'static,
        R: IntoResponse + 'static,
    {
        self.add_route("POST", pattern, handler)
    }

    /// Register a PUT handler.
    pub fn put<H, R>(&mut self, pattern: &str, handler: H) -> &mut Self
    where
        H: Fn(&CgiRequest) -> R + Send + Sync + 'static,
        R: IntoResponse + 'static,
    {
        self.add_route("PUT", pattern, handler)
    }

    /// Register a DELETE handler.
    pub fn delete<H, R>(&mut self, pattern: &str, handler: H) -> &mut Self
    where
        H: Fn(&CgiRequest) -> R + Send + Sync + 'static,
        R: IntoResponse + 'static,
    {
        self.add_route("DELETE", pattern, handler)
    }

    /// Register a handler for the given HTTP method and path pattern.
    pub fn handle<H, R>(&mut self, method: &str, pattern: &str, handler: H) -> &mut Self
    where
        H: Fn(&CgiRequest) -> R + Send + Sync + 'static,
        R: IntoResponse + 'static,
    {
        self.add_route(method, pattern, handler)
    }

    fn add_route<H, R>(&mut self, method: &str, pattern: &str, handler: H) -> &mut Self
    where
        H: Fn(&CgiRequest) -> R + Send + Sync + 'static,
        R: IntoResponse + 'static,
    {
        let pattern = PathPattern::parse(pattern);
        self.routes.push(Route {
            method: method.to_string(),
            pattern,
            handler: Box::new(move |req| handler(req).into_response()),
        });
        self
    }

    /// Mount `other` under `prefix`, rewriting its route patterns.
    ///
    /// `prefix` is normalized to a path without a trailing slash (`"/api"`, or
    /// `""` / `"/"` for no prefix). Nested fallbacks apply only under that
    /// prefix (and still work if the parent later sets [`Router::fallback`]).
    pub fn nest(&mut self, prefix: &str, mut other: Router) -> &mut Self {
        let prefix = normalize_prefix(prefix);
        for route in other.routes.drain(..) {
            self.routes.push(Route {
                method: route.method,
                pattern: route.pattern.with_prefix(&prefix),
                handler: route.handler,
            });
        }
        for (nested_prefix, fb) in other.scoped_fallbacks.drain(..) {
            let full = join_prefix(&prefix, &nested_prefix);
            self.scoped_fallbacks.push((full, fb));
        }
        if let Some(nested_fb) = other.fallback.take() {
            self.scoped_fallbacks.push((prefix, nested_fb));
        }
        self
    }

    /// Append all routes (and scoped fallbacks) from `other`.
    ///
    /// If this router has no global fallback, it takes `other`'s. If both define
    /// a global fallback, this router's is kept.
    pub fn merge(&mut self, mut other: Router) -> &mut Self {
        self.routes.append(&mut other.routes);
        self.scoped_fallbacks.append(&mut other.scoped_fallbacks);
        if self.fallback.is_none() {
            self.fallback = other.fallback.take();
        }
        self
    }

    /// Set a fallback handler for requests that don't match any registered route.
    pub fn fallback<H, R>(&mut self, handler: H) -> &mut Self
    where
        H: Fn(&CgiRequest) -> R + Send + Sync + 'static,
        R: IntoResponse + 'static,
    {
        self.fallback = Some(Box::new(move |req| handler(req).into_response()));
        self
    }

    /// Parse the CGI request from the environment, match a route, and write the response.
    ///
    /// This is the main entry point for a CGI binary. It:
    /// 1. Reads the CGI request from environment variables + stdin
    /// 2. Strips `SCRIPT_NAME` from the path to get the relative route path
    /// 3. Matches the first route whose method + pattern match
    /// 4. Calls the handler with extracted path parameters
    /// 5. Writes the CGI response to stdout
    pub fn serve(&self) -> Result<()> {
        let req = CgiRequest::from_env()?;
        let resp = self.handle_request(req);
        resp.write_response_to_stdout()
    }

    /// Match an already parsed CGI request and return a response.
    ///
    /// This is useful for callers that need to inspect request metadata before
    /// routing, while still using the library router and response machinery.
    pub fn handle_request(&self, req: CgiRequest) -> CgiResponse {
        let path = relative_path(&req);

        // Try to match a route
        for route in &self.routes {
            if route.method != req.method {
                continue;
            }
            if let Some(params) = route.pattern.match_path(path.as_ref()) {
                let mut req = req;
                req.path_params = params;
                let resp = (route.handler)(&req);
                return resp;
            }
        }

        let path = path.as_ref();

        // Nested/scoped fallbacks: longest matching prefix wins.
        let mut best: Option<&(String, BoxedHandler)> = None;
        for entry in &self.scoped_fallbacks {
            if path_has_prefix(path, &entry.0)
                && best.as_ref().is_none_or(|b| entry.0.len() >= b.0.len())
            {
                best = Some(entry);
            }
        }
        if let Some((_, fb)) = best {
            return fb(&req);
        }

        if let Some(fallback) = &self.fallback {
            return fallback(&req);
        }

        CgiResponse::new(404, "Not found\n")
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

/// Normalize a nest prefix: no trailing slash; empty/`/` → `""`; otherwise starts with `/`.
fn normalize_prefix(prefix: &str) -> String {
    let t = prefix.trim();
    if t.is_empty() || t == "/" {
        return String::new();
    }
    let mut p = if t.starts_with('/') {
        t.to_string()
    } else {
        format!("/{t}")
    };
    while p.ends_with('/') && p.len() > 1 {
        p.pop();
    }
    p
}

fn path_has_prefix(path: &str, prefix: &str) -> bool {
    if prefix.is_empty() {
        return true;
    }
    path == prefix || path.starts_with(&format!("{prefix}/"))
}

fn join_prefix(outer: &str, inner: &str) -> String {
    match (outer.is_empty(), inner.is_empty()) {
        (true, true) => String::new(),
        (true, false) => inner.to_string(),
        (false, true) => outer.to_string(),
        (false, false) => format!("{outer}{inner}"),
    }
}

// ---- Path pattern matching ----

/// A parsed path pattern like `/hello/{name}` or `/static/{*path}`.
#[derive(Debug, Clone)]
struct PathPattern {
    segments: Vec<PathSegment>,
}

/// A single segment in a path pattern.
#[derive(Debug, Clone)]
enum PathSegment {
    /// A literal segment that must match exactly (e.g., `"hello"`).
    Literal(String),
    /// A parameter segment that matches any single segment (e.g., `{name}`).
    Param(String),
    /// Catch-all: matches zero or more remaining segments (must be last).
    CatchAll(String),
}

impl PathPattern {
    /// Parse a path pattern string into segments.
    ///
    /// Examples:
    /// - `"/"` → empty segments
    /// - `"/hello/{name}"` → [Literal("hello"), Param("name")]
    /// - `"/static/{*path}"` → [Literal("static"), CatchAll("path")]
    fn parse(pattern: &str) -> Self {
        let segments: Vec<PathSegment> = pattern
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|seg| {
                if seg.starts_with("{*") && seg.ends_with('}') && seg.len() > 3 {
                    let name = &seg[2..seg.len() - 1];
                    assert!(
                        !name.is_empty(),
                        "catch-all parameter name must not be empty"
                    );
                    PathSegment::CatchAll(name.to_string())
                } else if seg.starts_with('{') && seg.ends_with('}') && seg.len() > 2 {
                    PathSegment::Param(seg[1..seg.len() - 1].to_string())
                } else {
                    PathSegment::Literal(seg.to_string())
                }
            })
            .collect();

        if let Some(pos) = segments
            .iter()
            .position(|s| matches!(s, PathSegment::CatchAll(_)))
        {
            assert!(
                pos == segments.len() - 1,
                "catch-all {{*…}} must be the last path segment"
            );
        }

        PathPattern { segments }
    }

    fn with_prefix(&self, prefix: &str) -> Self {
        if prefix.is_empty() {
            return self.clone();
        }
        let mut segments = PathPattern::parse(prefix).segments;
        assert!(
            !segments
                .iter()
                .any(|s| matches!(s, PathSegment::CatchAll(_))),
            "nest prefix must not contain a catch-all"
        );
        segments.extend(self.segments.iter().cloned());
        PathPattern { segments }
    }

    /// Try to match a path against this pattern.
    ///
    /// Returns `Some(params)` with extracted path parameters if the path matches,
    /// or `None` if it doesn't.
    fn match_path(&self, path: &str) -> Option<HashMap<String, String>> {
        let path_segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        let catch_all = self
            .segments
            .last()
            .and_then(|s| match s {
                PathSegment::CatchAll(name) => Some(name.as_str()),
                _ => None,
            });

        if let Some(catch_name) = catch_all {
            let fixed = &self.segments[..self.segments.len() - 1];
            if path_segs.len() < fixed.len() {
                return None;
            }
            let mut params = HashMap::new();
            for (segment, value) in fixed.iter().zip(path_segs.iter()) {
                match segment {
                    PathSegment::Literal(lit) => {
                        if lit != *value {
                            return None;
                        }
                    }
                    PathSegment::Param(name) => {
                        params.insert(name.clone(), percent_decode_segment(value));
                    }
                    PathSegment::CatchAll(_) => unreachable!("catch-all only last"),
                }
            }
            let rest: Vec<String> = path_segs[fixed.len()..]
                .iter()
                .map(|s| percent_decode_segment(s))
                .collect();
            params.insert(catch_name.to_string(), rest.join("/"));
            return Some(params);
        }

        if path_segs.len() != self.segments.len() {
            return None;
        }

        let mut params = HashMap::new();
        for (segment, value) in self.segments.iter().zip(path_segs.iter()) {
            match segment {
                PathSegment::Literal(lit) => {
                    if lit != *value {
                        return None;
                    }
                }
                PathSegment::Param(name) => {
                    params.insert(name.clone(), percent_decode_segment(value));
                }
                PathSegment::CatchAll(_) => unreachable!("catch-all handled above"),
            }
        }

        Some(params)
    }
}

/// Decode `%XX` sequences in one path segment (CGI `PATH_INFO` is typically still encoded).
fn percent_decode_segment(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (from_hex(bytes[i + 1]), from_hex(bytes[i + 2])) {
                out.push((hi << 4) | lo);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn from_hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_req(path_info: &str) -> CgiRequest {
        CgiRequest {
            method: "GET".into(),
            script_name: "/cgi-bin/app".into(),
            path_info: path_info.into(),
            query_string: "".into(),
            uri: String::new(),
            #[cfg(feature = "hyper-cgi")]
            headers: Default::default(),
            body: vec![],
            path_params: HashMap::new(),
        }
    }

    #[test]
    fn match_root() {
        let p = PathPattern::parse("/");
        assert_eq!(p.match_path("/"), Some(HashMap::new()));
        assert_eq!(p.match_path("/foo"), None);
    }

    #[test]
    fn match_literal() {
        let p = PathPattern::parse("/hello");
        assert_eq!(p.match_path("/hello"), Some(HashMap::new()));
        assert_eq!(p.match_path("/world"), None);
    }

    #[test]
    fn match_param() {
        let p = PathPattern::parse("/hello/{name}");
        let m = p.match_path("/hello/world");
        assert!(m.is_some());
        assert_eq!(m.unwrap().get("name").map(|s| s.as_str()), Some("world"));
    }

    #[test]
    fn match_param_percent_decoded() {
        let p = PathPattern::parse("/hello/{name}");
        let m = p.match_path("/hello/%3Ca%20href%3D%22%2F%22%3EBiff%3C%2Fa%3E");
        assert_eq!(
            m.unwrap().get("name").map(|s| s.as_str()),
            Some(r#"<a href="/">Biff</a>"#)
        );
    }

    #[test]
    fn match_multi_param() {
        let p = PathPattern::parse("/users/{id}/posts/{post_id}");
        let m = p.match_path("/users/42/posts/99");
        assert!(m.is_some());
        let m = m.unwrap();
        assert_eq!(m.get("id").map(|s| s.as_str()), Some("42"));
        assert_eq!(m.get("post_id").map(|s| s.as_str()), Some("99"));
    }

    #[test]
    fn match_no_trailing() {
        let p = PathPattern::parse("/primes/{limit}");
        assert!(p.match_path("/primes").is_none()); // missing segment
        assert!(p.match_path("/primes/400000/extra").is_none()); // too many segments
    }

    #[test]
    fn match_catch_all() {
        let p = PathPattern::parse("/static/{*path}");
        let m = p.match_path("/static/a/b.css").unwrap();
        assert_eq!(m.get("path").map(|s| s.as_str()), Some("a/b.css"));

        let empty = p.match_path("/static").unwrap();
        assert_eq!(empty.get("path").map(|s| s.as_str()), Some(""));

        assert!(p.match_path("/other/a").is_none());
    }

    #[test]
    fn match_catch_all_percent_decoded_segments() {
        let p = PathPattern::parse("/files/{*path}");
        let m = p.match_path("/files/dir/%20x").unwrap();
        assert_eq!(m.get("path").map(|s| s.as_str()), Some("dir/ x"));
    }

    #[test]
    #[should_panic(expected = "catch-all")]
    fn catch_all_must_be_last() {
        let _ = PathPattern::parse("/{*path}/extra");
    }

    #[test]
    fn nest_prefixes_routes() {
        let mut api = Router::new();
        api.get("/users/{id}", |req| {
            format!("user {}\n", req.param("id").unwrap_or("?"))
        });

        let mut root = Router::new();
        root.nest("/api", api);

        let resp = root.handle_request(get_req("/api/users/7"));
        assert_eq!(resp.status, "200");
        assert_eq!(std::str::from_utf8(&resp.body).unwrap(), "user 7\n");

        let miss = root.handle_request(get_req("/users/7"));
        assert_eq!(miss.status, "404");
    }

    #[test]
    fn nest_with_catch_all() {
        let mut files = Router::new();
        files.get("/{*path}", |req| {
            format!("{}\n", req.param("path").unwrap_or(""))
        });

        let mut root = Router::new();
        root.nest("/static", files);

        let resp = root.handle_request(get_req("/static/a/b"));
        assert_eq!(std::str::from_utf8(&resp.body).unwrap(), "a/b\n");
    }

    #[test]
    fn merge_appends_routes() {
        let mut a = Router::new();
        a.get("/a", |_| "A\n");
        let mut b = Router::new();
        b.get("/b", |_| "B\n");
        a.merge(b);

        assert_eq!(
            std::str::from_utf8(&a.handle_request(get_req("/a")).body).unwrap(),
            "A\n"
        );
        assert_eq!(
            std::str::from_utf8(&a.handle_request(get_req("/b")).body).unwrap(),
            "B\n"
        );
    }

    #[test]
    fn nest_fallback_scoped_to_prefix() {
        let mut api = Router::new();
        api.get("/ok", |_| "ok\n");
        api.fallback(|_| (404u16, "api-miss\n"));

        let mut root = Router::new();
        root.nest("/api", api);
        root.fallback(|_| (404u16, "root-miss\n"));

        let api_miss = root.handle_request(get_req("/api/missing"));
        assert_eq!(std::str::from_utf8(&api_miss.body).unwrap(), "api-miss\n");

        let root_miss = root.handle_request(get_req("/other"));
        assert_eq!(std::str::from_utf8(&root_miss.body).unwrap(), "root-miss\n");
    }

    #[test]
    fn relative_path_uses_path_info() {
        let req = CgiRequest {
            method: "GET".into(),
            script_name: "/cgi-bin/prog".into(),
            path_info: "/hello".into(),
            query_string: "".into(),
            uri: String::new(),
            #[cfg(feature = "hyper-cgi")]
            headers: Default::default(),
            body: vec![],
            path_params: HashMap::new(),
        };
        assert_eq!(relative_path(&req), "/hello");
    }

    #[test]
    fn relative_path_strips_script_name() {
        let req = CgiRequest {
            method: "GET".into(),
            script_name: "/cgi-bin/prog".into(),
            path_info: "".into(),
            query_string: "".into(),
            uri: "/cgi-bin/prog/hello".into(),
            #[cfg(feature = "hyper-cgi")]
            headers: Default::default(),
            body: vec![],
            path_params: HashMap::new(),
        };
        assert_eq!(relative_path(&req), "/hello");
    }
}
