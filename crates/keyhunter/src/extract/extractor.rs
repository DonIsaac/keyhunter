use log::error;
use miette::Result;
use oxc::{
    allocator::Allocator,
    ast::{ast::Program, Visit},
    parser::{Parser, ParserReturn},
    span::SourceType,
};
use std::sync::Arc;

use super::{
    error::ParserFailedDiagnostic,
    visit::{ApiKey, ApiKeyVisitor},
};
use crate::Config;

#[derive(Debug, Default)]
pub struct ApiKeyExtractor {
    config: Arc<Config>,
}

impl ApiKeyExtractor {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }

    pub fn extract_api_keys<'s, 'a: 's>(
        &'s self,
        allocator: &'a Allocator,
        source_code: &'a str,
    ) -> Result<Vec<ApiKey<'s>>> {
        let program = Self::parse(&allocator, &source_code)?;

        let mut visitor = ApiKeyVisitor::new(&self.config);
        visitor.visit_program(&program);

        Ok(visitor.into_inner())
    }

    fn parse<'a>(allocator: &'a Allocator, source_code: &'a str) -> Result<Program<'a>> {
        let ret: ParserReturn<'a> =
            Parser::new(allocator, source_code, SourceType::default()).parse();
        if ret.panicked {
            // TODO: error handling
            error!("parser panic'd");
            return Err(miette::miette!(
                code = "keyhunter::parse_failed",
                "Parser panicked while"
            ));
        } else if !ret.errors.is_empty() {
            return Err(ParserFailedDiagnostic::new(ret.errors).into());
        }

        Ok(ret.program)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Config;
    use std::sync::Arc;

    #[test]
    fn test_openai_api_key_name_variable() {
        let alloc = Allocator::default();
        let extractor = ApiKeyExtractor::default();

        const SOURCES: [&str; 3] = [
            r#"const OPENAI_API_KEY = "foo";"#,
            r#"const openai_api_key = "foo";"#,
            r#"const openAiApiKey   = "foo";"#,
            // r#"const openai-api-key = "foo";"#,
            // r#"const OPENAI-API-KEY = "foo";"#,
        ];
        for src in SOURCES {
            let keys = extractor.extract_api_keys(&alloc, src).unwrap();
            assert_eq!(keys.len(), 1, "Should have found API key in: {src}");
            assert_eq!(keys[0].api_key, "foo");
        }
    }

    #[test]
    fn test_openai_api_key_name_property() {
        let alloc = Allocator::default();
        let extractor = ApiKeyExtractor::default();

        const SOURCES: [&str; 1] = [r#"process.env.OPENAI_API_KEY = "foo";"#];
        for src in SOURCES {
            let keys = extractor.extract_api_keys(&alloc, src).unwrap();
            assert_eq!(keys.len(), 1, "Should have found API key in: {src}");
            assert_eq!(keys[0].api_key, "foo");
        }
    }

    #[test]
    fn test_aws_access_key_id_name() {
        let alloc = Allocator::default();
        let extractor = ApiKeyExtractor::default();

        const SOURCES: [&str; 3] = [
            r#"const AWS_ACCESS_KEY_ID = "foo";"#,
            r#"const aws_access_key_id = "foo";"#,
            // r#"const aws-access-key-id = "foo";"#,
            // r#"const awsAccessKeyId = "foo";"#,
            r#"const ACCESS_KEY_ID = "foo";"#,
        ];
        for src in SOURCES {
            let keys = extractor.extract_api_keys(&alloc, src).unwrap();
            assert_eq!(keys.len(), 1, "Should have found API key in: {src}");
            assert_eq!(keys[0].api_key, "foo");
        }
    }

    #[test]
    fn test_aws_access_key_id_value() {
        let alloc = Allocator::default();
        let extractor = ApiKeyExtractor::default();

        const SOURCES: [&str; 1] = [r#"const x = "AKIAXXXXXXXXXXXXXXXX";"#];
        for src in SOURCES {
            let keys = extractor.extract_api_keys(&alloc, src).unwrap();
            assert_eq!(keys.len(), 1);
        }
    }
}
