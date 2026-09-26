//! Point-ship trajectory: burns, drag, heating, and rails handoffs.
//!
//! Owns powered and atmospheric flight below the rails cutoff. Above
//! [`RAILS_ALTITUDE_M`] steps delegate to analytic rails in
//! [`crate::orbit`]; at or below the cutoff they use semi-implicit Euler
//! with shared gravity from [`crate::body`] and density from
//! [`crate::atmosphere`]. Regime labels and warp mapping live in
//! [`crate::regime`]; surface contact lives in [`crate::surface`].
//!
//! All transcendentals use `libm` so `x86_64` and `AArch64` agree. No
//! `std` sine, cosine, square root, exponential, or power appears here,
//! including in tests. `DVec3` helpers used here (`length_squared`, `dot`,
//! `cross`, operators) are pure IEEE arithmetic, never transcendentals.

use glam::DVec3;
use thiserror::Error;

use crate::atmosphere::{AtmosphereError, AtmosphereParams};
use crate::body::{BodyError, BodyParams};
use crate::orbit::{ClassicalElements, Mu, OrbitError, advance};
use crate::units::{Meters, MetersPerSecond, Seconds};

/// Rails cutoff altitude in meters.
///
/// Steps starting above this altitude coast on analytic rails. Equals
/// [`crate::atmosphere::ATMOSPHERE_CUTOFF_ALTITUDE_M`]; the taper below it
/// keeps the handoff `C0` continuous. Source: issue 4 step 1 handoff.
pub const RAILS_ALTITUDE_M: f64 = 120_000.0;

/// Preset ballistic coefficient in kilograms per square meter.
///
/// Per-vehicle drag scaling `B = m / (Cd * A)`. Source: issue 4 step 1
/// handoff (physicist preset for the M1 point-ship).
pub const PRESET_BALLISTIC_COEFFICIENT_KG_PER_M2: f64 = 120.0;

/// Preset nose radius in meters for stagnation heating.
///
/// Source: issue 4 step 1 handoff (physicist preset for the M1 point-ship).
pub const PRESET_NOSE_RADIUS_M: f64 = 1.0;

/// Sutton-Graves heating constant in SI units.
///
/// Stagnation-point correlation `q = k * sqrt(rho / r_n) * v^3`.
/// Source: Sutton and Graves (1971), SI value `1.9027e-4`.
pub const SUTTON_GRAVES_K_SI: f64 = 1.9027e-4;

/// Standard gravity in meters per second squared for g-load scaling.
///
/// G-load is aerodynamic acceleration over this constant. Source: exact
/// SI definition of standard gravity (`9.80665 m/s^2`).
pub const STANDARD_GRAVITY_MPS2: f64 = 9.80665;

/// Maximum step duration in seconds for atmosphere integration.
///
/// Atmosphere legs must use `dt <=` this value to keep the
/// semi-implicit Euler error bounded. Equals [`crate::sim::SIM_TICK_S`];
/// rails legs may use warp-scaled steps via [`crate::warp::tick_at_warp`]
/// but any split leg that integrates never exceeds one tick because warp
/// auto-drops to `X1` on entry. Source: D-012 via `SIM_TICK_S`.
pub const MAX_STEP_S: f64 = crate::sim::SIM_TICK_S.value();

/// Eccentricity below which an orbit converts as circular, dimensionless.
///
/// Periapsis is undefined there, so conversion pins it to zero.
/// Source: issue 4 step 2 handoff (mathematician guard).
pub const CIRCULAR_ECCENTRICITY_THRESHOLD: f64 = 1e-8;

/// Inclination below which an orbit converts as equatorial, in radians.
///
/// The node is undefined there, so conversion pins it to zero.
/// Source: issue 4 step 2 handoff (mathematician guard).
pub const EQUATORIAL_INCLINATION_THRESHOLD_RAD: f64 = 1e-8;

/// Full circle in radians for anomaly wrapping.
///
/// Mirrors [`crate::orbit`] without duplicating its constant.
/// Source: `core::f64::consts::TAU`.
const TAU_RAD: f64 = core::f64::consts::TAU;

/// Ballistic coefficient in kilograms per square meter.
///
/// Drag scaling `B = m / (Cd * A)`; larger values coast further.
/// See [`VehicleParams`] for the per-vehicle preset.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BallisticCoefficient(f64);

impl BallisticCoefficient {
    /// Create a ballistic coefficient from its SI magnitude.
    ///
    /// # Errors
    ///
    /// Returns [`TrajectoryError::InvalidBallisticCoefficient`] when
    /// `value_kg_per_m2` is not positive and finite.
    pub fn new(value_kg_per_m2: f64) -> Result<Self, TrajectoryError> {
        if value_kg_per_m2.is_finite() && value_kg_per_m2 > 0.0 {
            Ok(Self(value_kg_per_m2))
        } else {
            Err(TrajectoryError::InvalidBallisticCoefficient {
                value_kg_per_m2_f64: value_kg_per_m2,
            })
        }
    }

    /// Return the inner SI magnitude in kilograms per square meter.
    #[must_use]
    pub const fn value(self) -> f64 {
        self.0
    }
}

/// Stagnation-point heat flux in watts per square meter.
///
/// Output of the Sutton-Graves correlation in [`heating`]; unbounded
/// like [`Meters`] and never negative from that path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeatFlux(f64);

impl HeatFlux {
    /// Create a heat flux from its SI magnitude.
    #[must_use]
    pub const fn new(value_w_per_m2: f64) -> Self {
        Self(value_w_per_m2)
    }

    /// Return the inner SI magnitude in watts per square meter.
    #[must_use]
    pub const fn value(self) -> f64 {
        self.0
    }
}

/// Point-ship vehicle parameters for drag and heating.
///
/// Ballistic coefficient scales drag; nose radius scales Sutton-Graves
/// heating. No lift is modelled at MVP.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VehicleParams {
    /// Ballistic coefficient in kilograms per square meter.
    ballistic_coefficient: BallisticCoefficient,
    /// Nose radius in meters.
    nose_radius_m: Meters,
}

impl VehicleParams {
    /// Create vehicle parameters from unit-typed values.
    ///
    /// # Errors
    ///
    /// Returns [`TrajectoryError::InvalidNoseRadius`] when the nose radius
    /// is not positive and finite.
    pub fn new(
        ballistic_coefficient: BallisticCoefficient,
        nose_radius_m: Meters,
    ) -> Result<Self, TrajectoryError> {
        let radius_f64 = nose_radius_m.value();
        if !radius_f64.is_finite() || radius_f64 <= 0.0 {
            return Err(TrajectoryError::InvalidNoseRadius {
                value_m_f64: radius_f64,
            });
        }
        Ok(Self {
            ballistic_coefficient,
            nose_radius_m,
        })
    }

    /// M1 point-ship preset for tests and tools.
    ///
    /// Uses [`PRESET_BALLISTIC_COEFFICIENT_KG_PER_M2`] and
    /// [`PRESET_NOSE_RADIUS_M`]; valid by construction and never fails.
    #[must_use]
    pub fn preset() -> Self {
        Self {
            ballistic_coefficient: BallisticCoefficient(PRESET_BALLISTIC_COEFFICIENT_KG_PER_M2),
            nose_radius_m: Meters::new(PRESET_NOSE_RADIUS_M),
        }
    }

    /// Ballistic coefficient in kilograms per square meter.
    #[must_use]
    pub fn ballistic_coefficient(&self) -> BallisticCoefficient {
        self.ballistic_coefficient
    }

    /// Nose radius in meters.
    #[must_use]
    pub fn nose_radius_m(&self) -> Meters {
        self.nose_radius_m
    }
}

/// Inertial ship state at one instant.
///
/// Body-centered inertial position and velocity plus mission epoch.
/// Fields are public for inspection; [`StateVector::new`] validates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StateVector {
    /// Inertial position in meters.
    pub position_m: DVec3,
    /// Inertial velocity in meters per second.
    pub velocity_mps: DVec3,
    /// Mission epoch in seconds.
    pub epoch: Seconds,
}

impl StateVector {
    /// Create a state vector with finiteness checks.
    ///
    /// # Errors
    ///
    /// Returns [`TrajectoryError::NonFinite`] when any position or
    /// velocity component, or the epoch, is not finite.
    pub fn new(
        position_m: DVec3,
        velocity_mps: DVec3,
        epoch: Seconds,
    ) -> Result<Self, TrajectoryError> {
        checked_vector_m(position_m)?;
        checked_vector_m(velocity_mps)?;
        if !epoch.value().is_finite() {
            return Err(TrajectoryError::NonFinite {
                value_f64: epoch.value(),
            });
        }
        Ok(Self {
            position_m,
            velocity_mps,
            epoch,
        })
    }
}

/// Impulsive burn direction along the velocity vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BurnDirection {
    /// Along the velocity vector; raises energy.
    Prograde,
    /// Against the velocity vector; lowers energy.
    Retrograde,
}

/// Impulsive maneuver applied instantaneously to a state.
///
/// Direction plus non-negative magnitude; see [`apply_burn`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Burn {
    /// Burn direction along the velocity vector.
    direction: BurnDirection,
    /// Burn magnitude in meters per second.
    delta_v: MetersPerSecond,
}

impl Burn {
    /// Create a burn from a direction and unit-typed magnitude.
    ///
    /// # Errors
    ///
    /// Returns [`TrajectoryError::InvalidDeltaV`] when the magnitude is
    /// negative or not finite.
    pub fn new(
        direction: BurnDirection,
        delta_v: MetersPerSecond,
    ) -> Result<Self, TrajectoryError> {
        let magnitude_f64 = delta_v.value();
        if !magnitude_f64.is_finite() || magnitude_f64 < 0.0 {
            return Err(TrajectoryError::InvalidDeltaV {
                value_mps_f64: magnitude_f64,
            });
        }
        Ok(Self { direction, delta_v })
    }

    /// Burn direction along the velocity vector.
    #[must_use]
    pub const fn direction(&self) -> BurnDirection {
        self.direction
    }

    /// Burn magnitude in meters per second.
    #[must_use]
    pub const fn delta_v(&self) -> MetersPerSecond {
        self.delta_v
    }
}

/// One trajectory step output at the end state.
///
/// Dynamics state plus the aerodynamic loads sensed there. Above the
/// rails cutoff all loads are exactly zero.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepSample {
    /// End-of-step state.
    pub state: StateVector,
    /// Drag acceleration vector in meters per second squared.
    pub drag_accel_mps2: DVec3,
    /// Stagnation-point heat flux.
    pub heat_flux: HeatFlux,
    /// G-load as a multiple of standard gravity, dimensionless.
    pub g_load_g: f64,
}

/// Point-ship trajectory and conversion failures.
#[derive(Debug, Error)]
pub enum TrajectoryError {
    /// Input or computed value was not finite.
    #[error("non-finite value: {value_f64}")]
    NonFinite {
        /// Rejected value.
        value_f64: f64,
    },
    /// Ballistic coefficient was not positive and finite.
    #[error("invalid ballistic coefficient: {value_kg_per_m2_f64} kg/m2")]
    InvalidBallisticCoefficient {
        /// Rejected coefficient in kilograms per square meter.
        value_kg_per_m2_f64: f64,
    },
    /// Nose radius was not positive and finite.
    #[error("invalid nose radius: {value_m_f64} m")]
    InvalidNoseRadius {
        /// Rejected radius in meters.
        value_m_f64: f64,
    },
    /// Burn magnitude was negative or not finite.
    #[error("invalid delta-v: {value_mps_f64} m/s")]
    InvalidDeltaV {
        /// Rejected magnitude in meters per second.
        value_mps_f64: f64,
    },
    /// Step duration was not positive and finite.
    #[error("invalid step: {step_s_f64} s")]
    InvalidStep {
        /// Rejected step in seconds.
        step_s_f64: f64,
    },
    /// Burn at near-zero velocity has no direction.
    #[error("burn at near-zero velocity has no direction")]
    ZeroVelocity,
    /// Position at the body center has no altitude.
    #[error("position at the body center has no altitude")]
    AtCenter,
    /// Unbound energy cannot ride analytic rails.
    #[error("unbound energy {energy_j_per_kg_f64} J/kg cannot ride rails")]
    Unbound {
        /// Rejected specific energy in joules per kilogram.
        energy_j_per_kg_f64: f64,
    },
    /// Degenerate angular momentum cannot define an orbital plane.
    #[error("degenerate angular momentum cannot define an orbital plane")]
    DegenerateAngularMomentum,
    /// Body gravity lookup failed.
    #[error(transparent)]
    Body {
        /// Source body error.
        #[from]
        source: BodyError,
    },
    /// Atmosphere sampling failed.
    #[error(transparent)]
    Atmosphere {
        /// Source atmosphere error.
        #[from]
        source: AtmosphereError,
    },
    /// Orbit rails propagation or conversion failed.
    #[error(transparent)]
    Orbit {
        /// Source orbit error.
        #[from]
        source: OrbitError,
    },
}

/// Point-ship dynamics interface for the sim loop.
///
/// Implementors advance one state; the environment (body, atmosphere)
/// stays outside the object so one ship can fly many bodies.
pub trait Trajectory: Send + Sync {
    /// Report the current state without advancing.
    #[must_use]
    fn state(&self) -> StateVector;

    /// Advance one step and report the end-state sample.
    ///
    /// # Errors
    ///
    /// Returns [`TrajectoryError`] for bad steps, unbound rails states,
    /// or failed environment sampling.
    fn step(
        &mut self,
        step: Seconds,
        body: &BodyParams,
        atmosphere: &AtmosphereParams,
    ) -> Result<StepSample, TrajectoryError>;
}

/// Point-ship trajectory holding vehicle, gravity, and state.
///
/// Steps with [`step_point_ship`]: analytic rails above
/// [`RAILS_ALTITUDE_M`], semi-implicit Euler below, exact-time substep
/// split when a step straddles the cutoff.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointShipTrajectory {
    /// Vehicle drag and heating parameters.
    vehicle: VehicleParams,
    /// Gravitational parameter for rails legs.
    mu: Mu,
    /// Current state.
    current: StateVector,
}

impl PointShipTrajectory {
    /// Create a trajectory from vehicle, gravity, and initial state.
    ///
    /// The state is stored as given; conversion failures surface on the
    /// first rails step, never here.
    #[must_use]
    pub const fn new(vehicle: VehicleParams, mu: Mu, initial: StateVector) -> Self {
        Self {
            vehicle,
            mu,
            current: initial,
        }
    }

    /// Vehicle drag and heating parameters.
    #[must_use]
    pub const fn vehicle(&self) -> VehicleParams {
        self.vehicle
    }

    /// Gravitational parameter for rails legs.
    #[must_use]
    pub const fn mu(&self) -> Mu {
        self.mu
    }
}

impl Trajectory for PointShipTrajectory {
    fn state(&self) -> StateVector {
        self.current
    }

    fn step(
        &mut self,
        step: Seconds,
        body: &BodyParams,
        atmosphere: &AtmosphereParams,
    ) -> Result<StepSample, TrajectoryError> {
        let sample = step_point_ship(
            &self.current,
            step,
            body,
            atmosphere,
            &self.vehicle,
            self.mu,
        )?;
        self.current = sample.state;
        Ok(sample)
    }
}

/// Apply an impulsive burn to a state.
///
/// Adds (prograde) or subtracts (retrograde) the magnitude along the
/// unit velocity vector. Position and epoch are unchanged.
///
/// # Errors
///
/// Returns [`TrajectoryError::NonFinite`] for bad inputs and
/// [`TrajectoryError::ZeroVelocity`] when speed is zero.
pub fn apply_burn(state: &StateVector, burn: &Burn) -> Result<StateVector, TrajectoryError> {
    checked_vector_m(state.position_m)?;
    checked_vector_m(state.velocity_mps)?;
    if !state.epoch.value().is_finite() {
        return Err(TrajectoryError::NonFinite {
            value_f64: state.epoch.value(),
        });
    }
    let speed_squared_m2_s2 = state.velocity_mps.length_squared();
    if !speed_squared_m2_s2.is_finite() {
        return Err(TrajectoryError::NonFinite {
            value_f64: speed_squared_m2_s2,
        });
    }
    let speed_mps = libm::sqrt(speed_squared_m2_s2);
    if !speed_mps.is_finite() {
        return Err(TrajectoryError::NonFinite {
            value_f64: speed_mps,
        });
    }
    if speed_mps <= 0.0 {
        return Err(TrajectoryError::ZeroVelocity);
    }
    let magnitude_mps = burn.delta_v.value();
    let signed_mps = match burn.direction {
        BurnDirection::Prograde => magnitude_mps,
        BurnDirection::Retrograde => -magnitude_mps,
    };
    let kick_mps = state.velocity_mps * (signed_mps / speed_mps);
    StateVector::new(state.position_m, state.velocity_mps + kick_mps, state.epoch)
}

/// Compute drag acceleration in meters per second squared.
///
/// Uses `a = -rho * |vrel| * vrel / (2 * B)` with corotating relative
/// wind `vrel` and density from [`AtmosphereParams::sample_at_altitude`].
/// Above [`RAILS_ALTITUDE_M`] drag is exactly zero (rails coast).
/// No lift is modelled at MVP.
///
/// # Errors
///
/// Returns [`TrajectoryError`] for bad states or failed sampling.
pub fn drag_accel(
    state: &StateVector,
    body: &BodyParams,
    atmosphere: &AtmosphereParams,
    vehicle: &VehicleParams,
) -> Result<DVec3, TrajectoryError> {
    let (relative_mps, altitude_m, _) = relative_wind_mps(state, body)?;
    if altitude_m > RAILS_ALTITUDE_M {
        return Ok(DVec3::ZERO);
    }
    let density_kg_per_m3 = sample_density_kg_per_m3(altitude_m, body, atmosphere)?;
    Ok(drag_vector_mps2(
        relative_mps,
        density_kg_per_m3,
        vehicle.ballistic_coefficient.value(),
    ))
}

/// Compute Sutton-Graves stagnation heating.
///
/// Uses `q = k * sqrt(rho / r_n) * v^3` with corotating speed `v` and
/// [`SUTTON_GRAVES_K_SI`]. Above [`RAILS_ALTITUDE_M`] heating is
/// exactly zero (rails coast).
///
/// # Errors
///
/// Returns [`TrajectoryError`] for bad states or failed sampling.
pub fn heating(
    state: &StateVector,
    body: &BodyParams,
    atmosphere: &AtmosphereParams,
    vehicle: &VehicleParams,
) -> Result<HeatFlux, TrajectoryError> {
    let (relative_mps, altitude_m, _) = relative_wind_mps(state, body)?;
    if altitude_m > RAILS_ALTITUDE_M {
        return Ok(HeatFlux::new(0.0));
    }
    let density_kg_per_m3 = sample_density_kg_per_m3(altitude_m, body, atmosphere)?;
    Ok(heating_from_density(
        relative_mps,
        density_kg_per_m3,
        vehicle.nose_radius_m.value(),
    ))
}

/// Compute g-load as a multiple of standard gravity.
///
/// Divides the drag acceleration magnitude by
/// [`STANDARD_GRAVITY_MPS2`]. Non-finite inputs yield non-finite
/// output; callers pass finite sensed vectors.
#[must_use]
pub fn g_load_g(drag_accel_mps2: DVec3) -> f64 {
    libm::sqrt(drag_accel_mps2.length_squared()) / STANDARD_GRAVITY_MPS2
}

/// Advance a point-ship state by one step.
///
/// Above [`RAILS_ALTITUDE_M`] the step delegates to orbit rails
/// ([`advance`]); at or below it integrates semi-implicit Euler with
/// forces at step start. A step straddling the cutoff splits into two
/// substeps whose durations sum to the request, with the final epoch
/// set to start plus duration exactly. End states below the surface
/// keep their penetrating position for the surface handoff; aero
/// sampling there clamps to surface conditions.
///
/// Atmosphere legs must keep `step` at or below [`MAX_STEP_S`] (one
/// [`crate::sim::SIM_TICK_S`] tick); rails legs may use warp-scaled
/// steps because warp auto-drops to `X1` on entry, so no integrating
/// substep ever exceeds one tick.
///
/// # Errors
///
/// Returns [`TrajectoryError`] for bad steps or states, unbound rails
/// legs, or failed environment sampling.
pub fn step_point_ship(
    state: &StateVector,
    step: Seconds,
    body: &BodyParams,
    atmosphere: &AtmosphereParams,
    vehicle: &VehicleParams,
    mu: Mu,
) -> Result<StepSample, TrajectoryError> {
    checked_vector_m(state.position_m)?;
    checked_vector_m(state.velocity_mps)?;
    if !state.epoch.value().is_finite() {
        return Err(TrajectoryError::NonFinite {
            value_f64: state.epoch.value(),
        });
    }
    let duration_s = step.value();
    if !duration_s.is_finite() || duration_s <= 0.0 {
        return Err(TrajectoryError::InvalidStep {
            step_s_f64: duration_s,
        });
    }
    let start_altitude_m = altitude_m(state, body)?;
    let target_epoch = Seconds::new(state.epoch.value() + duration_s);
    let end_state = if start_altitude_m > RAILS_ALTITUDE_M {
        let trial_state = rails_advance(state, step, mu)?;
        let trial_altitude_m = altitude_m(&trial_state, body)?;
        if trial_altitude_m <= RAILS_ALTITUDE_M {
            split_descent(
                state,
                &trial_state,
                step,
                target_epoch,
                body,
                atmosphere,
                vehicle,
                mu,
            )?
        } else {
            trial_state
        }
    } else {
        let trial_state = integrate(state, step, body, atmosphere, vehicle)?;
        let trial_altitude_m = altitude_m(&trial_state, body)?;
        if trial_altitude_m > RAILS_ALTITUDE_M {
            split_ascent(
                state,
                &trial_state,
                step,
                target_epoch,
                body,
                atmosphere,
                vehicle,
                mu,
            )?
        } else {
            trial_state
        }
    };
    let end_altitude_m = altitude_m(&end_state, body)?;
    let (drag_mps2, heat_flux, load_g) = if end_altitude_m > RAILS_ALTITUDE_M {
        (DVec3::ZERO, HeatFlux::new(0.0), 0.0)
    } else {
        let sample_altitude_m = if end_altitude_m < 0.0 {
            0.0
        } else {
            end_altitude_m
        };
        end_aero_mps2(&end_state, sample_altitude_m, body, atmosphere, vehicle)?
    };
    Ok(StepSample {
        state: end_state,
        drag_accel_mps2: drag_mps2,
        heat_flux,
        g_load_g: load_g,
    })
}

/// Convert an inertial state to classical elements.
///
/// Uses specific energy `eps = v^2/2 - mu/r` and the eccentricity
/// vector with `atan2` angle recovery. Circular orbits (below
/// [`CIRCULAR_ECCENTRICITY_THRESHOLD`]) pin periapsis to zero;
/// equatorial orbits (below
/// [`EQUATORIAL_INCLINATION_THRESHOLD_RAD`]) pin the node to zero;
/// `acos` inputs stay clamped to `[-1, 1]`.
///
/// # Errors
///
/// Returns [`TrajectoryError::AtCenter`] at the origin,
/// [`TrajectoryError::Unbound`] for `eps >= 0` or `e >= 1`, and
/// [`TrajectoryError::DegenerateAngularMomentum`] for zero angular
/// momentum.
pub fn elements_from_state(
    state: &StateVector,
    mu: Mu,
) -> Result<ClassicalElements, TrajectoryError> {
    checked_vector_m(state.position_m)?;
    checked_vector_m(state.velocity_mps)?;
    if !state.epoch.value().is_finite() {
        return Err(TrajectoryError::NonFinite {
            value_f64: state.epoch.value(),
        });
    }
    let mu_f64 = mu.value();
    let radius_m = libm::sqrt(state.position_m.length_squared());
    if !radius_m.is_finite() {
        return Err(TrajectoryError::NonFinite {
            value_f64: radius_m,
        });
    }
    if radius_m <= 0.0 {
        return Err(TrajectoryError::AtCenter);
    }
    let speed_squared_m2_s2 = state.velocity_mps.length_squared();
    if !speed_squared_m2_s2.is_finite() {
        return Err(TrajectoryError::NonFinite {
            value_f64: speed_squared_m2_s2,
        });
    }
    let energy_j_per_kg = speed_squared_m2_s2 / 2.0 - mu_f64 / radius_m;
    if !energy_j_per_kg.is_finite() {
        return Err(TrajectoryError::NonFinite {
            value_f64: energy_j_per_kg,
        });
    }
    if energy_j_per_kg >= 0.0 {
        return Err(TrajectoryError::Unbound {
            energy_j_per_kg_f64: energy_j_per_kg,
        });
    }
    let semi_major_m = -mu_f64 / (2.0 * energy_j_per_kg);
    let momentum_m2_s = state.position_m.cross(state.velocity_mps);
    let momentum_mag = libm::sqrt(momentum_m2_s.length_squared());
    if !momentum_mag.is_finite() {
        return Err(TrajectoryError::NonFinite {
            value_f64: momentum_mag,
        });
    }
    if momentum_mag <= 0.0 {
        return Err(TrajectoryError::DegenerateAngularMomentum);
    }
    let dot_rv = state.position_m.dot(state.velocity_mps);
    if !dot_rv.is_finite() {
        return Err(TrajectoryError::NonFinite { value_f64: dot_rv });
    }
    let ecc_vector = (state.position_m * (speed_squared_m2_s2 - mu_f64 / radius_m)
        - state.velocity_mps * dot_rv)
        / mu_f64;
    let eccentricity_f64 = libm::sqrt(ecc_vector.length_squared());
    if !eccentricity_f64.is_finite() {
        return Err(TrajectoryError::NonFinite {
            value_f64: eccentricity_f64,
        });
    }
    if eccentricity_f64 >= 1.0 {
        return Err(TrajectoryError::Unbound {
            energy_j_per_kg_f64: energy_j_per_kg,
        });
    }
    let angles_rad = plane_angles_rad(
        state.position_m,
        radius_m,
        &ecc_vector,
        eccentricity_f64,
        &momentum_m2_s,
        momentum_mag,
    )?;
    Ok(ClassicalElements::new(
        Meters::new(semi_major_m),
        eccentricity_f64,
        angles_rad.inclination_rad,
        angles_rad.raan_rad,
        angles_rad.arg_periapsis_rad,
        angles_rad.mean_anomaly_rad,
        state.epoch,
    )?)
}

/// Plane angles recovered from eccentricity and momentum vectors.
///
/// Groups inclination, node, periapsis, and mean anomaly so
/// [`elements_from_state`] stays small. All angles use `atan2`
/// recovery; guards mirror the parent function.
#[expect(
    clippy::struct_field_names,
    reason = "radian suffixes are required by the unit naming rule"
)]
#[derive(Debug, Clone, Copy, PartialEq)]
struct PlaneAngles {
    /// Inclination in radians.
    inclination_rad: f64,
    /// Right ascension of ascending node in radians.
    raan_rad: f64,
    /// Argument of periapsis in radians.
    arg_periapsis_rad: f64,
    /// Mean anomaly at epoch in radians.
    mean_anomaly_rad: f64,
}

/// Recover plane angles from validated orbit vectors.
///
/// Takes position, radius, eccentricity vector and value, plus angular
/// momentum vector and magnitude. Circular orbits pin periapsis to
/// zero; equatorial orbits pin the node to zero.
///
/// # Errors
///
/// Returns [`TrajectoryError::DegenerateAngularMomentum`] for a zero
/// node magnitude.
fn plane_angles_rad(
    position_m: DVec3,
    radius_m: f64,
    ecc_vector: &DVec3,
    eccentricity_f64: f64,
    momentum_m2_s: &DVec3,
    momentum_mag: f64,
) -> Result<PlaneAngles, TrajectoryError> {
    let normal_m2_s = *momentum_m2_s / momentum_mag;
    let inclination_rad = libm::acos(clamp_unit_f64(normal_m2_s.z));
    let equatorial = inclination_rad < EQUATORIAL_INCLINATION_THRESHOLD_RAD;
    let node_x = if equatorial { 1.0 } else { -momentum_m2_s.y };
    let node_y = if equatorial { 0.0 } else { momentum_m2_s.x };
    let node_vector = DVec3::new(node_x, node_y, 0.0);
    let node_mag = libm::sqrt(node_vector.length_squared());
    if !node_mag.is_finite() || node_mag <= 0.0 {
        return Err(TrajectoryError::DegenerateAngularMomentum);
    }
    let node_unit = node_vector / node_mag;
    let raan_rad = if equatorial {
        0.0
    } else {
        wrap_two_pi_rad(libm::atan2(node_unit.y, node_unit.x))
    };
    let in_plane_unit = normal_m2_s.cross(node_unit);
    let arg_periapsis_rad = if eccentricity_f64 < CIRCULAR_ECCENTRICITY_THRESHOLD {
        0.0
    } else {
        wrap_two_pi_rad(libm::atan2(
            ecc_vector.dot(in_plane_unit),
            ecc_vector.dot(node_unit),
        ))
    };
    let radius_unit = position_m / radius_m;
    let latitude_rad = libm::atan2(radius_unit.dot(in_plane_unit), radius_unit.dot(node_unit));
    let true_anomaly_rad = latitude_rad - arg_periapsis_rad;
    let cos_true = libm::cos(true_anomaly_rad);
    let sin_true = libm::sin(true_anomaly_rad);
    let one_minus_e2 = (1.0 - eccentricity_f64) * (1.0 + eccentricity_f64);
    let eccentric_rad = libm::atan2(
        libm::sqrt(one_minus_e2) * sin_true,
        eccentricity_f64 + cos_true,
    );
    let mean_anomaly_rad =
        wrap_two_pi_rad(eccentric_rad - eccentricity_f64 * libm::sin(eccentric_rad));
    Ok(PlaneAngles {
        inclination_rad,
        raan_rad,
        arg_periapsis_rad,
        mean_anomaly_rad,
    })
}

/// Convert classical elements to an inertial state.
///
/// Propagates to the element epoch with [`advance`], so the result is
/// bit-identical to coasting to that epoch.
///
/// # Errors
///
/// Returns [`TrajectoryError::Orbit`] when propagation fails.
pub fn state_from_elements(
    elements: &ClassicalElements,
    mu: Mu,
) -> Result<StateVector, TrajectoryError> {
    let propagated = advance(mu, elements, Seconds::new(0.0))?;
    checked_vector_m(propagated.position_m)?;
    checked_vector_m(propagated.velocity_mps)?;
    Ok(StateVector {
        position_m: propagated.position_m,
        velocity_mps: propagated.velocity_mps,
        epoch: elements.epoch,
    })
}

/// Reject a vector with a non-finite component.
fn checked_vector_m(vector_m: DVec3) -> Result<DVec3, TrajectoryError> {
    if vector_m.x.is_finite() && vector_m.y.is_finite() && vector_m.z.is_finite() {
        Ok(vector_m)
    } else {
        Err(TrajectoryError::NonFinite {
            value_f64: vector_m.x + vector_m.y + vector_m.z,
        })
    }
}

/// Clamp a cosine-like value to `[-1, 1]` for `acos`.
///
/// NaN passes through and fails downstream range checks with a typed
/// error; bounds are ordered so this never panics.
fn clamp_unit_f64(value_f64: f64) -> f64 {
    value_f64.clamp(-1.0, 1.0)
}

/// Wrap an angle in radians to `[0, TAU)` with `libm`.
fn wrap_two_pi_rad(angle_rad: f64) -> f64 {
    let wrapped_rad = libm::fmod(angle_rad, TAU_RAD);
    if wrapped_rad < 0.0 {
        wrapped_rad + TAU_RAD
    } else {
        wrapped_rad
    }
}

/// Altitude above the surface in meters from a validated state.
fn altitude_m(state: &StateVector, body: &BodyParams) -> Result<f64, TrajectoryError> {
    let radius_m = libm::sqrt(state.position_m.length_squared());
    if !radius_m.is_finite() {
        return Err(TrajectoryError::NonFinite {
            value_f64: radius_m,
        });
    }
    if radius_m <= 0.0 {
        return Err(TrajectoryError::AtCenter);
    }
    Ok(radius_m - body.radius_m().value())
}

/// Corotating relative wind, altitude, and spin for a validated state.
///
/// Returns `(vrel_mps, altitude_m, spin_rad_s)` with spin about the
/// body z-axis. Shared by drag and heating so both sense one wind.
fn relative_wind_mps(
    state: &StateVector,
    body: &BodyParams,
) -> Result<(DVec3, f64, f64), TrajectoryError> {
    checked_vector_m(state.position_m)?;
    checked_vector_m(state.velocity_mps)?;
    let altitude_value_m = altitude_m(state, body)?;
    let spin_rad_s = TAU_RAD / body.rotation_period_s().value();
    if !spin_rad_s.is_finite() {
        return Err(TrajectoryError::NonFinite {
            value_f64: spin_rad_s,
        });
    }
    let corotation_mps = DVec3::new(
        -spin_rad_s * state.position_m.y,
        spin_rad_s * state.position_m.x,
        0.0,
    );
    let relative_mps = state.velocity_mps - corotation_mps;
    if !relative_mps.x.is_finite() || !relative_mps.y.is_finite() || !relative_mps.z.is_finite() {
        return Err(TrajectoryError::NonFinite {
            value_f64: relative_mps.x + relative_mps.y + relative_mps.z,
        });
    }
    Ok((relative_mps, altitude_value_m, spin_rad_s))
}

/// Sample air density in kilograms per cubic meter at altitude.
///
/// The caller guarantees `altitude_m >= 0`; below-surface inputs
/// return the atmosphere [`AtmosphereError`].
fn sample_density_kg_per_m3(
    altitude_m: f64,
    body: &BodyParams,
    atmosphere: &AtmosphereParams,
) -> Result<f64, TrajectoryError> {
    let sample = atmosphere.sample_at_altitude(Meters::new(altitude_m), body)?;
    let density_kg_per_m3 = sample.density_kg_per_m3().value();
    if !density_kg_per_m3.is_finite() {
        return Err(TrajectoryError::NonFinite {
            value_f64: density_kg_per_m3,
        });
    }
    Ok(density_kg_per_m3)
}

/// Drag vector from relative wind, density, and ballistic coefficient.
///
/// Implements `-rho * |vrel| * vrel / (2 * B)`; pure arithmetic.
fn drag_vector_mps2(
    relative_mps: DVec3,
    density_kg_per_m3: f64,
    ballistic_kg_per_m2: f64,
) -> DVec3 {
    let speed_mps = libm::sqrt(relative_mps.length_squared());
    relative_mps * (-density_kg_per_m3 * speed_mps / (2.0 * ballistic_kg_per_m2))
}

/// Sutton-Graves heat flux from relative wind and density.
///
/// Implements `k * sqrt(rho / r_n) * v^3`; pure `libm` math.
fn heating_from_density(
    relative_mps: DVec3,
    density_kg_per_m3: f64,
    nose_radius_m: f64,
) -> HeatFlux {
    let speed_mps = libm::sqrt(relative_mps.length_squared());
    let flux_w_per_m2 = SUTTON_GRAVES_K_SI
        * libm::sqrt(density_kg_per_m3 / nose_radius_m)
        * speed_mps
        * speed_mps
        * speed_mps;
    HeatFlux::new(flux_w_per_m2)
}

/// Aero loads at an end state sampled at a clamped altitude.
///
/// Computes drag, heat flux, and g-load from one density sample.
/// Pure combination of [`drag_vector_mps2`], [`heating_from_density`],
/// and [`g_load_g`].
fn end_aero_mps2(
    end_state: &StateVector,
    sample_altitude_m: f64,
    body: &BodyParams,
    atmosphere: &AtmosphereParams,
    vehicle: &VehicleParams,
) -> Result<(DVec3, HeatFlux, f64), TrajectoryError> {
    let (relative_mps, _, _) = relative_wind_mps(end_state, body)?;
    let density_kg_per_m3 = sample_density_kg_per_m3(sample_altitude_m, body, atmosphere)?;
    let drag_mps2 = drag_vector_mps2(
        relative_mps,
        density_kg_per_m3,
        vehicle.ballistic_coefficient.value(),
    );
    let heat_flux = heating_from_density(
        relative_mps,
        density_kg_per_m3,
        vehicle.nose_radius_m.value(),
    );
    Ok((drag_mps2, heat_flux, g_load_g(drag_mps2)))
}

/// Advance on analytic rails by converting through elements.
///
/// Bit-identical to [`advance`] to `epoch + step`; see
/// [`state_from_elements`].
fn rails_advance(
    state: &StateVector,
    step: Seconds,
    mu: Mu,
) -> Result<StateVector, TrajectoryError> {
    let elements = elements_from_state(state, mu)?;
    let target_epoch = Seconds::new(state.epoch.value() + step.value());
    if !target_epoch.value().is_finite() {
        return Err(TrajectoryError::NonFinite {
            value_f64: target_epoch.value(),
        });
    }
    let propagated = advance(mu, &elements, step)?;
    checked_vector_m(propagated.position_m)?;
    checked_vector_m(propagated.velocity_mps)?;
    Ok(StateVector {
        position_m: propagated.position_m,
        velocity_mps: propagated.velocity_mps,
        epoch: target_epoch,
    })
}

/// Integrate one atmosphere step with semi-implicit Euler.
///
/// Forces come from step-start gravity and drag; velocity updates
/// first, then position uses the new velocity. Needs `altitude >= 0`;
/// below-surface starts return the body error.
fn integrate(
    state: &StateVector,
    step: Seconds,
    body: &BodyParams,
    atmosphere: &AtmosphereParams,
    vehicle: &VehicleParams,
) -> Result<StateVector, TrajectoryError> {
    let duration_s = step.value();
    let start_radius_m = libm::sqrt(state.position_m.length_squared());
    if !start_radius_m.is_finite() || start_radius_m <= 0.0 {
        return Err(TrajectoryError::AtCenter);
    }
    let start_altitude_m = start_radius_m - body.radius_m().value();
    let gravity_mps2 = body
        .gravity_at_altitude(Meters::new(start_altitude_m))?
        .value();
    let (relative_mps, _, _) = relative_wind_mps(state, body)?;
    let density_kg_per_m3 = sample_density_kg_per_m3(start_altitude_m, body, atmosphere)?;
    let drag_mps2 = drag_vector_mps2(
        relative_mps,
        density_kg_per_m3,
        vehicle.ballistic_coefficient.value(),
    );
    let radius_unit = state.position_m / start_radius_m;
    let accel_mps2 = radius_unit * (-gravity_mps2) + drag_mps2;
    if !accel_mps2.x.is_finite() || !accel_mps2.y.is_finite() || !accel_mps2.z.is_finite() {
        return Err(TrajectoryError::NonFinite {
            value_f64: accel_mps2.x + accel_mps2.y + accel_mps2.z,
        });
    }
    let end_velocity_mps = state.velocity_mps + accel_mps2 * duration_s;
    let end_position_m = state.position_m + end_velocity_mps * duration_s;
    let target_epoch = Seconds::new(state.epoch.value() + duration_s);
    if !target_epoch.value().is_finite() {
        return Err(TrajectoryError::NonFinite {
            value_f64: target_epoch.value(),
        });
    }
    checked_vector_m(end_velocity_mps)?;
    checked_vector_m(end_position_m)?;
    Ok(StateVector {
        position_m: end_position_m,
        velocity_mps: end_velocity_mps,
        epoch: target_epoch,
    })
}

/// Split a rails-to-atmosphere step at the cutoff.
///
/// Spends the above-cutoff fraction on rails and the rest integrating,
/// with the final epoch pinned to the exact target.
#[expect(
    clippy::too_many_arguments,
    reason = "split needs both states plus environment"
)]
fn split_descent(
    start_state: &StateVector,
    trial_state: &StateVector,
    step: Seconds,
    target_epoch: Seconds,
    body: &BodyParams,
    atmosphere: &AtmosphereParams,
    vehicle: &VehicleParams,
    mu: Mu,
) -> Result<StateVector, TrajectoryError> {
    let start_altitude_m = altitude_m(start_state, body)?;
    let trial_altitude_m = altitude_m(trial_state, body)?;
    let fraction_f64 =
        (start_altitude_m - RAILS_ALTITUDE_M) / (start_altitude_m - trial_altitude_m);
    if !fraction_f64.is_finite() || fraction_f64 <= 0.0 || fraction_f64 >= 1.0 {
        return Ok(*trial_state);
    }
    let duration_s = step.value();
    let first_leg = Seconds::new(duration_s * fraction_f64);
    let second_leg = Seconds::new(duration_s - first_leg.value());
    let mid_state = rails_advance(start_state, first_leg, mu)?;
    let mut end_state = integrate(&mid_state, second_leg, body, atmosphere, vehicle)?;
    end_state.epoch = target_epoch;
    Ok(end_state)
}

/// Split an atmosphere-to-rails step at the cutoff.
///
/// Spends the below-cutoff fraction integrating and the rest on rails,
/// with the final epoch pinned to the exact target.
#[expect(
    clippy::too_many_arguments,
    reason = "split needs both states plus environment"
)]
fn split_ascent(
    start_state: &StateVector,
    trial_state: &StateVector,
    step: Seconds,
    target_epoch: Seconds,
    body: &BodyParams,
    atmosphere: &AtmosphereParams,
    vehicle: &VehicleParams,
    mu: Mu,
) -> Result<StateVector, TrajectoryError> {
    let start_altitude_m = altitude_m(start_state, body)?;
    let trial_altitude_m = altitude_m(trial_state, body)?;
    let fraction_f64 =
        (RAILS_ALTITUDE_M - start_altitude_m) / (trial_altitude_m - start_altitude_m);
    if !fraction_f64.is_finite() || fraction_f64 <= 0.0 || fraction_f64 >= 1.0 {
        return Ok(*trial_state);
    }
    let duration_s = step.value();
    let first_leg = Seconds::new(duration_s * fraction_f64);
    let second_leg = Seconds::new(duration_s - first_leg.value());
    let mid_state = integrate(start_state, first_leg, body, atmosphere, vehicle)?;
    let mut end_state = rails_advance(&mid_state, second_leg, mu)?;
    end_state.epoch = target_epoch;
    Ok(end_state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::Meters;

    /// Mars gravitational parameter in cubic meters per second squared.
    const MARS_MU_M3_S2: f64 = 4.282_837e13;
    /// Mars mean radius in meters.
    const MARS_RADIUS_M: f64 = 3_389_500.0;
    /// Low-orbit altitude in meters for burn fixtures.
    const LOW_ORBIT_ALTITUDE_M: f64 = 250_000.0;
    /// Burn magnitude in meters per second for apply checks.
    const TEST_BURN_MPS: f64 = 100.0;
    /// Burn round-trip tolerance in meters per second.
    const BURN_TOL_MPS: f64 = 1e-6;
    /// Vis-viva relative tolerance, dimensionless.
    const VIS_VIVA_TOL_F64: f64 = 1e-6;
    /// Elements round-trip relative tolerance, dimensionless.
    const ROUND_TRIP_TOL_F64: f64 = 1e-9;
    /// Fixed step in seconds matching `SIM_TICK_S`.
    const STEP_S: f64 = 0.05;
    /// Epoch exactness tolerance in seconds.
    const EPOCH_TOL_S: f64 = 0.0;
    /// Drag direction tolerance on the normalized cross product.
    const DIRECTION_TOL_F64: f64 = 1e-9;
    /// Heating hand-check relative tolerance, dimensionless.
    const HEATING_TOL_F64: f64 = 1e-9;
    /// Descent start altitude in meters for the headless profile.
    const DESCENT_START_ALTITUDE_M: f64 = 300_000.0;
    /// Descent deorbit retrograde magnitude in meters per second.
    const DESCENT_RETRO_MPS: f64 = 200.0;
    /// Descent step cap in iterations, dimensionless.
    const DESCENT_MAX_STEPS_U32: u32 = 200_000;
    /// Ascent launch radial speed in meters per second.
    const ASCENT_LAUNCH_RADIAL_MPS: f64 = 1_000.0;
    /// Ascent powered band top in meters.
    const ASCENT_BOOST_TOP_M: f64 = 20_000.0;
    /// Ascent per-tick boost in meters per second.
    const ASCENT_BOOST_MPS: f64 = 5.0;
    /// Ascent step cap in iterations, dimensionless.
    const ASCENT_MAX_STEPS_U32: u32 = 200_000;

    fn test_mu() -> Mu {
        let body = test_body();
        let Ok(mu) = Mu::new(body.gravitational_parameter_m3_s2()) else {
            panic!("body-derived mu must be valid");
        };
        mu
    }

    fn test_body() -> BodyParams {
        BodyParams::mars_like()
    }

    fn test_atmosphere() -> AtmosphereParams {
        let Ok(atmosphere) = AtmosphereParams::mars_like() else {
            panic!("Mars-like atmosphere must validate");
        };
        atmosphere
    }

    fn circular_state_at_altitude(altitude_m: f64) -> StateVector {
        let mu = test_mu();
        let radius_m = MARS_RADIUS_M + altitude_m;
        let speed_mps = libm::sqrt(mu.value() / radius_m);
        let Ok(state) = StateVector::new(
            DVec3::new(radius_m, 0.0, 0.0),
            DVec3::new(0.0, speed_mps, 0.0),
            Seconds::new(0.0),
        ) else {
            panic!("circular state must validate");
        };
        state
    }

    fn eccentric_state() -> StateVector {
        let mu = test_mu();
        let Ok(elements) = ClassicalElements::new(
            Meters::new(MARS_RADIUS_M + LOW_ORBIT_ALTITUDE_M),
            0.3,
            0.4,
            0.7,
            0.5,
            1.0,
            Seconds::new(0.0),
        ) else {
            panic!("eccentric elements must validate");
        };
        let Ok(state) = state_from_elements(&elements, mu) else {
            panic!("eccentric state must convert");
        };
        state
    }

    #[test]
    fn preset_matches_step1_values() {
        let vehicle = VehicleParams::preset();
        assert!(
            libm::fabs(
                vehicle.ballistic_coefficient().value() - PRESET_BALLISTIC_COEFFICIENT_KG_PER_M2
            ) < 1e-12
        );
        assert!(libm::fabs(vehicle.nose_radius_m().value() - PRESET_NOSE_RADIUS_M) < 1e-12);
        assert!(libm::fabs(PRESET_BALLISTIC_COEFFICIENT_KG_PER_M2 - 120.0) < 1e-12);
        assert!(libm::fabs(PRESET_NOSE_RADIUS_M - 1.0) < 1e-12);
        assert!(libm::fabs(SUTTON_GRAVES_K_SI - 1.9027e-4) / 1.9027e-4 < 1e-12);
        assert!(libm::fabs(STANDARD_GRAVITY_MPS2 - 9.80665) < 1e-12);
        assert!(libm::fabs(RAILS_ALTITUDE_M - 120_000.0) < 1e-12);
    }

    #[test]
    fn constructor_rejects_bad_inputs() {
        assert!(BallisticCoefficient::new(0.0).is_err());
        assert!(BallisticCoefficient::new(f64::NAN).is_err());
        let Ok(ballistic) = BallisticCoefficient::new(120.0) else {
            panic!("valid ballistic coefficient must pass");
        };
        assert!(
            VehicleParams::new(ballistic, Meters::new(0.0)).is_err(),
            "zero nose radius must fail"
        );
        assert!(
            Burn::new(BurnDirection::Prograde, MetersPerSecond::new(-1.0)).is_err(),
            "negative delta-v must fail"
        );
        assert!(
            Burn::new(BurnDirection::Prograde, MetersPerSecond::new(f64::NAN)).is_err(),
            "NaN delta-v must fail"
        );
        assert!(
            StateVector::new(
                DVec3::new(f64::NAN, 0.0, 0.0),
                DVec3::ZERO,
                Seconds::new(0.0)
            )
            .is_err(),
            "NaN position must fail"
        );
    }

    // AC2: prograde and retrograde burns shift speed by delta-v.
    #[test]
    fn burns_shift_speed_by_delta_v() {
        let state = circular_state_at_altitude(LOW_ORBIT_ALTITUDE_M);
        let start_speed_mps = libm::sqrt(state.velocity_mps.length_squared());
        let Ok(prograde) = Burn::new(BurnDirection::Prograde, MetersPerSecond::new(TEST_BURN_MPS))
        else {
            panic!("prograde burn must validate");
        };
        let Ok(after_prograde) = apply_burn(&state, &prograde) else {
            panic!("prograde burn must apply");
        };
        let end_speed_mps = libm::sqrt(after_prograde.velocity_mps.length_squared());
        assert!(
            libm::fabs((end_speed_mps - start_speed_mps) - TEST_BURN_MPS) < BURN_TOL_MPS,
            "prograde must add delta-v"
        );
        assert_eq!(after_prograde.position_m, state.position_m);
        assert_eq!(after_prograde.epoch, state.epoch);
        let Ok(retrograde) = Burn::new(
            BurnDirection::Retrograde,
            MetersPerSecond::new(TEST_BURN_MPS),
        ) else {
            panic!("retrograde burn must validate");
        };
        let Ok(after_retrograde) = apply_burn(&state, &retrograde) else {
            panic!("retrograde burn must apply");
        };
        let slow_speed_mps = libm::sqrt(after_retrograde.velocity_mps.length_squared());
        assert!(
            libm::fabs((start_speed_mps - slow_speed_mps) - TEST_BURN_MPS) < BURN_TOL_MPS,
            "retrograde must remove delta-v"
        );
    }

    #[test]
    fn burn_without_direction_fails() {
        let Ok(at_rest) = StateVector::new(
            DVec3::new(MARS_RADIUS_M, 0.0, 0.0),
            DVec3::ZERO,
            Seconds::new(0.0),
        ) else {
            panic!("rest state must validate");
        };
        let Ok(burn) = Burn::new(BurnDirection::Prograde, MetersPerSecond::new(10.0)) else {
            panic!("burn must validate");
        };
        assert!(matches!(
            apply_burn(&at_rest, &burn),
            Err(TrajectoryError::ZeroVelocity)
        ));
    }

    // AC2: state -> elements -> state round-trips within 1e-9 relative.
    #[test]
    fn elements_round_trip_within_tolerance() {
        let mu = test_mu();
        for state in [
            circular_state_at_altitude(LOW_ORBIT_ALTITUDE_M),
            eccentric_state(),
        ] {
            let Ok(elements) = elements_from_state(&state, mu) else {
                panic!("elements conversion must succeed");
            };
            let Ok(there_and_back) = state_from_elements(&elements, mu) else {
                panic!("state conversion must succeed");
            };
            let position_scale_m = libm::sqrt(state.position_m.length_squared()).max(1.0);
            let velocity_scale_mps = libm::sqrt(state.velocity_mps.length_squared()).max(1.0);
            let position_drift_m =
                libm::sqrt((there_and_back.position_m - state.position_m).length_squared());
            let velocity_drift_mps =
                libm::sqrt((there_and_back.velocity_mps - state.velocity_mps).length_squared());
            assert!(
                position_drift_m / position_scale_m < ROUND_TRIP_TOL_F64,
                "position round-trip must hold"
            );
            assert!(
                velocity_drift_mps / velocity_scale_mps < ROUND_TRIP_TOL_F64,
                "velocity round-trip must hold"
            );
            assert_eq!(there_and_back.epoch, state.epoch);
        }
    }

    // AC2: converted elements satisfy vis-viva within 1e-6 relative.
    #[test]
    fn converted_elements_satisfy_vis_viva() {
        let mu = test_mu();
        let state = eccentric_state();
        let Ok(elements) = elements_from_state(&state, mu) else {
            panic!("elements conversion must succeed");
        };
        let radius_m = libm::sqrt(state.position_m.length_squared());
        let speed_squared_m2_s2 = state.velocity_mps.length_squared();
        let predicted_m2_s2 =
            mu.value() * (2.0 / radius_m - 1.0 / elements.semi_major_axis.value());
        let scale_m2_s2 = libm::fabs(speed_squared_m2_s2).max(1.0);
        assert!(
            libm::fabs(speed_squared_m2_s2 - predicted_m2_s2) / scale_m2_s2 < VIS_VIVA_TOL_F64,
            "vis-viva must hold at the converted state"
        );
    }

    // Gap 5: Mu derives from BodyParams so orbit and atmosphere share g(z).
    #[test]
    fn mu_matches_body_gravity() {
        let body = test_body();
        let mu = test_mu();
        let body_mu_m3_s2 = body.gravitational_parameter_m3_s2();
        let scale_m3_s2 = libm::fabs(body_mu_m3_s2).max(1.0);
        assert!(
            libm::fabs(mu.value() - body_mu_m3_s2) / scale_m3_s2 < 1e-12,
            "trajectory mu must equal body gravity parameter"
        );
        let reference_rel =
            libm::fabs(mu.value() - MARS_MU_M3_S2) / libm::fabs(MARS_MU_M3_S2).max(1.0);
        assert!(
            reference_rel < 1e-4,
            "body-derived mu must match MU_MARS reference within 1e-4"
        );
        for altitude_m in [0.0, 50_000.0, 100_000.0, RAILS_ALTITUDE_M] {
            let Ok(sample) = body.gravity_at_altitude(Meters::new(altitude_m)) else {
                panic!("gravity at {altitude_m} must sample");
            };
            let radius_m = MARS_RADIUS_M + altitude_m;
            let expected_mps2 = mu.value() / (radius_m * radius_m);
            let scale_mps2 = libm::fabs(expected_mps2).max(1.0);
            assert!(
                libm::fabs(sample.value() - expected_mps2) / scale_mps2 < 1e-12,
                "body g(z) must equal mu over r-squared"
            );
        }
    }

    // AC2: post-burn apoapsis from vis-viva, prograde and retrograde.
    #[test]
    fn post_burn_apoapsis_matches_vis_viva() {
        let mu = test_mu();
        let start = circular_state_at_altitude(LOW_ORBIT_ALTITUDE_M);
        let start_radius_m = libm::sqrt(start.position_m.length_squared());
        for direction in [BurnDirection::Prograde, BurnDirection::Retrograde] {
            let Ok(burn) = Burn::new(direction, MetersPerSecond::new(TEST_BURN_MPS)) else {
                panic!("burn must validate");
            };
            let Ok(after) = apply_burn(&start, &burn) else {
                panic!("burn must apply");
            };
            let after_speed_squared_m2_s2 = after.velocity_mps.length_squared();
            let vis_semimajor_m =
                1.0 / (2.0 / start_radius_m - after_speed_squared_m2_s2 / mu.value());
            let Ok(elements) = elements_from_state(&after, mu) else {
                panic!("post-burn elements must convert");
            };
            let elem_semimajor_m = elements.semi_major_axis.value();
            let scale_a_m = libm::fabs(elem_semimajor_m).max(1.0);
            assert!(
                libm::fabs(vis_semimajor_m - elem_semimajor_m) / scale_a_m < VIS_VIVA_TOL_F64,
                "post-burn semi-major axis must match vis-viva"
            );
            let (vis_apse_m, elem_apse_m) = match direction {
                BurnDirection::Prograde => {
                    let vis_apoapsis_m = 2.0 * vis_semimajor_m - start_radius_m;
                    let elem_apoapsis_m = elem_semimajor_m * (1.0 + elements.eccentricity_f64);
                    (vis_apoapsis_m, elem_apoapsis_m)
                }
                BurnDirection::Retrograde => {
                    let vis_periapsis_m = 2.0 * vis_semimajor_m - start_radius_m;
                    let elem_periapsis_m = elem_semimajor_m * (1.0 - elements.eccentricity_f64);
                    (vis_periapsis_m, elem_periapsis_m)
                }
            };
            let scale_apse_m = libm::fabs(elem_apse_m).max(1.0);
            assert!(
                libm::fabs(vis_apse_m - elem_apse_m) / scale_apse_m < VIS_VIVA_TOL_F64,
                "post-burn apse must match vis-viva"
            );
        }
    }

    // AC2: a/e round-trip 1e-9 relative, angles 1e-9 rad.
    #[test]
    fn elements_a_e_angles_round_trip() {
        let mu = test_mu();
        let state = eccentric_state();
        let Ok(first) = elements_from_state(&state, mu) else {
            panic!("first conversion must succeed");
        };
        let Ok(mid) = state_from_elements(&first, mu) else {
            panic!("state conversion must succeed");
        };
        let Ok(second) = elements_from_state(&mid, mu) else {
            panic!("second conversion must succeed");
        };
        let scale_a_m = libm::fabs(first.semi_major_axis.value()).max(1.0);
        assert!(
            libm::fabs(second.semi_major_axis.value() - first.semi_major_axis.value()) / scale_a_m
                < ROUND_TRIP_TOL_F64,
            "semi-major axis must round-trip"
        );
        let scale_e = libm::fabs(first.eccentricity_f64).max(1.0);
        assert!(
            libm::fabs(second.eccentricity_f64 - first.eccentricity_f64) / scale_e
                < ROUND_TRIP_TOL_F64,
            "eccentricity must round-trip"
        );
        for (first_rad, second_rad, name) in [
            (first.inclination_rad, second.inclination_rad, "inclination"),
            (first.raan_rad, second.raan_rad, "node"),
            (
                first.arg_periapsis_rad,
                second.arg_periapsis_rad,
                "periapsis",
            ),
            (
                first.mean_anomaly_at_epoch_rad,
                second.mean_anomaly_at_epoch_rad,
                "anomaly",
            ),
        ] {
            let delta_rad = libm::fabs(first_rad - second_rad);
            let wrapped_rad = if delta_rad > core::f64::consts::PI {
                TAU_RAD - delta_rad
            } else {
                delta_rad
            };
            assert!(wrapped_rad < 1e-9, "{name} must round-trip within 1e-9 rad");
        }
    }

    #[test]
    fn circular_equatorial_conversion_has_no_singularity() {
        let mu = test_mu();
        let radius_m = MARS_RADIUS_M + LOW_ORBIT_ALTITUDE_M;
        let speed_mps = libm::sqrt(mu.value() / radius_m);
        let Ok(state) = StateVector::new(
            DVec3::new(radius_m, 0.0, 0.0),
            DVec3::new(0.0, speed_mps, 0.0),
            Seconds::new(10.0),
        ) else {
            panic!("equatorial state must validate");
        };
        let Ok(elements) = elements_from_state(&state, mu) else {
            panic!("equatorial conversion must succeed");
        };
        assert!(elements.eccentricity_f64 < CIRCULAR_ECCENTRICITY_THRESHOLD);
        assert!(elements.inclination_rad < EQUATORIAL_INCLINATION_THRESHOLD_RAD);
        assert!(libm::fabs(elements.raan_rad) < 1e-12);
        assert!(libm::fabs(elements.arg_periapsis_rad) < 1e-12);
        let Ok(there_and_back) = state_from_elements(&elements, mu) else {
            panic!("state conversion must succeed");
        };
        let drift_m = libm::sqrt((there_and_back.position_m - state.position_m).length_squared());
        assert!(drift_m / radius_m < ROUND_TRIP_TOL_F64);
    }

    #[test]
    fn unbound_and_degenerate_states_are_rejected() {
        let mu = test_mu();
        let radius_m = MARS_RADIUS_M + LOW_ORBIT_ALTITUDE_M;
        let escape_mps = libm::sqrt(2.0 * mu.value() / radius_m);
        let Ok(escape) = StateVector::new(
            DVec3::new(radius_m, 0.0, 0.0),
            DVec3::new(0.0, escape_mps * 1.01, 0.0),
            Seconds::new(0.0),
        ) else {
            panic!("escape state must validate");
        };
        assert!(matches!(
            elements_from_state(&escape, mu),
            Err(TrajectoryError::Unbound { .. })
        ));
        let Ok(rectilinear) = StateVector::new(
            DVec3::new(radius_m, 0.0, 0.0),
            DVec3::new(100.0, 0.0, 0.0),
            Seconds::new(0.0),
        ) else {
            panic!("rectilinear state must validate");
        };
        assert!(matches!(
            elements_from_state(&rectilinear, mu),
            Err(TrajectoryError::DegenerateAngularMomentum)
        ));
        let Ok(at_center) =
            StateVector::new(DVec3::ZERO, DVec3::new(1.0, 0.0, 0.0), Seconds::new(0.0))
        else {
            panic!("center state must validate");
        };
        assert!(matches!(
            elements_from_state(&at_center, mu),
            Err(TrajectoryError::AtCenter)
        ));
    }

    #[test]
    fn drag_opposes_relative_wind_below_cutoff() {
        let body = test_body();
        let atmosphere = test_atmosphere();
        let vehicle = VehicleParams::preset();
        let state = circular_state_at_altitude(50_000.0);
        let Ok(drag_mps2) = drag_accel(&state, &body, &atmosphere, &vehicle) else {
            panic!("drag must sample");
        };
        let drag_mag_mps2 = libm::sqrt(drag_mps2.length_squared());
        assert!(drag_mag_mps2 > 0.0, "drag must act at 50 km");
        let (relative_mps, _, _) = relative_wind_for_test(&state, &body);
        let relative_speed_mps = libm::sqrt(relative_mps.length_squared());
        let alignment_f64 = drag_mps2.dot(relative_mps) / (drag_mag_mps2 * relative_speed_mps);
        assert!(
            libm::fabs(alignment_f64 + 1.0) < DIRECTION_TOL_F64,
            "drag must oppose the relative wind"
        );
        let density = density_for_test(50_000.0, &body, &atmosphere);
        let speed = libm::sqrt(relative_mps.length_squared());
        let expected_mps2 =
            density * speed * speed / (2.0 * vehicle.ballistic_coefficient().value());
        assert!(
            libm::fabs(drag_mag_mps2 - expected_mps2) / expected_mps2 < HEATING_TOL_F64,
            "drag magnitude must match the formula"
        );
    }

    #[test]
    fn aero_loads_vanish_above_cutoff() {
        let body = test_body();
        let atmosphere = test_atmosphere();
        let vehicle = VehicleParams::preset();
        let state = circular_state_at_altitude(150_000.0);
        let Ok(drag_mps2) = drag_accel(&state, &body, &atmosphere, &vehicle) else {
            panic!("high drag must sample");
        };
        assert_eq!(drag_mps2, DVec3::ZERO);
        let Ok(heat_flux) = heating(&state, &body, &atmosphere, &vehicle) else {
            panic!("high heating must sample");
        };
        assert!(heat_flux.value().abs() < 1e-12);
    }

    #[test]
    fn heating_matches_sutton_graves() {
        let body = test_body();
        let atmosphere = test_atmosphere();
        let vehicle = VehicleParams::preset();
        let state = circular_state_at_altitude(50_000.0);
        let Ok(heat_flux) = heating(&state, &body, &atmosphere, &vehicle) else {
            panic!("heating must sample");
        };
        let (relative_mps, _, _) = relative_wind_for_test(&state, &body);
        let density = density_for_test(50_000.0, &body, &atmosphere);
        let speed = libm::sqrt(relative_mps.length_squared());
        let expected_w_per_m2 = SUTTON_GRAVES_K_SI
            * libm::sqrt(density / vehicle.nose_radius_m.value())
            * speed
            * speed
            * speed;
        assert!(
            libm::fabs(heat_flux.value() - expected_w_per_m2) / expected_w_per_m2 < HEATING_TOL_F64,
            "heating must match Sutton-Graves"
        );
        let Ok(drag_mps2) = drag_accel(&state, &body, &atmosphere, &vehicle) else {
            panic!("drag must sample");
        };
        let expected_g = libm::sqrt(drag_mps2.length_squared()) / STANDARD_GRAVITY_MPS2;
        assert!(
            libm::fabs(g_load_g(drag_mps2) - expected_g) < 1e-12,
            "g-load must scale drag by standard gravity"
        );
    }

    // Step 1 bands: drag and heating at 50 km and 100 km plus terminal velocity.
    #[test]
    fn step1_aero_bands_and_terminal_velocity() {
        let body = test_body();
        let atmosphere = test_atmosphere();
        let vehicle = VehicleParams::preset();
        let low_state = circular_state_at_altitude(50_000.0);
        let Ok(low_drag_mps2) = drag_accel(&low_state, &body, &atmosphere, &vehicle) else {
            panic!("50 km drag must sample");
        };
        let low_drag_mag_mps2 = libm::sqrt(low_drag_mps2.length_squared());
        assert!(
            low_drag_mag_mps2 > 1.0 && low_drag_mag_mps2 < 15.0,
            "50 km drag must sit in the Step 1 band"
        );
        let Ok(low_heat) = heating(&low_state, &body, &atmosphere, &vehicle) else {
            panic!("50 km heating must sample");
        };
        assert!(
            low_heat.value() > 20_000.0 && low_heat.value() < 200_000.0,
            "50 km heating must sit in the Step 1 band"
        );
        let high_state = circular_state_at_altitude(100_000.0);
        let Ok(high_drag_mps2) = drag_accel(&high_state, &body, &atmosphere, &vehicle) else {
            panic!("100 km drag must sample");
        };
        let high_drag_mag_mps2 = libm::sqrt(high_drag_mps2.length_squared());
        assert!(
            high_drag_mag_mps2 > 0.001 && high_drag_mag_mps2 < 0.05,
            "100 km drag must sit in the Step 1 band"
        );
        let Ok(high_heat) = heating(&high_state, &body, &atmosphere, &vehicle) else {
            panic!("100 km heating must sample");
        };
        assert!(
            high_heat.value() > 500.0 && high_heat.value() < 15_000.0,
            "100 km heating must sit in the Step 1 band"
        );
        assert!(
            high_drag_mag_mps2 < low_drag_mag_mps2,
            "drag must fall from 50 km to 100 km"
        );
        assert!(
            high_heat.value() < low_heat.value(),
            "heating must fall from 50 km to 100 km"
        );
        let Ok(surface_sample) = atmosphere.sample_at_altitude(Meters::new(0.0), &body) else {
            panic!("surface atmosphere must sample");
        };
        let surface_density_kg_per_m3 = surface_sample.density_kg_per_m3().value();
        let surface_gravity_mps2 = body.surface_gravity().value();
        let ballistic_kg_per_m2 = vehicle.ballistic_coefficient().value();
        let terminal_mps = libm::sqrt(
            2.0 * ballistic_kg_per_m2 * surface_gravity_mps2 / surface_density_kg_per_m3,
        );
        assert!(
            terminal_mps > 220.0 && terminal_mps < 260.0,
            "terminal velocity must sit 220-260 m/s"
        );
    }

    // Gap 6: atmosphere steps never exceed one SIM_TICK_S tick.
    #[test]
    fn atmosphere_steps_never_exceed_tick() {
        assert!(
            libm::fabs(MAX_STEP_S - crate::sim::SIM_TICK_S.value()) < 1e-12,
            "MAX_STEP_S must equal SIM_TICK_S"
        );
        assert!(
            libm::fabs(STEP_S - crate::sim::SIM_TICK_S.value()) < 1e-12,
            "harness step must equal SIM_TICK_S"
        );
        let mu = test_mu();
        let body = test_body();
        let atmosphere = test_atmosphere();
        let vehicle = VehicleParams::preset();
        let step = Seconds::new(crate::sim::SIM_TICK_S.value());
        let low = circular_state_at_altitude(50_000.0);
        let Ok(_) = step_point_ship(&low, step, &body, &atmosphere, &vehicle, mu) else {
            panic!("tick-size atmosphere step must succeed");
        };
        let sinking = sinking_state_through_cutoff();
        let Ok(_) = step_point_ship(&sinking, step, &body, &atmosphere, &vehicle, mu) else {
            panic!("tick-size cutoff step must succeed");
        };
    }

    #[test]
    fn still_air_still_senses_corotation() {
        let body = test_body();
        let atmosphere = test_atmosphere();
        let vehicle = VehicleParams::preset();
        let radius_m = MARS_RADIUS_M + 50_000.0;
        let Ok(state) = StateVector::new(
            DVec3::new(radius_m, 0.0, 0.0),
            DVec3::ZERO,
            Seconds::new(0.0),
        ) else {
            panic!("still state must validate");
        };
        let Ok(heat_flux) = heating(&state, &body, &atmosphere, &vehicle) else {
            panic!("still heating must sample");
        };
        assert!(
            heat_flux.value() > 0.0,
            "corotating wind must heat a still ship"
        );
    }

    // AC3: steps keep epoch exact and displace at most speed times dt.
    #[test]
    fn atmosphere_step_keeps_time_and_stays_bounded() {
        let mu = test_mu();
        let body = test_body();
        let atmosphere = test_atmosphere();
        let vehicle = VehicleParams::preset();
        let state = circular_state_at_altitude(100_000.0);
        let step = Seconds::new(STEP_S);
        let Ok(sample) = step_point_ship(&state, step, &body, &atmosphere, &vehicle, mu) else {
            panic!("atmosphere step must succeed");
        };
        assert!(
            libm::fabs(sample.state.epoch.value() - STEP_S) <= EPOCH_TOL_S,
            "epoch must advance exactly"
        );
        let speed_mps = libm::sqrt(state.velocity_mps.length_squared());
        let displacement_m =
            libm::sqrt((sample.state.position_m - state.position_m).length_squared());
        assert!(
            displacement_m <= speed_mps * STEP_S * 1.5,
            "one tick must not teleport"
        );
        assert!(sample.heat_flux.value() >= 0.0);
        assert!(sample.g_load_g >= 0.0);
    }

    // AC3: rails steps match analytic advance bit-for-bit.
    #[test]
    fn rails_step_matches_analytic_advance() {
        let mu = test_mu();
        let body = test_body();
        let atmosphere = test_atmosphere();
        let vehicle = VehicleParams::preset();
        let state = circular_state_at_altitude(250_000.0);
        let step = Seconds::new(STEP_S);
        let Ok(sample) = step_point_ship(&state, step, &body, &atmosphere, &vehicle, mu) else {
            panic!("rails step must succeed");
        };
        let Ok(elements) = elements_from_state(&state, mu) else {
            panic!("rails elements must convert");
        };
        let Ok(expected) = advance(mu, &elements, step) else {
            panic!("rails advance must succeed");
        };
        assert_eq!(sample.state.position_m, expected.position_m);
        assert_eq!(sample.state.velocity_mps, expected.velocity_mps);
        assert_eq!(sample.drag_accel_mps2, DVec3::ZERO);
        assert!(sample.heat_flux.value().abs() < 1e-12);
        assert!(sample.g_load_g.abs() < 1e-12);
    }

    // AC3: continuity spot across the cutoff in both directions.
    #[test]
    fn boundary_crossing_stays_continuous() {
        let mu = test_mu();
        let body = test_body();
        let atmosphere = test_atmosphere();
        let vehicle = VehicleParams::preset();
        let step = Seconds::new(STEP_S);
        let climbing = climbing_state_through_cutoff();
        let Ok(up_sample) = step_point_ship(&climbing, step, &body, &atmosphere, &vehicle, mu)
        else {
            panic!("ascent split must succeed");
        };
        assert!(
            libm::fabs(up_sample.state.epoch.value() - STEP_S) <= EPOCH_TOL_S,
            "ascent epoch must stay exact"
        );
        let climb_speed_mps = libm::sqrt(climbing.velocity_mps.length_squared());
        let climb_jump_m =
            libm::sqrt((up_sample.state.position_m - climbing.position_m).length_squared());
        assert!(
            climb_jump_m <= climb_speed_mps * STEP_S * 2.0,
            "ascent must not jump at the cutoff"
        );
        let sinking = sinking_state_through_cutoff();
        let Ok(down_sample) = step_point_ship(&sinking, step, &body, &atmosphere, &vehicle, mu)
        else {
            panic!("descent split must succeed");
        };
        assert!(
            libm::fabs(down_sample.state.epoch.value() - STEP_S) <= EPOCH_TOL_S,
            "descent epoch must stay exact"
        );
        let sink_speed_mps = libm::sqrt(sinking.velocity_mps.length_squared());
        let sink_jump_m =
            libm::sqrt((down_sample.state.position_m - sinking.position_m).length_squared());
        assert!(
            sink_jump_m <= sink_speed_mps * STEP_S * 2.0,
            "descent must not jump at the cutoff"
        );
        assert!(down_sample.heat_flux.value() >= 0.0);
    }

    /// Headless descent outcome for repeatability checks.
    struct DescentOutcome {
        /// Penetrating state at first sub-surface tick.
        penetrating: StateVector,
        /// Parked surface state via `rest_state`.
        parked: StateVector,
        /// Steps taken, dimensionless.
        steps_u32: u32,
        /// Time below the rails cutoff in seconds.
        atm_time_s: f64,
        /// Peak g-load, dimensionless.
        peak_g: f64,
        /// Peak heating in watts per square meter.
        peak_heat_w_per_m2: f64,
    }

    /// Run one headless descent from 300 km circular to penetration.
    ///
    /// Applies the deorbit retrograde burn, steps at one tick, asserts
    /// per-tick 8.4 readout deltas at both crossings, then parks via
    /// surface `rest_state`. Deterministic; callers run it twice for
    /// repeatability.
    #[expect(
        clippy::too_many_lines,
        reason = "headless profile asserts all 8.4 readouts at both crossings"
    )]
    fn run_descent_once() -> DescentOutcome {
        let mu = test_mu();
        let body = test_body();
        let atmosphere = test_atmosphere();
        let vehicle = VehicleParams::preset();
        let start = circular_state_at_altitude(DESCENT_START_ALTITUDE_M);
        let Ok(burn) = Burn::new(
            BurnDirection::Retrograde,
            MetersPerSecond::new(DESCENT_RETRO_MPS),
        ) else {
            panic!("descent burn must validate");
        };
        let Ok(mut current) = apply_burn(&start, &burn) else {
            panic!("descent burn must apply");
        };
        let step = Seconds::new(STEP_S);
        let mut count_u32: u32 = 0;
        let mut atm_time_s = 0.0;
        let mut peak_g = 0.0;
        let mut peak_heat_w_per_m2 = 0.0;
        let mut prev_alt_m: Option<f64> = None;
        let mut prev_readouts: Option<(f64, f64, f64, f64, f64, f64, f64)> = None;
        for _ in 0..DESCENT_MAX_STEPS_U32 {
            let Ok(sample) = step_point_ship(&current, step, &body, &atmosphere, &vehicle, mu)
            else {
                panic!("descent step must succeed");
            };
            let next_state = sample.state;
            let radius_m = libm::sqrt(next_state.position_m.length_squared());
            let altitude_m = radius_m - MARS_RADIUS_M;
            let clamped_m = if altitude_m < 0.0 { 0.0 } else { altitude_m };
            let Ok(atm_state) = atmosphere.sample_at_altitude(Meters::new(clamped_m), &body) else {
                panic!("descent atmosphere must sample");
            };
            let spin_rad_s = core::f64::consts::TAU / body.rotation_period_s().value();
            let corotation_mps = DVec3::new(
                -spin_rad_s * next_state.position_m.y,
                spin_rad_s * next_state.position_m.x,
                0.0,
            );
            let rel_mps = next_state.velocity_mps - corotation_mps;
            let rel_speed_mps = libm::sqrt(rel_mps.length_squared());
            let curr = (
                altitude_m,
                rel_speed_mps,
                atm_state.pressure_pa().value(),
                atm_state.temperature_k().value(),
                atm_state.density_kg_per_m3().value(),
                sample.heat_flux.value(),
                sample.g_load_g,
            );
            if let (Some(prev_alt), Some(prev)) = (prev_alt_m, prev_readouts) {
                let crossed_rails =
                    (prev_alt > RAILS_ALTITUDE_M) != (altitude_m > RAILS_ALTITUDE_M);
                let crossed_surface = (prev_alt > 0.0) != (altitude_m > 0.0);
                if crossed_rails {
                    assert!(
                        libm::fabs(curr.0 - prev.0) < 200.0,
                        "rails altitude must stay continuous"
                    );
                    assert!(
                        libm::fabs(curr.1 - prev.1) < 5.0,
                        "rails velocity must stay continuous"
                    );
                    assert!(
                        libm::fabs(curr.2 - prev.2) < 0.01,
                        "rails pressure must stay continuous"
                    );
                    assert!(
                        libm::fabs(curr.3 - prev.3) < 1.0,
                        "rails temperature must stay continuous"
                    );
                    assert!(
                        libm::fabs(curr.4 - prev.4) < 1e-6,
                        "rails density must stay continuous"
                    );
                    assert!(
                        libm::fabs(curr.5 - prev.5) < 1_000.0,
                        "rails heating must stay continuous"
                    );
                    assert!(
                        libm::fabs(curr.6 - prev.6) < 0.1,
                        "rails g-load must stay continuous"
                    );
                }
                if crossed_surface {
                    assert!(
                        libm::fabs(curr.0 - prev.0) < 100.0,
                        "surface altitude must stay continuous"
                    );
                    assert!(
                        libm::fabs(curr.1 - prev.1) < 20.0,
                        "surface velocity must stay continuous"
                    );
                    assert!(
                        libm::fabs(curr.2 - prev.2) < 10.0,
                        "surface pressure must stay continuous"
                    );
                    assert!(
                        libm::fabs(curr.3 - prev.3) < 1.0,
                        "surface temperature must stay continuous"
                    );
                    assert!(
                        libm::fabs(curr.4 - prev.4) < 2e-4,
                        "surface density must stay continuous"
                    );
                    assert!(
                        libm::fabs(curr.5 - prev.5) < 50_000.0,
                        "surface heating must stay continuous"
                    );
                    assert!(
                        libm::fabs(curr.6 - prev.6) < 10.0,
                        "surface g-load must stay continuous"
                    );
                }
            }
            prev_alt_m = Some(altitude_m);
            prev_readouts = Some(curr);
            current = next_state;
            if altitude_m <= RAILS_ALTITUDE_M && altitude_m > 0.0 {
                atm_time_s += STEP_S;
            }
            if sample.g_load_g > peak_g {
                peak_g = sample.g_load_g;
            }
            if sample.heat_flux.value() > peak_heat_w_per_m2 {
                peak_heat_w_per_m2 = sample.heat_flux.value();
            }
            count_u32 += 1;
            if altitude_m <= 0.0 {
                break;
            }
        }
        assert!(count_u32 > 0, "descent must take at least one step");
        assert!(
            count_u32 < DESCENT_MAX_STEPS_U32,
            "descent must reach the surface within the cap"
        );
        let radius_m = libm::sqrt(current.position_m.length_squared());
        let altitude_m = radius_m - MARS_RADIUS_M;
        assert!(altitude_m <= 0.0, "descent must end penetrating");
        assert!(
            atm_time_s > 200.0 && atm_time_s < 600.0,
            "descent atmosphere time must sit 200-600 s"
        );
        assert!(
            peak_g > 1.5 && peak_g < 6.0,
            "descent peak g must sit 1.5-6 g"
        );
        assert!(
            peak_heat_w_per_m2 > 50_000.0 && peak_heat_w_per_m2 < 150_000.0,
            "descent peak heating must sit in band"
        );
        let frame = crate::surface::SurfaceFrame::new(&body);
        let Ok(parked) = frame.rest_state(&current) else {
            panic!("descent rest state must build");
        };
        let config = crate::surface::TouchdownConfig::preset();
        let Ok(touched) = frame.is_touchdown(&parked, &config) else {
            panic!("descent touchdown check must run");
        };
        assert!(touched, "parked descent must count as touchdown");
        let Ok(relative_mps) = frame.surface_relative_velocity(&parked) else {
            panic!("parked relative velocity must sample");
        };
        assert!(
            libm::sqrt(relative_mps.length_squared())
                < config.velocity_tolerance_mps().value() + 1e-9,
            "touchdown must sit below 5 m/s"
        );
        DescentOutcome {
            penetrating: current,
            parked,
            steps_u32: count_u32,
            atm_time_s,
            peak_g,
            peak_heat_w_per_m2,
        }
    }

    // AC3: headless full descent 300 km circular to touchdown via rest_state.
    #[test]
    fn full_descent_300km_to_touchdown() {
        let first = run_descent_once();
        let second = run_descent_once();
        assert_eq!(
            first.penetrating.position_m, second.penetrating.position_m,
            "descent must repeat within FP noise"
        );
        assert_eq!(
            first.penetrating.velocity_mps, second.penetrating.velocity_mps,
            "descent velocity must repeat within FP noise"
        );
        assert_eq!(
            first.parked.position_m, second.parked.position_m,
            "parked position must repeat"
        );
        assert_eq!(
            first.steps_u32, second.steps_u32,
            "descent step count must repeat"
        );
        assert!(
            libm::fabs(first.atm_time_s - second.atm_time_s) < 1e-9,
            "atmosphere time must repeat"
        );
        assert!(
            libm::fabs(first.peak_g - second.peak_g) < 1e-12,
            "peak g must repeat"
        );
        assert!(
            libm::fabs(first.peak_heat_w_per_m2 - second.peak_heat_w_per_m2) < 1e-6,
            "peak heating must repeat"
        );
    }

    /// Headless ascent outcome for repeatability checks.
    struct AscentOutcome {
        /// State at first rails crossing while climbing.
        rails_state: StateVector,
        /// Circularized state at apoapsis after the plane burn.
        circular_state: StateVector,
        /// Closed-orbit elements after circularization.
        elements: ClassicalElements,
        /// Climb steps taken, dimensionless.
        steps_u32: u32,
        /// Peak g-load during climb, dimensionless.
        peak_g: f64,
    }

    /// Run one headless ascent from the surface to a closed orbit.
    ///
    /// Launches from a surface rest state with a radial kick, sustains
    /// the climb with per-tick prograde boosts below 20 km, asserts
    /// per-tick 8.4 readout deltas at both crossings, coasts to
    /// apoapsis, then circularizes to a closed orbit. Deterministic;
    /// callers run it twice for repeatability.
    #[expect(
        clippy::too_many_lines,
        reason = "headless profile asserts all 8.4 readouts at both crossings"
    )]
    fn run_ascent_once() -> AscentOutcome {
        let mu = test_mu();
        let body = test_body();
        let atmosphere = test_atmosphere();
        let vehicle = VehicleParams::preset();
        let spin_rad_s = core::f64::consts::TAU / body.rotation_period_s().value();
        let corotation_mps = spin_rad_s * MARS_RADIUS_M;
        let Ok(start_state) = StateVector::new(
            DVec3::new(MARS_RADIUS_M, 0.0, 0.0),
            DVec3::new(ASCENT_LAUNCH_RADIAL_MPS, corotation_mps, 0.0),
            Seconds::new(0.0),
        ) else {
            panic!("ascent start must validate");
        };
        let Ok(start_atm) = atmosphere.sample_at_altitude(Meters::new(0.0), &body) else {
            panic!("start atm must sample");
        };
        let start_rel = DVec3::new(ASCENT_LAUNCH_RADIAL_MPS, 0.0, 0.0);
        let start_speed = libm::sqrt(start_rel.length_squared());
        let mut prev_readouts = Some((
            0.0,
            start_speed,
            start_atm.pressure_pa().value(),
            start_atm.temperature_k().value(),
            start_atm.density_kg_per_m3().value(),
            0.0,
            0.0,
        ));
        let mut prev_alt_m: Option<f64> = Some(0.0);
        let mut current = start_state;
        let step = Seconds::new(STEP_S);
        let mut count_u32: u32 = 0;
        let mut peak_g = 0.0;
        let mut rails_state: Option<StateVector> = None;
        for _ in 0..ASCENT_MAX_STEPS_U32 {
            let radius_m = libm::sqrt(current.position_m.length_squared());
            let altitude_m = radius_m - MARS_RADIUS_M;
            if altitude_m > 0.0 && altitude_m < ASCENT_BOOST_TOP_M {
                let Ok(boost) = Burn::new(
                    BurnDirection::Prograde,
                    MetersPerSecond::new(ASCENT_BOOST_MPS),
                ) else {
                    panic!("boost must validate");
                };
                let Ok(kicked) = apply_burn(&current, &boost) else {
                    panic!("boost must apply");
                };
                current = kicked;
            }
            let Ok(sample) = step_point_ship(&current, step, &body, &atmosphere, &vehicle, mu)
            else {
                panic!("ascent step must succeed");
            };
            let next_state = sample.state;
            let next_radius_m = libm::sqrt(next_state.position_m.length_squared());
            let next_alt_m = next_radius_m - MARS_RADIUS_M;
            let clamped_m = if next_alt_m < 0.0 { 0.0 } else { next_alt_m };
            let Ok(atm_state) = atmosphere.sample_at_altitude(Meters::new(clamped_m), &body) else {
                panic!("ascent atmosphere must sample");
            };
            let corotation_mps_vec = DVec3::new(
                -spin_rad_s * next_state.position_m.y,
                spin_rad_s * next_state.position_m.x,
                0.0,
            );
            let rel_mps = next_state.velocity_mps - corotation_mps_vec;
            let rel_speed_mps = libm::sqrt(rel_mps.length_squared());
            let curr = (
                next_alt_m,
                rel_speed_mps,
                atm_state.pressure_pa().value(),
                atm_state.temperature_k().value(),
                atm_state.density_kg_per_m3().value(),
                sample.heat_flux.value(),
                sample.g_load_g,
            );
            if let (Some(prev_alt), Some(prev)) = (prev_alt_m, prev_readouts) {
                let crossed_rails =
                    (prev_alt > RAILS_ALTITUDE_M) != (next_alt_m > RAILS_ALTITUDE_M);
                let crossed_surface = (prev_alt > 0.0) != (next_alt_m > 0.0);
                if crossed_rails {
                    assert!(
                        libm::fabs(curr.0 - prev.0) < 200.0,
                        "ascent rails altitude must stay continuous"
                    );
                    assert!(
                        libm::fabs(curr.1 - prev.1) < 5.0,
                        "ascent rails velocity must stay continuous"
                    );
                    assert!(
                        libm::fabs(curr.2 - prev.2) < 0.01,
                        "ascent rails pressure must stay continuous"
                    );
                    assert!(
                        libm::fabs(curr.3 - prev.3) < 1.0,
                        "ascent rails temperature must stay continuous"
                    );
                    assert!(
                        libm::fabs(curr.4 - prev.4) < 1e-6,
                        "ascent rails density must stay continuous"
                    );
                    assert!(
                        libm::fabs(curr.5 - prev.5) < 1_000.0,
                        "ascent rails heating must stay continuous"
                    );
                    assert!(
                        libm::fabs(curr.6 - prev.6) < 0.1,
                        "ascent rails g-load must stay continuous"
                    );
                }
                if crossed_surface {
                    assert!(
                        libm::fabs(curr.0 - prev.0) < 100.0,
                        "ascent surface altitude must stay continuous"
                    );
                    assert!(
                        libm::fabs(curr.1 - prev.1) < 20.0,
                        "ascent surface velocity must stay continuous"
                    );
                    assert!(
                        libm::fabs(curr.2 - prev.2) < 10.0,
                        "ascent surface pressure must stay continuous"
                    );
                    assert!(
                        libm::fabs(curr.3 - prev.3) < 1.0,
                        "ascent surface temperature must stay continuous"
                    );
                    assert!(
                        libm::fabs(curr.4 - prev.4) < 2e-4,
                        "ascent surface density must stay continuous"
                    );
                    assert!(
                        libm::fabs(curr.5 - prev.5) < 50_000.0,
                        "ascent surface heating must stay continuous"
                    );
                    assert!(
                        libm::fabs(curr.6 - prev.6) < 10.0,
                        "ascent surface g-load must stay continuous"
                    );
                }
            }
            prev_alt_m = Some(next_alt_m);
            prev_readouts = Some(curr);
            current = next_state;
            if sample.g_load_g > peak_g {
                peak_g = sample.g_load_g;
            }
            count_u32 += 1;
            let radius_m = libm::sqrt(current.position_m.length_squared());
            let altitude_m = radius_m - MARS_RADIUS_M;
            if altitude_m > RAILS_ALTITUDE_M {
                let Ok(_) = elements_from_state(&current, mu) else {
                    panic!("ascent rails state must stay bound");
                };
                rails_state = Some(current);
                break;
            }
            assert!(
                altitude_m > -1.0,
                "ascent must not crash before reaching rails"
            );
        }
        let Some(climb_top) = rails_state else {
            panic!("ascent must reach rails within the cap");
        };
        assert!(
            count_u32 < ASCENT_MAX_STEPS_U32,
            "ascent must reach rails in time"
        );
        assert!(
            peak_g > 3.0 && peak_g < 10.0,
            "ascent peak g must sit 3-10 g"
        );
        let mut apoapsis_m = libm::sqrt(climb_top.position_m.length_squared());
        let mut apoapsis_state = climb_top;
        current = climb_top;
        let coast = Seconds::new(5.0);
        for _ in 0..5_000 {
            let Ok(sample) = step_point_ship(&current, coast, &body, &atmosphere, &vehicle, mu)
            else {
                panic!("ascent coast must succeed");
            };
            current = sample.state;
            let radius_m = libm::sqrt(current.position_m.length_squared());
            if radius_m > apoapsis_m {
                apoapsis_m = radius_m;
                apoapsis_state = current;
            }
            let falling = radius_m < apoapsis_m - 1.0;
            if falling {
                break;
            }
        }
        let apoapsis_alt_m = apoapsis_m - MARS_RADIUS_M;
        assert!(
            apoapsis_alt_m > RAILS_ALTITUDE_M,
            "apoapsis must clear rails"
        );
        let apoapsis_speed_mps = libm::sqrt(apoapsis_state.velocity_mps.length_squared());
        let circular_speed_mps = libm::sqrt(mu.value() / apoapsis_m);
        let need_mps = circular_speed_mps - apoapsis_speed_mps;
        assert!(need_mps > 0.0, "circularization must need prograde");
        let Ok(circular_burn) = Burn::new(BurnDirection::Prograde, MetersPerSecond::new(need_mps))
        else {
            panic!("circularization burn must validate");
        };
        let Ok(circular_state) = apply_burn(&apoapsis_state, &circular_burn) else {
            panic!("circularization burn must apply");
        };
        let Ok(elements) = elements_from_state(&circular_state, mu) else {
            panic!("circularized elements must convert");
        };
        assert!(
            elements.eccentricity_f64 < 0.05,
            "ascent must end in a near-circular closed orbit"
        );
        let periapsis_m = elements.semi_major_axis.value() * (1.0 - elements.eccentricity_f64);
        assert!(
            periapsis_m - MARS_RADIUS_M > 200_000.0,
            "circularized periapsis must clear the atmosphere"
        );
        AscentOutcome {
            rails_state: climb_top,
            circular_state,
            elements,
            steps_u32: count_u32,
            peak_g,
        }
    }

    // AC3: headless ascent from the surface to a closed orbit.
    #[test]
    fn full_ascent_surface_to_closed_orbit() {
        let first = run_ascent_once();
        let second = run_ascent_once();
        assert_eq!(
            first.circular_state.position_m, second.circular_state.position_m,
            "ascent must repeat within FP noise"
        );
        assert_eq!(
            first.circular_state.velocity_mps, second.circular_state.velocity_mps,
            "ascent velocity must repeat within FP noise"
        );
        assert_eq!(
            first.rails_state.position_m, second.rails_state.position_m,
            "rails crossing must repeat within FP noise"
        );
        assert_eq!(
            first.steps_u32, second.steps_u32,
            "ascent step count must repeat"
        );
        assert!(
            libm::fabs(first.peak_g - second.peak_g) < 1e-12,
            "ascent peak g must repeat"
        );
        assert!(
            libm::fabs(
                first.elements.semi_major_axis.value() - second.elements.semi_major_axis.value()
            ) < 1e-6,
            "ascent semi-major axis must repeat"
        );
    }

    #[test]
    fn point_ship_holds_state_across_steps() {
        let mu = test_mu();
        let body = test_body();
        let atmosphere = test_atmosphere();
        let vehicle = VehicleParams::preset();
        let initial = circular_state_at_altitude(250_000.0);
        let mut ship = PointShipTrajectory::new(vehicle, mu, initial);
        assert_eq!(ship.state(), initial);
        let Ok(first) = ship.step(Seconds::new(STEP_S), &body, &atmosphere) else {
            panic!("first ship step must succeed");
        };
        assert_eq!(ship.state(), first.state);
        let Ok(second) = ship.step(Seconds::new(STEP_S), &body, &atmosphere) else {
            panic!("second ship step must succeed");
        };
        assert!(
            libm::fabs(second.state.epoch.value() - 2.0 * STEP_S) < 1e-12,
            "ship epochs must accumulate"
        );
    }

    #[test]
    fn bad_steps_are_rejected() {
        let mu = test_mu();
        let body = test_body();
        let atmosphere = test_atmosphere();
        let vehicle = VehicleParams::preset();
        let state = circular_state_at_altitude(250_000.0);
        assert!(matches!(
            step_point_ship(&state, Seconds::new(0.0), &body, &atmosphere, &vehicle, mu),
            Err(TrajectoryError::InvalidStep { .. })
        ));
        assert!(matches!(
            step_point_ship(
                &state,
                Seconds::new(-STEP_S),
                &body,
                &atmosphere,
                &vehicle,
                mu
            ),
            Err(TrajectoryError::InvalidStep { .. })
        ));
    }

    /// State climbing through the cutoff within one tick.
    fn climbing_state_through_cutoff() -> StateVector {
        let radius_m = MARS_RADIUS_M + RAILS_ALTITUDE_M - 50.0;
        let Ok(state) = StateVector::new(
            DVec3::new(radius_m, 0.0, 0.0),
            DVec3::new(2_000.0, 3_400.0, 0.0),
            Seconds::new(0.0),
        ) else {
            panic!("climbing state must validate");
        };
        state
    }

    /// State sinking through the cutoff within one tick.
    fn sinking_state_through_cutoff() -> StateVector {
        let radius_m = MARS_RADIUS_M + RAILS_ALTITUDE_M + 50.0;
        let Ok(state) = StateVector::new(
            DVec3::new(radius_m, 0.0, 0.0),
            DVec3::new(-2_000.0, 3_400.0, 0.0),
            Seconds::new(0.0),
        ) else {
            panic!("sinking state must validate");
        };
        state
    }

    /// Corotating relative wind for direction assertions.
    fn relative_wind_for_test(state: &StateVector, body: &BodyParams) -> (DVec3, f64, f64) {
        let Ok(wind) = relative_wind_mps(state, body) else {
            panic!("test wind must compute");
        };
        wind
    }

    /// Sampled density for magnitude assertions.
    fn density_for_test(altitude_m: f64, body: &BodyParams, atmosphere: &AtmosphereParams) -> f64 {
        let Ok(density) = sample_density_kg_per_m3(altitude_m, body, atmosphere) else {
            panic!("test density must sample");
        };
        density
    }
}
