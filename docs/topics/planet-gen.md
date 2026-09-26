# Planet generation pipeline

Provenance: `specs.md` §5 plus the stages-4/6/7 availability-profile bullet from §7.9.

Deferred; not part of the MVP. Documented here because [scale.md](scale.md), [survival.md](survival.md), [machines.md](machines.md) and [resources.md](resources.md) consume its outputs.

Worlds are produced by a deterministic chain from the seed. Each stage takes the previous stage's output and adds one layer of consequence.

1. **Star.** Mass, age and luminosity. Sets the energy available to everything in the system and the radiation environment.
2. **Disk.** Mass and metallicity of the protoplanetary disk. Sets how much rock, metal and volatile material exists to build planets from.
3. **Planets.** For each body: mass, radius, orbital distance, eccentricity, rotation. Sets surface gravity, day length, and how much energy the body receives.
4. **Volatiles and atmosphere retention.** Whether the body can hold an atmosphere at all, and of what. Depends on gravity, temperature, and whether a magnetosphere protects it.
5. **Greenhouse balance.** Surface temperature and pressure that result from the atmosphere it kept. Feeds heat balance and breathing gas directly.
6. **Water phase.** Whether water is ice, liquid, vapor, or absent given that temperature and pressure. Feeds water and electrolytes.
7. **Geology.** Tectonics, volcanism, erosion. Produces terrain, regolith composition including contaminants (perchlorates, heavy metals, sulphates), mineral concentrations, caves, geothermal gradient. Feeds terrain, subterranean, construction, and the resource profile.
8. **Biology.** Only where every prior stage allows it. Rare by construction. Feeds contamination, creatures, and (under the concession in [resources.md](resources.md)) food.

Most worlds the chain produces are dead. That is intended: exploration is the search for, and the reading of, the rare habitable window. Resources are not assigned; they exist where the chain put them. Stages 4, 6 and 7 together emit a resource availability profile per body ([resources.md](resources.md)): stage 7 (geology) outputs regolith composition including contaminants; stage 6 outputs ice depth and distribution; stage 4 outputs atmospheric composition by species.

The pipeline runs once per seed and is cached. It is arithmetic, not rendering, and fits the mobile budget.

Related: [scale.md](scale.md), [survival.md](survival.md), [machines.md](machines.md), [resources.md](resources.md).
