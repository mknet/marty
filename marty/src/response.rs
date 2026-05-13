use crate::{Result, error};
use bytes::Bytes;
use hyper::{HeaderMap, http::HeaderValue};
use snafu::ResultExt;
use std::io::Write;

#[derive(Debug)]
pub struct CGIResponse {
    pub headers: HeaderMap<HeaderValue>,
    pub status: String,
    pub reason: Option<String>,
    pub body: Bytes,
}

impl CGIResponse {
    /// Write this response as a CGI "document" to `output` (typically stdout).
    pub fn write_response_to_output(self, mut output: impl Write) -> Result<()> {
        self.write_status(&mut output)?;
        self.write_headers(&mut output)?;
        self.write_body(&mut output)?;
        Ok(())
    }

    fn write_status(&self, output: &mut impl Write) -> Result<()> {
        if let Some(reason) = &self.reason {
            output
                .write(format!("Status: {} {}\n", self.status, reason).as_bytes())
                .context(error::WriteResponseSnafu)?;
        } else {
            output
                .write(format!("Status: {}\n", self.status).as_bytes())
                .context(error::WriteResponseSnafu)?;
        }
        Ok(())
    }

    fn write_headers(&self, output: &mut impl Write) -> Result<()> {
        for (key, value) in self.headers.iter() {
            let mut header_bytes = format!("{}: ", key).into_bytes();
            header_bytes.extend(value.as_bytes());
            header_bytes.extend(b"\n");
            output
                .write(&header_bytes)
                .context(error::WriteResponseSnafu)?;
        }

        output.write(b"\n").context(error::WriteResponseSnafu)?;

        Ok(())
    }

    fn write_body(self, output: &mut impl Write) -> Result<()> {
        output
            .write(self.body.as_ref())
            .context(error::WriteResponseSnafu)?;
        Ok(())
    }
}
