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
use regex::Regex;

#[derive(Debug)]
pub enum Pattern {
    Regex(Regex),
    String(String),
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

impl std::fmt::Display for Pattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Regex(pat) => pat.fmt(f),
            Self::String(pat) => {
                write!(f, "/")?;
                pat.fmt(f)?;
                write!(f, "/")
            }
        }
    }
}

impl Pattern {
    pub fn matches(&self, value: &str) -> bool {
        match self {
            Self::Regex(regex) => regex.is_match(value),
            Self::String(ref s) => value.contains(s),
        }
    }

    pub fn captures<'s>(&self, haystack: &'s str) -> Vec<(usize, &'s str)> {
        match self {
            Self::Regex(regex) => {
                let Some(captures) = regex.captures(haystack) else {
                    return vec![];
                };

                let mut found_keys = captures
                    .iter()
                    .filter_map(|cap| {
                        let cap = cap?;
                        let found = cap.as_str();

                        // empty strings are never API keys
                        if found.trim().is_empty() {
                            None
                        } else {
                            Some((cap.start(), found))
                        }
                    })
                    .collect::<Vec<_>>();

                found_keys.dedup();
                found_keys
            }
            Self::String(s) => {
                let Some(start) = haystack.find(s) else {
                    return vec![];
                };
                let capture_end = start + s.len();

                // extract full word that was found. walk past capture end until
                // whitespace is encountered.

                // walk past end until a breakable char is reached
                let mut i = 0;
                for trailing in haystack[capture_end..].chars() {
                    if Self::is_break_char(trailing) {
                        break;
                    }
                    i += 1;
                }
                let end = capture_end + i;

                i = 0;

                // walk backwards from start until a breakable char is reached
                if start > 0 {
                    for leading in haystack[0..(start + 1)].chars().rev() {
                        if Self::is_break_char(leading) {
                            break;
                        }
                        i += 1;
                    }
                }
                let start = start - i;

                vec![(start, &haystack[start..end])]
            }
        }
    }

    fn is_break_char(c: char) -> bool {
        c.is_whitespace() || c == ';'
    }
}
