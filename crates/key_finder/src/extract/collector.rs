use dashmap::DashSet;
use log::{debug, error, trace};
use std::{
    sync::{mpsc, Arc},
    time::Duration,
};

use miette::{Context as _, IntoDiagnostic as _, Result};
use oxc::span::SourceType;
use rand::Rng;
use ureq::{Agent, AgentBuilder};
use url::Url;

use crate::{ApiKeyExtractor, Config, ScriptReceiver};

use super::visit::ApiKey;

const USER_AGENTS: [&str; 9] = [
    "Windows 10/ Edge browser: Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) ",
    "Chrome/42.0.2311.135 Safari/537.36 Edge/12.246",
    "Windows 7/ Chrome browser: Mozilla/5.0 (Windows NT 6.1; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) ",
    "Chrome/47.0.2526.111 Safari/537.36",
    "Mac OS X10/Safari browser: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_11_2) AppleWebKit/601.3.9 (KHTML, ",
    "like Gecko) Version/9.0.2 Safari/601.3.9",
    "Linux PC/Firefox browser: Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:15.0) Gecko/20100101 Firefox/15.0.1",
    "Chrome OS/Chrome browser: Mozilla/5.0 (X11; CrOS x86_64 8172.45.0) AppleWebKit/537.36 (KHTML, like Gecko) ",
    "Chrome/51.0.2704.64 Safari/537.36"
];

pub(super) fn random_ua<R: Rng>(rng: &mut R) -> &'static str {
    let idx = rng.gen_range(0..USER_AGENTS.len());
    USER_AGENTS[idx]
}

// pub type UrlReceiver = mpsc::Receiver<Option<Url>>;

pub type ApiKeyMessage = Option<(String, Vec<ApiKey>)>;
pub type ApiKeySender = mpsc::Sender<ApiKeyMessage>;
pub type ApiKeyReceiver = mpsc::Receiver<ApiKeyMessage>;

#[derive(Debug)]
pub struct ApiKeyCollector {
    config: Arc<Config>,
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

        // CDNs serving static JS dependencies
        skip_domains.insert("cdn.jsdelivr.net");
        skip_domains.insert("unpkg.com");

        // Analytics scripts
        skip_domains.insert("events.framer.com");

        let skip_paths: Vec<&'static str> = vec!["jquery", "react", "lodash"];

        Self {
            config,
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
        let api_keys = ApiKeyExtractor::new(Arc::clone(&self.config))
            .extract_api_keys(SourceType::default(), &script);

        if !api_keys.is_empty() {
            let num_keys = api_keys.len();
            self.sender
                .send(Some((url.to_string(), api_keys)))
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

        return false;
    }
}
