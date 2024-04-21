use miette::Result;

use crate::ApiKeyError;

use super::graphical::GraphicalReportHandler;

#[derive(Debug)]
pub struct Reporter {
    handler: GraphicalReportHandler,
    keys_found: usize,
    urls_checked: usize,
}

impl Default for Reporter {
    fn default() -> Self {
        let handler = GraphicalReportHandler::default().with_error_style();
        Self {
            handler,
            keys_found: 0,
            urls_checked: 0,
        }
    }
}

impl Reporter {
    pub fn report_keys(&mut self, keys: Vec<ApiKeyError>) -> Result<()> {
        self.keys_found += keys.len();
        self.handler.report_keys(keys)
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
