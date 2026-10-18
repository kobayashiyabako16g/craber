//! `craber.toml`.
//!
//! Precedence is CLI flag > environment variable > config file > default. The
//! file exists so that a repository can commit its source, exclusions and
//! output settings once instead of repeating them in every CI invocation.
//!
//! Keys shared by every command sit at the top level; anything one command
//! alone understands lives in that command's section. The split is not
//! cosmetic: a top-level `output` would mean that `craber export --format json`
//! wrote a file instead of stdout the moment a config file appeared next to it,
//! which would break every pipeline that reads CrabER's output.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use craber_core::Dialect;
use craber_export::Format;
use serde::{Deserialize, Deserializer};

pub const DEFAULT_FILENAME: &str = "craber.toml";

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Config {
    pub file: Option<ConfigPath>,
    pub dsn: Option<String>,
    #[serde(default, deserialize_with = "dialect")]
    pub dialect: Option<Dialect>,
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
    pub depth: Option<usize>,

    #[serde(default)]
    pub build: Build,
    #[serde(default)]
    pub serve: Serve,
    #[serde(default)]
    pub export: Export,

    /// The directory the file was found in, which is what relative paths inside
    /// it are resolved against. Not a key — [`Config::load`] fills it in.
    #[serde(skip)]
    dir: PathBuf,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Build {
    /// Output directory.
    pub output: Option<ConfigPath>,
    /// URL prefix `craber build` writes the asset references against.
    pub base_path: Option<String>,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Serve {
    pub port: Option<u16>,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Export {
    pub format: Option<Format>,
    /// Output file. Absent means stdout, the same as omitting `-o`.
    pub output: Option<ConfigPath>,
}

/// A path read out of `craber.toml`.
///
/// A newtype rather than a bare `PathBuf` because a relative path in the config
/// file is resolved against the file's own directory, not the working
/// directory — and skipping that step produces no error, just a path quietly
/// pointing somewhere else. [`resolve`](Self::resolve) is the only way to get
/// the path out, so the rule travels with the value instead of having to be
/// remembered at each of the three places one is read.
#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
pub struct ConfigPath(PathBuf);

impl ConfigPath {
    /// The path as the file meant it: absolute as written, or relative to the
    /// directory `craber.toml` was found in.
    ///
    /// Against the file rather than the working directory so that `file =
    /// "db/schema.sql"` means the same thing however deep in the repository the
    /// command is run from.
    pub fn resolve(&self, config: &Config) -> PathBuf {
        if self.0.is_absolute() {
            self.0.clone()
        } else {
            config.dir.join(&self.0)
        }
    }
}

impl Config {
    /// Load `path` if given, otherwise the nearest `craber.toml` at or above the
    /// working directory. An explicitly named file that is missing is an error;
    /// finding no default one is not.
    ///
    /// The upward search is what lets a command run from a subdirectory of a
    /// repository and still draw the diagram the repository committed — the
    /// same reason `cargo`, `rustfmt` and friends walk up.
    pub fn load(path: Option<&Path>) -> Result<Self> {
        let cwd = std::env::current_dir().context("reading the working directory")?;

        let path = match path {
            Some(path) => path.to_path_buf(),
            None => match discover(&cwd) {
                Some(found) => found,
                None => {
                    return Ok(Self {
                        dir: cwd,
                        ..Self::default()
                    });
                }
            },
        };

        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading config {}", path.display()))?;
        let mut config: Self =
            toml::from_str(&text).with_context(|| format!("parsing config {}", path.display()))?;

        // `parent` is empty for a bare `craber.toml`, which means "here".
        config.dir = match path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
            _ => cwd,
        };
        Ok(config)
    }
}

/// The nearest `craber.toml` at or above `from`.
fn discover(from: &Path) -> Option<PathBuf> {
    from.ancestors()
        .map(|dir| dir.join(DEFAULT_FILENAME))
        .find(|candidate| candidate.is_file())
}

/// Deserialize `dialect` through [`Dialect`]'s `FromStr`.
///
/// Not a `Deserialize` derive on `Dialect` itself: `FromStr` also accepts
/// `postgresql`, `pg`, `mariadb` and `sqlite3`, and a derive would quietly make
/// the config file reject spellings `--dialect` takes. Parsing here also means
/// a bad dialect is reported when the file is read rather than at whichever of
/// the two use sites happens to be reached first.
fn dialect<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Dialect>, D::Error> {
    let Some(name) = Option::<String>::deserialize(deserializer)? else {
        return Ok(None);
    };
    name.parse().map(Some).map_err(serde::de::Error::custom)
}
