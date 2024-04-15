use std::{
    sync::{mpsc, Arc},
    thread::{self, JoinHandle},
};

use log::error;
use miette::{Context, Error, IntoDiagnostic as _};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::{
    extract::ApiKeyError, walk::ScriptSender, ApiKeyCollector, ApiKeyMessage, ApiKeyReceiver, Config, ScriptMessage, WebsiteWalker
};

#[derive(Debug)]
pub struct KeyHunter {
    config: Arc<Config>,
    // walker: WebsiteWalker,
    collector: ApiKeyCollector,
    script_sender: ScriptSender,
    key_receiver: ApiKeyReceiver,
}

impl Default for KeyHunter {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl KeyHunter {
    #[must_use]
    pub fn new(config: Arc<Config>) -> Self {
        let (script_sender, script_receiver) = mpsc::channel::<ScriptMessage>();
        let (key_sender, key_receiver) = mpsc::channel::<ApiKeyMessage>();
        // let walker = WebsiteWalker::new(script_sender.clone());
        let collector = ApiKeyCollector::new(config.clone(), script_receiver, key_sender);

        Self {
            config,
            // walker,
            collector,
            script_sender,
            key_receiver,
        }
    }
    pub fn collect_par_iter<'a, I: ParallelIterator<Item = &'a str> + 'static>(self, urls: I) {
        let collector_handle = thread::spawn(move || self.collector.collect());

        // let config = Arc::clone(&self.config);
        let walker_handle = thread::spawn(move || {
            urls.for_each(move |url| {
                let script_sender = self.script_sender.clone();
                let walker = WebsiteWalker::new(script_sender).with_max_walks(20);
                let result = walker
                    .walk(url)
                    .with_context(|| format!("Failed to crawl url '{url}'"));

                // let result = self.walker.walk()
                // let collector = ApiKeyCollector::new(self.config.clone())
            });
        });
    }

    #[must_use]
    fn walker(&self) -> WebsiteWalker {
        WebsiteWalker::new(self.script_sender.clone()).with_max_walks(20)
    }
}

#[derive(Debug)]
struct KeyHunterIter {
    collector_handle: JoinHandle<()>,
    walker_handle: JoinHandle<()>,
    key_receiver: ApiKeyReceiver,
}

impl Iterator for KeyHunterIter {
    type Item = ApiKeyError;

    fn next(&mut self) -> Option<Self::Item> {
        match self.key_receiver.recv().into_diagnostic() {
            Ok(maybe_key) => maybe_key,
            Err(err) => {
                error!("{}", err.context("KeyHunter failed to receive API keys over key channel"));
                None
            }
        }
    }
}

