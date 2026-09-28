//! Thermal-bundle: render-only tier stepping plus section-11 companions.
//!
//! Shared phone-plus-desktop policy for the Step 2c flight bundle: a
//! 2.0 s thermal poll with stepwise High-to-Medium-to-Low downgrade,
//! immediate Low on Serious or Critical with an instrument-grade notice,
//! plus the `system.txt` companion and the log-excerpt join for the
//! section-11 bundle layout. Tier changes never touch sim state; only
//! render scale reads the tier. Shell state only and never persists;
//! callers write files through [`export`](crate::export).

use crate::budget::ThermalTier;
use crate::flight_log::ThermalState;

/// Thermal poll interval in seconds (`THERMAL_POLL_S`).
///
/// Source: `docs/tech/mobile.md` thermal management.
pub const THERMAL_POLL_S_F64: f64 = 2.0;

/// Log-excerpt lines kept before the export tick, dimensionless.
///
/// Contract mirror: the ring literals live in
/// [`TraceLog::excerpt_around`](crate::log::TraceLog::excerpt_around);
/// this constant pins the section-11 25-before count for bundle tests.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Bundle contract seam; unit tests cover the excerpt count meanwhile."
    )
)]
pub const LOG_EXCERPT_BEFORE_LINES_USIZE: usize = 25;

/// Log-excerpt lines kept after the export tick, dimensionless.
///
/// Contract mirror: same owner and test pin as
/// [`LOG_EXCERPT_BEFORE_LINES_USIZE`].
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Bundle contract seam; unit tests cover the excerpt count meanwhile."
    )
)]
pub const LOG_EXCERPT_AFTER_LINES_USIZE: usize = 25;

/// Instrument notice posted when Serious heat forces Low, dimensionless text.
///
/// Source: `docs/tech/mobile.md` Apple Serious-tier pattern, which asks
/// for an instrument-grade message, never a blocking dialog.
pub const THERMAL_SERIOUS_NOTICE_TEXT: &str = "thermal serious: render tier low (render-only)";

/// Instrument notice posted when Critical heat forces Low, dimensionless text.
///
/// Source: same Serious-tier pattern as [`THERMAL_SERIOUS_NOTICE_TEXT`].
pub const THERMAL_CRITICAL_NOTICE_TEXT: &str = "thermal critical: render tier low (render-only)";

/// Thermal-bundle range-check failures.
///
/// Returned for non-finite clocks or costs, or for a poll time behind
/// the latched clock.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BundleError {
    /// Clock or cost was non-finite.
    NonFinite {
        /// Rejected value.
        value_f64: f64,
    },
    /// Clock delta or cost was negative.
    Negative {
        /// Rejected value.
        value_f64: f64,
    },
}

impl core::fmt::Display for BundleError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NonFinite { value_f64 } => {
                write!(formatter, "non-finite bundle value: {value_f64}")
            }
            Self::Negative { value_f64 } => {
                write!(formatter, "negative bundle value: {value_f64}")
            }
        }
    }
}

impl std::error::Error for BundleError {}

/// Step one thermal state onto the next render-only tier.
///
/// Serious plus Critical drop to Low immediately; Fair steps down one
/// tier (High to Medium, Medium to Low); Nominal holds. The tier never
/// rises by itself; upgrades are manual only.
#[must_use]
pub const fn next_tier_for_thermal_state(current: ThermalTier, state: ThermalState) -> ThermalTier {
    match state {
        ThermalState::Serious | ThermalState::Critical => ThermalTier::Low,
        ThermalState::Fair => match current {
            ThermalTier::High => ThermalTier::Medium,
            ThermalTier::Medium | ThermalTier::Low => ThermalTier::Low,
        },
        ThermalState::Nominal => current,
    }
}

/// Return the instrument notice when heat forces Low, if any.
///
/// Returns `Some` only on the transition into Low from Serious or
/// Critical; Fair steps and Nominal holds never notify. The caller
/// posts the line as an instrument readout, never a dialog.
#[must_use]
pub const fn thermal_notice_for(
    state: ThermalState,
    previous: ThermalTier,
    next: ThermalTier,
) -> Option<&'static str> {
    match (state, previous, next) {
        (ThermalState::Serious, ThermalTier::High | ThermalTier::Medium, ThermalTier::Low) => {
            Some(THERMAL_SERIOUS_NOTICE_TEXT)
        }
        (ThermalState::Critical, ThermalTier::High | ThermalTier::Medium, ThermalTier::Low) => {
            Some(THERMAL_CRITICAL_NOTICE_TEXT)
        }
        _ => None,
    }
}

/// One thermal observation with tier plus one-shot notice.
///
/// Plain data; the caller applies the tier render-only and posts the
/// notice as an instrument line, never a dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThermalNotice {
    /// Next render-only tier.
    tier: ThermalTier,
    /// True only on the poll that forced Low.
    forced_low_bool: bool,
    /// Instrument notice when Low was forced, if any.
    notice: Option<&'static str>,
}

impl ThermalNotice {
    /// Return the next render-only tier.
    #[must_use]
    pub const fn tier(self) -> ThermalTier {
        self.tier
    }

    /// Report whether this poll forced Low.
    #[must_use]
    pub const fn forced_low_bool(self) -> bool {
        self.forced_low_bool
    }

    /// Return the instrument notice, if any.
    #[must_use]
    pub const fn notice(self) -> Option<&'static str> {
        self.notice
    }
}

/// Render-only thermal controller with a 2.0 s poll clock.
///
/// Starts at Medium (reference phone); Fair steps down one tier and
/// Serious or Critical force Low with a one-shot instrument notice.
/// The controller never raises the tier by itself and never writes
/// sim state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThermalController {
    /// Current render-only tier.
    tier: ThermalTier,
    /// Latest observed thermal state.
    state: ThermalState,
    /// Last poll time in seconds.
    last_poll_s_f64: f64,
    /// True once Serious or Critical forced Low this session.
    did_force_low_bool: bool,
}

impl ThermalController {
    /// Build a controller at Medium with no forced downgrade.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            tier: ThermalTier::Medium,
            state: ThermalState::Nominal,
            last_poll_s_f64: 0.0,
            did_force_low_bool: false,
        }
    }

    /// Return the poll interval in seconds.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Bundle contract seam; unit tests cover the poll cadence meanwhile."
        )
    )]
    #[must_use]
    pub const fn interval_s_f64() -> f64 {
        THERMAL_POLL_S_F64
    }

    /// Return the current render-only tier.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Bundle contract seam; unit tests cover the tier stepping meanwhile."
        )
    )]
    #[must_use]
    pub const fn tier(self) -> ThermalTier {
        self.tier
    }

    /// Return the latest observed thermal state.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Bundle contract seam; unit tests cover the thermal state meanwhile."
        )
    )]
    #[must_use]
    pub const fn state(self) -> ThermalState {
        self.state
    }

    /// Return the last poll time in seconds.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Bundle contract seam; unit tests cover the poll clock meanwhile."
        )
    )]
    #[must_use]
    pub const fn last_poll_s_f64(self) -> f64 {
        self.last_poll_s_f64
    }

    /// Report whether Low was ever forced this session.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Bundle contract seam; unit tests cover the forced-Low flag meanwhile."
        )
    )]
    #[must_use]
    pub const fn did_force_low_bool(self) -> bool {
        self.did_force_low_bool
    }

    /// Report whether a poll is due and latch the clock.
    ///
    /// True once per 2.0 s interval; monotonic wall time only.
    ///
    /// # Errors
    ///
    /// Returns [`BundleError`] for a non-finite time or a time behind
    /// the last poll.
    pub fn should_poll(&mut self, now_s_f64: f64) -> Result<bool, BundleError> {
        if !now_s_f64.is_finite() {
            return Err(BundleError::NonFinite {
                value_f64: now_s_f64,
            });
        }
        let delta_s_f64 = now_s_f64 - self.last_poll_s_f64;
        if delta_s_f64 < 0.0 {
            return Err(BundleError::Negative {
                value_f64: delta_s_f64,
            });
        }
        if delta_s_f64 >= THERMAL_POLL_S_F64 {
            self.last_poll_s_f64 = now_s_f64;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Observe one thermal state and step the tier.
    ///
    /// Applies [`next_tier_for_thermal_state`] plus
    /// [`thermal_notice_for`]; latches the session forced-Low flag on
    /// the transition only. Render-only; sim behavior is unchanged.
    pub fn observe(&mut self, state: ThermalState) -> ThermalNotice {
        let previous = self.tier;
        let next = next_tier_for_thermal_state(previous, state);
        let notice = thermal_notice_for(state, previous, next);
        let forced_low_bool = notice.is_some();
        if forced_low_bool {
            self.did_force_low_bool = true;
        }
        self.tier = next;
        self.state = state;
        ThermalNotice {
            tier: next,
            forced_low_bool,
            notice,
        }
    }
}

impl Default for ThermalController {
    /// Default controller at Medium with no forced downgrade.
    fn default() -> Self {
        Self::new()
    }
}

/// Check one millisecond cost for the system companion.
fn check_cost_ms_f64(value_f64: f64) -> Result<(), BundleError> {
    if !value_f64.is_finite() {
        return Err(BundleError::NonFinite { value_f64 });
    }
    if value_f64 < 0.0 {
        return Err(BundleError::Negative { value_f64 });
    }
    Ok(())
}

/// Quote a TOML string with escapes without taking the buffer.
///
/// Allocates once per export file; never called in the frame loop.
/// Mirrors [`export`](crate::export) quoting so companions agree.
fn quoted(value: &str) -> String {
    let mut text = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => text.push_str("\\\""),
            '\\' => text.push_str("\\\\"),
            '\n' => text.push_str("\\n"),
            '\r' => text.push_str("\\r"),
            '\t' => text.push_str("\\t"),
            _ => text.push(ch),
        }
    }
    text.push('"');
    text
}

/// Format the section-11 `system.txt` companion for one bundle.
///
/// Fixed field order: device model, OS floor, backend, thermal tier,
/// poll cadence, frame plus shell costs, export tick. Costs carry
/// millisecond units; the tick is dimensionless.
///
/// # Errors
///
/// Returns [`BundleError`] for a non-finite or negative cost.
pub fn format_system_txt(
    device_model: &str,
    os_floor: &str,
    backend: &str,
    tier: ThermalTier,
    frame_ms_f64: f64,
    shell_avg_ms_f64: f64,
    tick_count_u64: u64,
) -> Result<String, BundleError> {
    check_cost_ms_f64(frame_ms_f64)?;
    check_cost_ms_f64(shell_avg_ms_f64)?;
    Ok(format!(
        "device_model = {device}\nos_floor = {os}\nbackend = {backend}\ntier = \"{tier}\"\nthermal_poll_s = {poll}\nframe_ms = {frame:.3}\nshell_avg_ms = {shell:.3}\ntick = {tick}\n",
        device = quoted(device_model),
        os = quoted(os_floor),
        backend = quoted(backend),
        tier = tier.label(),
        poll = THERMAL_POLL_S_F64,
        frame = frame_ms_f64,
        shell = shell_avg_ms_f64,
        tick = tick_count_u64,
    ))
}

/// Join log-excerpt lines into `log_excerpt.txt` text.
///
/// Lines come from
/// [`TraceLog::excerpt_around`](crate::log::TraceLog::excerpt_around)
/// (25 before plus 25 after the export tick); this join only adds the
/// trailing newline. Allocates once per export; never called in the
/// frame loop.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Bundle contract seam; unit tests cover the excerpt join meanwhile."
    )
)]
#[must_use]
pub fn join_log_excerpt(lines: &[String]) -> String {
    if lines.is_empty() {
        return String::new();
    }
    let mut text = lines.join("\n");
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMOKE_FRAME_MS_F64: f64 = 8.0;
    const SMOKE_SHELL_MS_F64: f64 = 0.4;
    const SMOKE_TICK_U64: u64 = 42;
    const POLL_TOL_S_F64: f64 = 1e-12;

    #[test]
    fn poll_interval_matches_mobile_and_android() {
        assert!((THERMAL_POLL_S_F64 - 2.0).abs() < POLL_TOL_S_F64);
        assert!((ThermalController::interval_s_f64() - 2.0).abs() < POLL_TOL_S_F64);
        assert!(
            (THERMAL_POLL_S_F64 - crate::android::ANDROID_THERMAL_POLL_INTERVAL_S_F64).abs()
                < POLL_TOL_S_F64
        );
        assert_eq!(ThermalController::default(), ThermalController::new());
    }

    #[test]
    fn tier_steps_down_before_throttling() {
        use ThermalState::{Critical, Fair, Nominal, Serious};
        use ThermalTier::{High, Low, Medium};
        for (current, state, expected) in [
            (High, Fair, Medium),
            (Medium, Fair, Low),
            (Low, Fair, Low),
            (High, Serious, Low),
            (Medium, Serious, Low),
            (Low, Serious, Low),
            (High, Critical, Low),
            (Medium, Critical, Low),
            (High, Nominal, High),
            (Medium, Nominal, Medium),
            (Low, Nominal, Low),
        ] {
            assert_eq!(
                next_tier_for_thermal_state(current, state),
                expected,
                "tier {current:?} on {state:?} must step to {expected:?}"
            );
        }
    }

    #[test]
    fn serious_and_critical_agree_with_android_immediate_low() {
        for state in [ThermalState::Serious, ThermalState::Critical] {
            assert_eq!(
                crate::android::tier_for_thermal_state(state),
                ThermalTier::Low
            );
            assert_eq!(
                next_tier_for_thermal_state(ThermalTier::High, state),
                ThermalTier::Low
            );
            assert_eq!(
                next_tier_for_thermal_state(ThermalTier::Medium, state),
                ThermalTier::Low
            );
        }
    }

    #[test]
    fn serious_and_critical_post_one_shot_notice() {
        let serious =
            thermal_notice_for(ThermalState::Serious, ThermalTier::Medium, ThermalTier::Low);
        assert_eq!(serious, Some(THERMAL_SERIOUS_NOTICE_TEXT));
        let critical =
            thermal_notice_for(ThermalState::Critical, ThermalTier::High, ThermalTier::Low);
        assert_eq!(critical, Some(THERMAL_CRITICAL_NOTICE_TEXT));
        for line in [serious, critical].into_iter().flatten() {
            assert!(line.contains("render-only"));
            assert!(line.contains("low"));
        }
        assert_eq!(
            thermal_notice_for(ThermalState::Fair, ThermalTier::High, ThermalTier::Medium),
            None
        );
        assert_eq!(
            thermal_notice_for(
                ThermalState::Nominal,
                ThermalTier::Medium,
                ThermalTier::Medium
            ),
            None
        );
        assert_eq!(
            thermal_notice_for(ThermalState::Serious, ThermalTier::Low, ThermalTier::Low),
            None
        );
    }

    #[test]
    fn controller_polls_every_two_seconds() {
        let mut controller = ThermalController::new();
        assert_eq!(controller.tier(), ThermalTier::Medium);
        assert_eq!(controller.state(), ThermalState::Nominal);
        assert!((controller.last_poll_s_f64() - 0.0).abs() < POLL_TOL_S_F64);
        assert!(!controller.did_force_low_bool());
        assert_eq!(controller.should_poll(0.0), Ok(false));
        assert_eq!(controller.should_poll(1.99), Ok(false));
        assert_eq!(controller.should_poll(2.0), Ok(true));
        assert!((controller.last_poll_s_f64() - 2.0).abs() < POLL_TOL_S_F64);
        assert_eq!(controller.should_poll(3.99), Ok(false));
        assert_eq!(controller.should_poll(4.0), Ok(true));
        assert!(matches!(
            controller.should_poll(f64::NAN),
            Err(BundleError::NonFinite { .. })
        ));
        assert!(matches!(
            controller.should_poll(3.0),
            Err(BundleError::Negative { .. })
        ));
    }

    #[test]
    fn controller_forces_low_once_then_holds_without_upgrade() {
        let mut controller = ThermalController::new();
        let first = controller.observe(ThermalState::Serious);
        assert_eq!(first.tier(), ThermalTier::Low);
        assert!(first.forced_low_bool());
        assert_eq!(first.notice(), Some(THERMAL_SERIOUS_NOTICE_TEXT));
        assert!(controller.did_force_low_bool());
        let second = controller.observe(ThermalState::Serious);
        assert_eq!(second.tier(), ThermalTier::Low);
        assert!(!second.forced_low_bool());
        assert_eq!(second.notice(), None);
        let held = controller.observe(ThermalState::Nominal);
        assert_eq!(held.tier(), ThermalTier::Low);
        assert!(!held.forced_low_bool());
        assert_eq!(held.notice(), None);
    }

    #[test]
    fn system_txt_carries_device_os_backend_tier() {
        let Ok(text) = format_system_txt(
            "smoke-device",
            "Android 26",
            "vulkan",
            ThermalTier::Low,
            SMOKE_FRAME_MS_F64,
            SMOKE_SHELL_MS_F64,
            SMOKE_TICK_U64,
        ) else {
            panic!("system companion must format")
        };
        assert!(text.contains("device_model = \"smoke-device\"\n"));
        assert!(text.contains("os_floor = \"Android 26\"\n"));
        assert!(text.contains("backend = \"vulkan\"\n"));
        assert!(text.contains("tier = \"low\"\n"));
        assert!(text.contains("thermal_poll_s = 2\n"));
        assert!(text.contains("frame_ms = 8.000\n"));
        assert!(text.contains("shell_avg_ms = 0.400\n"));
        assert!(text.contains("tick = 42\n"));
        assert!(matches!(
            format_system_txt(
                "d",
                "o",
                "b",
                ThermalTier::Low,
                f64::NAN,
                SMOKE_SHELL_MS_F64,
                SMOKE_TICK_U64,
            ),
            Err(BundleError::NonFinite { .. })
        ));
        assert!(matches!(
            format_system_txt(
                "d",
                "o",
                "b",
                ThermalTier::Low,
                SMOKE_FRAME_MS_F64,
                -0.5,
                SMOKE_TICK_U64,
            ),
            Err(BundleError::Negative { .. })
        ));
    }

    #[test]
    fn excerpt_keeps_25_before_and_after() {
        use crate::log::{LogLevel, TraceLog};
        let mut log = TraceLog::new();
        for tick_u64 in 0..61_u64 {
            assert!(
                log.push(tick_u64, LogLevel::Info, "sim", "tick").is_ok(),
                "excerpt push must succeed"
            );
        }
        let lines = log.excerpt_around(30);
        assert_eq!(
            lines.len(),
            LOG_EXCERPT_BEFORE_LINES_USIZE + LOG_EXCERPT_AFTER_LINES_USIZE + 1
        );
        let text = join_log_excerpt(&lines);
        assert!(text.starts_with("tick=5 info sim: tick\n"));
        assert!(text.trim_end().ends_with("tick=55 info sim: tick"));
        assert!(text.ends_with('\n'));
        assert_eq!(join_log_excerpt(&[]), String::new());
    }

    #[test]
    fn bundle_layout_matches_debug_section_11() {
        assert_eq!(
            crate::export::BUNDLE_FILE_NAMES,
            [
                "meta.toml",
                "seed_tree.toml",
                "inputs.csv",
                "hashes.csv",
                "snapshot.toml",
                "config.toml",
                "log_excerpt.txt",
                "system.txt",
            ]
        );
    }
}
