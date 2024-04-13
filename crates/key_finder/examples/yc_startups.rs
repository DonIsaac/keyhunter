use csv;
use key_finder::{
    ApiKey, ApiKeyCollector, ApiKeyExtractor, ApiKeyMessage, Config, ScriptMessage, WebsiteWalker,
};
use miette::{Context as _, Error, IntoDiagnostic as _, Result};
use rand::random;
use std::{
    fs::{self, File},
    io::{self, BufReader, BufWriter, Read, Write},
    path::PathBuf,
    sync::{mpsc, Arc, RwLock},
    thread,
    time::Duration,
};
use rayon::prelude::*;

fn yc_reader() -> csv::Reader<&'static [u8]> {
    let yc_sites_raw: &'static str = include_str!("../../../yc-companies-2.csv");
    csv::Reader::from_reader(yc_sites_raw.as_bytes())
}

/// Opens the CSV file where found api keys will be stored, creating it if it
/// doesn't exist and clearing existng contents.
///
/// Returns a buffered writer to this file.
fn outfile() -> Result<BufWriter<File>> {
    let rand: u32 = random();
    fs::create_dir_all("tmp").into_diagnostic()?;
    let outfile_name = PathBuf::from(format!("tmp/api-keys-{rand}.csv"));
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
    script_name: &String,
    api_keys: Vec<ApiKey>,
) -> Result<()> {
    println!(
        "[run] saving {} api keys from script '{}'",
        api_keys.len(),
        script_name
    );
    for key in api_keys {
        let ApiKey {
            span,
            rule_id,
            api_key,
        } = key;
        let start = span.start;
        let offset = span.size();
        writeln!(output, "{script_name},{rule_id},{api_key},{start},{offset}").into_diagnostic()?;
    }
    output.flush().into_diagnostic()?;
    Ok(())
}

fn main() -> Result<()> {
    let config = Arc::new(Config::default());

    let yc_reader = yc_reader();
    let mut key_writer = outfile()?;

    // Write CSV columns
    writeln!(key_writer, "Script Name,Rule,Key,Span Start,Span Offset").into_diagnostic()?;

    let (key_sender, key_receiver) = mpsc::channel::<ApiKeyMessage>();

    // keys will come in here
    thread::spawn(move || {
        while let Ok(Some((script_name, api_keys))) = key_receiver.recv() {
            write_keys(&mut key_writer, &script_name, api_keys)
                .context(format!("Failed to write api keys for script {script_name}"))
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
            let url = (&record[1]).to_string();

            println!("[run] Scraping keys for site {name}...");
            let (tx_scripts, rx_scripts) = mpsc::channel::<ScriptMessage>();
            let walker = WebsiteWalker::new(tx_scripts.clone());
            let collector = ApiKeyCollector::new(config.clone(), rx_scripts, key_sender.clone());

            // Visit pages in the target site, sending found script urls over the
            // script channel
            let moved_url = url.clone();
            let walk_handle = thread::spawn(move || {
                let result = walker.with_max_walks(10).walk(&moved_url);
                if result.is_err() {
                    println!(
                        "[run] failed to create walker: {}",
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
                    println!("[run] Done scraping for {name}");
                }
                Err(e) => {
                    println!("[run] Failed to scrape for '{url}': {e}");
                }
            }
        });

    Ok(())
}
