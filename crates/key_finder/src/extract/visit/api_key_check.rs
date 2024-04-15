use oxc::ast::ast::*;

use super::ApiKeyVisitor;
use super::GetStrValue as _;

pub trait IsApiKeyName<'c> {
    /// Check if this node looks like an API key based on name rules.
    ///
    /// Returns the ID of the matched rule if it is, or [`None`] if it is not.
    fn is_api_key_name(&self, visitor: &ApiKeyVisitor<'c>) -> Option<&'c str>;
}

pub trait IsApiKeyValue<'c> {
    /// Check if this node looks like an API key based on value rules.
    ///
    /// Returns the ID of the matched rule if it is, or [`None`] if it is not.
    fn is_api_key_value(&self, visitor: &ApiKeyVisitor<'c>) -> Option<&'c str>;
}

/// Implement [`IsApiKeyName`] and/or [`IsApiKeyValue`] for simple AST nodes.
macro_rules! impl_check {
    // Implement both traits, accessing `$prop` as a string for the check
    ($StructName:tt.$prop:tt) => {
        impl<'a, 'c> IsApiKeyName<'c> for $StructName<'a> {
            fn is_api_key_name(&self, visitor: &ApiKeyVisitor<'c>) -> Option<&'c str> {
                visitor.is_api_key_var(self.$prop.as_str())
            }
        }
        impl<'a, 'c> IsApiKeyValue<'c> for $StructName<'a> {
            fn is_api_key_value(&self, visitor: &ApiKeyVisitor<'c>) -> Option<&'c str> {
                visitor.is_api_key_value(self.$prop.as_str())
            }
        }
    };
    // Recursive implementation
    (rec $StructName:tt .$prop:tt) => {
        impl<'a, 'c> IsApiKeyName<'c> for $StructName<'a> {
            fn is_api_key_name(&self, visitor: &ApiKeyVisitor<'c>) -> Option<&'c str> {
                self.$prop.is_api_key_name(visitor)
            }
        }
        impl<'a, 'c> IsApiKeyValue<'c> for $StructName<'a> {
            fn is_api_key_value(&self, visitor: &ApiKeyVisitor<'c>) -> Option<&'c str> {
                self.$prop.is_api_key_value(visitor)
            }
        }
    };

    // Implement IsApiKeyName only
    (name $StructName:tt .$prop:tt) => {
        impl<'a, 'c> IsApiKeyName<'c> for $StructName<'a> {
            fn is_api_key_name(&self, visitor: &ApiKeyVisitor<'c>) -> Option<&'c str> {
                visitor.is_api_key_var(self.$prop.as_str())
            }
        }
    };

    // Recrusive IsApiKeyName only
    (rec name $StructName:tt .$prop:tt) => {
        impl<'a, 'c> IsApiKeyName<'c> for $StructName<'a> {
            fn is_api_key_name(&self, visitor: &ApiKeyVisitor<'c>) -> Option<&'c str> {
                self.$prop.is_api_key_name(visitor)
            }
        }
    };
}

impl_check!(name IdentifierReference .name);
impl_check!(name BindingIdentifier .name);
impl_check!(name IdentifierName .name);
impl_check!(name PrivateIdentifier .name);
impl_check!(StringLiteral.value);
impl_check!(rec ParenthesizedExpression .expression);
impl_check!(rec name AssignmentTargetWithDefault .binding);
impl_check!(rec name BindingPattern .kind);
impl_check!(rec name VariableDeclarator .id);

impl<'a, 'c> IsApiKeyName<'c> for Expression<'a> {
    fn is_api_key_name(&self, visitor: &ApiKeyVisitor<'c>) -> Option<&'c str> {
        match &self {
            // literals
            Self::StringLiteral(lit) => lit.is_api_key_name(visitor),
            Self::TemplateLiteral(lit) => lit.is_api_key_name(visitor),

            // identifiers & properties
            Self::Identifier(ident) => ident.is_api_key_name(visitor),
            Self::MemberExpression(member) => member.is_api_key_name(visitor),
            Self::PrivateInExpression(private) => private.left.is_api_key_name(visitor),

            // compound expressions
            Self::ChainExpression(chain) => chain.is_api_key_name(visitor),
            Self::ParenthesizedExpression(parens) => parens.expression.is_api_key_name(visitor),
            Self::SequenceExpression(seq) => seq
                .expressions
                .last()
                .and_then(|last| last.is_api_key_name(visitor)),
            _ => None,
        }
    }
}

// =============================================================================
// ============================= ASSIGNMENT TARGET =============================
// =============================================================================

impl<'a, 'c> IsApiKeyName<'c> for AssignmentTarget<'a> {
    fn is_api_key_name(&self, visitor: &ApiKeyVisitor<'c>) -> Option<&'c str> {
        match self {
            Self::SimpleAssignmentTarget(target) => target.is_api_key_name(visitor),
            Self::AssignmentTargetPattern(pat) => pat.is_api_key_name(visitor),
        }
    }
}

impl<'a, 'c> IsApiKeyName<'c> for AssignmentTargetMaybeDefault<'a> {
    fn is_api_key_name(&self, visitor: &ApiKeyVisitor<'c>) -> Option<&'c str> {
        match self {
            Self::AssignmentTarget(target) => target.is_api_key_name(visitor),
            Self::AssignmentTargetWithDefault(target) => target.is_api_key_name(visitor),
        }
    }
}

impl<'a, 'c> IsApiKeyName<'c> for SimpleAssignmentTarget<'a> {
    fn is_api_key_name(&self, visitor: &ApiKeyVisitor<'c>) -> Option<&'c str> {
        match self {
            Self::AssignmentTargetIdentifier(ident) => ident.is_api_key_name(visitor),
            Self::MemberAssignmentTarget(member) => member.is_api_key_name(visitor),
            _ => None,
        }
    }
}

impl<'a, 'c> IsApiKeyName<'c> for AssignmentTargetPattern<'a> {
    fn is_api_key_name(&self, visitor: &ApiKeyVisitor<'c>) -> Option<&'c str> {
        match self {
            Self::ArrayAssignmentTarget(arr) => {
                let name_from_elements = arr
                    .elements
                    .iter()
                    .filter_map(|el| el.as_ref().and_then(|el| el.is_api_key_name(visitor)))
                    .next();
                name_from_elements.or_else(|| {
                    arr.rest
                        .as_ref()
                        .and_then(|rest| rest.target.is_api_key_name(visitor))
                })
            }
            Self::ObjectAssignmentTarget(obj) => {
                let name_from_properties = obj
                    .properties
                    .iter()
                    .filter_map(|prop| prop.is_api_key_name(visitor))
                    .next();
                name_from_properties.or_else(|| {
                    obj.rest
                        .as_ref()
                        .and_then(|rest| rest.target.is_api_key_name(visitor))
                })
            }
        }
    }
}

impl<'a, 'c> IsApiKeyName<'c> for AssignmentTargetProperty<'a> {
    fn is_api_key_name(&self, visitor: &ApiKeyVisitor<'c>) -> Option<&'c str> {
        match self {
            Self::AssignmentTargetPropertyIdentifier(ident) => {
                ident.binding.is_api_key_name(visitor)
            }
            Self::AssignmentTargetPropertyProperty(property) => {
                property.binding.is_api_key_name(visitor)
            }
        }
    }
}

// =============================================================================

impl<'a, 'c> IsApiKeyName<'c> for MemberExpression<'a> {
    fn is_api_key_name(&self, visitor: &ApiKeyVisitor<'c>) -> Option<&'c str> {
        match &self {
            Self::ComputedMemberExpression(member) => member.expression.is_api_key_name(visitor),
            Self::PrivateFieldExpression(ident) => ident.field.is_api_key_name(visitor),
            Self::StaticMemberExpression(static_member) => {
                static_member.property.is_api_key_name(visitor)
            }
        }
    }
}
impl<'a, 'c> IsApiKeyName<'c> for ChainExpression<'a> {
    fn is_api_key_name(&self, visitor: &ApiKeyVisitor<'c>) -> Option<&'c str> {
        let ChainElement::MemberExpression(expr) = &self.expression else {
            return None;
        };
        expr.is_api_key_name(visitor)
    }
}

impl<'a, 'c> IsApiKeyName<'c> for BindingPatternKind<'a> {
    fn is_api_key_name(&self, visitor: &ApiKeyVisitor<'c>) -> Option<&'c str> {
        match &self {
            BindingPatternKind::BindingIdentifier(ident) => ident.is_api_key_name(visitor),
            BindingPatternKind::AssignmentPattern(assign) => {
                assign.left.kind.is_api_key_name(visitor)
            }
            _ => None,
        }
    }
}

impl<'a, 'c> IsApiKeyName<'c> for TemplateLiteral<'a> {
    fn is_api_key_name(&self, visitor: &ApiKeyVisitor<'c>) -> Option<&'c str> {
        if self.is_no_substitution_template() {
            let str_lit = self.quasi().unwrap();
            visitor.is_api_key_var(str_lit)
        } else {
            None
        }
    }
}

// =============================================================================
// ============================= IS API KEY VALUE ==============================
// =============================================================================

impl<'a, 'c> IsApiKeyValue<'c> for Expression<'a> {
    fn is_api_key_value(&self, visitor: &ApiKeyVisitor<'c>) -> Option<&'c str> {
        match self {
            Self::StringLiteral(lit) => lit.is_api_key_value(visitor),
            Self::TemplateLiteral(lit) => lit.is_api_key_value(visitor),
            Self::ParenthesizedExpression(parens) => parens.is_api_key_value(visitor),
            Self::SequenceExpression(seq) => seq
                .expressions
                .last()
                .and_then(|expr| expr.is_api_key_value(visitor)),
            Self::BinaryExpression(expr) => expr.is_api_key_value(visitor),
            Self::LogicalExpression(expr) => expr.is_api_key_value(visitor),
            _ => None,
        }
    }
}

impl<'a, 'c> IsApiKeyValue<'c> for TemplateLiteral<'a> {
    fn is_api_key_value(&self, visitor: &ApiKeyVisitor<'c>) -> Option<&'c str> {
        if self.is_no_substitution_template() {
            let str_lit = self.quasi().unwrap();
            visitor.is_api_key_value(str_lit)
        } else {
            None
        }
    }
}

impl<'a, 'c> IsApiKeyValue<'c> for BinaryExpression<'a> {
    fn is_api_key_value(&self, visitor: &ApiKeyVisitor<'c>) -> Option<&'c str> {
        let Some(const_str) = self.get_str_value() else {
            return None;
        };
        visitor.is_api_key_value(&const_str)
    }
}

impl<'a, 'c> IsApiKeyValue<'c> for LogicalExpression<'a> {
    fn is_api_key_value(&self, visitor: &ApiKeyVisitor<'c>) -> Option<&'c str> {
        let Some(const_str) = self.get_str_value() else {
            return None;
        };
        visitor.is_api_key_value(&const_str)
    }
}
