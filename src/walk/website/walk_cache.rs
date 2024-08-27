use dashmap::DashSet;
use rustc_hash::FxBuildHasher;
use std::{
    borrow::{Borrow, Cow},
    hash::Hash,
    ops::Deref,
    sync::Arc,
};
use url::Url;

#[derive(Debug, Default, Clone)]
pub struct WalkCache {
    /// Web pages already visited. Prevents cycles.
    seen_urls: Arc<DashSet<Arc<Url>, FxBuildHasher>>,
    /// Scripts already seen. Prevents duplicates from being sent over the
    /// script channel.
    seen_scripts: Arc<DashSet<Arc<Url>, FxBuildHasher>>,
}

impl WalkCache {
    pub fn see_script(&self, url: Arc<Url>) {
        self.seen_scripts.insert(url);
    }

    pub fn has_seen_script<Q>(&self, url: &Q) -> bool
    where
        Arc<Url>: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.seen_scripts.contains(url)
    }

    pub fn see_url(&self, url: Arc<Url>) {
        self.seen_urls.insert(url);
    }

    #[inline]
    pub fn url_cache_contains<Q>(&self, url: &Q) -> bool
    where
        Arc<Url>: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.seen_urls.contains(url)
    }

    pub fn has_seen_url(&self, url: &Arc<Url>) -> bool {
        debug_assert!(
            !url.cannot_be_a_base(),
            "skip_if_visited got a relative url"
        ); // should be absolute

        if url.query().is_none() && url.fragment().is_none() {
            return self.has_visited_url_clean(Arc::clone(url));
        }

        // remove #section hash and (most) query parameters from URL since they
        // don't affect what page the URL points to. Note that some applications
        // use query parameters to identify what page to go to, thus the below
        // query_pairs() check. We may need to update this list as new cases are
        // brought to light.
        let mut without_query_params = url.deref().clone();
        without_query_params.set_query(None);
        without_query_params.set_fragment(None);
        let new_params = Self::filter_query_params(url);

        if new_params.is_empty() {
            self.has_visited_url_clean(Arc::new(without_query_params))
        } else {
            let query = new_params
                .into_iter()
                .fold(String::new(), |acc, (key, value)| {
                    acc + format!("{key}={value}").as_str()
                });
            without_query_params.set_query(Some(query.as_str()));
            self.has_visited_url_clean(Arc::new(without_query_params))
        }
    }

    fn filter_query_params(url: &Url) -> Vec<(Cow<'_, str>, Cow<'_, str>)> {
        let mut new_params: Vec<(Cow<'_, str>, Cow<'_, str>)> = vec![];
        for (key, value) in url.query_pairs() {
            // TODO: use phf?
            if matches!(
                key.borrow(),
                "tab" | "tabid" | "tab_id" | "tab-id" | "id" | "page" | "page_id" | "page-id"
            ) {
                new_params.push((key, value))
            }
        }
        new_params
    }

    fn has_visited_url_clean(&self, url: Arc<Url>) -> bool {
        if self.url_cache_contains(&url) {
            true
        } else {
            self.see_url(url);
            false
        }
    }

    /// Clear all seens URLs and Scripts from the cache.
    pub fn clear(&mut self) {
        self.seen_scripts.clear();
        self.seen_urls.clear();
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;
    use url::Url;

    fn example() -> Arc<Url> {
        Arc::new(Url::parse("https://example.com").unwrap())
    }

    #[test]
    fn test_url() {
        let mut cache = super::WalkCache::default();
        let url = example();
        assert!(!cache.url_cache_contains(&url));

        cache.see_url(url.clone());
        assert!(cache.url_cache_contains(&url));

        cache.clear();
        assert!(!cache.url_cache_contains(&url));
    }

    #[test]
    fn test_script() {
        let mut cache = super::WalkCache::default();
        let url = example();
        assert!(!cache.has_seen_script(&url));

        cache.see_script(Arc::clone(&url));
        assert!(cache.has_seen_script(&url));

        cache.clear();
        assert!(!cache.has_seen_script(&url));
    }
}
