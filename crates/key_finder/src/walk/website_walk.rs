use dashmap::DashSet;
use miette::{Context as _, Error, IntoDiagnostic as _, Result};
use rand::Rng;
use std::{
    borrow::{Borrow, Cow},
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        mpsc, Arc, Once, OnceLock, RwLock,
    },
    time::Duration,
};

use tinyvec::TinyVec;
use ureq::{Agent, AgentBuilder};
use url::Url;

use rayon::{prelude::*, ThreadPool};

use super::{DomVisitor, error::{NoContentDiagnostic, NotHtmlDiagnostic}};
use crate::walk::DomWalker;

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
}

impl WebsiteWalker {
    #[must_use]
    pub fn default() -> (Self, ScriptReceiver) {
        let (sender, receiver) = mpsc::channel();
        (Self::new(sender), receiver)
    }

    #[must_use]
    pub fn new(sender: ScriptSender) -> Self {
        const TIMEOUT: u64 = 10;

        let agent = AgentBuilder::new()
            .timeout_connect(Duration::from_secs(TIMEOUT))
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

    pub fn unlimited_depth(mut self) -> Self {
        self.max_walks = None;
        self
    }

    #[must_use]
    pub fn whitelist_domain<S: Into<String>>(mut self, domain: S) -> Self {
        self.domain_whitelist.push(domain.into());
        self
    }

    pub fn walk(mut self, url: &str) -> Result<()> {
        let url = url.trim().trim_end_matches('/');
        let parsed = Url::parse(&url)
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

        // returns Err if entry url is not reachable, not html, etc.
        self.visit(parsed)
    }

    fn visit(&self, url: Url) -> Result<(), Error> {
        
        if self.done.load(Ordering::Relaxed) {
            return Ok(());
        }

        if self.has_visited_url(&url) {
            // println!("skipping {url}, already visited");
            return Ok(());
        }

        println!("[WebsiteWalker]\tvisiting {url}");

        self.in_progress.fetch_add(1, Ordering::Relaxed);

        let err = format!("Failed to walk webpage {url}");
        let result = self.walk_rec(url).context(err);

        let walks_remaining = self.in_progress.fetch_sub(1, Ordering::Relaxed);
        let walks_performed = self.walks_performed.fetch_add(1, Ordering::Relaxed);

        if walks_remaining == 0 {
            println!("[WebsiteWalker]\tstopping: No more walks are in progress");
            self.finish();
            return result;
        }

        if let Some(max_walks) = self.max_walks {
            if walks_performed > max_walks {
                println!("[WebsiteWalker]\tstopping: maximum number of walks reached");
                self.finish()
            } else {
                println!("[WebsiteWalker]\t{walks_performed}/{max_walks} walks performed")
            }
        }

        result
    }

    fn walk_rec(&self, url: Url) -> Result<(), Error> {
        let entrypoint = self
            .get_webpage(url.as_str())
            .context("Failed to fetch webpage")?;
        println!("[WebsiteWalker] ({url})\tBuilding DOM walker...");
        let dom_walker = DomWalker::new(&entrypoint).context("Failed to parse HTML")?;

        println!("[WebsiteWalker] ({url})\tExtracting links and scripts");
        let (_, links) = rayon::join(
            // Extract JS scripts from page, sending them over the channel
            || {
                let mut script_visitor = UrlVisitor::new("script", "src");
                dom_walker.walk(&mut script_visitor);
                self.send_scripts(script_visitor)
            },
            // Extract links to pages that will be traversed next
            || {
                let mut link_visitor = UrlVisitor::new("a", "href");
                dom_walker.walk(&mut link_visitor);
                let links = link_visitor.into_inner();
                links
                    .into_iter()
                    .filter_map(|link| self.is_allowed_link(link))
                    .collect::<Vec<_>>()
            },
        );

        // links.into_par_iter().for_each(|link| {
        //     let _ = self.visit(link);
        // });
        links.into_iter().for_each(|link| {
            let r = self.visit(link);
            if let Err(e) = r {
                let report = miette::miette!(e);
                println!("{report}");
            }
        });

        Ok(())
    }

    fn get_webpage(&self, url: &str) -> Result<String> {
        println!("[WebsiteWalker] ({url})\tgetting webpage");
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
                return NotHtmlDiagnostic::new(url, content_type).into()
            }
        }

        // Check that response was not empty
        if let Some(content_length) = response.header("Content-Length") {
            if let Ok(content_len) = usize::from_str_radix(content_length, 10) {
                if content_len == 0 {
                    return NoContentDiagnostic::new(url).into()
                }
            }
        }
        let webpage = response.into_string().into_diagnostic()?;
        println!("[WebsiteWalker] ({url})\tgot webpage");
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
            .context("[WebsiteWalker] Failed to send scripts over the channel")
            .unwrap();
    }

    fn is_allowed_link(&self, link: String) -> Option<Url> {
        {
            let link = link.trim();
            if link.is_empty() || link.starts_with('#') {
                return None;
            }
        }

        let resolved = if link.starts_with('/') || !link.contains("://") {
            self.base_url.get().unwrap().join(&link)
        } else {
            Url::parse(&link)
        };
        resolved.ok().and_then(|link| {
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
        self.domain_whitelist
            .iter()
            .find(|d| d.as_str() == domain)
            .is_some()
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
            return self.has_visited_url_clean(&without_query_params);
        } else {
            let query = new_params
                .into_iter()
                .fold(String::new(), |acc, (key, value)| {
                    acc + format!("{key}={value}").as_str()
                });
            without_query_params.set_query(Some(query.as_str()));
            return self.has_visited_url_clean(&without_query_params);
            // retur
        }
    }
    fn has_visited_url_clean(&self, url: &Url) -> bool {
        if self.seen_urls.contains(&url) {
            // println!("skipping {url}, already visited");
            // return Ok(());
            return true;
        } else {
            self.seen_urls.insert(url.clone());
            return false;
        }
    }
    fn finish(&self) {
        println!("[WebsiteWalker] ({}) finishing walk", self.base_url.get().unwrap());
        let already_done = self.done.swap(true, Ordering::Relaxed);
        if !already_done {
            let _ = self.sender.send(None);
        }
    }
}

#[derive(Debug)]
struct UrlVisitor {
    urls: Vec<String>,
    tag_name: &'static str,
    attr_names: TinyVec<[&'static str; 2]>,
}

impl UrlVisitor {
    pub fn new(tag_name: &'static str, attr_name: &'static str) -> Self {
        Self {
            tag_name,
            attr_names: tinyvec::tiny_vec!([&'static str; 2] => attr_name),
            urls: vec![],
        }
    }
    pub fn with_attr(mut self, attr: &'static str) -> Self {
        self.attr_names.push(attr);
        self
    }

    pub fn with_attrs<A: IntoIterator<Item = &'static str>>(mut self, attrs: A) -> Self {
        self.attr_names.extend(attrs);

        self
    }

    pub fn into_inner(self) -> Vec<String> {
        self.urls
    }
}

impl IntoIterator for UrlVisitor {
    type Item = String;
    type IntoIter = <Vec<String> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.urls.into_iter()
    }
}

impl<'dom> DomVisitor<'dom> for UrlVisitor {
    fn visit_element(&mut self, node: &'dom html_parser::Element) {
        let is_tag = node
            .name
            .as_str()
            .trim()
            .eq_ignore_ascii_case(self.tag_name);
        if !is_tag {
            return;
        }
        for attr in &self.attr_names {
            if let Some(Some(value)) = node.attributes.get(*attr) {
                self.urls.push(value.clone());
                return;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::WebsiteWalker;
    use std::thread::spawn;

    #[test]
    fn test_asta_gym() {
        // const URL: &str =
        // "http://asta-dev-gym.s3-website.us-east-2.amazonaws.com/";
        const URL: &str = "https://news.ycombinator.com/";
        let (walker, rx) = WebsiteWalker::default();

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
