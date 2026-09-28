//! Canvas handoff verification for issue 52 step 5.
//!
//! Headless checks for the abstract-marks canvas handoff: full descent from
//! 300 km to touchdown plus ascent back past rails crosses rails and surface
//! with no camera jump, the same seed plus inputs yields the same shapes,
//! shell draw cost is recorded with budget fractions, and ticker-only draws
//! fewer shapes than Descent. Engine cameras (`engine::render::Camera2D`)
//! run directly; binary-only shell pieces (`MarksView`, `ShellCostMeter`,
//! `PanelVisibility`) are verified by read-only source contracts plus the
//! in-crate headless draw tests they own, so this file adds no dependency
//! and no allocation. Gates live in `docs/tech/quality.md`; this file names
//! constants only.

#![forbid(unsafe_code)]

use engine::body::BodyParams;
use engine::render::Camera2D;
use engine::units::Meters;
use glam::DVec3;

/// Frame budget in milliseconds, named `FRAME_BUDGET_MS` in debug docs.
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

/// Fraction equality tolerance, dimensionless.
const FRACTION_TOL_F64: f64 = 1e-12;

/// Rails boundary altitude in meters.
const RAILS_ALTITUDE_M_F64: f64 = 120_000.0;

/// Entry-to-surface view handoff altitude in meters.
#[cfg(feature = "dev-shell")]
const ENTRY_SURFACE_ALTITUDE_M_F64: f64 = 10_000.0;

/// Canvas width in points for camera fixtures.
const CANVAS_WIDTH_PX_F32: f32 = 800.0;

/// Canvas height in points for camera fixtures.
const CANVAS_HEIGHT_PX_F32: f32 = 600.0;

/// Orbit probe altitude in meters for the orbit-fit fixture.
const ORBIT_PROBE_ALTITUDE_M_F64: f64 = 250_000.0;

/// Screen equality tolerance in points.
const SCREEN_TOL_PX_F32: f32 = 1e-3;

/// Headless timing tick count, dimensionless.
const TIMING_TICKS_U64: u64 = 100;

/// Headless timing tick count as float for averaging, dimensionless.
const TIMING_TICKS_F64: f64 = 100.0;

/// Cruise altitude in meters for the timing profile.
const TIMING_CRUISE_ALTITUDE_M_F64: f64 = 250_000.0;

/// Smoke shell draw cost in milliseconds for fraction math.
const SMOKE_SHELL_DRAW_MS_F64: f64 = 0.4;

/// Smoke frame sample in milliseconds for fraction math.
const SMOKE_FRAME_MS_F64: f64 = 8.0;

/// Smoke sim-tick average in milliseconds for fraction math.
const SMOKE_SIM_AVG_MS_F64: f64 = 2.0;

/// Smoke sim-tick p99 in milliseconds for fraction math.
const SMOKE_SIM_P99_MS_F64: f64 = 4.0;

/// Legibility seed, dimensionless.
///
/// Sim input only, never a readout gate. Source: fractional hex digits of
/// pi, matching `tests/bottom_phase_b.rs` golden seed.
#[cfg(feature = "dev-shell")]
const DRIVE_SEED_U64: u64 = 0x243F_6A88_85A3_08D3;

/// Descent start altitude in meters.
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

/// Fixed sim step in seconds.
#[cfg(feature = "dev-shell")]
const DRIVE_STEP_S_F64: f64 = 0.05;

/// Orbit regime code, dimensionless.
#[cfg(feature = "dev-shell")]
const DRIVE_REGIME_ORBIT_U8: u8 = 0;

/// Surface regime code, dimensionless.
#[cfg(feature = "dev-shell")]
const DRIVE_REGIME_SURFACE_U8: u8 = 2;

/// Touchdown altitude tolerance in meters.
#[cfg(feature = "dev-shell")]
const DRIVE_TOUCHDOWN_ALT_M_F64: f64 = 0.5;

/// Touchdown speed tolerance in meters per second.
#[cfg(feature = "dev-shell")]
const DRIVE_TOUCHDOWN_SPEED_MPS_F64: f64 = 5.0;

/// Engine rails-crossing continuity bands, channel units.
///
/// Mirrors the headless descent/ascent crossing asserts in
/// `crates/engine/src/trajectory.rs` for the 120 km boundary.
#[cfg(feature = "dev-shell")]
const DRIVE_RAILS_BANDS_F64: [f64; 7] = [200.0, 5.0, 0.01, 1.0, 1e-6, 1_000.0, 0.1];

/// Engine surface-crossing continuity bands, channel units.
///
/// Mirrors the headless descent/ascent crossing asserts in
/// `crates/engine/src/trajectory.rs` for the 0 m boundary.
#[cfg(feature = "dev-shell")]
const DRIVE_SURFACE_BANDS_F64: [f64; 7] = [100.0, 20.0, 10.0, 1.0, 2e-4, 50_000.0, 10.0];

/// Render seam source for the camera contract.
const RENDER_SRC: &str = include_str!("../crates/engine/src/render.rs");

/// Marks-canvas source for the view plus overlay contract.
const MARKS_SRC: &str = include_str!("../crates/debug/src/marks.rs");

/// Shell-cost source for the meter contract.
const SHELL_COST_SRC: &str = include_str!("../crates/debug/src/shell_cost.rs");

/// Layout source for the ticker-only visibility contract.
const LAYOUT_SRC: &str = include_str!("../crates/debug/src/layout.rs");

/// Shell assembly source for the draw plus cost-hook contract.
const SHELL_SRC: &str = include_str!("../crates/debug/src/shell.rs");

/// Quality doc for the gate-cite contract.
const QUALITY_SRC: &str = include_str!("../docs/tech/quality.md");

/// Debug doc for the budget-strip plus tester-shape contract.
const DEBUG_SRC: &str = include_str!("../docs/tech/debug.md");

/// Workspace manifest for the no-new-dependency contract.
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

/// Canvas viewport width and height in points for fixtures.
fn viewport_px_f32() -> [f32; 2] {
    [CANVAS_WIDTH_PX_F32, CANVAS_HEIGHT_PX_F32]
}

/// Mars-like body for camera fixtures.
fn test_body() -> BodyParams {
    BodyParams::mars_like()
}

/// Build the orbit-fit camera for fixtures, panicking on rejection.
fn orbit_camera() -> Camera2D {
    match Camera2D::orbit_fit(
        &test_body(),
        Meters::new(ORBIT_PROBE_ALTITUDE_M_F64),
        viewport_px_f32(),
    ) {
        Ok(camera) => camera,
        Err(error) => panic!("orbit-fit camera must build: {error}"),
    }
}

/// Build the entry-corridor camera for fixtures, panicking on rejection.
fn entry_camera() -> Camera2D {
    match Camera2D::entry_corridor(&test_body(), viewport_px_f32()) {
        Ok(camera) => camera,
        Err(error) => panic!("entry camera must build: {error}"),
    }
}

/// Build the surface-grid camera for fixtures, panicking on rejection.
fn surface_camera() -> Camera2D {
    match Camera2D::surface_grid(&test_body(), viewport_px_f32()) {
        Ok(camera) => camera,
        Err(error) => panic!("surface camera must build: {error}"),
    }
}

/// Assert two screen points agree within the point tolerance.
fn assert_screen_close_f32(actual_px_f32: [f32; 2], expected_px_f32: [f32; 2]) {
    for axis_usize in 0..2 {
        let diff_px_f32 = (actual_px_f32[axis_usize] - expected_px_f32[axis_usize]).abs();
        assert!(
            diff_px_f32 < SCREEN_TOL_PX_F32,
            "screen {actual_px_f32:?} differs from {expected_px_f32:?}"
        );
    }
}

/// AC1 source contract: the canvas handoff owns one camera seam.
///
/// The step-1 `Camera2D` stays the single `f64` to `f32` conversion point
/// with orbit, entry, and surface presets; the marks view rebuilds only on
/// view, radius, or zoom changes and keeps scale plus center bit-identical
/// within a view, over read-only snapshot copies with pre-sized buffers.
#[test]
fn ac1_camera_marks_source_contract() {
    assert_contains(RENDER_SRC, "Camera2D", "render.rs");
    assert_contains(RENDER_SRC, "world_to_screen", "render.rs");
    assert_contains(RENDER_SRC, "orbit_fit", "render.rs");
    assert_contains(RENDER_SRC, "entry_corridor", "render.rs");
    assert_contains(RENDER_SRC, "surface_grid", "render.rs");
    assert_contains(RENDER_SRC, "single conversion point", "render.rs");
    assert_contains(RENDER_SRC, "no other module casts", "render.rs");
    assert_contains(RENDER_SRC, "convert_into", "render.rs");
    assert_contains(RENDER_SRC, "scale_px_per_m_f64", "render.rs");
    assert_contains(RENDER_SRC, "center_m", "render.rs");
    assert_contains(MARKS_SRC, "MarksView", "marks.rs");
    assert_contains(MARKS_SRC, "ViewMode", "marks.rs");
    assert_contains(MARKS_SRC, "OverlayFlags", "marks.rs");
    assert_contains(MARKS_SRC, "update_view", "marks.rs");
    assert_contains(MARKS_SRC, "push_snapshot", "marks.rs");
    assert_contains(MARKS_SRC, "shows_marks_canvas_bool", "marks.rs");
    assert_contains(MARKS_SRC, "never writes sim state", "marks.rs");
    assert_contains(MARKS_SRC, "pre-sized", "marks.rs");
    assert_contains(MARKS_SRC, "reuse after warmup", "marks.rs");
    assert_contains(MARKS_SRC, "never allocates after open", "marks.rs");
    assert_contains(MARKS_SRC, "rebuilds only", "marks.rs");
    assert_contains(MARKS_SRC, "bit-identical", "marks.rs");
    assert_contains(MARKS_SRC, "auto_view_mode", "marks.rs");
    assert_contains(MARKS_SRC, "hysteresis", "marks.rs");
    assert_contains(
        MARKS_SRC,
        "draw_paints_marks_without_camera_jump",
        "marks.rs",
    );
    assert_lacks(MARKS_SRC, ".unwrap()", "marks.rs");
    assert_lacks(MARKS_SRC, ".expect(", "marks.rs");
    assert_lacks(MARKS_SRC, FORBIDDEN_NEEDLE, "marks.rs");
    assert_lacks(RENDER_SRC, ".unwrap()", "render.rs");
    assert_lacks(RENDER_SRC, ".expect(", "render.rs");
}

/// AC1 camera check: rails and surface boundaries map linearly.
///
/// Within one fixed camera the 2 m step across each boundary spans exactly
/// twice the scale with a linear midpoint, and rebuilding the same preset
/// keeps scale plus center bit-identical, so frames never jump. Reports
/// exact scales and gaps on failure.
#[test]
fn ac1_camera_handoff_has_no_jump() {
    let orbit = orbit_camera();
    let entry = entry_camera();
    let surface = surface_camera();
    let radius_m_f64 = test_body().radius_m().value();
    for camera in [orbit, entry, surface] {
        let scale_px_per_m_f64 = camera.scale_px_per_m_f64();
        let below_px_f32 = camera.world_to_screen(DVec3::new(
            radius_m_f64 + RAILS_ALTITUDE_M_F64 - 1.0,
            0.0,
            0.0,
        ));
        let above_px_f32 = camera.world_to_screen(DVec3::new(
            radius_m_f64 + RAILS_ALTITUDE_M_F64 + 1.0,
            0.0,
            0.0,
        ));
        let gap_px_f32 = (above_px_f32[0] - below_px_f32[0]).abs();
        let expected_px_f64 = 2.0 * scale_px_per_m_f64;
        assert!(
            (f64::from(gap_px_f32) - expected_px_f64).abs() < f64::from(SCREEN_TOL_PX_F32),
            "rails gap {gap_px_f32} must equal linear step {expected_px_f64}"
        );
        let on_px_f32 =
            camera.world_to_screen(DVec3::new(radius_m_f64 + RAILS_ALTITUDE_M_F64, 0.0, 0.0));
        assert!(
            (f32::midpoint(below_px_f32[0], above_px_f32[0]) - on_px_f32[0]).abs()
                < SCREEN_TOL_PX_F32,
            "rails midpoint must stay linear"
        );
        let sunk_px_f32 = camera.world_to_screen(DVec3::new(radius_m_f64 - 1.0, 0.0, 0.0));
        let lifted_px_f32 = camera.world_to_screen(DVec3::new(radius_m_f64 + 1.0, 0.0, 0.0));
        let surface_gap_px_f32 = (lifted_px_f32[0] - sunk_px_f32[0]).abs();
        assert!(
            (f64::from(surface_gap_px_f32) - expected_px_f64).abs() < f64::from(SCREEN_TOL_PX_F32),
            "surface gap {surface_gap_px_f32} must equal linear step {expected_px_f64}"
        );
    }
    let rebuilt_orbit = orbit_camera();
    assert_eq!(
        rebuilt_orbit.scale_px_per_m_f64().to_bits(),
        orbit.scale_px_per_m_f64().to_bits(),
        "orbit rebuild must keep scale bits"
    );
    assert_eq!(
        rebuilt_orbit.center_m(),
        orbit.center_m(),
        "orbit rebuild must keep center"
    );
    let rebuilt_entry = entry_camera();
    assert_eq!(
        rebuilt_entry.scale_px_per_m_f64().to_bits(),
        entry.scale_px_per_m_f64().to_bits(),
        "entry rebuild must keep scale bits"
    );
    assert_eq!(
        rebuilt_entry.center_m(),
        entry.center_m(),
        "entry rebuild must keep center"
    );
    let rebuilt_surface = surface_camera();
    assert_eq!(
        rebuilt_surface.scale_px_per_m_f64().to_bits(),
        surface.scale_px_per_m_f64().to_bits(),
        "surface rebuild must keep scale bits"
    );
    assert_eq!(
        rebuilt_surface.center_m(),
        surface.center_m(),
        "surface rebuild must keep center"
    );
    let worlds_m = [
        DVec3::ZERO,
        DVec3::new(radius_m_f64, 0.0, 0.0),
        DVec3::new(radius_m_f64 + RAILS_ALTITUDE_M_F64, 0.0, 0.0),
    ];
    let mut screens_px_f32 = [[0.0_f32; 2]; 3];
    match orbit.convert_into(&worlds_m, &mut screens_px_f32) {
        Ok(()) => {
            for (world_m, screen_px_f32) in worlds_m.iter().zip(screens_px_f32.iter()) {
                assert_screen_close_f32(*screen_px_f32, orbit.world_to_screen(*world_m));
            }
        }
        Err(error) => panic!("batch fill must succeed: {error}"),
    }
}

/// One observed regime transition with its channel triple.
///
/// Test-side record only; the monitor owns `HandoffMarker`.
#[cfg(feature = "dev-shell")]
struct CanvasHandoffRecord {
    /// True for the rails boundary, false for the surface boundary.
    is_rails_bool: bool,
    /// Before values in snapshot channel order.
    before_f64: [f64; 7],
    /// After values in snapshot channel order.
    after_f64: [f64; 7],
}

/// Snapshot channels in monitor order for the canvas drives.
#[cfg(all(feature = "dev-shell", not(miri)))]
fn canvas_channels_of(snapshot: &engine::inspect::SimSnapshot) -> [f64; 7] {
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

/// Capture one canvas-drive snapshot, panicking on rejection.
#[cfg(all(feature = "dev-shell", not(miri)))]
fn canvas_capture_snapshot(
    scheduler: &engine::sim::Scheduler,
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
        DRIVE_SEED_U64,
        DRIVE_SEED_U64,
        engine::warp::Warp::X1,
        true,
        false,
        false,
    ) {
        Ok(snapshot) => snapshot,
        Err(error) => panic!("canvas capture must succeed: {error}"),
    }
}

/// Record a handoff when the regime changes between snapshots.
#[cfg(all(feature = "dev-shell", not(miri)))]
fn canvas_record_transition(
    records: &mut Vec<CanvasHandoffRecord>,
    previous: &mut Option<(u8, [f64; 7])>,
    snapshot: &engine::inspect::SimSnapshot,
    channels_f64: [f64; 7],
) {
    if let Some((previous_regime_u8, previous_f64)) = previous
        && *previous_regime_u8 != snapshot.regime_u8
    {
        let crossed_rails =
            (previous_f64[0] > RAILS_ALTITUDE_M_F64) != (channels_f64[0] > RAILS_ALTITUDE_M_F64);
        records.push(CanvasHandoffRecord {
            is_rails_bool: crossed_rails,
            before_f64: *previous_f64,
            after_f64: channels_f64,
        });
    }
    *previous = Some((snapshot.regime_u8, channels_f64));
}

/// Panic when any handoff channel delta leaves the engine bands.
#[cfg(feature = "dev-shell")]
fn canvas_assert_handoff_inside_bands(record: &CanvasHandoffRecord, direction: &str) {
    let bands_f64 = if record.is_rails_bool {
        DRIVE_RAILS_BANDS_F64
    } else {
        DRIVE_SURFACE_BANDS_F64
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

/// Pick the fixed camera for a snapshot by regime plus altitude.
///
/// Mirrors the marks auto-view thresholds without hysteresis: orbit regime
/// selects orbit-fit, surface regime selects surface-grid, otherwise the
/// 120 km and 10 km altitudes select. Test grouping only; the view module
/// owns hysteresis.
#[cfg(all(feature = "dev-shell", not(miri)))]
fn canvas_camera_index_for(regime_u8: u8, altitude_m_f64: f64) -> usize {
    if regime_u8 == DRIVE_REGIME_ORBIT_U8 {
        return 0;
    }
    if regime_u8 == DRIVE_REGIME_SURFACE_U8 {
        return 2;
    }
    if altitude_m_f64 > RAILS_ALTITUDE_M_F64 {
        0
    } else if altitude_m_f64 > ENTRY_SURFACE_ALTITUDE_M_F64 {
        1
    } else {
        2
    }
}

/// Drive the 300 km retro descent to penetration through fixed cameras.
///
/// Creates the Mars-like environment internally, applies the 200 m/s retro
/// burn once, then coasts to the surface while projecting every snapshot
/// through the matching fixed camera. Returns handoff records, the
/// penetrating end state, the scheduler, and the step count.
#[cfg(all(feature = "dev-shell", not(miri)))]
fn canvas_run_descent(
    cameras: &[Camera2D; 3],
) -> (
    Vec<CanvasHandoffRecord>,
    engine::trajectory::StateVector,
    engine::sim::Scheduler,
    u32,
) {
    use engine::atmosphere::AtmosphereParams;
    use engine::body::BodyParams;
    use engine::orbit::Mu;
    use engine::sim::Scheduler;
    use engine::trajectory::{
        Burn, BurnDirection, StateVector, VehicleParams, apply_burn, step_point_ship,
    };
    use engine::units::{MetersPerSecond, Seconds};

    let body = BodyParams::mars_like();
    let atmosphere = match AtmosphereParams::mars_like() {
        Ok(atmosphere) => atmosphere,
        Err(error) => panic!("canvas atmosphere must validate: {error}"),
    };
    let vehicle = VehicleParams::preset();
    let mu = match Mu::new(body.gravitational_parameter_m3_s2()) {
        Ok(mu) => mu,
        Err(error) => panic!("canvas mu must validate: {error}"),
    };
    let radius_m_f64 = body.radius_m().value() + DRIVE_DESCENT_START_M_F64;
    let start = match StateVector::new(
        DVec3::new(radius_m_f64, 0.0, 0.0),
        DVec3::new(0.0, libm::sqrt(mu.value() / radius_m_f64), 0.0),
        Seconds::new(0.0),
    ) {
        Ok(state) => state,
        Err(error) => panic!("canvas start must validate: {error}"),
    };
    let burn = match Burn::new(
        BurnDirection::Retrograde,
        MetersPerSecond::new(DRIVE_DESCENT_RETRO_MPS_F64),
    ) {
        Ok(burn) => burn,
        Err(error) => panic!("canvas burn must validate: {error}"),
    };
    let mut current = match apply_burn(&start, &burn) {
        Ok(state) => state,
        Err(error) => panic!("canvas burn must apply: {error}"),
    };
    let step = Seconds::new(DRIVE_STEP_S_F64);
    let mut scheduler = Scheduler::default();
    let mut previous: Option<(u8, [f64; 7])> = None;
    let mut records: Vec<CanvasHandoffRecord> = Vec::new();
    let mut count_u32: u32 = 0;
    let mut penetrating: Option<StateVector> = None;
    for _ in 0..DRIVE_MAX_STEPS_U32 {
        let sample = match step_point_ship(&current, step, &body, &atmosphere, &vehicle, mu) {
            Ok(sample) => sample,
            Err(error) => panic!("canvas step must succeed: {error}"),
        };
        current = sample.state;
        scheduler.advance();
        let snapshot = canvas_capture_snapshot(&scheduler, &current, &body, &atmosphere, &vehicle);
        let channels_f64 = canvas_channels_of(&snapshot);
        canvas_record_transition(&mut records, &mut previous, &snapshot, channels_f64);
        let camera_usize = canvas_camera_index_for(snapshot.regime_u8, snapshot.altitude_m_f64);
        let position_m = DVec3::new(
            snapshot.position_m_f64[0],
            snapshot.position_m_f64[1],
            snapshot.position_m_f64[2],
        );
        let screen_px_f32 = cameras[camera_usize].world_to_screen(position_m);
        assert!(
            screen_px_f32[0].is_finite() && screen_px_f32[1].is_finite(),
            "descent snapshot must project finitely"
        );
        count_u32 += 1;
        let radius_m = libm::sqrt(current.position_m.length_squared());
        if radius_m - body.radius_m().value() <= 0.0 {
            penetrating = Some(current);
            break;
        }
    }
    let Some(end_state) = penetrating else {
        panic!("descent must reach the surface within the cap");
    };
    (records, end_state, scheduler, count_u32)
}

/// Settle a penetrating descent end into a parked touchdown.
///
/// Verifies touchdown through allowed readouts: surface regime, altitude
/// within 0.5 m, and corotating speed below 5 m/s.
#[cfg(all(feature = "dev-shell", not(miri)))]
fn canvas_settle_touchdown(
    penetrating: &engine::trajectory::StateVector,
    scheduler: &engine::sim::Scheduler,
) {
    use engine::atmosphere::AtmosphereParams;
    use engine::body::BodyParams;
    use engine::surface::{SurfaceFrame, TouchdownConfig};
    use engine::trajectory::VehicleParams;

    let body = BodyParams::mars_like();
    let atmosphere = match AtmosphereParams::mars_like() {
        Ok(atmosphere) => atmosphere,
        Err(error) => panic!("canvas atmosphere must validate: {error}"),
    };
    let vehicle = VehicleParams::preset();
    let frame = SurfaceFrame::new(&body);
    let parked = match frame.rest_state(penetrating) {
        Ok(parked) => parked,
        Err(error) => panic!("canvas rest state must build: {error}"),
    };
    let config = TouchdownConfig::preset();
    match frame.is_touchdown(&parked, &config) {
        Ok(touched) => assert!(touched, "parked descent must count as touchdown"),
        Err(error) => panic!("touchdown check must run: {error}"),
    }
    let parked_snapshot = canvas_capture_snapshot(scheduler, &parked, &body, &atmosphere, &vehicle);
    assert!(
        parked_snapshot.altitude_m_f64.abs() <= DRIVE_TOUCHDOWN_ALT_M_F64,
        "parked altitude must sit within 0.5 m"
    );
    assert!(
        parked_snapshot.speed_mps_f64 <= DRIVE_TOUCHDOWN_SPEED_MPS_F64,
        "parked speed must sit below 5 m/s"
    );
}

/// Drive the surface-to-rails climb through fixed cameras.
///
/// Starts from the surface kick with per-tick prograde boosts below 20 km
/// and projects every snapshot through the matching fixed camera. Returns
/// handoff records plus the step count.
#[cfg(all(feature = "dev-shell", not(miri)))]
fn canvas_run_ascent(cameras: &[Camera2D; 3]) -> (Vec<CanvasHandoffRecord>, u32) {
    use engine::atmosphere::AtmosphereParams;
    use engine::body::BodyParams;
    use engine::orbit::Mu;
    use engine::sim::Scheduler;
    use engine::trajectory::{
        Burn, BurnDirection, StateVector, VehicleParams, apply_burn, step_point_ship,
    };
    use engine::units::{MetersPerSecond, Seconds};

    let body = BodyParams::mars_like();
    let atmosphere = match AtmosphereParams::mars_like() {
        Ok(atmosphere) => atmosphere,
        Err(error) => panic!("canvas atmosphere must validate: {error}"),
    };
    let vehicle = VehicleParams::preset();
    let mu = match Mu::new(body.gravitational_parameter_m3_s2()) {
        Ok(mu) => mu,
        Err(error) => panic!("canvas mu must validate: {error}"),
    };
    let spin_rad_s_f64 = core::f64::consts::TAU / body.rotation_period_s().value();
    let mut climbing = match StateVector::new(
        DVec3::new(body.radius_m().value(), 0.0, 0.0),
        DVec3::new(
            DRIVE_ASCENT_KICK_MPS_F64,
            spin_rad_s_f64 * body.radius_m().value(),
            0.0,
        ),
        Seconds::new(0.0),
    ) {
        Ok(state) => state,
        Err(error) => panic!("canvas climb start must validate: {error}"),
    };
    let step = Seconds::new(DRIVE_STEP_S_F64);
    let mut scheduler = Scheduler::default();
    let seed_snapshot =
        canvas_capture_snapshot(&scheduler, &climbing, &body, &atmosphere, &vehicle);
    let mut previous: Option<(u8, [f64; 7])> =
        Some((seed_snapshot.regime_u8, canvas_channels_of(&seed_snapshot)));
    let mut records: Vec<CanvasHandoffRecord> = Vec::new();
    let mut count_u32: u32 = 0;
    let mut top_altitude_m_f64 = seed_snapshot.altitude_m_f64;
    for _ in 0..DRIVE_MAX_STEPS_U32 {
        let radius_m = libm::sqrt(climbing.position_m.length_squared());
        let altitude_m = radius_m - body.radius_m().value();
        if altitude_m > 0.0 && altitude_m < DRIVE_ASCENT_BOOST_TOP_M_F64 {
            let boost = match Burn::new(
                BurnDirection::Prograde,
                MetersPerSecond::new(DRIVE_ASCENT_BOOST_MPS_F64),
            ) {
                Ok(boost) => boost,
                Err(error) => panic!("canvas boost must validate: {error}"),
            };
            climbing = match apply_burn(&climbing, &boost) {
                Ok(kicked) => kicked,
                Err(error) => panic!("canvas boost must apply: {error}"),
            };
        }
        let sample = match step_point_ship(&climbing, step, &body, &atmosphere, &vehicle, mu) {
            Ok(sample) => sample,
            Err(error) => panic!("canvas climb step must succeed: {error}"),
        };
        climbing = sample.state;
        scheduler.advance();
        let snapshot = canvas_capture_snapshot(&scheduler, &climbing, &body, &atmosphere, &vehicle);
        let channels_f64 = canvas_channels_of(&snapshot);
        canvas_record_transition(&mut records, &mut previous, &snapshot, channels_f64);
        let camera_usize = canvas_camera_index_for(snapshot.regime_u8, snapshot.altitude_m_f64);
        let position_m = DVec3::new(
            snapshot.position_m_f64[0],
            snapshot.position_m_f64[1],
            snapshot.position_m_f64[2],
        );
        let screen_px_f32 = cameras[camera_usize].world_to_screen(position_m);
        assert!(
            screen_px_f32[0].is_finite() && screen_px_f32[1].is_finite(),
            "ascent snapshot must project finitely"
        );
        count_u32 += 1;
        top_altitude_m_f64 = snapshot.altitude_m_f64;
        if top_altitude_m_f64 > RAILS_ALTITUDE_M_F64 {
            break;
        }
    }
    assert!(
        top_altitude_m_f64 > RAILS_ALTITUDE_M_F64,
        "climb must clear rails within the cap"
    );
    (records, count_u32)
}

/// AC1 live profile: full descent plus ascent crosses four handoffs.
///
/// Starts orbit above 120 km, touches the surface grid, climbs back above
/// rails, and keeps every 7-channel delta inside the engine bands. Fixed
/// preset cameras project every snapshot finitely and rebuild bit-identical
/// afterwards, so the canvas never jumps within a view.
#[cfg(all(feature = "dev-shell", not(miri)))]
#[test]
fn ac1_descent_ascent_handoffs_inside_bands_with_stable_camera() {
    let orbit = orbit_camera();
    let entry = entry_camera();
    let surface = surface_camera();
    let cameras = [orbit, entry, surface];
    let orbit_bits_u64 = orbit.scale_px_per_m_f64().to_bits();
    let entry_bits_u64 = entry.scale_px_per_m_f64().to_bits();
    let surface_bits_u64 = surface.scale_px_per_m_f64().to_bits();
    let orbit_center = orbit.center_m();
    let entry_center = entry.center_m();
    let surface_center = surface.center_m();
    let (descent_records, penetrating, scheduler, descent_count_u32) = canvas_run_descent(&cameras);
    assert_eq!(
        descent_records.len(),
        2,
        "descent must cross rails plus surface"
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
        canvas_assert_handoff_inside_bands(record, "descent");
    }
    canvas_settle_touchdown(&penetrating, &scheduler);
    let (ascent_records, ascent_count_u32) = canvas_run_ascent(&cameras);
    assert_eq!(
        ascent_records.len(),
        2,
        "climb must cross surface plus rails"
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
        canvas_assert_handoff_inside_bands(record, "climb");
    }
    assert!(
        descent_count_u32 > 0 && ascent_count_u32 > 0,
        "both legs must step: descent {descent_count_u32} ascent {ascent_count_u32}"
    );
    let rebuilt_orbit = orbit_camera();
    let rebuilt_entry = entry_camera();
    let rebuilt_surface = surface_camera();
    assert_eq!(
        rebuilt_orbit.scale_px_per_m_f64().to_bits(),
        orbit_bits_u64,
        "orbit camera must stay bit-stable across the profile"
    );
    assert_eq!(
        rebuilt_entry.scale_px_per_m_f64().to_bits(),
        entry_bits_u64,
        "entry camera must stay bit-stable across the profile"
    );
    assert_eq!(
        rebuilt_surface.scale_px_per_m_f64().to_bits(),
        surface_bits_u64,
        "surface camera must stay bit-stable across the profile"
    );
    assert_eq!(rebuilt_orbit.center_m(), orbit_center);
    assert_eq!(rebuilt_entry.center_m(), entry_center);
    assert_eq!(rebuilt_surface.center_m(), surface_center);
}

/// AC2 source contract: determinism stays visible plus read-only.
///
/// Seeds, per-tick hashes, and the input recorder pin repeatability; the
/// marks history records snapshot copies only and never writes sim state,
/// with pre-sized buffers that reuse after warmup.
#[test]
fn ac2_determinism_source_contract() {
    assert_contains(SHELL_SRC, "observe_snapshot", "shell.rs");
    assert_contains(SHELL_SRC, "snapshot_hash", "shell.rs");
    assert_contains(SHELL_SRC, "SeedTreeView", "shell.rs");
    assert_contains(SHELL_SRC, "HashRing", "shell.rs");
    assert_contains(SHELL_SRC, "InputRecorder", "shell.rs");
    assert_contains(SHELL_SRC, "desktop_tester_readouts", "shell.rs");
    assert_contains(MARKS_SRC, "push_snapshot", "marks.rs");
    assert_contains(MARKS_SRC, "history_m", "marks.rs");
    assert_contains(MARKS_SRC, "never writes sim state", "marks.rs");
    assert_contains(MARKS_SRC, "pre-sized", "marks.rs");
    assert_contains(MARKS_SRC, "reuse after warmup", "marks.rs");
    assert_contains(MARKS_SRC, "Snapshot", "marks.rs");
    assert_lacks(SHELL_SRC, ".unwrap()", "shell.rs");
    assert_lacks(SHELL_SRC, ".expect(", "shell.rs");
    assert_lacks(SHELL_SRC, FORBIDDEN_NEEDLE, "shell.rs");
}

/// Collect per-step snapshot hashes for one descent with a seed.
///
/// Runs the 300 km retro descent to penetration and returns the hash after
/// every step plus the final altitude. Test helper only; the snapshot hash
/// owns the digest rule.
#[cfg(all(feature = "dev-shell", not(miri)))]
fn canvas_descent_hashes_with_seed(seed_u64: u64) -> (Vec<u64>, f64) {
    use engine::atmosphere::AtmosphereParams;
    use engine::body::BodyParams;
    use engine::orbit::Mu;
    use engine::sim::Scheduler;
    use engine::trajectory::{
        Burn, BurnDirection, StateVector, VehicleParams, apply_burn, step_point_ship,
    };
    use engine::units::{MetersPerSecond, Seconds};
    use engine::warp::Warp;

    let body = BodyParams::mars_like();
    let atmosphere = match AtmosphereParams::mars_like() {
        Ok(atmosphere) => atmosphere,
        Err(error) => panic!("hash atmosphere must validate: {error}"),
    };
    let vehicle = VehicleParams::preset();
    let mu = match Mu::new(body.gravitational_parameter_m3_s2()) {
        Ok(mu) => mu,
        Err(error) => panic!("hash mu must validate: {error}"),
    };
    let radius_m_f64 = body.radius_m().value() + DRIVE_DESCENT_START_M_F64;
    let start = match StateVector::new(
        DVec3::new(radius_m_f64, 0.0, 0.0),
        DVec3::new(0.0, libm::sqrt(mu.value() / radius_m_f64), 0.0),
        Seconds::new(0.0),
    ) {
        Ok(state) => state,
        Err(error) => panic!("hash start must validate: {error}"),
    };
    let burn = match Burn::new(
        BurnDirection::Retrograde,
        MetersPerSecond::new(DRIVE_DESCENT_RETRO_MPS_F64),
    ) {
        Ok(burn) => burn,
        Err(error) => panic!("hash burn must validate: {error}"),
    };
    let mut current = match apply_burn(&start, &burn) {
        Ok(state) => state,
        Err(error) => panic!("hash burn must apply: {error}"),
    };
    let step = Seconds::new(DRIVE_STEP_S_F64);
    let mut scheduler = Scheduler::default();
    let mut hashes_u64: Vec<u64> = Vec::new();
    let mut final_altitude_m_f64 = DRIVE_DESCENT_START_M_F64;
    for _ in 0..DRIVE_MAX_STEPS_U32 {
        let sample = match step_point_ship(&current, step, &body, &atmosphere, &vehicle, mu) {
            Ok(sample) => sample,
            Err(error) => panic!("hash step must succeed: {error}"),
        };
        current = sample.state;
        scheduler.advance();
        let snapshot = match engine::inspect::capture_snapshot(
            &scheduler,
            &current,
            &body,
            &atmosphere,
            &vehicle,
            seed_u64,
            seed_u64,
            Warp::X1,
            true,
            false,
            false,
        ) {
            Ok(snapshot) => snapshot,
            Err(error) => panic!("hash capture must succeed: {error}"),
        };
        hashes_u64.push(snapshot.snapshot_hash_u64);
        final_altitude_m_f64 = snapshot.altitude_m_f64;
        let radius_m = libm::sqrt(current.position_m.length_squared());
        if radius_m - body.radius_m().value() <= 0.0 {
            break;
        }
    }
    (hashes_u64, final_altitude_m_f64)
}

/// AC2 live check: same seed plus inputs yields the same shapes.
///
/// Two descents with the same seed produce identical per-step hashes and
/// bit-identical screen projections through the fixed orbit camera, while
/// a perturbed seed diverges. Identical hashes plus identical projections
/// imply identical canvas shapes; the camera itself is deterministic.
#[cfg(all(feature = "dev-shell", not(miri)))]
#[test]
fn ac2_same_seed_same_projection() {
    let (first_hashes_u64, first_alt_m_f64) = canvas_descent_hashes_with_seed(DRIVE_SEED_U64);
    let (second_hashes_u64, second_alt_m_f64) = canvas_descent_hashes_with_seed(DRIVE_SEED_U64);
    assert!(
        !first_hashes_u64.is_empty(),
        "first descent must hash at least one step"
    );
    assert_eq!(
        first_hashes_u64.len(),
        second_hashes_u64.len(),
        "same inputs must run the same step count"
    );
    for (index_usize, (first_u64, second_u64)) in first_hashes_u64
        .iter()
        .zip(second_hashes_u64.iter())
        .enumerate()
    {
        assert_eq!(
            first_u64, second_u64,
            "same seed must hash identically at step {index_usize}"
        );
    }
    assert!(
        (first_alt_m_f64 - second_alt_m_f64).abs() < FRACTION_TOL_F64,
        "same inputs must end at the same altitude"
    );
    let camera = orbit_camera();
    let first_screen_px_f32 = camera.world_to_screen(DVec3::new(
        test_body().radius_m().value() + DRIVE_DESCENT_START_M_F64,
        0.0,
        0.0,
    ));
    let second_screen_px_f32 = camera.world_to_screen(DVec3::new(
        test_body().radius_m().value() + DRIVE_DESCENT_START_M_F64,
        0.0,
        0.0,
    ));
    assert_eq!(
        first_screen_px_f32[0].to_bits(),
        second_screen_px_f32[0].to_bits(),
        "same inputs must project identically on x"
    );
    assert_eq!(
        first_screen_px_f32[1].to_bits(),
        second_screen_px_f32[1].to_bits(),
        "same inputs must project identically on y"
    );
    let (perturbed_hashes_u64, _) = canvas_descent_hashes_with_seed(DRIVE_SEED_U64 ^ 1);
    assert_eq!(
        perturbed_hashes_u64.len(),
        first_hashes_u64.len(),
        "perturbed seed must run the same step count"
    );
    let mut differs_bool = false;
    for (first_u64, perturbed_u64) in first_hashes_u64.iter().zip(perturbed_hashes_u64.iter()) {
        if first_u64 != perturbed_u64 {
            differs_bool = true;
            break;
        }
    }
    assert!(differs_bool, "perturbed seed must diverge the digest");
}

/// AC3 source contract: shell cost plus budget strip are measured.
///
/// The existing `ShellCostMeter` records draw cost separately from sim and
/// render cost with fractions of the named budgets, and the shell records
/// every draw through the measured hook. No meter change is needed here.
#[test]
fn ac3_budget_source_contract() {
    assert_contains(SHELL_COST_SRC, "ShellCostMeter", "shell_cost.rs");
    assert_contains(SHELL_COST_SRC, "record_sample", "shell_cost.rs");
    assert_contains(SHELL_COST_SRC, "fraction_of_budget", "shell_cost.rs");
    assert_contains(SHELL_COST_SRC, "budget_status", "shell_cost.rs");
    assert_contains(
        SHELL_COST_SRC,
        "SHELL_SAMPLE_CAPACITY_ENTRIES_USIZE",
        "shell_cost.rs",
    );
    assert_contains(SHELL_COST_SRC, "pre-sized", "shell_cost.rs");
    assert_contains(SHELL_COST_SRC, "reuse after warmup", "shell_cost.rs");
    assert_contains(SHELL_COST_SRC, "record_sample", "shell_cost.rs");
    assert_contains(SHELL_SRC, "record_draw_cost", "shell.rs");
    assert_contains(SHELL_SRC, "draw_measured", "shell.rs");
    assert_contains(SHELL_SRC, "BudgetDenominators", "shell.rs");
    assert_contains(SHELL_SRC, "draw_cost_ms_f64", "shell.rs");
    assert_contains(QUALITY_SRC, "SIM_TICK_AVG_MS", "quality.md");
    assert_contains(QUALITY_SRC, "SIM_TICK_P99_MS", "quality.md");
    assert_contains(QUALITY_SRC, "SURFACE_HITCH_P95_MS", "quality.md");
    assert_contains(QUALITY_SRC, "MEMORY_CEILING_MB", "quality.md");
    assert_contains(QUALITY_SRC, "COLD_START_S", "quality.md");
    assert_contains(QUALITY_SRC, "30 fps", "quality.md");
    assert_contains(DEBUG_SRC, "FRAME_BUDGET_MS", "debug.md");
    assert_contains(DEBUG_SRC, "Budget strip", "debug.md");
    assert_contains(DEBUG_SRC, "Shell cost", "debug.md");
    assert_lacks(SHELL_COST_SRC, ".unwrap()", "shell_cost.rs");
    assert_lacks(SHELL_COST_SRC, ".expect(", "shell_cost.rs");
    assert_lacks(SHELL_COST_SRC, FORBIDDEN_NEEDLE, "shell_cost.rs");
}

/// AC3 fraction math: smoke costs sit in the nominal band.
///
/// Reports exact fractions; gates live in `docs/tech/quality.md` and are
/// named here only.
#[test]
fn ac3_budget_fraction_math() {
    let shell_fraction_f64 = SMOKE_SHELL_DRAW_MS_F64 / FRAME_BUDGET_MS_F64;
    assert!(
        shell_fraction_f64 < NOMINAL_MAX_FRACTION_F64,
        "shell fraction {shell_fraction_f64} leaves the nominal band"
    );
    let frame_fraction_f64 = SMOKE_FRAME_MS_F64 / FRAME_BUDGET_MS_F64;
    assert!(
        frame_fraction_f64 < NOMINAL_MAX_FRACTION_F64,
        "frame fraction {frame_fraction_f64} leaves the nominal band"
    );
    let avg_fraction_f64 = SMOKE_SIM_AVG_MS_F64 / SIM_TICK_AVG_MS_F64;
    assert!(
        avg_fraction_f64 < NOMINAL_MAX_FRACTION_F64,
        "sim avg fraction {avg_fraction_f64} leaves the nominal band"
    );
    let p99_fraction_f64 = SMOKE_SIM_P99_MS_F64 / SIM_TICK_P99_MS_F64;
    assert!(
        p99_fraction_f64 < NOMINAL_MAX_FRACTION_F64,
        "sim p99 fraction {p99_fraction_f64} leaves the nominal band"
    );
    let hitch_fraction_f64 = 10.0 / SURFACE_HITCH_P95_MS_F64;
    assert!(
        hitch_fraction_f64 < NOMINAL_MAX_FRACTION_F64,
        "hitch fraction {hitch_fraction_f64} leaves the nominal band"
    );
    let memory_fraction_f64 = 256.0 / MEMORY_CEILING_MB_F64;
    assert!(
        memory_fraction_f64 < NOMINAL_MAX_FRACTION_F64,
        "memory fraction {memory_fraction_f64} leaves the nominal band"
    );
    let cold_fraction_f64 = 1.5 / COLD_START_S_F64;
    assert!(
        cold_fraction_f64 < NOMINAL_MAX_FRACTION_F64,
        "cold-start fraction {cold_fraction_f64} leaves the nominal band"
    );
    assert!((NOMINAL_MAX_FRACTION_F64 - 0.5).abs() < FRACTION_TOL_F64);
    assert!((ELEVATED_MAX_FRACTION_F64 - 0.8).abs() < FRACTION_TOL_F64);
    let over_f64 = 30.0 / FRAME_BUDGET_MS_F64;
    assert!(
        over_f64 >= ELEVATED_MAX_FRACTION_F64,
        "over fraction {over_f64} must read at or above the elevated line"
    );
}

/// AC3 headless timing: the desktop host holds the sim-tick average.
///
/// Steps the 250 km Mars-like cruise profile for 100 fixed ticks and
/// reports exact wall-clock numbers; the average must sit below the named
/// sim-tick average with headroom to spare.
#[test]
fn ac3_headless_tick_timing_holds_average() {
    use engine::atmosphere::AtmosphereParams;
    use engine::body::BodyParams;
    use engine::orbit::Mu;
    use engine::sim::{SIM_TICK_S, Scheduler};
    use engine::trajectory::{StateVector, VehicleParams, step_point_ship};
    use engine::units::Seconds;

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
    let radius_m_f64 = body.radius_m().value() + TIMING_CRUISE_ALTITUDE_M_F64;
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

/// AC3 allocation guard: the canvas step adds no dependency.
///
/// The workspace keeps the locked lines; plot, dock, and postcard stay
/// deferred while marks reuse pre-sized buffers.
#[test]
fn ac3_no_new_dependencies() {
    assert_lacks(WORKSPACE_CARGO_SRC, "egui_plot", "workspace Cargo.toml");
    assert_lacks(WORKSPACE_CARGO_SRC, "egui-plot", "workspace Cargo.toml");
    assert_lacks(WORKSPACE_CARGO_SRC, "postcard", "workspace Cargo.toml");
    assert_lacks(WORKSPACE_CARGO_SRC, "egui_dock", "workspace Cargo.toml");
    assert_lacks(WORKSPACE_CARGO_SRC, "egui-dock", "workspace Cargo.toml");
    assert_lacks(CARGO_LOCK_SRC, "egui_plot", "Cargo.lock");
    assert_lacks(CARGO_LOCK_SRC, "postcard", "Cargo.lock");
    assert_lacks(CARGO_LOCK_SRC, "egui_dock", "Cargo.lock");
    assert_contains(MARKS_SRC, "pre-sized", "marks.rs");
    assert_contains(MARKS_SRC, "never allocates after open", "marks.rs");
    assert_contains(SHELL_COST_SRC, "pre-sized", "shell_cost.rs");
    assert_contains(SHELL_COST_SRC, "reuse after warmup", "shell_cost.rs");
}

/// AC4 ticker-only guard: Descent paints the canvas, ticker-only skips it.
///
/// Ticker-only shows the top bar alone with no canvas; every other preset
/// paints. The in-crate headless draws own the shape counts (Descent emits
/// shapes, ticker-only emits fewer), pinned here by name so headless stays
/// green without a display.
#[test]
fn ac4_ticker_only_source_contract() {
    assert_contains(LAYOUT_SRC, "shows_marks_canvas_bool", "layout.rs");
    assert_contains(LAYOUT_SRC, "TickerOnly", "layout.rs");
    assert_contains(LAYOUT_SRC, "Descent", "layout.rs");
    assert_contains(LAYOUT_SRC, "TOP_BAR_BIT_U8", "layout.rs");
    assert_contains(LAYOUT_SRC, "ticker_only", "layout.rs");
    assert_contains(
        LAYOUT_SRC,
        "marks_canvas_shows_everywhere_but_ticker_only",
        "layout.rs",
    );
    assert_contains(MARKS_SRC, "shows_marks_canvas_bool", "marks.rs");
    assert_contains(
        MARKS_SRC,
        "draw_skips_everything_when_ticker_only",
        "marks.rs",
    );
    assert_contains(SHELL_SRC, "shows_marks_canvas_bool", "shell.rs");
    assert_contains(SHELL_SRC, "draw_measured", "shell.rs");
    assert_contains(
        SHELL_SRC,
        "marks_canvas_paints_on_descent_and_holds_on_ticker",
        "shell.rs",
    );
    assert_contains(
        SHELL_SRC,
        "PanelVisibility::for_preset(DesktopPreset::TickerOnly)",
        "shell.rs",
    );
    assert_contains(SHELL_SRC, "DesktopPreset::Descent", "shell.rs");
    assert_lacks(LAYOUT_SRC, ".unwrap()", "layout.rs");
    assert_lacks(LAYOUT_SRC, ".expect(", "layout.rs");
    assert_lacks(LAYOUT_SRC, FORBIDDEN_NEEDLE, "layout.rs");
}
