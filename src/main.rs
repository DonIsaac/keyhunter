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
use owo_colors::OwoColorize;
use std::{process::ExitCode, sync::Arc, thread};

use clap::Parser;
use cmd::{cli::Cli, runner::Runner};
use keyhunter::{
    report::{Reporter, ReporterBuilder},
    ApiKeyMessage, Config,
};

fn main() -> Result<ExitCode> {
    let cmd = Cli::parse();

    let mut builder = pretty_env_logger::formatted_timed_builder();

    miette::set_hook(Box::new(|_| {
        Box::new(
            miette::MietteHandlerOpts::new()
                .graphical_theme(GraphicalTheme::unicode())
                .terminal_links(true)
                .unicode(true)
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

    debug!("cli args: {:?}", cmd);

    let config = Config::gitleaks();

    let start = std::time::Instant::now();

    let reporter: Reporter<_> = ReporterBuilder::default()
        .with_redacted(cmd.is_redacted())
        .graphical();
    let reporter = Arc::new(reporter);
    let runner = Runner::new(
        Arc::new(config),
        cmd.max_args(),
        cmd.headers().into(),
        cmd.random_ua(),
    );
    let (key_receiver, handle) = runner.run(vec![cmd.entrypoint().clone()]);

    // Render reports for scraped credentials
    let moved_reporter = Arc::clone(&reporter);
    let recv_handle = thread::spawn(move || {
        let reporter = moved_reporter;
        while let Ok(message) = key_receiver.recv() {
            match message {
                ApiKeyMessage::Stop => break,
                ApiKeyMessage::Keys(api_keys) => {
                    reporter.report_keys(&api_keys).unwrap();
                }
                ApiKeyMessage::RecoverableFailure(err) => {
                    println!("{:?}", err);
                }
                ApiKeyMessage::DidScanScript => {
                    reporter.record_scripts_checked(1);
                }
                ApiKeyMessage::DidScrapePages(pages) => {
                    reporter.record_pages_crawled(pages);
                }
            }
            // println!("{:?}", api_key);
        }
    });

    let errors = handle.join().unwrap();
    recv_handle.join().unwrap();
    let end = std::time::Instant::now();
    let elapsed = (end - start).as_secs_f64();

    let num_scripts = reporter.scripts_checked();
    let num_keys = reporter.keys_found();
    let num_pages = reporter.pages_crawled();
    drop(reporter);

    println!(
        "Found {} {} across {} {} and {} {} in {:.2}{}",
        num_keys.yellow(),
        if num_keys == 1 { "key" } else { "keys" },
        num_scripts.yellow(),
        if num_scripts == 1 {
            "script"
        } else {
            "scripts"
        },
        num_pages.yellow(),
        if num_pages == 1 { "page" } else { "pages" },
        elapsed.cyan(),
        "s".cyan()
    );
    if errors.is_empty() {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::FAILURE)
    }
}
