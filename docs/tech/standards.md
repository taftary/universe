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

`engine` is the sole exception for `rust.unsafe_code` (see Unsafe below). All other crates keep the workspace value.

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
- Every other crate carries the workspace `forbid(unsafe_code)` value below. Any `unsafe` outside `engine` fails CI.
- `undocumented_unsafe_blocks` is denied everywhere, including `engine`.

## Lints (verbatim)

Copy this block into the workspace `Cargo.toml`. It is the single source of truth; CI enforces it with `-D warnings`.

```toml
[workspace.lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[workspace.lints.clippy]
pedantic = "warn"
perf = "warn"
correctness = "deny"
unwrap_used = "deny"
expect_used = "deny"
undocumented_unsafe_blocks = "deny"
cast_possible_truncation = "warn"
```

Notes: `engine` opts out of `rust.unsafe_code` inheritance and re-declares it as `allow` with the SAFETY rule above. All clippy groups above still apply to `engine`.

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
- Hot loops use struct-of-arrays layout and avoid pointer chasing.
- Zero steady-state allocation: no allocation in the sim tick or frame loop after warmup. Allocation is allowed during load and generation only.
- Every performance-sensitive change states its measured or estimated cost in the issue.

Related: [../tech.md](../tech.md), [architecture.md](architecture.md), [quality.md](quality.md), [mobile.md](mobile.md).
