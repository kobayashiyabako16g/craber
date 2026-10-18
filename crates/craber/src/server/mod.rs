//! `craber serve` — the diagram on a local HTTP port.
//!
//! The server is hand-written on `std::net`. CrabER's distribution argument is
//! a single binary with no runtime to install, and reaching for `axum` would
//! roughly double a dependency graph that currently has no async runtime in it
//! at all.
//! What `serve` actually has to do — hand a handful of files it already holds
//! in memory to a browser on the same machine — does not need one.
//!
//! The site comes from `craber_site::files`, the same list `craber build`
//! writes to disk, so the page served here and the page published by `build`
//! are the same page.
//!
//! This module owns the listener's lifetime and nothing else: [`routes`] is the
//! site as a URL table, [`http`] is one exchange on a socket, and [`mime`] is
//! the type a filename implies.

mod http;
mod mime;
mod routes;

use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use anyhow::{Context, Result};
use craber_core::Schema;

pub use crate::server::routes::Routes;

/// The site, by URL path. Swapped wholesale when `--watch` reloads.
pub type Site = Arc<RwLock<Arc<Routes>>>;

/// Browsers open connections speculatively and leave them idle. A timeout is
/// what stops each of those from pinning a thread for the life of the tab.
const IDLE_TIMEOUT: Duration = Duration::from_secs(10);

/// Serve `schema` on `port` until the process is stopped.
///
/// Binds to loopback only. This is a tool for looking at your own schema; there
/// is no reason for it to be reachable from the rest of the network, and a
/// diagram of a production database is not something to expose by accident.
pub fn run(schema: &Schema, port: u16) -> Result<Site> {
    let site: Site = Arc::new(RwLock::new(Arc::new(Routes::build(craber_site::files(
        schema, None,
    )?))));
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));

    let listener = TcpListener::bind(address).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AddrInUse {
            anyhow::anyhow!("port {port} is already in use; pass --port to pick another")
        } else {
            anyhow::Error::new(error).context(format!("binding {address}"))
        }
    })?;

    // Ask the listener rather than echoing `port`, so `--port 0` reports the
    // port the OS actually chose. The tests rely on this.
    let bound = listener.local_addr().context("reading the bound address")?;
    println!("CrabER is serving {} at http://{bound}/", schema.name);

    let accepting = Arc::clone(&site);
    std::thread::spawn(move || accept_loop(&listener, &accepting));

    Ok(site)
}

/// Replace what is being served, for `--watch`.
pub fn swap(site: &Site, schema: &Schema) -> Result<()> {
    let routes = Arc::new(Routes::build(craber_site::files(schema, None)?));
    *site.write().expect("the site lock is never poisoned") = routes;
    Ok(())
}

/// Hand each connection the site as it stands, on a thread of its own.
///
/// One thread per connection. A browser opens a handful and asks for five
/// files; a pool would be machinery without a problem to solve.
fn accept_loop(listener: &TcpListener, site: &Site) {
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let current = Arc::clone(&site.read().expect("the site lock is never poisoned"));
        std::thread::spawn(move || serve_one(&stream, &current));
    }
}

fn serve_one(stream: &TcpStream, routes: &Routes) {
    let _ = stream.set_read_timeout(Some(IDLE_TIMEOUT));
    let _ = stream.set_write_timeout(Some(IDLE_TIMEOUT));
    let _ = stream.set_nodelay(true);

    if let Err(error) = http::handle(stream, routes) {
        // A client that navigated away mid-response is ordinary, not something
        // to take the server down for.
        tracing::debug!(%error, "connection closed early");
    }
    http::close(stream);
}
