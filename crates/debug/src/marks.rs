//! Abstract-marks canvas over read-only snapshot copies.
//!
//! [`MarksView`] paints star, planet, atmosphere, orbit, trajectory,
//! ship-point, and surface-grid marks with the egui painter through the
//! step-1 [`Camera2D`](engine::render::Camera2D). Snapshots arrive by copy in
//! [`MarksView::push_snapshot`] and are never written back to the sim.
//! History and scratch buffers pre-size at open and reuse after warmup,
//! per the shell exemption in `docs/tech/debug.md` section 8. The
//! ticker-only preset skips the canvas; every other preset paints it.
//! Gated dev-shell only; the module gate lives in `main.rs`.

use std::collections::VecDeque;

use engine::body::{BodyParams, MARS_MASS_KG, MARS_RADIUS_M, MARS_ROTATION_PERIOD_S};
use engine::inspect::{REGIME_ORBIT_U8, REGIME_SURFACE_U8, SimSnapshot};
use engine::render::Camera2D;
use engine::trajectory::RAILS_ALTITUDE_M;
use engine::units::{Kilograms, Meters, Seconds};
use glam::DVec3;

use crate::layout::{BufferPlan, PanelVisibility};

/// Trajectory history capacity in entries, pre-sized at open.
///
/// Source: [`BufferPlan::shell_default`] plot history (`docs/tech/debug.md` 8).
pub const MARKS_HISTORY_CAP_ENTRIES_USIZE: usize =
    BufferPlan::shell_default().plot_history_entries_usize();

/// Default canvas viewport width and height in points before sync.
///
/// Fallback until the first draw syncs to the canvas rect; source is the
/// headless test canvas in issue 52 step 2.
pub const MARKS_DEFAULT_VIEWPORT_PX_F32: [f32; 2] = [800.0, 600.0];

/// Open-camera ship altitude in meters for the orbit-fit preset.
///
/// Covers the rails handoff band before any snapshot arrives; source is
/// [`RAILS_ALTITUDE_M`] in `engine::trajectory`.
pub const MARKS_OPEN_ALTITUDE_M_F64: f64 = RAILS_ALTITUDE_M;

/// Orbit/entry handoff altitude in meters for view auto-select.
///
/// Equals the rails boundary; orbit views fit everything above it.
/// Source: [`RAILS_ALTITUDE_M`] in `engine::trajectory`.
pub const VIEW_ORBIT_ENTRY_ALTITUDE_M_F64: f64 = RAILS_ALTITUDE_M;

/// Entry/surface handoff altitude in meters for view auto-select.
///
/// Source: issue 52 step 3 brief (surface view below 10 km).
pub const VIEW_ENTRY_SURFACE_ALTITUDE_M_F64: f64 = 10_000.0;

/// Hysteresis half-band in meters around each view handoff.
///
/// Leaving the current view needs the altitude past the handoff plus
/// this band, so boundary noise never flickers views. Source: issue 52
/// step 3 brief (2 km band).
pub const VIEW_HYSTERESIS_BAND_M_F64: f64 = 2_000.0;

/// Manual zoom-out limit as a scale ratio, dimensionless.
///
/// Source: issue 52 step 3 brief, hand-placed manual zoom clamp.
pub const VIEW_ZOOM_MIN_RATIO_F64: f64 = 0.25;

/// Manual zoom-in limit as a scale ratio, dimensionless.
///
/// Source: issue 52 step 3 brief, hand-placed manual zoom clamp.
pub const VIEW_ZOOM_MAX_RATIO_F64: f64 = 4.0;

/// Default manual zoom as a scale ratio, dimensionless.
///
/// Unity leaves the preset camera untouched. Source: issue 52 step 3.
pub const VIEW_ZOOM_DEFAULT_RATIO_F64: f64 = 1.0;

/// Compile-time ordering check for the manual zoom clamp range.
const _: () = {
    assert!(VIEW_ZOOM_MIN_RATIO_F64 < VIEW_ZOOM_DEFAULT_RATIO_F64);
    assert!(VIEW_ZOOM_DEFAULT_RATIO_F64 < VIEW_ZOOM_MAX_RATIO_F64);
};

/// Circle polyline resolution in segments per full circle, dimensionless.
///
/// Source: issue 52 step 2, hand-placed display resolution.
pub const CIRCLE_SEGMENT_COUNT_USIZE: usize = 128;

/// Circle polyline resolution as float for step math, dimensionless.
///
/// Kept in sync with [`CIRCLE_SEGMENT_COUNT_USIZE`]; see the counts test.
pub const CIRCLE_SEGMENT_COUNT_F64: f64 = 128.0;

/// Circle step in radians between polyline vertices.
///
/// Source: one full circle over [`CIRCLE_SEGMENT_COUNT_F64`].
pub const CIRCLE_STEP_RAD_F64: f64 = core::f64::consts::TAU / CIRCLE_SEGMENT_COUNT_F64;

/// Lower atmosphere display-band altitude in meters.
///
/// Source: issue 52 step 2 display bands (50, 100, 120 km).
pub const ATMO_LOW_ALTITUDE_M_F64: f64 = 50_000.0;

/// Upper atmosphere display-band altitude in meters.
///
/// Source: issue 52 step 2 display bands (50, 100, 120 km).
pub const ATMO_MID_ALTITUDE_M_F64: f64 = 100_000.0;

/// Trajectory decimation stride in samples, dimensionless.
///
/// Every Nth history point paints plus the latest point, keeping at most
/// about 512 trajectory shapes per full buffer. Source: issue 52 step 2.
pub const TRAJECTORY_STRIDE_SAMPLES_USIZE: usize = 4;

/// Surface grid tick count in ticks, dimensionless.
///
/// Odd so one tick lands on the sub-ship point. Source: issue 52 step 2.
pub const GRID_TICK_COUNT_USIZE: usize = 21;

/// Surface grid half tick count in ticks, dimensionless.
///
/// Half of [`GRID_TICK_COUNT_USIZE`] minus the center tick; see sync test.
pub const GRID_HALF_TICKS_F64: f64 = 10.0;

/// Surface grid angular step in radians between ticks.
///
/// Source: issue 52 step 2, hand-placed.
pub const GRID_TICK_STEP_RAD_F64: f64 = 0.006;

/// Surface grid tick length in meters, radial outward.
///
/// Source: issue 52 step 2, hand-placed.
pub const GRID_TICK_LENGTH_M_F64: f64 = 2_000.0;

/// Planet mark stroke width in points.
///
/// Source: issue 52 step 2, hand-placed.
pub const PLANET_STROKE_WIDTH_PT_F32: f32 = 1.5;

/// Atmosphere mark stroke width in points.
///
/// Source: issue 52 step 2, hand-placed.
pub const ATMO_STROKE_WIDTH_PT_F32: f32 = 1.0;

/// Trajectory mark stroke width in points.
///
/// Source: issue 52 step 2, hand-placed.
pub const TRAJECTORY_STROKE_WIDTH_PT_F32: f32 = 1.5;

/// Surface grid mark stroke width in points.
///
/// Source: issue 52 step 2, hand-placed.
pub const GRID_STROKE_WIDTH_PT_F32: f32 = 1.0;

/// Ship point radius in points.
///
/// Source: issue 52 step 2, hand-placed.
pub const SHIP_POINT_RADIUS_PT_F32: f32 = 4.0;

/// Star point radius in points.
///
/// Source: issue 52 step 4 brief, hand-placed display size.
pub const STAR_POINT_RADIUS_PT_F32: f32 = 3.0;

/// Orbit curve stroke width in points.
///
/// Source: issue 52 step 4 brief, hand-placed.
pub const ORBIT_STROKE_WIDTH_PT_F32: f32 = 1.0;

/// Star display offset on `X` in meters.
///
/// Hand-placed backdrop offset inside the orbit-fit view; the star is a
/// Lv1 to Lv2 backdrop asset, not simulation. Source: issue 52 step 4.
pub const STAR_OFFSET_X_M_F64: f64 = -2_000_000.0;

/// Star display offset on `Y` in meters.
///
/// Hand-placed backdrop offset inside the orbit-fit view; the star is a
/// Lv1 to Lv2 backdrop asset, not simulation. Source: issue 52 step 4.
pub const STAR_OFFSET_Y_M_F64: f64 = 1_500_000.0;

/// Planet mark color for dark backgrounds.
///
/// Source: `docs/tech/debug.md` section 7 theme mapping.
pub const PLANET_MARK_COLOR: egui::Color32 = egui::Color32::LIGHT_BLUE;

/// Atmosphere mark color for dark backgrounds.
///
/// Source: `docs/tech/debug.md` section 7 theme mapping.
pub const ATMO_MARK_COLOR: egui::Color32 = egui::Color32::GRAY;

/// Trajectory mark color for dark backgrounds.
///
/// Source: `docs/tech/debug.md` section 7 theme mapping.
pub const TRAJECTORY_MARK_COLOR: egui::Color32 = egui::Color32::GOLD;

/// Ship mark color for dark backgrounds.
///
/// Source: `docs/tech/debug.md` section 7 theme mapping.
pub const SHIP_MARK_COLOR: egui::Color32 = egui::Color32::WHITE;

/// Star mark color for dark backgrounds.
///
/// Source: `docs/tech/debug.md` section 7 theme mapping.
pub const STAR_MARK_COLOR: egui::Color32 = egui::Color32::YELLOW;

/// Orbit mark color for dark backgrounds.
///
/// Source: `docs/tech/debug.md` section 7 theme mapping.
pub const ORBIT_MARK_COLOR: egui::Color32 = egui::Color32::LIGHT_GREEN;

/// Surface grid mark color for dark backgrounds.
///
/// Source: `docs/tech/debug.md` section 7 theme mapping.
pub const GRID_MARK_COLOR: egui::Color32 = egui::Color32::DARK_GRAY;

/// Marks-view failures from camera or radius checks.
///
/// Mirrors the reachable [`Camera2D`](engine::render::Camera2D) build
/// failures as copyable scalars so [`ShellError`](crate::shell::ShellError)
/// keeps its `Copy` convention; no detail is lost in the mapping. Never
/// a sim write failure; the view is read-only.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MarksError {
    /// Camera value was not finite.
    NonFinite {
        /// Rejected value.
        value_f64: f64,
    },
    /// Camera scale was not positive and finite in points per meter.
    InvalidScale {
        /// Rejected scale in points per meter.
        scale_px_per_m_f64: f64,
    },
    /// Camera viewport was not positive and finite in points.
    InvalidViewport {
        /// Rejected width in points.
        width_px_f32: f32,
        /// Rejected height in points.
        height_px_f32: f32,
    },
    /// Open altitude was not finite in meters.
    InvalidAltitude {
        /// Rejected altitude in meters.
        altitude_m_f64: f64,
    },
    /// Batch buffers had different lengths, dimensionless counts.
    BatchLengthMismatch {
        /// World buffer length, dimensionless.
        worlds_len_usize: usize,
        /// Screen buffer length, dimensionless.
        screens_len_usize: usize,
    },
    /// Body radius was non-finite or not positive in meters.
    InvalidRadius {
        /// Rejected radius in meters.
        radius_m_f64: f64,
    },
}

impl core::fmt::Display for MarksError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NonFinite { value_f64 } => {
                write!(formatter, "marks non-finite value: {value_f64}")
            }
            Self::InvalidScale { scale_px_per_m_f64 } => {
                write!(formatter, "marks invalid scale: {scale_px_per_m_f64}")
            }
            Self::InvalidViewport {
                width_px_f32,
                height_px_f32,
            } => {
                write!(
                    formatter,
                    "marks invalid viewport: {width_px_f32} x {height_px_f32}"
                )
            }
            Self::InvalidAltitude { altitude_m_f64 } => {
                write!(formatter, "marks invalid altitude: {altitude_m_f64}")
            }
            Self::BatchLengthMismatch {
                worlds_len_usize,
                screens_len_usize,
            } => {
                write!(
                    formatter,
                    "marks batch mismatch: {worlds_len_usize} vs {screens_len_usize}"
                )
            }
            Self::InvalidRadius { radius_m_f64 } => {
                write!(formatter, "marks invalid radius in meters: {radius_m_f64}")
            }
        }
    }
}

impl std::error::Error for MarksError {}

impl From<engine::render::RenderError> for MarksError {
    /// Convert a camera build failure without losing detail.
    fn from(source: engine::render::RenderError) -> Self {
        use engine::render::RenderError;
        match source {
            RenderError::NonFinite { value_f64 } => Self::NonFinite { value_f64 },
            RenderError::InvalidScale { scale_px_per_m_f64 } => {
                Self::InvalidScale { scale_px_per_m_f64 }
            }
            RenderError::InvalidViewport {
                width_px_f32,
                height_px_f32,
            } => Self::InvalidViewport {
                width_px_f32,
                height_px_f32,
            },
            RenderError::InvalidAltitude { altitude_m_f64 } => {
                Self::InvalidAltitude { altitude_m_f64 }
            }
            RenderError::BatchLengthMismatch {
                worlds_len_usize,
                screens_len_usize,
            } => Self::BatchLengthMismatch {
                worlds_len_usize,
                screens_len_usize,
            },
        }
    }
}

/// Per-class overlay toggles for the marks canvas.
///
/// Mirrors the overlay list in `docs/tech/debug.md` section 4.2. Toggles
/// are safe views and never taint the run.
#[expect(
    clippy::struct_excessive_bools,
    reason = "one toggle per mark class per debug.md 4.2; bound directly to checkboxes"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverlayFlags {
    /// Draw the star backdrop point.
    pub star_bool: bool,
    /// Draw the planet circle mark.
    pub planet_bool: bool,
    /// Draw the atmosphere layer circles.
    pub atmosphere_bool: bool,
    /// Draw the orbit curve circle.
    pub orbit_bool: bool,
    /// Draw the decimated trajectory polyline.
    pub trajectory_bool: bool,
    /// Draw the ship point.
    pub ship_bool: bool,
    /// Draw the surface grid ticks.
    pub grid_bool: bool,
}

impl OverlayFlags {
    /// Return every overlay class switched on.
    #[must_use]
    pub const fn all() -> Self {
        Self {
            star_bool: true,
            planet_bool: true,
            atmosphere_bool: true,
            orbit_bool: true,
            trajectory_bool: true,
            ship_bool: true,
            grid_bool: true,
        }
    }

    /// Return every overlay class switched off.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            star_bool: false,
            planet_bool: false,
            atmosphere_bool: false,
            orbit_bool: false,
            trajectory_bool: false,
            ship_bool: false,
            grid_bool: false,
        }
    }

    /// Set the star toggle without touching sim state.
    ///
    /// Safe view only; never taints the run and needs no pause.
    pub const fn set_star_bool(&mut self, show_bool: bool) {
        self.star_bool = show_bool;
    }

    /// Set the planet toggle without touching sim state.
    ///
    /// Safe view only; never taints the run and needs no pause.
    pub const fn set_planet_bool(&mut self, show_bool: bool) {
        self.planet_bool = show_bool;
    }

    /// Set the atmosphere toggle without touching sim state.
    ///
    /// Safe view only; never taints the run and needs no pause.
    pub const fn set_atmosphere_bool(&mut self, show_bool: bool) {
        self.atmosphere_bool = show_bool;
    }

    /// Set the orbit toggle without touching sim state.
    ///
    /// Safe view only; never taints the run and needs no pause.
    pub const fn set_orbit_bool(&mut self, show_bool: bool) {
        self.orbit_bool = show_bool;
    }

    /// Set the trajectory toggle without touching sim state.
    ///
    /// Safe view only; never taints the run and needs no pause.
    pub const fn set_trajectory_bool(&mut self, show_bool: bool) {
        self.trajectory_bool = show_bool;
    }

    /// Set the ship toggle without touching sim state.
    ///
    /// Safe view only; never taints the run and needs no pause.
    pub const fn set_ship_bool(&mut self, show_bool: bool) {
        self.ship_bool = show_bool;
    }

    /// Set the grid toggle without touching sim state.
    ///
    /// Safe view only; never taints the run and needs no pause.
    pub const fn set_grid_bool(&mut self, show_bool: bool) {
        self.grid_bool = show_bool;
    }
}

/// Marks-canvas zoom-to-fit view over the descent regimes.
///
/// Mirrors the debug-camera list in `docs/tech/debug.md` section 4.2.
/// Auto-select follows regime plus altitude with hysteresis; a manual
/// override pins one view until cleared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Whole-orbit fit centered on the body center.
    OrbitFit,
    /// Fixed 0 to 120 km entry corridor on `+X`.
    EntryCorridor,
    /// Fixed touchdown patch on the surface at `+X`.
    SurfaceGrid,
}

impl ViewMode {
    /// All zoom-to-fit views in fit order.
    pub const ALL: [Self; 3] = [Self::OrbitFit, Self::EntryCorridor, Self::SurfaceGrid];

    /// Return the short view label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::OrbitFit => "orbit-fit",
            Self::EntryCorridor => "entry-corridor",
            Self::SurfaceGrid => "surface-grid",
        }
    }
}

/// Select the marks-canvas view from regime plus altitude with hysteresis.
///
/// Surface regime always selects [`ViewMode::SurfaceGrid`] and orbit
/// regime always selects [`ViewMode::OrbitFit`]; otherwise altitude picks
/// with a [`VIEW_HYSTERESIS_BAND_M_F64`] band around each handoff so
/// boundary noise never flickers. Altitudes above the orbit handoff pick
/// orbit and altitudes below the surface handoff pick surface even when
/// the regime code disagrees. Unknown regime codes fall back to altitude;
/// non-finite altitudes keep the current view, so bad samples never jump.
/// Thresholds derive from [`VIEW_ORBIT_ENTRY_ALTITUDE_M_F64`] and
/// [`VIEW_ENTRY_SURFACE_ALTITUDE_M_F64`].
#[must_use]
pub const fn auto_view_mode(regime_u8: u8, altitude_m_f64: f64, current: ViewMode) -> ViewMode {
    if regime_u8 == REGIME_SURFACE_U8 {
        return ViewMode::SurfaceGrid;
    }
    if regime_u8 == REGIME_ORBIT_U8 {
        return ViewMode::OrbitFit;
    }
    if !altitude_m_f64.is_finite() {
        return current;
    }
    match current {
        ViewMode::OrbitFit => {
            if altitude_m_f64 < VIEW_ENTRY_SURFACE_ALTITUDE_M_F64 - VIEW_HYSTERESIS_BAND_M_F64 {
                ViewMode::SurfaceGrid
            } else if altitude_m_f64 < VIEW_ORBIT_ENTRY_ALTITUDE_M_F64 - VIEW_HYSTERESIS_BAND_M_F64
            {
                ViewMode::EntryCorridor
            } else {
                ViewMode::OrbitFit
            }
        }
        ViewMode::EntryCorridor => {
            if altitude_m_f64 > VIEW_ORBIT_ENTRY_ALTITUDE_M_F64 + VIEW_HYSTERESIS_BAND_M_F64 {
                ViewMode::OrbitFit
            } else if altitude_m_f64
                < VIEW_ENTRY_SURFACE_ALTITUDE_M_F64 - VIEW_HYSTERESIS_BAND_M_F64
            {
                ViewMode::SurfaceGrid
            } else {
                ViewMode::EntryCorridor
            }
        }
        ViewMode::SurfaceGrid => {
            if altitude_m_f64 > VIEW_ORBIT_ENTRY_ALTITUDE_M_F64 + VIEW_HYSTERESIS_BAND_M_F64 {
                ViewMode::OrbitFit
            } else if altitude_m_f64
                > VIEW_ENTRY_SURFACE_ALTITUDE_M_F64 + VIEW_HYSTERESIS_BAND_M_F64
            {
                ViewMode::EntryCorridor
            } else {
                ViewMode::SurfaceGrid
            }
        }
    }
}

/// Read-only abstract-marks canvas over snapshot copies.
///
/// Owns a [`Camera2D`](engine::render::Camera2D) selected by [`ViewMode`],
/// a pre-sized trajectory ring, a pre-sized screen-point scratch buffer,
/// and overlay flags. The camera rebuilds only on view, radius, or zoom
/// changes, never per-frame, so frames never jump; only the viewport
/// tracks the canvas rect. Shell state only.
#[derive(Debug, Clone)]
pub struct MarksView {
    /// Fixed-scale camera; viewport only tracks the canvas rect.
    camera: Camera2D,
    /// Trajectory history in meters, oldest first, capped at open.
    history_m: VecDeque<DVec3>,
    /// Screen-point scratch reused every trajectory frame.
    screens_px_f32: Vec<[f32; 2]>,
    /// Per-class overlay toggles.
    overlays: OverlayFlags,
    /// Active zoom-to-fit view selecting the camera preset.
    mode: ViewMode,
    /// Manual override pinning one view until cleared, if any.
    manual_override: Option<ViewMode>,
    /// Manual zoom as a scale ratio, clamped to the named limits.
    zoom_factor_ratio_f64: f64,
    /// Cached body radius in meters feeding camera centers.
    body_radius_m_f64: f64,
    /// Last finite snapshot altitude in meters feeding orbit fit.
    last_altitude_m_f64: f64,
}

impl MarksView {
    /// Open a marks view with pre-sized buffers and an orbit camera.
    ///
    /// The camera fits the Mars-like body plus the rails handoff band;
    /// per-frame circles still use the radius passed to `draw`. History
    /// and scratch allocate here once and never again after warmup. Starts
    /// in [`ViewMode::OrbitFit`] at unity zoom with no manual override.
    ///
    /// # Errors
    ///
    /// Returns [`MarksError::Render`] when the orbit-fit preset rejects
    /// the default viewport; unreachable with the named default.
    pub fn open() -> Result<Self, MarksError> {
        let camera = Camera2D::orbit_fit(
            &BodyParams::mars_like(),
            Meters::new(MARKS_OPEN_ALTITUDE_M_F64),
            MARKS_DEFAULT_VIEWPORT_PX_F32,
        )?;
        Ok(Self {
            camera,
            history_m: VecDeque::with_capacity(MARKS_HISTORY_CAP_ENTRIES_USIZE),
            screens_px_f32: Vec::with_capacity(MARKS_HISTORY_CAP_ENTRIES_USIZE),
            overlays: OverlayFlags::all(),
            mode: ViewMode::OrbitFit,
            manual_override: None,
            zoom_factor_ratio_f64: VIEW_ZOOM_DEFAULT_RATIO_F64,
            body_radius_m_f64: MARS_RADIUS_M,
            last_altitude_m_f64: MARKS_OPEN_ALTITUDE_M_F64,
        })
    }

    /// Return the active zoom-to-fit view.
    #[must_use]
    pub const fn view(&self) -> ViewMode {
        self.mode
    }

    /// Return the manual view override, if any.
    #[must_use]
    pub const fn manual_override(&self) -> Option<ViewMode> {
        self.manual_override
    }

    /// Return the manual zoom as a scale ratio, dimensionless.
    #[must_use]
    pub const fn zoom_factor_ratio_f64(&self) -> f64 {
        self.zoom_factor_ratio_f64
    }

    /// Return the fixed-scale camera.
    #[must_use]
    pub const fn camera(&self) -> Camera2D {
        self.camera
    }

    /// Return the overlay toggles.
    #[must_use]
    pub const fn overlays(&self) -> OverlayFlags {
        self.overlays
    }

    /// Replace the overlay toggles without touching history.
    pub const fn set_overlays(&mut self, overlays: OverlayFlags) {
        self.overlays = overlays;
    }

    /// Return the trajectory history length in entries.
    #[must_use]
    pub fn history_len_usize(&self) -> usize {
        self.history_m.len()
    }

    /// Return the trajectory history capacity in entries.
    #[must_use]
    pub fn history_capacity_usize(&self) -> usize {
        self.history_m.capacity()
    }

    /// Record one snapshot position into the capped history.
    ///
    /// Skips non-finite positions without touching the buffer. Drops the
    /// oldest entry at capacity, so this never allocates after open and
    /// never writes sim state.
    pub fn push_snapshot(&mut self, snapshot: &SimSnapshot) {
        let position_m = snapshot_position_m(snapshot);
        if !position_m.x.is_finite() || !position_m.y.is_finite() || !position_m.z.is_finite() {
            return;
        }
        if self.history_m.len() == MARKS_HISTORY_CAP_ENTRIES_USIZE {
            self.history_m.pop_front();
        }
        self.history_m.push_back(position_m);
    }

    /// Update the view from regime plus altitude without per-frame jumps.
    ///
    /// Syncs the cached body radius when valid, records finite altitudes,
    /// then auto-selects with hysteresis unless a manual override pins the
    /// view. The camera rebuilds only when the effective view or the cached
    /// radius changes; repeated same-view calls keep scale and center
    /// bit-identical. Invalid radii and non-finite altitudes keep the
    /// previous camera. Call from snapshot observe, never from paint.
    pub fn update_view(&mut self, regime_u8: u8, altitude_m_f64: f64, body_radius_m_f64: f64) {
        if altitude_m_f64.is_finite() {
            self.last_altitude_m_f64 = altitude_m_f64;
        }
        let mut changed_bool = self.sync_body_radius_cache(body_radius_m_f64);
        let target = self
            .manual_override
            .unwrap_or_else(|| auto_view_mode(regime_u8, altitude_m_f64, self.mode));
        if target != self.mode {
            self.mode = target;
            changed_bool = true;
        }
        if changed_bool {
            self.rebuild_camera();
        }
    }

    /// Pin a zoom-to-fit view as the manual override.
    ///
    /// Records the override and refits that view at the current zoom with
    /// the given radius when valid, else the cached radius. Auto-select
    /// stays suspended until [`Self::clear_manual_override`] or
    /// [`Self::reset_view`]. Shell state only; never writes sim state.
    pub fn zoom_to_fit(&mut self, mode: ViewMode, body_radius_m_f64: f64) {
        self.manual_override = Some(mode);
        self.mode = mode;
        self.sync_body_radius_cache(body_radius_m_f64);
        self.rebuild_camera();
    }

    /// Clear the manual override so auto-select resumes next update.
    ///
    /// Leaves the camera untouched; the next [`Self::update_view`] picks
    /// the view from regime plus altitude.
    pub const fn clear_manual_override(&mut self) {
        self.manual_override = None;
    }

    /// Set the manual zoom as a scale ratio with clamping.
    ///
    /// Finite factors clamp to [`VIEW_ZOOM_MIN_RATIO_F64`] through
    /// [`VIEW_ZOOM_MAX_RATIO_F64`]; non-finite factors keep the previous
    /// value. The camera rebuilds only when the clamped factor changes, so
    /// repeated sets never drift. Shell state only.
    pub fn set_zoom_factor_ratio_f64(&mut self, zoom_factor_ratio_f64: f64) {
        if !zoom_factor_ratio_f64.is_finite() {
            return;
        }
        let clamped_ratio_f64 =
            zoom_factor_ratio_f64.clamp(VIEW_ZOOM_MIN_RATIO_F64, VIEW_ZOOM_MAX_RATIO_F64);
        if clamped_ratio_f64.to_bits() != self.zoom_factor_ratio_f64.to_bits() {
            self.zoom_factor_ratio_f64 = clamped_ratio_f64;
            self.rebuild_camera();
        }
    }

    /// Clear the manual override and restore unity zoom.
    ///
    /// Rebuilds the camera only when an override was set or zoom differed
    /// from unity, using the cached altitude and radius. Shell state only.
    pub fn reset_view(&mut self) {
        let had_manual_override_bool = self.manual_override.is_some();
        self.manual_override = None;
        let had_zoom_bool =
            self.zoom_factor_ratio_f64.to_bits() != VIEW_ZOOM_DEFAULT_RATIO_F64.to_bits();
        self.zoom_factor_ratio_f64 = VIEW_ZOOM_DEFAULT_RATIO_F64;
        if had_manual_override_bool || had_zoom_bool {
            self.rebuild_camera();
        }
    }

    /// Sync the cached body radius and refit the current view.
    ///
    /// Keeps the previous radius and camera when the given radius is not
    /// finite and positive. Shell state only; radius feeds camera centers.
    pub fn sync_body_radius_m_f64(&mut self, body_radius_m_f64: f64) {
        if self.sync_body_radius_cache(body_radius_m_f64) {
            self.rebuild_camera();
        }
    }

    /// Cache a valid body radius, reporting whether it changed.
    ///
    /// Returns false and keeps the previous radius for non-finite or
    /// non-positive input.
    fn sync_body_radius_cache(&mut self, body_radius_m_f64: f64) -> bool {
        if !body_radius_m_f64.is_finite() || body_radius_m_f64 <= 0.0 {
            return false;
        }
        if body_radius_m_f64.to_bits() == self.body_radius_m_f64.to_bits() {
            return false;
        }
        self.body_radius_m_f64 = body_radius_m_f64;
        true
    }

    /// Rebuild the camera for the active view at the current zoom.
    ///
    /// Uses the cached radius plus altitude with the current viewport; a
    /// rejected build keeps the previous camera so views never jump. The
    /// orbit preset fits the last finite altitude, while entry and surface
    /// use fixed linear ranges from `engine::render`.
    fn rebuild_camera(&mut self) {
        let viewport_px_f32 = self.camera.viewport_px_f32();
        let body = body_params_for_radius_m(self.body_radius_m_f64);
        let base_option = match self.mode {
            ViewMode::OrbitFit => Camera2D::orbit_fit(
                &body,
                Meters::new(self.last_altitude_m_f64),
                viewport_px_f32,
            )
            .ok(),
            ViewMode::EntryCorridor => Camera2D::entry_corridor(&body, viewport_px_f32).ok(),
            ViewMode::SurfaceGrid => Camera2D::surface_grid(&body, viewport_px_f32).ok(),
        };
        let Some(base) = base_option else {
            return;
        };
        let scaled_px_per_m_f64 = base.scale_px_per_m_f64() * self.zoom_factor_ratio_f64;
        if let Ok(zoomed) = Camera2D::new(base.center_m(), scaled_px_per_m_f64, viewport_px_f32) {
            self.camera = zoomed;
        }
    }

    /// Paint one canvas frame from a snapshot copy plus body radius.
    ///
    /// Returns early when the preset hides the canvas or for a degenerate
    /// canvas rect. Radius-gated sections skip on a bad radius while star,
    /// orbit, trajectory, and ship still paint. The snapshot is read-only
    /// and paint never reselects the view; the camera scale and center
    /// change only through [`Self::update_view`] and the zoom calls, so
    /// frames never jump. Scratch buffers reuse. The caller owns the
    /// `CentralPanel`; this paints into its available rect with the
    /// immediate-mode painter.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &SimSnapshot,
        body_radius_m_f64: f64,
        visibility: PanelVisibility,
    ) {
        if !visibility.shows_marks_canvas_bool() {
            return;
        }
        let rect = ui.available_rect_before_wrap();
        if !rect.is_finite() || rect.width() <= 0.0 || rect.height() <= 0.0 {
            return;
        }
        self.sync_viewport_px_f32([rect.width(), rect.height()]);
        let painter = ui.painter();
        let viewport_height_px_f32 = self.camera.viewport_px_f32()[1];
        let radius_ok_bool = body_radius_m_f64.is_finite() && body_radius_m_f64 > 0.0;
        if self.overlays.star_bool {
            self.draw_star(painter, rect.min, viewport_height_px_f32);
        }
        if radius_ok_bool {
            if self.overlays.planet_bool {
                self.draw_circle(
                    painter,
                    rect.min,
                    viewport_height_px_f32,
                    body_radius_m_f64,
                    egui::Stroke::new(PLANET_STROKE_WIDTH_PT_F32, PLANET_MARK_COLOR),
                );
            }
            if self.overlays.atmosphere_bool {
                for altitude_m_f64 in [
                    ATMO_LOW_ALTITUDE_M_F64,
                    ATMO_MID_ALTITUDE_M_F64,
                    RAILS_ALTITUDE_M,
                ] {
                    self.draw_circle(
                        painter,
                        rect.min,
                        viewport_height_px_f32,
                        body_radius_m_f64 + altitude_m_f64,
                        egui::Stroke::new(ATMO_STROKE_WIDTH_PT_F32, ATMO_MARK_COLOR),
                    );
                }
            }
        }
        if self.overlays.orbit_bool {
            self.draw_orbit(painter, rect.min, viewport_height_px_f32, snapshot);
        }
        if self.overlays.trajectory_bool {
            self.fill_screens_px_f32();
            self.draw_trajectory(painter, rect.min, viewport_height_px_f32);
        }
        if radius_ok_bool && self.overlays.grid_bool {
            self.draw_grid(
                painter,
                rect.min,
                viewport_height_px_f32,
                snapshot,
                body_radius_m_f64,
            );
        }
        if self.overlays.ship_bool {
            self.draw_ship(painter, rect.min, viewport_height_px_f32, snapshot);
        }
    }

    /// Track the canvas rect without touching center or scale.
    ///
    /// Rebuilds the camera with identical center and scale when the rect
    /// size changes; a rejected rect keeps the previous camera, so views
    /// never jump on resize races.
    fn sync_viewport_px_f32(&mut self, viewport_px_f32: [f32; 2]) {
        let current_px_f32 = self.camera.viewport_px_f32();
        if current_px_f32[0].to_bits() == viewport_px_f32[0].to_bits()
            && current_px_f32[1].to_bits() == viewport_px_f32[1].to_bits()
        {
            return;
        }
        if let Ok(synced) = Camera2D::new(
            self.camera.center_m(),
            self.camera.scale_px_per_m_f64(),
            viewport_px_f32,
        ) {
            self.camera = synced;
        }
    }

    /// Refresh the screen-point scratch from history without allocating.
    ///
    /// Clears and refills the pre-sized buffer through the step-1 batch
    /// conversion. Lengths match by construction; a mismatch clears the
    /// scratch so the caller paints no stale points this frame.
    fn fill_screens_px_f32(&mut self) {
        self.screens_px_f32.clear();
        self.screens_px_f32
            .resize(self.history_m.len(), [0.0_f32; 2]);
        let (front_m, back_m) = self.history_m.as_slices();
        let (front_px_f32, back_px_f32) = self.screens_px_f32.split_at_mut(front_m.len());
        if self.camera.convert_into(front_m, front_px_f32).is_err() {
            self.screens_px_f32.clear();
            return;
        }
        if self.camera.convert_into(back_m, back_px_f32).is_err() {
            self.screens_px_f32.clear();
        }
    }

    /// Paint the decimated trajectory polyline from the scratch buffer.
    ///
    /// Emits every Nth point plus the latest point, breaking the chain
    /// on non-finite points so one bad sample never drags a line.
    fn draw_trajectory(
        &self,
        painter: &egui::Painter,
        origin: egui::Pos2,
        viewport_height_px_f32: f32,
    ) {
        let stroke = egui::Stroke::new(TRAJECTORY_STROKE_WIDTH_PT_F32, TRAJECTORY_MARK_COLOR);
        let total_usize = self.screens_px_f32.len();
        let mut previous_opt: Option<egui::Pos2> = None;
        for (index_usize, screen_px_f32) in self.screens_px_f32.iter().enumerate() {
            let is_last_bool = index_usize + 1 == total_usize;
            if index_usize % TRAJECTORY_STRIDE_SAMPLES_USIZE != 0 && !is_last_bool {
                continue;
            }
            Self::emit_screen_segment(
                painter,
                stroke,
                origin,
                viewport_height_px_f32,
                *screen_px_f32,
                &mut previous_opt,
            );
        }
    }

    /// Paint one body-centered circle mark at a world radius.
    ///
    /// Draws a single stroked circle when the disc fits the viewport and
    /// an arc polyline otherwise; the radius in points derives from two
    /// camera projections, never from a narrowing cast.
    fn draw_circle(
        &self,
        painter: &egui::Painter,
        origin: egui::Pos2,
        viewport_height_px_f32: f32,
        radius_m_f64: f64,
        stroke: egui::Stroke,
    ) {
        let center_px_f32 = self.camera.world_to_screen(DVec3::ZERO);
        let edge_px_f32 = self
            .camera
            .world_to_screen(DVec3::new(radius_m_f64, 0.0, 0.0));
        if !center_px_f32[0].is_finite()
            || !center_px_f32[1].is_finite()
            || !edge_px_f32[0].is_finite()
            || !edge_px_f32[1].is_finite()
        {
            return;
        }
        let radius_px_f32 = (edge_px_f32[0] - center_px_f32[0]).abs();
        if !radius_px_f32.is_finite() || radius_px_f32 <= 0.0 {
            return;
        }
        let center = canvas_pos2(origin, viewport_height_px_f32, center_px_f32);
        if !center.is_finite() {
            return;
        }
        let viewport_px_f32 = self.camera.viewport_px_f32();
        if radius_px_f32 * 2.0 <= viewport_px_f32[0].min(viewport_px_f32[1]) {
            painter.circle_stroke(center, radius_px_f32, stroke);
            return;
        }
        let mut angle_rad_f64 = 0.0;
        let mut previous_opt: Option<egui::Pos2> = None;
        for _ in 0..CIRCLE_SEGMENT_COUNT_USIZE {
            let screen_px_f32 = self
                .camera
                .world_to_screen(circle_world_m(radius_m_f64, angle_rad_f64));
            angle_rad_f64 += CIRCLE_STEP_RAD_F64;
            Self::emit_screen_segment(
                painter,
                stroke,
                origin,
                viewport_height_px_f32,
                screen_px_f32,
                &mut previous_opt,
            );
        }
        Self::emit_screen_segment(
            painter,
            stroke,
            origin,
            viewport_height_px_f32,
            self.camera
                .world_to_screen(circle_world_m(radius_m_f64, core::f64::consts::TAU)),
            &mut previous_opt,
        );
    }

    /// Paint radial surface grid ticks around the sub-ship point.
    ///
    /// Skips when the ship sits at the body center or reads non-finite;
    /// tick angles accumulate from named step constants with no casts.
    fn draw_grid(
        &self,
        painter: &egui::Painter,
        origin: egui::Pos2,
        viewport_height_px_f32: f32,
        snapshot: &SimSnapshot,
        body_radius_m_f64: f64,
    ) {
        let ship_m = snapshot_position_m(snapshot);
        if !ship_m.x.is_finite() || !ship_m.y.is_finite() {
            return;
        }
        let ship_radius_m_f64 = libm::sqrt(ship_m.x * ship_m.x + ship_m.y * ship_m.y);
        if !ship_radius_m_f64.is_finite() || ship_radius_m_f64 <= 0.0 {
            return;
        }
        let ship_angle_rad_f64 = libm::atan2(ship_m.y, ship_m.x);
        if !ship_angle_rad_f64.is_finite() {
            return;
        }
        let stroke = egui::Stroke::new(GRID_STROKE_WIDTH_PT_F32, GRID_MARK_COLOR);
        let mut angle_rad_f64 = ship_angle_rad_f64 - GRID_HALF_TICKS_F64 * GRID_TICK_STEP_RAD_F64;
        for _ in 0..GRID_TICK_COUNT_USIZE {
            let direction_m = DVec3::new(libm::cos(angle_rad_f64), libm::sin(angle_rad_f64), 0.0);
            let inner_px_f32 = self.camera.world_to_screen(direction_m * body_radius_m_f64);
            let outer_px_f32 = self
                .camera
                .world_to_screen(direction_m * (body_radius_m_f64 + GRID_TICK_LENGTH_M_F64));
            angle_rad_f64 += GRID_TICK_STEP_RAD_F64;
            if !inner_px_f32[0].is_finite()
                || !inner_px_f32[1].is_finite()
                || !outer_px_f32[0].is_finite()
                || !outer_px_f32[1].is_finite()
            {
                continue;
            }
            painter.line_segment(
                [
                    canvas_pos2(origin, viewport_height_px_f32, inner_px_f32),
                    canvas_pos2(origin, viewport_height_px_f32, outer_px_f32),
                ],
                stroke,
            );
        }
    }

    /// Paint the ship point from the snapshot position copy.
    ///
    /// Skips non-finite positions; reads only, never writes sim state.
    fn draw_ship(
        &self,
        painter: &egui::Painter,
        origin: egui::Pos2,
        viewport_height_px_f32: f32,
        snapshot: &SimSnapshot,
    ) {
        let ship_m = snapshot_position_m(snapshot);
        if !ship_m.x.is_finite() || !ship_m.y.is_finite() || !ship_m.z.is_finite() {
            return;
        }
        let screen_px_f32 = self.camera.world_to_screen(ship_m);
        if !screen_px_f32[0].is_finite() || !screen_px_f32[1].is_finite() {
            return;
        }
        painter.circle_filled(
            canvas_pos2(origin, viewport_height_px_f32, screen_px_f32),
            SHIP_POINT_RADIUS_PT_F32,
            SHIP_MARK_COLOR,
        );
    }

    /// Paint the star backdrop point at its display offset.
    ///
    /// Backdrop only, never simulation; skips non-finite projections so bad
    /// cameras never drag a point.
    fn draw_star(&self, painter: &egui::Painter, origin: egui::Pos2, viewport_height_px_f32: f32) {
        let star_m = DVec3::new(STAR_OFFSET_X_M_F64, STAR_OFFSET_Y_M_F64, 0.0);
        let screen_px_f32 = self.camera.world_to_screen(star_m);
        if !screen_px_f32[0].is_finite() || !screen_px_f32[1].is_finite() {
            return;
        }
        let point = canvas_pos2(origin, viewport_height_px_f32, screen_px_f32);
        if !point.is_finite() {
            return;
        }
        painter.circle_filled(point, STAR_POINT_RADIUS_PT_F32, STAR_MARK_COLOR);
    }

    /// Paint the orbit curve circle from the snapshot copy.
    ///
    /// Uses the valid semi-major axis when present, else the ship radius;
    /// skips non-finite or non-positive radii. Reads only, never writes.
    fn draw_orbit(
        &self,
        painter: &egui::Painter,
        origin: egui::Pos2,
        viewport_height_px_f32: f32,
        snapshot: &SimSnapshot,
    ) {
        let Some(radius_m_f64) = orbit_radius_m_f64(snapshot) else {
            return;
        };
        self.draw_circle(
            painter,
            origin,
            viewport_height_px_f32,
            radius_m_f64,
            egui::Stroke::new(ORBIT_STROKE_WIDTH_PT_F32, ORBIT_MARK_COLOR),
        );
    }

    /// Append one validated screen segment to a painter chain.
    ///
    /// Maps the camera-space point into canvas coordinates, breaking the
    /// chain on non-finite input so bad samples never drag a line.
    fn emit_screen_segment(
        painter: &egui::Painter,
        stroke: egui::Stroke,
        origin: egui::Pos2,
        viewport_height_px_f32: f32,
        screen_px_f32: [f32; 2],
        previous_opt: &mut Option<egui::Pos2>,
    ) {
        if !screen_px_f32[0].is_finite() || !screen_px_f32[1].is_finite() {
            *previous_opt = None;
            return;
        }
        let point = canvas_pos2(origin, viewport_height_px_f32, screen_px_f32);
        if !point.is_finite() {
            *previous_opt = None;
            return;
        }
        if let Some(previous) = *previous_opt {
            painter.line_segment([previous, point], stroke);
        }
        *previous_opt = Some(point);
    }
}

/// Build camera body params for a cached radius in meters.
///
/// Falls back to [`BodyParams::mars_like`] when the radius is invalid;
/// mass and rotation stay Mars-like while the radius follows the shell.
fn body_params_for_radius_m(body_radius_m_f64: f64) -> BodyParams {
    if body_radius_m_f64.is_finite()
        && body_radius_m_f64 > 0.0
        && let Ok(body) = BodyParams::new(
            Kilograms::new(MARS_MASS_KG),
            Meters::new(body_radius_m_f64),
            Seconds::new(MARS_ROTATION_PERIOD_S),
        )
    {
        return body;
    }
    BodyParams::mars_like()
}

/// Copy the snapshot position into a world vector in meters.
///
/// Single extraction point so every mark reads the same copy.
fn snapshot_position_m(snapshot: &SimSnapshot) -> DVec3 {
    DVec3::new(
        snapshot.position_m_f64[0],
        snapshot.position_m_f64[1],
        snapshot.position_m_f64[2],
    )
}

/// Return the orbit-curve radius in meters for a snapshot copy.
///
/// Uses the valid semi-major axis when present, else the ship `XY`
/// radius; returns none for non-finite or non-positive radii so bad
/// samples never paint.
fn orbit_radius_m_f64(snapshot: &SimSnapshot) -> Option<f64> {
    if snapshot.elements_valid_u8 != 0
        && snapshot.semi_major_axis_m_f64.is_finite()
        && snapshot.semi_major_axis_m_f64 > 0.0
    {
        return Some(snapshot.semi_major_axis_m_f64);
    }
    let ship_m = snapshot_position_m(snapshot);
    if !ship_m.x.is_finite() || !ship_m.y.is_finite() {
        return None;
    }
    let radius_m_f64 = libm::sqrt(ship_m.x * ship_m.x + ship_m.y * ship_m.y);
    if !radius_m_f64.is_finite() || radius_m_f64 <= 0.0 {
        return None;
    }
    Some(radius_m_f64)
}

/// Return a body-centered circle point at a world radius in meters.
///
/// Side view projects the world `XY` plane with `y`-up per `engine::render`.
fn circle_world_m(radius_m_f64: f64, angle_rad_f64: f64) -> DVec3 {
    DVec3::new(
        radius_m_f64 * libm::cos(angle_rad_f64),
        radius_m_f64 * libm::sin(angle_rad_f64),
        0.0,
    )
}

/// Map a camera-space point into canvas coordinates in points.
///
/// The camera maps world `+Y` up from the viewport center; egui `y`
/// grows down, so the height flips around the canvas origin.
fn canvas_pos2(
    origin: egui::Pos2,
    viewport_height_px_f32: f32,
    screen_px_f32: [f32; 2],
) -> egui::Pos2 {
    egui::Pos2::new(
        origin.x + screen_px_f32[0],
        origin.y + viewport_height_px_f32 - screen_px_f32[1],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::DesktopPreset;
    use engine::body::MARS_RADIUS_M;
    use engine::inspect::{REGIME_ATMOSPHERE_U8, REGIME_ORBIT_U8, REGIME_SURFACE_U8};

    /// Canvas width in points for draw fixtures.
    const CANVAS_WIDTH_PX_F32: f32 = 800.0;
    /// Canvas height in points for draw fixtures.
    const CANVAS_HEIGHT_PX_F32: f32 = 600.0;
    /// Orbited ship altitude in meters for fixtures.
    const SHIP_ALTITUDE_M_F64: f64 = 250_000.0;
    /// History push stride in meters between fixture points.
    const PUSH_STEP_M_F64: f64 = 1_000.0;
    /// Extra pushes past capacity for the cap test.
    const PUSH_OVERFLOW_USIZE: usize = 100;

    fn smoke_marks() -> MarksView {
        let Ok(marks) = MarksView::open() else {
            panic!("marks view must open")
        };
        marks
    }

    fn orbit_snapshot_at(position_m_f64: [f64; 3]) -> SimSnapshot {
        SimSnapshot {
            tick_count_u64: 7,
            elapsed_s_f64: 0.35,
            ship_epoch_s_f64: 0.35,
            master_seed_u64: 0x1234_ABCD_5678_EF90,
            stream_seed_u64: 9,
            snapshot_hash_u64: 0xDEAD_BEEF_0000_4321,
            position_m_f64,
            velocity_mps_f64: [0.0, 3_400.0, 0.0],
            drag_mps2_f64: [0.0, 0.0, 0.0],
            vel_dir_f64: [0.0, 1.0, 0.0],
            altitude_m_f64: SHIP_ALTITUDE_M_F64,
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
            warp_code_u8: 0,
            drop_reason_u8: 0,
            warp_flags_u8: 3,
            regime_u8: 0,
            frame_level_u8: 5,
            frame_depth_u8: 2,
            elements_valid_u8: 1,
            pick_valid_u8: 1,
            mark_kind_u8: 6,
            _pad_u8: [0_u8; 3],
        }
    }

    /// Build a snapshot copy with a regime code plus altitude in meters.
    ///
    /// Copies the orbit fixture position shape; tests override regime and
    /// altitude to drive view auto-select.
    fn snapshot_with_regime_altitude(regime_u8: u8, altitude_m_f64: f64) -> SimSnapshot {
        let mut snapshot = orbit_snapshot_at([MARS_RADIUS_M + SHIP_ALTITUDE_M_F64, 0.0, 0.0]);
        snapshot.regime_u8 = regime_u8;
        snapshot.altitude_m_f64 = altitude_m_f64;
        snapshot
    }

    fn canvas_input() -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(CANVAS_WIDTH_PX_F32, CANVAS_HEIGHT_PX_F32),
            )),
            ..Default::default()
        }
    }

    fn run_canvas(
        ctx: &egui::Context,
        marks: &mut MarksView,
        snapshot: &SimSnapshot,
        body_radius_m_f64: f64,
        visibility: PanelVisibility,
    ) -> egui::FullOutput {
        let mut output = ctx.run_ui(canvas_input(), |ui| {
            marks.draw(ui, snapshot, body_radius_m_f64, visibility);
        });
        // Headless tests have no renderer; the OS window step applies texture deltas.
        output.textures_delta.clear();
        output
    }

    #[test]
    fn open_presizes_history_at_buffer_plan_cap() {
        let marks = smoke_marks();
        assert_eq!(
            MARKS_HISTORY_CAP_ENTRIES_USIZE,
            BufferPlan::shell_default().plot_history_entries_usize()
        );
        assert_eq!(marks.history_len_usize(), 0);
        assert!(marks.history_capacity_usize() >= MARKS_HISTORY_CAP_ENTRIES_USIZE);
        assert_eq!(marks.overlays(), OverlayFlags::all());
        assert_eq!(marks.view(), ViewMode::OrbitFit);
        assert_eq!(marks.manual_override(), None);
        assert!((marks.zoom_factor_ratio_f64() - VIEW_ZOOM_DEFAULT_RATIO_F64).abs() < 1e-12);
    }

    #[test]
    fn history_cap_holds_without_growth() {
        let mut marks = smoke_marks();
        let capacity_usize = marks.history_capacity_usize();
        let mut offset_m_f64 = 0.0;
        for _ in 0..(MARKS_HISTORY_CAP_ENTRIES_USIZE + PUSH_OVERFLOW_USIZE) {
            let snapshot = orbit_snapshot_at([MARS_RADIUS_M + offset_m_f64, 0.0, 0.0]);
            marks.push_snapshot(&snapshot);
            offset_m_f64 += PUSH_STEP_M_F64;
        }
        assert_eq!(marks.history_len_usize(), MARKS_HISTORY_CAP_ENTRIES_USIZE);
        assert_eq!(marks.history_capacity_usize(), capacity_usize);
    }

    #[test]
    fn push_skips_non_finite_positions() {
        let mut marks = smoke_marks();
        let good = orbit_snapshot_at([MARS_RADIUS_M + SHIP_ALTITUDE_M_F64, 0.0, 0.0]);
        marks.push_snapshot(&good);
        assert_eq!(marks.history_len_usize(), 1);
        let bad = orbit_snapshot_at([f64::NAN, 0.0, 0.0]);
        marks.push_snapshot(&bad);
        assert_eq!(marks.history_len_usize(), 1);
    }

    #[test]
    fn draw_paints_marks_without_camera_jump() {
        let mut marks = smoke_marks();
        let snapshot = orbit_snapshot_at([MARS_RADIUS_M + SHIP_ALTITUDE_M_F64, 0.0, 0.0]);
        marks.push_snapshot(&snapshot);
        let visibility = PanelVisibility::for_preset(DesktopPreset::Descent);
        let ctx = egui::Context::default();
        let first = run_canvas(&ctx, &mut marks, &snapshot, MARS_RADIUS_M, visibility);
        assert!(!first.shapes.is_empty(), "descent canvas must paint marks");
        let scale_bits_u64 = marks.camera().scale_px_per_m_f64().to_bits();
        let center_m = marks.camera().center_m();
        let ship_px_f32 = marks
            .camera()
            .world_to_screen(snapshot_position_m(&snapshot));
        let second = run_canvas(&ctx, &mut marks, &snapshot, MARS_RADIUS_M, visibility);
        assert!(
            !second.shapes.is_empty(),
            "repeated canvas must keep painting"
        );
        assert_eq!(
            marks.camera().scale_px_per_m_f64().to_bits(),
            scale_bits_u64
        );
        assert_eq!(marks.camera().center_m(), center_m);
        let again_px_f32 = marks
            .camera()
            .world_to_screen(snapshot_position_m(&snapshot));
        assert_eq!(again_px_f32[0].to_bits(), ship_px_f32[0].to_bits());
        assert_eq!(again_px_f32[1].to_bits(), ship_px_f32[1].to_bits());
    }

    #[test]
    fn draw_skips_everything_when_ticker_only() {
        let mut marks = smoke_marks();
        let snapshot = orbit_snapshot_at([MARS_RADIUS_M + SHIP_ALTITUDE_M_F64, 0.0, 0.0]);
        marks.push_snapshot(&snapshot);
        let visibility = PanelVisibility::for_preset(DesktopPreset::TickerOnly);
        let ctx = egui::Context::default();
        let output = run_canvas(&ctx, &mut marks, &snapshot, MARS_RADIUS_M, visibility);
        assert!(
            output.shapes.is_empty(),
            "ticker-only must skip the marks canvas"
        );
    }

    #[test]
    fn invalid_radius_skips_circles_but_keeps_ship() {
        let mut marks = smoke_marks();
        let snapshot = orbit_snapshot_at([MARS_RADIUS_M + SHIP_ALTITUDE_M_F64, 0.0, 0.0]);
        marks.push_snapshot(&snapshot);
        let visibility = PanelVisibility::for_preset(DesktopPreset::Descent);
        let mut planet_only = OverlayFlags::none();
        planet_only.planet_bool = true;
        marks.set_overlays(planet_only);
        let ctx = egui::Context::default();
        let skipped = run_canvas(&ctx, &mut marks, &snapshot, f64::NAN, visibility);
        assert!(
            skipped.shapes.is_empty(),
            "bad radius must skip radius-gated marks"
        );
        let painted = run_canvas(&ctx, &mut marks, &snapshot, MARS_RADIUS_M, visibility);
        assert!(
            !painted.shapes.is_empty(),
            "good radius must paint the planet mark"
        );
        marks.set_overlays(OverlayFlags::all());
        let ship_kept = run_canvas(&ctx, &mut marks, &snapshot, 0.0, visibility);
        assert!(
            !ship_kept.shapes.is_empty(),
            "bad radius must keep trajectory and ship"
        );
    }

    #[test]
    fn overlay_flags_default_all_with_safe_setters() {
        let all = OverlayFlags::all();
        assert!(all.star_bool);
        assert!(all.planet_bool);
        assert!(all.atmosphere_bool);
        assert!(all.orbit_bool);
        assert!(all.trajectory_bool);
        assert!(all.ship_bool);
        assert!(all.grid_bool);
        let none = OverlayFlags::none();
        assert!(!none.star_bool);
        assert!(!none.planet_bool);
        assert!(!none.atmosphere_bool);
        assert!(!none.orbit_bool);
        assert!(!none.trajectory_bool);
        assert!(!none.ship_bool);
        assert!(!none.grid_bool);
        let mut flags = OverlayFlags::none();
        flags.set_star_bool(true);
        assert!(flags.star_bool);
        assert!(!flags.planet_bool);
        flags.set_planet_bool(true);
        flags.set_atmosphere_bool(true);
        flags.set_orbit_bool(true);
        flags.set_trajectory_bool(true);
        flags.set_ship_bool(true);
        flags.set_grid_bool(true);
        assert_eq!(flags, OverlayFlags::all());
        flags.set_star_bool(false);
        assert!(!flags.star_bool);
        assert!(flags.planet_bool);
        assert!(flags.atmosphere_bool);
        assert!(flags.orbit_bool);
        assert!(flags.trajectory_bool);
        assert!(flags.ship_bool);
        assert!(flags.grid_bool);
    }

    #[test]
    fn overlay_setters_keep_history_untouched() {
        let mut marks = smoke_marks();
        let snapshot = orbit_snapshot_at([MARS_RADIUS_M + SHIP_ALTITUDE_M_F64, 0.0, 0.0]);
        marks.push_snapshot(&snapshot);
        let len_usize = marks.history_len_usize();
        marks.set_overlays(OverlayFlags::none());
        assert_eq!(marks.history_len_usize(), len_usize);
        let mut flags = OverlayFlags::none();
        flags.set_ship_bool(true);
        marks.set_overlays(flags);
        assert_eq!(marks.history_len_usize(), len_usize);
        assert_eq!(marks.overlays(), flags);
    }

    #[test]
    fn each_overlay_gate_paints_alone() {
        let visibility = PanelVisibility::for_preset(DesktopPreset::Descent);
        let ctx = egui::Context::default();
        let snapshot = orbit_snapshot_at([MARS_RADIUS_M + SHIP_ALTITUDE_M_F64, 0.0, 0.0]);
        for index_usize in 0..7 {
            let mut marks = smoke_marks();
            let mut offset_m_f64 = 0.0;
            for _ in 0..8 {
                let pushed = orbit_snapshot_at([
                    MARS_RADIUS_M + SHIP_ALTITUDE_M_F64 + offset_m_f64,
                    0.0,
                    0.0,
                ]);
                marks.push_snapshot(&pushed);
                offset_m_f64 += PUSH_STEP_M_F64;
            }
            let mut flags = OverlayFlags::none();
            let name = match index_usize {
                0 => {
                    flags.set_star_bool(true);
                    "star"
                }
                1 => {
                    flags.set_planet_bool(true);
                    "planet"
                }
                2 => {
                    flags.set_atmosphere_bool(true);
                    "atmosphere"
                }
                3 => {
                    flags.set_orbit_bool(true);
                    "orbit"
                }
                4 => {
                    flags.set_trajectory_bool(true);
                    "trajectory"
                }
                5 => {
                    flags.set_ship_bool(true);
                    "ship"
                }
                _ => {
                    flags.set_grid_bool(true);
                    "grid"
                }
            };
            marks.set_overlays(flags);
            let output = run_canvas(&ctx, &mut marks, &snapshot, MARS_RADIUS_M, visibility);
            assert!(
                !output.shapes.is_empty(),
                "{name} toggle alone must paint its mark class"
            );
        }
    }

    #[test]
    fn no_overlay_paints_nothing_on_descent() {
        let mut marks = smoke_marks();
        let snapshot = orbit_snapshot_at([MARS_RADIUS_M + SHIP_ALTITUDE_M_F64, 0.0, 0.0]);
        marks.push_snapshot(&snapshot);
        marks.set_overlays(OverlayFlags::none());
        let visibility = PanelVisibility::for_preset(DesktopPreset::Descent);
        let ctx = egui::Context::default();
        let output = run_canvas(&ctx, &mut marks, &snapshot, MARS_RADIUS_M, visibility);
        assert!(
            output.shapes.is_empty(),
            "all overlays off must paint nothing"
        );
        marks.set_overlays(OverlayFlags::all());
        let painted = run_canvas(&ctx, &mut marks, &snapshot, MARS_RADIUS_M, visibility);
        assert!(
            !painted.shapes.is_empty(),
            "all overlays on must paint marks"
        );
    }

    #[test]
    fn circle_counts_stay_in_sync() {
        assert_eq!(CIRCLE_SEGMENT_COUNT_USIZE, 128);
        assert!((CIRCLE_SEGMENT_COUNT_F64 - 128.0).abs() < 1e-12);
        assert!(
            (CIRCLE_STEP_RAD_F64 * CIRCLE_SEGMENT_COUNT_F64 - core::f64::consts::TAU).abs() < 1e-12
        );
        assert_eq!(GRID_TICK_COUNT_USIZE, 21);
        assert!((GRID_HALF_TICKS_F64 - 10.0).abs() < 1e-12);
    }

    #[test]
    fn view_modes_cover_debug_camera_list() {
        assert_eq!(ViewMode::ALL.len(), 3);
        assert_eq!(ViewMode::OrbitFit.label(), "orbit-fit");
        assert_eq!(ViewMode::EntryCorridor.label(), "entry-corridor");
        assert_eq!(ViewMode::SurfaceGrid.label(), "surface-grid");
    }

    #[test]
    fn auto_view_orbit_regime_always_orbit_fit() {
        for current in ViewMode::ALL {
            assert_eq!(
                auto_view_mode(REGIME_ORBIT_U8, 5_000.0, current),
                ViewMode::OrbitFit
            );
            assert_eq!(
                auto_view_mode(REGIME_ORBIT_U8, 250_000.0, current),
                ViewMode::OrbitFit
            );
        }
    }

    #[test]
    fn auto_view_surface_regime_always_surface_grid() {
        for current in ViewMode::ALL {
            assert_eq!(
                auto_view_mode(REGIME_SURFACE_U8, 250_000.0, current),
                ViewMode::SurfaceGrid
            );
            assert_eq!(
                auto_view_mode(REGIME_SURFACE_U8, 0.0, current),
                ViewMode::SurfaceGrid
            );
        }
    }

    #[test]
    fn auto_view_altitude_extremes_override_atmosphere_regime() {
        assert_eq!(
            auto_view_mode(REGIME_ATMOSPHERE_U8, 200_000.0, ViewMode::EntryCorridor),
            ViewMode::OrbitFit
        );
        assert_eq!(
            auto_view_mode(REGIME_ATMOSPHERE_U8, 5_000.0, ViewMode::EntryCorridor),
            ViewMode::SurfaceGrid
        );
    }

    #[test]
    fn auto_view_orbit_handoff_holds_two_km_band() {
        let low_edge_m_f64 = VIEW_ORBIT_ENTRY_ALTITUDE_M_F64 - VIEW_HYSTERESIS_BAND_M_F64;
        assert_eq!(
            auto_view_mode(
                REGIME_ATMOSPHERE_U8,
                low_edge_m_f64 + 1.0,
                ViewMode::OrbitFit
            ),
            ViewMode::OrbitFit
        );
        assert_eq!(
            auto_view_mode(
                REGIME_ATMOSPHERE_U8,
                low_edge_m_f64 - 1.0,
                ViewMode::OrbitFit
            ),
            ViewMode::EntryCorridor
        );
        let high_edge_m_f64 = VIEW_ORBIT_ENTRY_ALTITUDE_M_F64 + VIEW_HYSTERESIS_BAND_M_F64;
        assert_eq!(
            auto_view_mode(
                REGIME_ATMOSPHERE_U8,
                high_edge_m_f64 - 1.0,
                ViewMode::EntryCorridor
            ),
            ViewMode::EntryCorridor
        );
        assert_eq!(
            auto_view_mode(
                REGIME_ATMOSPHERE_U8,
                high_edge_m_f64 + 1.0,
                ViewMode::EntryCorridor
            ),
            ViewMode::OrbitFit
        );
    }

    #[test]
    fn auto_view_surface_handoff_holds_two_km_band() {
        let low_edge_m_f64 = VIEW_ENTRY_SURFACE_ALTITUDE_M_F64 - VIEW_HYSTERESIS_BAND_M_F64;
        assert_eq!(
            auto_view_mode(
                REGIME_ATMOSPHERE_U8,
                low_edge_m_f64 + 1.0,
                ViewMode::EntryCorridor
            ),
            ViewMode::EntryCorridor
        );
        assert_eq!(
            auto_view_mode(
                REGIME_ATMOSPHERE_U8,
                low_edge_m_f64 - 1.0,
                ViewMode::EntryCorridor
            ),
            ViewMode::SurfaceGrid
        );
        let high_edge_m_f64 = VIEW_ENTRY_SURFACE_ALTITUDE_M_F64 + VIEW_HYSTERESIS_BAND_M_F64;
        assert_eq!(
            auto_view_mode(
                REGIME_ATMOSPHERE_U8,
                high_edge_m_f64 - 1.0,
                ViewMode::SurfaceGrid
            ),
            ViewMode::SurfaceGrid
        );
        assert_eq!(
            auto_view_mode(
                REGIME_ATMOSPHERE_U8,
                high_edge_m_f64 + 1.0,
                ViewMode::SurfaceGrid
            ),
            ViewMode::EntryCorridor
        );
    }

    #[test]
    fn auto_view_boundary_oscillation_never_flickers() {
        let mut from_entry = ViewMode::EntryCorridor;
        let mut from_orbit = ViewMode::OrbitFit;
        for altitude_m_f64 in [119_000.0, 121_000.0, 119_500.0, 120_500.0] {
            from_entry = auto_view_mode(REGIME_ATMOSPHERE_U8, altitude_m_f64, from_entry);
            from_orbit = auto_view_mode(REGIME_ATMOSPHERE_U8, altitude_m_f64, from_orbit);
        }
        assert_eq!(from_entry, ViewMode::EntryCorridor);
        assert_eq!(from_orbit, ViewMode::OrbitFit);
    }

    #[test]
    fn auto_view_non_finite_altitude_keeps_current() {
        for current in ViewMode::ALL {
            assert_eq!(
                auto_view_mode(REGIME_ATMOSPHERE_U8, f64::NAN, current),
                current
            );
            assert_eq!(auto_view_mode(9_u8, f64::NAN, current), current);
        }
        assert_eq!(
            auto_view_mode(9_u8, 50_000.0, ViewMode::OrbitFit),
            ViewMode::EntryCorridor
        );
    }

    #[test]
    fn manual_override_pins_view_until_cleared() {
        let mut marks = smoke_marks();
        marks.zoom_to_fit(ViewMode::SurfaceGrid, MARS_RADIUS_M);
        assert_eq!(marks.view(), ViewMode::SurfaceGrid);
        assert_eq!(marks.manual_override(), Some(ViewMode::SurfaceGrid));
        let orbit_snapshot = snapshot_with_regime_altitude(REGIME_ORBIT_U8, 250_000.0);
        marks.update_view(
            orbit_snapshot.regime_u8,
            orbit_snapshot.altitude_m_f64,
            MARS_RADIUS_M,
        );
        assert_eq!(marks.view(), ViewMode::SurfaceGrid);
        marks.clear_manual_override();
        assert_eq!(marks.manual_override(), None);
        marks.update_view(
            orbit_snapshot.regime_u8,
            orbit_snapshot.altitude_m_f64,
            MARS_RADIUS_M,
        );
        assert_eq!(marks.view(), ViewMode::OrbitFit);
    }

    #[test]
    fn zoom_factor_clamps_to_named_limits() {
        let mut marks = smoke_marks();
        marks.set_zoom_factor_ratio_f64(VIEW_ZOOM_MAX_RATIO_F64 * 10.0);
        assert!((marks.zoom_factor_ratio_f64() - VIEW_ZOOM_MAX_RATIO_F64).abs() < 1e-12);
        marks.set_zoom_factor_ratio_f64(VIEW_ZOOM_MIN_RATIO_F64 / 10.0);
        assert!((marks.zoom_factor_ratio_f64() - VIEW_ZOOM_MIN_RATIO_F64).abs() < 1e-12);
        marks.set_zoom_factor_ratio_f64(-2.0);
        assert!((marks.zoom_factor_ratio_f64() - VIEW_ZOOM_MIN_RATIO_F64).abs() < 1e-12);
        marks.set_zoom_factor_ratio_f64(f64::NAN);
        assert!((marks.zoom_factor_ratio_f64() - VIEW_ZOOM_MIN_RATIO_F64).abs() < 1e-12);
        marks.set_zoom_factor_ratio_f64(f64::INFINITY);
        assert!((marks.zoom_factor_ratio_f64() - VIEW_ZOOM_MIN_RATIO_F64).abs() < 1e-12);
    }

    #[test]
    fn zoom_factor_scales_camera_without_changing_view() {
        let mut marks = smoke_marks();
        marks.update_view(REGIME_ORBIT_U8, 250_000.0, MARS_RADIUS_M);
        marks.set_zoom_factor_ratio_f64(2.0);
        assert_eq!(marks.view(), ViewMode::OrbitFit);
        let zoomed_scale_px_per_m_f64 = marks.camera().scale_px_per_m_f64();
        marks.set_zoom_factor_ratio_f64(VIEW_ZOOM_DEFAULT_RATIO_F64);
        let unity_scale_px_per_m_f64 = marks.camera().scale_px_per_m_f64();
        assert_eq!(
            zoomed_scale_px_per_m_f64.to_bits(),
            (unity_scale_px_per_m_f64 * 2.0).to_bits()
        );
        marks.reset_view();
        assert_eq!(marks.manual_override(), None);
        assert!((marks.zoom_factor_ratio_f64() - VIEW_ZOOM_DEFAULT_RATIO_F64).abs() < 1e-12);
        assert_eq!(
            marks.camera().scale_px_per_m_f64().to_bits(),
            unity_scale_px_per_m_f64.to_bits()
        );
    }

    #[test]
    fn repeated_same_view_updates_keep_camera_bits() {
        let mut marks = smoke_marks();
        marks.update_view(REGIME_ATMOSPHERE_U8, 50_000.0, MARS_RADIUS_M);
        assert_eq!(marks.view(), ViewMode::EntryCorridor);
        let scale_bits_u64 = marks.camera().scale_px_per_m_f64().to_bits();
        let center_m = marks.camera().center_m();
        for altitude_m_f64 in [55_000.0, 60_000.0, 45_000.0, 50_000.0] {
            marks.update_view(REGIME_ATMOSPHERE_U8, altitude_m_f64, MARS_RADIUS_M);
            assert_eq!(marks.view(), ViewMode::EntryCorridor);
            assert_eq!(
                marks.camera().scale_px_per_m_f64().to_bits(),
                scale_bits_u64
            );
            assert_eq!(marks.camera().center_m(), center_m);
        }
    }

    #[test]
    fn view_switch_rebuilds_camera_for_new_range() {
        let mut marks = smoke_marks();
        marks.update_view(REGIME_ORBIT_U8, 250_000.0, MARS_RADIUS_M);
        assert_eq!(marks.view(), ViewMode::OrbitFit);
        let orbit_scale_bits_u64 = marks.camera().scale_px_per_m_f64().to_bits();
        let orbit_center_m = marks.camera().center_m();
        marks.update_view(REGIME_ATMOSPHERE_U8, 50_000.0, MARS_RADIUS_M);
        assert_eq!(marks.view(), ViewMode::EntryCorridor);
        assert_ne!(
            marks.camera().scale_px_per_m_f64().to_bits(),
            orbit_scale_bits_u64
        );
        assert_ne!(marks.camera().center_m(), orbit_center_m);
    }

    #[test]
    fn update_view_with_bad_radius_keeps_cached_body() {
        let mut marks = smoke_marks();
        marks.update_view(REGIME_ATMOSPHERE_U8, 50_000.0, MARS_RADIUS_M);
        let entry_center_m = marks.camera().center_m();
        marks.update_view(REGIME_ATMOSPHERE_U8, 55_000.0, f64::NAN);
        assert_eq!(marks.view(), ViewMode::EntryCorridor);
        assert_eq!(marks.camera().center_m(), entry_center_m);
    }

    #[test]
    fn marks_error_labels_each_cause() {
        let radius = MarksError::InvalidRadius {
            radius_m_f64: f64::NAN,
        };
        assert!(format!("{radius}").contains("radius"));
        let failed =
            engine::render::Camera2D::new(glam::DVec3::ZERO, 0.0, MARKS_DEFAULT_VIEWPORT_PX_F32);
        let Err(camera_error) = failed else {
            panic!("zero scale must fail")
        };
        let mapped = MarksError::from(camera_error);
        assert!(matches!(mapped, MarksError::InvalidScale { .. }));
        assert!(format!("{mapped}").contains("scale"));
        assert!(std::error::Error::source(&mapped).is_none());
    }
}
