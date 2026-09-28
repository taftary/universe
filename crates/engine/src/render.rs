//! Render backend seam and camera-relative f64 to f32 transform.
//!
//! The concrete backend (`wgpu`, D-003) implements [`Renderer`]; gameplay
//! code never touches GPU types. The 2D abstract-marks canvas in
//! issue 52 step 1 maps sim `f64` world space to render `f32`
//! screen space through [`Camera2D`] only. [`Camera2D::world_to_screen`]
//! is the single conversion point; no other module casts sim
//! precision down. Side view is an orthographic projection onto the
//! world `XY` plane with `y`-up points; `y`-down backends negate `Y`.

use glam::DVec3;
use thiserror::Error;

use crate::body::BodyParams;
use crate::trajectory::RAILS_ALTITUDE_M;
use crate::units::Meters;

/// Camera rebase distance in meters.
///
/// The camera center follows the ship once the ship drifts further
/// than this from the center, so `f32` mantissa error stays below
/// visual tolerance at terrain scale. Source: `docs/tech/simulation.md`
/// floating origin (`f32` ULP at 5 km is about 0.5 mm).
pub const ORIGIN_REBASE_DISTANCE_M: f64 = 5000.0;

/// Orbit-fit margin in meters beyond the furthest fitted point.
///
/// Keeps the ship plus mark labels inside the orbit-fit view.
/// Source: issue 52 step 1 preset, hand-placed.
pub const ORBIT_FIT_MARGIN_M: f64 = 50_000.0;

/// Entry-corridor margin in meters beyond the handoff band.
///
/// Keeps the surface and the rails boundary inside the entry view.
/// Source: issue 52 step 1 preset, hand-placed.
pub const ENTRY_CORRIDOR_MARGIN_M: f64 = 10_000.0;

/// Surface-grid half extent in meters.
///
/// Twice [`ORIGIN_REBASE_DISTANCE_M`] so the touchdown patch spans one
/// rebase each way. Source: issue 52 step 1 preset, derived.
pub const SURFACE_GRID_HALF_EXTENT_M: f64 = ORIGIN_REBASE_DISTANCE_M * 2.0;

/// Render camera and view-range failures.
#[derive(Debug, Error, PartialEq)]
pub enum RenderError {
    /// Input or computed value was not finite.
    #[error("non-finite value: {value_f64}")]
    NonFinite {
        /// Rejected value.
        value_f64: f64,
    },
    /// Scale was not positive and finite in points per meter.
    #[error("invalid scale in points per meter: {scale_px_per_m_f64}")]
    InvalidScale {
        /// Rejected scale in points per meter.
        scale_px_per_m_f64: f64,
    },
    /// Viewport was not positive and finite in points.
    #[error("invalid viewport in points: {width_px_f32} x {height_px_f32}")]
    InvalidViewport {
        /// Rejected width in points.
        width_px_f32: f32,
        /// Rejected height in points.
        height_px_f32: f32,
    },
    /// Ship altitude was not finite in meters.
    #[error("invalid ship altitude in meters: {altitude_m_f64}")]
    InvalidAltitude {
        /// Rejected altitude in meters.
        altitude_m_f64: f64,
    },
    /// Batch buffers had different lengths, dimensionless counts.
    #[error("batch length mismatch: {worlds_len_usize} worlds vs {screens_len_usize} screens")]
    BatchLengthMismatch {
        /// World buffer length, dimensionless.
        worlds_len_usize: usize,
        /// Screen buffer length, dimensionless.
        screens_len_usize: usize,
    },
}

/// Render backend handle.
pub trait Renderer {
    /// Backend name for logs and diagnostics.
    fn name(&self) -> &'static str;
}

/// Linear 2D camera mapping sim meters to screen points.
///
/// Holds an `f64` world center, a points-per-meter scale, and an `f32`
/// viewport. [`Camera2D::world_to_screen`] subtracts in `f64`, scales in
/// `f64`, then narrows to `f32` once. Screen origin is the viewport
/// center; world `+X` maps right and world `+Y` maps up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera2D {
    /// World center in meters.
    center_m: DVec3,
    /// Scale in points per meter.
    scale_px_per_m_f64: f64,
    /// Viewport width and height in points.
    viewport_px_f32: [f32; 2],
}

impl Camera2D {
    /// Create a camera from a world center, scale, and viewport.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::NonFinite`] for a non-finite center,
    /// [`RenderError::InvalidScale`] for a scale that is not positive and
    /// finite, or [`RenderError::InvalidViewport`] for a viewport that is
    /// not positive and finite.
    pub fn new(
        center_m: DVec3,
        scale_px_per_m_f64: f64,
        viewport_px_f32: [f32; 2],
    ) -> Result<Self, RenderError> {
        if !center_m.x.is_finite() || !center_m.y.is_finite() || !center_m.z.is_finite() {
            return Err(RenderError::NonFinite {
                value_f64: center_m.x + center_m.y + center_m.z,
            });
        }
        if !scale_px_per_m_f64.is_finite() || scale_px_per_m_f64 <= 0.0 {
            return Err(RenderError::InvalidScale { scale_px_per_m_f64 });
        }
        check_viewport_px_f32(viewport_px_f32)?;
        Ok(Self {
            center_m,
            scale_px_per_m_f64,
            viewport_px_f32,
        })
    }

    /// Orbit-fit camera centered on the body center.
    ///
    /// Half extent is body radius plus the larger of ship altitude and
    /// [`RAILS_ALTITUDE_M`] plus [`ORBIT_FIT_MARGIN_M`], so the surface,
    /// the 120 km handoff, and the ship all stay inside without rescaling.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::InvalidAltitude`] for non-finite altitude or
    /// [`RenderError::InvalidViewport`] for a bad viewport.
    pub fn orbit_fit(
        body: &BodyParams,
        ship_altitude_m: Meters,
        viewport_px_f32: [f32; 2],
    ) -> Result<Self, RenderError> {
        let altitude_m_f64 = ship_altitude_m.value();
        if !altitude_m_f64.is_finite() {
            return Err(RenderError::InvalidAltitude { altitude_m_f64 });
        }
        let (_, _) = viewport_extent_f64(viewport_px_f32)?;
        let shell_m_f64 = altitude_m_f64.max(0.0).max(RAILS_ALTITUDE_M);
        let half_extent_m_f64 = body.radius_m().value() + shell_m_f64 + ORBIT_FIT_MARGIN_M;
        let scale_px_per_m_f64 = scale_for_half_extent_m(viewport_px_f32, half_extent_m_f64)?;
        Self::new(DVec3::ZERO, scale_px_per_m_f64, viewport_px_f32)
    }

    /// Entry-corridor camera over the 0 to 120 km band.
    ///
    /// Center sits at half the rails altitude on `+X`; half extent is
    /// half the rails altitude plus [`ENTRY_CORRIDOR_MARGIN_M`], so the
    /// surface and the 120 km handoff stay inside a fixed linear range.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::InvalidViewport`] for a bad viewport.
    pub fn entry_corridor(
        body: &BodyParams,
        viewport_px_f32: [f32; 2],
    ) -> Result<Self, RenderError> {
        let (_, _) = viewport_extent_f64(viewport_px_f32)?;
        let half_extent_m_f64 = RAILS_ALTITUDE_M / 2.0 + ENTRY_CORRIDOR_MARGIN_M;
        let center_m = DVec3::new(body.radius_m().value() + RAILS_ALTITUDE_M / 2.0, 0.0, 0.0);
        let scale_px_per_m_f64 = scale_for_half_extent_m(viewport_px_f32, half_extent_m_f64)?;
        Self::new(center_m, scale_px_per_m_f64, viewport_px_f32)
    }

    /// Surface-grid camera over the touchdown patch.
    ///
    /// Center sits on the surface at `+X`; half extent is
    /// [`SURFACE_GRID_HALF_EXTENT_M`], fixed so grid handoffs never rescale.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::InvalidViewport`] for a bad viewport.
    pub fn surface_grid(body: &BodyParams, viewport_px_f32: [f32; 2]) -> Result<Self, RenderError> {
        let (_, _) = viewport_extent_f64(viewport_px_f32)?;
        let center_m = DVec3::new(body.radius_m().value(), 0.0, 0.0);
        let scale_px_per_m_f64 =
            scale_for_half_extent_m(viewport_px_f32, SURFACE_GRID_HALF_EXTENT_M)?;
        Self::new(center_m, scale_px_per_m_f64, viewport_px_f32)
    }

    /// World center in meters.
    #[must_use]
    pub const fn center_m(&self) -> DVec3 {
        self.center_m
    }

    /// Scale in points per meter.
    #[must_use]
    pub const fn scale_px_per_m_f64(&self) -> f64 {
        self.scale_px_per_m_f64
    }

    /// Viewport width and height in points.
    #[must_use]
    pub const fn viewport_px_f32(&self) -> [f32; 2] {
        self.viewport_px_f32
    }

    /// Map a world position in meters to screen points.
    ///
    /// Subtracts the center and scales in `f64`, then narrows to `f32`.
    /// This is the single `f64` to `f32` conversion point; world `Z`
    /// drops as depth. Finite inputs through a valid camera map to
    /// finite outputs; non-finite inputs map to non-finite outputs.
    #[must_use]
    pub fn world_to_screen(&self, world_m: DVec3) -> [f32; 2] {
        let half_width_px_f64 = f64::from(self.viewport_px_f32[0]) / 2.0;
        let half_height_px_f64 = f64::from(self.viewport_px_f32[1]) / 2.0;
        [
            axis_to_screen_px_f32(
                world_m.x,
                self.center_m.x,
                half_width_px_f64,
                self.scale_px_per_m_f64,
            ),
            axis_to_screen_px_f32(
                world_m.y,
                self.center_m.y,
                half_height_px_f64,
                self.scale_px_per_m_f64,
            ),
        ]
    }

    /// Fill a caller-provided screen buffer without allocating.
    ///
    /// Each output equals [`Camera2D::world_to_screen`] at the same index,
    /// so steady-state frames reuse one pre-sized buffer.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::BatchLengthMismatch`] when the buffers differ
    /// in length.
    pub fn convert_into(
        &self,
        worlds_m: &[DVec3],
        screens_px_f32: &mut [[f32; 2]],
    ) -> Result<(), RenderError> {
        if worlds_m.len() != screens_px_f32.len() {
            return Err(RenderError::BatchLengthMismatch {
                worlds_len_usize: worlds_m.len(),
                screens_len_usize: screens_px_f32.len(),
            });
        }
        for (world_m, screen_px_f32) in worlds_m.iter().zip(screens_px_f32.iter_mut()) {
            *screen_px_f32 = self.world_to_screen(*world_m);
        }
        Ok(())
    }

    /// Report whether the target drifted past the rebase distance.
    ///
    /// Measures `f64` distance with `libm` so `x86_64` and `AArch64`
    /// agree. Non-finite targets never trigger a rebase; callers pass
    /// validated sim states.
    #[must_use]
    pub fn should_rebase(&self, target_m: DVec3) -> bool {
        let offset_m = target_m - self.center_m;
        let distance_m_f64 = libm::sqrt(offset_m.length_squared());
        distance_m_f64 > ORIGIN_REBASE_DISTANCE_M
    }

    /// Return this camera recentered on a finite target.
    ///
    /// Scale and viewport stay fixed, so relative geometry keeps its
    /// screen offsets up to float rounding and views never jump.
    #[must_use]
    pub const fn rebased(&self, target_m: DVec3) -> Self {
        Self {
            center_m: target_m,
            scale_px_per_m_f64: self.scale_px_per_m_f64,
            viewport_px_f32: self.viewport_px_f32,
        }
    }
}

/// Reject a viewport that is not positive and finite in points.
fn check_viewport_px_f32(viewport_px_f32: [f32; 2]) -> Result<(), RenderError> {
    let width_px_f32 = viewport_px_f32[0];
    let height_px_f32 = viewport_px_f32[1];
    if !width_px_f32.is_finite() || !height_px_f32.is_finite() {
        return Err(RenderError::InvalidViewport {
            width_px_f32,
            height_px_f32,
        });
    }
    if width_px_f32 <= 0.0 || height_px_f32 <= 0.0 {
        return Err(RenderError::InvalidViewport {
            width_px_f32,
            height_px_f32,
        });
    }
    Ok(())
}

/// Widen a validated viewport to `f64` extents in points.
fn viewport_extent_f64(viewport_px_f32: [f32; 2]) -> Result<(f64, f64), RenderError> {
    check_viewport_px_f32(viewport_px_f32)?;
    Ok((f64::from(viewport_px_f32[0]), f64::from(viewport_px_f32[1])))
}

/// Fit a half extent in meters into the smaller viewport side.
///
/// Linear only: scale is the smaller side over twice the half extent.
/// The half extent is positive by construction at every call site.
fn scale_for_half_extent_m(
    viewport_px_f32: [f32; 2],
    half_extent_m_f64: f64,
) -> Result<f64, RenderError> {
    let (width_px_f64, height_px_f64) = viewport_extent_f64(viewport_px_f32)?;
    if !half_extent_m_f64.is_finite() || half_extent_m_f64 <= 0.0 {
        return Err(RenderError::NonFinite {
            value_f64: half_extent_m_f64,
        });
    }
    Ok(width_px_f64.min(height_px_f64) / (2.0 * half_extent_m_f64))
}

/// Map one world axis in meters to a screen axis in points.
///
/// Shared by both screen axes so the two stay exactly linear.
fn axis_to_screen_px_f32(
    world_m_f64: f64,
    center_m_f64: f64,
    half_viewport_px_f64: f64,
    scale_px_per_m_f64: f64,
) -> f32 {
    let screen_px_f64 = (world_m_f64 - center_m_f64) * scale_px_per_m_f64 + half_viewport_px_f64;
    narrow_to_points_f32(screen_px_f64)
}

/// Narrow one `f64` screen coordinate to `f32` points.
///
/// Sole narrowing cast behind [`Camera2D::world_to_screen`].
#[expect(
    clippy::cast_possible_truncation,
    reason = "sole f64-to-f32 boundary; viewport ranges always fit f32"
)]
fn narrow_to_points_f32(value_px_f64: f64) -> f32 {
    value_px_f64 as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Viewport width in points for view fixtures.
    const VIEWPORT_WIDTH_PX_F32: f32 = 800.0;
    /// Viewport height in points for view fixtures.
    const VIEWPORT_HEIGHT_PX_F32: f32 = 600.0;
    /// Ship altitude in meters for the orbit-fit fixture.
    const SHIP_ALTITUDE_M_F64: f64 = 250_000.0;
    /// World step in meters for linearity checks.
    const WORLD_STEP_M_F64: f64 = 1_000.0;
    /// Screen equality tolerance in points.
    const SCREEN_TOL_PX_F32: f32 = 1e-3;
    /// Meters equality tolerance for constant checks.
    const METERS_TOL_F64: f64 = 1e-9;

    fn viewport_px_f32() -> [f32; 2] {
        [VIEWPORT_WIDTH_PX_F32, VIEWPORT_HEIGHT_PX_F32]
    }

    fn test_body() -> BodyParams {
        BodyParams::mars_like()
    }

    fn orbit_camera() -> Camera2D {
        let Ok(camera) = Camera2D::orbit_fit(
            &test_body(),
            Meters::new(SHIP_ALTITUDE_M_F64),
            viewport_px_f32(),
        ) else {
            panic!("orbit-fit camera must build");
        };
        camera
    }

    fn assert_screen_close_f32(actual_px_f32: [f32; 2], expected_px_f32: [f32; 2]) {
        for axis_usize in 0..2 {
            let diff_px_f32 = (actual_px_f32[axis_usize] - expected_px_f32[axis_usize]).abs();
            assert!(
                diff_px_f32 < SCREEN_TOL_PX_F32,
                "screen {actual_px_f32:?} differs from {expected_px_f32:?}"
            );
        }
    }

    fn assert_inside_viewport_px_f32(camera: &Camera2D, world_m: DVec3) {
        let screen_px_f32 = camera.world_to_screen(world_m);
        assert!(
            screen_px_f32[0].is_finite() && screen_px_f32[1].is_finite(),
            "screen must stay finite"
        );
        let viewport_px_f32 = camera.viewport_px_f32();
        assert!(
            screen_px_f32[0] >= 0.0
                && screen_px_f32[0] <= viewport_px_f32[0]
                && screen_px_f32[1] >= 0.0
                && screen_px_f32[1] <= viewport_px_f32[1],
            "world {world_m:?} maps outside {viewport_px_f32:?} at {screen_px_f32:?}"
        );
    }

    #[test]
    fn rebase_threshold_is_5000_m() {
        assert!(
            libm::fabs(ORIGIN_REBASE_DISTANCE_M - 5000.0) < METERS_TOL_F64,
            "rebase distance must stay 5000 m"
        );
    }

    #[test]
    fn rebase_fires_past_threshold_without_jump() {
        let Ok(camera) = Camera2D::new(DVec3::ZERO, 0.01, viewport_px_f32()) else {
            panic!("camera must build");
        };
        assert!(!camera.should_rebase(DVec3::new(ORIGIN_REBASE_DISTANCE_M - 1.0, 0.0, 0.0)));
        assert!(camera.should_rebase(DVec3::new(ORIGIN_REBASE_DISTANCE_M + 1.0, 0.0, 0.0)));
        let target_m = DVec3::new(6_000.0, 0.0, 0.0);
        let moved = camera.rebased(target_m);
        assert_eq!(moved.center_m(), target_m);
        assert_eq!(
            moved.scale_px_per_m_f64().to_bits(),
            camera.scale_px_per_m_f64().to_bits()
        );
        assert_screen_close_f32(
            moved.world_to_screen(target_m),
            [VIEWPORT_WIDTH_PX_F32 / 2.0, VIEWPORT_HEIGHT_PX_F32 / 2.0],
        );
    }

    #[test]
    fn mapping_is_linear_in_downrange_and_altitude() {
        let Ok(camera) = Camera2D::new(DVec3::ZERO, 0.01, viewport_px_f32()) else {
            panic!("camera must build");
        };
        let first_px_f32 = camera.world_to_screen(DVec3::ZERO);
        let second_px_f32 = camera.world_to_screen(DVec3::new(WORLD_STEP_M_F64, 0.0, 0.0));
        let third_px_f32 = camera.world_to_screen(DVec3::new(2.0 * WORLD_STEP_M_F64, 0.0, 0.0));
        let step_x_px_f32 = second_px_f32[0] - first_px_f32[0];
        assert!((step_x_px_f32 - (third_px_f32[0] - second_px_f32[0])).abs() < SCREEN_TOL_PX_F32);
        assert!((second_px_f32[1] - first_px_f32[1]).abs() < SCREEN_TOL_PX_F32);
        let north_px_f32 = camera.world_to_screen(DVec3::new(0.0, WORLD_STEP_M_F64, 0.0));
        assert!((north_px_f32[0] - first_px_f32[0]).abs() < SCREEN_TOL_PX_F32);
        assert!((north_px_f32[1] - first_px_f32[1] - step_x_px_f32).abs() < SCREEN_TOL_PX_F32);
        let deep_px_f32 = camera.world_to_screen(DVec3::new(0.0, 0.0, WORLD_STEP_M_F64));
        assert_screen_close_f32(deep_px_f32, first_px_f32);
        let mid_px_f32 = camera.world_to_screen(DVec3::new(WORLD_STEP_M_F64 / 2.0, 0.0, 0.0));
        assert!(
            (mid_px_f32[0] - f32::midpoint(first_px_f32[0], second_px_f32[0])).abs()
                < SCREEN_TOL_PX_F32
        );
    }

    #[test]
    fn conversion_stays_finite_across_orbit_view() {
        let camera = orbit_camera();
        let radius_m_f64 = test_body().radius_m().value();
        for world_m in [
            DVec3::ZERO,
            DVec3::new(radius_m_f64, 0.0, 0.0),
            DVec3::new(radius_m_f64 + RAILS_ALTITUDE_M, 0.0, 0.0),
            DVec3::new(radius_m_f64 + SHIP_ALTITUDE_M_F64, 0.0, 0.0),
            DVec3::new(0.0, radius_m_f64 + SHIP_ALTITUDE_M_F64, 0.0),
        ] {
            let screen_px_f32 = camera.world_to_screen(world_m);
            assert!(
                screen_px_f32[0].is_finite() && screen_px_f32[1].is_finite(),
                "world {world_m:?} must map to finite screen"
            );
        }
    }

    #[test]
    fn orbit_fit_contains_surface_and_rails_handoffs() {
        let camera = orbit_camera();
        let radius_m_f64 = test_body().radius_m().value();
        assert_inside_viewport_px_f32(&camera, DVec3::ZERO);
        assert_inside_viewport_px_f32(&camera, DVec3::new(radius_m_f64, 0.0, 0.0));
        assert_inside_viewport_px_f32(
            &camera,
            DVec3::new(radius_m_f64 + RAILS_ALTITUDE_M, 0.0, 0.0),
        );
        assert_inside_viewport_px_f32(
            &camera,
            DVec3::new(radius_m_f64 + SHIP_ALTITUDE_M_F64, 0.0, 0.0),
        );
    }

    #[test]
    fn entry_corridor_contains_handoff_band() {
        let Ok(camera) = Camera2D::entry_corridor(&test_body(), viewport_px_f32()) else {
            panic!("entry camera must build");
        };
        let radius_m_f64 = test_body().radius_m().value();
        assert_inside_viewport_px_f32(&camera, DVec3::new(radius_m_f64, 0.0, 0.0));
        assert_inside_viewport_px_f32(
            &camera,
            DVec3::new(radius_m_f64 + RAILS_ALTITUDE_M, 0.0, 0.0),
        );
        assert_screen_close_f32(
            camera.world_to_screen(DVec3::new(radius_m_f64 + RAILS_ALTITUDE_M / 2.0, 0.0, 0.0)),
            [VIEWPORT_WIDTH_PX_F32 / 2.0, VIEWPORT_HEIGHT_PX_F32 / 2.0],
        );
    }

    #[test]
    fn surface_grid_contains_touchdown_patch() {
        let Ok(camera) = Camera2D::surface_grid(&test_body(), viewport_px_f32()) else {
            panic!("surface camera must build");
        };
        let radius_m_f64 = test_body().radius_m().value();
        assert_screen_close_f32(
            camera.world_to_screen(DVec3::new(radius_m_f64, 0.0, 0.0)),
            [VIEWPORT_WIDTH_PX_F32 / 2.0, VIEWPORT_HEIGHT_PX_F32 / 2.0],
        );
        assert_inside_viewport_px_f32(
            &camera,
            DVec3::new(radius_m_f64 + SURFACE_GRID_HALF_EXTENT_M, 0.0, 0.0),
        );
        assert_inside_viewport_px_f32(
            &camera,
            DVec3::new(radius_m_f64 - SURFACE_GRID_HALF_EXTENT_M, 0.0, 0.0),
        );
    }

    #[test]
    fn handoff_mapping_has_no_jump() {
        let Ok(entry) = Camera2D::entry_corridor(&test_body(), viewport_px_f32()) else {
            panic!("entry camera must build");
        };
        for camera in [orbit_camera(), entry] {
            let radius_m_f64 = test_body().radius_m().value();
            let below_px_f32 =
                camera.world_to_screen(DVec3::new(radius_m_f64 + RAILS_ALTITUDE_M - 1.0, 0.0, 0.0));
            let above_px_f32 =
                camera.world_to_screen(DVec3::new(radius_m_f64 + RAILS_ALTITUDE_M + 1.0, 0.0, 0.0));
            let gap_px_f32 = (above_px_f32[0] - below_px_f32[0]).abs();
            let expected_px_f64 = 2.0 * camera.scale_px_per_m_f64();
            assert!(
                (f64::from(gap_px_f32) - expected_px_f64).abs() < f64::from(SCREEN_TOL_PX_F32),
                "handoff gap {gap_px_f32} must equal linear step {expected_px_f64}"
            );
            let on_px_f32 =
                camera.world_to_screen(DVec3::new(radius_m_f64 + RAILS_ALTITUDE_M, 0.0, 0.0));
            assert!(
                (f32::midpoint(below_px_f32[0], above_px_f32[0]) - on_px_f32[0]).abs()
                    < SCREEN_TOL_PX_F32,
                "handoff midpoint must stay linear"
            );
            let surface_px_f32 = camera.world_to_screen(DVec3::new(radius_m_f64 - 1.0, 0.0, 0.0));
            let lifted_px_f32 = camera.world_to_screen(DVec3::new(radius_m_f64 + 1.0, 0.0, 0.0));
            let surface_gap_px_f32 = (lifted_px_f32[0] - surface_px_f32[0]).abs();
            assert!(
                (f64::from(surface_gap_px_f32) - expected_px_f64).abs()
                    < f64::from(SCREEN_TOL_PX_F32),
                "surface gap {surface_gap_px_f32} must equal linear step {expected_px_f64}"
            );
        }
    }

    #[test]
    fn batch_fill_matches_single_without_alloc() {
        let camera = orbit_camera();
        let radius_m_f64 = test_body().radius_m().value();
        let worlds_m = [
            DVec3::ZERO,
            DVec3::new(radius_m_f64, 0.0, 0.0),
            DVec3::new(radius_m_f64 + RAILS_ALTITUDE_M, 0.0, 0.0),
        ];
        let mut screens_px_f32 = [[0.0_f32; 2]; 3];
        let Ok(()) = camera.convert_into(&worlds_m, &mut screens_px_f32) else {
            panic!("batch fill must succeed");
        };
        for (world_m, screen_px_f32) in worlds_m.iter().zip(screens_px_f32.iter()) {
            assert_screen_close_f32(*screen_px_f32, camera.world_to_screen(*world_m));
        }
        let mut short_px_f32 = [[0.0_f32; 2]; 2];
        assert!(matches!(
            camera.convert_into(&worlds_m, &mut short_px_f32),
            Err(RenderError::BatchLengthMismatch { .. })
        ));
    }

    #[test]
    fn constructor_rejects_bad_inputs() {
        assert!(matches!(
            Camera2D::new(DVec3::new(f64::NAN, 0.0, 0.0), 0.01, viewport_px_f32()),
            Err(RenderError::NonFinite { .. })
        ));
        assert!(matches!(
            Camera2D::new(DVec3::ZERO, 0.0, viewport_px_f32()),
            Err(RenderError::InvalidScale { .. })
        ));
        assert!(matches!(
            Camera2D::new(DVec3::ZERO, f64::INFINITY, viewport_px_f32()),
            Err(RenderError::InvalidScale { .. })
        ));
        assert!(matches!(
            Camera2D::new(DVec3::ZERO, 0.01, [0.0, VIEWPORT_HEIGHT_PX_F32]),
            Err(RenderError::InvalidViewport { .. })
        ));
        assert!(matches!(
            Camera2D::new(DVec3::ZERO, 0.01, [f32::NAN, VIEWPORT_HEIGHT_PX_F32]),
            Err(RenderError::InvalidViewport { .. })
        ));
        assert!(matches!(
            Camera2D::orbit_fit(&test_body(), Meters::new(f64::NAN), viewport_px_f32()),
            Err(RenderError::InvalidAltitude { .. })
        ));
        assert!(matches!(
            Camera2D::entry_corridor(&test_body(), [VIEWPORT_WIDTH_PX_F32, 0.0]),
            Err(RenderError::InvalidViewport { .. })
        ));
    }
}
