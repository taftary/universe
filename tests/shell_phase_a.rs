//! Shell Phase A exit verification (issue 34 step 6).
//!
//! Covers AC1 to AC6 through `universe-engine` public APIs plus
//! read-only source contracts for the binary-only `universe-debug`
//! shell (no library target, so `include_str` checks its public
//! behavior without duplicating logic). `SimSnapshot` paths are
//! `dev-shell` gated; the rest runs without features, proving shell
//! removal. The `#32` golden profile is byte-identical to
//! `tests/smoke.rs` and must stay so.

#![forbid(unsafe_code)]

use engine::regime::{Boundary, Regime, classify, distance, warp_context_for_regime};
use engine::sim::{SIM_TICK_S, Scheduler};
use engine::units::{Meters, Seconds};
use engine::warp::{Warp, WarpContext, apply_auto_drop, request_warp};

/// Fixed step in seconds, mirrors `SIM_TICK_S`.
const EXPECTED_TICK_S_F64: f64 = 0.05;

/// Ticks for the scheduler advance check, dimensionless.
const SCHEDULER_TICK_COUNT_U64: u64 = 8;

/// Drift tolerance in seconds for fixed-step accumulation.
const TIME_TOLERANCE_S_F64: f64 = 1e-9;

/// Hundredfold warp factor, dimensionless.
const WARP_FACTOR_HUNDRED_F64: f64 = 100.0;

/// Ten-thousandfold warp factor, dimensionless.
const WARP_FACTOR_MAX_F64: f64 = 10_000.0;

/// Fraction tolerance for warp factor comparison, dimensionless.
const FACTOR_TOL_F64: f64 = 1e-12;

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

/// Master seed for inspect checks, dimensionless.
#[cfg(feature = "dev-shell")]
const INSPECT_MASTER_SEED_U64: u64 = 0x243F_6A88_85A3_08D3;

/// Stream seed for inspect checks, dimensionless.
#[cfg(feature = "dev-shell")]
const INSPECT_STREAM_SEED_U64: u64 = 0x1319_8A2E_0370_7344;

/// Zero tolerance for aero fields above the rails, mixed units near zero.
#[cfg(feature = "dev-shell")]
const ZERO_TOL_F64: f64 = 1e-12;

/// Smoke shell draw cost in milliseconds for the measured-cost note.
const SMOKE_DRAW_MS_F64: f64 = 0.4;

/// Frame budget in milliseconds, named `FRAME_BUDGET_MS` in quality docs.
const FRAME_BUDGET_MS_F64: f64 = 33.33;

/// Nominal band upper bound as a budget fraction, dimensionless.
const NOMINAL_MAX_FRACTION_F64: f64 = 0.5;

/// Elevated band upper bound as a budget fraction, dimensionless.
const ELEVATED_MAX_FRACTION_F64: f64 = 0.8;

/// Minimum touch target in points for the phone contract.
const MIN_TOUCH_TARGET_PT_F64: f64 = 44.0;

/// Top-bar source for the desktop run-control contract.
const TOP_BAR_SRC: &str = include_str!("../crates/debug/src/top_bar.rs");

/// Shell-cost source for the separate-cost contract.
const SHELL_COST_SRC: &str = include_str!("../crates/debug/src/shell_cost.rs");

/// Input-router source for the routing contract.
const INPUT_SRC: &str = include_str!("../crates/debug/src/input.rs");

/// Inspect-view source for the read-only contract.
const INSPECT_VIEW_SRC: &str = include_str!("../crates/debug/src/inspect_view.rs");

/// Shell assembly source for defaults and cost hook.
const SHELL_SRC: &str = include_str!("../crates/debug/src/shell.rs");

/// Layout source for the phone one-handed contract.
const LAYOUT_SRC: &str = include_str!("../crates/debug/src/layout.rs");

/// Theme source for budget-band thresholds.
const THEME_SRC: &str = include_str!("../crates/debug/src/theme.rs");

/// Debug entry source for the allocator and headless demo.
const DEBUG_MAIN_SRC: &str = include_str!("../crates/debug/src/main.rs");

/// Engine scheduler source for the `egui` boundary check.
const ENGINE_SIM_SRC: &str = include_str!("../crates/engine/src/sim.rs");

/// Engine manifest for the dependency boundary check.
const ENGINE_CARGO_SRC: &str = include_str!("../crates/engine/Cargo.toml");

/// Debug manifest for the `dev-shell` removal check.
const DEBUG_CARGO_SRC: &str = include_str!("../crates/debug/Cargo.toml");

/// Workspace manifest for the feature removal check.
const WORKSPACE_CARGO_SRC: &str = include_str!("../Cargo.toml");

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

/// Build a scheduler, panicking on rejection.
fn new_scheduler(step_s_f64: f64) -> Scheduler {
    match Scheduler::new(Seconds::new(step_s_f64)) {
        Ok(scheduler) => scheduler,
        Err(error) => panic!("positive step rejected: {error}"),
    }
}

/// AC1: the fixed-step scheduler advances exact multiples of the step.
#[test]
fn ac1_scheduler_is_fixed_step() {
    let mut scheduler = new_scheduler(EXPECTED_TICK_S_F64);
    for _ in 0..SCHEDULER_TICK_COUNT_U64 {
        scheduler.advance();
    }
    assert_eq!(scheduler.step_count(), SCHEDULER_TICK_COUNT_U64);
    assert!((scheduler.step().value() - EXPECTED_TICK_S_F64).abs() <= TIME_TOLERANCE_S_F64);
    assert!((SIM_TICK_S.value() - EXPECTED_TICK_S_F64).abs() <= TIME_TOLERANCE_S_F64);
    let expected_elapsed_s_f64 =
        f64::from(u32::try_from(SCHEDULER_TICK_COUNT_U64).unwrap_or(0_u32)) * EXPECTED_TICK_S_F64;
    let drift_s_f64 = (scheduler.elapsed().value() - expected_elapsed_s_f64).abs();
    assert!(
        drift_s_f64 <= TIME_TOLERANCE_S_F64,
        "drift {drift_s_f64} exceeds {TIME_TOLERANCE_S_F64}"
    );
}

/// AC1: warp factors, allow rules, and deny rules match the top-bar policy.
#[test]
fn ac1_warp_allow_and_deny() {
    assert_eq!(Warp::ALL.len(), 5);
    assert!((Warp::X1.factor() - 1.0).abs() < FACTOR_TOL_F64);
    assert!((Warp::X10.factor() - 10.0).abs() < FACTOR_TOL_F64);
    assert!((Warp::X100.factor() - WARP_FACTOR_HUNDRED_F64).abs() < FACTOR_TOL_F64);
    assert!((Warp::X1000.factor() - 1_000.0).abs() < FACTOR_TOL_F64);
    assert!((Warp::X10000.factor() - WARP_FACTOR_MAX_F64).abs() < FACTOR_TOL_F64);
    match request_warp(Warp::X100, WarpContext::cruise()) {
        Ok(granted) => assert_eq!(granted, Warp::X100),
        Err(error) => panic!("cruise warp must grant: {error}"),
    }
    match request_warp(Warp::X10000, WarpContext::cruise()) {
        Ok(granted) => assert_eq!(granted, Warp::X10000),
        Err(error) => panic!("cruise max warp must grant: {error}"),
    }
    assert!(request_warp(Warp::X10, WarpContext::on_foot()).is_err());
    assert!(request_warp(Warp::X100, WarpContext::new(true, true, true, false, false)).is_err());
    assert!(request_warp(Warp::X100, WarpContext::new(true, true, false, true, false)).is_err());
    assert!(
        request_warp(
            Warp::X1000,
            WarpContext::new(true, true, false, false, true)
        )
        .is_err()
    );
    assert!(matches!(
        request_warp(Warp::X1, WarpContext::on_foot()),
        Ok(Warp::X1)
    ));
}

/// AC1: auto-drop forces 1x on entry, approach, and alarm only.
#[test]
fn ac1_auto_drop_reasons() {
    assert!(!WarpContext::cruise().should_auto_drop());
    assert_eq!(
        apply_auto_drop(Warp::X10000, WarpContext::cruise()),
        Warp::X10000
    );
    let entry = WarpContext::new(true, true, true, false, false);
    assert!(entry.should_auto_drop());
    assert_eq!(apply_auto_drop(Warp::X10000, entry), Warp::X1);
    let approach = WarpContext::new(true, true, false, true, false);
    assert!(approach.should_auto_drop());
    assert_eq!(apply_auto_drop(Warp::X10000, approach), Warp::X1);
    let alarm = WarpContext::new(true, true, false, false, true);
    assert!(alarm.should_auto_drop());
    assert_eq!(apply_auto_drop(Warp::X10000, alarm), Warp::X1);
    assert!(!WarpContext::on_foot().should_auto_drop());
}

/// AC1: desktop top-bar source owns pause, step, warp, and drop display.
#[test]
fn ac1_top_bar_source_contract() {
    assert_contains(TOP_BAR_SRC, "TopBarState", "top_bar.rs");
    assert_contains(TOP_BAR_SRC, "pause", "top_bar.rs");
    assert_contains(TOP_BAR_SRC, "resume", "top_bar.rs");
    assert_contains(TOP_BAR_SRC, "request_step", "top_bar.rs");
    assert_contains(TOP_BAR_SRC, "take_step", "top_bar.rs");
    assert_contains(TOP_BAR_SRC, "request_warp", "top_bar.rs");
    assert_contains(TOP_BAR_SRC, "AutoDropReason", "top_bar.rs");
    assert_contains(TOP_BAR_SRC, "\"manual\"", "top_bar.rs");
    assert_contains(TOP_BAR_SRC, "\"entry\"", "top_bar.rs");
    assert_contains(TOP_BAR_SRC, "\"approach\"", "top_bar.rs");
    assert_contains(TOP_BAR_SRC, "\"alarm\"", "top_bar.rs");
    assert_contains(TOP_BAR_SRC, "WarpDenied", "top_bar.rs");
    assert_contains(TOP_BAR_SRC, "observe_snapshot_view", "top_bar.rs");
    assert_contains(TOP_BAR_SRC, "tick_count_u64", "top_bar.rs");
    assert_contains(TOP_BAR_SRC, "elapsed_s_f64", "top_bar.rs");
    assert_contains(TOP_BAR_SRC, "never writes sim", "top_bar.rs");
    assert_lacks(TOP_BAR_SRC, "unsafe", "top_bar.rs");
}

/// AC2: phone layout source keeps one shell, chips, sheet, and 44 pt targets.
#[test]
fn ac2_phone_layout_source_contract() {
    assert_contains(LAYOUT_SRC, "MIN_TOUCH_TARGET_PT_F32", "layout.rs");
    assert_contains(LAYOUT_SRC, "44.0", "layout.rs");
    assert_contains(LAYOUT_SRC, "ChipAction", "layout.rs");
    assert_contains(LAYOUT_SRC, "Pause", "layout.rs");
    assert_contains(LAYOUT_SRC, "Step", "layout.rs");
    assert_contains(LAYOUT_SRC, "Warp", "layout.rs");
    assert_contains(LAYOUT_SRC, "AutoDrop", "layout.rs");
    assert_contains(LAYOUT_SRC, "PhoneTab", "layout.rs");
    assert_contains(LAYOUT_SRC, "BottomSheetDetent", "layout.rs");
    assert_contains(LAYOUT_SRC, "DevTag", "layout.rs");
    assert_contains(LAYOUT_SRC, "top-right", "layout.rs");
    assert_contains(LAYOUT_SRC, "pixels_per_point", "layout.rs");
    assert_contains(LAYOUT_SRC, "96.0", "layout.rs");
    assert_contains(
        LAYOUT_SRC,
        "PLOT_HISTORY_CAPACITY_ENTRIES_USIZE",
        "layout.rs",
    );
    assert_contains(
        LAYOUT_SRC,
        "LOG_HISTORY_CAPACITY_ENTRIES_USIZE",
        "layout.rs",
    );
    assert_contains(
        LAYOUT_SRC,
        "INPUT_RECORDER_CAPACITY_ENTRIES_USIZE",
        "layout.rs",
    );
    assert_contains(INPUT_SRC, "DEV_TAG_LONG_PRESS_S_F64", "input.rs");
    assert_contains(INPUT_SRC, "0.5", "input.rs");
    assert_contains(INPUT_SRC, "THREE_FINGER_TAP_COUNT_U8", "input.rs");
    assert_contains(INPUT_SRC, "TAP_PICK_TOLERANCE_PT_F32", "input.rs");
    assert_contains(INPUT_SRC, "one-handed", "input.rs");
    assert!((MIN_TOUCH_TARGET_PT_F64 - 44.0).abs() < FACTOR_TOL_F64);
    assert_lacks(LAYOUT_SRC, "unsafe", "layout.rs");
}

/// AC3: inspect copies snapshot scalars read-only with hash short form.
#[cfg(feature = "dev-shell")]
#[test]
fn ac3_inspect_read_only_live() {
    use engine::atmosphere::AtmosphereParams;
    use engine::body::BodyParams;
    use engine::inspect::{SNAPSHOT_SIZE_BYTES, capture_snapshot, snapshot_hash};
    use engine::orbit::Mu;
    use engine::trajectory::{StateVector, VehicleParams, step_point_ship};
    use glam::DVec3;

    assert_eq!(SNAPSHOT_SIZE_BYTES, 304);
    assert_eq!(core::mem::size_of::<engine::inspect::SimSnapshot>(), 304);
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
    let radius_m_f64 = body.radius_m().value() + GOLDEN_ALTITUDE_M_F64;
    let speed_mps_f64 = libm::sqrt(mu.value() / radius_m_f64);
    let state = match StateVector::new(
        DVec3::new(radius_m_f64, 0.0, 0.0),
        DVec3::new(0.0, speed_mps_f64, 0.0),
        Seconds::new(0.0),
    ) {
        Ok(state) => state,
        Err(error) => panic!("orbit state must validate: {error}"),
    };
    let scheduler = Scheduler::default();
    let snapshot = match capture_snapshot(
        &scheduler,
        &state,
        &body,
        &atmosphere,
        &vehicle,
        INSPECT_MASTER_SEED_U64,
        INSPECT_STREAM_SEED_U64,
        Warp::X1,
        true,
        false,
        false,
    ) {
        Ok(snapshot) => snapshot,
        Err(error) => panic!("orbit capture must succeed: {error}"),
    };
    assert_eq!(snapshot.tick_count_u64, 0);
    assert_eq!(snapshot.master_seed_u64, INSPECT_MASTER_SEED_U64);
    assert_eq!(snapshot.stream_seed_u64, INSPECT_STREAM_SEED_U64);
    assert_eq!(snapshot.snapshot_hash_u64, snapshot_hash(&snapshot));
    assert!((snapshot.altitude_m_f64 - GOLDEN_ALTITUDE_M_F64).abs() < 1e-6);
    assert!(snapshot.speed_mps_f64 > 0.0);
    assert!(snapshot.pressure_pa_f64 >= 0.0);
    assert!(snapshot.temperature_k_f64 > 0.0);
    assert!(snapshot.density_kg_m3_f64 >= 0.0);
    assert_eq!(snapshot.elements_valid_u8, 1_u8);
    assert!(snapshot.semi_major_axis_m_f64 > 0.0);
    let reread = snapshot;
    assert_eq!(reread.tick_count_u64, snapshot.tick_count_u64);
    assert_eq!(reread.snapshot_hash_u64, snapshot.snapshot_hash_u64);
    let mut pick_only = snapshot;
    pick_only.pick_altitude_m_f64 = 1_000.0;
    pick_only.pick_range_m_f64 = 2_000.0;
    pick_only.pick_body_id_u32 = 1_u32;
    pick_only.pick_valid_u8 = 1_u8;
    pick_only.mark_kind_u8 = 6_u8;
    assert_eq!(snapshot_hash(&pick_only), snapshot_hash(&snapshot));
    let mut moved = snapshot;
    moved.altitude_m_f64 += 1.0;
    assert_ne!(snapshot_hash(&moved), snapshot_hash(&snapshot));
    for value_f64 in snapshot.drag_mps2_f64 {
        assert!(
            libm::fabs(value_f64) < ZERO_TOL_F64,
            "rails drag must be zero, got {value_f64}"
        );
    }
    let sample = match step_point_ship(&state, SIM_TICK_S, &body, &atmosphere, &vehicle, mu) {
        Ok(sample) => sample,
        Err(error) => panic!("golden step must succeed: {error}"),
    };
    assert!(sample.state.epoch.value().is_finite());
}

/// AC3: inspect-view source exposes read-only copies with units, no write path.
#[test]
fn ac3_inspect_source_contract() {
    assert_contains(INSPECT_VIEW_SRC, "from_snapshot", "inspect_view.rs");
    assert_contains(INSPECT_VIEW_SRC, "&SimSnapshot", "inspect_view.rs");
    assert_contains(
        INSPECT_VIEW_SRC,
        "Never writes sim state",
        "inspect_view.rs",
    );
    assert_contains(INSPECT_VIEW_SRC, "tick_count_u64", "inspect_view.rs");
    assert_contains(INSPECT_VIEW_SRC, "elapsed_s_f64", "inspect_view.rs");
    assert_contains(INSPECT_VIEW_SRC, "master_seed_u64", "inspect_view.rs");
    assert_contains(INSPECT_VIEW_SRC, "warp", "inspect_view.rs");
    assert_contains(INSPECT_VIEW_SRC, "altitude_m_f64", "inspect_view.rs");
    assert_contains(INSPECT_VIEW_SRC, "pressure_pa_f64", "inspect_view.rs");
    assert_contains(INSPECT_VIEW_SRC, "temperature_k_f64", "inspect_view.rs");
    assert_contains(INSPECT_VIEW_SRC, "density_kg_m3_f64", "inspect_view.rs");
    assert_contains(INSPECT_VIEW_SRC, "snapshot_hash_u64", "inspect_view.rs");
    assert_contains(INSPECT_VIEW_SRC, "regime", "inspect_view.rs");
    assert_contains(INSPECT_VIEW_SRC, "frame_level_u8", "inspect_view.rs");
    assert_contains(INSPECT_VIEW_SRC, "mark_label", "inspect_view.rs");
    assert_lacks(INSPECT_VIEW_SRC, "&mut SimSnapshot", "inspect_view.rs");
    assert_lacks(INSPECT_VIEW_SRC, "&mut snapshot", "inspect_view.rs");
    assert_lacks(INSPECT_VIEW_SRC, "unsafe", "inspect_view.rs");
}

/// AC4: warp context policy gates orbit cruise versus entry, surface, and foot.
#[test]
fn ac4_warp_context_policy_live() {
    let cruise = warp_context_for_regime(Regime::Orbit, true, false, false);
    assert_eq!(cruise, WarpContext::cruise());
    assert!(request_warp(Warp::X10000, cruise).is_ok());
    let entry = warp_context_for_regime(Regime::Atmosphere, true, false, false);
    assert!(entry.should_auto_drop());
    assert_eq!(apply_auto_drop(Warp::X10000, entry), Warp::X1);
    assert!(request_warp(Warp::X100, entry).is_err());
    let grounded = warp_context_for_regime(Regime::Surface, true, false, false);
    assert!(request_warp(Warp::X100, grounded).is_err());
    let on_foot = warp_context_for_regime(Regime::Orbit, false, false, false);
    assert!(request_warp(Warp::X10, on_foot).is_err());
    let approach = warp_context_for_regime(Regime::Orbit, true, true, false);
    assert_eq!(apply_auto_drop(Warp::X1000, approach), Warp::X1);
    let alarm = warp_context_for_regime(Regime::Orbit, true, false, true);
    assert_eq!(apply_auto_drop(Warp::X1000, alarm), Warp::X1);
    let orbit_regime = match classify(Meters::new(200_000.0)) {
        Ok(regime) => regime,
        Err(error) => panic!("orbit altitude must classify: {error}"),
    };
    assert_eq!(orbit_regime, Regime::Orbit);
    let rails_gap = distance(Meters::new(150_000.0), Boundary::Rails);
    assert!(rails_gap.value() > 0.0);
}

/// AC4: router source owns F3, Escape, gestures, and the wants rule.
#[test]
fn ac4_router_source_contract() {
    assert_contains(INPUT_SRC, "Passthrough", "input.rs");
    assert_contains(INPUT_SRC, "Focused", "input.rs");
    assert_contains(INPUT_SRC, "F3", "input.rs");
    assert_contains(INPUT_SRC, "Escape", "input.rs");
    assert_contains(INPUT_SRC, "on_dev_tag_long_press", "input.rs");
    assert_contains(INPUT_SRC, "on_multi_finger_tap", "input.rs");
    assert_contains(INPUT_SRC, "wants_pointer_input", "input.rs");
    assert_contains(INPUT_SRC, "wants_keyboard_input", "input.rs");
    assert_contains(INPUT_SRC, "modal", "input.rs");
    assert_contains(INPUT_SRC, "three-finger", "input.rs");
    assert_contains(INPUT_SRC, "long-press", "input.rs");
    assert_contains(SHELL_SRC, "Passthrough", "shell.rs");
    assert_contains(SHELL_SRC, "TickerOnly", "shell.rs");
    assert_contains(SHELL_SRC, "handle_key", "shell.rs");
    assert_contains(SHELL_SRC, "handle_long_press", "shell.rs");
    assert_contains(SHELL_SRC, "handle_tap", "shell.rs");
    assert_contains(SHELL_SRC, "route", "shell.rs");
    assert_contains(SHELL_SRC, "never writes sim state", "shell.rs");
    assert_lacks(INPUT_SRC, "unsafe", "input.rs");
    assert_lacks(SHELL_SRC, "unsafe", "shell.rs");
}

/// AC5: shell-cost source measures separately with fraction plus close removal.
#[test]
fn ac5_shell_cost_source_contract() {
    assert_contains(
        SHELL_COST_SRC,
        "SHELL_SAMPLE_CAPACITY_ENTRIES_USIZE",
        "shell_cost.rs",
    );
    assert_contains(SHELL_COST_SRC, "256", "shell_cost.rs");
    assert_contains(SHELL_COST_SRC, "record_sample", "shell_cost.rs");
    assert_contains(SHELL_COST_SRC, "fraction_of_budget", "shell_cost.rs");
    assert_contains(SHELL_COST_SRC, "budget_status", "shell_cost.rs");
    assert_contains(SHELL_COST_SRC, "request_close", "shell_cost.rs");
    assert_contains(SHELL_COST_SRC, "reopen", "shell_cost.rs");
    assert_contains(SHELL_COST_SRC, "is_closed", "shell_cost.rs");
    assert_contains(SHELL_COST_SRC, "close shell", "shell_cost.rs");
    assert_contains(SHELL_COST_SRC, "FRAME_BUDGET_MS", "shell_cost.rs");
    assert_contains(SHELL_COST_SRC, "pre-sized", "shell_cost.rs");
    assert_contains(SHELL_COST_SRC, "never writes sim state", "shell_cost.rs");
    assert_contains(SHELL_SRC, "record_draw_cost", "shell.rs");
    assert_contains(SHELL_SRC, "request_close", "shell.rs");
    assert_contains(SHELL_SRC, "reopen", "shell.rs");
    assert_contains(DEBUG_CARGO_SRC, "required-features", "debug Cargo.toml");
    assert_contains(DEBUG_CARGO_SRC, "dev-shell", "debug Cargo.toml");
    assert_contains(WORKSPACE_CARGO_SRC, "dev-shell", "workspace Cargo.toml");
    assert_contains(DEBUG_MAIN_SRC, "mimalloc", "debug main.rs");
    assert_lacks(SHELL_COST_SRC, "unsafe", "shell_cost.rs");
}

/// AC5: smoke shell cost sits in the nominal band with numeric fraction.
#[test]
fn ac5_shell_fraction_math() {
    let fraction_f64 = SMOKE_DRAW_MS_F64 / FRAME_BUDGET_MS_F64;
    assert!(fraction_f64 < NOMINAL_MAX_FRACTION_F64);
    assert!(fraction_f64 < ELEVATED_MAX_FRACTION_F64);
    assert!(fraction_f64 > 0.0);
    assert_contains(THEME_SRC, "BUDGET_NOMINAL_MAX_FRACTION_F64", "theme.rs");
    assert_contains(THEME_SRC, "BUDGET_ELEVATED_MAX_FRACTION_F64", "theme.rs");
    assert_contains(THEME_SRC, "0.5", "theme.rs");
    assert_contains(THEME_SRC, "0.8", "theme.rs");
    assert_contains(THEME_SRC, "fraction", "theme.rs");
    assert_lacks(THEME_SRC, "unsafe", "theme.rs");
}

/// AC6: golden digest pins 100 snapshot ticks across platforms.
#[cfg(feature = "dev-shell")]
#[test]
fn ac6_golden_hash_still_passes() {
    let hash_u64 = golden_hash_after_ticks(GOLDEN_TICK_COUNT_U64, GOLDEN_SEED_U64);
    assert_eq!(hash_u64, GOLDEN_100_TICK_HASH_U64);
    let perturbed_u64 = golden_hash_after_ticks(GOLDEN_TICK_COUNT_U64, GOLDEN_SEED_U64 ^ 1);
    assert_ne!(perturbed_u64, GOLDEN_100_TICK_HASH_U64);
    let short_u64 = golden_hash_after_ticks(GOLDEN_TICK_COUNT_U64 - 1, GOLDEN_SEED_U64);
    assert_ne!(short_u64, GOLDEN_100_TICK_HASH_U64);
}

/// Run the headless M1 cruise profile, byte-identical to `tests/smoke.rs`.
#[cfg(feature = "dev-shell")]
fn golden_hash_after_ticks(tick_count_u64: u64, seed_u64: u64) -> u64 {
    use engine::atmosphere::AtmosphereParams;
    use engine::body::BodyParams;
    use engine::inspect::{capture_snapshot, snapshot_hash};
    use engine::orbit::Mu;
    use engine::trajectory::{StateVector, VehicleParams, step_point_ship};
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

/// AC6: snapshot stays a 304-byte plain-data view behind `dev-shell`.
#[cfg(feature = "dev-shell")]
#[test]
fn ac6_snapshot_pod_size() {
    use engine::inspect::{SNAPSHOT_SIZE_BYTES, SimSnapshot};

    assert_eq!(SNAPSHOT_SIZE_BYTES, 304);
    assert_eq!(core::mem::size_of::<SimSnapshot>(), 304);
    assert_eq!(core::mem::size_of::<SimSnapshot>(), SNAPSHOT_SIZE_BYTES);
}

/// AC6 plus AC8: engine sim boundary keeps no shell or GPU types.
#[test]
fn ac6_no_egui_in_engine_sim() {
    assert_lacks(ENGINE_SIM_SRC, "egui", "engine sim.rs");
    assert_lacks(ENGINE_SIM_SRC, "winit", "engine sim.rs");
    assert_lacks(ENGINE_SIM_SRC, "wgpu", "engine sim.rs");
    assert_lacks(ENGINE_SIM_SRC, "hecs", "engine sim.rs");
    assert_lacks(ENGINE_SIM_SRC, "unsafe", "engine sim.rs");
    assert_lacks(ENGINE_CARGO_SRC, "egui", "engine Cargo.toml");
    assert_lacks(ENGINE_CARGO_SRC, "winit", "engine Cargo.toml");
    assert_lacks(ENGINE_CARGO_SRC, "wgpu", "engine Cargo.toml");
    assert_lacks(ENGINE_CARGO_SRC, "hecs", "engine Cargo.toml");
    assert_contains(DEBUG_MAIN_SRC, "#![forbid(unsafe_code)]", "debug main.rs");
    assert_contains(DEBUG_CARGO_SRC, "[lints]", "debug Cargo.toml");
    assert_contains(DEBUG_CARGO_SRC, "workspace = true", "debug Cargo.toml");
}
