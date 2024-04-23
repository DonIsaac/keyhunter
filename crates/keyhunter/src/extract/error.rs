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
use std::borrow::Cow;

use miette::{self, Diagnostic, Error, NamedSource, SourceCode, SourceSpan};
use serde::ser::{Serialize, SerializeStruct, Serializer};
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
    pub source_code: NamedSource<String>,
    pub description: String,
    internal_rule_id: RuleId,
    pub rule_id: String,
    pub secret: String,
    pub key_name: Option<String>,
    pub url: String,
}

impl ApiKeyError {
    pub fn new<'c, 'a>(
        api_key: ApiKey<'a>,
        url: String,
        source_text: String,
        config: &'c Config,
    ) -> Self {
        let ApiKey {
            span,
            secret: api_key,
            rule_id,
            key_name,
        } = api_key;

        let source_span: SourceSpan = (span.start as usize, span.size() as usize).into();
        let source_code = NamedSource::new(&url, source_text).with_language("javascript");

        let description = config.get_description(rule_id).to_owned();
        let display_rule_id = config.get_display_id(rule_id).to_owned();

        Self {
            source_span,
            source_code,
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
        key.serialize_field("script_url", &self.url)?;

        key.end()
    }
}

#[derive(Debug, Error, Diagnostic)]
#[error("Parser failed with {num_errors} errors")]
#[diagnostic(code(keyhunter::parse_failed))]
pub struct ParserFailedDiagnostic {
    pub num_errors: usize,
    pub errors: Vec<Error>,
}
impl ParserFailedDiagnostic {
    pub fn new(errors: Vec<Error>) -> Self {
        Self {
            num_errors: errors.len(),
            errors,
        }
    }
}
