use miette::Result;

use crate::ApiKeyError;

use super::graphical::GraphicalReportHandler;

pub struct Reporter {
    handler: GraphicalReportHandler,
    keys_found: usize,
    urls_checked: usize,
}

impl Default for Reporter {
    fn default() -> Self {
        Self {
            handler: Default::default(),
            keys_found: 0,
            urls_checked: 0,
        }
    }
}

impl Reporter {
    pub fn report_keys(&mut self, keys: &Vec<ApiKeyError>) -> Result<()> {
        self.keys_found += keys.len();
        self.handler.report_keys(keys.iter())
    }
    pub fn report_key(&mut self, key: &ApiKeyError) -> Result<()> {
        self.keys_found += 1;
        self.handler.report_key(&key)
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
