//! Android flight shell: frame pacer, thread plan, thermal poll, SDK probe, capture.
//!
//! Headless-testable policy for the on-device `universe-debug` flight binary:
//! a 30 fps frame pacer with drop counter, one sim plus one render thread
//! with a worker pool sized to available parallelism minus two, a 2.0 s
//! `PowerManager` thermal poll with render-only tier downgrade, a minSdk 26
//! probe with fallback, and a 1 Hz 15-minute CSV capture plan. The
//! `GameActivity` packaging plus `cargo-ndk` build live in
//! `platform/android/`; the JNI event loop lands in Step 2c. No `winit`, no
//! `wgpu`, no `egui`, no file IO here; callers inject clocks and threads.

use crate::budget::ThermalTier;
use crate::flight_log::{FLIGHT_LOG_CAPACITY_ENTRIES_USIZE, ThermalState};
use crate::top_bar::MILLIS_PER_SECOND_F64;

/// Minimum Android API level, dimensionless.
///
/// Source: `docs/tech/mobile.md` OS floors (`minSdk = 26`).
pub const ANDROID_MIN_SDK_API_U32: u32 = 26;

/// Packaging target Android API level, dimensionless.
///
/// Source: project choice recorded here; `platform/android/` mirrors it.
pub const ANDROID_TARGET_SDK_API_U32: u32 = 34;

/// API level introducing thermal status polling, dimensionless.
///
/// `PowerManager.getCurrentThermalStatus` arrives here; older levels use the
/// probe plus fallback path. Source: Android SDK docs.
pub const ANDROID_THERMAL_STATUS_API_U32: u32 = 29;

/// Frame budget in milliseconds (`FRAME_BUDGET_MS`).
///
/// Source: `docs/tech/mobile.md` frame pacer plus `docs/tech/quality.md`
/// budget table (30 fps sustained floor).
pub const ANDROID_FRAME_BUDGET_MS_F64: f64 = 33.33;

/// Thermal poll interval in seconds (`THERMAL_POLL_S`).
///
/// Source: `docs/tech/mobile.md` thermal management.
pub const ANDROID_THERMAL_POLL_INTERVAL_S_F64: f64 = 2.0;

/// Flight capture cadence in seconds between samples.
///
/// 1 Hz capture for the phone run. Source: issue #56 Step 2a (AC1).
pub const ANDROID_FLIGHT_CAPTURE_INTERVAL_S_F64: f64 = 1.0;

/// Flight capture duration in seconds.
///
/// 15-minute sustained session. Source: `docs/tech/quality.md` thermal row.
pub const ANDROID_FLIGHT_CAPTURE_DURATION_S_F64: f64 = 900.0;

/// Flight capture minimum sample count, dimensionless.
///
/// 900 seconds at 1 Hz. Source: issue #56 AC1 (5 channels plus tier).
pub const ANDROID_FLIGHT_CAPTURE_MIN_SAMPLES_USIZE: usize = 900;

/// Maximum accepted capture gap in seconds.
///
/// Runs with a gap above this fail AC1. Source: issue #56 AC1.
pub const ANDROID_MAX_CAPTURE_GAP_S_F64: f64 = 5.0;

/// Sim threads on the flight binary, dimensionless.
///
/// Source: `docs/tech/mobile.md` few-core guidance (one sim thread).
pub const ANDROID_SIM_THREADS_USIZE: usize = 1;

/// Render threads on the flight binary, dimensionless.
///
/// Source: `docs/tech/mobile.md` few-core guidance (one render thread).
pub const ANDROID_RENDER_THREADS_USIZE: usize = 1;

/// Minimum generation-pool workers, dimensionless.
///
/// Source: `docs/tech/mobile.md` few-core guidance (pool floor).
pub const ANDROID_MIN_WORKER_THREADS_USIZE: usize = 1;

/// Reserved threads before sizing the worker pool, dimensionless.
///
/// Pool size is available parallelism minus this reserve.
/// Source: `docs/tech/mobile.md` few-core guidance.
pub const ANDROID_WORKER_RESERVE_THREADS_USIZE: usize = 2;

/// Usable cores assumed at most on the Low tier, dimensionless.
///
/// Never assume more than this on Low. Source: `docs/tech/mobile.md`.
pub const ANDROID_LOW_TIER_CORES_USIZE: usize = 4;

/// Maximum sim steps drained per frame, dimensionless.
///
/// Same warp catch-up bound as the desktop window; excess time is
/// dropped, never spiraled. Source: `crates/debug/src/os_window.rs`.
pub const ANDROID_MAX_SIM_STEPS_PER_FRAME_U32: u32 = 600;

/// Android thermal status for no heating, dimensionless.
///
/// Source: Android `PowerManager` thermal status docs.
pub const THERMAL_STATUS_NONE_U32: u32 = 0;

/// Android thermal status for light heating, dimensionless.
///
/// Source: Android `PowerManager` thermal status docs.
pub const THERMAL_STATUS_LIGHT_U32: u32 = 1;

/// Android thermal status for moderate heating, dimensionless.
///
/// Source: Android `PowerManager` thermal status docs.
pub const THERMAL_STATUS_MODERATE_U32: u32 = 2;

/// Android thermal status for severe heating, dimensionless.
///
/// Source: Android `PowerManager` thermal status docs.
pub const THERMAL_STATUS_SEVERE_U32: u32 = 3;

/// Android thermal status for critical heating, dimensionless.
///
/// Source: Android `PowerManager` thermal status docs.
pub const THERMAL_STATUS_CRITICAL_U32: u32 = 4;

/// Android thermal status for emergency heating, dimensionless.
///
/// Source: Android `PowerManager` thermal status docs.
pub const THERMAL_STATUS_EMERGENCY_U32: u32 = 5;

/// Android thermal status for shutdown heating, dimensionless.
///
/// Source: Android `PowerManager` thermal status docs.
pub const THERMAL_STATUS_SHUTDOWN_U32: u32 = 6;

/// Android flight-shell range-check and probe failures.
///
/// Returned for non-finite clocks, negative clocks, non-positive warp,
/// unsupported SDK levels, unknown thermal codes, or zero parallelism.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AndroidError {
    /// Clock or interval was non-finite.
    NonFinite {
        /// Rejected value.
        value_f64: f64,
    },
    /// Clock delta or sample was negative.
    Negative {
        /// Rejected value.
        value_f64: f64,
    },
    /// Warp factor was not strictly positive.
    NonPositiveWarp {
        /// Rejected warp factor, dimensionless.
        value_f64: f64,
    },
    /// Device SDK level is below the floor.
    UnsupportedSdk {
        /// Rejected API level, dimensionless.
        sdk_api_u32: u32,
    },
    /// Thermal status code is outside 0 through 6.
    UnknownThermalStatus {
        /// Rejected status code, dimensionless.
        status_u32: u32,
    },
    /// No usable parallelism was reported.
    NoParallelism,
}

impl core::fmt::Display for AndroidError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NonFinite { value_f64 } => {
                write!(formatter, "non-finite android value: {value_f64}")
            }
            Self::Negative { value_f64 } => {
                write!(formatter, "negative android value: {value_f64}")
            }
            Self::NonPositiveWarp { value_f64 } => {
                write!(formatter, "non-positive android warp: {value_f64}")
            }
            Self::UnsupportedSdk { sdk_api_u32 } => {
                write!(
                    formatter,
                    "unsupported android SDK: {sdk_api_u32} below {ANDROID_MIN_SDK_API_U32}"
                )
            }
            Self::UnknownThermalStatus { status_u32 } => {
                write!(formatter, "unknown android thermal status: {status_u32}")
            }
            Self::NoParallelism => {
                write!(
                    formatter,
                    "android worker pool needs parallelism of at least one"
                )
            }
        }
    }
}

impl std::error::Error for AndroidError {}

/// Report whether an SDK level meets the floor.
///
/// True at or above 26. Source: `docs/tech/mobile.md`.
#[must_use]
pub const fn is_sdk_supported(sdk_api_u32: u32) -> bool {
    sdk_api_u32 >= ANDROID_MIN_SDK_API_U32
}

/// Check an SDK level against the floor.
///
/// # Errors
///
/// Returns [`AndroidError::UnsupportedSdk`] below API 26.
pub const fn check_sdk(sdk_api_u32: u32) -> Result<(), AndroidError> {
    if is_sdk_supported(sdk_api_u32) {
        Ok(())
    } else {
        Err(AndroidError::UnsupportedSdk { sdk_api_u32 })
    }
}

/// Report whether thermal status polling exists.
///
/// True at or above API 29; older levels use the probe plus fallback
/// path. Source: Android SDK docs.
#[must_use]
pub const fn thermal_api_available(sdk_api_u32: u32) -> bool {
    sdk_api_u32 >= ANDROID_THERMAL_STATUS_API_U32
}

/// Map an Android thermal status code.
///
/// None maps to nominal, light plus moderate to fair, severe to
/// serious, and critical plus emergency plus shutdown to critical.
///
/// # Errors
///
/// Returns [`AndroidError::UnknownThermalStatus`] outside 0 through 6.
pub const fn map_thermal_status(status_u32: u32) -> Result<ThermalState, AndroidError> {
    match status_u32 {
        THERMAL_STATUS_NONE_U32 => Ok(ThermalState::Nominal),
        THERMAL_STATUS_LIGHT_U32 | THERMAL_STATUS_MODERATE_U32 => Ok(ThermalState::Fair),
        THERMAL_STATUS_SEVERE_U32 => Ok(ThermalState::Serious),
        THERMAL_STATUS_CRITICAL_U32
        | THERMAL_STATUS_EMERGENCY_U32
        | THERMAL_STATUS_SHUTDOWN_U32 => Ok(ThermalState::Critical),
        _ => Err(AndroidError::UnknownThermalStatus { status_u32 }),
    }
}

/// Map a thermal state to its render-only tier.
///
/// Serious plus critical drop to Low immediately; nominal plus fair
/// hold Medium. High is headroom only, never thermal-selected. Sim
/// behavior is identical across tiers per `docs/tech/quality.md`.
#[must_use]
pub const fn tier_for_thermal_state(state: ThermalState) -> ThermalTier {
    match state {
        ThermalState::Nominal | ThermalState::Fair => ThermalTier::Medium,
        ThermalState::Serious | ThermalState::Critical => ThermalTier::Low,
    }
}

/// One frame of pacer output in seconds.
///
/// Counts whole sim steps only; the 30 fps sleep keeps the loop
/// bounded and the drop counter records excess time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FramePlan {
    /// Whole sim steps to advance, dimensionless.
    sim_steps_u32: u32,
    /// Sleep before the next frame in seconds.
    sleep_s_f64: f64,
    /// Time dropped this frame in seconds.
    dropped_s_f64: f64,
}

impl FramePlan {
    /// Return whole sim steps, dimensionless.
    #[must_use]
    pub const fn sim_steps_u32(self) -> u32 {
        self.sim_steps_u32
    }

    /// Return sleep before the next frame in seconds.
    #[must_use]
    pub const fn sleep_s_f64(self) -> f64 {
        self.sleep_s_f64
    }

    /// Return time dropped this frame in seconds.
    #[must_use]
    pub const fn dropped_s_f64(self) -> f64 {
        self.dropped_s_f64
    }
}

/// Frame pacer with a 30 fps limiter and drop counter.
///
/// Accumulates wall time scaled by warp, drains whole sim steps up
/// to the catch-up bound, and drops excess time with a counter
/// instead of spiraling. Mirrors the desktop `advance_sim` bound.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FramePacer {
    /// Unconsumed sim time in seconds.
    accumulator_s_f64: f64,
    /// Total dropped time in seconds.
    dropped_total_s_f64: f64,
}

impl FramePacer {
    /// Build an empty pacer with no backlog.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            accumulator_s_f64: 0.0,
            dropped_total_s_f64: 0.0,
        }
    }

    /// Return the frame budget in seconds.
    #[must_use]
    pub fn frame_budget_s_f64() -> f64 {
        ANDROID_FRAME_BUDGET_MS_F64 / MILLIS_PER_SECOND_F64
    }

    /// Return unconsumed sim time in seconds.
    #[must_use]
    pub const fn accumulator_s_f64(self) -> f64 {
        self.accumulator_s_f64
    }

    /// Return total dropped time in seconds.
    #[must_use]
    pub const fn dropped_total_s_f64(self) -> f64 {
        self.dropped_total_s_f64
    }

    /// Clear backlog outside the frame loop.
    ///
    /// No allocation; called on fresh runs only.
    pub const fn clear(&mut self) {
        self.accumulator_s_f64 = 0.0;
        self.dropped_total_s_f64 = 0.0;
    }

    /// Observe one frame and plan sim steps plus sleep.
    ///
    /// Scales wall time by warp, drains whole steps up to the bound,
    /// keeps a sub-step remainder, and drops excess whole steps with
    /// a counter. Sleep clamps at zero when the frame overruns.
    ///
    /// # Errors
    ///
    /// Returns [`AndroidError`] for a non-finite or negative frame
    /// time, or a non-finite or non-positive warp factor.
    pub fn observe_frame(
        &mut self,
        frame_dt_s_f64: f64,
        warp_factor_f64: f64,
    ) -> Result<FramePlan, AndroidError> {
        if !frame_dt_s_f64.is_finite() || !warp_factor_f64.is_finite() {
            let bad_f64 = if frame_dt_s_f64.is_finite() {
                warp_factor_f64
            } else {
                frame_dt_s_f64
            };
            return Err(AndroidError::NonFinite { value_f64: bad_f64 });
        }
        if frame_dt_s_f64 < 0.0 {
            return Err(AndroidError::Negative {
                value_f64: frame_dt_s_f64,
            });
        }
        if warp_factor_f64 <= 0.0 {
            return Err(AndroidError::NonPositiveWarp {
                value_f64: warp_factor_f64,
            });
        }
        let budget_s_f64 = Self::frame_budget_s_f64();
        let sleep_s_f64 = (budget_s_f64 - frame_dt_s_f64).max(0.0);
        self.accumulator_s_f64 += frame_dt_s_f64 * warp_factor_f64;
        let step_s_f64 = engine::sim::SIM_TICK_S.value();
        let mut steps_u32 = 0_u32;
        while self.accumulator_s_f64 >= step_s_f64
            && steps_u32 < ANDROID_MAX_SIM_STEPS_PER_FRAME_U32
        {
            self.accumulator_s_f64 -= step_s_f64;
            steps_u32 += 1;
        }
        let mut dropped_s_f64 = 0.0;
        if steps_u32 >= ANDROID_MAX_SIM_STEPS_PER_FRAME_U32 && self.accumulator_s_f64 >= step_s_f64
        {
            let remainder_s_f64 = self.accumulator_s_f64 % step_s_f64;
            dropped_s_f64 = self.accumulator_s_f64 - remainder_s_f64;
            self.accumulator_s_f64 = remainder_s_f64;
        }
        self.dropped_total_s_f64 += dropped_s_f64;
        Ok(FramePlan {
            sim_steps_u32: steps_u32,
            sleep_s_f64,
            dropped_s_f64,
        })
    }
}

impl Default for FramePacer {
    /// Default empty pacer with no backlog.
    fn default() -> Self {
        Self::new()
    }
}

/// Flight thread assignment with one sim plus one render thread.
///
/// The generation pool is available parallelism minus the reserve
/// with a minimum of one worker. Totals above four exceed the Low
/// tier assumption per `docs/tech/mobile.md`.
#[expect(
    clippy::struct_field_names,
    reason = "unit suffixes are required by the naming rule"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadPlan {
    /// Sim threads, dimensionless.
    sim_threads_usize: usize,
    /// Render threads, dimensionless.
    render_threads_usize: usize,
    /// Generation-pool workers, dimensionless.
    worker_threads_usize: usize,
}

impl ThreadPlan {
    /// Size the pool from available parallelism.
    ///
    /// Workers are available minus the reserve with a minimum of
    /// one; sim plus render stay fixed at one each.
    ///
    /// # Errors
    ///
    /// Returns [`AndroidError::NoParallelism`] when available is zero.
    pub const fn for_available_parallelism(available_usize: usize) -> Result<Self, AndroidError> {
        if available_usize == 0 {
            return Err(AndroidError::NoParallelism);
        }
        let spare_usize = available_usize.saturating_sub(ANDROID_WORKER_RESERVE_THREADS_USIZE);
        let workers_usize = if spare_usize < ANDROID_MIN_WORKER_THREADS_USIZE {
            ANDROID_MIN_WORKER_THREADS_USIZE
        } else {
            spare_usize
        };
        Ok(Self {
            sim_threads_usize: ANDROID_SIM_THREADS_USIZE,
            render_threads_usize: ANDROID_RENDER_THREADS_USIZE,
            worker_threads_usize: workers_usize,
        })
    }

    /// Return sim threads, dimensionless.
    #[must_use]
    pub const fn sim_threads_usize(self) -> usize {
        self.sim_threads_usize
    }

    /// Return render threads, dimensionless.
    #[must_use]
    pub const fn render_threads_usize(self) -> usize {
        self.render_threads_usize
    }

    /// Return generation-pool workers, dimensionless.
    #[must_use]
    pub const fn worker_threads_usize(self) -> usize {
        self.worker_threads_usize
    }

    /// Return total threads, dimensionless.
    #[must_use]
    pub const fn total_threads_usize(self) -> usize {
        self.sim_threads_usize + self.render_threads_usize + self.worker_threads_usize
    }

    /// Report whether the total fits the Low tier core assumption.
    #[must_use]
    pub const fn fits_low_tier_cores(self) -> bool {
        self.total_threads_usize() <= ANDROID_LOW_TIER_CORES_USIZE
    }
}

/// Thermal poll clock firing every 2.0 seconds.
///
/// The JNI layer reads `PowerManager` on this cadence (or follows
/// `OnThermalStatusChangedListener` where events beat polling) and
/// maps codes through [`map_thermal_status`]; API 26 through 28 use
/// the temperature fallback per [`thermal_api_available`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThermalPoll {
    /// Last poll time in seconds.
    last_poll_s_f64: f64,
}

impl ThermalPoll {
    /// Build a poll clock starting at zero.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            last_poll_s_f64: 0.0,
        }
    }

    /// Return the poll interval in seconds.
    #[must_use]
    pub const fn interval_s_f64() -> f64 {
        ANDROID_THERMAL_POLL_INTERVAL_S_F64
    }

    /// Return the last poll time in seconds.
    #[must_use]
    pub const fn last_poll_s_f64(self) -> f64 {
        self.last_poll_s_f64
    }

    /// Report whether a poll is due and latch the clock.
    ///
    /// True once per interval; monotonic wall time only.
    ///
    /// # Errors
    ///
    /// Returns [`AndroidError`] for a non-finite time or a time
    /// behind the last poll.
    pub fn should_poll(&mut self, now_s_f64: f64) -> Result<bool, AndroidError> {
        if !now_s_f64.is_finite() {
            return Err(AndroidError::NonFinite {
                value_f64: now_s_f64,
            });
        }
        let delta_s_f64 = now_s_f64 - self.last_poll_s_f64;
        if delta_s_f64 < 0.0 {
            return Err(AndroidError::Negative {
                value_f64: delta_s_f64,
            });
        }
        if delta_s_f64 >= ANDROID_THERMAL_POLL_INTERVAL_S_F64 {
            self.last_poll_s_f64 = now_s_f64;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

impl Default for ThermalPoll {
    /// Default poll clock starting at zero.
    fn default() -> Self {
        Self::new()
    }
}

/// Flight capture clock at 1 Hz over 15 minutes.
///
/// The JNI layer pushes one [`FlightSample`](crate::flight_log::FlightSample)
/// per tick of this clock into the pre-sized ring; export formats
/// append-only CSV. Fits the Step 1 ring with margin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlightCapture {
    /// Last capture time in seconds.
    last_capture_s_f64: f64,
}

impl FlightCapture {
    /// Build a capture clock starting at zero.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            last_capture_s_f64: 0.0,
        }
    }

    /// Return the capture interval in seconds.
    #[must_use]
    pub const fn interval_s_f64() -> f64 {
        ANDROID_FLIGHT_CAPTURE_INTERVAL_S_F64
    }

    /// Return the minimum sample count, dimensionless.
    #[must_use]
    pub const fn required_samples_usize() -> usize {
        ANDROID_FLIGHT_CAPTURE_MIN_SAMPLES_USIZE
    }

    /// Report whether the run fits the Step 1 ring.
    #[must_use]
    pub const fn fits_in_flight_log() -> bool {
        ANDROID_FLIGHT_CAPTURE_MIN_SAMPLES_USIZE <= FLIGHT_LOG_CAPACITY_ENTRIES_USIZE
    }

    /// Return the last capture time in seconds.
    #[must_use]
    pub const fn last_capture_s_f64(self) -> f64 {
        self.last_capture_s_f64
    }

    /// Report whether a sample is due and latch the clock.
    ///
    /// True once per second; monotonic wall time only.
    ///
    /// # Errors
    ///
    /// Returns [`AndroidError`] for a non-finite time or a time
    /// behind the last capture.
    pub fn should_capture(&mut self, now_s_f64: f64) -> Result<bool, AndroidError> {
        if !now_s_f64.is_finite() {
            return Err(AndroidError::NonFinite {
                value_f64: now_s_f64,
            });
        }
        let delta_s_f64 = now_s_f64 - self.last_capture_s_f64;
        if delta_s_f64 < 0.0 {
            return Err(AndroidError::Negative {
                value_f64: delta_s_f64,
            });
        }
        if delta_s_f64 >= ANDROID_FLIGHT_CAPTURE_INTERVAL_S_F64 {
            self.last_capture_s_f64 = now_s_f64;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Report whether a capture gap meets AC1.
    ///
    /// Gaps above 5.0 seconds fail.
    ///
    /// # Errors
    ///
    /// Returns [`AndroidError`] for a non-finite or negative gap.
    pub fn gap_ok(gap_s_f64: f64) -> Result<bool, AndroidError> {
        if !gap_s_f64.is_finite() {
            return Err(AndroidError::NonFinite {
                value_f64: gap_s_f64,
            });
        }
        if gap_s_f64 < 0.0 {
            return Err(AndroidError::Negative {
                value_f64: gap_s_f64,
            });
        }
        Ok(gap_s_f64 <= ANDROID_MAX_CAPTURE_GAP_S_F64)
    }
}

impl Default for FlightCapture {
    /// Default capture clock starting at zero.
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME_DT_S_F64: f64 = 0.016;
    const SIM_STEP_DT_S_F64: f64 = 0.05;
    const HALF_FRAME_DT_S_F64: f64 = 0.01;
    const LONG_DT_S_F64: f64 = 60.0;
    const WARP_X1_F64: f64 = 1.0;
    const WARP_X10_F64: f64 = 10.0;
    const BUDGET_TOL_S_F64: f64 = 1e-9;
    const EXPECTED_BUDGET_S_F64: f64 = 0.033_33;
    const EXPECTED_WORKERS_4_USIZE: usize = 2;
    const EXPECTED_WORKERS_8_USIZE: usize = 6;

    #[test]
    fn frame_budget_matches_thirty_fps() {
        assert!((ANDROID_FRAME_BUDGET_MS_F64 - 33.33).abs() < BUDGET_TOL_S_F64);
        let budget_s_f64 = FramePacer::frame_budget_s_f64();
        assert!((budget_s_f64 - EXPECTED_BUDGET_S_F64).abs() < BUDGET_TOL_S_F64);
        assert_eq!(ANDROID_MIN_SDK_API_U32, 26);
        assert_eq!(ANDROID_TARGET_SDK_API_U32, 34);
        assert_eq!(FramePacer::new(), FramePacer::default());
    }

    #[test]
    fn pacer_limits_frame_and_counts_drops() {
        let mut pacer = FramePacer::new();
        let Ok(plan) = pacer.observe_frame(HALF_FRAME_DT_S_F64, WARP_X1_F64) else {
            panic!("short frame must plan")
        };
        assert_eq!(plan.sim_steps_u32(), 0);
        assert!(plan.sleep_s_f64() > 0.0);
        assert!(plan.dropped_s_f64().abs() < BUDGET_TOL_S_F64);
        let Ok(plan) = pacer.observe_frame(SIM_STEP_DT_S_F64, WARP_X1_F64) else {
            panic!("step frame must plan")
        };
        assert!(plan.sim_steps_u32() >= 1);
        let Ok(plan) = pacer.observe_frame(LONG_DT_S_F64, WARP_X10_F64) else {
            panic!("long frame must plan")
        };
        assert_eq!(plan.sim_steps_u32(), ANDROID_MAX_SIM_STEPS_PER_FRAME_U32);
        assert!(plan.dropped_s_f64() > 0.0);
        assert!(pacer.dropped_total_s_f64() > 0.0);
        assert!(pacer.accumulator_s_f64() < SIM_STEP_DT_S_F64);
        pacer.clear();
        assert!(pacer.dropped_total_s_f64().abs() < BUDGET_TOL_S_F64);
        assert!(pacer.accumulator_s_f64().abs() < BUDGET_TOL_S_F64);
    }

    #[test]
    fn pacer_rejects_bad_clocks_and_warp() {
        let mut pacer = FramePacer::new();
        assert!(matches!(
            pacer.observe_frame(f64::NAN, WARP_X1_F64),
            Err(AndroidError::NonFinite { .. })
        ));
        assert!(matches!(
            pacer.observe_frame(-FRAME_DT_S_F64, WARP_X1_F64),
            Err(AndroidError::Negative { .. })
        ));
        assert!(matches!(
            pacer.observe_frame(FRAME_DT_S_F64, 0.0),
            Err(AndroidError::NonPositiveWarp { .. })
        ));
        assert!(matches!(
            pacer.observe_frame(FRAME_DT_S_F64, f64::INFINITY),
            Err(AndroidError::NonFinite { .. })
        ));
    }

    #[test]
    fn thread_pool_sizes_to_avail_minus_two_min_one() {
        let Ok(one) = ThreadPlan::for_available_parallelism(1) else {
            panic!("one core must plan")
        };
        assert_eq!(one.sim_threads_usize(), ANDROID_SIM_THREADS_USIZE);
        assert_eq!(one.render_threads_usize(), ANDROID_RENDER_THREADS_USIZE);
        assert_eq!(one.worker_threads_usize(), ANDROID_MIN_WORKER_THREADS_USIZE);
        let Ok(two) = ThreadPlan::for_available_parallelism(2) else {
            panic!("two cores must plan")
        };
        assert_eq!(two.worker_threads_usize(), ANDROID_MIN_WORKER_THREADS_USIZE);
        let Ok(four) = ThreadPlan::for_available_parallelism(4) else {
            panic!("four cores must plan")
        };
        assert_eq!(four.worker_threads_usize(), EXPECTED_WORKERS_4_USIZE);
        assert!(four.fits_low_tier_cores());
        let Ok(eight) = ThreadPlan::for_available_parallelism(8) else {
            panic!("eight cores must plan")
        };
        assert_eq!(eight.worker_threads_usize(), EXPECTED_WORKERS_8_USIZE);
        assert_eq!(
            eight.total_threads_usize(),
            ANDROID_SIM_THREADS_USIZE + ANDROID_RENDER_THREADS_USIZE + EXPECTED_WORKERS_8_USIZE
        );
        assert!(!eight.fits_low_tier_cores());
        assert!(matches!(
            ThreadPlan::for_available_parallelism(0),
            Err(AndroidError::NoParallelism)
        ));
    }

    #[test]
    fn thermal_status_maps_seven_codes() {
        assert_eq!(
            map_thermal_status(THERMAL_STATUS_NONE_U32),
            Ok(ThermalState::Nominal)
        );
        assert_eq!(
            map_thermal_status(THERMAL_STATUS_LIGHT_U32),
            Ok(ThermalState::Fair)
        );
        assert_eq!(
            map_thermal_status(THERMAL_STATUS_MODERATE_U32),
            Ok(ThermalState::Fair)
        );
        assert_eq!(
            map_thermal_status(THERMAL_STATUS_SEVERE_U32),
            Ok(ThermalState::Serious)
        );
        assert_eq!(
            map_thermal_status(THERMAL_STATUS_CRITICAL_U32),
            Ok(ThermalState::Critical)
        );
        assert_eq!(
            map_thermal_status(THERMAL_STATUS_EMERGENCY_U32),
            Ok(ThermalState::Critical)
        );
        assert_eq!(
            map_thermal_status(THERMAL_STATUS_SHUTDOWN_U32),
            Ok(ThermalState::Critical)
        );
        assert!(matches!(
            map_thermal_status(7),
            Err(AndroidError::UnknownThermalStatus { .. })
        ));
        assert_eq!(
            tier_for_thermal_state(ThermalState::Nominal),
            ThermalTier::Medium
        );
        assert_eq!(
            tier_for_thermal_state(ThermalState::Fair),
            ThermalTier::Medium
        );
        assert_eq!(
            tier_for_thermal_state(ThermalState::Serious),
            ThermalTier::Low
        );
        assert_eq!(
            tier_for_thermal_state(ThermalState::Critical),
            ThermalTier::Low
        );
    }

    #[test]
    fn thermal_poll_fires_every_two_seconds() {
        let mut poll = ThermalPoll::new();
        assert!((ThermalPoll::interval_s_f64() - 2.0).abs() < BUDGET_TOL_S_F64);
        assert!(poll.last_poll_s_f64().abs() < BUDGET_TOL_S_F64);
        assert_eq!(poll.should_poll(0.0), Ok(false));
        assert_eq!(poll.should_poll(1.99), Ok(false));
        assert_eq!(poll.should_poll(2.0), Ok(true));
        assert!((poll.last_poll_s_f64() - 2.0).abs() < BUDGET_TOL_S_F64);
        assert_eq!(poll.should_poll(3.99), Ok(false));
        assert_eq!(poll.should_poll(4.0), Ok(true));
        assert!(matches!(
            poll.should_poll(f64::NAN),
            Err(AndroidError::NonFinite { .. })
        ));
        assert!(matches!(
            poll.should_poll(3.0),
            Err(AndroidError::Negative { .. })
        ));
        assert_eq!(ThermalPoll::default(), ThermalPoll::new());
    }

    #[test]
    fn sdk_probe_enforces_26_and_thermal_29() {
        assert!(!is_sdk_supported(25));
        assert!(check_sdk(25).is_err());
        assert!(is_sdk_supported(ANDROID_MIN_SDK_API_U32));
        assert!(check_sdk(ANDROID_MIN_SDK_API_U32).is_ok());
        assert!(is_sdk_supported(ANDROID_TARGET_SDK_API_U32));
        assert!(!thermal_api_available(28));
        assert!(thermal_api_available(ANDROID_THERMAL_STATUS_API_U32));
        assert!(matches!(
            check_sdk(25),
            Err(AndroidError::UnsupportedSdk { .. })
        ));
    }

    #[test]
    fn capture_holds_fifteen_minutes_at_one_hertz() {
        assert_eq!(
            FlightCapture::required_samples_usize(),
            ANDROID_FLIGHT_CAPTURE_MIN_SAMPLES_USIZE
        );
        assert!(FlightCapture::fits_in_flight_log());
        // 15 minutes at 1 Hz needs 900 samples; literals are test-only.
        assert!(
            (ANDROID_FLIGHT_CAPTURE_DURATION_S_F64 / ANDROID_FLIGHT_CAPTURE_INTERVAL_S_F64 - 900.0)
                .abs()
                < BUDGET_TOL_S_F64
        );
        let mut capture = FlightCapture::new();
        assert_eq!(capture.should_capture(0.0), Ok(false));
        assert_eq!(capture.should_capture(0.99), Ok(false));
        assert_eq!(capture.should_capture(1.0), Ok(true));
        assert_eq!(capture.should_capture(1.5), Ok(false));
        assert_eq!(FlightCapture::gap_ok(5.0), Ok(true));
        assert_eq!(FlightCapture::gap_ok(5.01), Ok(false));
        assert!(matches!(
            FlightCapture::gap_ok(f64::NAN),
            Err(AndroidError::NonFinite { .. })
        ));
        assert!(matches!(
            FlightCapture::gap_ok(-1.0),
            Err(AndroidError::Negative { .. })
        ));
        assert!(matches!(
            capture.should_capture(0.5),
            Err(AndroidError::Negative { .. })
        ));
        assert_eq!(FlightCapture::default(), FlightCapture::new());
    }
}
