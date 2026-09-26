//! Flight regimes: orbit, atmosphere, and surface handoffs.
//!
//! Classifies altitude into the three M1 regimes, gates direct
//! transitions, measures signed distance to handoff boundaries, and maps
//! regimes onto [`WarpContext`] policy. Dynamics live in
//! [`crate::trajectory`]; contact detail lives in [`crate::surface`].
//!
//! Only `libm` math appears here, matching the engine determinism rule.

use thiserror::Error;

use crate::body::BodyParams;
use crate::trajectory::{RAILS_ALTITUDE_M, StateVector};
use crate::units::Meters;
use crate::warp::WarpContext;

/// Surface boundary altitude in meters.
///
/// Classification treats zero and below as surface contact. Source:
/// issue 4 step 3 handoff (architect boundary).
pub const SURFACE_BOUNDARY_ALTITUDE_M: f64 = 0.0;

/// Flight regime for warp policy and integrator choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Regime {
    /// Above the rails cutoff; analytic coast, warp allowed.
    Orbit,
    /// Between the surface and the cutoff; integration, no warp.
    Atmosphere,
    /// At or below the surface; contact logic, no warp.
    Surface,
}

/// Handoff boundary for distance measurement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Boundary {
    /// Orbit-atmosphere cutoff at [`RAILS_ALTITUDE_M`].
    Rails,
    /// Atmosphere-surface contact at zero altitude.
    Surface,
}

impl Boundary {
    /// Boundary altitude in meters.
    #[must_use]
    pub const fn altitude_m(self) -> Meters {
        match self {
            Boundary::Rails => Meters::new(RAILS_ALTITUDE_M),
            Boundary::Surface => Meters::new(SURFACE_BOUNDARY_ALTITUDE_M),
        }
    }
}

/// Regime classification and transition failures.
#[derive(Debug, Error, Clone, Copy, PartialEq)]
pub enum RegimeError {
    /// Altitude was not finite.
    #[error("non-finite altitude: {value_f64} m")]
    NonFinite {
        /// Rejected altitude in meters.
        value_f64: f64,
    },
    /// Direct transition skips the middle regime.
    #[error("invalid transition from {from:?} to {to:?}")]
    InvalidTransition {
        /// Starting regime.
        from: Regime,
        /// Requested regime.
        to: Regime,
    },
}

/// Classify altitude into a flight regime.
///
/// Above [`RAILS_ALTITUDE_M`] is [`Regime::Orbit`], positive altitudes
/// at or below it are [`Regime::Atmosphere`], and zero or below is
/// [`Regime::Surface`]. Matches the trajectory integrator choice.
///
/// # Errors
///
/// Returns [`RegimeError::NonFinite`] for non-finite altitudes.
pub fn classify(altitude: Meters) -> Result<Regime, RegimeError> {
    let altitude_f64 = altitude.value();
    if !altitude_f64.is_finite() {
        return Err(RegimeError::NonFinite {
            value_f64: altitude_f64,
        });
    }
    if altitude_f64 > RAILS_ALTITUDE_M {
        Ok(Regime::Orbit)
    } else if altitude_f64 > SURFACE_BOUNDARY_ALTITUDE_M {
        Ok(Regime::Atmosphere)
    } else {
        Ok(Regime::Surface)
    }
}

/// Classify a ship state with body radius.
///
/// Derives altitude as `|r| - R` without sampling the atmosphere, so
/// this stays consistent with [`crate::surface`] geometry.
///
/// # Errors
///
/// Returns [`RegimeError::NonFinite`] for non-finite positions.
pub fn classify_state(state: &StateVector, body: &BodyParams) -> Result<Regime, RegimeError> {
    if !state.position_m.x.is_finite()
        || !state.position_m.y.is_finite()
        || !state.position_m.z.is_finite()
    {
        return Err(RegimeError::NonFinite {
            value_f64: state.position_m.x + state.position_m.y + state.position_m.z,
        });
    }
    let radius_m = libm::sqrt(state.position_m.length_squared());
    if !radius_m.is_finite() {
        return Err(RegimeError::NonFinite {
            value_f64: radius_m,
        });
    }
    classify(Meters::new(radius_m - body.radius_m().value()))
}

/// Gate a regime handoff between ticks.
///
/// Same-regime and adjacent-regime moves are allowed. A direct
/// [`Regime::Orbit`] to [`Regime::Surface`] jump (or the reverse)
/// skips the atmosphere in one tick, which exceeds the per-tick travel
/// bound, so it is rejected.
///
/// # Errors
///
/// Returns [`RegimeError::InvalidTransition`] for a direct
/// orbit-surface skip in either direction.
pub fn transition(from: Regime, to: Regime) -> Result<Regime, RegimeError> {
    match (from, to) {
        (Regime::Orbit | Regime::Atmosphere, Regime::Orbit)
        | (Regime::Atmosphere | Regime::Orbit | Regime::Surface, Regime::Atmosphere)
        | (Regime::Surface | Regime::Atmosphere, Regime::Surface) => Ok(to),
        (Regime::Orbit | Regime::Surface, Regime::Orbit | Regime::Surface) => {
            Err(RegimeError::InvalidTransition { from, to })
        }
    }
}

/// Signed distance from an altitude to a boundary.
///
/// Returns `altitude - boundary` in meters: positive above the
/// boundary, negative below, zero exactly on it. Debug overlays use
/// this for the distance-to-handoff readout.
#[must_use]
pub fn distance(altitude: Meters, boundary: Boundary) -> Meters {
    Meters::new(altitude.value() - boundary.altitude_m().value())
}

/// Build warp policy for a regime.
///
/// Maps orbit to cruise (warp eligible), atmosphere to entry (warp
/// drops via the atmosphere flag), and surface to grounded (warp
/// denied since the ship is not in transit). Ship presence, approach,
/// and alarm pass through; [`crate::warp::request_warp`] then
/// enforces the policy.
#[must_use]
pub fn warp_context_for_regime(
    regime: Regime,
    in_ship: bool,
    approaching: bool,
    alarm: bool,
) -> WarpContext {
    match regime {
        Regime::Orbit => WarpContext::new(in_ship, true, false, approaching, alarm),
        Regime::Atmosphere => WarpContext::new(in_ship, true, true, approaching, alarm),
        Regime::Surface => WarpContext::new(in_ship, false, false, approaching, alarm),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trajectory::StateVector;
    use crate::units::Seconds;
    use crate::warp::{Warp, apply_auto_drop, request_warp};
    use glam::DVec3;

    /// Mars mean radius in meters.
    const MARS_RADIUS_M: f64 = 3_389_500.0;
    /// Distance tolerance in meters.
    const DISTANCE_TOL_M: f64 = 1e-6;

    fn state_at_altitude(altitude_m: f64) -> StateVector {
        let Ok(state) = StateVector::new(
            DVec3::new(MARS_RADIUS_M + altitude_m, 0.0, 0.0),
            DVec3::new(0.0, 3_400.0, 0.0),
            Seconds::new(0.0),
        ) else {
            panic!("regime test state must validate");
        };
        state
    }

    #[test]
    fn classify_matches_architect_bands() {
        let Ok(orbit) = classify(Meters::new(RAILS_ALTITUDE_M + 1.0)) else {
            panic!("high altitude must classify");
        };
        assert_eq!(orbit, Regime::Orbit);
        let Ok(at_cutoff) = classify(Meters::new(RAILS_ALTITUDE_M)) else {
            panic!("cutoff altitude must classify");
        };
        assert_eq!(at_cutoff, Regime::Atmosphere);
        let Ok(mid) = classify(Meters::new(60_000.0)) else {
            panic!("mid altitude must classify");
        };
        assert_eq!(mid, Regime::Atmosphere);
        let Ok(just_above) = classify(Meters::new(0.5)) else {
            panic!("low altitude must classify");
        };
        assert_eq!(just_above, Regime::Atmosphere);
        let Ok(at_surface) = classify(Meters::new(SURFACE_BOUNDARY_ALTITUDE_M)) else {
            panic!("surface altitude must classify");
        };
        assert_eq!(at_surface, Regime::Surface);
        let Ok(sunk) = classify(Meters::new(-10.0)) else {
            panic!("negative altitude must classify");
        };
        assert_eq!(sunk, Regime::Surface);
        assert!(matches!(
            classify(Meters::new(f64::NAN)),
            Err(RegimeError::NonFinite { .. })
        ));
    }

    #[test]
    fn classify_state_agrees_with_altitude() {
        let body = BodyParams::mars_like();
        for (altitude_f64, expected) in [
            (200_000.0, Regime::Orbit),
            (50_000.0, Regime::Atmosphere),
            (0.0, Regime::Surface),
            (-5.0, Regime::Surface),
        ] {
            let Ok(regime) = classify_state(&state_at_altitude(altitude_f64), &body) else {
                panic!("state at {altitude_f64} must classify");
            };
            assert_eq!(regime, expected);
        }
        let bad = StateVector {
            position_m: DVec3::new(f64::INFINITY, 0.0, 0.0),
            velocity_mps: DVec3::ZERO,
            epoch: Seconds::new(0.0),
        };
        assert!(matches!(
            classify_state(&bad, &body),
            Err(RegimeError::NonFinite { .. })
        ));
    }

    #[test]
    fn transition_gates_orbit_surface_skips() {
        assert_eq!(transition(Regime::Orbit, Regime::Orbit), Ok(Regime::Orbit));
        assert_eq!(
            transition(Regime::Orbit, Regime::Atmosphere),
            Ok(Regime::Atmosphere)
        );
        assert_eq!(
            transition(Regime::Atmosphere, Regime::Surface),
            Ok(Regime::Surface)
        );
        assert_eq!(
            transition(Regime::Surface, Regime::Atmosphere),
            Ok(Regime::Atmosphere)
        );
        assert_eq!(
            transition(Regime::Atmosphere, Regime::Orbit),
            Ok(Regime::Orbit)
        );
        assert!(matches!(
            transition(Regime::Orbit, Regime::Surface),
            Err(RegimeError::InvalidTransition { .. })
        ));
        assert!(matches!(
            transition(Regime::Surface, Regime::Orbit),
            Err(RegimeError::InvalidTransition { .. })
        ));
    }

    #[test]
    fn distance_reports_signed_margins() {
        let above = distance(Meters::new(150_000.0), Boundary::Rails);
        assert!(libm::fabs(above.value() - 30_000.0) < DISTANCE_TOL_M);
        let below = distance(Meters::new(100_000.0), Boundary::Rails);
        assert!(libm::fabs(below.value() + 20_000.0) < DISTANCE_TOL_M);
        let height = distance(Meters::new(1_000.0), Boundary::Surface);
        assert!(libm::fabs(height.value() - 1_000.0) < DISTANCE_TOL_M);
        let sunk = distance(Meters::new(-3.0), Boundary::Surface);
        assert!(libm::fabs(sunk.value() + 3.0) < DISTANCE_TOL_M);
        assert!(
            libm::fabs(Boundary::Rails.altitude_m().value() - RAILS_ALTITUDE_M) < DISTANCE_TOL_M
        );
        assert!(
            libm::fabs(Boundary::Surface.altitude_m().value() - SURFACE_BOUNDARY_ALTITUDE_M)
                < DISTANCE_TOL_M
        );
    }

    // AC3: full descent/ascent regime sequence gates both crossings both ways.
    #[test]
    fn full_profile_regime_sequence_gates_both_crossings() {
        let body = BodyParams::mars_like();
        let legs = [
            (300_000.0, Regime::Orbit),
            (50_000.0, Regime::Atmosphere),
            (-5.0, Regime::Surface),
            (50_000.0, Regime::Atmosphere),
            (150_000.0, Regime::Orbit),
        ];
        let mut previous: Option<Regime> = None;
        for (altitude_m, expected) in legs {
            let Ok(regime) = classify_state(&state_at_altitude(altitude_m), &body) else {
                panic!("state at {altitude_m} must classify");
            };
            assert_eq!(regime, expected);
            if let Some(from) = previous {
                let Ok(next) = transition(from, regime) else {
                    panic!("handoffs must gate without skipping");
                };
                assert_eq!(next, regime);
            }
            previous = Some(regime);
        }
        assert!(matches!(
            transition(Regime::Orbit, Regime::Surface),
            Err(RegimeError::InvalidTransition { .. })
        ));
        assert!(matches!(
            transition(Regime::Surface, Regime::Orbit),
            Err(RegimeError::InvalidTransition { .. })
        ));
        let rails_gap = distance(Meters::new(120_001.0), Boundary::Rails);
        assert!(rails_gap.value() > 0.0);
        let surface_gap = distance(Meters::new(-3.0), Boundary::Surface);
        assert!(surface_gap.value() < 0.0);
    }

    // AC4: orbit cruise allows warp, atmosphere and surface deny it.
    #[test]
    fn warp_mapping_follows_architect_policy() {
        let cruise = warp_context_for_regime(Regime::Orbit, true, false, false);
        assert_eq!(cruise, WarpContext::cruise());
        assert!(matches!(
            request_warp(Warp::X10000, cruise),
            Ok(Warp::X10000)
        ));
        let entry = warp_context_for_regime(Regime::Atmosphere, true, false, false);
        assert!(entry.should_auto_drop());
        assert_eq!(apply_auto_drop(Warp::X10000, entry), Warp::X1);
        assert!(matches!(
            request_warp(Warp::X100, entry),
            Err(crate::orbit::OrbitError::WarpDenied { .. })
        ));
        let grounded = warp_context_for_regime(Regime::Surface, true, false, false);
        assert!(matches!(
            request_warp(Warp::X100, grounded),
            Err(crate::orbit::OrbitError::WarpDenied { .. })
        ));
        let on_foot_orbit = warp_context_for_regime(Regime::Orbit, false, false, false);
        assert!(matches!(
            request_warp(Warp::X10, on_foot_orbit),
            Err(crate::orbit::OrbitError::WarpDenied { .. })
        ));
        let approach = warp_context_for_regime(Regime::Orbit, true, true, false);
        assert_eq!(apply_auto_drop(Warp::X1000, approach), Warp::X1);
        let alarm = warp_context_for_regime(Regime::Orbit, true, false, true);
        assert_eq!(apply_auto_drop(Warp::X1000, alarm), Warp::X1);
    }
}
