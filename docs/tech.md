# Tech Stack — Decision Record

**Status:** Decided for M0. This file is the index of locked decisions. Detail lives in `tech/`.
**Spelling:** US English throughout `docs/tech/` (behavior, meters, kilometers, organize).
**Last updated:** 2026-09-27.

## Fixed constraints (from README)

From [../README.md](../README.md) and [specs.md](specs.md#2-global-conventions) section 2. These are not decisions; they bound all decisions below.

- Mobile-first: reference target is a mid-range phone; desktop is the same build scaled up.
- Performance budget: 30 fps floor, thermal-aware (sustained, not peak). Full budget in [tech/quality.md](tech/quality.md).
- Real SI units everywhere in simulation; floating-origin / camera-relative rendering for seamless scale. See [tech/simulation.md](tech/simulation.md).
- Solo-buildable: one set of physical models reused everywhere; content is inputs, not new mechanics.

## Decisions

Decisions D-001..D-008 were confirmed on 2026-09-26; D-012..D-014 were locked on 2026-09-26 for #20; D-015..D-016 were locked on 2026-09-26 for #22; D-017..D-018 were locked on 2026-09-26 for #23; D-019..D-021 were locked on 2026-09-26 for #26; D-022 was locked on 2026-09-27 for #32; D-009 was locked on 2026-09-27 for #34. Wording below is verbatim. Each links to its tracking issue.

| ID | Decision (verbatim) | Issue | Rationale |
| --- | --- | --- | --- |
| D-001 | Language Rust edition 2024, MSRV pinned in rust-version, Cargo.lock committed. | #1, #2 | Single language for device and tests; reproducible builds; MSRV floor is derived, see [tech/stack.md](tech/stack.md). |
| D-002 | Workspace: flat crates/ (engine, game, debug, tools) + tests/, [workspace.lints] + [workspace.dependencies]. | #1, #2 | Solo-buildable layout with one lint and dependency source; detail in [tech/architecture.md](tech/architecture.md) and [tech/standards.md](tech/standards.md). |
| D-003 | Renderer: custom runtime on wgpu + winit + naga (WGSL); kill-switch: stage-1 abstract-marks build fails 30fps sustained at 0.6x scale on Low-tier device, or a reference device lacks a working wgpu backend -> reopen with ash as fallback. | #1, #2 | Cross-platform safe graphics with GLES fallback; evidence and kill-switch in [tech/stack.md](tech/stack.md). |
| D-004 | ECS: hecs storage + custom fixed-step scheduler, hecs types never in public APIs outside engine::sim (bevy_ecs standalone is named alternative). | #1, #2 | Minimal storage without a framework; boundary keeps sim headless-testable; see [tech/architecture.md](tech/architecture.md). |
| D-005 | UI: egui + egui-wgpu for instruments/readouts/plots/debug shell. | #1, #2 | Immediate-mode instruments fit the MVP readout requirement in [topics/mvp.md](topics/mvp.md); no retained UI framework cost. |
| D-006 | Math/precision: glam (DVec3/DQuat sim, f32 render), unit newtypes over f64, single camera-relative f64->f32 conversion point. | #1, #2 | Double-precision sim with single-precision render per [specs.md](specs.md#2-global-conventions) section 2; detail in [tech/simulation.md](tech/simulation.md). |
| D-007 | Errors: thiserror enums per library crate, anyhow only in binaries, panics only on contract violations. | #1, #2 | Library errors are typed; application errors are ad hoc; full rule in [tech/standards.md](tech/standards.md). |
| D-008 | Logging/profiling: tracing + Tracy (tracing-tracy) in dev, criterion benchmarks. | #1, #2 | Structured spans from day one; continuous profiling on the reference device; gates in [tech/quality.md](tech/quality.md). |
| D-009 | Debug render bridge: egui-wgpu paired with egui 0.36.2, locked at first use in #34 Phase A. | #14, #34 | Immediate-mode shell draws on top of the game view via the D-005 stack; no draw call outside `engine::render` or `debug`. |
| D-012 | Fixed sim step SIM_TICK_S = 0.05 s, never derived from frame time. | #20 | 20 Hz base rate balances orbital coast cost against powered-flight error on the reference phone; detail in [tech/simulation.md](tech/simulation.md). |
| D-013 | Project PRNG xoshiro256** via rand_xoshiro with SplitMix64 domain split (gen_star/gen_body/gen_terrain). | #20 | Seeded reproducible streams per domain keep star, body, and terrain generation independent; detail in [tech/simulation.md](tech/simulation.md). |
| D-014 | Snapshot hash xxh3-64 via xxhash-rust for golden-hash tests. | #20 | Fast non-cryptographic 64-bit hash gives cross-platform state comparison without saving procedural content; policy in [tech/quality.md](tech/quality.md). |
| D-015 | NASA Mars Fact Sheet anchor (p0 610 Pa, T0 210 K, R 3389500 m, g0 3.71 m/s2, M 6.4171e23 kg), analytic hydrostatic profile, MCD validation envelope only. | #22 | Hand-tuned Mars-like reference planet anchors the M1 descent path in [topics/mvp.md](topics/mvp.md); Mars Climate Database is a validation envelope, not an input; detail in [tech/simulation.md](tech/simulation.md). |
| D-016 | Analytic 2-segment T(z) (T0+L1*z to 50 km, 150 K above), altitude-dependent g(z)=g0*(R/(R+z))^2, cutoff 120 km with 100-120 km taper, C0-exact. | #22 | Two-segment temperature with altitude-dependent gravity and a tapered vacuum handoff keeps every readout continuous across the orbit to surface path; detail in [tech/simulation.md](tech/simulation.md). |
| D-017 | Newton-Raphson on E, E0=M for e<0.8 else PI, KEPLER_TOL_RAD 1e-12, KEPLER_MAX_ITER 50, M normalized, e<1e-8 circular guard, MVP 0<=e<1, typed errors, transcendentals via libm crate only. | #23 | Deterministic Kepler coast with named tolerance and cap keeps warp on rails; erratum: the constant is named KEPLER_MAX_ITERATIONS in engine::orbit (see [tech/simulation.md](tech/simulation.md)); detail in [tech/simulation.md](tech/simulation.md). |
| D-018 | Classical Keplerian a/e/i/Omega/omega/M0 + epoch Seconds + mu, mission-elapsed Seconds, Warp enum X1/X10/X100/X1000/X10000 with MAX_WARP_FACTOR 10000.0, SOI owned by orbit.rs, per-tick propagate/advance/request_warp interface. | #23 | Classical elements with unit-typed epoch plus the enum warp ladder encode [specs.md](specs.md#2-global-conventions) section 2 and [topics/mvp.md](topics/mvp.md); detail in [tech/simulation.md](tech/simulation.md). |
| D-019 | Impulsive prograde/retrograde burns along the inertial velocity unit vector with non-negative unit-typed delta-v, unchanged position and epoch, zero-speed rejection, vis-viva post-burn check within 1e-6 relative and elements round-trip within 1e-9 relative. | #26 | Burns shift energy along the velocity direction without touching the coasting model; detail in [tech/simulation.md](tech/simulation.md). |
| D-020 | Point-ship trajectory with analytic rails above 120 km and semi-implicit Euler at SIM_TICK_S 0.05 s below, shared body g(z) and corotating relative wind, drag a = -rho * \|vrel\| * vrel / (2 * B) with B 120 kg/m2, Sutton-Graves q = k * sqrt(rho / r_n) * v^3 with k 1.9027e-4 SI and r_n 1.0 m, g-load over 9.80665 m/s2, exact-time split at the cutoff with C0 aero handoff, Mu from BodyParams, transcendentals via libm crate only. | #26 | One gravity field and one wind feed rails and integration so the descent path stays continuous; detail in [tech/simulation.md](tech/simulation.md). |
| D-021 | Flight regimes Orbit above 120 km / Atmosphere in (0, 120 km] / Surface at or below 0 m with classify/classify_state, adjacent-only transitions, signed distance to Rails/Surface boundaries, warp mapping orbit-cruise / atmosphere-entry auto-drop / surface-grounded, surface contact via altitude plus corotating relative speed with touchdown at 0.5 m and 5 m/s. | #26 | Regime labels gate the integrator choice and warp policy while surface geometry settles touchdown; detail in [tech/simulation.md](tech/simulation.md). |
| D-022 | SimSnapshot 304-byte bytemuck Pod transport behind dev-shell feature, 263-byte little-endian snapshot_hash, capture_snapshot MVP derivation. | #32 | Plain-data snapshot crosses the sim to render boundary as memcopy-safe bytes with dev-only Pod derive; golden digest pins repeatability; detail in [tech/architecture.md](tech/architecture.md), [tech/simulation.md](tech/simulation.md), and [tech/debug.md](tech/debug.md). |

### D-009 activation note (#34 Phase A)

D-009 locks in #34 Phase A as the egui 0.36.2 plus paired egui-wgpu render bridge for the debug shell top bar, input router, inspect view, and DevDark-Pro base theme per [tech/debug.md](tech/debug.md) sections 4.1, 5, 7, 8, and 9 Phase A. Phase A reads the existing SimSnapshot (304-byte Pod, 263-byte hashed prefix from #32) and the existing warp codes (X1 through X10000), drop-reason codes (none, entry, approach, alarm), and `request_warp` / `should_auto_drop` / `apply_auto_drop` policy as-is with no engine sim change. Gaps found during Phase A become follow-up fixes, not scope expansion. `egui_plot`, `postcard`, and `egui_dock` stay deferred under D-010 and D-011. D-005 is unchanged.

### D-003 first use note (#44 Step 6)

Step 6 wires the winit 0.30.13 event loop plus wgpu 30.0.1 surface plus egui-wgpu 0.36.2 renderer in `crates/debug/src/os_window.rs` behind `dev-shell`, driving `DesktopWindow` ticker-only on the main thread with a fixed-step demo orbit. Headless CI never opens: explicit `--run-window` flag plus display gate. The direct `wgpu` edge stays on the locked 30.0 line with no new lock entries; engine sim stays winit-free and wgpu-free.

### D-010/D-011 Phase B note (#36 Step 1)

Phase B draws continuity plots with egui Painter only, with no `egui_plot` lock. The bottom panel stays hand-placed tabs, with no `postcard` lock and no `egui_dock` lock. `Cargo.lock` is unchanged. Revisit D-010 or D-011 only with measured draw-cost evidence.

### D-010/D-011 Phase C note (#38 Step 1)

Phase C renders seed trees, hashes, and recorder rows as text and writes bug bundles as TOML plus CSV text, with no `egui_plot` lock. Bundle text needs no `postcard` lock and panels need no `egui_dock` lock. `Cargo.lock` is unchanged. Revisit D-010 or D-011 only with measured draw-cost evidence.

### Candidate decisions for #14 (design, not locked)

Proposed in [tech/debug.md](tech/debug.md) for issue #14. Each locks at first use per dependency hygiene in [tech/standards.md](tech/standards.md).

| ID | Candidate decision | Issue | Rationale |
| --- | --- | --- | --- |
| D-010 | Debug plots: egui_plot for continuity monitor curves, locked at first use; history uses pre-sized buffers only. | #14 | Readout-over-time curves with handoff markers serve the continuity check in [topics/mvp.md](../topics/mvp.md) pass/fail item 1. |
| D-011 | Shell preset serializer: postcard is the default candidate for shell layout presets; bug-bundle exports use TOML plus CSV per debug.md section 11; egui_dock stays deferred and the section 6.1 4-dock layout is hand-placed panels until a dock crate is justified by measured layout cost. | #14 | Presets stay transient per [tech/persistence.md](tech/persistence.md); default aligns with the save-envelope candidate without locking a second serializer. |

## Open questions

Not blocking M0. Each names its default where one exists.

- Reference device names: Low / Medium / High tiers are defined in [tech/quality.md](tech/quality.md); concrete phone models are still open.
- Audio backend kira vs oddio: deferred. No audio in the stage-1 abstract-marks build.
- Save format crate: postcard is the default candidate for the versioned binary envelope; final lock at scaffold. See [tech/persistence.md](tech/persistence.md).
- Asset pipeline: deferred. Stage 1 uses hand-placed reference data only, per [topics/mvp.md](topics/mvp.md).

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
