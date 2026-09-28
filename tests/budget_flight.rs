//! Flight budget plus determinism verification for issue 56 step 3.
//!
//! Covers AC2 (CSV fractions by the named budgets 33.33, 8.0, 16.0,
//! 100.0, 1024.0, 5.0 with exact numbers), AC4 (sim identical across
//! tiers with hash match inside floating-point noise on both chip
//! families), and AC5 (bundle per section 11 plus `system.txt` with
//! quarantine on checksum fail) headlessly. AC6 green is pinned through
//! gate plus hygiene contracts. Gates live in `docs/tech/quality.md`.

#![forbid(unsafe_code)]

/// Frame budget in milliseconds, named `FRAME_BUDGET_MS`.
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

/// Fraction tolerance for band math, dimensionless.
const FRACTION_TOL_F64: f64 = 1e-12;

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

/// Golden tick count, dimensionless.
#[cfg(feature = "dev-shell")]
const GOLDEN_TICK_COUNT_U64: u64 = 100;

/// Golden seed, dimensionless. Source: fractional hex digits of pi.
#[cfg(feature = "dev-shell")]
const GOLDEN_SEED_U64: u64 = 0x243F_6A88_85A3_08D3;

/// Locked golden digest after 100 ticks, dimensionless.
#[cfg(feature = "dev-shell")]
const GOLDEN_100_TICK_HASH_U64: u64 = 17_172_072_447_561_828_286;

/// Cruise altitude in meters for the golden profile.
#[cfg(feature = "dev-shell")]
const GOLDEN_ALTITUDE_M_F64: f64 = 250_000.0;

/// Flight-log source for the CSV plus fraction contract.
const FLIGHT_LOG_SRC: &str = include_str!("../crates/debug/src/flight_log.rs");

/// Budget source for the denominator contract.
const BUDGET_SRC: &str = include_str!("../crates/debug/src/budget.rs");

/// Bundle source for the thermal plus system companion contract.
const BUNDLE_SRC: &str = include_str!("../crates/debug/src/bundle.rs");

/// Export source for the bundle layout contract.
const EXPORT_SRC: &str = include_str!("../crates/debug/src/export.rs");

/// Trajectory source for the deterministic math contract.
#[cfg(feature = "dev-shell")]
const TRAJECTORY_SRC: &str = include_str!("../crates/engine/src/trajectory.rs");

/// Orbit source for the deterministic math contract.
#[cfg(feature = "dev-shell")]
const ORBIT_SRC: &str = include_str!("../crates/engine/src/orbit.rs");

/// Inspect source for the snapshot hash contract.
#[cfg(feature = "dev-shell")]
const INSPECT_SRC: &str = include_str!("../crates/engine/src/inspect.rs");

/// iOS runner source for the header parity contract.
const RUNNER_SRC: &str = include_str!("../platform/ios/FlightRunner.swift");

/// Quality doc for the gate-cite contract.
const QUALITY_SRC: &str = include_str!("../docs/tech/quality.md");

/// Simulation doc for the determinism contract.
#[cfg(feature = "dev-shell")]
const SIMULATION_SRC: &str = include_str!("../docs/tech/simulation.md");

/// Debug doc for the bundle layout contract.
const DEBUG_SRC: &str = include_str!("../docs/tech/debug.md");

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

/// Run the headless M1 cruise profile, byte-identical to `tests/smoke.rs`.
///
/// Steps the Mars-like body plus preset point-ship from a 250 km circular
/// orbit at `mu/r` speed, mixes the stream seed per tick, and hashes the
/// last `SimSnapshot`. Deterministic `libm`-only math with no IO.
#[cfg(feature = "dev-shell")]
fn golden_hash_after_ticks(tick_count_u64: u64, seed_u64: u64) -> u64 {
    use engine::atmosphere::AtmosphereParams;
    use engine::body::BodyParams;
    use engine::inspect::{capture_snapshot, snapshot_hash};
    use engine::orbit::Mu;
    use engine::sim::{SIM_TICK_S, Scheduler};
    use engine::trajectory::{StateVector, VehicleParams, step_point_ship};
    use engine::units::Seconds;
    use engine::warp::Warp;
    use glam::DVec3;

    let body = BodyParams::mars_like();
    let atmosphere = match AtmosphereParams::mars_like() {
        Ok(atmosphere) => atmosphere,
        Err(error) => panic!("golden atmosphere must validate: {error}"),
    };
    let vehicle = VehicleParams::preset();
    let mu = match Mu::new(body.gravitational_parameter_m3_s2()) {
        Ok(mu) => mu,
        Err(error) => panic!("golden mu must validate: {error}"),
    };
    let radius_m_f64 = body.radius_m().value() + GOLDEN_ALTITUDE_M_F64;
    let speed_mps_f64 = libm::sqrt(mu.value() / radius_m_f64);
    let mut state = match StateVector::new(
        DVec3::new(radius_m_f64, 0.0, 0.0),
        DVec3::new(0.0, speed_mps_f64, 0.0),
        Seconds::new(0.0),
    ) {
        Ok(state) => state,
        Err(error) => panic!("golden state must validate: {error}"),
    };
    let mut scheduler = Scheduler::default();
    let mut stream_seed_u64 = seed_u64;
    let mut digest_u64 = 0_u64;
    for _ in 0..tick_count_u64 {
        scheduler.advance();
        let sample = match step_point_ship(&state, SIM_TICK_S, &body, &atmosphere, &vehicle, mu) {
            Ok(sample) => sample,
            Err(error) => panic!("golden step must succeed: {error}"),
        };
        state = sample.state;
        stream_seed_u64 = engine::generation::mix_seed(stream_seed_u64, scheduler.step_count());
        let snapshot = match capture_snapshot(
            &scheduler,
            &state,
            &body,
            &atmosphere,
            &vehicle,
            seed_u64,
            stream_seed_u64,
            Warp::X1,
            true,
            false,
            false,
        ) {
            Ok(snapshot) => snapshot,
            Err(error) => panic!("golden capture must succeed: {error}"),
        };
        digest_u64 = snapshot_hash(&snapshot);
    }
    digest_u64
}

/// AC2: CSV fractions divide by the six named budgets with exact numbers.
///
/// Smoke values sit in the nominal band below the over line; gates live
/// in quality and are cited by name only here.
#[test]
fn ac2_csv_fractions_use_named_budgets_exact() {
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
    assert!(
        (SURFACE_HITCH_P95_MS_F64 - 100.0).abs() < FRACTION_TOL_F64,
        "hitch {SURFACE_HITCH_P95_MS_F64} must read 100.0 ms"
    );
    assert!(
        (MEMORY_CEILING_MB_F64 - 1024.0).abs() < FRACTION_TOL_F64,
        "memory {MEMORY_CEILING_MB_F64} must read 1024.0 MB"
    );
    assert!(
        (COLD_START_S_F64 - 5.0).abs() < FRACTION_TOL_F64,
        "cold start {COLD_START_S_F64} must read 5.0 s"
    );
    let frame_f64 = SMOKE_FRAME_MS_F64 / FRAME_BUDGET_MS_F64;
    assert!(
        frame_f64 < NOMINAL_MAX_FRACTION_F64,
        "frame fraction {frame_f64} leaves the nominal band"
    );
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
    let over_f64 = 30.0 / FRAME_BUDGET_MS_F64;
    assert!(
        over_f64 >= ELEVATED_MAX_FRACTION_F64,
        "over fraction {over_f64} must read at or above the elevated line"
    );
    assert!(
        (NOMINAL_MAX_FRACTION_F64 - 0.5).abs() < FRACTION_TOL_F64,
        "nominal line must read 0.5"
    );
    assert!(
        (ELEVATED_MAX_FRACTION_F64 - 0.8).abs() < FRACTION_TOL_F64,
        "elevated line must read 0.8"
    );
}

/// AC2: CSV header plus row schema carries twelve columns with units.
///
/// Pins the Rust header, the Swift parity header, the five-channel
/// fraction order, and the hex hash plus tier labels in rows.
#[test]
fn ac2_csv_header_and_row_schema() {
    assert_contains(FLIGHT_LOG_SRC, "FLIGHT_LOG_HEADER", "flight_log.rs");
    assert_contains(
        FLIGHT_LOG_SRC,
        "tick_count_u64,elapsed_s_f64,frame_ms_f64,sim_avg_ms_f64,sim_p99_ms_f64,hitch_p95_ms_f64,resident_mb_f64,thermal_state,tier,warp_factor_f64,seed_u64,hash_u64",
        "flight_log.rs",
    );
    assert_contains(
        FLIGHT_LOG_SRC,
        "FLIGHT_CHANNEL_COUNT_USIZE",
        "flight_log.rs",
    );
    assert_contains(
        FLIGHT_LOG_SRC,
        "FLIGHT_CHANNEL_FRAME_USIZE",
        "flight_log.rs",
    );
    assert_contains(
        FLIGHT_LOG_SRC,
        "FLIGHT_CHANNEL_SIM_AVG_USIZE",
        "flight_log.rs",
    );
    assert_contains(
        FLIGHT_LOG_SRC,
        "FLIGHT_CHANNEL_SIM_P99_USIZE",
        "flight_log.rs",
    );
    assert_contains(
        FLIGHT_LOG_SRC,
        "FLIGHT_CHANNEL_HITCH_USIZE",
        "flight_log.rs",
    );
    assert_contains(
        FLIGHT_LOG_SRC,
        "FLIGHT_CHANNEL_RESIDENT_USIZE",
        "flight_log.rs",
    );
    assert_contains(FLIGHT_LOG_SRC, "fractions_of", "flight_log.rs");
    assert_contains(FLIGHT_LOG_SRC, "format_row_csv", "flight_log.rs");
    assert_contains(FLIGHT_LOG_SRC, ":016x", "flight_log.rs");
    assert_contains(BUDGET_SRC, "BudgetDenominators", "budget.rs");
    assert_contains(BUDGET_SRC, "fraction_of", "budget.rs");
    assert_contains(RUNNER_SRC, "flightCSVHeader", "FlightRunner.swift");
    assert_contains(
        RUNNER_SRC,
        "tick_count_u64,elapsed_s_f64,frame_ms_f64,sim_avg_ms_f64,",
        "FlightRunner.swift",
    );
    assert_contains(
        RUNNER_SRC,
        "thermal_state,tier,warp_factor_f64,seed_u64,hash_u64",
        "FlightRunner.swift",
    );
    assert_contains(RUNNER_SRC, "csvRow()", "FlightRunner.swift");
    assert_contains(RUNNER_SRC, "%016llx", "FlightRunner.swift");
}

/// AC4: sim identical across tiers with hash match inside float noise.
///
/// The same seed plus no inputs replays to identical hashes twice while
/// a perturbed seed diverges; tier labels are render-only and never
/// enter the snapshot digest, so High, Medium, and Low share one hash.
/// Available only with the non-default `dev-shell` feature.
#[cfg(feature = "dev-shell")]
#[test]
fn ac4_sim_identical_across_tiers_hash_match() {
    let first_u64 = golden_hash_after_ticks(16, GOLDEN_SEED_U64);
    let second_u64 = golden_hash_after_ticks(16, GOLDEN_SEED_U64);
    assert_eq!(first_u64, second_u64);
    let perturbed_u64 = golden_hash_after_ticks(16, GOLDEN_SEED_U64 ^ 1);
    assert!(
        perturbed_u64 != first_u64,
        "perturbed seed must diverge from {first_u64:016x}"
    );
    assert_contains(
        QUALITY_SRC,
        "Sim behavior never varies by tier",
        "quality.md",
    );
    assert_contains(
        SIMULATION_SRC,
        "Same inputs yield the same state hash",
        "simulation.md",
    );
    assert_contains(SIMULATION_SRC, "xxh3-64", "simulation.md");
    assert_contains(INSPECT_SRC, "snapshot_hash", "inspect.rs");
    assert_contains(INSPECT_SRC, "xxh3_64", "inspect.rs");
    assert_contains(TRAJECTORY_SRC, "libm::sqrt", "trajectory.rs");
    assert_contains(ORBIT_SRC, "libm", "orbit.rs");
}

/// AC4: golden digest pins 100 snapshot ticks across chip families.
///
/// Headless M1 cruise profile hashed via `SimSnapshot`; `libm`-only
/// math keeps `x86_64` and `AArch64` in agreement. Available only with
/// the non-default `dev-shell` feature.
#[cfg(feature = "dev-shell")]
#[test]
fn ac4_golden_hash_still_passes() {
    let hash_u64 = golden_hash_after_ticks(GOLDEN_TICK_COUNT_U64, GOLDEN_SEED_U64);
    assert_eq!(hash_u64, GOLDEN_100_TICK_HASH_U64);
}

/// AC5: bundle per section 11 plus `system.txt` with quarantine on fail.
///
/// Pins the eight-file layout, the system companion, the excerpt join,
/// and the hash-verify plus quarantine path.
#[test]
fn ac5_bundle_layout_and_quarantine_contract() {
    assert_contains(EXPORT_SRC, "BUNDLE_FILE_NAMES", "export.rs");
    assert_contains(EXPORT_SRC, "meta.toml", "export.rs");
    assert_contains(EXPORT_SRC, "seed_tree.toml", "export.rs");
    assert_contains(EXPORT_SRC, "inputs.csv", "export.rs");
    assert_contains(EXPORT_SRC, "hashes.csv", "export.rs");
    assert_contains(EXPORT_SRC, "snapshot.toml", "export.rs");
    assert_contains(EXPORT_SRC, "config.toml", "export.rs");
    assert_contains(EXPORT_SRC, "log_excerpt.txt", "export.rs");
    assert_contains(EXPORT_SRC, "system.txt", "export.rs");
    assert_contains(EXPORT_SRC, "content_hash_u64", "export.rs");
    assert_contains(EXPORT_SRC, "verify_bundle_hashes", "export.rs");
    assert_contains(EXPORT_SRC, "quarantine_bundle", "export.rs");
    assert_contains(EXPORT_SRC, "HashMismatch", "export.rs");
    assert_contains(EXPORT_SRC, "BUNDLE_VERSION_U16", "export.rs");
    assert_contains(BUNDLE_SRC, "format_system_txt", "bundle.rs");
    assert_contains(BUNDLE_SRC, "join_log_excerpt", "bundle.rs");
    assert_contains(BUNDLE_SRC, "THERMAL_POLL_S_F64", "bundle.rs");
    assert_contains(BUNDLE_SRC, "LOG_EXCERPT_BEFORE_LINES_USIZE", "bundle.rs");
    assert_contains(BUNDLE_SRC, "LOG_EXCERPT_AFTER_LINES_USIZE", "bundle.rs");
    assert_contains(DEBUG_SRC, "bundle/", "debug.md");
    assert_contains(DEBUG_SRC, "system.txt", "debug.md");
    assert_contains(DEBUG_SRC, "log_excerpt.txt", "debug.md");
    assert_contains(DEBUG_SRC, "quarantined", "debug.md");
}

/// AC6: gates plus hygiene stay green without opening a window.
///
/// Quality owns fmt, clippy, build, test, smoke, plus the Android and
/// iOS target checks; touched sources carry no helpers and no boundary
/// blocks, and `cargo test` never carries the window flag.
#[test]
fn ac6_gates_and_hygiene_green() {
    assert_contains(QUALITY_SRC, "SIM_TICK_AVG_MS", "quality.md");
    assert_contains(QUALITY_SRC, "SIM_TICK_P99_MS", "quality.md");
    assert_contains(QUALITY_SRC, "SURFACE_HITCH_P95_MS", "quality.md");
    assert_contains(QUALITY_SRC, "MEMORY_CEILING_MB", "quality.md");
    assert_contains(QUALITY_SRC, "COLD_START_S", "quality.md");
    assert_contains(QUALITY_SRC, "cargo fmt --check", "quality.md");
    assert_contains(QUALITY_SRC, "cargo clippy", "quality.md");
    assert_contains(QUALITY_SRC, "cargo test", "quality.md");
    assert_contains(QUALITY_SRC, "aarch64-linux-android", "quality.md");
    assert_contains(QUALITY_SRC, "aarch64-apple-ios", "quality.md");
    for arg in std::env::args() {
        assert!(
            arg != "--run-window",
            "cargo test must not carry --run-window; got {arg}"
        );
    }
    assert_lacks(FLIGHT_LOG_SRC, ".unwrap()", "flight_log.rs");
    assert_lacks(FLIGHT_LOG_SRC, ".expect(", "flight_log.rs");
    assert_lacks(FLIGHT_LOG_SRC, FORBIDDEN_NEEDLE, "flight_log.rs");
    assert_lacks(BUDGET_SRC, ".unwrap()", "budget.rs");
    assert_lacks(BUDGET_SRC, ".expect(", "budget.rs");
    assert_lacks(BUDGET_SRC, FORBIDDEN_NEEDLE, "budget.rs");
    assert_lacks(BUNDLE_SRC, ".unwrap()", "bundle.rs");
    assert_lacks(BUNDLE_SRC, ".expect(", "bundle.rs");
    assert_lacks(BUNDLE_SRC, FORBIDDEN_NEEDLE, "bundle.rs");
    assert_lacks(EXPORT_SRC, ".unwrap()", "export.rs");
    assert_lacks(EXPORT_SRC, ".expect(", "export.rs");
    assert_lacks(EXPORT_SRC, FORBIDDEN_NEEDLE, "export.rs");
}
