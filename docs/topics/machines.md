# Machines

Provenance: `specs.md` §6. Propellant consequences live in [resources.md](resources.md).

Deferred beyond the ship's trajectory (see [mvp.md](mvp.md)). Every machine is a physical object with a budget.

**Ships.** Dry mass, propellant mass, specific impulse, thrust, heat-shield capacity, life-support consumables per crew-hour. Maneuvers cost propellant according to mass and engine performance. Reentry is a trajectory with heating and g-load; exceeding heat-shield or structural limits destroys the ship. Time-warp is available only inside a ship in orbit or transit ([specs.md](../specs.md#2-global-conventions)). The ship carries its own return propellant; landed mass is bounded by the mass needed to leave ([resources.md](resources.md)). Landing is a commitment; see propellant in [resources.md](resources.md).

**Aircraft.** Lift and drag derive from the actual atmospheric density of the current world. Thin atmospheres barely support flight; thick ones fly easily and impose their own pressure hazards.

**Ground vehicles.** Traction, power, thermal management, dust and terrain interaction. Range is an energy budget.

**Colony machinery.** Power sources, radiators, extraction and processing equipment. Only meaningful once power, thermal and material budgets are simulated ([resources.md](resources.md) and [specs.md](../specs.md)).

Related: [scale.md](scale.md), [resources.md](resources.md), [mvp.md](mvp.md).
