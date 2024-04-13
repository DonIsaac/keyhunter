use anyhow::Error;
use dashmap::DashSet;
use rand::{Rng};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering}, mpsc, Once, OnceLock, RwLock
    },
    time::Duration,
};

use tinyvec::TinyVec;
use ureq::{Agent, AgentBuilder};
use url::Url;

use rayon::{prelude::*, ThreadPool};

use super::DomVisitor;
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

fn random_ua<R: Rng>(rng: &mut R) -> &'static str {
    let idx = rng.gen_range(0..USER_AGENTS.len());
    USER_AGENTS[idx]
}

pub type ScriptMessage = Option<Vec<Url>>;
pub type ScriptSender = mpsc::Sender<ScriptMessage>;
pub type ScriptReceiver = mpsc::Receiver<ScriptMessage>;

#[derive(Debug)]
pub struct WebsiteWalker {
    agent: Agent,
    ua: &'static str,
    // scripts: RwLock<Vec<String>>
    sender: mpsc::Sender<ScriptMessage>,
    in_progress: AtomicU64,
    // domain_blacklist: Option<Vec<String>>,
    domain_whitelist: Vec<String>,
    walks_performed: AtomicUsize,
    max_walks: Option<usize>,
    done: AtomicBool,
    base_url: OnceLock<Url>,
    seen_urls: DashSet<Url>
}

impl WebsiteWalker {
    pub fn default() -> (Self, ScriptReceiver) {
        let (sender, receiver) = mpsc::channel();
        (Self::new(sender), receiver)
    }

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
            seen_urls: Default::default()
        }
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

    pub fn walk(mut self, url: &str) -> Result<(), Error> {
        let url = url.trim().trim_end_matches('/');
        let parsed = Url::parse(&url)?;

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

        if self.seen_urls.contains(&url) {
            println!("skipping {url}, already visited");
            return Ok(())
        } else {
            self.seen_urls.insert(url.clone());
        }

        println!("visiting {url}");

        self.in_progress.fetch_add(1, Ordering::Relaxed);

        let result = self.walk_rec(url);

        let walks_remaining = self.in_progress.fetch_sub(1, Ordering::Relaxed);
        let walks_performed = self.walks_performed.fetch_add(1, Ordering::Relaxed);
        let walk_limit_reached = self
            .max_walks
            .is_some_and(|max_walks| walks_performed > max_walks);

        if walks_remaining == 0 || walk_limit_reached {
            self.finish()
        }

        result
    }

    fn walk_rec(&self, url: Url) -> Result<(), Error> {
        let entrypoint = self.get_webpage(url.as_str())?;
        let dom_walker = DomWalker::new(&entrypoint)?;

        // let script_handle = rayon::spawn(|| {

        // })
        let mut script_visitor = UrlVisitor::new("script", "src");
        let mut link_visitor = UrlVisitor::new("a", "href");

        dom_walker.walk(&mut script_visitor);
        dom_walker.walk(&mut link_visitor);

        let scripts = script_visitor.into_inner();
        let scripts = scripts.into_iter().filter_map(|script| {
            let base_url = self.base_url.get().unwrap();
            base_url.join(&script).ok()
        }).collect::<Vec<_>>();
        self.sender.send(Some(scripts))?;

        let links = link_visitor.into_inner();
        println!("found links: {links:?}");

        links
            .into_par_iter()
            .filter_map(|link| self.is_allowed_link(link))
            .for_each(|link| {
                let _ = self.visit(link);
            });

        Ok(())
    }

    fn get_webpage(&self, url: &str) -> Result<String, Error> {
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
            .call()?;

        // Check that we got HTML back
        if let Some(content_type) = response.header("Content-Type") {
            if !content_type.contains("html") {
                return Err(Error::msg(format!(
                    "Expected {url} to return HTML, got {content_type}"
                )));
            }
        }

        // Check that response was not empty
        if let Some(content_length) = response.header("Content-Length") {
            if let Ok(content_len) = usize::from_str_radix(content_length, 10) {
                if content_len == 0 {
                    return Err(Error::msg("Cannot parse webpage: Server returned no data"));
                }
            }
        }
        let webpage = response.into_string()?;
        Ok(webpage)
    }

    fn is_allowed_link(&self, link: String) -> Option<Url> {
        {
            let link = link.trim();
            if link.is_empty() || link.starts_with('#') {
                return None
            }
        }

        let resolved = if link.starts_with('/') || !link.contains("://") {
            self.base_url.get().unwrap().join(&link)
        } else {
            Url::parse(&link)
        };
        resolved.ok().and_then(|link| {
            let is_whitelisted = link.domain()
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

    fn finish(&self) {
        let _ = self.sender.send(None);
        self.done.swap(false, Ordering::Relaxed);
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
        const URL: &str = "http://asta-dev-gym.s3-website.us-east-2.amazonaws.com/";
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
