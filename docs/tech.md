# Tech Stack — Decision Record

**Status:** Decided for M0. This file is the index of locked decisions. Detail lives in `tech/`.
**Spelling:** US English throughout `docs/tech/` (behavior, meters, kilometers, organize).
**Last updated:** 2026-09-26.

## Fixed constraints (from README)

From [../README.md](../README.md) and [specs.md](specs.md) section 2. These are not decisions; they bound all decisions below.

- Mobile-first: reference target is a mid-range phone; desktop is the same build scaled up.
- Performance budget: 30 fps floor, thermal-aware (sustained, not peak). Full budget in [tech/quality.md](tech/quality.md).
- Real SI units everywhere in simulation; floating-origin / camera-relative rendering for seamless scale. See [tech/simulation.md](tech/simulation.md).
- Solo-buildable: one set of physical models reused everywhere; content is inputs, not new mechanics.

## Decisions

Decisions D-001..D-008 were confirmed on 2026-09-26. Wording below is verbatim. Each links to its tracking issue.

| ID | Decision (verbatim) | Issue | Rationale |
| --- | --- | --- | --- |
| D-001 | Language Rust edition 2024, MSRV pinned in rust-version, Cargo.lock committed. | #1, #2 | Single language for device and tests; reproducible builds; MSRV floor is derived, see [tech/stack.md](tech/stack.md). |
| D-002 | Workspace: flat crates/ (engine, game, debug, tools) + tests/, [workspace.lints] + [workspace.dependencies]. | #1, #2 | Solo-buildable layout with one lint and dependency source; detail in [tech/architecture.md](tech/architecture.md) and [tech/standards.md](tech/standards.md). |
| D-003 | Renderer: custom runtime on wgpu + winit + naga (WGSL); kill-switch: stage-1 abstract-marks build fails 30fps sustained at 0.6x scale on Low-tier device, or a reference device lacks a working wgpu backend -> reopen with ash as fallback. | #1, #2 | Cross-platform safe graphics with GLES fallback; evidence and kill-switch in [tech/stack.md](tech/stack.md). |
| D-004 | ECS: hecs storage + custom fixed-step scheduler, hecs types never in public APIs outside engine::sim (bevy_ecs standalone is named alternative). | #1, #2 | Minimal storage without a framework; boundary keeps sim headless-testable; see [tech/architecture.md](tech/architecture.md). |
| D-005 | UI: egui + egui-wgpu for instruments/readouts/plots/debug shell. | #1, #2 | Immediate-mode instruments fit the MVP readout requirement in [specs.md](specs.md) section 8; no retained UI framework cost. |
| D-006 | Math/precision: glam (DVec3/DQuat sim, f32 render), unit newtypes over f64, single camera-relative f64->f32 conversion point. | #1, #2 | Double-precision sim with single-precision render per [specs.md](specs.md) section 2; detail in [tech/simulation.md](tech/simulation.md). |
| D-007 | Errors: thiserror enums per library crate, anyhow only in binaries, panics only on contract violations. | #1, #2 | Library errors are typed; application errors are ad hoc; full rule in [tech/standards.md](tech/standards.md). |
| D-008 | Logging/profiling: tracing + Tracy (tracing-tracy) in dev, criterion benchmarks. | #1, #2 | Structured spans from day one; continuous profiling on the reference device; gates in [tech/quality.md](tech/quality.md). |

### Candidate decisions for #14 (design, not locked)

Proposed in [tech/debug.md](tech/debug.md) for issue #14. Each locks at first use per dependency hygiene in [tech/standards.md](tech/standards.md).

| ID | Candidate decision | Issue | Rationale |
| --- | --- | --- | --- |
| D-009 | Debug render bridge: egui-wgpu paired with egui 0.36.2, locked at first use. | #14 | Immediate-mode shell draws on top of the game view via the D-005 stack; no draw call outside `engine::render` or `debug`. |
| D-010 | Debug plots: egui_plot for continuity monitor curves, locked at first use; history uses pre-sized buffers only. | #14 | Readout-over-time curves with handoff markers serve the continuity check in [specs.md](specs.md) section 8.7.1. |
| D-011 | Shell preset serializer: postcard is the default candidate for shell layout presets; bug-bundle exports use TOML plus CSV per debug.md section 11; egui_dock stays deferred and the section 6.1 4-dock layout is hand-placed panels until a dock crate is justified by measured layout cost. | #14 | Presets stay transient per [tech/persistence.md](tech/persistence.md); default aligns with the save-envelope candidate without locking a second serializer. |

## Open questions

Not blocking M0. Each names its default where one exists.

- Reference device names: Low / Medium / High tiers are defined in [tech/quality.md](tech/quality.md); concrete phone models are still open.
- Audio backend kira vs oddio: deferred. No audio in the stage-1 abstract-marks build.
- Save format crate: postcard is the default candidate for the versioned binary envelope; final lock at scaffold. See [tech/persistence.md](tech/persistence.md).
- Asset pipeline: deferred. Stage 1 uses hand-placed reference data only, per [specs.md](specs.md) section 8.

## Folder index of docs/tech/

| File | Purpose |
| --- | --- |
| [tech/stack.md](tech/stack.md) | Crate table with proposed locks, why wgpu over vulkano and over Bevy, kill-switch. |
| [tech/architecture.md](tech/architecture.md) | Workspace tree, boundary rules, module map, threading model. |
| [tech/simulation.md](tech/simulation.md) | Frames, precision, units, integrators, time-warp, seeds, determinism. |
| [tech/persistence.md](tech/persistence.md) | Save envelope, atomic writes, quarantine, never-saved list. |
| [tech/standards.md](tech/standards.md) | Layout, naming, errors, unsafe, lints, docs, dependencies, performance. |
| [tech/quality.md](tech/quality.md) | Budgets, tiers, CI gates, test policy. |
| [tech/mobile.md](tech/mobile.md) | OS floors, frame pacer, thermal, textures, GPU set, NDK, threads. |
| [tech/debug.md](tech/debug.md) | Debug shell panels, inspect view, registry, budget strip, phasing. |
| [tech/references.md](tech/references.md) | Accepted and rejected sources with reasons. |

Related: [../README.md](../README.md), [specs.md](specs.md).
