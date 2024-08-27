use core::fmt;

use url::{ParseError, ParseOptions, Url};

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
use super::{
    dom_walker::{self, DomVisitor},
    Script,
};

const BANNED_EXTENSIONS: [&str; 3] = [".pdf", ".png", ".jpg"];

/// Extracts URLs to webpages and scripts from HTML.
pub(crate) struct UrlExtractor<'html> {
    /// URL of the page being parsed.
    page_url: &'html Url,
    opts: ParseOptions<'html>,
    pages: Vec<Url>,
    scripts: Vec<Script>,
}

impl fmt::Debug for UrlExtractor<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UrlExtractor")
            .field("pages", &self.pages)
            .field("scripts", &self.scripts)
            .finish()
    }
}

impl<'html> UrlExtractor<'html> {
    pub fn new(base_url: &'html Url, page_url: &'html Url) -> Self {
        const CAP: usize = 10;
        debug_assert!(!base_url.cannot_be_a_base());

        Self {
            page_url,
            opts: Url::options().base_url(Some(base_url)),
            pages: Vec::with_capacity(CAP),
            scripts: Vec::with_capacity(CAP),
        }
    }

    /// (pages, scripts)
    #[must_use]
    pub fn into_inner(self) -> (Vec<Url>, Vec<Script>) {
        (self.pages, self.scripts)
    }

    fn resolve(&self, url: &'html str) -> Result<Url, ParseError> {
        self.opts.parse(url)
    }

    fn record_remote_script(&mut self, script_url: &'html str) {
        let Ok(script_url) = self.resolve(script_url) else {
            return;
        };
        self.scripts.push(Script::Url(script_url));
    }

    fn record_embedded_script(&mut self, script: &str) {
        self.scripts
            .push(Script::Embedded(script.to_string(), self.page_url.clone()));
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
    fn visit_element(&mut self, node: dom_walker::ElementRef<'dom>) {
        match node.name() {
            "script" => match node.attr("src") {
                Some(script_url) => self.record_remote_script(script_url),
                None => {
                    self.record_embedded_script(node.text().collect::<String>().trim());
                }
            },
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

        let mut extractor = UrlExtractor::new(&url, &url);
        let dom = DomWalker::new(html).unwrap();
        dom.walk(&mut extractor);
        let (pages, scripts) = extractor.into_inner();

        assert_eq!(
            scripts,
            vec![Script::Url(
                Url::parse("https://example.com/main.js").unwrap()
            )]
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

        let mut extractor = UrlExtractor::new(&url, &url);
        let dom = DomWalker::new(html).unwrap();
        dom.walk(&mut extractor);
        let (pages, scripts) = extractor.into_inner();

        assert!(pages.is_empty(), "found pages: {pages:#?}");
        assert!(scripts.is_empty());
    }

    #[test]
    fn test_embedded_script() {
        let url = Url::parse("https://example.com").unwrap();
        let html = r#"
    <html>
    <head>
        <script>
            console.log("hello, world");
        </script>
        <script>
            console.log("goodbye, world");
        </script>
    </head>
    <body></body>
    </html>
    "#;

        let mut extractor = UrlExtractor::new(&url, &url);
        let dom = DomWalker::new(html).unwrap();
        dom.walk(&mut extractor);
        let (pages, scripts) = extractor.into_inner();

        assert!(pages.is_empty(), "found pages: {pages:#?}");
        assert_eq!(
            scripts,
            vec![
                Script::Embedded("console.log(\"hello, world\");".to_string(), url.clone()),
                Script::Embedded("console.log(\"goodbye, world\");".to_string(), url),
            ]
        );
    }
}
