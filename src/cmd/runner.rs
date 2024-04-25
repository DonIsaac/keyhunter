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
use log::trace;
use std::{
    sync::{mpsc, Arc},
    thread::{self, JoinHandle},
};

use keyhunter::{
    ApiKeyCollector, ApiKeyMessage, ApiKeyReceiver, Config, ScriptMessage, WebsiteWalker,
};
use miette::{Context as _, Error, IntoDiagnostic as _, Result};

#[derive(Debug)]
pub struct Runner {
    config: Arc<Config>,
    max_walks: usize,
    headers: Vec<(String, String)>,
    random_ua: bool,
}

impl Runner {
    pub fn new(
        config: Arc<Config>,
        max_walks: usize,
        headers: Vec<(String, String)>,
        random_ua: bool,
    ) -> Self {
        Self {
            config,
            max_walks,
            headers,
            random_ua,
        }
    }

    pub fn run<U: IntoIterator<Item = String> + Send + 'static>(
        &self,
        urls: U,
    ) -> (ApiKeyReceiver, JoinHandle<Vec<Error>>) {
        let (key_sender, key_receiver) = mpsc::channel::<ApiKeyMessage>();
        let config = self.config.clone();
        let max_walks = self.max_walks;
        let headers = Arc::new(self.headers.clone());
        let random_ua = self.random_ua;

        trace!("Starting runner thread");
        // let mut errors: Arc<RwLock<Vec<Error>>> = Default::default();

        let handle = thread::spawn(move || {
            let mut errors: Vec<Error> = Default::default();
            urls.into_iter().for_each(|url| {
                let url = if !url.contains("://") {
                    String::from("https://") + &url
                } else {
                    url
                };
                info!("Scraping keys for site '{url}'...");

                let (tx_scripts, rx_scripts) = mpsc::channel::<ScriptMessage>();
                let walker = WebsiteWalker::new(tx_scripts.clone()).with_random_ua(random_ua);
                let collector =
                    ApiKeyCollector::new(config.clone(), rx_scripts, key_sender.clone())
                        .with_random_ua(random_ua);

                // Visit pages in the target site, sending found script urls over the
                // script channel
                let moved_url = url.clone();
                let moved_headers = Arc::clone(&headers);
                let walk_handle = thread::spawn(move || {
                    let result = walker
                        .with_max_walks(max_walks)
                        .with_headers(moved_headers.iter().cloned())
                        .walk(&moved_url);
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
                if let Err(error) = Self::join(&url, collector_handle, walk_handle) {
                    errors.push(error)
                }
            });

            key_sender
                .send(ApiKeyMessage::Stop)
                .into_diagnostic()
                .context("Failed to close API key channel")
                .unwrap();

            debug!("Scraping completed");
            errors
        });

        (key_receiver, handle)
    }

    fn join<S: AsRef<str>>(
        url: S,
        collector_handle: JoinHandle<()>,
        walk_handle: JoinHandle<Result<()>>,
    ) -> Result<()> {
        let _url = url.as_ref();

        collector_handle
            .join()
            .expect("ApiKeyCollector thread should have joined successfully");

        // match walk_result {
        //     Ok(_) => {
        //         info!(target: "keyhunter::main", "Done scraping for {url}");
        //     }
        //     Err(e) => {
        //         error!(target: "keyhunter::main", "[run] Failed to scrape for '{url}': {e}");
        //     }
        // }
        walk_handle
            .join()
            .expect("WebsiteWalker thread should have joined successfully")
    }
}
