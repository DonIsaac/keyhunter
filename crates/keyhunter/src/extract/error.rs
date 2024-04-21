use miette::{self, Diagnostic, Error, NamedSource, SourceCode, SourceSpan};
use thiserror::Error;

use crate::{ApiKey, Config};

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
    pub rule_id: String,
    pub api_key: String,
    pub url: String,
}

impl ApiKeyError {
    pub fn new<'c>(
        api_key: ApiKey<'c>,
        url: String,
        source_text: String,
        config: &'c Config,
    ) -> Self {
        let ApiKey {
            span,
            api_key,
            rule_id,
        } = api_key;

        let source_span: SourceSpan = (span.start as usize, span.size() as usize).into();
        let source_code = NamedSource::new(&url, source_text).with_language("javascript");

        let violated_rule = config.get_rule(&rule_id).ok_or_else(|| Error::msg(
            format!( "Found violation for rule with id '{rule_id}' but no rule with that ID could be found in the config. This is a bug." )
        )).unwrap();
        let description = violated_rule.description().clone();

        Self {
            source_span,
            source_code,
            description,
            rule_id: rule_id.to_string(),
            api_key,
            url,
        }
    }

    /// Read the bytes for a specific span from this SourceCode, keeping a certain number of lines before and after the span as context.
    pub fn read_span<'a>(
        &'a self,
        context_lines_before: usize,
        context_lines_after: usize,
    ) -> Result<Box<dyn miette::SpanContents<'a> + 'a>, miette::MietteError> {
        self.source_code
            .read_span(&self.source_span, context_lines_before, context_lines_after)
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
