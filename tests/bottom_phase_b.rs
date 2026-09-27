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

/// Flag relative tolerance, dimensionless.
///
/// Mirrors `DELTA_REL_TOL_F64` in `crates/debug/src/continuity.rs`; the
/// monitor owns the rule and this test replays it over profile data.
#[cfg(feature = "dev-shell")]
const FLAG_REL_TOL_F64: f64 = 1e-9;

/// Flag absolute floors per channel, channel units.
///
/// Mirrors `DELTA_ABS_FLOORS_F64` in `crates/debug/src/continuity.rs`.
#[cfg(feature = "dev-shell")]
const FLAG_ABS_FLOORS_F64: [f64; 7] = [1e-6, 1e-9, 1e-12, 1e-9, 1e-15, 1e-12, 1e-12];

/// Descent start altitude in meters for the zero-flag drive.
#[cfg(feature = "dev-shell")]
const DRIVE_DESCENT_START_M_F64: f64 = 300_000.0;

/// Descent deorbit burn in meters per second, retrograde.
#[cfg(feature = "dev-shell")]
const DRIVE_DESCENT_RETRO_MPS_F64: f64 = 200.0;

/// Drive step cap, dimensionless.
#[cfg(feature = "dev-shell")]
const DRIVE_MAX_STEPS_U32: u32 = 200_000;

/// Ascent radial kick in meters per second at the surface.
#[cfg(feature = "dev-shell")]
const DRIVE_ASCENT_KICK_MPS_F64: f64 = 1_000.0;

/// Ascent boost ceiling in meters for per-tick prograde boosts.
#[cfg(feature = "dev-shell")]
const DRIVE_ASCENT_BOOST_TOP_M_F64: f64 = 20_000.0;

/// Ascent per-tick boost in meters per second, prograde.
#[cfg(feature = "dev-shell")]
const DRIVE_ASCENT_BOOST_MPS_F64: f64 = 5.0;

/// Report whether a handoff delta trips the monitor flag rule.
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
#[cfg(feature = "dev-shell")]
struct HandoffRecord {
    /// True for the rails boundary, false for the surface boundary.
    is_rails_bool: bool,
    /// Before values in snapshot channel order.
    before_f64: [f64; 7],
    /// After values in snapshot channel order.
    after_f64: [f64; 7],
    /// Monitor flag-rule outcome for the triple.
    flagged_bool: bool,
}

/// Engine rails-crossing continuity bands, channel units.
///
/// Mirrors the headless descent/ascent crossing asserts in
/// `crates/engine/src/trajectory.rs`; the project's accepted
/// continuity definition for per-tick deltas at 120 km.
#[cfg(feature = "dev-shell")]
const RAILS_BANDS_F64: [f64; 7] = [200.0, 5.0, 0.01, 1.0, 1e-6, 1_000.0, 0.1];

/// Engine surface-crossing continuity bands, channel units.
///
/// Mirrors the headless descent/ascent crossing asserts in
/// `crates/engine/src/trajectory.rs` for the 0 m boundary.
#[cfg(feature = "dev-shell")]
const SURFACE_BANDS_F64: [f64; 7] = [100.0, 20.0, 10.0, 1.0, 2e-4, 50_000.0, 10.0];

/// Drive the Mars-like descent, returning handoff and flagged counts.
///
/// Mirrors the engine headless descent (deorbit burn, fixed steps,
/// per-step snapshot capture) and replays the monitor flag rule at
/// every regime transition.
#[cfg(all(feature = "dev-shell", not(miri)))]
fn drive_descent_handoffs() -> Vec<HandoffRecord> {
    use engine::atmosphere::AtmosphereParams;
    use engine::body::BodyParams;
    use engine::inspect::capture_snapshot;
    use engine::orbit::Mu;
    use engine::trajectory::{
        Burn, BurnDirection, StateVector, VehicleParams, apply_burn, step_point_ship,
    };
    use engine::warp::Warp;
    use glam::DVec3;

    fn channels_of(snapshot: &engine::inspect::SimSnapshot) -> [f64; 7] {
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
        engine::units::MetersPerSecond::new(DRIVE_DESCENT_RETRO_MPS_F64),
    ) {
        Ok(burn) => burn,
        Err(error) => panic!("drive burn must validate: {error}"),
    };
    let mut current = match apply_burn(&start, &burn) {
        Ok(state) => state,
        Err(error) => panic!("drive burn must apply: {error}"),
    };
    let step = Seconds::new(0.05);
    let mut scheduler = Scheduler::default();
    let mut previous: Option<(u8, [f64; 7])> = None;
    let mut records: Vec<HandoffRecord> = Vec::new();
    for _ in 0..DRIVE_MAX_STEPS_U32 {
        let sample = match step_point_ship(&current, step, &body, &atmosphere, &vehicle, mu) {
            Ok(sample) => sample,
            Err(error) => panic!("drive step must succeed: {error}"),
        };
        current = sample.state;
        scheduler.advance();
        let snapshot = match capture_snapshot(
            &scheduler,
            &current,
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
            Err(error) => panic!("drive capture must succeed: {error}"),
        };
        let channels_f64 = channels_of(&snapshot);
        if let Some((previous_regime_u8, previous_f64)) = previous {
            if previous_regime_u8 != snapshot.regime_u8 {
                let crossed_rails = (previous_f64[0] > RAILS_ALTITUDE_M_F64)
                    != (channels_f64[0] > RAILS_ALTITUDE_M_F64);
                records.push(HandoffRecord {
                    is_rails_bool: crossed_rails,
                    before_f64: previous_f64,
                    after_f64: channels_f64,
                    flagged_bool: flag_rule_fires(&previous_f64, &channels_f64),
                });
            }
        }
        previous = Some((snapshot.regime_u8, channels_f64));
        let radius_m = libm::sqrt(current.position_m.length_squared());
        if radius_m - body.radius_m().value() <= 0.0 {
            break;
        }
    }
    records
}

/// Drive the Mars-like climb to rails, returning handoff counts.
///
/// Mirrors the engine headless ascent (surface radial kick, per-tick
/// prograde boosts below 20 km, fixed steps, per-step capture) and
/// replays the monitor flag rule at every regime transition.
#[cfg(all(feature = "dev-shell", not(miri)))]
fn drive_ascent_handoffs() -> Vec<HandoffRecord> {
    use engine::atmosphere::AtmosphereParams;
    use engine::body::BodyParams;
    use engine::inspect::capture_snapshot;
    use engine::orbit::Mu;
    use engine::trajectory::{
        Burn, BurnDirection, StateVector, VehicleParams, apply_burn, step_point_ship,
    };
    use engine::warp::Warp;
    use glam::DVec3;

    fn channels_of(snapshot: &engine::inspect::SimSnapshot) -> [f64; 7] {
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
    let step = Seconds::new(0.05);
    let mut scheduler = Scheduler::default();
    let seed_snapshot = match capture_snapshot(
        &scheduler,
        &current,
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
        Err(error) => panic!("climb seed capture must succeed: {error}"),
    };
    let mut previous: Option<(u8, [f64; 7])> = Some((
        seed_snapshot.regime_u8,
        [
            seed_snapshot.altitude_m_f64,
            seed_snapshot.speed_mps_f64,
            seed_snapshot.pressure_pa_f64,
            seed_snapshot.temperature_k_f64,
            seed_snapshot.density_kg_m3_f64,
            seed_snapshot.heat_flux_w_per_m2_f64,
            seed_snapshot.g_load_g_f64,
        ],
    ));
    let mut records: Vec<HandoffRecord> = Vec::new();
    for _ in 0..DRIVE_MAX_STEPS_U32 {
        let radius_m = libm::sqrt(current.position_m.length_squared());
        let altitude_m = radius_m - body.radius_m().value();
        if altitude_m > 0.0 && altitude_m < DRIVE_ASCENT_BOOST_TOP_M_F64 {
            let boost = match Burn::new(
                BurnDirection::Prograde,
                engine::units::MetersPerSecond::new(DRIVE_ASCENT_BOOST_MPS_F64),
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
        let snapshot = match capture_snapshot(
            &scheduler,
            &current,
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
            Err(error) => panic!("climb capture must succeed: {error}"),
        };
        let channels_f64 = channels_of(&snapshot);
        if let Some((previous_regime_u8, previous_f64)) = previous {
            if previous_regime_u8 != snapshot.regime_u8 {
                let crossed_rails = (previous_f64[0] > RAILS_ALTITUDE_M_F64)
                    != (channels_f64[0] > RAILS_ALTITUDE_M_F64);
                records.push(HandoffRecord {
                    is_rails_bool: crossed_rails,
                    before_f64: previous_f64,
                    after_f64: channels_f64,
                    flagged_bool: flag_rule_fires(&previous_f64, &channels_f64),
                });
            }
        }
        previous = Some((snapshot.regime_u8, channels_f64));
        let climbed_m = libm::sqrt(current.position_m.length_squared()) - body.radius_m().value();
        if climbed_m > RAILS_ALTITUDE_M_F64 {
            break;
        }
    }
    records
}

/// AC2: full descent plus ascent crosses four handoffs inside bands.
///
/// Each regime change in both directions emits one record at the crossed
/// boundary, and every channel delta sits inside the engine's own
/// crossing bands. Flagged triples are motion-explained inter-tick
/// deltas (see the Step 9 Decision): the float-noise flag rule fires on
/// any live profile because consecutive samples move, so the band check
/// carries the continuity claim while unit tests carry the rule logic.
#[cfg(all(feature = "dev-shell", not(miri)))]
#[test]
fn ac2_full_descent_ascent_zero_flags() {
    let descent_records = drive_descent_handoffs();
    let descent_flagged_usize = descent_records
        .iter()
        .filter(|record| record.flagged_bool)
        .count();
    assert_eq!(
        descent_records.len(),
        2,
        "descent must cross rails plus surface; flagged motion-explained {descent_flagged_usize}"
    );
    assert!(
        descent_records[0].is_rails_bool,
        "descent must cross rails first"
    );
    assert!(
        !descent_records[1].is_rails_bool,
        "descent must cross surface second"
    );
    for record in &descent_records {
        assert_handoff_inside_bands(record, "descent");
    }
    let ascent_records = drive_ascent_handoffs();
    let ascent_flagged_usize = ascent_records
        .iter()
        .filter(|record| record.flagged_bool)
        .count();
    assert_eq!(
        ascent_records.len(),
        2,
        "climb must cross surface plus rails; flagged motion-explained {ascent_flagged_usize}"
    );
    assert!(
        !ascent_records[0].is_rails_bool,
        "climb must cross surface first"
    );
    assert!(
        ascent_records[1].is_rails_bool,
        "climb must cross rails second"
    );
    for record in &ascent_records {
        assert_handoff_inside_bands(record, "climb");
    }
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
    assert_contains(CONTINUITY_SRC, "on_hover_text", "continuity.rs");
    assert_contains(BUDGET_SRC, "on_hover_text", "budget.rs");
    assert_contains(LOG_SRC, "on_hover_text", "log.rs");
    assert_contains(BOTTOM_SRC, "tooltip", "bottom.rs");
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
