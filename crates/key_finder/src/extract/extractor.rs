use log::error;
use oxc::{
    allocator::Allocator, ast::Visit, parser::Parser, semantic::SemanticBuilder, span::SourceType,
};
use std::{rc::Rc, sync::Arc};

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

        let ret = SemanticBuilder::new(source_code, source_type).build(&program);
        if !ret.errors.is_empty() {
            error!(
                "SemanticBuilder returned {} errors: {:#?}",
                ret.errors.len(),
                ret.errors
            );
            return vec![];
        }
        let semantic = Rc::new(ret.semantic);

        let mut visitor = ApiKeyVisitor::new(&self.config, semantic.clone());
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
    fn test_api_key_name() {
        let config: Arc<Config> = Default::default();
        const SOURCES: [&str; 3] = [
            r#"const OPENAI_API_KEY = "foo";"#,
            r#"const openai_api_key = "foo";"#,
            r#"const openAiApiKey   = "foo";"#,
            // r#"const openai-api-key = "foo";"#,
            // r#"const OPENAI-API-KEY = "foo";"#,
            // r#"const AWS_ACCESS_KEY_ID = "foo";"#,
            // r#"const ACCESS_KEY_ID = "foo";"#,
        ];
        for src in SOURCES {
            let keys =
                ApiKeyExtractor::new(config.clone()).extract_api_keys(SourceType::default(), src);
            assert_eq!(keys.len(), 1, "Should have found API key in: {src}");
            assert_eq!(keys[0].api_key, "foo");
        }
    }
}
