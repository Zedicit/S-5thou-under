# Requirements

IDs are stable. Reference them in issues, commits and tests (for example
`test_fr_par_007_partial_export`). Priority: **M** = must (v1.0), **S** = should,
**C** = could (later).

## Functional requirements

### Parsing (FR-PAR)

| ID | Pri | Requirement |
|---|---|---|
| FR-PAR-001 | M | Read a full-project L5X (`RSLogix5000Content` with `TargetType="Controller"`). |
| FR-PAR-002 | M | Read the controller header: name, processor type, `SoftwareRevision`, export date, export options. |
| FR-PAR-003 | M | Read all `DataTypes` (UDTs), including members, array dimensions, hidden BOOL host members, descriptions. |
| FR-PAR-004 | M | Read all `AddOnInstructionDefinitions`: parameters (usage, type, required/visible), local tags, all routines (Logic, Prescan, EnableInFalse, Postscan), revision, and nested AOI usage. |
| FR-PAR-005 | M | Read controller-scope and program-scope `Tags`: base, alias, produced, consumed; data type, dimensions, external access, constant flag, descriptions, initial values. |
| FR-PAR-006 | M | Read `Programs` (and `EquipmentPhases` where present): main routine, fault routine, disabled flag, parameters, routines. |
| FR-PAR-007 | M | Read partial exports (`TargetType` = Program, Routine, DataType, AddOnInstructionDefinition, Rung). |
| FR-PAR-008 | M | Read `Tasks`: type (continuous/periodic/event), period, priority, watchdog, scheduled programs and their order. |
| FR-PAR-009 | M | Keep routine bodies for all four languages: RLL rungs (neutral text + comments), ST lines, FBD sheets, SFC content. |
| FR-PAR-010 | M | Detect source-protected or encoded content (`EncodedData`, `SourceProtected`) and mark it opaque instead of failing. |
| FR-PAR-011 | S | Read `Modules` (I/O tree), so module-defined tag types (e.g. `Local:1:I`) resolve. |
| FR-PAR-012 | S | Read decorated tag data (`Format="Decorated"`) for initial values; keep `Format="L5K"` data as raw text. |
| FR-PAR-013 | C | Read L5K input (see ADR-0001). |
| FR-PAR-014 | M | Unknown elements/attributes produce a warning with location, never a failure. |

### Logic parsing (FR-LOG)

| ID | Pri | Requirement |
|---|---|---|
| FR-LOG-001 | M | Parse ladder neutral text (`XIC(a)[OTE(b),OTL(c)];`) into an AST with branches and instruction operands. |
| FR-LOG-002 | M | Parse Structured Text into an AST (assignments, IF/CASE/FOR/WHILE/REPEAT, function and AOI calls, comments). |
| FR-LOG-003 | M | Model FBD sheets as a graph: blocks, input/output references, wires, feedback, AOI blocks. |
| FR-LOG-004 | M | Model SFC: steps, transitions, branches (selection/simultaneous), directed links, actions, initial step. |
| FR-LOG-005 | M | Instruction catalogue that knows, per instruction, which operands are read, written, or both (e.g. `MOV` reads source, writes dest; `TON` reads and writes a `TIMER`). |
| FR-LOG-006 | M | Parse operand expressions: tag paths, member access, array indexing (constant and indirect), bit access (`.5`, `[idx]`), literals, CPT/CMP expressions. |
| FR-LOG-007 | M | Every AST node keeps a source location (routine, rung number or line number, block ID). |

### Analysis (FR-ANA)

| ID | Pri | Requirement |
|---|---|---|
| FR-ANA-001 | M | Build a type table: atomic, predefined (TIMER, COUNTER, CONTROL, MESSAGE, STRING, PID ...), module-defined, UDT, AOI. Detect cycles and missing types. |
| FR-ANA-002 | M | Compute UDT/AOI dependency order and composition graph. |
| FR-ANA-003 | M | Build scoped symbol tables (controller → program → AOI instance) with shadowing rules. |
| FR-ANA-004 | M | Resolve every operand to a symbol and type; resolve aliases to their base tag. Unresolved references become diagnostics. |
| FR-ANA-005 | M | Cross-reference: for each tag and member, list reads, writes, and the location and instruction of each. |
| FR-ANA-006 | M | Call graph: Task → Program → MainRoutine → JSR targets (with parameters), AOI invocations, fault and power-up handlers. Flag unreachable routines. |
| FR-ANA-007 | M | SFC → state chart directly from the SFC model. |
| FR-ANA-008 | S | Ladder/ST sequence extraction: detect step-variable patterns (`EQU Step 10 ... MOV 20 Step`, `CASE Step OF`) and latch/unlatch step bits; output states and guarded transitions with links to source. |
| FR-ANA-009 | S | Data flow: tag-to-tag influence graph (which inputs affect which outputs). |
| FR-ANA-010 | S | Inter-program communication: program parameters, produced/consumed tags, controller-scope tags used by more than one program, MSG instructions. |
| FR-ANA-011 | C | Diff two models: added/removed/changed routines, rungs, tags and types. |
| FR-ANA-012 | C | Lint-style findings: duplicate destructive writes (OTE to same bit in two places), unused tags, unused routines, never-written tags that are read. |

### Output (FR-OUT)

| ID | Pri | Requirement |
|---|---|---|
| FR-OUT-001 | M | Mermaid output (flowchart, stateDiagram, classDiagram, sequenceDiagram). |
| FR-OUT-002 | M | Graphviz DOT output (scales better for large graphs). |
| FR-OUT-003 | S | PlantUML output. |
| FR-OUT-004 | M | JSON export of the analysed model and of every graph (stable, versioned schema). |
| FR-OUT-005 | S | Static HTML report: project overview, per-program pages, diagrams, searchable cross-reference. Works offline, no server. |
| FR-OUT-006 | M | Diagram scoping: by task/program/routine, depth limit, include/exclude patterns, collapse AOIs. |
| FR-OUT-007 | M | Diagram labels can use tag/rung descriptions; every node links back to source location. |

### CLI (FR-CLI)

| ID | Pri | Requirement |
|---|---|---|
| FR-CLI-001 | M | Subcommands: `inspect`, `index`, `graph`, `xref`, `report`, `validate`. |
| FR-CLI-002 | M | Progress bar for long operations; `--quiet` and `--json` modes for scripting. |
| FR-CLI-003 | M | Exit codes: 0 success, 1 diagnostics at `error` level, 2 usage error, 3 I/O or parse failure. |
| FR-CLI-004 | S | Config file (`s5k.toml`) for sequence-pattern rules, naming conventions and diagram defaults. |
| FR-CLI-005 | S | `s5k sanitize` to rename tags/descriptions in an L5X so users can share bug-repro files. |

## Non-functional requirements

| ID | Pri | Requirement |
|---|---|---|
| NFR-PERF-001 | M | Parse + index a 100 MB L5X in ≤ 60 s on a 4-core / 16 GB machine. |
| NFR-PERF-002 | M | Peak memory ≤ 20× input file size, target ≤ 10×. |
| NFR-PERF-003 | M | Streaming XML read; never build a full DOM of the file. |
| NFR-PERF-004 | M | Analysis passes that are independent per routine/program run in parallel. |
| NFR-PERF-005 | S | Cached index: re-running `graph` on an unchanged file must not re-parse (cache keyed on file hash + tool version). |
| NFR-PERF-006 | S | Cancellable long operations (Ctrl-C leaves no corrupt cache). |
| NFR-REL-001 | M | No panics on any input. Fuzzed parser entry points. `unsafe` code forbidden in all crates. |
| NFR-REL-002 | M | Errors carry file offset/line and XML path. Partial results are still produced when possible. |
| NFR-SEC-001 | M | No network access at runtime. No external entity resolution in XML. |
| NFR-SEC-002 | M | Logs never print tag values or descriptions above `debug` level (customer IP). |
| NFR-PORT-001 | M | Linux, Windows and macOS. Windows is the primary user platform. |
| NFR-MAINT-001 | M | MSRV pinned; clippy clean with `-D warnings`; rustfmt enforced; CI on every PR. |
| NFR-MAINT-002 | M | Public API of library crates documented (`#![warn(missing_docs)]`). |
| NFR-COMP-001 | M | Support L5X from Studio 5000 / RSLogix 5000 v20 through current. Record tested versions in `docs/DOMAIN_NOTES.md`. |
