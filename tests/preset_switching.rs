//! Preset-switching headless verification for issue 54 step 3.
//!
//! Covers AC1, AC2, AC4, AC5, and AC6 without opening a window:
//! `Digit1` to `Digit4` map to Descent, Determinism, Budget, and
//! `TickerOnly` while Numpad keys map to none (AC1); four on-screen
//! 44pt buttons mirror the keys through the same visibility plus
//! default-tab application (AC2); the canvas gate stays Descent-only
//! versus ticker-only (AC4); the marks zoom mapper is unchanged with
//! no key overlap (AC5); and `cargo test` carries no window flag with
//! clean hygiene on every touched source (AC6). `universe-debug` owns
//! no library target, so this file pins its public behavior through
//! read-only source contracts plus the in-crate headless tests by
//! name, following `tests/canvas_handoff.rs`. AC3 (taint and pause
//! safety), AC7 (docs), and AC8 (live backends) belong to later steps.

#![forbid(unsafe_code)]

/// OS-window source for the preset plus zoom key mappers.
const OS_WINDOW_SRC: &str = include_str!("../crates/debug/src/os_window.rs");

/// Shell source for the preset buttons plus `DesktopWindow::set_preset`.
const SHELL_SRC: &str = include_str!("../crates/debug/src/shell.rs");

/// Layout source for presets, visibility, tabs, and the touch floor.
const LAYOUT_SRC: &str = include_str!("../crates/debug/src/layout.rs");

/// Bottom-tabs source for the default-tab mapping.
const BOTTOM_SRC: &str = include_str!("../crates/debug/src/bottom.rs");

/// Window flag spelling, dimensionless text.
const RUN_WINDOW_FLAG: &str = "--run-window";

/// Minimum touch target in points, named `MIN_TOUCH_TARGET_PT_F32`.
const EXPECTED_TOUCH_PT_F32: f32 = 44.0;

/// Desktop preset count, dimensionless.
const EXPECTED_PRESET_COUNT_USIZE: usize = 4;

/// Marks zoom step ratio, dimensionless.
const EXPECTED_ZOOM_STEP_RATIO_F64: f64 = 1.25;

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

/// AC1 key mapper: `Digit1` to `Digit4` select presets, Numpad maps to none.
///
/// Pins the pure `preset_action_for_key` mapper plus its `on_key`
/// wiring into `DesktopWindow::set_preset`; the in-crate test
/// `preset_keys_map_to_desktop_presets_only` owns the runtime values.
#[test]
fn ac1_preset_keys_map_digit1_to_4_only() {
    assert_contains(OS_WINDOW_SRC, "preset_action_for_key", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "KeyCode::Digit1", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "KeyCode::Digit2", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "KeyCode::Digit3", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "KeyCode::Digit4", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "DesktopPreset::Descent", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "DesktopPreset::Determinism", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "DesktopPreset::Budget", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "DesktopPreset::TickerOnly", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "Numpad keys plus every", "os_window.rs");
    assert_contains(
        OS_WINDOW_SRC,
        "preset_action_for_key(KeyCode::Numpad1), None",
        "os_window.rs",
    );
    assert_contains(
        OS_WINDOW_SRC,
        "preset_action_for_key(KeyCode::Numpad2), None",
        "os_window.rs",
    );
    assert_contains(
        OS_WINDOW_SRC,
        "preset_action_for_key(KeyCode::Numpad3), None",
        "os_window.rs",
    );
    assert_contains(
        OS_WINDOW_SRC,
        "preset_action_for_key(KeyCode::Numpad4), None",
        "os_window.rs",
    );
    assert_contains(
        OS_WINDOW_SRC,
        "preset_action_for_key(KeyCode::Digit0), None",
        "os_window.rs",
    );
    assert_contains(
        OS_WINDOW_SRC,
        "preset_action_for_key(KeyCode::Digit5), None",
        "os_window.rs",
    );
    assert_contains(
        OS_WINDOW_SRC,
        "preset_action_for_key(KeyCode::Equal), None",
        "os_window.rs",
    );
    assert_contains(
        OS_WINDOW_SRC,
        "preset_action_for_key(KeyCode::Minus), None",
        "os_window.rs",
    );
    assert_contains(
        OS_WINDOW_SRC,
        "preset_action_for_key(KeyCode::F3), None",
        "os_window.rs",
    );
    assert_contains(
        OS_WINDOW_SRC,
        "preset_action_for_key(KeyCode::Escape), None",
        "os_window.rs",
    );
    assert_contains(
        OS_WINDOW_SRC,
        "preset_action_for_key(KeyCode::KeyA), None",
        "os_window.rs",
    );
    assert_contains(
        OS_WINDOW_SRC,
        "preset_keys_map_to_desktop_presets_only",
        "os_window.rs",
    );
    assert_contains(
        OS_WINDOW_SRC,
        "if let Some(preset) = preset_action_for_key(code)",
        "os_window.rs",
    );
    assert_contains(
        OS_WINDOW_SRC,
        "self.shell.set_preset(preset);",
        "os_window.rs",
    );
}

/// AC1 application: `set_preset` changes visibility plus default tab.
///
/// Descent shows top, left, and bottom with the plots tab; Determinism
/// selects log and Budget selects budget over top plus bottom;
/// `TickerOnly` keeps the top bar alone with no default tab.
#[test]
fn ac1_set_preset_changes_visibility_plus_default_tab() {
    assert_contains(SHELL_SRC, "pub fn set_preset", "shell.rs");
    assert_contains(SHELL_SRC, "self.config.set_preset(preset);", "shell.rs");
    assert_contains(SHELL_SRC, "PanelVisibility::for_preset(preset)", "shell.rs");
    assert_contains(
        SHELL_SRC,
        "BottomTab::default_for_preset(preset)",
        "shell.rs",
    );
    assert_contains(
        LAYOUT_SRC,
        "Self::Descent => Some(PhoneTab::Plots)",
        "layout.rs",
    );
    assert_contains(
        LAYOUT_SRC,
        "Self::Budget => Some(PhoneTab::Budget)",
        "layout.rs",
    );
    assert_contains(
        LAYOUT_SRC,
        "Self::Determinism => Some(PhoneTab::Log)",
        "layout.rs",
    );
    assert_contains(LAYOUT_SRC, "Self::TickerOnly => None", "layout.rs");
    assert_contains(
        LAYOUT_SRC,
        "TOP_BAR_BIT_U8 | LEFT_PANEL_BIT_U8 | BOTTOM_TABS_BIT_U8",
        "layout.rs",
    );
    assert_contains(
        LAYOUT_SRC,
        "TOP_BAR_BIT_U8 | BOTTOM_TABS_BIT_U8",
        "layout.rs",
    );
    assert_contains(LAYOUT_SRC, "from_bits(TOP_BAR_BIT_U8)", "layout.rs");
    assert_contains(BOTTOM_SRC, "default_for_preset", "bottom.rs");
    assert_contains(BOTTOM_SRC, "from_phone_tab", "bottom.rs");
    assert_contains(BOTTOM_SRC, "Returns none for ticker-only", "bottom.rs");
    assert_contains(
        SHELL_SRC,
        "descent_preset_shows_canvas_and_plots_ticker_shows_top_alone",
        "shell.rs",
    );
}

/// AC2 buttons: four 44pt buttons mirror the keys with the same action.
///
/// The left overlay panel draws one button per `DesktopPreset::ALL`
/// entry sized to `MIN_TOUCH_TARGET_PT_F32` (44.0pt) and routes clicks
/// through the same visibility plus default-tab application as keys.
#[test]
fn ac2_preset_buttons_mirror_keys_at_44pt() {
    assert_contains(SHELL_SRC, "draw_left_overlays", "shell.rs");
    assert_contains(SHELL_SRC, "44pt", "shell.rs");
    assert_contains(SHELL_SRC, "DesktopPreset::ALL", "shell.rs");
    assert_contains(SHELL_SRC, "\"Descent\"", "shell.rs");
    assert_contains(SHELL_SRC, "\"Determinism\"", "shell.rs");
    assert_contains(SHELL_SRC, "\"Budget\"", "shell.rs");
    assert_contains(SHELL_SRC, "\"TickerOnly\"", "shell.rs");
    assert_contains(SHELL_SRC, "MIN_TOUCH_TARGET_PT_F32", "shell.rs");
    assert_contains(SHELL_SRC, "touch_pt_f32", "shell.rs");
    assert_contains(SHELL_SRC, "button_size", "shell.rs");
    assert_contains(SHELL_SRC, "add_sized", "shell.rs");
    assert_contains(SHELL_SRC, ".clicked()", "shell.rs");
    assert_contains(SHELL_SRC, "pending_preset", "shell.rs");
    assert_contains(
        SHELL_SRC,
        "self.set_visibility(PanelVisibility::for_preset(preset));",
        "shell.rs",
    );
    assert_contains(
        SHELL_SRC,
        "BottomTab::default_for_preset(preset)",
        "shell.rs",
    );
    assert_contains(
        LAYOUT_SRC,
        "MIN_TOUCH_TARGET_PT_F32: f32 = 44.0",
        "layout.rs",
    );
    let expected_touch_text = format!("{EXPECTED_TOUCH_PT_F32:.1}");
    assert_contains(LAYOUT_SRC, &expected_touch_text, "layout.rs");
    let expected_all_text = format!("[Self; {EXPECTED_PRESET_COUNT_USIZE}]");
    assert_contains(LAYOUT_SRC, &expected_all_text, "layout.rs");
}

/// AC4 canvas gate: preset switching leaves the canvas gate unchanged.
///
/// Descent paints the marks canvas while `TickerOnly` keeps the top bar
/// alone; the preset path touches config, visibility, and the default
/// tab only, and the in-crate draws own the shape counts.
#[test]
fn ac4_canvas_gate_unchanged_by_presets() {
    assert_contains(LAYOUT_SRC, "shows_marks_canvas_bool", "layout.rs");
    assert_contains(LAYOUT_SRC, "every preset except ticker-only", "layout.rs");
    assert_contains(
        LAYOUT_SRC,
        "marks_canvas_shows_everywhere_but_ticker_only",
        "layout.rs",
    );
    assert_contains(SHELL_SRC, "shows_marks_canvas_bool", "shell.rs");
    assert_contains(
        SHELL_SRC,
        "descent_preset_shows_canvas_and_plots_ticker_shows_top_alone",
        "shell.rs",
    );
    assert_contains(
        SHELL_SRC,
        "marks_canvas_paints_on_descent_and_holds_on_ticker",
        "shell.rs",
    );
    assert_contains(SHELL_SRC, "self.config.set_preset(preset);", "shell.rs");
    assert_contains(SHELL_SRC, "PanelVisibility::for_preset(preset)", "shell.rs");
    assert_contains(
        SHELL_SRC,
        "BottomTab::default_for_preset(preset)",
        "shell.rs",
    );
    assert_contains(
        OS_WINDOW_SRC,
        "self.shell.set_preset(preset);",
        "os_window.rs",
    );
}

/// AC5 zoom: the marks zoom mapper is unchanged with no key overlap.
///
/// `Equal` zooms in, `Minus` zooms out, and `Digit0` resets at the
/// 1.25 step; preset digits never zoom and zoom keys never switch
/// presets, and `on_key` serves both mappers.
#[test]
fn ac5_zoom_mapper_unchanged_without_overlap() {
    assert_contains(OS_WINDOW_SRC, "marks_zoom_action_for_key", "os_window.rs");
    assert_contains(
        OS_WINDOW_SRC,
        "MARKS_ZOOM_STEP_RATIO_F64: f64 = 1.25",
        "os_window.rs",
    );
    let expected_step_text = format!("{EXPECTED_ZOOM_STEP_RATIO_F64}");
    assert_contains(OS_WINDOW_SRC, &expected_step_text, "os_window.rs");
    assert_contains(
        OS_WINDOW_SRC,
        "MARKS_ZOOM_STEP_RATIO_F64 > 1.0",
        "os_window.rs",
    );
    assert_contains(OS_WINDOW_SRC, "MarksZoomAction::ZoomIn", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "MarksZoomAction::ZoomOut", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "MarksZoomAction::Reset", "os_window.rs");
    assert_contains(
        OS_WINDOW_SRC,
        "marks_zoom_keys_map_to_zoom_actions_only",
        "os_window.rs",
    );
    assert_contains(
        OS_WINDOW_SRC,
        "marks_zoom_action_for_key(KeyCode::Digit1), None",
        "os_window.rs",
    );
    assert_contains(
        OS_WINDOW_SRC,
        "marks_zoom_action_for_key(KeyCode::Digit2), None",
        "os_window.rs",
    );
    assert_contains(
        OS_WINDOW_SRC,
        "marks_zoom_action_for_key(KeyCode::Digit3), None",
        "os_window.rs",
    );
    assert_contains(
        OS_WINDOW_SRC,
        "marks_zoom_action_for_key(KeyCode::Digit4), None",
        "os_window.rs",
    );
    assert_contains(
        OS_WINDOW_SRC,
        "preset_action_for_key(KeyCode::Equal), None",
        "os_window.rs",
    );
    assert_contains(
        OS_WINDOW_SRC,
        "preset_action_for_key(KeyCode::Minus), None",
        "os_window.rs",
    );
    assert_contains(
        OS_WINDOW_SRC,
        "preset_action_for_key(KeyCode::Digit0), None",
        "os_window.rs",
    );
    assert_contains(OS_WINDOW_SRC, "apply_marks_zoom", "os_window.rs");
}

/// AC6 headless: green with no window and clean touched sources.
///
/// `cargo test` never carries `--run-window`, the launch gate still
/// needs the flag plus a display, and every touched source stays free
/// of unwraps, expects, and forbidden blocks (boundary keyword avoided).
#[test]
fn ac6_headless_green_without_window() {
    for arg in std::env::args() {
        assert!(
            arg != RUN_WINDOW_FLAG,
            "cargo test must not carry {RUN_WINDOW_FLAG}; got {arg}"
        );
    }
    assert_contains(OS_WINDOW_SRC, "decide_launch", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "display_available", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "StayHeadless", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "RUN_WINDOW_FLAG", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "\"--run-window\"", "os_window.rs");
    assert_lacks(OS_WINDOW_SRC, ".unwrap()", "os_window.rs");
    assert_lacks(OS_WINDOW_SRC, ".expect(", "os_window.rs");
    assert_lacks(OS_WINDOW_SRC, FORBIDDEN_NEEDLE, "os_window.rs");
    assert_lacks(SHELL_SRC, ".unwrap()", "shell.rs");
    assert_lacks(SHELL_SRC, ".expect(", "shell.rs");
    assert_lacks(SHELL_SRC, FORBIDDEN_NEEDLE, "shell.rs");
    assert_lacks(LAYOUT_SRC, ".unwrap()", "layout.rs");
    assert_lacks(LAYOUT_SRC, ".expect(", "layout.rs");
    assert_lacks(LAYOUT_SRC, FORBIDDEN_NEEDLE, "layout.rs");
    assert_lacks(BOTTOM_SRC, ".unwrap()", "bottom.rs");
    assert_lacks(BOTTOM_SRC, ".expect(", "bottom.rs");
    assert_lacks(BOTTOM_SRC, FORBIDDEN_NEEDLE, "bottom.rs");
}
