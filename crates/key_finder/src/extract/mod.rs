mod visit;
mod error;

use std::{rc::Rc, sync::Arc};
use oxc::{allocator::Allocator, ast::Visit, parser::Parser, semantic::SemanticBuilder, span::SourceType};

use crate::Config;
use visit::{ApiKey, ApiKeyVisitor};

#[derive(Debug)]
pub struct ApiKeyExtractor<'c> {
    config: &'c Config

}

impl<'c> ApiKeyExtractor<'c> {
    pub fn new(config: &'c Config) -> Self {
        Self { config }
    }

    pub fn extract_api_keys<'a: 'c >(&self, source_type: SourceType, source_code: &'a str) -> Vec<ApiKey<'c>> {
        let allocator = Allocator::default();
        let ret = Parser::new(&allocator,  source_code, source_type).parse();

        if ret.panicked {
            // TODO: error handling
            panic!("parser panic'd");
        }
        else if !ret.errors.is_empty() {
            panic!("Parser returned {} errors: {:#?}", ret.errors.len(), ret.errors);
        }
        let program = ret.program;

        let ret = SemanticBuilder::new(source_code, source_type).build(&program);
        if !ret.errors.is_empty() {
            panic!("SemanticBuilder returned {} errors: {:#?}", ret.errors.len(), ret.errors);
        }
        let semantic = Rc::new(ret.semantic);

        let mut visitor = ApiKeyVisitor::new(&self.config, semantic.clone());
        visitor.visit_program(&program);
        let keys = visitor.into_inner();

        keys
    }
}

