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
    ) -> Result<Vec<ApiKey>> {
        let program = Self::parse(allocator, source_code)?;

        let mut visitor = ApiKeyVisitor::new(&self.config);
        visitor.visit_program(&program);

        Ok(visitor.into_inner())
    }

    fn parse<'a>(allocator: &'a Allocator, source_code: &'a str) -> Result<Program<'a>> {
        let ret: ParserReturn<'a> =
            Parser::new(allocator, source_code, SourceType::default()).parse();
        if !ret.errors.is_empty() {
            return Err(ParserFailedDiagnostic::new(ret.errors).into());
        } else if ret.panicked {
            // TODO: error handling
            return Err(miette::miette!(
                code = "keyhunter::parse_failed",
                "Parser panicked"
            ));
        }

        Ok(ret.program)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // #[test]
    // fn test_openai_api_key_name_variable() {
    //     let alloc = Allocator::default();
    //     let extractor = ApiKeyExtractor::default();

    //     const SOURCES: [&str; 3] = [
    //         r#"const OPENAI_API_KEY = "foo";"#,
    //         r#"const openai_api_key = "foo";"#,
    //         r#"const openAiApiKey   = "foo";"#,
    //         // r#"const openai-api-key = "foo";"#,
    //         // r#"const OPENAI-API-KEY = "foo";"#,
    //     ];
    //     for src in SOURCES {
    //         let keys = extractor.extract_api_keys(&alloc, src).unwrap();
    //         assert_eq!(keys.len(), 1, "Should have found API key in: {src}");
    //         assert_eq!(keys[0].secret, "foo");
    //     }
    // }

    // #[test]
    // fn test_openai_api_key_name_property() {
    //     let alloc = Allocator::default();
    //     let extractor = ApiKeyExtractor::default();

    //     const SOURCES: [&str; 1] = [r#"process.env.OPENAI_API_KEY = "foo";"#];
    //     for src in SOURCES {
    //         let keys = extractor.extract_api_keys(&alloc, src).unwrap();
    //         assert_eq!(keys.len(), 1, "Should have found API key in: {src}");
    //         assert_eq!(keys[0].secret, "foo");
    //     }
    // }

    // #[test]
    // fn test_aws_access_key_id_name() {
    //     let alloc = Allocator::default();
    //     let extractor = ApiKeyExtractor::default();

    //     const SOURCES: [&str; 3] = [
    //         r#"const AWS_ACCESS_KEY_ID = "foo";"#,
    //         r#"const aws_access_key_id = "foo";"#,
    //         // r#"const aws-access-key-id = "foo";"#,
    //         // r#"const awsAccessKeyId = "foo";"#,
    //         r#"const ACCESS_KEY_ID = "foo";"#,
    //     ];
    //     for src in SOURCES {
    //         let keys = extractor.extract_api_keys(&alloc, src).unwrap();
    //         assert_eq!(keys.len(), 1, "Should have found API key in: {src}");
    //         assert_eq!(keys[0].secret, "foo");
    //     }
    // }

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
