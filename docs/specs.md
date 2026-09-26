# Game Design Spec — Seamless Realistic Universe

Status: v0.4 split-into-topics — realism-first redesign, resources and extraction added. Supersedes v0.3 (single-file spec). Domain content lives in `topics/`; this file is the spine.
Last updated: September 2026

This document is governed by the fixed constraints in README.md: mobile-first, simulation realism over visual realism, real physics with no FTL, lethal physiology, solo-buildable. Where this spec and the README disagree, the README wins and this spec is wrong.

## 1. Vision

A single, unbroken universe at real scale. The player travels from orbit above a star system down to the ground of a planet and back, with no loading screens and no compression of distance, time, pressure or temperature. Real durations are made playable with player-controlled time-warp; real conditions are made legible through instruments.

Survival is the game. The permanent loop is keeping a human body alive on a world that was not made for it, using equipment whose capabilities are engineering values rather than stat multipliers. Realism lives in the numbers the simulation produces; the presentation of those numbers is kept cheap enough to run on a mid-range phone.

## 2. Global Conventions

These apply to every system in every topic below.

**Units.** SI throughout: meters, seconds, kilograms, kelvin, pascals. Derived quantities (velocity, density, dose) use SI derivations. No gameplay unit is ever compressed or rescaled. Time is tracked as seconds elapsed from a fixed in-game epoch.

**Precision.** World positions and velocities are held in double precision. Anything presented to the player is expressed relative to the player's current reference frame (camera-relative / floating origin). This is a requirement of the simulation, not an engine choice.

**Time-warp.** Warp is a player tool with hard rules:

- 1x only while on foot or on EVA.
- Up to 10,000x only while inside a ship that is in orbit or in transit.
- Warp drops to 1x automatically on atmospheric entry, on approach to any body or object, and whenever any physiological alarm is raised.
- Under warp, orbits are propagated analytically (on rails), not integrated step by step. Consumables and physiology are integrated at coarse steps sized to the warp factor.

**Determinism.** Every world derives entirely from a seed. The same seed produces the same world on every device, every time. Nothing in generation depends on frame rate, device, or wall-clock time.

**Performance budget.** Reference target is a mid-range phone. Frame time must never exceed the 30 fps floor during sustained play, including under thermal throttling. Fixed or tracked in [tech.md](tech.md) and [tech/quality.md](tech/quality.md); mobile behavior in [tech/mobile.md](tech/mobile.md).

**Presentation.** Nothing is drawn that the simulation did not produce. Visual fidelity is added only where it helps the player read the simulation.

## Topic index

| Topic file | Area label | Content |
| --- | --- | --- |
| [topics/scale.md](topics/scale.md) | `area:scale` | Scale levels, handoffs, patched conics, gravitational parameters |
| [topics/survival.md](topics/survival.md) | `area:survival` | Physiology, suit, gear, death and persistence, per-person-day inputs |
| [topics/planet-gen.md](topics/planet-gen.md) | `area:planet-gen` | Deterministic generation pipeline, availability-profile emission |
| [topics/machines.md](topics/machines.md) | `area:machines` | Ships, aircraft, ground vehicles, colony machinery |
| [topics/resources.md](topics/resources.md) | `area:resources` | Tracked substances, processes, verbs, archetypes, concessions, propellant |
| [topics/mvp.md](topics/mvp.md) | `area:docs` | MVP abstract descent: purpose, scope, readouts, pass/fail, risk |

## Dependency order

Ordered as in the README roadmap. Each depends on the one before it.

1. **Physiology and minimal replenishment on the MVP planet.** Requires the MVP's continuous environment model as input. Introduces the suit, lethal outcomes, and the stage-2 processes from [topics/resources.md](topics/resources.md): melt ice, electrolysis, scrubber swap, lander solar. Food is carried only; the survival horizon is finite by design. Minimal in-situ replenishment ships with physiology: without it the physiology stage can prove only the dying loop, not the survival loop.
2. **Planet generation pipeline.** Requires physiology to exist so generated worlds can be judged by the survival problems they pose. Must emit the resource availability profile ([topics/resources.md](topics/resources.md)) and prove that archetypes invert each other's bottlenecks.
3. **Ships and transit.** Requires generation so there is more than one body to travel between. Introduces mass, propellant, consumables, and the landing mass budget ([topics/resources.md](topics/resources.md)).
4. **Ground and air vehicles.** Requires a rendered surface and a real atmosphere profile per world.
5. **Subterranean depth.** Requires geology from the pipeline.
6. **Colonies and robots.** Requires power, thermal and material budgets on top of machines. Metals, agriculture and large-scale propellant live here, executed by robots ([topics/resources.md](topics/resources.md)). They are never player hand verbs.
7. **Creatures and riding.** Requires a world whose pipeline produced a biosphere.
8. **Rendering beyond abstract marks.** Introduced incrementally from step 1 onward, always within the mobile budget and never ahead of the simulation it presents.

## Open questions for next pass

Cross-cutting only; domain questions live in their topic file.

- How sleep and fatigue behave under high time-warp: does a long transit under warp count as rest, and how is that made honest?
- Whether physiology can integrate safely at 10,000x or whether physiological state forces a lower warp cap.
- Whether light-time delay at Lv3 matters enough to model for observation and communication.
- How orbital maneuver planning is exposed on a touch screen without hiding the physics.
- Reference device definition and memory ceiling — fixed or tracked in [tech.md](tech.md), [tech/quality.md](tech/quality.md), and [tech/simulation.md](tech/simulation.md).
- Which planet archetypes follow Mars-like as hand-tuned test worlds before generation exists.
- Whether a process-and-wait resource loop ([topics/resources.md](topics/resources.md)) holds attention on its own, or needs an intrinsic skill layer the way orbital mechanics provides one for transit.
- Starting stores: how many person-days of food and LiOH the lander carries, and therefore the length of the stage-2 clock.
- Whether prebreathe ([topics/resources.md](topics/resources.md)) is worth its complexity at stage 2 or belongs with ships at stage 4.
- Water recycling: closed-loop fraction of the habitat, which sets net daily water demand.
