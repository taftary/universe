# Game Design Spec — Seamless Procedural Universe Survival

Status: Draft v0.1 — MVP defined, survival system defined, full-scale system defined for later iterations  
Last updated: September 2026

## 1. Vision

A seamless, zoomable universe — from the cosmic web down to a single blade of grass on a planet's surface, with no loading screens. Structurally similar in ambition to No Man's Sky, but built around a survival-first progression loop rather than a builder/colony core.

Core design philosophy: Survival is not a means to an end — it is the game. The loop must remain compelling indefinitely, without relying on a builder/colony layer to sustain long-term interest.

## 2. The Eight Canonical Scale Levels

Ordered widest → deepest. Each level's parent is the row above; its child is the row below. Real-world ranges are reference only — the game compresses these ranges for playability.

| Lv | Name | Real-world reference | Game extent (compressed) | v1 representation | Key entities | Domain |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | Universe Level | 10²⁶ m | Container only: one generated galaxy per save | Generated cosmic web (stage 0, v0.3.2): player-traversable in the Game Demo; cosmic-web backdrop in the galaxy map | Cosmic web, galaxy cluster cues | Deep Space |
| 2 | Galactic Scale | 10²¹ m | 10k–100k light-years (compressed) | Star points + nebula impostors, log/compressed space | Spiral arms, galactic core, interstellar medium | Deep Space |
| 3 | Stellar System Level | 10¹³ m | AU-scale (compressed) | Orbital map, patched positions | Central star, planetary orbits, Kuiper belt, heliosphere | Deep Space |
| 4 | Planetary System Domain | 10⁷–10⁸ m | 2–64 km gameplay radius (not real-Earth scale) + visual companions | Spherical LOD mesh, far view; moons/rings as backdrop | Main planet, moons, ring systems, planet shadows | Orbital & Atmospheric |
| 5 | Orbital Expanse | 10⁵–10⁶ m | 5–50 km altitude | Ship/orbital camera, planet fills view | Low/high orbits, magnetosphere (visual), reentry corridors, stations (future) | Orbital & Atmospheric |
| 6 | Atmospheric & Sky Boundary | 10²–10⁴ m | 0–10 km | Scattering shell + sky transition | Thermosphere, troposphere, weather (visual), flight altitudes | Orbital & Atmospheric |
| 7 | Terrain & Human Dimension | 10⁰–10³ m | 0–2 km → ground | Terrain chunks, colonies, robots | Topography, ecosystems, human structures, bodily scale | Surface & Biological |
| 8 | Subterranean Domain | 10⁻¹–10⁶ m | (below surface) | v1: surface markers only | Caves/vents as POI markers on the surface map; no geometry | Planetary Interior |

## 3. Survival System (Design Target — Post-MVP)

Note: Not part of the current MVP (see Section 4). Documented here as the design target once the MVP proves the scale/navigation system works.

Philosophy: Survival is the core, indefinite loop. Every stat decays and threatens the player, but harm is escalating and recoverable if the player acts in time — no instant death from neglect. Gear, biome awareness, and resource planning are part of the game's baseline tension.

| Stat | Decay | Resource loop | Risk mechanic |
| --- | --- | --- | --- |
| Oxygen | Universal slow drain, everywhere | Canisters, crafted from a common gas/mineral resource | Zero-O₂ = escalating debuff/damage, survivable if the player acts fast |
| Hydration | Universal slow drain | Water sources, visually indistinguishable clean/dirty | Scanning (costs battery/charge) reveals a graded contamination level (not binary); drinking blind = escalating risk |
| Hunger | Universal slow drain | Foraging + hunting + solo micro-farming (small-scale, no colony required) | Food has a time-based shelf life; spoiled food is permanently ruined and causes damage (same escalating logic as all other hazards) |
| Temperature | Not consumable — a gear check, not an ongoing resource | Correct suit/clothing per biome (two-sided: heat AND cold) | Wrong or inadequate gear = escalating debuff/damage. No fuel resource requirement; temperature is handled as equipment adequacy, not a consumable loop |

Shared design language:

- 3 of 4 stats (Oxygen, Hydration, Hunger) are active, ongoing resource hunts.
- Temperature is the exception: a one-time gear-preparation check per biome, not a continuous drain.
- All harm across all stats follows the same escalating-debuff logic — no stat is instantly lethal. This keeps the system simple to learn even as planet conditions vary wildly.
- Every planet is expected to offer some version of each resource type, reskinned to fit its biome (per the "universal resource, local flavor" design principle).

## 4. MVP — "Seamless Scale Prototype"

### 4.1 Purpose

Prove that a player can travel from the cosmic web (Lv1) down to standing on a planet's surface (Lv7), and back up again, with no loading screens, no jarring pop-in, and no broken sense of scale.

This MVP tests the game's single riskiest and most defining technical promise — seamless scale transition — in isolation, before any survival, resource, or content systems are layered on top.

### 4.2 Explicit Assumption

The MVP is a thin vertical slice through all 8 levels, each built at its cheapest possible representation. It is not a demonstration of gameplay depth — it is a demonstration that the navigation/rendering stack works end-to-end across scale.

### 4.3 Definition of Done

A single, unbroken, player-controlled (not scripted/cutscene) camera path from Lv1 to Lv7, and back, that does not break immersion.

### 4.4 Scope per Level

| Lv | Name | MVP build target | Explicitly cut for MVP |
| --- | --- | --- | --- |
| 1 | Universe | Reuse existing stage-0 cosmic web (v0.3.2) — static, one generated galaxy, player can fly toward it | Multiple galaxies, real cosmic structure accuracy |
| 2 | Galactic | Star points as billboarded impostors + one nebula sprite; no real galactic simulation | Real star density, interstellar medium effects |
| 3 | Stellar System | One hand-placed star + one hand-placed planet on a fixed orbit path (no physics simulation) | Multiple planets, Kuiper belt, heliosphere visuals |
| 4 | Planetary System | One spherical LOD planet mesh, far-view only, no moons/rings yet | Moon(s), ring systems, probes/rockets |
| 5 | Orbital Expanse | Fly-down camera path, planet fills view progressively; scripted approach corridor, no orbital mechanics | Stations, magnetosphere visuals, multiple orbit altitudes |
| 6 | Atmospheric | Single scattering shader shell, fades in as the boundary altitude is crossed | Weather, dynamic sky, flight physics |
| 7 | Terrain | One hand-built terrain chunk (a few km²), flat-ish with basic height variation, player can walk on foot | Ecosystems, structures, robots, colonies, multiple biomes |
| 8 | Subterranean | Fully out of MVP — not even POI markers yet | Everything |

### 4.5 Core Transition Requirements

These are the actual hard engineering problems the MVP exists to solve:

- No loading screens between any level, in either direction (descending or ascending).
- No visible pop-in/seam, specifically at the Lv5 → Lv6 → Lv7 boundary (orbit-to-surface). This is the single highest technical risk in the entire project and deserves disproportionate engineering attention.
- Player must be able to reverse direction at any point (start descending, change their mind, ascend again) without breaking game state.
- Frame rate must stay within a defined acceptable band throughout — recommend setting an explicit minimum (e.g., 30fps) so "working" is a measurable target, not a subjective impression.

### 4.6 Explicitly Out of Scope for This MVP

- All 4 survival stats (Oxygen, Hydration, Hunger, Temperature)
- Any resource gathering, crafting, or scanning
- Colonies, robots, combat
- Procedural generation at any level (everything is hand-placed/hard-coded)
- Multiple star systems or planets
- Subterranean domain, including surface POI markers

### 4.7 Pass/Fail Test

Put a player who has not seen the build in front of it and ask them to fly from deep space down to the planet's surface, and back up, unprompted.

Success criteria: They do not notice a "seam," do not ask "did it just load something?", and describe the transition as one continuous motion.

This is a concrete, testable bar — not just "the code runs without crashing."

### 4.8 Immediate Risk Flag

The Lv5 → Lv6 → Lv7 transition (orbit to surface) is where almost every game in this genre (No Man's Sky, Star Citizen) has invested the most engineering effort and still shows visible compromises. This is the primary technical gate for the project.

## 5. Deferred to Future Iterations (Not MVP)

- Full survival system implementation (Section 3)
- Robot-operated colonies
- Building/base construction
- Combat
- Procedural planet generation
- Multiple star systems, planets, moons
- Subterranean Domain geometry (currently surface-marker-only, per Lv8 spec)
- Farming beyond solo micro-scale
- Gear/tool progression tiers (scanner tiers, suit tiers, etc.)

## 6. Open Questions for Next Pass

- Gear/progression curve details (tiers, unlock conditions) — flagged as a follow-up in earlier design discussion, not yet defined.
- How resources map to specific planets (the "reskinning" logic for O2/water/food/temperature per biome).
- Death/failure state specifics — what happens when stat decay is left unmanaged for too long (compounding effects? game over? nothing permanent?).
- Whether Lv1–4 need any interactive content in a post-MVP pass, or remain pure navigation layers indefinitely.
