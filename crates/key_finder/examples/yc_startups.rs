use anyhow::{Result};
use csv;
use key_finder::{ApiKeyExtractor, Config, ScriptMessage, WebsiteWalker};
use rand::random;
use std::{
    fs, io::{self, BufReader, Read}, path::PathBuf, sync::{mpsc, Arc, RwLock}, thread, time::Duration
};

fn main() -> Result<()> {
    let yc_sites_raw = include_str!("../../../yc-companies.csv");
    let reader = csv::Reader::from_reader(yc_sites_raw.as_bytes());
    // let outfile = {
    //     let rand: u32 = random();
    //     let outfile_name = PathBuf::from(format!("api-keys-{rand}.csv"));
    //     let file = fs::File::options().write(true).append(false).open(outfile_name)?;
    //     Arc::new(RwLock::new(file))
    // };
    // // let config = Config::from_gitleaks_file("../gitleaks.toml")?;
    // let config = Config::default();
    // let extractor = ApiKeyExtractor::new(&config);
    // let js_agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(5)).build();

    reader
        .into_records()
        // .par_bridge()
        .filter(Result::is_ok)
        .map(Result::unwrap)
        .for_each(|record| {
            let name = &record[0];
            let url = (&record[1]).to_string();

            println!("Scraping keys for site {name}...");
            let (tx_scripts, rx_scripts) = mpsc::channel::<ScriptMessage>();
            let walker = WebsiteWalker::new(tx_scripts.clone());

            let moved_url = url.clone();
            let walk_handle = thread::spawn(move || {
                let result = walker.with_max_walks(60).walk(&moved_url);
                if result.is_err() {
                    println!("failed to create walker: {}", result.as_ref().unwrap_err());
                    tx_scripts.send(None).unwrap();
                }
                result
            });

            let rx_handle = thread::spawn(move || {
                while let Ok(Some(scripts)) = rx_scripts.recv() {
                    for script in scripts {
                        // agent.get()
                        // js_agent.get(url.as_str()).set("User-Agent", "Windows 10/ Edge browser: Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko)").call();
                        println!("Found new script: {script}");
                    }
                }
            });

            rx_handle.join().unwrap();
            let walk_result = walk_handle.join().unwrap();
            match walk_result {
                Ok(_) => {
                    println!("Done scraping for {name}");
                }
                Err(e) => {
                    println!("Failed to scrape for '{url}': {e}");
                }
            }
        });

    Ok(())
}
