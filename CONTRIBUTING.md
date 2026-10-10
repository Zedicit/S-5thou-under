# Contributing

## Workflow

1. Open or pick an issue; reference requirement IDs from `docs/REQUIREMENTS.md`.
2. Branch from `main`: `feat/…`, `fix/…`, `docs/…`.
3. Keep PRs small and focused. One milestone feature per PR.
4. CI must be green; at least one review before merge.

## Commits

[Conventional Commits](https://www.conventionalcommits.org/):
`feat(l5x): parse partial routine exports (FR-PAR-007)`.
Scopes: `model`, `l5x`, `logic`, `analysis`, `render`, `cli`, `docs`, `ci`.

## Code rules

- `#![forbid(unsafe_code)]` in every crate.
- Library crates: no `unwrap()`/`expect()` on input-derived data, no printing,
  no `std::process::exit`. Return errors and diagnostics.
- Public items documented. Errors via `thiserror`.
- Names from the PLC are case-insensitive: compare through the model's
  case-folded name type, never with `==` on raw strings.
- Never log tag values or descriptions above `debug` level.
- Add a dependency only with a reason in the PR description; `cargo deny` must pass.

## Local checks

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Decisions

Significant design choices get an ADR in `docs/adr/`.
