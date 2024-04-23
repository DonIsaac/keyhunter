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
extern crate pretty_env_logger;
#[macro_use]
extern crate log;

mod cmd;

use miette::{GraphicalTheme, IntoDiagnostic, Result};
use std::{process::ExitCode, sync::Arc, thread};

use clap::Parser;
use cmd::{cli::Cli, runner::Runner};
use keyhunter::{report::Reporter, ApiKeyMessage, Config};

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

    let config = Config::gitleaks();

    let mut reporter = Reporter::default().with_redacted(cmd.is_redacted());
    let runner = Runner::new(Arc::new(config), cmd.max_args());
    let (key_receiver, handle) = runner.run(vec![cmd.entrypoint().clone()]);

    let recv_handle = thread::spawn(move || {
        while let Ok(message) = key_receiver.recv() {
            match message {
                ApiKeyMessage::Stop => break,
                ApiKeyMessage::Keys(api_keys) => {
                    reporter.report_keys(&api_keys).unwrap();
                }
                ApiKeyMessage::RecoverableFailure(err) => {
                    println!("{:?}", err);
                }
            }
            // println!("{:?}", api_key);
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
