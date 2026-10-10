# ADR-0002: Output-centric, whole-project analysis

- Status: Accepted
- Date: 2026-10-10

## Context

The project owner's main goal is to **understand how outputs turn on**. A
machine process is usually not contained in one program: a permissive may be
computed in a `Safety` program, a sequence step in `Sequencer`, a mode in
`Modes`, and the output finally energised in `Outputs` through an AOI instance
and an alias to a module output tag.

## Decision

1. **The headline feature is a backward trace** from any output (a physical
   module output point, or any tag/member) to the logic that controls it:
   writers → their rung/line conditions → the tags those conditions read →
   their writers, recursively, with a depth limit and cycle detection.
2. **Analysis is always whole-project.** Symbol tables, cross-reference and the
   trace graph span all programs, tasks and AOIs. Per-program views are filters
   applied to the whole-project graph, never separate analyses.
3. **Every path across a boundary is a first-class edge**, so a trace can cross:
   - controller-scope tags shared between programs,
   - program parameters (`\Program.Param`) and their connections,
   - aliases (alias → base tag → module I/O tag),
   - AOI instances: arguments → parameters, including `InOut` references,
   - JSR parameters (input and return),
   - produced/consumed tags (to another controller; recorded as an external edge),
   - I/O modules: module output tag → module → slot/point.
4. **Scheduling context is attached to every writer**: which task and program
   run the routine, and whether the routine is reachable (dead code writing an
   output is reported, not silently shown as active).
5. Partial exports lose cross-program context. The tool warns when a trace
   reaches a tag whose writers may live outside the exported file.

## Consequences

- I/O module parsing (FR-PAR-011) is promoted to **must** in M1, because the
  physical output is a module tag.
- The cross-reference must classify writes precisely: `OTE` (non-retentive,
  last writer in scan wins), `OTL`/`OTU` (retentive set/reset), `MOV`/`:=`
  value writes, timer/counter done bits, AOI output parameters. The trace
  diagram shows the kind of each write.
- Multiple destructive writers to one output (two `OTE`s) are highlighted,
  because scan order then decides the result.
- Chart priority (owner decision): (1) call tree, (2) SFC/step state charts,
  (3) UDT composition, (4) inter-program sequence diagram, (5) per-routine
  logic flowchart. The output trace is delivered alongside (1) in M4, since it
  shares the cross-reference and call-graph machinery.
