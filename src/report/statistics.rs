use dashmap::DashSet;
use rustc_hash::FxBuildHasher;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Default)]
pub(super) struct Statistics {
    // TODO: record how many instances of each key were found?
    keys_found: DashSet<String, FxBuildHasher>,
    scripts_checked: AtomicUsize,
    pages_crawled: AtomicUsize,
}

impl Statistics {
    #[inline]
    pub fn record_keys_found<I: IntoIterator<Item = String>>(&self, keys: I) {
        for key in keys {
            self.keys_found.insert(key);
        }
    }

    #[inline]
    pub fn record_scripts_checked(&self, count: usize) {
        self.scripts_checked.fetch_add(count, Ordering::Relaxed);
    }

    #[inline]
    pub fn record_pages_crawled(&self, count: usize) {
        self.pages_crawled.fetch_add(count, Ordering::Relaxed);
    }

    #[inline]
    pub fn keys_found(&self) -> usize {
        self.keys_found.len()
    }

    #[inline]
    pub fn scripts_checked(&self) -> usize {
        self.scripts_checked.load(Ordering::SeqCst)
    }

    #[inline]
    pub fn pages_crawled(&self) -> usize {
        self.pages_crawled.load(Ordering::SeqCst)
    }
}
