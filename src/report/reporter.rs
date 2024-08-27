use miette::Result;

use crate::ApiKeyError;

use super::{statistics::Statistics, ReportHandler};

#[must_use]
pub struct Reporter<R> {
    handler: R,
    stats: Statistics,
}

impl<R: ReportHandler + Default> Default for Reporter<R> {
    #[inline]
    fn default() -> Self {
        Self::new(R::default())
    }
}

impl<R> Reporter<R> {
    pub fn new(handler: R) -> Self {
        Self {
            handler,
            stats: Statistics::default(),
        }
    }

    #[inline]
    pub fn record_scripts_checked(&self, count: usize) {
        self.stats.record_scripts_checked(count);
    }

    #[inline]
    pub fn record_pages_crawled(&self, count: usize) {
        self.stats.record_pages_crawled(count);
    }

    // pub fn with_redacted(mut self, yes: bool) -> Self {
    //     debug!("Setting redacted to {}", yes);
    //     // self.handler = self.handler.with_redacted(yes);
    //     self.handler.set_redacted(yes);
    //     self
    // }

    #[inline]
    pub fn keys_found(&self) -> usize {
        self.stats.keys_found()
    }

    #[inline]
    pub fn scripts_checked(&self) -> usize {
        self.stats.scripts_checked()
    }

    #[inline]
    pub fn pages_crawled(&self) -> usize {
        self.stats.pages_crawled()
    }
}

impl<R: ReportHandler> Reporter<R> {
    pub fn report_keys(&self, keys: &[ApiKeyError]) -> Result<()> {
        self.stats
            .record_keys_found(keys.iter().map(|k| k.secret.clone()));
        self.handler.report_keys(keys.iter())
    }

    pub fn report_key(&self, key: &ApiKeyError) -> Result<()> {
        self.stats.record_keys_found([key.secret.clone()]);
        self.handler.report_key(key)
    }
}
