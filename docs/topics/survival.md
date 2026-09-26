# Survival — physiology and the suit

Provenance: `specs.md` §4 plus the per-person-day table from §7.2 as Reference inputs.

Replaces the v0.1 four-stat survival system. The player manages what a real person in a real suit manages. For each system below: what is tracked, what moves it, what the player perceives, and how it ends if ignored.

**Breathing gas.** Tracked: partial pressure of oxygen, partial pressure of carbon dioxide, total suit or cabin pressure, remaining scrubber capacity, remaining oxygen supply. Moved by: metabolic consumption and production, scrubber performance, leaks, external atmosphere on any breach. Perceived: gauges and alarms on the suit; then headache, confusion, tunnel vision. Ends: depressurization kills in seconds; hypoxia or carbon dioxide buildup kills in minutes.

**Heat balance.** Tracked: core temperature, net heat flow between body and environment through the gear worn. Moved by: metabolic heat from activity, insulation of the gear, external temperature, wind, contact with surfaces, sweating and radiative loss. Perceived: suit thermal readout; then shivering, numbness, or dizziness and sweating. Ends: extreme cold or heat kills in minutes to hours depending on how far conditions are from survivable.

**Water and electrolytes.** Tracked: hydration state, electrolyte balance, intake quality. Moved by: drinking, sweating, exertion, contamination of the source (chemical, biological, salinity). Perceived: thirst, fatigue, cramping; contamination is only revealed by testing the source. Ends: dehydration kills over days; toxic water kills over hours to days depending on what is in it.

**Energy.** Tracked: energy reserve, intake, food condition. Moved by: eating, basal metabolism, activity, heat generation in the cold. Food spoils as a function of time and temperature; spoiled food is unsafe. Perceived: hunger, weakness, slowed work. Ends: starvation over weeks. Food has no source on a dead world ([resources.md](resources.md)); the starvation clock is the long-term survival pressure by design.

**Radiation.** Tracked: cumulative dose. Moved by: the local radiation environment (star, cosmic background, planetary belts) reduced by whatever shielding sits between the player and it: atmosphere, magnetosphere, mass of shelter, depth underground. Perceived: dosimeter only; no early symptoms. Ends: acute exposure kills in hours to days; chronic accumulation raises long-term risk.

**Sleep and fatigue.** Tracked: sleep pressure, work capacity. Moved by: time awake, exertion, rest. Perceived: slower actions, reduced precision, errors. Ends: no direct death, but sustained degradation that makes every other system more dangerous.

**Injury.** Tracked: trauma, bleeding, infection, decompression injury. Moved by: falls, impacts, breaches, untreated wounds. Perceived: pain, mobility loss, visible damage, suit breach alarms. Ends: seconds to days depending on the injury.

**The suit as instrument.** The suit is the player's primary interface with all of the above. It carries tanks and scrubbers with real capacities, sensors for the external environment, and alarms. Alarms are the only softening the game offers: they warn early and honestly. The outcome is never delayed.

**Gear as engineering.** Every piece of equipment is described by physical values: insulation, scrubber capacity per hour, oxygen supply volume, shielding mass, battery energy. Upgrades change those values. There are no abstract multipliers.

**Death and persistence.** Death is permanent for the character. The world is not reset: seed, elapsed time, and every placed or dropped object persist. The body and its gear remain where they fell. A new character begins in the same world and may reach and recover them. There is no reload to undo a death.

## Reference inputs — per person-day consumption

Provenance: `specs.md` §7.2. These values drive the physiology model above directly. They are inputs to the physiology model, not tuning parameters.

| Need | Amount | Notes |
| --- | --- | --- |
| Oxygen | ~0.84 kg | Rises with exertion; hard EVA ~0.10–0.15 kg/h |
| CO₂ produced | ~1.0 kg | Must be scrubbed or vented |
| Water | 2.5–3.5 kg drinking | More for hygiene; recycling reduces net loss |
| Food | ~0.6 kg dry, 2500–4000 kcal | Upper range in cold or heavy EVA |
| Buffer gas (N₂) | Airlock cycles and leaks | Avoidable with a low-pressure pure-O₂ habitat at the cost of fire risk and prebreathe ([resources.md](resources.md)) |

Related: [resources.md](resources.md), [planet-gen.md](planet-gen.md).
