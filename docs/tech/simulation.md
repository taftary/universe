# Simulation — frames, precision, units, time-warp, determinism

**Status:** M0 locked. Implements D-006 and the determinism half of [specs.md](../specs.md) section 2.

## Frames for navigable levels 3-8

Levels 1-2 (Universe, Galactic) are a static backdrop asset, never simulated. Levels 3-8 nest:

```text
Stellar System (Lv3, star-centered inertial, meters f64)
  Planetary System (Lv4, body-centered inertial, meters f64)
    Orbital Expanse (Lv5, local orbital frame, meters f64)
      Atmospheric (Lv6, topocentric East-North-Up, meters f64)
        Terrain (Lv7, local tangent plane, meters f64)
          Subterranean (Lv8, deferred, same frame as Lv7)
```

One active center at a time. Handoffs translate state between parent and child frames without rescaling units. Every readout required by [specs.md](../specs.md) section 8 stays continuous in both directions. The orbit to surface handoff is `C0`-exact by construction: the atmosphere tapers to vacuum at 120 km (D-016) so pressure and density meet the vacuum state with no jump; see Reference planet below.

## Floating origin

- Sim holds world positions and velocities in f64 (`glam::DVec3`, `glam::DQuat`).
- A single camera-relative conversion point turns f64 sim coordinates into f32 render coordinates. Only `engine::render` calls it. Render-side structs that cross the boundary are plain data (`bytemuck` `Pod`) so the conversion stays a memcopy-safe cast.
- Origin rebase threshold is named: `ORIGIN_REBASE_DISTANCE_M = 5000.0 meters`, chosen so f32 mantissa error stays below visual tolerance at terrain scale (f32 ULP at 5 km is about 0.5 mm). Source: project error budget (first scaffold measures it).
- No other module converts precision. Passing f32 into sim is a bug.

## Unit newtypes

- All gameplay quantities are newtypes over f64: `Meters(f64)`, `Seconds(f64)`, `Kilograms(f64)`, `Kelvin(f64)`, `Pascals(f64)`, `MetersPerSecond(f64)`, `MetersPerSecondSquared(f64)`.
- Heterogeneous operator outputs are explicit: `Meters / Seconds = MetersPerSecond`; `MetersPerSecond / Seconds = MetersPerSecondSquared`; `Pascals` never mixes with `Kelvin`.
- Boilerplate is macro-generated (`impl_units!` in `engine::units`). Hand-written impls are rejected in review.
- Ranged quantities use fallible constructors in `thiserror` style: `Kelvin::new(value_kelvin_f64)` returns `UnitError::BelowAbsoluteZero` when `value_kelvin_f64 < 0.0`. Absolute zero constant: `ABSOLUTE_ZERO_K = 0.0 kelvin`, source: SI definition.
- Raw f64 never crosses a module boundary. Suffixes (`_m`, `_s`, `_pa`, `_k`) are required on locals that have not yet been wrapped.
- Reference-planet sampling follows the same rule with explicit newtype signatures: `BodyParams::gravity_at_altitude(altitude: Meters) -> Result<MetersPerSecondSquared, BodyError>` and `AtmosphereParams::sample_at_altitude(altitude: Meters, body: &BodyParams) -> Result<AtmosphereState, AtmosphereError>`. Scalar coefficients without a newtype (`lapse_rate_k_per_m_f64`, `gas_constant_j_per_kg_k_f64`) carry units in their names. Bare f64 physical quantities in any other public signature are rejected in review.

## Reference planet (D-015/D-016)

Hand-tuned Mars-like planet and thin carbon-dioxide atmosphere for the M1 descent path in [specs.md](../specs.md) section 8. Anchors per D-015: `p0 610 Pa`, `T0 210 K`, `R 3389500 m`, `g0 3.71 m/s2`, `M 6.4171e23 kg` (NASA Mars Fact Sheet); analytic hydrostatic profile; Mars Climate Database is a validation envelope only.

- Temperature is two-segment per D-016: `T(z) = T0 + L1 * z` with `L1 = -0.0012 K/m` to 50 km, isothermal 150 K above. Stored as a positive cooling rate `MARS_LAPSE_RATE_K_PER_M = 0.0012 K/m` so `T(z) = T0 - rate * z` below the tropopause.
- Gravity is altitude-dependent per D-016: `g(z) = g0 * (R / (R + z))^2` via `BodyParams::gravity_at_altitude` (`mu / (R + z)^2`). Pressure integrates `dp/dz = -p * g(z) / (R_specific * T(z))` from the surface in fixed `PRESSURE_INTEGRATION_STEP_M = 100.0 m` slabs with midpoint temperature and gravity; `libm` provides the exponentials so x86_64 and AArch64 agree.
- Cutoff 120 km with a 100-120 km linear taper to exactly 0 keeps the atmosphere-to-vacuum handoff `C0` continuous. Profile is `C0`-continuous on 0-120000 m with monotonic pressure and density.
- Reference values from the locked profile: 610 Pa / 210 K at the surface, 224.4 Pa within 5 percent at one scale height (10695 m, measured 217.0 Pa), about 2.6 Pa at 50 km, about 0.005 Pa at 100 km, exactly 0 at 120 km.

## Integrators and scaling

- Coasting orbits propagate analytically (Kepler propagation, on rails under warp per the time-warp rules below). Numerical integration (semi-implicit Euler at MVP, velocity Verlet when the error budget needs it) is only for powered flight and atmospheric descent. Energy drift is tested as an invariant in [quality.md](quality.md).
- Internal non-dimensionalisation is allowed inside a solver for conditioning. SI remains the interface at the gameplay level per [specs.md](../specs.md) section 2.
- Step sizes are named constants with units, locked in `tech.md` (D-012). No frame-rate-dependent `dt`.

## Time-warp state machine

Mirrors [specs.md](../specs.md) section 2 rules exactly:

```text
1x [on foot or EVA, always allowed]
  10x, 100x, 1000x, 10000x [only inside a ship in orbit or transit]
```

- Warp drops to 1x automatically on atmospheric entry, on approach to any body or object, and whenever any physiological alarm is raised.
- Under warp, orbits propagate analytically (on rails). Consumables and physiology integrate at coarse steps sized to the warp factor.
- Maximum warp constant: `MAX_WARP_FACTOR = 10000.0 dimensionless`, source: [specs.md](../specs.md) section 2. Minimum tick: `SIM_TICK_S = 0.05 s` (D-012), never derived from frame time.

## Seeds and procedural content

- Hierarchical seeds with domain separation: `master_seed_u64` splits into `gen_star`, `gen_body`, `gen_terrain` streams via distinct SplitMix64 domain tags. Changing one domain never changes another (D-013).
- Project PRNG is pinned to xoshiro256** via `rand_xoshiro` (`rand_core` traits for injection). No device RNG in sim or gen. Algorithm and domain tags are recorded here per D-013.
- Procedural content is never saved. Only the seed and player-placed state persist; see [persistence.md](persistence.md).

## Determinism rules

- Injected RNG, clock, and IO. Sim takes a project `ProjectRng` built on `TryRng` with the blanket `Rng` impl (xoshiro256** per D-013; rand_core 0.10 deprecates the old single core trait), a `tick count` clock, and no direct file or network access.
- Project-owned PRNG. No device RNG in sim or gen. Algorithm is pinned (D-013) and recorded in Seeds above.
- No platform `libm` transcendentals in sim. Use the pinned project math path so x86_64 and AArch64 agree.
- No `HashMap` iteration in sim. Order-dependent state uses `BTreeMap` or indexed vectors.
- Fixed `rayon` fold order where parallelism exists. Document the split count; test with two thread counts.
- No `target-cpu=native`. Release flags are shared; see [standards.md](standards.md).
- Golden-hash tests run across x86_64 and AArch64. Snapshots hash with xxh3-64 via `xxhash-rust` (D-014). Same inputs yield the same state hash to within documented floating-point noise. Policy in [quality.md](quality.md).

Related: [../tech.md](../tech.md), [architecture.md](architecture.md), [quality.md](quality.md), [standards.md](standards.md).
