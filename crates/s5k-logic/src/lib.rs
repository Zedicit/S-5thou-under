//! Parsers for Logix routine bodies.
//!
//! Planned modules (milestone M3, `docs/ROADMAP.md`):
//! - `ladder`: neutral rung text, e.g. `XIC(a)[OTE(b),OTL(c)];`
//! - `st`: Structured Text
//! - `fbd`: function block sheets as a wired graph
//! - `sfc`: steps, transitions, branches and links
//! - `catalog`: per-instruction operand read/write semantics

#![forbid(unsafe_code)]
