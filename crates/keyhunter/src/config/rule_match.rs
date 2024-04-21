use super::{Pattern, Rule};

impl Rule {
    /// Finds rule violations against a candidate source string. For name rules
    /// the candidate will usually be a variable or property name, and for value
    /// rules it will usually be a string literal.
    ///
    /// Returns a list of matched keys if any are found, [`None`] if the rule is
    /// not violated. The api key itself is provided as well as the offset from
    /// `candidate`'s start for where the key is located. Because this method
    /// only returns [`Some`] when keys are found, the contained [`Vec`] is
    /// guaranteed to not be empty.
    ///
    /// ## Algorithm
    /// - Perform a quick keyword check against the string for rules with
    ///   keywords.
    /// - Run the rule's pattern against `candidate`, finding any substrings
    ///   that get matched. For regex patterns, this will be all substrings
    ///   caught in capture groups.
    /// - Filter out captured keys that do not meet the rule's minimum Shannon
    ///   entropy threshold. This check is skipped for rules without this
    ///   threshold.
    ///
    pub fn captures<'s>(&self, candidate: &'s str) -> Option<Vec<(usize, &'s str)>> {
        // keyword check to quickly weed out non-candidates
        if !self.check_keywords(candidate) {
            return None;
        }

        match self.pattern() {
            Pattern::Regex(regex) => {
                let captures: regex::Captures<'s> = regex.captures(candidate)?;

                let found_keys = captures
                    .iter()
                    .filter_map(|cap| {
                        let cap = cap?;
                        let found = self.check_entropy(cap.as_str())?;

                        // Hack used by gitleaks
                        // see: https://github.com/gitleaks/gitleaks/blob/57ac4b3dc7f926b4c40882e476a951506675c95a/detect/detect.go#L356
                        if self.is_value_rule()
                            && self.id.starts_with("generic")
                            && !found.contains_digit()
                        {
                            return None;
                        }

                        // empty strings are never API keys
                        if found.trim().is_empty() {
                            None
                        } else {
                            Some((cap.start(), found))
                        }
                    })
                    .collect::<Vec<_>>();

                // Do not return Some(vec) when vec is empty
                if found_keys.is_empty() {
                    None
                } else {
                    Some(found_keys)
                }
            }
            Pattern::String(s) => {
                let start = candidate.find(s)?;
                let capture_end = start + s.len();

                // extract full word that was found. walk past capture end until
                // whitespace is encountered. TODO: walk backwards from start.
                let mut i = 0;
                for trailing in candidate[capture_end..].chars() {
                    if trailing.is_whitespace() {
                        break;
                    }
                    i += 1;
                }
                let end = capture_end + i;
                let key = self.check_entropy(&candidate[start..end])?;

                Some(vec![(start, key)])
            }
        }
    }

    pub fn matches_name(&self, identifier: &str, value: &str) -> bool {
        debug_assert!(
            self.is_name_rule(),
            "Rule::matches_name can only be called on name rules"
        );
        if !self.matches(identifier) {
            return false;
        }

        return self.check_entropy(value).is_some();
    }

    fn matches(&self, haystack: &str) -> bool {
        if !self.check_keywords(haystack) {
            return false;
        }
        match self.pattern() {
            Pattern::String(pat) => haystack.contains(pat),
            Pattern::Regex(pat) => pat.is_match(haystack),
        }
    }

    fn check_keywords(&self, candidate: &str) -> bool {
        match self.keywords() {
            None => true,
            Some(kw) => kw.iter().any(|kw| candidate.contains(kw)),
        }
    }

    /// Apply minimum entropy check on matched api keys if the rule has one.
    /// Using [`Option`] for clean and easy control flow.
    fn check_entropy<'s>(&self, found_key: &'s str) -> Option<&'s str> {
        match self.entropy {
            Some(entropy) if entropy < found_key.entropy() => Some(found_key),
            None => Some(found_key),
            _ => None,
        }
    }
}

/// Calculates the Shannon entropy of a byte string.
///
/// Implementation borrowed from [Rosetta Code](https://rosettacode.org/wiki/Entropy#Rust).
///
/// see: [Entropy (Wikipedial)](https://en.wikipedia.org/wiki/Entropy_(information_theory))
fn entropy<S: AsRef<[u8]>>(string: S) -> f32 {
    let mut histogram = [0u32; 256];
    let bytes = string.as_ref();
    let len = bytes.len() as f32;

    for &b in bytes {
        histogram[b as usize] += 1;
    }

    histogram
        .iter()
        .cloned()
        .filter(|&h| h != 0)
        .map(|h| h as f32 / len)
        .map(|ratio| -ratio * ratio.log2())
        .sum()
}

trait Entropy {
    fn entropy(&self) -> f32;
}

impl<S> Entropy for S
where
    S: AsRef<[u8]>,
{
    fn entropy(&self) -> f32 {
        entropy(self)
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

#[cfg(test)]
mod test {
    use super::*;
    use regex::{Regex, RegexBuilder};

    #[derive(Debug)]
    struct Tester {
        pub rule: Rule,
    }
    impl<P> From<P> for Tester
    where
        Pattern: From<P>,
    {
        fn from(pattern: P) -> Self {
            Self {
                rule: Rule {
                    pattern: pattern.into(),
                    ..Default::default()
                },
            }
        }
    }
    impl Tester {
        pub fn test_captures<'s, T>(&self, test_cases: T)
        where
            T: IntoIterator<Item = (&'s str, Vec<(usize, &'s str)>)>,
        {
            for (input, expected) in test_cases {
                let actual = self.rule.captures(input);
                assert_eq!(actual, Some(expected), "'{}'", input);
            }
        }
    }
    #[test]
    fn test_string_capture() {
        let tester: Tester = "api-key".into();
        tester.test_captures([
            // TODO: should be "x-api-key"
            ("x-api-key", vec![(2, "api-key")]),
            ("api-key-header", vec![(0, "api-key-header")]),
            ("api-key header", vec![(0, "api-key")]),
            ("some api-key header", vec![(5, "api-key")]),
        ]);
    }

    #[test]
    fn test_regex_capture() {
        // FIXME: gitleak's regex patterns check for variable names
        let pat = r#"(?i)(?:key|api|token|secret|client|passwd|password|auth|access)(?:[0-9a-z\-_\t .]{0,20})(?:[\s|']|[\s|"]){0,3}(?:=|>|:{1,3}=|\|\|:|<=|=>|:|\?=)(?:'|\"|\s|=|\x60){0,5}([0-9a-z\-_.=]{10,150})(?:['|\"|\n|\r|\s|\x60|;]|$)"#;
        let tester: Tester = Regex::new(pat).unwrap().into();

        tester.test_captures([
            (
                "const Discord_api_key = e7322523fb86ed64c836a979cf8465fbd436378c653c1db38f9ae87bc62a6fd5",
                vec![
                    (14, "api_key = e7322523fb86ed64c836a979cf8465fbd436378c653c1db38f9ae87bc62a6fd5"),
                    (
                    24,
                    "e7322523fb86ed64c836a979cf8465fbd436378c653c1db38f9ae87bc62a6fd5",
                )],
            ),
        ]);
    }

    #[test]
    fn test_entropy() {
        let test_cases = vec![
            ("hello world", "hello world".entropy()),
            ("hello world", b"hello world".entropy()),
            ("hello world", String::from("hello world").entropy()),
            ("hello world", 2.8453512),
        ];

        for (input, expected) in test_cases {
            let actual = entropy(input);
            assert!(
                (actual - expected).abs() < f32::EPSILON,
                "expected entropy({input}) to be {expected}, got {actual}"
            );
        }
    }

    #[test]
    fn test_contains_digit() {
        let yes = vec![
            "1",
            "abcde1",
            "0abcdefg",
            "asdfvoapsdhfoaisdhf9apoisdhoiashdfp",
        ];
        let no = vec!["", "abc", "apisodhfapiosdhfoasihdfoiahsgdiophasdg"];

        for y in yes {
            assert!(y.contains_digit());
        }

        for n in no {
            assert!(!n.contains_digit());
        }
    }
}
