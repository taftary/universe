# Scale — levels, handoffs, orbits

Provenance: `specs.md` §3 (pre-split). Design fact only; M1 sequencing lives in `docs/milestones/M1-orbit-to-surface.md`.

Eight canonical levels in two groups. Observable levels are seen and navigated by; they are never traversed because there is no FTL. Navigable levels run in real units and are where the player's body or ship physically exists.

| Lv | Name | Group | Real extent | What is simulated | What is presented | Player interaction |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | Universe | Observable | Cosmic web | Nothing. Static backdrop | Existing cosmic-web asset (v0.3.2) as sky backdrop | Look only |
| 2 | Galactic | Observable | Galaxy, tens of thousands of light-years | Star catalog positions for the sky and the map | Star field, star map | Look, identify, plan (no travel) |
| 3 | Stellar System | Navigable | Astronomical-unit distances | Keplerian orbits of all bodies around the star; sphere-of-influence boundaries | Orbit map: points and curves | Transfer planning, warp transit |
| 4 | Planetary System | Navigable | Planet radius to sphere of influence | Real masses and radii; moons; handoff between spheres of influence (patched conics) | Body and moons as points/circles; orbits as curves | Approach, capture, orbit insertion |
| 5 | Orbital Expanse | Navigable | Surface to top of atmosphere | Altitude, orbital velocity, entry corridor; heating and g-load as outputs of the trajectory | Trajectory curve, readouts | Burns, deorbit, entry |
| 6 | Atmospheric | Navigable | Top of atmosphere to ground | Composition-driven pressure, temperature and density profile by altitude. Weather deferred | Layer boundaries, readouts. Sky/scattering deferred (rendering) | Descent, flight (post-MVP) |
| 7 | Terrain | Navigable | Ground, human scale | Local gravity, surface pressure and temperature; streamed terrain tiles in meters (post-MVP) | Grid in MVP; tiles later | EVA on foot, landing |
| 8 | Subterranean | Navigable | Below ground | Geothermal gradient, groundwater, caves as shelter. Deferred | Deferred | Deferred |

Boundaries between levels are handoffs in the simulation, not scene changes. Every readout must be continuous across every boundary in both directions.

Orbital mechanics on levels 3-5 are Keplerian two-body arcs with patched-conics handoffs at sphere-of-influence boundaries. Coasting orbits propagate analytically (on rails under warp per [Global Conventions](../specs.md#2-global-conventions)); the dominant center is selected by Newtonian acceleration with Laplace sphere-of-influence radii. Burn execution (prograde and retrograde maneuvers) is trajectory scope, not part of the coasting model.

M1 note: `SimSnapshot` (#32) carries the active frame path (frame level, depth, body ids) as read-only dev-shell transport; it observes Lv3-Lv7 handoffs and changes no level, extent, or handoff rule above.

M1 flight note (#56): the 15-minute flight-log gate steps a 250 km Mars-like cruise profile (Lv5 orbital expanse) for 900 ticks at 1 Hz with no gaps above 5 s; per-tick hashes match across High, Medium, and Low render tiers, so tiers never move sim state across the Lv3-Lv7 handoffs above. Headless 12/12 PASS with `dev-shell` (`tests/flight_15min.rs`, `tests/budget_flight.rs`); live phone run stays OPEN in #48. Budgets cited by name only; gates live in `docs/tech/quality.md`.

Reference gravitational parameters in cubic meters per square second and semi-major axes in meters for hand-placed bodies and tests. This table is the only copy of these values in `docs/topics/`; `mvp.md` points here and does not duplicate them.

| Body | Parameter | Value |
| --- | --- | --- |
| Sun | `MU_SUN` | 1.32712440018e20 m3/s2 |
| Earth | `MU_EARTH` | 3.986004418e14 m3/s2 |
| Mars | `MU_MARS` | 4.282837e13 m3/s2 |
| Earth orbit | semi-major axis `a` | 1.495978707e11 m, e 0.0167 |
| Moon orbit | semi-major axis `a` | 3.84399e8 m |
| Mars orbit | semi-major axis `a` | 2.279392e11 m |

Related: [specs.md](../specs.md#2-global-conventions), [mvp.md](mvp.md), [planet-gen.md](planet-gen.md).
