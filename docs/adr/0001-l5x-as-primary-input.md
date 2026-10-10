# ADR-0001: L5X as the primary input format

- Status: Accepted
- Date: 2026-10-10

## Context

Studio 5000 Logix Designer exports projects in two text formats:

- **L5K**: proprietary ASCII format with its own grammar
  (`CONTROLLER ... DATATYPE ... END_DATATYPE ... PROGRAM ... ROUTINE ... END_ROUTINE`).
- **L5X**: XML (`RSLogix5000Content` root).

Both can describe a complete project. We need to pick one to build first.

## Decision

Support **L5X** first and only, for v1.0.

## Rationale

| Aspect | L5X | L5K |
|---|---|---|
| Completeness for whole project | Yes | Yes |
| Partial exports (program, routine, AOI, UDT, rungs) | Yes | No (whole project only) |
| FBD/SFC graphics with element IDs and wires | Structured elements | Encoded as text blocks, harder to read |
| Rung comments, multi-language descriptions | Separate elements, `Lang` attributes | Inline, less structured |
| Parsing effort | Off-the-shelf streaming XML + small grammars for rung text and ST | Full custom grammar for the entire file |
| Tooling ecosystem (validators, diff tools, other libraries) | Broad | Little |
| File size | Larger (XML overhead) | Smaller |

Larger files are the only drawback, and a streaming parser handles them.
Ladder rung text inside L5X uses the same neutral text as L5K, so the
ladder grammar we write can be reused if L5K support is added later.

## Consequences

- Users with `.ACD` or `.L5K` files must export to L5X from Studio 5000.
- `s5k-model` stays format-agnostic so an `s5k-l5k` crate can be added later
  without changing analysis or rendering.
