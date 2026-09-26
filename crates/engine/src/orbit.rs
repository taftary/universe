//! Keplerian two-body propagation and patched-conics selection.
//!
//! Owns analytic coasting orbits used on rails under warp. Warp policy
//! lives in [`crate::warp`]; this module owns the math. All transcendentals
//! use the `libm` crate so `x86_64` and `AArch64` agree. No `std` trigonometry
//! or square root appears here, including in tests.
//!
//! Prograde and retrograde burns are out of scope here. This module only
//! coasts; trajectory burns live in issue 4 scope, not this issue.
//!
//! Cost: one [`propagate`] call costs one [`solve_kepler`] run (at most
//! [`KEPLER_MAX_ITERATIONS`] Newton steps, each one `sin` plus one `cos`)
//! plus a fixed tail of about six `sin`/`cos`, two `sqrt`, and one `atan2`.
//! No allocation occurs after warmup; all state is stack-local.
//!
//! File split: [`crate::warp`] owns `Warp`, `WarpContext`, and `request_warp`
//! and imports orbit error types. This file owns everything else.

use glam::DVec3;
use thiserror::Error;

use crate::units::{Kilograms, Meters, Seconds};

/// Kepler solver tolerance in radians. Source: mathematician contract.
pub const KEPLER_TOL_RAD: f64 = 1e-12;

/// Kepler solver iteration cap, dimensionless. Source: mathematician contract.
pub const KEPLER_MAX_ITERATIONS: u32 = 50;

/// Full circle in radians. Source: `core::f64::consts::TAU`.
pub const TAU: f64 = core::f64::consts::TAU;

/// Eccentricity threshold for the high-e initial guess, dimensionless.
///
/// Below this value the solver starts from the mean anomaly; at or above
/// it starts from pi. Source: mathematician contract.
pub const HIGH_ECC: f64 = 0.8;

/// Sphere-of-influence exponent `2/5`, dimensionless.
///
/// Source: Laplace sphere-of-influence radius `a (m / M)^(2/5)`.
pub const SOI_EXPONENT: f64 = 0.4;

/// Standard gravitational parameter in cubic meters per square second.
///
/// Generic two-body parameter. For the reference planet, build it with
/// `BodyParams::gravitational_parameter_m3_s2` in [`crate::body`]; issue 4
/// owns the orbit-body integration, so body constants are not duplicated here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mu {
    /// Gravitational parameter in `m^3/s^2`, always positive and finite.
    value_m3_s2: f64,
}

impl Mu {
    /// Create a gravitational parameter from its SI magnitude.
    ///
    /// # Errors
    ///
    /// Returns [`OrbitError::InvalidMu`] when `value_m3_s2` is not positive and finite.
    pub fn new(value_m3_s2: f64) -> Result<Self, OrbitError> {
        if value_m3_s2.is_finite() && value_m3_s2 > 0.0 {
            Ok(Self { value_m3_s2 })
        } else {
            Err(OrbitError::InvalidMu {
                mu_m3_s2: value_m3_s2,
            })
        }
    }

    /// Return the inner SI magnitude in `m^3/s^2`.
    #[must_use]
    pub const fn value(self) -> f64 {
        self.value_m3_s2
    }
}

/// Classical Keplerian elements at an epoch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClassicalElements {
    /// Semi-major axis in meters, always positive and finite.
    pub semi_major_axis: Meters,
    /// Eccentricity, dimensionless, always in `[0, 1)`.
    pub eccentricity_f64: f64,
    /// Inclination in radians, always finite.
    pub inclination_rad: f64,
    /// Right ascension of ascending node in radians, always finite.
    pub raan_rad: f64,
    /// Argument of periapsis in radians, always finite.
    pub arg_periapsis_rad: f64,
    /// Mean anomaly at epoch in radians, always finite.
    pub mean_anomaly_at_epoch_rad: f64,
    /// Epoch in seconds, always finite.
    pub epoch: Seconds,
}

impl ClassicalElements {
    /// Create classical elements with range checks.
    ///
    /// All angles are in radians and accept any finite value.
    ///
    /// # Errors
    ///
    /// Returns [`OrbitError`] when the semi-major axis, eccentricity, any
    /// angle, or epoch is out of range.
    pub fn new(
        semi_major_axis: Meters,
        eccentricity_f64: f64,
        inclination_rad: f64,
        raan_rad: f64,
        arg_periapsis_rad: f64,
        mean_anomaly_at_epoch_rad: f64,
        epoch: Seconds,
    ) -> Result<Self, OrbitError> {
        let axis_m = semi_major_axis.value();
        if !(axis_m.is_finite() && axis_m > 0.0) {
            return Err(OrbitError::InvalidSemiMajorAxis {
                semi_major_axis_m: axis_m,
            });
        }
        if !(eccentricity_f64.is_finite() && (0.0..1.0).contains(&eccentricity_f64)) {
            return Err(OrbitError::InvalidEccentricity { eccentricity_f64 });
        }
        if !inclination_rad.is_finite() {
            return Err(OrbitError::InvalidAngle {
                value_rad: inclination_rad,
            });
        }
        if !raan_rad.is_finite() {
            return Err(OrbitError::InvalidAngle {
                value_rad: raan_rad,
            });
        }
        if !arg_periapsis_rad.is_finite() {
            return Err(OrbitError::InvalidAngle {
                value_rad: arg_periapsis_rad,
            });
        }
        if !mean_anomaly_at_epoch_rad.is_finite() {
            return Err(OrbitError::InvalidMeanAnomaly {
                mean_anomaly_rad: mean_anomaly_at_epoch_rad,
            });
        }
        if !epoch.value().is_finite() {
            return Err(OrbitError::InvalidEpoch {
                epoch_s: epoch.value(),
            });
        }
        Ok(Self {
            semi_major_axis,
            eccentricity_f64,
            inclination_rad,
            raan_rad,
            arg_periapsis_rad,
            mean_anomaly_at_epoch_rad,
            epoch,
        })
    }
}

/// Inertial position and velocity at one instant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropagatedState {
    /// Inertial position in meters.
    pub position_m: DVec3,
    /// Inertial velocity in meters per second.
    pub velocity_mps: DVec3,
}

/// One patched-conics gravity center.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GravitationalCenter {
    /// Stable center name for logs and snapshots.
    pub name: &'static str,
    /// Gravitational parameter in `m^3/s^2`.
    pub mu: Mu,
    /// Body mass in kilograms, always positive and finite.
    pub mass: Kilograms,
    /// Body radius in meters, always positive and finite.
    pub radius: Meters,
    /// Center position in meters in the parent frame, always finite.
    pub position_m: DVec3,
}

impl GravitationalCenter {
    /// Create a gravity center with range checks.
    ///
    /// # Errors
    ///
    /// Returns [`OrbitError::InvalidMass`] for a bad mass,
    /// [`OrbitError::InvalidSemiMajorAxis`] for a bad radius, or
    /// [`OrbitError::InvalidPosition`] for a non-finite position.
    pub fn new(
        name: &'static str,
        mu: Mu,
        mass: Kilograms,
        radius: Meters,
        position_m: DVec3,
    ) -> Result<Self, OrbitError> {
        let mass_kg = mass.value();
        if !(mass_kg.is_finite() && mass_kg > 0.0) {
            return Err(OrbitError::InvalidMass { mass_kg });
        }
        let radius_m = radius.value();
        if !(radius_m.is_finite() && radius_m > 0.0) {
            return Err(OrbitError::InvalidSemiMajorAxis {
                semi_major_axis_m: radius_m,
            });
        }
        if !(position_m.x.is_finite() && position_m.y.is_finite() && position_m.z.is_finite()) {
            return Err(OrbitError::InvalidPosition);
        }
        Ok(Self {
            name,
            mu,
            mass,
            radius,
            position_m,
        })
    }
}

/// Typed orbit and warp-policy failures.
#[derive(Debug, Error, Clone, Copy, PartialEq)]
pub enum OrbitError {
    /// Eccentricity was outside `[0, 1)`.
    #[error("eccentricity {eccentricity_f64} out of range [0, 1)")]
    InvalidEccentricity {
        /// Rejected eccentricity, dimensionless.
        eccentricity_f64: f64,
    },
    /// Semi-major axis or radius was not positive and finite.
    #[error("semi-major axis {semi_major_axis_m} must be positive and finite")]
    InvalidSemiMajorAxis {
        /// Rejected length in meters.
        semi_major_axis_m: f64,
    },
    /// Gravitational parameter was not positive and finite.
    #[error("gravitational parameter {mu_m3_s2} must be positive and finite")]
    InvalidMu {
        /// Rejected parameter in `m^3/s^2`.
        mu_m3_s2: f64,
    },
    /// Mean anomaly was not finite.
    #[error("mean anomaly {mean_anomaly_rad} must be finite")]
    InvalidMeanAnomaly {
        /// Rejected anomaly in radians.
        mean_anomaly_rad: f64,
    },
    /// Generic angle was not finite.
    #[error("angle {value_rad} must be finite")]
    InvalidAngle {
        /// Rejected angle in radians.
        value_rad: f64,
    },
    /// Epoch was not finite.
    #[error("epoch {epoch_s} must be finite")]
    InvalidEpoch {
        /// Rejected epoch in seconds.
        epoch_s: f64,
    },
    /// Time step was not finite.
    #[error("time step {delta_s} must be finite")]
    InvalidDelta {
        /// Rejected step in seconds.
        delta_s: f64,
    },
    /// Mass was not positive and finite.
    #[error("mass {mass_kg} must be positive and finite")]
    InvalidMass {
        /// Rejected mass in kilograms.
        mass_kg: f64,
    },
    /// No gravity centers were provided.
    #[error("no gravitational centers provided")]
    EmptyCenters,
    /// Kepler solver did not converge in time.
    #[error("Kepler solver did not converge in {iterations_u32} iterations")]
    NonConvergence {
        /// Iteration cap that was exhausted, dimensionless.
        iterations_u32: u32,
    },
    /// Requested warp factor was denied by policy.
    #[error("warp {requested_factor_f64}x denied")]
    WarpDenied {
        /// Denied factor, dimensionless.
        requested_factor_f64: f64,
    },
    /// Position was not finite.
    #[error("position must be finite")]
    InvalidPosition,
}

/// Solve Kepler's equation for eccentric anomaly.
///
/// Input mean anomaly in radians and dimensionless eccentricity yield the
/// eccentric anomaly in radians via Newton iteration.
///
/// # Errors
///
/// Returns [`OrbitError::InvalidMeanAnomaly`] or
/// [`OrbitError::InvalidEccentricity`] for out-of-range inputs, or
/// [`OrbitError::NonConvergence`] when the cap is exhausted.
#[must_use = "solver output must be used"]
pub fn solve_kepler(mean_anomaly_rad: f64, eccentricity_f64: f64) -> Result<f64, OrbitError> {
    if !mean_anomaly_rad.is_finite() {
        return Err(OrbitError::InvalidMeanAnomaly { mean_anomaly_rad });
    }
    if !(eccentricity_f64.is_finite() && (0.0..1.0).contains(&eccentricity_f64)) {
        return Err(OrbitError::InvalidEccentricity { eccentricity_f64 });
    }
    let normalized_rad = libm::fmod(mean_anomaly_rad, TAU);
    let wrapped_rad = if normalized_rad < 0.0 {
        normalized_rad + TAU
    } else {
        normalized_rad
    };
    let mut current_rad = if eccentricity_f64 < HIGH_ECC {
        wrapped_rad
    } else {
        core::f64::consts::PI
    };
    for _ in 0..KEPLER_MAX_ITERATIONS {
        let sine = libm::sin(current_rad);
        let cosine = libm::cos(current_rad);
        let residual = current_rad - eccentricity_f64 * sine - wrapped_rad;
        let derivative = 1.0 - eccentricity_f64 * cosine;
        let step = residual / derivative;
        current_rad -= step;
        if libm::fabs(step) < KEPLER_TOL_RAD {
            return Ok(current_rad);
        }
    }
    Err(OrbitError::NonConvergence {
        iterations_u32: KEPLER_MAX_ITERATIONS,
    })
}

/// Compute mean motion in radians per second.
///
/// Takes a gravitational parameter and semi-major axis and returns `n`.
///
/// # Errors
///
/// Returns [`OrbitError::InvalidSemiMajorAxis`] for a bad axis.
#[must_use = "solver output must be used"]
pub fn mean_motion(mu: Mu, semi_major_axis: Meters) -> Result<f64, OrbitError> {
    let axis_m = semi_major_axis.value();
    if !(axis_m.is_finite() && axis_m > 0.0) {
        return Err(OrbitError::InvalidSemiMajorAxis {
            semi_major_axis_m: axis_m,
        });
    }
    let cubed_m3 = axis_m * axis_m * axis_m;
    Ok(libm::sqrt(mu.value() / cubed_m3))
}

/// Compute the orbital period in seconds.
///
/// Takes a gravitational parameter and semi-major axis and returns `2 pi / n`.
///
/// # Errors
///
/// Returns [`OrbitError::InvalidSemiMajorAxis`] for a bad axis.
#[must_use = "solver output must be used"]
pub fn orbital_period(mu: Mu, semi_major_axis: Meters) -> Result<Seconds, OrbitError> {
    let motion_rad_s = mean_motion(mu, semi_major_axis)?;
    Ok(Seconds::new(TAU / motion_rad_s))
}

/// Compute specific orbital energy in joules per kilogram.
///
/// Takes a gravitational parameter and semi-major axis and returns `-mu / (2a)`.
/// Pure IEEE arithmetic, no transcendentals.
///
/// # Errors
///
/// Returns [`OrbitError::InvalidSemiMajorAxis`] for a bad axis.
#[must_use = "solver output must be used"]
pub fn specific_energy(mu: Mu, semi_major_axis: Meters) -> Result<f64, OrbitError> {
    let axis_m = semi_major_axis.value();
    if !(axis_m.is_finite() && axis_m > 0.0) {
        return Err(OrbitError::InvalidSemiMajorAxis {
            semi_major_axis_m: axis_m,
        });
    }
    Ok(-mu.value() / (2.0 * axis_m))
}

/// Compute specific angular momentum magnitude in square meters per second.
///
/// Takes a gravitational parameter, semi-major axis, and eccentricity and
/// returns `sqrt(mu * p)` with `p = a (1 - e^2)` via `libm`.
///
/// # Errors
///
/// Returns [`OrbitError::InvalidSemiMajorAxis`] for a bad axis, or
/// [`OrbitError::InvalidEccentricity`] for eccentricity outside `[0, 1)`.
#[must_use = "solver output must be used"]
pub fn specific_angular_momentum(
    mu: Mu,
    semi_major_axis: Meters,
    eccentricity_f64: f64,
) -> Result<f64, OrbitError> {
    let axis_m = semi_major_axis.value();
    if !(axis_m.is_finite() && axis_m > 0.0) {
        return Err(OrbitError::InvalidSemiMajorAxis {
            semi_major_axis_m: axis_m,
        });
    }
    if !(eccentricity_f64.is_finite() && (0.0..1.0).contains(&eccentricity_f64)) {
        return Err(OrbitError::InvalidEccentricity { eccentricity_f64 });
    }
    let semi_latus_m = axis_m * (1.0 - eccentricity_f64 * eccentricity_f64);
    Ok(libm::sqrt(mu.value() * semi_latus_m))
}

/// Propagate elements to an absolute target time.
///
/// Solves Kepler's equation at the target epoch and rotates the perifocal
/// state into the inertial frame.
///
/// # Errors
///
/// Returns [`OrbitError`] for a non-finite target, a bad derived anomaly,
/// or a failed Kepler solve.
#[expect(
    clippy::similar_names,
    reason = "perifocal x/y and rotation rows are conventional"
)]
pub fn propagate(
    mu: Mu,
    elements: &ClassicalElements,
    target: Seconds,
) -> Result<PropagatedState, OrbitError> {
    if !target.value().is_finite() {
        return Err(OrbitError::InvalidEpoch {
            epoch_s: target.value(),
        });
    }
    let delta_s = target.value() - elements.epoch.value();
    if !delta_s.is_finite() {
        return Err(OrbitError::InvalidDelta { delta_s });
    }
    let motion_rad_s = mean_motion(mu, elements.semi_major_axis)?;
    let mean_rad = elements.mean_anomaly_at_epoch_rad + motion_rad_s * delta_s;
    if !mean_rad.is_finite() {
        return Err(OrbitError::InvalidMeanAnomaly {
            mean_anomaly_rad: mean_rad,
        });
    }
    let eccentric_rad = solve_kepler(mean_rad, elements.eccentricity_f64)?;
    let axis_m = elements.semi_major_axis.value();
    let ecc = elements.eccentricity_f64;
    let cos_ecc = libm::cos(eccentric_rad);
    let sin_ecc = libm::sin(eccentric_rad);
    let one_minus_e_cos = 1.0 - ecc * cos_ecc;
    let radius_m = axis_m * one_minus_e_cos;
    let cos_true = (cos_ecc - ecc) / one_minus_e_cos;
    let one_minus_e2 = (1.0 - ecc) * (1.0 + ecc);
    let sin_true = (libm::sqrt(one_minus_e2) * sin_ecc) / one_minus_e_cos;
    let pos_pf_x_m = radius_m * cos_true;
    let pos_pf_y_m = radius_m * sin_true;
    let semi_latus_m = axis_m * (1.0 - ecc * ecc);
    let speed_factor_mps = libm::sqrt(mu.value() / semi_latus_m);
    let vel_pf_x_mps = -speed_factor_mps * sin_true;
    let vel_pf_y_mps = speed_factor_mps * (ecc + cos_true);
    let cos_raan = libm::cos(elements.raan_rad);
    let sin_raan = libm::sin(elements.raan_rad);
    let cos_inc = libm::cos(elements.inclination_rad);
    let sin_inc = libm::sin(elements.inclination_rad);
    let cos_arg = libm::cos(elements.arg_periapsis_rad);
    let sin_arg = libm::sin(elements.arg_periapsis_rad);
    let row_x_x = cos_raan * cos_arg - sin_raan * sin_arg * cos_inc;
    let row_x_y = -cos_raan * sin_arg - sin_raan * cos_arg * cos_inc;
    let row_y_x = sin_raan * cos_arg + cos_raan * sin_arg * cos_inc;
    let row_y_y = -sin_raan * sin_arg + cos_raan * cos_arg * cos_inc;
    let row_z_x = sin_arg * sin_inc;
    let row_z_y = cos_arg * sin_inc;
    Ok(PropagatedState {
        position_m: DVec3::new(
            row_x_x * pos_pf_x_m + row_x_y * pos_pf_y_m,
            row_y_x * pos_pf_x_m + row_y_y * pos_pf_y_m,
            row_z_x * pos_pf_x_m + row_z_y * pos_pf_y_m,
        ),
        velocity_mps: DVec3::new(
            row_x_x * vel_pf_x_mps + row_x_y * vel_pf_y_mps,
            row_y_x * vel_pf_x_mps + row_y_y * vel_pf_y_mps,
            row_z_x * vel_pf_x_mps + row_z_y * vel_pf_y_mps,
        ),
    })
}

/// Advance elements by a relative time step.
///
/// Adds `delta` to the element epoch and calls [`propagate`], so it is
/// bit-identical to propagating to `epoch + delta`.
///
/// # Errors
///
/// Returns [`OrbitError::InvalidDelta`] for a non-finite step, or any
/// [`propagate`] error for the derived target.
pub fn advance(
    mu: Mu,
    elements: &ClassicalElements,
    delta: Seconds,
) -> Result<PropagatedState, OrbitError> {
    if !delta.value().is_finite() {
        return Err(OrbitError::InvalidDelta {
            delta_s: delta.value(),
        });
    }
    let target = Seconds::new(elements.epoch.value() + delta.value());
    propagate(mu, elements, target)
}

/// Select the dominant gravity center by Newtonian acceleration.
///
/// Returns the index into `centers` with the largest `mu / r^2`. An exact
/// center hit returns immediately. Ties keep the first maximum.
///
/// # Errors
///
/// Returns [`OrbitError::InvalidPosition`] for a non-finite position, or
/// [`OrbitError::EmptyCenters`] when `centers` is empty.
pub fn select_center(
    position_m: DVec3,
    centers: &[GravitationalCenter],
) -> Result<usize, OrbitError> {
    if !(position_m.x.is_finite() && position_m.y.is_finite() && position_m.z.is_finite()) {
        return Err(OrbitError::InvalidPosition);
    }
    if centers.is_empty() {
        return Err(OrbitError::EmptyCenters);
    }
    let mut best_index_usize = 0_usize;
    let mut best_accel_mps2 = f64::NEG_INFINITY;
    for (index_usize, center) in centers.iter().enumerate() {
        let offset_m = position_m - center.position_m;
        let distance_squared_m2 = offset_m.length_squared();
        if distance_squared_m2 <= 0.0 {
            return Ok(index_usize);
        }
        let accel_mps2 = center.mu.value() / distance_squared_m2;
        if accel_mps2 > best_accel_mps2 {
            best_accel_mps2 = accel_mps2;
            best_index_usize = index_usize;
        }
    }
    Ok(best_index_usize)
}

/// Compute the Laplace sphere-of-influence radius in meters.
///
/// Implements `a (m / M)^(2/5)` for the secondary semi-major axis `a`.
///
/// # Errors
///
/// Returns [`OrbitError::InvalidMass`] for bad masses, or
/// [`OrbitError::InvalidSemiMajorAxis`] for a bad axis.
pub fn sphere_of_influence(
    primary_mass: Kilograms,
    secondary_mass: Kilograms,
    semi_major_axis: Meters,
) -> Result<Meters, OrbitError> {
    let primary_kg = primary_mass.value();
    if !(primary_kg.is_finite() && primary_kg > 0.0) {
        return Err(OrbitError::InvalidMass {
            mass_kg: primary_kg,
        });
    }
    let secondary_kg = secondary_mass.value();
    if !(secondary_kg.is_finite() && secondary_kg > 0.0) {
        return Err(OrbitError::InvalidMass {
            mass_kg: secondary_kg,
        });
    }
    let axis_m = semi_major_axis.value();
    if !(axis_m.is_finite() && axis_m > 0.0) {
        return Err(OrbitError::InvalidSemiMajorAxis {
            semi_major_axis_m: axis_m,
        });
    }
    let ratio = secondary_kg / primary_kg;
    Ok(Meters::new(axis_m * libm::pow(ratio, SOI_EXPONENT)))
}

#[cfg(test)]
mod tests {
    use super::*;

    // T1 reference: low Mars orbit circular case.
    const MARS_MU_M3_S2: f64 = 4.282_837e13;
    const MARS_RADIUS_M: f64 = 3_389_500.0;
    const LMO_ALTITUDE_M: f64 = 250_000.0;
    const EXPECTED_PERIOD_S: f64 = 6_666.2;
    const PERIOD_TOL_S: f64 = 2.0;
    const EXPECTED_SPEED_MPS: f64 = 3_430.4;
    const SPEED_TOL_MPS: f64 = 1.0;
    const CLOSURE_TOL_M: f64 = 10.0;

    // T2 reference: Kepler solve value.
    const T2_MEAN_RAD: f64 = 1.0;
    const T2_ECC: f64 = 0.5;
    const T2_EXPECTED_RAD: f64 = 1.498_701_3;
    const T2_TOL_RAD: f64 = 1e-6;

    // T5 reference: Mars sphere of influence.
    const SUN_MASS_KG: f64 = 1.988_47e30;
    const MARS_MASS_KG: f64 = 6.417_1e23;
    const MARS_AXIS_M: f64 = 2.279_392e11;
    const EXPECTED_SOI_M: f64 = 5.78e8;
    const SOI_TOL_M: f64 = 0.05e8;

    // G1 invariant tolerances: absolute tolerances on state-derived energy/momentum.
    const ENERGY_TOL_J_PER_KG: f64 = 10.0;
    const MOMENTUM_TOL_M2_S: f64 = 100.0;
    const G1_ECC: f64 = 0.3;
    const QUARTER_PERIOD_DIVISOR: f64 = 4.0;

    fn test_mu() -> Mu {
        let Ok(mu) = Mu::new(MARS_MU_M3_S2) else {
            panic!("Mars mu must be valid");
        };
        mu
    }

    fn circular_lmo() -> ClassicalElements {
        let axis_m = MARS_RADIUS_M + LMO_ALTITUDE_M;
        let Ok(elements) = ClassicalElements::new(
            Meters::new(axis_m),
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            Seconds::new(0.0),
        ) else {
            panic!("circular LMO elements must be valid");
        };
        elements
    }

    fn test_center() -> GravitationalCenter {
        let Ok(center) = GravitationalCenter::new(
            "mars",
            test_mu(),
            Kilograms::new(MARS_MASS_KG),
            Meters::new(MARS_RADIUS_M),
            DVec3::ZERO,
        ) else {
            panic!("test center must be valid");
        };
        center
    }

    // T1: circular LMO period, speed, and one-period closure.
    #[test]
    fn t1_circular_lmo_period_speed_closure() {
        let mu = test_mu();
        let elements = circular_lmo();
        let Ok(period) = orbital_period(mu, elements.semi_major_axis) else {
            panic!("period must succeed");
        };
        assert!(
            libm::fabs(period.value() - EXPECTED_PERIOD_S) < PERIOD_TOL_S,
            "period must match LMO reference"
        );
        let Ok(state) = propagate(mu, &elements, Seconds::new(0.0)) else {
            panic!("epoch propagate must succeed");
        };
        let speed_mps = libm::sqrt(
            state.velocity_mps.x * state.velocity_mps.x
                + state.velocity_mps.y * state.velocity_mps.y
                + state.velocity_mps.z * state.velocity_mps.z,
        );
        assert!(
            libm::fabs(speed_mps - EXPECTED_SPEED_MPS) < SPEED_TOL_MPS,
            "circular speed must match LMO reference"
        );
        let Ok(closed) = propagate(mu, &elements, period) else {
            panic!("period propagate must succeed");
        };
        let offset_m = closed.position_m - state.position_m;
        let distance_squared = offset_m.length_squared();
        assert!(
            distance_squared < CLOSURE_TOL_M * CLOSURE_TOL_M,
            "one-period closure must hold"
        );
    }

    // T2: Kepler reference value.
    #[test]
    fn t2_solve_kepler_reference() {
        let Ok(value_rad) = solve_kepler(T2_MEAN_RAD, T2_ECC) else {
            panic!("reference solve must succeed");
        };
        assert!(
            libm::fabs(value_rad - T2_EXPECTED_RAD) < T2_TOL_RAD,
            "solve_kepler(1.0, 0.5) must match reference"
        );
    }

    fn energy_of_state_j_per_kg(mu_m3_s2: f64, state: &PropagatedState) -> f64 {
        let radius_m = libm::sqrt(
            state.position_m.x * state.position_m.x
                + state.position_m.y * state.position_m.y
                + state.position_m.z * state.position_m.z,
        );
        state.velocity_mps.length_squared() / 2.0 - mu_m3_s2 / radius_m
    }

    fn momentum_of_state_m2_s(state: &PropagatedState) -> f64 {
        let momentum_vec = state.position_m.cross(state.velocity_mps);
        libm::sqrt(
            momentum_vec.x * momentum_vec.x
                + momentum_vec.y * momentum_vec.y
                + momentum_vec.z * momentum_vec.z,
        )
    }

    fn check_invariants(
        mu: Mu,
        elements: &ClassicalElements,
        expected_energy_j_per_kg: f64,
        expected_momentum_m2_s: f64,
    ) {
        let Ok(period) = orbital_period(mu, elements.semi_major_axis) else {
            panic!("period must succeed");
        };
        let quarter_s = period.value() / QUARTER_PERIOD_DIVISOR;
        for step_f64 in [0.0, 1.0, 2.0, 3.0, 4.0] {
            let target = Seconds::new(elements.epoch.value() + step_f64 * quarter_s);
            let Ok(state) = propagate(mu, elements, target) else {
                panic!("propagate must succeed");
            };
            let energy_j_per_kg = energy_of_state_j_per_kg(mu.value(), &state);
            assert!(
                libm::fabs(energy_j_per_kg - expected_energy_j_per_kg) < ENERGY_TOL_J_PER_KG,
                "energy must hold around orbit"
            );
            let momentum_m2_s = momentum_of_state_m2_s(&state);
            assert!(
                libm::fabs(momentum_m2_s - expected_momentum_m2_s) < MOMENTUM_TOL_M2_S,
                "momentum must hold around orbit"
            );
        }
    }

    // G1: circular energy (-mu/2a) and momentum (sqrt(mu a)) invariants.
    #[test]
    fn energy_and_momentum_circular() {
        let mu = test_mu();
        let circular = circular_lmo();
        let axis_m = MARS_RADIUS_M + LMO_ALTITUDE_M;
        let Ok(expected_energy_j_per_kg) = specific_energy(mu, Meters::new(axis_m)) else {
            panic!("circular energy must succeed");
        };
        let Ok(expected_momentum_m2_s) = specific_angular_momentum(mu, Meters::new(axis_m), 0.0)
        else {
            panic!("circular momentum must succeed");
        };
        assert!(
            libm::fabs(expected_energy_j_per_kg - (-MARS_MU_M3_S2 / (2.0 * axis_m)))
                < ENERGY_TOL_J_PER_KG,
            "circular energy must equal -mu/2a"
        );
        assert!(
            libm::fabs(expected_momentum_m2_s - libm::sqrt(MARS_MU_M3_S2 * axis_m))
                < MOMENTUM_TOL_M2_S,
            "circular momentum must equal sqrt(mu a)"
        );
        check_invariants(
            mu,
            &circular,
            expected_energy_j_per_kg,
            expected_momentum_m2_s,
        );
    }

    // G1: eccentric energy (-mu/2a) and momentum (sqrt(mu p)) invariants.
    #[test]
    fn energy_and_momentum_eccentric() {
        let mu = test_mu();
        let axis_m = MARS_RADIUS_M + LMO_ALTITUDE_M;
        let Ok(eccentric) = ClassicalElements::new(
            Meters::new(axis_m),
            G1_ECC,
            0.0,
            0.0,
            0.0,
            0.0,
            Seconds::new(0.0),
        ) else {
            panic!("eccentric elements must be valid");
        };
        let Ok(expected_energy_j_per_kg) = specific_energy(mu, Meters::new(axis_m)) else {
            panic!("eccentric energy must succeed");
        };
        let semi_latus_m = axis_m * (1.0 - G1_ECC * G1_ECC);
        let Ok(expected_momentum_m2_s) = specific_angular_momentum(mu, Meters::new(axis_m), G1_ECC)
        else {
            panic!("eccentric momentum must succeed");
        };
        assert!(
            libm::fabs(expected_momentum_m2_s - libm::sqrt(MARS_MU_M3_S2 * semi_latus_m))
                < MOMENTUM_TOL_M2_S,
            "eccentric momentum must equal sqrt(mu p)"
        );
        check_invariants(
            mu,
            &eccentric,
            expected_energy_j_per_kg,
            expected_momentum_m2_s,
        );
        assert!(matches!(
            specific_energy(mu, Meters::new(0.0)),
            Err(OrbitError::InvalidSemiMajorAxis { .. })
        ));
        assert!(matches!(
            specific_angular_momentum(mu, Meters::new(axis_m), 1.0),
            Err(OrbitError::InvalidEccentricity { .. })
        ));
    }

    // T3: error cases for orbit inputs.
    #[test]
    fn t3_orbit_error_cases() {
        assert!(matches!(Mu::new(0.0), Err(OrbitError::InvalidMu { .. })));
        assert!(matches!(
            Mu::new(f64::NAN),
            Err(OrbitError::InvalidMu { .. })
        ));
        assert!(matches!(
            Mu::new(f64::INFINITY),
            Err(OrbitError::InvalidMu { .. })
        ));
        assert!(matches!(
            solve_kepler(f64::NAN, 0.1),
            Err(OrbitError::InvalidMeanAnomaly { .. })
        ));
        assert!(matches!(
            solve_kepler(1.0, -0.1),
            Err(OrbitError::InvalidEccentricity { .. })
        ));
        assert!(matches!(
            solve_kepler(1.0, 1.0),
            Err(OrbitError::InvalidEccentricity { .. })
        ));
        assert!(matches!(
            solve_kepler(1.0, f64::NAN),
            Err(OrbitError::InvalidEccentricity { .. })
        ));
        assert!(matches!(
            mean_motion(test_mu(), Meters::new(0.0)),
            Err(OrbitError::InvalidSemiMajorAxis { .. })
        ));
        assert!(matches!(
            mean_motion(test_mu(), Meters::new(f64::NAN)),
            Err(OrbitError::InvalidSemiMajorAxis { .. })
        ));
        assert!(matches!(
            ClassicalElements::new(
                Meters::new(-5.0),
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                Seconds::new(0.0)
            ),
            Err(OrbitError::InvalidSemiMajorAxis { .. })
        ));
        assert!(matches!(
            ClassicalElements::new(
                Meters::new(7_000_000.0),
                1.5,
                0.0,
                0.0,
                0.0,
                0.0,
                Seconds::new(0.0)
            ),
            Err(OrbitError::InvalidEccentricity { .. })
        ));
        assert!(matches!(
            select_center(DVec3::new(0.0, 0.0, 0.0), &[]),
            Err(OrbitError::EmptyCenters)
        ));
        assert!(matches!(
            select_center(DVec3::new(f64::NAN, 0.0, 0.0), &[test_center()]),
            Err(OrbitError::InvalidPosition)
        ));
        assert!(matches!(
            sphere_of_influence(
                Kilograms::new(0.0),
                Kilograms::new(MARS_MASS_KG),
                Meters::new(MARS_AXIS_M)
            ),
            Err(OrbitError::InvalidMass { .. })
        ));
    }

    // T5: Mars sphere of influence.
    #[test]
    fn t5_mars_sphere_of_influence() {
        let Ok(radius) = sphere_of_influence(
            Kilograms::new(SUN_MASS_KG),
            Kilograms::new(MARS_MASS_KG),
            Meters::new(MARS_AXIS_M),
        ) else {
            panic!("SOI must succeed");
        };
        assert!(
            libm::fabs(radius.value() - EXPECTED_SOI_M) < SOI_TOL_M,
            "Mars SOI must match reference"
        );
    }

    // High-eccentricity solve still converges below the cap.
    #[test]
    fn high_eccentricity_converges() {
        let Ok(value_rad) = solve_kepler(0.5, 0.9) else {
            panic!("high-e solve must converge");
        };
        let residual = value_rad - 0.9 * libm::sin(value_rad) - 0.5;
        assert!(libm::fabs(residual) < 1e-9, "high-e residual must be small");
    }

    // Patched-conics selection prefers the nearer dominant center.
    #[test]
    fn select_center_prefers_dominant() {
        let Ok(near) = GravitationalCenter::new(
            "phobos",
            test_mu(),
            Kilograms::new(1.0e16),
            Meters::new(11_000.0),
            DVec3::new(9_376_000.0, 0.0, 0.0),
        ) else {
            panic!("near center must be valid");
        };
        let Ok(far) = GravitationalCenter::new(
            "mars",
            test_mu(),
            Kilograms::new(MARS_MASS_KG),
            Meters::new(MARS_RADIUS_M),
            DVec3::ZERO,
        ) else {
            panic!("far center must be valid");
        };
        let centers = [near, far];
        let Ok(index_usize) = select_center(DVec3::new(9_376_100.0, 0.0, 0.0), &centers) else {
            panic!("selection must succeed");
        };
        assert_eq!(index_usize, 0);
    }
}
