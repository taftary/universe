//! Flight log with append-only CSV for phone runs.
//!
//! Records five budget channels plus thermal tier per observed frame
//! in a pre-sized ring, then formats append-only CSV at export time.
//! Fractions reuse [`BudgetStrip::fraction_of`](crate::budget::BudgetStrip::fraction_of)
//! with caller-passed [`BudgetDenominators`](crate::budget::BudgetDenominators);
//! the ring pattern mirrors [`TraceLog`](crate::log::TraceLog). Shell state
//! only and never persists; the caller writes the file.

use crate::budget::{BudgetDenominators, BudgetError, BudgetStrip, ThermalTier};
use crate::theme::BudgetStatus;

/// Flight-log reservation in entries at shell open.
///
/// Holds a 15-minute phone run at 1 Hz with margin; matches the
/// plot-history reservation scale. Source: issue #56 Step 1 design
/// plus `docs/tech/debug.md` section 8 pre-sized-buffer exemption.
pub const FLIGHT_LOG_CAPACITY_ENTRIES_USIZE: usize = 2_048;

/// Flight-log CSV header in fixed column order.
///
/// Implements the Step 1 schema (`tick_u64`, `elapsed_s`, `frame_ms`,
/// `sim_avg_ms`, `sim_p99_ms`, `hitch_p95_ms`, `resident_mb`,
/// `thermal_state`, `tier`, `warp`, `seed`, `hash`) with unit suffixes
/// per `docs/tech/standards.md`; `thermal_state` plus `tier` carry labels.
pub const FLIGHT_LOG_HEADER: &str = "tick_count_u64,elapsed_s_f64,frame_ms_f64,sim_avg_ms_f64,sim_p99_ms_f64,hitch_p95_ms_f64,resident_mb_f64,thermal_state,tier,warp_factor_f64,seed_u64,hash_u64\n";

/// Flight-log channel count, dimensionless.
///
/// Five budget channels: frame, sim average, sim p99, hitch p95, resident.
pub const FLIGHT_CHANNEL_COUNT_USIZE: usize = 5;

/// Frame channel index into the fraction array, dimensionless.
pub const FLIGHT_CHANNEL_FRAME_USIZE: usize = 0;

/// Sim average channel index into the fraction array, dimensionless.
pub const FLIGHT_CHANNEL_SIM_AVG_USIZE: usize = 1;

/// Sim p99 channel index into the fraction array, dimensionless.
pub const FLIGHT_CHANNEL_SIM_P99_USIZE: usize = 2;

/// Hitch p95 channel index into the fraction array, dimensionless.
pub const FLIGHT_CHANNEL_HITCH_USIZE: usize = 3;

/// Resident memory channel index into the fraction array, dimensionless.
pub const FLIGHT_CHANNEL_RESIDENT_USIZE: usize = 4;

/// Flight-log range-check failures.
///
/// Returned for non-finite samples, negative samples and clocks, or a
/// non-positive warp factor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FlightLogError {
    /// Value was non-finite and unusable.
    NonFinite {
        /// Rejected value.
        value_f64: f64,
    },
    /// Value was negative and below zero.
    Negative {
        /// Rejected value.
        value_f64: f64,
    },
    /// Warp factor was not strictly positive.
    NonPositiveWarp {
        /// Rejected warp factor, dimensionless.
        value_f64: f64,
    },
}

impl core::fmt::Display for FlightLogError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NonFinite { value_f64 } => {
                write!(formatter, "non-finite flight log value: {value_f64}")
            }
            Self::Negative { value_f64 } => {
                write!(formatter, "negative flight log value: {value_f64}")
            }
            Self::NonPositiveWarp { value_f64 } => {
                write!(formatter, "non-positive warp factor: {value_f64}")
            }
        }
    }
}

impl std::error::Error for FlightLogError {}

/// Operating-system thermal state at one flight sample.
///
/// Portable view over `ProcessInfo.thermalState` plus Android thermal
/// status; `Serious` triggers the immediate Low-tier drop. Source:
/// `docs/tech/mobile.md` thermal management.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermalState {
    /// Nominal operating state.
    Nominal,
    /// Fair warming state.
    Fair,
    /// Serious heat; drop to Low immediately.
    Serious,
    /// Critical heat state.
    Critical,
}

impl ThermalState {
    /// All thermal states in severity order.
    pub const ALL: [Self; 4] = [Self::Nominal, Self::Fair, Self::Serious, Self::Critical];

    /// Return the short state label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Nominal => "nominal",
            Self::Fair => "fair",
            Self::Serious => "serious",
            Self::Critical => "critical",
        }
    }
}

/// One flight sample with five channels plus tier.
///
/// Plain data only; samples arrive by copy and denominators arrive at
/// draw or export time. Shell state only and never persists.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlightSample {
    /// Tick count, dimensionless.
    tick_count_u64: u64,
    /// Elapsed sim time in seconds.
    elapsed_s_f64: f64,
    /// Frame time in milliseconds.
    frame_ms_f64: f64,
    /// Sim tick average in milliseconds.
    sim_avg_ms_f64: f64,
    /// Sim tick p99 in milliseconds.
    sim_p99_ms_f64: f64,
    /// Surface hitch p95 in milliseconds.
    hitch_p95_ms_f64: f64,
    /// Resident memory in megabytes.
    resident_mb_f64: f64,
    /// Operating-system thermal state.
    thermal_state: ThermalState,
    /// Render-only thermal tier.
    tier: ThermalTier,
    /// Warp factor, dimensionless.
    warp_factor_f64: f64,
    /// Master seed, dimensionless.
    seed_u64: u64,
    /// Snapshot hash, dimensionless.
    hash_u64: u64,
}

impl FlightSample {
    /// Build a sample from tick, clocks, channels, tier, and determinism.
    ///
    /// # Errors
    ///
    /// Returns [`FlightLogError`] for a non-finite or negative clock or
    /// channel, or a non-finite or non-positive warp factor.
    #[expect(
        clippy::too_many_arguments,
        reason = "one sample carries the full 12-column schema"
    )]
    pub fn new(
        tick_count_u64: u64,
        elapsed_s_f64: f64,
        frame_ms_f64: f64,
        sim_avg_ms_f64: f64,
        sim_p99_ms_f64: f64,
        hitch_p95_ms_f64: f64,
        resident_mb_f64: f64,
        thermal_state: ThermalState,
        tier: ThermalTier,
        warp_factor_f64: f64,
        seed_u64: u64,
        hash_u64: u64,
    ) -> Result<Self, FlightLogError> {
        check_clock_s_f64(elapsed_s_f64)?;
        check_sample_ms_f64(frame_ms_f64)?;
        check_sample_ms_f64(sim_avg_ms_f64)?;
        check_sample_ms_f64(sim_p99_ms_f64)?;
        check_sample_ms_f64(hitch_p95_ms_f64)?;
        check_sample_mb_f64(resident_mb_f64)?;
        check_warp_factor_f64(warp_factor_f64)?;
        Ok(Self {
            tick_count_u64,
            elapsed_s_f64,
            frame_ms_f64,
            sim_avg_ms_f64,
            sim_p99_ms_f64,
            hitch_p95_ms_f64,
            resident_mb_f64,
            thermal_state,
            tier,
            warp_factor_f64,
            seed_u64,
            hash_u64,
        })
    }

    /// Return the tick count, dimensionless.
    #[must_use]
    pub const fn tick_count_u64(self) -> u64 {
        self.tick_count_u64
    }

    /// Return elapsed sim time in seconds.
    #[must_use]
    pub const fn elapsed_s_f64(self) -> f64 {
        self.elapsed_s_f64
    }

    /// Return frame time in milliseconds.
    #[must_use]
    pub const fn frame_ms_f64(self) -> f64 {
        self.frame_ms_f64
    }

    /// Return sim tick average in milliseconds.
    #[must_use]
    pub const fn sim_avg_ms_f64(self) -> f64 {
        self.sim_avg_ms_f64
    }

    /// Return sim tick p99 in milliseconds.
    #[must_use]
    pub const fn sim_p99_ms_f64(self) -> f64 {
        self.sim_p99_ms_f64
    }

    /// Return surface hitch p95 in milliseconds.
    #[must_use]
    pub const fn hitch_p95_ms_f64(self) -> f64 {
        self.hitch_p95_ms_f64
    }

    /// Return resident memory in megabytes.
    #[must_use]
    pub const fn resident_mb_f64(self) -> f64 {
        self.resident_mb_f64
    }

    /// Return the operating-system thermal state.
    #[must_use]
    pub const fn thermal_state(self) -> ThermalState {
        self.thermal_state
    }

    /// Return the render-only thermal tier.
    #[must_use]
    pub const fn tier(self) -> ThermalTier {
        self.tier
    }

    /// Return the warp factor, dimensionless.
    #[must_use]
    pub const fn warp_factor_f64(self) -> f64 {
        self.warp_factor_f64
    }

    /// Return the master seed, dimensionless.
    #[must_use]
    pub const fn seed_u64(self) -> u64 {
        self.seed_u64
    }

    /// Return the snapshot hash, dimensionless.
    #[must_use]
    pub const fn hash_u64(self) -> u64 {
        self.hash_u64
    }
}

/// Check one elapsed clock in seconds.
fn check_clock_s_f64(value_f64: f64) -> Result<(), FlightLogError> {
    if !value_f64.is_finite() {
        return Err(FlightLogError::NonFinite { value_f64 });
    }
    if value_f64 < 0.0 {
        return Err(FlightLogError::Negative { value_f64 });
    }
    Ok(())
}

/// Check one millisecond channel sample.
fn check_sample_ms_f64(value_f64: f64) -> Result<(), FlightLogError> {
    if !value_f64.is_finite() {
        return Err(FlightLogError::NonFinite { value_f64 });
    }
    if value_f64 < 0.0 {
        return Err(FlightLogError::Negative { value_f64 });
    }
    Ok(())
}

/// Check one megabyte channel sample.
fn check_sample_mb_f64(value_f64: f64) -> Result<(), FlightLogError> {
    if !value_f64.is_finite() {
        return Err(FlightLogError::NonFinite { value_f64 });
    }
    if value_f64 < 0.0 {
        return Err(FlightLogError::Negative { value_f64 });
    }
    Ok(())
}

/// Check one warp factor, dimensionless.
fn check_warp_factor_f64(value_f64: f64) -> Result<(), FlightLogError> {
    if !value_f64.is_finite() {
        return Err(FlightLogError::NonFinite { value_f64 });
    }
    if value_f64 <= 0.0 {
        return Err(FlightLogError::NonPositiveWarp { value_f64 });
    }
    Ok(())
}

/// Append-only flight log with a pre-sized ring.
///
/// Buffers allocate once at open and reuse after warmup; log state
/// never persists. Fractions take denominators at draw or export time
/// and are never stored; gates live in `docs/tech/quality.md`.
#[derive(Debug, Clone)]
pub struct FlightLog {
    /// Pre-sized sample ring, oldest overwritten once full.
    samples: Vec<FlightSample>,
    /// Next write index into the ring, dimensionless.
    next_index_usize: usize,
}

impl FlightLog {
    /// Build an empty log with pre-sized buffers.
    ///
    /// Reserves the flight-log capacity; no allocation happens after
    /// warmup. Shell state only and never persists.
    #[must_use]
    pub fn new() -> Self {
        Self {
            samples: Vec::with_capacity(FLIGHT_LOG_CAPACITY_ENTRIES_USIZE),
            next_index_usize: 0,
        }
    }

    /// Return filled sample count.
    #[must_use]
    pub fn len_usize(&self) -> usize {
        self.samples.len()
    }

    /// Report whether no samples are stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Clear samples, keeping the reservation for reuse.
    ///
    /// No allocation; called on fresh runs outside the frame loop.
    pub fn clear(&mut self) {
        self.samples.clear();
        self.next_index_usize = 0;
    }

    /// Push one validated sample, overwriting oldest when full.
    ///
    /// No allocation after open; range checks live in
    /// [`FlightSample::new`].
    pub fn push(&mut self, sample: FlightSample) {
        if self.samples.len() < FLIGHT_LOG_CAPACITY_ENTRIES_USIZE {
            self.samples.push(sample);
        } else {
            self.samples[self.next_index_usize] = sample;
            self.next_index_usize = (self.next_index_usize + 1) % FLIGHT_LOG_CAPACITY_ENTRIES_USIZE;
        }
    }

    /// Build plus push one reading from raw channels.
    ///
    /// Validates clocks, channels, and warp before appending; the ring
    /// still overwrites oldest when full with no allocation.
    ///
    /// # Errors
    ///
    /// Returns [`FlightLogError`] for a bad clock, channel, or warp.
    #[expect(
        clippy::too_many_arguments,
        reason = "one reading carries the full 12-column schema"
    )]
    pub fn push_reading(
        &mut self,
        tick_count_u64: u64,
        elapsed_s_f64: f64,
        frame_ms_f64: f64,
        sim_avg_ms_f64: f64,
        sim_p99_ms_f64: f64,
        hitch_p95_ms_f64: f64,
        resident_mb_f64: f64,
        thermal_state: ThermalState,
        tier: ThermalTier,
        warp_factor_f64: f64,
        seed_u64: u64,
        hash_u64: u64,
    ) -> Result<(), FlightLogError> {
        let sample = FlightSample::new(
            tick_count_u64,
            elapsed_s_f64,
            frame_ms_f64,
            sim_avg_ms_f64,
            sim_p99_ms_f64,
            hitch_p95_ms_f64,
            resident_mb_f64,
            thermal_state,
            tier,
            warp_factor_f64,
            seed_u64,
            hash_u64,
        )?;
        self.push(sample);
        Ok(())
    }

    /// Return the latest sample when one exists.
    #[must_use]
    pub fn latest(&self) -> Option<FlightSample> {
        if self.samples.is_empty() {
            return None;
        }
        if self.samples.len() < FLIGHT_LOG_CAPACITY_ENTRIES_USIZE {
            return self.samples.last().copied();
        }
        let newest_index_usize = (self.next_index_usize + FLIGHT_LOG_CAPACITY_ENTRIES_USIZE - 1)
            % FLIGHT_LOG_CAPACITY_ENTRIES_USIZE;
        self.samples.get(newest_index_usize).copied()
    }

    /// Classify five channels as fractions of named budgets.
    ///
    /// Reuses [`BudgetStrip::fraction_of`] per channel in
    /// `frame, sim_avg, sim_p99, hitch, resident` order; denominators
    /// arrive by name at call time and are never stored.
    ///
    /// # Errors
    ///
    /// Returns [`BudgetError`] for a bad sample or denominator.
    pub fn fractions_of(
        sample: FlightSample,
        budgets: BudgetDenominators,
    ) -> Result<[BudgetStatus; FLIGHT_CHANNEL_COUNT_USIZE], BudgetError> {
        Ok([
            BudgetStrip::fraction_of(sample.frame_ms_f64(), budgets.frame_ms_f64())?,
            BudgetStrip::fraction_of(sample.sim_avg_ms_f64(), budgets.sim_avg_ms_f64())?,
            BudgetStrip::fraction_of(sample.sim_p99_ms_f64(), budgets.sim_p99_ms_f64())?,
            BudgetStrip::fraction_of(sample.hitch_p95_ms_f64(), budgets.hitch_ms_f64())?,
            BudgetStrip::fraction_of(sample.resident_mb_f64(), budgets.memory_mb_f64())?,
        ])
    }

    /// Format one sample as a CSV row in header order.
    ///
    /// Allocates once per export; never called in the frame loop.
    #[must_use]
    pub fn format_row_csv(sample: FlightSample) -> String {
        format!(
            "{tick},{elapsed},{frame},{sim_avg},{sim_p99},{hitch},{resident},{thermal},{tier},{warp},{seed},{hash:016x}\n",
            tick = sample.tick_count_u64(),
            elapsed = sample.elapsed_s_f64(),
            frame = sample.frame_ms_f64(),
            sim_avg = sample.sim_avg_ms_f64(),
            sim_p99 = sample.sim_p99_ms_f64(),
            hitch = sample.hitch_p95_ms_f64(),
            resident = sample.resident_mb_f64(),
            thermal = sample.thermal_state().label(),
            tier = sample.tier().label(),
            warp = sample.warp_factor_f64(),
            seed = sample.seed_u64(),
            hash = sample.hash_u64(),
        )
    }

    /// Format header plus rows oldest-first as append-only CSV.
    ///
    /// Allocates once per export; never called in the frame loop.
    #[must_use]
    pub fn format_csv(&self) -> String {
        let (head, tail) = self.samples.split_at(self.next_index_usize);
        let mut text = String::from(FLIGHT_LOG_HEADER);
        // Filling ring reads insertion order; a full ring reads oldest
        // first from the write index, mirroring the trace-log ring.
        if self.samples.len() < FLIGHT_LOG_CAPACITY_ENTRIES_USIZE {
            for sample in &self.samples {
                text.push_str(&Self::format_row_csv(*sample));
            }
        } else {
            for sample in tail.iter().chain(head.iter()) {
                text.push_str(&Self::format_row_csv(*sample));
            }
        }
        text
    }

    /// Draw the latest fractions plus tier for one frame.
    ///
    /// Immediate-mode widgets only; creates no renderer. Color never
    /// carries meaning alone; every channel shows fraction plus percent
    /// plus band. Available only with the non-default `dev-shell`
    /// feature.
    #[cfg(feature = "dev-shell")]
    pub fn draw(&self, ui: &mut egui::Ui, budgets: BudgetDenominators) {
        let Some(sample) = self.latest() else {
            ui.label("flight log: no samples yet");
            return;
        };
        let rows: [(&str, f64, &str, f64); FLIGHT_CHANNEL_COUNT_USIZE] = [
            (
                "frame ms",
                sample.frame_ms_f64(),
                "ms",
                budgets.frame_ms_f64(),
            ),
            (
                "sim avg ms",
                sample.sim_avg_ms_f64(),
                "ms",
                budgets.sim_avg_ms_f64(),
            ),
            (
                "sim p99 ms",
                sample.sim_p99_ms_f64(),
                "ms",
                budgets.sim_p99_ms_f64(),
            ),
            (
                "surface hitch p95 ms",
                sample.hitch_p95_ms_f64(),
                "ms",
                budgets.hitch_ms_f64(),
            ),
            (
                "resident MB",
                sample.resident_mb_f64(),
                "MB",
                budgets.memory_mb_f64(),
            ),
        ];
        for (name, value_f64, unit, budget_f64) in rows {
            match BudgetStrip::fraction_of(value_f64, budget_f64) {
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
                    )
                    .on_hover_text(format!(
                        "{name} budget {budget_f64:.3} {unit}; raw value {value_f64:.6}"
                    ));
                }
                Err(error) => {
                    ui.label(format!("flight log {name} error: {error}"));
                }
            }
        }
        ui.label(format!(
            "thermal state={thermal} tier={tier} tick={tick} warp={warp}x (render-only)",
            thermal = sample.thermal_state().label(),
            tier = sample.tier().label(),
            tick = sample.tick_count_u64(),
            warp = sample.warp_factor_f64()
        ));
    }
}

impl Default for FlightLog {
    /// Default empty log with pre-sized buffers.
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::BudgetLevel;

    const SMOKE_TICK_U64: u64 = 7;
    const SMOKE_ELAPSED_S_F64: f64 = 0.35;
    const SMOKE_FRAME_MS_F64: f64 = 8.0;
    const SMOKE_SIM_AVG_MS_F64: f64 = 2.0;
    const SMOKE_SIM_P99_MS_F64: f64 = 4.0;
    const SMOKE_HITCH_MS_F64: f64 = 10.0;
    const SMOKE_RESIDENT_MB_F64: f64 = 256.0;
    const SMOKE_WARP_F64: f64 = 1.0;
    const SMOKE_SEED_U64: u64 = 0x1234_ABCD_5678_EF90;
    const SMOKE_HASH_U64: u64 = 0xDEAD_BEEF_0000_4321;
    const FRACTION_TOL_F64: f64 = 1e-12;

    /// Smoke denominators by name; gates live in `docs/tech/quality.md`.
    fn smoke_budgets() -> BudgetDenominators {
        let Ok(budgets) = BudgetDenominators::new(33.33, 8.0, 16.0, 100.0, 1024.0, 5.0) else {
            panic!("smoke budgets must build")
        };
        budgets
    }

    fn smoke_sample() -> FlightSample {
        let Ok(sample) = FlightSample::new(
            SMOKE_TICK_U64,
            SMOKE_ELAPSED_S_F64,
            SMOKE_FRAME_MS_F64,
            SMOKE_SIM_AVG_MS_F64,
            SMOKE_SIM_P99_MS_F64,
            SMOKE_HITCH_MS_F64,
            SMOKE_RESIDENT_MB_F64,
            ThermalState::Nominal,
            ThermalTier::Medium,
            SMOKE_WARP_F64,
            SMOKE_SEED_U64,
            SMOKE_HASH_U64,
        ) else {
            panic!("smoke sample must build")
        };
        sample
    }

    #[test]
    fn header_matches_twelve_column_schema() {
        assert_eq!(
            FLIGHT_LOG_HEADER,
            "tick_count_u64,elapsed_s_f64,frame_ms_f64,sim_avg_ms_f64,sim_p99_ms_f64,hitch_p95_ms_f64,resident_mb_f64,thermal_state,tier,warp_factor_f64,seed_u64,hash_u64\n"
        );
        assert_eq!(FLIGHT_CHANNEL_COUNT_USIZE, 5);
        assert_eq!(ThermalState::ALL.len(), 4);
        assert_eq!(ThermalTier::ALL.len(), 3);
    }

    #[test]
    fn push_records_and_reports_latest() {
        let mut log = FlightLog::new();
        assert!(log.is_empty());
        assert_eq!(log.len_usize(), 0);
        assert!(log.latest().is_none());
        assert!(
            log.push_reading(
                SMOKE_TICK_U64,
                SMOKE_ELAPSED_S_F64,
                SMOKE_FRAME_MS_F64,
                SMOKE_SIM_AVG_MS_F64,
                SMOKE_SIM_P99_MS_F64,
                SMOKE_HITCH_MS_F64,
                SMOKE_RESIDENT_MB_F64,
                ThermalState::Nominal,
                ThermalTier::Medium,
                SMOKE_WARP_F64,
                SMOKE_SEED_U64,
                SMOKE_HASH_U64,
            )
            .is_ok()
        );
        assert_eq!(log.len_usize(), 1);
        let Some(latest) = log.latest() else {
            panic!("latest sample must exist")
        };
        assert_eq!(latest.tick_count_u64(), SMOKE_TICK_U64);
        assert_eq!(latest.thermal_state(), ThermalState::Nominal);
        assert_eq!(latest.tier(), ThermalTier::Medium);
        assert_eq!(latest.seed_u64(), SMOKE_SEED_U64);
        assert_eq!(latest.hash_u64(), SMOKE_HASH_U64);
        assert_eq!(ThermalState::Serious.label(), "serious");
        log.clear();
        assert!(log.is_empty());
    }

    #[test]
    fn fractions_divide_by_named_budgets() {
        let sample = smoke_sample();
        let budgets = smoke_budgets();
        let Ok(fractions) = FlightLog::fractions_of(sample, budgets) else {
            panic!("fractions must classify")
        };
        assert_eq!(fractions.len(), FLIGHT_CHANNEL_COUNT_USIZE);
        let expected_f64 = [
            SMOKE_FRAME_MS_F64 / 33.33,
            SMOKE_SIM_AVG_MS_F64 / 8.0,
            SMOKE_SIM_P99_MS_F64 / 16.0,
            SMOKE_HITCH_MS_F64 / 100.0,
            SMOKE_RESIDENT_MB_F64 / 1024.0,
        ];
        for (status, expected) in fractions.iter().zip(expected_f64.iter()) {
            assert!((status.fraction_ratio_f64() - expected).abs() < FRACTION_TOL_F64);
        }
        assert_eq!(
            fractions[FLIGHT_CHANNEL_FRAME_USIZE].level(),
            BudgetLevel::Nominal
        );
        assert_eq!(
            fractions[FLIGHT_CHANNEL_RESIDENT_USIZE].level(),
            BudgetLevel::Nominal
        );
    }

    #[test]
    fn csv_appends_header_plus_row() {
        let mut log = FlightLog::new();
        assert!(
            log.push_reading(
                SMOKE_TICK_U64,
                SMOKE_ELAPSED_S_F64,
                SMOKE_FRAME_MS_F64,
                SMOKE_SIM_AVG_MS_F64,
                SMOKE_SIM_P99_MS_F64,
                SMOKE_HITCH_MS_F64,
                SMOKE_RESIDENT_MB_F64,
                ThermalState::Serious,
                ThermalTier::Low,
                SMOKE_WARP_F64,
                SMOKE_SEED_U64,
                SMOKE_HASH_U64,
            )
            .is_ok()
        );
        let csv = log.format_csv();
        assert!(csv.starts_with(FLIGHT_LOG_HEADER));
        assert!(csv.contains("serious,low,"));
        assert!(csv.contains("deadbeef00004321"));
        let row = FlightSample::new(
            SMOKE_TICK_U64,
            SMOKE_ELAPSED_S_F64,
            SMOKE_FRAME_MS_F64,
            SMOKE_SIM_AVG_MS_F64,
            SMOKE_SIM_P99_MS_F64,
            SMOKE_HITCH_MS_F64,
            SMOKE_RESIDENT_MB_F64,
            ThermalState::Nominal,
            ThermalTier::High,
            SMOKE_WARP_F64,
            SMOKE_SEED_U64,
            SMOKE_HASH_U64,
        );
        let Ok(single) = row else {
            panic!("row sample must build")
        };
        assert!(FlightLog::format_row_csv(single).contains("nominal,high,"));
    }

    #[test]
    fn rejects_bad_clocks_channels_and_warp() {
        let mut log = FlightLog::new();
        assert!(matches!(
            FlightSample::new(
                SMOKE_TICK_U64,
                f64::NAN,
                SMOKE_FRAME_MS_F64,
                SMOKE_SIM_AVG_MS_F64,
                SMOKE_SIM_P99_MS_F64,
                SMOKE_HITCH_MS_F64,
                SMOKE_RESIDENT_MB_F64,
                ThermalState::Nominal,
                ThermalTier::Medium,
                SMOKE_WARP_F64,
                SMOKE_SEED_U64,
                SMOKE_HASH_U64,
            ),
            Err(FlightLogError::NonFinite { .. })
        ));
        assert!(matches!(
            FlightSample::new(
                SMOKE_TICK_U64,
                SMOKE_ELAPSED_S_F64,
                -1.0,
                SMOKE_SIM_AVG_MS_F64,
                SMOKE_SIM_P99_MS_F64,
                SMOKE_HITCH_MS_F64,
                SMOKE_RESIDENT_MB_F64,
                ThermalState::Nominal,
                ThermalTier::Medium,
                SMOKE_WARP_F64,
                SMOKE_SEED_U64,
                SMOKE_HASH_U64,
            ),
            Err(FlightLogError::Negative { .. })
        ));
        assert!(matches!(
            FlightSample::new(
                SMOKE_TICK_U64,
                SMOKE_ELAPSED_S_F64,
                SMOKE_FRAME_MS_F64,
                SMOKE_SIM_AVG_MS_F64,
                SMOKE_SIM_P99_MS_F64,
                SMOKE_HITCH_MS_F64,
                SMOKE_RESIDENT_MB_F64,
                ThermalState::Nominal,
                ThermalTier::Medium,
                0.0,
                SMOKE_SEED_U64,
                SMOKE_HASH_U64,
            ),
            Err(FlightLogError::NonPositiveWarp { .. })
        ));
        assert!(matches!(
            log.push_reading(
                SMOKE_TICK_U64,
                SMOKE_ELAPSED_S_F64,
                f64::INFINITY,
                SMOKE_SIM_AVG_MS_F64,
                SMOKE_SIM_P99_MS_F64,
                SMOKE_HITCH_MS_F64,
                SMOKE_RESIDENT_MB_F64,
                ThermalState::Nominal,
                ThermalTier::Medium,
                SMOKE_WARP_F64,
                SMOKE_SEED_U64,
                SMOKE_HASH_U64,
            ),
            Err(FlightLogError::NonFinite { .. })
        ));
        assert!(log.is_empty());
    }

    #[test]
    fn ring_reuses_buffer_at_capacity() {
        let mut log = FlightLog::new();
        for tick_u64 in 0..(FLIGHT_LOG_CAPACITY_ENTRIES_USIZE as u64 + 5) {
            assert!(
                log.push_reading(
                    tick_u64,
                    SMOKE_ELAPSED_S_F64,
                    SMOKE_FRAME_MS_F64,
                    SMOKE_SIM_AVG_MS_F64,
                    SMOKE_SIM_P99_MS_F64,
                    SMOKE_HITCH_MS_F64,
                    SMOKE_RESIDENT_MB_F64,
                    ThermalState::Nominal,
                    ThermalTier::Medium,
                    SMOKE_WARP_F64,
                    SMOKE_SEED_U64,
                    SMOKE_HASH_U64,
                )
                .is_ok(),
                "ring push must succeed"
            );
        }
        assert_eq!(log.len_usize(), FLIGHT_LOG_CAPACITY_ENTRIES_USIZE);
        let Some(latest) = log.latest() else {
            panic!("latest sample must exist")
        };
        assert_eq!(
            latest.tick_count_u64(),
            FLIGHT_LOG_CAPACITY_ENTRIES_USIZE as u64 + 4
        );
        let csv = log.format_csv();
        assert!(csv.starts_with(FLIGHT_LOG_HEADER));
    }
}
