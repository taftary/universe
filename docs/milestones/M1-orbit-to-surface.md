# M1 Orbit to Surface

GitHub milestone 2. Open. Delivered so far: #14, #20, #22, #23. Pending: #26.

## Goal

[../specs.md](../specs.md) section 8: abstract real-scale descent on one hand-tuned planet, continuous instruments, player-controlled descent/ascent with time-warp.

## Exit

Section 8.7 four tests: continuity, repeatability, budget, legibility.

## Delivered

- #14 Debug shell design ([../tech/debug.md](../tech/debug.md), closed)
- #20 Close foundations gap: units, SIM_TICK_S, PRNG, hash (this issue — closed by the PR merge)
- #22 Reference planet and atmosphere (closed)
- #23 Orbits and warp (closed)
- #26 Trajectory, surface, and handoffs (pending, this issue)

## Plan

Six-issue chain, one unproven system at a time:

1. Foundations gap: units, SIM_TICK_S, PRNG, hash
2. Reference planet and atmosphere (parallel with 3)
3. Orbits and warp (parallel with 2)
4. Trajectory, surface, and handoffs (after 2+3)
5. SimSnapshot and golden test (after 4)
6. Shell Phase A (after 5)

## Carry-over (agreed)

Scaffold left SIM_TICK_S and PRNG unpinned although simulation doc says fixed/pinned at scaffold. Agreed values to lock in M1 issue 1: SIM_TICK_S = 0.05 s (D-012), xoshiro256** via rand_xoshiro with SplitMix64 domain split (D-013), xxh3-64 snapshot hash (D-014).

## Definition of Done

- Entry: six issues above assigned to M1.
- Exit: 8.7 four tests pass, CI gates green.
- Release: tag referenced here when cut.

Related: [../../README.md](../../README.md), [../tech.md](../tech.md), [../agents/gh-orchestrator.md](../agents/gh-orchestrator.md).
