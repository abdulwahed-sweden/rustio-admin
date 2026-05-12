//! The HTTP primitives. `Request` and `Response` are thin wrappers around
//! hyper's types that carry a typed context and a few conveniences.

use std::any::{Any, TypeId};
use std::collections::HashMap;

use bytes::Bytes;
use hyper::{Method, StatusCode};

use crate::error::{Error, Result};

// public:
/// A per-request typed store. Middleware attaches things here
/// (the authenticated user, the DB handle, etc.) and handlers read them.
#[derive(Default)]
pub struct Context {
    inner: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Context {
    // public:
    pub fn insert<T: Any + Send + Sync>(&mut self, value: T) {
        self.inner.insert(TypeId::of::<T>(), Box::new(value));
    }

    // public:
    pub fn get<T: Any + Send + Sync>(&self) -> Option<&T> {
        self.inner
            .get(&TypeId::of::<T>())
            .and_then(|b| b.downcast_ref::<T>())
    }

    // public:
    pub fn get_mut<T: Any + Send + Sync>(&mut self) -> Option<&mut T> {
        self.inner
            .get_mut(&TypeId::of::<T>())
            .and_then(|b| b.downcast_mut::<T>())
    }
}

// public:
pub struct Request {
    method: Method,
    path: String,
    query: String,
    headers: HashMap<String, String>,
    params: HashMap<String, String>,
    body: Bytes,
    ctx: Context,
}

impl Request {
    pub(crate) fn new(
        method: Method,
        path: String,
        query: String,
        headers: HashMap<String, String>,
        body: Bytes,
    ) -> Self {
        Self {
            method,
            path,
            query,
            headers,
            params: HashMap::new(),
            body,
            ctx: Context::default(),
        }
    }

    // public:
    pub fn method(&self) -> &Method {
        &self.method
    }

    // public:
    pub fn path(&self) -> &str {
        &self.path
    }

    // public:
    pub fn query_string(&self) -> &str {
        &self.query
    }

    // public:
    pub fn query(&self) -> FormData {
        FormData::from_urlencoded(&self.query)
    }

    // public:
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .get(&name.to_ascii_lowercase())
            .map(|s| s.as_str())
    }

    // public:
    pub fn param(&self, name: &str) -> Option<&str> {
        self.params.get(name).map(|s| s.as_str())
    }

    // public:
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    // public:
    pub fn body_text(&self) -> Result<&str> {
        std::str::from_utf8(&self.body)
            .map_err(|_| Error::BadRequest("body is not valid utf-8".into()))
    }

    // public:
    pub fn form(&self) -> Result<FormData> {
        let text = self.body_text()?;
        Ok(FormData::from_urlencoded(text))
    }

    // public:
    pub fn ctx(&self) -> &Context {
        &self.ctx
    }

    // public:
    pub fn ctx_mut(&mut self) -> &mut Context {
        &mut self.ctx
    }

    pub(crate) fn set_params(&mut self, params: HashMap<String, String>) {
        self.params = params;
    }

    // internal: test-only constructor, doc-hidden + cfg-gated
    /// Test-only minimal-Request constructor. Doc-hidden, gated by
    /// the `integration-test` feature so it does not appear on the
    /// public API surface of a regular build. Used by
    /// `crate::__integration::fake_request()` in the testcontainers
    /// integration suite — see `DESIGN_R2_ORGANISATIONAL.md` §10.3.
    #[doc(hidden)]
    #[cfg(feature = "integration-test")]
    pub fn __integration_test_fake(path: String, headers: HashMap<String, String>) -> Self {
        Self::new(
            hyper::Method::POST,
            path,
            String::new(),
            headers,
            bytes::Bytes::new(),
        )
    }
}

// public:
/// Parsed form body (application/x-www-form-urlencoded) or query string.
/// Values are owned so handlers can move them into DB calls freely.
#[derive(Debug, Default, Clone)]
pub struct FormData {
    fields: HashMap<String, String>,
}

impl FormData {
    // public:
    pub fn from_urlencoded(input: &str) -> Self {
        let mut fields = HashMap::new();
        for pair in input.split('&') {
            if pair.is_empty() {
                continue;
            }
            let (raw_key, raw_val) = match pair.split_once('=') {
                Some((k, v)) => (k, v),
                None => (pair, ""),
            };
            let key = decode(raw_key);
            let val = decode(raw_val);
            fields.insert(key, val);
        }
        Self { fields }
    }

    // public:
    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields.get(key).map(|s| s.as_str())
    }

    // public:
    pub fn required(&self, key: &str) -> Result<&str> {
        self.get(key)
            .ok_or_else(|| Error::BadRequest(format!("field {key} is required")))
    }

    // public:
    pub fn bool_flag(&self, key: &str) -> bool {
        // HTML checkboxes: present means true, absent means false.
        matches!(self.get(key), Some("on" | "true" | "1" | "yes"))
    }

    // public:
    pub fn contains(&self, key: &str) -> bool {
        self.fields.contains_key(key)
    }

    // public:
    pub fn as_map(&self) -> &HashMap<String, String> {
        &self.fields
    }
}

fn decode(s: &str) -> String {
    let spaced = s.replace('+', " ");
    urlencoding::decode(&spaced)
        .map(|c| c.into_owned())
        .unwrap_or(spaced)
}

// public:
/// An outbound HTTP response.
pub struct Response {
    pub status: StatusCode,
    pub headers: Vec<(String, String)>,
    pub body: Bytes,
}

impl Response {
    // public:
    pub fn new(status: StatusCode, body: impl Into<Bytes>) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: body.into(),
        }
    }

    // public:
    pub fn ok(body: impl Into<Bytes>) -> Self {
        Self::new(StatusCode::OK, body)
    }

    // public:
    pub fn html(body: impl Into<String>) -> Self {
        let text = body.into();
        Self {
            status: StatusCode::OK,
            headers: vec![("content-type".into(), "text/html; charset=utf-8".into())],
            body: Bytes::from(text),
        }
    }

    // public:
    pub fn json_raw(body: impl Into<String>) -> Self {
        let text = body.into();
        Self {
            status: StatusCode::OK,
            headers: vec![("content-type".into(), "application/json".into())],
            body: Bytes::from(text),
        }
    }

    // public:
    pub fn redirect(to: impl Into<String>) -> Self {
        let url = to.into();
        Self {
            status: StatusCode::SEE_OTHER,
            headers: vec![("location".into(), url)],
            body: Bytes::new(),
        }
    }

    // public:
    pub fn text(body: impl Into<String>) -> Self {
        let text = body.into();
        Self {
            status: StatusCode::OK,
            headers: vec![("content-type".into(), "text/plain; charset=utf-8".into())],
            body: Bytes::from(text),
        }
    }

    // public:
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    // public:
    pub fn with_status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }
}

pub(crate) fn response_from_error(err: &Error) -> Response {
    let status = StatusCode::from_u16(err.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let body = err.client_message().to_string();
    Response {
        status,
        headers: vec![("content-type".into(), "text/plain; charset=utf-8".into())],
        body: Bytes::from(body),
    }
}
