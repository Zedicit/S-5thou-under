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
- `cargo run -p s5k-cli -- inspect tests/fixtures/multi_program_output.L5X -v`
- `cargo xtask gen-fixture --size-mb 100 -o target/big.L5X` (perf checks)

## Rules
- Streaming XML only (quick-xml events), never a DOM.
- Libraries: no unwrap/expect on input data, no printing; return errors plus `Diagnostics`.
- Logix names are case-insensitive: use `s5k_model::Name`.
- Never commit real customer L5X/L5K/ACD files; fixtures must be synthetic or sanitized.
- Never log tag values or descriptions above debug level.
- New diagnostic codes go in `crates/s5k-l5x/src/lib.rs::codes` and `docs/DIAGNOSTICS.md`.
- The goal is tracing how outputs turn on across the whole project (ADR-0002); never design an analysis that only works within one program.
