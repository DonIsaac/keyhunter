mod api_key_check;
mod string;

use log::debug;
use std::fmt;

use oxc::ast::visit::walk::{walk_expression, walk_template_literal};
use oxc::ast::{ast::*, Visit};
use oxc::span::{Atom, Span};

use crate::{Config, Rule};

use api_key_check::IsApiKeyName;
use string::GetStrValue;

#[derive(Debug)]
pub struct ApiKey<'c> {
    pub span: Span,
    pub rule_id: &'c str,
    pub api_key: String,
}

pub(super) struct ApiKeyVisitor<'c> {
    config: &'c Config,
    api_keys: Vec<ApiKey<'c>>,
    seen_api_key_name_rule_id: Option<&'c str>,
}

impl<'c> ApiKeyVisitor<'c> {
    pub fn new(config: &'c Config) -> Self {
        Self {
            config,
            api_keys: vec![],
            seen_api_key_name_rule_id: None,
        }
    }

    pub fn into_inner(self) -> Vec<ApiKey<'c>> {
        self.api_keys
    }

    fn record_api_key_usage<'a>(&mut self, rule_id: &'c str, api_key: Atom<'a>, span: Span) {
        debug!("Recording api key {api_key} found with rule {rule_id}");
        self.api_keys.push(ApiKey {
            rule_id,
            api_key: api_key.into_string(),
            span,
        })
    }

    fn find_and_report_api_keys<'a>(&mut self, maybe_key: &Atom<'a>, span: Span) {
        // fn find_and_report_api_keys<'a>(&mut self, maybe_key: &'a str, span: Span) {
        let found_keys = self.config.value_rules().iter().filter_map(|rule| {
            rule.matches(&maybe_key)
                .map(|found_keys| (rule.id(), found_keys))
        });

        for (rule_id, found_keys) in found_keys {
            for (key_start, found_key) in found_keys {
                let span = Span::new(span.start + (key_start as u32), found_key.len() as u32);
                self.api_keys.push(ApiKey {
                    rule_id,
                    span,
                    api_key: found_key.to_string(),
                })
            }
        }

        if let Some(seen_rule_id) = self.seen_api_key_name_rule_id {
            self.api_keys.push(ApiKey {
                rule_id: seen_rule_id,
                span,
                api_key: maybe_key.to_string(),
            })
        }
    }

    fn is_api_key<'a>(&self, maybe_key: &'a str) -> Option<&'c str> {
        if self.seen_api_key_name_rule_id.is_some() {
            self.seen_api_key_name_rule_id
        } else {
            self.is_api_key_value(maybe_key)
        }
    }

    fn is_api_key_var<'a>(&self, varname: &'a str) -> Option<&'c str> {
        self.config
            .name_rules()
            .iter()
            .find(|rule| rule.pattern().matches(varname))
            .map(Rule::id)
    }

    fn is_api_key_value<'a>(&self, literal: &'a str) -> Option<&'c str> {
        self.config
            .value_rules()
            .iter()
            .find(|rule| rule.pattern().matches(literal))
            .map(Rule::id)
    }
}

// NOTE: not running name rules right now, need to figure out when and where
// they're useful
impl<'c, 'a> Visit<'a> for ApiKeyVisitor<'a>
where
    'c: 'a,
{
    fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
        let Some(init) = &declarator.init else { return };
        let prev_rule_id = self.seen_api_key_name_rule_id;

        if let Some(rule_id) = declarator.is_api_key_name(self) {
            self.seen_api_key_name_rule_id = Some(rule_id)
        }

        walk_expression(self, init);
        self.seen_api_key_name_rule_id = prev_rule_id;
    }

    fn visit_assignment_expression(&mut self, expr: &AssignmentExpression<'a>) {
        let prev_rule_id = self.seen_api_key_name_rule_id;
        self.seen_api_key_name_rule_id = expr.left.is_api_key_name(self).or(prev_rule_id);
        walk_expression(self, &expr.right);
        self.seen_api_key_name_rule_id = prev_rule_id;
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
            let str_lit = lit.quasi().expect("TemplateLiteral.is_no_substitution_template should have checked that at least one quasis exists.");
            self.find_and_report_api_keys(&str_lit, lit.span)
            // if let Some(rule_id) = self.is_api_key(str_lit) {
            //     warn!(
            //         "Rule {} reported template literal '{}' as an API key",
            //         rule_id, &str_lit
            //     );
            //     self.record_api_key_usage(rule_id, str_lit.clone(), lit.span)
            // }
        } else {
            walk_template_literal(self, lit)
        }
    }
}

impl fmt::Debug for ApiKeyVisitor<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ApiKeyVisitor")
            .field("semantic", &"<omitted>")
            .field("config", &self.config)
            .field("api_keys", &self.api_keys)
            .field("seen_api_key_name_rule_id", &self.seen_api_key_name_rule_id)
            .finish()
    }
}
