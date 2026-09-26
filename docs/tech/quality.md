# Quality — budgets, gates, test policy

**Status:** M0 locked. This file is the single source of truth for CI gates.

## Budget table

Sustained load on the reference device. Peak values never substitute for sustained values.

| Metric | Floor / ceiling | Notes |
| --- | --- | --- |
| Frame rate | 30 fps sustained floor | Full descent and ascent at any warp factor, per [specs.md](../specs.md) section 8. |
| Hitch | p95 hitch below 100 ms on the surface | Named: `SURFACE_HITCH_P95_MS = 100.0 ms`; generation hitches use worker threads. |
| Sim tick | Average below 8 ms, p99 below 16 ms | Named: `SIM_TICK_AVG_MS = 8.0 ms`, `SIM_TICK_P99_MS = 16.0 ms`; headless-testable. |
| Memory | Below 1 GB resident | Named: `MEMORY_CEILING_MB = 1024.0 MB`; render caches excluded from sim accounting. |
| Cold start | Below 5 s to interactive menu | Named: `COLD_START_S = 5.0 s`; no generation on the start path. |
| Thermal | 15-minute sustained session with no throttle-induced drop below 30 fps | Tier downgrade engages before throttling; see [mobile.md](mobile.md). |

Tolerances are exact bounds, not goals. A miss blocks close.

## Tiers

| Tier | Purpose | Rendering scale |
| --- | --- | --- |
| Low | Budget gate; oldest supported devices | 0.6x scale must hold 30 fps sustained. |
| Medium | Reference phone behavior | 1.0x scale, default texture packs. |
| High | Headroom only | Visual density may rise; sim behavior is identical. |

Sim behavior never varies by tier. Only rendering scale, texture packs, and worker counts vary.

## CI gates (single source of truth)

Run in this order. Any failure blocks merge.

```text
cargo fmt --check
cargo clippy --all-targets --all-features -D warnings
cargo build
cargo test (includes doc tests)
headless sim smoke (tests/ headless descent profile)
cargo check --target aarch64-linux-android
cargo check --target aarch64-apple-ios
cargo audit
cargo deny check
cargo hack check
Miri on unsafe crates (engine only)
MSRV job (rust-version floor; verifies the max-MSRV rule in stack.md)
```

- `fmt`, `clippy`, `build`, and `test` run on every change.
- Android and iOS checks run on every change that touches `engine`, `game`, or platform traits.
- `audit`, `deny`, and `hack` run daily and on any `Cargo.lock` change.
- Miri runs on `engine` when `unsafe` blocks or FFI-adjacent code change.

## Test policy

- Reference values with tolerances. Example: Earth sea-level `SEA_LEVEL_PRESSURE_PA = 101325.0 Pa` within `PRESSURE_TOLERANCE_PA = 1.0 Pa`, `SEA_LEVEL_TEMPERATURE_K = 288.15 K` within `TEMPERATURE_TOLERANCE_K = 0.01 K`. Source: US Standard Atmosphere 1976.
- Invariants: energy and mass conservation within documented drift per integrator; no silent clamping.
- Determinism: same seed and inputs yield the same state hash across runs and across x86_64 and AArch64; see [simulation.md](simulation.md).
- Property tests: round-trip saves, monotonic pressure versus decreasing altitude, warp up and down without state corruption.
- Every physical model has at least one test against an independent reference value. Exact numbers are reported, never visual judgment.

Related: [../tech.md](../tech.md), [standards.md](standards.md), [simulation.md](simulation.md), [mobile.md](mobile.md).
