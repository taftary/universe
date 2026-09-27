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
}

impl BufferPlan {
    /// Build the shell-default reservation plan.
    #[must_use]
    pub const fn shell_default() -> Self {
        Self {
            plot_history_entries_usize: PLOT_HISTORY_CAPACITY_ENTRIES_USIZE,
            log_history_entries_usize: LOG_HISTORY_CAPACITY_ENTRIES_USIZE,
            input_recorder_entries_usize: INPUT_RECORDER_CAPACITY_ENTRIES_USIZE,
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
}
