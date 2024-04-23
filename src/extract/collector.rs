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
use dashmap::DashSet;
use log::{debug, error, trace};
use std::{
    sync::{mpsc, Arc},
    time::Duration,
};

use miette::{Context as _, Error, IntoDiagnostic as _, Result};
use oxc::allocator::Allocator;

use ureq::{Agent, AgentBuilder};
use url::Url;

use crate::{http::random_ua, ApiKeyExtractor, Config, ScriptReceiver};

use super::ApiKeyError;

#[derive(Debug)]
pub enum ApiKeyMessage {
    Keys(Vec<ApiKeyError>),
    RecoverableFailure(Error),
    Stop,
}
impl From<Vec<ApiKeyError>> for ApiKeyMessage {
    fn from(keys: Vec<ApiKeyError>) -> Self {
        Self::Keys(keys)
    }
}

pub type ApiKeySender = mpsc::Sender<ApiKeyMessage>;
pub type ApiKeyReceiver = mpsc::Receiver<ApiKeyMessage>;

/// Collects API keys from scripts.
///
/// The collector receives URLs of scripts over a [`ScriptReceiver`] channel and
/// sends all extracted API keys over a [`ApiKeySender`] channel.
#[derive(Debug)]
pub struct ApiKeyCollector {
    config: Arc<Config>,

    extractor: ApiKeyExtractor,

    /// Receives script URLs
    receiver: ScriptReceiver,

    /// Sends extracted API keys
    sender: ApiKeySender,

    /// HTTP agent for making requests
    agent: Agent,

    /// Random user agent (to make requests appear as originating from a browser)
    ua: &'static str,

    /// Skip scripts originating from these domains
    skip_domains: DashSet<&'static str>,

    /// Skip scripts that contain these strs in their URL path
    skip_paths: Vec<&'static str>,
}

impl ApiKeyCollector {
    pub fn new(config: Arc<Config>, recv: ScriptReceiver, sender: ApiKeySender) -> Self {
        let agent = AgentBuilder::new().timeout(Duration::from_secs(10)).build();
        let ua = random_ua(&mut rand::thread_rng());

        let skip_domains: DashSet<&'static str> = Default::default();
        // Google APIs, GTM, and analytics
        skip_domains.insert("ajax.googleapis.com");
        skip_domains.insert("apis.google.com");
        skip_domains.insert("youtube.com");
        skip_domains.insert("www.googletagmanager.com");
        skip_domains.insert("assets.calendly.com");

        // CDNs serving static JS dependencies
        skip_domains.insert("cdn.jsdelivr.net");
        skip_domains.insert("unpkg.com");

        // Analytics scripts
        skip_domains.insert("events.framer.com");

        let skip_paths: Vec<&'static str> = vec!["jquery", "react", "lodash", "unpkg"];

        let extractor = ApiKeyExtractor::new(Arc::clone(&config));
        Self {
            config,
            extractor,
            receiver: recv,
            sender,
            agent,
            ua,
            skip_domains,
            skip_paths,
        }
    }

    /// Run the collector.
    ///
    /// This method is blocking and will run until [`None`] is sent over the
    /// script channel. It should be run in a separate thread to leave the main
    /// thread available for other tasks.
    pub fn collect(self) {
        while let Ok(Some(urls)) = self.receiver.recv() {
            // todo: parallellize
            for url in urls {
                if self.should_skip_url(&url) {
                    continue;
                }

                debug!("({url}) checking for api keys...");
                let js = self.download_script(&url);
                match js {
                    Ok(js) => {
                        self.parse_and_send(url, &js);
                    }
                    Err(e) => {
                        let report = e.context(format!("Could not download script at {url}"));
                        error!("{report}");
                    }
                }
            }
        }
        // tell sender we're done sending keys
        // debug!("No more keys to receive, sending stop signal");
        // let _ = self.sender.send(None);
    }

    fn download_script(&self, url: &Url) -> Result<String> {
        let js = self
            .agent
            .get(url.as_str())
            .set("User-Agent", self.ua)
            .call()
            .into_diagnostic()?
            .into_string()
            .into_diagnostic()?;
        trace!("({url}) Downloaded script");
        Ok(js)
    }

    fn parse_and_send(&self, url: Url, script: &str) {
        trace!("({url}) Parsing script");
        let alloc = Allocator::default();
        let extract_result = self
            .extractor
            .extract_api_keys(&alloc, script)
            .with_context(|| format!("Failed to parse script at '{url}'"));
        let api_keys = match extract_result {
            Ok(api_keys) => api_keys,
            Err(e) => {
                self.sender
                    .send(ApiKeyMessage::RecoverableFailure(e))
                    .into_diagnostic()
                    .unwrap();
                return;
            }
        };

        if !api_keys.is_empty() {
            let num_keys = api_keys.len();
            let url_string = url.to_string();
            let api_keys = api_keys
                .into_iter()
                .map(|api_key| {
                    ApiKeyError::new(
                        api_key,
                        url_string.clone(),
                        script.to_string(),
                        &self.config,
                    )
                })
                .collect::<Vec<_>>();
            self.sender
                .send(ApiKeyMessage::Keys(api_keys))
                .into_diagnostic()
                .context(format!(
                    "Failed to send {} keys over channel: channel is closed",
                    num_keys
                ))
                .unwrap();
        }
    }

    fn should_skip_url(&self, url: &Url) -> bool {
        if let Some(domain) = url.domain() {
            if self.skip_domains.contains(domain) {
                trace!("URL {url} has an ignored domain, skipping");
                return true;
            }
        }

        for skip_path_pattern in &self.skip_paths {
            if url.path().contains(skip_path_pattern) {
                trace!(
                    "URL {url} has a path matching ignored pattern {skip_path_pattern}, skipping"
                );
                return true;
            }
        }

        false
    }
}
