use miette::{self, GraphicalTheme, IntoDiagnostic as _, Result};
use owo_colors::OwoColorize as _;
use std::{
    io::{self, stdout, Stdout, Write},
    str::from_utf8,
};

use crate::ApiKeyError;

#[derive(Debug)]
pub struct GraphicalReportHandler {
    writer: Stdout,
    theme: GraphicalTheme,
}

impl Default for GraphicalReportHandler {
    fn default() -> Self {
        Self {
            writer: stdout(),
            theme: Default::default(),
        }
    }
}
impl GraphicalReportHandler {
    pub fn with_error_style(mut self) -> Self {
        self.theme.styles.error = self.theme.styles.error.bright_red();
        self
    }
}
impl GraphicalReportHandler {
    const KEY_EMOJI: &'static str = "🔑";
    const INDENT: &'static str = "  ";
    const CHAR_HANG: &'static str = "   ";
    /// If the line in the source code containing the key is longer than this,
    /// then we treat it as a minified file and do not print the source code.
    const LINE_LEN_THRESHOLD: usize = 120;

    pub fn report_keys<K>(&self, keys: K) -> Result<()>
    where
        K: IntoIterator<Item = ApiKeyError>,
    {
        let mut lock = self.writer.lock();
        for key in keys {
            self.report_key(&mut lock, &key)?
        }

        Ok(())
    }

    pub fn report_key(&self, f: &mut impl Write, key: &ApiKeyError) -> Result<()> {
        self.render_header(f, &key).into_diagnostic()?;
        self.render_subheader(f, &key).into_diagnostic()?;
        if self.should_render_source(&key) {
            self.render_source(f, &key)?;
        }
        self.render_data_table(f, &key).into_diagnostic()?;
        self.render_footer(f, &key).into_diagnostic()?;
        writeln!(f).into_diagnostic()?;

        Ok(())
    }

    fn render_header(&self, f: &mut impl Write, key: &ApiKeyError) -> io::Result<()> {
        let style = self.theme.styles.error;
        write!(f, "{} ", Self::KEY_EMOJI)?;
        writeln!(
            f,
            "{}{} {}",
            key.rule_id.style(style),
            ":".style(style),
            key.description.style(style)
        )?;

        Ok(())
    }

    fn render_subheader(&self, f: &mut impl Write, key: &ApiKeyError) -> io::Result<()> {
        let styles = &self.theme.styles;
        let context = key.read_span(0, 0).unwrap();
        let line = context.line();
        let column = context.column();
        writeln!(
            f,
            "{}Found key \"{}\" in script {} at ({}:{}) ",
            Self::CHAR_HANG,
            key.api_key.style(styles.warning),
            key.url.style(styles.link),
            line,
            column
        )?;

        Ok(())
    }

    fn should_render_source(&self, key: &ApiKeyError) -> bool {
        let start_pos = key.source_span.offset();
        let source = key.source_code.inner().as_str();
        let bytes = source.as_bytes();

        let mut prev_newline = start_pos;
        let mut next_newline = start_pos + 1;
        debug_assert!(next_newline < bytes.len());

        while is_not_newline(bytes[prev_newline]) {
            if prev_newline == 0 {
                break;
            } else {
                prev_newline -= 1;
            }
        }

        while next_newline < bytes.len() && is_not_newline(bytes[next_newline]) {
            next_newline += 1;
        }

        let line_len = next_newline - prev_newline;
        line_len <= Self::LINE_LEN_THRESHOLD
    }

    fn render_source(&self, f: &mut impl Write, key: &ApiKeyError) -> Result<()> {
        let styles = &self.theme.styles;
        let Ok(contents) = key.read_span(1, 1) else {
            return Ok(());
        };
        let snippet = from_utf8(contents.data()).into_diagnostic()?;
        let mut line_num = contents.line();

        write!(f, "\n").into_diagnostic()?;

        for line in snippet.lines() {
            let pretty_line_num = format!("{}", line_num.style(styles.linum));
            writeln!(f, "{}{} {}", Self::INDENT, pretty_line_num, line).into_diagnostic()?;
            line_num += 1;
        }

        write!(f, "\n").into_diagnostic()?;

        Ok(())
    }

    fn render_data_table(&self, f: &mut impl Write, key: &ApiKeyError) -> io::Result<()> {
        // TODO
        Ok(())
    }

    fn render_footer(&self, f: &mut impl Write, key: &ApiKeyError) -> io::Result<()> {
        // TODO
        Ok(())
    }
}

const fn is_not_newline(c: u8) -> bool {
    !matches!(c as char, '\n')
}
