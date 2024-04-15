use std::path::PathBuf;

use clap::{Parser, ValueHint};

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
