//! Time-warp policy on top of analytic orbits.
//!
//! Split choice: warp policy lives here, Kepler math lives in
//! [`crate::orbit`]. This file imports orbit error types and advances orbits
//! through [`crate::orbit`] functions, so warp determinism reduces to Kepler
//! determinism. All transcendentals stay in [`crate::orbit`] via `libm`.
//!
//! Prograde and retrograde burns are out of scope here; trajectory burns
//! live in issue 4 scope, not this issue.

use crate::orbit::OrbitError;
use crate::units::Seconds;

/// Maximum warp factor, dimensionless. Source: `docs/tech/simulation.md`.
pub const MAX_WARP_FACTOR: f64 = 10_000.0;

/// Minimum warp factor, dimensionless. Source: `docs/tech/simulation.md`.
pub const MIN_WARP_FACTOR: f64 = 1.0;

/// Tenfold warp step, dimensionless. Source: `docs/tech/simulation.md`.
pub const WARP_STEP_TEN: f64 = 10.0;

/// Hundredfold warp factor, dimensionless. Source: `docs/tech/simulation.md`.
pub const WARP_FACTOR_HUNDRED: f64 = 100.0;

/// Thousandfold warp factor, dimensionless. Source: `docs/tech/simulation.md`.
pub const WARP_FACTOR_THOUSAND: f64 = 1_000.0;

/// Playable time-warp factors, dimensionless.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Warp {
    /// Real time, always allowed.
    X1,
    /// Tenfold warp, inside a ship in orbit or transit.
    X10,
    /// Hundredfold warp, inside a ship in orbit or transit.
    X100,
    /// Thousandfold warp, inside a ship in orbit or transit.
    X1000,
    /// Ten-thousandfold warp, inside a ship in orbit or transit.
    X10000,
}

impl Warp {
    /// All warp factors from lowest to highest.
    pub const ALL: [Warp; 5] = [Warp::X1, Warp::X10, Warp::X100, Warp::X1000, Warp::X10000];

    /// Return the dimensionless multiplication factor.
    #[must_use]
    pub const fn factor(self) -> f64 {
        match self {
            Warp::X1 => MIN_WARP_FACTOR,
            Warp::X10 => WARP_STEP_TEN,
            Warp::X100 => WARP_FACTOR_HUNDRED,
            Warp::X1000 => WARP_FACTOR_THOUSAND,
            Warp::X10000 => MAX_WARP_FACTOR,
        }
    }
}

/// Conditions gating a warp request.
#[expect(
    clippy::struct_excessive_bools,
    reason = "five independent warp interlocks"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WarpContext {
    /// True when the player is inside a ship.
    pub in_ship: bool,
    /// True when the ship is in orbit or in transit.
    pub in_orbit_or_transit: bool,
    /// True when inside an atmosphere.
    pub in_atmosphere: bool,
    /// True when approaching any body or object.
    pub approaching: bool,
    /// True when any physiological alarm is raised.
    pub alarm: bool,
}

impl WarpContext {
    /// Create warp conditions from five flags.
    #[expect(
        clippy::fn_params_excessive_bools,
        reason = "five independent warp interlocks"
    )]
    #[must_use]
    pub const fn new(
        in_ship: bool,
        in_orbit_or_transit: bool,
        in_atmosphere: bool,
        approaching: bool,
        alarm: bool,
    ) -> Self {
        Self {
            in_ship,
            in_orbit_or_transit,
            in_atmosphere,
            approaching,
            alarm,
        }
    }

    /// Standard on-rails cruise context for tests and tools.
    #[must_use]
    pub const fn cruise() -> Self {
        Self {
            in_ship: true,
            in_orbit_or_transit: true,
            in_atmosphere: false,
            approaching: false,
            alarm: false,
        }
    }

    /// Standard on-foot context where only `1x` is allowed.
    #[must_use]
    pub const fn on_foot() -> Self {
        Self {
            in_ship: false,
            in_orbit_or_transit: false,
            in_atmosphere: false,
            approaching: false,
            alarm: false,
        }
    }

    /// Report whether warp must drop to `1x`.
    ///
    /// True on atmospheric entry, on approach to any body
    /// or object, and whenever any physiological alarm is raised.
    #[must_use]
    pub const fn should_auto_drop(self) -> bool {
        self.in_atmosphere || self.approaching || self.alarm
    }
}

/// Request a warp factor under the simulation rules.
///
/// `1x` is always allowed. Higher factors require a ship in orbit or
/// transit with no atmosphere, no approach, and no alarm.
///
/// # Errors
///
/// Returns [`OrbitError::WarpDenied`] when the rules reject the request.
pub fn request_warp(requested: Warp, ctx: WarpContext) -> Result<Warp, OrbitError> {
    if requested == Warp::X1 {
        return Ok(Warp::X1);
    }
    if !ctx.in_ship || !ctx.in_orbit_or_transit {
        return Err(OrbitError::WarpDenied {
            requested_factor_f64: requested.factor(),
        });
    }
    if ctx.in_atmosphere || ctx.approaching || ctx.alarm {
        return Err(OrbitError::WarpDenied {
            requested_factor_f64: requested.factor(),
        });
    }
    Ok(requested)
}

/// Scale the fixed tick by the warp factor.
///
/// Returns `SIM_TICK_S` times the factor, tying warp steps to D-012.
#[must_use]
pub fn tick_at_warp(warp: Warp) -> Seconds {
    Seconds::new(crate::sim::SIM_TICK_S.value() * warp.factor())
}

/// Force warp to `1x` when auto-drop conditions hold.
///
/// Takes the requested warp and context and returns `X1` on
/// atmospheric entry, approach, or alarm, else the request.
#[must_use]
pub const fn apply_auto_drop(requested: Warp, ctx: WarpContext) -> Warp {
    if ctx.should_auto_drop() {
        Warp::X1
    } else {
        requested
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orbit::{ClassicalElements, Mu, advance, propagate};
    use crate::units::Meters;

    const MARS_MU_M3_S2: f64 = 4.282_837e13;
    const MARS_AXIS_M: f64 = 3_639_500.0;

    fn test_mu() -> Mu {
        let Ok(mu) = Mu::new(MARS_MU_M3_S2) else {
            panic!("Mars mu must be valid");
        };
        mu
    }

    fn test_elements() -> ClassicalElements {
        let Ok(elements) = ClassicalElements::new(
            Meters::new(MARS_AXIS_M),
            0.2,
            0.3,
            0.7,
            0.5,
            1.0,
            Seconds::new(100.0),
        ) else {
            panic!("elements must be valid");
        };
        elements
    }

    // T4: warp determinism, advance equals propagate bit-for-bit.
    #[test]
    fn t4_advance_equals_propagate() {
        let mu = test_mu();
        let elements = test_elements();
        let delta = Seconds::new(1_234.5);
        let Ok(via_advance) = advance(mu, &elements, delta) else {
            panic!("advance must succeed");
        };
        let target = Seconds::new(elements.epoch.value() + delta.value());
        let Ok(via_propagate) = propagate(mu, &elements, target) else {
            panic!("propagate must succeed");
        };
        assert_eq!(via_advance, via_propagate);
        // Warp-scaled step stays on the same path.
        let step = tick_at_warp(Warp::X100);
        let long_delta = Seconds::new(step.value() * 10.0);
        let long_target = Seconds::new(elements.epoch.value() + long_delta.value());
        let Ok(via_long_advance) = advance(mu, &elements, long_delta) else {
            panic!("long advance must succeed");
        };
        let Ok(via_long_propagate) = propagate(mu, &elements, long_target) else {
            panic!("long propagate must succeed");
        };
        assert_eq!(via_long_advance, via_long_propagate);
    }

    // Warp allow and deny cases.
    #[test]
    fn warp_request_allow_and_deny() {
        assert!(matches!(
            request_warp(Warp::X1, WarpContext::on_foot()),
            Ok(Warp::X1)
        ));
        assert!(matches!(
            request_warp(Warp::X100, WarpContext::cruise()),
            Ok(Warp::X100)
        ));
        assert!(matches!(
            request_warp(Warp::X10000, WarpContext::cruise()),
            Ok(Warp::X10000)
        ));
        assert!(matches!(
            request_warp(Warp::X10, WarpContext::on_foot()),
            Err(OrbitError::WarpDenied { .. })
        ));
        assert!(matches!(
            request_warp(
                Warp::X100,
                WarpContext::new(true, false, false, false, false)
            ),
            Err(OrbitError::WarpDenied { .. })
        ));
        assert!(matches!(
            request_warp(Warp::X100, WarpContext::new(true, true, true, false, false)),
            Err(OrbitError::WarpDenied { .. })
        ));
        assert!(matches!(
            request_warp(Warp::X100, WarpContext::new(true, true, false, true, false)),
            Err(OrbitError::WarpDenied { .. })
        ));
        assert!(matches!(
            request_warp(
                Warp::X1000,
                WarpContext::new(true, true, false, false, true)
            ),
            Err(OrbitError::WarpDenied { .. })
        ));
    }

    // Warp factors and tick scaling follow the locked constants.
    #[test]
    fn warp_factors_and_tick_scaling() {
        assert!(libm::fabs(Warp::X1.factor() - MIN_WARP_FACTOR) < 1e-12);
        assert!(libm::fabs(Warp::X10000.factor() - MAX_WARP_FACTOR) < 1e-12);
        assert_eq!(Warp::ALL.len(), 5);
        let base_s = crate::sim::SIM_TICK_S.value();
        assert!(libm::fabs(tick_at_warp(Warp::X1).value() - base_s) < 1e-12);
        assert!(libm::fabs(tick_at_warp(Warp::X10000).value() - base_s * MAX_WARP_FACTOR) < 1e-9);
    }

    // G2: X10000 drops to X1 on entry, approach, and alarm.
    #[test]
    fn x10000_drops_on_entry_approach_alarm() {
        assert!(!WarpContext::cruise().should_auto_drop());
        assert_eq!(
            apply_auto_drop(Warp::X10000, WarpContext::cruise()),
            Warp::X10000
        );
        let entry = WarpContext::new(true, true, true, false, false);
        assert!(entry.should_auto_drop());
        assert_eq!(apply_auto_drop(Warp::X10000, entry), Warp::X1);
        let approach = WarpContext::new(true, true, false, true, false);
        assert!(approach.should_auto_drop());
        assert_eq!(apply_auto_drop(Warp::X10000, approach), Warp::X1);
        let alarm = WarpContext::new(true, true, false, false, true);
        assert!(alarm.should_auto_drop());
        assert_eq!(apply_auto_drop(Warp::X10000, alarm), Warp::X1);
        let combined = WarpContext::new(true, true, true, true, true);
        assert!(combined.should_auto_drop());
        assert_eq!(apply_auto_drop(Warp::X10000, combined), Warp::X1);
        assert!(!WarpContext::on_foot().should_auto_drop());
    }
}
