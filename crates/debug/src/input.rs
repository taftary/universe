//! Shell input router with passthrough and focused modes.
//!
//! Plain data over [`ShellInputMode`](crate::layout::ShellInputMode) with desktop
//! `F3` and `Escape` keys, phone dev-tag long-press and three-finger tap, the
//! `wants_pointer` and `wants_keyboard` routing rule, and tap-pick point
//! tolerance display. Mode state only; never touches sim state.

use crate::layout::ShellInputMode;

/// Tap-pick point tolerance in points, scaled by `pixels_per_point`.
///
/// Source: `docs/tech/debug.md` section 5 tap-pick point rule.
pub const TAP_PICK_TOLERANCE_PT_F32: f32 = 8.0;

/// Dev-tag long-press threshold in seconds for one-handed shell open.
///
/// Source: `docs/tech/debug.md` section 5 phone rule.
pub const DEV_TAG_LONG_PRESS_S_F64: f64 = 0.5;

/// Three-finger tap count for shell toggle, dimensionless.
///
/// Source: `docs/tech/debug.md` section 5 phone rule.
pub const THREE_FINGER_TAP_COUNT_U8: u8 = 3;

/// Input-router range-check failures.
///
/// Returned for non-finite or negative durations and tolerances.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputRouterError {
    /// Duration or tolerance was non-finite.
    NonFinite {
        /// Rejected value.
        value_f64: f64,
    },
    /// Duration or tolerance was negative.
    Negative {
        /// Rejected value.
        value_f64: f64,
    },
}

impl core::fmt::Display for InputRouterError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NonFinite { value_f64 } => {
                write!(formatter, "non-finite input-router value: {value_f64}")
            }
            Self::Negative { value_f64 } => {
                write!(formatter, "negative input-router value: {value_f64}")
            }
        }
    }
}

impl std::error::Error for InputRouterError {}

/// Desktop key driving router transitions.
///
/// `F3` toggles focus; `Escape` returns to passthrough.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouterKey {
    /// Toggle between passthrough and focused.
    F3,
    /// Return to passthrough and close any modal.
    Escape,
}

impl RouterKey {
    /// All router keys.
    pub const ALL: [Self; 2] = [Self::F3, Self::Escape];

    /// Return the short key label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::F3 => "f3",
            Self::Escape => "escape",
        }
    }
}

/// Per-frame routing decision for one input frame.
///
/// Decided after the shell checks its `wants` flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputRoute {
    /// Game consumes the frame; shell keeps badges only.
    Game,
    /// Shell consumes the frame; game sees no copy.
    Shell,
}

impl InputRoute {
    /// Return the short route label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Game => "game",
            Self::Shell => "shell",
        }
    }

    /// Report whether the shell consumes the frame.
    #[must_use]
    pub const fn shell_consumes(self) -> bool {
        match self {
            Self::Game => false,
            Self::Shell => true,
        }
    }

    /// Report whether the game still sees a copy.
    #[must_use]
    pub const fn game_sees_copy(self) -> bool {
        match self {
            Self::Game => true,
            Self::Shell => false,
        }
    }
}

/// Shell input router with mode, modal, and pick tolerance.
///
/// Plain data only; holds the routing mode plus the tap-pick tolerance
/// shown in the inspector. Shell state only and never persists.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InputRouter {
    /// Current routing mode.
    mode: ShellInputMode,
    /// True while a shell modal dialog is open.
    modal_open: bool,
    /// Tap-pick point tolerance in points.
    pick_tolerance_pt_f32: f32,
}

impl InputRouter {
    /// Build a passthrough router with default pick tolerance.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            mode: ShellInputMode::Passthrough,
            modal_open: false,
            pick_tolerance_pt_f32: TAP_PICK_TOLERANCE_PT_F32,
        }
    }

    /// Return the current routing mode.
    #[must_use]
    pub const fn mode(self) -> ShellInputMode {
        self.mode
    }

    /// Report whether the game gets all input.
    #[must_use]
    pub const fn is_passthrough(self) -> bool {
        match self.mode {
            ShellInputMode::Passthrough => true,
            ShellInputMode::Focused => false,
        }
    }

    /// Report whether shell widgets get input first.
    #[must_use]
    pub const fn is_focused(self) -> bool {
        !self.is_passthrough()
    }

    /// Report whether a shell modal dialog is open.
    #[must_use]
    pub const fn is_modal_open(self) -> bool {
        self.modal_open
    }

    /// Mark a shell modal dialog open for modal-first escape.
    pub const fn open_modal(&mut self) {
        self.modal_open = true;
    }

    /// Mark the shell modal dialog closed.
    pub const fn close_modal(&mut self) {
        self.modal_open = false;
    }

    /// Handle a desktop toggle key and return the new mode.
    ///
    /// `F3` toggles between passthrough and focused. `Escape` closes any
    /// modal first and always returns to passthrough.
    pub const fn on_key(&mut self, key: RouterKey) -> ShellInputMode {
        match key {
            RouterKey::F3 => {
                self.mode = self.mode.toggle();
            }
            RouterKey::Escape => {
                self.modal_open = false;
                self.mode = ShellInputMode::Passthrough;
            }
        }
        self.mode
    }

    /// Handle a dev-tag long-press and return the new mode.
    ///
    /// Toggles only when the press reaches the long-press threshold;
    /// shorter presses leave the mode unchanged.
    ///
    /// # Errors
    ///
    /// Returns [`InputRouterError`] when `duration_s_f64` is non-finite or negative.
    pub fn on_dev_tag_long_press(
        &mut self,
        duration_s_f64: f64,
    ) -> Result<ShellInputMode, InputRouterError> {
        if !duration_s_f64.is_finite() {
            return Err(InputRouterError::NonFinite {
                value_f64: duration_s_f64,
            });
        }
        if duration_s_f64 < 0.0 {
            return Err(InputRouterError::Negative {
                value_f64: duration_s_f64,
            });
        }
        if duration_s_f64 >= DEV_TAG_LONG_PRESS_S_F64 {
            self.mode = self.mode.toggle();
        }
        Ok(self.mode)
    }

    /// Handle a multi-finger tap and return the new mode.
    ///
    /// Toggles only on a three-finger tap; other counts are picks or
    /// system gestures and leave the mode unchanged.
    pub const fn on_multi_finger_tap(&mut self, finger_count_u8: u8) -> ShellInputMode {
        if finger_count_u8 == THREE_FINGER_TAP_COUNT_U8 {
            self.mode = self.mode.toggle();
        }
        self.mode
    }

    /// Decide one frame under the `wants` routing rule.
    ///
    /// In passthrough the game always consumes except the toggle gesture,
    /// which bypasses the frame. In focused the shell consumes when it
    /// wants pointer or keyboard input; otherwise the game consumes.
    #[must_use]
    pub const fn route(self, wants_pointer_input: bool, wants_keyboard_input: bool) -> InputRoute {
        match self.mode {
            ShellInputMode::Passthrough => InputRoute::Game,
            ShellInputMode::Focused => {
                if wants_pointer_input || wants_keyboard_input {
                    InputRoute::Shell
                } else {
                    InputRoute::Game
                }
            }
        }
    }

    /// Return the tap-pick point tolerance in points.
    #[must_use]
    pub const fn pick_tolerance_pt_f32(self) -> f32 {
        self.pick_tolerance_pt_f32
    }

    /// Set the tap-pick point tolerance shown in the inspector.
    ///
    /// # Errors
    ///
    /// Returns [`InputRouterError`] when `tolerance_pt_f32` is non-finite or negative.
    pub fn set_pick_tolerance(&mut self, tolerance_pt_f32: f32) -> Result<(), InputRouterError> {
        let value_f64 = f64::from(tolerance_pt_f32);
        if !value_f64.is_finite() {
            return Err(InputRouterError::NonFinite { value_f64 });
        }
        if value_f64 < 0.0 {
            return Err(InputRouterError::Negative { value_f64 });
        }
        self.pick_tolerance_pt_f32 = tolerance_pt_f32;
        Ok(())
    }

    /// Draw the router mode plus pick tolerance.
    ///
    /// Immediate-mode widgets only; creates no renderer. Step 5 owns
    /// renderer creation. Available only with `dev-shell`.
    #[cfg(feature = "dev-shell")]
    pub fn draw(self, ui: &mut egui::Ui) {
        ui.label(format!(
            "input mode={mode} pick_tol_pt={tol:.1}",
            mode = self.mode.label(),
            tol = self.pick_tolerance_pt_f32
        ));
    }
}

impl Default for InputRouter {
    /// Default router in passthrough with default tolerance.
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHORT_PRESS_S_F64: f64 = 0.1;
    const SMOKE_TOLERANCE_PT_F32: f32 = 12.0;
    const NEGATIVE_TOLERANCE_PT_F32: f32 = -1.0;
    const TWO_FINGER_COUNT_U8: u8 = 2;

    #[test]
    fn new_router_starts_in_passthrough() {
        let router = InputRouter::new();
        assert_eq!(router.mode(), ShellInputMode::Passthrough);
        assert!(router.is_passthrough());
        assert!(!router.is_focused());
        assert!(!router.is_modal_open());
        assert!((router.pick_tolerance_pt_f32() - TAP_PICK_TOLERANCE_PT_F32).abs() < f32::EPSILON);
        assert_eq!(InputRouter::default(), router);
        assert_eq!(RouterKey::ALL.len(), 2);
    }

    #[test]
    fn f3_toggles_both_ways() {
        let mut router = InputRouter::new();
        assert_eq!(router.on_key(RouterKey::F3), ShellInputMode::Focused);
        assert!(router.is_focused());
        assert_eq!(router.on_key(RouterKey::F3), ShellInputMode::Passthrough);
        assert!(router.is_passthrough());
        assert_eq!(RouterKey::F3.label(), "f3");
        assert_eq!(RouterKey::Escape.label(), "escape");
    }

    #[test]
    fn escape_closes_modal_and_returns_to_passthrough() {
        let mut router = InputRouter::new();
        router.on_key(RouterKey::F3);
        router.open_modal();
        assert!(router.is_modal_open());
        assert_eq!(
            router.on_key(RouterKey::Escape),
            ShellInputMode::Passthrough
        );
        assert!(router.is_passthrough());
        assert!(!router.is_modal_open());
        router.close_modal();
        assert_eq!(
            router.on_key(RouterKey::Escape),
            ShellInputMode::Passthrough
        );
    }

    #[test]
    fn long_press_toggles_only_at_threshold() {
        let mut router = InputRouter::new();
        let Ok(short) = router.on_dev_tag_long_press(SHORT_PRESS_S_F64) else {
            panic!("short press must measure")
        };
        assert_eq!(short, ShellInputMode::Passthrough);
        let Ok(held) = router.on_dev_tag_long_press(DEV_TAG_LONG_PRESS_S_F64) else {
            panic!("threshold press must toggle")
        };
        assert_eq!(held, ShellInputMode::Focused);
        assert!(matches!(
            router.on_dev_tag_long_press(f64::NAN),
            Err(InputRouterError::NonFinite { .. })
        ));
        assert!(matches!(
            router.on_dev_tag_long_press(-0.5),
            Err(InputRouterError::Negative { .. })
        ));
        assert!(router.is_focused());
    }

    #[test]
    fn three_finger_tap_toggles_other_counts_ignore() {
        let mut router = InputRouter::new();
        assert_eq!(
            router.on_multi_finger_tap(TWO_FINGER_COUNT_U8),
            ShellInputMode::Passthrough
        );
        assert_eq!(
            router.on_multi_finger_tap(THREE_FINGER_TAP_COUNT_U8),
            ShellInputMode::Focused
        );
        assert_eq!(
            router.on_multi_finger_tap(THREE_FINGER_TAP_COUNT_U8),
            ShellInputMode::Passthrough
        );
    }

    #[test]
    fn route_follows_wants_rule() {
        let passthrough = InputRouter::new();
        for wants_pointer in [false, true] {
            for wants_keyboard in [false, true] {
                assert_eq!(
                    passthrough.route(wants_pointer, wants_keyboard),
                    InputRoute::Game
                );
            }
        }
        let mut focused = InputRouter::new();
        focused.on_key(RouterKey::F3);
        assert_eq!(focused.route(false, false), InputRoute::Game);
        assert_eq!(focused.route(true, false), InputRoute::Shell);
        assert_eq!(focused.route(false, true), InputRoute::Shell);
        assert_eq!(focused.route(true, true), InputRoute::Shell);
        let shell = InputRoute::Shell;
        assert!(shell.shell_consumes());
        assert!(!shell.game_sees_copy());
        let game = InputRoute::Game;
        assert!(!game.shell_consumes());
        assert!(game.game_sees_copy());
        assert_eq!(game.label(), "game");
        assert_eq!(shell.label(), "shell");
    }

    #[test]
    fn tolerance_sets_and_rejects_bad_values() {
        let mut router = InputRouter::new();
        assert!(
            router.set_pick_tolerance(SMOKE_TOLERANCE_PT_F32).is_ok(),
            "smoke tolerance must set"
        );
        assert!((router.pick_tolerance_pt_f32() - SMOKE_TOLERANCE_PT_F32).abs() < f32::EPSILON);
        assert!(matches!(
            router.set_pick_tolerance(f32::NAN),
            Err(InputRouterError::NonFinite { .. })
        ));
        assert!(matches!(
            router.set_pick_tolerance(NEGATIVE_TOLERANCE_PT_F32),
            Err(InputRouterError::Negative { .. })
        ));
        assert!((router.pick_tolerance_pt_f32() - SMOKE_TOLERANCE_PT_F32).abs() < f32::EPSILON);
    }
}
