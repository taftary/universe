//! OS window ticker-only headless gate for issue 44 step 7.
//!
//! The OS window (`crates/debug/src/os_window.rs`, Step 6, dev-shell only)
//! opens only with the explicit `--run-window` flag plus a display gate,
//! drives the same `DesktopWindow` ticker-only on the main thread, and
//! leaves headless CI green without a display. This file proves the gate
//! without opening a window: source contracts pin the flag, the display
//! probe, the ticker-only assembly, and the measured-cost draw, while one
//! runtime test proves `cargo test` itself never carries the flag.

#![forbid(unsafe_code)]

/// OS window source for the flag plus display-gate contract.
const OS_WINDOW_SRC: &str = include_str!("../crates/debug/src/os_window.rs");

/// Debug entry source for the launch-decision wiring contract.
const DEBUG_MAIN_SRC: &str = include_str!("../crates/debug/src/main.rs");

/// Shell source for the ticker-only assembly contract.
const SHELL_SRC: &str = include_str!("../crates/debug/src/shell.rs");

/// Layout source for the ticker-only config contract.
const LAYOUT_SRC: &str = include_str!("../crates/debug/src/layout.rs");

/// Debug crate manifest for the dev-shell-only wiring contract.
const DEBUG_CARGO_SRC: &str = include_str!("../crates/debug/Cargo.toml");

/// Engine manifest for the sim-stays-windowing-free contract.
const ENGINE_CARGO_SRC: &str = include_str!("../crates/engine/Cargo.toml");

/// Workspace manifest for the locked-version contract.
const WORKSPACE_CARGO_SRC: &str = include_str!("../Cargo.toml");

/// Debug doc for the headless-proven note contract.
const DEBUG_SRC: &str = include_str!("../docs/tech/debug.md");

/// Window flag spelling, dimensionless text.
const RUN_WINDOW_FLAG: &str = "--run-window";

/// Backend flag spelling, dimensionless text.
const BACKEND_FLAG: &str = "--backend";

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

/// Headless gate exists: flag plus display probe plus headless reasons.
///
/// The window opens only through `decide_launch` with `RUN_WINDOW_FLAG`
/// plus `display_available`; every other path stays headless with a named
/// `HeadlessReason` for CI logs.
#[test]
fn headless_gate_source_contract() {
    assert_contains(OS_WINDOW_SRC, "RUN_WINDOW_FLAG", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, RUN_WINDOW_FLAG, "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "decide_launch", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "display_available", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "HeadlessReason", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "FlagMissing", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "DisplayMissing", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "StayHeadless", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "OpenWindow", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "headless", "os_window.rs");
    assert_contains(DEBUG_MAIN_SRC, "decide_launch", "debug main.rs");
    assert_contains(DEBUG_MAIN_SRC, "StayHeadless", "debug main.rs");
    assert_contains(DEBUG_MAIN_SRC, "headless reason=", "debug main.rs");
    assert_contains(DEBUG_MAIN_SRC, "run_window", "debug main.rs");
}

/// Ticker-only assembly exists: blind top-bar window plus measured draw.
///
/// `DesktopWindow::open` starts ticker-only and blind; the OS window sizes
/// from `DesktopWindowConfig::ticker_only` and records shell draw cost on
/// every draw through `draw_measured`.
#[test]
fn ticker_only_assembly_source_contract() {
    assert_contains(SHELL_SRC, "DesktopWindow", "shell.rs");
    assert_contains(
        SHELL_SRC,
        "PanelVisibility::for_preset(DesktopPreset::TickerOnly)",
        "shell.rs",
    );
    assert_contains(SHELL_SRC, "draw_measured", "shell.rs");
    assert_contains(SHELL_SRC, "record_draw_cost", "shell.rs");
    assert_contains(SHELL_SRC, "tester_readouts", "shell.rs");
    assert_contains(LAYOUT_SRC, "ticker_only", "layout.rs");
    assert_contains(LAYOUT_SRC, "TickerOnly", "layout.rs");
    assert_contains(LAYOUT_SRC, "DESKTOP_WINDOW_TITLE", "layout.rs");
    assert_contains(OS_WINDOW_SRC, "DesktopWindow", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "ticker-only", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "draw_measured", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "DESKTOP_WINDOW_TITLE", "os_window.rs");
}

/// `cargo test` never carries the window flag, so no test opens a window.
///
/// The gate needs the explicit flag; the test harness never passes
/// command-line flags through, and this process asserts that directly.
#[test]
fn cargo_test_carries_no_window_flag() {
    for arg in std::env::args() {
        assert!(
            arg != RUN_WINDOW_FLAG,
            "cargo test must not carry {RUN_WINDOW_FLAG}; got {arg}"
        );
    }
    assert_contains(OS_WINDOW_SRC, "\"--run-window\"", "os_window.rs");
}

/// Dev-shell-only wiring: locked versions behind the feature, sim stays free.
///
/// `winit` 0.30.13 plus `wgpu` 30 plus `egui` 0.36.2 live in
/// `universe-debug` behind the non-default `dev-shell` feature; the binary
/// requires that feature and `engine::sim` stays windowing-free.
#[test]
fn dev_shell_only_wiring_contract() {
    assert_contains(WORKSPACE_CARGO_SRC, "winit", "workspace Cargo.toml");
    assert_contains(WORKSPACE_CARGO_SRC, "0.30.13", "workspace Cargo.toml");
    assert_contains(WORKSPACE_CARGO_SRC, "wgpu", "workspace Cargo.toml");
    assert_contains(WORKSPACE_CARGO_SRC, "egui", "workspace Cargo.toml");
    assert_contains(WORKSPACE_CARGO_SRC, "0.36.2", "workspace Cargo.toml");
    assert_contains(WORKSPACE_CARGO_SRC, "dev-shell", "workspace Cargo.toml");
    assert_contains(DEBUG_CARGO_SRC, "required-features", "debug Cargo.toml");
    assert_contains(DEBUG_CARGO_SRC, "dev-shell", "debug Cargo.toml");
    assert_contains(DEBUG_CARGO_SRC, "optional = true", "debug Cargo.toml");
    assert_lacks(ENGINE_CARGO_SRC, "winit", "engine Cargo.toml");
    assert_lacks(ENGINE_CARGO_SRC, "wgpu", "engine Cargo.toml");
    assert_lacks(ENGINE_CARGO_SRC, "egui", "engine Cargo.toml");
}

/// Docs record the headless-proven window plus this gate.
///
/// `docs/tech/debug.md` owns the tester shape, the OS window note, and the
/// postponed phone run; this gate file is cited there so the note stays
/// in sync with the code.
#[test]
fn docs_headless_proven_contract() {
    assert_contains(DEBUG_SRC, "os_window.rs", "debug.md");
    assert_contains(DEBUG_SRC, "--run-window", "debug.md");
    assert_contains(DEBUG_SRC, "DesktopWindow", "debug.md");
    assert_contains(DEBUG_SRC, "ticker-only", "debug.md");
    assert_contains(DEBUG_SRC, "draw_measured", "debug.md");
    assert_contains(DEBUG_SRC, "display gate", "debug.md");
    assert_contains(DEBUG_SRC, "headless", "debug.md");
    assert_contains(DEBUG_SRC, "headless-proven", "debug.md");
    assert_contains(DEBUG_SRC, "tests/os_window_ticker.rs", "debug.md");
    assert_contains(DEBUG_SRC, "FRAME_BUDGET_MS", "debug.md");
    assert_contains(DEBUG_SRC, "quality.md", "debug.md");
}

/// Backend override exists: flag plus selection plus restricted probes.
///
/// `--backend auto|vulkan|dx12|gl` threads through the preflight probe and the
/// window-bound pick so both agree; `auto` keeps the scored Vulkan-first order
/// and named values restrict enumeration plus fallback to one backend with a
/// typed error for unknown values and a boot log of the effective backend.
#[test]
fn backend_override_source_contract() {
    assert_contains(OS_WINDOW_SRC, "BACKEND_FLAG", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, BACKEND_FLAG, "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "\"--backend\"", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "BackendSelection", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "parse_backend_selection", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "BACKEND_AUTO_LABEL", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "BACKEND_VULKAN_LABEL", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "BACKEND_DX12_LABEL", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "BACKEND_GL_LABEL", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "\"auto\"", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "\"vulkan\"", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "\"dx12\"", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "\"gl\"", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "order_label", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "backends_label", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "instance_backends", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "create_instance", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "log_backend_selection", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "os_window backend=", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "Backend(", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "preflight_adapter_probe", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "ActiveWindow::create", "os_window.rs");
    assert_contains(DEBUG_MAIN_SRC, "parse_backend_selection", "debug main.rs");
    assert_contains(DEBUG_MAIN_SRC, "backend_selection", "debug main.rs");
    assert_contains(DEBUG_MAIN_SRC, BACKEND_FLAG, "debug main.rs");
}

/// Backend flag never weakens the headless gate.
///
/// The window still opens only with `RUN_WINDOW_FLAG` plus a display;
/// `--backend` alone stays headless and `cargo test` carries neither flag.
#[test]
fn backend_headless_gate_unchanged() {
    assert_contains(OS_WINDOW_SRC, "decide_launch", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "display_available", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "FlagMissing", "os_window.rs");
    assert_contains(OS_WINDOW_SRC, "DisplayMissing", "os_window.rs");
    assert_contains(DEBUG_MAIN_SRC, "decide_launch", "debug main.rs");
    for arg in std::env::args() {
        assert!(
            arg != BACKEND_FLAG && !arg.starts_with("--backend="),
            "cargo test must not carry {BACKEND_FLAG}; got {arg}"
        );
    }
    assert_contains(OS_WINDOW_SRC, "\"--backend\"", "os_window.rs");
}
