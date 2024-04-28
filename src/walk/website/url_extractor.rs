use url::{ParseError, Url};

// Copyright © 2024 Don Isaac
//
// This file is part of KeyHunter.
//
// KeyHunter is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version.
//
// KeyHunter is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
// FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for
// more details.
//
// You should have received a copy of the GNU General Public License along with
// KeyHunter. If not, see <https://www.gnu.org/licenses/>.
use super::dom_walker::DomVisitor;

const BANNED_EXTENSIONS: [&str; 3] = [".pdf", ".png", ".jpg"];

/// Extracts URLs to webpages and scripts from HTML.
#[derive(Debug)]
pub(crate) struct UrlExtractor<'html> {
    base_url: &'html Url,
    pages: Vec<Url>,
    scripts: Vec<Url>,
}

impl<'html> UrlExtractor<'html> {
    pub fn new(base_url: &'html Url) -> Self {
        const CAP: usize = 10;
        debug_assert!(!base_url.cannot_be_a_base());

        Self {
            base_url,
            pages: Vec::with_capacity(CAP),
            scripts: Vec::with_capacity(CAP),
        }
    }

    /// (pages, scripts)
    #[must_use]
    pub fn into_inner(self) -> (Vec<Url>, Vec<Url>) {
        (self.pages, self.scripts)
    }

    fn resolve(&self, url: &'html str) -> Result<Url, ParseError> {
        if url.starts_with('/') || !url.contains("://") {
            self.base_url.join(url)
        } else {
            Url::parse(url)
        }
    }

    fn record_script(&mut self, script_url: &'html str) {
        let Ok(script_url) = self.resolve(script_url) else {
            return;
        };
        self.scripts.push(script_url);
    }

    fn record_page(&mut self, page_url: &'html str) {
        let page_url = page_url.trim();
        if page_url.is_empty()
            || page_url.starts_with('#')
            || page_url.starts_with("mailto:")
            || page_url.starts_with("javascript:")
        {
            return;
        }

        let Ok(page_url) = self.resolve(page_url) else {
            return;
        };

        // Many image links have query parameters, so we do this check after
        // parsing the URL
        if BANNED_EXTENSIONS
            .iter()
            .any(|ext| page_url.path().ends_with(ext))
        {
            return;
        }

        self.pages.push(page_url);
    }
}

impl<'dom> DomVisitor<'dom> for UrlExtractor<'dom> {
    fn visit_element(&mut self, node: &'dom scraper::node::Element) {
        match node.name() {
            "script" => {
                let Some(script_url) = node.attr("src") else {
                    return;
                };
                self.record_script(script_url);
            }
            "a" => {
                let Some(page_url) = node.attr("href") else {
                    return;
                };
                self.record_page(page_url);
            }
            _ => { /* noop */ }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::walk::website::dom_walker::DomWalker;

    use super::*;
    use url::Url;
    #[test]

    fn test_basic() {
        let url = Url::parse("https://example.com").unwrap();
        let html = r#"
<html>
<head>
        <script src="main.js"></script>
</head>
<body>
        <a href="https://example.com/foo">foo</a>
        <a href="bar">bar</a>
        <a href="/baz">baz</a>
</body>
</html>
        "#;
        let mut extractor = UrlExtractor::new(&url);
        let dom = DomWalker::new(&html).unwrap();
        dom.walk(&mut extractor);
        let (pages, scripts) = extractor.into_inner();

        assert_eq!(
            scripts,
            vec![Url::parse("https://example.com/main.js").unwrap()]
        );

        assert_eq!(pages.len(), 3);
        for expected in [
            "https://example.com/foo",
            "https://example.com/bar",
            "https://example.com/baz",
        ] {
            let u = Url::parse(expected).unwrap();
            assert!(pages.contains(&u), "{u} is not in extracted pages list");
        }
    }

    #[test]
    fn test_ignored() {
        let url = Url::parse("https://example.com").unwrap();
        let html = r"
<html>
<body>
        <a href='#section'>intra-page links</a>
        <a href='mailto:foo@example.com'>emails</a>
        <a href='javascript:void(0)'>js</a>
        <a href='/assets/pic.jpg?id=123'>images</a>
</body>
</html>
        ";

        let mut extractor = UrlExtractor::new(&url);
        let dom = DomWalker::new(&dbg!(html)).unwrap();
        dom.walk(&mut extractor);
        let (pages, scripts) = extractor.into_inner();

        assert!(pages.is_empty(), "found pages: {pages:#?}");
        assert!(scripts.is_empty());
    }
}
