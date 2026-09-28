# MVP — Abstract Real-Scale Descent

Provenance: `specs.md` §8 (8.1-8.8). Constants live in `docs/tech.md` and `docs/tech/simulation.md`; gravitational parameters live in [scale.md](scale.md). This file keeps the readout list and the locked-profile reference values only.

## 8.1 Purpose

Prove that a real-scale, physically continuous orbit → surface → orbit path runs on a mid-range phone within budget, before any rendering or generation exists. This isolates the single riskiest technical claim in the project: that the simulation itself is cheap enough and continuous enough to build everything else on.

## 8.2 Representation

Nothing is rendered in the conventional sense. There are no meshes, textures, terrain, sky, or lighting. Every entity is an abstract mark:

- Star: a point.
- Planet: a circle at real radius.
- Atmosphere: concentric circles marking layer boundaries.
- Orbits and trajectories: curves.
- Ship / player: a point.
- Surface: a grid.
- Instruments: numeric readouts and readout-over-time plots.

Everything is hand-placed from published reference data. There is no generation. The marks paint on a 2D abstract canvas (immediate-mode egui painter through the `Camera2D` side view); this is presentation only with no value change, and there are still no meshes, textures, terrain, sky, or lighting.

## 8.3 Scope

- One star, as a point with real mass.
- One planet, Mars-like: real radius, mass, surface gravity, rotation, and a thin carbon-dioxide atmosphere with a reference pressure and temperature profile by altitude. Chosen because reference data is abundant and unambiguous; swappable for another archetype without changing the MVP's structure. Anchors and profile per D-015/D-016 in [tech.md](../tech.md) with detail in [simulation.md](../tech/simulation.md).
- No moons, no other bodies.
- Player controls a point-ship: prograde and retrograde burns, time-warp from 1x to 10,000x under the [Global Conventions](../specs.md#2-global-conventions) rules, atmospheric entry, descent to the surface grid, landing, ascent back to a stable orbit. The [machines.md](machines.md) ship trajectory uses the same body gravity `g(z)` as the atmosphere pressure integration, so orbit and atmosphere share one gravity field. Burn execution is trajectory scope, not part of the coasting model.

## 8.4 Required readouts

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

Reference model (D-015/D-016). Analytic hydrostatic profile anchored on the NASA Mars Fact Sheet; Mars Climate Database is a validation envelope only. Locked decisions in [tech.md](../tech.md); profile detail in [simulation.md](../tech/simulation.md).

Reference values from the locked profile: 610 Pa / 210 K at the surface; 224.4 Pa within 5 percent at one scale height (10695 m); about 2.6 Pa at 50 km; about 0.005 Pa at 100 km; exactly 0 at 120 km. Profile is `C0`-continuous on 0-120000 m with monotonic pressure and density.

Reference orbit model (D-017/D-018). Coasting orbits use classical Keplerian elements with analytic propagation on rails under warp; patched-conics handoffs use Laplace sphere-of-influence radii. Burns are trajectory scope, not part of the coasting model. Locked decisions in [tech.md](../tech.md); solver detail in [simulation.md](../tech/simulation.md). Gravitational parameters for tests live in [scale.md](scale.md) and are not duplicated here.

Reference trajectory model (D-019/D-020/D-021). Point-ship burns, rails-to-atmosphere integration, flight regimes, and surface contact share one body gravity field and one corotating wind, with a `C0`-exact handoff at 120 km in both directions. Locked decisions in [tech.md](../tech.md); stepping, drag, heating, regime, and touchdown detail in [simulation.md](../tech/simulation.md).

## 8.5 Explicitly cut

Rendering of any kind; generation; physiology and the suit; terrain tiles; weather; moons; additional bodies; ship mass or propellant budget (the point-ship has unlimited delta-v for this test).

## 8.6 Transition requirements

- No discontinuity in any readout at the orbit ↔ atmosphere ↔ surface handoffs, in either direction.
- The player can reverse direction at any moment without breaking simulation state.
- Warp rules honored per [Global Conventions](../specs.md#2-global-conventions): automatic drop to 1x on entry, approach, and alarm.
- 30 fps floor on the reference phone with the full simulation running at every warp factor.

## 8.7 Pass/Fail

The MVP passes only if all four hold:

1. **Continuity.** Plotted readout curves show no jumps at regime boundaries, and behave the way the underlying physics says they should (for example, pressure and density rise smoothly as altitude falls).
2. **Repeatability.** The same sequence of inputs produces the same mission profile on every run, to within floating-point noise, on every device tested.
3. **Budget.** Frame time never exceeds the 30 fps floor on the reference device during a complete descent and ascent, at any warp factor, under sustained load.
4. **Legibility.** A tester who has not seen the build can descend from orbit to the grid and return to orbit unprompted, using only the readouts.

## 8.8 Primary risk

That real-scale double-precision simulation with analytic orbit propagation and an atmosphere profile cannot hold the frame budget on the reference phone, or cannot be made continuous across the handoffs. Everything downstream depends on this being false.

Related: [scale.md](scale.md), [machines.md](machines.md), [specs.md](../specs.md#2-global-conventions).
