use dashmap::DashSet;
use rustc_hash::FxBuildHasher;
use std::{borrow::Borrow, hash::Hash, sync::Arc};
use url::Url;

#[derive(Debug, Default, Clone)]
pub struct WalkCache {
    /// Web pages already visited. Prevents cycles.
    seen_urls: Arc<DashSet<Url, FxBuildHasher>>,
    /// Scripts already seen. Prevents duplicates from being sent over the
    /// script channel.
    seen_scripts: Arc<DashSet<Url, FxBuildHasher>>,
}

impl WalkCache {
    pub fn see_url(&self, url: Url) {
        self.seen_urls.insert(url);
    }

    pub fn has_seen_url<Q>(&self, url: &Q) -> bool
    where
        Url: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.seen_urls.contains(url)
    }

    pub fn see_script(&self, url: Url) {
        self.seen_scripts.insert(url);
    }

    pub fn has_seen_script<Q>(&self, url: &Q) -> bool
    where
        Url: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.seen_scripts.contains(url)
    }

    /// Clear all seens URLs and Scripts from the cache.
    pub fn clear(&mut self) {
        self.seen_scripts.clear();
        self.seen_urls.clear();
    }
}
