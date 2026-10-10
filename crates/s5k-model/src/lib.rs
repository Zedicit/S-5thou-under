//! Format-agnostic domain model for Rockwell Logix 5000 projects.
//!
//! Every input format (L5X today, possibly L5K later) produces these types,
//! and every analysis and renderer consumes them. See `docs/ARCHITECTURE.md`.

#![forbid(unsafe_code)]

mod diagnostics;
mod name;
mod types;

pub use diagnostics::{Diagnostic, Diagnostics, Severity};
pub use name::Name;
pub use types::{AtomicType, TypeCategory};

use serde::{Deserialize, Serialize};

/// What an L5X file contains, from the root `TargetType` attribute.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportTarget {
    /// A whole controller project.
    Controller,
    /// A single program, with context if exported that way.
    Program,
    /// A single routine.
    Routine,
    /// A single user-defined type.
    DataType,
    /// A single Add-On Instruction definition.
    AddOnInstructionDefinition,
    /// One or more rungs.
    Rung,
    /// A value this version of the tool does not know.
    Other(String),
}

impl ExportTarget {
    /// Map the raw `TargetType` attribute value.
    pub fn from_attr(value: &str) -> Self {
        match value {
            "Controller" => Self::Controller,
            "Program" => Self::Program,
            "Routine" => Self::Routine,
            "DataType" => Self::DataType,
            "AddOnInstructionDefinition" => Self::AddOnInstructionDefinition,
            "Rung" => Self::Rung,
            other => Self::Other(other.to_owned()),
        }
    }
}

/// Export metadata from the `RSLogix5000Content` root element.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportHeader {
    /// `SchemaRevision`, e.g. `1.0`.
    pub schema_revision: Option<String>,
    /// `SoftwareRevision` of Studio 5000 that wrote the file, e.g. `33.01`.
    pub software_revision: Option<String>,
    /// `TargetName`: name of the exported controller, program, routine, etc.
    pub target_name: Option<String>,
    /// `TargetType`.
    pub target_type: Option<ExportTarget>,
    /// `ContainsContext`: whether dependencies of a partial export are included.
    pub contains_context: bool,
    /// `ExportDate` as written in the file.
    pub export_date: Option<String>,
}
