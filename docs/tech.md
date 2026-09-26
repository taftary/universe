# Tech Stack — Decision Record

**Status:** Undecided. This file records constraints now and decisions as they are taken.

## Fixed constraints (from README)

- Mobile-first: reference target is a mid-range phone; desktop is the same build scaled up.
- Performance budget: 30 fps floor, thermal-aware (sustained, not peak).
- Real SI units everywhere in simulation; floating-origin / camera-relative rendering for seamless scale.
- Solo-buildable: one set of physical models reused everywhere.

## Decisions

| # | Date | Decision | Issue | Rationale |
| --- | --- | --- | --- | --- |
| — | — | none yet | — | — |

## Open questions

- Engine / runtime.
- Language for the physics core (must run identically on device and in tests).
- Test strategy for physical models (reference values, tolerances).
