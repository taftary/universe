# M1 Orbit to Surface

GitHub milestone 2. Open. Delivered so far: #14, #20, #22, #23, #26, #32, #34, #36, #38, #40 (upon merge).

## Goal

[mvp.md](../topics/mvp.md): abstract real-scale descent on one hand-tuned planet, continuous instruments, player-controlled descent/ascent with time-warp.

## Exit

[mvp.md](../topics/mvp.md) pass/fail four tests: continuity, repeatability, budget, legibility.

## Delivered

- #14 Debug shell design ([../tech/debug.md](../tech/debug.md), closed)
- #20 Close foundations gap: units, SIM_TICK_S, PRNG, hash (closed)
- #22 Reference planet and atmosphere (closed)
- #23 Orbits and warp (closed)
- #26 Trajectory, surface, and handoffs (closed)
- #32 SimSnapshot and golden test (closed)
- #34 Shell Phase A: top bar, input router, inspect view, DevDark-Pro theme; exit per [debug.md](../tech/debug.md) section 9 Phase A (closed)
- #36 Shell Phase B: continuity plots, budget strip, tracing log; exit per [debug.md](../tech/debug.md) section 9 Phase B (closed)
- #38 Shell Phase C: determinism window, replay, inspector, console; exit per [debug.md](../tech/debug.md) section 9 Phase C (closed)
- #40 Shell Phase D: phone polish plus presets; exit per [debug.md](../tech/debug.md) section 9 Phase D (this issue — closed by the PR merge)

## Plan

Nine-issue chain, one unproven system at a time:
1. Foundations gap: units, SIM_TICK_S, PRNG, hash
2. Reference planet and atmosphere (parallel with 3)
3. Orbits and warp (parallel with 2)
4. Trajectory, surface, and handoffs (after 2+3)
5. SimSnapshot and golden test (after 4)
6. #34 Shell Phase A (after 5): top bar, input router, inspect view, DevDark-Pro theme; exit per [debug.md](../tech/debug.md) section 9 Phase A.
7. #36 Shell Phase B (after 6): continuity plots with handoff markers, budget strip with fraction colors, tracing log; exit per [debug.md](../tech/debug.md) section 9 Phase B. D-010/D-011 stay deferred; no new dependencies.
8. #38 Shell Phase C (after 7): seed tree, per-tick hash, input recorder, replay compare, bug-bundle export, tweak registry, console; exit per [debug.md](../tech/debug.md) section 9 Phase C. D-010/D-011 stay deferred; no new dependencies.
9. #40 Shell Phase D (after 8): bottom-sheet tabs, chip row, 44 pt targets, pixels_per_point scaling, four desktop presets, single-plot phone optimization; exit per [debug.md](../tech/debug.md) section 9 Phase D. D-010/D-011 stay deferred; no new dependencies.

Sequencing (moved from specs sections 3 and 8.3; design fact in `topics/scale.md` and `topics/mvp.md`): orbit coasting and warp ship first; burn execution is trajectory scope after coasting and warp.

## Carry-over (agreed)

Scaffold left SIM_TICK_S and PRNG unpinned although simulation doc says fixed/pinned at scaffold. Agreed values to lock in M1 issue 1: SIM_TICK_S = 0.05 s (D-012), xoshiro256** via rand_xoshiro with SplitMix64 domain split (D-013), xxh3-64 snapshot hash (D-014).

## Definition of Done

- Entry: nine issues above assigned to M1.
- Exit: mvp.md pass/fail four tests pass, CI gates green.
- Release: tag referenced here when cut.

## Verification

Verification pass 2026-09-27 on branch `task/42-m1-verification-release`, PR 43 (draft). Report: [M1-verification.md](M1-verification.md). GitHub milestone 2 remains open, 0 of 23 issues closed.

Step 13 review verdict: PARTIAL READY (Steps 11-13 done). M1 closes partial; remainder moves to M2.

- AC1 Continuity: PASS.
- AC2 Repeatability: PASS, single-platform only.
- AC3 Budget: OPEN, no on-device run (headless pass only).
- AC4 Legibility: DEFERRED to M2, blind run per ux protocol appendix.
- AC5 CI gates: conditional PASS, PR 43 CI green on android/gates/ios/msrv (run 36334941078); local cross-targets NOT RUN.
- AC6 Report: PASS.
- AC7 Release: OPEN, no tag cut; no release claimed.

Steps 11-13 done: ux legibility protocol appended, PO ruled mvp.md 8.7 unchanged, verification re-marked PARTIAL READY.

M2 follow-up (draft): blind legibility run per protocol appendix (AC4), on-device budget run (AC3), tag (AC7).

## Next

Partial close: AC1/AC2/AC6 pass. M2 carries AC4 blind run per protocol appendix, AC3 on-device run, and tag (AC7). No release claimed, no tag cut.

#50 debug OS window auto->dx12 quarantine on Intel UHD 620 driver 31.0.101.2130 plus --backend diagnostics flag (Step 5c PASS): auto 2/2 alive, dx12 3/3 alive, zero attributed faults; independent of #48 phone/human/tag tail.

Related: [../../README.md](../../README.md), [../tech.md](../tech.md), [../agents/gh-orchestrator.md](../agents/gh-orchestrator.md).
