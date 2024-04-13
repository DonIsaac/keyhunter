mod gitleaks;
mod rule;

use anyhow::Result;
use regex::Regex;
use std::{env, path::PathBuf};

use gitleaks::{GitLeaksConfig, GitLeaksRule};
pub use rule::{Pattern, Rule, RuleKind};

#[derive(Debug)]
pub struct Config {
    /// TODO: Glob
    ignore_patterns: Vec<String>,
    ignore_files: Vec<String>,
    name_rules: Vec<Rule>,
    value_rules: Vec<Rule>,
    // name_rules: Vec<NameRule>,
    // value_rules: Vec<ValueRule>,
}

impl Default for Config {
    fn default() -> Self {
        const GITIGNORE: &str = ".gitignore";
        let gitignore: PathBuf = env::var("PWD")
            .ok()
            .and_then(|pwd| PathBuf::from(pwd).join(GITIGNORE).canonicalize().ok())
            .unwrap_or_else(|| PathBuf::from(GITIGNORE));

        let ignore_files = if gitignore.is_file() {
            vec![gitignore.to_string_lossy().to_string()]
        } else {
            vec![]
        };

        Self {
            ignore_patterns: Default::default(),
            ignore_files,
            name_rules: Rule::default_name_rules(),
            value_rules: vec![],
        }
    }
}

impl Config {
    pub fn from_gitleaks_file(config_path: &str) -> Result<Config> {
        let src = std::fs::read_to_string(config_path)?;
        Self::from_gitleaks_file(&src)
    }
    pub fn from_gitleaks_config(source_text: &str) -> Result<Config> {
        let gitleaks_config: GitLeaksConfig = toml::from_str(source_text)?;
        Ok(gitleaks_config.into())
    }

    pub fn name_rules(&self) -> &[Rule] {
        &self.name_rules
    }

    pub fn value_rules(&self) -> &[Rule] {
        &self.value_rules
    }
}

impl From<GitLeaksConfig> for Config {
    fn from(value: GitLeaksConfig) -> Self {
        let value_rules: Vec<Rule> = value
            .rules
            .into_iter()
            .filter_map(|r| {
                let reg = Regex::new(r.regex.as_str()).ok()?;
                Some(Rule::new_value(reg)
                    .with_id(r.id)
                    .with_description(r.description))
            })
            // .map(|r| Regex::new(r.regex.as_str()))
            // .filter(Result::is_ok)
            // .map(Result::unwrap)
            // .map(|regex| Rule::new_value(regex))
            .collect::<Vec<_>>();

        Self {
            value_rules,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_default() {
        let cfg = Config::default();
        assert_eq!(cfg.ignore_patterns.len(), 0);
    }

    #[test]
    fn from_gitleaks() -> Result<()> {
        const GITLEAKS: &str = include_str!("../gitleaks.toml");
        let config = Config::from_gitleaks_config(GITLEAKS)?;
        assert!(config.value_rules().len() > 0);
        assert!(config.name_rules().len() > 0);

        Ok(())
    }
}
