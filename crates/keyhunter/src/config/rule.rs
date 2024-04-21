use regex::{Regex, RegexBuilder};
use tinyvec::TinyVec;

use super::gitleaks::GitLeaksRule;

#[derive(Debug)]
pub struct Rule {
    pub(super) id: String,
    pub(super) pattern: Pattern,
    /// Used for error messages
    pub(super) description: String,
    pub(super) ignore_patterns: Option<Vec<String>>,
    pub(super) kind: RuleKind,
    pub(super) entropy: Option<f32>,
    pub(super) keywords: Option<TinyVec<[String; 2]>>,
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
            kind: RuleKind::default(),
            entropy: None,
            keywords: None,
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

    pub fn description(&self) -> &String {
        &self.description
    }

    pub fn pattern(&self) -> &Pattern {
        &self.pattern
    }

    pub fn keywords(&self) -> Option<&[String]> {
        self.keywords.as_ref().map(|keywords| keywords.as_slice())
    }

    pub const fn is_name_rule(&self) -> bool {
        matches!(self.kind, RuleKind::Name)
    }

    pub const fn is_value_rule(&self) -> bool {
        matches!(self.kind, RuleKind::Value)
    }

    #[must_use]
    pub(crate) fn default_name_rules() -> Vec<Self> {
        vec![
            Self {
                id: "keyfinder-api-key".into(),
                description: "Detected a generic api key".into(),
                kind: RuleKind::Name,
                pattern: RegexBuilder::new("api[-_]?key")
                    .case_insensitive(true)
                    .build()
                    .unwrap()
                    .into(),
                ..Default::default()
            },
            Self {
                id: "keyfinder-api-token".into(),
                description: "Detected a generic api token".into(),
                kind: RuleKind::Name,
                pattern: RegexBuilder::new("api[-_]?token")
                    .case_insensitive(true)
                    .build()
                    .unwrap()
                    .into(),
                ..Default::default()
            },
            Self {
                id: "keyfinder-openai-api-key-name".into(),
                description: "Detected an OpenAI API key.".into(),
                kind: RuleKind::Name,
                pattern: RegexBuilder::new("openai[_-]?api[_-]?key")
                    .case_insensitive(true)
                    // .unicode(false)
                    .build()
                    .unwrap()
                    .into(),
                ..Default::default()
            },
            Self {
                id: "keyfinder-aws-access-key-id-name".into(),
                description: "Detected an AWS Access Key ID".into(),
                kind: RuleKind::Name,
                pattern: RegexBuilder::new("(?:aws[\\-_]?)?access[\\-_]?key[\\-_]?(?:id)?")
                    .case_insensitive(true)
                    .build()
                    .unwrap()
                    .into(),
                ..Default::default()
            },
            Self {
                id: "keyfinder-aws-secret-access-key-name".into(),
                description: "Detected an AWS Secret Access Key".into(),
                kind: RuleKind::Name,
                pattern: RegexBuilder::new("(?:aws[-_])?secret[-_]?access[-_]?key[-_]?(?:id)?")
                    .case_insensitive(true)
                    .build()
                    .unwrap()
                    .into(),
                ..Default::default()
            },
        ]
    }
}

impl TryFrom<GitLeaksRule> for Rule {
    type Error = regex::Error;

    fn try_from(rule: GitLeaksRule) -> Result<Self, Self::Error> {
        let reg = Regex::new(rule.regex.as_str())?;
        Ok(Self {
            id: rule.id,
            description: rule.description,
            pattern: reg.into(),
            keywords: rule.keywords,
            entropy: rule.entropy,
            kind: RuleKind::Value,
            ignore_patterns: None, // TODO
        })
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
            Self::String(ref s) => s == value,
        }
    }
}
