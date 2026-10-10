use serde::{Deserialize, Serialize};

/// How serious a [`Diagnostic`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    /// Informational; nothing is wrong.
    Info,
    /// Something was skipped or guessed; results may be incomplete.
    Warning,
    /// Part of the input could not be understood.
    Error,
}

/// A problem found while reading or analysing a project.
///
/// Library crates report problems as diagnostics and keep going wherever they
/// can ("degrade, don't die"). Codes are stable, e.g. `S5K-L5X-0001`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Severity level.
    pub severity: Severity,
    /// Stable machine-readable code.
    pub code: String,
    /// Human-readable message. Must not contain tag values or descriptions.
    pub message: String,
    /// Byte offset in the input file, when known.
    pub offset: Option<u64>,
}

/// An ordered collection of diagnostics.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Diagnostics(Vec<Diagnostic>);

impl Diagnostics {
    /// Record a diagnostic.
    pub fn push(&mut self, d: Diagnostic) {
        self.0.push(d);
    }

    /// All diagnostics in the order they were recorded.
    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.0.iter()
    }

    /// Highest severity recorded, if any.
    pub fn max_severity(&self) -> Option<Severity> {
        self.0.iter().map(|d| d.severity).max()
    }
}
