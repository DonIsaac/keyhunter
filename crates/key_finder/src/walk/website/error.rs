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
