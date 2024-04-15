extern crate log;
extern crate pretty_env_logger;

use key_finder::{
    ApiKeyCollector, ApiKeyError, ApiKeyMessage, Config, ScriptMessage, WebsiteWalker,
};
use log::{error, info};
use miette::{Context as _, Error, IntoDiagnostic as _, Result};
use rand::random;
use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::PathBuf,
    sync::{mpsc, Arc},
    thread,
};

fn yc_reader() -> csv::Reader<&'static [u8]> {
    let yc_sites_raw: &'static str = include_str!("../../../yc-companies-2.csv");
    csv::Reader::from_reader(yc_sites_raw.as_bytes())
}

/// Opens the CSV file where found api keys will be stored, creating it if it
/// doesn't exist and clearing existng contents.
///
/// Returns a buffered writer to this file.
fn outfile() -> Result<BufWriter<File>> {
    pretty_env_logger::init();
    let rand: u32 = random();
    fs::create_dir_all("tmp").into_diagnostic()?;
    let outfile_name = PathBuf::from(format!("tmp/api-keys-{rand}.csv"));
    info!(target:"key_finder::main", "API keys will be stored in {}", outfile_name.display());
    let file = File::options()
        .create(true)
        .write(true)
        .append(false)
        .open(outfile_name)
        .into_diagnostic()?;

    let writer = BufWriter::new(file);
    Ok(writer)
}

/// Write any found API keys to a CSV
fn write_keys(
    output: &mut BufWriter<File>,
    // script_name: &String,
    api_key: ApiKeyError,
) -> Result<()> {
    println!("{}", Error::from(api_key));
    // warn!(
    //     target: "key_finder::main",
    //     "[run] saving api key from script '{}' - {:?}",
    //     &api_key.url,
    //     // script_name,
    //     &api_key.api_key
    //     // api_keys.iter().map(|k| &k.api_key).collect::<Vec<_>>()
    // );
    // for key in api_keys {
    //     let ApiKey {
    //         span,
    //         rule_id,
    //         api_key,
    //     } = key;
    //     let start = span.start;
    //     let offset = span.size();
    // }
    // writeln!(output, "{script_name},{rule_id},{api_key},{start},{offset}").into_diagnostic()?;
    output.flush().into_diagnostic()?;
    Ok(())
}

fn main() -> Result<()> {
    const MAX_WALKS: usize = 20;
    let config = Arc::new(Config::default());

    let yc_reader = yc_reader();
    let mut key_writer = outfile()?;

    // Write CSV columns
    writeln!(key_writer, "Script Name,Rule,Key,Span Start,Span Offset").into_diagnostic()?;

    let (key_sender, key_receiver) = mpsc::channel::<ApiKeyMessage>();

    // keys will come in here
    thread::spawn(move || {
        while let Ok(Some(api_key)) = key_receiver.recv() {
            let url = api_key.url.clone();
            write_keys(&mut key_writer, api_key)
                .context(format!("Failed to write api keys for script {}", &url))
                .unwrap();
        }
        let _ = key_writer.flush();
    });

    yc_reader
        .into_records()
        // .par_bridge()
        .filter(Result::is_ok)
        .map(Result::unwrap)
        .for_each(|record| {
            let name = &record[0];
            let url = record[1].to_string();

            info!(target: "key_finder::main", "Scraping keys for site {name}...");
            let (tx_scripts, rx_scripts) = mpsc::channel::<ScriptMessage>();
            let walker = WebsiteWalker::new(tx_scripts.clone());
            let collector = ApiKeyCollector::new(config.clone(), rx_scripts, key_sender.clone());

            // Visit pages in the target site, sending found script urls over the
            // script channel
            let moved_url = url.clone();
            let walk_handle = thread::spawn(move || {
                let result = walker.with_max_walks(MAX_WALKS).walk(&moved_url);
                if result.is_err() {
                    error!(target: "key_finder::main",
                        "failed to create walker: {}",
                        result.as_ref().unwrap_err()
                    );
                    tx_scripts
                        .send(None)
                        .into_diagnostic()
                        .context("Failed to send stop signal over script channel")
                        .unwrap();
                }
                result
            });

            // receive scripts from website walker. Download them parse them,
            // and look for API keys
            let collector_handle = thread::spawn(move || collector.collect());

            collector_handle
                .join()
                .expect("ApiKeyCollector thread should have joined successfully");
            let walk_result = walk_handle
                .join()
                .expect("WebsiteWalker thread should have joined successfully");
            match walk_result {
                Ok(_) => {
                    info!(target: "key_finder::main", "Done scraping for {name}");
                }
                Err(e) => {
                    error!(target: "key_finder::main", "[run] Failed to scrape for '{url}': {e}");
                }
            }
        });

    key_sender
        .send(None)
        .into_diagnostic()
        .context("Failed to close API key channel")
        .unwrap();

    info!("Scraping completed");

    Ok(())
}
