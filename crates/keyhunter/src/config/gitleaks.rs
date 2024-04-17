use serde::Deserialize;
use tinyvec::TinyVec;

#[derive(Debug, Deserialize)]
pub struct GitLeaksConfig {
    pub title: Option<String>,
    pub allowlist: Option<GitLeaksAllowList>,
    pub rules: Vec<GitLeaksRule>,
}

#[derive(Debug, Deserialize)]
pub struct GitLeaksAllowList {
    pub description: Option<String>,
    pub paths: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct GitLeaksRule {
    pub id: String,
    pub description: String,
    pub regex: String,
    pub keywords: Option<TinyVec<[String; 2]>>,
    pub entropy: Option<f32>,
}

impl GitLeaksConfig {
    pub const DEFAULT_CONFIG: &'static str = include_str!("../../gitleaks.toml");

    pub fn default_config() -> Self {
        toml::from_str(Self::DEFAULT_CONFIG).unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let text = r#"
            title = "test"

            [[rules]]
            id = "foo"
            description = "Found a foo api key"
            regex = "(some)[p]attern$"
            keywords = [
                "foo",
            ]

            [[rules]]
            id = "bar"
            description = "Detected a bar api key"
            regex = "(?:another)api-[kK]ey$"
            keywords = [
                "bar",
            ]
        "#;
        // let config: Config = toml::from_str(text).unwrap();
        let config: GitLeaksConfig = toml::from_str(text).unwrap();

        assert_eq!(config.title.unwrap().as_str(), "test");
        assert_eq!(config.rules.len(), 2);
    }

    #[test]
    fn test_parse() {
        let config: GitLeaksConfig = toml::from_str(GitLeaksConfig::DEFAULT_CONFIG).unwrap();
        assert_eq!(config.title.unwrap().as_str(), "gitleaks config");
        assert!(config.rules.len() > 10);
    }
}
