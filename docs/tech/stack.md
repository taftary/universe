# Stack — locked crates

**Status:** Proposed locks verified 2026-09-26 via crates.io API (`max_stable_version`). Locked for v1 unless a decision row overturns them.
**Lock rule:** Exact version pinned in `Cargo.lock`; `Cargo.toml` uses caret within the locked major line. A bump across majors needs a new D-row in [../tech.md](../tech.md).
**MSRV rule:** Floor equals the maximum MSRV of the locked dependencies, at least 1.87. No fabricated MSRV number is recorded here; the scaffold sets `rust-version` from `cargo tree` output. Consequence: `egui` / `egui-wgpu` 0.36.2 declare `rust-version 1.95` (verified 2026-09-26 via crates.io API), so the max-MSRV rule currently floors the workspace at 1.95, not 1.87.

## Crate table

| Crate | Proposed lock (verified 2026-09-26) | Role | Feature flags |
| --- | --- | --- | --- |
| wgpu | 30.0.1 proposed lock (verified 2026-09-26) | Safe cross-platform GPU abstraction; sole render backend. | `wgsl`, `vulkan`, `metal`, `dx12`, `gles`; no `angle` unless a reference device needs it. |
| winit | 0.30.13 proposed lock (verified 2026-09-26) | Window and event loop; drives the frame pacer in [mobile.md](mobile.md). | Default features; no extra backends. |
| naga | 30.0.1 proposed lock (verified 2026-09-26) | WGSL shader validation via wgpu; pinned to match wgpu line. | Aligned with wgpu; no standalone use in `game`. |
| glam | 0.33.10 proposed lock (verified 2026-09-26) | Math: `DVec3` / `DQuat` in sim (f64), `Vec3` / `Quat` in render (f32). | Default; no `serde` unless persistence needs it. |
| hecs | 0.11.1 proposed lock (verified 2026-09-26) | ECS storage only; scheduler is project-owned. Never in public APIs outside `engine::sim`. | Default; no `serde`, no `row` features. |
| egui | 0.36.2 proposed lock (verified 2026-09-26) | Immediate-mode instruments, readouts, plots, debug shell. | Default; paired with `egui-wgpu` (to be locked at first use). |
| tracing | 0.1.44 proposed lock (verified 2026-09-26) | Structured spans and events; Tracy bridge via `tracing-tracy` in dev. | Default; `attributes` for instrument macros. |

Companions locked at scaffold: `thiserror` 2.0.21, `anyhow` 1.0.104, `tracing` 0.1.44 (`attributes` for instrument macros), `mimalloc` 0.1.52 (global allocator in binaries). Still to be locked at first use (no guess recorded here): `egui-wgpu`, `tracing-tracy`, `criterion`, `postcard` (save default candidate), `bytemuck` (`Pod` on render-side structs; see [simulation.md](simulation.md)).

If a version cannot be verified at scaffold time, write `to be locked at scaffold` in `Cargo.toml` comments instead of guessing.

## Why wgpu over vulkano

- vulkano targets Vulkan only. On Apple platforms it routes through MoltenVK, and it offers no GLES fallback for low-tier Android devices. That breaks the mobile-first constraint in [../tech.md](../tech.md).
- wgpu exposes Vulkan, Metal, DX12, and GLES from one safe API, which keeps one render path for the Low tier defined in [quality.md](quality.md).
- Driver evidence: a vulkano Android segfault report and the Pixel 10 PowerVR caveats in [mobile.md](mobile.md) show Vulkan-only paths carry the highest mobile risk. wgpu runtime probing lets the build fall back instead of crashing.

## Why wgpu over Bevy 0.19

- Mobile is under-staffed upstream; mobile-specific rendering and performance paths lag desktop.
- Bevy `Transform` is f32-only, which conflicts with the f64 sim plus floating-origin requirement in [simulation.md](simulation.md).
- GPU-driven culling work was aborted on Pixel 10 PowerVR-class hardware, which is inside our device envelope.
- Bevy ships breaking releases on a roughly 3-month cycle, which is costly for a solo builder pinning a v1 stack.
- Bevy remains a reference for rapid prototyping, not the runtime. Named alternative for ECS storage is `bevy_ecs` standalone; the locked choice stays hecs plus a custom scheduler per D-004.

## Kill-switch

If the stage-1 abstract-marks build fails 30 fps sustained at 0.6x scale on the Low-tier device, or a reference device lacks a working wgpu backend, reopen D-003 with `ash` as fallback. The abstract-marks criterion is defined in [specs.md](../specs.md) section 8. Record the failing device, backend, and frame-time distribution in the reopening issue.

Related: [../tech.md](../tech.md), [architecture.md](architecture.md), [mobile.md](mobile.md), [references.md](references.md).
