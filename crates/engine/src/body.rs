//! Reference-planet body parameters and gravity profile.
//!
//! Hand-placed Mars-like values per `docs/specs.md` section 8; see [`BodyParams::mars_like`].
//! Gravity follows Newton with [`GRAVITATIONAL_CONSTANT_M3_KG_S2`]; altitude profiles
//! stay continuous down to the surface. For atmosphere sampling see `crate::atmosphere`.

use thiserror::Error;

use crate::units::{Kilograms, Meters, MetersPerSecondSquared, Seconds};

/// Gravitational constant in cubic meters per kilogram per second squared.
///
/// Source: CODATA 2018 (Tiesinga et al., Rev. Mod. Phys. 93, 2021).
pub const GRAVITATIONAL_CONSTANT_M3_KG_S2: f64 = 6.674_30e-11;

/// Mars-like mass in kilograms.
///
/// Source: NASA Mars Fact Sheet (mean mass 6.4171e23 kg).
pub const MARS_MASS_KG: f64 = 6.417_1e23;

/// Mars-like mean radius in meters.
///
/// Source: NASA Mars Fact Sheet (mean radius 3389.5 km).
pub const MARS_RADIUS_M: f64 = 3_389_500.0;

/// Mars-like rotation period in seconds.
///
/// Source: NASA Mars Fact Sheet (sidereal day 24 h 37 m 22.7 s).
pub const MARS_ROTATION_PERIOD_S: f64 = 88_642.0;

/// Zero altitude in meters for surface comparisons.
///
/// Surface gravity evaluates [`BodyParams::gravity_at_altitude`] at this altitude.
pub const SURFACE_ALTITUDE_M: f64 = 0.0;

/// Body constructor and sampling failures.
#[derive(Debug, Error)]
pub enum BodyError {
    /// Altitude was below the surface.
    #[error("altitude below surface: {altitude_m} m")]
    BelowSurface {
        /// Rejected altitude in meters.
        altitude_m: f64,
    },
    /// Input or computed value was not finite.
    #[error("non-finite value: {value_f64}")]
    NonFinite {
        /// Rejected value.
        value_f64: f64,
    },
    /// Mass was not positive and finite.
    #[error("invalid mass in kilograms: {mass_kg_f64} kg")]
    InvalidMass {
        /// Rejected mass in kilograms.
        mass_kg_f64: f64,
    },
    /// Radius was not positive and finite.
    #[error("invalid radius in meters: {radius_m_f64} m")]
    InvalidRadius {
        /// Rejected radius in meters.
        radius_m_f64: f64,
    },
    /// Rotation period was not positive and finite.
    #[error("invalid rotation period in seconds: {period_s_f64} s")]
    InvalidRotationPeriod {
        /// Rejected period in seconds.
        period_s_f64: f64,
    },
}

/// Reference-planet body parameters.
///
/// Mass, radius, and rotation period define surface gravity and the
/// altitude profile. All fields use unit newtypes; raw `f64` never
/// crosses the constructor boundary unwrapped.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyParams {
    /// Mass in kilograms.
    mass_kg: Kilograms,
    /// Mean radius in meters.
    radius_m: Meters,
    /// Rotation period in seconds.
    rotation_period_s: Seconds,
}

impl BodyParams {
    /// Create body parameters from unit-typed mass, radius, and rotation.
    ///
    /// # Errors
    ///
    /// Returns [`BodyError::InvalidMass`], [`BodyError::InvalidRadius`], or
    /// [`BodyError::InvalidRotationPeriod`] when the corresponding value is
    /// not positive and finite.
    pub fn new(
        mass_kg: Kilograms,
        radius_m: Meters,
        rotation_period_s: Seconds,
    ) -> Result<Self, BodyError> {
        let mass_f64 = mass_kg.value();
        let radius_f64 = radius_m.value();
        let period_f64 = rotation_period_s.value();
        if !mass_f64.is_finite() {
            return Err(BodyError::NonFinite {
                value_f64: mass_f64,
            });
        }
        if !radius_f64.is_finite() {
            return Err(BodyError::NonFinite {
                value_f64: radius_f64,
            });
        }
        if !period_f64.is_finite() {
            return Err(BodyError::NonFinite {
                value_f64: period_f64,
            });
        }
        if mass_f64 <= 0.0 {
            return Err(BodyError::InvalidMass {
                mass_kg_f64: mass_f64,
            });
        }
        if radius_f64 <= 0.0 {
            return Err(BodyError::InvalidRadius {
                radius_m_f64: radius_f64,
            });
        }
        if period_f64 <= 0.0 {
            return Err(BodyError::InvalidRotationPeriod {
                period_s_f64: period_f64,
            });
        }
        Ok(Self {
            mass_kg,
            radius_m,
            rotation_period_s,
        })
    }

    /// Mars-like reference planet for the MVP descent test.
    ///
    /// Uses [`MARS_MASS_KG`], [`MARS_RADIUS_M`], and [`MARS_ROTATION_PERIOD_S`];
    /// valid by construction and never fails. See `docs/specs.md` section 8.3.
    #[must_use]
    pub fn mars_like() -> Self {
        Self {
            mass_kg: Kilograms::new(MARS_MASS_KG),
            radius_m: Meters::new(MARS_RADIUS_M),
            rotation_period_s: Seconds::new(MARS_ROTATION_PERIOD_S),
        }
    }

    /// Mass in kilograms.
    #[must_use]
    pub fn mass_kg(&self) -> Kilograms {
        self.mass_kg
    }

    /// Mean radius in meters.
    #[must_use]
    pub fn radius_m(&self) -> Meters {
        self.radius_m
    }

    /// Rotation period in seconds.
    #[must_use]
    pub fn rotation_period_s(&self) -> Seconds {
        self.rotation_period_s
    }

    /// Standard gravitational parameter in cubic meters per second squared.
    ///
    /// Product of [`GRAVITATIONAL_CONSTANT_M3_KG_S2`] and mass; deterministic
    /// IEEE arithmetic, no transcendentals.
    #[must_use]
    pub fn gravitational_parameter_m3_s2(&self) -> f64 {
        GRAVITATIONAL_CONSTANT_M3_KG_S2 * self.mass_kg.value()
    }

    /// Surface gravity at zero altitude.
    ///
    /// Evaluates [`Self::gravity_at_altitude`] at [`SURFACE_ALTITUDE_M`], which
    /// is valid by construction, so this accessor never fails.
    #[must_use]
    pub fn surface_gravity(&self) -> MetersPerSecondSquared {
        let radius_f64 = self.radius_m.value();
        let gravity_f64 = self.gravitational_parameter_m3_s2() / (radius_f64 * radius_f64);
        MetersPerSecondSquared::new(gravity_f64)
    }

    /// Gravity magnitude at altitude above the surface.
    ///
    /// Newtonian profile `g(z) = mu / (R + z)^2` with `mu` from
    /// [`Self::gravitational_parameter_m3_s2`].
    ///
    /// # Errors
    ///
    /// Returns [`BodyError::BelowSurface`] when `altitude` is negative and
    /// [`BodyError::NonFinite`] when `altitude` or the result is not finite.
    pub fn gravity_at_altitude(
        &self,
        altitude: Meters,
    ) -> Result<MetersPerSecondSquared, BodyError> {
        let altitude_f64 = altitude.value();
        if !altitude_f64.is_finite() {
            return Err(BodyError::NonFinite {
                value_f64: altitude_f64,
            });
        }
        if altitude_f64 < SURFACE_ALTITUDE_M {
            return Err(BodyError::BelowSurface {
                altitude_m: altitude_f64,
            });
        }
        let distance_f64 = self.radius_m.value() + altitude_f64;
        let gravity_f64 = self.gravitational_parameter_m3_s2() / (distance_f64 * distance_f64);
        if !gravity_f64.is_finite() {
            return Err(BodyError::NonFinite {
                value_f64: gravity_f64,
            });
        }
        Ok(MetersPerSecondSquared::new(gravity_f64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Absolute tolerance for gravity comparisons in meters per second squared.
    const GRAVITY_EPS_MPS2: f64 = 0.05;

    #[test]
    fn mars_surface_gravity_matches_reference() {
        let body = BodyParams::mars_like();
        let gravity = body.surface_gravity().value();
        assert!(
            (gravity - 3.71).abs() < GRAVITY_EPS_MPS2,
            "surface gravity {gravity} differs from 3.71"
        );
    }

    #[test]
    fn gravity_decreases_with_altitude() {
        let body = BodyParams::mars_like();
        let mut previous_f64 = body.surface_gravity().value();
        let checkpoints_m = [10_000.0, 50_000.0, 100_000.0, 120_000.0];
        for altitude_f64 in checkpoints_m {
            let Ok(sample) = body.gravity_at_altitude(Meters::new(altitude_f64)) else {
                panic!("altitude {altitude_f64} must sample");
            };
            assert!(
                sample.value() < previous_f64,
                "gravity must decrease at {altitude_f64}"
            );
            previous_f64 = sample.value();
        }
    }

    #[test]
    fn surface_matches_zero_altitude_sample() {
        let body = BodyParams::mars_like();
        let Ok(at_surface) = body.gravity_at_altitude(Meters::new(SURFACE_ALTITUDE_M)) else {
            panic!("surface altitude must sample");
        };
        assert_eq!(at_surface, body.surface_gravity());
    }

    #[test]
    fn negative_altitude_is_below_surface() {
        let body = BodyParams::mars_like();
        let Err(err) = body.gravity_at_altitude(Meters::new(-1.0)) else {
            panic!("negative altitude must fail");
        };
        assert!(matches!(err, BodyError::BelowSurface { .. }));
    }

    #[test]
    fn non_finite_altitude_fails() {
        let body = BodyParams::mars_like();
        for bad_f64 in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let Err(err) = body.gravity_at_altitude(Meters::new(bad_f64)) else {
                panic!("non-finite {bad_f64} must fail");
            };
            assert!(matches!(err, BodyError::NonFinite { .. }));
        }
    }

    #[test]
    fn constructor_rejects_invalid_params() {
        assert!(
            BodyParams::new(
                Kilograms::new(0.0),
                Meters::new(MARS_RADIUS_M),
                Seconds::new(MARS_ROTATION_PERIOD_S),
            )
            .is_err()
        );
        assert!(
            BodyParams::new(
                Kilograms::new(MARS_MASS_KG),
                Meters::new(-1.0),
                Seconds::new(MARS_ROTATION_PERIOD_S),
            )
            .is_err()
        );
        assert!(
            BodyParams::new(
                Kilograms::new(f64::NAN),
                Meters::new(MARS_RADIUS_M),
                Seconds::new(MARS_ROTATION_PERIOD_S),
            )
            .is_err()
        );
    }
}
