use miette::{
    self,
    highlighters::{BlankHighlighter, Highlighter, SyntectHighlighter},
    GraphicalTheme, IntoDiagnostic as _, Result,
};
use owo_colors::{style, OwoColorize as _};
use std::{
    borrow::Cow,
    io::{self, stdout, Stdout, Write},
    str::from_utf8,
};

use crate::ApiKeyError;

pub struct GraphicalReportHandler {
    writer: Stdout,
    theme: GraphicalTheme,
    context_lines: u8,
    highlighter: Box<dyn Highlighter + Send + Sync>,
    redacted: bool,
}

impl Default for GraphicalReportHandler {
    fn default() -> Self {
        let context_lines = 3;
        let mut theme = GraphicalTheme::default();
        // not using colors
        if theme.styles.error == style() {
            Self {
                writer: stdout(),
                theme,
                context_lines,
                highlighter: Box::new(BlankHighlighter),
                redacted: false,
            }
        } else {
            theme.styles.error = theme.styles.error.bright_red();
            Self {
                writer: stdout(),
                theme,
                context_lines,
                highlighter: Box::<SyntectHighlighter>::default(),
                redacted: false,
            }
        }
    }
}

impl GraphicalReportHandler {
    const KEY_EMOJI: &'static str = "🔑";
    const INDENT: &'static str = "  ";
    const CHAR_HANG: &'static str = "   ";
    /// If the line in the source code containing the key is longer than this,
    /// then we treat it as a minified file and do not print the source code.
    const LINE_LEN_THRESHOLD: usize = 120;

    /// Redact API keys in rendered reports.
    #[must_use]
    pub fn with_redacted(mut self, yes: bool) -> Self {
        self.redacted = yes;
        self
    }

    pub fn report_keys<'k, K>(&self, keys: K) -> Result<()>
    where
        K: IntoIterator<Item = &'k ApiKeyError>,
    {
        let mut lock = self.writer.lock();
        for key in keys {
            self._report_key(&mut lock, key)?
        }

        Ok(())
    }

    pub fn report_key(&self, key: &ApiKeyError) -> Result<()> {
        let mut lock = self.writer.lock();
        self._report_key(&mut lock, key)
    }

    fn _report_key(&self, f: &mut impl Write, key: &ApiKeyError) -> Result<()> {
        self.render_header(f, key).into_diagnostic()?;
        self.render_subheader(f, key).into_diagnostic()?;
        if self.should_render_source(key) {
            self.render_source(f, key)?;
        }
        self.render_data_table(f, key).into_diagnostic()?;
        self.render_footer(f, key).into_diagnostic()?;
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
        let line = context.line() + 1;
        let column = context.column() + 1;
        let formatted_secret = self.format_secret(&key.secret);
        writeln!(
            f,
            "{}Found key \"{}\" in script {} at ({}:{}) ",
            Self::CHAR_HANG,
            formatted_secret.style(styles.warning),
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
        let Ok(contents) = key.read_span(self.context_lines as usize, self.context_lines as usize)
        else {
            return Ok(());
        };
        let snippet = from_utf8(contents.data()).into_diagnostic()?;
        let mut line_num = contents.line();

        writeln!(f).into_diagnostic()?;

        let mut highlighter_state = self.highlighter.start_highlighter_state(contents.as_ref());
        for line in snippet.lines() {
            let pretty_line_num = format!("{}", line_num.style(styles.linum));
            let line_num_padding = pretty_line_num.len();
            let highlighted_lines = highlighter_state.highlight_line(line);

            for (i, styled_line) in highlighted_lines.into_iter().enumerate() {
                if i == 0 {
                    writeln!(f, "{}{} {}", Self::INDENT, pretty_line_num, styled_line)
                        .into_diagnostic()?;
                } else {
                    writeln!(
                        f,
                        "{}{} {}",
                        Self::INDENT,
                        " ".repeat(line_num_padding),
                        styled_line
                    )
                    .into_diagnostic()?;
                }
            }
            line_num += 1;
        }

        Ok(())
    }

    fn render_data_table(&self, f: &mut impl Write, key: &ApiKeyError) -> io::Result<()> {
        let contents = key.read_span(0, 0).unwrap();
        let line = contents.line() + 1;
        let column = contents.column() + 1;
        let formatted_secret = self.format_secret(&key.secret);
        let key_name = key
            .key_name
            .as_ref()
            .map(std::borrow::Cow::from)
            .unwrap_or("<None>".into());

        writeln!(f)?;
        writeln!(f, "{}Rule ID:      {}", Self::CHAR_HANG, &key.rule_id)?;
        writeln!(f, "{}Script URL:   {}", Self::CHAR_HANG, &key.url)?;
        writeln!(f, "{}API Key Name: {}", Self::CHAR_HANG, key_name)?;
        writeln!(f, "{}Secret:       {}", Self::CHAR_HANG, &formatted_secret)?;
        writeln!(f, "{}Line:         {}", Self::CHAR_HANG, line)?;
        writeln!(f, "{}Column:       {}", Self::CHAR_HANG, column)?;
        Ok(())
    }

    fn render_footer(&self, _f: &mut impl Write, _key: &ApiKeyError) -> io::Result<()> {
        // TODO
        Ok(())
    }

    fn format_secret<'s>(&self, secret: &'s str) -> Cow<'s, str> {
        const SHOW_AMOUT: usize = 4;
        if self.redacted {
            let l = secret.len();
            if l <= SHOW_AMOUT {
                return Cow::Owned("•".repeat(l));
            } else {
                let (show, hide) = secret.split_at(SHOW_AMOUT);
                let hidden = "•".repeat(hide.len());
                return Cow::Owned(format!("{}{}", show, hidden));
            }
        } else {
            Cow::Borrowed(secret)
        }
    }
}

const fn is_not_newline(c: u8) -> bool {
    !matches!(c as char, '\n')
}
