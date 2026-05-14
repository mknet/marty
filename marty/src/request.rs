use crate::{MetaVariable, MetaVariableKind, Result, error};
use http_body_util::Full;
use hyper::Request;
use hyper::body::{Body, Bytes};
use snafu::ResultExt;
use std::io::{Read, stdin};

pub struct CGIRequest<B> {
    pub request_body: B,
}

impl<B> CGIRequest<B>
where
    B: Body,
{
    pub fn from_env() -> Result<CGIRequest<Full<Bytes>>> {
        let content_length = MetaVariableKind::ContentLength
            .from_env()
            .map(|content_length| {
                content_length
                    .as_str()
                    .and_then(|s| s.parse().context(error::InvalidContentLengthSnafu))
            })
            .transpose()?
            .unwrap_or_default();

        let read_content = Self::request_body_from_env(content_length)?;

        let request_body = Bytes::from(read_content);

        let full = Full::from(request_body);

        let result = CGIRequest { request_body: full };

        Ok(result)
    }

    pub fn var(&self, kind: MetaVariableKind) -> Option<MetaVariable> {
        kind.from_env()
    }

    fn try_var(&self, kind: MetaVariableKind) -> Result<MetaVariable> {
        kind.try_from_env()
    }

    fn request_body_from_env(content_length: usize) -> Result<Vec<u8>> {
        let mut request_body = vec![0u8; content_length];
        stdin()
            .read_exact(&mut request_body)
            .context(error::ReadRequestBodySnafu)
            .and(Ok(request_body))
    }

    pub fn uri(&self) -> Result<String> {
        let script = MetaVariableKind::ScriptName.try_from_env()?;
        let script_str = script.as_str()?;
        let path_info = self
            .var(MetaVariableKind::PathInfo)
            .map(|v| v.as_str().map(str::to_string).unwrap_or_default())
            .unwrap_or_default();
        let query = self
            .var(MetaVariableKind::QueryString)
            .map(|v| v.as_str().map(str::to_string).unwrap_or_default())
            .unwrap_or_default();

        let mut logical =
            String::with_capacity(script_str.len() + path_info.len() + query.len() + 1);
        logical.push_str(script_str);
        logical.push_str(&path_info);
        if !query.is_empty() {
            logical.push('?');
            logical.push_str(&query);
        }

        // Apache mod_rewrite often keeps REQUEST_URI as the public path (/routing/…) while
        // SCRIPT_NAME + PATH_INFO are the RFC 3875 application path under the CGI script. Axum
        // routes are almost always registered under the latter; prefer it when REQUEST_URI does
        // not already start with SCRIPT_NAME.
        match self.var(MetaVariableKind::RequestUri) {
            Some(uri) => {
                let req = uri.as_str()?;
                if req == logical.as_str() {
                    return Ok(logical);
                }
                if !req.starts_with(script_str) {
                    return Ok(logical);
                }
                Ok(req.to_string())
            }
            None => Ok(logical),
        }
    }
}

macro_rules! try_set_headers {
    ($request_builder:expr, $cgi_request:expr, $([$header:expr, $value:expr]),* $(,)?) => {
        $(
            if let Some(value) = $cgi_request.var($value) {
                $request_builder = $request_builder.header($header, value.as_bytes());
            }
        )*
    };
}

impl<B> TryFrom<CGIRequest<B>> for Request<B>
where
    B: Body,
{
    type Error = crate::CGIError;

    fn try_from(cgi_request: CGIRequest<B>) -> Result<Self> {
        let mut request_builder = Request::builder()
            .method(
                cgi_request
                    .try_var(MetaVariableKind::RequestMethod)?
                    .as_bytes(),
            )
            .uri(cgi_request.uri()?);

        try_set_headers!(
            request_builder,
            cgi_request,
            ["Content-Length", MetaVariableKind::ContentLength],
            ["Authorization", MetaVariableKind::HttpAuthorization],
            ["Accept", MetaVariableKind::HttpAccept],
            ["Host", MetaVariableKind::HttpHost],
            ["User-Agent", MetaVariableKind::HttpUserAgent],
            ["Cookie", MetaVariableKind::HttpCookie],
        );

        request_builder
            .body(cgi_request.request_body)
            .context(error::RequestParseSnafu)
    }
}
