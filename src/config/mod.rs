/// Copyright © 2024 Don Isaac
///
/// This file is part of KeyHunter.
///
/// KeyHunter is free software: you can redistribute it and/or modify it
/// under the terms of the GNU General Public License as published by the Free
/// Software Foundation, either version 3 of the License, or (at your option)
/// any later version.
///
/// KeyHunter is distributed in the hope that it will be useful, but WITHOUT
/// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
/// FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for
/// more details.
///
/// You should have received a copy of the GNU General Public License along with
/// KeyHunter. If not, see <https://www.gnu.org/licenses/>.
mod entropy;
mod gitleaks;
mod pattern;
mod rule_match;

use std::hash::Hash;

use index_vec::{define_index_type, IndexVec};
use log::warn;
use miette::{IntoDiagnostic as _, Result};
use regex::RegexBuilder;
use tinyvec::TinyVec;

use gitleaks::GitLeaksConfig;
pub use pattern::Pattern;

define_index_type! {
    pub struct RuleId = u32;

    DISABLE_MAX_INDEX_CHECK = cfg!(not(debug_assertions));
    DISPLAY_FORMAT = "{}";
}

// TODO: documentation

/// Configures how API keys are found
///
/// ## Rule Structure
/// Rules are stored in Struct-of-Arrays (SoA) format for better caching during
/// iteration.
///
/// Note that a rule's minimum entropy requirement (which is stored in the
/// rule's metadata) will only be run against values ("secrets"). This means
/// that entropy requirements for name-only rules will not be applied at all.
#[derive(Debug)]
pub struct Config {
    /// Maps internal IDs to display ids, which are taken from configs and
    /// reported to users when violations are found
    // ids: DashMap<RuleId, String, BuildNoHashHasher<RuleId>>,
    rule_ids: IndexVec<RuleId, String>,
    /// Criteria that identifiers must match before the identifier's value is
    /// checked. Rules without value crtieria will match instantly. Rules that
    /// do not have name criteria will always be run against values.
    rule_name_criteria: IndexVec<RuleId, Option<Pattern>>,
    rule_value_criteria: IndexVec<RuleId, Pattern>,
    rule_keywords: IndexVec<RuleId, TinyVec<[String; 1]>>,
    rule_entropy: IndexVec<RuleId, Option<f32>>,
    rule_descriptions: IndexVec<RuleId, String>,
}

impl Default for Config {
    fn default() -> Self {
        Self::gitleaks()
    }
}

impl Config {
    pub fn empty() -> Self {
        Self {
            rule_ids: Default::default(),
            rule_name_criteria: Default::default(),
            rule_value_criteria: Default::default(),
            rule_keywords: Default::default(),
            rule_entropy: Default::default(),
            rule_descriptions: Default::default(),
        }
    }

    pub fn with_capacity(initial_capacity: usize) -> Self {
        Self {
            rule_ids: IndexVec::with_capacity(initial_capacity),
            rule_name_criteria: IndexVec::with_capacity(initial_capacity),
            rule_value_criteria: IndexVec::with_capacity(initial_capacity),
            rule_keywords: IndexVec::with_capacity(initial_capacity),
            rule_entropy: IndexVec::with_capacity(initial_capacity),
            rule_descriptions: IndexVec::with_capacity(initial_capacity),
        }
    }

    /// Create a new [`Config`] from the default gitleaks config file.
    /// 
    /// See: [`gitleaks.toml`](https://github.com/gitleaks/gitleaks/blob/master/config/gitleaks.toml)
    #[must_use]
    pub fn gitleaks() -> Self {
        GitLeaksConfig::default_config().into()
    }

    pub fn from_gitleaks_file(config_path: &str) -> Result<Self> {
        let src = std::fs::read_to_string(config_path).into_diagnostic()?;
        Self::from_gitleaks_file(&src)
    }

    pub fn from_gitleaks_config(source_text: &str) -> Result<Self> {
        let gitleaks_config: GitLeaksConfig = toml::from_str(source_text).into_diagnostic()?;
        Ok(gitleaks_config.into())
    }

    pub fn get_name_criteria(&self, rule_id: RuleId) -> Option<&Pattern> {
        self.rule_name_criteria
            .get(rule_id)
            .and_then(Option::as_ref)
    }

    pub fn get_value_criteria(&self, rule_id: RuleId) -> &Pattern {
        &self.rule_value_criteria[rule_id]
    }

    /// Get a rule's description
    pub fn get_description(&self, rule_id: RuleId) -> &str {
        &self.rule_descriptions[rule_id]
    }

    pub fn get_display_id(&self, rule_id: RuleId) -> &str {
        &self.rule_ids[rule_id]
    }

    pub fn iter_value_criteria(&self) -> impl Iterator<Item = (RuleId, &Pattern)> {
        self.rule_value_criteria.iter_enumerated()
    }
    pub fn iter_name_criteria(&self) -> impl Iterator<Item = (RuleId, Option<&Pattern>)> {
        self.rule_name_criteria
            .iter_enumerated()
            .map(|(rule_id, pat)| (rule_id, pat.as_ref()))
    }

    /// Returns the number of rules in the config.
    #[inline]
    pub fn len(&self) -> usize {
        self.rule_ids.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.rule_ids.is_empty()
    }

    fn add_rule(
        &mut self,
        id: String,
        name: Option<Pattern>,
        value: Pattern,
        keywords: TinyVec<[String; 1]>,
        entropy: Option<f32>,
        description: String,
    ) -> RuleId {
        let rule_id = self.rule_ids.push(id);
        self.rule_name_criteria.push(name);
        self.rule_value_criteria.push(value);
        self.rule_keywords.push(keywords);
        self.rule_entropy.push(entropy);
        self.rule_descriptions.push(description);

        rule_id
    }
}

impl From<GitLeaksConfig> for Config {
    fn from(gitleaks_config: GitLeaksConfig) -> Self {
        const CASE_INSENSITIVE: &str = "(?i)";
        const ASSIGNMENT_REGEX_PATTERN: &str =
            r#"(?:[\s|']|[\s|"]){0,3}(?:=|>|:{1,3}=|\|\|:|<=|=>|:|\?=)(?:'|\"|\s|=|\x60){0,5}"#;

        let mut config = Self::with_capacity(gitleaks_config.rules.len());

        for rule in gitleaks_config.rules {
            let case_insensitive = rule.regex.starts_with(CASE_INSENSITIVE);
            // remove case-insensitive prefix from regex pattern. We'll add it
            // back to both then name and value patterns later.
            let pattern = if case_insensitive {
                &rule.regex[CASE_INSENSITIVE.len()..]
            } else {
                rule.regex.as_str()
            };

            let (name, value) = if pattern.contains(ASSIGNMENT_REGEX_PATTERN) {
                let mut split = pattern.split(ASSIGNMENT_REGEX_PATTERN);
                let name = split.next().unwrap();
                let value = split.next().unwrap();
                (Some(name), value)
            } else {
                (None, pattern)
            };

            let name = name.and_then(|name| {
                RegexBuilder::new(name)
                    .case_insensitive(case_insensitive)
                    .build()
                    .ok()
                    .map(Pattern::from)
            });

            let compiled_value = RegexBuilder::new(value)
                .case_insensitive(case_insensitive)
                .build()
                .into_diagnostic();

            let value: Pattern = match compiled_value {
                Ok(regex) => regex.into(),
                Err(e) => {
                    warn!(
                        "{:?}",
                        e.context(format!(
                            "Failed to compile value pattern for rule {}",
                            rule.id
                        ))
                    );
                    continue;
                }
            };

            config.add_rule(
                rule.id,
                name,
                value,
                rule.keywords.unwrap_or_default(),
                rule.entropy,
                rule.description,
            );
        }

        config
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // #[test]
    // fn test_default() {
    //     let config = Config::default();
    //     assert!(!config.name_rules().is_empty());
    // }

    #[test]
    fn from_gitleaks() -> Result<()> {
        let config = Config::gitleaks();
        assert!(!config.is_empty());
        assert_eq!(config.len(), config.iter_name_criteria().count());
        assert_eq!(config.len(), config.iter_value_criteria().count());
        assert!(
            config
                .iter_name_criteria()
                .filter(|(_, pat)| pat.is_some())
                .count()
                > 0
        );

        const NAME_COL_WIDTH: usize = 80;
        for id in 0..config.len() {
            let rule_id = RuleId::from_usize(id);
            let display_id = config.get_display_id(rule_id);
            let name = config.get_name_criteria(rule_id);
            let value = config.get_value_criteria(rule_id);
            let name_str = format!("{name:?}");
            let padding = if name_str.len() > NAME_COL_WIDTH {
                2
            } else {
                (NAME_COL_WIDTH - name_str.len()).max(2)
            };

            let rule_id_padding = match id {
                id if id < 10 => 3,
                id if id < 100 => 2,
                _ => 1,
            };
            let id_padding = 40 - display_id.len();
            let name_str: String = match name {
                Some(pat) => format!("{}", pat),
                None => "None".into(),
            };

            println!(
                "Rule {rule_id}{}({}){}:\t{}{}{}",
                " ".repeat(rule_id_padding),
                display_id,
                " ".repeat(id_padding),
                name_str,
                " ".repeat(padding),
                value
            );
        }

        Ok(())
    }
}
