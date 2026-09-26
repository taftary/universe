//! Surface contact: altitude, surface-relative velocity, touchdown.
//!
//! Pure functions of [`crate::trajectory::StateVector`] plus body shape
//! and spin. No atmosphere sampling happens here; density and heating
//! belong to [`crate::trajectory`]. Regime labels live in
//! [`crate::regime`].
//!
//! Only `libm` transcendentals appear here, matching the engine
//! determinism rule; vector helpers are pure IEEE arithmetic.

use glam::DVec3;
use thiserror::Error;

use crate::body::BodyParams;
use crate::trajectory::StateVector;
use crate::units::{Meters, MetersPerSecond, Seconds};

/// Touchdown altitude tolerance in meters.
///
/// Contact counts when altitude sits at or below this height.
/// Source: issue 4 step 3 handoff (architect preset).
pub const TOUCHDOWN_ALTITUDE_TOLERANCE_M: f64 = 0.5;

/// Touchdown velocity tolerance in meters per second.
///
/// Contact counts when surface-relative speed sits at or below this
/// value. Source: issue 4 step 3 handoff (architect preset).
pub const TOUCHDOWN_VELOCITY_TOLERANCE_MPS: f64 = 5.0;

/// Full circle in radians for spin-rate computation.
///
/// Source: `core::f64::consts::TAU`.
const TAU_RAD: f64 = core::f64::consts::TAU;

/// Surface contact and configuration failures.
#[derive(Debug, Error)]
pub enum SurfaceError {
    /// Input or computed value was not finite.
    #[error("non-finite value: {value_f64}")]
    NonFinite {
        /// Rejected value.
        value_f64: f64,
    },
    /// Position at the body center has no surface direction.
    #[error("position at the body center has no surface direction")]
    AtCenter,
    /// Altitude tolerance was negative or not finite.
    #[error("invalid altitude tolerance: {tolerance_m_f64} m")]
    InvalidAltitudeTolerance {
        /// Rejected tolerance in meters.
        tolerance_m_f64: f64,
    },
    /// Velocity tolerance was negative or not finite.
    #[error("invalid velocity tolerance: {tolerance_mps_f64} m/s")]
    InvalidVelocityTolerance {
        /// Rejected tolerance in meters per second.
        tolerance_mps_f64: f64,
    },
}

/// Touchdown detection thresholds.
///
/// Pairs an altitude band with a surface-relative speed cap; see
/// [`SurfaceFrame::is_touchdown`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TouchdownConfig {
    /// Altitude tolerance in meters.
    altitude_tolerance_m: Meters,
    /// Velocity tolerance in meters per second.
    velocity_tolerance_mps: MetersPerSecond,
}

impl TouchdownConfig {
    /// Create touchdown thresholds from unit-typed values.
    ///
    /// # Errors
    ///
    /// Returns [`SurfaceError::InvalidAltitudeTolerance`] or
    /// [`SurfaceError::InvalidVelocityTolerance`] when a tolerance is
    /// negative or not finite.
    pub fn new(
        altitude_tolerance_m: Meters,
        velocity_tolerance_mps: MetersPerSecond,
    ) -> Result<Self, SurfaceError> {
        let altitude_f64 = altitude_tolerance_m.value();
        if !altitude_f64.is_finite() || altitude_f64 < 0.0 {
            return Err(SurfaceError::InvalidAltitudeTolerance {
                tolerance_m_f64: altitude_f64,
            });
        }
        let velocity_f64 = velocity_tolerance_mps.value();
        if !velocity_f64.is_finite() || velocity_f64 < 0.0 {
            return Err(SurfaceError::InvalidVelocityTolerance {
                tolerance_mps_f64: velocity_f64,
            });
        }
        Ok(Self {
            altitude_tolerance_m,
            velocity_tolerance_mps,
        })
    }

    /// Architect preset for tests and tools.
    ///
    /// Uses [`TOUCHDOWN_ALTITUDE_TOLERANCE_M`] and
    /// [`TOUCHDOWN_VELOCITY_TOLERANCE_MPS`]; valid by construction.
    #[must_use]
    pub fn preset() -> Self {
        Self {
            altitude_tolerance_m: Meters::new(TOUCHDOWN_ALTITUDE_TOLERANCE_M),
            velocity_tolerance_mps: MetersPerSecond::new(TOUCHDOWN_VELOCITY_TOLERANCE_MPS),
        }
    }

    /// Altitude tolerance in meters.
    #[must_use]
    pub const fn altitude_tolerance_m(&self) -> Meters {
        self.altitude_tolerance_m
    }

    /// Velocity tolerance in meters per second.
    #[must_use]
    pub const fn velocity_tolerance_mps(&self) -> MetersPerSecond {
        self.velocity_tolerance_mps
    }
}

/// Body surface frame derived from shape and spin.
///
/// Holds the radius and rotation period copied from [`BodyParams`], so
/// every method below is a pure function of ship state plus body.
/// The frame spins about the body z-axis, matching [`crate::trajectory`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceFrame {
    /// Body mean radius in meters.
    radius_m: Meters,
    /// Body rotation period in seconds.
    rotation_period_s: Seconds,
}

impl SurfaceFrame {
    /// Build a surface frame from body parameters.
    ///
    /// Infallible: [`BodyParams`] is valid by construction.
    #[must_use]
    pub fn new(body: &BodyParams) -> Self {
        Self {
            radius_m: body.radius_m(),
            rotation_period_s: body.rotation_period_s(),
        }
    }

    /// Body mean radius in meters.
    #[must_use]
    pub const fn radius_m(&self) -> Meters {
        self.radius_m
    }

    /// Body rotation period in seconds.
    #[must_use]
    pub const fn rotation_period_s(&self) -> Seconds {
        self.rotation_period_s
    }

    /// Altitude above the surface in meters.
    ///
    /// Reports `|r| - R`, negative when penetrating. Pure geometry;
    /// never samples the atmosphere.
    ///
    /// # Errors
    ///
    /// Returns [`SurfaceError::NonFinite`] for bad inputs and
    /// [`SurfaceError::AtCenter`] at the origin.
    pub fn altitude(&self, state: &StateVector) -> Result<Meters, SurfaceError> {
        let radius_m = checked_radius_m(state.position_m)?;
        Ok(Meters::new(radius_m - self.radius_m.value()))
    }

    /// Velocity relative to the corotating surface.
    ///
    /// Subtracts the spin velocity `omega x r` from inertial velocity
    /// and returns the remainder in meters per second. A ship parked
    /// on the equator reports near zero, not orbital speed.
    ///
    /// # Errors
    ///
    /// Returns [`SurfaceError::NonFinite`] for bad inputs.
    pub fn surface_relative_velocity(&self, state: &StateVector) -> Result<DVec3, SurfaceError> {
        checked_vector_m(state.position_m)?;
        checked_vector_m(state.velocity_mps)?;
        let corotation_mps = self.corotation_mps(state.position_m)?;
        let relative_mps = state.velocity_mps - corotation_mps;
        checked_vector_m(relative_mps)
    }

    /// Report whether the state counts as touched down.
    ///
    /// True when altitude sits at or below tolerance and
    /// surface-relative speed sits at or below tolerance. Slight
    /// penetration still counts as contact so a landing step that
    /// crosses zero in one tick settles instead of erroring.
    ///
    /// # Errors
    ///
    /// Returns [`SurfaceError`] for bad states.
    pub fn is_touchdown(
        &self,
        state: &StateVector,
        config: &TouchdownConfig,
    ) -> Result<bool, SurfaceError> {
        let altitude_m = self.altitude(state)?.value();
        let relative_mps = self.surface_relative_velocity(state)?;
        let speed_mps = libm::sqrt(relative_mps.length_squared());
        if !speed_mps.is_finite() {
            return Err(SurfaceError::NonFinite {
                value_f64: speed_mps,
            });
        }
        Ok(altitude_m <= config.altitude_tolerance_m.value()
            && speed_mps <= config.velocity_tolerance_mps.value())
    }

    /// Report whether the state has left the surface.
    ///
    /// True when altitude rises above tolerance. This is the touchdown
    /// release: a fast rover at zero altitude is neither touched down
    /// (too fast) nor lifted off (still low), and stays a surface
    /// concern until it climbs.
    ///
    /// # Errors
    ///
    /// Returns [`SurfaceError`] for bad states.
    pub fn is_liftoff(
        &self,
        state: &StateVector,
        config: &TouchdownConfig,
    ) -> Result<bool, SurfaceError> {
        let altitude_m = self.altitude(state)?.value();
        Ok(altitude_m > config.altitude_tolerance_m.value())
    }

    /// Build the corotating rest state under the ship.
    ///
    /// Projects the position onto the surface along its radius and sets
    /// velocity to the local corotation velocity. Epoch is preserved.
    /// Consumes a penetrating end-of-step state after touchdown.
    ///
    /// # Errors
    ///
    /// Returns [`SurfaceError::NonFinite`] for bad inputs and
    /// [`SurfaceError::AtCenter`] at the origin.
    pub fn rest_state(&self, state: &StateVector) -> Result<StateVector, SurfaceError> {
        checked_vector_m(state.position_m)?;
        checked_vector_m(state.velocity_mps)?;
        if !state.epoch.value().is_finite() {
            return Err(SurfaceError::NonFinite {
                value_f64: state.epoch.value(),
            });
        }
        let radius_m = checked_radius_m(state.position_m)?;
        let radius_unit = state.position_m / radius_m;
        let surface_position_m = radius_unit * self.radius_m.value();
        let corotation_mps = self.corotation_mps(surface_position_m)?;
        checked_vector_m(surface_position_m)?;
        Ok(StateVector {
            position_m: surface_position_m,
            velocity_mps: corotation_mps,
            epoch: state.epoch,
        })
    }

    /// Spin velocity `omega x r` in meters per second.
    ///
    /// Spin runs about the body z-axis with rate `TAU / period`.
    fn corotation_mps(&self, position_m: DVec3) -> Result<DVec3, SurfaceError> {
        let spin_rad_s = TAU_RAD / self.rotation_period_s.value();
        if !spin_rad_s.is_finite() {
            return Err(SurfaceError::NonFinite {
                value_f64: spin_rad_s,
            });
        }
        let corotation_mps = DVec3::new(-spin_rad_s * position_m.y, spin_rad_s * position_m.x, 0.0);
        checked_vector_m(corotation_mps)
    }
}

/// Reject a vector with a non-finite component.
fn checked_vector_m(vector_m: DVec3) -> Result<DVec3, SurfaceError> {
    if vector_m.x.is_finite() && vector_m.y.is_finite() && vector_m.z.is_finite() {
        Ok(vector_m)
    } else {
        Err(SurfaceError::NonFinite {
            value_f64: vector_m.x + vector_m.y + vector_m.z,
        })
    }
}

/// Radius in meters, rejecting non-finite values and the origin.
fn checked_radius_m(position_m: DVec3) -> Result<f64, SurfaceError> {
    checked_vector_m(position_m)?;
    let radius_m = libm::sqrt(position_m.length_squared());
    if !radius_m.is_finite() {
        return Err(SurfaceError::NonFinite {
            value_f64: radius_m,
        });
    }
    if radius_m <= 0.0 {
        return Err(SurfaceError::AtCenter);
    }
    Ok(radius_m)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::Seconds;

    /// Mars mean radius in meters.
    const MARS_RADIUS_M: f64 = 3_389_500.0;
    /// Corotation check tolerance in meters per second.
    const COROTATION_TOL_MPS: f64 = 1e-6;
    /// Altitude check tolerance in meters.
    const ALTITUDE_TOL_M: f64 = 1e-6;

    fn test_frame() -> SurfaceFrame {
        SurfaceFrame::new(&BodyParams::mars_like())
    }

    fn state_at_altitude_mps(altitude_m: f64, velocity_mps: DVec3) -> StateVector {
        let Ok(state) = StateVector::new(
            DVec3::new(MARS_RADIUS_M + altitude_m, 0.0, 0.0),
            velocity_mps,
            Seconds::new(42.0),
        ) else {
            panic!("surface test state must validate");
        };
        state
    }

    #[test]
    fn preset_matches_architect_values() {
        let config = TouchdownConfig::preset();
        assert!(libm::fabs(config.altitude_tolerance_m().value() - 0.5) < 1e-12);
        assert!(libm::fabs(config.velocity_tolerance_mps().value() - 5.0) < 1e-12);
        assert!(libm::fabs(TOUCHDOWN_ALTITUDE_TOLERANCE_M - 0.5) < 1e-12);
        assert!(libm::fabs(TOUCHDOWN_VELOCITY_TOLERANCE_MPS - 5.0) < 1e-12);
    }

    #[test]
    fn constructor_rejects_bad_tolerances() {
        let Ok(altitude_m) = TouchdownConfig::new(Meters::new(0.5), MetersPerSecond::new(5.0))
        else {
            panic!("valid config must pass");
        };
        assert!(libm::fabs(altitude_m.altitude_tolerance_m().value() - 0.5) < 1e-12);
        assert!(
            TouchdownConfig::new(Meters::new(-0.1), MetersPerSecond::new(5.0)).is_err(),
            "negative altitude tolerance must fail"
        );
        assert!(
            TouchdownConfig::new(Meters::new(0.5), MetersPerSecond::new(f64::NAN)).is_err(),
            "NaN velocity tolerance must fail"
        );
    }

    #[test]
    fn altitude_tracks_radius_offset() {
        let frame = test_frame();
        for altitude_f64 in [0.0, 0.25, 1_000.0, 120_000.0] {
            let state = state_at_altitude_mps(altitude_f64, DVec3::ZERO);
            let Ok(altitude_m) = frame.altitude(&state) else {
                panic!("altitude must sample");
            };
            assert!(
                libm::fabs(altitude_m.value() - altitude_f64) < ALTITUDE_TOL_M,
                "altitude must equal radius offset"
            );
        }
        let sunk = state_at_altitude_mps(-2.0, DVec3::ZERO);
        let Ok(sunk_altitude_m) = frame.altitude(&sunk) else {
            panic!("penetrating altitude must sample");
        };
        assert!(sunk_altitude_m.value() < 0.0);
    }

    #[test]
    fn parked_ship_reports_near_zero_relative_velocity() {
        let frame = test_frame();
        let spin_rad_s = TAU_RAD / BodyParams::mars_like().rotation_period_s().value();
        let radius_m = MARS_RADIUS_M;
        let parked = state_at_altitude_mps(0.0, DVec3::new(0.0, spin_rad_s * radius_m, 0.0));
        let Ok(relative_mps) = frame.surface_relative_velocity(&parked) else {
            panic!("relative velocity must sample");
        };
        assert!(
            libm::sqrt(relative_mps.length_squared()) < COROTATION_TOL_MPS,
            "parked ship must co-rotate with the surface"
        );
        let orbital = state_at_altitude_mps(250_000.0, DVec3::new(0.0, 3_430.0, 0.0));
        let Ok(fast_mps) = frame.surface_relative_velocity(&orbital) else {
            panic!("orbital relative velocity must sample");
        };
        let fast_speed_mps = libm::sqrt(fast_mps.length_squared());
        assert!(
            fast_speed_mps > 3_000.0,
            "orbital ship must keep inertial-scale relative speed"
        );
    }

    #[test]
    fn touchdown_and_liftoff_follow_config() {
        let frame = test_frame();
        let config = TouchdownConfig::preset();
        let spin_rad_s = TAU_RAD / BodyParams::mars_like().rotation_period_s().value();
        let corotation_mps = spin_rad_s * MARS_RADIUS_M;
        let gentle = state_at_altitude_mps(0.2, DVec3::new(0.0, corotation_mps + 3.0, 0.0));
        let Ok(touched_down) = frame.is_touchdown(&gentle, &config) else {
            panic!("touchdown check must run");
        };
        assert!(touched_down, "gentle contact must count");
        let Ok(aloft_after_gentle) = frame.is_liftoff(&gentle, &config) else {
            panic!("liftoff check must run");
        };
        assert!(!aloft_after_gentle, "contact must not read as liftoff");
        let penetrated = state_at_altitude_mps(-1.0, DVec3::new(0.0, corotation_mps + 1.0, 0.0));
        let Ok(touched_while_sunk) = frame.is_touchdown(&penetrated, &config) else {
            panic!("penetrating check must run");
        };
        assert!(touched_while_sunk, "slight penetration must settle");
        let speedy = state_at_altitude_mps(0.1, DVec3::new(0.0, corotation_mps + 50.0, 0.0));
        let Ok(touched_while_fast) = frame.is_touchdown(&speedy, &config) else {
            panic!("fast check must run");
        };
        assert!(!touched_while_fast, "fast surface pass must not count");
        let climbing = state_at_altitude_mps(10.0, DVec3::new(0.0, corotation_mps + 50.0, 0.0));
        let Ok(aloft) = frame.is_liftoff(&climbing, &config) else {
            panic!("climb check must run");
        };
        assert!(aloft, "climb above tolerance must read as liftoff");
        let Ok(still_touching) = frame.is_touchdown(&climbing, &config) else {
            panic!("climb touchdown check must run");
        };
        assert!(!still_touching, "climb must not read as touchdown");
    }

    #[test]
    fn rest_state_parks_on_the_surface() {
        let frame = test_frame();
        let incoming = state_at_altitude_mps(-1.5, DVec3::new(-20.0, 100.0, 5.0));
        let Ok(parked) = frame.rest_state(&incoming) else {
            panic!("rest state must build");
        };
        let Ok(parked_altitude_m) = frame.altitude(&parked) else {
            panic!("parked altitude must sample");
        };
        assert!(libm::fabs(parked_altitude_m.value()) < ALTITUDE_TOL_M);
        let Ok(relative_mps) = frame.surface_relative_velocity(&parked) else {
            panic!("parked relative velocity must sample");
        };
        assert!(
            libm::sqrt(relative_mps.length_squared()) < COROTATION_TOL_MPS,
            "rest state must co-rotate"
        );
        assert_eq!(parked.epoch, incoming.epoch);
    }

    // AC3: penetrating descent end settles via rest_state into touchdown.
    #[test]
    fn penetrating_descent_end_settles_into_touchdown() {
        let frame = test_frame();
        let config = TouchdownConfig::preset();
        let incoming = state_at_altitude_mps(-3.8, DVec3::new(-800.0, 900.0, 50.0));
        let Ok(altitude_m) = frame.altitude(&incoming) else {
            panic!("penetrating altitude must sample");
        };
        assert!(altitude_m.value() < 0.0);
        let Ok(parked) = frame.rest_state(&incoming) else {
            panic!("descent rest state must build");
        };
        let Ok(parked_altitude_m) = frame.altitude(&parked) else {
            panic!("parked altitude must sample");
        };
        assert!(libm::fabs(parked_altitude_m.value()) < ALTITUDE_TOL_M);
        let Ok(relative_mps) = frame.surface_relative_velocity(&parked) else {
            panic!("parked relative velocity must sample");
        };
        assert!(
            libm::sqrt(relative_mps.length_squared()) < config.velocity_tolerance_mps().value(),
            "settled touchdown must sit below 5 m/s"
        );
        let Ok(touched) = frame.is_touchdown(&parked, &config) else {
            panic!("touchdown check must run");
        };
        assert!(touched, "settled descent end must count as touchdown");
        let Ok(lifted) = frame.is_liftoff(&parked, &config) else {
            panic!("liftoff check must run");
        };
        assert!(!lifted, "settled touchdown must not read as liftoff");
        assert_eq!(parked.epoch, incoming.epoch);
    }

    #[test]
    fn center_and_non_finite_states_fail() {
        let frame = test_frame();
        let config = TouchdownConfig::preset();
        let Ok(at_center) = StateVector::new(DVec3::ZERO, DVec3::ZERO, Seconds::new(0.0)) else {
            panic!("center state must validate");
        };
        assert!(matches!(
            frame.altitude(&at_center),
            Err(SurfaceError::AtCenter)
        ));
        assert!(matches!(
            frame.rest_state(&at_center),
            Err(SurfaceError::AtCenter)
        ));
        let bad = StateVector {
            position_m: DVec3::new(f64::NAN, 0.0, 0.0),
            velocity_mps: DVec3::ZERO,
            epoch: Seconds::new(0.0),
        };
        assert!(matches!(
            frame.altitude(&bad),
            Err(SurfaceError::NonFinite { .. })
        ));
        assert!(matches!(
            frame.surface_relative_velocity(&bad),
            Err(SurfaceError::NonFinite { .. })
        ));
        assert!(matches!(
            frame.is_touchdown(&bad, &config),
            Err(SurfaceError::NonFinite { .. })
        ));
        assert!(matches!(
            frame.is_liftoff(&bad, &config),
            Err(SurfaceError::NonFinite { .. })
        ));
    }
}
