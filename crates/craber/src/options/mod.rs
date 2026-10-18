//! Command-line flags and `craber.toml`, resolved into what a command needs.
//!
//! Precedence is one rule — the flag wins outright, the config file supplies
//! what the flag omits — and it used to be written out at every option that
//! obeys it: seven `flag.or(config.x)` expressions spread across `main` and
//! `pipeline`. It lives here now, in [`precedence`] and [`dialect`], so the
//! answer to "what beats what" is in one place rather than reconstructed from
//! the call sites.
//!
//! Resolving is also where an invocation is refused. A command that cannot be
//! satisfied — no source anywhere, `--watch` on a connection, a `--dialect`
//! that contradicts the DSN, a binary with no frontend in it — fails while
//! building these structs, before anything is read and before any database is
//! dialled. What a command's `run` receives is therefore an invocation that is
//! already known to make sense.
//!
//! Each `resolve` takes the whole `Args` struct clap parsed rather than a
//! positional copy of its fields, which is what lets `main` be a dispatch and
//! nothing else.

mod dialect;
mod precedence;

use std::path::PathBuf;

use anyhow::{Result, bail};
use craber_core::Dialect;
use craber_export::Format;

use crate::cli::{BuildArgs, ExportArgs, ServeArgs};
use crate::config::{self, Config};
use crate::options::precedence::{pick, pick_source};

/// `serve`'s port when neither the flag nor the config names one.
const DEFAULT_PORT: u16 = 4300;

/// Where the schema comes from, once the question has been settled.
///
/// An enum rather than the pair of `Option`s this used to be: the two are
/// exclusive, and a caller asking "is this a file?" should not have to spell it
/// as "is the first element of the tuple present?".
#[derive(Debug, Clone)]
pub enum Source {
    File(PathBuf),
    Dsn(String),
}

/// Everything [`crate::pipeline::load`] needs: where to read from, in which
/// dialect, and which part of the result to keep.
#[derive(Debug, Clone)]
pub struct Input {
    pub source: Source,
    pub dialect: Dialect,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub depth: Option<usize>,
}

pub struct ExportOptions {
    pub input: Input,
    pub format: Format,
    pub output: Option<PathBuf>,
}

pub struct BuildOptions {
    pub input: Input,
    pub output: PathBuf,
    pub base_path: Option<String>,
}

pub struct ServeOptions {
    pub input: Input,
    pub port: u16,
    /// The file to poll, when `--watch` was asked for. `None` is "do not
    /// watch" — a live connection has already been refused by the time this
    /// exists, so there is no third case for the caller to handle.
    pub watch: Option<PathBuf>,
}

impl ExportArgs {
    pub fn resolve(&self, config: &Config) -> Result<ExportOptions> {
        Ok(ExportOptions {
            input: Input::resolve(&self.source, &self.scope, config)?,
            // `--format` carries no clap default for this reason: a default
            // would be indistinguishable from the user asking for json, and the
            // config could never win.
            format: self.format.or(config.export.format).unwrap_or_default(),
            output: self
                .output
                .clone()
                .or_else(|| config.export.output.as_ref().map(|p| p.resolve(config))),
        })
    }
}

impl BuildArgs {
    pub fn resolve(&self, config: &Config) -> Result<BuildOptions> {
        if self.single_file {
            return Err(crate::milestone::not_yet(
                "`--single-file`",
                "M3",
                Some("drop the flag to write a directory"),
            ));
        }

        // Before the source, so that an invocation missing both is told about
        // the output directory first — the one `build` cannot do anything
        // without, whatever it ends up drawing.
        let Some(output) = self
            .output
            .clone()
            .or_else(|| config.build.output.as_ref().map(|p| p.resolve(config)))
        else {
            bail!(
                "no output directory: pass --output, or set `output` under [build] in {}",
                config::DEFAULT_FILENAME
            )
        };

        let options = BuildOptions {
            input: Input::resolve(&self.source, &self.scope, config)?,
            output,
            base_path: self
                .base_path
                .clone()
                .or_else(|| config.build.base_path.clone()),
        };
        // Last, so it is the invocation that is judged first: a binary built
        // without the frontend cannot write a site, and the user should hear
        // that immediately rather than after a database round trip. Resolving
        // dials nothing, and an invocation that does not make sense is wrong
        // whether or not this binary has a bundle in it.
        craber_site::ensure_embedded()?;
        Ok(options)
    }
}

impl ServeArgs {
    pub fn resolve(&self, config: &Config) -> Result<ServeOptions> {
        // Before the source is required to exist: `--watch --dsn` is wrong for
        // a reason of its own, and saying "no schema source" to an invocation
        // that named one would be a worse answer.
        let watched = match (self.watch, pick_source(&self.source, config)) {
            (false, _) => None,
            (true, Some(Source::File(path))) => Some(path),
            (true, _) => {
                bail!("--watch needs --file: there is nothing to watch for a live connection")
            }
        };

        let options = ServeOptions {
            input: Input::resolve(&self.source, &self.scope, config)?,
            port: self.port.or(config.serve.port).unwrap_or(DEFAULT_PORT),
            watch: watched,
        };
        // As for `build`: nothing to serve without a frontend, and saying so
        // should not cost a database round trip.
        craber_site::ensure_embedded()?;
        Ok(options)
    }
}

impl Input {
    fn resolve(
        source: &crate::cli::Source,
        scope: &crate::cli::Scope,
        config: &Config,
    ) -> Result<Self> {
        let Some(source) = pick_source(source, config) else {
            bail!(
                "no schema source: pass --file or --dsn, or set one in {}",
                config::DEFAULT_FILENAME
            )
        };
        let dialect = dialect::of(&source, scope, config)?;
        Ok(Self {
            source,
            dialect,
            include: pick(&scope.include, &config.include).to_vec(),
            exclude: pick(&scope.exclude, &config.exclude).to_vec(),
            depth: scope.depth.or(config.depth),
        })
    }
}
