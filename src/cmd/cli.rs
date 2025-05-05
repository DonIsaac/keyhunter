/// Copyright © 2024 Don Isaac
///
/// This file is part of KeyHunter.
///
/// KeyHunter is free software: you can redistribute it and/or modify it
/// under the terms of the GNU General Public License as published by the Free
/// Software Foundation, either version 3 of the License, or (at your option)
/// any later version.
///
/// KeyHunter is distributed in the hope that it will be useful, but WITHOUT
/// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
/// FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for
/// more details.
///
/// You should have received a copy of the GNU General Public License along with
/// KeyHunter. If not, see <https://www.gnu.org/licenses/>.
use std::path::PathBuf;

use clap::{ArgAction, Parser, ValueHint};
use clap_verbosity_flag::Verbosity;
use miette::{self, IntoDiagnostic as _, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// The URL to start crawling from.
    ///
    /// You omit the protocol (e.g. `http://`, `https://`) and KeyHunter will
    /// automatically use `https://`.
    #[arg(name = "url")]
    #[arg(value_hint = ValueHint::Url)]
    entrypoint: String,

    /// Path to a file where the output will be written.
    ///
    /// Best used in combination with `--format json`.
    #[arg(long, short)]
    #[arg(value_hint = ValueHint::AnyPath)]
    output: Option<PathBuf>,

    #[arg(long, short)]
    #[arg(default_value = "default")]
    format: OutputFormat,

    #[command(flatten)]
    verbose: Verbosity,

    /// Redact secrets from output.
    ///
    /// Does nothing when output format is JSON.
    #[arg(long, short)]
    #[arg(default_value = "false")]
    redact: bool,

    /// Maximum number of page links to crawl.
    ///
    /// Must be greater than 0.
    #[arg(long, short)]
    #[arg(default_value = "20")]
    max_walks: Option<usize>,

    /// Do not use a random User-Agent header.
    ///
    /// By default, KeyHunter uses a random User-Agent header to make requests
    /// appear as if they are coming from a browser. However, some sites may have
    /// proxies that block requests from certain browser user agents. If this is
    /// happening in your case, use this flag to disable the random User-Agent.
    ///
    /// Note that setting a "User-Agent" header with -H, --header will override
    /// random User-Agent behavior.
    #[arg(long = "no-random-ua", short = 'U', action=ArgAction::SetFalse)]
    #[arg(default_value = "true")]
    random_ua: bool,
    /// Extra request header to add to each request.
    ///
    /// Affects requests when fetching new pages to visit and when fetching
    /// scripts to scrape.
    #[arg(long)]
    #[arg(short = 'H', value_parser = parse_header_and_value::<String, String>)]
    header: Vec<(String, String)>,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum OutputFormat {
    #[default]
    Default,
    Json,
}

impl OutputFormat {
    #[inline]
    pub fn is_default(self) -> bool {
        matches!(self, Self::Default)
    }
}

impl<S: AsRef<str>> From<S> for OutputFormat {
    fn from(value: S) -> Self {
        match value.as_ref() {
            "json" => Self::Json,
            _ => Self::Default,
        }
    }
}

impl Cli {
    const DEFAULT_MAX_WALKS: usize = 20;

    pub fn entrypoint(&self) -> &String {
        &self.entrypoint
    }

    pub fn log_level_filter(&self) -> log::LevelFilter {
        self.verbose.log_level_filter()
    }

    pub fn is_redacted(&self) -> bool {
        self.redact
    }

    pub fn max_args(&self) -> usize {
        if let Some(max_walks) = self.max_walks {
            if max_walks == 0 {
                panic!("max_walks arg must be greater than 0.");
            }
            max_walks
        } else {
            Self::DEFAULT_MAX_WALKS
        }
    }

    pub fn random_ua(&self) -> bool {
        self.random_ua
    }

    pub fn headers(&self) -> &[(String, String)] {
        self.header.as_slice()
    }
    pub fn format(&self) -> OutputFormat {
        self.format
    }
}

/// Parse a single key-value pair
fn parse_header_and_value<T, U>(s: &str) -> Result<(T, U)>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
    U: std::str::FromStr,
    U::Err: std::error::Error + Send + Sync + 'static,
{
    let pos = s.find(':').ok_or_else(|| {
        miette::miette!(
            code = "keyhunter::cli::invalid_header",
            "invalid Header:  value: no `:` found in `{s}`"
        )
    })?;

    Ok((
        s[..pos].trim().parse().into_diagnostic()?,
        s[pos + 1..].trim().parse().into_diagnostic()?,
    ))
}
