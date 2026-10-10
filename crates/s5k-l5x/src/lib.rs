//! Streaming reader for Studio 5000 `.L5X` exports.
//!
//! The reader never builds a DOM of the file (NFR-PERF-003): it walks
//! quick-xml events and maps each element straight into `s5k_model::Project`.
//! Routine bodies are kept as raw rung text, ST lines, or FBD/SFC element
//! trees; `s5k-logic` parses them later.
//!
//! ```no_run
//! use std::{fs::File, io::BufReader};
//! let file = File::open("project.L5X")?;
//! let parsed = s5k_l5x::read_project(BufReader::new(file), &s5k_l5x::NoProgress)?;
//! println!("{} programs", parsed.project.programs.len());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

#![forbid(unsafe_code)]

mod project;
mod xml;

use std::io::BufRead;

use s5k_model::{Diagnostic, Diagnostics, ExportHeader, ExportTarget, Project, Severity};

use crate::project::Parser;
use crate::xml::{El, Xml};

/// Stable diagnostic codes emitted by this crate. Documented in `docs/DIAGNOSTICS.md`.
pub mod codes {
    /// The XML became malformed or ended early; the project is partial.
    pub const MALFORMED_XML: &str = "S5K-L5X-0000";
    /// An element this version does not understand was skipped.
    pub const UNKNOWN_ELEMENT: &str = "S5K-L5X-0001";
    /// An attribute value could not be interpreted.
    pub const BAD_ATTRIBUTE: &str = "S5K-L5X-0002";
    /// A rung carries a pending online-edit type.
    pub const PENDING_EDIT: &str = "S5K-L5X-0003";
    /// Source-protected (encrypted) content; logic is opaque.
    pub const SOURCE_PROTECTED: &str = "S5K-L5X-0004";
    /// An element that needs a `Name` attribute has none and was skipped.
    pub const MISSING_NAME: &str = "S5K-L5X-0005";
}

/// Errors that stop reading before anything useful was found.
///
/// Problems after the root element was read become diagnostics instead, and
/// [`read_project`] returns the partial project.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The XML itself is malformed.
    #[error("malformed XML at byte {offset}: {source}")]
    Xml {
        /// Byte offset where the error was detected.
        offset: u64,
        /// Underlying parser error.
        #[source]
        source: quick_xml::Error,
    },
    /// The input ended inside an element.
    #[error("unexpected end of file inside <{element}>")]
    UnexpectedEof {
        /// Element that was still open.
        element: String,
    },
    /// The document has no `RSLogix5000Content` root element.
    #[error("not an L5X file: root element `RSLogix5000Content` not found")]
    NotL5x,
}

/// Receives parse progress (FR-PAR-015). Implementations must be cheap: it is
/// called once per top-level item (tag, routine, type ...).
pub trait Progress {
    /// `bytes` of input have been consumed so far.
    fn advance(&self, bytes: u64);
}

/// A [`Progress`] that ignores updates.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoProgress;

impl Progress for NoProgress {
    fn advance(&self, _bytes: u64) {}
}

/// Result of [`read_project`].
#[derive(Debug)]
pub struct Parsed {
    /// Everything that could be read.
    pub project: Project,
    /// Warnings and errors found while reading.
    pub diagnostics: Diagnostics,
    /// True when reading stopped early (malformed or truncated XML). The
    /// project then holds every element completed before that point.
    pub truncated: bool,
}

const ROOT: &str = "RSLogix5000Content";

/// Read the first element and require it to be the L5X root.
fn open_root<R: BufRead>(xml: &mut Xml<R>) -> Result<El, Error> {
    match xml.next_top_level()? {
        Some(el) if el.name == ROOT => Ok(el),
        _ => Err(Error::NotL5x),
    }
}

/// Read only the `RSLogix5000Content` root attributes and stop.
///
/// Cheap even for very large files: it returns as soon as the root element's
/// start tag has been read.
pub fn read_header<R: BufRead>(input: R) -> Result<ExportHeader, Error> {
    let mut xml = Xml::new(input);
    let root = open_root(&mut xml)?;
    Ok(header_from(&root))
}

/// Read a complete or partial L5X export into a [`Project`].
///
/// Fails only when the input is not L5X at all. Malformed XML later in the
/// file yields a partial project, `truncated = true` and an error diagnostic.
pub fn read_project<R: BufRead>(input: R, progress: &dyn Progress) -> Result<Parsed, Error> {
    let mut xml = Xml::new(input);
    let root = open_root(&mut xml)?;
    let mut parser = Parser::new(xml, progress);
    parser.project.header = Some(header_from(&root));

    let outcome = parser.root(&root).and_then(|()| parser.trailing());
    progress.advance(parser.xml.position());
    let Parser {
        project, mut diags, ..
    } = parser;
    let truncated = match outcome {
        Ok(()) => false,
        Err(err) => {
            let offset = match &err {
                Error::Xml { offset, .. } => Some(*offset),
                _ => None,
            };
            diags.push(Diagnostic::new(
                Severity::Error,
                codes::MALFORMED_XML,
                format!("reading stopped early, results are partial: {err}"),
                offset,
            ));
            true
        }
    };
    Ok(Parsed {
        project,
        diagnostics: diags,
        truncated,
    })
}

fn header_from(root: &El) -> ExportHeader {
    let get = |k: &str| root.get(k).map(str::to_owned);
    ExportHeader {
        schema_revision: get("SchemaRevision"),
        software_revision: get("SoftwareRevision"),
        target_name: get("TargetName"),
        target_type: root.get("TargetType").map(ExportTarget::from_attr),
        contains_context: root
            .get("ContainsContext")
            .is_some_and(|v| v.eq_ignore_ascii_case("true")),
        export_date: get("ExportDate"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fr_par_002_reads_header_of_minimal_controller() {
        let xml = include_str!("../../../tests/fixtures/minimal_controller.L5X");
        let h = read_header(xml.as_bytes()).unwrap();
        assert_eq!(h.software_revision.as_deref(), Some("33.01"));
        assert_eq!(h.target_name.as_deref(), Some("Demo_Line"));
        assert_eq!(h.target_type, Some(ExportTarget::Controller));
        assert!(!h.contains_context);
    }

    #[test]
    fn rejects_non_l5x_xml() {
        let err = read_header(&b"<?xml version=\"1.0\"?><Other/>"[..]).unwrap_err();
        assert!(matches!(err, Error::NotL5x));
        let err = read_project(&b"<Other/>"[..], &NoProgress).unwrap_err();
        assert!(matches!(err, Error::NotL5x));
    }

    #[test]
    fn rejects_empty_input() {
        assert!(matches!(read_header(&b""[..]), Err(Error::NotL5x)));
    }
}
