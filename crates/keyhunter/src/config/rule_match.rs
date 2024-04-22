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
use super::{entropy::Entropy as _, Config, RuleId};
// use rayon::prelude::*;

impl Config {
    pub fn check_name(&self, rule_id: RuleId, identifier_name: &str) -> bool {
        let Some(name_criteria) = self.get_name_criteria(rule_id) else {
            return true;
        };

        name_criteria.matches(identifier_name)
    }

    pub fn check_values<'c, 's: 'c>(
        &'c self,
        haystack: &'s str,
    ) -> impl Iterator<Item = (RuleId, usize, &'s str)> + 'c {
        let collected = self
            .iter_value_criteria()
            // .par_bridge()
            .flat_map(|(rule_id, pat)| {
                Some(
                    pat.captures(haystack)
                        .into_iter()
                        .flat_map(move |cap| Some((rule_id, cap.0, cap.1))),
                )
            })
            .flatten()
            .filter(|cap| {
                // Hack for generic rules used by gitleaks
                if self.get_display_id(cap.0).starts_with("generic") && !cap.2.contains_digit() {
                    return false;
                }
                if let Some(entropy) = self.rule_entropy[cap.0] {
                    entropy <= cap.2.entropy()
                } else {
                    true
                }
            });

        collected
    }
}

trait ContainsDigit {
    /// Returns `true` if this string contains at least one ASCII digit.
    fn contains_digit(&self) -> bool;
}
impl<S: AsRef<str>> ContainsDigit for S {
    fn contains_digit(&self) -> bool {
        self.as_ref().chars().any(|c| c.is_ascii_digit())
    }
}

// #[cfg(test)]
// mod test {
//     use super::*;
//     use regex::Regex;

//     #[derive(Debug)]
//     struct Tester {
//         pub rule: RuleOld,
//     }
//     impl<P> From<P> for Tester
//     where
//         Pattern: From<P>,
//     {
//         fn from(pattern: P) -> Self {
//             Self {
//                 rule: RuleOld {
//                     pattern: pattern.into(),
//                     ..Default::default()
//                 },
//             }
//         }
//     }
//     impl Tester {
//         pub fn test_captures<'s, T>(&self, test_cases: T)
//         where
//             T: IntoIterator<Item = (&'s str, Vec<(usize, &'s str)>)>,
//         {
//             for (input, expected) in test_cases {
//                 let actual = self.rule.captures(input);
//                 assert_eq!(actual, Some(expected), "'{}'", input);
//             }
//         }
//     }
//     #[test]
//     fn test_string_capture() {
//         let tester: Tester = "api-key".into();
//         tester.test_captures([
//             // TODO: should be "x-api-key"
//             ("x-api-key", vec![(2, "api-key")]),
//             ("api-key-header", vec![(0, "api-key-header")]),
//             ("api-key header", vec![(0, "api-key")]),
//             ("some api-key header", vec![(5, "api-key")]),
//         ]);
//     }

//     #[test]
//     fn test_regex_capture() {
//         // FIXME: gitleak's regex patterns check for variable names
//         let pat = r#"(?i)(?:key|api|token|secret|client|passwd|password|auth|access)(?:[0-9a-z\-_\t .]{0,20})(?:[\s|']|[\s|"]){0,3}(?:=|>|:{1,3}=|\|\|:|<=|=>|:|\?=)(?:'|\"|\s|=|\x60){0,5}([0-9a-z\-_.=]{10,150})(?:['|\"|\n|\r|\s|\x60|;]|$)"#;
//         let tester: Tester = Regex::new(pat).unwrap().into();

//         tester.test_captures([
//             (
//                 "const Discord_api_key = e7322523fb86ed64c836a979cf8465fbd436378c653c1db38f9ae87bc62a6fd5",
//                 vec![
//                     (14, "api_key = e7322523fb86ed64c836a979cf8465fbd436378c653c1db38f9ae87bc62a6fd5"),
//                     (
//                     24,
//                     "e7322523fb86ed64c836a979cf8465fbd436378c653c1db38f9ae87bc62a6fd5",
//                 )],
//             ),
//         ]);
//     }

//     #[test]
//     fn test_contains_digit() {
//         let yes = vec![
//             "1",
//             "abcde1",
//             "0abcdefg",
//             "asdfvoapsdhfoaisdhf9apoisdhoiashdfp",
//         ];
//         let no = vec!["", "abc", "apisodhfapiosdhfoasihdfoiahsgdiophasdg"];

//         for y in yes {
//             assert!(y.contains_digit());
//         }

//         for n in no {
//             assert!(!n.contains_digit());
//         }
//     }
// }
