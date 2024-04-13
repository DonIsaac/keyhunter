use regex::{Regex, RegexBuilder};

#[derive(Debug)]
pub struct Rule {
    id: String,
    pattern: Pattern,
    /// Used for error messages
    description: String,
    ignore_patterns: Option<Vec<String>>,
    kind: RuleKind,
}

#[derive(Debug, Default)]
pub enum RuleKind {
    Name,
    #[default]
    Value,
}

#[derive(Debug)]
pub enum Pattern {
    Regex(Regex),
    String(String),
}


impl Default for Rule {
    fn default() -> Self {
        Self {
            id: "default".into(),
            pattern: Pattern::default(),
            description: "Detected an API key.".into(),
            ignore_patterns: None,
            kind: RuleKind::default()
        }
    }
}

impl Rule {
    #[must_use]
    pub fn new_name<P: Into<Pattern>>(pattern: P) -> Self {
        Self {
            pattern: pattern.into(),
            kind: RuleKind::Name,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn new_value<P: Into<Pattern>>(pattern: P) -> Self {
        Self {
            pattern: pattern.into(),
            kind: RuleKind::Value,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn with_id<S: Into<String>>(mut self, id: S) -> Self {
        self.id = id.into();
        self
    }

    #[must_use]
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = description.into();
        self
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn pattern(&self) -> &Pattern {
        &self.pattern
    }

    #[must_use]
    pub fn default_name_rules() -> Vec<Self> {
        vec![
            Self {
                id: "keyfinder-openai-api-key-name".into(),
                description: "Detected an OpenAI API key.".into(),
                kind: RuleKind::Name,
                pattern: RegexBuilder::new("openai_?api_?key")
                    .case_insensitive(true)
                    .unicode(false)
                    .build()
                    .unwrap()
                    .into(),
                ..Default::default()
            },
            Self {
                id: "keyfinder-aws-access-key-id-name".into(),
                description: "Detected an AWS Access Key ID".into(),
                kind: RuleKind::Name,
                pattern: RegexBuilder::new("aws[-_]access[-_]key[-_](?:id)?")
                    .case_insensitive(true)
                    .build()
                    .unwrap()
                    .into(),
                ..Default::default()
            },
        ]
    }
}

impl Default for Pattern {
    fn default() -> Self {
        Self::String("OPENAI_API_KEY".into())
    }
}

impl From<Regex> for Pattern {
    fn from(regex: Regex) -> Self {
        Self::Regex(regex)
    }
}

impl From<&str> for Pattern {
    fn from(value: &str) -> Self {
        Self::String(value.into())
    }
}

impl From<String> for Pattern {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}
impl Pattern {
    pub fn matches(&self, value: &str) -> bool {
        match self {
            Self::Regex(regex) => regex.is_match(value),
            Self::String(ref s) => s == value
        }
    }
}
