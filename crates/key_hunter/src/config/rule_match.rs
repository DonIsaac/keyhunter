use crate::Rule;

impl Rule {
    pub fn matches(&self, candidate: &str) -> bool {
        if let Some(keywords) = self.keywords() {
            if !keywords.iter().any(|keyword| candidate.contains(keyword)) {
                return false;
            }
        }

        self.pattern().matches(candidate)
    }
}
