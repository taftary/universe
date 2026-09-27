//! Desktop 4-dock plus phone skeleton as plain data.
//!
//! Hand-placed panels only; `egui_dock` stays deferred until measured layout
//! cost justifies it. Content never forks between presets or devices.
//! Input routing (`Passthrough` and `Focused`) lands in step 5, and
//! shell-cost wiring lands in step 4. Sizes scale with `pixels_per_point()`.

/// Minimum shell touch target in points.
///
/// Source: `docs/tech/debug.md` section 6.2.
pub const MIN_TOUCH_TARGET_PT_F32: f32 = 44.0;

/// Minimum plot height in points at half detent.
///
/// Source: `docs/tech/debug.md` section 6.2.
pub const PLOT_MIN_HEIGHT_PT_F32: f32 = 96.0;

/// Bottom-sheet half-detent height as a fraction.
///
/// Source: `docs/tech/debug.md` section 6.2.
pub const BOTTOM_SHEET_HALF_FRACTION_F64: f64 = 0.5;

/// Bottom-sheet full-detent height as a fraction.
///
/// Full detent never covers the dev tag; step 4 measures the inset.
/// Source: `docs/tech/debug.md` section 6.2.
pub const BOTTOM_SHEET_FULL_FRACTION_F64: f64 = 1.0;

/// Dev-tag reserve in points.
///
/// Equals the minimum touch target so full detent leaves the tag visible.
/// Source: `docs/tech/debug.md` section 6.2.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Bin smoke wiring lands in a later step; unit tests cover it meanwhile."
    )
)]
pub const DEV_TAG_RESERVE_PT_F32: f32 = MIN_TOUCH_TARGET_PT_F32;

/// Plot-history reservation in entries at shell open.
///
/// Skeleton size; later phases confirm it by measurement.
/// Source: `docs/tech/debug.md` section 8.
pub const PLOT_HISTORY_CAPACITY_ENTRIES_USIZE: usize = 2_048;

/// Log-history reservation in entries at shell open.
///
/// Skeleton size; later phases confirm it by measurement.
/// Source: `docs/tech/debug.md` section 8.
pub const LOG_HISTORY_CAPACITY_ENTRIES_USIZE: usize = 512;

/// Input-recorder reservation in entries at shell open.
///
/// Skeleton size; later phases confirm it by measurement.
/// Source: `docs/tech/debug.md` section 8.
pub const INPUT_RECORDER_CAPACITY_ENTRIES_USIZE: usize = 1_024;

/// Hash-history reservation in entries at shell open.
///
/// One hash per observed frame, matching the plot-history reservation.
/// Source: `docs/tech/debug.md` section 8.
pub const HASH_HISTORY_CAPACITY_ENTRIES_USIZE: usize = 2_048;

/// Top-bar visibility bit for dock presets.
///
/// Source: `docs/tech/debug.md` section 6.1.
pub const TOP_BAR_BIT_U8: u8 = 0x01;

/// Left-panel visibility bit for dock presets.
///
/// Source: `docs/tech/debug.md` section 6.1.
pub const LEFT_PANEL_BIT_U8: u8 = 0x02;

/// Right-panel visibility bit for dock presets.
///
/// Source: `docs/tech/debug.md` section 6.1.
pub const RIGHT_PANEL_BIT_U8: u8 = 0x04;

/// Bottom-tabs visibility bit for dock presets.
///
/// Source: `docs/tech/debug.md` section 6.1.
pub const BOTTOM_TABS_BIT_U8: u8 = 0x08;

/// Layout range-check failures.
///
/// Returned for non-finite, negative, or non-positive inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayoutError {
    /// Value was non-finite and unusable.
    NonFinite {
        /// Rejected value.
        value_f64: f64,
    },
    /// Value was negative and below zero.
    Negative {
        /// Rejected value.
        value_f64: f64,
    },
    /// Budget denominator was not strictly positive.
    NonPositiveBudget {
        /// Rejected budget value.
        value_f64: f64,
    },
    /// Value was not strictly positive.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Bin smoke wiring lands in a later step; unit tests cover it meanwhile."
        )
    )]
    NonPositive {
        /// Rejected value.
        value_f64: f64,
    },
}

impl core::fmt::Display for LayoutError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NonFinite { value_f64 } => {
                write!(formatter, "non-finite layout value: {value_f64}")
            }
            Self::Negative { value_f64 } => {
                write!(formatter, "negative layout value: {value_f64}")
            }
            Self::NonPositiveBudget { value_f64 } => {
                write!(formatter, "non-positive budget: {value_f64}")
            }
            Self::NonPositive { value_f64 } => {
                write!(formatter, "non-positive layout value: {value_f64}")
            }
        }
    }
}

impl std::error::Error for LayoutError {}

/// Hand-placed dock region around the game view.
///
/// Regions follow `docs/tech/debug.md` section 6.1 on desktop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockRegion {
    /// Top bar with run control and badges.
    TopBar,
    /// Left panel with frames, overlays, and camera.
    LeftPanel,
    /// Right panel with inspector and tweakables.
    RightPanel,
    /// Bottom panel with tabbed monitor views.
    BottomTabs,
    /// Game view; the shell never owns it.
    CenterView,
}

impl DockRegion {
    /// All dock regions including the game view.
    pub const ALL: [Self; 5] = [
        Self::TopBar,
        Self::LeftPanel,
        Self::RightPanel,
        Self::BottomTabs,
        Self::CenterView,
    ];

    /// Return the short region label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::TopBar => "top-bar",
            Self::LeftPanel => "left-panel",
            Self::RightPanel => "right-panel",
            Self::BottomTabs => "bottom-tabs",
            Self::CenterView => "center-view",
        }
    }
}

/// Dock visibility bitmask for one preset.
///
/// Same panels everywhere; presets only switch visibility and size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelVisibility {
    /// Visible-region bits from the region constants.
    bits_u8: u8,
}

impl PanelVisibility {
    /// Build visibility from raw region bits.
    #[must_use]
    pub const fn from_bits(bits_u8: u8) -> Self {
        Self { bits_u8 }
    }

    /// Build visibility for a desktop preset.
    #[must_use]
    pub const fn for_preset(preset: DesktopPreset) -> Self {
        preset.visibility()
    }

    /// Return the raw visibility bits.
    #[must_use]
    pub const fn bits_u8(self) -> u8 {
        self.bits_u8
    }

    /// Report top-bar visibility.
    #[must_use]
    pub const fn shows_top_bar(self) -> bool {
        self.bits_u8 & TOP_BAR_BIT_U8 != 0
    }

    /// Report left-panel visibility.
    #[must_use]
    pub const fn shows_left_panel(self) -> bool {
        self.bits_u8 & LEFT_PANEL_BIT_U8 != 0
    }

    /// Report right-panel visibility.
    #[must_use]
    pub const fn shows_right_panel(self) -> bool {
        self.bits_u8 & RIGHT_PANEL_BIT_U8 != 0
    }

    /// Report bottom-tabs visibility.
    #[must_use]
    pub const fn shows_bottom_tabs(self) -> bool {
        self.bits_u8 & BOTTOM_TABS_BIT_U8 != 0
    }
}

/// Desktop visibility preset switching size without content change.
///
/// Preset choice is shell state only and never persists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopPreset {
    /// Handoff watching with top, left, and bottom.
    Descent,
    /// Replay work with top and bottom log.
    Determinism,
    /// Sustained runs with top and budget strip.
    Budget,
    /// Minimal occlusion with the top bar alone.
    TickerOnly,
}

impl DesktopPreset {
    /// All desktop presets.
    pub const ALL: [Self; 4] = [
        Self::Descent,
        Self::Determinism,
        Self::Budget,
        Self::TickerOnly,
    ];

    /// Return dock visibility for a preset.
    #[must_use]
    pub const fn visibility(self) -> PanelVisibility {
        match self {
            Self::Descent => {
                PanelVisibility::from_bits(TOP_BAR_BIT_U8 | LEFT_PANEL_BIT_U8 | BOTTOM_TABS_BIT_U8)
            }
            // Determinism and Budget share dock bits; their window and tab
            // selection differs in later phases.
            Self::Determinism | Self::Budget => {
                PanelVisibility::from_bits(TOP_BAR_BIT_U8 | BOTTOM_TABS_BIT_U8)
            }
            Self::TickerOnly => PanelVisibility::from_bits(TOP_BAR_BIT_U8),
        }
    }

    /// Return the short preset label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Descent => "descent",
            Self::Determinism => "determinism",
            Self::Budget => "budget",
            Self::TickerOnly => "ticker-only",
        }
    }

    /// Return the default bottom tab for a preset.
    ///
    /// Descent selects plots, Budget selects budget, Determinism selects
    /// log, and ticker-only selects none. Phase B uses the plots, budget,
    /// and log slice only; replay and console stay deferred.
    #[must_use]
    pub const fn default_bottom_tab(self) -> Option<PhoneTab> {
        match self {
            Self::Descent => Some(PhoneTab::Plots),
            Self::Budget => Some(PhoneTab::Budget),
            Self::Determinism => Some(PhoneTab::Log),
            Self::TickerOnly => None,
        }
    }
}

/// Applied desktop preset with visibility plus default tab.
///
/// One call site applies visibility and default tab together; sizes stay
/// user-resizable and content never changes per `docs/tech/debug.md` 6.1.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Bin smoke wiring lands in a later step; unit tests cover it meanwhile."
    )
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresetApplication {
    /// Dock visibility for the preset.
    visibility: PanelVisibility,
    /// Default bottom tab for the preset, if any.
    default_bottom_tab: Option<PhoneTab>,
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Bin smoke wiring lands in a later step; unit tests cover it meanwhile."
    )
)]
impl PresetApplication {
    /// Return dock visibility for the application.
    #[must_use]
    pub const fn visibility(self) -> PanelVisibility {
        self.visibility
    }

    /// Return the default bottom tab, if any.
    #[must_use]
    pub const fn default_bottom_tab(self) -> Option<PhoneTab> {
        self.default_bottom_tab
    }
}

/// Apply a desktop preset to visibility plus default tab.
///
/// Sizes stay user-resizable; content never changes. Combines
/// [`DesktopPreset::visibility`] with [`DesktopPreset::default_bottom_tab`].
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Bin smoke wiring lands in a later step; unit tests cover it meanwhile."
    )
)]
#[must_use]
pub const fn apply_desktop_preset(preset: DesktopPreset) -> PresetApplication {
    PresetApplication {
        visibility: preset.visibility(),
        default_bottom_tab: preset.default_bottom_tab(),
    }
}

/// Transient desktop preset choice held as shell state.
///
/// The choice never persists: presets, plot history, log filters, console
/// history, and recorder drafts are transient per `docs/tech/debug.md` 8.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Bin smoke wiring lands in a later step; unit tests cover it meanwhile."
    )
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresetSelection {
    /// Currently selected desktop preset.
    preset: DesktopPreset,
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Bin smoke wiring lands in a later step; unit tests cover it meanwhile."
    )
)]
impl PresetSelection {
    /// Whether a preset choice persists across runs.
    ///
    /// Always false; shell state is on the never-saved list.
    pub const PERSISTED_BOOL: bool = false;

    /// Build a transient selection from a preset.
    #[must_use]
    pub const fn new(preset: DesktopPreset) -> Self {
        Self { preset }
    }

    /// Return the selected preset.
    #[must_use]
    pub const fn preset(self) -> DesktopPreset {
        self.preset
    }

    /// Select another preset without touching content.
    pub const fn select(&mut self, preset: DesktopPreset) {
        self.preset = preset;
    }

    /// Apply the selection to visibility plus default tab.
    #[must_use]
    pub const fn apply(self) -> PresetApplication {
        apply_desktop_preset(self.preset)
    }
}

/// Desktop window default width in points.
///
/// Scaffold default only; the window is user-resizable and content never
/// forks with size. Source: `docs/tech/debug.md` section 6.1.
pub const DESKTOP_WINDOW_WIDTH_PT_F32: f32 = 1280.0;

/// Desktop window default height in points.
///
/// Scaffold default only; the window is user-resizable and content never
/// forks with size. Source: `docs/tech/debug.md` section 6.1.
pub const DESKTOP_WINDOW_HEIGHT_PT_F32: f32 = 800.0;

/// Desktop window title text.
///
/// Names the dev-only shell binary. Source: `crates/debug/Cargo.toml` binary name.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Bin smoke wiring lands in a later step; unit tests cover it meanwhile."
    )
)]
pub const DESKTOP_WINDOW_TITLE: &str = "universe-debug";

/// Desktop flight-window configuration as plain data.
///
/// Holds the scaffold size plus the visibility preset for the same-build
/// scaled-up desktop shell. Content never forks between presets or devices;
/// presets only switch visibility and size per `docs/tech/debug.md` 6.1.
/// The OS window (winit event loop plus wgpu surface) lands in a later step;
/// this config drives the headless-proven assembly meanwhile with no new
/// dependencies and no lockfile change.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DesktopWindowConfig {
    /// Window width in points.
    width_pt_f32: f32,
    /// Window height in points.
    height_pt_f32: f32,
    /// Visibility preset; shell state only and never persists.
    preset: DesktopPreset,
}

impl DesktopWindowConfig {
    /// Build the ticker-only default window configuration.
    ///
    /// Opens blind with the top bar alone for minimal occlusion; the tester
    /// switches to Descent for handoff watching.
    #[must_use]
    pub const fn ticker_only() -> Self {
        Self {
            width_pt_f32: DESKTOP_WINDOW_WIDTH_PT_F32,
            height_pt_f32: DESKTOP_WINDOW_HEIGHT_PT_F32,
            preset: DesktopPreset::TickerOnly,
        }
    }

    /// Build a window configuration from explicit values.
    ///
    /// Sizes are in points and must be finite and strictly positive.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutError`] when a size is non-finite or not positive.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Bin smoke wiring lands in a later step; unit tests cover it meanwhile."
        )
    )]
    pub fn new(
        width_pt_f32: f32,
        height_pt_f32: f32,
        preset: DesktopPreset,
    ) -> Result<Self, LayoutError> {
        for size_pt_f32 in [width_pt_f32, height_pt_f32] {
            if !size_pt_f32.is_finite() {
                return Err(LayoutError::NonFinite {
                    value_f64: f64::from(size_pt_f32),
                });
            }
            if size_pt_f32 <= 0.0 {
                return Err(LayoutError::NonPositive {
                    value_f64: f64::from(size_pt_f32),
                });
            }
        }
        Ok(Self {
            width_pt_f32,
            height_pt_f32,
            preset,
        })
    }

    /// Return the window width in points.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Bin smoke wiring lands in a later step; unit tests cover it meanwhile."
        )
    )]
    #[must_use]
    pub const fn width_pt_f32(self) -> f32 {
        self.width_pt_f32
    }

    /// Return the window height in points.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Bin smoke wiring lands in a later step; unit tests cover it meanwhile."
        )
    )]
    #[must_use]
    pub const fn height_pt_f32(self) -> f32 {
        self.height_pt_f32
    }

    /// Return the visibility preset.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Bin smoke wiring lands in a later step; unit tests cover it meanwhile."
        )
    )]
    #[must_use]
    pub const fn preset(self) -> DesktopPreset {
        self.preset
    }

    /// Select another preset without touching content.
    pub const fn set_preset(&mut self, preset: DesktopPreset) {
        self.preset = preset;
    }
}

/// Phone bottom-sheet tab with one visible at a time.
///
/// Same panels as desktop in a different arrangement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhoneTab {
    /// Run control tab.
    Run,
    /// View and overlay tab.
    View,
    /// Inspector tab.
    Inspect,
    /// Plots tab; off-screen plots skip draw but keep recording.
    Plots,
    /// Budget strip tab.
    Budget,
    /// Tracing log tab.
    Log,
    /// Replay tab.
    Replay,
}

impl PhoneTab {
    /// All phone tabs in sheet order.
    pub const ALL: [Self; 7] = [
        Self::Run,
        Self::View,
        Self::Inspect,
        Self::Plots,
        Self::Budget,
        Self::Log,
        Self::Replay,
    ];

    /// Return the short tab label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Run => "run",
            Self::View => "view",
            Self::Inspect => "inspect",
            Self::Plots => "plots",
            Self::Budget => "budget",
            Self::Log => "log",
            Self::Replay => "replay",
        }
    }

    /// Report whether the tab ships in Phase B.
    ///
    /// True for plots, budget, and log only. Run, view, inspect, and
    /// replay stay outside the Phase B bottom slice.
    #[must_use]
    pub const fn is_phase_b(self) -> bool {
        match self {
            Self::Plots | Self::Budget | Self::Log => true,
            Self::Run | Self::View | Self::Inspect | Self::Replay => false,
        }
    }

    /// Report whether the tab uses the single-plot optimization.
    ///
    /// True for plots only; off-screen plots skip draw but keep recording
    /// into pre-sized buffers per `docs/tech/debug.md` section 6.2.
    #[must_use]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Bin smoke wiring lands in a later step; unit tests cover it meanwhile."
        )
    )]
    pub const fn is_single_plot_optimization(self) -> bool {
        match self {
            Self::Plots => true,
            Self::Run | Self::View | Self::Inspect | Self::Budget | Self::Log | Self::Replay => {
                false
            }
        }
    }
}

/// Report whether a candidate plot draws for the selection.
///
/// True only when the candidate is the selected tab and targets the
/// single-plot optimization; off-screen plots skip draw per 6.2.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Bin smoke wiring lands in a later step; unit tests cover it meanwhile."
    )
)]
#[must_use]
pub const fn should_draw_plot_bool(selected_tab: PhoneTab, candidate_tab: PhoneTab) -> bool {
    match selected_tab {
        PhoneTab::Plots => matches!(candidate_tab, PhoneTab::Plots),
        PhoneTab::Run
        | PhoneTab::View
        | PhoneTab::Inspect
        | PhoneTab::Budget
        | PhoneTab::Log
        | PhoneTab::Replay => false,
    }
}

/// Report whether a candidate plot keeps recording.
///
/// Always true: off-screen plots skip draw but keep recording into
/// pre-sized buffers so continuity data is not lost per 6.2.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Bin smoke wiring lands in a later step; unit tests cover it meanwhile."
    )
)]
#[must_use]
pub const fn should_record_plot_bool(_selected_tab: PhoneTab, _candidate_tab: PhoneTab) -> bool {
    true
}

/// Bottom-sheet height detent.
///
/// Phone renders at most one plot tab at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottomSheetDetent {
    /// Half-height sheet detent.
    Half,
    /// Full-height sheet detent.
    Full,
}

impl BottomSheetDetent {
    /// All sheet detents.
    pub const ALL: [Self; 2] = [Self::Half, Self::Full];

    /// Return sheet height as a screen fraction.
    #[must_use]
    pub const fn height_fraction_f64(self) -> f64 {
        match self {
            Self::Half => BOTTOM_SHEET_HALF_FRACTION_F64,
            Self::Full => BOTTOM_SHEET_FULL_FRACTION_F64,
        }
    }

    /// Return the short detent label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Half => "half",
            Self::Full => "full",
        }
    }
}

/// Chip-row action mirroring top-bar run control.
///
/// Horizontally scrolling row above the sheet for one-handed use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChipAction {
    /// Pause chip.
    Pause,
    /// Single-step tick chip.
    Step,
    /// Warp selector chip.
    Warp,
    /// Warp auto-drop reason chip.
    AutoDrop,
}

impl ChipAction {
    /// All chip actions in row order.
    pub const ALL: [Self; 4] = [Self::Pause, Self::Step, Self::Warp, Self::AutoDrop];

    /// Return the short chip label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Pause => "pause",
            Self::Step => "step",
            Self::Warp => "warp",
            Self::AutoDrop => "auto-drop",
        }
    }

    /// Return the mirrored top-bar run-control name.
    ///
    /// Order matches section 4.1 run control: pause, single-step tick,
    /// warp selector, warp auto-drop reason. Source `docs/tech/debug.md`.
    #[must_use]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Bin smoke wiring lands in a later step; unit tests cover it meanwhile."
        )
    )]
    pub const fn top_bar_control_label(self) -> &'static str {
        match self {
            Self::Pause => "pause",
            Self::Step => "single-step tick",
            Self::Warp => "warp selector",
            Self::AutoDrop => "warp auto-drop reason",
        }
    }
}

/// Scale the 44pt touch target to pixels.
///
/// Multiplies [`MIN_TOUCH_TARGET_PT_F32`] by `pixels_per_point_f32`.
///
/// # Errors
///
/// Returns [`LayoutError`] when `pixels_per_point_f32` is non-finite or not positive.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Bin smoke wiring lands in a later step; unit tests cover it meanwhile."
    )
)]
pub fn scaled_touch_target_px_f32(pixels_per_point_f32: f32) -> Result<f32, LayoutError> {
    check_scale_positive_f32(pixels_per_point_f32)?;
    Ok(MIN_TOUCH_TARGET_PT_F32 * pixels_per_point_f32)
}

/// Scale the 96pt plot floor to pixels.
///
/// Multiplies [`PLOT_MIN_HEIGHT_PT_F32`] by `pixels_per_point_f32`.
///
/// # Errors
///
/// Returns [`LayoutError`] when `pixels_per_point_f32` is non-finite or not positive.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Bin smoke wiring lands in a later step; unit tests cover it meanwhile."
    )
)]
pub fn scaled_plot_min_height_px_f32(pixels_per_point_f32: f32) -> Result<f32, LayoutError> {
    check_scale_positive_f32(pixels_per_point_f32)?;
    Ok(PLOT_MIN_HEIGHT_PT_F32 * pixels_per_point_f32)
}

/// Scale the dev-tag reserve to pixels.
///
/// Multiplies [`DEV_TAG_RESERVE_PT_F32`] by `pixels_per_point_f64`.
///
/// # Errors
///
/// Returns [`LayoutError`] when `pixels_per_point_f64` is non-finite or not positive.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Bin smoke wiring lands in a later step; unit tests cover it meanwhile."
    )
)]
pub fn dev_tag_reserve_px_f64(pixels_per_point_f64: f64) -> Result<f64, LayoutError> {
    check_positive_f64(pixels_per_point_f64)?;
    Ok(f64::from(DEV_TAG_RESERVE_PT_F32) * pixels_per_point_f64)
}

/// Return full-detent height in pixels reserving the tag.
///
/// Subtracts the scaled [`DEV_TAG_RESERVE_PT_F32`] from `screen_height_px_f64`.
///
/// # Errors
///
/// Returns [`LayoutError`] for a bad screen or scale, or when the screen
/// leaves no positive height after the reserve.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Bin smoke wiring lands in a later step; unit tests cover it meanwhile."
    )
)]
pub fn full_height_px_f64(
    screen_height_px_f64: f64,
    pixels_per_point_f64: f64,
) -> Result<f64, LayoutError> {
    check_positive_f64(screen_height_px_f64)?;
    let reserve_px_f64 = dev_tag_reserve_px_f64(pixels_per_point_f64)?;
    let full_px_f64 = screen_height_px_f64 - reserve_px_f64;
    if full_px_f64 <= 0.0 {
        return Err(LayoutError::NonPositive {
            value_f64: full_px_f64,
        });
    }
    Ok(full_px_f64)
}

/// Return sheet height in pixels for a detent.
///
/// Half scales the screen by [`BOTTOM_SHEET_HALF_FRACTION_F64`]; full reserves
/// the dev tag through [`full_height_px_f64`].
///
/// # Errors
///
/// Returns [`LayoutError`] when `screen_height_px_f64` or `pixels_per_point_f64`
/// is non-finite or not positive.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Bin smoke wiring lands in a later step; unit tests cover it meanwhile."
    )
)]
pub fn sheet_height_px_f64(
    screen_height_px_f64: f64,
    detent: BottomSheetDetent,
    pixels_per_point_f64: f64,
) -> Result<f64, LayoutError> {
    check_positive_f64(screen_height_px_f64)?;
    check_positive_f64(pixels_per_point_f64)?;
    match detent {
        BottomSheetDetent::Half => Ok(screen_height_px_f64 * BOTTOM_SHEET_HALF_FRACTION_F64),
        BottomSheetDetent::Full => full_height_px_f64(screen_height_px_f64, pixels_per_point_f64),
    }
}

/// Report whether a sheet height covers the tag.
///
/// Compares `sheet_height_px_f64` plus `dev_tag_height_px_f64` against
/// `screen_height_px_f64`; equality only touches and returns false.
///
/// # Errors
///
/// Returns [`LayoutError`] for non-finite inputs, negative sheet or tag
/// heights, or a non-positive screen height.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Bin smoke wiring lands in a later step; unit tests cover it meanwhile."
    )
)]
pub fn sheet_covers_dev_tag_bool(
    sheet_height_px_f64: f64,
    screen_height_px_f64: f64,
    dev_tag_height_px_f64: f64,
) -> Result<bool, LayoutError> {
    if !sheet_height_px_f64.is_finite() {
        return Err(LayoutError::NonFinite {
            value_f64: sheet_height_px_f64,
        });
    }
    if sheet_height_px_f64 < 0.0 {
        return Err(LayoutError::Negative {
            value_f64: sheet_height_px_f64,
        });
    }
    check_positive_f64(screen_height_px_f64)?;
    if !dev_tag_height_px_f64.is_finite() {
        return Err(LayoutError::NonFinite {
            value_f64: dev_tag_height_px_f64,
        });
    }
    if dev_tag_height_px_f64 < 0.0 {
        return Err(LayoutError::Negative {
            value_f64: dev_tag_height_px_f64,
        });
    }
    Ok(sheet_height_px_f64 + dev_tag_height_px_f64 > screen_height_px_f64)
}

/// Report whether a control meets the 44pt floor.
///
/// Compares `control_px_f32` against the scaled [`MIN_TOUCH_TARGET_PT_F32`].
///
/// # Errors
///
/// Returns [`LayoutError`] for a non-finite or negative control size, or a
/// non-finite or non-positive scale.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Bin smoke wiring lands in a later step; unit tests cover it meanwhile."
    )
)]
pub fn meets_min_touch_target_bool(
    control_px_f32: f32,
    pixels_per_point_f32: f32,
) -> Result<bool, LayoutError> {
    if !control_px_f32.is_finite() {
        return Err(LayoutError::NonFinite {
            value_f64: f64::from(control_px_f32),
        });
    }
    if control_px_f32 < 0.0 {
        return Err(LayoutError::Negative {
            value_f64: f64::from(control_px_f32),
        });
    }
    let target_px_f32 = scaled_touch_target_px_f32(pixels_per_point_f32)?;
    Ok(control_px_f32 >= target_px_f32)
}

/// Report whether a plot meets the 96pt floor.
///
/// Compares `plot_height_px_f32` against the scaled [`PLOT_MIN_HEIGHT_PT_F32`].
///
/// # Errors
///
/// Returns [`LayoutError`] for a non-finite or negative plot height, or a
/// non-finite or non-positive scale.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Bin smoke wiring lands in a later step; unit tests cover it meanwhile."
    )
)]
pub fn meets_plot_min_height_bool(
    plot_height_px_f32: f32,
    pixels_per_point_f32: f32,
) -> Result<bool, LayoutError> {
    if !plot_height_px_f32.is_finite() {
        return Err(LayoutError::NonFinite {
            value_f64: f64::from(plot_height_px_f32),
        });
    }
    if plot_height_px_f32 < 0.0 {
        return Err(LayoutError::Negative {
            value_f64: f64::from(plot_height_px_f32),
        });
    }
    let floor_px_f32 = scaled_plot_min_height_px_f32(pixels_per_point_f32)?;
    Ok(plot_height_px_f32 >= floor_px_f32)
}

/// Check a `pixels_per_point` scale in logical pixels per point.
///
/// Private range gate for the phone scaling helpers.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Bin smoke wiring lands in a later step; unit tests cover it meanwhile."
    )
)]
fn check_scale_positive_f32(pixels_per_point_f32: f32) -> Result<(), LayoutError> {
    if !pixels_per_point_f32.is_finite() {
        return Err(LayoutError::NonFinite {
            value_f64: f64::from(pixels_per_point_f32),
        });
    }
    if pixels_per_point_f32 <= 0.0 {
        return Err(LayoutError::NonPositive {
            value_f64: f64::from(pixels_per_point_f32),
        });
    }
    Ok(())
}

/// Check a strictly positive pixel or scale value.
///
/// Private range gate for the sheet-height helpers.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Bin smoke wiring lands in a later step; unit tests cover it meanwhile."
    )
)]
fn check_positive_f64(value_f64: f64) -> Result<(), LayoutError> {
    if !value_f64.is_finite() {
        return Err(LayoutError::NonFinite { value_f64 });
    }
    if value_f64 <= 0.0 {
        return Err(LayoutError::NonPositive { value_f64 });
    }
    Ok(())
}

/// Phone bottom-sheet selection with one visible tab.
///
/// Half and full detents only; full reserves the dev tag above.
/// One `selected_tab` field enforces one-at-a-time display.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Bin smoke wiring lands in a later step; unit tests cover it meanwhile."
    )
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhoneSheetState {
    /// Currently visible sheet tab.
    selected_tab: PhoneTab,
    /// Currently selected height detent.
    detent: BottomSheetDetent,
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Bin smoke wiring lands in a later step; unit tests cover it meanwhile."
    )
)]
impl PhoneSheetState {
    /// Build a sheet state from one tab and detent.
    #[must_use]
    pub const fn new(selected_tab: PhoneTab, detent: BottomSheetDetent) -> Self {
        Self {
            selected_tab,
            detent,
        }
    }

    /// Return the visible tab.
    #[must_use]
    pub const fn selected_tab(self) -> PhoneTab {
        self.selected_tab
    }

    /// Return the height detent.
    #[must_use]
    pub const fn detent(self) -> BottomSheetDetent {
        self.detent
    }

    /// Select the one visible tab.
    pub const fn select(&mut self, tab: PhoneTab) {
        self.selected_tab = tab;
    }

    /// Select the height detent.
    pub const fn set_detent(&mut self, detent: BottomSheetDetent) {
        self.detent = detent;
    }

    /// Return sheet height in pixels for the state detent.
    ///
    /// Delegates to [`sheet_height_px_f64`] with the selected detent.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutError`] when `screen_height_px_f64` or `pixels_per_point_f64`
    /// is non-finite or not positive.
    pub fn sheet_height_px_f64(
        self,
        screen_height_px_f64: f64,
        pixels_per_point_f64: f64,
    ) -> Result<f64, LayoutError> {
        sheet_height_px_f64(screen_height_px_f64, self.detent, pixels_per_point_f64)
    }

    /// Report whether full detent keeps the tag visible.
    ///
    /// Returns true when the reserved full height plus the scaled tag
    /// reserve stays within the screen height.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutError`] when `screen_height_px_f64` or `pixels_per_point_f64`
    /// is non-finite or not positive.
    pub fn full_keeps_tag_visible_bool(
        screen_height_px_f64: f64,
        pixels_per_point_f64: f64,
    ) -> Result<bool, LayoutError> {
        let full_px_f64 = full_height_px_f64(screen_height_px_f64, pixels_per_point_f64)?;
        let reserve_px_f64 = dev_tag_reserve_px_f64(pixels_per_point_f64)?;
        let covers_bool =
            sheet_covers_dev_tag_bool(full_px_f64, screen_height_px_f64, reserve_px_f64)?;
        Ok(!covers_bool)
    }
}

/// Shell input-routing mode; step 5 owns routing.
///
/// Desktop toggles with `F3` (`Escape` returns); phone uses long-press or three-finger tap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellInputMode {
    /// Game gets all input; shell shows badges only.
    Passthrough,
    /// Shell widgets get input first.
    Focused,
}

impl ShellInputMode {
    /// Toggle between passthrough and focused modes.
    #[must_use]
    pub const fn toggle(self) -> Self {
        match self {
            Self::Passthrough => Self::Focused,
            Self::Focused => Self::Passthrough,
        }
    }

    /// Return the short mode label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Passthrough => "passthrough",
            Self::Focused => "focused",
        }
    }
}

/// Persistent phone badge with frame fraction, tick, and warp.
///
/// Anchored top-right; long-press opens the shell. Only shell element visible in `Passthrough`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DevTag {
    /// Displayed tick count, dimensionless.
    tick_count_u64: u64,
    /// Displayed frame cost as a budget fraction.
    frame_fraction_ratio_f64: f64,
}

impl DevTag {
    /// Build a dev-tag snapshot from tick and frame fraction.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutError`] when `frame_fraction_ratio_f64` is non-finite or negative.
    pub fn new(tick_count_u64: u64, frame_fraction_ratio_f64: f64) -> Result<Self, LayoutError> {
        if !frame_fraction_ratio_f64.is_finite() {
            return Err(LayoutError::NonFinite {
                value_f64: frame_fraction_ratio_f64,
            });
        }
        if frame_fraction_ratio_f64 < 0.0 {
            return Err(LayoutError::Negative {
                value_f64: frame_fraction_ratio_f64,
            });
        }
        Ok(Self {
            tick_count_u64,
            frame_fraction_ratio_f64,
        })
    }

    /// Return the displayed tick count.
    #[must_use]
    pub const fn tick_count_u64(self) -> u64 {
        self.tick_count_u64
    }

    /// Return the displayed frame fraction.
    #[must_use]
    pub const fn frame_fraction_ratio_f64(self) -> f64 {
        self.frame_fraction_ratio_f64
    }

    /// Return the anchor edge name.
    #[must_use]
    pub const fn anchor() -> &'static str {
        "top-right"
    }
}

/// Report whether closing the shell keeps run control legible.
///
/// Always true: the dev tag stays anchored top-right with frame fraction,
/// tick, and warp, so ticker-only runs stay legible per 6.1 and 6.2.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Bin smoke wiring lands in a later step; unit tests cover it meanwhile."
    )
)]
#[must_use]
pub const fn shell_closed_preserves_run_control_bool() -> bool {
    true
}

/// Shell draw-cost sample; step 4 owns budget wiring.
///
/// Draw cost stays separate from sim and render cost; closing the shell removes it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellCostSample {
    /// Sampled shell draw cost in milliseconds.
    draw_ms_f64: f64,
}

impl ShellCostSample {
    /// Build a shell-cost sample from draw milliseconds.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutError`] when `draw_ms_f64` is non-finite or negative.
    pub fn new(draw_ms_f64: f64) -> Result<Self, LayoutError> {
        if !draw_ms_f64.is_finite() {
            return Err(LayoutError::NonFinite {
                value_f64: draw_ms_f64,
            });
        }
        if draw_ms_f64 < 0.0 {
            return Err(LayoutError::Negative {
                value_f64: draw_ms_f64,
            });
        }
        Ok(Self { draw_ms_f64 })
    }

    /// Return the sampled draw cost in milliseconds.
    #[must_use]
    pub const fn draw_ms_f64(self) -> f64 {
        self.draw_ms_f64
    }

    /// Return draw cost as a fraction of a budget.
    ///
    /// Pass `FRAME_BUDGET_MS` as `budget_ms_f64` in step 4; gates live in quality.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutError`] when `budget_ms_f64` is non-finite or not positive.
    pub fn fraction_of_budget(self, budget_ms_f64: f64) -> Result<f64, LayoutError> {
        if !budget_ms_f64.is_finite() {
            return Err(LayoutError::NonFinite {
                value_f64: budget_ms_f64,
            });
        }
        if budget_ms_f64 <= 0.0 {
            return Err(LayoutError::NonPositiveBudget {
                value_f64: budget_ms_f64,
            });
        }
        Ok(self.draw_ms_f64 / budget_ms_f64)
    }
}

/// Pre-sized shell buffer reservations for plot, log, and recorder.
///
/// Buffers allocate at shell open and reuse after warmup; shell state never persists.
#[expect(
    clippy::struct_field_names,
    reason = "unit suffixes are required by the naming rule"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferPlan {
    /// Plot-history reservation in entries.
    plot_history_entries_usize: usize,
    /// Log-history reservation in entries.
    log_history_entries_usize: usize,
    /// Input-recorder reservation in entries.
    input_recorder_entries_usize: usize,
    /// Hash-history reservation in entries.
    hash_history_entries_usize: usize,
}

impl BufferPlan {
    /// Build the shell-default reservation plan.
    #[must_use]
    pub const fn shell_default() -> Self {
        Self {
            plot_history_entries_usize: PLOT_HISTORY_CAPACITY_ENTRIES_USIZE,
            log_history_entries_usize: LOG_HISTORY_CAPACITY_ENTRIES_USIZE,
            input_recorder_entries_usize: INPUT_RECORDER_CAPACITY_ENTRIES_USIZE,
            hash_history_entries_usize: HASH_HISTORY_CAPACITY_ENTRIES_USIZE,
        }
    }

    /// Return the plot-history reservation in entries.
    #[must_use]
    pub const fn plot_history_entries_usize(self) -> usize {
        self.plot_history_entries_usize
    }

    /// Return the log-history reservation in entries.
    #[must_use]
    pub const fn log_history_entries_usize(self) -> usize {
        self.log_history_entries_usize
    }

    /// Return the input-recorder reservation in entries.
    #[must_use]
    pub const fn input_recorder_entries_usize(self) -> usize {
        self.input_recorder_entries_usize
    }

    /// Return the hash-history reservation in entries.
    #[must_use]
    pub const fn hash_history_entries_usize(self) -> usize {
        self.hash_history_entries_usize
    }
}

/// Return the `egui-wgpu` renderer type name for the shell bridge.
///
/// Step 3 lock proof only; the renderer is created in Step 4-5.
/// Available only with the non-default `dev-shell` feature.
#[cfg(feature = "dev-shell")]
#[must_use]
pub fn egui_wgpu_renderer_type_name() -> &'static str {
    core::any::type_name::<egui_wgpu::Renderer>()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMOKE_TICK_COUNT_U64: u64 = 7;
    const SMOKE_FRACTION_F64: f64 = 0.2;
    const NEGATIVE_FRACTION_F64: f64 = -0.1;
    const SMOKE_DRAW_MS_F64: f64 = 0.4;
    const SMOKE_BUDGET_MS_F64: f64 = 2.0;
    const EXPECTED_HALF_OF_TWO_F64: f64 = 0.2;
    const FRACTION_TOL_F64: f64 = 1e-12;
    const SHEET_SCREEN_PX_F64: f64 = 800.0;
    const SHEET_PPP_F64: f64 = 2.0;
    const SHEET_PPP_F32: f32 = 2.0;
    const PX_TOL_F64: f64 = 1e-9;
    const PX_TOL_F32: f32 = 1e-6;

    #[test]
    fn descent_shows_left_and_bottom_ticker_hides_them() {
        let descent = DesktopPreset::Descent.visibility();
        assert!(descent.shows_top_bar());
        assert!(descent.shows_left_panel());
        assert!(descent.shows_bottom_tabs());
        let ticker = PanelVisibility::for_preset(DesktopPreset::TickerOnly);
        assert!(ticker.shows_top_bar());
        assert!(!ticker.shows_left_panel());
        assert!(!ticker.shows_right_panel());
        assert!(!ticker.shows_bottom_tabs());
        assert_eq!(DesktopPreset::ALL.len(), 4);
    }

    #[test]
    fn phone_tabs_cover_run_through_replay() {
        assert_eq!(PhoneTab::ALL.len(), 7);
        assert_eq!(PhoneTab::ALL[0], PhoneTab::Run);
        assert_eq!(PhoneTab::ALL[6], PhoneTab::Replay);
        assert_eq!(DockRegion::ALL.len(), 5);
        assert_eq!(ChipAction::ALL.len(), 4);
    }

    #[test]
    fn phase_b_tabs_and_preset_defaults() {
        assert!(PhoneTab::Plots.is_phase_b());
        assert!(PhoneTab::Budget.is_phase_b());
        assert!(PhoneTab::Log.is_phase_b());
        assert!(!PhoneTab::Run.is_phase_b());
        assert!(!PhoneTab::Replay.is_phase_b());
        assert_eq!(
            DesktopPreset::Descent.default_bottom_tab(),
            Some(PhoneTab::Plots)
        );
        assert_eq!(
            DesktopPreset::Budget.default_bottom_tab(),
            Some(PhoneTab::Budget)
        );
        assert_eq!(
            DesktopPreset::Determinism.default_bottom_tab(),
            Some(PhoneTab::Log)
        );
        assert_eq!(DesktopPreset::TickerOnly.default_bottom_tab(), None);
    }

    #[test]
    fn detents_match_named_fractions() {
        assert!(
            (BottomSheetDetent::Half.height_fraction_f64() - BOTTOM_SHEET_HALF_FRACTION_F64).abs()
                < FRACTION_TOL_F64
        );
        assert!(
            (BottomSheetDetent::Full.height_fraction_f64() - BOTTOM_SHEET_FULL_FRACTION_F64).abs()
                < FRACTION_TOL_F64
        );
    }

    #[test]
    fn dev_tag_rejects_bad_fractions() {
        let Ok(tag) = DevTag::new(SMOKE_TICK_COUNT_U64, SMOKE_FRACTION_F64) else {
            panic!("smoke dev tag must build")
        };
        assert_eq!(tag.tick_count_u64(), SMOKE_TICK_COUNT_U64);
        assert_eq!(DevTag::anchor(), "top-right");
        assert!(matches!(
            DevTag::new(SMOKE_TICK_COUNT_U64, f64::NAN),
            Err(LayoutError::NonFinite { .. })
        ));
        assert!(matches!(
            DevTag::new(SMOKE_TICK_COUNT_U64, NEGATIVE_FRACTION_F64),
            Err(LayoutError::Negative { .. })
        ));
    }

    #[test]
    fn shell_cost_fraction_divides_by_budget() {
        let Ok(sample) = ShellCostSample::new(SMOKE_DRAW_MS_F64) else {
            panic!("smoke cost sample must build")
        };
        let Ok(fraction_f64) = sample.fraction_of_budget(SMOKE_BUDGET_MS_F64) else {
            panic!("smoke fraction must divide")
        };
        assert!((fraction_f64 - EXPECTED_HALF_OF_TWO_F64).abs() < FRACTION_TOL_F64);
        assert!(matches!(
            sample.fraction_of_budget(0.0),
            Err(LayoutError::NonPositiveBudget { .. })
        ));
        assert!(matches!(
            ShellCostSample::new(f64::NAN),
            Err(LayoutError::NonFinite { .. })
        ));
    }

    #[test]
    fn buffer_plan_uses_named_capacities() {
        let plan = BufferPlan::shell_default();
        assert_eq!(
            plan.plot_history_entries_usize(),
            PLOT_HISTORY_CAPACITY_ENTRIES_USIZE
        );
        assert_eq!(
            plan.log_history_entries_usize(),
            LOG_HISTORY_CAPACITY_ENTRIES_USIZE
        );
        assert_eq!(
            plan.input_recorder_entries_usize(),
            INPUT_RECORDER_CAPACITY_ENTRIES_USIZE
        );
        assert_eq!(
            plan.hash_history_entries_usize(),
            HASH_HISTORY_CAPACITY_ENTRIES_USIZE
        );
    }

    #[test]
    fn input_mode_toggles_both_ways() {
        assert_eq!(
            ShellInputMode::Passthrough.toggle(),
            ShellInputMode::Focused
        );
        assert_eq!(
            ShellInputMode::Focused.toggle(),
            ShellInputMode::Passthrough
        );
    }

    #[test]
    fn full_detent_reserves_tag_and_holds_one_tab() {
        let mut sheet = PhoneSheetState::new(PhoneTab::Run, BottomSheetDetent::Half);
        assert_eq!(sheet.selected_tab(), PhoneTab::Run);
        assert_eq!(sheet.detent(), BottomSheetDetent::Half);
        sheet.select(PhoneTab::Plots);
        assert_eq!(sheet.selected_tab(), PhoneTab::Plots);
        sheet.set_detent(BottomSheetDetent::Full);
        assert_eq!(sheet.detent(), BottomSheetDetent::Full);
        let Ok(half_px_f64) =
            sheet_height_px_f64(SHEET_SCREEN_PX_F64, BottomSheetDetent::Half, SHEET_PPP_F64)
        else {
            panic!("half sheet height must compute")
        };
        assert!(
            (half_px_f64 - SHEET_SCREEN_PX_F64 * BOTTOM_SHEET_HALF_FRACTION_F64).abs() < PX_TOL_F64
        );
        let Ok(full_px_f64) =
            sheet_height_px_f64(SHEET_SCREEN_PX_F64, BottomSheetDetent::Full, SHEET_PPP_F64)
        else {
            panic!("full sheet height must compute")
        };
        let Ok(reserve_px_f64) = dev_tag_reserve_px_f64(SHEET_PPP_F64) else {
            panic!("tag reserve must compute")
        };
        assert!((full_px_f64 - (SHEET_SCREEN_PX_F64 - reserve_px_f64)).abs() < PX_TOL_F64);
        assert!(full_px_f64 < SHEET_SCREEN_PX_F64);
        assert!(half_px_f64 < full_px_f64);
        let Ok(covers_bool) =
            sheet_covers_dev_tag_bool(full_px_f64, SHEET_SCREEN_PX_F64, reserve_px_f64)
        else {
            panic!("cover check must compute")
        };
        assert!(!covers_bool);
        let Ok(keeps_bool) =
            PhoneSheetState::full_keeps_tag_visible_bool(SHEET_SCREEN_PX_F64, SHEET_PPP_F64)
        else {
            panic!("tag visibility must compute")
        };
        assert!(keeps_bool);
        let Ok(state_px_f64) = sheet.sheet_height_px_f64(SHEET_SCREEN_PX_F64, SHEET_PPP_F64) else {
            panic!("state sheet height must compute")
        };
        assert!((state_px_f64 - full_px_f64).abs() < PX_TOL_F64);
        for scale_f64 in [1.0_f64, 2.0, 3.0] {
            let Ok(keeps_scale_bool) =
                PhoneSheetState::full_keeps_tag_visible_bool(SHEET_SCREEN_PX_F64, scale_f64)
            else {
                panic!("tag visibility must compute for every scale")
            };
            assert!(keeps_scale_bool);
        }
        let Ok(floor_px_f32) = scaled_plot_min_height_px_f32(SHEET_PPP_F32) else {
            panic!("plot floor must scale")
        };
        assert!(half_px_f64 >= f64::from(floor_px_f32));
    }

    #[test]
    fn single_plot_optimization_targets_plots_only() {
        assert!(PhoneTab::Plots.is_single_plot_optimization());
        for tab in PhoneTab::ALL {
            if tab == PhoneTab::Plots {
                continue;
            }
            assert!(!tab.is_single_plot_optimization());
        }
        let count_usize = PhoneTab::ALL
            .iter()
            .filter(|tab| tab.is_single_plot_optimization())
            .count();
        assert_eq!(count_usize, 1);
    }

    #[test]
    fn chip_row_mirrors_top_bar_run_control() {
        assert_eq!(
            ChipAction::ALL,
            [
                ChipAction::Pause,
                ChipAction::Step,
                ChipAction::Warp,
                ChipAction::AutoDrop
            ]
        );
        assert_eq!(ChipAction::Pause.top_bar_control_label(), "pause");
        assert_eq!(ChipAction::Step.top_bar_control_label(), "single-step tick");
        assert_eq!(ChipAction::Warp.top_bar_control_label(), "warp selector");
        assert_eq!(
            ChipAction::AutoDrop.top_bar_control_label(),
            "warp auto-drop reason"
        );
    }

    #[test]
    fn touch_target_and_plot_floor_scale_with_ppp() {
        let Ok(touch_px_f32) = scaled_touch_target_px_f32(1.0) else {
            panic!("touch target must scale")
        };
        assert!((touch_px_f32 - MIN_TOUCH_TARGET_PT_F32).abs() < PX_TOL_F32);
        let Ok(touch_scaled_px_f32) = scaled_touch_target_px_f32(SHEET_PPP_F32) else {
            panic!("touch target must scale with sheet ppp")
        };
        assert!((touch_scaled_px_f32 - MIN_TOUCH_TARGET_PT_F32 * SHEET_PPP_F32).abs() < PX_TOL_F32);
        let Ok(floor_px_f32) = scaled_plot_min_height_px_f32(1.0) else {
            panic!("plot floor must scale")
        };
        assert!((floor_px_f32 - PLOT_MIN_HEIGHT_PT_F32).abs() < PX_TOL_F32);
        let Ok(floor_scaled_px_f32) = scaled_plot_min_height_px_f32(SHEET_PPP_F32) else {
            panic!("plot floor must scale with sheet ppp")
        };
        assert!((floor_scaled_px_f32 - PLOT_MIN_HEIGHT_PT_F32 * SHEET_PPP_F32).abs() < PX_TOL_F32);
        let Ok(meets_bool) = meets_min_touch_target_bool(MIN_TOUCH_TARGET_PT_F32, 1.0) else {
            panic!("touch check must compute")
        };
        assert!(meets_bool);
        let Ok(short_bool) = meets_min_touch_target_bool(MIN_TOUCH_TARGET_PT_F32 - 0.5, 1.0) else {
            panic!("short touch check must compute")
        };
        assert!(!short_bool);
        let Ok(plot_meets_bool) = meets_plot_min_height_bool(PLOT_MIN_HEIGHT_PT_F32, 1.0) else {
            panic!("plot check must compute")
        };
        assert!(plot_meets_bool);
        let Ok(plot_short_bool) = meets_plot_min_height_bool(PLOT_MIN_HEIGHT_PT_F32 - 0.5, 1.0)
        else {
            panic!("short plot check must compute")
        };
        assert!(!plot_short_bool);
    }

    #[test]
    fn phone_geometry_rejects_bad_inputs() {
        assert!(matches!(
            scaled_touch_target_px_f32(f32::NAN),
            Err(LayoutError::NonFinite { .. })
        ));
        assert!(matches!(
            scaled_touch_target_px_f32(0.0),
            Err(LayoutError::NonPositive { .. })
        ));
        assert!(matches!(
            scaled_touch_target_px_f32(-1.0),
            Err(LayoutError::NonPositive { .. })
        ));
        assert!(matches!(
            scaled_plot_min_height_px_f32(f32::NAN),
            Err(LayoutError::NonFinite { .. })
        ));
        assert!(matches!(
            scaled_plot_min_height_px_f32(0.0),
            Err(LayoutError::NonPositive { .. })
        ));
        assert!(matches!(
            sheet_height_px_f64(f64::NAN, BottomSheetDetent::Half, SHEET_PPP_F64),
            Err(LayoutError::NonFinite { .. })
        ));
        assert!(matches!(
            sheet_height_px_f64(0.0, BottomSheetDetent::Half, SHEET_PPP_F64),
            Err(LayoutError::NonPositive { .. })
        ));
        assert!(matches!(
            sheet_height_px_f64(SHEET_SCREEN_PX_F64, BottomSheetDetent::Half, f64::NAN),
            Err(LayoutError::NonFinite { .. })
        ));
        assert!(matches!(
            sheet_height_px_f64(SHEET_SCREEN_PX_F64, BottomSheetDetent::Full, 0.0),
            Err(LayoutError::NonPositive { .. })
        ));
        assert!(matches!(
            full_height_px_f64(10.0, SHEET_PPP_F64),
            Err(LayoutError::NonPositive { .. })
        ));
        assert!(matches!(
            meets_min_touch_target_bool(f32::NAN, 1.0),
            Err(LayoutError::NonFinite { .. })
        ));
        assert!(matches!(
            meets_min_touch_target_bool(-1.0, 1.0),
            Err(LayoutError::Negative { .. })
        ));
        assert!(matches!(
            meets_plot_min_height_bool(PLOT_MIN_HEIGHT_PT_F32, 0.0),
            Err(LayoutError::NonPositive { .. })
        ));
        assert!(matches!(
            sheet_covers_dev_tag_bool(-1.0, SHEET_SCREEN_PX_F64, 10.0),
            Err(LayoutError::Negative { .. })
        ));
        assert!(matches!(
            sheet_covers_dev_tag_bool(10.0, 0.0, 10.0),
            Err(LayoutError::NonPositive { .. })
        ));
    }

    #[cfg(feature = "dev-shell")]
    #[test]
    fn renderer_type_name_mentions_renderer() {
        assert!(super::egui_wgpu_renderer_type_name().contains("Renderer"));
    }

    #[test]
    fn preset_application_locks_visibility_and_defaults() {
        let descent = apply_desktop_preset(DesktopPreset::Descent);
        assert!(descent.visibility().shows_top_bar());
        assert!(descent.visibility().shows_left_panel());
        assert!(!descent.visibility().shows_right_panel());
        assert!(descent.visibility().shows_bottom_tabs());
        assert_eq!(descent.default_bottom_tab(), Some(PhoneTab::Plots));
        let ticker = apply_desktop_preset(DesktopPreset::TickerOnly);
        assert!(ticker.visibility().shows_top_bar());
        assert!(!ticker.visibility().shows_left_panel());
        assert!(!ticker.visibility().shows_right_panel());
        assert!(!ticker.visibility().shows_bottom_tabs());
        assert_eq!(ticker.default_bottom_tab(), None);
        let determinism = apply_desktop_preset(DesktopPreset::Determinism);
        assert!(determinism.visibility().shows_top_bar());
        assert!(!determinism.visibility().shows_left_panel());
        assert!(determinism.visibility().shows_bottom_tabs());
        assert_eq!(determinism.default_bottom_tab(), Some(PhoneTab::Log));
        assert_eq!(
            apply_desktop_preset(DesktopPreset::Budget).default_bottom_tab(),
            Some(PhoneTab::Budget)
        );
        for preset in DesktopPreset::ALL {
            let applied = apply_desktop_preset(preset);
            assert_eq!(applied.visibility(), preset.visibility());
            assert_eq!(applied.default_bottom_tab(), preset.default_bottom_tab());
        }
        let mut selection = PresetSelection::new(DesktopPreset::TickerOnly);
        assert_eq!(selection.preset(), DesktopPreset::TickerOnly);
        selection.select(DesktopPreset::Descent);
        assert_eq!(
            selection.apply(),
            apply_desktop_preset(DesktopPreset::Descent)
        );
    }

    #[test]
    fn preset_choice_never_persists() {
        const _: () = assert!(!PresetSelection::PERSISTED_BOOL);
        let selection = PresetSelection::new(DesktopPreset::Budget);
        assert_eq!(selection.preset(), DesktopPreset::Budget);
        assert_eq!(
            selection.apply().default_bottom_tab(),
            Some(PhoneTab::Budget)
        );
    }

    #[test]
    fn off_screen_plots_skip_draw_but_keep_recording() {
        assert!(should_draw_plot_bool(PhoneTab::Plots, PhoneTab::Plots));
        assert!(!should_draw_plot_bool(PhoneTab::Plots, PhoneTab::Budget));
        assert!(!should_draw_plot_bool(PhoneTab::Budget, PhoneTab::Plots));
        assert!(!should_draw_plot_bool(PhoneTab::Log, PhoneTab::Log));
        for selected_tab in PhoneTab::ALL {
            for candidate_tab in PhoneTab::ALL {
                assert!(should_record_plot_bool(selected_tab, candidate_tab));
                if should_draw_plot_bool(selected_tab, candidate_tab) {
                    assert_eq!(selected_tab, candidate_tab);
                    assert!(candidate_tab.is_single_plot_optimization());
                }
            }
        }
    }

    #[test]
    fn shell_closed_keeps_run_control_legible() {
        assert!(shell_closed_preserves_run_control_bool());
        assert_eq!(DevTag::anchor(), "top-right");
    }

    #[test]
    fn desktop_window_config_defaults_to_ticker_only() {
        let config = DesktopWindowConfig::ticker_only();
        assert!((config.width_pt_f32() - DESKTOP_WINDOW_WIDTH_PT_F32).abs() < PX_TOL_F32);
        assert!((config.height_pt_f32() - DESKTOP_WINDOW_HEIGHT_PT_F32).abs() < PX_TOL_F32);
        assert!(config.width_pt_f32() > 0.0);
        assert!(config.height_pt_f32() > 0.0);
        assert_eq!(config.preset(), DesktopPreset::TickerOnly);
        assert_eq!(DESKTOP_WINDOW_TITLE, "universe-debug");
    }

    #[test]
    fn desktop_window_config_checks_sizes_and_switches_preset() {
        assert!(matches!(
            DesktopWindowConfig::new(
                f32::NAN,
                DESKTOP_WINDOW_HEIGHT_PT_F32,
                DesktopPreset::TickerOnly
            ),
            Err(LayoutError::NonFinite { .. })
        ));
        assert!(matches!(
            DesktopWindowConfig::new(0.0, DESKTOP_WINDOW_HEIGHT_PT_F32, DesktopPreset::TickerOnly),
            Err(LayoutError::NonPositive { .. })
        ));
        assert!(matches!(
            DesktopWindowConfig::new(DESKTOP_WINDOW_WIDTH_PT_F32, -1.0, DesktopPreset::TickerOnly),
            Err(LayoutError::NonPositive { .. })
        ));
        let Ok(mut config) = DesktopWindowConfig::new(
            DESKTOP_WINDOW_WIDTH_PT_F32,
            DESKTOP_WINDOW_HEIGHT_PT_F32,
            DesktopPreset::TickerOnly,
        ) else {
            panic!("window config must build")
        };
        config.set_preset(DesktopPreset::Descent);
        assert_eq!(config.preset(), DesktopPreset::Descent);
    }
}
