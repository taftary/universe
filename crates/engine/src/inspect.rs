//! Dev-shell snapshot view, 304 bytes.
//!
//! Plain-data [`SimSnapshot`] for the debug shell behind `dev-shell`.
//! Fields are raw `u64`, `f64`, `u32`, `i32`, `u8` only; no `hecs`,
//! no `DVec3`, no `f32`, no `String`, no `Vec`, no IO handles.
//! The single `f64` to `f32` conversion stays in [`crate::render`].
//! Hashing uses `xxh3-64` little-endian in declaration order, skipping
//! transport and pick fields; see [`snapshot_hash`]. Capture derives
//! readouts from scheduler, state, body, atmosphere, and vehicle with
//! `libm` only; see [`capture_snapshot`].

use glam::DVec3;
use thiserror::Error;
use xxhash_rust::xxh3::xxh3_64;

use crate::atmosphere::{AtmosphereError, AtmosphereParams};
use crate::body::BodyParams;
use crate::orbit::{Mu, OrbitError};
use crate::regime::{Regime, RegimeError, classify_state, warp_context_for_regime};
use crate::sim::Scheduler;
use crate::trajectory::{
    RAILS_ALTITUDE_M, STANDARD_GRAVITY_MPS2, SUTTON_GRAVES_K_SI, StateVector, VehicleParams,
    elements_from_state,
};
use crate::units::Meters;
use crate::warp::{Warp, apply_auto_drop};

/// Full circle in radians for spin-rate computation.
///
/// Source: `core::f64::consts::TAU`.
const TAU_RAD: f64 = core::f64::consts::TAU;

/// Hash buffer length in bytes, declaration order skipping excluded.
///
/// Source: sum of hashed field sizes in [`SimSnapshot`], issue 32 step 1.
const HASH_BUFFER_LEN_BYTES: usize = 263;

/// Snapshot size in bytes, `repr(C)` total.
///
/// Source: field-size sum in issue 32 step 1 design.
pub const SNAPSHOT_SIZE_BYTES: usize = 304;

/// X1 warp code, dimensionless.
///
/// Source: issue 32 step 1 design.
pub const WARP_CODE_X1_U8: u8 = 0;

/// X10 warp code, dimensionless.
///
/// Source: issue 32 step 1 design.
pub const WARP_CODE_X10_U8: u8 = 1;

/// X100 warp code, dimensionless.
///
/// Source: issue 32 step 1 design.
pub const WARP_CODE_X100_U8: u8 = 2;

/// X1000 warp code, dimensionless.
///
/// Source: issue 32 step 1 design.
pub const WARP_CODE_X1000_U8: u8 = 3;

/// X10000 warp code, dimensionless.
///
/// Source: issue 32 step 1 design.
pub const WARP_CODE_X10000_U8: u8 = 4;

/// Drop reason none or manual, dimensionless.
///
/// Source: issue 32 step 1 design.
pub const DROP_REASON_NONE_U8: u8 = 0;

/// Drop reason atmospheric entry, dimensionless.
///
/// Source: issue 32 step 1 design.
pub const DROP_REASON_ENTRY_U8: u8 = 1;

/// Drop reason approach, dimensionless.
///
/// Source: issue 32 step 1 design.
pub const DROP_REASON_APPROACH_U8: u8 = 2;

/// Drop reason alarm, dimensionless.
///
/// Source: issue 32 step 1 design.
pub const DROP_REASON_ALARM_U8: u8 = 3;

/// Warp flag in-ship bit, dimensionless.
///
/// Source: issue 32 step 1 design.
pub const WARP_FLAG_IN_SHIP_U8: u8 = 0x01;

/// Warp flag in-orbit-or-transit bit, dimensionless.
///
/// Source: issue 32 step 1 design.
pub const WARP_FLAG_IN_ORBIT_OR_TRANSIT_U8: u8 = 0x02;

/// Warp flag in-atmosphere bit, dimensionless.
///
/// Source: issue 32 step 1 design.
pub const WARP_FLAG_IN_ATMOSPHERE_U8: u8 = 0x04;

/// Warp flag approaching bit, dimensionless.
///
/// Source: issue 32 step 1 design.
pub const WARP_FLAG_APPROACHING_U8: u8 = 0x08;

/// Warp flag alarm bit, dimensionless.
///
/// Source: issue 32 step 1 design.
pub const WARP_FLAG_ALARM_U8: u8 = 0x10;

/// Orbit regime code, dimensionless.
///
/// Source: issue 32 step 1 design.
pub const REGIME_ORBIT_U8: u8 = 0;

/// Atmosphere regime code, dimensionless.
///
/// Source: issue 32 step 1 design.
pub const REGIME_ATMOSPHERE_U8: u8 = 1;

/// Surface regime code, dimensionless.
///
/// Source: issue 32 step 1 design.
pub const REGIME_SURFACE_U8: u8 = 2;

/// Orbital frame level, dimensionless.
///
/// Source: issue 32 step 1 design, architecture Lv5.
pub const FRAME_LEVEL_ORBIT_U8: u8 = 5;

/// Atmospheric frame level, dimensionless.
///
/// Source: issue 32 step 1 design, architecture Lv6.
pub const FRAME_LEVEL_ATMOSPHERE_U8: u8 = 6;

/// Surface frame level, dimensionless.
///
/// Source: issue 32 step 1 design, architecture Lv7.
pub const FRAME_LEVEL_SURFACE_U8: u8 = 7;

/// M1 frame depth, dimensionless.
///
/// Source: issue 32 step 1 design, M1 depth 2.
pub const FRAME_DEPTH_M1_U8: u8 = 2;

/// Star body id, dimensionless.
///
/// Source: issue 32 step 1 design.
pub const BODY_ID_STAR_U32: u32 = 0;

/// Planet body id, dimensionless.
///
/// Source: issue 32 step 1 design, M1 planet.
pub const BODY_ID_PLANET_U32: u32 = 1;

/// No-body id, dimensionless.
///
/// Source: issue 32 step 1 design, `u32::MAX` none.
pub const BODY_ID_NONE_U32: u32 = u32::MAX;

/// None mark kind, dimensionless.
///
/// Source: issue 32 step 1 design.
pub const MARK_KIND_NONE_U8: u8 = 0;

/// Star-point mark kind, dimensionless.
///
/// Source: issue 32 step 1 design.
pub const MARK_KIND_STAR_POINT_U8: u8 = 1;

/// Planet-circle mark kind, dimensionless.
///
/// Source: issue 32 step 1 design.
pub const MARK_KIND_PLANET_CIRCLE_U8: u8 = 2;

/// Atmosphere-layer mark kind, dimensionless.
///
/// Source: issue 32 step 1 design.
pub const MARK_KIND_ATMO_LAYER_U8: u8 = 3;

/// Orbit-curve mark kind, dimensionless.
///
/// Source: issue 32 step 1 design.
pub const MARK_KIND_ORBIT_CURVE_U8: u8 = 4;

/// Trajectory-curve mark kind, dimensionless.
///
/// Source: issue 32 step 1 design.
pub const MARK_KIND_TRAJECTORY_CURVE_U8: u8 = 5;

/// Ship-point mark kind, dimensionless.
///
/// Source: issue 32 step 1 design.
pub const MARK_KIND_SHIP_POINT_U8: u8 = 6;

/// Surface-grid mark kind, dimensionless.
///
/// Source: issue 32 step 1 design.
pub const MARK_KIND_SURFACE_GRID_U8: u8 = 7;

/// Snapshot capture failures.
#[derive(Debug, Error)]
pub enum InspectError {
    /// Input or computed value was not finite.
    #[error("non-finite value: {value_f64}")]
    NonFinite {
        /// Rejected value.
        value_f64: f64,
    },
    /// Position at the body center has no altitude.
    #[error("position at the body center has no altitude")]
    AtCenter,
    /// Regime classification failed.
    #[error(transparent)]
    Regime {
        /// Source regime error.
        #[from]
        source: RegimeError,
    },
    /// Atmosphere sampling failed.
    #[error(transparent)]
    Atmosphere {
        /// Source atmosphere error.
        #[from]
        source: AtmosphereError,
    },
    /// Gravity parameter construction failed.
    #[error(transparent)]
    Orbit {
        /// Source orbit error.
        #[from]
        source: OrbitError,
    },
}

/// Dev-shell snapshot view, 304 bytes.
///
/// Plain data only with declaration-order hashing in [`snapshot_hash`].
/// Built by [`capture_snapshot`]; pick and pad fields are transport only.
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct SimSnapshot {
    /// Tick count, dimensionless.
    pub tick_count_u64: u64,
    /// Elapsed sim time in seconds.
    pub elapsed_s_f64: f64,
    /// Ship epoch in seconds.
    pub ship_epoch_s_f64: f64,
    /// Master seed, dimensionless.
    pub master_seed_u64: u64,
    /// Stream seed, dimensionless.
    pub stream_seed_u64: u64,
    /// Snapshot hash, excluded from hash.
    pub snapshot_hash_u64: u64,
    /// Position in meters, XYZ.
    pub position_m_f64: [f64; 3],
    /// Velocity in meters per second, XYZ.
    pub velocity_mps_f64: [f64; 3],
    /// Drag acceleration in meters per second squared, XYZ.
    pub drag_mps2_f64: [f64; 3],
    /// Corotating velocity direction, unit XYZ.
    pub vel_dir_f64: [f64; 3],
    /// Altitude above surface in meters.
    pub altitude_m_f64: f64,
    /// Corotating speed in meters per second.
    pub speed_mps_f64: f64,
    /// Pressure in pascals.
    pub pressure_pa_f64: f64,
    /// Temperature in kelvin.
    pub temperature_k_f64: f64,
    /// Density in kilograms per cubic meter.
    pub density_kg_m3_f64: f64,
    /// Heat flux in watts per square meter.
    pub heat_flux_w_per_m2_f64: f64,
    /// G-load in g units, dimensionless.
    pub g_load_g_f64: f64,
    /// Semi-major axis in meters.
    pub semi_major_axis_m_f64: f64,
    /// Eccentricity, dimensionless in `[0, 1)`.
    pub eccentricity_f64: f64,
    /// Inclination in radians.
    pub inclination_rad_f64: f64,
    /// Node longitude in radians.
    pub raan_rad_f64: f64,
    /// Argument of periapsis in radians.
    pub arg_periapsis_rad_f64: f64,
    /// Mean anomaly in radians.
    pub mean_anomaly_rad_f64: f64,
    /// Gravity parameter in cubic meters per second squared.
    pub mu_m3_s2_f64: f64,
    /// Pick altitude in meters, excluded.
    pub pick_altitude_m_f64: f64,
    /// Pick range in meters, excluded.
    pub pick_range_m_f64: f64,
    /// Frame body id, dimensionless.
    pub frame_body_id_u32: u32,
    /// Parent body id, dimensionless.
    pub parent_body_id_u32: u32,
    /// Pick body id, excluded.
    pub pick_body_id_u32: u32,
    /// Pick cell X, excluded.
    pub pick_cell_x_i32: i32,
    /// Pick cell Y, excluded.
    pub pick_cell_y_i32: i32,
    /// Warp code, dimensionless `0-4`.
    pub warp_code_u8: u8,
    /// Drop reason, dimensionless `0-3`.
    pub drop_reason_u8: u8,
    /// Warp flags bitmask, dimensionless.
    pub warp_flags_u8: u8,
    /// Regime code, dimensionless `0-2`.
    pub regime_u8: u8,
    /// Frame level, dimensionless `5-7`.
    pub frame_level_u8: u8,
    /// Frame depth, dimensionless M1 `2`.
    pub frame_depth_u8: u8,
    /// Elements valid flag, dimensionless `0-1`.
    pub elements_valid_u8: u8,
    /// Pick valid flag, excluded.
    pub pick_valid_u8: u8,
    /// Mark kind, excluded `0-7`.
    pub mark_kind_u8: u8,
    /// Padding zeros, excluded.
    #[expect(
        clippy::pub_underscore_fields,
        reason = "spec names padding with leading underscore"
    )]
    pub _pad_u8: [u8; 3],
}

/// Hash snapshot per policy, `xxh3-64`.
///
/// Covers declaration order skipping hash, pick, and pad fields
/// with little-endian bytes and `-0.0` normalized to `+0.0`.
#[must_use]
pub fn snapshot_hash(snapshot: &SimSnapshot) -> u64 {
    let mut buffer_u8 = [0_u8; HASH_BUFFER_LEN_BYTES];
    let mut offset_usize = 0_usize;
    push_u64_le(&mut buffer_u8, &mut offset_usize, snapshot.tick_count_u64);
    push_f64_le(&mut buffer_u8, &mut offset_usize, snapshot.elapsed_s_f64);
    push_f64_le(&mut buffer_u8, &mut offset_usize, snapshot.ship_epoch_s_f64);
    push_u64_le(&mut buffer_u8, &mut offset_usize, snapshot.master_seed_u64);
    push_u64_le(&mut buffer_u8, &mut offset_usize, snapshot.stream_seed_u64);
    for value_f64 in snapshot.position_m_f64 {
        push_f64_le(&mut buffer_u8, &mut offset_usize, value_f64);
    }
    for value_f64 in snapshot.velocity_mps_f64 {
        push_f64_le(&mut buffer_u8, &mut offset_usize, value_f64);
    }
    for value_f64 in snapshot.drag_mps2_f64 {
        push_f64_le(&mut buffer_u8, &mut offset_usize, value_f64);
    }
    for value_f64 in snapshot.vel_dir_f64 {
        push_f64_le(&mut buffer_u8, &mut offset_usize, value_f64);
    }
    push_f64_le(&mut buffer_u8, &mut offset_usize, snapshot.altitude_m_f64);
    push_f64_le(&mut buffer_u8, &mut offset_usize, snapshot.speed_mps_f64);
    push_f64_le(&mut buffer_u8, &mut offset_usize, snapshot.pressure_pa_f64);
    push_f64_le(
        &mut buffer_u8,
        &mut offset_usize,
        snapshot.temperature_k_f64,
    );
    push_f64_le(
        &mut buffer_u8,
        &mut offset_usize,
        snapshot.density_kg_m3_f64,
    );
    push_f64_le(
        &mut buffer_u8,
        &mut offset_usize,
        snapshot.heat_flux_w_per_m2_f64,
    );
    push_f64_le(&mut buffer_u8, &mut offset_usize, snapshot.g_load_g_f64);
    push_f64_le(
        &mut buffer_u8,
        &mut offset_usize,
        snapshot.semi_major_axis_m_f64,
    );
    push_f64_le(&mut buffer_u8, &mut offset_usize, snapshot.eccentricity_f64);
    push_f64_le(
        &mut buffer_u8,
        &mut offset_usize,
        snapshot.inclination_rad_f64,
    );
    push_f64_le(&mut buffer_u8, &mut offset_usize, snapshot.raan_rad_f64);
    push_f64_le(
        &mut buffer_u8,
        &mut offset_usize,
        snapshot.arg_periapsis_rad_f64,
    );
    push_f64_le(
        &mut buffer_u8,
        &mut offset_usize,
        snapshot.mean_anomaly_rad_f64,
    );
    push_f64_le(&mut buffer_u8, &mut offset_usize, snapshot.mu_m3_s2_f64);
    push_u32_le(
        &mut buffer_u8,
        &mut offset_usize,
        snapshot.frame_body_id_u32,
    );
    push_u32_le(
        &mut buffer_u8,
        &mut offset_usize,
        snapshot.parent_body_id_u32,
    );
    push_u8(&mut buffer_u8, &mut offset_usize, snapshot.warp_code_u8);
    push_u8(&mut buffer_u8, &mut offset_usize, snapshot.drop_reason_u8);
    push_u8(&mut buffer_u8, &mut offset_usize, snapshot.warp_flags_u8);
    push_u8(&mut buffer_u8, &mut offset_usize, snapshot.regime_u8);
    push_u8(&mut buffer_u8, &mut offset_usize, snapshot.frame_level_u8);
    push_u8(&mut buffer_u8, &mut offset_usize, snapshot.frame_depth_u8);
    push_u8(
        &mut buffer_u8,
        &mut offset_usize,
        snapshot.elements_valid_u8,
    );
    xxh3_64(&buffer_u8)
}

/// Capture snapshot from sim state, normalized.
///
/// Derives altitude, corotating speed and direction, atmosphere, aero,
/// elements, warp, regime, and fixed M1 frame ids with `-0.0` to `+0.0`.
///
/// # Errors
///
/// Returns [`InspectError::NonFinite`] for non-finite inputs or outputs,
/// [`InspectError::AtCenter`] at the body center, or wrapped regime,
/// atmosphere, and orbit errors from classification and sampling.
#[expect(
    clippy::too_many_arguments,
    reason = "snapshot needs scheduler, state, environment, seeds, and warp"
)]
pub fn capture_snapshot(
    scheduler: &Scheduler,
    state: &StateVector,
    body: &BodyParams,
    atmosphere: &AtmosphereParams,
    vehicle: &VehicleParams,
    master_seed_u64: u64,
    stream_seed_u64: u64,
    requested: Warp,
    in_ship: bool,
    approaching: bool,
    alarm: bool,
) -> Result<SimSnapshot, InspectError> {
    let elapsed_s_f64 = scheduler.elapsed().value();
    ensure_finite_inputs(state, elapsed_s_f64)?;
    let regime = classify_state(state, body)?;
    let altitude_m_f64 = altitude_from_state(state, body)?;
    let wind = corotating_wind(state, body)?;
    let air = sample_air(altitude_m_f64, body, atmosphere)?;
    let aero = aero_loads(altitude_m_f64, &wind, air.density_kg_m3_f64, vehicle)?;
    let mu_m3_s2_f64 = body.gravitational_parameter_m3_s2();
    if !mu_m3_s2_f64.is_finite() || mu_m3_s2_f64 <= 0.0 {
        return Err(InspectError::NonFinite {
            value_f64: mu_m3_s2_f64,
        });
    }
    let elements = elements_or_zeros(regime, state, mu_m3_s2_f64)?;
    let warp = warp_view(regime, requested, in_ship, approaching, alarm);
    let mut snapshot = SimSnapshot {
        tick_count_u64: scheduler.step_count(),
        elapsed_s_f64: normalize_zero_f64(elapsed_s_f64),
        ship_epoch_s_f64: normalize_zero_f64(state.epoch.value()),
        master_seed_u64,
        stream_seed_u64,
        snapshot_hash_u64: 0_u64,
        position_m_f64: normalize_array3_f64([
            state.position_m.x,
            state.position_m.y,
            state.position_m.z,
        ]),
        velocity_mps_f64: normalize_array3_f64([
            state.velocity_mps.x,
            state.velocity_mps.y,
            state.velocity_mps.z,
        ]),
        drag_mps2_f64: normalize_array3_f64([aero.drag_mps.x, aero.drag_mps.y, aero.drag_mps.z]),
        vel_dir_f64: normalize_array3_f64([
            wind.direction_mps.x,
            wind.direction_mps.y,
            wind.direction_mps.z,
        ]),
        altitude_m_f64: normalize_zero_f64(altitude_m_f64),
        speed_mps_f64: normalize_zero_f64(wind.speed_mps_f64),
        pressure_pa_f64: normalize_zero_f64(air.pressure_pa_f64),
        temperature_k_f64: normalize_zero_f64(air.temperature_k_f64),
        density_kg_m3_f64: normalize_zero_f64(air.density_kg_m3_f64),
        heat_flux_w_per_m2_f64: normalize_zero_f64(aero.heat_flux_w_per_m2_f64),
        g_load_g_f64: normalize_zero_f64(aero.g_load_g_f64),
        semi_major_axis_m_f64: normalize_zero_f64(elements.semi_major_axis_m_f64),
        eccentricity_f64: normalize_zero_f64(elements.eccentricity_f64),
        inclination_rad_f64: normalize_zero_f64(elements.inclination_rad_f64),
        raan_rad_f64: normalize_zero_f64(elements.raan_rad_f64),
        arg_periapsis_rad_f64: normalize_zero_f64(elements.arg_periapsis_rad_f64),
        mean_anomaly_rad_f64: normalize_zero_f64(elements.mean_anomaly_rad_f64),
        mu_m3_s2_f64: normalize_zero_f64(mu_m3_s2_f64),
        pick_altitude_m_f64: 0.0,
        pick_range_m_f64: 0.0,
        frame_body_id_u32: BODY_ID_PLANET_U32,
        parent_body_id_u32: BODY_ID_STAR_U32,
        pick_body_id_u32: BODY_ID_NONE_U32,
        pick_cell_x_i32: 0,
        pick_cell_y_i32: 0,
        warp_code_u8: warp.warp_code_u8,
        drop_reason_u8: warp.drop_reason_u8,
        warp_flags_u8: warp.warp_flags_u8,
        regime_u8: warp.regime_u8,
        frame_level_u8: warp.frame_level_u8,
        frame_depth_u8: FRAME_DEPTH_M1_U8,
        elements_valid_u8: elements.valid_u8,
        pick_valid_u8: 0_u8,
        mark_kind_u8: MARK_KIND_NONE_U8,
        _pad_u8: [0_u8; 3],
    };
    snapshot.snapshot_hash_u64 = snapshot_hash(&snapshot);
    Ok(snapshot)
}

/// Corotating wind with speed and direction.
#[derive(Debug, Clone, Copy)]
struct CorotatingWind {
    relative_mps: DVec3,
    speed_mps_f64: f64,
    direction_mps: DVec3,
}

/// Sampled air with pressure, temperature, and density.
#[expect(
    clippy::struct_field_names,
    reason = "unit suffixes are required by the naming rule"
)]
#[derive(Debug, Clone, Copy)]
struct AirSample {
    pressure_pa_f64: f64,
    temperature_k_f64: f64,
    density_kg_m3_f64: f64,
}

/// Aero loads with drag, heat flux, and g-load.
#[derive(Debug, Clone, Copy)]
struct AeroLoads {
    drag_mps: DVec3,
    heat_flux_w_per_m2_f64: f64,
    g_load_g_f64: f64,
}

/// Elements view with validity flag.
#[derive(Debug, Clone, Copy)]
struct ElementsView {
    semi_major_axis_m_f64: f64,
    eccentricity_f64: f64,
    inclination_rad_f64: f64,
    raan_rad_f64: f64,
    arg_periapsis_rad_f64: f64,
    mean_anomaly_rad_f64: f64,
    valid_u8: u8,
}

/// Warp view with regime and frame codes.
#[expect(
    clippy::struct_field_names,
    reason = "unit suffixes are required by the naming rule"
)]
#[derive(Debug, Clone, Copy)]
struct WarpView {
    warp_code_u8: u8,
    drop_reason_u8: u8,
    warp_flags_u8: u8,
    regime_u8: u8,
    frame_level_u8: u8,
}

/// Check state and elapsed finiteness.
fn ensure_finite_inputs(state: &StateVector, elapsed_s_f64: f64) -> Result<(), InspectError> {
    if !state.position_m.x.is_finite()
        || !state.position_m.y.is_finite()
        || !state.position_m.z.is_finite()
    {
        return Err(InspectError::NonFinite {
            value_f64: state.position_m.x + state.position_m.y + state.position_m.z,
        });
    }
    if !state.velocity_mps.x.is_finite()
        || !state.velocity_mps.y.is_finite()
        || !state.velocity_mps.z.is_finite()
    {
        return Err(InspectError::NonFinite {
            value_f64: state.velocity_mps.x + state.velocity_mps.y + state.velocity_mps.z,
        });
    }
    if !state.epoch.value().is_finite() {
        return Err(InspectError::NonFinite {
            value_f64: state.epoch.value(),
        });
    }
    if !elapsed_s_f64.is_finite() {
        return Err(InspectError::NonFinite {
            value_f64: elapsed_s_f64,
        });
    }
    Ok(())
}

/// Altitude from radius minus body radius.
fn altitude_from_state(state: &StateVector, body: &BodyParams) -> Result<f64, InspectError> {
    let radius_m_f64 = libm::sqrt(state.position_m.length_squared());
    if !radius_m_f64.is_finite() {
        return Err(InspectError::NonFinite {
            value_f64: radius_m_f64,
        });
    }
    if radius_m_f64 <= 0.0 {
        return Err(InspectError::AtCenter);
    }
    let altitude_m_f64 = radius_m_f64 - body.radius_m().value();
    if !altitude_m_f64.is_finite() {
        return Err(InspectError::NonFinite {
            value_f64: altitude_m_f64,
        });
    }
    Ok(altitude_m_f64)
}

/// Corotating relative wind shared with trajectory.
fn corotating_wind(state: &StateVector, body: &BodyParams) -> Result<CorotatingWind, InspectError> {
    let spin_rad_s_f64 = TAU_RAD / body.rotation_period_s().value();
    if !spin_rad_s_f64.is_finite() {
        return Err(InspectError::NonFinite {
            value_f64: spin_rad_s_f64,
        });
    }
    let corotation_mps = DVec3::new(
        -spin_rad_s_f64 * state.position_m.y,
        spin_rad_s_f64 * state.position_m.x,
        0.0,
    );
    if !corotation_mps.x.is_finite()
        || !corotation_mps.y.is_finite()
        || !corotation_mps.z.is_finite()
    {
        return Err(InspectError::NonFinite {
            value_f64: corotation_mps.x + corotation_mps.y + corotation_mps.z,
        });
    }
    let relative_mps = state.velocity_mps - corotation_mps;
    if !relative_mps.x.is_finite() || !relative_mps.y.is_finite() || !relative_mps.z.is_finite() {
        return Err(InspectError::NonFinite {
            value_f64: relative_mps.x + relative_mps.y + relative_mps.z,
        });
    }
    let speed_mps_f64 = libm::sqrt(relative_mps.length_squared());
    if !speed_mps_f64.is_finite() {
        return Err(InspectError::NonFinite {
            value_f64: speed_mps_f64,
        });
    }
    let direction_mps = if speed_mps_f64 > 0.0 {
        let direction_mps = relative_mps / speed_mps_f64;
        if !direction_mps.x.is_finite()
            || !direction_mps.y.is_finite()
            || !direction_mps.z.is_finite()
        {
            return Err(InspectError::NonFinite {
                value_f64: direction_mps.x + direction_mps.y + direction_mps.z,
            });
        }
        direction_mps
    } else {
        DVec3::ZERO
    };
    Ok(CorotatingWind {
        relative_mps,
        speed_mps_f64,
        direction_mps,
    })
}

/// Sample pressure, temperature, and density at clamped altitude.
fn sample_air(
    altitude_m_f64: f64,
    body: &BodyParams,
    atmosphere: &AtmosphereParams,
) -> Result<AirSample, InspectError> {
    let clamped_alt_m_f64 = if altitude_m_f64 < 0.0 {
        0.0
    } else {
        altitude_m_f64
    };
    let sample = atmosphere.sample_at_altitude(Meters::new(clamped_alt_m_f64), body)?;
    let pressure_pa_f64 = sample.pressure_pa().value();
    let temperature_k_f64 = sample.temperature_k().value();
    let density_kg_m3_f64 = sample.density_kg_per_m3().value();
    if !pressure_pa_f64.is_finite()
        || !temperature_k_f64.is_finite()
        || !density_kg_m3_f64.is_finite()
    {
        return Err(InspectError::NonFinite {
            value_f64: pressure_pa_f64 + temperature_k_f64 + density_kg_m3_f64,
        });
    }
    Ok(AirSample {
        pressure_pa_f64,
        temperature_k_f64,
        density_kg_m3_f64,
    })
}

/// Aero loads matching step end-aero, zero on rails.
fn aero_loads(
    altitude_m_f64: f64,
    wind: &CorotatingWind,
    density_kg_m3_f64: f64,
    vehicle: &VehicleParams,
) -> Result<AeroLoads, InspectError> {
    if altitude_m_f64 > RAILS_ALTITUDE_M {
        return Ok(AeroLoads {
            drag_mps: DVec3::ZERO,
            heat_flux_w_per_m2_f64: 0.0,
            g_load_g_f64: 0.0,
        });
    }
    let ballistic_kg_per_m2_f64 = vehicle.ballistic_coefficient().value();
    let nose_radius_m_f64 = vehicle.nose_radius_m().value();
    if !ballistic_kg_per_m2_f64.is_finite() || !nose_radius_m_f64.is_finite() {
        return Err(InspectError::NonFinite {
            value_f64: ballistic_kg_per_m2_f64 + nose_radius_m_f64,
        });
    }
    let drag_mps = wind.relative_mps
        * (-density_kg_m3_f64 * wind.speed_mps_f64 / (2.0 * ballistic_kg_per_m2_f64));
    if !drag_mps.x.is_finite() || !drag_mps.y.is_finite() || !drag_mps.z.is_finite() {
        return Err(InspectError::NonFinite {
            value_f64: drag_mps.x + drag_mps.y + drag_mps.z,
        });
    }
    let heat_flux_w_per_m2_f64 = SUTTON_GRAVES_K_SI
        * libm::sqrt(density_kg_m3_f64 / nose_radius_m_f64)
        * wind.speed_mps_f64
        * wind.speed_mps_f64
        * wind.speed_mps_f64;
    if !heat_flux_w_per_m2_f64.is_finite() {
        return Err(InspectError::NonFinite {
            value_f64: heat_flux_w_per_m2_f64,
        });
    }
    let g_load_g_f64 = libm::sqrt(drag_mps.length_squared()) / STANDARD_GRAVITY_MPS2;
    if !g_load_g_f64.is_finite() {
        return Err(InspectError::NonFinite {
            value_f64: g_load_g_f64,
        });
    }
    Ok(AeroLoads {
        drag_mps,
        heat_flux_w_per_m2_f64,
        g_load_g_f64,
    })
}

/// Elements in orbit, zeros otherwise, never `NaN`.
fn elements_or_zeros(
    regime: Regime,
    state: &StateVector,
    mu_m3_s2_f64: f64,
) -> Result<ElementsView, InspectError> {
    match regime {
        Regime::Orbit => {
            let mu = Mu::new(mu_m3_s2_f64)?;
            match elements_from_state(state, mu) {
                Ok(elements) => {
                    let axis_m_f64 = elements.semi_major_axis.value();
                    let ecc_f64 = elements.eccentricity_f64;
                    let incl_rad_f64 = elements.inclination_rad;
                    let raan_rad_f64 = elements.raan_rad;
                    let arg_rad_f64 = elements.arg_periapsis_rad;
                    let mean_rad_f64 = elements.mean_anomaly_at_epoch_rad;
                    if !axis_m_f64.is_finite()
                        || !ecc_f64.is_finite()
                        || !incl_rad_f64.is_finite()
                        || !raan_rad_f64.is_finite()
                        || !arg_rad_f64.is_finite()
                        || !mean_rad_f64.is_finite()
                    {
                        return Err(InspectError::NonFinite {
                            value_f64: axis_m_f64 + ecc_f64 + incl_rad_f64,
                        });
                    }
                    Ok(ElementsView {
                        semi_major_axis_m_f64: axis_m_f64,
                        eccentricity_f64: ecc_f64,
                        inclination_rad_f64: incl_rad_f64,
                        raan_rad_f64,
                        arg_periapsis_rad_f64: arg_rad_f64,
                        mean_anomaly_rad_f64: mean_rad_f64,
                        valid_u8: 1_u8,
                    })
                }
                Err(_) => Ok(ElementsView {
                    semi_major_axis_m_f64: 0.0,
                    eccentricity_f64: 0.0,
                    inclination_rad_f64: 0.0,
                    raan_rad_f64: 0.0,
                    arg_periapsis_rad_f64: 0.0,
                    mean_anomaly_rad_f64: 0.0,
                    valid_u8: 0_u8,
                }),
            }
        }
        Regime::Atmosphere | Regime::Surface => Ok(ElementsView {
            semi_major_axis_m_f64: 0.0,
            eccentricity_f64: 0.0,
            inclination_rad_f64: 0.0,
            raan_rad_f64: 0.0,
            arg_periapsis_rad_f64: 0.0,
            mean_anomaly_rad_f64: 0.0,
            valid_u8: 0_u8,
        }),
    }
}

/// Warp, drop, flags, regime, and level from regime context.
fn warp_view(
    regime: Regime,
    requested: Warp,
    in_ship: bool,
    approaching: bool,
    alarm: bool,
) -> WarpView {
    let ctx = warp_context_for_regime(regime, in_ship, approaching, alarm);
    let effective = apply_auto_drop(requested, ctx);
    let warp_code_u8 = match effective {
        Warp::X1 => WARP_CODE_X1_U8,
        Warp::X10 => WARP_CODE_X10_U8,
        Warp::X100 => WARP_CODE_X100_U8,
        Warp::X1000 => WARP_CODE_X1000_U8,
        Warp::X10000 => WARP_CODE_X10000_U8,
    };
    let drop_reason_u8 = if ctx.in_atmosphere {
        DROP_REASON_ENTRY_U8
    } else if ctx.approaching {
        DROP_REASON_APPROACH_U8
    } else if ctx.alarm {
        DROP_REASON_ALARM_U8
    } else {
        DROP_REASON_NONE_U8
    };
    let mut warp_flags_u8 = 0_u8;
    if ctx.in_ship {
        warp_flags_u8 |= WARP_FLAG_IN_SHIP_U8;
    }
    if ctx.in_orbit_or_transit {
        warp_flags_u8 |= WARP_FLAG_IN_ORBIT_OR_TRANSIT_U8;
    }
    if ctx.in_atmosphere {
        warp_flags_u8 |= WARP_FLAG_IN_ATMOSPHERE_U8;
    }
    if ctx.approaching {
        warp_flags_u8 |= WARP_FLAG_APPROACHING_U8;
    }
    if ctx.alarm {
        warp_flags_u8 |= WARP_FLAG_ALARM_U8;
    }
    let (regime_u8, frame_level_u8) = match regime {
        Regime::Orbit => (REGIME_ORBIT_U8, FRAME_LEVEL_ORBIT_U8),
        Regime::Atmosphere => (REGIME_ATMOSPHERE_U8, FRAME_LEVEL_ATMOSPHERE_U8),
        Regime::Surface => (REGIME_SURFACE_U8, FRAME_LEVEL_SURFACE_U8),
    };
    WarpView {
        warp_code_u8,
        drop_reason_u8,
        warp_flags_u8,
        regime_u8,
        frame_level_u8,
    }
}

fn normalize_zero_f64(value_f64: f64) -> f64 {
    if value_f64 == 0.0 { 0.0 } else { value_f64 }
}

fn normalize_array3_f64(values_f64: [f64; 3]) -> [f64; 3] {
    [
        normalize_zero_f64(values_f64[0]),
        normalize_zero_f64(values_f64[1]),
        normalize_zero_f64(values_f64[2]),
    ]
}

fn push_u64_le(
    buffer_u8: &mut [u8; HASH_BUFFER_LEN_BYTES],
    offset_usize: &mut usize,
    value_u64: u64,
) {
    let bytes = value_u64.to_le_bytes();
    let next_usize = *offset_usize + 8;
    buffer_u8[*offset_usize..next_usize].copy_from_slice(&bytes);
    *offset_usize = next_usize;
}

fn push_u32_le(
    buffer_u8: &mut [u8; HASH_BUFFER_LEN_BYTES],
    offset_usize: &mut usize,
    value_u32: u32,
) {
    let bytes = value_u32.to_le_bytes();
    let next_usize = *offset_usize + 4;
    buffer_u8[*offset_usize..next_usize].copy_from_slice(&bytes);
    *offset_usize = next_usize;
}

fn push_u8(buffer_u8: &mut [u8; HASH_BUFFER_LEN_BYTES], offset_usize: &mut usize, value_u8: u8) {
    buffer_u8[*offset_usize] = value_u8;
    *offset_usize += 1;
}

fn push_f64_le(
    buffer_u8: &mut [u8; HASH_BUFFER_LEN_BYTES],
    offset_usize: &mut usize,
    value_f64: f64,
) {
    let normalized_f64 = normalize_zero_f64(value_f64);
    let bytes = normalized_f64.to_bits().to_le_bytes();
    let next_usize = *offset_usize + 8;
    buffer_u8[*offset_usize..next_usize].copy_from_slice(&bytes);
    *offset_usize = next_usize;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::Seconds;

    const MARS_RADIUS_M: f64 = 3_389_500.0;
    const ORBIT_ALTITUDE_M: f64 = 250_000.0;
    const ATMO_ALTITUDE_M: f64 = 50_000.0;
    const MASTER_SEED_U64: u64 = 0x243F_6A88_85A3_08D3;
    const STREAM_SEED_U64: u64 = 0x1319_8A2E_0370_7344;
    const DIR_TOL_F64: f64 = 1e-12;
    const ZERO_TOL_F64: f64 = 1e-12;

    fn test_body() -> BodyParams {
        BodyParams::mars_like()
    }

    fn test_atmosphere() -> AtmosphereParams {
        let Ok(atmosphere) = AtmosphereParams::mars_like() else {
            panic!("mars atmosphere must validate")
        };
        atmosphere
    }

    fn test_vehicle() -> VehicleParams {
        VehicleParams::preset()
    }

    fn state_at_altitude(altitude_m: f64, speed_mps: f64) -> StateVector {
        let Ok(state) = StateVector::new(
            DVec3::new(MARS_RADIUS_M + altitude_m, 0.0, 0.0),
            DVec3::new(0.0, speed_mps, 0.0),
            Seconds::new(0.0),
        ) else {
            panic!("test state must validate")
        };
        state
    }

    fn orbit_state() -> StateVector {
        let body = test_body();
        let mu_f64 = body.gravitational_parameter_m3_s2();
        let radius_m = MARS_RADIUS_M + ORBIT_ALTITUDE_M;
        state_at_altitude(ORBIT_ALTITUDE_M, libm::sqrt(mu_f64 / radius_m))
    }

    fn capture_orbit() -> SimSnapshot {
        let scheduler = Scheduler::default();
        let Ok(snapshot) = capture_snapshot(
            &scheduler,
            &orbit_state(),
            &test_body(),
            &test_atmosphere(),
            &test_vehicle(),
            MASTER_SEED_U64,
            STREAM_SEED_U64,
            Warp::X100,
            true,
            false,
            false,
        ) else {
            panic!("orbit capture must succeed")
        };
        snapshot
    }

    fn assert_zero_f64(value_f64: f64) {
        assert!(
            libm::fabs(value_f64) < ZERO_TOL_F64,
            "expected zero, got {value_f64}"
        );
    }

    #[test]
    fn snapshot_size_is_304() {
        assert_eq!(core::mem::size_of::<SimSnapshot>(), SNAPSHOT_SIZE_BYTES);
        assert_eq!(SNAPSHOT_SIZE_BYTES, 304);
        let snapshot = capture_orbit();
        assert_eq!(bytemuck::bytes_of(&snapshot).len(), 304);
    }

    #[expect(
        clippy::used_underscore_binding,
        reason = "pad exclusion check needs direct field"
    )]
    #[test]
    fn hash_excludes_transport_and_pick() {
        let baseline = capture_orbit();
        let baseline_hash = snapshot_hash(&baseline);
        assert_eq!(baseline.snapshot_hash_u64, baseline_hash);
        let mut transport = baseline;
        transport.snapshot_hash_u64 = baseline_hash.wrapping_add(1);
        assert_eq!(snapshot_hash(&transport), baseline_hash);
        let mut pick = baseline;
        pick.pick_altitude_m_f64 = 1_000.0;
        pick.pick_range_m_f64 = 2_000.0;
        pick.pick_body_id_u32 = BODY_ID_PLANET_U32;
        pick.pick_cell_x_i32 = 3;
        pick.pick_cell_y_i32 = 4;
        pick.pick_valid_u8 = 1_u8;
        pick.mark_kind_u8 = MARK_KIND_SHIP_POINT_U8;
        pick._pad_u8 = [9_u8, 9_u8, 9_u8];
        assert_eq!(snapshot_hash(&pick), baseline_hash);
        let mut included = baseline;
        included.altitude_m_f64 += 1.0;
        assert_ne!(snapshot_hash(&included), baseline_hash);
    }

    #[test]
    fn hash_normalizes_negative_zero() {
        let baseline = capture_orbit();
        let mut negated = baseline;
        negated.g_load_g_f64 = -0.0;
        negated.pressure_pa_f64 = -0.0;
        negated.drag_mps2_f64 = [-0.0, -0.0, -0.0];
        assert_eq!(snapshot_hash(&negated), snapshot_hash(&baseline));
    }

    #[test]
    fn capture_rejects_non_finite() {
        let scheduler = Scheduler::default();
        let bad = StateVector {
            position_m: DVec3::new(f64::NAN, 0.0, 0.0),
            velocity_mps: DVec3::ZERO,
            epoch: Seconds::new(0.0),
        };
        assert!(matches!(
            capture_snapshot(
                &scheduler,
                &bad,
                &test_body(),
                &test_atmosphere(),
                &test_vehicle(),
                MASTER_SEED_U64,
                STREAM_SEED_U64,
                Warp::X1,
                true,
                false,
                false,
            ),
            Err(InspectError::NonFinite { .. } | InspectError::Regime { .. })
        ));
    }

    #[test]
    fn capture_rejects_center() {
        let scheduler = Scheduler::default();
        let Ok(at_center) = StateVector::new(DVec3::ZERO, DVec3::ZERO, Seconds::new(0.0)) else {
            panic!("center state must validate")
        };
        assert!(matches!(
            capture_snapshot(
                &scheduler,
                &at_center,
                &test_body(),
                &test_atmosphere(),
                &test_vehicle(),
                MASTER_SEED_U64,
                STREAM_SEED_U64,
                Warp::X1,
                true,
                false,
                false,
            ),
            Err(InspectError::AtCenter | InspectError::Regime { .. })
        ));
    }

    #[test]
    fn capture_elements_valid_only_in_orbit() {
        let orbit = capture_orbit();
        assert_eq!(orbit.elements_valid_u8, 1_u8);
        assert!(orbit.semi_major_axis_m_f64 > 0.0);
        assert!(orbit.eccentricity_f64.is_finite());
        let scheduler = Scheduler::default();
        let atmo_state = state_at_altitude(ATMO_ALTITUDE_M, 3_000.0);
        let Ok(atmo) = capture_snapshot(
            &scheduler,
            &atmo_state,
            &test_body(),
            &test_atmosphere(),
            &test_vehicle(),
            MASTER_SEED_U64,
            STREAM_SEED_U64,
            Warp::X1,
            true,
            false,
            false,
        ) else {
            panic!("atmo capture must succeed")
        };
        assert_eq!(atmo.elements_valid_u8, 0_u8);
        assert_zero_f64(atmo.semi_major_axis_m_f64);
        assert_zero_f64(atmo.eccentricity_f64);
        assert_zero_f64(atmo.inclination_rad_f64);
    }

    #[test]
    fn capture_warp_and_frame_mapping() {
        let orbit = capture_orbit();
        assert_eq!(orbit.warp_code_u8, WARP_CODE_X100_U8);
        assert_eq!(orbit.drop_reason_u8, DROP_REASON_NONE_U8);
        assert_eq!(orbit.regime_u8, REGIME_ORBIT_U8);
        assert_eq!(orbit.frame_level_u8, FRAME_LEVEL_ORBIT_U8);
        assert_eq!(orbit.frame_depth_u8, FRAME_DEPTH_M1_U8);
        assert_eq!(orbit.frame_body_id_u32, BODY_ID_PLANET_U32);
        assert_eq!(orbit.parent_body_id_u32, BODY_ID_STAR_U32);
        assert_eq!(orbit.pick_body_id_u32, BODY_ID_NONE_U32);
        assert_eq!(orbit.pick_valid_u8, 0_u8);
        assert_eq!(orbit.mark_kind_u8, MARK_KIND_NONE_U8);
        let scheduler = Scheduler::default();
        let atmo_state = state_at_altitude(ATMO_ALTITUDE_M, 3_000.0);
        let Ok(entry) = capture_snapshot(
            &scheduler,
            &atmo_state,
            &test_body(),
            &test_atmosphere(),
            &test_vehicle(),
            MASTER_SEED_U64,
            STREAM_SEED_U64,
            Warp::X100,
            true,
            false,
            false,
        ) else {
            panic!("entry capture must succeed")
        };
        assert_eq!(entry.warp_code_u8, WARP_CODE_X1_U8);
        assert_eq!(entry.drop_reason_u8, DROP_REASON_ENTRY_U8);
        assert_eq!(entry.regime_u8, REGIME_ATMOSPHERE_U8);
        assert_eq!(entry.frame_level_u8, FRAME_LEVEL_ATMOSPHERE_U8);
        let Ok(approach) = capture_snapshot(
            &scheduler,
            &orbit_state(),
            &test_body(),
            &test_atmosphere(),
            &test_vehicle(),
            MASTER_SEED_U64,
            STREAM_SEED_U64,
            Warp::X1000,
            true,
            true,
            false,
        ) else {
            panic!("approach capture must succeed")
        };
        assert_eq!(approach.drop_reason_u8, DROP_REASON_APPROACH_U8);
        assert_eq!(approach.warp_code_u8, WARP_CODE_X1_U8);
        let Ok(alarm) = capture_snapshot(
            &scheduler,
            &orbit_state(),
            &test_body(),
            &test_atmosphere(),
            &test_vehicle(),
            MASTER_SEED_U64,
            STREAM_SEED_U64,
            Warp::X1000,
            true,
            false,
            true,
        ) else {
            panic!("alarm capture must succeed")
        };
        assert_eq!(alarm.drop_reason_u8, DROP_REASON_ALARM_U8);
    }

    #[test]
    fn capture_tick_and_aero_match_step_sample_rules() {
        let mut scheduler = Scheduler::default();
        scheduler.advance();
        scheduler.advance();
        let Ok(snapshot) = capture_snapshot(
            &scheduler,
            &orbit_state(),
            &test_body(),
            &test_atmosphere(),
            &test_vehicle(),
            MASTER_SEED_U64,
            STREAM_SEED_U64,
            Warp::X1,
            true,
            false,
            false,
        ) else {
            panic!("ticked capture must succeed")
        };
        assert_eq!(snapshot.tick_count_u64, 2_u64);
        assert!(libm::fabs(snapshot.elapsed_s_f64 - 0.1) < 1e-12);
        for value_f64 in snapshot.drag_mps2_f64 {
            assert_zero_f64(value_f64);
        }
        assert_zero_f64(snapshot.heat_flux_w_per_m2_f64);
        assert_zero_f64(snapshot.g_load_g_f64);
        assert_zero_f64(snapshot.pressure_pa_f64);
        assert_zero_f64(snapshot.density_kg_m3_f64);
        let dir_squared = snapshot.vel_dir_f64[0] * snapshot.vel_dir_f64[0]
            + snapshot.vel_dir_f64[1] * snapshot.vel_dir_f64[1]
            + snapshot.vel_dir_f64[2] * snapshot.vel_dir_f64[2];
        assert!(libm::fabs(dir_squared - 1.0) < DIR_TOL_F64);
        assert!(snapshot.mu_m3_s2_f64 > 0.0);
    }
}
