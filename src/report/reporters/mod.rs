mod graphical;
mod json;

use crate::ApiKeyError;
use miette::Result;

pub use graphical::GraphicalReportHandler;
pub use json::JsonReportHandler;

pub type SyncBufWriter<W> = std::sync::Mutex<std::io::BufWriter<W>>;

pub trait ReportHandler {
    // fn report_keys<'k, K>(&self, keys: K) -> Result<()>
    // where
    //     K: IntoIterator<Item = &'k ApiKeyError>,
    // {
    //     keys.into_iter().try_for_each(|key| self.report_key(key))
    // }
    fn report_keys(&self, mut keys: std::slice::Iter<'_, ApiKeyError>) -> Result<()> {
        keys.try_for_each(|key| self.report_key(key))
    }

    fn report_key(&self, key: &ApiKeyError) -> Result<()>;
}

impl<R> ReportHandler for Box<R>
where
    R: ReportHandler + ?Sized,
{
    fn report_key(&self, key: &ApiKeyError) -> Result<()> {
        (**self).report_key(key)
    }
    fn report_keys(&self, keys: std::slice::Iter<'_, ApiKeyError>) -> Result<()> {
        (**self).report_keys(keys)
    }
}
