use codspeed_criterion_compat::{criterion_group, Criterion};
use keyhunter::WebsiteWalkBuilder;
use miette::{miette, IntoDiagnostic as _, Result};
use std::{
    env, ops,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::Duration,
};

#[cfg(not(tarpaulin_include))]
fn root() -> PathBuf {
    PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .canonicalize()
        .unwrap()
}

#[cfg(not(tarpaulin_include))]
fn setup_sb_admin() -> Result<PathBuf> {
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

const ONE_SECOND: Duration = Duration::from_secs(1);

#[cfg(not(tarpaulin_include))]
fn poll_server(site_url: &str) -> Result<()> {
    const MAX_ATTEMPTS: u32 = 5;
    let mut i = MAX_ATTEMPTS;
    let mut delay = ONE_SECOND;

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
        let secs = delay.as_secs();
        println!(
            "site is not ready, retrying in {secs} second{}...",
            if secs == 1 { "" } else { "s" }
        );
        thread::sleep(delay);
        delay += ONE_SECOND + ONE_SECOND;
    }
}

/// Start a web server at port 8080 to serve static assets located in `site_dir`
///
/// This function relies on npx, meaning you must have node installed on your
/// machine for this to work.
fn serve_local(site_dir: &Path, site_url: &str) -> Result<AutoKilledChild> {
    let mut server = Command::new("npx")
        .args(["http-server", "-p", "8080"])
        .arg(site_dir)
        // .stdout(Stdio::null())
        // .stderr(Stdio::null())
        .spawn()
        .into_diagnostic()?;

    match poll_server(site_url) {
        Ok(_) => Ok(server.into()),
        Err(e) => {
            if let Err(kill_err) = server.kill().into_diagnostic() {
                Err(e.context(kill_err))
            } else {
                Err(e)
            }
        }
    }
}
const SITE_URL: &str = "http://localhost:8080";

fn force_teardown() {
    println!("Force-killing web server...");
    Command::new(root().join("tasks/kill_8080.sh"))
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
}

#[cfg(not(tarpaulin_include))]
fn benchmark_script_collection(c: &mut Criterion) {
    // let mut group = c.benchmark_group("sb_admin");
    // group.sample_size(10).bench_function("collect", |b| {
    //     b.iter(|| WebsiteWalkBuilder::default().collect(SITE_URL))
    // });
    // group.finish();
    c.bench_function("collect", |b| {
        b.iter(|| WebsiteWalkBuilder::default().collect(SITE_URL))
    });
}

criterion_group!(benches, benchmark_script_collection);

fn main() {
    println!("Setting up sb admin files...");
    let site_dir = setup_sb_admin().unwrap();

    println!("Starting web server...");
    let server = serve_local(&site_dir, SITE_URL).unwrap();

    #[cfg(not(codspeed))]
    {
        benches();
        drop(server);
        force_teardown();
        Criterion::default().configure_from_args().final_summary();
    }

    #[cfg(codspeed)]
    {
        let mut criterion = Criterion::new_instrumented();
        benches(&mut criterion);
        drop(server);
        force_teardown()
    }
}
