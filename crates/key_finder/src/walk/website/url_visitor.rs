use dashmap::DashSet;
use log::{debug, error, info, trace, warn};
use miette::{Context as _, Error, IntoDiagnostic as _, Result};
use rand::Rng;
use std::{
    borrow::{Borrow, Cow},
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        mpsc, Arc, Once, OnceLock, RwLock,
    },
    time::Duration,
};

use tinyvec::TinyVec;
use ureq::{Agent, AgentBuilder};
use url::Url;

use rayon::{prelude::*, ThreadPool};

use super::{
    dom_walker::{DomVisitor, DomWalker},
    error::{NoContentDiagnostic, NotHtmlDiagnostic},
};

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
    pub fn with_attr(mut self, attr: &'static str) -> Self {
        self.attr_names.push(attr);
        self
    }

    pub fn with_attrs<A: IntoIterator<Item = &'static str>>(mut self, attrs: A) -> Self {
        self.attr_names.extend(attrs);

        self
    }

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
            if let Some(value) = node.attr(*attr) {
                self.urls.push(value.to_string());
                return;
            }
        }
    }
}
