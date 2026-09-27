//! Bottom tabs with continuity, budget, and log views.
//!
//! One tab registry over the Phase B slice (plots, budget, log). Desktop
//! and phone share content and differ only in arrangement; at most one tab
//! draws at a time while history keeps recording. Tab selection is shell
//! state only and never persists.

use crate::layout::{DesktopPreset, PhoneTab};

#[cfg(feature = "dev-shell")]
use crate::budget::{BudgetDenominators, BudgetStrip};
#[cfg(feature = "dev-shell")]
use crate::console::Console;
#[cfg(feature = "dev-shell")]
use crate::continuity::ContinuityMonitor;
#[cfg(feature = "dev-shell")]
use crate::log::TraceLog;
#[cfg(feature = "dev-shell")]
use crate::tweak::TweakBoard;

/// Bottom tab backed by one registry.
///
/// Maps onto the `PhoneTab` plots, budget, and log slice plus the
/// console; replay and console stay deferred to later phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottomTab {
    /// Continuity plots with handoff markers.
    Continuity,
    /// Budget strip with fraction colors.
    Budget,
    /// Tracing log with filters.
    Log,
    /// Tweak board plus console over the registry.
    Console,
}

impl BottomTab {
    /// All bottom tabs in registry order.
    pub const ALL: [Self; 4] = [Self::Continuity, Self::Budget, Self::Log, Self::Console];

    /// Return the short tab label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Continuity => "plots",
            Self::Budget => "budget",
            Self::Log => "log",
            Self::Console => "console",
        }
    }

    /// Return the tab tooltip.
    ///
    /// Available only with the non-default `dev-shell` feature.
    #[cfg(feature = "dev-shell")]
    #[must_use]
    pub const fn tooltip(self) -> &'static str {
        match self {
            Self::Continuity => "readout curves with handoff before/after/delta markers",
            Self::Budget => "cost fractions of named budgets with bands",
            Self::Log => "bounded tracing ring with level and module filters",
            Self::Console => "tweak board plus command line over the registry",
        }
    }

    /// Map a phone tab onto the bottom registry.
    ///
    /// Returns none for tabs outside the Phase B slice. The phone
    /// console lives inside the Replay tab, so `Console` maps back to
    /// `Replay` instead.
    #[must_use]
    pub const fn from_phone_tab(tab: PhoneTab) -> Option<Self> {
        match tab {
            PhoneTab::Plots => Some(Self::Continuity),
            PhoneTab::Budget => Some(Self::Budget),
            PhoneTab::Log => Some(Self::Log),
            PhoneTab::Run | PhoneTab::View | PhoneTab::Inspect | PhoneTab::Replay => None,
        }
    }

    /// Map a bottom tab back onto its phone tab.
    #[must_use]
    pub const fn to_phone_tab(self) -> PhoneTab {
        match self {
            Self::Continuity => PhoneTab::Plots,
            Self::Budget => PhoneTab::Budget,
            Self::Log => PhoneTab::Log,
            Self::Console => PhoneTab::Replay,
        }
    }

    /// Return the default tab for a desktop preset.
    ///
    /// Returns none for ticker-only, which shows the top bar alone.
    #[must_use]
    pub fn default_for_preset(preset: DesktopPreset) -> Option<Self> {
        preset.default_bottom_tab().and_then(Self::from_phone_tab)
    }
}

/// Draw inputs borrowed for one bottom frame.
///
/// Available only with the non-default `dev-shell` feature.
#[cfg(feature = "dev-shell")]
pub struct BottomDraw<'a> {
    /// Continuity monitor with plot history.
    pub monitor: &'a ContinuityMonitor,
    /// Budget strip with latest samples.
    pub strip: &'a BudgetStrip,
    /// Named budget denominators for the bars.
    pub budgets: BudgetDenominators,
    /// Tracing log with filters.
    pub log: &'a TraceLog,
    /// Tracy connection flag supplied by the caller.
    pub tracy_connected_bool: bool,
    /// Tweak board with drafts plus confirm state.
    pub tweaks: &'a mut TweakBoard,
    /// Console with input plus history plus output.
    pub console: &'a mut Console,
    /// Scheduler hold state for pause-only gating.
    pub paused_bool: bool,
}

/// Bottom tab assembly with selection state.
///
/// Plain state only; draws the selected tab and keeps the rest recording.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BottomTabs {
    /// Currently drawn tab.
    selected: BottomTab,
}

impl BottomTabs {
    /// Build tabs with continuity selected.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            selected: BottomTab::Continuity,
        }
    }

    /// Return the selected tab.
    #[must_use]
    pub const fn selected(self) -> BottomTab {
        self.selected
    }

    /// Select a tab without touching sim state.
    pub const fn select(&mut self, tab: BottomTab) {
        self.selected = tab;
    }

    /// Draw the tab row plus the selected tab in one bottom pass.
    ///
    /// Immediate-mode widgets only; creates no renderer. At most one tab
    /// draws at a time on every device. Available only with the
    /// non-default `dev-shell` feature.
    #[cfg(feature = "dev-shell")]
    pub fn draw(&mut self, ui: &mut egui::Ui, content: BottomDraw<'_>) {
        ui.horizontal(|ui| {
            for tab in BottomTab::ALL {
                ui.selectable_value(&mut self.selected, tab, tab.label())
                    .on_hover_text(tab.tooltip());
            }
        });
        match self.selected {
            BottomTab::Continuity => content.monitor.draw(ui),
            BottomTab::Budget => content.strip.draw(ui, content.budgets),
            BottomTab::Log => content.log.draw(ui, content.tracy_connected_bool),
            BottomTab::Console => {
                let BottomDraw {
                    tweaks,
                    console,
                    paused_bool,
                    ..
                } = content;
                tweaks.draw(ui, paused_bool);
                console.draw(ui);
            }
        }
    }
}

impl Default for BottomTabs {
    /// Default tabs with continuity selected.
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_maps_phase_b_slice() {
        assert_eq!(BottomTab::ALL.len(), 4);
        assert_eq!(
            BottomTab::from_phone_tab(PhoneTab::Plots),
            Some(BottomTab::Continuity)
        );
        assert_eq!(
            BottomTab::from_phone_tab(PhoneTab::Budget),
            Some(BottomTab::Budget)
        );
        assert_eq!(
            BottomTab::from_phone_tab(PhoneTab::Log),
            Some(BottomTab::Log)
        );
        assert_eq!(BottomTab::from_phone_tab(PhoneTab::Replay), None);
        assert_eq!(BottomTab::from_phone_tab(PhoneTab::Run), None);
        assert_eq!(BottomTab::Continuity.to_phone_tab(), PhoneTab::Plots);
        assert!(BottomTab::Continuity.to_phone_tab().is_phase_b());
        assert_eq!(BottomTab::Console.to_phone_tab(), PhoneTab::Replay);
        assert_eq!(BottomTab::Console.label(), "console");
    }

    #[test]
    fn preset_defaults_follow_layout() {
        assert_eq!(
            BottomTab::default_for_preset(DesktopPreset::Descent),
            Some(BottomTab::Continuity)
        );
        assert_eq!(
            BottomTab::default_for_preset(DesktopPreset::Budget),
            Some(BottomTab::Budget)
        );
        assert_eq!(
            BottomTab::default_for_preset(DesktopPreset::TickerOnly),
            None
        );
    }

    #[test]
    fn selection_switches_tabs() {
        let mut tabs = BottomTabs::new();
        assert_eq!(tabs.selected(), BottomTab::Continuity);
        tabs.select(BottomTab::Log);
        assert_eq!(tabs.selected(), BottomTab::Log);
        assert_eq!(BottomTabs::default().selected(), BottomTab::Continuity);
    }
}
