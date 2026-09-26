# Standards — how code is written

**Status:** M0 locked. Implementation briefs cite this file. See handoff rule in [../agents/process-orchestrator.md](../agents/process-orchestrator.md).
**Spelling:** US English. Units explicit in names or types.

## Workspace and crate layout

Implements D-002 from [../tech.md](../tech.md). Flat `crates/` plus `tests/`:

```text
crates/engine/  # sim, gen, render abstraction, platform traits
crates/game/    # gameplay wiring; no GPU code
crates/debug/   # dev-only shell; feature-gated
crates/tools/   # offline utilities
tests/          # headless sim smoke, golden-hash tests
```

Every crate inherits workspace dependencies and lints:

```toml
[lints]
workspace = true
```

Every non-engine crate (`game`, `debug`, `tools`, tests) adds `#![forbid(unsafe_code)]` at its crate root. `engine` keeps the workspace `deny` value below and justifies each use with a per-item `#[expect(unsafe_code, reason = "...")]` (see Unsafe below).

## Naming and API design

- Units in names or types: `altitude_m`, `pressure_pa`, `temperature_k`. Raw f64 without a unit newtype never crosses a module boundary; see [simulation.md](simulation.md).
- Every physical constant is a named item with a unit suffix and a source comment. Example pattern (names are illustrative):

```rust
/// Sea-level pressure in pascals. Source: US Standard Atmosphere 1976.
pub const SEA_LEVEL_PRESSURE_PA: f64 = 101325.0;
```

- No magic numbers. Literals outside the constant definition and tests are rejected in review.
- Public APIs take unit newtypes and return `Result` where a range check exists. `hecs` types never appear in public APIs outside `engine::sim`.
- Prefer small pure functions over methods that mix IO and math.

## Error handling (D-007)

- Library crates define `thiserror` enums per crate. Binaries use `anyhow` at the top level only.
- Panics only on contract violations (out-of-range internal invariant, broken boundary rule). Player input, file content, and device behavior always produce typed errors.
- Corrupt saves map to the quarantine path in [persistence.md](persistence.md), never to a panic.

## Unsafe discipline

- `unsafe` is allowed only in `engine`, only with a `// SAFETY:` comment stating the invariant, and only after techlead review.
- Every non-engine crate (`game`, `debug`, `tools`, tests) adds `#![forbid(unsafe_code)]` at its crate root. `engine` keeps the workspace `deny` and uses per-item `#[expect(unsafe_code, reason = "...")]` where justified.
- CI enforces the boundary with `undocumented_unsafe_blocks = "deny"` plus a grep gate for `unsafe` outside `crates/engine`. Any `unsafe` outside `engine` fails CI.
- `undocumented_unsafe_blocks` is denied everywhere, including `engine`.

## Lints (verbatim)

Copy this block into the workspace `Cargo.toml`. It is the single source of truth; CI enforces it with `-D warnings`.

```toml
[workspace.lints.rust]
unsafe_code = "deny"
unsafe_op_in_unsafe_fn = "deny"
missing_docs = "warn"
missing_debug_implementations = "warn"

[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }
suspicious = { level = "warn", priority = -1 }
style = { level = "warn", priority = -1 }
complexity = { level = "warn", priority = -1 }
perf = { level = "warn", priority = -1 }
correctness = { level = "deny", priority = -1 }
unwrap_used = "deny"
expect_used = "deny"
undocumented_unsafe_blocks = "deny"
cast_possible_truncation = "warn"
```

Notes: `engine` keeps the workspace `deny` for `rust.unsafe_code`; justified uses carry a per-item `#[expect(unsafe_code, reason = "...")]` with the SAFETY rule above. Non-engine crates additionally declare `#![forbid(unsafe_code)]` at the crate root. `pedantic` stays `warn`: under CI's `-D warnings` that is blocking, which is intentional for a solo project (lint noise is fixed, not tolerated). Group entries use the `{ level, priority }` form so Cargo emits no lint-priority warnings on every build. Override a lint with `#[expect(lint, reason = "...")]`, never `#[allow]`. All clippy groups above still apply to `engine`.

## Doc rules

- Every public item has a doc comment. The first sentence is a single sentence of at most 20 words and stands alone as the summary.
- Docs state units, ranges, and error cases. They link related modules with relative paths.
- Internal invariants that use `panic!` are documented as contract violations.

## Dependency hygiene

- One source: `[workspace.dependencies]`. No per-crate version drift.
- `cargo audit` blocks on known vulnerabilities. `cargo deny` blocks on unapproved licenses and duplicate versions. `cargo hack` checks every feature combination (`--feature-powerset` on workspace members where practical).
- New dependencies need an architect decision record in [../tech.md](../tech.md) before use.

## Performance rules

Release profile (verbatim):

```toml
[profile.release]
lto = "fat"
codegen-units = 1
overflow-checks = true
```

- Global allocator is `mimalloc` in binaries. No per-crate allocator choice.
- Fast hashers only for trusted internal keys: `foldhash` for in-memory maps with non-adversarial keys. Any map keyed by external input (save content, player text, network data) uses the default `SipHash` hasher.
- Hot loops use struct-of-arrays layout and avoid pointer chasing.
- Zero steady-state allocation: no allocation in the sim tick or frame loop after warmup. Allocation is allowed during load and generation only.
- Every performance-sensitive change states its measured or estimated cost in the issue.

Related: [../tech.md](../tech.md), [architecture.md](architecture.md), [quality.md](quality.md), [mobile.md](mobile.md).
