use miette::{self, Diagnostic, Error, NamedSource, SourceSpan};
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
    pub fn new(api_key: ApiKey, url: String, source_text: String, config: &Config) -> Self {
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
            rule_id,
            api_key,
            url,
        }
    }
}
