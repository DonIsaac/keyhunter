extern crate log;
extern crate pretty_env_logger;

use keyhunter::{
    report::Reporter, ApiKeyCollector, ApiKeyError, ApiKeyMessage, Config, ScriptMessage,
    WebsiteWalkBuilder,
};
use log::{error, info};
use miette::{miette, Context as _, Error, IntoDiagnostic as _, Result};
use rand::random;
use std::{
    env,
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    sync::{mpsc, Arc, RwLock},
    thread,
    time::Duration,
};

type SyncReporter = Arc<RwLock<Reporter>>;

fn yc_path() -> Result<PathBuf> {
    let file_path = PathBuf::from(file!()).canonicalize().into_diagnostic()?;
    let root_dir = file_path
        .parent() // examples/
        .and_then(Path::parent) // keyhunter/
        // .and_then(Path::parent) // crates/
        // .and_then(Path::parent) // repo root
        .ok_or_else(|| miette!("Could not resolve repo root directory"))?;
    println!("{}", root_dir.display());
    let yc_companies = root_dir.join("tmp/yc-companies.csv");
    assert!(
        yc_companies.exists(),
        "YC Companies CSV not found. Did you run `make yc-companies.csv`? (path: {})",
        yc_companies.display()
    );
    assert!(
        yc_companies.is_file(),
        "YC Companies entry at {} is not a file.",
        yc_companies.display()
    );

    Ok(yc_companies)
}

fn yc_file() -> Result<String> {
    let yc_sites_path =
        yc_path().with_context(|| Error::msg("Could not find path to YC Companies CSV"))?;

    fs::read_to_string(yc_sites_path)
        .into_diagnostic()
        .context("Failed to open YC Companies CSV file")
}

/// Opens the CSV file where found api keys will be stored, creating it if it
/// doesn't exist and clearing existng contents.
///
/// Returns a buffered writer to this file.
fn outfile() -> Result<BufWriter<File>> {
    let rand: u32 = random();
    fs::create_dir_all("tmp").into_diagnostic()?;
    let outfile_name = PathBuf::from(format!("tmp/api-keys-{rand}.jsonl"));
    info!(target:"keyhunter::main", "API keys will be stored in {}", outfile_name.display());
    let file = File::options()
        .create(true)
        .truncate(true)
        .write(true)
        .append(false)
        .open(outfile_name)
        .into_diagnostic()?;

    let writer = BufWriter::new(file);
    Ok(writer)
}

/// Write any found API keys to the output file as a single-line JSON object
fn write_keys(output: &mut BufWriter<File>, api_key: ApiKeyError) -> Result<()> {
    let line = serde_json::to_string(&api_key).into_diagnostic()?;
    writeln!(output, "{}", line).into_diagnostic()
}

fn main() -> Result<()> {
    // Use RUST_LOG=keyhunter=info if RUST_LOG is not set
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "keyhunter=info");
    }
    pretty_env_logger::init();

    // Sets the maximum number of pages that will be visited for some entrypoint
    // URL. Higher values may cause a lot of wasted cycles as script/link
    // extraction finds fewer and fewer new unique values, while lower values
    // may leave a lot of stones unturned.
    let max_walks: usize = env::var("MAX_WALKS")
        .into_diagnostic()
        .and_then(|w| w.parse().into_diagnostic())
        .unwrap_or(30);
    assert!(
        max_walks > 0,
        "MAX_WALKS cannot be zero otherwise no pages will be checked!"
    );

    let config = Arc::new(Config::gitleaks());

    let reporter: SyncReporter = Arc::new(RwLock::new(Reporter::default().with_redacted(true)));

    let yc_sites_raw = yc_file().unwrap();
    let yc_reader = csv::Reader::from_reader(yc_sites_raw.as_bytes());
    let mut key_writer = outfile()?;

    let (key_sender, key_receiver) = mpsc::channel::<ApiKeyMessage>();

    // keys will come in here
    thread::spawn(move || {
        while let Ok(message) = key_receiver.recv() {
            match message {
                ApiKeyMessage::Keys(api_keys) => {
                    reporter.write().unwrap().report_keys(&api_keys).unwrap();
                    for api_key in api_keys {
                        let url = api_key.url.clone();
                        write_keys(&mut key_writer, api_key)
                            .context(format!("Failed to write api keys for script {}", &url))
                            .unwrap();
                    }
                    let _ = key_writer.flush();
                }
                ApiKeyMessage::RecoverableFailure(e) => {
                    println!("{:?}", e)
                }
                ApiKeyMessage::Stop => {
                    break;
                }
            }
        }
        let _ = key_writer.flush();
    });

    let walk_builder = WebsiteWalkBuilder::new()
        .with_max_walks(max_walks)
        .with_random_ua(true)
        .with_cookie_jar(true)
        .with_shared_cache(true)
        .with_close_channel(false)
        .with_timeout(Duration::from_secs(15))
        .with_timeout_connect(Duration::from_secs(2));
    yc_reader
        .into_records()
        // .par_bridge()
        .flatten()
        .for_each(|record| {
            let name = &record[0];
            let url = record[1].to_string();
            if name.eq_ignore_ascii_case("million") {
                return;
            }

            info!(target: "keyhunter::main", "Scraping keys for site {name}...");
            let (tx_scripts, rx_scripts) = mpsc::channel::<ScriptMessage>();
            let walker = walk_builder.build(tx_scripts.clone());
            let collector = ApiKeyCollector::new(config.clone(), rx_scripts, key_sender.clone());

            // Visit pages in the target site, sending found script urls over the
            // script channel
            let moved_url = url.clone();
            let walk_handle = thread::spawn(move || {
                let result = walker.walk(&moved_url);
                if result.is_err() {
                    error!(target: "keyhunter::main",
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
                    info!(target: "keyhunter::main", "Done scraping for {name}");
                }
                Err(e) => {
                    error!(target: "keyhunter::main", "[run] Failed to scrape for '{url}': {e}");
                }
            }
        });

    key_sender
        .send(ApiKeyMessage::Stop)
        .into_diagnostic()
        .context("Failed to close API key channel")
        .unwrap();

    info!("Scraping completed");

    Ok(())
}
