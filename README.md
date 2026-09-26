# Project README — Seamless Realistic Universe

**Status:** Design phase (pre-production) — realism-first redesign  
**Last updated:** September 2026

**Design constraints (fixed):**

- **Mobile-first.** Reference target is a mid-range phone. Desktop gets the same game, scaled up — never the reverse.
- **Simulation realism over visual realism.** Every number the player sees is physically derived. Visuals are plausible and cheap.
- **Real physics, no FTL.** Real distances, real orbits, real time — made legible with player-controlled time-warp.
- **Lethal physiology.** Where physics kills, the game kills. Warning comes from instruments and symptoms, not from softened rules.
- **Solo-buildable.** One set of physical models reused everywhere; content is inputs, not new mechanics.

## What This Is

A game built around one core fantasy: a single, unbroken universe you can travel through — from orbit above a star system down to the ground beneath your feet — and survive on, for as long as your engineering and your stores allow, once you land.

Two ideas, combined:

- **Real-scale, seamless traversal.** No loading screens between space and surface, no separate "modes." Distances, times, pressures and temperatures are not compressed. A descent from orbit is a real reentry; a trip to the next planet takes real months, lived through time-warp.
- **Bounded personal survival.** The permanent gameplay loop is keeping a human body alive on a world that was not made for it. Not a means to an end — survival is the game. Air and water can be replenished locally with power; food cannot on a dead world, so every landing is on a clock. Progression is engineering that pushes that horizon outward: better insulation, better scrubbers, more shielding, more delta-v, more watts.

Reference points for feel (not structure): **Kerbal Space Program** for orbital mechanics and time-warp, **Outer Wilds** for a small, physically coherent system you learn by observing, **Project Zomboid** for survival that is lethal but always legible.

Robot-operated colonies, base-building, creatures and riding remain part of the long-term vision, but they now follow from the physical systems below rather than being layered on top of them. The project is built in narrow, provable slices.

## The Core Pillars

### 1. Real Scale, Real Physics

The universe is structured as 8 canonical levels, split into two groups:

```
Observable (read-only):   Universe → Galactic
Navigable (real SI units): Stellar System → Planetary System → Orbital Expanse → Atmospheric → Terrain → Subterranean
```

- **Observable levels (Lv1–2)** are the sky and the star map. Without FTL they are never traversed — they are what you see when you look up, and what you navigate *by*. The existing cosmic-web prototype becomes a backdrop asset here.
- **Navigable levels (Lv3–8)** run in real units: meters, seconds, kilograms, kelvin, pascals. Orbits are Keplerian (patched conics first, n-body if ever justified). Moving between bodies costs delta-v and propellant. Time-warp is the player's tool for making real durations playable — 1x on foot, accelerated in orbit and transit.

The hardest, highest-risk part remains the orbit-to-surface transition (Lv5→6→7), but the bar is redefined. It is not enough for it to *look* seamless: altitude, velocity, air pressure, external temperature and reentry heating must all come from one continuous physical model, readable on the suit or ship instruments the entire way down and back up.

Mobile implication: seamlessness is achieved through floating-origin / camera-relative rendering and cheap streamed LOD, not through rendering detail. A phone can integrate an orbit and an atmosphere profile trivially; it cannot render a forest. Spend accordingly.

### 2. Lethal Physiology

The four abstract survival bars are replaced by a body model. The player manages what a real human in a real suit would manage:

| System | What is tracked | Real lethal window (approx.) |
| --- | --- | --- |
| Breathing gas | ppO₂, ppCO₂, total pressure, scrubber capacity | Vacuum / depressurization: seconds. Hypoxia or CO₂ buildup: minutes |
| Heat balance | Metabolic heat vs. conduction, convection, radiation, evaporation; insulation of gear | Extreme cold or heat: minutes to hours |
| Water & electrolytes | Intake, sweat loss, contamination (chemical, biological, salinity) | Dehydration: days. Toxic water: hours to days |
| Energy | kcal in vs. basal metabolism + activity + thermogenesis; food spoilage by temperature and time | Starvation: weeks |
| Radiation | Cumulative dose (mSv); shielding by mass, depth, atmosphere, magnetosphere | Acute: hours to days. Chronic: cumulative |
| Sleep & fatigue | Sleep pressure, work capacity | Degradation over days |
| Injury | Trauma, bleeding, infection, decompression injury | Seconds to days depending on injury |

Design rules:

- **Warnings come from instruments and symptoms** — suit alarms, gauges, blurred vision, shivering — not from a rule that delays the outcome. If the player ignores a depressurization alarm, they die in seconds, as they would.
- **Difficulty comes from the planet, not from tuning.** A thin CO₂ atmosphere at 210 K is hard because of what it is. Nothing is tuned to feel hard.
- **Gear is engineering, not multipliers.** A suit has an insulation value, a scrubber has a capacity in CO₂ per hour, a shelter has a shielding mass. Upgrades change those numbers.
- **Open-ended by construction.** Because conditions come from the world, every new world is a new problem without new mechanics.

### 3. Planets From Physics

Most worlds are dead. That is the realistic premise, and it is what gives exploration its purpose: you are looking for — and learning to read — a habitable window.

Planets are generated by a deterministic pipeline, not by reskinning a resource list:

```
Star (mass, age, luminosity)
→ Protoplanetary disk (mass, metallicity)
→ Planet mass, radius, orbit, eccentricity, rotation
→ Volatile inventory and atmosphere retention (escape velocity, temperature, magnetosphere)
→ Greenhouse balance → surface temperature and pressure
→ Water phase (ice, liquid, vapor, none) → hydrology
→ Geology (tectonics, volcanism, erosion) → terrain, regolith, minerals
→ (Rarely) biology → biosphere
```

Resources emerge from that chain. Water is where the pressure/temperature allow it; oxygen only where something produced it; metals where geology concentrated them. An ice world and a hot desert world differ because their inputs differ, not because they were assigned different flavors.

Getting those resources into the player's hands is engineering, not gathering. Almost nothing on a dead world is usable as picked up: ice must be melted, water split, CO₂ cracked, each at a real kWh-per-kg cost. Power is the primary resource. The player's verbs are *prospect* (instruments), *deploy* and *maintain* (machines), and *wait* (time-warp); hands collect only loose ice, meteorite fragments, samples and salvage. Worlds invert each other's bottlenecks — a Mars-like world has oxidiser but no fuel-friendly nitrogen, a Titan-like world has fuel but no oxidiser, an icy moon has water but no metals — which is what makes generation matter. Food has no dead-world source; it is carried, grown at colony scale, or found on a rare living world. There is no gather-and-craft loop.

Mobile fit: the pipeline runs once per seed and caches. It is arithmetic, not rendering.

### 4. Machines as Engineering

Because there is no FTL, ships are not optional. Every machine is a physical object with a budget:

- **Ships** — mass, propellant, specific impulse, thrust, heat-shield capacity, life-support consumables per crew-hour. Reentry is a trajectory with real heating and g-loads, not a camera path. The ship carries its own return propellant, so what you can land is bounded by what you need to leave; landing is a commitment. Making propellant on the surface is a months-long plant operation, never a refuel stop.
- **Aircraft** — lift and drag from the actual air density of the world you are on. Thin atmospheres barely fly; thick ones fly easily and crush you.
- **Ground vehicles** — traction, power, thermal management, dust.
- **Time-warp** lives here: 1x on foot, accelerated inside a ship in orbit or transit, always under player control.

## Long-Term Vision (Grounded in the Pillars)

These remain part of the intended game but are explicitly deferred. Each depends on a physical system above being real first.

- **Colonies and robots** — a base is a power, thermal, and mass budget: solar or nuclear input, radiator area, ISRU throughput. Metals from ore, agriculture, and ascent-scale propellant live here: they are megawatt or months-long processes executed by robots, never by the player's hands. Worker bots do repetitive extraction and hauling; defender bots handle threats. Neither is meaningful until power/thermal/ISRU are modeled.
- **Creatures and riding** — only exist on worlds whose pipeline actually produced a biosphere, which makes them rare and significant. Behavior and habitat follow from the world's climate and chemistry. Riding is a traversal option on those worlds.
- **Subterranean domain** — real depth: geothermal gradient (~25 K/km on an Earth-like world), groundwater tables, lava tubes and caves as natural radiation shelter. Replaces the earlier "surface markers only" placeholder.

## Design Philosophy

- **Causality over rules.** Prefer one physical model that produces many outcomes over many tuned rules. If a behavior can be derived, do not author it.
- **Lethal where physics is lethal, legible everywhere.** Instruments and symptoms give warning; outcomes are never softened. The player is expected to learn the world, not the rules.
- **Prove the hardest thing first.** Each milestone targets the single riskiest unknown. Right now: a real-scale, physically continuous orbit-to-surface descent running on a phone.
- **One model, every world.** The same equations run on every planet; only inputs change. This is what keeps a universe-scale premise buildable solo.
- **Realism in numbers, plausibility in pixels.** Performance budget: mid-range phone, 30 fps floor, thermal-aware (sustained, not peak). Visual fidelity is spent only where it helps the player read the simulation.

## Current State of the Project

- **Scale system:** Redefined into Observable (Lv1–2) and Navigable (Lv3–8) groups. The stage-0 cosmic web prototype (v0.3.2) is retained as a Lv1–2 backdrop asset, no longer a traversable layer.
- **Survival system:** The v1 four-stat design is superseded by the body model in Pillar 2. Detailed physiology spec not yet written.
- **Planet generation:** Pipeline defined at the outline level (Pillar 3). Not implemented.
- **Resources and extraction:** Defined in specs.md Section 7 — ten tracked substances, eight processes with energy costs, availability by archetype, two deliberate realism concessions. Not implemented.
- **Machines:** Requirements defined (Pillar 4). Not designed in detail.
- **Tech stack:** Decided in M0. See `docs/tech.md` and `docs/tech/` (`stack.md`, `architecture.md`, `simulation.md`, `persistence.md`, `standards.md`, `quality.md`, `mobile.md`, `references.md`). The README commits to a performance budget, not an engine; the budget lives in `docs/tech/quality.md`.
- **Not started:** Everything below the design level.

## Roadmap Shape (High Level, Not Yet Scheduled)

1. **Real-scale orbit → surface on a phone.** One star, one hand-tuned planet with a real atmosphere profile. Player-controlled descent and ascent with time-warp, continuous instrument readouts (altitude, velocity, pressure, temperature, heating) from one model. No loading, no pop-in, 30 fps floor on the reference device.
2. **Body model and minimal replenishment on that planet.** Lethal physiology live, suit as the primary instrument, plus the smallest honest resource loop: melt ice, electrolyse water, swap scrubber canisters, run on lander solar. Food is carried only. Prove survival is legible without being softened, and that the loop is survival rather than just dying slowly.
3. **Planet generation pipeline.** Physics-derived worlds; most dead. Each body emits a resource availability profile. Prove that generated inputs produce meaningfully different survival problems — that archetypes invert each other's bottlenecks.
4. **Ships and transit.** Delta-v, propellant, consumables, time-warp travel between bodies in one system. Landing mass bounded by return propellant.
5. **Ground and air vehicles.** Traversal that respects the local atmosphere and gravity.
6. **Subterranean depth.** Geothermal, groundwater, caves as shelter.
7. **Colonies and robots.** Power/thermal/ISRU budgets, then automation. Metals, agriculture and ascent-scale propellant arrive here, as robot work.
8. **Creatures and riding.** Only on living worlds.

Each stage adds one major unproven system at a time, so failures are easy to isolate.

## Companion Documents

- **[docs/README.md](docs/README.md)** — index of all project documentation.
- **[docs/specs.md](docs/specs.md)** — detailed spec (Draft v0.3). Matches this README; Section 7 holds the resource and extraction model.
- **[docs/tech.md](docs/tech.md)** — locked stack decisions D-001..D-008 with detail in `docs/tech/`.
- **[AGENTS.md](AGENTS.md)** — entry point for AI agents working on this repository.

This README is the standing overview; docs/specs.md is where implementation-level detail lives.
