//! Shell draw-cost meter with budget fractions.
//!
//! Measures shell draw separately from sim and render cost in milliseconds,
//! reports fractions of a caller-provided budget by parameter, and removes
//! itself by close flag or by building without `dev-shell`. Read-only for
//! the sim; the meter never writes sim state.

use crate::theme::{BudgetStatus, ThemeError};

/// Shell sample ring capacity in entries at shell open.
///
/// Pre-sized at open and reused after warmup under the shell-only exemption.
/// Source: `docs/tech/debug.md` section 8.
pub const SHELL_SAMPLE_CAPACITY_ENTRIES_USIZE: usize = 256;

/// Shell-cost range-check failures.
///
/// Returned for non-finite costs, negative costs, or non-positive budgets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShellCostError {
    /// Draw cost or budget was non-finite.
    NonFinite {
        /// Rejected value.
        value_f64: f64,
    },
    /// Draw cost was negative.
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

impl core::fmt::Display for ShellCostError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NonFinite { value_f64 } => {
                write!(formatter, "non-finite shell cost: {value_f64}")
            }
            Self::Negative { value_f64 } => {
                write!(formatter, "negative shell cost: {value_f64}")
            }
            Self::NonPositiveBudget { value_f64 } => {
                write!(formatter, "non-positive shell budget: {value_f64}")
            }
        }
    }
}

impl std::error::Error for ShellCostError {}

/// Shell draw-cost meter with a pre-sized ring.
///
/// Buffers allocate once at open and reuse after warmup; shell state never
/// persists. Timed separately from sim and render cost and removable by
/// close flag or by building without `dev-shell`.
#[derive(Debug, Clone)]
pub struct ShellCostMeter {
    /// Ring of recent draw samples in milliseconds.
    samples_ms_f64: [f64; SHELL_SAMPLE_CAPACITY_ENTRIES_USIZE],
    /// Filled entries in the ring, dimensionless.
    len_usize: usize,
    /// Next write index into the ring, dimensionless.
    next_index_usize: usize,
    /// Latest draw cost in milliseconds.
    latest_draw_ms_f64: f64,
    /// True after the close-shell button is pressed.
    closed: bool,
}

impl ShellCostMeter {
    /// Build an empty meter with a pre-sized ring.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            samples_ms_f64: [0.0; SHELL_SAMPLE_CAPACITY_ENTRIES_USIZE],
            len_usize: 0,
            next_index_usize: 0,
            latest_draw_ms_f64: 0.0,
            closed: false,
        }
    }

    /// Return the ring capacity in entries.
    #[must_use]
    pub const fn capacity_usize() -> usize {
        SHELL_SAMPLE_CAPACITY_ENTRIES_USIZE
    }

    /// Return filled entries in the ring.
    #[must_use]
    pub const fn len_usize(&self) -> usize {
        self.len_usize
    }

    /// Report whether the ring holds no samples.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len_usize == 0
    }

    /// Return the latest draw cost in milliseconds.
    #[must_use]
    pub const fn latest_draw_ms_f64(&self) -> f64 {
        self.latest_draw_ms_f64
    }

    /// Report whether the close-shell button was pressed.
    #[must_use]
    pub const fn is_closed(&self) -> bool {
        self.closed
    }

    /// Record one draw sample in milliseconds.
    ///
    /// Overwrites the oldest entry once full; no allocation after open.
    ///
    /// # Errors
    ///
    /// Returns [`ShellCostError`] when `draw_ms_f64` is non-finite or negative.
    pub fn record_sample(&mut self, draw_ms_f64: f64) -> Result<(), ShellCostError> {
        if !draw_ms_f64.is_finite() {
            return Err(ShellCostError::NonFinite {
                value_f64: draw_ms_f64,
            });
        }
        if draw_ms_f64 < 0.0 {
            return Err(ShellCostError::Negative {
                value_f64: draw_ms_f64,
            });
        }
        self.samples_ms_f64[self.next_index_usize] = draw_ms_f64;
        self.next_index_usize = (self.next_index_usize + 1) % SHELL_SAMPLE_CAPACITY_ENTRIES_USIZE;
        if self.len_usize < SHELL_SAMPLE_CAPACITY_ENTRIES_USIZE {
            self.len_usize += 1;
        }
        self.latest_draw_ms_f64 = draw_ms_f64;
        Ok(())
    }

    /// Return the mean of filled entries in milliseconds.
    ///
    /// Returns zero when the ring holds no samples.
    #[must_use]
    pub fn average_ms_f64(&self) -> f64 {
        if self.len_usize == 0 {
            return 0.0;
        }
        let mut total_ms_f64 = 0.0;
        let mut count_f64 = 0.0;
        for index_usize in 0..self.len_usize {
            total_ms_f64 += self.samples_ms_f64[index_usize];
            count_f64 += 1.0;
        }
        total_ms_f64 / count_f64
    }

    /// Return latest draw cost as a fraction of a budget.
    ///
    /// Pass `FRAME_BUDGET_MS` as `budget_ms_f64`; gates live in quality docs.
    ///
    /// # Errors
    ///
    /// Returns [`ShellCostError`] when `budget_ms_f64` is non-finite or not positive.
    pub fn fraction_of_budget(&self, budget_ms_f64: f64) -> Result<f64, ShellCostError> {
        if !budget_ms_f64.is_finite() {
            return Err(ShellCostError::NonFinite {
                value_f64: budget_ms_f64,
            });
        }
        if budget_ms_f64 <= 0.0 {
            return Err(ShellCostError::NonPositiveBudget {
                value_f64: budget_ms_f64,
            });
        }
        Ok(self.latest_draw_ms_f64 / budget_ms_f64)
    }

    /// Return latest draw cost with its budget band.
    ///
    /// Color never carries meaning alone; the numeric fraction always shows.
    ///
    /// # Errors
    ///
    /// Returns [`ShellCostError`] for a bad budget or an unclassifiable fraction.
    pub fn budget_status(&self, budget_ms_f64: f64) -> Result<BudgetStatus, ShellCostError> {
        let fraction_ratio_f64 = self.fraction_of_budget(budget_ms_f64)?;
        BudgetStatus::new(fraction_ratio_f64).map_err(|source: ThemeError| match source {
            ThemeError::NonFinite { value_f64 } => ShellCostError::NonFinite { value_f64 },
            ThemeError::NegativeFraction { value_f64 } => ShellCostError::Negative { value_f64 },
        })
    }

    /// Mark the shell closed via the close-shell button.
    pub const fn request_close(&mut self) {
        self.closed = true;
    }

    /// Reopen the shell after a close.
    pub const fn reopen(&mut self) {
        self.closed = false;
    }

    /// Draw shell cost with fraction plus close button.
    ///
    /// Immediate-mode widgets only; creates no renderer. Step 5 owns renderer
    /// creation. Skips everything when closed. Available only with `dev-shell`.
    #[cfg(feature = "dev-shell")]
    pub fn draw_inline(&mut self, ui: &mut egui::Ui, budget_ms_f64: f64) {
        if self.closed {
            return;
        }
        match self.budget_status(budget_ms_f64) {
            Ok(status) => {
                let rgb_u8 = status.color_rgb_u8().to_array_u8();
                let color = egui::Color32::from_rgb(rgb_u8[0], rgb_u8[1], rgb_u8[2]);
                ui.colored_label(
                    color,
                    format!(
                        "shell {ms:.3} ms ({percent:.1}%) [{band}]",
                        ms = self.latest_draw_ms_f64,
                        percent = status.fraction_percent_f64(),
                        band = status.level().label()
                    ),
                );
            }
            Err(error) => {
                ui.label(format!("shell cost error: {error}"));
            }
        }
        if ui.button("close shell").clicked() {
            self.closed = true;
        }
    }
}

impl Default for ShellCostMeter {
    /// Default meter with an empty pre-sized ring.
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMOKE_DRAW_MS_F64: f64 = 0.4;
    const SMOKE_BUDGET_MS_F64: f64 = 2.0;
    const EXPECTED_FRACTION_F64: f64 = 0.2;
    const FRACTION_TOL_F64: f64 = 1e-12;

    #[test]
    fn cost_fraction_divides_by_budget() {
        let mut meter = ShellCostMeter::new();
        assert!(meter.is_empty());
        assert_eq!(
            ShellCostMeter::capacity_usize(),
            SHELL_SAMPLE_CAPACITY_ENTRIES_USIZE
        );
        assert!(
            meter.record_sample(SMOKE_DRAW_MS_F64).is_ok(),
            "smoke sample must record"
        );
        assert!(!meter.is_empty());
        assert_eq!(meter.len_usize(), 1);
        let Ok(fraction_ratio_f64) = meter.fraction_of_budget(SMOKE_BUDGET_MS_F64) else {
            panic!("smoke fraction must divide")
        };
        assert!((fraction_ratio_f64 - EXPECTED_FRACTION_F64).abs() < FRACTION_TOL_F64);
        let Ok(status) = meter.budget_status(SMOKE_BUDGET_MS_F64) else {
            panic!("smoke status must classify")
        };
        assert!((status.fraction_ratio_f64() - EXPECTED_FRACTION_F64).abs() < FRACTION_TOL_F64);
        assert!(
            (meter.average_ms_f64() - SMOKE_DRAW_MS_F64).abs() < FRACTION_TOL_F64,
            "average of one sample must equal it"
        );
    }

    #[test]
    fn cost_rejects_bad_values() {
        let mut meter = ShellCostMeter::new();
        assert!(matches!(
            meter.record_sample(f64::NAN),
            Err(ShellCostError::NonFinite { .. })
        ));
        assert!(matches!(
            meter.record_sample(-0.5),
            Err(ShellCostError::Negative { .. })
        ));
        assert!(
            meter.record_sample(SMOKE_DRAW_MS_F64).is_ok(),
            "smoke sample must record"
        );
        assert!(matches!(
            meter.fraction_of_budget(0.0),
            Err(ShellCostError::NonPositiveBudget { .. })
        ));
        assert!(matches!(
            meter.fraction_of_budget(f64::NAN),
            Err(ShellCostError::NonFinite { .. })
        ));
    }

    #[test]
    fn close_flag_removes_shell() {
        let mut meter = ShellCostMeter::new();
        assert!(!meter.is_closed());
        meter.request_close();
        assert!(meter.is_closed());
        meter.reopen();
        assert!(!meter.is_closed());
    }

    #[test]
    fn ring_reuses_buffer_at_capacity() {
        let mut meter = ShellCostMeter::new();
        let capacity_usize = ShellCostMeter::capacity_usize();
        let mut sample_ms_f64 = 1.0;
        for _ in 0..(capacity_usize + 3) {
            assert!(
                meter.record_sample(sample_ms_f64).is_ok(),
                "ring sample must record"
            );
            sample_ms_f64 += 1.0;
        }
        assert_eq!(meter.len_usize(), capacity_usize);
        assert_eq!(
            ShellCostMeter::capacity_usize(),
            SHELL_SAMPLE_CAPACITY_ENTRIES_USIZE
        );
    }
}
