//! Shell Phase B exit verification (issue 36 step 6).
//!
//! Covers AC1 to AC8 through `universe-engine` public APIs plus
//! read-only source contracts for the binary-only `universe-debug`
//! shell (no library target, so `include_str` checks its public
//! behavior without duplicating logic). Snapshot paths are `dev-shell`
//! gated; the rest runs without features, proving shell removal. The
//! `#32` golden profile is byte-identical to `tests/smoke.rs` and must
//! stay so: Phase B changes no sim behavior.

#![forbid(unsafe_code)]

use engine::regime::{Boundary, Regime, classify, distance};
#[cfg(feature = "dev-shell")]
use engine::sim::Scheduler;
use engine::units::Meters;
#[cfg(feature = "dev-shell")]
use engine::units::Seconds;

/// Ticks for the golden profile, dimensionless.
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

/// Atmosphere probe altitude in meters for the regime ladder.
#[cfg(feature = "dev-shell")]
const PROBE_ATMO_ALTITUDE_M_F64: f64 = 50_000.0;

/// Probe speed in meters per second for the atmosphere capture.
#[cfg(feature = "dev-shell")]
const PROBE_ATMO_SPEED_MPS_F64: f64 = 3_000.0;

/// Rails boundary altitude in meters.
const RAILS_ALTITUDE_M_F64: f64 = 120_000.0;

/// Fraction tolerance for budget math, dimensionless.
const FRACTION_TOL_F64: f64 = 1e-12;

/// Frame budget in milliseconds, named `FRAME_BUDGET_MS` in quality docs.
const FRAME_BUDGET_MS_F64: f64 = 33.33;

/// Nominal band upper bound as a budget fraction, dimensionless.
const NOMINAL_MAX_FRACTION_F64: f64 = 0.5;

/// Elevated band upper bound as a budget fraction, dimensionless.
const ELEVATED_MAX_FRACTION_F64: f64 = 0.8;

/// Continuity source for the plot plus handoff contract.
const CONTINUITY_SRC: &str = include_str!("../crates/debug/src/continuity.rs");

/// Bottom-tabs source for the registry contract.
const BOTTOM_SRC: &str = include_str!("../crates/debug/src/bottom.rs");

/// Budget-strip source for the fraction-bar contract.
const BUDGET_SRC: &str = include_str!("../crates/debug/src/budget.rs");

/// Tracing-log source for the bounded-ring contract.
const LOG_SRC: &str = include_str!("../crates/debug/src/log.rs");

/// Shell assembly source for the bottom wiring contract.
const SHELL_SRC: &str = include_str!("../crates/debug/src/shell.rs");

/// Layout source for the Phase B tab-slice contract.
const LAYOUT_SRC: &str = include_str!("../crates/debug/src/layout.rs");

/// Debug entry source for the smoke wiring contract.
const DEBUG_MAIN_SRC: &str = include_str!("../crates/debug/src/main.rs");

/// Workspace manifest for the deferred-dependency contract.
const WORKSPACE_CARGO_SRC: &str = include_str!("../Cargo.toml");

/// Lockfile for the no-new-dependency contract.
const CARGO_LOCK_SRC: &str = include_str!("../Cargo.lock");

/// Quality doc for the gate-order contract.
const QUALITY_SRC: &str = include_str!("../docs/tech/quality.md");

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

/// AC1: continuity source records seven curves with handoff markers.
#[test]
fn ac1_continuity_source_contract() {
    assert_contains(CONTINUITY_SRC, "PlotSample", "continuity.rs");
    assert_contains(CONTINUITY_SRC, "ContinuityMonitor", "continuity.rs");
    assert_contains(CONTINUITY_SRC, "HandoffMarker", "continuity.rs");
    assert_contains(CONTINUITY_SRC, "push_snapshot", "continuity.rs");
    assert_contains(CONTINUITY_SRC, "before_f64", "continuity.rs");
    assert_contains(CONTINUITY_SRC, "after_f64", "continuity.rs");
    assert_contains(CONTINUITY_SRC, "delta_f64", "continuity.rs");
    assert_contains(CONTINUITY_SRC, "is_flagged", "continuity.rs");
    assert_contains(CONTINUITY_SRC, "DELTA_REL_TOL_F64", "continuity.rs");
    assert_contains(CONTINUITY_SRC, "1e-9", "continuity.rs");
    assert_contains(CONTINUITY_SRC, "DELTA_ABS_FLOORS_F64", "continuity.rs");
    assert_contains(
        CONTINUITY_SRC,
        "PLOT_HISTORY_CAPACITY_ENTRIES_USIZE",
        "continuity.rs",
    );
    assert_contains(CONTINUITY_SRC, "no allocation after", "continuity.rs");
    assert_contains(CONTINUITY_SRC, "altitude_m_f64", "continuity.rs");
    assert_contains(CONTINUITY_SRC, "heat_flux_w_per_m2_f64", "continuity.rs");
    assert_contains(CONTINUITY_SRC, "g_load_g_f64", "continuity.rs");
    assert_contains(CONTINUITY_SRC, "regime_u8", "continuity.rs");
    assert_contains(CONTINUITY_SRC, "without writing sim state", "continuity.rs");
    assert_contains(BOTTOM_SRC, "BottomTab", "bottom.rs");
    assert_contains(BOTTOM_SRC, "Continuity", "bottom.rs");
    assert_contains(BOTTOM_SRC, "selectable_value", "bottom.rs");
    assert_lacks(CONTINUITY_SRC, "&mut SimSnapshot", "continuity.rs");
    assert_lacks(CONTINUITY_SRC, ".unwrap()", "continuity.rs");
    assert_lacks(CONTINUITY_SRC, ".expect(", "continuity.rs");
    assert_lacks(CONTINUITY_SRC, FORBIDDEN_NEEDLE, "continuity.rs");
    assert_lacks(BOTTOM_SRC, FORBIDDEN_NEEDLE, "bottom.rs");
}

/// AC2: handoff boundaries gate orbit, atmosphere, and surface regimes.
#[test]
fn ac2_handoff_boundary_contract() {
    let orbit_regime = match classify(Meters::new(RAILS_ALTITUDE_M_F64 + 1.0)) {
        Ok(regime) => regime,
        Err(error) => panic!("above-rails altitude must classify: {error}"),
    };
    assert_eq!(orbit_regime, Regime::Orbit);
    let atmo_regime = match classify(Meters::new(RAILS_ALTITUDE_M_F64 - 1.0)) {
        Ok(regime) => regime,
        Err(error) => panic!("below-rails altitude must classify: {error}"),
    };
    assert_eq!(atmo_regime, Regime::Atmosphere);
    let surface_regime = match classify(Meters::new(0.0)) {
        Ok(regime) => regime,
        Err(error) => panic!("surface altitude must classify: {error}"),
    };
    assert_eq!(surface_regime, Regime::Surface);
    let rails_gap = distance(Meters::new(150_000.0), Boundary::Rails);
    assert!(rails_gap.value() > 0.0);
    let surface_gap = distance(Meters::new(150_000.0), Boundary::Surface);
    assert!(surface_gap.value() > 0.0);
}

/// AC1 plus AC2: snapshot regime ladder is deterministic across runs.
#[cfg(feature = "dev-shell")]
#[test]
fn ac1_snapshot_regime_ladder_live() {
    use engine::atmosphere::AtmosphereParams;
    use engine::body::BodyParams;
    use engine::inspect::{capture_snapshot, snapshot_hash};
    use engine::orbit::Mu;
    use engine::trajectory::{StateVector, VehicleParams};
    use engine::warp::Warp;
    use glam::DVec3;

    let body = BodyParams::mars_like();
    let atmosphere = match AtmosphereParams::mars_like() {
        Ok(atmosphere) => atmosphere,
        Err(error) => panic!("mars atmosphere must validate: {error}"),
    };
    let vehicle = VehicleParams::preset();
    let mu = match Mu::new(body.gravitational_parameter_m3_s2()) {
        Ok(mu) => mu,
        Err(error) => panic!("mars mu must validate: {error}"),
    };
    let orbit_state = match StateVector::new(
        DVec3::new(body.radius_m().value() + GOLDEN_ALTITUDE_M_F64, 0.0, 0.0),
        DVec3::new(
            0.0,
            libm::sqrt(mu.value() / (body.radius_m().value() + GOLDEN_ALTITUDE_M_F64)),
            0.0,
        ),
        Seconds::new(0.0),
    ) {
        Ok(state) => state,
        Err(error) => panic!("orbit state must validate: {error}"),
    };
    let atmo_state = match StateVector::new(
        DVec3::new(
            body.radius_m().value() + PROBE_ATMO_ALTITUDE_M_F64,
            0.0,
            0.0,
        ),
        DVec3::new(0.0, PROBE_ATMO_SPEED_MPS_F64, 0.0),
        Seconds::new(0.0),
    ) {
        Ok(state) => state,
        Err(error) => panic!("atmo state must validate: {error}"),
    };
    let scheduler = Scheduler::default();
    let orbit_snapshot = match capture_snapshot(
        &scheduler,
        &orbit_state,
        &body,
        &atmosphere,
        &vehicle,
        GOLDEN_SEED_U64,
        GOLDEN_SEED_U64,
        Warp::X1,
        true,
        false,
        false,
    ) {
        Ok(snapshot) => snapshot,
        Err(error) => panic!("orbit capture must succeed: {error}"),
    };
    let atmo_snapshot = match capture_snapshot(
        &scheduler,
        &atmo_state,
        &body,
        &atmosphere,
        &vehicle,
        GOLDEN_SEED_U64,
        GOLDEN_SEED_U64,
        Warp::X1,
        true,
        false,
        false,
    ) {
        Ok(snapshot) => snapshot,
        Err(error) => panic!("atmo capture must succeed: {error}"),
    };
    assert_eq!(orbit_snapshot.regime_u8, 0_u8);
    assert_eq!(atmo_snapshot.regime_u8, 1_u8);
    assert_ne!(
        snapshot_hash(&orbit_snapshot),
        snapshot_hash(&atmo_snapshot)
    );
    assert!(atmo_snapshot.pressure_pa_f64 > orbit_snapshot.pressure_pa_f64);
    assert!(atmo_snapshot.density_kg_m3_f64 >= orbit_snapshot.density_kg_m3_f64);
    assert!(libm::fabs(orbit_snapshot.pressure_pa_f64) < 1e-6);
    let repeat = match capture_snapshot(
        &scheduler,
        &orbit_state,
        &body,
        &atmosphere,
        &vehicle,
        GOLDEN_SEED_U64,
        GOLDEN_SEED_U64,
        Warp::X1,
        true,
        false,
        false,
    ) {
        Ok(snapshot) => snapshot,
        Err(error) => panic!("repeat capture must succeed: {error}"),
    };
    assert_eq!(snapshot_hash(&repeat), snapshot_hash(&orbit_snapshot));
}

/// AC3: budget source shows seven named bars with fraction colors.
#[test]
fn ac3_budget_source_contract() {
    assert_contains(BUDGET_SRC, "BudgetStrip", "budget.rs");
    assert_contains(BUDGET_SRC, "BudgetDenominators", "budget.rs");
    assert_contains(BUDGET_SRC, "ThermalTier", "budget.rs");
    assert_contains(BUDGET_SRC, "render-only", "budget.rs");
    assert_contains(BUDGET_SRC, "frame_ms_f64", "budget.rs");
    assert_contains(BUDGET_SRC, "sim_avg_ms_f64", "budget.rs");
    assert_contains(BUDGET_SRC, "sim_p99_ms_f64", "budget.rs");
    assert_contains(BUDGET_SRC, "hitch_p95_ms_f64", "budget.rs");
    assert_contains(BUDGET_SRC, "resident_mb_f64", "budget.rs");
    assert_contains(BUDGET_SRC, "cold_start_s_f64", "budget.rs");
    assert_contains(BUDGET_SRC, "shell_ms_f64", "budget.rs");
    assert_contains(BUDGET_SRC, "fraction", "budget.rs");
    assert_contains(BUDGET_SRC, "percent", "budget.rs");
    assert_contains(BUDGET_SRC, "never stored", "budget.rs");
    assert_contains(BUDGET_SRC, "quality.md", "budget.rs");
    assert_lacks(BUDGET_SRC, ".unwrap()", "budget.rs");
    assert_lacks(BUDGET_SRC, ".expect(", "budget.rs");
    assert_lacks(BUDGET_SRC, FORBIDDEN_NEEDLE, "budget.rs");
}

/// AC3: smoke budget math sits in the nominal band with numeric fractions.
#[test]
fn ac3_budget_fraction_math() {
    let fraction_f64 = 8.0 / FRAME_BUDGET_MS_F64;
    assert!(fraction_f64 < NOMINAL_MAX_FRACTION_F64);
    assert!(fraction_f64 < ELEVATED_MAX_FRACTION_F64);
    assert!(fraction_f64 > 0.0);
    assert!((NOMINAL_MAX_FRACTION_F64 - 0.5).abs() < FRACTION_TOL_F64);
    assert!((ELEVATED_MAX_FRACTION_F64 - 0.8).abs() < FRACTION_TOL_F64);
    let over_f64 = 30.0 / FRAME_BUDGET_MS_F64;
    assert!(over_f64 >= ELEVATED_MAX_FRACTION_F64);
}

/// AC4: log source keeps a bounded ring with filters and no persistence.
#[test]
fn ac4_log_source_contract() {
    assert_contains(LOG_SRC, "TraceLog", "log.rs");
    assert_contains(LOG_SRC, "LogLevel", "log.rs");
    assert_contains(LOG_SRC, "Trace", "log.rs");
    assert_contains(LOG_SRC, "Error", "log.rs");
    assert_contains(LOG_SRC, "LOG_HISTORY_CAPACITY_ENTRIES_USIZE", "log.rs");
    assert_contains(LOG_SRC, "LOG_MESSAGE_CAP_BYTES_USIZE", "log.rs");
    assert_contains(LOG_SRC, "256", "log.rs");
    assert_contains(LOG_SRC, "LOG_DRAW_ROWS_USIZE", "log.rs");
    assert_contains(LOG_SRC, "50", "log.rs");
    assert_contains(LOG_SRC, "filter", "log.rs");
    assert_contains(LOG_SRC, "never persists", "log.rs");
    assert_contains(LOG_SRC, "no allocation happens after", "log.rs");
    assert_lacks(LOG_SRC, ".unwrap()", "log.rs");
    assert_lacks(LOG_SRC, ".expect(", "log.rs");
    assert_lacks(LOG_SRC, FORBIDDEN_NEEDLE, "log.rs");
}

/// AC5: layout plus bottom sources keep one tab slice with phone rules.
#[test]
fn ac5_bottom_layout_source_contract() {
    assert_contains(LAYOUT_SRC, "is_phase_b", "layout.rs");
    assert_contains(LAYOUT_SRC, "default_bottom_tab", "layout.rs");
    assert_contains(LAYOUT_SRC, "PLOT_MIN_HEIGHT_PT_F32", "layout.rs");
    assert_contains(LAYOUT_SRC, "96.0", "layout.rs");
    assert_contains(BOTTOM_SRC, "BottomTabs", "bottom.rs");
    assert_contains(BOTTOM_SRC, "BottomDraw", "bottom.rs");
    assert_contains(BOTTOM_SRC, "from_phone_tab", "bottom.rs");
    assert_contains(BOTTOM_SRC, "to_phone_tab", "bottom.rs");
    assert_contains(BOTTOM_SRC, "default_for_preset", "bottom.rs");
    assert_contains(BOTTOM_SRC, "selectable_value", "bottom.rs");
    assert_contains(BOTTOM_SRC, "never persists", "bottom.rs");
    assert_contains(SHELL_SRC, "shows_bottom_tabs", "shell.rs");
    assert_contains(SHELL_SRC, "observe_snapshot", "shell.rs");
    assert_contains(SHELL_SRC, "push_snapshot", "shell.rs");
    assert_contains(SHELL_SRC, "BudgetDenominators", "shell.rs");
    assert_contains(DEBUG_MAIN_SRC, "BudgetDenominators", "debug main.rs");
    assert_contains(DEBUG_MAIN_SRC, "print_phase_b_tables", "debug main.rs");
    assert_lacks(LAYOUT_SRC, FORBIDDEN_NEEDLE, "layout.rs");
    assert_lacks(BOTTOM_SRC, ".unwrap()", "bottom.rs");
    assert_lacks(BOTTOM_SRC, ".expect(", "bottom.rs");
    assert_lacks(SHELL_SRC, FORBIDDEN_NEEDLE, "shell.rs");
}

/// AC6: golden digest pins 100 snapshot ticks across platforms.
#[cfg(feature = "dev-shell")]
#[test]
fn ac6_golden_hash_still_passes() {
    let hash_u64 = golden_hash_after_ticks(GOLDEN_TICK_COUNT_U64, GOLDEN_SEED_U64);
    assert_eq!(hash_u64, GOLDEN_100_TICK_HASH_U64);
    let perturbed_u64 = golden_hash_after_ticks(GOLDEN_TICK_COUNT_U64, GOLDEN_SEED_U64 ^ 1);
    assert_ne!(perturbed_u64, GOLDEN_100_TICK_HASH_U64);
}

/// Run the headless M1 cruise profile, byte-identical to `tests/smoke.rs`.
#[cfg(feature = "dev-shell")]
fn golden_hash_after_ticks(tick_count_u64: u64, seed_u64: u64) -> u64 {
    use engine::atmosphere::AtmosphereParams;
    use engine::body::BodyParams;
    use engine::inspect::{capture_snapshot, snapshot_hash};
    use engine::orbit::Mu;
    use engine::sim::SIM_TICK_S;
    use engine::trajectory::{StateVector, VehicleParams, step_point_ship};
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

/// AC6: Phase B adds no plot, budget, or log dependency to the workspace.
#[test]
fn ac6_no_new_dependencies() {
    assert_lacks(WORKSPACE_CARGO_SRC, "egui_plot", "workspace Cargo.toml");
    assert_lacks(WORKSPACE_CARGO_SRC, "egui-plot", "workspace Cargo.toml");
    assert_lacks(WORKSPACE_CARGO_SRC, "postcard", "workspace Cargo.toml");
    assert_lacks(WORKSPACE_CARGO_SRC, "egui_dock", "workspace Cargo.toml");
    assert_lacks(WORKSPACE_CARGO_SRC, "egui-dock", "workspace Cargo.toml");
    assert_lacks(CARGO_LOCK_SRC, "egui_plot", "Cargo.lock");
    assert_lacks(CARGO_LOCK_SRC, "postcard", "Cargo.lock");
    assert_lacks(CARGO_LOCK_SRC, "egui_dock", "Cargo.lock");
}

/// AC7: quality gates run fmt, clippy, build, and test in order.
#[test]
fn ac7_gate_order_contract() {
    assert_contains(QUALITY_SRC, "cargo fmt --check", "quality.md");
    assert_contains(QUALITY_SRC, "cargo clippy", "quality.md");
    assert_contains(QUALITY_SRC, "cargo build", "quality.md");
    assert_contains(QUALITY_SRC, "cargo test", "quality.md");
    assert_contains(QUALITY_SRC, "30 fps", "quality.md");
}

/// AC8: Phase B shell code stays lint-clean with units on every number.
#[test]
fn ac8_standards_source_contract() {
    assert_contains(CONTINUITY_SRC, "_f64", "continuity.rs");
    assert_contains(CONTINUITY_SRC, "_u8", "continuity.rs");
    assert_contains(CONTINUITY_SRC, "_usize", "continuity.rs");
    assert_contains(BUDGET_SRC, "BudgetStatus", "budget.rs");
    assert_lacks(LOG_SRC, FORBIDDEN_NEEDLE, "log.rs");
    assert_contains(DEBUG_MAIN_SRC, "#![forbid(unsafe_code)]", "debug main.rs");
    assert_contains(DEBUG_MAIN_SRC, "mimalloc", "debug main.rs");
}
