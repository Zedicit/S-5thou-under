//! Streaming reader for Studio 5000 `.L5X` exports.
//!
//! The reader never builds a DOM of the file (NFR-PERF-003). Currently it reads
//! the export header (FR-PAR-002 subset); the full structural parse is
//! milestone M1 in `docs/ROADMAP.md`.

#![forbid(unsafe_code)]

use std::io::BufRead;

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use s5k_model::{ExportHeader, ExportTarget};

/// Errors that stop reading entirely. Recoverable problems become diagnostics.
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
    /// The document has no `RSLogix5000Content` root element.
    #[error("not an L5X file: root element `RSLogix5000Content` not found")]
    NotL5x,
}

const ROOT: &[u8] = b"RSLogix5000Content";

/// Read only the `RSLogix5000Content` root attributes and stop.
///
/// This is cheap even for very large files: it returns as soon as the root
/// element's start tag has been read.
pub fn read_header<R: BufRead>(input: R) -> Result<ExportHeader, Error> {
    let mut reader = Reader::from_reader(input);
    let mut buf = Vec::new();
    loop {
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|source| Error::Xml {
                offset: reader.buffer_position(),
                source,
            })?;
        match event {
            Event::Start(e) | Event::Empty(e) if e.local_name().as_ref() == ROOT => {
                return Ok(header_from(&e));
            }
            // The first element is not the L5X root.
            Event::Start(_) | Event::Empty(_) | Event::Eof => return Err(Error::NotL5x),
            _ => {}
        }
        buf.clear();
    }
}

fn header_from(e: &BytesStart<'_>) -> ExportHeader {
    let mut h = ExportHeader {
        schema_revision: None,
        software_revision: None,
        target_name: None,
        target_type: None,
        contains_context: false,
        export_date: None,
    };
    // Malformed individual attributes are skipped rather than failing the read.
    for attr in e.attributes().flatten() {
        let Ok(value) = attr.unescape_value() else {
            continue;
        };
        match attr.key.as_ref() {
            b"SchemaRevision" => h.schema_revision = Some(value.into_owned()),
            b"SoftwareRevision" => h.software_revision = Some(value.into_owned()),
            b"TargetName" => h.target_name = Some(value.into_owned()),
            b"TargetType" => h.target_type = Some(ExportTarget::from_attr(&value)),
            b"ContainsContext" => h.contains_context = value.eq_ignore_ascii_case("true"),
            b"ExportDate" => h.export_date = Some(value.into_owned()),
            _ => {}
        }
    }
    h
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
    }

    #[test]
    fn rejects_empty_input() {
        assert!(matches!(read_header(&b""[..]), Err(Error::NotL5x)));
    }
}
