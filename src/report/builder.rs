use super::{
    reporters::{GraphicalReportHandler, SyncBufWriter},
    Reporter,
};
use std::io::Write;

#[derive(Debug, Default, Clone)]
pub struct ReporterBuilder {
    redacted: bool,
}

impl ReporterBuilder {
    #[inline]
    #[must_use]
    pub fn new(redacted: bool) -> Self {
        Self { redacted }
    }

    #[inline]
    #[must_use]
    pub fn with_redacted(mut self, yes: bool) -> Self {
        self.redacted = yes;
        self
    }

    pub fn graphical(&self) -> Reporter<GraphicalReportHandler> {
        Reporter::new(GraphicalReportHandler::new_stdout())
    }

    pub fn graphical_with_writer<W>(
        &self,
        writer: W,
    ) -> Reporter<GraphicalReportHandler<SyncBufWriter<W>>>
    where
        W: Write,
    {
        let handler = GraphicalReportHandler::new_buffered(writer).with_redacted(self.redacted);
        Reporter::new(handler)
    }
}
