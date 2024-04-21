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

// pub type UrlReceiver = mpsc::Receiver<Option<Url>>;

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
        Ok(js)
    }

    fn parse_and_send<'a>(&self, url: Url, script: &'a str) {
        let alloc = Allocator::default();
        let api_keys = match self.extractor.extract_api_keys(&alloc, script) {
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
            // for error in errors {
            //     self.sender
            //         .send(error)
            //         .into_diagnostic()
            //         .context(format!(
            //             "Failed to send {} keys over channel: channel is closed",
            //             num_keys
            //         ))
            //         .unwrap();
            // }
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
