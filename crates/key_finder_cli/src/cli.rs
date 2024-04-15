use std::path::PathBuf;

use clap::{Parser, Args, ValueHint};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[arg(name = "url")]
    #[arg(value_hint = ValueHint::Url)]
    entrypoint: String,

    #[arg(long, short)]
    #[arg(value_hint = ValueHint::AnyPath)]
    output: Option<PathBuf>
}

impl Cli {
    pub fn entrypoint(&self) -> &String {
        &self.entrypoint
    }
}
