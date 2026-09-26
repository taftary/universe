# Resources and extraction

Provenance: `specs.md` §7 (7.1, 7.3-7.8, 7.10). Per-person-day consumption (old 7.2) lives in [survival.md](survival.md). Old 7.9 dissolves: its stages-4/6/7 bullet lives in [planet-gen.md](planet-gen.md); its stage-1 and stage-6 bullets live in [specs.md](specs.md) dependency order.

Defines how material produced by the pipeline ([planet-gen.md](planet-gen.md)) enters the player's hands, and what it costs. Governed by the same rule as everything else: resources are physical quantities moved by physical processes with real energy costs. There is no gather-and-craft loop.

## Survival is bounded, not indefinite

On a dead world a human is on a clock. Water and oxygen can be replenished from local ice and atmosphere with power. Food cannot: there is no food source on a dead world, and an independently evolved biosphere is not assumed edible. The realistic long-term food paths are carried stores (roughly 250–300 kg dry mass per person-year), Earth-stock agriculture in a pressurised volume (colony scale, [specs.md](specs.md)), or an assayed living world with the concession below.

The survival horizon is therefore an engineering result: `stores + local replenishment + what you can build`. Progression is pushing that horizon outward. The game never promises that it becomes infinite.

## Tracked substances

Ten quantities, all in kilograms except stored energy in kilowatt-hours. Nothing else is tracked as a resource before the colony stages in [specs.md](specs.md).

```
H2O   CO2   O2   N2   hydrocarbons (CH4 and heavier)   Fe-metal   regolith   organics   propellant   stored energy
```

Each substance carries a contamination state where relevant (perchlorate, heavy metal, salinity, biological) that is unknown until assayed. Consuming an unassayed substance is permitted; it is how the player poisons themselves.

## Processes

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

Stage numbers refer to the dependency order in [specs.md](specs.md).

Power sources are objects of the same kind, described by watts, mass, and how output is derived from the world: solar from stellar flux at the body's distance, reduced by atmosphere, dust, latitude and night; radioisotope (hundreds of W, decaying); fission (kilowatts, ~1500 kg); battery (kWh, kg). Power is the primary resource: every kilogram of anything else is a kilowatt-hour figure.

## Player verbs

The player is a person in a pressure suit doing ~100–200 W of useful work with reduced dexterity. That bounds what hands can do.

- **Prospect.** Instruments locate resources: neutron spectrometer for subsurface hydrogen, spectrometer for composition, ground-penetrating radar for ice depth, drill core for ground truth. Nothing is known until measured.
- **Collect by hand.** Loose ice or snow, meteoritic iron fragments, rock samples, salvage from wrecks and the player's own vehicle. That is the complete list. Ice-cemented regolith at 200 K is as hard as concrete; oxides do not release oxygen without a furnace.
- **Deploy and maintain.** Place a machine, feed it power, clear dust, replace worn parts, swap consumable canisters.
- **Wait.** Processes run in real time. Time-warp ([specs.md](specs.md#2-global-conventions)) makes that playable.

EVA is itself a cost: every hour outside spends oxygen, scrubber capacity, and thermal margin. Prospecting is never free.

## Availability by archetype

[planet-gen.md](planet-gen.md) must output, for every body, a resource availability profile from which the following follow as consequences rather than assignments. Reference archetypes:

| Archetype | Plentiful | Scarce or absent | Hazard on the resource |
| --- | --- | --- | --- |
| Airless rocky (Moon-like) | Oxygen bound in oxides (~40 wt%), full solar | Water except polar cold traps; N; C | Long nights on slow rotators; abrasive dust |
| Thin CO₂ (Mars-like) | CO₂ for O₂ and carbon; subsurface ice at mid to high latitude; hydrated minerals; surface Fe-Ni meteorites | N₂ (~3% of a thin atmosphere); free water | Perchlorates in regolith and melt water; dust storms cut solar 30–50% for weeks |
| Icy moon (Europa-like) | Water unlimited; O₂ via electrolysis | Metals (rock under kilometres of ice); C; N; solar (~1–4% of Earth) | Surface radiation may be lethal in hours |
| Thick cold N₂/CH₄ (Titan-like) | N₂; hydrocarbons as fuel; water-ice bedrock | Oxidiser; solar (~1%) | ~94 K: every mechanism is a cryogenic problem |
| Thick hot (Venus-like) | CO₂; N₂; sulphur | Water; any survivable surface | Not landable early. The realistic resource is the decision not to land |
| Asteroid, C-type | ~10% water in hydrated minerals; organics | Everything else | Microgravity: anchoring, drilling, no gravity separation |
| Asteroid, M-type | Fe-Ni-Co; platinum group | Volatiles | Same |
| Living world (rare) | Free O₂; liquid water; biomass | Reliable food (see concessions below) | Biological contamination; toxicity unknown until assayed |

Worlds invert each other's bottlenecks: a Mars-like world has oxidiser and lacks fuel-friendly nitrogen; a Titan-like world has fuel and lacks oxidiser; a Europa-like world has water and lacks metals. This is the intended proof that generation produces different survival problems without new mechanics.

## Deliberate realism concessions

Exactly two. Anything else that softens the resource model is a bug.

1. **Some alien biomass is edible after assay and processing.** Strictly, independently evolved biochemistry is unlikely to be nutritious. Without this concession a living world provides air and water and still starves the player, and the rarest content becomes a disappointment. Gated behind testing: eating unassayed biomass remains lethal where the world says so.
2. **Prebreathe is modelled but compressed.** The realistic answer to scarce nitrogen is a low-pressure pure-O₂ habitat, which makes decompression protocol real: transitions between mixed-gas and pure-O₂ environments require a prebreathe of tens of minutes to hours. The protocol and its cost are kept; the player warps through the wait. Skipping it produces decompression injury ([survival.md](survival.md)).

## Propellant

Ascent from a Mars-mass body requires tonnes of propellant. Local production via electrolysis and Sabatier at ~10 kW yields on the order of 1 kg/h of methane-oxygen, so refuelling for ascent is a months-long plant operation with tonnes of water feedstock. Consequence for [machines.md](machines.md): the ship carries its own return propellant, and the mass landed is bounded by the mass needed to leave. Landing is a commitment. In-situ propellant is a strategic project, never a refuel stop.

## Reference hour

A typical hour on a Mars-like world at the physiology stage: read dosimeter and ppCO₂; swap a LiOH canister; walk to the neutron spectrometer reading that flagged subsurface hydrogen 40 m away; drill a core; assay it (perchlorate positive, melt water will need filtering); clear dust from the panel array; set the melter running; warp until it finishes or an alarm fires. No mining, no crafting. Every step reads off an instrument. Everything in that paragraph is buildable from tracked substances and processes alone.

Related: [survival.md](survival.md), [planet-gen.md](planet-gen.md), [machines.md](machines.md), [mvp.md](mvp.md).
