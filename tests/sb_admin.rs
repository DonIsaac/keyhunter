use keyhunter::{ApiKeyCollector, ApiKeyMessage, WebsiteWalkBuilder};
use miette::{miette, IntoDiagnostic as _, Result};
use std::{
    env, fs, io, ops,
    path::{Path, PathBuf},
    process::{self, Child, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
};

/// absolute path to `target` dir
fn target() -> PathBuf {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .canonicalize()
        .unwrap();
    let target = root.join("target");
    fs::create_dir_all(&target).unwrap();

    assert!(target.is_absolute());
    assert!(target.is_dir());

    target
}

/// Download zipped startbootstrap admin dashboard source code to `archive_path`.
fn download_archive(archive_path: &Path) -> Result<()> {
    const URL: &'static str =
        "https://github.com/startbootstrap/startbootstrap-sb-admin-2/archive/gh-pages.zip";

    // Pipe response body into file's write stream
    let mut archive = fs::File::create(archive_path).into_diagnostic()?;
    let mut res = ureq::get(URL).call().unwrap().into_reader();
    io::copy(&mut res, &mut archive).into_diagnostic()?;

    Ok(())
}

fn sb_admin_setup() -> Result<PathBuf> {
    let target = target();
    let sites = target.join("sites");
    fs::create_dir_all(&sites).into_diagnostic()?;
    let sb_admin = sites.join("sb_admin");

    if !sb_admin.exists() {
        let archive_path = sb_admin.join("sb_admin.zip");
        println!("downloading sb_admin archive to {}", archive_path.display());
        download_archive(&archive_path)?;
        println!("unzipping archive");

        let mut unzip = process::Command::new("unzip");
        unzip.arg(&archive_path);
        unzip.arg("-d");
        unzip.arg(&sb_admin);
        let mut child = unzip.spawn().into_diagnostic()?;
        child.wait().into_diagnostic()?;

        println!("unzip complete, deleting archive");
        fs::remove_file(&archive_path).into_diagnostic()?;
    }

    // actual site will be the only folder in sb_admin
    let site_dir = fs::read_dir(&sb_admin)
        .into_diagnostic()?
        .next()
        .ok_or(miette!("sb_admin folder is empty"))?
        .into_diagnostic()?
        .path();
    println!("site dir: {}", site_dir.display());
    assert!(
        site_dir.is_dir(),
        "site dir '{}' is not a directory",
        site_dir.display()
    );

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
        .arg(&site_dir)
        .stdout(Stdio::null());
    serve.spawn().into_diagnostic().map(AutoKilledChild::from)
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

    let site_dir = sb_admin_setup()?;

    // Serve the dashboard site on localhost:8080
    println!("starting server");
    let mut serve_child = serve_local(&site_dir)?;
    let site_url = "http://localhost:8080";

    // wait until the server has started
    println!("waiting for server to be ready");
    let mut i = 5;
    loop {
        if i == 0 {
            panic!("Could not reach local sb admin site after 5 attempts");
        }
        if ureq::get(&site_url)
            .timeout(Duration::from_millis(500))
            .call()
            .is_ok()
        {
            break;
        }
        i -= 1;
        println!("site is not ready, retrying in 1 second...");
        thread::sleep(Duration::from_secs(1));
    }

    let builder = WebsiteWalkBuilder::new()
        .with_timeout(Duration::from_secs(1))
        .with_shared_cache(false);

    // first pass to test that expected # of urls were collected while walking
    let scripts_res = builder.collect(site_url);

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
    let walk_res = walker.walk(site_url);

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
