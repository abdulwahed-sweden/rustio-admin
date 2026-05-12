//! A small, opinionated router.
//!
//! - Path segments starting with `:` are captured into `req.param(name)`.
//! - Middleware is a chain of `async fn(Request, Next) -> Result<Response>`.
//! - 404 vs 405 is distinguished (path matched but method didn't → 405).

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use hyper::Method;

use crate::error::{Error, Result};
use crate::http::{response_from_error, Request, Response};

// public:
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

// public:
pub type HandlerFn =
    Arc<dyn Fn(Request) -> BoxFuture<'static, Result<Response>> + Send + Sync + 'static>;

// public:
pub type MiddlewareFn =
    Arc<dyn Fn(Request, Next) -> BoxFuture<'static, Result<Response>> + Send + Sync + 'static>;

// public:
pub struct Next {
    chain: Vec<MiddlewareFn>,
    handler: HandlerFn,
    index: usize,
}

impl Next {
    // public:
    pub fn run(mut self, req: Request) -> BoxFuture<'static, Result<Response>> {
        Box::pin(async move {
            if self.index < self.chain.len() {
                let mw = self.chain[self.index].clone();
                self.index += 1;
                mw(req, self).await
            } else {
                (self.handler)(req).await
            }
        })
    }
}

struct Route {
    method: Method,
    segments: Vec<Segment>,
    handler: HandlerFn,
}

enum Segment {
    Static(String),
    Param(String),
}

// public:
pub struct Router {
    routes: Vec<Route>,
    middleware: Vec<MiddlewareFn>,
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

impl Router {
    // public:
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            middleware: Vec::new(),
        }
    }

    // public:
    pub fn middleware<F, Fut>(mut self, mw: F) -> Self
    where
        F: Fn(Request, Next) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Response>> + Send + 'static,
    {
        let wrapped: MiddlewareFn = Arc::new(move |req, next| Box::pin(mw(req, next)));
        self.middleware.push(wrapped);
        self
    }

    // public:
    pub fn get<F, Fut>(self, path: &str, handler: F) -> Self
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Response>> + Send + 'static,
    {
        self.route(Method::GET, path, handler)
    }

    // public:
    pub fn post<F, Fut>(self, path: &str, handler: F) -> Self
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Response>> + Send + 'static,
    {
        self.route(Method::POST, path, handler)
    }

    // public:
    pub fn route<F, Fut>(mut self, method: Method, path: &str, handler: F) -> Self
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Response>> + Send + 'static,
    {
        let segments = parse_path(path);
        let handler: HandlerFn = Arc::new(move |req| Box::pin(handler(req)));
        self.routes.push(Route {
            method,
            segments,
            handler,
        });
        self
    }

    /// Look up a handler for the given method+path.
    fn find(&self, method: &Method, path: &str) -> MatchResult {
        let mut path_segs: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        // Normalise trailing slash: `/admin/posts/` and `/admin/posts`
        // address the same handler. Skip when the only segment is empty
        // (the root path `/`), so root-route lookups still work.
        if path_segs.len() > 1 && path_segs.last() == Some(&"") {
            path_segs.pop();
        }
        let mut path_matched = false;

        for route in &self.routes {
            if !segments_match(&route.segments, &path_segs) {
                continue;
            }
            path_matched = true;
            if route.method == *method {
                let params = extract_params(&route.segments, &path_segs);
                return MatchResult::Ok {
                    handler: route.handler.clone(),
                    params,
                };
            }
        }

        if path_matched {
            MatchResult::MethodNotAllowed
        } else {
            MatchResult::NotFound
        }
    }

    // public:
    pub async fn dispatch(&self, mut req: Request) -> Response {
        let matched = self.find(req.method(), req.path());

        let outcome = match matched {
            MatchResult::Ok { handler, params } => {
                req.set_params(params);
                let next = Next {
                    chain: self.middleware.clone(),
                    handler,
                    index: 0,
                };
                next.run(req).await
            }
            MatchResult::NotFound => Err(Error::NotFound(format!("no route for {}", req.path()))),
            MatchResult::MethodNotAllowed => Err(Error::MethodNotAllowed(format!(
                "{} not allowed",
                req.method()
            ))),
        };

        match outcome {
            Ok(resp) => resp,
            Err(err) => response_from_error(&err),
        }
    }
}

enum MatchResult {
    Ok {
        handler: HandlerFn,
        params: std::collections::HashMap<String, String>,
    },
    NotFound,
    MethodNotAllowed,
}

fn parse_path(path: &str) -> Vec<Segment> {
    path.trim_start_matches('/')
        .split('/')
        .map(|seg| {
            if let Some(name) = seg.strip_prefix(':') {
                Segment::Param(name.to_string())
            } else {
                Segment::Static(seg.to_string())
            }
        })
        .collect()
}

fn segments_match(route: &[Segment], path: &[&str]) -> bool {
    if route.len() != path.len() {
        return false;
    }
    for (r, p) in route.iter().zip(path.iter()) {
        match r {
            Segment::Static(s) if s != p => return false,
            _ => {}
        }
    }
    true
}

fn extract_params(route: &[Segment], path: &[&str]) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    for (r, p) in route.iter().zip(path.iter()) {
        if let Segment::Param(name) = r {
            out.insert(name.clone(), (*p).to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn matches_static_path() {
        let router = Router::new().get("/hello", |_req| async { Ok(Response::text("hi")) });
        let req = Request::new(
            Method::GET,
            "/hello".into(),
            String::new(),
            Default::default(),
            bytes::Bytes::new(),
        );
        let resp = router.dispatch(req).await;
        assert_eq!(resp.status.as_u16(), 200);
    }

    #[tokio::test]
    async fn captures_param() {
        let router = Router::new().get("/users/:id", |req| async move {
            let id = req.param("id").unwrap_or("").to_string();
            Ok(Response::text(id))
        });
        let req = Request::new(
            Method::GET,
            "/users/42".into(),
            String::new(),
            Default::default(),
            bytes::Bytes::new(),
        );
        let resp = router.dispatch(req).await;
        assert_eq!(resp.status.as_u16(), 200);
        assert_eq!(&resp.body[..], b"42");
    }

    #[tokio::test]
    async fn distinguishes_404_from_405() {
        let router = Router::new().get("/things", |_| async { Ok(Response::text("ok")) });

        let post = Request::new(
            Method::POST,
            "/things".into(),
            String::new(),
            Default::default(),
            bytes::Bytes::new(),
        );
        assert_eq!(router.dispatch(post).await.status.as_u16(), 405);

        let missing = Request::new(
            Method::GET,
            "/nope".into(),
            String::new(),
            Default::default(),
            bytes::Bytes::new(),
        );
        assert_eq!(router.dispatch(missing).await.status.as_u16(), 404);
    }

    #[tokio::test]
    async fn trailing_slash_is_normalised_for_static_and_param_routes() {
        let router = Router::new()
            .get("/admin/:name", |req| async move {
                let name = req.param("name").unwrap_or("").to_string();
                Ok(Response::text(name))
            })
            .get("/admin/:name/:id/edit", |req| async move {
                Ok(Response::text(format!(
                    "{}/{}",
                    req.param("name").unwrap_or(""),
                    req.param("id").unwrap_or(""),
                )))
            });

        for path in ["/admin/posts", "/admin/posts/"] {
            let req = Request::new(
                Method::GET,
                path.into(),
                String::new(),
                Default::default(),
                bytes::Bytes::new(),
            );
            let resp = router.dispatch(req).await;
            assert_eq!(resp.status.as_u16(), 200, "GET {path} should be 200");
            assert_eq!(&resp.body[..], b"posts", "GET {path} body");
        }

        for path in ["/admin/posts/1/edit", "/admin/posts/1/edit/"] {
            let req = Request::new(
                Method::GET,
                path.into(),
                String::new(),
                Default::default(),
                bytes::Bytes::new(),
            );
            let resp = router.dispatch(req).await;
            assert_eq!(resp.status.as_u16(), 200, "GET {path} should be 200");
            assert_eq!(&resp.body[..], b"posts/1", "GET {path} body");
        }
    }

    #[tokio::test]
    async fn root_path_still_matches_after_trailing_slash_normalisation() {
        // Regression check: the trailing-slash strip must NOT collapse
        // the single empty segment that represents the root path.
        let router = Router::new().get("/", |_| async { Ok(Response::text("home")) });
        let req = Request::new(
            Method::GET,
            "/".into(),
            String::new(),
            Default::default(),
            bytes::Bytes::new(),
        );
        let resp = router.dispatch(req).await;
        assert_eq!(resp.status.as_u16(), 200);
        assert_eq!(&resp.body[..], b"home");
    }
}
