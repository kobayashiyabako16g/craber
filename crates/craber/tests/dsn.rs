//! `--dsn` at the level the user meets it: argv, exit status and streams.
//!
//! None of this needs a database. What it covers is everything CrabER decides
//! *before* connecting, plus the one thing that must hold when connecting fails.

mod common;

use common::craber;

/// The password in a connection string must not reach stderr, and stderr is
/// where CrabER writes everything — including into a CI log.
///
/// sqlx's configuration error wraps the `url` crate's, which echoes its input,
/// so this is a real path rather than a hypothetical one.
#[test]
fn a_failed_connection_does_not_print_the_password() {
    // Port 1 on loopback: nothing listens, so this fails fast and locally.
    let output = craber(&[
        "export",
        "--dsn",
        "postgres://someone:hunter2@127.0.0.1:1/nowhere",
    ]);
    assert!(
        !output.status.success(),
        "connecting to a closed port should fail"
    );

    let streams = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !streams.contains("hunter2"),
        "the password reached a stream:\n{streams}"
    );
    assert!(
        streams.contains("postgres database"),
        "the error should still say what it was reading:\n{streams}"
    );
}

/// Speaking MySQL to a PostgreSQL port cannot work, and silently picking one of
/// the two turns into a long afternoon with a connection error.
#[test]
fn a_dialect_that_contradicts_the_scheme_is_refused() {
    let output = craber(&[
        "export",
        "--dsn",
        "postgres://127.0.0.1/db",
        "--dialect",
        "mysql",
    ]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("contradicts the connection string"),
        "{stderr}"
    );
}

#[test]
fn a_scheme_nobody_recognises_asks_for_a_dialect() {
    let output = craber(&["export", "--dsn", "oracle://127.0.0.1/db"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("pass --dialect"), "{stderr}");
}

/// The scheme is enough on its own — this gets as far as trying to connect,
/// which is further than a dialect failure would.
#[test]
fn the_scheme_alone_settles_the_dialect() {
    let output = craber(&["export", "--dsn", "mysql://127.0.0.1:1/db"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("mysql database"), "{stderr}");
}

/// `--watch` has nothing to watch for a connection, and must not fall back to
/// whatever file `craber.toml` happens to name.
#[test]
fn watching_a_live_connection_is_refused() {
    let output = craber(&[
        "serve",
        "--port",
        "0",
        "--dsn",
        "sqlite://nope.db",
        "--watch",
    ]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--watch needs --file"), "{stderr}");
}
