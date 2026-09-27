# M1 Verification

Branch `task/42-m1-verification-release` at `2fb6829` (post-#40). Plan AC1-AC7 from issue #42 Step 2. All commands run locally on Windows x86_64, rustc 1.97.1.

## Per-AC verdicts

- AC1 Continuity descent/ascent: PASS. `cargo test -p universe-engine trajectory` 24 passed, including `boundary_crossing_stays_continuous`, `full_descent_300km_to_touchdown` (300 km circular, 200 m/s retro burn, atmo time 200-600 s, peak g 1.5-6, peak heating 50-150 kW/m2, touchdown below 5 m/s), `full_ascent_surface_to_closed_orbit`. `cargo test --features dev-shell --test bottom_phase_b ac2_full_descent_ascent_zero_flags` passed: descent crosses rails then surface (2 records), ascent crosses surface then rails (2 records), every 7-channel delta inside RAILS [200, 5, 0.01, 1, 1e-6, 1000, 0.1] and SURFACE [100, 20, 10, 1, 2e-4, 50000, 10] bands. Atmosphere anchors (`cargo test -p universe-engine atmosphere`, 13 passed): 610 Pa / 210 K surface, 224.4 Pa +- 11.3 Pa at 10695 m, 1.5-4.0 Pa at 50 km, 0.002-0.01 Pa at 100 km, exactly 0 at 120 km, monotonic pressure and density.
- AC2 Repeatability golden+replay: PASS. Golden hash `17172072447561828286` after 100 ticks, seed `0x243F6A8885A308D3`, 250 km cruise, confirmed in four places: `smoke.rs golden_hash_100_ticks`, `shell_phase_a ac6_golden_hash_still_passes`, `bottom_phase_b ac6_golden_hash_still_passes`, `replay_golden ac7_golden_hash_still_passes`. `ac3_replay_is_deterministic_across_runs` passed (repeat identical, seed^1 differs). Snapshot 304 bytes (`ac6_snapshot_pod_size`, `ac7_snapshot_pod_size`). Perturbed seed and truncated run change the digest.
- AC3 Budget headless+shell: HEADLESS PASS, ON-DEVICE OPEN. `bottom_phase_b ac3_budget_fraction_math` passed (8.0 / 33.33 below 0.5 nominal band). `shell_phase_a ac5_shell_fraction_math` passed (0.4 ms shell draw, fraction 0.012 of 33.33 ms). Headless timing on this box (debug build): full descent (300 km to touchdown, ~28k steps) plus full ascent (surface past 120 km) with per-step snapshot capture ran in 1.76 s wall, i.e. per-tick cost far below `SIM_TICK_AVG_MS = 8.0 ms`. Reference device is still open per Step 2 decision: no on-device 30 fps sustained claim is made here.
- AC4 Legibility ticker-only: DEFERRED to M2. Shell support verified: Ticker-only preset (bits=1, top bar alone) per `docs/tech/debug.md` 6.1 and 9 Phase D; `shell_closed_keeps_run_control_legible` unit test passed; 44 pt targets and `pixels_per_point` scaling covered by `ac2_phone_layout_source_contract`; headless demo prints ticker/preset lines. No flight binary: `universe-debug` is 4-tick smoke (`crates/debug/src/main.rs:88`); scripted drives (`tests/bottom_phase_b.rs:321/388`) are not human evidence. Blind human run (unprompted orbit-to-grid-to-orbit on readouts alone) deferred to M2 follow-up.
- AC5 CI gates order: PASS with two environment-blocked items (see table). `cargo fmt --check` exit 0. `cargo clippy --all-targets --all-features -- -D warnings` exit 0. `cargo build` exit 0. `cargo test` exit 0 (122 tests: 1 + 9 + 9 + 11 + 5 + 75 engine + 12 doc). `cargo test --features dev-shell` exit 0 (141 tests). Headless demo `cargo run -p universe-debug --features dev-shell` exit 0 (`ticks=4 elapsed_s=0.2`, bundle export plus tamper-quarantine lines). `cargo audit` exit 0 (1271 advisories, no vulnerabilities). `cargo deny check` exit 0 (advisories, bans, licenses, sources ok). `cargo hack check` exit 0.
- AC6 Report exists: PASS. This file covers AC1-AC5 with commands, hashes, and numbers.
- AC7 Release v0.2.0: OPEN. Workspace version is still `0.1.0` (Cargo.toml line 12); no tag cut. Pending Steps 6 (dev bump) and 7 (gh-orchestrator tag).

## CI gate table (docs/tech/quality.md order)

| Gate | Result |
| --- | --- |
| cargo fmt --check | PASS, exit 0 |
| cargo clippy --all-targets --all-features -- -D warnings | PASS, exit 0 |
| cargo build | PASS, exit 0 |
| cargo test (includes doc tests) | PASS, exit 0, 122 tests |
| headless sim smoke (tests/ headless descent profile) | PASS, full descent plus ascent inside bands |
| cargo check --target aarch64-linux-android | NOT RUN, exit 101, missing `aarch64-linux-android-clang` on this Windows box, CI must confirm |
| cargo check --target aarch64-apple-ios | NOT RUN, exit 101, missing `clang`/`xcrun` on this Windows box, CI must confirm |
| cargo audit | PASS, exit 0 |
| cargo deny check | PASS, exit 0 |
| cargo hack check | PASS, exit 0 |
| Miri on unsafe crates (engine only) | N/A, no `unsafe` blocks in `crates/engine/src` (only a doc comment in lib.rs) |
| MSRV job (floor 1.95) | PASS locally, rustc 1.97.1 exceeds floor |

## Open items

1. AC4 blind tester run DEFERRED to M2 follow-up: unprompted descent plus ascent on readouts alone, ticker-only, result recorded in M2.
2. AC3 on-device budget: 30 fps sustained floor on a named reference phone at warps 1x-10000x (reference model still open, owner PO/techlead).
3. Cross-target checks need CI confirmation (Android, iOS toolchains absent here).
4. Steps 4-7 pending: techlead review, ppo milestone update, dev version bump 0.1.0 to 0.2.0, tag v0.2.0.

## Verdict

PARTIAL READY for release. AC1, AC2, AC6 pass plus CI; AC4 DEFERRED to M2; AC3 on-device OPEN; AC7 OPEN (no tag, version still 0.1.0).
