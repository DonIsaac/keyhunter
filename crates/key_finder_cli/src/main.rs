extern crate pretty_env_logger;
#[macro_use]
extern crate log;

mod cli;
mod runner;

use log::trace;
use miette::{Error, Result, GraphicalReportHandler, GraphicalTheme};
use std::{sync::Arc, thread};

use clap::Parser;
use cli::Cli;
use key_finder::Config;
use runner::Runner;

fn main() {
    pretty_env_logger::init();

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
                // .with_syntax_highlighting(highlighter)
                .build(),
        )
    }))
    .unwrap();

    let config = Config::from_default_gitleaks_config();
    let cmd = Cli::parse();

    let runner = Runner::new(Arc::new(config), cmd.max_args());
    let (key_receiver, handle) = runner.run(vec![cmd.entrypoint().clone()]);

    let recv_handle = thread::spawn(move || {
        while let Ok(Some(api_key)) = key_receiver.recv() {
            println!("{:?}", api_key);
        }
    });

    handle.join().unwrap();
    recv_handle.join().unwrap();
}
