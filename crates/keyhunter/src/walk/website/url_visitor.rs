use tinyvec::TinyVec;

use rayon::prelude::*;

use super::dom_walker::DomVisitor;

#[derive(Debug)]
pub(crate) struct UrlVisitor {
    urls: Vec<String>,
    tag_name: &'static str,
    attr_names: TinyVec<[&'static str; 2]>,
}

impl UrlVisitor {
    pub fn new(tag_name: &'static str, attr_name: &'static str) -> Self {
        Self {
            tag_name,
            attr_names: tinyvec::tiny_vec!([&'static str; 2] => attr_name),
            urls: vec![],
        }
    }

    #[must_use]
    pub fn into_inner(self) -> Vec<String> {
        self.urls
    }
}

impl IntoIterator for UrlVisitor {
    type Item = String;
    type IntoIter = <Vec<String> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.urls.into_iter()
    }
}

impl<'dom> DomVisitor<'dom> for UrlVisitor {
    fn visit_element(&mut self, node: &'dom scraper::node::Element) {
        let is_tag = node
            .name
            .local
            .as_parallel_string()
            .eq_ignore_ascii_case(self.tag_name);
        if !is_tag {
            return;
        }
        for attr in &self.attr_names {
            if let Some(value) = node.attr(attr) {
                self.urls.push(value.to_string());
                return;
            }
        }
    }
}
