# Roadmap

Each milestone ends in a tagged release with its exit criteria met in CI.

## M0: Foundations (this commit)
- Workspace, CI, lint/format policy, documentation set.
- **Exit:** `cargo build/test/clippy/fmt` green on Linux, Windows and macOS.

## M1: Structural parse (FR-PAR-001…010, 014)
- Streaming L5X reader into `s5k-model`: controller, data types, AOIs, tags, programs, routines (raw bodies), tasks.
- `s5k inspect` prints a summary and counts.
- Synthetic fixture generator (`xtask gen-fixture --size 100MB`).
- **Exit:** all fixtures parse; fuzz target runs 10 min without crash; 100 MB synthetic file parses in < 20 s.

## M2: Types and symbols (FR-ANA-001…004)
- Built-in predefined type table; UDT/AOI composition; BOOL host packing; scoped symbol tables; alias resolution.
- `s5k graph --kind udt` (Mermaid classDiagram, DOT).
- **Exit:** every tag in the fixtures resolves to a type; cycle/missing type diagnostics tested.

## M3: Logic parsing (FR-LOG-001…007)
- Ladder neutral-text parser, ST parser, FBD and SFC graph builders, instruction catalogue (start with the ~80 most common instructions, fall back to "unknown: read+write all operands").
- **Exit:** 100% of rungs/lines in the fixtures parse or produce a documented diagnostic; property test round-trips ladder text.

## M4: Cross-reference and call graph (FR-ANA-005, 006, 010)
- `s5k xref`, `s5k graph --kind call-tree`, unreachable routine report, InOut propagation through AOIs.
- `s5k index` cache.
- **Exit:** xref matches hand-checked expectations on fixture projects; 100 MB file indexes in < 60 s.

## M5: Sequences (FR-ANA-007, 008)
- SFC → state chart. Step-integer and step-bit heuristics; `s5k.toml` overrides.
- **Exit:** extracted charts for fixture sequences match the expected golden files.

## M6: Report and 1.0 (FR-OUT-005, FR-CLI-005)
- Static HTML report, `s5k sanitize`, user documentation, Windows installer/zip.
- **Exit:** user review on at least two real projects; docs complete; release binaries published.

## Post 1.0 candidates
- L5K input, project diff, lint rules, data-flow graph (FR-ANA-009), interactive web viewer, LSP/VS Code extension, PhaseManager/ISA-88 views.
