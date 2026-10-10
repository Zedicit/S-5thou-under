# S-5thou-under

**Studio 5000 Understand**: a Rust toolchain that reads Rockwell Automation
Studio 5000 Logix Designer project exports (`.L5X`) and turns them into
diagrams people can read: call graphs, sequence/state charts, data-flow and
data-type diagrams.

> Status: **pre-alpha, design phase.** The workspace compiles, but the crates
> are mostly stubs. See [`docs/ROADMAP.md`](docs/ROADMAP.md).

## Why

Large Logix projects contain hundreds of routines, thousands of rungs, many
Add-On Instructions (AOIs) and deeply nested User-Defined Types (UDTs). Studio
5000 is good for editing one routine at a time. It is poor at answering
"what does this machine actually *do*, and in what order?" This project answers
that question offline, from the export file alone.

## Planned usage

```text
s5k inspect  project.L5X                       # summary: controller, tasks, programs, counts
s5k index    project.L5X -o project.s5kidx     # parse once, cache the analysed model
s5k graph    project.s5kidx --kind call-tree   --format mermaid
s5k graph    project.s5kidx --kind sequence    --program Filler --format plantuml
s5k graph    project.s5kidx --kind udt         --root Recipe_UDT --format dot
s5k xref     project.s5kidx --tag Conveyor1.Run          # who reads/writes this tag
s5k report   project.s5kidx -o report/                   # static HTML site with all diagrams
```

## Input format choice: L5X, not L5K

L5X (XML) is the primary and, for v1, the only supported input. Summary of the
reasons (full record in [`docs/adr/0001-l5x-as-primary-input.md`](docs/adr/0001-l5x-as-primary-input.md)):

- It carries everything L5K carries, plus structured FBD/SFC graphics (blocks,
  wires, steps, transitions, links, with IDs), localized descriptions, rung
  comments as separate elements, and explicit export metadata.
- It supports partial exports (one Program, Routine, AOI, UDT or rung set),
  which are ideal for tests and for users who cannot share a full project.
- It is XML, so the outer structure has a stable, well-documented shape and
  existing streaming parsers. L5K needs a hand-written grammar for the whole file.

## Repository layout

```text
crates/
  s5k-model     Core domain types (Controller, Task, Program, Routine, Tag, DataType, AOI ...)
  s5k-l5x       Streaming L5X reader → s5k-model
  s5k-logic     Parsers for routine bodies: ladder neutral text, ST, FBD, SFC
  s5k-analysis  Type resolution, symbol tables, cross-reference, call graph, sequence extraction
  s5k-render    Diagram back-ends: Mermaid, Graphviz DOT, PlantUML, HTML report
  s5k-cli       The `s5k` binary
docs/           Charter, requirements, architecture, domain notes, roadmap, ADRs
tests/fixtures  Small, sanitized L5X samples (never commit customer projects)
```

## Building

```bash
cargo build --workspace
cargo test  --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Documentation

| Document | Purpose |
|---|---|
| [`docs/PROJECT_CHARTER.md`](docs/PROJECT_CHARTER.md) | Vision, scope, non-goals, stakeholders, success criteria |
| [`docs/REQUIREMENTS.md`](docs/REQUIREMENTS.md) | Functional and non-functional requirements with IDs |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | Pipeline, crate boundaries, data model, performance strategy |
| [`docs/DOMAIN_NOTES.md`](docs/DOMAIN_NOTES.md) | L5X structure, Logix data types, and how they interact |
| [`docs/ROADMAP.md`](docs/ROADMAP.md) | Milestones and exit criteria |
| [`docs/OPEN_QUESTIONS.md`](docs/OPEN_QUESTIONS.md) | Decisions still needed, gaps in the original brief |
| [`docs/TESTING.md`](docs/TESTING.md) | Test strategy, fixture policy, benchmarks |
| [`CONTRIBUTING.md`](CONTRIBUTING.md) | Workflow, commit and code conventions |

## Disclaimer

Not affiliated with or endorsed by Rockwell Automation. "Studio 5000",
"Logix Designer" and "RSLogix" are trademarks of Rockwell Automation, Inc.
This tool only reads export files; it never connects to a controller.

## License

To be decided. See [`docs/OPEN_QUESTIONS.md`](docs/OPEN_QUESTIONS.md).
