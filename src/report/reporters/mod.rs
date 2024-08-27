mod graphical;

use crate::ApiKeyError;
use miette::Result;

pub use graphical::GraphicalReportHandler;

pub trait ReportHandler {
    fn report_keys<'k, K>(&self, keys: K) -> Result<()>
    where
        K: IntoIterator<Item = &'k ApiKeyError>;

    fn report_key(&self, key: &ApiKeyError) -> Result<()> {
        self.report_keys([key])
    }
}
