use std::fmt;

use miette::{self, Diagnostic, Result};
use thiserror::{self, Error};

#[derive(Debug, Error, Diagnostic)]
#[error("Expected {url} to return HTML, but it returned content of type {content_type}")]
pub struct NotHtmlDiagnostic {
    url: String,
    content_type: String,
}
impl NotHtmlDiagnostic {
    pub fn new<S: Into<String>, Z: Into<String>>(url: S, content_type: Z) -> Self {
        Self {
            url: url.into(),
            content_type: content_type.into(),
        }
    }
}
impl<T> From<NotHtmlDiagnostic> for Result<T> {
    fn from(val: NotHtmlDiagnostic) -> Self {
        Err(val.into())
    }
}

#[derive(Debug, Error, Diagnostic)]
#[error("Server responded to requests for {url} with no content")]
pub struct NoContentDiagnostic {
    url: String,
}
impl NoContentDiagnostic {
    pub fn new<S: Into<String>>(url: S) -> Self {
        Self { url: url.into() }
    }
}
impl<T> From<NoContentDiagnostic> for Result<T> {
    fn from(val: NoContentDiagnostic) -> Self {
        Err(val.into())
    }
}

#[derive(Debug, Error, Diagnostic)]
pub struct WalkFailedDiagnostic {
    url: String,
    verbose: bool,
    inner: WalkFailedDiagnosticInner,
}

#[derive(Debug)]
enum WalkFailedDiagnosticInner {
    Status {
        status_code: u16,
        status_text: String,
        body: Option<String>,
        headers: Vec<(String, String)>,
    },
    Transport {
        // inner: ureq::Error
        // #[source]
        source: ureq::Transport,
    },
}
impl WalkFailedDiagnostic {
    pub fn new(url: String, source: ureq::Error) -> Self {
        let inner = match source {
            ureq::Error::Status(status_code, res) => {
                let status_text = res.status_text().to_string();
                let headers = res
                    .headers_names()
                    .into_iter()
                    .map(|header_name| {
                        let values = res.all(&header_name);
                        (header_name, values.join(", "))
                    })
                    .collect::<Vec<_>>();
                let body = res.into_string().ok();
                // res.headers_names()
                WalkFailedDiagnosticInner::Status {
                    status_code,
                    status_text,
                    body,
                    headers,
                }
            }
            ureq::Error::Transport(t) => WalkFailedDiagnosticInner::Transport { source: t },
        };

        Self {
            url,
            // TODO: toggle this based on verbosity CLI flag
            verbose: false,
            inner,
        }
    }
}

impl fmt::Display for WalkFailedDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Failed to walk site at '{}': ", self.url)?;

        // write!(f, "Failed to walk site at '{}': ", self.url())?;
        match &self.inner {
            WalkFailedDiagnosticInner::Status {
                status_code,
                status_text,
                body,
                headers,
                ..
            } => {
                write!(
                    f,
                    "Server responded with status code {} ({})",
                    status_code, status_text
                )?;

                if self.verbose {
                    writeln!(f, "\n\nResponse headers:")?;
                    for (header, value) in headers {
                        writeln!(f, "  {}: {}", header, value)?;
                    }
                    if let Some(body) = &body {
                        write!(f, "\n\nResponse body:\n{}", body)
                    } else {
                        write!(f, "\n\nNo response body")
                    }
                } else {
                    Ok(())
                }
            }
            WalkFailedDiagnosticInner::Transport { source, .. } => {
                write!(f, "{}", source)
            }
        }
    }
}
