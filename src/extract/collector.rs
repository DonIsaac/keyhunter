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
use log::{debug, trace, warn};
use rustc_hash::FxBuildHasher;
use std::{
    sync::{mpsc, Arc},
    time::Duration,
};

use miette::{Context as _, Error, IntoDiagnostic as _, Result};
use oxc::allocator::Allocator;

use ureq::{Agent, AgentBuilder};
use url::Url;

use crate::{http::random_ua, ApiKeyExtractor, Config, ScriptMessage, ScriptReceiver};

use super::{error::DownloadScriptDiagnostic, ApiKeyError};

#[derive(Debug)]
pub enum ApiKeyMessage {
    Keys(Vec<ApiKeyError>),
    RecoverableFailure(Error),
    DidScanScript,
    DidScrapePages(usize),
    Stop,
}
impl From<Vec<ApiKeyError>> for ApiKeyMessage {
    fn from(keys: Vec<ApiKeyError>) -> Self {
        Self::Keys(keys)
    }
}

pub type ApiKeySender = mpsc::Sender<ApiKeyMessage>;
pub type ApiKeyReceiver = mpsc::Receiver<ApiKeyMessage>;

/// A service for collecting API keys from scripts.
///
/// The collector receives URLs of scripts over a [`ScriptReceiver`] channel and
/// sends all extracted API keys over a [`ApiKeySender`] channel. Keys are
/// extracted with [`ApiKeyExtractor`]
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
    ua: Option<&'static str>,

    /// Skip scripts originating from these domains
    skip_domains: DashSet<&'static str, FxBuildHasher>,

    /// Skip scripts that contain these substrings in their URL path, e.g.
    /// "jquery"
    skip_paths: Vec<&'static str>,

    /// Other headers to include in requests when downloading JS resources
    extra_headers: Vec<(String, String)>,
}

impl ApiKeyCollector {
    pub fn new(config: Arc<Config>, recv: ScriptReceiver, sender: ApiKeySender) -> Self {
        let agent = AgentBuilder::new().timeout(Duration::from_secs(10)).build();

        let skip_domains: DashSet<&'static str, FxBuildHasher> = Default::default();
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
            ua: None,
            skip_domains,
            skip_paths,
            extra_headers: vec![],
        }
    }

    /// Set the `User-Agent` header to a random, browser-like value.
    pub fn with_random_ua(mut self, yes: bool) -> Self {
        if yes && self.ua.is_none() {
            self.ua = Some(random_ua(&mut rand::thread_rng()));
        } else {
            self.ua = None;
        }

        self
    }

    /// Include this header in all requests
    pub fn with_headers<I>(mut self, headers: I) -> Self
    where
        I: IntoIterator<Item = (String, String)>,
    {
        self.extra_headers.extend(headers);
        self
    }

    /// Run the collector.
    ///
    /// This method is blocking and will run until [`None`] is sent over the
    /// script channel. It should be run in a separate thread to leave the main
    /// thread available for other tasks.
    pub fn collect(self) {
        while let Ok(msg) = self.receiver.recv() {
            match msg {
                ScriptMessage::Done => {
                    break;
                }
                ScriptMessage::DidWalkPage => {
                    self.send(ApiKeyMessage::DidScrapePages(1));
                }
                ScriptMessage::Scripts(urls) => {
                    // todo: parallelize
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
                            #[allow(unused_variables)]
                            Err(DownloadScriptDiagnostic::NotJavascript(url, ct)) => {
                                #[cfg(debug_assertions)]
                                warn!("({url}) Skipping non-JS script with content type {ct}");
                            }
                            Err(e) => {
                                let report = Error::from(e)
                                    .context(format!("Could not download script at {url}"));
                                warn!("{report:?}");
                            }
                        }
                    }
                }
            }
        }
        // tell sender we're done sending keys
        // debug!("No more keys to receive, sending stop signal");
        // let _ = self.sender.send(None);
    }

    /// Download a JS script from a URL.
    fn download_script(&self, url: &Url) -> Result<String, DownloadScriptDiagnostic> {
        let request = self.agent.get(url.as_str());

        let request = if let Some(ua) = self.ua {
            request.set("User-Agent", ua)
        } else {
            request
        };
        let request = self
            .extra_headers
            .iter()
            .fold(request, |req, (key, value)| req.set(key, value));

        let res = request.call()?;
        if !res.content_type().contains("javascript") {
            return Err(DownloadScriptDiagnostic::NotJavascript(
                url.to_string(),
                res.content_type().to_string(),
            ));
        }

        let js: String = res
            .into_string()
            .map_err(|e| DownloadScriptDiagnostic::CannotReadBody(url.to_string(), e))?;
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

        self.sender
            .send(ApiKeyMessage::DidScanScript)
            .into_diagnostic()
            .unwrap();

        // Report recoverable extraction failures
        let api_keys = match extract_result {
            Ok(api_keys) => api_keys,
            Err(e) => {
                self.send(ApiKeyMessage::RecoverableFailure(e));
                return;
            }
        };

        // convert into an ApiKeyError and send over channel
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

    /// Returns `true` if the resource at `url` should not be downloaded and
    /// checked for API keys.
    fn should_skip_url(&self, url: &Url) -> bool {
        // has an ignored domain
        if let Some(domain) = url.domain() {
            if self.skip_domains.contains(domain) {
                trace!("({url}) URL has an ignored domain, skipping");
                return true;
            }
        }

        // has an ignored path
        // TODO: use SIMD memmem? https://docs.rs/memchr/latest/memchr/
        for skip_path_pattern in &self.skip_paths {
            if url.path().contains(skip_path_pattern) {
                trace!(
                    "({url}) URL has a path matching ignored pattern {skip_path_pattern}, skipping"
                );
                return true;
            }
        }

        // needs checking
        false
    }

    fn send(&self, msg: ApiKeyMessage) {
        self.sender
            .send(msg)
            .into_diagnostic()
            .context("Failed to send message over API key channel")
            .unwrap();
    }
}
