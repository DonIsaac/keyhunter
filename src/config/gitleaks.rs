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
    pub keywords: Option<TinyVec<[String; 1]>>,
    pub entropy: Option<f32>,
}

impl GitLeaksConfig {
    pub const DEFAULT_CONFIG: &'static str = include_str!("./gitleaks.toml");

    pub fn default_config() -> Self {
        toml::from_str(Self::DEFAULT_CONFIG).unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GitLeaksConfig::default_config();
        assert_eq!(config.title.as_ref().map(String::as_str), Some("gitleaks config"));
        assert!(config.rules.len() > 100);
    }

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
