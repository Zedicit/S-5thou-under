# Open Questions and Gaps in the Original Brief

The original brief: *"Open a Studio 5000 .L5K or .L5X file, read all programs,
routines, AOIs and UDTs, understand the base data types and how they interact,
and create flow charts or sequence charts. Files are complex and need time to
process."*

This file lists what the brief does not say, with a proposed default for each.
Resolve an item by moving it into `REQUIREMENTS.md` or an ADR.

## A. Decisions needed from the project owner

| # | Question | Proposed default |
|---|---|---|
| A1 | ~~Which chart types matter most?~~ **Resolved:** proposed order accepted; output trace is the guiding goal (ADR-0002). "Flow chart" and "sequence chart" can mean a routine call tree, a machine state machine, a ladder-rung flowchart, or a UML sequence diagram between programs. | Priority: (1) call tree, (2) SFC/step state charts, (3) UDT composition, (4) inter-program sequence diagram, (5) per-routine logic flowchart. |
| A2 | Who reads the output: engineers in Studio 5000, managers, or auditors? | Engineers. Labels use tag names plus descriptions. |
| A3 | Output medium: text files (Mermaid/DOT), images (SVG/PNG), or an interactive viewer? | Text formats plus a static HTML report with pre-rendered SVG. |
| A4 | Studio 5000 versions to support? | v20 to current. |
| A5 | Are real (sanitized) project files available for testing? | Critical. Without them, synthetic fixtures only and higher risk. |
| A6 | Library, CLI, or GUI? | CLI + library crates first; GUI/web later. |
| A7 | ~~License?~~ **Resolved:** MIT. | |
| A8 | Naming conventions in your projects (step tags, sequence routines)? These drive sequence heuristics. | Configurable through `s5k.toml`. |
| A9 | Safety programs (GuardLogix safety task) in scope? | Parse and show; mark as safety; no special analysis. |

## B. Technical aspects not mentioned in the brief

1. **Tasks and execution order.** Programs do nothing unless a task schedules
   them, and routines run only via MainRoutine/JSR. A correct flow chart must
   start from Tasks (continuous, periodic, event) and their priorities.
2. **Tag scope and resolution.** Controller vs program scope, shadowing,
   aliases, program parameters (`\Program.Param`), and AOI parameter scoping.
3. **Predefined and module-defined types.** TIMER, COUNTER, CONTROL, MESSAGE,
   STRING, PID, motion and I/O module types aren't in the file as UDTs. A built-in
   catalogue is required.
4. **BOOL packing in UDTs.** Hidden `ZZZZZZZZZZ…` host members and `BIT`
   members with `Target`/`BitNumber`.
5. **Four languages, not one.** Ladder (neutral text needs its own grammar),
   Structured Text (a real language grammar), FBD (a wired block graph) and
   SFC (already a state machine).
6. **Instruction semantics.** Cross-references and data flow need to know
   which operands each instruction reads and writes. That means an instruction
   catalogue covering hundreds of instructions.
7. **Indirect addressing.** `Array[Index]` where `Index` is a tag; the
   target can't be resolved statically, so it's recorded as "any element".
8. **Source-protected AOIs/routines.** Logic is encrypted; must degrade
   gracefully.
9. **Partial exports and context.** Users often export one program or AOI.
10. **I/O modules.** Mapping `Local:3:I.Data.5` to a physical card/point makes
    diagrams useful for maintenance.
11. **Inter-program and inter-controller communication.** Produced/consumed
    tags, MSG instructions, program parameters. These are the "sequence
    diagram" edges between programs or controllers.
12. **Phase manager / ISA-88** equipment phases have defined state routines.
13. **Comments and descriptions.** Rung comments and tag descriptions are the
    best source of human meaning for chart labels. Some are multi-language.
14. **Diagram scale.** A 3000-routine call graph is unreadable. Needs filtering,
    clustering by program, depth limits and drill-down.
15. **Confidentiality.** Project files are customer IP: offline only, careful
    logging, sanitizer for bug reports, no customer files in git.
16. **Robustness.** Hand-edited or corrupt files, unknown elements in newer
    versions, online-edit pending rungs.
17. **Caching / incremental processing.** "Needs time" means parse once and
    query many times.
18. **Distribution.** Most users are on Windows without a Rust toolchain:
    signed release binaries.

## C. Features that would make it a usable product

| Feature | Value |
|---|---|
| Tag cross-reference (`who writes Motor1.Run?`) | Most-used troubleshooting question; foundation for everything else |
| "Why is this output off?" backward trace | Walks the conditions that drive a given output |
| Project diff between two L5X versions | Change review, management of change, audits |
| Unused/dead code report | Unused tags, routines never called, AOIs never instantiated |
| Lint rules | Duplicate destructive OTE, writes to constants, unguarded MOV to step |
| HMI/SCADA interface list | Tags with external access, grouped by program |
| I/O list | Module tree with tags and descriptions, exportable to CSV |
| Searchable HTML report | Shareable docs without a Studio 5000 license |
| JSON output | Integrates with other tools, scripts, CI checks on PLC code |
| Watch mode / VS Code extension | Live docs while editing exported code |
| Sanitizer | Lets users share reproducible bug reports safely |
