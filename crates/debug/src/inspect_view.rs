//! Read-only inspect view over `SimSnapshot` scalars.
//!
//! Plain-data copies of tick, clocks, seeds, warp, readouts with units, hash
//! short form, frame path, regime, and pick result. Code maps cite the
//! `crates/engine/src/inspect.rs` constant names. Never writes sim state.

use engine::regime::Regime;
use engine::warp::Warp;

#[cfg(feature = "dev-shell")]
use engine::inspect::SimSnapshot;

/// Orbit regime code, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `REGIME_ORBIT_U8`.
pub const INSPECT_REGIME_ORBIT_U8: u8 = 0;

/// Atmosphere regime code, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `REGIME_ATMOSPHERE_U8`.
pub const INSPECT_REGIME_ATMOSPHERE_U8: u8 = 1;

/// Surface regime code, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `REGIME_SURFACE_U8`.
pub const INSPECT_REGIME_SURFACE_U8: u8 = 2;

/// Orbital frame level, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `FRAME_LEVEL_ORBIT_U8`.
pub const INSPECT_FRAME_ORBIT_U8: u8 = 5;

/// Atmospheric frame level, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `FRAME_LEVEL_ATMOSPHERE_U8`.
pub const INSPECT_FRAME_ATMOSPHERE_U8: u8 = 6;

/// Surface frame level, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `FRAME_LEVEL_SURFACE_U8`.
pub const INSPECT_FRAME_SURFACE_U8: u8 = 7;

/// M1 frame depth, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `FRAME_DEPTH_M1_U8`.
pub const INSPECT_FRAME_DEPTH_M1_U8: u8 = 2;

/// X1 warp code, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `WARP_CODE_X1_U8`.
pub const INSPECT_WARP_X1_U8: u8 = 0;

/// X10 warp code, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `WARP_CODE_X10_U8`.
pub const INSPECT_WARP_X10_U8: u8 = 1;

/// X100 warp code, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `WARP_CODE_X100_U8`.
pub const INSPECT_WARP_X100_U8: u8 = 2;

/// X1000 warp code, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `WARP_CODE_X1000_U8`.
pub const INSPECT_WARP_X1000_U8: u8 = 3;

/// X10000 warp code, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `WARP_CODE_X10000_U8`.
pub const INSPECT_WARP_X10000_U8: u8 = 4;

/// Drop reason none or manual, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `DROP_REASON_NONE_U8`.
pub const INSPECT_DROP_NONE_U8: u8 = 0;

/// Drop reason atmospheric entry, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `DROP_REASON_ENTRY_U8`.
pub const INSPECT_DROP_ENTRY_U8: u8 = 1;

/// Drop reason approach, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `DROP_REASON_APPROACH_U8`.
pub const INSPECT_DROP_APPROACH_U8: u8 = 2;

/// Drop reason alarm, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `DROP_REASON_ALARM_U8`.
pub const INSPECT_DROP_ALARM_U8: u8 = 3;

/// None mark kind, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `MARK_KIND_NONE_U8`.
pub const INSPECT_MARK_NONE_U8: u8 = 0;

/// Star-point mark kind, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `MARK_KIND_STAR_POINT_U8`.
pub const INSPECT_MARK_STAR_POINT_U8: u8 = 1;

/// Planet-circle mark kind, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `MARK_KIND_PLANET_CIRCLE_U8`.
pub const INSPECT_MARK_PLANET_CIRCLE_U8: u8 = 2;

/// Atmosphere-layer mark kind, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `MARK_KIND_ATMO_LAYER_U8`.
pub const INSPECT_MARK_ATMO_LAYER_U8: u8 = 3;

/// Orbit-curve mark kind, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `MARK_KIND_ORBIT_CURVE_U8`.
pub const INSPECT_MARK_ORBIT_CURVE_U8: u8 = 4;

/// Trajectory-curve mark kind, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `MARK_KIND_TRAJECTORY_CURVE_U8`.
pub const INSPECT_MARK_TRAJECTORY_CURVE_U8: u8 = 5;

/// Ship-point mark kind, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `MARK_KIND_SHIP_POINT_U8`.
pub const INSPECT_MARK_SHIP_POINT_U8: u8 = 6;

/// Surface-grid mark kind, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `MARK_KIND_SURFACE_GRID_U8`.
pub const INSPECT_MARK_SURFACE_GRID_U8: u8 = 7;

/// Low 16-bit mask for short seed and hash display.
///
/// Source: display truncation only, issue 34 step 5.
pub const INSPECT_SHORT_MASK_U64: u64 = 0xFFFF;

/// Inspect-view range-check and code-map failures.
///
/// Returned for bad tolerance, bad clocks, and unknown codes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InspectViewError {
    /// Clock or tolerance value was non-finite.
    NonFinite {
        /// Rejected value.
        value_f64: f64,
    },
    /// Tolerance value was negative.
    Negative {
        /// Rejected value.
        value_f64: f64,
    },
    /// Warp code was outside `0` to `4`.
    InvalidWarpCode {
        /// Rejected code, dimensionless.
        code_u8: u8,
    },
    /// Drop-reason code was outside `0` to `3`.
    InvalidDropCode {
        /// Rejected code, dimensionless.
        code_u8: u8,
    },
    /// Regime code was outside `0` to `2`.
    InvalidRegimeCode {
        /// Rejected code, dimensionless.
        code_u8: u8,
    },
    /// Frame level was outside `5` to `7`.
    InvalidFrameLevel {
        /// Rejected level, dimensionless.
        code_u8: u8,
    },
    /// Mark kind was outside `0` to `7`.
    InvalidMarkKind {
        /// Rejected kind, dimensionless.
        code_u8: u8,
    },
}

impl core::fmt::Display for InspectViewError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NonFinite { value_f64 } => {
                write!(formatter, "non-finite inspect value: {value_f64}")
            }
            Self::Negative { value_f64 } => {
                write!(formatter, "negative inspect value: {value_f64}")
            }
            Self::InvalidWarpCode { code_u8 } => {
                write!(formatter, "invalid warp code: {code_u8}")
            }
            Self::InvalidDropCode { code_u8 } => {
                write!(formatter, "invalid drop-reason code: {code_u8}")
            }
            Self::InvalidRegimeCode { code_u8 } => {
                write!(formatter, "invalid regime code: {code_u8}")
            }
            Self::InvalidFrameLevel { code_u8 } => {
                write!(formatter, "invalid frame level: {code_u8}")
            }
            Self::InvalidMarkKind { code_u8 } => {
                write!(formatter, "invalid mark kind: {code_u8}")
            }
        }
    }
}

impl std::error::Error for InspectViewError {}

/// Read-only inspect view over snapshot scalars.
///
/// Plain data only; holds copies of snapshot fields plus the tap-pick
/// tolerance shown beside the pick result. Replaced wholesale on each
/// snapshot; exposes no write path to the sim.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InspectView {
    /// Tick count, dimensionless.
    tick_count_u64: u64,
    /// Elapsed sim time in seconds.
    elapsed_s_f64: f64,
    /// Ship epoch in seconds.
    ship_epoch_s_f64: f64,
    /// Master seed, dimensionless.
    master_seed_u64: u64,
    /// Stream seed, dimensionless.
    stream_seed_u64: u64,
    /// Snapshot hash, dimensionless.
    snapshot_hash_u64: u64,
    /// Altitude above surface in meters.
    altitude_m_f64: f64,
    /// Corotating speed in meters per second.
    speed_mps_f64: f64,
    /// Pressure in pascals.
    pressure_pa_f64: f64,
    /// Temperature in kelvin.
    temperature_k_f64: f64,
    /// Density in kilograms per cubic meter.
    density_kg_m3_f64: f64,
    /// Heat flux in watts per square meter.
    heat_flux_w_per_m2_f64: f64,
    /// G-load in g units, dimensionless.
    g_load_g_f64: f64,
    /// Semi-major axis in meters.
    semi_major_axis_m_f64: f64,
    /// Eccentricity, dimensionless.
    eccentricity_f64: f64,
    /// Inclination in radians.
    inclination_rad_f64: f64,
    /// Node longitude in radians.
    raan_rad_f64: f64,
    /// Argument of periapsis in radians.
    arg_periapsis_rad_f64: f64,
    /// Mean anomaly in radians.
    mean_anomaly_rad_f64: f64,
    /// Gravity parameter in cubic meters per second squared.
    mu_m3_s2_f64: f64,
    /// Pick altitude in meters.
    pick_altitude_m_f64: f64,
    /// Pick range in meters.
    pick_range_m_f64: f64,
    /// Frame body id, dimensionless.
    frame_body_id_u32: u32,
    /// Parent body id, dimensionless.
    parent_body_id_u32: u32,
    /// Pick body id, dimensionless.
    pick_body_id_u32: u32,
    /// Pick grid cell X, dimensionless.
    pick_cell_x_i32: i32,
    /// Pick grid cell Y, dimensionless.
    pick_cell_y_i32: i32,
    /// Warp code, dimensionless `0` to `4`.
    warp_code_u8: u8,
    /// Drop reason, dimensionless `0` to `3`.
    drop_reason_u8: u8,
    /// Regime code, dimensionless `0` to `2`.
    regime_u8: u8,
    /// Frame level, dimensionless `5` to `7`.
    frame_level_u8: u8,
    /// Frame depth, dimensionless.
    frame_depth_u8: u8,
    /// Elements valid flag, dimensionless `0` to `1`.
    elements_valid_u8: u8,
    /// Pick valid flag, dimensionless `0` to `1`.
    pick_valid_u8: u8,
    /// Mark kind, dimensionless `0` to `7`.
    mark_kind_u8: u8,
    /// Tap-pick point tolerance in points.
    pick_tolerance_pt_f32: f32,
}

impl InspectView {
    /// Build an empty view at tick zero.
    ///
    /// Codes default to valid nominals (`X1`, manual, orbit, orbital
    /// frame); readouts and pick fields default to zero.
    ///
    /// # Errors
    ///
    /// Returns [`InspectViewError`] when `pick_tolerance_pt_f32` is non-finite or negative.
    pub fn new(pick_tolerance_pt_f32: f32) -> Result<Self, InspectViewError> {
        Self::check_tolerance_pt_f32(pick_tolerance_pt_f32)?;
        Ok(Self {
            tick_count_u64: 0,
            elapsed_s_f64: 0.0,
            ship_epoch_s_f64: 0.0,
            master_seed_u64: 0,
            stream_seed_u64: 0,
            snapshot_hash_u64: 0,
            altitude_m_f64: 0.0,
            speed_mps_f64: 0.0,
            pressure_pa_f64: 0.0,
            temperature_k_f64: 0.0,
            density_kg_m3_f64: 0.0,
            heat_flux_w_per_m2_f64: 0.0,
            g_load_g_f64: 0.0,
            semi_major_axis_m_f64: 0.0,
            eccentricity_f64: 0.0,
            inclination_rad_f64: 0.0,
            raan_rad_f64: 0.0,
            arg_periapsis_rad_f64: 0.0,
            mean_anomaly_rad_f64: 0.0,
            mu_m3_s2_f64: 0.0,
            pick_altitude_m_f64: 0.0,
            pick_range_m_f64: 0.0,
            frame_body_id_u32: 0,
            parent_body_id_u32: 0,
            pick_body_id_u32: 0,
            pick_cell_x_i32: 0,
            pick_cell_y_i32: 0,
            warp_code_u8: INSPECT_WARP_X1_U8,
            drop_reason_u8: INSPECT_DROP_NONE_U8,
            regime_u8: INSPECT_REGIME_ORBIT_U8,
            frame_level_u8: INSPECT_FRAME_ORBIT_U8,
            frame_depth_u8: INSPECT_FRAME_DEPTH_M1_U8,
            elements_valid_u8: 0,
            pick_valid_u8: 0,
            mark_kind_u8: INSPECT_MARK_NONE_U8,
            pick_tolerance_pt_f32,
        })
    }

    /// Copy a snapshot into a new view without writing sim state.
    ///
    /// Copies scalars by value and maps codes; the snapshot is only read.
    /// Available only with the non-default `dev-shell` feature, when the
    /// engine snapshot view exists.
    ///
    /// # Errors
    ///
    /// Returns [`InspectViewError`] for a bad tolerance, a non-finite
    /// elapsed time, or an unknown warp, drop, regime, frame, or mark code.
    #[cfg(feature = "dev-shell")]
    pub fn from_snapshot(
        snapshot: &SimSnapshot,
        pick_tolerance_pt_f32: f32,
    ) -> Result<Self, InspectViewError> {
        Self::check_tolerance_pt_f32(pick_tolerance_pt_f32)?;
        if !snapshot.elapsed_s_f64.is_finite() {
            return Err(InspectViewError::NonFinite {
                value_f64: snapshot.elapsed_s_f64,
            });
        }
        Self::warp_for(snapshot.warp_code_u8)?;
        Self::drop_label_for(snapshot.drop_reason_u8)?;
        Self::regime_for(snapshot.regime_u8)?;
        Self::frame_label_for(snapshot.frame_level_u8)?;
        Self::mark_label_for(snapshot.mark_kind_u8)?;
        Ok(Self {
            tick_count_u64: snapshot.tick_count_u64,
            elapsed_s_f64: snapshot.elapsed_s_f64,
            ship_epoch_s_f64: snapshot.ship_epoch_s_f64,
            master_seed_u64: snapshot.master_seed_u64,
            stream_seed_u64: snapshot.stream_seed_u64,
            snapshot_hash_u64: snapshot.snapshot_hash_u64,
            altitude_m_f64: snapshot.altitude_m_f64,
            speed_mps_f64: snapshot.speed_mps_f64,
            pressure_pa_f64: snapshot.pressure_pa_f64,
            temperature_k_f64: snapshot.temperature_k_f64,
            density_kg_m3_f64: snapshot.density_kg_m3_f64,
            heat_flux_w_per_m2_f64: snapshot.heat_flux_w_per_m2_f64,
            g_load_g_f64: snapshot.g_load_g_f64,
            semi_major_axis_m_f64: snapshot.semi_major_axis_m_f64,
            eccentricity_f64: snapshot.eccentricity_f64,
            inclination_rad_f64: snapshot.inclination_rad_f64,
            raan_rad_f64: snapshot.raan_rad_f64,
            arg_periapsis_rad_f64: snapshot.arg_periapsis_rad_f64,
            mean_anomaly_rad_f64: snapshot.mean_anomaly_rad_f64,
            mu_m3_s2_f64: snapshot.mu_m3_s2_f64,
            pick_altitude_m_f64: snapshot.pick_altitude_m_f64,
            pick_range_m_f64: snapshot.pick_range_m_f64,
            frame_body_id_u32: snapshot.frame_body_id_u32,
            parent_body_id_u32: snapshot.parent_body_id_u32,
            pick_body_id_u32: snapshot.pick_body_id_u32,
            pick_cell_x_i32: snapshot.pick_cell_x_i32,
            pick_cell_y_i32: snapshot.pick_cell_y_i32,
            warp_code_u8: snapshot.warp_code_u8,
            drop_reason_u8: snapshot.drop_reason_u8,
            regime_u8: snapshot.regime_u8,
            frame_level_u8: snapshot.frame_level_u8,
            frame_depth_u8: snapshot.frame_depth_u8,
            elements_valid_u8: snapshot.elements_valid_u8,
            pick_valid_u8: snapshot.pick_valid_u8,
            mark_kind_u8: snapshot.mark_kind_u8,
            pick_tolerance_pt_f32,
        })
    }

    /// Map a snapshot warp code to a warp factor.
    ///
    /// # Errors
    ///
    /// Returns [`InspectViewError::InvalidWarpCode`] when `code_u8` exceeds `4`.
    pub const fn warp_for(code_u8: u8) -> Result<Warp, InspectViewError> {
        match code_u8 {
            INSPECT_WARP_X1_U8 => Ok(Warp::X1),
            INSPECT_WARP_X10_U8 => Ok(Warp::X10),
            INSPECT_WARP_X100_U8 => Ok(Warp::X100),
            INSPECT_WARP_X1000_U8 => Ok(Warp::X1000),
            INSPECT_WARP_X10000_U8 => Ok(Warp::X10000),
            _ => Err(InspectViewError::InvalidWarpCode { code_u8 }),
        }
    }

    /// Map a snapshot drop code to its reason label.
    ///
    /// # Errors
    ///
    /// Returns [`InspectViewError::InvalidDropCode`] when `code_u8` exceeds `3`.
    pub const fn drop_label_for(code_u8: u8) -> Result<&'static str, InspectViewError> {
        match code_u8 {
            INSPECT_DROP_NONE_U8 => Ok("manual"),
            INSPECT_DROP_ENTRY_U8 => Ok("entry"),
            INSPECT_DROP_APPROACH_U8 => Ok("approach"),
            INSPECT_DROP_ALARM_U8 => Ok("alarm"),
            _ => Err(InspectViewError::InvalidDropCode { code_u8 }),
        }
    }

    /// Map a snapshot regime code to its regime.
    ///
    /// # Errors
    ///
    /// Returns [`InspectViewError::InvalidRegimeCode`] when `code_u8` exceeds `2`.
    pub const fn regime_for(code_u8: u8) -> Result<Regime, InspectViewError> {
        match code_u8 {
            INSPECT_REGIME_ORBIT_U8 => Ok(Regime::Orbit),
            INSPECT_REGIME_ATMOSPHERE_U8 => Ok(Regime::Atmosphere),
            INSPECT_REGIME_SURFACE_U8 => Ok(Regime::Surface),
            _ => Err(InspectViewError::InvalidRegimeCode { code_u8 }),
        }
    }

    /// Map a snapshot regime code to its label.
    ///
    /// # Errors
    ///
    /// Returns [`InspectViewError::InvalidRegimeCode`] when `code_u8` exceeds `2`.
    pub const fn regime_label_for(code_u8: u8) -> Result<&'static str, InspectViewError> {
        match code_u8 {
            INSPECT_REGIME_ORBIT_U8 => Ok("orbit"),
            INSPECT_REGIME_ATMOSPHERE_U8 => Ok("atmosphere"),
            INSPECT_REGIME_SURFACE_U8 => Ok("surface"),
            _ => Err(InspectViewError::InvalidRegimeCode { code_u8 }),
        }
    }

    /// Map a snapshot frame level to its label.
    ///
    /// # Errors
    ///
    /// Returns [`InspectViewError::InvalidFrameLevel`] when `level_u8` is outside `5` to `7`.
    pub const fn frame_label_for(level_u8: u8) -> Result<&'static str, InspectViewError> {
        match level_u8 {
            INSPECT_FRAME_ORBIT_U8 => Ok("orbital"),
            INSPECT_FRAME_ATMOSPHERE_U8 => Ok("atmospheric"),
            INSPECT_FRAME_SURFACE_U8 => Ok("terrain"),
            _ => Err(InspectViewError::InvalidFrameLevel { code_u8: level_u8 }),
        }
    }

    /// Map a snapshot mark kind to its label.
    ///
    /// # Errors
    ///
    /// Returns [`InspectViewError::InvalidMarkKind`] when `kind_u8` exceeds `7`.
    pub const fn mark_label_for(kind_u8: u8) -> Result<&'static str, InspectViewError> {
        match kind_u8 {
            INSPECT_MARK_NONE_U8 => Ok("none"),
            INSPECT_MARK_STAR_POINT_U8 => Ok("star-point"),
            INSPECT_MARK_PLANET_CIRCLE_U8 => Ok("planet-circle"),
            INSPECT_MARK_ATMO_LAYER_U8 => Ok("atmo-layer"),
            INSPECT_MARK_ORBIT_CURVE_U8 => Ok("orbit-curve"),
            INSPECT_MARK_TRAJECTORY_CURVE_U8 => Ok("trajectory-curve"),
            INSPECT_MARK_SHIP_POINT_U8 => Ok("ship-point"),
            INSPECT_MARK_SURFACE_GRID_U8 => Ok("surface-grid"),
            _ => Err(InspectViewError::InvalidMarkKind { code_u8: kind_u8 }),
        }
    }

    /// Return the tick count, dimensionless.
    #[must_use]
    pub const fn tick_count_u64(&self) -> u64 {
        self.tick_count_u64
    }

    /// Return elapsed sim time in seconds.
    #[must_use]
    pub const fn elapsed_s_f64(&self) -> f64 {
        self.elapsed_s_f64
    }

    /// Return ship epoch in seconds.
    #[must_use]
    pub const fn ship_epoch_s_f64(&self) -> f64 {
        self.ship_epoch_s_f64
    }

    /// Return the master seed, dimensionless.
    #[must_use]
    pub const fn master_seed_u64(&self) -> u64 {
        self.master_seed_u64
    }

    /// Return the stream seed, dimensionless.
    #[must_use]
    pub const fn stream_seed_u64(&self) -> u64 {
        self.stream_seed_u64
    }

    /// Return the snapshot hash, dimensionless.
    #[must_use]
    pub const fn snapshot_hash_u64(&self) -> u64 {
        self.snapshot_hash_u64
    }

    /// Return the low 16 bits of the master seed.
    #[must_use]
    pub const fn seed_short_u16(&self) -> u16 {
        (self.master_seed_u64 & INSPECT_SHORT_MASK_U64) as u16
    }

    /// Return the low 16 bits of the snapshot hash.
    #[must_use]
    pub const fn hash_short_u16(&self) -> u16 {
        (self.snapshot_hash_u64 & INSPECT_SHORT_MASK_U64) as u16
    }

    /// Return altitude above surface in meters.
    #[must_use]
    pub const fn altitude_m_f64(&self) -> f64 {
        self.altitude_m_f64
    }

    /// Return corotating speed in meters per second.
    #[must_use]
    pub const fn speed_mps_f64(&self) -> f64 {
        self.speed_mps_f64
    }

    /// Return ambient pressure in pascals.
    #[must_use]
    pub const fn pressure_pa_f64(&self) -> f64 {
        self.pressure_pa_f64
    }

    /// Return ambient temperature in kelvin.
    #[must_use]
    pub const fn temperature_k_f64(&self) -> f64 {
        self.temperature_k_f64
    }

    /// Return air density in kilograms per cubic meter.
    #[must_use]
    pub const fn density_kg_m3_f64(&self) -> f64 {
        self.density_kg_m3_f64
    }

    /// Return heating proxy in watts per square meter.
    #[must_use]
    pub const fn heat_flux_w_per_m2_f64(&self) -> f64 {
        self.heat_flux_w_per_m2_f64
    }

    /// Return g-load in g units, dimensionless.
    #[must_use]
    pub const fn g_load_g_f64(&self) -> f64 {
        self.g_load_g_f64
    }

    /// Return the semi-major axis in meters.
    #[must_use]
    pub const fn semi_major_axis_m_f64(&self) -> f64 {
        self.semi_major_axis_m_f64
    }

    /// Return eccentricity, dimensionless.
    #[must_use]
    pub const fn eccentricity_f64(&self) -> f64 {
        self.eccentricity_f64
    }

    /// Return inclination in radians.
    #[must_use]
    pub const fn inclination_rad_f64(&self) -> f64 {
        self.inclination_rad_f64
    }

    /// Return node longitude in radians.
    #[must_use]
    pub const fn raan_rad_f64(&self) -> f64 {
        self.raan_rad_f64
    }

    /// Return argument of periapsis in radians.
    #[must_use]
    pub const fn arg_periapsis_rad_f64(&self) -> f64 {
        self.arg_periapsis_rad_f64
    }

    /// Return mean anomaly in radians.
    #[must_use]
    pub const fn mean_anomaly_rad_f64(&self) -> f64 {
        self.mean_anomaly_rad_f64
    }

    /// Return the gravity parameter in cubic meters per second squared.
    #[must_use]
    pub const fn mu_m3_s2_f64(&self) -> f64 {
        self.mu_m3_s2_f64
    }

    /// Report whether orbital elements are valid.
    #[must_use]
    pub const fn elements_valid(&self) -> bool {
        self.elements_valid_u8 != 0
    }

    /// Return the warp factor for the stored warp code.
    ///
    /// Unknown stored codes map to `X1`; use [`Self::warp_for`] to
    /// range-check a code explicitly.
    #[must_use]
    pub const fn warp_factor_f64(&self) -> f64 {
        match Self::warp_for(self.warp_code_u8) {
            Ok(warp) => warp.factor(),
            Err(_) => Warp::X1.factor(),
        }
    }

    /// Return the warp code, dimensionless.
    #[must_use]
    pub const fn warp_code_u8(&self) -> u8 {
        self.warp_code_u8
    }

    /// Return the auto-drop reason label.
    ///
    /// Unknown stored codes report `manual`; use
    /// [`Self::drop_label_for`] to range-check explicitly.
    #[must_use]
    pub const fn drop_label(&self) -> &'static str {
        match Self::drop_label_for(self.drop_reason_u8) {
            Ok(label) => label,
            Err(_) => "manual",
        }
    }

    /// Return the regime for the stored regime code.
    ///
    /// Unknown stored codes map to orbit; use [`Self::regime_for`] to
    /// range-check a code explicitly.
    #[must_use]
    pub const fn regime(&self) -> Regime {
        match Self::regime_for(self.regime_u8) {
            Ok(regime) => regime,
            Err(_) => Regime::Orbit,
        }
    }

    /// Return the regime label for the stored code.
    #[must_use]
    pub const fn regime_label(&self) -> &'static str {
        match Self::regime_label_for(self.regime_u8) {
            Ok(label) => label,
            Err(_) => "orbit",
        }
    }

    /// Return the frame level label for the stored level.
    #[must_use]
    pub const fn frame_label(&self) -> &'static str {
        match Self::frame_label_for(self.frame_level_u8) {
            Ok(label) => label,
            Err(_) => "orbital",
        }
    }

    /// Return the frame body id, dimensionless.
    #[must_use]
    pub const fn frame_body_id_u32(&self) -> u32 {
        self.frame_body_id_u32
    }

    /// Return the parent body id, dimensionless.
    #[must_use]
    pub const fn parent_body_id_u32(&self) -> u32 {
        self.parent_body_id_u32
    }

    /// Return the frame depth, dimensionless.
    #[must_use]
    pub const fn frame_depth_u8(&self) -> u8 {
        self.frame_depth_u8
    }

    /// Report whether the pick result is valid.
    #[must_use]
    pub const fn pick_valid(&self) -> bool {
        self.pick_valid_u8 != 0
    }

    /// Return the pick body id, dimensionless.
    #[must_use]
    pub const fn pick_body_id_u32(&self) -> u32 {
        self.pick_body_id_u32
    }

    /// Return pick altitude in meters.
    #[must_use]
    pub const fn pick_altitude_m_f64(&self) -> f64 {
        self.pick_altitude_m_f64
    }

    /// Return pick range in meters.
    #[must_use]
    pub const fn pick_range_m_f64(&self) -> f64 {
        self.pick_range_m_f64
    }

    /// Return the pick grid cell as `(x, y)`, dimensionless.
    #[must_use]
    pub const fn pick_cell(&self) -> (i32, i32) {
        (self.pick_cell_x_i32, self.pick_cell_y_i32)
    }

    /// Return the picked mark label for the stored kind.
    #[must_use]
    pub const fn mark_label(&self) -> &'static str {
        match Self::mark_label_for(self.mark_kind_u8) {
            Ok(label) => label,
            Err(_) => "none",
        }
    }

    /// Return the tap-pick point tolerance in points.
    #[must_use]
    pub const fn pick_tolerance_pt_f32(&self) -> f32 {
        self.pick_tolerance_pt_f32
    }

    /// Draw the inspector rows with value, unit, and source.
    ///
    /// Immediate-mode widgets only; creates no renderer. Step 5 owns
    /// renderer creation. Available only with `dev-shell`.
    #[cfg(feature = "dev-shell")]
    pub fn draw(&self, ui: &mut egui::Ui) {
        ui.label(format!(
            "inspect tick={tick} elapsed_s={elapsed:.2} seed={seed:04x} hash={hash:04x}",
            tick = self.tick_count_u64,
            elapsed = self.elapsed_s_f64,
            seed = self.seed_short_u16(),
            hash = self.hash_short_u16()
        ));
        ui.label(format!(
            "warp={factor}x drop={drop} regime={regime} frame=Lv{level} {frame} body={body} parent={parent} depth={depth}",
            factor = self.warp_factor_f64(),
            drop = self.drop_label(),
            regime = self.regime_label(),
            level = self.frame_level_u8,
            frame = self.frame_label(),
            body = self.frame_body_id_u32,
            parent = self.parent_body_id_u32,
            depth = self.frame_depth_u8
        ));
        ui.label(format!(
            "altitude_m={alt:.1} (trajectory) speed_mps={speed:.1} (trajectory)",
            alt = self.altitude_m_f64,
            speed = self.speed_mps_f64
        ));
        ui.label(format!(
            "pressure_pa={p:.3} (atmo) temperature_k={t:.2} (atmo) density_kg_m3={d:.6} (atmo)",
            p = self.pressure_pa_f64,
            t = self.temperature_k_f64,
            d = self.density_kg_m3_f64
        ));
        ui.label(format!(
            "heat_flux_w_m2={heat:.1} (trajectory) g_load_g={g:.2} (trajectory)",
            heat = self.heat_flux_w_per_m2_f64,
            g = self.g_load_g_f64
        ));
        ui.label(format!(
            "elements valid={valid} axis_m={axis:.1} (orbits) ecc={ecc:.4} (orbits)",
            valid = self.elements_valid(),
            axis = self.semi_major_axis_m_f64,
            ecc = self.eccentricity_f64
        ));
        ui.label(format!(
            "pick valid={valid} mark={mark} body={body} alt_m={alt:.1} range_m={range:.1} cell=({cx},{cy}) tol_pt={tol:.1}",
            valid = self.pick_valid(),
            mark = self.mark_label(),
            body = self.pick_body_id_u32,
            alt = self.pick_altitude_m_f64,
            range = self.pick_range_m_f64,
            cx = self.pick_cell_x_i32,
            cy = self.pick_cell_y_i32,
            tol = self.pick_tolerance_pt_f32
        ));
    }

    /// Check a tap-pick tolerance in points.
    ///
    /// # Errors
    ///
    /// Returns [`InspectViewError`] when `tolerance_pt_f32` is non-finite or negative.
    const fn check_tolerance_pt_f32(tolerance_pt_f32: f32) -> Result<(), InspectViewError> {
        let value_f64 = tolerance_pt_f32 as f64;
        if !value_f64.is_finite() {
            return Err(InspectViewError::NonFinite { value_f64 });
        }
        if value_f64 < 0.0 {
            return Err(InspectViewError::Negative { value_f64 });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMOKE_TOLERANCE_PT_F32: f32 = 8.0;
    const NEGATIVE_TOLERANCE_PT_F32: f32 = -1.0;
    const UNKNOWN_CODE_U8: u8 = 9;
    const UNKNOWN_LEVEL_U8: u8 = 4;
    const EXPECTED_X100_F64: f64 = 100.0;
    const FRACTION_TOL_F64: f64 = 1e-12;

    #[test]
    fn empty_view_uses_nominal_defaults() {
        let Ok(view) = InspectView::new(SMOKE_TOLERANCE_PT_F32) else {
            panic!("smoke view must build")
        };
        assert_eq!(view.tick_count_u64(), 0);
        assert!((view.warp_factor_f64() - 1.0).abs() < FRACTION_TOL_F64);
        assert_eq!(view.warp_code_u8(), INSPECT_WARP_X1_U8);
        assert_eq!(view.drop_label(), "manual");
        assert_eq!(view.regime(), Regime::Orbit);
        assert_eq!(view.regime_label(), "orbit");
        assert_eq!(view.frame_label(), "orbital");
        assert_eq!(view.frame_body_id_u32(), 0);
        assert_eq!(view.parent_body_id_u32(), 0);
        assert_eq!(view.frame_depth_u8(), INSPECT_FRAME_DEPTH_M1_U8);
        assert!(!view.elements_valid());
        assert!(!view.pick_valid());
        assert_eq!(view.mark_label(), "none");
        assert_eq!(view.pick_cell(), (0, 0));
        assert!((view.pick_tolerance_pt_f32() - SMOKE_TOLERANCE_PT_F32).abs() < f32::EPSILON);
        assert_eq!(view.seed_short_u16(), 0);
        assert_eq!(view.hash_short_u16(), 0);
    }

    #[test]
    fn new_rejects_bad_tolerance() {
        assert!(matches!(
            InspectView::new(f32::NAN),
            Err(InspectViewError::NonFinite { .. })
        ));
        assert!(matches!(
            InspectView::new(NEGATIVE_TOLERANCE_PT_F32),
            Err(InspectViewError::Negative { .. })
        ));
    }

    #[test]
    fn code_maps_cover_all_snapshot_codes() {
        let Ok(warp) = InspectView::warp_for(INSPECT_WARP_X100_U8) else {
            panic!("x100 code must map")
        };
        assert!((warp.factor() - EXPECTED_X100_F64).abs() < FRACTION_TOL_F64);
        assert!(matches!(
            InspectView::warp_for(UNKNOWN_CODE_U8),
            Err(InspectViewError::InvalidWarpCode { .. })
        ));
        let Ok(drop) = InspectView::drop_label_for(INSPECT_DROP_ENTRY_U8) else {
            panic!("entry code must map")
        };
        assert_eq!(drop, "entry");
        assert!(matches!(
            InspectView::drop_label_for(UNKNOWN_CODE_U8),
            Err(InspectViewError::InvalidDropCode { .. })
        ));
        let Ok(regime) = InspectView::regime_for(INSPECT_REGIME_SURFACE_U8) else {
            panic!("surface code must map")
        };
        assert_eq!(regime, Regime::Surface);
        let Ok(regime_label) = InspectView::regime_label_for(INSPECT_REGIME_ATMOSPHERE_U8) else {
            panic!("atmosphere code must map")
        };
        assert_eq!(regime_label, "atmosphere");
        assert!(matches!(
            InspectView::regime_for(UNKNOWN_CODE_U8),
            Err(InspectViewError::InvalidRegimeCode { .. })
        ));
        let Ok(frame) = InspectView::frame_label_for(INSPECT_FRAME_SURFACE_U8) else {
            panic!("surface frame must map")
        };
        assert_eq!(frame, "terrain");
        assert!(matches!(
            InspectView::frame_label_for(UNKNOWN_LEVEL_U8),
            Err(InspectViewError::InvalidFrameLevel { .. })
        ));
        let Ok(mark) = InspectView::mark_label_for(INSPECT_MARK_SHIP_POINT_U8) else {
            panic!("ship mark must map")
        };
        assert_eq!(mark, "ship-point");
        assert!(matches!(
            InspectView::mark_label_for(UNKNOWN_CODE_U8),
            Err(InspectViewError::InvalidMarkKind { .. })
        ));
    }

    #[cfg(feature = "dev-shell")]
    #[test]
    fn from_snapshot_copies_scalars_read_only() {
        let snapshot = SimSnapshot {
            tick_count_u64: 42,
            elapsed_s_f64: 2.1,
            ship_epoch_s_f64: 2.1,
            master_seed_u64: 0x1234_ABCD_5678_EF90,
            stream_seed_u64: 0x1319_8A2E_0370_7344,
            snapshot_hash_u64: 0xDEAD_BEEF_0000_4321,
            position_m_f64: [3_639_500.0, 0.0, 0.0],
            velocity_mps_f64: [0.0, 3_400.0, 0.0],
            drag_mps2_f64: [0.0, 0.0, 0.0],
            vel_dir_f64: [0.0, 1.0, 0.0],
            altitude_m_f64: 250_000.0,
            speed_mps_f64: 3_400.0,
            pressure_pa_f64: 0.0,
            temperature_k_f64: 210.0,
            density_kg_m3_f64: 0.0,
            heat_flux_w_per_m2_f64: 0.0,
            g_load_g_f64: 0.0,
            semi_major_axis_m_f64: 3_639_500.0,
            eccentricity_f64: 0.01,
            inclination_rad_f64: 0.3,
            raan_rad_f64: 0.7,
            arg_periapsis_rad_f64: 0.5,
            mean_anomaly_rad_f64: 1.0,
            mu_m3_s2_f64: 4.282_837e13,
            pick_altitude_m_f64: 249_000.0,
            pick_range_m_f64: 1_000.0,
            frame_body_id_u32: 1,
            parent_body_id_u32: 0,
            pick_body_id_u32: 1,
            pick_cell_x_i32: 3,
            pick_cell_y_i32: -2,
            warp_code_u8: INSPECT_WARP_X100_U8,
            drop_reason_u8: INSPECT_DROP_NONE_U8,
            warp_flags_u8: 0,
            regime_u8: INSPECT_REGIME_ORBIT_U8,
            frame_level_u8: INSPECT_FRAME_ORBIT_U8,
            frame_depth_u8: INSPECT_FRAME_DEPTH_M1_U8,
            elements_valid_u8: 1,
            pick_valid_u8: 1,
            mark_kind_u8: INSPECT_MARK_SHIP_POINT_U8,
            _pad_u8: [0_u8; 3],
        };
        let Ok(view) = InspectView::from_snapshot(&snapshot, SMOKE_TOLERANCE_PT_F32) else {
            panic!("smoke snapshot must map")
        };
        assert_eq!(view.tick_count_u64(), 42);
        assert!((view.elapsed_s_f64() - 2.1).abs() < FRACTION_TOL_F64);
        assert!((view.warp_factor_f64() - EXPECTED_X100_F64).abs() < FRACTION_TOL_F64);
        assert_eq!(view.regime(), Regime::Orbit);
        assert_eq!(view.frame_label(), "orbital");
        assert_eq!(view.seed_short_u16(), 0xEF90);
        assert_eq!(view.hash_short_u16(), 0x4321);
        assert!(view.elements_valid());
        assert!(view.pick_valid());
        assert_eq!(view.pick_body_id_u32(), 1);
        assert_eq!(view.mark_label(), "ship-point");
        assert_eq!(view.pick_cell(), (3, -2));
        assert!((view.altitude_m_f64() - 250_000.0).abs() < FRACTION_TOL_F64);
        assert!((view.semi_major_axis_m_f64() - 3_639_500.0).abs() < 1e-6);
        assert_eq!(snapshot.tick_count_u64, 42);
        let mut bad_warp = snapshot;
        bad_warp.warp_code_u8 = UNKNOWN_CODE_U8;
        assert!(matches!(
            InspectView::from_snapshot(&bad_warp, SMOKE_TOLERANCE_PT_F32),
            Err(InspectViewError::InvalidWarpCode { .. })
        ));
        let mut bad_elapsed = snapshot;
        bad_elapsed.elapsed_s_f64 = f64::NAN;
        assert!(matches!(
            InspectView::from_snapshot(&bad_elapsed, SMOKE_TOLERANCE_PT_F32),
            Err(InspectViewError::NonFinite { .. })
        ));
    }
}
