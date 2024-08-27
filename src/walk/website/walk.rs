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
use log::{debug, trace, warn};
use miette::{Context as _, Error, IntoDiagnostic as _, Result};

use std::{
    borrow::{Borrow, Cow},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc, Mutex, OnceLock,
    },
};

use rayon::prelude::*;
use ureq::Agent;
use url::Url;

use super::{
    dom_walker::DomWalker,
    error::{NoContentDiagnostic, NotHtmlDiagnostic},
    url_extractor::UrlExtractor,
    walk_cache::WalkCache,
    WebsiteWalkBuilder,
};
use crate::walk::website::error::WalkFailedDiagnostic;

pub type ScriptSender = mpsc::Sender<ScriptMessage>;
pub type ScriptReceiver = mpsc::Receiver<ScriptMessage>;

// TODO: use Arc for embedded page urls
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Script {
    /// A JS script fetchable at some URL
    Url(Url),
    /// JS embedded in a `<script>` tag within HTML
    Embedded(/* source code */ String, /* page url */ Url),
}

#[derive(Debug, Clone)]
pub enum ScriptMessage {
    Scripts(Vec<Script>),
    DidWalkPage,
    Done,
}
impl IntoIterator for ScriptMessage {
    type Item = Script;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            ScriptMessage::Scripts(scripts) => scripts.into_iter(),
            ScriptMessage::Done | ScriptMessage::DidWalkPage => vec![].into_iter(),
        }
    }
}

#[derive(Debug)]
pub struct WebsiteWalker {
    /// Found URLs of JS scripts are sent over this channel
    sender: mpsc::Sender<ScriptMessage>,
    /// ureq agent for making HTTP requests
    agent: Agent,
    /// Random user agent to make us look like a browser
    // ua: Option<&'static str>,
    headers: Vec<(String, String)>,

    /// Domains that can be visited (and have their scripts extracted)
    domain_whitelist: Vec<String>,
    /// Base url of the path where the walk started. Used to resolve relative URLs.
    base_url: OnceLock<Url>,

    /// Number of page visits currently in progress. When this reaches `0`, the
    /// walk is over
    in_progress: Mutex<usize>,
    /// Number of pages visited/walked
    walks_performed: AtomicUsize,
    /// Max # of walks that can be performed
    max_walks: Option<usize>,
    /// Web pages and scripts already visited. Prevents cycles and duplicate checks.
    cache: WalkCache,

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
    pub fn new(builder: &WebsiteWalkBuilder, sender: ScriptSender) -> Self {
        let agent = builder.build_agent();
        let headers = builder
            .headers()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();

        Self {
            agent,
            headers,
            sender,
            in_progress: 0.into(),
            domain_whitelist: builder.domain_whitelist.clone(),
            walks_performed: 0.into(),
            max_walks: builder.max_walks,
            done: false.into(),
            base_url: Default::default(),
            cache: builder.cache.clone().unwrap_or_default(),
            close_channel_when_done: builder.close_channel_when_done,
        }
    }

    pub fn sender(&self) -> &ScriptSender {
        &self.sender
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

        debug!("({parsed}) Starting walk ");
        // returns Err if entry url is not reachable, not html, etc.
        self.visit_many(vec![parsed])
    }

    fn visit_many(&mut self, urls: Vec<Url>) -> Result<(), Error> {
        let pages_to_visit = self.reserve_walk_count(urls.len());
        if pages_to_visit == 0 && *self.in_progress.get_mut().unwrap() == 0 {
            debug!("Finishing walk, No more pages to visit");
            self.finish();
            return Ok(());
        }

        match urls.len() {
            0 => Ok(()),
            1 => {
                let url = &urls[0];
                if self.has_visited_url(url) {
                    return Ok(());
                }
                let webpage = self.get_webpage(url.as_str())?;
                let result = self.walk_rec(url, &webpage);
                self.on_visit_end(1);
                result
            }
            num_urls => {
                let urls_and_webpages = urls
                    .into_iter()
                    .filter(|url| self.is_whitelisted_link(url) && !self.has_visited_url(url))
                    .take(pages_to_visit)
                    .par_bridge()
                    .map(|url| {
                        self.get_webpage(url.as_str())
                            .map(|webpage| (url, webpage))
                            .map_err(|e| {
                                warn!("{e:?}");
                                e
                            })
                    })
                    .filter_map(Result::ok)
                    .collect::<Vec<_>>();

                if urls_and_webpages.is_empty() {
                    return Ok(());
                }

                let num_walked = urls_and_webpages.len();
                let walk_success_count: u32 = urls_and_webpages
                    .into_iter()
                    .map(|(url, webpage)| {
                        debug!("({url}) walking page");
                        self.walk_rec(&url, &webpage)
                    })
                    .map(|result| {
                        if let Err(e) = result {
                            warn!("{e:?}");
                            0
                        } else {
                            1
                        }
                    })
                    .sum();

                // FIXME: should we record visits only for successful walks or
                // for all walks?
                self.on_visit_end(num_walked);

                if walk_success_count == 0 {
                    Err(miette::miette!("Failed to visit all {} urls", num_urls))
                } else {
                    Ok(())
                }
            }
        }
    }

    fn on_visit_end(&mut self, num_visits: usize) {
        let in_progress = self.in_progress.get_mut().unwrap();
        let previously_in_progress = *in_progress;
        let walks_remaining = previously_in_progress - num_visits;
        *in_progress = walks_remaining;
        let walks_performed = self
            .walks_performed
            .fetch_add(num_visits, Ordering::Relaxed);

        if walks_remaining == 0 {
            debug!("stopping: No more walks are in progress");
            self.finish();
            return;
        }

        if let Some(max_walks) = self.max_walks {
            if walks_performed > max_walks {
                debug!("stopping: maximum number of walks reached");
                self.finish()
            } else {
                trace!("{walks_performed}/{max_walks} walks performed")
            }
        }
    }

    fn walk_rec(&mut self, url: &Url, webpage: &str) -> Result<(), Error> {
        trace!("Building DOM walker for '{url}'");
        let dom_walker = DomWalker::new(webpage).context("Failed to parse HTML")?;

        trace!("Extracting links and scripts for '{url}'");
        let mut url_visitor = UrlExtractor::new(self.base_url.get().unwrap(), url);
        dom_walker.walk(&mut url_visitor);
        let (pages, scripts) = url_visitor.into_inner();

        self.send(ScriptMessage::DidWalkPage);
        self.send_scripts(scripts);

        self.visit_many(pages)
    }

    fn get_webpage(&self, url: &str) -> Result<String> {
        trace!("getting webpage for '{url}'");

        let req = self
            .headers
            .iter()
            .fold(self.agent.get(url), |req, (key, value)| {
                // trace!("Adding extra header {key}: {value}");
                req.set(key, value)
            });
        let response = req
            .call()
            .map_err(|e| WalkFailedDiagnostic::new(url.to_string(), e))
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

    fn send_scripts(&self, scripts: Vec<Script>) {
        let base_url = self.base_url.get().unwrap();

        let scripts = scripts
            .into_iter()
            // filter out scripts that have already been sent
            .filter_map(|script| match script {
                Script::Url(script) => {
                    if self.cache.has_seen_script(&script) {
                        trace!("({script}) not sending script - already seen");
                        None
                    } else {
                        self.cache.see_script(script.clone());
                        Some(Script::Url(script))
                    }
                }
                embed => Some(embed),
            })
            .collect::<Vec<_>>();
        trace!("({}) Sending {} new scripts", base_url, scripts.len());

        self.send(ScriptMessage::Scripts(scripts));
    }

    fn send(&self, message: ScriptMessage) {
        self.sender
            .send(message)
            .into_diagnostic()
            .context("Failed to send message over the scripts channel")
            .unwrap();
    }

    fn is_whitelisted_link(&self, link: &Url) -> bool {
        link.domain()
            .is_some_and(|domain| self.is_allowed_domain(domain))
    }

    fn is_allowed_domain(&self, domain: &str) -> bool {
        self.domain_whitelist.iter().any(|d| d.as_str() == domain)
    }

    fn has_visited_url(&self, url: &Url) -> bool {
        debug_assert!(
            !url.cannot_be_a_base(),
            "skip_if_visited got a relative url"
        ); // should be absolute

        if url.query().is_none() && url.fragment().is_none() {
            return self.has_visited_url_clean(url);
        }

        // remove #section hash and (most) query parameters from URL since they
        // don't affect what page the URL points to. Note that some applications
        // use query parameters to identify what page to go to, thus the below
        // query_pairs() check. We may need to update this list as new cases are
        // brought to light.
        let mut without_query_params = url.clone();
        without_query_params.set_query(None);
        without_query_params.set_fragment(None);
        let mut new_params: Vec<(Cow<'_, str>, Cow<'_, str>)> = vec![];
        for (key, value) in url.query_pairs() {
            // TODO: use phf?
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
        }
    }
    fn has_visited_url_clean(&self, url: &Url) -> bool {
        if self.cache.has_seen_url(url) {
            true
        } else {
            self.cache.see_url(url.clone());
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
            let _ = self.sender.send(ScriptMessage::Done);
        }
    }

    fn reserve_walk_count(&mut self, walks_desired: usize) -> usize {
        if self.is_done() {
            return 0;
        };

        let in_progress = self.in_progress.get_mut().unwrap();
        let Some(max_walks) = self.max_walks else {
            *in_progress += walks_desired;
            return walks_desired;
        };
        let walks_performed = self.walks_performed.fetch_add(0, Ordering::Relaxed);

        // # of walks we've done & are doing, but not ones we want to stat
        let total_walks = *in_progress + walks_performed;
        // walk limit already reached, walk will stop once in-progress walks are done.
        if total_walks >= max_walks {
            return 0;
        }
        // Try to provide `walks_desired`` walks to the caller, but limit it to the
        // # of walks remaining. Then, "reserve" the desired walk capacity within
        // `in_progress`` so that future callers asking for capacity cannot start
        // more than `max_walks` # of walks
        let walks_available = max_walks - total_walks;
        let walks_reserved = walks_desired.min(walks_available);
        *in_progress += walks_reserved;

        walks_reserved
    }

    #[inline]
    fn is_done(&self) -> bool {
        self.done.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod test {
    use crate::walk::website::WebsiteWalkBuilder;
    use std::time::Duration;

    #[test]
    fn test_yc() {
        const URL: &str = "https://news.ycombinator.com/";
        let scripts = WebsiteWalkBuilder::default()
            .with_random_ua(true)
            .with_max_walks(20)
            .with_timeout(Duration::from_secs(5))
            .with_timeout_connect(Duration::from_secs(2))
            .collect(URL)
            .unwrap();

        assert!(!scripts.is_empty());
    }
}
