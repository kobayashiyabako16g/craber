//! End-to-end: `craber serve` hands the diagram to a browser.
//!
//! The server is hand-written on `std::net`, so HTTP correctness is
//! CrabER's to own. These tests are that ownership: they speak raw HTTP at the
//! real binary rather than going through a client library.

mod common;

use common::{Server, bundle_or_skip, fixture, get, request};

fn tiny() -> Server {
    let path = fixture("tiny.sql");
    Server::start(&["--file", path.to_str().unwrap()])
}

#[test]
fn the_root_is_the_diagram() {
    if !bundle_or_skip() {
        return;
    }
    let server = tiny();
    let response = get(server.port, "/");

    assert_eq!(response.status, 200);
    // A browser will not run an ES module served as a download, and it will not
    // render HTML it was told is one either. This is the header that decides
    // whether the page works at all.
    assert!(
        response.header("content-type").starts_with("text/html"),
        "`/` served as {:?}",
        response.header("content-type")
    );
    let html = response.text();
    assert!(html.contains("<title>tiny — CrabER</title>"), "{html}");
    assert!(html.contains("./assets/"), "{html}");
}

/// Every asset `index.html` asks for has to be there, with a type the browser
/// will execute. This is the check that catches a blank page.
#[test]
fn every_asset_the_page_asks_for_is_served() {
    if !bundle_or_skip() {
        return;
    }
    let server = tiny();
    let index = get(server.port, "/").text();

    let mut found = 0;
    for reference in common::asset_references(&index) {
        let path = reference.trim_start_matches('.');
        let response = get(server.port, path);
        assert_eq!(response.status, 200, "{path} was not served");
        let content_type = response.header("content-type");
        assert!(
            content_type.starts_with("text/javascript") || content_type.starts_with("text/css"),
            "{path} served as {content_type:?}"
        );
        found += 1;
    }
    assert!(found > 0, "index.html references no assets");
}

#[test]
fn the_schema_is_served_as_json() {
    if !bundle_or_skip() {
        return;
    }
    let server = tiny();
    let response = get(server.port, "/schema.json");

    assert_eq!(response.status, 200);
    assert!(
        response
            .header("content-type")
            .starts_with("application/json")
    );

    let schema: serde_json::Value = serde_json::from_slice(&response.body).expect("valid JSON");
    assert_eq!(schema["name"], "tiny");
    assert_eq!(schema["tables"].as_array().expect("tables").len(), 6);
}

/// The frontend keeps its view state in the query string, so this is the shape
/// of a shared link — the path still has to resolve to the page.
#[test]
fn a_shared_link_with_view_state_still_serves_the_page() {
    if !bundle_or_skip() {
        return;
    }
    let server = tiny();
    let response = get(server.port, "/?table=public.users&at=1,2,0.5");
    assert_eq!(response.status, 200);
    assert!(response.header("content-type").starts_with("text/html"));
}

#[test]
fn an_unknown_path_is_a_404() {
    if !bundle_or_skip() {
        return;
    }
    let server = tiny();
    assert_eq!(get(server.port, "/nope").status, 404);
}

/// The server touches the filesystem exactly once, at startup. After that a
/// path is a key in a map, so there is no traversal to defend against — this
/// test pins that property rather than a defence.
#[test]
fn there_is_nothing_to_traverse_to() {
    if !bundle_or_skip() {
        return;
    }
    let server = tiny();
    for path in [
        "/../../../../etc/passwd",
        "/..%2f..%2fetc%2fpasswd",
        "/./././etc/passwd",
    ] {
        let response = get(server.port, path);
        assert_eq!(response.status, 404, "{path}");
        assert!(!response.text().contains("root:"), "{path} returned a file");
    }
}

#[test]
fn head_returns_the_headers_without_the_body() {
    if !bundle_or_skip() {
        return;
    }
    let server = tiny();
    let get_response = get(server.port, "/index.html");
    let head_response = request(server.port, "HEAD", "/index.html");

    assert_eq!(head_response.status, 200);
    assert!(head_response.body.is_empty(), "HEAD returned a body");
    assert_eq!(
        head_response.header("content-length"),
        get_response.body.len().to_string(),
        "HEAD must report the length GET would send"
    );
}

#[test]
fn other_methods_are_refused() {
    if !bundle_or_skip() {
        return;
    }
    let server = tiny();
    for method in ["POST", "PUT", "DELETE"] {
        assert_eq!(request(server.port, method, "/").status, 405, "{method}");
    }
}

/// A predictable URL is worth more than a server that starts anyway somewhere
/// else, so a taken port is an error rather than a silent increment.
#[test]
fn a_taken_port_is_reported_rather_than_worked_around() {
    if !bundle_or_skip() {
        return;
    }
    // Bind port 0 and keep it: whatever the OS hands out is a port that is
    // genuinely taken, with no window for another test to take it first.
    let held = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("holding a port");
    let port = held.local_addr().expect("the held address").port();

    let path = fixture("tiny.sql");
    let output = common::craber(&[
        "serve",
        "--file",
        path.to_str().unwrap(),
        "--port",
        &port.to_string(),
    ]);

    assert!(!output.status.success(), "serve should have refused");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&port.to_string()) && stderr.contains("--port"),
        "the error should name the port and the flag: {stderr}"
    );
}

/// A binary built without the frontend has nothing to serve, and has to say so
/// before it binds anything.
#[test]
fn a_missing_bundle_is_reported_rather_than_served_as_a_blank_page() {
    if craber_site::is_embedded() {
        return;
    }
    let path = fixture("tiny.sql");
    let output = common::craber(&["serve", "--file", path.to_str().unwrap()]);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("web bundle"), "{stderr}");
}
