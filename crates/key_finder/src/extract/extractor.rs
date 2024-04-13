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

    pub fn extract_api_keys<'a>(
        &self,
        source_type: SourceType,
        source_code: &'a str,
    ) -> Vec<ApiKey> {
        let allocator = Allocator::default();
        let ret = Parser::new(&allocator, source_code, source_type).parse();

        if ret.panicked {
            // TODO: error handling
            println!("parser panic'd");
        } else if !ret.errors.is_empty() {
            panic!(
                "Parser returned {} errors: {:#?}",
                ret.errors.len(),
                ret.errors
            );
        }
        let program = ret.program;

        let ret = SemanticBuilder::new(source_code, source_type).build(&program);
        if !ret.errors.is_empty() {
            panic!(
                "SemanticBuilder returned {} errors: {:#?}",
                ret.errors.len(),
                ret.errors
            );
        }
        let semantic = Rc::new(ret.semantic);

        let mut visitor = ApiKeyVisitor::new(&self.config, semantic.clone());
        visitor.visit_program(&program);
        let keys = visitor.into_inner();

        keys
    }
}
