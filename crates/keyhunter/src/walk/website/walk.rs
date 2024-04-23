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
use miette::{Context as _, Error, IntoDiagnostic as _, Result};

use std::{
    borrow::{Borrow, Cow},
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        mpsc, OnceLock,
    },
    time::Duration,
};

use ureq::{Agent, AgentBuilder};
use url::Url;

use super::{
    dom_walker::DomWalker,
    error::{NoContentDiagnostic, NotHtmlDiagnostic},
    url_visitor::UrlVisitor,
};
use crate::http::random_ua;

pub type ScriptMessage = Option<Vec<Url>>;
pub type ScriptSender = mpsc::Sender<ScriptMessage>;
pub type ScriptReceiver = mpsc::Receiver<ScriptMessage>;

#[derive(Debug)]
pub struct WebsiteWalker {
    /// Found URLs of JS scripts are sent over this channel
    sender: mpsc::Sender<ScriptMessage>,
    /// ureq agent for making HTTP requests
    agent: Agent,
    /// Random user agent to make us look like a browser
    ua: &'static str,

    /// Domains that can be visited (and have their scripts extracted)
    domain_whitelist: Vec<String>,
    /// Base url of the path where the walk started. Used to resolve relative URLs.
    base_url: OnceLock<Url>,

    /// Number of page visits currently in progress. When this reaches `0`, the
    /// walk is over
    in_progress: AtomicU64,
    /// Number of pages visited/walked
    walks_performed: AtomicUsize,
    /// Max # of walks that can be performed
    max_walks: Option<usize>,
    /// Web pages already visited. Prevents cycles.
    seen_urls: DashSet<Url>,
    seen_scripts: DashSet<Url>,

    /// Set to `true` when any ^ stop condition is reached to prevent further
    /// page loads
    done: AtomicBool,
    /// When `true`, [`None`] will be sent over the script channel to close it.
    ///
    /// Default `true`
    close_channel_when_done: bool,
}

impl WebsiteWalker {
    #[must_use]
    pub fn new_with_receiver() -> (Self, ScriptReceiver) {
        let (sender, receiver) = mpsc::channel();
        (Self::new(sender), receiver)
    }

    #[must_use]
    pub fn new(sender: ScriptSender) -> Self {
        const TIMEOUT: u64 = 10;

        let agent = AgentBuilder::new()
            .timeout_connect(Duration::from_secs(2))
            .timeout_read(Duration::from_secs(TIMEOUT))
            .timeout_write(Duration::from_secs(TIMEOUT))
            .build();

        let mut rng = rand::thread_rng();
        let ua = random_ua(&mut rng);

        Self {
            agent,
            ua,
            sender,
            in_progress: 0.into(),
            domain_whitelist: vec![],
            walks_performed: 0.into(),
            max_walks: None,
            done: false.into(), // domain_blacklist: None
            base_url: Default::default(),
            seen_urls: Default::default(),
            seen_scripts: Default::default(),
            close_channel_when_done: true,
        }
    }

    pub fn sender(&self) -> &ScriptSender {
        &self.sender
    }

    #[must_use]
    pub fn with_max_walks(mut self, max_walks: usize) -> Self {
        self.max_walks = Some(max_walks);
        self
    }

    #[must_use]
    pub fn unlimited_depth(mut self) -> Self {
        self.max_walks = None;
        self
    }

    #[must_use]
    pub fn whitelist_domain<S: Into<String>>(mut self, domain: S) -> Self {
        self.domain_whitelist.push(domain.into());
        self
    }

    #[must_use]
    pub fn with_close_channel(mut self, yes: bool) -> Self {
        self.close_channel_when_done = yes;
        self
    }

    pub fn walk(mut self, url: &str) -> Result<()> {
        let url = url.trim().trim_end_matches('/');
        let parsed = Url::parse(url)
            .into_diagnostic()
            .context(format!("Failed to start walk at {url}"))?;

        let domain = parsed
            .domain()
            .ok_or(Error::msg("Cannot start walk: url is invalid"))?;
        self.domain_whitelist.push(domain.to_string());

        let mut base_url = parsed.clone();
        base_url.set_path("");
        self.base_url.set(base_url).unwrap();

        self.domain_whitelist.sort_unstable();
        self.domain_whitelist.dedup();
        self.domain_whitelist.shrink_to_fit();

        debug!("Starting walk over '{parsed}'");
        // returns Err if entry url is not reachable, not html, etc.
        self.visit(parsed)
    }

    fn visit(&self, url: Url) -> Result<(), Error> {
        if self.done.load(Ordering::Relaxed) {
            return Ok(());
        }

        if self.has_visited_url(&url) {
            trace!("skipping '{url}', already visited");
            return Ok(());
        }

        debug!("visiting '{url}'");

        self.in_progress.fetch_add(1, Ordering::Relaxed);

        let err = format!("Failed to walk webpage {url}");
        let result = self.walk_rec(url).context(err);

        let walks_remaining = self.in_progress.fetch_sub(1, Ordering::Relaxed);
        let walks_performed = self.walks_performed.fetch_add(1, Ordering::Relaxed);

        if walks_remaining == 0 {
            debug!("stopping: No more walks are in progress");
            self.finish();
            return result;
        }

        if let Some(max_walks) = self.max_walks {
            if walks_performed > max_walks {
                debug!("stopping: maximum number of walks reached");
                self.finish()
            } else {
                trace!("{walks_performed}/{max_walks} walks performed")
            }
        }

        result
    }

    fn walk_rec(&self, url: Url) -> Result<(), Error> {
        let entrypoint = self
            .get_webpage(url.as_str())
            .context("Failed to fetch webpage")?;
        trace!("Building DOM walker for '{url}'");
        let dom_walker = DomWalker::new(&entrypoint).context("Failed to parse HTML")?;

        trace!("Extracting links and scripts for '{url}'");
        {
            let mut script_visitor = UrlVisitor::new("script", "src");
            dom_walker.walk(&mut script_visitor);
            self.send_scripts(script_visitor);
        }
        let links = {
            let mut link_visitor = UrlVisitor::new("a", "href");
            dom_walker.walk(&mut link_visitor);
            let links = link_visitor.into_inner();
            links
                .into_iter()
                .filter_map(|link| self.is_allowed_link(link))
                .collect::<Vec<_>>()
        };

        links.into_iter().for_each(|link| {
            let r = self.visit(link);
            if let Err(e) = r {
                let report = miette::miette!(e);
                warn!("{report}");
            }
        });

        Ok(())
    }

    fn get_webpage(&self, url: &str) -> Result<String> {
        trace!("getting webpage for '{url}'");
        let response = self
            .agent
            .get(url)
            .set("User-Agent", self.ua)
            .set(
                "Accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8",
            )
            .set("Keep-Alive", "timeout=5, max=100")
            .set("DNT", "1")
            .call()
            .into_diagnostic()?;

        // Check that we got HTML back
        if let Some(content_type) = response.header("Content-Type") {
            if !content_type.contains("html") {
                return NotHtmlDiagnostic::new(url, content_type).into();
            }
        }

        // Check that response was not empty
        if let Some(content_length) = response.header("Content-Length") {
            if let Ok(content_len) = content_length.parse::<usize>() {
                if content_len == 0 {
                    return NoContentDiagnostic::new(url).into();
                }
            }
        }
        let webpage = response.into_string().into_diagnostic()?;
        trace!("got webpage for '{url}'");
        Ok(webpage)
    }

    fn send_scripts(&self, script_visitor: UrlVisitor) {
        let base_url = self.base_url.get().unwrap();

        let scripts = script_visitor
            .into_iter()
            // TODO: resolve with base url
            .filter_map(|script| base_url.join(&script).ok())
            // filter out scripts that have already been sent
            .filter_map(|script| {
                if self.seen_scripts.contains(&script) {
                    None
                } else {
                    self.seen_scripts.insert(script.clone());
                    Some(script)
                }
            })
            .collect();

        self.sender
            .send(Some(scripts))
            .into_diagnostic()
            .context("Failed to send scripts over the channel")
            .unwrap();
    }

    fn is_allowed_link(&self, link: String) -> Option<Url> {
        const BANNED_EXTENSIONS: [&str; 3] = [".pdf", ".png", ".jpg"];
        let link = link.trim();
        if link.is_empty() || link.starts_with('#') {
            return None;
        }
        if link.starts_with("mailto:") || link.starts_with("javascript:") {
            return None;
        }

        let resolved = if link.starts_with('/') || !link.contains("://") {
            self.base_url.get().unwrap().join(link)
        } else {
            Url::parse(link)
        };
        resolved.ok().and_then(|link| {
            if BANNED_EXTENSIONS
                .iter()
                .any(|ext| link.path().ends_with(ext))
            {
                return None;
            }

            let is_whitelisted = link
                .domain()
                .is_some_and(|domain| self.is_allowed_domain(domain));

            if is_whitelisted {
                Some(link)
            } else {
                None
            }
        })
    }

    // pub fn resolve_maybe_relative(&self, link: String) -> Result<String, Error> {
    //     if link.starts_with('/') || !link.contains("://") {
    //         let resolved = self.base_url.get().unwrap().join(&link);
    //         Ok(resolved)
    //     } else {
    //         Ok(link)
    //     }
    // }

    fn is_allowed_domain(&self, domain: &str) -> bool {
        self.domain_whitelist.iter().any(|d| d.as_str() == domain)
    }

    fn has_visited_url(&self, url: &Url) -> bool {
        debug_assert!(
            !url.cannot_be_a_base(),
            "skip_if_visited got a relative url"
        ); // should be absolute

        if url.query().is_none() {
            return self.has_visited_url_clean(url);
        }

        let mut without_query_params = url.clone();
        without_query_params.set_query(None);
        let mut new_params: Vec<(Cow<'_, str>, Cow<'_, str>)> = vec![];
        for (key, value) in url.query_pairs() {
            if matches!(
                key.borrow(),
                "tab" | "tabid" | "tab_id" | "tab-id" | "id" | "page" | "page_id" | "page-id"
            ) {
                new_params.push((key, value))
            }
        }

        if new_params.is_empty() {
            self.has_visited_url_clean(&without_query_params)
        } else {
            let query = new_params
                .into_iter()
                .fold(String::new(), |acc, (key, value)| {
                    acc + format!("{key}={value}").as_str()
                });
            without_query_params.set_query(Some(query.as_str()));
            self.has_visited_url_clean(&without_query_params)
            // retur
        }
    }
    fn has_visited_url_clean(&self, url: &Url) -> bool {
        if self.seen_urls.contains(url) {
            true
        } else {
            self.seen_urls.insert(url.clone());
            false
        }
    }
    fn finish(&self) {
        debug!("({}) finishing walk", self.base_url.get().unwrap());

        if !self.close_channel_when_done {
            return;
        }

        let already_done = self.done.swap(true, Ordering::Relaxed);
        if !already_done {
            let _ = self.sender.send(None);
        }
    }
}

#[cfg(test)]
mod test {
    use super::WebsiteWalker;
    use std::thread::spawn;

    #[test]
    fn test_yc() {
        const URL: &str = "https://news.ycombinator.com/";
        let (walker, rx) = WebsiteWalker::new_with_receiver();

        let handle = spawn(move || walker.with_max_walks(20).walk(URL));

        let rx_handle = spawn(move || {
            while let Ok(Some(scripts)) = rx.recv() {
                let _stdlock = std::io::stdout().lock();
                for script in scripts {
                    println!("found script:\t{script}");
                }
                // drop(stdlock)
            }
        });

        handle.join().unwrap().unwrap();
        rx_handle.join().unwrap();
    }
}
