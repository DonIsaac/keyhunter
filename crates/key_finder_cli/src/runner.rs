use log::trace;
use std::{
    sync::{mpsc, Arc},
    thread::{self, JoinHandle},
};

use key_finder::{
    ApiKeyCollector, ApiKeyMessage, ApiKeyReceiver, Config, ScriptMessage, WebsiteWalker,
};
use miette::{Context as _, Error, GraphicalReportHandler, IntoDiagnostic as _, Result};

#[derive(Debug)]
pub struct Runner {
    config: Arc<Config>,
    max_walks: usize,
    reporter: GraphicalReportHandler
}

impl Runner {
    pub fn new(config: Arc<Config>, max_walks: usize) -> Self {
        let reporter = GraphicalReportHandler::new();
        Self { config, max_walks, reporter }
    }

    pub fn run<U: IntoIterator<Item = String> + Send + 'static>(
        &self,
        urls: U,
    ) -> (ApiKeyReceiver, JoinHandle<()>) {
        let (key_sender, key_receiver) = mpsc::channel::<ApiKeyMessage>();
        let config = self.config.clone();
        let max_walks = self.max_walks;

        trace!("Starting runner thread");
        let handle = thread::spawn(move || {
            urls.into_iter().for_each(|url| {
                info!("Scraping keys for site {url}...");

                let (tx_scripts, rx_scripts) = mpsc::channel::<ScriptMessage>();
                let walker = WebsiteWalker::new(tx_scripts.clone());
                let collector =
                    ApiKeyCollector::new(config.clone(), rx_scripts, key_sender.clone());

                // Visit pages in the target site, sending found script urls over the
                // script channel
                let moved_url = url.clone();
                let walk_handle = thread::spawn(move || {
                    let result = walker.with_max_walks(max_walks).walk(&moved_url);
                    if let Err(ref err) = result {
                        // println!("failed to create walker: {}", err);
                        println!("{:?}", err);
                        tx_scripts
                            .send(None)
                            .into_diagnostic()
                            .context("Failed to send stop signal over script channel")
                            .unwrap();
                    }

                    result
                });

                let collector_handle = thread::spawn(move || collector.collect());
                Self::join(&url, collector_handle, walk_handle);
            });

            key_sender
                .send(None)
                .into_diagnostic()
                .context("Failed to close API key channel")
                .unwrap();

            info!("Scraping completed");
        });

        (key_receiver, handle)
    }

    fn join<S: AsRef<str>>(
        url: S,
        collector_handle: JoinHandle<()>,
        walk_handle: JoinHandle<Result<()>>,
    ) {
        let url = url.as_ref();

        collector_handle
            .join()
            .expect("ApiKeyCollector thread should have joined successfully");
        let walk_result = walk_handle
            .join()
            .expect("WebsiteWalker thread should have joined successfully");

        match walk_result {
            Ok(_) => {
                info!(target: "key_finder::main", "Done scraping for {url}");
            }
            Err(e) => {
                error!(target: "key_finder::main", "[run] Failed to scrape for '{url}': {e}");
            }
        }
    }
}