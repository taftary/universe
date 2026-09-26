# Debug shell — egui instruments for the abstract descent

**Status:** Design for issue #14. Location `docs/tech/debug.md` is locked.
**Spelling:** US English throughout (behavior, meters, kilometers, organize, color).
**Last updated:** 2026-09-26.
**Scope:** Phone plus desktop shell for the stage-1 abstract-marks build in [specs.md](../specs.md) section 8. Same shell on both; desktop is scaled up, never a separate shell.

Gates and budgets live in [quality.md](quality.md). This file names constants only and does not duplicate gate tables.

## 1 Purpose and non-goals

Purpose: give the developer and tester a cheap, legible window into the one continuous model required by [specs.md](../specs.md) section 8, so the section 8.7 pass/fail can be checked directly:

- Continuity (8.7.1): plotted readout curves with no jumps at orbit to atmosphere to surface handoffs, in both directions, behaving as physics says (pressure and density rise smoothly as altitude falls).
- Repeatability (8.7.2): same seed plus same inputs yields the same mission profile within floating-point noise on every device.
- Budget (8.7.3): full descent and ascent at any warp factor holds the 30 fps floor on the reference phone under sustained load.
- Legibility (8.7.4): an unseen tester can descend to the grid and return to orbit using only the readouts.

The shell serves all four. Continuity monitor plots and handoff deltas serve 8.7.1. Seed tree, per-tick hash, input recorder, and replay loader serve 8.7.2. Budget strip, frame badge, and tracing log serve 8.7.3. Top bar run control, left overlays, and readout grouping serve 8.7.4 by keeping the same readouts the player uses visible and continuous.

Non-goals:

- No gameplay UI. Player instruments live in `game` via egui. The debug shell lives in `debug` and never leaks into release; see [architecture.md](architecture.md).
- No rendering debuggers. All 3D-rendering panels are cut to the deferred list in section 10.
- No generation UI beyond seed display and replay. Generation is pure and deterministic; see [simulation.md](simulation.md).
- No persistence of shell state. Shell state is on the never-saved list in [persistence.md](persistence.md).
- No tuning knobs that hide physics. Tweakables that alter sim state taint the run and invalidate repeatability claims.

## 2 Principles

- Budget-relative. Every cost readout is shown as a fraction of its named budget constant (`FRAME_BUDGET_MS = 33.33 ms`, `SIM_TICK_AVG_MS = 8.0 ms`, `SIM_TICK_P99_MS = 16.0 ms`, `SURFACE_HITCH_P95_MS = 100.0 ms`, `MEMORY_CEILING_MB = 1024.0 MB`, `COLD_START_S = 5.0 s`). Absolute numbers alone are rejected. Colors encode fractions, see section 7.
- Determinism visible. Seed, tick count, per-tick hash, and warp factor are always on screen when the shell is open. Any divergence between two runs is shown as first-diverging tick plus hash, never as a vague mismatch.
- Read-only default with tainting. The shell opens read-only. Safe views (overlays, plots, log filters, camera) never touch sim state. Any write through a tweakable or console command marks the run tainted, shows a taint badge in the top bar, and excludes the run from 8.7.2 repeatability claims until replay from a clean seed.
- Shell cost measured and removable. Shell draw and layout cost is measured separately from sim and render cost, shown in the budget strip, and removable by closing the shell or building without the `dev-shell` feature. Shell cost never changes sim behavior. Thermal tier downgrade is render-only; sim behavior is identical across tiers per [quality.md](quality.md) and [mobile.md](mobile.md).
- Same shell on phone and desktop. One panel registry, one theme, one input router. Layout adapts (section 6); content does not fork.
- Units everywhere. Every numeric readout carries its unit in name or type (`altitude_m`, `pressure_pa`, `temperature_k`, `density_kg_m3`, `velocity_m_s`, `elapsed_s`). Raw f64 never crosses a module boundary unwrapped; see [simulation.md](simulation.md).

## 3 egui layer map

Stack is egui 0.36.2 plus egui-wgpu immediate-mode, per D-005 in [tech.md](../tech.md) and [stack.md](stack.md). Tracing plus Tracy (`tracing-tracy`) in dev per D-008; criterion for benchmarks.

Render order per frame:

1. Sim tick first. Fixed-step scheduler with accumulator advances `SIM_TICK_S` steps; render interpolates. Excess time is dropped with a counter, never spiraled; see [mobile.md](mobile.md). Warp 1x to 10000x under player control, with automatic drop to 1x on atmospheric entry, on approach to any body or object, and on any alarm, per [simulation.md](simulation.md).
2. Game abstract-mark pass draws the section 8 marks: star as point, planet as circle at real radius, atmosphere as concentric layer circles, orbits and trajectories as curves, ship and player as point, surface as grid.
3. Debug-draw pass draws egui on top via egui-wgpu. No draw call outside `engine::render` or `debug` is allowed; see [architecture.md](architecture.md).
4. Camera-relative conversion happens once in `engine::render`. Sim holds f64 (`DVec3`, `DQuat`); render uses f32. `SimSnapshot` crossing the boundary is plain data (`bytemuck` `Pod`). No `hecs` types in public APIs outside `engine::sim` per D-004. No f32 flows into sim.

egui layer order inside the debug pass, using `egui::Area::Order`:

- `Background`: full-screen dim only when a modal (bug-bundle export, replay load confirm) is open.
- `TopBottomPanel`: top bar (section 4.1).
- `SidePanel`: left frames plus overlays (section 4.2) and right inspector plus tweakables (section 4.3).
- `CentralPanel`: never owned by the shell. The game view keeps `CentralPanel`; the shell docks around it and never covers it fully on desktop.
- `Window`: determinism and replay panel, console window, modal dialogs. Collapsible and closable.
- `Tooltip`: readout tooltips with unit, source, and budget fraction.
- `Foreground`: taint badge, determinism-divergence flash, thermal-tier notice.

Picking: tap-pick and click-pick run against abstract marks in the game pass, not against egui widgets. The shell reads the pick result from `SimSnapshot` (body id, mark kind, altitude_m, range_m). Points use a screen-space tolerance in points; curves use distance-to-polyline; grid uses cell lookup. Picking never mutates sim state.

## 4 Panels, prioritized

Panels are listed in build and review order. Priority follows 8.7: run control and clocks first, continuity second, determinism third, convenience last.

### 4.1 Top bar (build first)

Single `TopBottomPanel::top` row, always visible when the shell is open:

- Run control: pause, resume, single-step tick, warp selector (1x, 10x, 100x, 1000x, 10000x), warp auto-drop indicator with reason (`entry`, `approach`, `alarm`, `manual`). Warp outside a ship in orbit or transit is rejected with a typed message.
- Sim clock: `tick_count_u64`, `elapsed_s`, `warp_factor`, `SIM_TICK_S` step size. Mission elapsed time and warp factor mirror the section 8 required readouts.
- Frame badge: current fps plus frame time in ms plus fraction of `FRAME_BUDGET_MS`. Color follows section 7 budget-fraction scale.
- Health ticker: one-line status (`nominal`, `entry`, `approach`, `alarm name`, `tainted`, `replaying`). Alarm names come from the sim, never invented by the shell.
- Determinism badge: `seed_u64` short form plus per-tick hash short form plus clean or tainted state. Click opens the determinism panel (section 4.5).
- Shell cost readout: shell draw ms plus fraction of frame budget, with a close-shell button next to it.

### 4.2 Left: frames, overlays, debug camera

`SidePanel::left`, resizable, collapsible:

- Frame tree: active center and frame path from Lv3 to Lv7 (`stellar`, `planetary`, `orbital`, `atmospheric`, `terrain`), with handoff boundaries marked. Lv8 shows as deferred stub. Lv1 to Lv2 show as backdrop asset, not simulation, per [architecture.md](architecture.md).
- Overlays: toggles for each abstract mark class (star point, planet circle, atmosphere layer circles, orbit curves, trajectory curves, ship point, surface grid) plus per-overlay label density. Overlay toggles are safe and never taint.
- Debug camera: follow target (ship, planet center, surface point), zoom to fit (orbit, entry corridor, surface grid), camera-relative origin age and distance to `ORIGIN_REBASE_DISTANCE_M = 5000.0 meters`. Camera moves never touch sim state.
- Regime indicator: current regime (orbit, atmosphere, surface) plus distance to next handoff in meters. Used for the continuity check.

### 4.3 Right: inspector and tweakables

`SidePanel::right`, resizable, collapsible:

- Inspector (safe, read-only): selected entity or pick result with all section 8 required readouts derived from one model: altitude above surface in meters, velocity magnitude and direction relative to the surface, orbital elements while in orbit, ambient pressure in pascals, ambient temperature in kelvin, air density in kilograms per cubic meter, heating proxy during entry, g-load, mission elapsed time, warp factor. Each row shows value, unit, and source module (`orbits`, `trajectory`, `atmo`, `surface`).
- Tweakables (tainting, pause-only): a short allow-list with range, unit, default, and taint flag. Examples: atmosphere density scale for sensitivity checks, heating proxy gain for display only, plot window length. Edits apply only while paused, require explicit apply, and set the taint badge. Anything outside the allow-list is console-only and also taints.
- Safe versus taint split is visual: safe rows use default text; tainting rows carry a marker plus a confirm step. The registry schema in section 11 enforces this.

### 4.4 Bottom: continuity monitor, budget strip, tracing log, console

`TopBottomPanel::bottom`, tabbed, shares one registry (section 11):

- Continuity monitor plots: readout-over-time curves for altitude_m, velocity_m_s, pressure_pa, temperature_k, density_kg_m3, heating proxy, g-load. X axis is `elapsed_s` (and tick count on zoom). Each handoff draws a vertical marker with before, after, and delta values for every readout. Any delta above floating-point noise at a handoff is flagged red and linked to the diverging tick. Plots use `egui_plot` (to be locked at first use; needs a D-row per section 8). Plot history uses pre-sized buffers; see section 8.
- Budget strip: bars for frame ms over `FRAME_BUDGET_MS`, sim tick avg and p99 over `SIM_TICK_AVG_MS` and `SIM_TICK_P99_MS`, surface hitch p95 over `SURFACE_HITCH_P95_MS`, resident MB over `MEMORY_CEILING_MB`, cold start s over `COLD_START_S`. Each bar shows fraction plus color from section 7. Thermal tier (High, Medium, Low) shown as render-only label; sim tick budget line does not move with tier.
- Tracing log: `tracing` events and spans for the frame, filtered by level and module. Tracy connection status shown in dev builds. Log history is bounded and pre-sized; never persisted per [persistence.md](persistence.md).
- Console: command line over the same registry as tweakables. Read commands (get, watch, seed, hash) are safe. Write commands (set, warp, load, replay) taint unless run from a clean replay. Console shares completion and history with the registry.

### 4.5 Determinism and replay panel

`Window`, opens from the determinism badge:

- Seed tree: `master_seed_u64` plus domain-split streams (`gen_star`, `gen_body`, `gen_terrain`) with domain tags, per [simulation.md](simulation.md). Changing one domain never changes another; the panel shows this by rehashing each domain separately.
- Per-tick hash: hash of `SimSnapshot` at each tick, displayed as short hex with full value on tooltip. Hash covers seed, tick count, inputs, and sim state; excludes render caches, profiling spans, and shell state.
- Input recorder: append-only log of (tick_count_u64, input kind, input payload with units). Starts clean on run start; stops on pause or export. Recorder is the source of truth for 8.7.2 replay.
- Replay loader: loads a seed plus input log, steps the fixed-step scheduler headlessly, and compares per-tick hashes. First divergence shows tick, expected hash, actual hash, and input at that tick.
- Bug-bundle export: one action writes the bundle format in section 11 (snapshot, input log, seed tree, hashes, config, log excerpt, system info). Export never includes procedural content or render caches, only seeds per [simulation.md](simulation.md).

## 5 Input routing

Modes: `Passthrough` (game gets all input; shell shows badges only) and `Focused` (shell widgets get input first).

- Desktop: `F3` toggles `Passthrough` to `Focused` and back. `Escape` always returns to `Passthrough` and closes modal first if one is open.
- Phone: long-press on the dev tag (section 6) or three-finger tap toggles to `Focused`. No two-handed gestures required for any shell action once open; all toggles are reachable one-handed.
- Routing rule: when `Focused`, the shell checks `ctx.wants_pointer_input()` and `ctx.wants_keyboard_input()` before the game consumes events. When either is true, the game sees no copy. When `Passthrough`, the shell consumes nothing except its toggle gesture.
- Tap-pick: single tap in `Passthrough` with game handling still picks marks with point tolerance in points (scaled by `pixels_per_point`). In `Focused`, taps on widgets behave as UI; taps on the exposed game view still pick. Pick tolerance and picked mark kind are shown in the inspector so legibility checks can cite them.
- Keyboard: console opens with backquote when `Focused`. All tweakable edits require pause; typing while running edits a draft only.

## 6 Layouts

### 6.1 Desktop (4-dock)

Default desktop layout docks four regions around the game `CentralPanel`: top bar, left panel, right panel, bottom tabs. Four presets switch visibility and size without changing content:

- Descent: top bar plus bottom continuity plots plus left regime indicator. For 8.7.1 handoff watching.
- Determinism: top bar plus determinism window plus bottom tracing log plus input recorder. For 8.7.2 replay.
- Budget: top bar plus bottom budget strip plus shell cost plus thermal tier. For 8.7.3 sustained runs.
- Ticker-only: top bar alone with health ticker and badges. Minimal occlusion for 8.7.4 legibility runs.

Preset choice is shell state only and never persists per [persistence.md](persistence.md).

### 6.2 Phone portrait

Phone uses the same panels with a different arrangement:

- Dev tag: small persistent badge (frame ms fraction plus tick plus warp) anchored top-right. Long-press opens the shell. The tag is the only shell element visible in `Passthrough`.
- Bottom sheet: shell opens as a bottom sheet with tabs (Run, View, Inspect, Plots, Budget, Log, Replay). One tab at a time; swipe or tap switches. Sheet detents are half and full; full never covers the dev tag.
- Chip row: above the sheet, a horizontally scrolling chip row mirrors top-bar run control (pause, step, warp, auto-drop reason) for one-handed use.
- Touch targets: all shell controls are at least 44 pt. Text scales with `ctx.pixels_per_point()`; plots keep a minimum 96 pt height at half detent. No control requires multi-finger input.
- Performance: phone layout renders at most one plot tab at a time. Off-screen plots skip draw but keep recording into pre-sized buffers so continuity data is not lost.

## 7 DevDark-Pro theme

Theme is code as `egui::Visuals` plus `egui::Style`, named `DevDark-Pro`, owned by this file:

- Base: dark visuals with high-contrast text for outdoor legibility checks. Panel strokes are thin; no shadows, no blur, no animation. All styling is immediate-mode compatible and holds the frame budget on the reference phone.
- Budget-fraction colors, applied to frame badge, budget strip, and plot limit lines: below 50 percent of budget uses green, 50 to 80 percent uses amber, above 80 percent uses red. Numeric fraction always accompanies color; color alone never carries meaning.
- Sim-thread accent: sim clock, tick count, and determinism badge share one accent color distinct from render and shell accents, so sim state is recognizable at a glance. Render cost and shell cost use muted tones.
- Fonts: embedded monospace for numbers and hashes, proportional for labels. Fonts ship dev-only behind the `dev-shell` feature and are never in release builds. Tabular numerals for all readouts so values do not jitter.
- Dual metric sets: every readout shows SI primary (meters, seconds, pascals, kelvin) plus raw secondary on tooltip or second line (tick count, frame count, bytes, hash hex). Plot axes label both where space allows (seconds plus ticks).

## 8 Architecture implications

- `engine` gains an `inspect` module behind the `dev-shell` feature that exposes a `SimSnapshot` plain-data view: tick count, elapsed_s, seed, warp factor, active frame path, section 8 readouts with units, pick results, per-tick hash. `SimSnapshot` contains no `hecs` types, no f32 sim inputs, and no IO handles. The single f64 to f32 conversion stays in `engine::render` per [simulation.md](simulation.md).
- `universe-debug` (crate `crates/debug`, binary `universe-debug`) hosts the shell. Current scaffold is a headless tick demo (`SIM_TICK_S = 0.05 s`, four ticks) with `#![forbid(unsafe_code)]`, `mimalloc` global allocator, and `anyhow` at the top level only. Mobile dev builds compile the same binary target for on-device runs; release builds exclude `debug` as a non-default member per [architecture.md](architecture.md). No `winit`, `wgpu`, or `egui` dependency enters `engine::sim`.
- New dependencies need D-rows in [tech.md](../tech.md) before use, per the dependency hygiene rule in [standards.md](standards.md): `egui-wgpu` (to be locked at first use, paired with egui 0.36.2), `egui_plot` (plot curves for section 4.4), config serializer for shell layout presets (default candidate postcard per [persistence.md](persistence.md); if layout uses another serializer, record it). `egui_dock` stays deferred; the 4-dock layout in section 6.1 is hand-placed panels until a dock crate is justified by measured layout cost.
- Persistence clarification: shell presets, plot history, log filters, console history, and recorder drafts are transient and never saved. Bug bundles and replay files are explicit exports, not saves, and follow the atomic-write plus quarantine discipline in [persistence.md](persistence.md) for their own files.
- Standards exemption: the zero steady-state allocation rule in [standards.md](standards.md) exempts the shell only with pre-sized buffers. Plot history, log history, and input recorder pre-allocate at shell open (named capacities with units and source comments) and reuse. Any shell allocation in the frame loop after warmup is a bug unless covered by this exemption and stated with measured cost in the issue.

## 9 Phasing A to D

- Phase A (shell plus run control): top bar, input router with `Passthrough` and `Focused`, dev tag plus `F3` toggle, `SimSnapshot` inspect view, DevDark-Pro base theme. Exit when pause, step, warp, and auto-drop reason display work on desktop and phone with shell cost shown.
- Phase B (continuity plus budget): bottom continuity plots with handoff before, after, and delta markers, budget strip with fraction colors, tracing log tab. Exit when a full descent and ascent shows no unexplained handoff delta and budget fractions stay legible at 30 fps sustained.
- Phase C (determinism plus inspector): seed tree, per-tick hash, input recorder, replay loader, bug-bundle export, right inspector with safe versus taint split and pause-only edits. Exit when the same input log replays to the same hash on x86_64 and AArch64.
- Phase D (phone polish plus presets): bottom-sheet tabs, chip row, 44 pt targets, `pixels_per_point` scaling, four desktop presets, single-plot phone optimization. Exit when the 8.7.4 legibility check can run ticker-only with the shell closed and Ticker-only plus Descent presets verified one-handed on the reference phone.

## 10 Deferred 3D panels with reopen condition

Cut for stage 1. No shell work in this list until the reopen condition holds:

G-buffer and normals views, overdraw heatmap, PBR material inspector, wireframe overlay, material editor, NavMesh view, skeleton view, shadow cascade view, light probe view, draw-call list, VRAM breakdown, flamegraph with flycam WASD controls.

Reopen condition, tied to [specs.md](../specs.md) section 9 step 8 (rendering beyond abstract marks, introduced incrementally from step 1 onward, always within the mobile budget and never ahead of the simulation it presents): a concrete rendering step with a named technique plus a budget note in [quality.md](quality.md) requests a specific panel, and the panel shows only what that step renders. Abstract marks in section 8 never trigger this condition.

## 11 Console and tweakable registry schema plus bug-bundle format

One registry backs right-panel tweakables, bottom console, and preset defaults. Schema per entry:

```text
name: dotted path, e.g. plots.window_s, overlay.atmo_labels, camera.follow
kind: bool | i64 | f64 | string | enum
unit: SI unit or dimensionless, e.g. s, m, Pa, K, dimensionless
range: inclusive min and max with units; enforced before apply
default: value with units plus source comment
effect: safe | taint
edit_rule: live | pause-only
description: one sentence, at most 20 words
```

Rules: `safe` entries never write sim state. `taint` entries set the taint badge, require pause plus explicit apply, and log (tick_count_u64, name, old value, new value) to the input recorder. Unknown names are typed errors, never panics, per [standards.md](standards.md).

Bug-bundle format (directory or archive with this layout, field order fixed):

```text
bundle/
  meta.toml         # bundle_version_u16, created_utc, app_version, platform, tier
  seed_tree.toml    # master_seed_u64, gen_star, gen_body, gen_terrain, domain tags
  inputs.csv        # tick_count_u64, kind, payload with units
  hashes.csv        # tick_count_u64, snapshot_hash_hex
  snapshot.toml     # SimSnapshot at export tick with units on every field
  config.toml       # registry values with units plus preset name
  log_excerpt.txt   # bounded tracing excerpt around first divergence or export tick
  system.txt        # device model, OS floor, backend, thermal tier, frame-time summary
```

Only seeds and placed state persist per [persistence.md](persistence.md); procedural content and render caches are excluded by construction. Corrupt bundles fail checksum before parse and are quarantined, never retried automatically.

## 12 References without duplicating gates

- [specs.md](../specs.md) section 8 (8.1 purpose through 8.8 primary risk) is the requirement source: abstract marks, one Mars-like planet, unlimited delta-v point-ship, required readouts, transition requirements, 8.7 pass and fail. Section 9 step 8 governs the deferred-panel reopen condition.
- [tech.md](../tech.md) D-004 (hecs boundary), D-005 (egui plus egui-wgpu), D-006 (f64 sim with single conversion point), D-008 (tracing plus Tracy in dev).
- [stack.md](stack.md): egui 0.36.2 lock, `egui-wgpu` and `tracing-tracy` to be locked at first use, MSRV floor rule, wgpu kill-switch (out of scope for this shell).
- [architecture.md](architecture.md): `engine::sim` headless boundary, `game` has no GPU code, `debug` non-default member excluded from release, threading with sim thread plus render thread plus workers.
- [simulation.md](simulation.md): frames Lv3 to Lv7, floating origin with `ORIGIN_REBASE_DISTANCE_M`, unit newtypes, fixed-step accumulator, warp state machine with `MAX_WARP_FACTOR = 10000.0 dimensionless`, hierarchical seeds, determinism rules.
- [quality.md](quality.md): sole source for gates and budget table. This file cites `FRAME_BUDGET_MS`, `SIM_TICK_AVG_MS`, `SIM_TICK_P99_MS`, `SURFACE_HITCH_P95_MS`, `MEMORY_CEILING_MB`, `COLD_START_S` by name only.
- [mobile.md](mobile.md): frame pacer plus 30 fps limiter, thermal poll `THERMAL_POLL_S = 2.0 s` with render-only tier downgrade, `cargo-ndk` plus `GameActivity` with `minSdk = 26`, iOS 15 floor, few-core thread sizing.
- [persistence.md](persistence.md): versioned envelope, atomic writes, quarantine, never-saved list (procedural content, render caches, profiling spans, debug shell state).
- [standards.md](standards.md): implementation notes below cite its layout, lint, and performance rules.

## Implementation notes (cite [standards.md](standards.md))

- Layout: flat `crates/debug` shell code inherits `[workspace.dependencies]` and `[lints] workspace = true`. Public APIs take unit newtypes and return `Result` where a range check exists. `hecs` types never appear outside `engine::sim`. Constants carry unit suffixes and source comments; no magic numbers.
- Lints: CI runs `cargo fmt --check` and `cargo clippy --all-targets --all-features -- -D warnings` per [quality.md](quality.md). Keep `pedantic`, `suspicious`, `style`, `complexity`, `perf` at `warn` (blocking under `-D warnings`), `correctness`, `unwrap_used`, `expect_used`, `undocumented_unsafe_blocks` at `deny`. Non-engine crate root keeps `#![forbid(unsafe_code)]`; `engine` per-item `#[expect(unsafe_code, reason = "...")]` with `// SAFETY:` only after techlead review. Override with `#[expect(lint, reason = "...")]`, never `#[allow]`.
- Errors and docs: `thiserror` enums in library crates, `anyhow` only in `universe-debug` top level, panics only on contract violations. Every public item has a doc comment whose first sentence stands alone in at most 20 words, stating units, ranges, and error cases.
- Performance: release profile `lto = "fat"`, `codegen-units = 1`, `overflow-checks = true`. Global allocator `mimalloc` in the binary. Fast hashers (`foldhash`) only for trusted internal keys; external-input keys use `SipHash`. Hot loops use struct-of-arrays and avoid pointer chasing. Shell plot, log, and recorder buffers are pre-sized at open under the section 8 exemption; every performance-sensitive shell change states its measured cost in the issue.

Related: [../README.md](../../README.md), [../specs.md](../specs.md), [../tech.md](../tech.md).
