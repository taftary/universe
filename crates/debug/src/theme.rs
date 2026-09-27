//! DevDark-Pro base theme as plain data.
//!
//! Skeleton for issue 34 step 2 with no `egui` dependency. Step 3
//! maps these types onto `egui::Visuals` plus `egui::Style`. Budget
//! gates live in `docs/tech/quality.md`; this file cites budget names
//! such as `FRAME_BUDGET_MS` by name only and never restates values.

/// Eight-bit RGB color for theme swatches.
///
/// Channels are display values only, never physics inputs.
#[expect(
    clippy::struct_field_names,
    reason = "unit suffixes are required by the naming rule"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbU8 {
    /// Red channel intensity, `0` to `255`.
    pub red_u8: u8,
    /// Green channel intensity, `0` to `255`.
    pub green_u8: u8,
    /// Blue channel intensity, `0` to `255`.
    pub blue_u8: u8,
}

impl RgbU8 {
    /// Build a color from three channel intensities.
    #[must_use]
    pub const fn new(red_u8: u8, green_u8: u8, blue_u8: u8) -> Self {
        Self {
            red_u8,
            green_u8,
            blue_u8,
        }
    }

    /// Return channels as an array in RGB order.
    #[must_use]
    pub const fn to_array_u8(self) -> [u8; 3] {
        [self.red_u8, self.green_u8, self.blue_u8]
    }
}

/// Upper fraction bound for nominal cost display.
///
/// Source: `docs/tech/debug.md` section 7.
pub const BUDGET_NOMINAL_MAX_FRACTION_F64: f64 = 0.5;

/// Upper fraction bound for elevated cost display.
///
/// Source: `docs/tech/debug.md` section 7.
pub const BUDGET_ELEVATED_MAX_FRACTION_F64: f64 = 0.8;

/// Nominal budget color in 8-bit channels.
///
/// Source: `docs/tech/debug.md` section 7.
pub const BUDGET_NOMINAL_RGB_U8: RgbU8 = RgbU8::new(46_u8, 204_u8, 113_u8);

/// Elevated budget color in 8-bit channels.
///
/// Source: `docs/tech/debug.md` section 7.
pub const BUDGET_ELEVATED_RGB_U8: RgbU8 = RgbU8::new(240_u8, 178_u8, 32_u8);

/// Over-budget color in 8-bit channels.
///
/// Source: `docs/tech/debug.md` section 7.
pub const BUDGET_OVER_RGB_U8: RgbU8 = RgbU8::new(224_u8, 76_u8, 60_u8);

/// Sim-thread accent color in 8-bit channels.
///
/// Distinct from render and shell accents for at-a-glance sim state.
/// Source: `docs/tech/debug.md` section 7.
pub const SIM_ACCENT_RGB_U8: RgbU8 = RgbU8::new(53_u8, 196_u8, 255_u8);

/// Render-cost accent color in 8-bit channels.
///
/// Muted tone next to the sim accent.
/// Source: `docs/tech/debug.md` section 7.
pub const RENDER_ACCENT_RGB_U8: RgbU8 = RgbU8::new(148_u8, 163_u8, 184_u8);

/// Shell-cost accent color in 8-bit channels.
///
/// Muted tone next to the sim accent.
/// Source: `docs/tech/debug.md` section 7.
pub const SHELL_ACCENT_RGB_U8: RgbU8 = RgbU8::new(190_u8, 175_u8, 155_u8);

/// Theme background color in 8-bit channels.
///
/// Dark base with high-contrast text for outdoor legibility.
/// Source: `docs/tech/debug.md` section 7.
pub const BASE_BACKGROUND_RGB_U8: RgbU8 = RgbU8::new(13_u8, 17_u8, 23_u8);

/// Theme text color in 8-bit channels.
///
/// High-contrast text over the dark base.
/// Source: `docs/tech/debug.md` section 7.
pub const BASE_TEXT_RGB_U8: RgbU8 = RgbU8::new(237_u8, 242_u8, 247_u8);

/// Theme range-check failures.
///
/// Returned when a budget fraction is non-finite or negative.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThemeError {
    /// Fraction was non-finite and unclassifiable.
    NonFinite {
        /// Rejected fraction value.
        value_f64: f64,
    },
    /// Fraction was negative and below zero.
    NegativeFraction {
        /// Rejected fraction value.
        value_f64: f64,
    },
}

impl core::fmt::Display for ThemeError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NonFinite { value_f64 } => {
                write!(formatter, "non-finite budget fraction: {value_f64}")
            }
            Self::NegativeFraction { value_f64 } => {
                write!(formatter, "negative budget fraction: {value_f64}")
            }
        }
    }
}

impl std::error::Error for ThemeError {}

/// Budget-fraction band for cost readouts.
///
/// Bands follow `docs/tech/debug.md` section 7 thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetLevel {
    /// Below half of the named budget.
    Nominal,
    /// At least half but below four-fifths.
    Elevated,
    /// At least four-fifths of the named budget.
    Over,
}

impl BudgetLevel {
    /// Classify a cost fraction into a band.
    ///
    /// # Errors
    ///
    /// Returns [`ThemeError`] when `fraction_ratio_f64` is non-finite or negative.
    pub fn classify(fraction_ratio_f64: f64) -> Result<Self, ThemeError> {
        if !fraction_ratio_f64.is_finite() {
            return Err(ThemeError::NonFinite {
                value_f64: fraction_ratio_f64,
            });
        }
        if fraction_ratio_f64 < 0.0 {
            return Err(ThemeError::NegativeFraction {
                value_f64: fraction_ratio_f64,
            });
        }
        if fraction_ratio_f64 < BUDGET_NOMINAL_MAX_FRACTION_F64 {
            Ok(Self::Nominal)
        } else if fraction_ratio_f64 < BUDGET_ELEVATED_MAX_FRACTION_F64 {
            Ok(Self::Elevated)
        } else {
            Ok(Self::Over)
        }
    }

    /// Return the band color; numeric fraction always accompanies it.
    #[must_use]
    pub const fn color_rgb_u8(self) -> RgbU8 {
        match self {
            Self::Nominal => BUDGET_NOMINAL_RGB_U8,
            Self::Elevated => BUDGET_ELEVATED_RGB_U8,
            Self::Over => BUDGET_OVER_RGB_U8,
        }
    }

    /// Return the short band label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Nominal => "nominal",
            Self::Elevated => "elevated",
            Self::Over => "over",
        }
    }
}

/// Multiplier from unit fraction to percentage value.
const PERCENT_PER_FRACTION_F64: f64 = 100.0;

/// Classified budget fraction with its band.
///
/// Color never carries meaning alone; the numeric fraction always shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BudgetStatus {
    /// Cost as a fraction of its named budget.
    fraction_ratio_f64: f64,
    /// Band derived from the fraction.
    level: BudgetLevel,
}

impl BudgetStatus {
    /// Build a classified status from a cost fraction.
    ///
    /// # Errors
    ///
    /// Returns [`ThemeError`] when `fraction_ratio_f64` is non-finite or negative.
    pub fn new(fraction_ratio_f64: f64) -> Result<Self, ThemeError> {
        Ok(Self {
            fraction_ratio_f64,
            level: BudgetLevel::classify(fraction_ratio_f64)?,
        })
    }

    /// Return the stored cost fraction.
    #[must_use]
    pub const fn fraction_ratio_f64(self) -> f64 {
        self.fraction_ratio_f64
    }

    /// Return the classified band.
    #[must_use]
    pub const fn level(self) -> BudgetLevel {
        self.level
    }

    /// Return the band color for this status.
    #[must_use]
    pub const fn color_rgb_u8(self) -> RgbU8 {
        self.level.color_rgb_u8()
    }

    /// Return the fraction as a percentage value.
    #[must_use]
    pub const fn fraction_percent_f64(self) -> f64 {
        self.fraction_ratio_f64 * PERCENT_PER_FRACTION_F64
    }
}

/// Cost-domain accent for readouts.
///
/// Sim state uses one accent distinct from render and shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeAccent {
    /// Simulation thread accent for clocks and determinism badge.
    Sim,
    /// Render cost accent in a muted tone.
    Render,
    /// Shell cost accent in a muted tone.
    Shell,
}

impl ThemeAccent {
    /// All accents from sim through shell.
    pub const ALL: [Self; 3] = [Self::Sim, Self::Render, Self::Shell];

    /// Return the accent color.
    #[must_use]
    pub const fn color_rgb_u8(self) -> RgbU8 {
        match self {
            Self::Sim => SIM_ACCENT_RGB_U8,
            Self::Render => RENDER_ACCENT_RGB_U8,
            Self::Shell => SHELL_ACCENT_RGB_U8,
        }
    }

    /// Return the short accent label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Sim => "sim",
            Self::Render => "render",
            Self::Shell => "shell",
        }
    }
}

/// Font role for shell text.
///
/// Numbers use embedded monospace with tabular figures; labels use proportional.
/// Faces ship dev-only behind the `dev-shell` feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontRole {
    /// Numbers and hashes in monospace with tabular figures.
    Numbers,
    /// Labels in the proportional face.
    Labels,
}

impl FontRole {
    /// All font roles.
    pub const ALL: [Self; 2] = [Self::Numbers, Self::Labels];

    /// Report whether the role uses monospace.
    #[must_use]
    pub const fn monospace(self) -> bool {
        match self {
            Self::Numbers => true,
            Self::Labels => false,
        }
    }

    /// Report whether figures stay tabular without jitter.
    #[must_use]
    pub const fn tabular_numerals(self) -> bool {
        match self {
            Self::Numbers => true,
            Self::Labels => false,
        }
    }

    /// Return the short role label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Numbers => "numbers",
            Self::Labels => "labels",
        }
    }
}

/// DevDark-Pro base theme proposal as plain data.
///
/// Dark high-contrast base with thin strokes and no effects. Every
/// readout shows SI primary with raw secondary on tooltip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DevDarkProTheme {
    /// True when readouts lead with SI units.
    si_primary: bool,
    /// True when numerals use tabular figures.
    tabular_numerals: bool,
}

impl DevDarkProTheme {
    /// Build the step 2 proposal: SI primary with tabular numerals.
    #[must_use]
    pub const fn proposal() -> Self {
        Self {
            si_primary: true,
            tabular_numerals: true,
        }
    }

    /// Report whether readouts lead with SI units.
    #[must_use]
    pub const fn si_primary(self) -> bool {
        self.si_primary
    }

    /// Report whether numerals use tabular figures.
    #[must_use]
    pub const fn tabular_numerals(self) -> bool {
        self.tabular_numerals
    }

    /// Return the theme background color.
    #[must_use]
    pub const fn background_rgb_u8() -> RgbU8 {
        BASE_BACKGROUND_RGB_U8
    }

    /// Return the theme text color.
    #[must_use]
    pub const fn text_rgb_u8() -> RgbU8 {
        BASE_TEXT_RGB_U8
    }

    /// Return the accent color for a cost domain.
    #[must_use]
    pub const fn accent_rgb_u8(accent: ThemeAccent) -> RgbU8 {
        accent.color_rgb_u8()
    }
}

/// Build the Step 3 stub `egui` visuals for the dev shell.
///
/// Step 3 wiring only; the full DevDark-Pro mapping onto `egui::Visuals`
/// plus `egui::Style` lands in Step 4. Available only with the non-default
/// `dev-shell` feature.
#[cfg(feature = "dev-shell")]
#[must_use]
pub fn dev_dark_pro_visuals() -> egui::Visuals {
    egui::Visuals::dark()
}

#[cfg(test)]
mod tests {
    use super::*;

    const QUARTER_FRACTION_F64: f64 = 0.25;
    const ELEVATED_FRACTION_F64: f64 = 0.6;
    const OVER_FRACTION_F64: f64 = 0.9;
    const NEGATIVE_FRACTION_F64: f64 = -0.5;
    const EXPECTED_QUARTER_PERCENT_F64: f64 = 25.0;
    const PERCENT_TOL_F64: f64 = 1e-12;

    #[test]
    fn classify_bands_follow_thresholds() {
        let Ok(nominal) = BudgetLevel::classify(QUARTER_FRACTION_F64) else {
            panic!("quarter fraction must classify")
        };
        assert_eq!(nominal, BudgetLevel::Nominal);
        let Ok(elevated) = BudgetLevel::classify(ELEVATED_FRACTION_F64) else {
            panic!("elevated fraction must classify")
        };
        assert_eq!(elevated, BudgetLevel::Elevated);
        let Ok(over) = BudgetLevel::classify(OVER_FRACTION_F64) else {
            panic!("over fraction must classify")
        };
        assert_eq!(over, BudgetLevel::Over);
    }

    #[test]
    fn classify_rejects_bad_fractions() {
        assert!(matches!(
            BudgetLevel::classify(f64::NAN),
            Err(ThemeError::NonFinite { .. })
        ));
        assert!(matches!(
            BudgetLevel::classify(NEGATIVE_FRACTION_F64),
            Err(ThemeError::NegativeFraction { .. })
        ));
    }

    #[test]
    fn status_pairs_color_with_numeric_fraction() {
        let Ok(status) = BudgetStatus::new(QUARTER_FRACTION_F64) else {
            panic!("quarter fraction must build a status")
        };
        assert_eq!(status.level(), BudgetLevel::Nominal);
        assert_eq!(status.color_rgb_u8(), BUDGET_NOMINAL_RGB_U8);
        assert!(
            (status.fraction_percent_f64() - EXPECTED_QUARTER_PERCENT_F64).abs() < PERCENT_TOL_F64
        );
    }

    #[test]
    fn accents_stay_distinct_across_domains() {
        assert_ne!(
            ThemeAccent::Sim.color_rgb_u8(),
            ThemeAccent::Render.color_rgb_u8()
        );
        assert_ne!(
            ThemeAccent::Sim.color_rgb_u8(),
            ThemeAccent::Shell.color_rgb_u8()
        );
        assert_eq!(ThemeAccent::ALL.len(), 3);
    }

    #[test]
    fn numbers_use_monospace_tabular_labels_do_not() {
        assert!(FontRole::Numbers.monospace());
        assert!(FontRole::Numbers.tabular_numerals());
        assert!(!FontRole::Labels.monospace());
        assert!(!FontRole::Labels.tabular_numerals());
    }

    #[test]
    fn proposal_leads_with_si_and_tabular() {
        let proposal = DevDarkProTheme::proposal();
        assert!(proposal.si_primary());
        assert!(proposal.tabular_numerals());
        assert_eq!(DevDarkProTheme::background_rgb_u8(), BASE_BACKGROUND_RGB_U8);
        assert_eq!(DevDarkProTheme::text_rgb_u8(), BASE_TEXT_RGB_U8);
    }

    #[cfg(feature = "dev-shell")]
    #[test]
    fn stub_visuals_are_dark() {
        assert!(super::dev_dark_pro_visuals().dark_mode);
    }
}
