use log::warn;
use std::{fmt, rc::Rc};

use oxc::ast::visit::walk::walk_template_literal;
use oxc::ast::{ast::*, AstKind, Visit};
use oxc::semantic::Semantic;
use oxc::span::{Atom, Span};

use crate::{Config, Rule};

#[derive(Debug)]
pub struct ApiKey {
    pub span: Span,
    // pub rule_id: &'a str,
    // pub api_key: Atom<'a>
    pub rule_id: String,
    pub api_key: String,
}

pub(super) struct ApiKeyVisitor<'c, 'a> {
    semantic: Rc<Semantic<'a>>,
    config: &'c Config,
    api_keys: Vec<ApiKey>,
    seen_api_key_name_rule_id: Option<&'c str>,
}
impl<'c, 'a> ApiKeyVisitor<'c, 'a> {
    pub fn new(config: &'c Config, semantic: Rc<Semantic<'a>>) -> Self {
        Self {
            config,
            semantic,
            api_keys: vec![],
            seen_api_key_name_rule_id: None,
        }
    }

    pub fn into_inner(self) -> Vec<ApiKey> {
        self.api_keys
    }

    fn record_api_key_usage(&mut self, rule_id: &'c str, api_key: Atom<'a>, span: Span) {
        self.api_keys.push(ApiKey {
            rule_id: rule_id.to_string(),
            api_key: api_key.into_string(),
            span,
        })
    }

    fn is_api_key_binding(&self, binding: &BindingPatternKind<'a>) -> Option<&'c str> {
        match binding {
            BindingPatternKind::BindingIdentifier(ident) => {
                self.is_api_key_var(ident.name.as_str())
            }
            BindingPatternKind::AssignmentPattern(assign) => {
                self.is_api_key_binding(&assign.left.kind)
            }
            _ => None,
        }
    }

    fn is_api_key(&self, maybe_key: &str) -> Option<&'c str> {
        if self.seen_api_key_name_rule_id.is_some() {
            self.seen_api_key_name_rule_id
        } else {
            self.is_api_key_value(maybe_key)
        }
    }

    fn is_api_key_var(&self, varname: &str) -> Option<&'c str> {
        self.config
            .name_rules()
            .iter()
            .find(|rule| rule.pattern().matches(varname))
            .map(Rule::id)
    }

    fn is_api_key_value(&self, literal: &str) -> Option<&'c str> {
        self.config
            .value_rules()
            .iter()
            .find(|rule| rule.pattern().matches(literal))
            .map(Rule::id)
    }
}

impl<'c, 'a> Visit<'a> for ApiKeyVisitor<'c, 'a>
where
    'c: 'a,
{
    fn visit_string_literal(&mut self, lit: &StringLiteral<'a>) {
        if let Some(rule_id) = self.is_api_key(&lit.value) {
            warn!(
                "Rule {} reported string literal '{}' as an API key",
                rule_id, &lit.value
            );
            self.record_api_key_usage(rule_id, lit.value.clone(), lit.span)
        }
    }

    fn visit_template_literal(&mut self, lit: &TemplateLiteral<'a>) {
        if lit.is_no_substitution_template() {
            let str_lit = lit.quasi().unwrap();
            if let Some(rule_id) = self.is_api_key(str_lit) {
                warn!(
                    "Rule {} reported template literal '{}' as an API key",
                    rule_id, &str_lit
                );
                self.record_api_key_usage(rule_id, str_lit.clone(), lit.span)
            }
        }
        walk_template_literal(self, lit)
    }

    fn enter_node(&mut self, kind: AstKind<'a>) {
        match &kind {
            AstKind::VariableDeclarator(decl) => {
                self.seen_api_key_name_rule_id = self.is_api_key_binding(&decl.id.kind);
                #[cfg(debug_assertions)]
                if let Some(rule_id) = self.seen_api_key_name_rule_id {
                    warn!(
                        "Rule {} suggests that variable declaration {:?} could be an API key",
                        rule_id, &decl.id.kind
                    )
                }
            }
            _ => {}
        }
    }

    fn leave_node(&mut self, kind: AstKind<'a>) {
        match kind {
            AstKind::VariableDeclarator(_) => {
                self.seen_api_key_name_rule_id = None;
            }
            _ => {}
        }
    }
}

impl fmt::Debug for ApiKeyVisitor<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ApiKeyVisitor")
            .field("semantic", &"<omitted>")
            .field("config", &self.config)
            .field("api_keys", &self.api_keys)
            .field("seen_api_key_name_rule_id", &self.seen_api_key_name_rule_id)
            .finish()
    }
}
