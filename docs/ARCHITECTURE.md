# Architecture

## 1. Pipeline

```text
 .L5X file
    │  (memory-mapped or buffered read)
    ▼
┌────────────┐  streaming quick-xml events, one top-level element at a time
│  s5k-l5x   │──► raw model: strings for rung text / ST lines, FBD/SFC element lists
└────────────┘
    │  Project (s5k-model)
    ▼
┌────────────┐  per-routine, in parallel (rayon)
│ s5k-logic  │──► ASTs: LadderRung, StStatement, FbdGraph, SfcGraph
└────────────┘
    │
    ▼
┌──────────────┐  passes, each a pure function Model → Facts
│ s5k-analysis │  1 type table   2 symbol tables   3 operand resolution
│              │  4 xref         5 call graph       6 sequence extraction
└──────────────┘
    │  AnalysedProject  ──► optional cache file (.s5kidx)
    ▼
┌────────────┐
│ s5k-render │──► Mermaid / DOT / PlantUML / JSON / HTML
└────────────┘
    ▲
┌────────────┐
│  s5k-cli   │  argument parsing, progress, config, exit codes
└────────────┘
```

## 2. Crate boundaries

| Crate | Depends on | Owns | Must not |
|---|---|---|---|
| `s5k-model` | serde | Domain types, IDs, source locations, diagnostics type | do I/O, parse anything |
| `s5k-l5x` | model, quick-xml | XML → model mapping, version quirks | interpret logic semantics |
| `s5k-logic` | model | Grammars for ladder neutral text and ST; FBD/SFC graph building; instruction catalogue | read files |
| `s5k-analysis` | model, logic, rayon | Resolution, xref, call graph, sequence extraction | know about output formats |
| `s5k-render` | model, analysis | Text generation for each diagram format, HTML report | do analysis |
| `s5k-cli` | all | UX | contain business logic |

Rule: library crates return `Result<T, Error>` plus a `Diagnostics` list. Only
`s5k-cli` prints or exits.

## 3. Core data model (sketch)

All cross-references use typed integer IDs into arenas, not `Rc`/references.
This keeps the model `Send + Sync`, cheap to serialize, and lets passes run in
parallel.

```rust
pub struct Project {
    pub controller: ControllerInfo,          // name, processor, SoftwareRevision
    pub data_types: Arena<DataTypeId, DataType>,
    pub aois:       Arena<AoiId, AoiDefinition>,
    pub tags:       Arena<TagId, Tag>,       // all scopes; Tag.scope says where
    pub programs:   Arena<ProgramId, Program>,
    pub routines:   Arena<RoutineId, Routine>,
    pub tasks:      Arena<TaskId, Task>,
    pub modules:    Arena<ModuleId, Module>,
    pub strings:    Interner,                // tag/type names repeat millions of times
}

pub enum Scope { Controller, Program(ProgramId), Aoi(AoiId) }

pub enum RoutineBody {
    Ladder(Vec<Rung>),        // Rung { number, kind, text, comment }
    StructuredText(Vec<StLine>),
    FunctionBlock(Vec<FbdSheet>),
    Sfc(SfcContent),
    Opaque { reason: OpaqueReason }, // source-protected / encoded
}

pub struct SourceLoc {
    pub routine: RoutineId,
    pub at: LocKind,          // Rung(u32) | Line(u32) | Block{sheet, id} | SfcElement(id)
    pub xml_offset: u64,      // byte offset in the source file, for diagnostics
}
```

Names in Logix are case-insensitive. Every name lookup goes through a
case-folded key; the original spelling is kept for display.

## 4. Performance strategy (large projects)

1. **Streaming read.** `quick-xml` event reader over a buffered file. Each
   top-level collection (`DataTypes`, `AddOnInstructionDefinitions`, `Tags`,
   `Programs`, `Tasks`) is decoded element by element. No DOM.
2. **Skip bulky data early.** Tag `<Data>` blocks (initial values) can be
   large; by default they are skipped or kept as byte ranges and decoded on
   demand (`--with-values`).
3. **String interning.** Tag, type and member names are interned (`lasso` or
   similar). Large projects reuse the same names constantly.
4. **Parallel logic parsing.** Routine bodies are stored as raw text during the
   XML pass, then parsed with `rayon::par_iter` in phase 2.
5. **Phase-ordered analysis.** Types → symbols → resolution → xref/call graph.
   Each phase is parallel internally where data is independent.
6. **Index cache.** `s5k index` serializes `AnalysedProject` (postcard/bincode
   with a version header) keyed by `blake3(file) + tool version`. Other
   commands load the cache when it matches.
7. **Progress.** Parser reports bytes consumed / file size; analysis reports
   routines done / total. The CLI renders this with `indicatif`. Library
   crates take a `&dyn Progress` trait object and never depend on a UI crate.
8. **Benchmarks.** `criterion` benches on synthetic projects of 1, 10 and
   100 MB guard NFR-PERF-001/002 in CI (nightly job, not per-PR).

## 5. Error handling and diagnostics

- `thiserror` error enums in libraries, `anyhow`/`miette` only in the CLI.
- Diagnostics: `{ severity, code, message, location }`. Codes are stable
  (`S5K-L5X-0001` …) and documented, so users can filter or suppress them.
- Philosophy: **degrade, don't die.** An unparsable rung becomes a diagnostic
  plus an `Unparsed` node; the rest of the project still analyses.

## 6. Sequence extraction approach

Sequences are not explicit in ladder/ST, so extraction is pattern-based and
configurable:

| Pattern | Detection | Output |
|---|---|---|
| SFC routine | direct from model | exact state chart |
| Integer step variable | tag compared with `EQU`/`=`/`CASE` constants and written with `MOV`/`:=` constants in guarded logic | states = constants, transitions = guarding conditions |
| Step bits | `OTL Step[n]` + `OTU Step[n-1]` pairs, or one-hot BOOL arrays | states = bits |
| AOI state machines | same patterns inside AOI logic, reported per AOI definition | per-AOI chart |
| PhaseManager | equipment phase state routines (Running, Holding, ...) | ISA-88 state chart |

Users can declare step tags explicitly in `s5k.toml` when heuristics miss or
over-detect. Every extracted state and transition links to its source rung or line.

## 7. Output trace (ADR-0002)

The trace is a query over two whole-project facts built in M4:

- **xref**: for every resolved tag path, its readers and writers with write kind and location;
- **binding edges**: alias → base, AOI argument ↔ parameter (InOut is a reference), JSR
  input/return parameters, program parameter connections, module I/O tag → module point.

`trace(target)` walks writers of `target`; for each writer it collects the tags read
in the same rung/line condition (branch-aware for ladder, the enclosing IF/CASE
conditions for ST), and recurses on those tags, following binding edges in both
directions where they are references. The result is a DAG with cycle markers that
renderers draw grouped by program.

## 8. Extensibility

- New output format: implement `trait Renderer { fn render(&self, g: &Graph, opts) -> String }`.
- New input format (L5K, later): produce the same `s5k-model::Project`; nothing
  downstream changes.
- New analysis: add a pass in `s5k-analysis` that consumes existing facts.
