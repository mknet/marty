use crate::Result;
use crate::cgi::{CgiRequest, CgiResponse, IntoResponse, relative_path};
use std::collections::HashMap;

type BoxedHandler = Box<dyn Fn(&CgiRequest) -> CgiResponse + Send + Sync>;

/// A synchronous CGI router with method-based routing and path parameters.
///
/// Routes are matched in registration order. Path parameters use `{name}` syntax:
/// - `"/"` matches the root
/// - `"/hello/{name}"` matches `/hello/world` → `req.param("name") == Some("world")`
///
/// The Router automatically strips `SCRIPT_NAME` from the request path so that
/// routes are relative to the CGI script's mount point.
///
/// # Example
///
/// ```rust,ignore
/// use marty::Router;
///
/// let mut router = Router::new();
/// router.get("/hello/{name}", |req| {
///     let name = req.param("name").unwrap_or("world");
///     format!("Hello, {name}!\n")
/// });
/// router.fallback(|_| (404, "Not found\n"));
/// router.serve().unwrap();
/// ```
pub struct Router {
    routes: Vec<Route>,
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
            fallback: None,
        }
    }

    /// Register a GET handler for the given path pattern.
    ///
    /// Path patterns support `{param}` syntax. Examples:
    /// - `"/"` → root
    /// - `"/hello/{name}"` → one path parameter
    /// - `"/primes/{limit}"` → another single parameter
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

        // Try fallback
        if let Some(fallback) = &self.fallback {
            return fallback(&req);
        }

        // Default 404
        CgiResponse::new(404, "Not found\n")
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

// ---- Path pattern matching ----

/// A parsed path pattern like `/hello/{name}`.
#[derive(Debug, Clone)]
struct PathPattern {
    segments: Vec<PathSegment>,
}

/// A single segment in a path pattern.
#[derive(Debug, Clone)]
enum PathSegment {
    /// A literal segment that must match exactly (e.g., `"hello"`).
    Literal(String),
    /// A parameter segment that matches any value (e.g., `{name}`).
    Param(String),
}

impl PathPattern {
    /// Parse a path pattern string into segments.
    ///
    /// Examples:
    /// - `"/"` → empty segments
    /// - `"/hello/{name}"` → [Literal("hello"), Param("name")]
    fn parse(pattern: &str) -> Self {
        let segments: Vec<PathSegment> = pattern
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|seg| {
                if seg.starts_with('{') && seg.ends_with('}') && seg.len() > 2 {
                    PathSegment::Param(seg[1..seg.len() - 1].to_string())
                } else {
                    PathSegment::Literal(seg.to_string())
                }
            })
            .collect();
        PathPattern { segments }
    }

    /// Try to match a path against this pattern.
    ///
    /// Returns `Some(params)` with extracted path parameters if the path matches,
    /// or `None` if it doesn't.
    fn match_path(&self, path: &str) -> Option<HashMap<String, String>> {
        let path_segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

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
                    params.insert(name.clone(), value.to_string());
                }
            }
        }

        Some(params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
