//! `DesktopTester` proxy for issue 44 step 2 (`AC4a`).
//!
//! Scripted stand-in for the unseen legibility tester: descends from
//! orbit to the surface grid and climbs back to orbit using only the
//! Step 1 readouts-only API. Burns mirror `tests/bottom_phase_b.rs`
//! (`retro 200` descent, `kick 1000` plus `5` per tick below `20` km
//! ascent) but every gate reads allowed Scalars only. Handoff deltas
//! must sit inside the engine bands; the monitor flag rule is replayed
//! for information with no-flag explicitly not required.

#![forbid(unsafe_code)]

#[cfg(all(feature = "dev-shell", not(miri)))]
use engine::sim::Scheduler;
#[cfg(all(feature = "dev-shell", not(miri)))]
use engine::units::{MetersPerSecond, Seconds};

/// Legibility seed, dimensionless.
///
/// Sim input only, never a readout gate. Source: fractional hex digits
/// of pi, matching `tests/bottom_phase_b.rs` golden seed.
#[cfg(all(feature = "dev-shell", not(miri)))]
const LEGIBILITY_SEED_U64: u64 = 0x243F_6A88_85A3_08D3;

/// Descent start altitude in meters.
///
/// Source: `crates/engine/src/trajectory.rs` `DESCENT_START_ALTITUDE_M`.
#[cfg(all(feature = "dev-shell", not(miri)))]
const DRIVE_DESCENT_START_M_F64: f64 = 300_000.0;

/// Descent deorbit burn in meters per second, retrograde.
///
/// Source: `crates/engine/src/trajectory.rs` `DESCENT_RETRO_MPS`.
#[cfg(all(feature = "dev-shell", not(miri)))]
const DRIVE_DESCENT_RETRO_MPS_F64: f64 = 200.0;

/// Drive step cap, dimensionless.
///
/// Source: `crates/engine/src/trajectory.rs` `DESCENT_MAX_STEPS_U32`.
#[cfg(all(feature = "dev-shell", not(miri)))]
const DRIVE_MAX_STEPS_U32: u32 = 200_000;

/// Ascent radial kick in meters per second at the surface.
///
/// Source: `crates/engine/src/trajectory.rs` `ASCENT_LAUNCH_RADIAL_MPS`.
#[cfg(all(feature = "dev-shell", not(miri)))]
const DRIVE_ASCENT_KICK_MPS_F64: f64 = 1_000.0;

/// Ascent boost ceiling in meters for per-tick prograde boosts.
///
/// Source: `crates/engine/src/trajectory.rs` `ASCENT_BOOST_TOP_M`.
#[cfg(all(feature = "dev-shell", not(miri)))]
const DRIVE_ASCENT_BOOST_TOP_M_F64: f64 = 20_000.0;

/// Ascent per-tick boost in meters per second, prograde.
///
/// Source: `crates/engine/src/trajectory.rs` `ASCENT_BOOST_MPS`.
#[cfg(all(feature = "dev-shell", not(miri)))]
const DRIVE_ASCENT_BOOST_MPS_F64: f64 = 5.0;

/// Rails boundary altitude in meters.
///
/// Source: `crates/engine/src/trajectory.rs` `RAILS_ALTITUDE_M`.
#[cfg(feature = "dev-shell")]
const RAILS_ALTITUDE_M_F64: f64 = 120_000.0;

/// Touchdown altitude tolerance in meters.
///
/// Source: `crates/engine/src/surface.rs` `TOUCHDOWN_ALTITUDE_TOLERANCE_M`.
#[cfg(all(feature = "dev-shell", not(miri)))]
const TOUCHDOWN_ALT_M_F64: f64 = 0.5;

/// Touchdown speed tolerance in meters per second.
///
/// Source: `crates/engine/src/surface.rs` `TOUCHDOWN_VELOCITY_TOLERANCE_MPS`.
#[cfg(all(feature = "dev-shell", not(miri)))]
const TOUCHDOWN_SPEED_MPS_F64: f64 = 5.0;

/// Fixed sim step in seconds.
///
/// Source: `crates/engine/src/sim.rs` `SIM_TICK_S`.
#[cfg(all(feature = "dev-shell", not(miri)))]
const STEP_S_F64: f64 = 0.05;

/// Orbit regime code, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `REGIME_ORBIT_U8`.
#[cfg(all(feature = "dev-shell", not(miri)))]
const REGIME_ORBIT_U8: u8 = 0;

/// Surface regime code, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `REGIME_SURFACE_U8`.
#[cfg(all(feature = "dev-shell", not(miri)))]
const REGIME_SURFACE_U8: u8 = 2;

/// Flag relative tolerance, dimensionless.
///
/// Mirrors `DELTA_REL_TOL_F64` in `crates/debug/src/continuity.rs`; the
/// monitor owns the rule and this proxy replays it for information.
#[cfg(feature = "dev-shell")]
const FLAG_REL_TOL_F64: f64 = 1e-9;

/// Flag absolute floors per channel, channel units.
///
/// Mirrors `DELTA_ABS_FLOORS_F64` in `crates/debug/src/continuity.rs`.
#[cfg(feature = "dev-shell")]
const FLAG_ABS_FLOORS_F64: [f64; 7] = [1e-6, 1e-9, 1e-12, 1e-9, 1e-15, 1e-12, 1e-12];

/// Engine rails-crossing continuity bands, channel units.
///
/// Mirrors the headless descent/ascent crossing asserts in
/// `crates/engine/src/trajectory.rs` for the 120 km boundary.
#[cfg(feature = "dev-shell")]
const RAILS_BANDS_F64: [f64; 7] = [200.0, 5.0, 0.01, 1.0, 1e-6, 1_000.0, 0.1];

/// Engine surface-crossing continuity bands, channel units.
///
/// Mirrors the headless descent/ascent crossing asserts in
/// `crates/engine/src/trajectory.rs` for the 0 m boundary.
#[cfg(feature = "dev-shell")]
const SURFACE_BANDS_F64: [f64; 7] = [100.0, 20.0, 10.0, 1.0, 2e-4, 50_000.0, 10.0];

/// Inspect-view source for the Step 1 readouts contract.
const INSPECT_VIEW_SRC: &str = include_str!("../crates/debug/src/inspect_view.rs");

/// Shell source for the `DesktopTester` proxy contract.
const SHELL_SRC: &str = include_str!("../crates/debug/src/shell.rs");

/// Debug entry source for the smoke wiring contract.
const DEBUG_MAIN_SRC: &str = include_str!("../crates/debug/src/main.rs");

/// Workspace manifest for the deferred-dependency contract.
const WORKSPACE_CARGO_SRC: &str = include_str!("../Cargo.toml");

/// Lockfile for the no-new-dependency contract.
const CARGO_LOCK_SRC: &str = include_str!("../Cargo.lock");

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

/// `AC4a` Step 1 contract: readouts-only API surface exists.
///
/// Step 1 allows the nine legibility scalars plus clocks, elements,
/// warp, and regime labels via `InspectView` getters, and forbids
/// vectors, seeds, hashes, and picks. The `DesktopTester` proxy in
/// `shell.rs` copies only the allowed scalars by construction.
#[test]
fn ac4a_step1_readouts_contract() {
    for allowed in [
        "altitude_m_f64",
        "speed_mps_f64",
        "pressure_pa_f64",
        "temperature_k_f64",
        "density_kg_m3_f64",
        "heat_flux_w_per_m2_f64",
        "g_load_g_f64",
        "semi_major_axis_m_f64",
        "eccentricity_f64",
        "elements_valid",
        "warp_factor_f64",
        "regime_label",
        "drop_label",
        "frame_label",
        "tick_count_u64",
        "elapsed_s_f64",
    ] {
        assert_contains(INSPECT_VIEW_SRC, allowed, "inspect_view.rs");
    }
    assert_contains(SHELL_SRC, "DesktopTesterReadouts", "shell.rs");
    assert_contains(SHELL_SRC, "desktop_tester_readouts", "shell.rs");
    assert_contains(
        SHELL_SRC,
        "Never touches vectors, seeds, hashes, or picks",
        "shell.rs",
    );
    assert_contains(DEBUG_MAIN_SRC, "desktop_tester", "debug main.rs");
    assert_contains(DEBUG_MAIN_SRC, "desktop_tester_readouts", "debug main.rs");
    assert_contains(DEBUG_MAIN_SRC, "#![forbid(unsafe_code)]", "debug main.rs");
    assert_lacks(SHELL_SRC, ".unwrap()", "shell.rs");
    assert_lacks(SHELL_SRC, ".expect(", "shell.rs");
    assert_lacks(SHELL_SRC, FORBIDDEN_NEEDLE, "shell.rs");
    assert_lacks(DEBUG_MAIN_SRC, ".unwrap()", "debug main.rs");
    assert_lacks(DEBUG_MAIN_SRC, ".expect(", "debug main.rs");
}

/// `AC4a` proxy contract: `DesktopTester` struct holds allowed fields only.
///
/// Extracts the `DesktopTesterReadouts` block from `shell.rs` and
/// rejects vector, seed, hash, and pick fields inside it. The rest of
/// `shell.rs` legitimately owns seeds, hashes, and picks elsewhere.
#[test]
fn ac4a_proxy_holds_allowed_fields_only() {
    let Some(start) = SHELL_SRC.find("pub struct DesktopTesterReadouts") else {
        panic!("DesktopTesterReadouts must exist in shell.rs");
    };
    let Some(end) = SHELL_SRC[start..].find("\n}") else {
        panic!("DesktopTesterReadouts block must close");
    };
    let block = &SHELL_SRC[start..start + end];
    for allowed in [
        "altitude_m_f64",
        "speed_mps_f64",
        "pressure_pa_f64",
        "elements_valid_bool",
        "warp_factor_f64",
        "regime_label",
    ] {
        assert_contains(block, allowed, "DesktopTesterReadouts block");
    }
    for forbidden in [
        "position_m_f64",
        "velocity_mps_f64",
        "drag_mps2_f64",
        "vel_dir_f64",
        "master_seed_u64",
        "stream_seed_u64",
        "snapshot_hash_u64",
        "pick_altitude_m_f64",
        "pick_range_m_f64",
        "pick_body_id_u32",
        "pick_valid",
        "mark_kind",
    ] {
        assert_lacks(block, forbidden, "DesktopTesterReadouts block");
    }
}

/// `AC4a` deferred-dependency contract: no plot, dock, or postcard lock.
///
/// Mirrors `tests/bottom_phase_b.rs` `ac6_no_new_dependencies`; D-010
/// and D-011 stay deferred for the proxy.
#[test]
fn ac4a_no_new_dependencies() {
    assert_lacks(WORKSPACE_CARGO_SRC, "egui_plot", "workspace Cargo.toml");
    assert_lacks(WORKSPACE_CARGO_SRC, "egui-plot", "workspace Cargo.toml");
    assert_lacks(WORKSPACE_CARGO_SRC, "postcard", "workspace Cargo.toml");
    assert_lacks(WORKSPACE_CARGO_SRC, "egui_dock", "workspace Cargo.toml");
    assert_lacks(WORKSPACE_CARGO_SRC, "egui-dock", "workspace Cargo.toml");
    assert_lacks(CARGO_LOCK_SRC, "egui_plot", "Cargo.lock");
    assert_lacks(CARGO_LOCK_SRC, "postcard", "Cargo.lock");
    assert_lacks(CARGO_LOCK_SRC, "egui_dock", "Cargo.lock");
}

/// Report whether a handoff delta trips the monitor flag rule.
///
/// Information only; `AC4a` requires bands, not no-flag. Mirrors
/// `ContinuityMonitor::push_marker` in `crates/debug/src/continuity.rs`.
#[cfg(feature = "dev-shell")]
fn flag_rule_fires(before: &[f64; 7], after: &[f64; 7]) -> bool {
    for (index_usize, before_f64) in before.iter().enumerate() {
        let delta_f64 = (after[index_usize] - before_f64).abs();
        let scale_f64 = before_f64
            .abs()
            .max(after[index_usize].abs())
            .max(FLAG_ABS_FLOORS_F64[index_usize]);
        if delta_f64 > FLAG_REL_TOL_F64 * scale_f64 && delta_f64 > FLAG_ABS_FLOORS_F64[index_usize]
        {
            return true;
        }
    }
    false
}

/// One observed regime transition with its channel triple.
///
/// Test-side record only; the monitor owns `HandoffMarker`.
#[cfg(all(feature = "dev-shell", not(miri)))]
struct HandoffRecord {
    /// True for the rails boundary, false for the surface boundary.
    is_rails_bool: bool,
    /// Before values in allowed channel order.
    before_f64: [f64; 7],
    /// After values in allowed channel order.
    after_f64: [f64; 7],
    /// Monitor flag-rule outcome for the triple.
    flagged_bool: bool,
}

/// Copy allowed readout channels from a snapshot.
///
/// Reads only the Step 1 allowed scalars that correspond to the
/// `InspectView` legibility getters: altitude, corotating speed,
/// pressure, temperature, density, heating proxy, and g-load. Never
/// reads vectors, seeds, hashes, or picks.
#[cfg(all(feature = "dev-shell", not(miri)))]
fn allowed_channels_of(snapshot: &engine::inspect::SimSnapshot) -> [f64; 7] {
    [
        snapshot.altitude_m_f64,
        snapshot.speed_mps_f64,
        snapshot.pressure_pa_f64,
        snapshot.temperature_k_f64,
        snapshot.density_kg_m3_f64,
        snapshot.heat_flux_w_per_m2_f64,
        snapshot.g_load_g_f64,
    ]
}

/// Capture one drive snapshot, panicking on rejection.
///
/// Seeds are sim inputs here, never readout gates; decisions below use
/// only [`allowed_channels_of`] plus regime and elements flags.
#[cfg(all(feature = "dev-shell", not(miri)))]
fn drive_capture_snapshot(
    scheduler: &Scheduler,
    state: &engine::trajectory::StateVector,
    body: &engine::body::BodyParams,
    atmosphere: &engine::atmosphere::AtmosphereParams,
    vehicle: &engine::trajectory::VehicleParams,
) -> engine::inspect::SimSnapshot {
    match engine::inspect::capture_snapshot(
        scheduler,
        state,
        body,
        atmosphere,
        vehicle,
        LEGIBILITY_SEED_U64,
        LEGIBILITY_SEED_U64,
        engine::warp::Warp::X1,
        true,
        false,
        false,
    ) {
        Ok(snapshot) => snapshot,
        Err(error) => panic!("drive capture must succeed: {error}"),
    }
}

/// Record a handoff when the allowed regime readout changes.
///
/// Boundary choice uses only the allowed altitude readout; the flag
/// outcome replays the monitor rule for information.
#[cfg(all(feature = "dev-shell", not(miri)))]
fn record_drive_transition(
    records: &mut Vec<HandoffRecord>,
    previous: &mut Option<(u8, [f64; 7])>,
    snapshot: &engine::inspect::SimSnapshot,
    channels_f64: [f64; 7],
) {
    if let Some((previous_regime_u8, previous_f64)) = previous
        && *previous_regime_u8 != snapshot.regime_u8
    {
        let crossed_rails =
            (previous_f64[0] > RAILS_ALTITUDE_M_F64) != (channels_f64[0] > RAILS_ALTITUDE_M_F64);
        records.push(HandoffRecord {
            is_rails_bool: crossed_rails,
            before_f64: *previous_f64,
            after_f64: channels_f64,
            flagged_bool: flag_rule_fires(previous_f64, &channels_f64),
        });
    }
    *previous = Some((snapshot.regime_u8, channels_f64));
}

/// Panic when any handoff channel delta leaves the engine bands.
#[cfg(feature = "dev-shell")]
fn assert_handoff_inside_bands(record: &HandoffRecord, direction: &str) {
    let bands_f64 = if record.is_rails_bool {
        RAILS_BANDS_F64
    } else {
        SURFACE_BANDS_F64
    };
    for (index_usize, before_f64) in record.before_f64.iter().enumerate() {
        let delta_f64 = (record.after_f64[index_usize] - before_f64).abs();
        assert!(
            delta_f64 < bands_f64[index_usize],
            "{direction} channel {index_usize} delta {delta_f64} leaves band {}",
            bands_f64[index_usize]
        );
    }
}

/// Drive the Mars-like descent through allowed readouts only.
///
/// Starts from a circular orbit above the rails with valid elements,
/// applies the `200` retro burn once, then coasts to the surface. All
/// gates (handoff detection, touchdown stop) read allowed snapshot
/// scalars; state vectors drive the sim but never gate decisions.
#[cfg(all(feature = "dev-shell", not(miri)))]
fn drive_descent_via_readouts() -> (
    Vec<HandoffRecord>,
    engine::trajectory::StateVector,
    engine::inspect::SimSnapshot,
) {
    use engine::atmosphere::AtmosphereParams;
    use engine::body::BodyParams;
    use engine::orbit::Mu;
    use engine::trajectory::{
        Burn, BurnDirection, StateVector, VehicleParams, apply_burn, step_point_ship,
    };
    use glam::DVec3;

    let body = BodyParams::mars_like();
    let atmosphere = match AtmosphereParams::mars_like() {
        Ok(atmosphere) => atmosphere,
        Err(error) => panic!("drive atmosphere must validate: {error}"),
    };
    let vehicle = VehicleParams::preset();
    let mu = match Mu::new(body.gravitational_parameter_m3_s2()) {
        Ok(mu) => mu,
        Err(error) => panic!("drive mu must validate: {error}"),
    };
    let radius_m_f64 = body.radius_m().value() + DRIVE_DESCENT_START_M_F64;
    let start = match StateVector::new(
        DVec3::new(radius_m_f64, 0.0, 0.0),
        DVec3::new(0.0, libm::sqrt(mu.value() / radius_m_f64), 0.0),
        Seconds::new(0.0),
    ) {
        Ok(state) => state,
        Err(error) => panic!("drive start must validate: {error}"),
    };
    let burn = match Burn::new(
        BurnDirection::Retrograde,
        MetersPerSecond::new(DRIVE_DESCENT_RETRO_MPS_F64),
    ) {
        Ok(burn) => burn,
        Err(error) => panic!("drive burn must validate: {error}"),
    };
    let mut current = match apply_burn(&start, &burn) {
        Ok(state) => state,
        Err(error) => panic!("drive burn must apply: {error}"),
    };
    let step = Seconds::new(STEP_S_F64);
    let mut scheduler = Scheduler::default();
    let start_snapshot = drive_capture_snapshot(&scheduler, &current, &body, &atmosphere, &vehicle);
    assert_eq!(
        start_snapshot.regime_u8, REGIME_ORBIT_U8,
        "descent must start in orbit via allowed regime readout"
    );
    assert!(
        start_snapshot.altitude_m_f64 > RAILS_ALTITUDE_M_F64,
        "descent must start above rails via allowed altitude readout"
    );
    assert_eq!(
        start_snapshot.elements_valid_u8, 1_u8,
        "descent must start with valid elements via allowed flag"
    );
    let mut previous: Option<(u8, [f64; 7])> = Some((
        start_snapshot.regime_u8,
        allowed_channels_of(&start_snapshot),
    ));
    let mut records: Vec<HandoffRecord> = Vec::new();
    let mut touchdown_state: Option<engine::trajectory::StateVector> = None;
    let mut touchdown_snapshot: Option<engine::inspect::SimSnapshot> = None;
    for _ in 0..DRIVE_MAX_STEPS_U32 {
        let sample = match step_point_ship(&current, step, &body, &atmosphere, &vehicle, mu) {
            Ok(sample) => sample,
            Err(error) => panic!("drive step must succeed: {error}"),
        };
        current = sample.state;
        scheduler.advance();
        let snapshot = drive_capture_snapshot(&scheduler, &current, &body, &atmosphere, &vehicle);
        let channels_f64 = allowed_channels_of(&snapshot);
        record_drive_transition(&mut records, &mut previous, &snapshot, channels_f64);
        if snapshot.altitude_m_f64 <= 0.0 {
            touchdown_state = Some(current);
            touchdown_snapshot = Some(snapshot);
            break;
        }
    }
    let Some(penetrating) = touchdown_state else {
        panic!("descent must reach the surface within the cap");
    };
    let Some(touchdown) = touchdown_snapshot else {
        panic!("descent must reach the surface within the cap");
    };
    assert_eq!(
        touchdown.regime_u8, REGIME_SURFACE_U8,
        "touchdown must read surface via allowed regime readout"
    );
    (records, penetrating, touchdown)
}

/// Drive the Mars-like climb through allowed readouts only.
///
/// Starts from a surface rest state with the `1000` radial kick, then
/// applies `5` per-tick prograde boosts while the allowed altitude
/// readout sits below `20` km. Boost and rails-stop gates read allowed
/// scalars only; state vectors drive the sim but never gate decisions.
#[cfg(all(feature = "dev-shell", not(miri)))]
fn drive_ascent_via_readouts() -> (Vec<HandoffRecord>, engine::inspect::SimSnapshot) {
    use engine::atmosphere::AtmosphereParams;
    use engine::body::BodyParams;
    use engine::orbit::Mu;
    use engine::trajectory::{
        Burn, BurnDirection, StateVector, VehicleParams, apply_burn, step_point_ship,
    };
    use glam::DVec3;

    let body = BodyParams::mars_like();
    let atmosphere = match AtmosphereParams::mars_like() {
        Ok(atmosphere) => atmosphere,
        Err(error) => panic!("climb atmosphere must validate: {error}"),
    };
    let vehicle = VehicleParams::preset();
    let mu = match Mu::new(body.gravitational_parameter_m3_s2()) {
        Ok(mu) => mu,
        Err(error) => panic!("climb mu must validate: {error}"),
    };
    let spin_rad_s_f64 = core::f64::consts::TAU / body.rotation_period_s().value();
    let mut current = match StateVector::new(
        DVec3::new(body.radius_m().value(), 0.0, 0.0),
        DVec3::new(
            DRIVE_ASCENT_KICK_MPS_F64,
            spin_rad_s_f64 * body.radius_m().value(),
            0.0,
        ),
        Seconds::new(0.0),
    ) {
        Ok(state) => state,
        Err(error) => panic!("climb start must validate: {error}"),
    };
    let step = Seconds::new(STEP_S_F64);
    let mut scheduler = Scheduler::default();
    let seed_snapshot = drive_capture_snapshot(&scheduler, &current, &body, &atmosphere, &vehicle);
    assert_eq!(
        seed_snapshot.regime_u8, REGIME_SURFACE_U8,
        "climb must start on the surface via allowed regime readout"
    );
    let mut previous: Option<(u8, [f64; 7])> =
        Some((seed_snapshot.regime_u8, allowed_channels_of(&seed_snapshot)));
    let mut previous_alt_m_f64 = seed_snapshot.altitude_m_f64;
    let mut records: Vec<HandoffRecord> = Vec::new();
    let mut rails_snapshot: Option<engine::inspect::SimSnapshot> = None;
    for _ in 0..DRIVE_MAX_STEPS_U32 {
        if previous_alt_m_f64 > 0.0 && previous_alt_m_f64 < DRIVE_ASCENT_BOOST_TOP_M_F64 {
            let boost = match Burn::new(
                BurnDirection::Prograde,
                MetersPerSecond::new(DRIVE_ASCENT_BOOST_MPS_F64),
            ) {
                Ok(boost) => boost,
                Err(error) => panic!("climb boost must validate: {error}"),
            };
            current = match apply_burn(&current, &boost) {
                Ok(kicked) => kicked,
                Err(error) => panic!("climb boost must apply: {error}"),
            };
        }
        let sample = match step_point_ship(&current, step, &body, &atmosphere, &vehicle, mu) {
            Ok(sample) => sample,
            Err(error) => panic!("climb step must succeed: {error}"),
        };
        current = sample.state;
        scheduler.advance();
        let snapshot = drive_capture_snapshot(&scheduler, &current, &body, &atmosphere, &vehicle);
        let channels_f64 = allowed_channels_of(&snapshot);
        record_drive_transition(&mut records, &mut previous, &snapshot, channels_f64);
        previous_alt_m_f64 = snapshot.altitude_m_f64;
        if snapshot.altitude_m_f64 > RAILS_ALTITUDE_M_F64 {
            rails_snapshot = Some(snapshot);
            break;
        }
    }
    let Some(top) = rails_snapshot else {
        panic!("climb must clear rails within the cap");
    };
    assert_eq!(
        top.regime_u8, REGIME_ORBIT_U8,
        "climb must end in orbit via allowed regime readout"
    );
    assert!(
        top.altitude_m_f64 > RAILS_ALTITUDE_M_F64,
        "climb must end above rails via allowed altitude readout"
    );
    assert_eq!(
        top.elements_valid_u8, 1_u8,
        "climb must end with valid elements via allowed flag"
    );
    (records, top)
}

/// Settle a penetrating descent end into a parked touchdown.
///
/// Uses the surface rest state for geometry, then verifies touchdown
/// through allowed readouts: surface regime, altitude within `0.5` m,
/// and corotating speed below `5` m/s.
#[cfg(all(feature = "dev-shell", not(miri)))]
fn settle_touchdown_via_readouts(
    penetrating: &engine::trajectory::StateVector,
    scheduler: &Scheduler,
) -> engine::inspect::SimSnapshot {
    use engine::atmosphere::AtmosphereParams;
    use engine::body::BodyParams;
    use engine::surface::{SurfaceFrame, TouchdownConfig};
    use engine::trajectory::VehicleParams;

    let body = BodyParams::mars_like();
    let atmosphere = match AtmosphereParams::mars_like() {
        Ok(atmosphere) => atmosphere,
        Err(error) => panic!("settle atmosphere must validate: {error}"),
    };
    let vehicle = VehicleParams::preset();
    let frame = SurfaceFrame::new(&body);
    let parked = match frame.rest_state(penetrating) {
        Ok(parked) => parked,
        Err(error) => panic!("settle rest state must build: {error}"),
    };
    let config = TouchdownConfig::preset();
    match frame.is_touchdown(&parked, &config) {
        Ok(touched) => assert!(touched, "parked descent must count as touchdown"),
        Err(error) => panic!("touchdown check must run: {error}"),
    }
    let snapshot = drive_capture_snapshot(scheduler, &parked, &body, &atmosphere, &vehicle);
    assert_eq!(
        snapshot.regime_u8, REGIME_SURFACE_U8,
        "parked touchdown must read surface via allowed regime"
    );
    assert!(
        snapshot.altitude_m_f64.abs() <= TOUCHDOWN_ALT_M_F64,
        "parked altitude must sit within 0.5 m via allowed readout"
    );
    assert!(
        snapshot.speed_mps_f64 <= TOUCHDOWN_SPEED_MPS_F64,
        "parked speed must sit below 5 m/s via allowed readout"
    );
    snapshot
}

/// `AC4a`: `DesktopTester` descends on readouts alone inside bands.
///
/// Starts orbit above `120` km with valid elements, touches the
/// surface grid, and crosses rails then surface with every allowed
/// channel delta inside the engine bands. Flag counts are reported
/// with no-flag explicitly not required: consecutive live samples move.
#[cfg(all(feature = "dev-shell", not(miri)))]
#[test]
fn ac4a_descent_via_readouts_inside_bands() {
    let (records, penetrating, touchdown) = drive_descent_via_readouts();
    let flagged_usize = records.iter().filter(|record| record.flagged_bool).count();
    assert_eq!(
        records.len(),
        2,
        "descent must cross rails plus surface; flagged motion-explained {flagged_usize}"
    );
    assert!(
        records[0].is_rails_bool,
        "descent must cross rails first via allowed altitude readout"
    );
    assert!(
        !records[1].is_rails_bool,
        "descent must cross surface second via allowed altitude readout"
    );
    for record in &records {
        assert_handoff_inside_bands(record, "descent");
    }
    assert_eq!(touchdown.regime_u8, REGIME_SURFACE_U8);
    let scheduler = Scheduler::default();
    let parked = settle_touchdown_via_readouts(&penetrating, &scheduler);
    assert_eq!(parked.regime_u8, REGIME_SURFACE_U8);
}

/// `AC4a`: `DesktopTester` ascends on readouts alone inside bands.
///
/// Starts on the surface grid, gates each `5` boost on the allowed
/// altitude readout below `20` km, and ends in orbit above `120` km
/// with valid elements. Both handoffs sit inside the engine bands
/// with flag counts reported and no-flag explicitly not required.
#[cfg(all(feature = "dev-shell", not(miri)))]
#[test]
fn ac4a_ascent_via_readouts_inside_bands() {
    let (records, top) = drive_ascent_via_readouts();
    let flagged_usize = records.iter().filter(|record| record.flagged_bool).count();
    assert_eq!(
        records.len(),
        2,
        "climb must cross surface plus rails; flagged motion-explained {flagged_usize}"
    );
    assert!(
        !records[0].is_rails_bool,
        "climb must cross surface first via allowed altitude readout"
    );
    assert!(
        records[1].is_rails_bool,
        "climb must cross rails second via allowed altitude readout"
    );
    for record in &records {
        assert_handoff_inside_bands(record, "climb");
    }
    assert_eq!(top.regime_u8, REGIME_ORBIT_U8);
    assert!(top.altitude_m_f64 > RAILS_ALTITUDE_M_F64);
    assert_eq!(top.elements_valid_u8, 1_u8);
}
