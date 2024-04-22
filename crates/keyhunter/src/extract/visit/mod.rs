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
// mod api_key_check;
mod ident_name;
mod string;

use log::debug;
use std::fmt;

use oxc::ast::visit::walk;
use oxc::ast::{ast::*, Visit};
use oxc::span::{Atom, Span};

use crate::config::RuleId;
use crate::Config;

// use api_key_check::IsApiKeyName;
use ident_name::GetIdentifier as _;
use string::GetStrValue;

#[derive(Debug)]
pub struct ApiKey<'a> {
    pub span: Span,
    pub rule_id: RuleId,
    pub secret: &'a str,
    pub key_name: Option<&'a str>,
}

fn atom_as_source_str<'a>(atom: &Atom<'a>) -> &'a str {
    // SAFETY: Atom<'a>s store strs with a lifetime of &'a, but are downcasted
    // to &'self via the as_str() method.
    unsafe { std::mem::transmute(atom.as_str()) }
}

pub(super) struct ApiKeyVisitor<'c, 'a> {
    config: &'c Config,
    api_keys: Vec<ApiKey<'a>>,
    // seen_api_key_name_rule_id: Option<&'c str>,
    current_identifier: Option<Atom<'a>>,
}

impl<'c, 'a> ApiKeyVisitor<'c, 'a> {
    pub fn new(config: &'c Config) -> Self {
        Self {
            config,
            api_keys: vec![],
            current_identifier: None,
        }
    }

    pub fn into_inner(self) -> Vec<ApiKey<'a>> {
        self.api_keys
    }

    // fn record_api_key_usage(&mut self, rule_id: &'c str, api_key: Atom<'a>, span: Span) {
    //     debug!("Recording api key {api_key} found with rule {rule_id}");
    //     self.api_keys.push(ApiKey {
    //         rule_id,
    //         api_key: api_key.into_string(),
    //         span,
    //     })
    // }

    fn find_and_report_api_keys(&mut self, maybe_secret: &Atom<'a>, span: Span) {
        let haystack = atom_as_source_str(maybe_secret);
        let possible_found_secrets = self.config.check_values(haystack);

        if let Some(identifier) = self.current_identifier.clone() {
            let violations = possible_found_secrets
                .filter(|(rule_id, _, _)| self.config.check_name(*rule_id, &identifier));
            self.record_with_span(span, Some(atom_as_source_str(&identifier)), violations);
        } else {
            let violations = possible_found_secrets
                .filter(|(rule_id, _, _)| self.config.get_name_criteria(*rule_id).is_none());
            self.record_with_span(span, None, violations);
        };
    }

    fn record_with_span(
        &mut self,
        span: Span,
        identifier: Option<&'a str>,
        violations: impl Iterator<Item = (RuleId, usize, &'a str)>,
    ) {
        violations.for_each(|(rule_id, key_start, found_key)| {
            let start = span.start + key_start as u32;
            let len = found_key.len() as u32;
            let span = Span::new(start, start + len);
            self.api_keys.push(ApiKey {
                rule_id,
                span,
                key_name: identifier,
                secret: found_key,
            });
        });
    }

    // fn is_api_key(&self, maybe_key: &'a str) -> Option<&'c str> {
    //     if self.seen_api_key_name_rule_id.is_some() {
    //         self.seen_api_key_name_rule_id
    //     } else {
    //         self.is_api_key_value(maybe_key)
    //     }
    // }

    // fn is_api_key_var<'a>(&self, varname: &'a str) -> Option<&'c str> {
    //     self.config
    //         .name_rules()
    //         .iter()
    //         .find(|rule| rule.pattern().matches(varname))
    //         .map(RuleOld::id)
    // }

    // fn is_api_key_value<'a>(&self, literal: &'a str) -> Option<&'c str> {
    //     self.config
    //         .value_rules()
    //         .iter()
    //         .find(|rule| rule.pattern().matches(literal))
    //         .map(RuleOld::id)
    // }
}

// NOTE: not running name rules right now, need to figure out when and where
// they're useful
impl<'c, 'a> Visit<'a> for ApiKeyVisitor<'c, 'a>
where
    'c: 'a,
{
    fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
        let Some(init) = &declarator.init else { return };
        let temp = self.current_identifier.take();

        self.current_identifier = declarator.id.get_identifier_name().cloned();
        walk::walk_expression(self, init);
        self.current_identifier = temp;
    }

    fn visit_assignment_expression(&mut self, expr: &AssignmentExpression<'a>) {
        let temp = self.current_identifier.take();
        walk::walk_expression(self, &expr.right);
        self.current_identifier = temp;
    }

    fn visit_property_definition(&mut self, def: &PropertyDefinition<'a>) {
        let Some(value) = def.value.as_ref() else {
            return;
        };
        let temp = self.current_identifier.take();

        self.current_identifier = def.key.get_identifier_name().cloned();
        walk::walk_expression(self, value);
        self.current_identifier = temp;
    }

    fn visit_call_expression(&mut self, expr: &CallExpression<'a>) {
        let temp = self.current_identifier.take();
        walk::walk_call_expression(self, expr);
        self.current_identifier = temp;
    }

    fn visit_string_literal(&mut self, lit: &StringLiteral<'a>) {
        self.find_and_report_api_keys(&lit.value, lit.span);
        // if let Some(rule_id) = self.is_api_key(&lit.value) {
        //     warn!(
        //         "Rule {} reported string literal '{}' as an API key",
        //         rule_id, &lit.value
        //     );
        //     self.record_api_key_usage(rule_id, lit.value.clone(), lit.span)
        // }
    }

    fn visit_template_literal(&mut self, lit: &TemplateLiteral<'a>) {
        if lit.is_no_substitution_template() {
            let str_lit = lit.quasi().expect("TemplateLiteral.is_no_substitution_template() should have checked that at least one quasis exists.");
            self.find_and_report_api_keys(str_lit, lit.span)
            // if let Some(rule_id) = self.is_api_key(str_lit) {
            //     warn!(
            //         "Rule {} reported template literal '{}' as an API key",
            //         rule_id, &str_lit
            //     );
            //     self.record_api_key_usage(rule_id, str_lit.clone(), lit.span)
            // }
        } else {
            walk::walk_template_literal(self, lit)
        }
    }
}

impl fmt::Debug for ApiKeyVisitor<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ApiKeyVisitor")
            .field("semantic", &"<omitted>")
            .field("config", &self.config)
            .field("api_keys", &self.api_keys)
            .field("current_identifier", &self.current_identifier)
            .finish()
    }
}
