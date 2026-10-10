# Project Charter

## 1. Vision

Give controls engineers, maintenance staff, integrators and reviewers a fast,
offline way to understand what a Rockwell Logix controller program does, by
turning a Studio 5000 `.L5X` export into navigable diagrams and cross-references.

## 2. Problem statement

- Logix projects are large: hundreds of routines, tens of thousands of rungs,
  nested UDTs and AOIs, several languages (Ladder, ST, FBD, SFC) in one project.
- Sequence logic is spread across routines and expressed through conventions
  (step integers, latch/unlatch bits, timers) rather than explicit state machines.
- Studio 5000 is licensed, Windows-only, and shows logic one routine at a time.
  Reviewing a project without it, or comparing two versions, is hard.

## 3. Goals (v1.0)

| # | Goal |
|---|---|
| G1 | Parse a complete L5X project export, and partial exports, into a typed in-memory model. |
| G2 | Resolve every tag reference to its declared type, including UDT members, AOI parameters, arrays, bit addressing and aliases. |
| G3 | Build an execution/call graph: Task → Program → Routine (MainRoutine, JSR, FOR, AOI calls, fault/power-up handlers). |
| G4 | Build a tag cross-reference: every read and write, per rung/line/block, with the instruction that does it. |
| G5 | Generate diagrams: call tree, data-type composition, SFC charts, and sequence/state charts derived from ladder/ST step patterns. |
| G6 | Handle large projects (100+ MB L5X) with bounded memory, progress reporting and a reusable cache. |
| G7 | Produce a static HTML report that links all diagrams and cross-references. |

## 4. Non-goals (v1.0)

- Editing or writing L5X files back (read-only tool).
- Simulating or executing logic; no scan-time or timing analysis.
- Talking to a live controller (no CIP/EtherNet/IP).
- Decrypting source-protected routines or AOIs. They are reported as opaque.
- Reading `.ACD` project files (proprietary binary). Users export to L5X first.
- L5K input (may be added later; see ADR-0001).
- Other vendors (Siemens, Codesys, PLCopen XML). The architecture should not
  prevent this, but it is not planned.

## 5. Users

| Persona | Need |
|---|---|
| Controls engineer inheriting a project | "Where does this machine start, what are its states, what drives this output?" |
| Maintenance / troubleshooting | "Why is this output not on? Which rungs write it?" |
| Integrator / reviewer | Documentation deliverables, design review, change review between versions. |
| Tool integrator | Use the crates as a library, or consume JSON output in other tools. |

## 6. Success criteria

- Parses every L5X in the test corpus (target ≥ 20 real-world-shaped projects,
  firmware v20 to current) with zero panics and documented warnings only.
- A 100 MB L5X parses and indexes in under 60 s on a 4-core laptop, peak memory
  under 2 GB (see NFRs in `REQUIREMENTS.md`).
- An engineer unfamiliar with a sample project can identify its main sequences
  from the generated report alone (validated by user review).

## 7. Constraints and assumptions

- Input files are confidential customer IP. The tool runs fully offline and
  never sends data anywhere. Test fixtures must be synthetic or sanitized.
- Rockwell's L5X schema is documented in the *Logix 5000 Controllers
  Import/Export Reference Manual* (publication 1756-RM084) but not published as
  an XSD. The parser must be tolerant of unknown elements and attributes.
- Studio 5000 versions change the format (for example program parameters arrived
  in v24). The model must record `SoftwareRevision` and the parser must not
  assume the latest version.

## 8. Risks

| Risk | Impact | Mitigation |
|---|---|---|
| No access to large real projects for testing | Parser bugs found late | Synthetic fixture generator; sanitizer tool so users can share safe files; fuzzing |
| Sequence extraction from ladder is heuristic | Wrong or noisy charts | Make patterns configurable, show confidence, always link back to source rungs |
| Huge graphs are unreadable | Diagrams not useful | Scoping/filtering, clustering by program, depth limits, interactive HTML |
| Format variations across versions/firmware | Parse failures | Tolerant parser, warnings not errors, version matrix in the corpus |
| Ladder neutral-text grammar edge cases | Wrong cross-references | Grammar built from the instruction set reference, property tests, round-trip tests |
