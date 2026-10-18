//! Partial-failure plumbing.
//!
//! A real schema will contain syntax CrabER cannot yet parse, and a diagram of
//! the 95% we understood is far more useful than an error. So the parsing and
//! introspection layers return an [`Outcome`] — a value *plus* the things that
//! went wrong — instead of a `Result` that throws the whole schema away.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// How badly a [`Diagnostic`] affects the resulting schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Severity {
    /// Something was skipped; the rest of the schema is intact.
    Warning,
    /// Something was lost that the user almost certainly cares about.
    Error,
}

/// One thing that went wrong while building a schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    /// The offending SQL statement or object name, when we have one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

impl Diagnostic {
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            context: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            context: None,
        }
    }

    #[must_use]
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let level = match self.severity {
            Severity::Warning => "warning",
            Severity::Error => "error",
        };
        write!(f, "{level}: {}", self.message)?;
        if let Some(context) = &self.context {
            write!(f, " ({context})")?;
        }
        Ok(())
    }
}

/// A value produced alongside the problems encountered producing it.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome<T> {
    pub value: T,
    pub diagnostics: Vec<Diagnostic>,
}

impl<T> Outcome<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            diagnostics: Vec::new(),
        }
    }

    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// True if anything was lost that the caller should treat as a failure.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }

    /// The error-severity diagnostics, rendered, for a caller that is about to
    /// give up and has to say why.
    ///
    /// Warnings are left out on purpose: they are the ones the caller goes on
    /// to report alongside a diagram that was produced anyway.
    pub fn errors(&self) -> Vec<String> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .map(ToString::to_string)
            .collect()
    }

    pub fn into_parts(self) -> (T, Vec<Diagnostic>) {
        (self.value, self.diagnostics)
    }
}
