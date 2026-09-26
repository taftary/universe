# Architecture — workspace and boundaries

**Status:** M0 locked. Implements D-002 and D-004 from [../tech.md](../tech.md).

## Workspace tree

```text
crates/
  engine/   # sim, gen, render abstraction, platform traits
  game/     # gameplay wiring; no GPU code
  debug/    # debug shell and plots; dev-only
  tools/    # offline utilities (seed inspection, golden-hash checks)
tests/      # headless sim smoke + golden-hash tests
```

All crates share `[workspace.dependencies]` and `[workspace.lints]`. Full lint block lives in [standards.md](standards.md). `Cargo.lock` is committed.

## Boundary rules

- `engine::sim` is headless-testable. It depends only on `glam`, unit types, and the project PRNG. No `winit`, no `wgpu`, no `egui`, no file IO.
- `engine::gen` is pure and deterministic. Same seed yields same inputs to sim. No wall clock, no thread-dependent order. Procedural content is never saved; see [simulation.md](simulation.md) and [persistence.md](persistence.md).
- `game` has no GPU code. It wires sim, gen, and instruments. Any draw call outside `engine::render` or `debug` is a bug.
- `debug` never leaks into release. It is gated behind a `debug` feature and excluded from release builds. Instruments shown to players live in `game` via `egui`; the debug shell lives in `debug`.
- Platform code sits behind traits in `engine`. Callers use `PlatformClock`, `PlatformFs`, `PlatformThermal` traits. Concrete implementations are injected; tests inject fakes.
- `tools` may use a `test-internals` style feature to reach `engine` internals. That feature is never enabled in `game` or `debug` release builds.

Violation handling: techlead review blocks merge when a boundary is crossed.

## Module map vs specs.md section 3

Maps [specs.md](../specs.md) section 3 levels 3-8 to owners. Levels 1-2 are a static backdrop asset, not simulation.

| Specs level | Owner | Notes |
| --- | --- | --- |
| Lv3 Stellar System | `engine::sim::orbits` | Patched-conics propagation; analytic under warp. |
| Lv4 Planetary System | `engine::sim::orbits` | Sphere-of-influence handoff; one active center at a time. |
| Lv5 Orbital Expanse | `engine::sim::trajectory` | Entry corridor, heating proxy output, g-load output. |
| Lv6 Atmospheric | `engine::sim::atmo` | Composition-driven pressure, temperature, density profile by altitude. |
| Lv7 Terrain | `engine::sim::surface` + `engine::gen` | Local gravity, surface pressure and temperature; tiles streamed later. |
| Lv8 Subterranean | Deferred | Trait stub only; no behavior. |

Readouts stay continuous across every handoff in both directions, per [specs.md](../specs.md) section 8.

## Threading and frame loop

- Pattern is `std::thread` plus `mpsc` workers. One render thread owns `wgpu`; one sim thread owns the fixed-step scheduler; one worker pool handles generation chunks.
- No async runtime in the frame loop. No `tokio`, no `async-std` on the hot path. Offline `tools` may use blocking IO only.
- Fixed-step sim tick with accumulator; render interpolates. The 30 fps floor and tick budget are defined in [quality.md](quality.md).
- Frame pacer and thermal downgrade live in [mobile.md](mobile.md).

Related: [../tech.md](../tech.md), [stack.md](stack.md), [simulation.md](simulation.md), [standards.md](standards.md).
