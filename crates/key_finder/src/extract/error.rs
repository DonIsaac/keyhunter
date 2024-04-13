use oxc::span::Span;

#[derive(Debug, Clone)]
pub struct ApiKeyError<'a> {
    span: Span,
    rule_id: &'a str,
    api_key: &'a str,
}
