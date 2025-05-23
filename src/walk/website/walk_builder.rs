use std::{borrow::Cow, num::NonZeroUsize, sync::mpsc, time::Duration};

use miette::{Error, MietteDiagnostic, Result};
use ureq::{Agent, AgentBuilder};

use super::{walk::ScriptSender, walk_cache::WalkCache, Script};
use crate::{http::random_ua, ScriptReceiver, WebsiteWalker};

#[derive(Debug, Clone)]
#[must_use]
#[non_exhaustive]
pub struct WebsiteWalkBuilder {
    /// Maximum number of pages that can be visited.
    ///
    /// [`None`] means there is no limit.
    ///
    /// Default [`None`]
    pub(crate) max_walks: Option<NonZeroUsize>,
    /// User agent header to use when making requests
    ///
    /// Default [`Some`] user agent
    pub(crate) ua: Option<Cow<'static, str>>,
    /// Extra headers to add to requests
    ///
    /// By default, the following headers are added:
    /// - `Accept: text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8`
    /// - `Keep-Alive: timeout=5, max=100`
    /// - `Connection: keep-alive`
    /// - `Accept-Language: en-US,en;q=0.5`
    /// - `Accept-Encoding: gzip, deflate, br`
    /// - `DNT: 1`
    pub(crate) headers: Vec<(String, String)>,
    /// Domains that can be visited (and have their scripts extracted)
    ///
    /// When a walk begins, the domain of the URL is checked against this list.
    ///
    /// Default `[]`
    pub(crate) domain_whitelist: Vec<String>,
    /// When `true`, [`None`] will be sent over the script channel to close it.
    ///
    /// Default `true`
    pub(crate) close_channel_when_done: bool,
    /// When `true`, cookies will be stored and used across requests.
    ///
    /// Default `true`
    store_cookies: bool,
    /// Shared cache across walks
    pub(crate) cache: Option<WalkCache>,
    /// Timeout for requests
    ///
    /// See: [`AgentBuilder::timeout`]
    ///
    /// Default [`None`]
    pub(crate) timeout: Option<Duration>,
    /// Timeout for connecting to a server
    ///
    /// See: [`AgentBuilder::timeout_connect`]
    ///
    /// Default [`None`]
    pub(crate) timeout_connect: Option<Duration>,
}

impl Default for WebsiteWalkBuilder {
    fn default() -> Self {
        let headers: Vec<(String, String)> = vec![
            (
                "Accept".into(),
                "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8".into(),
            ),
            ("Keep-Alive".into(), "timeout=5, max=100".into()),
            ("Connection".into(), "keep-alive".into()),
            ("Accept-Language".into(), "en-US,en;q=0.5".into()),
            // TODO: use flat2 to decompress responses
            // ("Accept-Encoding".into(), "gzip, deflate, br".into()),
            ("DNT".into(), "1".into()),
        ];

        let mut rng = rand::rng();
        let ua = Some(Cow::Borrowed(random_ua(&mut rng)));

        Self {
            max_walks: None,
            ua,
            headers,
            domain_whitelist: Vec::new(),
            close_channel_when_done: true,
            timeout: None,
            timeout_connect: None,
            store_cookies: true,
            cache: None,
        }
    }
}

impl WebsiteWalkBuilder {
    const USER_AGENT: &'static str = "User-Agent";

    /// Create a new builder with default settings
    pub fn new() -> Self {
        Default::default()
    }

    /// Set the maximum number of pages that can be visited.
    ///
    /// Use [`WebsiteWalkBuilder::with_unlimited_walks`] to remove the limit.
    ///
    /// By default, there is no limit.
    ///
    /// # Panics
    /// if `max_walks` is zero.
    pub fn with_max_walks(mut self, max_walks: usize) -> Self {
        let max_walks = NonZeroUsize::new(max_walks)
            .ok_or_else(|| {
                Error::msg(
                    "max_walks must be greater than zero, otherwise no pages will be checked.",
                )
                .context("Failed to configure WebsiteWalkBuilder")
            })
            .unwrap();
        self.max_walks = Some(max_walks);
        self
    }

    /// Do not limit the number of pages that can be visited.
    ///
    /// Use [`WebsiteWalkBuilder::with_max_walks`] to set a walk limit.
    ///
    /// By default, there is no limit. Using this method on
    /// [`WebsiteWalkBuilder::default()`] will have no effect.
    pub fn with_unlimited_walks(mut self) -> Self {
        self.max_walks = None;
        self
    }

    /// Use a random, browser-like `User-Agent` header when making requests.
    ///
    /// Using a mock UA can help bypass bot detection on some websites. However,
    /// there are some cases where specific browsers are prevented from
    /// accessing websites, and so using a random UA may not be ideal.
    ///
    /// This is a semi-specific case of
    /// [`WebsiteWalkBuilder::with_header`]. `User-Agent`s set with this
    /// method will take precedence.
    ///
    /// By default, no `User-Agent` header is set.
    pub fn with_random_ua(mut self, yes: bool) -> Self {
        if yes && self.ua.is_none() {
            let mut rng = rand::rng();
            self.ua = Some(Cow::Borrowed(random_ua(&mut rng)));
        } else if !yes {
            self.ua = None;
        }

        self
    }

    /// Add an extra header to all requests.
    ///
    /// Use [`WebsiteWalkBuilder::with_headers`] for adding multiple headers.
    #[inline]
    pub fn with_header<S: Into<String>>(mut self, key: S, value: S) -> Self {
        let key = key.into();
        if key == Self::USER_AGENT {
            self.ua = Some(Cow::Owned(value.into()));
        } else {
            self.headers.push((key, value.into()));
        }

        self
    }

    /// Add extra headers to all requests
    ///
    /// Use [`WebsiteWalkBuilder::with_header`] for adding a single header.
    pub fn with_headers<I>(mut self, headers: I) -> Self
    where
        I: IntoIterator<Item = (String, String)>,
    {
        self.headers.extend(headers);
        self
    }

    /// Whitelist a domain for crawling. Only domains in this list will have
    /// their pages scanned for scripts.
    ///
    /// This setting does not affect what scripts will be checked; cross-origin
    /// scripts will still be sent to the script channel.
    ///
    /// Use [`WebsiteWalkBuilder::with_whitelisted_domains`] to add multiple
    /// domains.
    #[inline]
    pub fn with_whitelisted_domain<S: Into<String>>(mut self, domain: S) -> Self {
        self.domain_whitelist.push(domain.into());
        self
    }

    /// Whitelist multiple domains for crawling. Only domains in this list will have
    /// their pages scanned for scripts.
    ///
    /// This setting does not affect what scripts will be checked; cross-origin
    /// scripts will still be sent to the script channel.
    ///
    /// Use [`WebsiteWalkBuilder::with_whitelisted_domain`] to add a single
    /// domain.
    pub fn with_whitelisted_domains<I, S>(mut self, domains: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.domain_whitelist
            .extend(domains.into_iter().map(|s| s.into()));
        self
    }

    /// Close the script channel when the walk is done. If you plan on
    /// performing multiple walks, leave the channel open.
    ///
    /// By default, the script channel will be closed when the walk is done.
    pub fn with_close_channel(mut self, yes: bool) -> Self {
        self.close_channel_when_done = yes;
        self
    }

    /// Store cookies and use them across requests.
    ///
    /// This is enabled by default.
    pub fn with_cookie_jar(mut self, yes: bool) -> Self {
        self.store_cookies = yes;
        self
    }

    /// Share a URL and script cache across walks.
    ///
    /// Useful for avoiding duplicate work when performing multiple walks.
    ///
    /// By default, each walk has its own cache.
    pub fn with_shared_cache(mut self, yes: bool) -> Self {
        if yes && self.cache.is_none() {
            self.cache = Some(WalkCache::default());
        } else if !yes {
            self.cache = None;
        }
        self
    }

    pub fn clear_cache(&mut self) {
        self.cache.as_mut().map(WalkCache::clear);
    }

    /// Overall timeout for page requests. You can override socket connection
    /// timeouts using [`WebsiteWalkBuilder::with_timeout_connect`].
    ///
    /// See: [`AgentBuilder::timeout`]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Timeout for socket connection to a server. Overrides [`WebsiteWalkBuilder::with_timeout`].
    ///
    /// See: [`AgentBuilder::timeout_connect`]
    pub fn with_timeout_connect(mut self, timeout: Duration) -> Self {
        self.timeout_connect = Some(timeout);
        self
    }

    pub(crate) fn build_agent(&self) -> Agent {
        let mut builder = AgentBuilder::new();

        // enable/disable cookie jar
        if self.store_cookies {
            builder = builder.cookie_store(Default::default());
        }

        // set default timeout
        if let Some(timeout) = self.timeout {
            builder = builder.timeout(timeout);
        }

        // set connect timeout override
        if let Some(connect_timeout) = self.timeout_connect {
            builder = builder.timeout_connect(connect_timeout);
        }

        builder.build()
    }

    pub(crate) fn headers(&self) -> impl Iterator<Item = (&str, &str)> + '_ {
        self.ua
            .as_ref()
            .map(|ua| (Self::USER_AGENT, ua.as_ref()))
            .into_iter()
            .chain(self.headers.iter().map(|(k, v)| (k.as_str(), v.as_str())))
    }

    pub fn build(&self, sender: ScriptSender) -> WebsiteWalker {
        WebsiteWalker::new(self, sender)
    }

    pub fn build_with_channel(&self) -> (WebsiteWalker, ScriptReceiver) {
        let (tx, rx) = mpsc::channel();
        let walker = WebsiteWalker::new(self, tx);
        (walker, rx)
    }

    pub fn collect<S: AsRef<str>>(&self, entrypoint: S) -> Result<Vec<Script>> {
        const ACC_INITIAL_CAPACITY: usize = 32;

        let (walker, receiver) = self.build_with_channel();
        let recv_handle = std::thread::spawn(move || {
            receiver
                .into_iter()
                .fold(Vec::with_capacity(ACC_INITIAL_CAPACITY), |mut acc, el| {
                    acc.extend(el);
                    acc
                })
        });
        walker.walk(entrypoint.as_ref())?;

        recv_handle.join().map_err(|e| {
            match e.downcast::<MietteDiagnostic>() {
                Ok(e) => {
                    Error::new_boxed(e)
                },
                Err(e) => {
                    match e.downcast::<String> () {
                        Ok(e) => {
                            Error::msg(e).context(format!("Failed to join script receiver handle while walking '{}'", entrypoint.as_ref()))
                        },
                        Err(_) => {
                            Error::msg(format!("Failed to join script receiver handle while walking '{}': an unknown error occurred", entrypoint.as_ref()))
                        }

                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod test {

    use super::*;
    #[test]
    fn test_builder() {
        let builder = WebsiteWalkBuilder::default()
            .with_max_walks(20)
            .with_shared_cache(true)
            .with_cookie_jar(true);
        let (sender, _receiver) = mpsc::channel();

        let _walker: WebsiteWalker = builder.build(sender);
    }

    #[test]
    fn test_headers() {
        let mut builder = WebsiteWalkBuilder::default();
        let headers: Vec<_> = builder.headers().collect();

        assert_eq!(headers.len(), 6);
        assert!(headers.iter().any(|(k, _)| *k == "User-Agent"));

        // FIXME: `builder.with_headers` duplicates the UA header
        builder = builder.with_header("User-Agent", "test");
        assert_eq!(builder.headers().count(), 6);
        let ua = builder
            .headers()
            .find(|(k, _)| *k == "User-Agent")
            .expect("No UA header");
        assert_eq!(ua.1, "test");
    }

    #[test]
    fn test_ua() {
        let builder = WebsiteWalkBuilder::default();
        // by default, walker starts with a random user agent
        assert!(builder.ua.is_some());
        let ua = builder
            .ua
            .as_ref()
            .expect("Walk builder should start with a random user agent")
            .clone();

        // setting a random ua when one exists is a no-op
        let builder = builder.with_random_ua(true);
        let new_ua = builder.ua.as_ref().unwrap();
        assert_eq!(
            &ua, new_ua,
            "with_random_ua should not replace an existing user agent"
        );

        let builder = builder.with_random_ua(false);
        assert!(
            builder.ua.is_none(),
            "with_random_ua(false) should remove the user agent"
        );

        // setting a random ua when none exists adds one
        let builder = builder.with_random_ua(true);
        assert!(
            builder.ua.is_some(),
            "with_random_ua(true) should add a user agent"
        );
    }
}
