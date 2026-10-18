//! The `craber` command surface.
//!
//! The full command set is declared here even though most of it is not built
//! yet, so that the interface users script against is settled before anyone
//! depends on it. Unimplemented subcommands fail loudly with the milestone they
//! are scheduled for rather than silently doing nothing.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use craber_core::Dialect;
use craber_export::Format;

#[derive(Debug, Parser)]
#[command(
    name = "craber",
    version,
    about = "CrabER — visualize ER diagrams from SQL DDL or a live database",
    max_term_width = 100
)]
pub struct Cli {
    /// Path to craber.toml. Defaults to ./craber.toml when it exists.
    #[arg(long, global = true, value_name = "PATH")]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Serve an interactive diagram on a local HTTP port.
    Serve(ServeArgs),

    /// Write a static site that can be deployed as-is.
    Build(BuildArgs),

    /// Export the diagram as an image or a text format.
    Export(ExportArgs),

    /// Report what changed between two schemas.
    Diff(DiffArgs),
}

/// One struct per subcommand rather than fields on the variant, so that
/// [`resolve`](crate::options) takes the invocation itself instead of a
/// positional copy of each of its fields. `main` then has nothing to destructure
/// and no argument order to get wrong.
#[derive(Debug, Args)]
pub struct ServeArgs {
    #[command(flatten)]
    pub source: Source,
    #[command(flatten)]
    pub scope: Scope,
    /// Port to listen on. Defaults to 4300.
    #[arg(long)]
    pub port: Option<u16>,
    /// Rebuild when the input file changes.
    #[arg(long)]
    pub watch: bool,
}

#[derive(Debug, Args)]
pub struct BuildArgs {
    #[command(flatten)]
    pub source: Source,
    #[command(flatten)]
    pub scope: Scope,
    /// Output directory, or the HTML file when --single-file is set.
    #[arg(short, long, value_name = "PATH")]
    pub output: Option<PathBuf>,
    /// Inline every asset into one self-contained HTML file.
    #[arg(long)]
    pub single_file: bool,
    /// URL prefix the site is served under, e.g. /myrepo/.
    #[arg(long, value_name = "PATH")]
    pub base_path: Option<String>,
}

#[derive(Debug, Args)]
pub struct ExportArgs {
    #[command(flatten)]
    pub source: Source,
    #[command(flatten)]
    pub scope: Scope,
    /// Output format. Defaults to json.
    #[arg(long, value_enum)]
    pub format: Option<Format>,
    /// Output file. Writes to stdout when omitted.
    #[arg(short, long, value_name = "PATH")]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct DiffArgs {
    #[arg(long, value_name = "PATH")]
    pub from: PathBuf,
    #[arg(long, value_name = "PATH")]
    pub to: PathBuf,
}

/// Where the schema comes from. At most one of these — when neither is given,
/// `craber.toml` supplies it, and `pipeline::load` is what refuses an
/// invocation that has no source anywhere.
#[derive(Debug, Clone, Args)]
#[group(required = false, multiple = false)]
pub struct Source {
    /// Read DDL from a .sql file.
    #[arg(short, long, value_name = "PATH")]
    pub file: Option<PathBuf>,

    /// Connect to a live database.
    ///
    /// Prefer the environment variable in CI so the DSN never lands in a
    /// command line or a shell history.
    #[arg(long, value_name = "URL", env = "CRABER_DSN", hide_env_values = true)]
    pub dsn: Option<String>,
}

/// Which part of the schema to draw.
#[derive(Debug, Clone, Args)]
pub struct Scope {
    /// SQL dialect. Inferred from the filename or the DSN when omitted.
    #[arg(long, value_name = "postgres|mysql|sqlite")]
    pub dialect: Option<Dialect>,

    /// Only include tables matching these globs. Repeatable.
    #[arg(long, value_name = "GLOB")]
    pub include: Vec<String>,

    /// Drop tables matching these globs, even if --include matched. Repeatable.
    #[arg(long, value_name = "GLOB")]
    pub exclude: Vec<String>,

    /// Also include tables within N foreign-key hops of the included ones.
    #[arg(long, value_name = "N")]
    pub depth: Option<usize>,
}
