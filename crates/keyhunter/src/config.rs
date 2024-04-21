mod gitleaks;
mod rule;
mod rule_match;

use anyhow::Result;

use gitleaks::GitLeaksConfig;
pub use rule::{Pattern, Rule, RuleKind};

#[derive(Debug)]
pub struct Config {
    /// TODO: Glob
    // ignore_patterns: Vec<String>,
    // ignore_files: Vec<String>,
    name_rules: Vec<Rule>,
    value_rules: Vec<Rule>,
}

impl Default for Config {
    fn default() -> Self {
        // const GITIGNORE: &str = ".gitignore";
        // let gitignore: PathBuf = env::var("PWD")
        //     .ok()
        //     .and_then(|pwd| PathBuf::from(pwd).join(GITIGNORE).canonicalize().ok())
        //     .unwrap_or_else(|| PathBuf::from(GITIGNORE));

        // let ignore_files = if gitignore.is_file() {
        //     vec![gitignore.to_string_lossy().to_string()]
        // } else {
        //     vec![]
        // };

        Self {
            // ignore_patterns: Default::default(),
            // ignore_files,
            name_rules: Rule::default_name_rules(),
            value_rules: vec![],
        }
        // Self::from_default_gitleaks_config()
    }
}

impl Config {
    pub fn empty() -> Self {
        Self {
            name_rules: vec![],
            value_rules: vec![],
        }
    }

    #[must_use]
    pub fn from_default_gitleaks_config() -> Self {
        GitLeaksConfig::default_config().into()
    }

    pub fn from_gitleaks_file(config_path: &str) -> Result<Config> {
        let src = std::fs::read_to_string(config_path)?;
        Self::from_gitleaks_file(&src)
    }

    pub fn from_gitleaks_config(source_text: &str) -> Result<Config> {
        let gitleaks_config: GitLeaksConfig = toml::from_str(source_text)?;
        Ok(gitleaks_config.into())
    }

    pub fn with_default_name_rules(mut self) -> Self {
        self.name_rules = Rule::default_name_rules();
        self
    }

    pub fn name_rules(&self) -> &[Rule] {
        &self.name_rules
    }

    pub fn value_rules(&self) -> &[Rule] {
        &self.value_rules
    }

    pub fn get_rule(&self, id: &str) -> Option<&Rule> {
        // todo: create rule index over their ids
        self.iter_rules().find(|r| r.id() == id)
    }

    fn iter_rules(&self) -> impl Iterator<Item = &Rule> + '_ {
        self.name_rules.iter().chain(self.value_rules.iter())
    }
}

impl From<GitLeaksConfig> for Config {
    fn from(value: GitLeaksConfig) -> Self {
        let value_rules: Vec<Rule> = value
            .rules
            .into_iter()
            .filter_map(|r| Rule::try_from(r).ok())
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
        let config = Config::default();
        assert!(!config.name_rules().is_empty());
    }

    #[test]
    fn from_gitleaks() -> Result<()> {
        let config = Config::from_gitleaks_config(GitLeaksConfig::DEFAULT_CONFIG)?;
        assert!(!config.value_rules().is_empty());
        assert!(!config.name_rules().is_empty());

        Ok(())
    }
}