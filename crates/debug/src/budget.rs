//! Budget strip with fraction colors over named budgets.
//!
//! Holds the latest cost samples plus a render-only thermal tier and draws
//! one bar per metric as fraction plus percent plus band. Budget
//! denominators arrive at draw time and are never stored; gates live in
//! `docs/tech/quality.md`. Read-only for the sim; never writes sim state.

use crate::theme::{BudgetStatus, ThemeError};

/// Budget-strip range-check failures.
///
/// Returned for non-finite samples, negative samples, non-positive
/// denominators, or unclassifiable fractions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BudgetError {
    /// Sample or denominator was non-finite.
    NonFinite {
        /// Rejected value.
        value_f64: f64,
    },
    /// Sample was negative.
    Negative {
        /// Rejected value.
        value_f64: f64,
    },
    /// Budget denominator was not strictly positive.
    NonPositiveBudget {
        /// Rejected budget value.
        value_f64: f64,
    },
}

impl core::fmt::Display for BudgetError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NonFinite { value_f64 } => {
                write!(formatter, "non-finite budget value: {value_f64}")
            }
            Self::Negative { value_f64 } => {
                write!(formatter, "negative budget value: {value_f64}")
            }
            Self::NonPositiveBudget { value_f64 } => {
                write!(formatter, "non-positive budget: {value_f64}")
            }
        }
    }
}

impl std::error::Error for BudgetError {}

/// Render-only thermal tier for the budget strip.
///
/// Tier downgrade is render-only; sim behavior is identical across tiers
/// per `docs/tech/quality.md` and `docs/tech/mobile.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermalTier {
    /// Full rendering scale headroom.
    High,
    /// Reference phone behavior.
    Medium,
    /// Budget gate oldest devices.
    Low,
}

impl ThermalTier {
    /// All thermal tiers.
    pub const ALL: [Self; 3] = [Self::High, Self::Medium, Self::Low];

    /// Return the short tier label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }
}

/// Budget denominators passed at draw time, never stored.
///
/// Names cite `docs/tech/quality.md`; values come from the caller so the
/// strip never restates a gate.
#[expect(
    clippy::struct_field_names,
    reason = "unit suffixes are required by the naming rule"
)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BudgetDenominators {
    /// Frame budget in milliseconds (`FRAME_BUDGET_MS`).
    frame_ms_f64: f64,
    /// Sim tick average budget in milliseconds (`SIM_TICK_AVG_MS`).
    sim_avg_ms_f64: f64,
    /// Sim tick p99 budget in milliseconds (`SIM_TICK_P99_MS`).
    sim_p99_ms_f64: f64,
    /// Surface hitch p95 budget in milliseconds (`SURFACE_HITCH_P95_MS`).
    hitch_ms_f64: f64,
    /// Memory ceiling in megabytes (`MEMORY_CEILING_MB`).
    memory_mb_f64: f64,
    /// Cold-start budget in seconds (`COLD_START_S`).
    cold_start_s_f64: f64,
}

impl BudgetDenominators {
    /// Build denominators from the six named budgets.
    ///
    /// # Errors
    ///
    /// Returns [`BudgetError`] for a non-finite or non-positive budget.
    pub fn new(
        frame_ms_f64: f64,
        sim_avg_ms_f64: f64,
        sim_p99_ms_f64: f64,
        hitch_ms_f64: f64,
        memory_mb_f64: f64,
        cold_start_s_f64: f64,
    ) -> Result<Self, BudgetError> {
        for budget_ms_f64 in [
            frame_ms_f64,
            sim_avg_ms_f64,
            sim_p99_ms_f64,
            hitch_ms_f64,
            memory_mb_f64,
            cold_start_s_f64,
        ] {
            if !budget_ms_f64.is_finite() {
                return Err(BudgetError::NonFinite {
                    value_f64: budget_ms_f64,
                });
            }
            if budget_ms_f64 <= 0.0 {
                return Err(BudgetError::NonPositiveBudget {
                    value_f64: budget_ms_f64,
                });
            }
        }
        Ok(Self {
            frame_ms_f64,
            sim_avg_ms_f64,
            sim_p99_ms_f64,
            hitch_ms_f64,
            memory_mb_f64,
            cold_start_s_f64,
        })
    }
}

/// Budget strip with latest samples plus a thermal tier.
///
/// Plain data only; samples arrive by copy and denominators arrive at draw
/// time. Shell state only and never persists.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BudgetStrip {
    /// Latest frame time in milliseconds.
    frame_ms_f64: f64,
    /// Latest sim tick average in milliseconds.
    sim_avg_ms_f64: f64,
    /// Latest sim tick p99 in milliseconds.
    sim_p99_ms_f64: f64,
    /// Latest surface hitch p95 in milliseconds.
    hitch_p95_ms_f64: f64,
    /// Latest resident memory in megabytes.
    resident_mb_f64: f64,
    /// Latest cold start in seconds.
    cold_start_s_f64: f64,
    /// Latest shell draw cost in milliseconds.
    shell_ms_f64: f64,
    /// Render-only thermal tier.
    tier: ThermalTier,
}

impl BudgetStrip {
    /// Build a zeroed strip at the medium tier.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            frame_ms_f64: 0.0,
            sim_avg_ms_f64: 0.0,
            sim_p99_ms_f64: 0.0,
            hitch_p95_ms_f64: 0.0,
            resident_mb_f64: 0.0,
            cold_start_s_f64: 0.0,
            shell_ms_f64: 0.0,
            tier: ThermalTier::Medium,
        }
    }

    /// Set frame time in milliseconds.
    ///
    /// # Errors
    ///
    /// Returns [`BudgetError`] for a non-finite or negative value.
    pub fn set_frame_ms_f64(&mut self, value_f64: f64) -> Result<(), BudgetError> {
        Self::check_sample_f64(value_f64)?;
        self.frame_ms_f64 = value_f64;
        Ok(())
    }

    /// Set sim tick average in milliseconds.
    ///
    /// # Errors
    ///
    /// Returns [`BudgetError`] for a non-finite or negative value.
    pub fn set_sim_avg_ms_f64(&mut self, value_f64: f64) -> Result<(), BudgetError> {
        Self::check_sample_f64(value_f64)?;
        self.sim_avg_ms_f64 = value_f64;
        Ok(())
    }

    /// Set sim tick p99 in milliseconds.
    ///
    /// # Errors
    ///
    /// Returns [`BudgetError`] for a non-finite or negative value.
    pub fn set_sim_p99_ms_f64(&mut self, value_f64: f64) -> Result<(), BudgetError> {
        Self::check_sample_f64(value_f64)?;
        self.sim_p99_ms_f64 = value_f64;
        Ok(())
    }

    /// Set surface hitch p95 in milliseconds.
    ///
    /// # Errors
    ///
    /// Returns [`BudgetError`] for a non-finite or negative value.
    pub fn set_hitch_p95_ms_f64(&mut self, value_f64: f64) -> Result<(), BudgetError> {
        Self::check_sample_f64(value_f64)?;
        self.hitch_p95_ms_f64 = value_f64;
        Ok(())
    }

    /// Set resident memory in megabytes.
    ///
    /// # Errors
    ///
    /// Returns [`BudgetError`] for a non-finite or negative value.
    pub fn set_resident_mb_f64(&mut self, value_f64: f64) -> Result<(), BudgetError> {
        Self::check_sample_f64(value_f64)?;
        self.resident_mb_f64 = value_f64;
        Ok(())
    }

    /// Set cold start in seconds.
    ///
    /// # Errors
    ///
    /// Returns [`BudgetError`] for a non-finite or negative value.
    pub fn set_cold_start_s_f64(&mut self, value_f64: f64) -> Result<(), BudgetError> {
        Self::check_sample_f64(value_f64)?;
        self.cold_start_s_f64 = value_f64;
        Ok(())
    }

    /// Set shell draw cost in milliseconds.
    ///
    /// # Errors
    ///
    /// Returns [`BudgetError`] for a non-finite or negative value.
    pub fn set_shell_ms_f64(&mut self, value_f64: f64) -> Result<(), BudgetError> {
        Self::check_sample_f64(value_f64)?;
        self.shell_ms_f64 = value_f64;
        Ok(())
    }

    /// Set the render-only thermal tier.
    pub const fn set_thermal_tier(&mut self, tier: ThermalTier) {
        self.tier = tier;
    }

    /// Return the thermal tier.
    #[must_use]
    pub const fn thermal_tier(self) -> ThermalTier {
        self.tier
    }

    /// Return one metric as a fraction of its denominator.
    ///
    /// # Errors
    ///
    /// Returns [`BudgetError`] for a bad denominator or fraction.
    pub fn fraction_of(value_f64: f64, budget_f64: f64) -> Result<BudgetStatus, BudgetError> {
        if !value_f64.is_finite() || !budget_f64.is_finite() {
            return Err(BudgetError::NonFinite {
                value_f64: value_f64 + budget_f64,
            });
        }
        if value_f64 < 0.0 {
            return Err(BudgetError::Negative { value_f64 });
        }
        if budget_f64 <= 0.0 {
            return Err(BudgetError::NonPositiveBudget {
                value_f64: budget_f64,
            });
        }
        BudgetStatus::new(value_f64 / budget_f64).map_err(|source: ThemeError| match source {
            ThemeError::NonFinite { value_f64 } => BudgetError::NonFinite { value_f64 },
            ThemeError::NegativeFraction { value_f64 } => BudgetError::Negative { value_f64 },
        })
    }

    /// Draw seven budget bars with fraction colors plus the tier label.
    ///
    /// Immediate-mode widgets only; creates no renderer. Color never
    /// carries meaning alone; every bar shows fraction plus percent plus
    /// band. Available only with the non-default `dev-shell` feature.
    #[cfg(feature = "dev-shell")]
    pub fn draw(&self, ui: &mut egui::Ui, budgets: BudgetDenominators) {
        let rows: [(&str, f64, &str, f64); 7] = [
            ("frame ms", self.frame_ms_f64, "ms", budgets.frame_ms_f64),
            (
                "sim avg ms",
                self.sim_avg_ms_f64,
                "ms",
                budgets.sim_avg_ms_f64,
            ),
            (
                "sim p99 ms",
                self.sim_p99_ms_f64,
                "ms",
                budgets.sim_p99_ms_f64,
            ),
            (
                "surface hitch p95 ms",
                self.hitch_p95_ms_f64,
                "ms",
                budgets.hitch_ms_f64,
            ),
            (
                "resident MB",
                self.resident_mb_f64,
                "MB",
                budgets.memory_mb_f64,
            ),
            (
                "cold start s",
                self.cold_start_s_f64,
                "s",
                budgets.cold_start_s_f64,
            ),
            ("shell ms", self.shell_ms_f64, "ms", budgets.frame_ms_f64),
        ];
        for (name, value_f64, unit, budget_f64) in rows {
            match Self::fraction_of(value_f64, budget_f64) {
                Ok(status) => {
                    let rgb_u8 = status.color_rgb_u8().to_array_u8();
                    let color = egui::Color32::from_rgb(rgb_u8[0], rgb_u8[1], rgb_u8[2]);
                    ui.colored_label(
                        color,
                        format!(
                            "{name} {value:.3} {unit} ({percent:.1}%) [{band}]",
                            value = value_f64,
                            percent = status.fraction_percent_f64(),
                            band = status.level().label()
                        ),
                    );
                }
                Err(error) => {
                    ui.label(format!("budget {name} error: {error}"));
                }
            }
        }
        ui.label(format!(
            "thermal tier={tier} (render-only)",
            tier = self.tier.label()
        ));
    }

    /// Check one sample in any unit.
    ///
    /// # Errors
    ///
    /// Returns [`BudgetError`] for a non-finite or negative value.
    const fn check_sample_f64(value_f64: f64) -> Result<(), BudgetError> {
        if !value_f64.is_finite() {
            return Err(BudgetError::NonFinite { value_f64 });
        }
        if value_f64 < 0.0 {
            return Err(BudgetError::Negative { value_f64 });
        }
        Ok(())
    }
}

impl Default for BudgetStrip {
    /// Default zeroed strip at the medium tier.
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::BudgetLevel;

    const SMOKE_FRAME_MS_F64: f64 = 8.0;
    const SMOKE_BUDGET_MS_F64: f64 = 32.0;
    const EXPECTED_QUARTER_F64: f64 = 0.25;
    const FRACTION_TOL_F64: f64 = 1e-12;

    fn smoke_budgets() -> BudgetDenominators {
        let Ok(budgets) = BudgetDenominators::new(32.0, 8.0, 16.0, 100.0, 1024.0, 5.0) else {
            panic!("smoke budgets must build")
        };
        budgets
    }

    #[test]
    fn fractions_match_known_quarters() {
        let mut strip = BudgetStrip::new();
        assert!(strip.set_frame_ms_f64(SMOKE_FRAME_MS_F64).is_ok());
        assert!(strip.set_sim_avg_ms_f64(2.0).is_ok());
        assert!(strip.set_shell_ms_f64(0.4).is_ok());
        strip.set_thermal_tier(ThermalTier::Low);
        assert_eq!(strip.thermal_tier(), ThermalTier::Low);
        let budgets = smoke_budgets();
        let Ok(status) = BudgetStrip::fraction_of(SMOKE_FRAME_MS_F64, SMOKE_BUDGET_MS_F64) else {
            panic!("frame fraction must classify")
        };
        assert!((status.fraction_ratio_f64() - EXPECTED_QUARTER_F64).abs() < FRACTION_TOL_F64);
        assert_eq!(status.level(), BudgetLevel::Nominal);
        let _ = budgets;
    }

    #[test]
    fn bands_turn_over_past_four_fifths() {
        let Ok(status) = BudgetStrip::fraction_of(30.0, 32.0) else {
            panic!("high fraction must classify")
        };
        assert_eq!(status.level(), BudgetLevel::Over);
    }

    #[test]
    fn rejects_bad_samples_and_budgets() {
        let mut strip = BudgetStrip::new();
        assert!(matches!(
            strip.set_frame_ms_f64(f64::NAN),
            Err(BudgetError::NonFinite { .. })
        ));
        assert!(matches!(
            strip.set_resident_mb_f64(-1.0),
            Err(BudgetError::Negative { .. })
        ));
        assert!(matches!(
            BudgetStrip::fraction_of(1.0, 0.0),
            Err(BudgetError::NonPositiveBudget { .. })
        ));
        assert!(matches!(
            BudgetDenominators::new(0.0, 8.0, 16.0, 100.0, 1024.0, 5.0),
            Err(BudgetError::NonPositiveBudget { .. })
        ));
        assert!(matches!(
            BudgetDenominators::new(32.0, f64::NAN, 16.0, 100.0, 1024.0, 5.0),
            Err(BudgetError::NonFinite { .. })
        ));
        assert_eq!(ThermalTier::ALL.len(), 3);
    }
}
