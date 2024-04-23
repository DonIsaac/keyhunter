use miette::Result;

use crate::ApiKeyError;

use super::graphical::GraphicalReportHandler;

#[derive(Default)]
pub struct Reporter {
    handler: GraphicalReportHandler,
    keys_found: usize,
    urls_checked: usize,
}

impl Reporter {
    pub fn report_keys(&mut self, keys: &Vec<ApiKeyError>) -> Result<()> {
        self.keys_found += keys.len();
        self.handler.report_keys(keys.iter())
    }
    pub fn report_key(&mut self, key: &ApiKeyError) -> Result<()> {
        self.keys_found += 1;
        self.handler.report_key(key)
    }

    #[inline]
    pub fn keys_found(&self) -> usize {
        self.keys_found
    }

    #[inline]
    pub fn urls_checked(&self) -> usize {
        self.urls_checked
    }
}
