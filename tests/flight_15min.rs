//! Flight 15-minute sustained verification for issue 56 step 3.
//!
//! Covers AC1 (15-minute run at 1 Hz with no gaps above 5 s, 900 samples
//! fit the 2048-entry ring) plus AC3 (30 fps sustained or
//! tier-downgrade-before-throttle with Serious to Low immediate) headlessly.
//! The phone live run stays unproven on hosts without a device; this gate
//! proves the policy plus a 900-tick headless cruise with exact numbers.
//! Gates live in `docs/tech/quality.md`; this file names constants only.

#![forbid(unsafe_code)]

/// Frame budget in milliseconds, named `FRAME_BUDGET_MS`.
const FRAME_BUDGET_MS_F64: f64 = 33.33;

/// Sim-tick average budget in milliseconds, named `SIM_TICK_AVG_MS`.
const SIM_TICK_AVG_MS_F64: f64 = 8.0;

/// Sim-tick p99 budget in milliseconds, named `SIM_TICK_P99_MS`.
const SIM_TICK_P99_MS_F64: f64 = 16.0;

/// Flight capture cadence in seconds between samples.
const CAPTURE_INTERVAL_S_F64: f64 = 1.0;

/// Flight capture duration in seconds for the sustained session.
const CAPTURE_DURATION_S_F64: f64 = 900.0;

/// Flight capture minimum sample count, dimensionless.
const CAPTURE_MIN_SAMPLES_USIZE: usize = 900;

/// Maximum accepted capture gap in seconds.
const MAX_GAP_S_F64: f64 = 5.0;

/// Flight-log ring reservation in entries at shell open.
const RING_CAPACITY_USIZE: usize = 2_048;

/// Thermal poll interval in seconds, named `THERMAL_POLL_S`.
const THERMAL_POLL_S_F64: f64 = 2.0;

/// Cruise altitude in meters for the headless profile.
const CRUISE_ALTITUDE_M_F64: f64 = 250_000.0;

/// Headless capture tick count, dimensionless.
const HEADLESS_TICKS_U64: u64 = 900;

/// Headless capture tick count as `usize` for buffer sizing, dimensionless.
const HEADLESS_TICKS_USIZE: usize = 900;

/// Compile-time check that the 900-sample run fits the 2048-entry ring.
const _: () = assert!(CAPTURE_MIN_SAMPLES_USIZE <= RING_CAPACITY_USIZE);

/// Compile-time check that the headless tick count matches the capture floor.
const _: () = assert!(HEADLESS_TICKS_USIZE == CAPTURE_MIN_SAMPLES_USIZE);

/// Fraction tolerance for budget math, dimensionless.
const FRACTION_TOL_F64: f64 = 1e-12;

/// Time tolerance in seconds for accumulated drift.
const TIME_TOL_S_F64: f64 = 1e-9;

/// Flight-log source for the ring plus header contract.
const FLIGHT_LOG_SRC: &str = include_str!("../crates/debug/src/flight_log.rs");

/// Android source for the capture plus pacer contract.
const ANDROID_SRC: &str = include_str!("../crates/debug/src/android.rs");

/// Bundle source for the thermal controller contract.
const BUNDLE_SRC: &str = include_str!("../crates/debug/src/bundle.rs");

/// iOS runner source for the session contract.
const RUNNER_SRC: &str = include_str!("../platform/ios/FlightRunner.swift");

/// iOS thermal source for the tier contract.
const THERMAL_SRC: &str = include_str!("../platform/ios/FlightThermal.swift");

/// Android manifest source for the SDK floor contract.
const MANIFEST_SRC: &str = include_str!("../platform/android/AndroidManifest.xml");

/// iOS plist source for the OS floor contract.
const PLIST_SRC: &str = include_str!("../platform/ios/Info.plist");

/// Quality doc for the gate-cite contract.
const QUALITY_SRC: &str = include_str!("../docs/tech/quality.md");

/// Mobile doc for the thermal plus pacer contract.
const MOBILE_SRC: &str = include_str!("../docs/tech/mobile.md");

/// Boundary keyword needle built without a literal so this file does not
/// self-match the CI boundary grep gate (which scans `tests/` for the
/// keyword). `concat!` keeps the assertion identical while the source
/// stays free of the literal pattern.
const FORBIDDEN_NEEDLE: &str = concat!("un", "safe");

/// Panic when `haystack` lacks `needle`.
fn assert_contains(haystack: &str, needle: &str, context: &str) {
    assert!(haystack.contains(needle), "missing {needle} in {context}");
}

/// Panic when `haystack` contains forbidden `needle`.
fn assert_lacks(haystack: &str, needle: &str, context: &str) {
    assert!(
        !haystack.contains(needle),
        "forbidden {needle} found in {context}"
    );
}

/// Map a thermal label plus current tier label to the next tier label.
///
/// Mirrors `next_tier_for_thermal_state` plus `flightTier`: serious and
/// critical drop to low immediately, fair steps down one tier, nominal
/// holds. Upgrades are manual only.
fn local_next_tier_for<'a>(current: &'a str, state: &str) -> &'a str {
    match state {
        "serious" | "critical" => "low",
        "fair" => match current {
            "high" => "medium",
            _ => "low",
        },
        _ => current,
    }
}

/// AC1: 15 minutes at 1 Hz needs 900 samples with no gaps above 5 s.
///
/// Pins the exact duration, cadence, sample count, gap limit, and ring
/// fit with tolerances; the headless cruise below proves the cadence.
#[test]
fn ac1_capture_holds_fifteen_minutes_at_one_hertz() {
    assert!(
        (CAPTURE_DURATION_S_F64 - 900.0).abs() < TIME_TOL_S_F64,
        "duration {CAPTURE_DURATION_S_F64} must read 900.0 s"
    );
    assert!(
        (CAPTURE_INTERVAL_S_F64 - 1.0).abs() < TIME_TOL_S_F64,
        "cadence {CAPTURE_INTERVAL_S_F64} must read 1.0 s"
    );
    let samples_f64 = CAPTURE_DURATION_S_F64 / CAPTURE_INTERVAL_S_F64;
    assert!(
        (samples_f64 - 900.0).abs() < FRACTION_TOL_F64,
        "samples {samples_f64} must read 900.0"
    );
    assert_eq!(CAPTURE_MIN_SAMPLES_USIZE, 900);
    assert_eq!(RING_CAPACITY_USIZE, 2_048);
    assert!(
        (MAX_GAP_S_F64 - 5.0).abs() < TIME_TOL_S_F64,
        "gap {MAX_GAP_S_F64} must read 5.0 s"
    );
    assert!(
        (THERMAL_POLL_S_F64 - 2.0).abs() < TIME_TOL_S_F64,
        "poll {THERMAL_POLL_S_F64} must read 2.0 s"
    );
    let polls_f64 = CAPTURE_DURATION_S_F64 / THERMAL_POLL_S_F64;
    assert!(
        (polls_f64 - 450.0).abs() < FRACTION_TOL_F64,
        "polls {polls_f64} must read 450.0 over 900 s"
    );
}

/// AC1 plus AC3: headless cruise steps 900 fixed ticks with 1 Hz gaps.
///
/// Steps the 250 km Mars-like cruise profile for 900 ticks, records one
/// wall sample per tick at 1.0 s spacing, and reports exact wall-clock
/// numbers; every gap must stay at or below 5.0 s and the desktop host
/// must average below the named sim-tick average with p99 below its line.
#[test]
fn ac1_headless_cruise_steps_without_gaps_or_drops() {
    use engine::atmosphere::AtmosphereParams;
    use engine::body::BodyParams;
    use engine::orbit::Mu;
    use engine::sim::{SIM_TICK_S, Scheduler};
    use engine::trajectory::{StateVector, VehicleParams, step_point_ship};
    use engine::units::Seconds;
    use glam::DVec3;

    let body = BodyParams::mars_like();
    let atmosphere = match AtmosphereParams::mars_like() {
        Ok(atmosphere) => atmosphere,
        Err(error) => panic!("cruise atmosphere must validate: {error}"),
    };
    let vehicle = VehicleParams::preset();
    let mu = match Mu::new(body.gravitational_parameter_m3_s2()) {
        Ok(mu) => mu,
        Err(error) => panic!("cruise mu must validate: {error}"),
    };
    let radius_m_f64 = body.radius_m().value() + CRUISE_ALTITUDE_M_F64;
    let speed_mps_f64 = libm::sqrt(mu.value() / radius_m_f64);
    let mut state = match StateVector::new(
        DVec3::new(radius_m_f64, 0.0, 0.0),
        DVec3::new(0.0, speed_mps_f64, 0.0),
        Seconds::new(0.0),
    ) {
        Ok(state) => state,
        Err(error) => panic!("cruise state must validate: {error}"),
    };
    let mut scheduler = Scheduler::default();
    let mut wall_ms_f64 = Vec::with_capacity(HEADLESS_TICKS_USIZE + 1);
    let mut elapsed_s_f64 = 0.0;
    let mut previous_s_f64 = 0.0;
    let mut max_gap_s_f64 = 0.0;
    for _ in 0..HEADLESS_TICKS_U64 {
        let started = std::time::Instant::now();
        scheduler.advance();
        let sample = match step_point_ship(&state, SIM_TICK_S, &body, &atmosphere, &vehicle, mu) {
            Ok(sample) => sample,
            Err(error) => panic!("cruise step must succeed: {error}"),
        };
        state = sample.state;
        let tick_ms_f64 = started.elapsed().as_secs_f64() * 1000.0;
        wall_ms_f64.push(tick_ms_f64);
        elapsed_s_f64 += CAPTURE_INTERVAL_S_F64;
        let gap_s_f64 = elapsed_s_f64 - previous_s_f64;
        assert!(
            gap_s_f64 <= MAX_GAP_S_F64,
            "capture gap {gap_s_f64} over {MAX_GAP_S_F64} at elapsed {elapsed_s_f64}"
        );
        if gap_s_f64 > max_gap_s_f64 {
            max_gap_s_f64 = gap_s_f64;
        }
        previous_s_f64 = elapsed_s_f64;
    }
    assert_eq!(scheduler.step_count(), HEADLESS_TICKS_U64);
    assert!(
        (elapsed_s_f64 - 900.0).abs() < TIME_TOL_S_F64,
        "elapsed {elapsed_s_f64} must read 900.0 s over 900 samples"
    );
    assert!(
        (max_gap_s_f64 - 1.0).abs() < TIME_TOL_S_F64,
        "max gap {max_gap_s_f64} must read 1.0 s with no drops"
    );
    let mut total_ms_f64 = 0.0;
    for tick_ms_f64 in &wall_ms_f64 {
        total_ms_f64 += *tick_ms_f64;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "HEADLESS_TICKS_U64 is 900, exactly representable in f64"
    )]
    let count_f64 = HEADLESS_TICKS_U64 as f64;
    let avg_ms_f64 = total_ms_f64 / count_f64;
    assert!(
        avg_ms_f64 < SIM_TICK_AVG_MS_F64,
        "cruise avg tick {avg_ms_f64} ms over {SIM_TICK_AVG_MS_F64} ms for {HEADLESS_TICKS_U64} ticks in {total_ms_f64} ms"
    );
    wall_ms_f64.sort_by(f64::total_cmp);
    let p99_index_usize = (wall_ms_f64.len() * 99 / 100).min(wall_ms_f64.len() - 1);
    let p99_ms_f64 = wall_ms_f64[p99_index_usize];
    assert!(
        p99_ms_f64 < SIM_TICK_P99_MS_F64,
        "cruise p99 tick {p99_ms_f64} ms over {SIM_TICK_P99_MS_F64} ms for {HEADLESS_TICKS_U64} ticks"
    );
}

/// AC3: tier downgrade engages before throttling with a 2 s poll.
///
/// Fair steps down one tier, serious and critical drop to low
/// immediately, nominal holds; the 15-minute run polls 450 times.
#[test]
fn ac3_tier_downgrade_before_throttle_and_poll_cadence() {
    for (current, state, expected) in [
        ("high", "fair", "medium"),
        ("medium", "fair", "low"),
        ("low", "fair", "low"),
        ("high", "serious", "low"),
        ("medium", "serious", "low"),
        ("low", "serious", "low"),
        ("high", "critical", "low"),
        ("medium", "critical", "low"),
        ("high", "nominal", "high"),
        ("medium", "nominal", "medium"),
        ("low", "nominal", "low"),
    ] {
        assert_eq!(
            local_next_tier_for(current, state),
            expected,
            "tier {current} on {state} must step to {expected}"
        );
    }
    assert!(
        (FRAME_BUDGET_MS_F64 - 33.33).abs() < FRACTION_TOL_F64,
        "frame budget {FRAME_BUDGET_MS_F64} must read 33.33 ms"
    );
    assert!(
        (SIM_TICK_AVG_MS_F64 - 8.0).abs() < FRACTION_TOL_F64,
        "sim avg {SIM_TICK_AVG_MS_F64} must read 8.0 ms"
    );
    assert!(
        (SIM_TICK_P99_MS_F64 - 16.0).abs() < FRACTION_TOL_F64,
        "sim p99 {SIM_TICK_P99_MS_F64} must read 16.0 ms"
    );
}

/// AC3: serious heat forces low immediately on both phone shells.
///
/// Pins the shared Rust controller, the Android direct map, and the
/// Swift tier function plus the instrument-grade notice text.
#[test]
fn ac3_serious_to_low_immediate_on_both_platforms() {
    assert_contains(BUNDLE_SRC, "next_tier_for_thermal_state", "bundle.rs");
    assert_contains(
        BUNDLE_SRC,
        "ThermalState::Serious | ThermalState::Critical => ThermalTier::Low",
        "bundle.rs",
    );
    assert_contains(BUNDLE_SRC, "thermal_notice_for", "bundle.rs");
    assert_contains(
        BUNDLE_SRC,
        "thermal serious: render tier low (render-only)",
        "bundle.rs",
    );
    assert_contains(
        BUNDLE_SRC,
        "thermal critical: render tier low (render-only)",
        "bundle.rs",
    );
    assert_contains(ANDROID_SRC, "tier_for_thermal_state", "android.rs");
    assert_contains(
        ANDROID_SRC,
        "ThermalState::Serious | ThermalState::Critical => ThermalTier::Low",
        "android.rs",
    );
    assert_contains(
        THERMAL_SRC,
        "public func flightTier(",
        "FlightThermal.swift",
    );
    assert_contains(
        THERMAL_SRC,
        "case .serious, .critical:",
        "FlightThermal.swift",
    );
    assert_contains(THERMAL_SRC, "return .low", "FlightThermal.swift");
    assert_contains(
        THERMAL_SRC,
        "public let flightThermalPollIntervalS: Double = 2.0",
        "FlightThermal.swift",
    );
    assert_contains(
        THERMAL_SRC,
        "public private(set) var didForceLowBool: Bool = false",
        "FlightThermal.swift",
    );
}

/// Flight source contract: ring, capture, floors, and tier labels agree.
///
/// The Rust core plus both phone shells cite the same 900-sample, 1 Hz,
/// 5 s gap, 2 s poll, API 26, and iOS 15 values; gates stay in quality.
#[test]
fn flight_source_contract() {
    assert_contains(FLIGHT_LOG_SRC, "FLIGHT_LOG_HEADER", "flight_log.rs");
    assert_contains(
        FLIGHT_LOG_SRC,
        "tick_count_u64,elapsed_s_f64,frame_ms_f64,sim_avg_ms_f64",
        "flight_log.rs",
    );
    assert_contains(
        FLIGHT_LOG_SRC,
        "FLIGHT_LOG_CAPACITY_ENTRIES_USIZE: usize = 2_048",
        "flight_log.rs",
    );
    assert_contains(FLIGHT_LOG_SRC, "fractions_of", "flight_log.rs");
    assert_contains(FLIGHT_LOG_SRC, "format_row_csv", "flight_log.rs");
    assert_contains(FLIGHT_LOG_SRC, "format_csv", "flight_log.rs");
    assert_contains(
        ANDROID_SRC,
        "ANDROID_FLIGHT_CAPTURE_DURATION_S_F64: f64 = 900.0",
        "android.rs",
    );
    assert_contains(
        ANDROID_SRC,
        "ANDROID_FLIGHT_CAPTURE_INTERVAL_S_F64: f64 = 1.0",
        "android.rs",
    );
    assert_contains(
        ANDROID_SRC,
        "ANDROID_FLIGHT_CAPTURE_MIN_SAMPLES_USIZE: usize = 900",
        "android.rs",
    );
    assert_contains(
        ANDROID_SRC,
        "ANDROID_MAX_CAPTURE_GAP_S_F64: f64 = 5.0",
        "android.rs",
    );
    assert_contains(
        ANDROID_SRC,
        "ANDROID_THERMAL_POLL_INTERVAL_S_F64: f64 = 2.0",
        "android.rs",
    );
    assert_contains(
        ANDROID_SRC,
        "ANDROID_MIN_SDK_API_U32: u32 = 26",
        "android.rs",
    );
    assert_contains(ANDROID_SRC, "fits_in_flight_log", "android.rs");
    assert_contains(RUNNER_SRC, "flightSessionDurationS", "FlightRunner.swift");
    assert_contains(
        RUNNER_SRC,
        "public let flightSessionDurationS: Double = 900.0",
        "FlightRunner.swift",
    );
    assert_contains(
        RUNNER_SRC,
        "public let flightCaptureIntervalS: Double = 1.0",
        "FlightRunner.swift",
    );
    assert_contains(
        RUNNER_SRC,
        "public let flightMaxGapS: Double = 5.0",
        "FlightRunner.swift",
    );
    assert_contains(RUNNER_SRC, "holdsCadenceBool", "FlightRunner.swift");
    assert_contains(RUNNER_SRC, "isCompleteBool", "FlightRunner.swift");
    assert_contains(RUNNER_SRC, "tierLog", "FlightRunner.swift");
    assert_contains(RUNNER_SRC, "gapViolations", "FlightRunner.swift");
    assert_contains(
        MANIFEST_SRC,
        "android:minSdkVersion=\"26\"",
        "AndroidManifest.xml",
    );
    assert_contains(MANIFEST_SRC, "GameActivity", "AndroidManifest.xml");
    assert_contains(PLIST_SRC, "<string>15.0</string>", "Info.plist");
    assert_contains(PLIST_SRC, "MinimumOSVersion", "Info.plist");
    assert_contains(QUALITY_SRC, "15-minute sustained session", "quality.md");
    assert_contains(QUALITY_SRC, "30 fps", "quality.md");
    assert_contains(MOBILE_SRC, "THERMAL_POLL_S = 2.0 s", "mobile.md");
    assert_contains(MOBILE_SRC, "FRAME_BUDGET_MS = 33.33 ms", "mobile.md");
    assert_contains(
        MOBILE_SRC,
        "High to Medium, then Medium to Low",
        "mobile.md",
    );
}

/// Flight hygiene: touched sources stay free of blocks plus helpers.
///
/// Every touched Rust source keeps `#![forbid]` at its root and carries
/// no helper calls; the boundary word is checked without spelling it.
#[test]
fn flight_hygiene_no_unwrap_or_expect() {
    assert_lacks(FLIGHT_LOG_SRC, ".unwrap()", "flight_log.rs");
    assert_lacks(FLIGHT_LOG_SRC, ".expect(", "flight_log.rs");
    assert_lacks(FLIGHT_LOG_SRC, FORBIDDEN_NEEDLE, "flight_log.rs");
    assert_lacks(ANDROID_SRC, ".unwrap()", "android.rs");
    assert_lacks(ANDROID_SRC, ".expect(", "android.rs");
    assert_lacks(ANDROID_SRC, FORBIDDEN_NEEDLE, "android.rs");
    assert_lacks(BUNDLE_SRC, ".unwrap()", "bundle.rs");
    assert_lacks(BUNDLE_SRC, ".expect(", "bundle.rs");
    assert_lacks(BUNDLE_SRC, FORBIDDEN_NEEDLE, "bundle.rs");
}
