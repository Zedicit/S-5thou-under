# CLAUDE.md

Rust workspace that reads Studio 5000 `.L5X` exports and produces diagrams
(call trees, sequence/state charts, UDT composition) and cross-references.

## Read first
- `docs/ARCHITECTURE.md`: pipeline and crate boundaries (respect the "Must not" column).
- `docs/DOMAIN_NOTES.md`: L5X structure, Logix data types, scoping rules.
- `docs/REQUIREMENTS.md`: requirement IDs; reference them in tests and commits.
- `docs/ROADMAP.md`: current milestone.

## Commands
- `cargo fmt --all`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo run -p s5k-cli -- inspect tests/fixtures/minimal_controller.L5X`

## Rules
- Streaming XML only (quick-xml events), never a DOM.
- Libraries: no unwrap/expect on input data, no printing; return errors plus `Diagnostics`.
- Logix names are case-insensitive: use `s5k_model::Name`.
- Never commit real customer L5X/L5K/ACD files; fixtures must be synthetic or sanitized.
- Never log tag values or descriptions above debug level.
