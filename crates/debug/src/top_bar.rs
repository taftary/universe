//! Top bar run control plus clocks and badges.
//!
//! Plain data over read-only snapshot scalars with typed warp rejects; never
//! writes sim state. Snapshot codes cite `crates/engine/src/inspect.rs`
//! constant names; warp policy reuses `engine::warp` read-only.

use engine::sim::SIM_TICK_S;
use engine::warp::{Warp, WarpContext, request_warp};

#[cfg(feature = "dev-shell")]
use crate::shell_cost::ShellCostMeter;
use crate::theme::{BudgetStatus, ThemeError};

/// Milliseconds per second for frame-rate display.
///
/// Source: time definition, 1 s equals 1000 ms.
pub const MILLIS_PER_SECOND_F64: f64 = 1000.0;

/// Low 16-bit mask for short seed and hash display.
///
/// Source: display truncation only, issue 34 step 4.
pub const SHORT_DISPLAY_MASK_U64: u64 = 0xFFFF;

/// X1 warp code, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `WARP_CODE_X1_U8`.
pub const TOP_BAR_WARP_CODE_X1_U8: u8 = 0;

/// X10 warp code, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `WARP_CODE_X10_U8`.
pub const TOP_BAR_WARP_CODE_X10_U8: u8 = 1;

/// X100 warp code, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `WARP_CODE_X100_U8`.
pub const TOP_BAR_WARP_CODE_X100_U8: u8 = 2;

/// X1000 warp code, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `WARP_CODE_X1000_U8`.
pub const TOP_BAR_WARP_CODE_X1000_U8: u8 = 3;

/// X10000 warp code, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `WARP_CODE_X10000_U8`.
pub const TOP_BAR_WARP_CODE_X10000_U8: u8 = 4;

/// Drop reason none or manual, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `DROP_REASON_NONE_U8`.
pub const TOP_BAR_DROP_NONE_U8: u8 = 0;

/// Drop reason atmospheric entry, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `DROP_REASON_ENTRY_U8`.
pub const TOP_BAR_DROP_ENTRY_U8: u8 = 1;

/// Drop reason approach, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `DROP_REASON_APPROACH_U8`.
pub const TOP_BAR_DROP_APPROACH_U8: u8 = 2;

/// Drop reason alarm, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `DROP_REASON_ALARM_U8`.
pub const TOP_BAR_DROP_ALARM_U8: u8 = 3;

/// Top-bar range-check and warp-policy failures.
///
/// Returned for bad clocks, bad costs, unknown codes, and denied warp.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TopBarError {
    /// Clock, cost, or budget value was non-finite.
    NonFinite {
        /// Rejected value.
        value_f64: f64,
    },
    /// Clock or cost value was negative.
    Negative {
        /// Rejected value.
        value_f64: f64,
    },
    /// Budget denominator was not strictly positive.
    NonPositiveBudget {
        /// Rejected budget value.
        value_f64: f64,
    },
    /// Warp code was outside `0` to `4`.
    InvalidWarpCode {
        /// Rejected code, dimensionless.
        code_u8: u8,
    },
    /// Drop-reason code was outside `0` to `3`.
    InvalidDropCode {
        /// Rejected code, dimensionless.
        code_u8: u8,
    },
    /// Warp above `1x` was denied outside a ship.
    WarpDenied {
        /// Denied factor, dimensionless.
        requested_factor_f64: f64,
    },
}

impl core::fmt::Display for TopBarError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NonFinite { value_f64 } => {
                write!(formatter, "non-finite top-bar value: {value_f64}")
            }
            Self::Negative { value_f64 } => {
                write!(formatter, "negative top-bar value: {value_f64}")
            }
            Self::NonPositiveBudget { value_f64 } => {
                write!(formatter, "non-positive top-bar budget: {value_f64}")
            }
            Self::InvalidWarpCode { code_u8 } => {
                write!(formatter, "invalid warp code: {code_u8}")
            }
            Self::InvalidDropCode { code_u8 } => {
                write!(formatter, "invalid drop-reason code: {code_u8}")
            }
            Self::WarpDenied {
                requested_factor_f64,
            } => {
                write!(
                    formatter,
                    "warp {requested_factor_f64}x denied outside ship"
                )
            }
        }
    }
}

impl std::error::Error for TopBarError {}

/// Warp auto-drop reason shown in the top bar.
///
/// `Manual` means no auto-drop is active and warp follows manual control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoDropReason {
    /// No auto-drop active; warp follows manual control.
    Manual,
    /// Auto-dropped on atmospheric entry.
    Entry,
    /// Auto-dropped on approach to a body or object.
    Approach,
    /// Auto-dropped on a physiological alarm.
    Alarm,
}

impl AutoDropReason {
    /// All auto-drop reasons.
    pub const ALL: [Self; 4] = [Self::Manual, Self::Entry, Self::Approach, Self::Alarm];

    /// Return the short reason label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Entry => "entry",
            Self::Approach => "approach",
            Self::Alarm => "alarm",
        }
    }

    /// Map a snapshot drop code to a reason.
    ///
    /// Code `0` maps to manual; codes follow the inspect drop constants.
    ///
    /// # Errors
    ///
    /// Returns [`TopBarError::InvalidDropCode`] when `code_u8` exceeds `3`.
    pub const fn from_drop_code(code_u8: u8) -> Result<Self, TopBarError> {
        match code_u8 {
            TOP_BAR_DROP_NONE_U8 => Ok(Self::Manual),
            TOP_BAR_DROP_ENTRY_U8 => Ok(Self::Entry),
            TOP_BAR_DROP_APPROACH_U8 => Ok(Self::Approach),
            TOP_BAR_DROP_ALARM_U8 => Ok(Self::Alarm),
            _ => Err(TopBarError::InvalidDropCode { code_u8 }),
        }
    }

    /// Report whether the reason reflects an automatic drop.
    #[must_use]
    pub const fn is_auto(self) -> bool {
        match self {
            Self::Manual => false,
            Self::Entry | Self::Approach | Self::Alarm => true,
        }
    }
}

/// One-line health ticker shown in the top bar.
///
/// Alarm display names come from the sim in later phases; this step shows
/// the generic alarm band only and never invents a specific alarm name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// Steady state with no events.
    Nominal,
    /// Atmospheric entry regime active.
    Entry,
    /// Approach to a body or object active.
    Approach,
    /// A sim-raised alarm is active.
    Alarm,
    /// Run was tainted by a write and leaves repeatability claims.
    Tainted,
    /// Clean replay from seed plus input log is running.
    Replaying,
}

impl HealthStatus {
    /// All health states.
    pub const ALL: [Self; 6] = [
        Self::Nominal,
        Self::Entry,
        Self::Approach,
        Self::Alarm,
        Self::Tainted,
        Self::Replaying,
    ];

    /// Return the short health label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Nominal => "nominal",
            Self::Entry => "entry",
            Self::Approach => "approach",
            Self::Alarm => "alarm",
            Self::Tainted => "tainted",
            Self::Replaying => "replaying",
        }
    }
}

/// Top-bar run control plus clocks and badges.
///
/// Plain data only; holds copies of snapshot scalars and never writes sim
/// state. Shell state only and never persists.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TopBarState {
    /// True while the fixed-step scheduler is held.
    paused: bool,
    /// True while a single-step tick is pending.
    step_pending: bool,
    /// Tick count, dimensionless.
    tick_count_u64: u64,
    /// Elapsed sim time in seconds.
    elapsed_s_f64: f64,
    /// Requested warp before auto-drop.
    requested_warp: Warp,
    /// Effective warp after auto-drop.
    effective_warp: Warp,
    /// Auto-drop reason for the effective warp.
    auto_drop_reason: AutoDropReason,
    /// Latest frame time in milliseconds.
    frame_ms_f64: f64,
    /// One-line health ticker.
    health: HealthStatus,
    /// Master seed, dimensionless.
    seed_u64: u64,
    /// Per-tick snapshot hash, dimensionless.
    snapshot_hash_u64: u64,
    /// False once a write taints the run.
    clean: bool,
    /// Latest shell draw cost in milliseconds.
    shell_draw_ms_f64: f64,
}

impl TopBarState {
    /// Build the default top bar at `1x` nominal.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            paused: false,
            step_pending: false,
            tick_count_u64: 0,
            elapsed_s_f64: 0.0,
            requested_warp: Warp::X1,
            effective_warp: Warp::X1,
            auto_drop_reason: AutoDropReason::Manual,
            frame_ms_f64: 0.0,
            health: HealthStatus::Nominal,
            seed_u64: 0,
            snapshot_hash_u64: 0,
            clean: true,
            shell_draw_ms_f64: 0.0,
        }
    }

    /// Hold the fixed-step scheduler.
    pub const fn pause(&mut self) {
        self.paused = true;
    }

    /// Release the fixed-step scheduler.
    pub const fn resume(&mut self) {
        self.paused = false;
    }

    /// Report whether the scheduler is held.
    #[must_use]
    pub const fn is_paused(&self) -> bool {
        self.paused
    }

    /// Request one fixed-step tick while paused.
    ///
    /// Sets a shell flag only; Step 5 consumes it in the scheduler.
    pub const fn request_step(&mut self) {
        self.step_pending = true;
    }

    /// Report whether a single-step tick is pending.
    #[must_use]
    pub const fn step_pending(&self) -> bool {
        self.step_pending
    }

    /// Consume the pending single-step flag.
    ///
    /// Returns true when a step was pending.
    pub const fn take_step(&mut self) -> bool {
        if self.step_pending {
            self.step_pending = false;
            true
        } else {
            false
        }
    }

    /// Request a warp factor under the sim rules.
    ///
    /// Higher factors need a ship in orbit or transit with no atmosphere,
    /// no approach, and no alarm. On success updates requested warp,
    /// effective warp, and the auto-drop display; on deny leaves state.
    ///
    /// # Errors
    ///
    /// Returns [`TopBarError::WarpDenied`] when the rules reject the request.
    pub fn request_warp(&mut self, requested: Warp, ctx: WarpContext) -> Result<Warp, TopBarError> {
        match request_warp(requested, ctx) {
            Ok(granted) => {
                self.requested_warp = requested;
                self.effective_warp = granted;
                self.auto_drop_reason = Self::reason_from_context(requested, ctx);
                Ok(granted)
            }
            Err(_) => Err(TopBarError::WarpDenied {
                requested_factor_f64: requested.factor(),
            }),
        }
    }

    /// Map a snapshot warp code to a warp factor.
    ///
    /// Codes follow the inspect warp constants from `0` to `4`.
    ///
    /// # Errors
    ///
    /// Returns [`TopBarError::InvalidWarpCode`] when `code_u8` exceeds `4`.
    pub const fn warp_from_code(code_u8: u8) -> Result<Warp, TopBarError> {
        match code_u8 {
            TOP_BAR_WARP_CODE_X1_U8 => Ok(Warp::X1),
            TOP_BAR_WARP_CODE_X10_U8 => Ok(Warp::X10),
            TOP_BAR_WARP_CODE_X100_U8 => Ok(Warp::X100),
            TOP_BAR_WARP_CODE_X1000_U8 => Ok(Warp::X1000),
            TOP_BAR_WARP_CODE_X10000_U8 => Ok(Warp::X10000),
            _ => Err(TopBarError::InvalidWarpCode { code_u8 }),
        }
    }

    /// Return the requested warp before auto-drop.
    #[must_use]
    pub const fn requested_warp(&self) -> Warp {
        self.requested_warp
    }

    /// Return the effective warp after auto-drop.
    #[must_use]
    pub const fn effective_warp(&self) -> Warp {
        self.effective_warp
    }

    /// Return the requested factor, dimensionless.
    #[must_use]
    pub const fn requested_factor_f64(&self) -> f64 {
        self.requested_warp.factor()
    }

    /// Return the effective factor, dimensionless.
    #[must_use]
    pub const fn effective_factor_f64(&self) -> f64 {
        self.effective_warp.factor()
    }

    /// Return the effective warp code, dimensionless.
    #[must_use]
    pub const fn effective_warp_code_u8(&self) -> u8 {
        match self.effective_warp {
            Warp::X1 => TOP_BAR_WARP_CODE_X1_U8,
            Warp::X10 => TOP_BAR_WARP_CODE_X10_U8,
            Warp::X100 => TOP_BAR_WARP_CODE_X100_U8,
            Warp::X1000 => TOP_BAR_WARP_CODE_X1000_U8,
            Warp::X10000 => TOP_BAR_WARP_CODE_X10000_U8,
        }
    }

    /// Return the auto-drop reason.
    #[must_use]
    pub const fn auto_drop_reason(&self) -> AutoDropReason {
        self.auto_drop_reason
    }

    /// Return the auto-drop reason label.
    #[must_use]
    pub const fn auto_drop_label(&self) -> &'static str {
        self.auto_drop_reason.label()
    }

    /// Set clocks from snapshot scalars.
    ///
    /// # Errors
    ///
    /// Returns [`TopBarError`] when `elapsed_s_f64` is non-finite or negative.
    pub fn set_clocks(
        &mut self,
        tick_count_u64: u64,
        elapsed_s_f64: f64,
    ) -> Result<(), TopBarError> {
        if !elapsed_s_f64.is_finite() {
            return Err(TopBarError::NonFinite {
                value_f64: elapsed_s_f64,
            });
        }
        if elapsed_s_f64 < 0.0 {
            return Err(TopBarError::Negative {
                value_f64: elapsed_s_f64,
            });
        }
        self.tick_count_u64 = tick_count_u64;
        self.elapsed_s_f64 = elapsed_s_f64;
        Ok(())
    }

    /// Return the tick count, dimensionless.
    #[must_use]
    pub const fn tick_count_u64(&self) -> u64 {
        self.tick_count_u64
    }

    /// Return elapsed sim time in seconds.
    #[must_use]
    pub const fn elapsed_s_f64(&self) -> f64 {
        self.elapsed_s_f64
    }

    /// Return the fixed step size in seconds.
    #[must_use]
    pub const fn tick_step_s_f64() -> f64 {
        SIM_TICK_S.value()
    }

    /// Set the latest frame time in milliseconds.
    ///
    /// # Errors
    ///
    /// Returns [`TopBarError`] when `frame_ms_f64` is non-finite or negative.
    pub fn set_frame(&mut self, frame_ms_f64: f64) -> Result<(), TopBarError> {
        if !frame_ms_f64.is_finite() {
            return Err(TopBarError::NonFinite {
                value_f64: frame_ms_f64,
            });
        }
        if frame_ms_f64 < 0.0 {
            return Err(TopBarError::Negative {
                value_f64: frame_ms_f64,
            });
        }
        self.frame_ms_f64 = frame_ms_f64;
        Ok(())
    }

    /// Return the latest frame time in milliseconds.
    #[must_use]
    pub const fn frame_ms_f64(&self) -> f64 {
        self.frame_ms_f64
    }

    /// Return the frame rate in frames per second.
    ///
    /// Returns zero when no frame time is recorded yet.
    #[must_use]
    pub const fn frame_fps_f64(&self) -> f64 {
        if self.frame_ms_f64 <= 0.0 {
            0.0
        } else {
            MILLIS_PER_SECOND_F64 / self.frame_ms_f64
        }
    }

    /// Return frame time as a fraction of a budget.
    ///
    /// Pass `FRAME_BUDGET_MS` as `budget_ms_f64`; gates live in quality docs.
    ///
    /// # Errors
    ///
    /// Returns [`TopBarError`] when `budget_ms_f64` is non-finite or not positive.
    pub fn frame_fraction_of_budget(&self, budget_ms_f64: f64) -> Result<f64, TopBarError> {
        Self::check_budget_ms_f64(budget_ms_f64)?;
        Ok(self.frame_ms_f64 / budget_ms_f64)
    }

    /// Return frame time with its budget band.
    ///
    /// Color never carries meaning alone; the numeric fraction always shows.
    ///
    /// # Errors
    ///
    /// Returns [`TopBarError`] for a bad budget or an unclassifiable fraction.
    pub fn frame_budget_status(&self, budget_ms_f64: f64) -> Result<BudgetStatus, TopBarError> {
        let fraction_ratio_f64 = self.frame_fraction_of_budget(budget_ms_f64)?;
        BudgetStatus::new(fraction_ratio_f64).map_err(|source: ThemeError| match source {
            ThemeError::NonFinite { value_f64 } => TopBarError::NonFinite { value_f64 },
            ThemeError::NegativeFraction { value_f64 } => TopBarError::Negative { value_f64 },
        })
    }

    /// Set the one-line health ticker.
    pub const fn set_health(&mut self, health: HealthStatus) {
        self.health = health;
    }

    /// Return the health ticker.
    #[must_use]
    pub const fn health(&self) -> HealthStatus {
        self.health
    }

    /// Return the health label.
    #[must_use]
    pub const fn health_label(&self) -> &'static str {
        self.health.label()
    }

    /// Set determinism badge values by copy.
    pub const fn set_determinism(&mut self, seed_u64: u64, snapshot_hash_u64: u64, clean: bool) {
        self.seed_u64 = seed_u64;
        self.snapshot_hash_u64 = snapshot_hash_u64;
        self.clean = clean;
    }

    /// Return the master seed, dimensionless.
    #[must_use]
    pub const fn seed_u64(&self) -> u64 {
        self.seed_u64
    }

    /// Return the per-tick snapshot hash, dimensionless.
    #[must_use]
    pub const fn snapshot_hash_u64(&self) -> u64 {
        self.snapshot_hash_u64
    }

    /// Report whether the run is clean.
    #[must_use]
    pub const fn is_clean(&self) -> bool {
        self.clean
    }

    /// Report whether the run is tainted.
    #[must_use]
    pub const fn is_tainted(&self) -> bool {
        !self.clean
    }

    /// Return the low 16 bits of the seed for short display.
    #[must_use]
    pub const fn seed_short_u16(&self) -> u16 {
        (self.seed_u64 & SHORT_DISPLAY_MASK_U64) as u16
    }

    /// Return the low 16 bits of the hash for short display.
    #[must_use]
    pub const fn hash_short_u16(&self) -> u16 {
        (self.snapshot_hash_u64 & SHORT_DISPLAY_MASK_U64) as u16
    }

    /// Mark the run tainted and raise the taint badge.
    ///
    /// Taint sticks; replay from a clean seed clears it in later phases.
    pub const fn mark_tainted(&mut self) {
        self.clean = false;
        self.health = HealthStatus::Tainted;
    }

    /// Set the latest shell draw cost in milliseconds.
    ///
    /// Mirrors the meter latest for badge display without dev-shell.
    ///
    /// # Errors
    ///
    /// Returns [`TopBarError`] when `draw_ms_f64` is non-finite or negative.
    pub fn set_shell_cost(&mut self, draw_ms_f64: f64) -> Result<(), TopBarError> {
        if !draw_ms_f64.is_finite() {
            return Err(TopBarError::NonFinite {
                value_f64: draw_ms_f64,
            });
        }
        if draw_ms_f64 < 0.0 {
            return Err(TopBarError::Negative {
                value_f64: draw_ms_f64,
            });
        }
        self.shell_draw_ms_f64 = draw_ms_f64;
        Ok(())
    }

    /// Return the shell draw cost in milliseconds.
    #[must_use]
    pub const fn shell_draw_ms_f64(&self) -> f64 {
        self.shell_draw_ms_f64
    }

    /// Return shell cost as a fraction of a budget.
    ///
    /// Pass `FRAME_BUDGET_MS` as `budget_ms_f64`; gates live in quality docs.
    ///
    /// # Errors
    ///
    /// Returns [`TopBarError`] when `budget_ms_f64` is non-finite or not positive.
    pub fn shell_fraction_of_budget(&self, budget_ms_f64: f64) -> Result<f64, TopBarError> {
        Self::check_budget_ms_f64(budget_ms_f64)?;
        Ok(self.shell_draw_ms_f64 / budget_ms_f64)
    }

    /// Observe snapshot scalars by copy without writing sim state.
    ///
    /// Maps warp and drop codes, copies clocks plus seed and hash, and
    /// promotes a nominal ticker to entry, approach, or alarm on drops.
    /// Never clears tainted or replaying health.
    ///
    /// # Errors
    ///
    /// Returns [`TopBarError`] for bad clocks or unknown warp and drop codes.
    pub fn observe_snapshot_view(
        &mut self,
        tick_count_u64: u64,
        elapsed_s_f64: f64,
        warp_code_u8: u8,
        drop_code_u8: u8,
        seed_u64: u64,
        snapshot_hash_u64: u64,
    ) -> Result<(), TopBarError> {
        let warp = Self::warp_from_code(warp_code_u8)?;
        let reason = AutoDropReason::from_drop_code(drop_code_u8)?;
        self.set_clocks(tick_count_u64, elapsed_s_f64)?;
        self.effective_warp = warp;
        self.auto_drop_reason = reason;
        self.seed_u64 = seed_u64;
        self.snapshot_hash_u64 = snapshot_hash_u64;
        if self.health == HealthStatus::Nominal {
            match reason {
                AutoDropReason::Entry => self.health = HealthStatus::Entry,
                AutoDropReason::Approach => self.health = HealthStatus::Approach,
                AutoDropReason::Alarm => self.health = HealthStatus::Alarm,
                AutoDropReason::Manual => {}
            }
        }
        Ok(())
    }

    /// Draw the single top row for run control and badges.
    ///
    /// Immediate-mode widgets in a top panel after the sim tick and
    /// the game pass; creates no renderer. Step 5 owns renderer creation
    /// plus the pass root `Ui`. `egui::Panel::top` is the 0.36 successor
    /// of `TopBottomPanel::top` named in the layer map. Available only
    /// with the non-default `dev-shell` feature.
    #[cfg(feature = "dev-shell")]
    pub fn draw(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        shell: &mut ShellCostMeter,
        frame_budget_ms_f64: f64,
    ) {
        ctx.set_visuals(crate::theme::dev_dark_pro_visuals());
        egui::Panel::top("universe-top-bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("pause").clicked() {
                    self.pause();
                }
                if ui.button("resume").clicked() {
                    self.resume();
                }
                if ui.button("step").clicked() {
                    self.request_step();
                }
                ui.label(format!(
                    "tick={tick} elapsed_s={elapsed:.2} warp={warp}x drop={drop} step_s={step}",
                    tick = self.tick_count_u64,
                    elapsed = self.elapsed_s_f64,
                    warp = self.effective_factor_f64(),
                    drop = self.auto_drop_label(),
                    step = Self::tick_step_s_f64()
                ));
                ui.label(format!(
                    "frame {ms:.2} ms {fps:.0} fps health={health}",
                    ms = self.frame_ms_f64,
                    fps = self.frame_fps_f64(),
                    health = self.health_label()
                ));
                ui.label(format!(
                    "seed={seed:04x} hash={hash:04x} {state}",
                    seed = self.seed_short_u16(),
                    hash = self.hash_short_u16(),
                    state = if self.clean { "clean" } else { "tainted" }
                ));
                shell.draw_inline(ui, frame_budget_ms_f64);
            });
        });
    }

    /// Derive the display reason from request plus context.
    ///
    /// Priority matches the inspect view: entry, then approach, then alarm.
    fn reason_from_context(requested: Warp, ctx: WarpContext) -> AutoDropReason {
        if requested == Warp::X1 || !ctx.should_auto_drop() {
            return AutoDropReason::Manual;
        }
        if ctx.in_atmosphere {
            return AutoDropReason::Entry;
        }
        if ctx.approaching {
            return AutoDropReason::Approach;
        }
        AutoDropReason::Alarm
    }

    /// Check a budget denominator in milliseconds.
    ///
    /// # Errors
    ///
    /// Returns [`TopBarError`] when `budget_ms_f64` is non-finite or not positive.
    const fn check_budget_ms_f64(budget_ms_f64: f64) -> Result<(), TopBarError> {
        if !budget_ms_f64.is_finite() {
            return Err(TopBarError::NonFinite {
                value_f64: budget_ms_f64,
            });
        }
        if budget_ms_f64 <= 0.0 {
            return Err(TopBarError::NonPositiveBudget {
                value_f64: budget_ms_f64,
            });
        }
        Ok(())
    }
}

impl Default for TopBarState {
    /// Default top bar at `1x` nominal.
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMOKE_TICK_COUNT_U64: u64 = 12;
    const SMOKE_ELAPSED_S_F64: f64 = 0.6;
    const SMOKE_FRAME_MS_F64: f64 = 8.0;
    const SMOKE_BUDGET_MS_F64: f64 = 32.0;
    const EXPECTED_QUARTER_F64: f64 = 0.25;
    const SMOKE_SEED_U64: u64 = 0x1234_ABCD_5678_EF90;
    const SMOKE_HASH_U64: u64 = 0xDEAD_BEEF_0000_4321;
    const EXPECTED_SEED_SHORT_U16: u16 = 0xEF90;
    const EXPECTED_HASH_SHORT_U16: u16 = 0x4321;
    const EXPECTED_X100_F64: f64 = 100.0;
    const FRACTION_TOL_F64: f64 = 1e-12;

    #[test]
    fn pause_resume_and_step_flags() {
        let mut bar = TopBarState::new();
        assert!(!bar.is_paused());
        assert!(!bar.step_pending());
        bar.pause();
        assert!(bar.is_paused());
        bar.request_step();
        assert!(bar.step_pending());
        assert!(bar.take_step());
        assert!(!bar.step_pending());
        assert!(!bar.take_step());
        bar.resume();
        assert!(!bar.is_paused());
    }

    #[test]
    fn warp_allow_and_typed_deny() {
        let mut bar = TopBarState::new();
        let Ok(granted) = bar.request_warp(Warp::X100, WarpContext::cruise()) else {
            panic!("cruise warp must grant")
        };
        assert_eq!(granted, Warp::X100);
        assert!((bar.effective_factor_f64() - EXPECTED_X100_F64).abs() < FRACTION_TOL_F64);
        assert_eq!(bar.auto_drop_reason(), AutoDropReason::Manual);
        assert!(matches!(
            bar.request_warp(Warp::X10, WarpContext::on_foot()),
            Err(TopBarError::WarpDenied { .. })
        ));
        assert_eq!(bar.effective_warp(), Warp::X100);
    }

    #[test]
    fn warp_auto_drop_reasons_follow_context() {
        let mut bar = TopBarState::new();
        let entry = WarpContext::new(true, true, true, false, false);
        assert!(entry.should_auto_drop());
        assert!(matches!(
            bar.request_warp(Warp::X100, entry),
            Err(TopBarError::WarpDenied { .. })
        ));
        let approach = WarpContext::new(true, true, false, true, false);
        assert!(approach.should_auto_drop());
        let alarm = WarpContext::new(true, true, false, false, true);
        assert!(alarm.should_auto_drop());
        assert!(!WarpContext::cruise().should_auto_drop());
        assert!(
            bar.observe_snapshot_view(
                SMOKE_TICK_COUNT_U64,
                SMOKE_ELAPSED_S_F64,
                TOP_BAR_WARP_CODE_X1_U8,
                TOP_BAR_DROP_ENTRY_U8,
                SMOKE_SEED_U64,
                SMOKE_HASH_U64,
            )
            .is_ok(),
            "entry snapshot must observe"
        );
        assert_eq!(bar.auto_drop_reason(), AutoDropReason::Entry);
        assert!(bar.auto_drop_reason().is_auto());
        assert_eq!(bar.health(), HealthStatus::Entry);
        assert!(!AutoDropReason::Manual.is_auto());
    }

    #[test]
    fn warp_and_drop_codes_map() {
        let Ok(warp) = TopBarState::warp_from_code(TOP_BAR_WARP_CODE_X10000_U8) else {
            panic!("x10000 code must map")
        };
        assert_eq!(warp, Warp::X10000);
        assert!(matches!(
            TopBarState::warp_from_code(9_u8),
            Err(TopBarError::InvalidWarpCode { .. })
        ));
        let Ok(reason) = AutoDropReason::from_drop_code(TOP_BAR_DROP_APPROACH_U8) else {
            panic!("approach code must map")
        };
        assert_eq!(reason, AutoDropReason::Approach);
        assert!(matches!(
            AutoDropReason::from_drop_code(9_u8),
            Err(TopBarError::InvalidDropCode { .. })
        ));
        assert_eq!(AutoDropReason::ALL.len(), 4);
        assert_eq!(HealthStatus::ALL.len(), 6);
    }

    #[test]
    fn clocks_frames_and_cost_fractions() {
        let mut bar = TopBarState::new();
        assert!(
            bar.set_clocks(SMOKE_TICK_COUNT_U64, SMOKE_ELAPSED_S_F64)
                .is_ok(),
            "smoke clocks must set"
        );
        assert_eq!(bar.tick_count_u64(), SMOKE_TICK_COUNT_U64);
        assert!(TopBarState::tick_step_s_f64() > 0.0);
        assert!(matches!(
            bar.set_clocks(0, f64::NAN),
            Err(TopBarError::NonFinite { .. })
        ));
        assert!(matches!(
            bar.set_clocks(0, -1.0),
            Err(TopBarError::Negative { .. })
        ));
        assert!(
            bar.set_frame(SMOKE_FRAME_MS_F64).is_ok(),
            "smoke frame must set"
        );
        let Ok(fraction_ratio_f64) = bar.frame_fraction_of_budget(SMOKE_BUDGET_MS_F64) else {
            panic!("frame fraction must divide")
        };
        assert!((fraction_ratio_f64 - EXPECTED_QUARTER_F64).abs() < FRACTION_TOL_F64);
        assert!(matches!(
            bar.frame_fraction_of_budget(0.0),
            Err(TopBarError::NonPositiveBudget { .. })
        ));
        assert!(
            bar.set_shell_cost(SMOKE_FRAME_MS_F64).is_ok(),
            "shell cost must set"
        );
        let Ok(shell_fraction_f64) = bar.shell_fraction_of_budget(SMOKE_BUDGET_MS_F64) else {
            panic!("shell fraction must divide")
        };
        assert!((shell_fraction_f64 - EXPECTED_QUARTER_F64).abs() < FRACTION_TOL_F64);
    }

    #[test]
    fn badges_show_seed_hash_and_taint() {
        let mut bar = TopBarState::new();
        bar.set_determinism(SMOKE_SEED_U64, SMOKE_HASH_U64, true);
        assert_eq!(bar.seed_u64(), SMOKE_SEED_U64);
        assert_eq!(bar.snapshot_hash_u64(), SMOKE_HASH_U64);
        assert!(bar.is_clean());
        assert!(!bar.is_tainted());
        assert_eq!(bar.seed_short_u16(), EXPECTED_SEED_SHORT_U16);
        assert_eq!(bar.hash_short_u16(), EXPECTED_HASH_SHORT_U16);
        bar.mark_tainted();
        assert!(bar.is_tainted());
        assert_eq!(bar.health(), HealthStatus::Tainted);
        assert_eq!(bar.health_label(), "tainted");
    }
}
