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

use clap::{Parser, ValueHint};
use clap_verbosity_flag::Verbosity;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[arg(name = "url")]
    #[arg(value_hint = ValueHint::Url)]
    entrypoint: String,

    #[arg(long, short)]
    #[arg(value_hint = ValueHint::AnyPath)]
    output: Option<PathBuf>,

    #[command(flatten)]
    verbose: Verbosity,

    /// Maximum number of page links to crawl.
    ///
    /// Must be greater than 0.
    #[arg(long, short)]
    #[arg(default_value = "20")]
    max_walks: Option<usize>,
}

impl Cli {
    const DEFAULT_MAX_WALKS: usize = 20;

    pub fn entrypoint(&self) -> &String {
        &self.entrypoint
    }

    pub fn log_level_filter(&self) -> log::LevelFilter {
        self.verbose.log_level_filter()
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
}
