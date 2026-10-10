# Diagnostic Codes

Codes are stable: never reuse or renumber one. Messages never contain tag
values or descriptions (NFR-SEC-002); names and byte offsets are fine.

## Reader (`s5k-l5x`)

| Code | Severity | Meaning | What to do |
|---|---|---|---|
| `S5K-L5X-0000` | Error | XML became malformed or ended early. Everything completed before that point is kept; `truncated` is set. | Re-export from Studio 5000; check the file wasn't cut off when copied. |
| `S5K-L5X-0001` | Warning | Unknown element skipped (newer Studio 5000 version, or an element not modelled yet). | Report it with the element name and SoftwareRevision so support can be added. |
| `S5K-L5X-0002` | Warning | Attribute value not understood (bad number or dimension list). The attribute is treated as absent. | Usually a hand-edited file. |
| `S5K-L5X-0003` | Info | Rung has a pending online-edit type (`Type` other than `N`). Its text is kept as is. | Accept or cancel pending edits before exporting for an exact picture. |
| `S5K-L5X-0004` | Info | Source-protected AOI or routine; logic is opaque. AOI parameters remain visible. | Nothing; traces show the block as a black box. |
| `S5K-L5X-0005` | Warning | Element without a required `Name` attribute skipped. | Hand-edited or corrupt file. |

## CLI exit codes (FR-CLI-003)

| Code | Meaning |
|---|---|
| 0 | Success (warnings/info allowed) |
| 1 | Finished, but at least one `Error` diagnostic (e.g. partial read) |
| 2 | Usage error |
| 3 | I/O failure, or the file is not L5X |
