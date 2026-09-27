//! Debug shell assembly over top bar, cost meter, router, and inspect.
//!
//! Owns [`TopBarState`](crate::top_bar::TopBarState),
//! [`ShellCostMeter`](crate::shell_cost::ShellCostMeter),
//! [`InputRouter`](crate::input::InputRouter),
//! [`InspectView`](crate::inspect_view::InspectView), and dock visibility with
//! a `Passthrough` default. Records shell draw cost separately from sim and
//! render cost; never writes sim state; removed by close flag or by building
//! without `dev-shell`.

use crate::input::{
    InputRoute, InputRouter, InputRouterError, RouterKey, TAP_PICK_TOLERANCE_PT_F32,
};
use crate::inspect_view::{InspectView, InspectViewError};
use crate::layout::{DesktopPreset, PanelVisibility, ShellInputMode};
use crate::shell_cost::{ShellCostError, ShellCostMeter};
use crate::top_bar::{TopBarError, TopBarState};

#[cfg(feature = "dev-shell")]
use engine::inspect::SimSnapshot;

/// Shell assembly failures from the owned parts.
///
/// Returned for bad draw costs, bad tolerances, bad snapshot codes, and
/// bad snapshot clocks. Never a sim write failure; the shell is read-only.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShellError {
    /// Top-bar clock, warp, or cost check failed.
    TopBar(TopBarError),
    /// Shell-cost sample or budget check failed.
    ShellCost(ShellCostError),
    /// Input-router duration or tolerance check failed.
    Input(InputRouterError),
    /// Inspect code map or tolerance check failed.
    Inspect(InspectViewError),
}

impl core::fmt::Display for ShellError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TopBar(source) => write!(formatter, "shell top bar: {source}"),
            Self::ShellCost(source) => write!(formatter, "shell cost: {source}"),
            Self::Input(source) => write!(formatter, "shell input: {source}"),
            Self::Inspect(source) => write!(formatter, "shell inspect: {source}"),
        }
    }
}

impl std::error::Error for ShellError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::TopBar(source) => Some(source),
            Self::ShellCost(source) => Some(source),
            Self::Input(source) => Some(source),
            Self::Inspect(source) => Some(source),
        }
    }
}

impl From<TopBarError> for ShellError {
    /// Convert a top-bar failure into a shell failure.
    fn from(source: TopBarError) -> Self {
        Self::TopBar(source)
    }
}

impl From<ShellCostError> for ShellError {
    /// Convert a shell-cost failure into a shell failure.
    fn from(source: ShellCostError) -> Self {
        Self::ShellCost(source)
    }
}

impl From<InputRouterError> for ShellError {
    /// Convert an input-router failure into a shell failure.
    fn from(source: InputRouterError) -> Self {
        Self::Input(source)
    }
}

impl From<InspectViewError> for ShellError {
    /// Convert an inspect-view failure into a shell failure.
    fn from(source: InspectViewError) -> Self {
        Self::Inspect(source)
    }
}

/// Debug shell assembly with run control, cost, input, and inspect.
///
/// Plain state only; observes snapshot scalars by copy and records its
/// own draw cost. Shell state only and never persists.
#[derive(Debug, Clone)]
pub struct Shell {
    /// Run control plus clocks and badges.
    top_bar: TopBarState,
    /// Shell draw-cost meter with close flag.
    meter: ShellCostMeter,
    /// Input router with passthrough default.
    router: InputRouter,
    /// Read-only inspect view over snapshot scalars.
    inspect: InspectView,
    /// Dock visibility for the current preset.
    visibility: PanelVisibility,
}

impl Shell {
    /// Build a passthrough shell with ticker-only visibility.
    ///
    /// Starts closed-never: badges only, game keeps all input, top bar
    /// alone for minimal occlusion.
    ///
    /// # Errors
    ///
    /// Returns [`ShellError`] when the default pick tolerance is rejected.
    pub fn new() -> Result<Self, ShellError> {
        Ok(Self {
            top_bar: TopBarState::new(),
            meter: ShellCostMeter::new(),
            router: InputRouter::new(),
            inspect: InspectView::new(TAP_PICK_TOLERANCE_PT_F32)?,
            visibility: PanelVisibility::for_preset(DesktopPreset::TickerOnly),
        })
    }

    /// Return the current input-routing mode.
    #[must_use]
    pub const fn mode(&self) -> ShellInputMode {
        self.router.mode()
    }

    /// Report whether the game gets all input.
    #[must_use]
    pub const fn is_passthrough(&self) -> bool {
        self.router.is_passthrough()
    }

    /// Report whether shell widgets get input first.
    #[must_use]
    pub const fn is_focused(&self) -> bool {
        self.router.is_focused()
    }

    /// Report whether the close-shell button was pressed.
    #[must_use]
    pub const fn is_closed(&self) -> bool {
        self.meter.is_closed()
    }

    /// Return run control plus clocks and badges.
    #[must_use]
    pub const fn top_bar(&self) -> &TopBarState {
        &self.top_bar
    }

    /// Return run control for pause, step, and warp.
    ///
    /// Shell state only; run control never writes sim state directly.
    pub const fn top_bar_mut(&mut self) -> &mut TopBarState {
        &mut self.top_bar
    }

    /// Return the shell draw-cost meter.
    #[must_use]
    pub const fn meter(&self) -> &ShellCostMeter {
        &self.meter
    }

    /// Return the input router.
    #[must_use]
    pub const fn router(&self) -> &InputRouter {
        &self.router
    }

    /// Return the read-only inspect view.
    #[must_use]
    pub const fn inspect(&self) -> &InspectView {
        &self.inspect
    }

    /// Return dock visibility for the current preset.
    #[must_use]
    pub const fn visibility(&self) -> PanelVisibility {
        self.visibility
    }

    /// Switch dock visibility without changing content.
    pub const fn set_visibility(&mut self, visibility: PanelVisibility) {
        self.visibility = visibility;
    }

    /// Handle a desktop toggle key and return the new mode.
    pub const fn handle_key(&mut self, key: RouterKey) -> ShellInputMode {
        self.router.on_key(key)
    }

    /// Handle a dev-tag long-press and return the new mode.
    ///
    /// # Errors
    ///
    /// Returns [`ShellError`] when `duration_s_f64` is non-finite or negative.
    pub fn handle_long_press(&mut self, duration_s_f64: f64) -> Result<ShellInputMode, ShellError> {
        Ok(self.router.on_dev_tag_long_press(duration_s_f64)?)
    }

    /// Handle a multi-finger tap and return the new mode.
    pub const fn handle_tap(&mut self, finger_count_u8: u8) -> ShellInputMode {
        self.router.on_multi_finger_tap(finger_count_u8)
    }

    /// Decide one frame under the `wants` routing rule.
    #[must_use]
    pub const fn route(&self, wants_pointer_input: bool, wants_keyboard_input: bool) -> InputRoute {
        self.router.route(wants_pointer_input, wants_keyboard_input)
    }

    /// Record one shell draw sample as the cost timing hook.
    ///
    /// Updates the meter ring plus the top-bar shell badge. Call with the
    /// measured draw milliseconds after each shell draw; pass
    /// `FRAME_BUDGET_MS` at draw time for fractions, never stored here.
    ///
    /// # Errors
    ///
    /// Returns [`ShellError`] when `draw_ms_f64` is non-finite or negative.
    pub fn record_draw_cost(&mut self, draw_ms_f64: f64) -> Result<(), ShellError> {
        self.meter.record_sample(draw_ms_f64)?;
        self.top_bar.set_shell_cost(draw_ms_f64)?;
        Ok(())
    }

    /// Return the latest shell draw cost in milliseconds.
    #[must_use]
    pub const fn draw_cost_ms_f64(&self) -> f64 {
        self.meter.latest_draw_ms_f64()
    }

    /// Return the mean shell draw cost in milliseconds.
    #[must_use]
    pub fn average_draw_ms_f64(&self) -> f64 {
        self.meter.average_ms_f64()
    }

    /// Observe a snapshot by copy without writing sim state.
    ///
    /// Forwards clocks plus warp, seed, and hash to the top bar and
    /// replaces the inspect view. Available only with `dev-shell`.
    ///
    /// # Errors
    ///
    /// Returns [`ShellError`] for bad snapshot clocks or unknown codes.
    #[cfg(feature = "dev-shell")]
    pub fn observe_snapshot(&mut self, snapshot: &SimSnapshot) -> Result<(), ShellError> {
        let view = InspectView::from_snapshot(snapshot, self.router.pick_tolerance_pt_f32())?;
        self.top_bar.observe_snapshot_view(
            snapshot.tick_count_u64,
            snapshot.elapsed_s_f64,
            snapshot.warp_code_u8,
            snapshot.drop_reason_u8,
            snapshot.master_seed_u64,
            snapshot.snapshot_hash_u64,
        )?;
        self.inspect = view;
        Ok(())
    }

    /// Mark the shell closed via the close-shell button.
    pub const fn request_close(&mut self) {
        self.meter.request_close();
    }

    /// Reopen the shell after a close.
    pub const fn reopen(&mut self) {
        self.meter.reopen();
    }

    /// Draw top bar, router, and inspector in one shell pass.
    ///
    /// Immediate-mode widgets only; creates no renderer. Step 5 owns
    /// renderer creation. Skips everything when closed. Available only
    /// with the non-default `dev-shell` feature.
    #[cfg(feature = "dev-shell")]
    pub fn draw(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, frame_budget_ms_f64: f64) {
        if self.meter.is_closed() {
            return;
        }
        self.top_bar
            .draw(ctx, ui, &mut self.meter, frame_budget_ms_f64);
        self.router.draw(ui);
        self.inspect.draw(ui);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::DesktopPreset;

    const SMOKE_DRAW_MS_F64: f64 = 0.4;
    const SMOKE_BUDGET_MS_F64: f64 = 32.0;
    const EXPECTED_FRACTION_F64: f64 = 0.0125;
    const FRACTION_TOL_F64: f64 = 1e-12;

    fn smoke_shell() -> Shell {
        let Ok(shell) = Shell::new() else {
            panic!("smoke shell must build")
        };
        shell
    }

    #[test]
    fn new_shell_defaults_to_passthrough_ticker() {
        let shell = smoke_shell();
        assert_eq!(shell.mode(), ShellInputMode::Passthrough);
        assert!(shell.is_passthrough());
        assert!(!shell.is_focused());
        assert!(!shell.is_closed());
        assert_eq!(
            shell.visibility(),
            PanelVisibility::for_preset(DesktopPreset::TickerOnly)
        );
        assert!(shell.visibility().shows_top_bar());
        assert!(!shell.visibility().shows_left_panel());
        assert_eq!(shell.top_bar().tick_count_u64(), 0);
        assert!(shell.meter().is_empty());
        assert_eq!(shell.router().mode(), ShellInputMode::Passthrough);
        assert_eq!(shell.inspect().tick_count_u64(), 0);
        assert!((shell.draw_cost_ms_f64() - 0.0).abs() < FRACTION_TOL_F64);
    }

    #[test]
    fn keys_and_gestures_route_through_shell() {
        let mut shell = smoke_shell();
        assert_eq!(shell.handle_key(RouterKey::F3), ShellInputMode::Focused);
        assert!(shell.is_focused());
        assert_eq!(shell.route(false, false), InputRoute::Game);
        assert_eq!(shell.route(true, false), InputRoute::Shell);
        let Ok(mode) = shell.handle_long_press(0.5) else {
            panic!("threshold press must route")
        };
        assert_eq!(mode, ShellInputMode::Passthrough);
        assert_eq!(shell.handle_tap(3), ShellInputMode::Focused);
        shell.top_bar_mut().pause();
        assert!(shell.top_bar().is_paused());
    }

    #[test]
    fn draw_cost_hook_updates_meter_and_badge() {
        let mut shell = smoke_shell();
        assert!(
            shell.record_draw_cost(SMOKE_DRAW_MS_F64).is_ok(),
            "smoke cost must record"
        );
        assert!((shell.draw_cost_ms_f64() - SMOKE_DRAW_MS_F64).abs() < FRACTION_TOL_F64);
        assert!((shell.average_draw_ms_f64() - SMOKE_DRAW_MS_F64).abs() < FRACTION_TOL_F64);
        assert!((shell.top_bar().shell_draw_ms_f64() - SMOKE_DRAW_MS_F64).abs() < FRACTION_TOL_F64);
        let Ok(fraction) = shell.meter().fraction_of_budget(SMOKE_BUDGET_MS_F64) else {
            panic!("smoke fraction must divide")
        };
        assert!((fraction - EXPECTED_FRACTION_F64).abs() < FRACTION_TOL_F64);
        assert!(matches!(
            shell.record_draw_cost(f64::NAN),
            Err(ShellError::ShellCost(_))
        ));
        assert!(matches!(
            shell.handle_long_press(f64::NAN),
            Err(ShellError::Input(_))
        ));
    }

    #[test]
    fn close_flag_removes_shell_and_reopen_restores() {
        let mut shell = smoke_shell();
        shell.request_close();
        assert!(shell.is_closed());
        shell.reopen();
        assert!(!shell.is_closed());
        shell.set_visibility(PanelVisibility::for_preset(DesktopPreset::Descent));
        assert!(shell.visibility().shows_left_panel());
        assert!(shell.visibility().shows_bottom_tabs());
    }

    #[test]
    fn shell_error_labels_each_part() {
        let top = ShellError::from(TopBarError::NonFinite { value_f64: 1.0 });
        assert!(format!("{top}").contains("top bar"));
        let cost = ShellError::from(ShellCostError::Negative { value_f64: 1.0 });
        assert!(format!("{cost}").contains("cost"));
        let input = ShellError::from(InputRouterError::Negative { value_f64: 1.0 });
        assert!(format!("{input}").contains("input"));
        let inspect = ShellError::from(InspectViewError::Negative { value_f64: 1.0 });
        assert!(format!("{inspect}").contains("inspect"));
        assert!(std::error::Error::source(&top).is_some());
    }

    #[cfg(feature = "dev-shell")]
    #[test]
    fn observe_snapshot_replaces_view_and_clocks() {
        use crate::inspect_view::{
            INSPECT_DROP_NONE_U8, INSPECT_FRAME_DEPTH_M1_U8, INSPECT_FRAME_ORBIT_U8,
            INSPECT_MARK_SHIP_POINT_U8, INSPECT_REGIME_ORBIT_U8, INSPECT_WARP_X1_U8,
        };
        let mut shell = smoke_shell();
        let snapshot = SimSnapshot {
            tick_count_u64: 7,
            elapsed_s_f64: 0.35,
            ship_epoch_s_f64: 0.35,
            master_seed_u64: 0x1234_ABCD_5678_EF90,
            stream_seed_u64: 9,
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
            warp_code_u8: INSPECT_WARP_X1_U8,
            drop_reason_u8: INSPECT_DROP_NONE_U8,
            warp_flags_u8: 3,
            regime_u8: INSPECT_REGIME_ORBIT_U8,
            frame_level_u8: INSPECT_FRAME_ORBIT_U8,
            frame_depth_u8: INSPECT_FRAME_DEPTH_M1_U8,
            elements_valid_u8: 1,
            pick_valid_u8: 1,
            mark_kind_u8: INSPECT_MARK_SHIP_POINT_U8,
            _pad_u8: [0_u8; 3],
        };
        assert!(
            shell.observe_snapshot(&snapshot).is_ok(),
            "smoke snapshot must observe"
        );
        assert_eq!(shell.inspect().tick_count_u64(), 7);
        assert_eq!(shell.top_bar().tick_count_u64(), 7);
        assert_eq!(shell.inspect().mark_label(), "ship-point");
        assert!(shell.inspect().pick_valid());
        let mut bad = snapshot;
        bad.warp_code_u8 = 9;
        assert!(matches!(
            shell.observe_snapshot(&bad),
            Err(ShellError::Inspect(_))
        ));
    }
}
