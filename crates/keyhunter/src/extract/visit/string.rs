use std::borrow::Cow;

use oxc::{
    ast::ast::*,
    syntax::operator::{BinaryOperator, LogicalOperator},
};

pub trait GetStrValue {
    fn get_str_value(&self) -> Option<Cow<'_, str>>;
}

impl GetStrValue for Expression<'_> {
    fn get_str_value(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::StringLiteral(s) => s.get_str_value(),
            Self::TemplateLiteral(t) => t.get_str_value(),
            Self::ParenthesizedExpression(p) => p.get_str_value(),
            Self::SequenceExpression(seq) => seq.get_str_value(),
            Self::BinaryExpression(expr) => expr.get_str_value(),
            Self::LogicalExpression(expr) => expr.get_str_value(),
            _ => None,
        }
    }
}
impl GetStrValue for StringLiteral<'_> {
    fn get_str_value(&self) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed(&self.value))
    }
}

impl GetStrValue for TemplateLiteral<'_> {
    fn get_str_value(&self) -> Option<Cow<'_, str>> {
        if !self.is_no_substitution_template() {
            return None;
        }
        self.quasi().map(|q| Cow::Borrowed(q.as_str()))
    }
}

impl GetStrValue for ParenthesizedExpression<'_> {
    fn get_str_value(&self) -> Option<Cow<'_, str>> {
        self.expression.get_str_value()
    }
}

impl GetStrValue for SequenceExpression<'_> {
    fn get_str_value(&self) -> Option<Cow<'_, str>> {
        self.expressions
            .last()
            .and_then(|expr| expr.get_str_value())
    }
}

impl GetStrValue for BinaryExpression<'_> {
    fn get_str_value(&self) -> Option<Cow<'_, str>> {
        if self.operator == BinaryOperator::Addition {
            let left = self.left.get_str_value();
            let right = self.right.get_str_value();

            if let (Some(lhs), Some(rhs)) = (left, right) {
                Some(lhs + rhs)
            } else {
                None
            }
        } else {
            None
        }
    }
}
impl GetStrValue for LogicalExpression<'_> {
    fn get_str_value(&self) -> Option<Cow<'_, str>> {
        match self.operator {
            LogicalOperator::And => self.left.get_str_value(),
            _ => self
                .left
                .get_str_value()
                .or_else(|| self.right.get_str_value()),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use oxc::{
        allocator::Allocator,
        ast::ast::{Program, Statement},
        parser::Parser,
        span::SourceType,
    };

    fn parse<'a>(allocator: &'a Allocator, source: &'a str) -> Program<'a> {
        let ret = Parser::new(allocator, source, SourceType::default()).parse();

        if ret.panicked {
            panic!("Parser paniced while parsing {source}");
        }
        if !ret.errors.is_empty() {
            panic!("Parser finished with errors: {:#?}", ret.errors)
        }
        assert!(
            !ret.program.is_empty(),
            "Parsed source to an empty program: {source}"
        );

        ret.program
    }

    #[test]
    fn test_string_lit() {
        let allocator = Allocator::default();
        let test_cases = vec![
            r#"let x = 'Hello, World!'"#,
            r#"let x = `Hello, World!`"#,
            r#"let x = ('Hello, World!')"#,
        ];

        for test in test_cases {
            let program = parse(&allocator, test);

            let Statement::Declaration(Declaration::VariableDeclaration(decls)) = &program.body[0]
            else {
                panic!("Program body should not be empty: {test}")
            };
            let decl = &decls.declarations[0];
            let expr = decl.init.as_ref().unwrap();

            assert_eq!(expr.get_str_value(), Some("Hello, World!".into()));
        }
    }

    #[test]
    fn test_const_str_expressions() {
        let allocator = Allocator::default();
        let test_cases = vec![
            r#"
                'Hello, ' + 'World!';
            "#,
            r#"
                'Hello' + ', ' + 'World!';
            "#,
            r#"
                (console.log("foo"), "Hello, World!");
            "#,
            r#"
                "Hello, " + ('World') + '!'
            "#,
            r#"
                ("Hello, " + ('World')) + (x, y, ('!'))
            "#,
            r#"
                x || "Hello, World!"
            "#,
            r#"
                "Hello, World!" || x
            "#,
            r#"
                x ?? "Hello, World!"
            "#,
            r#"
                "Hello, World!" ?? x
            "#,
            r#"
                "Hello, World!" && "foo"
            "#,
            r#"
                "Hello" + ", " + ("World!" || false)
            "#,
        ];

        for test in test_cases {
            let program = parse(&allocator, test);
            let Statement::ExpressionStatement(stmt) = &program.body[0] else {
                panic!("Expected program to contain an expression statement: {test}");
            };

            assert_eq!(
                stmt.expression.get_str_value(),
                Some("Hello, World!".into())
            );
        }
    }
}
