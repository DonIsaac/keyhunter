use core::fmt;
use std::sync::Arc;

/// Copyright © 2024 Don Isaac
///
/// This file is part of KeyHunter.
///
/// KeyHunter is free software: you can redistribute it and/or modify it
/// under the terms of the GNU General Public License as published by the Free
/// Software Foundation, either version 3 of the License, or (at your option)
/// any later version.
///
/// KeyHunter is distributed in the hope that it will be useful, but WITHOUT
/// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
/// FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for
/// more details.
///
/// You should have received a copy of the GNU General Public License along with
/// KeyHunter. If not, see <https://www.gnu.org/licenses/>.
use miette::{self, Diagnostic, Error, NamedSource, SourceCode, SourceSpan};
use serde::ser::{Serialize, SerializeStruct};
use thiserror::Error;

use crate::{config::RuleId, Config};

use super::ApiKey;

#[derive(Debug, Clone, Error, Diagnostic)]
#[error("{rule_id}: {description}")]
#[diagnostic(code(keyhunter::api_key_found))]
pub struct ApiKeyError {
    // pub span: Span,
    #[label]
    pub source_span: SourceSpan,
    #[source_code]
    // pub source_code: NamedSource<String>,
    pub source_code: Arc<NamedSource<String>>,
    pub description: String,
    internal_rule_id: RuleId,
    pub rule_id: String,
    pub secret: String,
    pub key_name: Option<String>,
    pub url: Arc<String>,
}

impl ApiKeyError {
    pub fn new(
        api_key: ApiKey<'_>,
        url: Arc<String>,
        source: &Arc<NamedSource<String>>,
        config: &Config,
    ) -> Self {
        let ApiKey {
            span,
            secret: api_key,
            rule_id,
            key_name,
        } = api_key;

        let source_span: SourceSpan = (span.start as usize, span.size() as usize).into();

        let description = config.get_description(rule_id).to_owned();
        let display_rule_id = config.get_display_id(rule_id).to_owned();

        Self {
            source_span,
            source_code: Arc::clone(source),
            description,
            internal_rule_id: rule_id,
            rule_id: display_rule_id,
            secret: api_key.to_owned(),
            key_name: key_name.map(str::to_string),
            url,
        }
    }

    /// Read the bytes for a specific span from this SourceCode, keeping a
    /// certain number of lines before and after the span as context.
    pub fn read_span<'a>(
        &'a self,
        context_lines_before: usize,
        context_lines_after: usize,
    ) -> Result<Box<dyn miette::SpanContents<'a> + 'a>, miette::MietteError> {
        self.source_code
            .read_span(&self.source_span, context_lines_before, context_lines_after)
    }
}

impl Serialize for ApiKeyError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let context = self.read_span(0, 0).unwrap();
        let line = context.line() + 1;
        let column = context.column() + 1;

        let mut key = serializer.serialize_struct("ApiKeyError", 6)?;
        key.serialize_field("rule_id", &self.rule_id)?;
        key.serialize_field("key_name", &self.key_name)?;
        key.serialize_field("secret", &self.secret)?;
        key.serialize_field("line", &line)?;
        key.serialize_field("column", &column)?;
        key.serialize_field("script_url", self.url.as_ref())?;

        key.end()
    }
}

#[derive(Debug, Error, Diagnostic)]
// #[error("Parser failed with {num_errors} errors")]
#[diagnostic(code(keyhunter::parse_failed))]
pub struct ParserFailedDiagnostic {
    pub num_errors: usize,
    pub errors: Vec<Error>,
}

impl Default for ParserFailedDiagnostic {
    fn default() -> Self {
        Self::empty()
    }
}

impl ParserFailedDiagnostic {
    pub fn new(errors: Vec<Error>) -> Self {
        Self {
            num_errors: errors.len(),
            errors,
        }
    }
    pub fn empty() -> Self {
        Self {
            num_errors: 0,
            errors: vec![],
        }
    }
}
impl<D: Diagnostic + 'static + Send + Sync> FromIterator<D> for ParserFailedDiagnostic {
    fn from_iter<T: IntoIterator<Item = D>>(iter: T) -> Self {
        Self::new(iter.into_iter().map(Error::from).collect())
    }
}

// impl FromIterator<OxcDiagnostic> for ParserFailedDiagnostic {
//     fn from_iter<T: IntoIterator<Item = OxcDiagnostic>>(iter: T) -> Self {
//         Self::new(iter.into_iter().map(Error::from).collect())
//     }
// }

impl fmt::Display for ParserFailedDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.num_errors == 0 {
            writeln!(f, "Parser panicked for unknown reasons")
        } else {
            writeln!(f, "Parser panicked with {} errors:", self.num_errors)?;
            for error in &self.errors {
                writeln!(f, "{}", error)?;
            }
            Ok(())
        }
    }
}

#[derive(Debug, Error, Diagnostic)]
pub enum DownloadScriptDiagnostic {
    /// Failed to download a script from a server
    #[error(transparent)]
    #[diagnostic(code(keyhunter::extract::download::req_failed))]
    Request(Box<ureq::Error>),

    #[error("Failed to read body of response from {0}: {1}")]
    #[diagnostic(code(keyhunter::extract::download::read_failed))]
    CannotReadBody(/* url */ String, #[source] std::io::Error),

    /// Downloaded the resource at a URL but it was not a JavaScript file
    #[error("Resource at {0} is not a JavaScript file, but is instead {1}")]
    #[diagnostic(code(keyhunter::extract::download::not_js))]
    NotJavascript(/* url */ String, /* content type */ String),
}

impl From<ureq::Error> for DownloadScriptDiagnostic {
    fn from(e: ureq::Error) -> Self {
        Self::Request(Box::new(e))
    }
}
