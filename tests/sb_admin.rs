use keyhunter::{ApiKeyCollector, ApiKeyMessage, WebsiteWalkBuilder};
use miette::{miette, IntoDiagnostic as _, Result};
use std::{
    env, ops,
    path::{Path, PathBuf},
    process::{self, Child, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
};

/// Get the absolute path to the root of the project (where the Cargo.toml is)
#[cfg(not(tarpaulin_include))]
fn root() -> PathBuf {
    PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .canonicalize()
        .unwrap()
}

#[cfg(not(tarpaulin_include))]
fn setup_sb_admin() -> Result<PathBuf> {
    use std::process::Command;

    let root = root();
    let setup_script = root.join("tasks/sb_admin.sh");
    assert!(setup_script.is_file());
    // Run the setup script. Prints absolute path to the sb admin site
    // directory, which we capture and return.
    let site_dir = String::from_utf8(
        Command::new(setup_script)
            .stderr(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()
            .into_diagnostic()?
            .wait_with_output()
            .into_diagnostic()?
            .stdout,
    )
    .into_diagnostic()?;

    let site_dir = PathBuf::from(site_dir.trim());
    assert!(site_dir.is_dir());

    Ok(site_dir)
}

/// Start a web server at port 8080 to serve static assets located in `site_dir`
///
/// This function relies on npx, meaning you must have node installed on your
/// machine for this to work.
fn serve_local(site_dir: &Path) -> Result<AutoKilledChild> {
    let mut serve = process::Command::new("npx");
    serve
        .args(["http-server", "-p", "8080"])
        .arg(site_dir)
        .stdout(Stdio::null());
    serve.spawn().into_diagnostic().map(AutoKilledChild::from)
}

fn poll_server(site_url: &str) -> Result<()> {
    const MAX_ATTEMPTS: u32 = 6;
    let mut i = MAX_ATTEMPTS;

    loop {
        if i == 0 {
            return Err(miette!(
                "Server at '{}' was not ready after '{}' attempts",
                site_url,
                MAX_ATTEMPTS
            ));
        }
        if ureq::get(site_url)
            .timeout(Duration::from_millis(500))
            .call()
            .is_ok()
        {
            return Ok(());
        }
        i -= 1;
        println!("site is not ready, retrying in 1 second...");
        thread::sleep(Duration::from_secs(1));
    }
}

#[derive(Debug)]
struct AutoKilledChild(Child);

impl ops::Deref for AutoKilledChild {
    type Target = Child;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for AutoKilledChild {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Child> for AutoKilledChild {
    fn from(child: Child) -> Self {
        Self(child)
    }
}
impl Drop for AutoKilledChild {
    fn drop(&mut self) {
        self.0.kill().unwrap()
    }
}

#[test]
fn test_sb_admin() -> Result<()> {
    miette::set_hook(Box::new(|_| {
        Box::new(
            miette::MietteHandlerOpts::new()
                .graphical_theme(miette::GraphicalTheme::default())
                .terminal_links(true)
                .unicode(true)
                .context_lines(3)
                .width(180)
                .color(true)
                .with_cause_chain()
                .build(),
        )
    }))
    .unwrap();

    let site_dir = setup_sb_admin()?;

    // Serve the dashboard site on localhost:8080
    println!("starting server");
    let mut serve_child = serve_local(&site_dir)?;
    const SITE_URL: &str = "http://localhost:8080";

    // wait until the server has started
    println!("waiting for server to be ready");
    if let Err(e) = poll_server(SITE_URL) {
        drop(serve_child);
        panic!("{e}");
    }

    let builder = WebsiteWalkBuilder::new()
        .with_timeout(Duration::from_secs(1))
        .with_shared_cache(false);

    // first pass to test that expected # of urls were collected while walking
    let scripts_res = builder.collect(SITE_URL);

    // second pass that sends scripts to ApiKeyCollector to tests key extraction/collection
    let (key_sender, key_receiver) = mpsc::channel();
    let (script_sender, script_receiver) = mpsc::channel();

    let key_handle = thread::spawn(move || {
        let mut keys = vec![];
        while let Ok(message) = key_receiver.recv() {
            match message {
                ApiKeyMessage::Stop => break,
                ApiKeyMessage::Keys(api_keys) => {
                    keys.extend(api_keys);
                }
                ApiKeyMessage::RecoverableFailure(err) => {
                    println!("{:?}", err);
                }
                _ => {}
            }
        }
        keys
    });

    let collector =
        ApiKeyCollector::new(Default::default(), script_receiver, key_sender).with_random_ua(true);
    let collector_handle = thread::spawn(move || {
        collector.collect();
    });

    let walker = builder.build(script_sender);
    let walk_res = walker.walk(SITE_URL);

    // wait for collection to stop before terminating server. Do not check
    // results until after server has terminated, otherwise the process will
    // remain open.

    let collector_handle_result = collector_handle.join();
    let key_handle_result = key_handle.join();

    serve_child.kill().into_diagnostic()?;

    let mut scripts = scripts_res?;
    collector_handle_result.unwrap();
    let api_keys = key_handle_result.unwrap();
    walk_res?;

    // =========================================================================
    // check script urls found in first pass

    println!("Found {} scripts:\n{:#?}", scripts.len(), scripts);
    assert_eq!(scripts.len(), 11);

    // no duplicates
    scripts.sort_unstable();
    scripts.dedup();
    assert_eq!(scripts.len(), 11);

    // =========================================================================
    // check keys found in second pass

    assert!(api_keys.is_empty());

    Ok(())
}
