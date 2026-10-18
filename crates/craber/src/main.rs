//! The `craber` binary.
//!
//! This file is the dispatch and nothing else: start logging, find the config,
//! hand the invocation to the command that does the work. The three steps a
//! command shares — resolve, load, report — live in [`options`], [`pipeline`]
//! and [`commands`] respectively, and refusing an invocation that cannot be
//! satisfied is part of resolving it.

mod cli;
mod commands;
mod config;
mod milestone;
mod options;
mod pipeline;
mod server;
mod watch;

use anyhow::Result;
use clap::Parser;

use crate::cli::{Cli, Command};
use crate::config::Config;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "craber=info".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    let config = Config::load(cli.config.as_deref())?;

    match &cli.command {
        Command::Export(args) => commands::export::run(&args.resolve(&config)?),
        Command::Build(args) => commands::build::run(&args.resolve(&config)?),
        Command::Serve(args) => commands::serve::run(&args.resolve(&config)?),
        Command::Diff(_) => Err(milestone::not_yet("`craber diff`", "M7", None)),
    }
}
