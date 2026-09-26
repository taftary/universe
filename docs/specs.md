# Game Design Spec — Seamless Realistic Universe

Status: Draft v0.3 — realism-first redesign, resources and extraction added. Supersedes v0.1 (compressed scale table, four-stat survival, rendered MVP).  
Last updated: September 2026

This document is governed by the fixed constraints in README.md: mobile-first, simulation realism over visual realism, real physics with no FTL, lethal physiology, solo-buildable. Where this spec and the README disagree, the README wins and this spec is wrong.

## 1. Vision

A single, unbroken universe at real scale. The player travels from orbit above a star system down to the ground of a planet and back, with no loading screens and no compression of distance, time, pressure or temperature. Real durations are made playable with player-controlled time-warp; real conditions are made legible through instruments.

Survival is the game. The permanent loop is keeping a human body alive on a world that was not made for it, using equipment whose capabilities are engineering values rather than stat multipliers. Realism lives in the numbers the simulation produces; the presentation of those numbers is kept cheap enough to run on a mid-range phone.

## 2. Global Conventions

These apply to every system in every section below.

**Units.** SI throughout: meters, seconds, kilograms, kelvin, pascals. Derived quantities (velocity, density, dose) use SI derivations. No gameplay unit is ever compressed or rescaled. Time is tracked as seconds elapsed from a fixed in-game epoch.

**Precision.** World positions and velocities are held in double precision. Anything presented to the player is expressed relative to the player's current reference frame (camera-relative / floating origin). This is a requirement of the simulation, not an engine choice.

**Time-warp.** Warp is a player tool with hard rules:

- 1x only while on foot or on EVA.
- Up to 10,000x only while inside a ship that is in orbit or in transit.
- Warp drops to 1x automatically on atmospheric entry, on approach to any body or object, and whenever any physiological alarm is raised.
- Under warp, orbits are propagated analytically (on rails), not integrated step by step. Consumables and physiology are integrated at coarse steps sized to the warp factor.

**Determinism.** Every world derives entirely from a seed. The same seed produces the same world on every device, every time. Nothing in generation depends on frame rate, device, or wall-clock time.

**Performance budget.** Reference target is a mid-range phone. Frame time must never exceed the 30 fps floor during sustained play, including under thermal throttling. Memory ceiling and reference device definition are to be fixed in tech.md.

**Presentation.** Nothing is drawn that the simulation did not produce. Visual fidelity is added only where it helps the player read the simulation.

## 3. Scale Levels

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

## 4. Physiology

Replaces the v0.1 four-stat survival system. The player manages what a real person in a real suit manages. For each system below: what is tracked, what moves it, what the player perceives, and how it ends if ignored.

**Breathing gas.** Tracked: partial pressure of oxygen, partial pressure of carbon dioxide, total suit or cabin pressure, remaining scrubber capacity, remaining oxygen supply. Moved by: metabolic consumption and production, scrubber performance, leaks, external atmosphere on any breach. Perceived: gauges and alarms on the suit; then headache, confusion, tunnel vision. Ends: depressurization kills in seconds; hypoxia or carbon dioxide buildup kills in minutes.

**Heat balance.** Tracked: core temperature, net heat flow between body and environment through the gear worn. Moved by: metabolic heat from activity, insulation of the gear, external temperature, wind, contact with surfaces, sweating and radiative loss. Perceived: suit thermal readout; then shivering, numbness, or dizziness and sweating. Ends: extreme cold or heat kills in minutes to hours depending on how far conditions are from survivable.

**Water and electrolytes.** Tracked: hydration state, electrolyte balance, intake quality. Moved by: drinking, sweating, exertion, contamination of the source (chemical, biological, salinity). Perceived: thirst, fatigue, cramping; contamination is only revealed by testing the source. Ends: dehydration kills over days; toxic water kills over hours to days depending on what is in it.

**Energy.** Tracked: energy reserve, intake, food condition. Moved by: eating, basal metabolism, activity, heat generation in the cold. Food spoils as a function of time and temperature; spoiled food is unsafe. Perceived: hunger, weakness, slowed work. Ends: starvation over weeks. Food has no source on a dead world (Section 7.1); the starvation clock is the long-term survival pressure by design.

**Radiation.** Tracked: cumulative dose. Moved by: the local radiation environment (star, cosmic background, planetary belts) reduced by whatever shielding sits between the player and it: atmosphere, magnetosphere, mass of shelter, depth underground. Perceived: dosimeter only; no early symptoms. Ends: acute exposure kills in hours to days; chronic accumulation raises long-term risk.

**Sleep and fatigue.** Tracked: sleep pressure, work capacity. Moved by: time awake, exertion, rest. Perceived: slower actions, reduced precision, errors. Ends: no direct death, but sustained degradation that makes every other system more dangerous.

**Injury.** Tracked: trauma, bleeding, infection, decompression injury. Moved by: falls, impacts, breaches, untreated wounds. Perceived: pain, mobility loss, visible damage, suit breach alarms. Ends: seconds to days depending on the injury.

**The suit as instrument.** The suit is the player's primary interface with all of the above. It carries tanks and scrubbers with real capacities, sensors for the external environment, and alarms. Alarms are the only softening the game offers: they warn early and honestly. The outcome is never delayed.

**Gear as engineering.** Every piece of equipment is described by physical values: insulation, scrubber capacity per hour, oxygen supply volume, shielding mass, battery energy. Upgrades change those values. There are no abstract multipliers.

**Death and persistence.** Death is permanent for the character. The world is not reset: seed, elapsed time, and every placed or dropped object persist. The body and its gear remain where they fell. A new character begins in the same world and may reach and recover them. There is no reload to undo a death.

## 5. Planet Generation Pipeline

Deferred; not part of the MVP. Documented here because Sections 3, 4, 6 and 7 consume its outputs.

Worlds are produced by a deterministic chain from the seed. Each stage takes the previous stage's output and adds one layer of consequence.

1. **Star.** Mass, age and luminosity. Sets the energy available to everything in the system and the radiation environment.
2. **Disk.** Mass and metallicity of the protoplanetary disk. Sets how much rock, metal and volatile material exists to build planets from.
3. **Planets.** For each body: mass, radius, orbital distance, eccentricity, rotation. Sets surface gravity, day length, and how much energy the body receives.
4. **Volatiles and atmosphere retention.** Whether the body can hold an atmosphere at all, and of what. Depends on gravity, temperature, and whether a magnetosphere protects it.
5. **Greenhouse balance.** Surface temperature and pressure that result from the atmosphere it kept. Feeds heat balance and breathing gas directly.
6. **Water phase.** Whether water is ice, liquid, vapor, or absent given that temperature and pressure. Feeds water and electrolytes.
7. **Geology.** Tectonics, volcanism, erosion. Produces terrain, regolith composition including contaminants (perchlorates, heavy metals, sulphates), mineral concentrations, caves, geothermal gradient. Feeds terrain, subterranean, construction, and the resource profile.
8. **Biology.** Only where every prior stage allows it. Rare by construction. Feeds contamination, creatures, and (under the concession in 7.7) food.

Most worlds the chain produces are dead. That is intended: exploration is the search for, and the reading of, the rare habitable window. Resources are not assigned; they exist where the chain put them. Stages 4, 6 and 7 together emit a resource availability profile per body (Section 7.6): atmospheric species and partial pressures, ice depth and distribution, regolith composition and contaminants.

The pipeline runs once per seed and is cached. It is arithmetic, not rendering, and fits the mobile budget.

## 6. Machines

Deferred beyond the ship's trajectory (see MVP). Every machine is a physical object with a budget.

**Ships.** Dry mass, propellant mass, specific impulse, thrust, heat-shield capacity, life-support consumables per crew-hour. Maneuvers cost propellant according to mass and engine performance. Reentry is a trajectory with heating and g-load; exceeding heat-shield or structural limits destroys the ship. Time-warp is available only inside a ship in orbit or transit (Section 2). The ship carries its own return propellant; landed mass is bounded by the mass needed to leave (Section 7.8).

**Aircraft.** Lift and drag derive from the actual atmospheric density of the current world. Thin atmospheres barely support flight; thick ones fly easily and impose their own pressure hazards.

**Ground vehicles.** Traction, power, thermal management, dust and terrain interaction. Range is an energy budget.

**Colony machinery.** Power sources, radiators, extraction and processing equipment. Only meaningful once power, thermal and material budgets are simulated (Sections 7 and 9).

## 7. Resources and Extraction

Defines how material produced by the pipeline (Section 5) enters the player's hands, and what it costs. Governed by the same rule as everything else: resources are physical quantities moved by physical processes with real energy costs. There is no gather-and-craft loop.

### 7.1 Survival is bounded, not indefinite

On a dead world a human is on a clock. Water and oxygen can be replenished from local ice and atmosphere with power. Food cannot: there is no food source on a dead world, and an independently evolved biosphere is not assumed edible. The realistic long-term food paths are carried stores (roughly 250–300 kg dry mass per person-year), Earth-stock agriculture in a pressurised volume (colony scale, Section 9), or an assayed living world with the concession in 7.7.

The survival horizon is therefore an engineering result: `stores + local replenishment + what you can build`. Progression is pushing that horizon outward. The game never promises that it becomes infinite.

### 7.2 Per person-day consumption (reference values)

| Need | Amount | Notes |
| --- | --- | --- |
| Oxygen | ~0.84 kg | Rises with exertion; hard EVA ~0.10–0.15 kg/h |
| CO₂ produced | ~1.0 kg | Must be scrubbed or vented |
| Water | 2.5–3.5 kg drinking | More for hygiene; recycling reduces net loss |
| Food | ~0.6 kg dry, 2500–4000 kcal | Upper range in cold or heavy EVA |
| Buffer gas (N₂) | Airlock cycles and leaks | Avoidable with a low-pressure pure-O₂ habitat at the cost of fire risk and prebreathe (7.7) |

These values drive Section 4 directly. They are inputs to the physiology model, not tuning parameters.

### 7.3 Tracked substances

Ten quantities, all in kilograms except stored energy in kilowatt-hours. Nothing else is tracked as a resource before Section 9.

```
H2O   CO2   O2   N2   hydrocarbons (CH4 and heavier)   Fe-metal   regolith   organics   propellant   stored energy
```

Each substance carries a contamination state where relevant (perchlorate, heavy metal, salinity, biological) that is unknown until assayed. Consuming an unassayed substance is permitted; it is how the player poisons themselves.

### 7.4 Processes

Every extraction or conversion is a process defined by three numbers: energy per kilogram of output, kilograms of output per hour, and machine mass. There are no other parameters. Approximate reference values:

| Process | Input | Output | Energy | Earliest stage |
| --- | --- | --- | --- | --- |
| Melt / sublimate ice | Ice-bearing regolith | H₂O | ~0.15 kWh/kg H₂O | 2 |
| Electrolysis | H₂O | O₂ + H₂ | ~5–6 kWh/kg H₂O | 2 |
| LiOH absorption | CO₂ | Spent canister | None; ~1.1 kg LiOH per kg CO₂, consumable | 2 |
| Regenerable scrubber | CO₂ | Vented CO₂ | Heat or vacuum swing, tens to hundreds of W | 2 |
| CO₂ splitting (solid-oxide electrolysis) | Atmospheric CO₂ | O₂ + CO | ~10–15 kWh/kg O₂ at scale | 3 |
| Bake hydrated minerals | Regolith (gypsum, clays) | H₂O | ~0.5–1 kWh/kg H₂O at 300–600 °C | 3 |
| Sabatier | H₂ + CO₂ | CH₄ + H₂O | Exothermic; bottleneck is H₂ supply | 4 |
| Regolith reduction / smelting | Oxides | O₂ or Fe | 1000 °C+, megawatt-scale | 9 (robots only) |

Power sources are objects of the same kind, described by watts, mass, and how output is derived from the world: solar from stellar flux at the body's distance, reduced by atmosphere, dust, latitude and night; radioisotope (hundreds of W, decaying); fission (kilowatts, ~1500 kg); battery (kWh, kg). Power is the primary resource: every kilogram of anything else is a kilowatt-hour figure.

### 7.5 Player verbs

The player is a person in a pressure suit doing ~100–200 W of useful work with reduced dexterity. That bounds what hands can do.

- **Prospect.** Instruments locate resources: neutron spectrometer for subsurface hydrogen, spectrometer for composition, ground-penetrating radar for ice depth, drill core for ground truth. Nothing is known until measured.
- **Collect by hand.** Loose ice or snow, meteoritic iron fragments, rock samples, salvage from wrecks and the player's own vehicle. That is the complete list. Ice-cemented regolith at 200 K is as hard as concrete; oxides do not release oxygen without a furnace.
- **Deploy and maintain.** Place a machine, feed it power, clear dust, replace worn parts, swap consumable canisters.
- **Wait.** Processes run in real time. Time-warp (Section 2) makes that playable.

EVA is itself a cost: every hour outside spends oxygen, scrubber capacity, and thermal margin. Prospecting is never free.

### 7.6 Availability by archetype

Section 5 must output, for every body, a resource availability profile from which the following follow as consequences rather than assignments. Reference archetypes:

| Archetype | Plentiful | Scarce or absent | Hazard on the resource |
| --- | --- | --- | --- |
| Airless rocky (Moon-like) | Oxygen bound in oxides (~40 wt%), full solar | Water except polar cold traps; N; C | Long nights on slow rotators; abrasive dust |
| Thin CO₂ (Mars-like) | CO₂ for O₂ and carbon; subsurface ice at mid to high latitude; hydrated minerals; surface Fe-Ni meteorites | N₂ (~3% of a thin atmosphere); free water | Perchlorates in regolith and melt water; dust storms cut solar 30–50% for weeks |
| Icy moon (Europa-like) | Water unlimited; O₂ via electrolysis | Metals (rock under kilometres of ice); C; N; solar (~1–4% of Earth) | Surface radiation may be lethal in hours |
| Thick cold N₂/CH₄ (Titan-like) | N₂; hydrocarbons as fuel; water-ice bedrock | Oxidiser; solar (~1%) | ~94 K: every mechanism is a cryogenic problem |
| Thick hot (Venus-like) | CO₂; N₂; sulphur | Water; any survivable surface | Not landable early. The realistic resource is the decision not to land |
| Asteroid, C-type | ~10% water in hydrated minerals; organics | Everything else | Microgravity: anchoring, drilling, no gravity separation |
| Asteroid, M-type | Fe-Ni-Co; platinum group | Volatiles | Same |
| Living world (rare) | Free O₂; liquid water; biomass | Reliable food (see 7.7) | Biological contamination; toxicity unknown until assayed |

Worlds invert each other's bottlenecks: a Mars-like world has oxidiser and lacks fuel-friendly nitrogen; a Titan-like world has fuel and lacks oxidiser; a Europa-like world has water and lacks metals. This is the intended proof that generation produces different survival problems without new mechanics.

### 7.7 Deliberate realism concessions

Exactly two. Anything else that softens the resource model is a bug.

1. **Some alien biomass is edible after assay and processing.** Strictly, independently evolved biochemistry is unlikely to be nutritious. Without this concession a living world provides air and water and still starves the player, and the rarest content becomes a disappointment. Gated behind testing: eating unassayed biomass remains lethal where the world says so.
2. **Prebreathe is modelled but compressed.** The realistic answer to scarce nitrogen is a low-pressure pure-O₂ habitat, which makes decompression protocol real: transitions between mixed-gas and pure-O₂ environments require a prebreathe of tens of minutes to hours. The protocol and its cost are kept; the player warps through the wait. Skipping it produces decompression injury (Section 4).

### 7.8 Propellant

Ascent from a Mars-mass body requires tonnes of propellant. Local production via electrolysis and Sabatier at ~10 kW yields on the order of 1 kg/h of methane-oxygen, so refuelling for ascent is a months-long plant operation with tonnes of water feedstock. Consequence for Section 6: the ship carries its own return propellant, and the mass landed is bounded by the mass needed to leave. Landing is a commitment. In-situ propellant is a strategic project, never a refuel stop.

### 7.9 Consequences for other sections

- Section 4, Energy: food has no dead-world source. The starvation clock is the long-term survival pressure by design.
- Section 5: stage 7 (geology) outputs regolith composition including contaminants; stage 6 outputs ice depth and distribution; stage 4 outputs atmospheric composition by species. Together they form the availability profile in 7.6.
- Section 9, stage 1: minimal in-situ replenishment (melt ice, electrolysis, scrubber swap, lander solar) ships with physiology. Without it the physiology stage can prove only the dying loop, not the survival loop.
- Section 9, stage 6: metals, agriculture, and large-scale propellant are colony processes executed by robots. They are never player hand verbs.

### 7.10 Reference hour

A typical hour on a Mars-like world at the physiology stage: read dosimeter and ppCO₂; swap a LiOH canister; walk to the neutron spectrometer reading that flagged subsurface hydrogen 40 m away; drill a core; assay it (perchlorate positive, melt water will need filtering); clear dust from the panel array; set the melter running; warp until it finishes or an alarm fires. No mining, no crafting. Every step reads off an instrument. Everything in that paragraph is buildable from 7.3 and 7.4 alone.

## 8. MVP — "Abstract Real-Scale Descent"

### 8.1 Purpose

Prove that a real-scale, physically continuous orbit → surface → orbit path runs on a mid-range phone within budget, before any rendering or generation exists. This isolates the single riskiest technical claim in the project: that the simulation itself is cheap enough and continuous enough to build everything else on.

### 8.2 Representation

Nothing is rendered in the conventional sense. There are no meshes, textures, terrain, sky, or lighting. Every entity is an abstract mark:

- Star: a point.
- Planet: a circle at real radius.
- Atmosphere: concentric circles marking layer boundaries.
- Orbits and trajectories: curves.
- Ship / player: a point.
- Surface: a grid.
- Instruments: numeric readouts and readout-over-time plots.

Everything is hand-placed from published reference data. There is no generation.

### 8.3 Scope

- One star, as a point with real mass.
- One planet, Mars-like: real radius, mass, surface gravity, rotation, and a thin carbon-dioxide atmosphere with a reference pressure and temperature profile by altitude. Chosen because reference data is abundant and unambiguous; swappable for another archetype without changing the MVP's structure.
- No moons, no other bodies.
- Player controls a point-ship: prograde and retrograde burns, time-warp from 1x to 10,000x under the Section 2 rules, atmospheric entry, descent to the surface grid, landing, ascent back to a stable orbit.

### 8.4 Required Readouts

All derived from one continuous model across the three regimes (orbit, atmosphere, surface):

- Altitude above surface
- Velocity (magnitude and direction relative to the surface)
- Orbital elements while in orbit
- Ambient pressure
- Ambient temperature
- Air density
- Heating proxy during entry
- g-load
- Mission elapsed time and current warp factor

### 8.5 Explicitly Cut

Rendering of any kind; generation; physiology and the suit; terrain tiles; weather; moons; additional bodies; ship mass or propellant budget (the point-ship has unlimited delta-v for this test).

### 8.6 Transition Requirements

- No discontinuity in any readout at the orbit ↔ atmosphere ↔ surface handoffs, in either direction.
- The player can reverse direction at any moment without breaking simulation state.
- Warp rules honored: automatic drop to 1x on entry and approach.
- 30 fps floor on the reference phone with the full simulation running at every warp factor.

### 8.7 Pass/Fail

The MVP passes only if all four hold:

1. **Continuity.** Plotted readout curves show no jumps at regime boundaries, and behave the way the underlying physics says they should (for example, pressure and density rise smoothly as altitude falls).
2. **Repeatability.** The same sequence of inputs produces the same mission profile on every run, to within floating-point noise, on every device tested.
3. **Budget.** Frame time never exceeds the 30 fps floor on the reference device during a complete descent and ascent, at any warp factor, under sustained load.
4. **Legibility.** A tester who has not seen the build can descend from orbit to the grid and return to orbit unprompted, using only the readouts.

### 8.8 Primary Risk

That real-scale double-precision simulation with analytic orbit propagation and an atmosphere profile cannot hold the frame budget on the reference phone, or cannot be made continuous across the handoffs. Everything downstream depends on this being false.

## 9. Deferred to Future Iterations

Ordered as in the README roadmap. Each depends on the one before it.

1. **Physiology and minimal replenishment on the MVP planet.** Requires the MVP's continuous environment model as input. Introduces the suit, lethal outcomes, and the stage-2 processes from Section 7.4: melt ice, electrolysis, scrubber swap, lander solar. Food is carried only; the survival horizon is finite by design.
2. **Planet generation pipeline.** Requires physiology to exist so generated worlds can be judged by the survival problems they pose. Must emit the resource availability profile (Section 7.6) and prove that archetypes invert each other's bottlenecks.
3. **Ships and transit.** Requires generation so there is more than one body to travel between. Introduces mass, propellant, consumables, and the landing mass budget (Section 7.8).
4. **Ground and air vehicles.** Requires a rendered surface and a real atmosphere profile per world.
5. **Subterranean depth.** Requires geology from the pipeline.
6. **Colonies and robots.** Requires power, thermal and material budgets on top of machines. Metals, agriculture and large-scale propellant live here, executed by robots (Section 7.9).
7. **Creatures and riding.** Requires a world whose pipeline produced a biosphere.
8. **Rendering beyond abstract marks.** Introduced incrementally from step 1 onward, always within the mobile budget and never ahead of the simulation it presents.

## 10. Open Questions for Next Pass

- How sleep and fatigue behave under high time-warp: does a long transit under warp count as rest, and how is that made honest?
- Whether physiology can integrate safely at 10,000x or whether physiological state forces a lower warp cap.
- Whether light-time delay at Lv3 matters enough to model for observation and communication.
- How orbital maneuver planning is exposed on a touch screen without hiding the physics.
- Reference device definition, memory ceiling, and integration step sizes — to be fixed in tech.md.
- Which planet archetypes follow Mars-like as hand-tuned test worlds before generation exists.
- Whether a process-and-wait resource loop (Section 7.5) holds attention on its own, or needs an intrinsic skill layer the way orbital mechanics provides one for transit.
- Starting stores: how many person-days of food and LiOH the lander carries, and therefore the length of the stage-2 clock.
- Whether prebreathe (Section 7.7) is worth its complexity at stage 2 or belongs with ships at stage 4.
- Water recycling: closed-loop fraction of the habitat, which sets net daily water demand.
