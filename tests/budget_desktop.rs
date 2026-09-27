//! Desktop budget gate for issue 44 step 4 (desktop-first).
//!
//! Headless timing plus budget-fraction math on the desktop host. Gates live
//! in `docs/tech/quality.md`; this file names constants only and never copies
//! the gate table. The reference-phone sustained run stays postponed.

#![forbid(unsafe_code)]

/// Frame budget in milliseconds, named `FRAME_BUDGET_MS` in quality docs.
const FRAME_BUDGET_MS_F64: f64 = 33.33;

/// Sim-tick average budget in milliseconds, named `SIM_TICK_AVG_MS`.
const SIM_TICK_AVG_MS_F64: f64 = 8.0;

/// Sim-tick p99 budget in milliseconds, named `SIM_TICK_P99_MS`.
const SIM_TICK_P99_MS_F64: f64 = 16.0;

/// Surface-hitch p95 budget in milliseconds, named `SURFACE_HITCH_P95_MS`.
const SURFACE_HITCH_P95_MS_F64: f64 = 100.0;

/// Memory ceiling in megabytes, named `MEMORY_CEILING_MB`.
const MEMORY_CEILING_MB_F64: f64 = 1024.0;

/// Cold-start ceiling in seconds, named `COLD_START_S`.
const COLD_START_S_F64: f64 = 5.0;

/// Nominal band upper bound as a budget fraction, dimensionless.
const NOMINAL_MAX_FRACTION_F64: f64 = 0.5;

/// Elevated band upper bound as a budget fraction, dimensionless.
const ELEVATED_MAX_FRACTION_F64: f64 = 0.8;

/// Headless timing tick count, dimensionless.
const TIMING_TICKS_U64: u64 = 100;

/// Headless timing tick count as float for averaging, dimensionless.
const TIMING_TICKS_F64: f64 = 100.0;

/// Cruise altitude in meters for the timing profile.
const CRUISE_ALTITUDE_M_F64: f64 = 250_000.0;

/// Smoke frame sample in milliseconds for the fraction math.
const SMOKE_FRAME_MS_F64: f64 = 8.0;

/// Smoke sim-tick average in milliseconds for the fraction math.
const SMOKE_SIM_AVG_MS_F64: f64 = 2.0;

/// Smoke sim-tick p99 in milliseconds for the fraction math.
const SMOKE_SIM_P99_MS_F64: f64 = 4.0;

/// Smoke surface-hitch p95 in milliseconds for the fraction math.
const SMOKE_HITCH_P95_MS_F64: f64 = 10.0;

/// Smoke resident memory in megabytes for the fraction math.
const SMOKE_RESIDENT_MB_F64: f64 = 256.0;

/// Smoke cold-start sample in seconds for the fraction math.
const SMOKE_COLD_START_S_F64: f64 = 1.5;

/// Quality doc for the gate-cite contract.
const QUALITY_SRC: &str = include_str!("../docs/tech/quality.md");

/// Debug doc for the tester-shape contract.
const DEBUG_SRC: &str = include_str!("../docs/tech/debug.md");

/// Panic when `haystack` lacks `needle`.
fn assert_contains(haystack: &str, needle: &str, context: &str) {
    assert!(haystack.contains(needle), "missing {needle} in {context}");
}

/// Desktop frame math sits in the nominal band below the over line.
#[test]
fn desktop_frame_fraction_is_nominal() {
    let fraction_f64 = SMOKE_FRAME_MS_F64 / FRAME_BUDGET_MS_F64;
    assert!(
        fraction_f64 < NOMINAL_MAX_FRACTION_F64,
        "frame fraction {fraction_f64} leaves the nominal band"
    );
    let over_f64 = 30.0 / FRAME_BUDGET_MS_F64;
    assert!(
        over_f64 >= ELEVATED_MAX_FRACTION_F64,
        "over fraction {over_f64} must read at or above the elevated line"
    );
}

/// Desktop sim-tick math sits in the nominal band on both lines.
#[test]
fn desktop_sim_tick_fractions_are_nominal() {
    let avg_f64 = SMOKE_SIM_AVG_MS_F64 / SIM_TICK_AVG_MS_F64;
    assert!(
        avg_f64 < NOMINAL_MAX_FRACTION_F64,
        "sim avg fraction {avg_f64} leaves the nominal band"
    );
    let p99_f64 = SMOKE_SIM_P99_MS_F64 / SIM_TICK_P99_MS_F64;
    assert!(
        p99_f64 < NOMINAL_MAX_FRACTION_F64,
        "sim p99 fraction {p99_f64} leaves the nominal band"
    );
}

/// Desktop hitch, memory, and cold-start math sits in the nominal band.
#[test]
fn desktop_hitch_memory_cold_fractions_are_nominal() {
    let hitch_f64 = SMOKE_HITCH_P95_MS_F64 / SURFACE_HITCH_P95_MS_F64;
    assert!(
        hitch_f64 < NOMINAL_MAX_FRACTION_F64,
        "hitch fraction {hitch_f64} leaves the nominal band"
    );
    let memory_f64 = SMOKE_RESIDENT_MB_F64 / MEMORY_CEILING_MB_F64;
    assert!(
        memory_f64 < NOMINAL_MAX_FRACTION_F64,
        "memory fraction {memory_f64} leaves the nominal band"
    );
    let cold_f64 = SMOKE_COLD_START_S_F64 / COLD_START_S_F64;
    assert!(
        cold_f64 < NOMINAL_MAX_FRACTION_F64,
        "cold-start fraction {cold_f64} leaves the nominal band"
    );
}

/// Desktop headless timing holds the sim-tick average line.
///
/// Steps the 250 km Mars-like cruise profile for 100 fixed ticks and reports
/// exact wall-clock numbers; the desktop host must average below the named
/// sim-tick average with headroom to spare.
#[test]
fn desktop_headless_tick_timing_holds_average() {
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
        Err(error) => panic!("timing atmosphere must validate: {error}"),
    };
    let vehicle = VehicleParams::preset();
    let mu = match Mu::new(body.gravitational_parameter_m3_s2()) {
        Ok(mu) => mu,
        Err(error) => panic!("timing mu must validate: {error}"),
    };
    let radius_m_f64 = body.radius_m().value() + CRUISE_ALTITUDE_M_F64;
    let speed_mps_f64 = libm::sqrt(mu.value() / radius_m_f64);
    let mut state = match StateVector::new(
        DVec3::new(radius_m_f64, 0.0, 0.0),
        DVec3::new(0.0, speed_mps_f64, 0.0),
        Seconds::new(0.0),
    ) {
        Ok(state) => state,
        Err(error) => panic!("timing state must validate: {error}"),
    };
    let mut scheduler = Scheduler::default();
    let started = std::time::Instant::now();
    for _ in 0..TIMING_TICKS_U64 {
        scheduler.advance();
        let sample = match step_point_ship(&state, SIM_TICK_S, &body, &atmosphere, &vehicle, mu) {
            Ok(sample) => sample,
            Err(error) => panic!("timing step must succeed: {error}"),
        };
        state = sample.state;
    }
    let wall_ms_f64 = started.elapsed().as_secs_f64() * 1000.0;
    let avg_ms_f64 = wall_ms_f64 / TIMING_TICKS_F64;
    assert!(
        avg_ms_f64 < SIM_TICK_AVG_MS_F64,
        "desktop avg tick {avg_ms_f64} ms over {SIM_TICK_AVG_MS_F64} ms for {TIMING_TICKS_U64} ticks in {wall_ms_f64} ms"
    );
    assert!(
        wall_ms_f64 < SIM_TICK_AVG_MS_F64 * TIMING_TICKS_F64,
        "desktop wall total {wall_ms_f64} ms over budget for {TIMING_TICKS_U64} ticks"
    );
}

/// Desktop gate cites budgets by name; tester shape names the proxy.
///
/// Quality stays the single source for gates while the debug doc owns the
/// tester shape, the headless-proven window, and the postponed phone run.
#[test]
fn desktop_gate_source_contract() {
    assert_contains(QUALITY_SRC, "30 fps", "quality.md");
    assert_contains(QUALITY_SRC, "SIM_TICK_AVG_MS", "quality.md");
    assert_contains(QUALITY_SRC, "SIM_TICK_P99_MS", "quality.md");
    assert_contains(QUALITY_SRC, "SURFACE_HITCH_P95_MS", "quality.md");
    assert_contains(QUALITY_SRC, "MEMORY_CEILING_MB", "quality.md");
    assert_contains(QUALITY_SRC, "COLD_START_S", "quality.md");
    assert_contains(QUALITY_SRC, "cargo fmt --check", "quality.md");
    assert_contains(QUALITY_SRC, "cargo clippy", "quality.md");
    assert_contains(QUALITY_SRC, "cargo test", "quality.md");
    assert_contains(DEBUG_SRC, "DesktopTesterReadouts", "debug.md");
    assert_contains(DEBUG_SRC, "DesktopWindow", "debug.md");
    assert_contains(DEBUG_SRC, "desktop_tester", "debug.md");
    assert_contains(DEBUG_SRC, "tests/budget_desktop.rs", "debug.md");
    assert_contains(DEBUG_SRC, "postponed", "debug.md");
    assert_contains(DEBUG_SRC, "FRAME_BUDGET_MS", "debug.md");
    assert_contains(DEBUG_SRC, "quality.md", "debug.md");
}
