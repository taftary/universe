# M1 Orbit to Surface

GitHub milestone 2. Open. Delivered so far: #14, #20, #22, #23, #26, #32.

## Goal

[mvp.md](../topics/mvp.md): abstract real-scale descent on one hand-tuned planet, continuous instruments, player-controlled descent/ascent with time-warp.

## Exit

[mvp.md](../topics/mvp.md) pass/fail four tests: continuity, repeatability, budget, legibility.

## Delivered

- #14 Debug shell design ([../tech/debug.md](../tech/debug.md), closed)
- #20 Close foundations gap: units, SIM_TICK_S, PRNG, hash (this issue — closed by the PR merge)
- #22 Reference planet and atmosphere (closed)
- #23 Orbits and warp (closed)
- #26 Trajectory, surface, and handoffs (closed)
- #32 SimSnapshot and golden test (closed)

## Plan

Six-issue chain, one unproven system at a time:

1. Foundations gap: units, SIM_TICK_S, PRNG, hash
2. Reference planet and atmosphere (parallel with 3)
3. Orbits and warp (parallel with 2)
4. Trajectory, surface, and handoffs (after 2+3)
5. SimSnapshot and golden test (after 4)
6. #34 Shell Phase A (after 5): top bar, input router, inspect view, DevDark-Pro theme; exit per [debug.md](../tech/debug.md) section 9 Phase A.

Sequencing (moved from specs sections 3 and 8.3; design fact in `topics/scale.md` and `topics/mvp.md`): orbit coasting and warp ship first; burn execution is trajectory scope after coasting and warp.

## Carry-over (agreed)

Scaffold left SIM_TICK_S and PRNG unpinned although simulation doc says fixed/pinned at scaffold. Agreed values to lock in M1 issue 1: SIM_TICK_S = 0.05 s (D-012), xoshiro256** via rand_xoshiro with SplitMix64 domain split (D-013), xxh3-64 snapshot hash (D-014).

## Definition of Done

- Entry: six issues above assigned to M1.
- Exit: mvp.md pass/fail four tests pass, CI gates green.
- Release: tag referenced here when cut.

Related: [../../README.md](../../README.md), [../tech.md](../tech.md), [../agents/gh-orchestrator.md](../agents/gh-orchestrator.md).
