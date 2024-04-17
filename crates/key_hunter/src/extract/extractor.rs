use log::error;
use oxc::{allocator::Allocator, ast::Visit, parser::Parser, span::SourceType};
use std::sync::Arc;

use super::visit::{ApiKey, ApiKeyVisitor};
use crate::Config;

#[derive(Debug)]
pub struct ApiKeyExtractor {
    config: Arc<Config>,
}

impl ApiKeyExtractor {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }

    pub fn extract_api_keys(&self, source_type: SourceType, source_code: &str) -> Vec<ApiKey> {
        let allocator = Allocator::default();
        let ret = Parser::new(&allocator, source_code, source_type).parse();

        if ret.panicked {
            // TODO: error handling
            error!("parser panic'd");
            return vec![];
        } else if !ret.errors.is_empty() {
            error!(
                "Parser returned {} errors: {:#?}",
                ret.errors.len(),
                ret.errors
            );
            return vec![];
        }
        let program = ret.program;

        let mut visitor = ApiKeyVisitor::new(&self.config);
        visitor.visit_program(&program);

        visitor.into_inner()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Config;
    use std::sync::Arc;

    #[test]
    fn test_openai_api_key_name_variable() {
        let config: Arc<Config> = Default::default();
        const SOURCES: [&str; 3] = [
            r#"const OPENAI_API_KEY = "foo";"#,
            r#"const openai_api_key = "foo";"#,
            r#"const openAiApiKey   = "foo";"#,
            // r#"const openai-api-key = "foo";"#,
            // r#"const OPENAI-API-KEY = "foo";"#,
        ];
        for src in SOURCES {
            let keys =
                ApiKeyExtractor::new(config.clone()).extract_api_keys(SourceType::default(), src);
            assert_eq!(keys.len(), 1, "Should have found API key in: {src}");
            assert_eq!(keys[0].api_key, "foo");
        }
    }

    #[test]
    fn test_openai_api_key_name_property() {
        let config: Arc<Config> = Default::default();
        const SOURCES: [&str; 1] = [r#"process.env.OPENAI_API_KEY = "foo";"#];
        for src in SOURCES {
            let keys =
                ApiKeyExtractor::new(config.clone()).extract_api_keys(SourceType::default(), src);
            assert_eq!(keys.len(), 1, "Should have found API key in: {src}");
            assert_eq!(keys[0].api_key, "foo");
        }
    }

    #[test]
    fn test_aws_access_key_id_name() {
        let config: Arc<Config> = Default::default();
        const SOURCES: [&str; 3] = [
            r#"const AWS_ACCESS_KEY_ID = "foo";"#,
            r#"const aws_access_key_id = "foo";"#,
            // r#"const aws-access-key-id = "foo";"#,
            // r#"const awsAccessKeyId = "foo";"#,
            r#"const ACCESS_KEY_ID = "foo";"#,
        ];
        for src in SOURCES {
            let keys =
                ApiKeyExtractor::new(config.clone()).extract_api_keys(SourceType::default(), src);
            assert_eq!(keys.len(), 1, "Should have found API key in: {src}");
            assert_eq!(keys[0].api_key, "foo");
        }
    }

    #[test]
    fn test_aws_access_key_id_value() {
        let config = Arc::new(Config::from_default_gitleaks_config());

        const SOURCES: [&str; 1] = [r#"const x = "AKIAXXXXXXXXXXXXXXXX";"#];
        for src in SOURCES {
            let keys =
                ApiKeyExtractor::new(config.clone()).extract_api_keys(SourceType::default(), src);
            assert_eq!(keys.len(), 1);
        }
    }
}
