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
    pub fn matches<'s>(&self, candidate: &'s str) -> Option<Vec<(usize, &'s str)>> {
        // keyword check to quickly weed out non-candidates
        if self
            .keywords()
            .is_some_and(|keywords| !keywords.iter().any(|keyword| candidate.contains(keyword)))
        {
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
                let end = start + s.len();
                let end = candidate[(start + s.len())..]
                    .find(char::is_whitespace)
                    .unwrap_or(end);
                let key = self.check_entropy(&candidate[start..end])?;

                Some(vec![(start, key)])
            }
        }
    }

    /// Apply minimum entropy check on matched api keys if the rule has one.
    /// Using [`Option`] for clean and easy control flow.
    fn check_entropy<'s>(&self, found_key: &'s str) -> Option<&'s str> {
        match self.entropy {
            Some(entropy) if entropy < found_key.entropy() => Some(found_key),
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
        entropy(&self)
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
}
