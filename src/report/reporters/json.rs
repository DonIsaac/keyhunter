use std::io::{stdout, Stdout, Write};

use super::ReportHandler;
use crate::ApiKeyError;
use miette::{IntoDiagnostic, Result};
// use miette::JSONReportHandler
use serde::Serialize;
use serde_json::value::Serializer;

pub struct JsonReportHandler {
    writer: Stdout,
}
impl Default for JsonReportHandler {
    fn default() -> Self {
        Self { writer: stdout() }
    }
}

impl ReportHandler for JsonReportHandler {
    fn report_keys<'k, K>(&self, keys: K) -> Result<()>
    where
        K: IntoIterator<Item = &'k ApiKeyError>,
    {
        let mut w = self.writer.lock();
        for key in keys {
            self._report_key(&mut w, key)?;
        }
        Ok(())
    }

    fn report_key(&self, key: &ApiKeyError) -> Result<()> {
        let mut w = self.writer.lock();
        self._report_key(&mut w, key)
    }
}

impl JsonReportHandler {
    fn _report_key<W: Write>(&self, w: &mut W, key: &ApiKeyError) -> Result<()> {
        let json = key.serialize(Serializer).into_diagnostic()?;
        writeln!(w, "{json}").into_diagnostic()
    }
}
