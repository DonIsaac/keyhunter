extern crate pretty_env_logger;
#[macro_use]
extern crate log;

mod cli;
mod runner;

use miette::{GraphicalTheme, IntoDiagnostic, Result};
use std::{process::ExitCode, sync::Arc, thread};

use clap::Parser;
use cli::Cli;
use keyhunter::Config;
use runner::Runner;

fn main() -> Result<ExitCode> {
    let cmd = Cli::parse();
    let mut builder = pretty_env_logger::formatted_timed_builder();

    miette::set_hook(Box::new(|_| {
        Box::new(
            miette::MietteHandlerOpts::new()
                .graphical_theme(GraphicalTheme::unicode())
                .terminal_links(true)
                .unicode(false)
                .context_lines(3)
                .width(120)
                .color(true)
                .with_cause_chain()
                .build(),
        )
    }))
    .unwrap();

    if let Ok(rust_log) = std::env::var("RUST_LOG") {
        builder.parse_filters(&rust_log);
    } else {
        builder.filter_module("keyhunter", cmd.log_level_filter());
    }
    builder.try_init().into_diagnostic().unwrap();

    let config = Config::from_default_gitleaks_config();

    let runner = Runner::new(Arc::new(config), cmd.max_args());
    let (key_receiver, handle) = runner.run(vec![cmd.entrypoint().clone()]);

    let recv_handle = thread::spawn(move || {
        while let Ok(Some(api_key)) = key_receiver.recv() {
            println!("{:?}", api_key);
        }
    });

    let errors = handle.join().unwrap();
    recv_handle.join().unwrap();
    if errors.is_empty() {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::FAILURE)
    }
}
