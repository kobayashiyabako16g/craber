//! Helpers shared by the `craber` integration tests.
//!
//! Every test here drives the real binary as a subprocess rather than calling
//! into the library: the CLI's contract is its argv, its exit status and its
//! streams, and a test that bypasses `main` cannot check any of them.
//!
//! Cargo builds each integration test as its own crate, so this module is
//! compiled once per test file and every file uses only part of it — hence the
//! blanket `dead_code` allow, which is the conventional shape for a
//! `tests/common` module.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A schema in `fixtures/`, by filename. See `fixtures/README.md` for what each
/// one is for.
pub fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
}

/// The repository's `examples/` directory.
pub fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

/// Run the `craber` binary this test was built alongside.
pub fn craber(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_craber"))
        .args(args)
        .output()
        .expect("running craber")
}

/// Run `craber` from `dir`, for the tests that turn on where the command was
/// started: config discovery walks up from the working directory, and a
/// relative path inside a config file is resolved against the file.
pub fn craber_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_craber"))
        .current_dir(dir)
        .args(args)
        .output()
        .expect("running craber")
}

/// A temporary directory that exists and is empty, for a test that writes a
/// `craber.toml` and runs inside it.
///
/// Every config test needs one: discovery walks up from the working directory,
/// so a test that ran in the crate directory would see whatever the repository
/// happens to have above it.
pub fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("craber-config-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("scratch directory");
    path
}

/// Fail with the command's own stderr, which is the only place a CLI explains
/// itself.
pub fn assert_success(output: &Output, what: &str) {
    assert!(
        output.status.success(),
        "{what} failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Run `craber` and parse its stdout as JSON, failing with stderr if either step
/// does not work.
pub fn craber_json(args: &[&str]) -> serde_json::Value {
    json_output(&craber(args))
}

/// [`craber_json`] from a given directory, for the config tests: discovery walks
/// up from the working directory, so where the command ran is the thing under
/// test.
pub fn craber_json_in(dir: &Path, args: &[&str]) -> serde_json::Value {
    json_output(&craber_in(dir, args))
}

/// The stdout of a successful run, as JSON.
///
/// Both failures report stderr, because a CLI that could not do what was asked
/// says so there and a test that hides it is a test that says only "None".
fn json_output(output: &Output) -> serde_json::Value {
    assert_success(output, "craber");
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout was not JSON ({error}):\n{}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

/// A temporary directory that does not exist yet, so a test also covers the
/// command creating its own output directory.
///
/// Named after the test rather than randomly, so a failed run leaves something
/// a human can go and look at.
pub fn out_dir(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("craber-build-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    path
}

/// One table out of an exported `schema.json`.
pub fn table<'a>(schema: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    schema["tables"]
        .as_array()
        .expect("tables")
        .iter()
        .find(|table| table["name"] == name)
        .unwrap_or_else(|| panic!("no table named {name}"))
}

/// Table names, in the order the export put them.
pub fn table_names(schema: &serde_json::Value) -> Vec<&str> {
    schema["tables"]
        .as_array()
        .expect("tables")
        .iter()
        .map(|table| table["name"].as_str().expect("table name"))
        .collect()
}

/// Assert a path exists, naming what was looked for.
pub fn assert_exists(path: &Path) {
    assert!(path.is_file(), "{} was not written", path.display());
}

/// Whether there is a web bundle to inspect, and a note on stderr when there is
/// not.
///
/// The bundle is compiled into the binary, so a Rust-only checkout —
/// CI's Rust job, or a contributor without Node — has nothing embedded. That
/// case has tests of its own; every test that needs a real site starts here
/// instead, because there is nothing for it to look at.
pub fn bundle_or_skip() -> bool {
    if craber_site::is_embedded() {
        return true;
    }
    eprintln!("skipped: no web bundle embedded, run `just web-build` and rebuild");
    false
}

/// One column out of a table in an exported `schema.json`.
pub fn column<'a>(table: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    table["columns"]
        .as_array()
        .expect("columns")
        .iter()
        .find(|column| column["name"] == name)
        .unwrap_or_else(|| panic!("no column named {name}"))
}

/// A `craber serve` child process, stopped when it goes out of scope.
///
/// The `Drop` matters: without it a failing assertion leaves a server holding a
/// port for the rest of the session.
pub struct Server {
    child: std::process::Child,
    pub port: u16,
}

impl Server {
    /// Start `craber serve` on a port the OS picks, and read back which one.
    ///
    /// Asking for `--port 0` and parsing the address the server prints is the
    /// only way to do this without a race: picking a free port here and hoping
    /// it is still free when the child binds it fails whenever two of these
    /// tests run at once, which is the default.
    pub fn start(extra: &[&str]) -> Self {
        let mut args = vec!["serve", "--port", "0"];
        args.extend_from_slice(extra);
        Self::start_in(
            &std::env::current_dir().expect("a working directory"),
            &args,
        )
    }

    /// Start `craber` from `dir` with exactly `args`, and read back the port it
    /// says it bound.
    ///
    /// Unlike [`start`](Self::start) this adds no arguments of its own, which
    /// is what lets a test leave `--port` off entirely and check that the
    /// config file supplied it.
    pub fn start_in(dir: &Path, args: &[&str]) -> Self {
        use std::io::BufRead as _;

        let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_craber"))
            .current_dir(dir)
            .args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("spawning craber serve");

        let stdout = child.stdout.take().expect("a piped stdout");
        let mut server = Self { child, port: 0 };

        // The first line is "CrabER is serving <name> at http://127.0.0.1:<port>/".
        let mut line = String::new();
        std::io::BufReader::new(stdout)
            .read_line(&mut line)
            .expect("reading the startup line");
        server.port = line
            .rsplit_once(':')
            .and_then(|(_, tail)| tail.trim_end().trim_end_matches('/').parse().ok())
            .unwrap_or_else(|| panic!("could not read a port out of {line:?}"));

        server
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// One HTTP response: status code, headers lowercased by name, and the body.
pub struct Response {
    pub status: u16,
    pub headers: std::collections::HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn header(&self, name: &str) -> &str {
        self.headers.get(name).map_or("", String::as_str)
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

/// Send a raw request and read the whole reply.
///
/// Written by hand rather than with an HTTP client because pulling in a client
/// to test a server we wrote precisely to avoid dependencies would be absurd.
pub fn request(port: u16, method: &str, path: &str) -> Response {
    use std::io::{Read, Write};

    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connecting");
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n"
    )
    .expect("writing the request");
    // Half-close, the way a `Connection: close` client does. It tells the
    // server the request is complete, so it never waits for more.
    stream
        .shutdown(std::net::Shutdown::Write)
        .expect("half-closing the request side");

    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).expect("reading the response");

    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("a header/body separator");
    let head = String::from_utf8_lossy(&raw[..split]).into_owned();
    let body = raw[split + 4..].to_vec();

    let mut lines = head.lines();
    let status = lines
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse().ok())
        .expect("a status code");

    let headers = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_owned()))
        .collect();

    Response {
        status,
        headers,
        body,
    }
}

pub fn get(port: u16, path: &str) -> Response {
    request(port, "GET", path)
}

/// The `src`/`href` targets `index.html` asks the browser to load.
pub fn asset_references(html: &str) -> Vec<String> {
    let mut found = Vec::new();
    for attribute in ["src=\"", "href=\""] {
        let mut rest = html;
        while let Some(start) = rest.find(attribute) {
            rest = &rest[start + attribute.len()..];
            let Some(end) = rest.find('"') else { break };
            let value = &rest[..end];
            if value.contains("assets/") {
                found.push(value.to_owned());
            }
            rest = &rest[end..];
        }
    }
    assert!(!found.is_empty(), "index.html loads no assets at all");
    found
}
