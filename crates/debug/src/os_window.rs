//! OS window: winit event loop plus wgpu surface plus egui-wgpu renderer.
//!
//! Drives [`DesktopWindow`](crate::shell::DesktopWindow) ticker-only on the main
//! thread under D-003. The main thread owns winit plus the wgpu surface per
//! `docs/tech/architecture.md`; the sim plus render thread split lands later,
//! so one loop advances a fixed-step demo orbit and renders the shell here.
//! Headless CI never opens a window: [`decide_launch`] requires the explicit
//! [`RUN_WINDOW_FLAG`] plus a display. Keyboard widget input beyond `F3` and
//! `Escape` stays deferred; pointer input plus those two router keys drive the
//! ticker-only top bar meanwhile.

use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context as TaskContext, Poll, Wake, Waker};
use std::thread;
use std::time::Instant;

use crate::budget::BudgetDenominators;
use crate::input::RouterKey;
use crate::layout::DESKTOP_WINDOW_TITLE;
use crate::shell::{DesktopWindow, ShellError};
use crate::theme::BASE_BACKGROUND_RGB_U8;
use crate::top_bar::MILLIS_PER_SECOND_F64;
use engine::inspect::InspectError;
use engine::sim::{SIM_TICK_S, Scheduler};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition, PhysicalSize};
use winit::event::{
    ElementState, MouseButton, MouseScrollDelta, TouchPhase as WinitTouchPhase, WindowEvent,
};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

/// Command-line flag requesting the OS window.
///
/// The window opens only with this flag plus a display; see [`decide_launch`].
pub const RUN_WINDOW_FLAG: &str = "--run-window";

/// Command-line flag forcing the software fallback adapter.
///
/// Takes effect only alongside [`RUN_WINDOW_FLAG`]; see [`software_requested`].
pub const SOFTWARE_FLAG: &str = "--software";

/// Frame budget in milliseconds (`FRAME_BUDGET_MS`).
///
/// Source: `docs/tech/debug.md` section 2; gates live in `docs/tech/quality.md`.
pub const FRAME_BUDGET_MS_F64: f64 = 33.33;

/// Sim-tick average budget in milliseconds (`SIM_TICK_AVG_MS`).
///
/// Source: `docs/tech/debug.md` section 2; gates live in `docs/tech/quality.md`.
pub const SIM_TICK_AVG_BUDGET_MS_F64: f64 = 8.0;

/// Sim-tick p99 budget in milliseconds (`SIM_TICK_P99_MS`).
///
/// Source: `docs/tech/debug.md` section 2; gates live in `docs/tech/quality.md`.
pub const SIM_TICK_P99_BUDGET_MS_F64: f64 = 16.0;

/// Surface-hitch p95 budget in milliseconds (`SURFACE_HITCH_P95_MS`).
///
/// Source: `docs/tech/debug.md` section 2; gates live in `docs/tech/quality.md`.
pub const SURFACE_HITCH_P95_BUDGET_MS_F64: f64 = 100.0;

/// Memory ceiling in megabytes (`MEMORY_CEILING_MB`).
///
/// Source: `docs/tech/debug.md` section 2; gates live in `docs/tech/quality.md`.
pub const MEMORY_CEILING_MB_F64: f64 = 1024.0;

/// Cold-start budget in seconds (`COLD_START_S`).
///
/// Source: `docs/tech/debug.md` section 2; gates live in `docs/tech/quality.md`.
pub const COLD_START_BUDGET_S_F64: f64 = 5.0;

/// Demo master seed, dimensionless.
///
/// Source: `tests/shell_phase_a.rs` golden seed (fractional hex digits of pi).
pub const DEMO_MASTER_SEED_U64: u64 = 0x243F_6A88_85A3_08D3;

/// Demo circular-orbit altitude in meters.
///
/// Source: `tests/shell_phase_a.rs` golden cruise profile at 250 km.
pub const DEMO_CRUISE_ALTITUDE_M_F64: f64 = 250_000.0;

/// Maximum catch-up sim steps per frame, dimensionless.
///
/// Bounds one redraw under warp; excess wall time is dropped, never spiraled,
/// per the frame-pacer rule in `docs/tech/mobile.md`. Demo bound only.
pub const MAX_CATCH_UP_STEPS_PER_FRAME_U64: u64 = 600;

/// Minimum surface extent in pixels.
///
/// Guards resize-to-zero (minimized window) before surface reconfiguration.
pub const MIN_SURFACE_EXTENT_PX_U32: u32 = 1;

/// Desired surface frame latency in frames.
///
/// Init-time wgpu default; the frame loop never reconfigures for latency.
pub const SURFACE_MAX_LATENCY_U32: u32 = 2;

/// Clear alpha in the opaque range.
///
/// Source: wgpu opaque clear (alpha 1.0).
pub const CLEAR_ALPHA_F64: f64 = 1.0;

/// Eight-bit channel maximum for clear-color conversion.
///
/// Source: 8-bit display channels in `crate::theme`.
pub const RGB_CHANNEL_MAX_F64: f64 = 255.0;

/// Fallback pixels-per-point when the OS scale is unusable.
///
/// Source: winit default scale factor of 1.0.
pub const FALLBACK_PIXELS_PER_POINT_F64: f64 = 1.0;

/// X11 display environment variable name.
///
/// Source: X11 convention; probed on every host in [`display_available`].
pub const DISPLAY_ENV_NAME: &str = "DISPLAY";

/// Wayland display environment variable name.
///
/// Source: Wayland convention; probed on every host in [`display_available`].
pub const WAYLAND_DISPLAY_ENV_NAME: &str = "WAYLAND_DISPLAY";

/// Wgpu backends probed for the OS window, dimensionless label.
///
/// Source: wgpu 30 `Backends::all` (primary DX12, Vulkan, Metal plus
/// secondary GL over ANGLE on Windows); both adapter probes enable all of
/// them so DX11-class Windows hosts fall back through GL instead of failing
/// a DX12-only attempt.
pub const ADAPTER_BACKENDS_LABEL: &str = "dx12+vulkan+metal+gl(ANGLE)";

/// Adapter backend probe order, dimensionless label.
///
/// Source: Vulkan-capable GPU floor; attempts enumerate Vulkan first, then
/// Dx12, then Metal on Apple hosts, then GL over ANGLE. The
/// [`ADAPTER_BACKENDS_LABEL`] all-backends set is unchanged; this label
/// records the scored attempt order only.
pub const ADAPTER_BACKEND_ORDER_LABEL: &str = "vulkan>dx12>metal>gl";

/// Device-type score for a discrete GPU, dimensionless.
///
/// Source: reference adapter scoring discrete above integrated above virtual
/// above CPU (`game-ref` boot path); highest rank wins.
const DEVICE_SCORE_DISCRETE_U8: u8 = 4;

/// Device-type score for an integrated GPU, dimensionless.
///
/// Source: reference adapter scoring; ranks below discrete, above unknown.
const DEVICE_SCORE_INTEGRATED_U8: u8 = 3;

/// Device-type score for an unknown or other device, dimensionless.
///
/// Source: this module only; unknown hardware outranks virtual and CPU but
/// never outranks known integrated or discrete GPUs.
const DEVICE_SCORE_OTHER_U8: u8 = 2;

/// Device-type score for a virtual GPU, dimensionless.
///
/// Source: reference adapter scoring; ranks below hardware, above CPU.
const DEVICE_SCORE_VIRTUAL_U8: u8 = 1;

/// Device-type score for CPU or software rendering, dimensionless.
///
/// Source: reference adapter scoring; lowest rank, chosen only when no
/// hardware adapter is surface-compatible.
const DEVICE_SCORE_CPU_U8: u8 = 0;

/// Backend score for Vulkan, dimensionless.
///
/// Source: Vulkan-capable GPU floor; highest rank so Vulkan-capable hosts
/// prefer Vulkan first.
const BACKEND_SCORE_VULKAN_U8: u8 = 4;

/// Backend score for Dx12, dimensionless.
///
/// Source: Vulkan-capable floor order; ranks below Vulkan, above Metal and GL.
const BACKEND_SCORE_DX12_U8: u8 = 3;

/// Backend score for Metal, dimensionless.
///
/// Source: Apple-host order; ranks below Dx12, above GL.
const BACKEND_SCORE_METAL_U8: u8 = 2;

/// Backend score for GL over ANGLE, dimensionless.
///
/// Source: secondary-tier fallback; ranks below the primary backends.
const BACKEND_SCORE_GL_U8: u8 = 1;

/// Backend score for any other backend, dimensionless.
///
/// Source: this module only; unknown backends rank lowest and never win
/// over a known backend.
const BACKEND_SCORE_OTHER_U8: u8 = 0;

/// Device debug label, dimensionless.
///
/// Source: this module only; tags the conservative device request so
/// driver logs point at the OS window path.
const DEVICE_LABEL: &str = "universe-os-window";

/// Reason for staying headless without opening a window.
///
/// Returned by [`decide_launch`] so CI logs stay explicit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadlessReason {
    /// The explicit [`RUN_WINDOW_FLAG`] was absent.
    FlagMissing,
    /// The flag was present but no display was detected.
    DisplayMissing,
}

impl HeadlessReason {
    /// Return the short headless reason label.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Headless gate seam; unit tests cover the decision meanwhile."
        )
    )]
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::FlagMissing => "flag-missing",
            Self::DisplayMissing => "display-missing",
        }
    }
}

impl core::fmt::Display for HeadlessReason {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::FlagMissing => {
                write!(formatter, "missing {RUN_WINDOW_FLAG}; headless demo only")
            }
            Self::DisplayMissing => {
                write!(formatter, "no display detected; headless demo only")
            }
        }
    }
}

/// Launch decision for the debug binary entry point.
///
/// Computed by [`decide_launch`] before any window or GPU work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchDecision {
    /// Open the OS window with the ticker-only shell.
    OpenWindow,
    /// Stay headless and run the tick demo instead.
    StayHeadless(HeadlessReason),
}

/// OS window failures with typed variants.
///
/// External winit plus wgpu errors keep their rendered detail as strings;
/// shell plus sim plus snapshot failures keep their typed sources.
#[derive(Debug)]
pub enum OsWindowError {
    /// Event-loop creation or run failed with rendered detail.
    EventLoop(String),
    /// OS window creation failed with rendered detail.
    Window(String),
    /// Wgpu surface creation failed with rendered detail.
    Surface(String),
    /// No compatible wgpu adapter was found, with host plus backend detail.
    NoAdapter(String),
    /// The adapter offered no surface format.
    NoSurfaceFormat,
    /// Wgpu device request failed with rendered detail.
    Device(String),
    /// Demo sim construction failed with rendered detail.
    SimInit(String),
    /// Demo sim step failed with rendered detail.
    SimStep(String),
    /// Snapshot capture failed.
    Snapshot(InspectError),
    /// Shell assembly, observation, or draw failed.
    Shell(ShellError),
    /// Budget denominators were rejected with rendered detail.
    Budgets(String),
}

impl core::fmt::Display for OsWindowError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::EventLoop(detail) => write!(formatter, "os window event loop: {detail}"),
            Self::Window(detail) => write!(formatter, "os window creation: {detail}"),
            Self::Surface(detail) => write!(formatter, "os window surface: {detail}"),
            Self::NoAdapter(detail) => {
                write!(
                    formatter,
                    "os window: no compatible wgpu adapter ({detail})"
                )
            }
            Self::NoSurfaceFormat => {
                write!(formatter, "os window: adapter offered no surface format")
            }
            Self::Device(detail) => write!(formatter, "os window device: {detail}"),
            Self::SimInit(detail) => write!(formatter, "os window sim init: {detail}"),
            Self::SimStep(detail) => write!(formatter, "os window sim step: {detail}"),
            Self::Snapshot(source) => write!(formatter, "os window snapshot: {source}"),
            Self::Shell(source) => write!(formatter, "os window shell: {source}"),
            Self::Budgets(detail) => write!(formatter, "os window budgets: {detail}"),
        }
    }
}

impl std::error::Error for OsWindowError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Snapshot(source) => Some(source),
            Self::Shell(source) => Some(source),
            Self::EventLoop(_)
            | Self::Window(_)
            | Self::Surface(_)
            | Self::NoAdapter(_)
            | Self::NoSurfaceFormat
            | Self::Device(_)
            | Self::SimInit(_)
            | Self::SimStep(_)
            | Self::Budgets(_) => None,
        }
    }
}

impl From<ShellError> for OsWindowError {
    /// Convert a shell failure into an OS window failure.
    fn from(source: ShellError) -> Self {
        Self::Shell(source)
    }
}

impl From<InspectError> for OsWindowError {
    /// Convert a snapshot failure into an OS window failure.
    fn from(source: InspectError) -> Self {
        Self::Snapshot(source)
    }
}

/// Report whether software fallback rendering was requested.
///
/// True when `args` contains [`SOFTWARE_FLAG`]; window still needs [`RUN_WINDOW_FLAG`].
#[must_use]
pub fn software_requested(args: &[String]) -> bool {
    args.iter().any(|arg| arg == SOFTWARE_FLAG)
}

/// Decide whether the process opens the OS window.
///
/// Opens only with [`RUN_WINDOW_FLAG`] in `args` plus a display from
/// [`display_available`]; otherwise returns the headless reason. Reads no
/// window or GPU state, so headless tests call this freely.
#[must_use]
pub fn decide_launch(args: &[String]) -> LaunchDecision {
    decide_launch_with(args, display_available())
}

/// Decide launch with an injected display flag.
///
/// Test seam for [`decide_launch`]: `display_present_bool` replaces the OS
/// display probe so unit tests stay deterministic on every host.
#[must_use]
pub fn decide_launch_with(args: &[String], display_present_bool: bool) -> LaunchDecision {
    let requested_bool = args.iter().any(|arg| arg == RUN_WINDOW_FLAG);
    if !requested_bool {
        return LaunchDecision::StayHeadless(HeadlessReason::FlagMissing);
    }
    if !display_present_bool {
        return LaunchDecision::StayHeadless(HeadlessReason::DisplayMissing);
    }
    LaunchDecision::OpenWindow
}

/// Report whether the host offers a display.
///
/// Windows plus macOS always return true; other targets require a non-empty
/// `DISPLAY` or `WAYLAND_DISPLAY` so headless CI stays window-free. The probe
/// runs on every host so the gate stays testable anywhere.
#[must_use]
pub fn display_available() -> bool {
    let env_bool = display_available_with(
        std::env::var(DISPLAY_ENV_NAME).ok().as_deref(),
        std::env::var(WAYLAND_DISPLAY_ENV_NAME).ok().as_deref(),
    );
    cfg!(any(target_os = "windows", target_os = "macos")) || env_bool
}

/// Check display variables without touching the process environment.
///
/// Shared probe for [`display_available`] plus its tests: true with either
/// non-empty value.
#[must_use]
pub fn display_available_with(display_value: Option<&str>, wayland_value: Option<&str>) -> bool {
    display_value.is_some_and(|value| !value.is_empty())
        || wayland_value.is_some_and(|value| !value.is_empty())
}

/// Build the `NoAdapter` detail from a wgpu report.
///
/// Names the probed backends plus the host OS and points at the headless
/// demo so GPU-less logs stay actionable without opening a window.
#[must_use]
fn no_adapter_detail(wgpu_detail: &str) -> String {
    format!(
        "{wgpu_detail}; backends={ADAPTER_BACKENDS_LABEL}; host={}; {RUN_WINDOW_FLAG} needs a working GPU; run without the flag for the headless demo",
        std::env::consts::OS
    )
}

/// Score a wgpu device type for adapter ranking.
///
/// Higher wins; discrete outranks integrated, unknown, virtual, then CPU.
/// Reference scoring only; init-time helper for adapter selection.
#[must_use]
fn device_type_score_u8(device_type: wgpu::DeviceType) -> u8 {
    match device_type {
        wgpu::DeviceType::DiscreteGpu => DEVICE_SCORE_DISCRETE_U8,
        wgpu::DeviceType::IntegratedGpu => DEVICE_SCORE_INTEGRATED_U8,
        wgpu::DeviceType::Other => DEVICE_SCORE_OTHER_U8,
        wgpu::DeviceType::VirtualGpu => DEVICE_SCORE_VIRTUAL_U8,
        wgpu::DeviceType::Cpu => DEVICE_SCORE_CPU_U8,
    }
}

/// Score a wgpu backend for adapter ranking.
///
/// Higher wins; Vulkan outranks Dx12, Metal, GL, then unknown.
/// Vulkan-capable floor order; init-time helper for adapter selection.
#[must_use]
fn backend_score_u8(backend: wgpu::Backend) -> u8 {
    match backend {
        wgpu::Backend::Vulkan => BACKEND_SCORE_VULKAN_U8,
        wgpu::Backend::Dx12 => BACKEND_SCORE_DX12_U8,
        wgpu::Backend::Metal => BACKEND_SCORE_METAL_U8,
        wgpu::Backend::Gl => BACKEND_SCORE_GL_U8,
        wgpu::Backend::Noop | wgpu::Backend::BrowserWebGpu => BACKEND_SCORE_OTHER_U8,
    }
}

/// List backend probes in scored attempt order.
///
/// Vulkan first, then Dx12, then Metal, then GL over ANGLE.
/// The relative Vulkan to Dx12 to GL order holds on every host.
#[must_use]
fn ordered_backend_probes() -> [(wgpu::Backends, &'static str); 4] {
    [
        (wgpu::Backends::VULKAN, "vulkan"),
        (wgpu::Backends::DX12, "dx12"),
        (wgpu::Backends::METAL, "metal"),
        (wgpu::Backends::GL, "gl"),
    ]
}

/// Format adapter info for boot logging.
///
/// Names backend, adapter name, device type, driver, and driver detail.
/// Pure formatter so headless tests cover it without a GPU.
#[must_use]
fn format_adapter_info(info: &wgpu::AdapterInfo) -> String {
    format!(
        "adapter backend={} name={} device_type={:?} driver={} driver_info={} vendor={} device={}",
        info.backend,
        info.name,
        info.device_type,
        info.driver,
        info.driver_info,
        info.vendor,
        info.device
    )
}

/// Log the chosen adapter at boot.
///
/// Emits the [`format_adapter_info`] summary through `tracing::info!` plus
/// stdout so CI logs show it without a subscriber. Init-time only.
fn log_adapter_info(adapter: &wgpu::Adapter) {
    let summary = format_adapter_info(&adapter.get_info());
    tracing::info!("os_window {summary}");
    println!("os_window {summary}");
}

/// Report whether an adapter can present to the surface.
///
/// True with no surface constraint; otherwise true only when the surface
/// offers a default configuration for the adapter. Init-time helper only.
#[must_use]
fn is_surface_compatible(
    compatible_surface: Option<&wgpu::Surface<'_>>,
    adapter: &wgpu::Adapter,
) -> bool {
    let Some(surface) = compatible_surface else {
        return true;
    };
    surface
        .get_default_config(
            adapter,
            MIN_SURFACE_EXTENT_PX_U32,
            MIN_SURFACE_EXTENT_PX_U32,
        )
        .is_some()
}

/// Pick the best surface-compatible adapter from one backend list.
///
/// Scores each compatible adapter by backend then device type and clones
/// the winner. Returns `None` when no entry is surface-compatible.
#[must_use]
fn best_adapter_in_list(
    adapters: &[wgpu::Adapter],
    compatible_surface: Option<&wgpu::Surface<'_>>,
) -> Option<wgpu::Adapter> {
    let mut best_rank_u8: Option<(u8, u8)> = None;
    let mut best_adapter: Option<wgpu::Adapter> = None;
    for adapter in adapters {
        if !is_surface_compatible(compatible_surface, adapter) {
            continue;
        }
        let info = adapter.get_info();
        let rank_u8 = (
            backend_score_u8(info.backend),
            device_type_score_u8(info.device_type),
        );
        let wins_bool = match best_rank_u8 {
            None => true,
            Some(current_u8) => rank_u8 > current_u8,
        };
        if wins_bool {
            best_rank_u8 = Some(rank_u8);
            best_adapter = Some(adapter.clone());
        }
    }
    best_adapter
}

/// Request a wgpu adapter across scored backends then fallback attempts.
///
/// Enumerates [`ordered_backend_probes`] (Vulkan, Dx12, Metal, GL) and picks
/// the best surface-compatible adapter in each backend by device score
/// (discrete, integrated, other, virtual, CPU). Returns the first backend
/// with a pick and logs it with [`log_adapter_info`]. When enumeration
/// yields no compatible adapter, falls back to the hardware then software
/// request chain under the given surface constraint, logging that winner
/// too. With `software_mode_bool` (from [`SOFTWARE_FLAG`]) the scored
/// enumeration plus hardware requests are skipped and the low-power
/// force-fallback software request runs first, labeled `fallback software`.
/// The instance enables [`ADAPTER_BACKENDS_LABEL`], so Windows hosts
/// also reach the secondary GL path over ANGLE. When every attempt fails,
/// returns [`OsWindowError::NoAdapter`] joining each labeled attempt with
/// host plus headless guidance. Init-time only; the frame loop never calls this.
fn request_adapter_with_fallbacks(
    instance: &wgpu::Instance,
    compatible_surface: Option<&wgpu::Surface<'_>>,
    software_mode_bool: bool,
) -> Result<wgpu::Adapter, OsWindowError> {
    let mut reports: Vec<String> = Vec::new();
    if software_mode_bool {
        reports.push(format!(
            "software mode ({SOFTWARE_FLAG}): skipped hardware enumeration; order={ADAPTER_BACKEND_ORDER_LABEL}"
        ));
    } else {
        for (backends, backend_label) in ordered_backend_probes() {
            let adapters = block_on_init(instance.enumerate_adapters(backends));
            if let Some(adapter) = best_adapter_in_list(&adapters, compatible_surface) {
                log_adapter_info(&adapter);
                return Ok(adapter);
            }
            reports.push(format!(
                "{backend_label}: no compatible adapter (enumerated {}); order={ADAPTER_BACKEND_ORDER_LABEL}",
                adapters.len()
            ));
        }
    }
    let hardware_fallback_attempts: [(wgpu::PowerPreference, bool, &str); 3] = [
        (
            wgpu::PowerPreference::HighPerformance,
            false,
            "high-performance hardware",
        ),
        (wgpu::PowerPreference::LowPower, false, "low-power hardware"),
        (
            wgpu::PowerPreference::HighPerformance,
            true,
            "fallback software",
        ),
    ];
    let software_attempts: [(wgpu::PowerPreference, bool, &str); 1] =
        [(wgpu::PowerPreference::LowPower, true, "fallback software")];
    let attempts: &[(wgpu::PowerPreference, bool, &str)] = if software_mode_bool {
        &software_attempts
    } else {
        &hardware_fallback_attempts
    };
    for (preference, fallback_bool, label) in attempts.iter().copied() {
        let options = wgpu::RequestAdapterOptions {
            power_preference: preference,
            force_fallback_adapter: fallback_bool,
            compatible_surface,
            apply_limit_buckets: false,
        };
        match block_on_init(instance.request_adapter(&options)) {
            Ok(adapter) => {
                log_adapter_info(&adapter);
                return Ok(adapter);
            }
            Err(error) => reports.push(format!("{label}: {error}")),
        }
    }
    Err(OsWindowError::NoAdapter(no_adapter_detail(
        &reports.join("; "),
    )))
}

/// Build the conservative device descriptor for the OS window.
///
/// Requests no extra features, [`wgpu::Limits::downlevel_defaults`], the
/// [`DEVICE_LABEL`] tag, and performance memory hints, so Intel-class
/// integrated GPUs stay within their supported set. Init-time only; the
/// frame loop never builds a descriptor.
fn conservative_device_descriptor() -> wgpu::DeviceDescriptor<'static> {
    wgpu::DeviceDescriptor {
        label: Some(DEVICE_LABEL),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::downlevel_defaults(),
        memory_hints: wgpu::MemoryHints::Performance,
        ..Default::default()
    }
}

/// Request the logical device with one force-fallback retry.
///
/// Tries `adapter` first with [`conservative_device_descriptor`]; on `Err`
/// requests a force-fallback adapter once and retries the same descriptor.
/// Maps every reported failure to [`OsWindowError::Device`]; a fault inside
/// the driver call itself (issue #44 Step 8) still aborts before any `Err`
/// can be mapped. Init-time only; the frame loop never calls this.
///
/// # Errors
///
/// Returns [`OsWindowError::Device`] when the first request plus the
/// fallback retry both report failures, or when the fallback adapter itself
/// is unavailable.
fn request_device_with_fallback(
    instance: &wgpu::Instance,
    surface: &wgpu::Surface<'_>,
    adapter: &wgpu::Adapter,
) -> Result<(wgpu::Device, wgpu::Queue), OsWindowError> {
    let descriptor = conservative_device_descriptor();
    match block_on_init(adapter.request_device(&descriptor)) {
        Ok(pair) => Ok(pair),
        Err(first_error) => {
            let fallback_options = wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: true,
                compatible_surface: Some(surface),
                apply_limit_buckets: false,
            };
            match block_on_init(instance.request_adapter(&fallback_options)) {
                Ok(fallback_adapter) => {
                    match block_on_init(fallback_adapter.request_device(&descriptor)) {
                        Ok(pair) => Ok(pair),
                        Err(retry_error) => Err(OsWindowError::Device(format!(
                            "conservative request failed ({first_error}); fallback retry failed ({retry_error}); backends={ADAPTER_BACKENDS_LABEL}; device={DEVICE_LABEL}"
                        ))),
                    }
                }
                Err(adapter_error) => Err(OsWindowError::Device(format!(
                    "conservative request failed ({first_error}); fallback adapter unavailable ({adapter_error}); backends={ADAPTER_BACKENDS_LABEL}; device={DEVICE_LABEL}"
                ))),
            }
        }
    }
}

/// Probe for any usable wgpu adapter before touching winit or a surface.
///
/// Builds a short-lived headless instance over [`ADAPTER_BACKENDS_LABEL`]
/// and runs the scored backend enumeration
/// ([`ADAPTER_BACKEND_ORDER_LABEL`], Vulkan then Dx12 then Metal then GL,
/// device score discrete then integrated then other then virtual then CPU)
/// with no surface constraint, then the hardware-then-fallback request chain.
/// With `software_mode_bool` (from [`SOFTWARE_FLAG`]) the probe skips hardware
/// and requests the low-power force-fallback software adapter first.
/// The winner logs backend, name, device type, and driver via
/// [`log_adapter_info`]. Zero usable adapters return typed
/// [`OsWindowError::NoAdapter`] before any window opens, so headless stays
/// the default on GPU-less hosts.
///
/// Known upstream limitation (issue #44 Step 8, measured 2026-09-27 on a
/// Windows host with Intel UHD 620, driver 31.0.101.2130, dx12 backend):
/// the headless probe below (instance plus surface-less request, including
/// the force-fallback attempt) succeeds and enumerates a dx12 adapter, and
/// the window-bound path reaches window, surface, and compatible-adapter
/// creation; the process then exits with `STATUS_ACCESS_VIOLATION` inside
/// `Adapter::request_device` before wgpu reports any `Err`. That fault lies
/// below this module: our code holds no raw handles on this path and
/// performs no unchecked blocks, `unwrap`, or `expect`, mapping every
/// reported `Err` to typed `NoAdapter` or `Device`. When the fault triggers,
/// no typed error can be produced because the process dies inside the driver
/// call. Mitigation is the D-003 kill-switch in `docs/tech/stack.md`:
/// record host, driver, and backend, then reopen D-003 (`ash` fallback) per
/// that file. This probe keeps the graceful typed path for every failure
/// wgpu does report, and fails fast before any window opens on hosts with
/// zero usable adapters.
fn preflight_adapter_probe(software_mode_bool: bool) -> Result<(), OsWindowError> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    request_adapter_with_fallbacks(&instance, None, software_mode_bool).map(|_adapter| ())
}

/// Run the ticker-only OS window until close.
///
/// Creates the winit event loop, wgpu surface, and egui-wgpu renderer on the
/// main thread, drives one [`DesktopWindow`](crate::shell::DesktopWindow) with
/// a fixed-step demo orbit, and returns after close or a fatal error.
/// With `software_mode_bool` (from [`SOFTWARE_FLAG`]) both the pre-flight
/// probe and the window-bound adapter pick skip hardware and request the
/// low-power force-fallback software adapter first.
///
/// # Errors
///
/// Returns [`OsWindowError`] for event-loop, window, surface, adapter,
/// device, sim, snapshot, shell, or budget failures. GPU-less hosts fail
/// fast with typed [`OsWindowError::NoAdapter`] from the pre-flight probe
/// before any window opens; see [`preflight_adapter_probe`].
pub fn run_window(software_mode_bool: bool) -> Result<(), OsWindowError> {
    preflight_adapter_probe(software_mode_bool)?;
    let event_loop =
        EventLoop::new().map_err(|error| OsWindowError::EventLoop(error.to_string()))?;
    let mut app = WindowApp {
        software_mode_bool,
        ..WindowApp::default()
    };
    event_loop
        .run_app(&mut app)
        .map_err(|error| OsWindowError::EventLoop(error.to_string()))?;
    match app.fatal {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

/// Winit application owning the optional GPU-bound window state.
///
/// Stays empty until the first resume; fatal callback errors park in
/// `fatal` because winit callbacks return no `Result`.
#[derive(Default)]
struct WindowApp {
    /// GPU-bound window state after resume, if creation succeeded.
    active: Option<ActiveWindow>,
    /// Fatal error captured inside event callbacks, if any.
    fatal: Option<OsWindowError>,
    /// True when [`SOFTWARE_FLAG`] forces the fallback adapter.
    software_mode_bool: bool,
}

impl ApplicationHandler for WindowApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.active.is_some() {
            return;
        }
        match ActiveWindow::create(event_loop, self.software_mode_bool) {
            Ok(active) => {
                event_loop.set_control_flow(ControlFlow::Poll);
                self.active = Some(active);
            }
            Err(error) => {
                self.fatal = Some(error);
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.fatal.is_some() {
            event_loop.exit();
            return;
        }
        let Some(active) = self.active.as_mut() else {
            return;
        };
        match event {
            WindowEvent::CloseRequested | WindowEvent::Destroyed => event_loop.exit(),
            WindowEvent::Resized(size) => active.on_resized(size.width, size.height),
            WindowEvent::RedrawRequested => {
                if let Err(error) = active.redraw() {
                    self.fatal = Some(error);
                    event_loop.exit();
                }
            }
            WindowEvent::KeyboardInput { event, .. } => active.on_key(&event),
            WindowEvent::ModifiersChanged(modifiers) => active.on_modifiers(modifiers),
            WindowEvent::CursorMoved { position, .. } => active.on_cursor_moved(position),
            WindowEvent::CursorLeft { .. } => active.on_cursor_left(),
            WindowEvent::MouseInput { state, button, .. } => {
                active.on_mouse_button(state, button);
            }
            WindowEvent::MouseWheel { delta, phase, .. } => active.on_wheel(delta, phase),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.fatal.is_some() {
            event_loop.exit();
            return;
        }
        if let Some(active) = self.active.as_ref() {
            active.window.request_redraw();
        }
    }
}

/// Demo sim orbit behind the ticker-only shell.
///
/// Fixed-step scheduler plus a circular Mars-like orbit matching the
/// `tests/shell_phase_a.rs` golden profile; seeds advance per tick.
#[derive(Debug)]
struct SimDriver {
    /// Fixed-step scheduler owning tick count plus elapsed time.
    scheduler: Scheduler,
    /// Current point-ship state vector.
    state: engine::trajectory::StateVector,
    /// Mars-like reference body parameters.
    body: engine::body::BodyParams,
    /// Mars-like atmosphere parameters.
    atmosphere: engine::atmosphere::AtmosphereParams,
    /// Point-ship vehicle parameters.
    vehicle: engine::trajectory::VehicleParams,
    /// Gravity parameter for the coast model.
    mu: engine::orbit::Mu,
    /// Run master seed, dimensionless.
    master_seed_u64: u64,
    /// Per-tick stream seed, dimensionless.
    stream_seed_u64: u64,
}

impl SimDriver {
    /// Build the circular demo orbit at the cruise altitude.
    ///
    /// # Errors
    ///
    /// Returns [`OsWindowError`] for rejected atmosphere, gravity, or state.
    fn cruise() -> Result<Self, OsWindowError> {
        let body = engine::body::BodyParams::mars_like();
        let atmosphere = engine::atmosphere::AtmosphereParams::mars_like()
            .map_err(|error| OsWindowError::SimInit(error.to_string()))?;
        let vehicle = engine::trajectory::VehicleParams::preset();
        let mu = engine::orbit::Mu::new(body.gravitational_parameter_m3_s2())
            .map_err(|error| OsWindowError::SimInit(error.to_string()))?;
        let radius_m_f64 = body.radius_m().value() + DEMO_CRUISE_ALTITUDE_M_F64;
        let speed_mps_f64 = libm::sqrt(mu.value() / radius_m_f64);
        let state = engine::trajectory::StateVector::new(
            glam::DVec3::new(radius_m_f64, 0.0, 0.0),
            glam::DVec3::new(0.0, speed_mps_f64, 0.0),
            engine::units::Seconds::new(0.0),
        )
        .map_err(|error| OsWindowError::SimInit(error.to_string()))?;
        Ok(Self {
            scheduler: Scheduler::default(),
            state,
            body,
            atmosphere,
            vehicle,
            mu,
            master_seed_u64: DEMO_MASTER_SEED_U64,
            stream_seed_u64: DEMO_MASTER_SEED_U64,
        })
    }
}

/// Accumulated egui input between redraws.
///
/// Pointer position stays `None` until the first cursor move; buttons without
/// a known position are dropped rather than guessed.
#[derive(Debug, Default)]
struct InputAccum {
    /// Pending egui events drained on every redraw.
    events: Vec<egui::Event>,
    /// Current modifier state from the OS.
    modifiers: egui::Modifiers,
    /// Last cursor position in points, if seen.
    pointer_pos_points: Option<egui::Pos2>,
}

/// Frame clocks plus the fixed-step accumulator.
///
/// Wall-clock only; sim time advances through the scheduler, never from these.
#[derive(Debug)]
struct FrameClocks {
    /// Process instant at window creation.
    start: Instant,
    /// Instant of the previous redraw.
    last: Instant,
    /// Elapsed wall time in seconds for egui animation time.
    elapsed_s_f64: f64,
    /// Last frame delta in seconds for the sim accumulator.
    frame_dt_s_f64: f64,
    /// Unconsumed sim time in seconds.
    accumulator_s_f64: f64,
}

impl FrameClocks {
    /// Start all clocks at window creation.
    fn start_now() -> Self {
        let now = Instant::now();
        Self {
            start: now,
            last: now,
            elapsed_s_f64: 0.0,
            frame_dt_s_f64: 0.0,
            accumulator_s_f64: 0.0,
        }
    }
}

/// GPU-bound window state created on first resume.
///
/// Owns the winit window, wgpu surface plus device, egui-wgpu renderer, egui
/// context, ticker-only shell, and demo sim on the main thread.
struct ActiveWindow {
    /// OS window handle shared with the wgpu surface.
    window: Arc<Window>,
    /// Wgpu presentation surface bound to the window.
    surface: wgpu::Surface<'static>,
    /// Wgpu logical device for shell rendering.
    device: wgpu::Device,
    /// Wgpu queue for shell uploads plus submits.
    queue: wgpu::Queue,
    /// Current surface configuration for reconfigure on resize.
    surface_config: wgpu::SurfaceConfiguration,
    /// Immediate-mode shell renderer over the surface format.
    renderer: egui_wgpu::Renderer,
    /// Immediate-mode context feeding the shell draw.
    egui: egui::Context,
    /// Ticker-only shell assembly with run control plus inspect.
    shell: DesktopWindow,
    /// Budget denominators passed at draw time, never stored elsewhere.
    budgets: BudgetDenominators,
    /// Demo orbit behind the shell snapshots.
    sim: SimDriver,
    /// Accumulated input drained per redraw.
    input: InputAccum,
    /// Frame clocks plus sim accumulator.
    clocks: FrameClocks,
}

impl ActiveWindow {
    /// Create the window, surface, device, renderer, shell, and sim.
    ///
    /// Runs on the main thread inside resume; blocks only here while the
    /// adapter plus device resolve. The frame loop never blocks. Adapter
    /// resolution runs the scored backend enumeration plus fallback chain in
    /// [`request_adapter_with_fallbacks`], or the forced low-power
    /// force-fallback software path when `software_mode_bool` holds
    /// ([`SOFTWARE_FLAG`]); device resolution uses the
    /// conservative descriptor plus one force-fallback retry in
    /// [`request_device_with_fallback`]. The upstream-access-violation
    /// caveat on GPU-less Windows hosts is documented on
    /// [`preflight_adapter_probe`].
    ///
    /// # Errors
    ///
    /// Returns [`OsWindowError`] for window, surface, adapter, device,
    /// shell, sim, or budget failures.
    fn create(
        event_loop: &ActiveEventLoop,
        software_mode_bool: bool,
    ) -> Result<Self, OsWindowError> {
        let shell = DesktopWindow::open()?;
        let size_config = shell.config();
        let attrs = Window::default_attributes()
            .with_title(DESKTOP_WINDOW_TITLE)
            .with_inner_size(LogicalSize::new(
                size_config.width_pt_f32(),
                size_config.height_pt_f32(),
            ));
        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .map_err(|error| OsWindowError::Window(error.to_string()))?,
        );
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance
            .create_surface(Arc::clone(&window))
            .map_err(|error| OsWindowError::Surface(error.to_string()))?;
        let adapter =
            request_adapter_with_fallbacks(&instance, Some(&surface), software_mode_bool)?;
        let capabilities = surface.get_capabilities(&adapter);
        let Some(format) = capabilities
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .or_else(|| capabilities.formats.first().copied())
        else {
            return Err(OsWindowError::NoSurfaceFormat);
        };
        let (device, queue) = request_device_with_fallback(&instance, &surface, &adapter)?;
        let size = window.inner_size();
        let mut surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width: size.width.max(MIN_SURFACE_EXTENT_PX_U32),
            height: size.height.max(MIN_SURFACE_EXTENT_PX_U32),
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: SURFACE_MAX_LATENCY_U32,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: Vec::new(),
        };
        surface.configure(&device, &surface_config);
        surface_config.width = size.width.max(MIN_SURFACE_EXTENT_PX_U32);
        surface_config.height = size.height.max(MIN_SURFACE_EXTENT_PX_U32);
        let renderer =
            egui_wgpu::Renderer::new(&device, format, egui_wgpu::RendererOptions::default());
        let egui = egui::Context::default();
        egui.set_visuals(crate::theme::dev_dark_pro_visuals());
        let budgets = BudgetDenominators::new(
            FRAME_BUDGET_MS_F64,
            SIM_TICK_AVG_BUDGET_MS_F64,
            SIM_TICK_P99_BUDGET_MS_F64,
            SURFACE_HITCH_P95_BUDGET_MS_F64,
            MEMORY_CEILING_MB_F64,
            COLD_START_BUDGET_S_F64,
        )
        .map_err(|error| OsWindowError::Budgets(error.to_string()))?;
        let sim = SimDriver::cruise()?;
        Ok(Self {
            window,
            surface,
            device,
            queue,
            surface_config,
            renderer,
            egui,
            shell,
            budgets,
            sim,
            input: InputAccum::default(),
            clocks: FrameClocks::start_now(),
        })
    }

    /// Reconfigure the surface after a non-zero resize.
    ///
    /// Zero extents (minimized window) are skipped; the next non-zero resize
    /// or frame reconfigures.
    fn on_resized(&mut self, width_px_u32: u32, height_px_u32: u32) {
        if width_px_u32 < MIN_SURFACE_EXTENT_PX_U32 || height_px_u32 < MIN_SURFACE_EXTENT_PX_U32 {
            return;
        }
        self.surface_config.width = width_px_u32;
        self.surface_config.height = height_px_u32;
        self.surface.configure(&self.device, &self.surface_config);
    }

    /// Handle one keyboard event for the shell router only.
    ///
    /// `F3` toggles focus and `Escape` returns to passthrough per
    /// `docs/tech/debug.md` section 5; remaining keys stay deferred until
    /// widget text input lands.
    fn on_key(&mut self, event: &winit::event::KeyEvent) {
        if event.state != ElementState::Pressed || event.repeat {
            return;
        }
        match event.physical_key {
            PhysicalKey::Code(KeyCode::F3) => {
                self.shell.handle_key(RouterKey::F3);
            }
            PhysicalKey::Code(KeyCode::Escape) => {
                self.shell.handle_key(RouterKey::Escape);
            }
            _ => {}
        }
    }

    /// Store the OS modifier state for the next redraw.
    ///
    /// Queues a modifier-changed event alongside the stored state because
    /// egui 0.36 carries modifiers per event, not per frame.
    fn on_modifiers(&mut self, modifiers: winit::event::Modifiers) {
        let state = modifiers.state();
        let ctrl_bool = state.control_key();
        let super_bool = state.super_key();
        let mac_bool = cfg!(target_os = "macos");
        self.input.modifiers = egui::Modifiers {
            alt: state.alt_key(),
            ctrl: ctrl_bool,
            shift: state.shift_key(),
            mac_cmd: mac_bool && super_bool,
            command: if mac_bool { super_bool } else { ctrl_bool },
        };
        self.input
            .events
            .push(egui::Event::ModifiersChanged(self.input.modifiers));
    }

    /// Store the cursor position and queue a pointer move.
    fn on_cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        let scale_f64 = self.window.scale_factor();
        let pos = egui::pos2(
            physical_px_to_points_f32(position.x, scale_f64),
            physical_px_to_points_f32(position.y, scale_f64),
        );
        self.input.pointer_pos_points = Some(pos);
        self.input.events.push(egui::Event::PointerMoved(pos));
    }

    /// Queue a pointer-gone event and forget the position.
    fn on_cursor_left(&mut self) {
        self.input.pointer_pos_points = None;
        self.input.events.push(egui::Event::PointerGone);
    }

    /// Queue a mouse button press or release at the known position.
    ///
    /// Buttons without a known cursor position are dropped rather than
    /// guessed; unmapped extra buttons are ignored.
    fn on_mouse_button(&mut self, state: ElementState, button: MouseButton) {
        let mapped: Option<egui::PointerButton> = match button {
            MouseButton::Left => Some(egui::PointerButton::Primary),
            MouseButton::Right => Some(egui::PointerButton::Secondary),
            MouseButton::Middle => Some(egui::PointerButton::Middle),
            MouseButton::Back => Some(egui::PointerButton::Extra1),
            MouseButton::Forward => Some(egui::PointerButton::Extra2),
            MouseButton::Other(_) => None,
        };
        let (Some(mapped_button), Some(pos)) = (mapped, self.input.pointer_pos_points) else {
            return;
        };
        self.input.events.push(egui::Event::PointerButton {
            pos,
            button: mapped_button,
            pressed: state == ElementState::Pressed,
            modifiers: self.input.modifiers,
        });
    }

    /// Queue a mouse wheel scroll with the current modifiers.
    fn on_wheel(&mut self, delta: MouseScrollDelta, phase: WinitTouchPhase) {
        let scale_f64 = self.window.scale_factor();
        let (unit, scrolled) = match delta {
            MouseScrollDelta::LineDelta(x_f32, y_f32) => {
                (egui::MouseWheelUnit::Line, egui::vec2(x_f32, y_f32))
            }
            MouseScrollDelta::PixelDelta(position) => (
                egui::MouseWheelUnit::Point,
                egui::vec2(
                    physical_px_to_points_f32(position.x, scale_f64),
                    physical_px_to_points_f32(position.y, scale_f64),
                ),
            ),
        };
        let mapped_phase = match phase {
            WinitTouchPhase::Started => egui::TouchPhase::Start,
            WinitTouchPhase::Moved => egui::TouchPhase::Move,
            WinitTouchPhase::Ended => egui::TouchPhase::End,
            WinitTouchPhase::Cancelled => egui::TouchPhase::Cancel,
        };
        self.input.events.push(egui::Event::MouseWheel {
            unit,
            delta: scrolled,
            phase: mapped_phase,
            modifiers: self.input.modifiers,
        });
    }

    /// Advance the demo sim under pause plus step plus warp control.
    ///
    /// Paused shells advance only on an explicit step; running shells scale
    /// wall time by the requested warp factor with per-frame catch-up capped
    /// by [`MAX_CATCH_UP_STEPS_PER_FRAME_U64`]. The demo orbit always cruises,
    /// so snapshot warp context stays cruise.
    ///
    /// # Errors
    ///
    /// Returns [`OsWindowError`] for sim-step or snapshot failures.
    fn advance_sim(&mut self) -> Result<(), OsWindowError> {
        if self.shell.shell().top_bar().is_paused() {
            if self.shell.shell_mut().top_bar_mut().take_step() {
                self.step_sim_once()?;
            }
            return Ok(());
        }
        let factor_f64 = self.shell.shell().top_bar().requested_warp().factor();
        self.clocks.accumulator_s_f64 += self.clocks.frame_dt_s_f64 * factor_f64;
        let mut steps_u64 = 0_u64;
        while self.clocks.accumulator_s_f64 >= SIM_TICK_S.value()
            && steps_u64 < MAX_CATCH_UP_STEPS_PER_FRAME_U64
        {
            self.step_sim_once()?;
            self.clocks.accumulator_s_f64 -= SIM_TICK_S.value();
            steps_u64 += 1;
        }
        if steps_u64 >= MAX_CATCH_UP_STEPS_PER_FRAME_U64 {
            self.clocks.accumulator_s_f64 = 0.0;
        }
        Ok(())
    }

    /// Step the scheduler plus state once and observe the snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`OsWindowError`] for sim-step, snapshot, or shell failures.
    fn step_sim_once(&mut self) -> Result<(), OsWindowError> {
        self.sim.scheduler.advance();
        let sample = engine::trajectory::step_point_ship(
            &self.sim.state,
            SIM_TICK_S,
            &self.sim.body,
            &self.sim.atmosphere,
            &self.sim.vehicle,
            self.sim.mu,
        )
        .map_err(|error| OsWindowError::SimStep(error.to_string()))?;
        self.sim.state = sample.state;
        self.sim.stream_seed_u64 =
            engine::generation::mix_seed(self.sim.stream_seed_u64, self.sim.scheduler.step_count());
        let requested_warp = self.shell.shell().top_bar().requested_warp();
        let snapshot = engine::inspect::capture_snapshot(
            &self.sim.scheduler,
            &self.sim.state,
            &self.sim.body,
            &self.sim.atmosphere,
            &self.sim.vehicle,
            self.sim.master_seed_u64,
            self.sim.stream_seed_u64,
            requested_warp,
            true,
            false,
            false,
        )?;
        self.shell.shell_mut().observe_snapshot(&snapshot)?;
        Ok(())
    }

    /// Run one redraw: clocks, sim, egui pass, and wgpu present.
    ///
    /// Surface loss reconfigures and skips the frame; occlusion plus timeout
    /// skips without error; shell or sim failures return for the caller to
    /// park as fatal and exit.
    ///
    /// # Errors
    ///
    /// Returns [`OsWindowError`] for sim, snapshot, or shell failures.
    fn redraw(&mut self) -> Result<(), OsWindowError> {
        self.tick_clocks();
        self.advance_sim()?;
        let size = self.window.inner_size();
        if size.width < MIN_SURFACE_EXTENT_PX_U32 || size.height < MIN_SURFACE_EXTENT_PX_U32 {
            return Ok(());
        }
        if size.width != self.surface_config.width || size.height != self.surface_config.height {
            self.on_resized(size.width, size.height);
        }
        let scale_f64 = self.window.scale_factor();
        let output = self.run_shell_pass(size, scale_f64)?;
        self.present_frame(output, size, scale_f64);
        Ok(())
    }

    /// Advance wall clocks and sample the frame time.
    ///
    /// Feeds the budget-strip frame badge; unusable deltas are skipped
    /// without touching shell state.
    fn tick_clocks(&mut self) {
        let now = Instant::now();
        let frame_dt_s_f64 = now.duration_since(self.clocks.last).as_secs_f64();
        self.clocks.last = now;
        self.clocks.elapsed_s_f64 = now.duration_since(self.clocks.start).as_secs_f64();
        self.clocks.frame_dt_s_f64 = frame_dt_s_f64;
        let frame_ms_f64 = frame_dt_s_f64 * MILLIS_PER_SECOND_F64;
        if frame_ms_f64.is_finite() && frame_ms_f64 >= 0.0 {
            let _ = self
                .shell
                .shell_mut()
                .budget_strip_mut()
                .set_frame_ms_f64(frame_ms_f64);
        }
    }

    /// Run the immediate-mode shell pass and return its output.
    ///
    /// Drains accumulated input into one [`DesktopWindow`](crate::shell::DesktopWindow)
    /// draw through the measured cost hook. Export requests stay deferred:
    /// bundle identity needs the game loop.
    ///
    /// # Errors
    ///
    /// Returns [`OsWindowError`] for shell draw failures.
    fn run_shell_pass(
        &mut self,
        size: PhysicalSize<u32>,
        scale_f64: f64,
    ) -> Result<egui::FullOutput, OsWindowError> {
        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(
                    physical_px_to_points_f32(f64::from(size.width), scale_f64),
                    physical_px_to_points_f32(f64::from(size.height), scale_f64),
                ),
            )),
            time: Some(self.clocks.elapsed_s_f64),
            events: core::mem::take(&mut self.input.events),
            ..Default::default()
        };
        let mut shell_error: Option<ShellError> = None;
        let budgets = self.budgets;
        let pass_ctx = self.egui.clone();
        let output = pass_ctx.run_ui(raw_input, |ui| {
            match self
                .shell
                .draw_measured(&pass_ctx, ui, FRAME_BUDGET_MS_F64, budgets, false)
            {
                Ok(_action) => {}
                Err(error) => {
                    shell_error = Some(error);
                }
            }
        });
        if let Some(error) = shell_error {
            return Err(OsWindowError::Shell(error));
        }
        Ok(output)
    }

    /// Present one shell output frame through the wgpu surface.
    ///
    /// Uploads texture deltas before painting, tessellates, acquires the next
    /// surface texture, renders the shell pass, presents, then frees retired
    /// textures and clears the delta so the debug-only `TexturesDelta` drop
    /// guard never fires. Surface loss reconfigures and skips; occlusion plus
    /// timeout skips silently; both skip paths still free and clear.
    fn present_frame(
        &mut self,
        mut output: egui::FullOutput,
        size: PhysicalSize<u32>,
        scale_f64: f64,
    ) {
        for (texture_id, image_deltas) in &output.textures_delta.set {
            for image_delta in image_deltas {
                self.renderer
                    .update_texture(&self.device, &self.queue, *texture_id, image_delta);
            }
        }
        let pixels_per_point_f32 = pixels_per_point_f32(scale_f64);
        let paint_jobs = self.egui.tessellate(output.shapes, output.pixels_per_point);
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [size.width, size.height],
            pixels_per_point: pixels_per_point_f32,
        };
        let frame_option = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => Some(frame),
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.on_resized(size.width, size.height);
                None
            }
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => None,
        };
        if let Some(frame) = frame_option {
            let view = frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("os-window-egui-encoder"),
                });
            let user_buffers = self.renderer.update_buffers(
                &self.device,
                &self.queue,
                &mut encoder,
                &paint_jobs,
                &screen_descriptor,
            );
            {
                let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("os-window-egui-pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(clear_color_f64()),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                self.renderer
                    .render(&mut pass.forget_lifetime(), &paint_jobs, &screen_descriptor);
            }
            self.queue.submit(
                user_buffers
                    .into_iter()
                    .chain(std::iter::once(encoder.finish())),
            );
            self.queue.present(frame);
        }
        for texture_id in &output.textures_delta.free {
            self.renderer.free_texture(texture_id);
        }
        output.textures_delta.clear();
    }
}

/// Convert a physical pixel measure into egui points.
///
/// Divides by the OS scale factor; unusable scales fall back to
/// [`FALLBACK_PIXELS_PER_POINT_F64`].
#[must_use]
pub fn physical_px_to_points_f32(physical_px_f64: f64, scale_factor_f64: f64) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "egui points are f32; sub-pixel loss is invisible"
    )]
    let points_f32 = (physical_px_f64 / sanitized_scale_f64(scale_factor_f64)) as f32;
    points_f32
}

/// Convert an OS scale factor into egui pixels-per-point.
///
/// Falls back to [`FALLBACK_PIXELS_PER_POINT_F64`] for non-finite or
/// non-positive scales.
#[must_use]
pub fn pixels_per_point_f32(scale_factor_f64: f64) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "DPI scale sits near 1; f32 precision suffices for egui"
    )]
    let narrowed_f32 = sanitized_scale_f64(scale_factor_f64) as f32;
    narrowed_f32
}

/// Sanitize an OS scale factor to a strictly positive finite value.
///
/// Returns the scale unchanged when finite and positive, else
/// [`FALLBACK_PIXELS_PER_POINT_F64`].
fn sanitized_scale_f64(scale_factor_f64: f64) -> f64 {
    if scale_factor_f64.is_finite() && scale_factor_f64 > 0.0 {
        scale_factor_f64
    } else {
        FALLBACK_PIXELS_PER_POINT_F64
    }
}

/// Build the opaque clear color from the DevDark-Pro base.
///
/// Channels convert as gamma-space values matching the theme swatch; the
/// exact output curve stays deferred with the phone color checks.
fn clear_color_f64() -> wgpu::Color {
    let channels = BASE_BACKGROUND_RGB_U8.to_array_u8();
    wgpu::Color {
        r: f64::from(channels[0]) / RGB_CHANNEL_MAX_F64,
        g: f64::from(channels[1]) / RGB_CHANNEL_MAX_F64,
        b: f64::from(channels[2]) / RGB_CHANNEL_MAX_F64,
        a: CLEAR_ALPHA_F64,
    }
}

/// Park the current thread until an init future resolves.
///
/// Init-time only: adapter plus device requests park here during resume. The
/// frame loop never blocks; wgpu wakes the parker from its worker threads.
///
/// [`Future`]: std::future::Future
fn block_on_init<Fut>(future: Fut) -> Fut::Output
where
    Fut: Future,
{
    /// Waker parking the init thread until wgpu workers resolve.
    struct ParkWaker {
        /// Thread to unpark on wake.
        owner: thread::Thread,
    }

    impl Wake for ParkWaker {
        fn wake(self: Arc<Self>) {
            self.owner.unpark();
        }
    }

    let waker: Waker = Waker::from(Arc::new(ParkWaker {
        owner: thread::current(),
    }));
    let mut context = TaskContext::from_waker(&waker);
    let mut pinned = pin!(future);
    loop {
        match pinned.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => thread::park(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMOKE_SCALE_F64: f64 = 2.0;
    const EXPECTED_HALF_POINTS_F32: f32 = 100.0;
    const EXPECTED_SCALE_POINTS_F32: f32 = 2.0;
    const EXPECTED_FALLBACK_POINTS_F32: f32 = 1.0;
    const EXPECTED_FULL_POINTS_F32: f32 = 200.0;
    const SMOKE_PHYSICAL_PX_F64: f64 = 200.0;
    const POINTS_TOL_F32: f32 = 1e-6;

    fn args_of(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| String::from(*value)).collect()
    }

    #[test]
    fn flag_constant_spells_run_window() {
        assert_eq!(RUN_WINDOW_FLAG, "--run-window");
        assert_eq!(HeadlessReason::FlagMissing.label(), "flag-missing");
        assert_eq!(HeadlessReason::DisplayMissing.label(), "display-missing");
    }

    #[test]
    fn launch_needs_flag_and_display() {
        let flagged = args_of(&["universe-debug", "--run-window"]);
        assert_eq!(
            decide_launch_with(&flagged, true),
            LaunchDecision::OpenWindow
        );
        assert_eq!(
            decide_launch_with(&flagged, false),
            LaunchDecision::StayHeadless(HeadlessReason::DisplayMissing)
        );
        let bare = args_of(&["universe-debug"]);
        assert_eq!(
            decide_launch_with(&bare, true),
            LaunchDecision::StayHeadless(HeadlessReason::FlagMissing)
        );
        assert_eq!(
            decide_launch_with(&bare, false),
            LaunchDecision::StayHeadless(HeadlessReason::FlagMissing)
        );
        let empty: Vec<String> = Vec::new();
        assert_eq!(
            decide_launch_with(&empty, true),
            LaunchDecision::StayHeadless(HeadlessReason::FlagMissing)
        );
    }

    #[test]
    fn display_probe_requires_a_named_display() {
        assert!(!display_available_with(None, None));
        assert!(!display_available_with(Some(""), None));
        assert!(!display_available_with(None, Some("")));
        assert!(display_available_with(Some(":0"), None));
        assert!(display_available_with(None, Some("wayland-0")));
        assert!(display_available_with(Some(":0"), Some("wayland-0")));
    }

    #[test]
    fn scale_helpers_convert_and_fall_back() {
        assert!(
            (physical_px_to_points_f32(SMOKE_PHYSICAL_PX_F64, SMOKE_SCALE_F64)
                - EXPECTED_HALF_POINTS_F32)
                .abs()
                < POINTS_TOL_F32
        );
        assert!(
            (pixels_per_point_f32(SMOKE_SCALE_F64) - EXPECTED_SCALE_POINTS_F32).abs()
                < POINTS_TOL_F32
        );
        assert!(
            (pixels_per_point_f32(f64::NAN) - EXPECTED_FALLBACK_POINTS_F32).abs() < POINTS_TOL_F32
        );
        assert!((pixels_per_point_f32(0.0) - EXPECTED_FALLBACK_POINTS_F32).abs() < POINTS_TOL_F32);
        assert!(
            (physical_px_to_points_f32(SMOKE_PHYSICAL_PX_F64, f64::NAN) - EXPECTED_FULL_POINTS_F32)
                .abs()
                < POINTS_TOL_F32
        );
    }

    #[test]
    fn frame_budgets_build_for_the_window() {
        let Ok(_budgets) = BudgetDenominators::new(
            FRAME_BUDGET_MS_F64,
            SIM_TICK_AVG_BUDGET_MS_F64,
            SIM_TICK_P99_BUDGET_MS_F64,
            SURFACE_HITCH_P95_BUDGET_MS_F64,
            MEMORY_CEILING_MB_F64,
            COLD_START_BUDGET_S_F64,
        ) else {
            panic!("window budgets must build")
        };
    }

    #[test]
    fn ticker_only_config_sizes_the_window() {
        let config = crate::layout::DesktopWindowConfig::ticker_only();
        assert!(config.width_pt_f32() > 0.0);
        assert!(config.height_pt_f32() > 0.0);
        assert!(!DESKTOP_WINDOW_TITLE.is_empty());
    }

    #[test]
    fn block_on_runs_an_init_future() {
        let output = block_on_init(async { 2_u32 + 2_u32 });
        assert_eq!(output, 4_u32);
    }

    #[test]
    fn headless_reason_renders_for_ci_logs() {
        assert!(
            HeadlessReason::FlagMissing
                .to_string()
                .contains(RUN_WINDOW_FLAG)
        );
        assert!(!HeadlessReason::DisplayMissing.to_string().is_empty());
    }

    #[test]
    fn adapter_backends_label_covers_fallback() {
        assert!(ADAPTER_BACKENDS_LABEL.contains("dx12"));
        assert!(ADAPTER_BACKENDS_LABEL.contains("gl"));
        assert!(!ADAPTER_BACKENDS_LABEL.is_empty());
    }

    #[test]
    fn no_adapter_detail_names_backends_host_and_headless_path() {
        let detail = no_adapter_detail("wgpu-probe-report");
        assert!(detail.contains("wgpu-probe-report"));
        assert!(detail.contains(ADAPTER_BACKENDS_LABEL));
        assert!(detail.contains(std::env::consts::OS));
        assert!(detail.contains(RUN_WINDOW_FLAG));
        assert!(detail.contains("headless"));
    }

    #[test]
    fn no_adapter_error_renders_detail() {
        let error = OsWindowError::NoAdapter(String::from("probe-detail"));
        let rendered = error.to_string();
        assert!(rendered.contains("no compatible wgpu adapter"));
        assert!(rendered.contains("probe-detail"));
    }

    #[test]
    fn conservative_descriptor_stays_within_downlevel_limits() {
        let descriptor = conservative_device_descriptor();
        assert_eq!(descriptor.label, Some(DEVICE_LABEL));
        assert!(descriptor.required_features.is_empty());
        assert_eq!(
            descriptor.required_limits,
            wgpu::Limits::downlevel_defaults()
        );
        assert!(matches!(
            descriptor.memory_hints,
            wgpu::MemoryHints::Performance
        ));
    }

    #[test]
    fn device_error_renders_detail() {
        let error = OsWindowError::Device(String::from("device-detail"));
        let rendered = error.to_string();
        assert!(rendered.contains("os window device"));
        assert!(rendered.contains("device-detail"));
    }

    #[test]
    fn backend_order_label_names_scored_probes() {
        assert_eq!(ADAPTER_BACKEND_ORDER_LABEL, "vulkan>dx12>metal>gl");
        assert!(ADAPTER_BACKEND_ORDER_LABEL.contains("vulkan"));
        assert!(ADAPTER_BACKEND_ORDER_LABEL.contains("dx12"));
        assert!(ADAPTER_BACKEND_ORDER_LABEL.contains("gl"));
    }

    #[test]
    fn device_scores_rank_discrete_above_cpu() {
        assert!(
            device_type_score_u8(wgpu::DeviceType::DiscreteGpu)
                > device_type_score_u8(wgpu::DeviceType::IntegratedGpu)
        );
        assert!(
            device_type_score_u8(wgpu::DeviceType::IntegratedGpu)
                > device_type_score_u8(wgpu::DeviceType::Other)
        );
        assert!(
            device_type_score_u8(wgpu::DeviceType::Other)
                > device_type_score_u8(wgpu::DeviceType::VirtualGpu)
        );
        assert!(
            device_type_score_u8(wgpu::DeviceType::VirtualGpu)
                > device_type_score_u8(wgpu::DeviceType::Cpu)
        );
    }

    #[test]
    fn backend_scores_rank_vulkan_above_gl() {
        assert!(backend_score_u8(wgpu::Backend::Vulkan) > backend_score_u8(wgpu::Backend::Dx12));
        assert!(backend_score_u8(wgpu::Backend::Dx12) > backend_score_u8(wgpu::Backend::Metal));
        assert!(backend_score_u8(wgpu::Backend::Metal) > backend_score_u8(wgpu::Backend::Gl));
    }

    #[test]
    fn backend_probes_enumerate_vulkan_first() {
        let probes = ordered_backend_probes();
        assert_eq!(probes.len(), 4);
        assert_eq!(probes[0].1, "vulkan");
        assert_eq!(probes[1].1, "dx12");
        assert_eq!(probes[2].1, "metal");
        assert_eq!(probes[3].1, "gl");
        assert_eq!(probes[0].0, wgpu::Backends::VULKAN);
        assert_eq!(probes[1].0, wgpu::Backends::DX12);
        assert_eq!(probes[2].0, wgpu::Backends::METAL);
        assert_eq!(probes[3].0, wgpu::Backends::GL);
    }

    #[test]
    fn adapter_info_summary_names_backend_and_driver() {
        let mut info = wgpu::AdapterInfo::new(wgpu::DeviceType::DiscreteGpu, wgpu::Backend::Vulkan);
        info.name = String::from("test-adapter");
        info.driver = String::from("test-driver");
        info.driver_info = String::from("driver-detail");
        let summary = format_adapter_info(&info);
        assert!(summary.contains("vulkan"));
        assert!(summary.contains("test-adapter"));
        assert!(summary.contains("DiscreteGpu"));
        assert!(summary.contains("test-driver"));
        assert!(summary.contains("driver-detail"));
    }

    #[test]
    fn best_adapter_in_empty_list_is_none() {
        let empty: Vec<wgpu::Adapter> = Vec::new();
        assert!(best_adapter_in_list(&empty, None).is_none());
    }

    #[test]
    fn software_flag_constant_spells_software() {
        assert_eq!(SOFTWARE_FLAG, "--software");
    }

    #[test]
    fn software_requested_detects_flag() {
        let bare = args_of(&["universe-debug"]);
        assert!(!software_requested(&bare));
        let window_only = args_of(&["universe-debug", "--run-window"]);
        assert!(!software_requested(&window_only));
        let software_only = args_of(&["universe-debug", "--software"]);
        assert!(software_requested(&software_only));
        let both = args_of(&["universe-debug", "--run-window", "--software"]);
        assert!(software_requested(&both));
    }

    #[test]
    fn software_alone_stays_headless_flag_missing() {
        let software_only = args_of(&["universe-debug", "--software"]);
        assert_eq!(
            decide_launch_with(&software_only, true),
            LaunchDecision::StayHeadless(HeadlessReason::FlagMissing)
        );
        assert_eq!(
            decide_launch_with(&software_only, false),
            LaunchDecision::StayHeadless(HeadlessReason::FlagMissing)
        );
    }

    #[test]
    fn software_with_run_window_keeps_display_gate() {
        let both = args_of(&["universe-debug", "--run-window", "--software"]);
        assert_eq!(decide_launch_with(&both, true), LaunchDecision::OpenWindow);
        assert_eq!(
            decide_launch_with(&both, false),
            LaunchDecision::StayHeadless(HeadlessReason::DisplayMissing)
        );
    }

    #[test]
    fn live_frame_delta_apply_then_clear_leaves_no_unapplied() {
        let mut delta = egui::epaint::textures::TexturesDelta::default();
        assert!(delta.is_empty());
        let image = egui::epaint::ColorImage::filled([1, 1], egui::epaint::Color32::WHITE);
        let image_data = egui::epaint::ImageData::Color(std::sync::Arc::new(image));
        delta.push(
            egui::epaint::TextureId::Managed(0),
            egui::epaint::ImageDelta::full(
                image_data,
                egui::epaint::textures::TextureOptions::default(),
            ),
        );
        delta.free(egui::epaint::TextureId::Managed(1));
        let mut applied_usize = 0_usize;
        for image_deltas in delta.set.values() {
            applied_usize += image_deltas.len();
        }
        assert_eq!(applied_usize, 1);
        assert_eq!(delta.free.len(), 1);
        delta.clear();
        assert!(delta.is_empty());
    }
}
