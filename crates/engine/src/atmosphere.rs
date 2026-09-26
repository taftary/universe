//! Mars-like reference atmosphere with altitude sampling.
//!
//! Hand-placed MVP profile per `docs/specs.md` section 8; see [`AtmosphereParams::mars_like`].
//! Temperature is two-segment per D-016: `T0 + L1 * z` below the tropopause
//! and isothermal above; pressure integrates the hydrostatic equation with
//! altitude-dependent gravity from [`crate::body::BodyParams::gravity_at_altitude`]
//! using pure-Rust `libm` transcendentals so `x86_64` and `AArch64` agree.
//! Above [`ATMOSPHERE_CUTOFF_ALTITUDE_M`] the state is vacuum.

use thiserror::Error;

use crate::body::{BodyError, BodyParams};
use crate::units::{Kelvin, KilogramsPerCubicMeter, Meters, Pascals, UnitError};

/// Mars-like surface pressure in pascals.
///
/// Source: NASA Mars Fact Sheet (mean surface pressure near 610 Pa).
pub const MARS_SURFACE_PRESSURE_PA: f64 = 610.0;

/// Mars-like surface temperature in kelvin.
///
/// Source: NASA Mars Fact Sheet (mean surface temperature near 210 K).
pub const MARS_SURFACE_TEMPERATURE_K: f64 = 210.0;

/// Gas constant for carbon dioxide in joules per kilogram per kelvin.
///
/// Source: `R_universal / M_CO2` with `M_CO2 = 44.0095 g/mol` (CRC Handbook).
pub const MARS_GAS_CONSTANT_J_PER_KG_K: f64 = 188.92;

/// Mars-like tropopause altitude in meters.
///
/// Source: D-016 (analytic 2-segment profile; `T0 + L1 * z` to 50 km,
/// isothermal above). Mars has no sharp tropopause; 50 km is hand-placed
/// for profile shape only.
pub const MARS_TROPOPAUSE_ALTITUDE_M: f64 = 50_000.0;

/// Mars-like lapse-rate magnitude in kelvin per meter.
///
/// Source: D-016 (`L1 = -0.0012 K/m`, stored as a positive cooling rate so
/// `T(z) = T0 - rate * z` below the tropopause). Yields 150 K at 50 km.
pub const MARS_LAPSE_RATE_K_PER_M: f64 = 0.001_2;

/// Mars-like upper-atmosphere isothermal temperature in kelvin.
///
/// Source: D-016 (`T0 + L1 * 50 km = 150 K`). The sampler derives the top
/// temperature from surface, lapse, and tropopause; this constant records
/// the locked value for tests and docs.
pub const MARS_UPPER_ATMOSPHERE_TEMPERATURE_K: f64 = 150.0;

/// Atmosphere cutoff altitude in meters.
///
/// MVP continuum-flow top; Mars entry interface is commonly 125 km (NASA),
/// rounded to 120 km for the reference planet.
pub const ATMOSPHERE_CUTOFF_ALTITUDE_M: f64 = 120_000.0;

/// Taper thickness below the cutoff in meters.
///
/// Source: D-016 (100-120 km taper to exactly 0). Pressure and density
/// fade linearly to zero across this band so the vacuum handoff is `C0`
/// continuous.
pub const TAPER_THICKNESS_M: f64 = 20_000.0;

/// Hydrostatic pressure integration step in meters.
///
/// Source: project continuity budget (first scaffold measures it). Halving
/// to 50 m changes surface-to-cutoff pressure by well under 0.1 percent.
pub const PRESSURE_INTEGRATION_STEP_M: f64 = 100.0;

/// Vacuum pressure in pascals returned above the cutoff.
pub const VACUUM_PRESSURE_PA: f64 = 0.0;

/// Vacuum density in kilograms per cubic meter returned above the cutoff.
pub const VACUUM_DENSITY_KG_PER_M3: f64 = 0.0;

/// Atmosphere constructor and sampling failures.
#[derive(Debug, Error)]
pub enum AtmosphereError {
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
    /// Surface pressure was negative or not finite.
    #[error("invalid surface pressure in pascals: {pressure_pa_f64} Pa")]
    InvalidSurfacePressure {
        /// Rejected pressure in pascals.
        pressure_pa_f64: f64,
    },
    /// Surface temperature was not positive and finite.
    #[error("invalid surface temperature in kelvin: {temperature_k_f64} K")]
    InvalidSurfaceTemperature {
        /// Rejected temperature in kelvin.
        temperature_k_f64: f64,
    },
    /// Tropopause altitude was negative or not finite.
    #[error("invalid tropopause altitude in meters: {altitude_m_f64} m")]
    InvalidTropopause {
        /// Rejected altitude in meters.
        altitude_m_f64: f64,
    },
    /// Lapse rate was not finite or drove the top cold.
    #[error("invalid lapse rate in kelvin per meter: {lapse_k_per_m_f64} K/m")]
    InvalidLapseRate {
        /// Rejected lapse rate in kelvin per meter.
        lapse_k_per_m_f64: f64,
    },
    /// Gas constant was not positive and finite.
    #[error("invalid gas constant: {gas_constant_f64} J/(kg K)")]
    InvalidGasConstant {
        /// Rejected gas constant in joules per kilogram per kelvin.
        gas_constant_f64: f64,
    },
    /// Gravity was not positive and finite.
    ///
    /// Retained for callers that validate a surface gravity scalar directly;
    /// [`AtmosphereParams::sample_at_altitude`] takes a body and maps
    /// [`BodyError`] instead.
    #[error("invalid gravity in meters per second squared: {gravity_mps2_f64}")]
    InvalidGravity {
        /// Rejected gravity in meters per second squared.
        gravity_mps2_f64: f64,
    },
    /// Body gravity lookup failed while integrating pressure.
    #[error(transparent)]
    Body {
        /// Source body error.
        #[from]
        source: BodyError,
    },
    /// Computed temperature fell below absolute zero.
    #[error("computed temperature below absolute zero")]
    InvalidTemperature {
        /// Source unit rejection.
        #[from]
        source: UnitError,
    },
}

/// Reference-atmosphere parameters.
///
/// Surface pressure and temperature plus the tropopause, lapse rate, and gas
/// constant define the full profile. Sampling takes a [`BodyParams`] so
/// pressure integrates with altitude-dependent gravity from
/// [`BodyParams::gravity_at_altitude`]; one atmosphere works with any body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AtmosphereParams {
    /// Surface pressure in pascals.
    surface_pressure_pa: Pascals,
    /// Surface temperature in kelvin.
    surface_temperature_k: Kelvin,
    /// Tropopause altitude in meters.
    tropopause_altitude_m: Meters,
    /// Lapse rate in kelvin per meter.
    lapse_rate_k_per_m_f64: f64,
    /// Gas constant in joules per kilogram per kelvin.
    gas_constant_j_per_kg_k_f64: f64,
}

impl AtmosphereParams {
    /// Create atmosphere parameters from unit-typed values and scalar coefficients.
    ///
    /// Lapse rate uses kelvin per meter and the gas constant uses joules per
    /// kilogram per kelvin; both carry units in their names.
    ///
    /// # Errors
    ///
    /// Returns [`AtmosphereError`] when pressure, temperature, tropopause,
    /// lapse rate, or gas constant is out of range or not finite.
    pub fn new(
        surface_pressure_pa: Pascals,
        surface_temperature_k: Kelvin,
        tropopause_altitude_m: Meters,
        lapse_rate_k_per_m_f64: f64,
        gas_constant_j_per_kg_k_f64: f64,
    ) -> Result<Self, AtmosphereError> {
        let pressure_f64 = surface_pressure_pa.value();
        let temperature_f64 = surface_temperature_k.value();
        let tropopause_f64 = tropopause_altitude_m.value();
        if !pressure_f64.is_finite() {
            return Err(AtmosphereError::NonFinite {
                value_f64: pressure_f64,
            });
        }
        if !temperature_f64.is_finite() {
            return Err(AtmosphereError::NonFinite {
                value_f64: temperature_f64,
            });
        }
        if !tropopause_f64.is_finite() {
            return Err(AtmosphereError::NonFinite {
                value_f64: tropopause_f64,
            });
        }
        if !lapse_rate_k_per_m_f64.is_finite() {
            return Err(AtmosphereError::NonFinite {
                value_f64: lapse_rate_k_per_m_f64,
            });
        }
        if !gas_constant_j_per_kg_k_f64.is_finite() {
            return Err(AtmosphereError::NonFinite {
                value_f64: gas_constant_j_per_kg_k_f64,
            });
        }
        if pressure_f64 < 0.0 {
            return Err(AtmosphereError::InvalidSurfacePressure {
                pressure_pa_f64: pressure_f64,
            });
        }
        if temperature_f64 <= 0.0 {
            return Err(AtmosphereError::InvalidSurfaceTemperature {
                temperature_k_f64: temperature_f64,
            });
        }
        if tropopause_f64 < 0.0 {
            return Err(AtmosphereError::InvalidTropopause {
                altitude_m_f64: tropopause_f64,
            });
        }
        if gas_constant_j_per_kg_k_f64 <= 0.0 {
            return Err(AtmosphereError::InvalidGasConstant {
                gas_constant_f64: gas_constant_j_per_kg_k_f64,
            });
        }
        let top_temperature_f64 = temperature_f64 - lapse_rate_k_per_m_f64 * tropopause_f64;
        if top_temperature_f64 < 0.0 {
            return Err(AtmosphereError::InvalidLapseRate {
                lapse_k_per_m_f64: lapse_rate_k_per_m_f64,
            });
        }
        Ok(Self {
            surface_pressure_pa,
            surface_temperature_k,
            tropopause_altitude_m,
            lapse_rate_k_per_m_f64,
            gas_constant_j_per_kg_k_f64,
        })
    }

    /// Mars-like reference atmosphere for the MVP descent test.
    ///
    /// Two-segment D-016 profile: 210 K and 610 Pa at the surface, cooling
    /// at 0.0012 K/m to 150 K at 50 km, isothermal above. Valid by
    /// construction; fails only if a constant ever violates validation.
    ///
    /// # Errors
    ///
    /// Returns [`AtmosphereError`] when a module constant fails validation.
    pub fn mars_like() -> Result<Self, AtmosphereError> {
        let temperature_k = Kelvin::new(MARS_SURFACE_TEMPERATURE_K)?;
        Self::new(
            Pascals::new(MARS_SURFACE_PRESSURE_PA),
            temperature_k,
            Meters::new(MARS_TROPOPAUSE_ALTITUDE_M),
            MARS_LAPSE_RATE_K_PER_M,
            MARS_GAS_CONSTANT_J_PER_KG_K,
        )
    }

    /// Surface pressure in pascals.
    #[must_use]
    pub fn surface_pressure_pa(&self) -> Pascals {
        self.surface_pressure_pa
    }

    /// Surface temperature in kelvin.
    #[must_use]
    pub fn surface_temperature_k(&self) -> Kelvin {
        self.surface_temperature_k
    }

    /// Tropopause altitude in meters.
    #[must_use]
    pub fn tropopause_altitude_m(&self) -> Meters {
        self.tropopause_altitude_m
    }

    /// Lapse rate in kelvin per meter.
    #[must_use]
    pub fn lapse_rate_k_per_m(&self) -> f64 {
        self.lapse_rate_k_per_m_f64
    }

    /// Gas constant in joules per kilogram per kelvin.
    #[must_use]
    pub fn gas_constant_j_per_kg_k(&self) -> f64 {
        self.gas_constant_j_per_kg_k_f64
    }

    /// Temperature at the tropopause in kelvin.
    ///
    /// Valid by construction via [`Self::new`], so the value is finite and
    /// non-negative for every stored parameter set.
    #[must_use]
    pub fn tropopause_temperature_k(&self) -> Kelvin {
        let top_f64 = self.surface_temperature_k.value()
            - self.lapse_rate_k_per_m_f64 * self.tropopause_altitude_m.value();
        match Kelvin::new(top_f64) {
            Ok(temperature_k) => temperature_k,
            Err(_) => self.surface_temperature_k,
        }
    }

    /// Sample pressure, temperature, and density at altitude.
    ///
    /// Two-segment temperature per D-016 with hydrostatic pressure integrated
    /// against altitude-dependent gravity from `body`; `libm` provides the
    /// exponentials so platforms agree. Above the cutoff the state is vacuum
    /// at the tropopause temperature.
    ///
    /// # Errors
    ///
    /// Returns [`AtmosphereError::BelowSurface`] for negative altitudes,
    /// [`AtmosphereError::NonFinite`] for non-finite inputs or outputs, and
    /// [`AtmosphereError::Body`] when the gravity lookup fails.
    pub fn sample_at_altitude(
        &self,
        altitude: Meters,
        body: &BodyParams,
    ) -> Result<AtmosphereState, AtmosphereError> {
        let altitude_f64 = altitude.value();
        if !altitude_f64.is_finite() {
            return Err(AtmosphereError::NonFinite {
                value_f64: altitude_f64,
            });
        }
        if altitude_f64 < 0.0 {
            return Err(AtmosphereError::BelowSurface {
                altitude_m: altitude_f64,
            });
        }
        let top_temperature_k = self.tropopause_temperature_k();
        if is_above_cutoff(altitude) {
            return Ok(AtmosphereState {
                pressure_pa: Pascals::new(VACUUM_PRESSURE_PA),
                temperature_k: top_temperature_k,
                density_kg_per_m3: KilogramsPerCubicMeter::new(VACUUM_DENSITY_KG_PER_M3),
            });
        }
        let gas_constant_f64 = self.gas_constant_j_per_kg_k_f64;
        let temperature_f64 = self.temperature_at_altitude_f64(altitude_f64);
        if !temperature_f64.is_finite() {
            return Err(AtmosphereError::NonFinite {
                value_f64: temperature_f64,
            });
        }
        let temperature_k = Kelvin::new(temperature_f64)?;
        let pressure_f64 = self.pressure_at_altitude_f64(altitude_f64, body)?;
        let tapered_f64 = apply_cutoff_taper(pressure_f64, altitude_f64);
        let density_f64 = if tapered_f64 <= 0.0 {
            0.0
        } else {
            tapered_f64 / (gas_constant_f64 * temperature_f64)
        };
        if !density_f64.is_finite() {
            return Err(AtmosphereError::NonFinite {
                value_f64: density_f64,
            });
        }
        Ok(AtmosphereState {
            pressure_pa: Pascals::new(tapered_f64),
            temperature_k,
            density_kg_per_m3: KilogramsPerCubicMeter::new(density_f64),
        })
    }

    /// Two-segment temperature at altitude in kelvin.
    ///
    /// `T0 - rate * z` at and below the tropopause, tropopause temperature
    /// above. For the Mars-like constants this yields 150 K at 50 km per
    /// D-016 (`T0 + L1 * z` with `L1 = -0.0012 K/m`).
    fn temperature_at_altitude_f64(&self, altitude_f64: f64) -> f64 {
        let surface_temperature_f64 = self.surface_temperature_k.value();
        let tropopause_f64 = self.tropopause_altitude_m.value();
        if altitude_f64 <= tropopause_f64 {
            surface_temperature_f64 - self.lapse_rate_k_per_m_f64 * altitude_f64
        } else {
            self.tropopause_temperature_k().value()
        }
    }

    /// Hydrostatic pressure at altitude in pascals.
    ///
    /// Integrates `dp/dz = -p * g(z) / (R * T(z))` from the surface in fixed
    /// [`PRESSURE_INTEGRATION_STEP_M`] slabs with midpoint temperature and
    /// gravity from [`BodyParams::gravity_at_altitude`]. Uses `libm` only,
    /// never platform transcendentals.
    fn pressure_at_altitude_f64(
        &self,
        altitude_f64: f64,
        body: &BodyParams,
    ) -> Result<f64, AtmosphereError> {
        let surface_pressure_f64 = self.surface_pressure_pa.value();
        let gas_constant_f64 = self.gas_constant_j_per_kg_k_f64;
        if altitude_f64 <= 0.0 {
            return Ok(surface_pressure_f64);
        }
        let mut pressure_f64 = surface_pressure_f64;
        let mut base_f64 = 0.0;
        while base_f64 < altitude_f64 {
            let step_f64 = (altitude_f64 - base_f64).min(PRESSURE_INTEGRATION_STEP_M);
            let mid_f64 = base_f64 + step_f64 * 0.5;
            let temperature_f64 = self.temperature_at_altitude_f64(mid_f64);
            let gravity_f64 = body.gravity_at_altitude(Meters::new(mid_f64))?.value();
            let exponent_f64 = -gravity_f64 * step_f64 / (gas_constant_f64 * temperature_f64);
            pressure_f64 *= libm::exp(exponent_f64);
            if !pressure_f64.is_finite() {
                return Err(AtmosphereError::NonFinite {
                    value_f64: pressure_f64,
                });
            }
            base_f64 += step_f64;
        }
        Ok(pressure_f64)
    }
}

/// Sampled atmosphere state at one altitude.
///
/// Pressure, temperature, and density share the sample altitude; above the
/// cutoff pressure and density are zero while temperature holds the top.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AtmosphereState {
    /// Pressure in pascals.
    pressure_pa: Pascals,
    /// Temperature in kelvin.
    temperature_k: Kelvin,
    /// Density in kilograms per cubic meter.
    density_kg_per_m3: KilogramsPerCubicMeter,
}

impl AtmosphereState {
    /// Pressure in pascals.
    #[must_use]
    pub fn pressure_pa(&self) -> Pascals {
        self.pressure_pa
    }

    /// Temperature in kelvin.
    #[must_use]
    pub fn temperature_k(&self) -> Kelvin {
        self.temperature_k
    }

    /// Density in kilograms per cubic meter.
    #[must_use]
    pub fn density_kg_per_m3(&self) -> KilogramsPerCubicMeter {
        self.density_kg_per_m3
    }
}

/// Report whether altitude sits above the atmosphere cutoff.
///
/// Pure function of [`ATMOSPHERE_CUTOFF_ALTITUDE_M`]; non-finite altitudes
/// never count as above so sampling still returns the typed error.
#[must_use]
pub fn is_above_cutoff(altitude: Meters) -> bool {
    let altitude_f64 = altitude.value();
    altitude_f64.is_finite() && altitude_f64 > ATMOSPHERE_CUTOFF_ALTITUDE_M
}

/// Fade pressure linearly to zero across the taper band.
///
/// Full value at and below `cutoff - taper`, zero at the cutoff, vacuum
/// above. Keeps the atmosphere-to-vacuum handoff `C0` continuous.
fn apply_cutoff_taper(pressure_pa_f64: f64, altitude_f64: f64) -> f64 {
    let taper_start_f64 = ATMOSPHERE_CUTOFF_ALTITUDE_M - TAPER_THICKNESS_M;
    if altitude_f64 <= taper_start_f64 {
        pressure_pa_f64
    } else if altitude_f64 >= ATMOSPHERE_CUTOFF_ALTITUDE_M {
        0.0
    } else {
        let fade_f64 = (ATMOSPHERE_CUTOFF_ALTITUDE_M - altitude_f64) / TAPER_THICKNESS_M;
        pressure_pa_f64 * fade_f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::BodyParams;
    use crate::units::{Kilograms, Seconds};

    /// Tolerance for surface pressure in pascals.
    const PRESSURE_EPS_PA: f64 = 1.0;
    /// Tolerance for temperature in kelvin.
    const TEMPERATURE_EPS_K: f64 = 0.5;
    /// Tolerance for surface density in kilograms per cubic meter.
    const DENSITY_EPS_KG_PER_M3: f64 = 0.000_8;
    /// Tolerance for the scale-height pressure check in pascals (5 percent).
    const SCALE_HEIGHT_EPS_PA: f64 = 11.3;
    /// Tolerance for exact-vacuum assertions.
    const VACUUM_EPS: f64 = 1e-9;
    /// Literal one-scale-height altitude in meters (D-015 reference value).
    const SCALE_HEIGHT_LITERAL_M: f64 = 10_695.0;
    /// Reference pressure at one scale height in pascals.
    const SCALE_HEIGHT_PRESSURE_PA: f64 = 224.4;
    /// Earth-like mass in kilograms for the generic lapse test.
    const EARTH_TEST_MASS_KG: f64 = 5.972_e24;
    /// Earth-like radius in meters for the generic lapse test.
    const EARTH_TEST_RADIUS_M: f64 = 6_371_000.0;
    /// Earth-like rotation period in seconds for the generic lapse test.
    const EARTH_TEST_PERIOD_S: f64 = 86_400.0;

    /// Mars-like parameters or test failure.
    fn mars_params() -> AtmosphereParams {
        let Ok(params) = AtmosphereParams::mars_like() else {
            panic!("mars-like parameters must validate");
        };
        params
    }

    /// Mars-like body for gravity-dependent sampling.
    fn mars_body() -> BodyParams {
        BodyParams::mars_like()
    }

    /// Earth-like body for the generic lapse path.
    fn earth_body() -> BodyParams {
        let Ok(body) = BodyParams::new(
            Kilograms::new(EARTH_TEST_MASS_KG),
            Meters::new(EARTH_TEST_RADIUS_M),
            Seconds::new(EARTH_TEST_PERIOD_S),
        ) else {
            panic!("earth test body must validate");
        };
        body
    }

    #[test]
    fn surface_matches_reference_values() {
        let params = mars_params();
        let body = mars_body();
        let Ok(state) = params.sample_at_altitude(Meters::new(0.0), &body) else {
            panic!("surface sample must succeed");
        };
        assert!(
            (state.pressure_pa().value() - 610.0).abs() < PRESSURE_EPS_PA,
            "surface pressure differs from 610 Pa"
        );
        assert!(
            (state.temperature_k().value() - 210.0).abs() < TEMPERATURE_EPS_K,
            "surface temperature differs from 210 K"
        );
        assert!(
            (state.density_kg_per_m3().value() - 0.015_38).abs() < DENSITY_EPS_KG_PER_M3,
            "surface density differs from 0.01538 kg/m3"
        );
    }

    #[test]
    fn scale_height_pressure_matches_reference() {
        let params = mars_params();
        let body = mars_body();
        let Ok(state) = params.sample_at_altitude(Meters::new(SCALE_HEIGHT_LITERAL_M), &body)
        else {
            panic!("scale-height sample must succeed");
        };
        assert!(
            (state.pressure_pa().value() - SCALE_HEIGHT_PRESSURE_PA).abs() < SCALE_HEIGHT_EPS_PA,
            "scale-height pressure differs from 224.4 Pa"
        );
    }

    #[test]
    fn two_segment_temperature_matches_locked_profile() {
        let params = mars_params();
        let body = mars_body();
        assert!(
            (params.tropopause_altitude_m().value() - 50_000.0).abs() < 1.0,
            "tropopause differs from 50 km"
        );
        assert!(
            (params.tropopause_temperature_k().value() - MARS_UPPER_ATMOSPHERE_TEMPERATURE_K).abs()
                < TEMPERATURE_EPS_K,
            "tropopause temperature differs from 150 K"
        );
        let Ok(at_tropo) = params.sample_at_altitude(Meters::new(50_000.0), &body) else {
            panic!("50 km sample must succeed");
        };
        assert!(
            (at_tropo.temperature_k().value() - 150.0).abs() < 1.0,
            "50 km temperature differs from 150 K"
        );
        let Ok(above) = params.sample_at_altitude(Meters::new(75_000.0), &body) else {
            panic!("75 km sample must succeed");
        };
        assert!(
            (above.temperature_k().value() - 150.0).abs() < 1.0,
            "upper atmosphere must hold 150 K"
        );
    }

    #[test]
    fn reference_altitudes_match_expected_bands() {
        let params = mars_params();
        let body = mars_body();
        let Ok(at_50km) = params.sample_at_altitude(Meters::new(50_000.0), &body) else {
            panic!("50 km sample must succeed");
        };
        let pressure_50km_f64 = at_50km.pressure_pa().value();
        assert!(
            pressure_50km_f64 > 1.5 && pressure_50km_f64 < 4.0,
            "50 km pressure {pressure_50km_f64} Pa out of band"
        );
        let Ok(at_100km) = params.sample_at_altitude(Meters::new(100_000.0), &body) else {
            panic!("100 km sample must succeed");
        };
        let pressure_100km_f64 = at_100km.pressure_pa().value();
        assert!(
            pressure_100km_f64 > 0.002 && pressure_100km_f64 < 0.01,
            "100 km pressure {pressure_100km_f64} Pa out of band"
        );
        let Ok(at_120km) = params.sample_at_altitude(Meters::new(120_000.0), &body) else {
            panic!("120 km sample must succeed");
        };
        assert!(at_120km.pressure_pa().value().abs() < VACUUM_EPS);
        assert!(at_120km.density_kg_per_m3().value().abs() < VACUUM_EPS);
        assert!(
            at_50km.pressure_pa().value() > at_100km.pressure_pa().value(),
            "pressure must fall from 50 km to 100 km"
        );
        assert!(
            at_100km.pressure_pa().value() > at_120km.pressure_pa().value(),
            "pressure must fall from 100 km to 120 km"
        );
    }

    #[test]
    fn profile_is_monotonic() {
        let params = mars_params();
        let body = mars_body();
        let checkpoints_m = [
            0.0, 10_000.0, 25_000.0, 50_000.0, 75_000.0, 100_000.0, 115_000.0,
        ];
        let mut previous_pa = f64::INFINITY;
        let mut previous_density = f64::INFINITY;
        for altitude_f64 in checkpoints_m {
            let Ok(state) = params.sample_at_altitude(Meters::new(altitude_f64), &body) else {
                panic!("altitude {altitude_f64} must sample");
            };
            assert!(
                state.pressure_pa().value() <= previous_pa,
                "pressure must not rise at {altitude_f64}"
            );
            assert!(
                state.density_kg_per_m3().value() <= previous_density,
                "density must not rise at {altitude_f64}"
            );
            previous_pa = state.pressure_pa().value();
            previous_density = state.density_kg_per_m3().value();
        }
    }

    #[test]
    fn vacuum_above_cutoff() {
        let params = mars_params();
        let body = mars_body();
        assert!(!is_above_cutoff(Meters::new(119_999.0)));
        assert!(is_above_cutoff(Meters::new(120_001.0)));
        let Ok(state) = params.sample_at_altitude(Meters::new(130_000.0), &body) else {
            panic!("130 km sample must succeed");
        };
        assert!(state.pressure_pa().value().abs() < VACUUM_EPS);
        assert!(state.density_kg_per_m3().value().abs() < VACUUM_EPS);
    }

    #[test]
    fn below_surface_fails() {
        let params = mars_params();
        let body = mars_body();
        let Err(err) = params.sample_at_altitude(Meters::new(-10.0), &body) else {
            panic!("negative altitude must fail");
        };
        assert!(matches!(err, AtmosphereError::BelowSurface { .. }));
    }

    #[test]
    fn non_finite_inputs_fail() {
        let params = mars_params();
        let body = mars_body();
        let Err(err) = params.sample_at_altitude(Meters::new(f64::NAN), &body) else {
            panic!("NaN altitude must fail");
        };
        assert!(matches!(err, AtmosphereError::NonFinite { .. }));
    }

    #[test]
    fn kelvin_rejects_negative_values() {
        match Kelvin::new(-1.0) {
            Ok(_) => panic!("negative kelvin must fail"),
            Err(err) => assert!(matches!(err, UnitError::BelowAbsoluteZero { .. })),
        }
    }

    #[test]
    fn lapse_path_cools_with_altitude() {
        let Ok(temperature_k) = Kelvin::new(288.0) else {
            panic!("288 K must validate");
        };
        let Ok(params) = AtmosphereParams::new(
            Pascals::new(101_325.0),
            temperature_k,
            Meters::new(11_000.0),
            0.006_5,
            287.05,
        ) else {
            panic!("lapse parameters must validate");
        };
        let body = earth_body();
        let Ok(low) = params.sample_at_altitude(Meters::new(0.0), &body) else {
            panic!("surface lapse sample must succeed");
        };
        let Ok(high) = params.sample_at_altitude(Meters::new(5_000.0), &body) else {
            panic!("5 km lapse sample must succeed");
        };
        assert!(high.temperature_k().value() < low.temperature_k().value());
        assert!(high.pressure_pa().value() < low.pressure_pa().value());
        assert!(
            (high.temperature_k().value() - 255.5).abs() < 1.0,
            "lapse temperature differs from 255.5 K"
        );
    }

    #[test]
    fn constructor_rejects_bad_params() {
        let Ok(temperature_k) = Kelvin::new(210.0) else {
            panic!("210 K must validate");
        };
        assert!(
            AtmosphereParams::new(
                Pascals::new(-1.0),
                temperature_k,
                Meters::new(MARS_TROPOPAUSE_ALTITUDE_M),
                MARS_LAPSE_RATE_K_PER_M,
                MARS_GAS_CONSTANT_J_PER_KG_K,
            )
            .is_err()
        );
        assert!(
            AtmosphereParams::new(
                Pascals::new(MARS_SURFACE_PRESSURE_PA),
                temperature_k,
                Meters::new(MARS_TROPOPAUSE_ALTITUDE_M),
                MARS_LAPSE_RATE_K_PER_M,
                0.0,
            )
            .is_err()
        );
        assert!(
            AtmosphereParams::new(
                Pascals::new(MARS_SURFACE_PRESSURE_PA),
                temperature_k,
                Meters::new(MARS_TROPOPAUSE_ALTITUDE_M),
                f64::NAN,
                MARS_GAS_CONSTANT_J_PER_KG_K,
            )
            .is_err()
        );
    }
}
