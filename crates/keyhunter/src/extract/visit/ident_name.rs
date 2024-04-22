use oxc::{ast::ast::*, span::Atom};

pub trait GetIdentifier<'a> {
    fn get_identifier_name(&self) -> Option<&Atom<'a>>;
}

macro_rules! impl_ident_name {
    ($StructName:tt.$prop:tt) => {
        impl<'a> GetIdentifier<'a> for $StructName<'a> {
            fn get_identifier_name(&self) -> Option<&Atom<'a>> {
                Some(&self.$prop)
            }
        }
    };
}

impl_ident_name!(IdentifierName.name);
impl_ident_name!(IdentifierReference.name);
impl_ident_name!(BindingIdentifier.name);
impl_ident_name!(PrivateIdentifier.name);

// impl_ident_name!(BindingPattern .name);
impl<'a> GetIdentifier<'a> for BindingPattern<'a> {
    fn get_identifier_name(&self) -> Option<&Atom<'a>> {
        self.get_identifier()
    }
}
impl<'a> GetIdentifier<'a> for BindingPatternKind<'a> {
    fn get_identifier_name(&self) -> Option<&Atom<'a>> {
        self.get_identifier()
    }
}

impl<'a> GetIdentifier<'a> for MemberExpression<'a> {
    fn get_identifier_name(&self) -> Option<&Atom<'a>> {
        match self {
            Self::ComputedMemberExpression(expr) => match &expr.expression {
                Expression::StringLiteral(lit) => Some(&lit.value),
                Expression::TemplateLiteral(lit) if lit.is_no_substitution_template() => {
                    lit.quasi()
                }
                _ => None,
            },
            Self::PrivateFieldExpression(field) => field.field.get_identifier_name(),
            Self::StaticMemberExpression(field) => field.property.get_identifier_name(),
        }
    }
}

impl<'a> GetIdentifier<'a> for AssignmentTarget<'a> {
    fn get_identifier_name(&self) -> Option<&Atom<'a>> {
        match self {
            Self::SimpleAssignmentTarget(pat) => pat.get_identifier_name(),
            Self::AssignmentTargetPattern(pat) => None,
        }
    }
}
impl<'a> GetIdentifier<'a> for SimpleAssignmentTarget<'a> {
    fn get_identifier_name(&self) -> Option<&Atom<'a>> {
        match self {
            Self::AssignmentTargetIdentifier(ident) => ident.get_identifier_name(),
            Self::MemberAssignmentTarget(member) => member.get_identifier_name(),
            Self::TSAsExpression(expr) => expr.expression.get_identifier_name(),
            Self::TSNonNullExpression(expr) => expr.expression.get_identifier_name(),
            Self::TSSatisfiesExpression(expr) => expr.expression.get_identifier_name(),
            Self::TSTypeAssertion(expr) => expr.expression.get_identifier_name(),
        }
    }
}
impl<'a> GetIdentifier<'a> for PropertyKey<'a> {
    fn get_identifier_name(&self) -> Option<&Atom<'a>> {
        match self {
            Self::Expression(expr) => expr.get_identifier_name(),
            Self::Identifier(ident) => ident.get_identifier_name(),
            Self::PrivateIdentifier(ident) => ident.get_identifier_name(),
        }
    }
}

impl<'a> GetIdentifier<'a> for ObjectProperty<'a> {
    fn get_identifier_name(&self) -> Option<&Atom<'a>> {
        self.key.get_identifier_name()
    }
}

impl<'a> GetIdentifier<'a> for Expression<'a> {
    fn get_identifier_name(&self) -> Option<&Atom<'a>> {
        match self.get_inner_expression() {
            Self::Identifier(ident) => ident.get_identifier_name(),
            Self::AssignmentExpression(expr) => expr.left.get_identifier_name(),
            Self::MemberExpression(expr) => expr.get_identifier_name(),
            _ => None,
        }
    }
}
