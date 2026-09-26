# Project README — Seamless Procedural Universe

**Status:** Design phase (pre-production)  
**Last updated:** September 2026

## What This Is

A game built around one core fantasy: a single, unbroken universe you can zoom into — from the cosmic web all the way down to the ground beneath your feet — and survive on, indefinitely, once you land.

Structurally, it combines two proven ideas that are rarely combined well:

- **Seamless scale traversal**, in the spirit of No Man's Sky — no loading screens between space and surface, no separated "modes." One continuous universe.
- **Indefinite personal survival** — the core, permanent gameplay loop is managing your own survival on a hostile planet, forever, with gear progression as the reward structure. Not a means to an end (like escaping or winning) — survival is the game.

Robot-operated colonies, base-building, and large-scale automation are part of the long-term vision but are explicitly deferred — they are not part of the current design pass or the near-term roadmap. The project is being built in deliberately narrow, provable slices.

## The Core Pillars

### 1. Seamless Scale

The universe is structured as 8 canonical zoom levels, from the cosmic web down to standing on terrain, with an acknowledged (but currently unbuilt) subterranean layer beneath that. Full detail lives in the scale table (see game_spec.md), but the short version:

```
Universe → Galactic → Stellar System → Planetary System → Orbital Expanse → Atmospheric → Terrain → Subterranean
```

The hardest, highest-risk part of this system is the orbit-to-surface transition (levels 5→6→7) — this is where most games in this genre show visible seams or compromises, and it's treated as the single most important technical problem to solve first.

### 2. Indefinite Survival

Once a player lands, the game becomes about managing four independent survival stats: **Oxygen**, **Hydration**, **Hunger**, and **Temperature**. Three of these are ongoing resource hunts (you're always low on something); Temperature is the exception — a gear-preparation check rather than a consumable. All harm is escalating and recoverable if the player reacts in time — nothing kills instantly. Gear upgrades reduce consumption and risk rather than adding new systems, which is what lets survival stay interesting indefinitely without needing new mechanics bolted on over time.

### 3. Procedural Planets (Future)

Every planet is meant to offer some version of every core resource, just reskinned to fit its biome — an ice planet might have abundant fuel but scarce oxygen, a toxic planet the reverse. This is what gives exploration purpose: you're not hunting exotic materials, you're hunting enough of the basics, and the mix changes everywhere you go. Not yet implemented — current planets are hand-built, not procedural.

### 4. Building & Robot-Operated Colonies (Future, deferred)

The long-term vision includes a building system the player uses to establish colonies after landing, operated primarily through robots rather than direct manual labor. Two robot roles are planned:

- **Worker bots** — handle repetitive tasks on the player's behalf (resource extraction, hauling, maintenance), so the colony automates itself over time rather than demanding constant manual attention.
- **Warrior bots** — defend the player and the colony, handling combat so the player isn't solely responsible for base defense.

Eventually, colonies extract resources, sustain themselves, and support launching onward to new systems. This layer has been intentionally set aside so the foundational systems (scale traversal, survival) can be proven first, on their own, without colony complexity muddying the test.

### 5. Creatures & Riding (Future, deferred)

Planets are intended to host varied native creatures, some of which the player can tame and ride for traversal. This is planned as a later addition once base survival and terrain systems are solid — creatures need believable habitats and behavior to be worth building, which depends on the procedural planet work landing first.

### 6. Vehicles & Machines

Traversal and colony operation are expected to rely on a range of player-operable machines beyond the base character on foot: ground vehicles for surface travel, aircraft for atmospheric flight, and ships for orbital and interplanetary travel, alongside whatever supporting machines the building and colony systems require (e.g., transport for resources, construction equipment). These tie directly into the scale system — vehicles and aircraft are the natural way a player would move through the Atmospheric and Terrain layers, and ships handle the Orbital Expanse and beyond. Not yet designed in detail; noted here as a required system once traversal needs to extend past walking on foot.

## Design Philosophy

A few principles have guided every decision so far, worth stating explicitly so future additions stay consistent:

- **Simplicity over completeness.** When faced with a choice between a richer system and a simpler one, simpler has consistently won (e.g., 4 survival stats instead of more, universal resource categories instead of true per-planet chemistry).
- **Escalating, recoverable harm.** No mechanic in the game kills the player outright from neglect. Every stat gives a warning window. This keeps the system teachable even as content varies wildly.
- **Prove the hardest thing first.** Rather than building broad and shallow, each milestone targets the single riskiest unknown (right now: can the scale transition actually feel seamless?) before adding scope on top of it.
- **Universal systems, local flavor.** Rather than invent new mechanics per planet or per layer, the same small set of systems (4 stats, 8 scale levels) is reused everywhere, with only the content changing. This keeps the design buildable at a small team/solo scale despite the scope of the premise.

## Current State of the Project

- **Scale system:** Fully specified across all 8 levels (see game_spec.md), with a stage-0 cosmic web prototype already in progress (referenced as v0.3.2).
- **Survival system:** Fully specified at the design level (all 4 stats defined with resource loops and risk mechanics). Not yet implemented.
- **MVP in progress:** A narrow vertical slice proving the scale/zoom system works end-to-end, Lv1 through Lv7, with no survival mechanics and no procedural content. See game_spec.md for full MVP scope.
- **Not started:** Procedural generation, building system, colonies, worker/warrior robots, creatures and riding, vehicles/aircraft/ships and other machines, combat, subterranean geometry, gear progression tiers.

## Roadmap Shape (High Level, Not Yet Scheduled)

1. **Prove the scale system** — current MVP. No survival, no content, just seamless traversal.
2. **Prove the survival loop** — a second, separate MVP: one hand-built planet, all 4 stats live, minimal tools. Proves the other core pillar in isolation.
3. **Combine the two** — survival on a planet reached by actually flying down through the scale system, rather than a hand-placed test scene.
4. **Introduce proceduralism** — planets generated rather than hand-built, resource "reskinning" logic applied.
5. **Introduce building and robot-operated colonies** — the building system, worker bots for repetitive tasks, and warrior bots for defense, once the foundation is proven solid.
6. **Introduce creatures and riding** — native creatures with believable habitats and behavior, tameable for traversal.
7. **Introduce vehicles and machines** — ground vehicles, aircraft, ships, and colony-support machinery, extending traversal beyond walking on foot.
8. **Subterranean domain** — real geometry below the surface, beyond the current placeholder markers.

This ordering is deliberate: each stage only adds one major unproven system at a time, so failures are easy to isolate.

## Companion Documents

- **game_spec.md** — detailed spec: full 8-level scale table, full survival system table, and the current MVP definition with pass/fail criteria.

This README is the standing overview; game_spec.md is where implementation-level detail lives and will keep expanding as each system is worked out further.
