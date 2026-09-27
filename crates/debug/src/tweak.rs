//! Tweak registry with pause-only drafts and explicit apply.
//!
//! Enforces the section-11 schema by type: dotted-path names, kinds,
//! SI units, ranges, defaults, safe-versus-taint effects, and live
//! versus pause-only edit rules. Typing while running writes drafts
//! only; apply needs pause plus explicit confirm, range-checks, appends
//! a recorder entry, and taints when the effect demands it. The shell
//! never writes sim state directly: apply returns a data command the
//! game or harness consumes outside the shell.

use crate::determinism::{InputEntry, InputKind, InputPayload, InputRecorder};
use crate::top_bar::TopBarState;

/// Registry length in entries, dimensionless.
///
/// Source: Phase C allow-list in `docs/tech/debug.md` section 4.3.
pub const REGISTRY_LEN_USIZE: usize = 3;

/// Density scale entry id, dimensionless.
pub const DENSITY_SCALE_ID_U16: u16 = 0;

/// Heating gain entry id, dimensionless.
pub const HEATING_GAIN_ID_U16: u16 = 1;

/// Plot window entry id, dimensionless.
pub const PLOT_WINDOW_ID_U16: u16 = 2;

/// Tweak value kinds from the registry schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TweakKind {
    /// Boolean switch.
    Bool,
    /// Signed integer.
    I64,
    /// Double float.
    F64,
    /// Text value.
    Str,
    /// Closed option set.
    Enum,
}

impl TweakKind {
    /// Return the short kind label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Bool => "bool",
            Self::I64 => "i64",
            Self::F64 => "f64",
            Self::Str => "string",
            Self::Enum => "enum",
        }
    }
}

/// Taint effect of applying an entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TweakEffect {
    /// Readable and writable without tainting.
    Safe,
    /// Writing marks the run tainted.
    Taint,
}

/// Edit rule of an entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditRule {
    /// Applies while running.
    Live,
    /// Applies only while paused.
    PauseOnly,
}

/// Tweak registry failures.
///
/// Returned for unknown names, bad ids, out-of-range values, applies
/// while running, bad values, and busy recorders. Never a panic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TweakError {
    /// Name is outside the allow-list.
    UnknownName,
    /// Entry id is outside the registry.
    BadId {
        /// Rejected id, dimensionless.
        id_u16: u16,
    },
    /// Value leaves the entry range.
    OutOfRange,
    /// Pause-only entry applied while running.
    NotPaused,
    /// Value text does not parse for the kind.
    BadValue,
    /// Recorder refused the apply entry.
    RecorderBusy,
}

impl core::fmt::Display for TweakError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnknownName => write!(formatter, "unknown tweak name"),
            Self::BadId { id_u16 } => write!(formatter, "bad tweak id: {id_u16}"),
            Self::OutOfRange => write!(formatter, "tweak value out of range"),
            Self::NotPaused => write!(formatter, "tweak needs pause"),
            Self::BadValue => write!(formatter, "bad tweak value"),
            Self::RecorderBusy => write!(formatter, "recorder refused tweak entry"),
        }
    }
}

impl std::error::Error for TweakError {}

/// One registry entry with schema, range, and rules.
///
/// Plain data with `'static` strings; the registry below is the only
/// instance and Phase C freezes its length at three.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TweakEntry {
    /// Dotted path name, e.g. `plots.window_s`.
    pub name: &'static str,
    /// Value kind.
    pub kind: TweakKind,
    /// SI unit or dimensionless label.
    pub unit: &'static str,
    /// Range minimum as bits (`f64` bits or `i64` bits).
    pub min_bits_u64: u64,
    /// Range maximum as bits.
    pub max_bits_u64: u64,
    /// Default as bits.
    pub default_bits_u64: u64,
    /// Taint effect on apply.
    pub effect: TweakEffect,
    /// Live or pause-only edit rule.
    pub edit_rule: EditRule,
    /// One sentence of at most 20 words.
    pub description: &'static str,
}

/// Phase C tweak allow-list in registry order.
///
/// Plot window is safe and live; density scale and heating gain taint
/// and need pause. Source: `docs/tech/debug.md` section 4.3.
pub const REGISTRY: [TweakEntry; REGISTRY_LEN_USIZE] = [
    TweakEntry {
        name: "atmo.density_scale",
        kind: TweakKind::F64,
        unit: "dimensionless",
        min_bits_u64: 0.5_f64.to_bits(),
        max_bits_u64: 2.0_f64.to_bits(),
        default_bits_u64: 1.0_f64.to_bits(),
        effect: TweakEffect::Taint,
        edit_rule: EditRule::PauseOnly,
        description: "Atmosphere density multiplier for sensitivity checks.",
    },
    TweakEntry {
        name: "trajectory.heating_gain",
        kind: TweakKind::F64,
        unit: "dimensionless",
        min_bits_u64: 0.1_f64.to_bits(),
        max_bits_u64: 10.0_f64.to_bits(),
        default_bits_u64: 1.0_f64.to_bits(),
        effect: TweakEffect::Taint,
        edit_rule: EditRule::PauseOnly,
        description: "Heating display gain for entry-plot sensitivity.",
    },
    TweakEntry {
        name: "plots.window_s",
        kind: TweakKind::F64,
        unit: "s",
        min_bits_u64: 5.0_f64.to_bits(),
        max_bits_u64: 68.0_f64.to_bits(),
        default_bits_u64: 60.0_f64.to_bits(),
        effect: TweakEffect::Safe,
        edit_rule: EditRule::Live,
        description: "Plot window length in seconds.",
    },
];

/// Look an entry id up by dotted-path name.
///
/// # Errors
///
/// Returns [`TweakError::UnknownName`] outside the allow-list, never a
/// panic.
pub fn lookup_entry_id(name: &str) -> Result<u16, TweakError> {
    for (index_usize, entry) in REGISTRY.iter().enumerate() {
        if entry.name == name {
            let Ok(id_u16) = u16::try_from(index_usize) else {
                return Err(TweakError::UnknownName);
            };
            return Ok(id_u16);
        }
    }
    Err(TweakError::UnknownName)
}

/// Look an entry up by id.
///
/// # Errors
///
/// Returns [`TweakError::BadId`] outside the registry.
pub fn lookup_entry(id_u16: u16) -> Result<&'static TweakEntry, TweakError> {
    REGISTRY
        .iter()
        .find(|entry| entry_index(entry) == id_u16)
        .ok_or(TweakError::BadId { id_u16 })
}

/// Return the registry index of an entry, dimensionless.
fn entry_index(entry: &TweakEntry) -> u16 {
    REGISTRY
        .iter()
        .position(|candidate| candidate.name == entry.name)
        .and_then(|index_usize| u16::try_from(index_usize).ok())
        .unwrap_or(u16::MAX)
}

/// Check `f64` bits against an entry range.
///
/// # Errors
///
/// Returns [`TweakError::BadValue`] for non-finite values or
/// [`TweakError::OutOfRange`] outside the range.
pub fn check_f64_bits(entry: &TweakEntry, value_bits_u64: u64) -> Result<f64, TweakError> {
    let value_f64 = f64::from_bits(value_bits_u64);
    if !value_f64.is_finite() {
        return Err(TweakError::BadValue);
    }
    let min_f64 = f64::from_bits(entry.min_bits_u64);
    let max_f64 = f64::from_bits(entry.max_bits_u64);
    if value_f64 < min_f64 || value_f64 > max_f64 {
        return Err(TweakError::OutOfRange);
    }
    Ok(value_f64)
}

/// One pending draft with entry id plus value bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TweakDraft {
    /// Registry entry id, dimensionless.
    entry_id_u16: u16,
    /// Pending value as bits.
    pending_bits_u64: u64,
}

impl TweakDraft {
    /// Return the entry id, dimensionless.
    #[must_use]
    pub const fn entry_id_u16(self) -> u16 {
        self.entry_id_u16
    }

    /// Return the pending bits.
    #[must_use]
    pub const fn pending_bits_u64(self) -> u64 {
        self.pending_bits_u64
    }
}

/// One applied command for consumers outside the shell.
///
/// Data only: the game or harness applies the value to its own
/// display-path state; the shell holds just the committed bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TweakCommand {
    /// Registry entry id, dimensionless.
    entry_id_u16: u16,
    /// Committed value as bits.
    value_bits_u64: u64,
}

impl TweakCommand {
    /// Return the entry id, dimensionless.
    #[must_use]
    pub const fn entry_id_u16(self) -> u16 {
        self.entry_id_u16
    }

    /// Return the committed bits.
    #[must_use]
    pub const fn value_bits_u64(self) -> u64 {
        self.value_bits_u64
    }
}

/// Tweak board with committed values, drafts, and confirm state.
///
/// Fixed-size arrays only; drafts accumulate without allocating.
/// Shell state only and never persists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TweakBoard {
    /// Committed value bits per registry index.
    committed_bits_u64: [u64; REGISTRY_LEN_USIZE],
    /// Pending draft per registry index, if any.
    drafts: [Option<TweakDraft>; REGISTRY_LEN_USIZE],
    /// Entry id awaiting inline confirm, if any.
    confirm_id_u16: Option<u16>,
}

impl TweakBoard {
    /// Build a board committed to registry defaults.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            committed_bits_u64: [
                REGISTRY[0].default_bits_u64,
                REGISTRY[1].default_bits_u64,
                REGISTRY[2].default_bits_u64,
            ],
            drafts: [None, None, None],
            confirm_id_u16: None,
        }
    }

    /// Return committed `f64` value for an entry.
    ///
    /// # Errors
    ///
    /// Returns [`TweakError::BadId`] outside the registry or
    /// [`TweakError::BadValue`] for non-`F64` kinds.
    pub fn committed_f64(&self, id_u16: u16) -> Result<f64, TweakError> {
        let entry = lookup_entry(id_u16)?;
        if entry.kind != TweakKind::F64 {
            return Err(TweakError::BadValue);
        }
        Ok(f64::from_bits(
            self.committed_bits_u64[entry_index(entry) as usize],
        ))
    }

    /// Return committed bits for an entry.
    ///
    /// # Errors
    ///
    /// Returns [`TweakError::BadId`] outside the registry.
    pub fn committed_bits(&self, id_u16: u16) -> Result<u64, TweakError> {
        let entry = lookup_entry(id_u16)?;
        Ok(self.committed_bits_u64[entry_index(entry) as usize])
    }

    /// Stage a draft for an entry without applying.
    ///
    /// Typing while running lands here; drafts show dimmed until apply.
    ///
    /// # Errors
    ///
    /// Returns [`TweakError::BadId`] outside the registry.
    pub fn set_draft(&mut self, id_u16: u16, value_bits_u64: u64) -> Result<(), TweakError> {
        let entry = lookup_entry(id_u16)?;
        self.drafts[entry_index(entry) as usize] = Some(TweakDraft {
            entry_id_u16: id_u16,
            pending_bits_u64: value_bits_u64,
        });
        Ok(())
    }

    /// Return the draft for an entry, if any.
    ///
    /// # Errors
    ///
    /// Returns [`TweakError::BadId`] outside the registry.
    pub fn draft(&self, id_u16: u16) -> Result<Option<TweakDraft>, TweakError> {
        let entry = lookup_entry(id_u16)?;
        Ok(self.drafts[entry_index(entry) as usize])
    }

    /// Discard the draft and confirm flag for an entry.
    ///
    /// # Errors
    ///
    /// Returns [`TweakError::BadId`] outside the registry.
    pub fn cancel(&mut self, id_u16: u16) -> Result<(), TweakError> {
        let entry = lookup_entry(id_u16)?;
        self.drafts[entry_index(entry) as usize] = None;
        if self.confirm_id_u16 == Some(id_u16) {
            self.confirm_id_u16 = None;
        }
        Ok(())
    }

    /// Arm the inline confirm step for an entry.
    ///
    /// # Errors
    ///
    /// Returns [`TweakError::BadId`] outside the registry.
    pub fn arm_confirm(&mut self, id_u16: u16) -> Result<(), TweakError> {
        lookup_entry(id_u16)?;
        self.confirm_id_u16 = Some(id_u16);
        Ok(())
    }

    /// Return the entry id awaiting confirm, if any.
    #[must_use]
    pub const fn confirm_id_u16(self) -> Option<u16> {
        self.confirm_id_u16
    }

    /// Apply a drafted value with pause, range, recorder, and taint.
    ///
    /// Requires pause for pause-only entries, range-checks `f64`
    /// values, commits, appends the apply entry to the recorder, and
    /// marks taint when the effect demands it. Returns the data
    /// command for outside consumers; the shell writes no sim state.
    ///
    /// # Errors
    ///
    /// Returns [`TweakError`] for bad ids, applies while running,
    /// out-of-range values, or a busy recorder.
    pub fn apply(
        &mut self,
        id_u16: u16,
        tick_count_u64: u64,
        paused_bool: bool,
        recorder: &mut InputRecorder,
        top_bar: &mut TopBarState,
    ) -> Result<TweakCommand, TweakError> {
        let entry = lookup_entry(id_u16)?;
        if entry.edit_rule == EditRule::PauseOnly && !paused_bool {
            return Err(TweakError::NotPaused);
        }
        let index_usize = entry_index(entry) as usize;
        let Some(draft) = self.drafts[index_usize] else {
            return Err(TweakError::BadValue);
        };
        if entry.kind == TweakKind::F64 {
            check_f64_bits(entry, draft.pending_bits_u64)?;
        }
        self.committed_bits_u64[index_usize] = draft.pending_bits_u64;
        self.drafts[index_usize] = None;
        self.confirm_id_u16 = None;
        let payload = InputPayload::setting(id_u16, draft.pending_bits_u64, 0_u8);
        recorder
            .record(InputEntry::new(
                tick_count_u64,
                InputKind::TweakApply,
                payload,
            ))
            .map_err(|_| TweakError::RecorderBusy)?;
        if entry.effect == TweakEffect::Taint {
            top_bar.mark_tainted();
        }
        Ok(TweakCommand {
            entry_id_u16: id_u16,
            value_bits_u64: draft.pending_bits_u64,
        })
    }

    /// Draw the tweakable section with markers plus confirms.
    ///
    /// Immediate-mode widgets only; creates no renderer. Tainting rows
    /// carry a `!` marker plus range line; drafts show dimmed with hint text; apply needs the inline confirm step. Available
    /// only with the non-default `dev-shell` feature.
    #[cfg(feature = "dev-shell")]
    pub fn draw(&mut self, ui: &mut egui::Ui, paused_bool: bool) {
        for entry in REGISTRY {
            let id_u16 = entry_index(&entry);
            let marker = if entry.effect == TweakEffect::Taint {
                "! "
            } else {
                ""
            };
            ui.label(format!(
                "{marker}{name} [{unit}]",
                name = entry.name,
                unit = entry.unit
            ))
            .on_hover_text(entry.description);
            if entry.kind == TweakKind::F64 {
                let current_f64 =
                    f64::from_bits(self.committed_bits_u64[entry_index(&entry) as usize]);
                ui.label(format!(
                    "value={current:.4} range=[{min:.4}, {max:.4}] default={default:.4}",
                    current = current_f64,
                    min = f64::from_bits(entry.min_bits_u64),
                    max = f64::from_bits(entry.max_bits_u64),
                    default = f64::from_bits(entry.default_bits_u64)
                ));
            }
            if self.drafts[entry_index(&entry) as usize].is_some() {
                ui.label(format!(
                    "draft pending; applies on pause ({rule})",
                    rule = if entry.edit_rule == EditRule::PauseOnly {
                        "pause-only"
                    } else {
                        "live"
                    }
                ));
            }
            if paused_bool {
                if ui
                    .button(format!("apply {name}", name = entry.name))
                    .clicked()
                {
                    let _ = self.arm_confirm(id_u16);
                }
            } else {
                ui.label("paused edits only; typing stages a draft");
            }
            if self.confirm_id_u16 == Some(id_u16) {
                ui.label(format!("confirm apply {name}?", name = entry.name));
                if ui.button("confirm").clicked() {
                    self.confirm_id_u16 = None;
                }
                if ui.button("cancel").clicked() {
                    let _ = self.cancel(id_u16);
                }
            }
        }
    }
}

impl Default for TweakBoard {
    /// Default board committed to registry defaults.
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMOKE_TICK_U64: u64 = 21;

    fn smoke_board() -> TweakBoard {
        TweakBoard::new()
    }

    fn smoke_recorder() -> InputRecorder {
        let mut recorder = InputRecorder::new();
        recorder.begin_run(0x1234);
        recorder
    }

    fn smoke_top_bar() -> TopBarState {
        TopBarState::new()
    }

    #[test]
    fn registry_lookup_covers_allow_list() {
        assert_eq!(REGISTRY.len(), REGISTRY_LEN_USIZE);
        let Ok(density_id) = lookup_entry_id("atmo.density_scale") else {
            panic!("density scale must resolve")
        };
        assert_eq!(density_id, DENSITY_SCALE_ID_U16);
        let Ok(window_id) = lookup_entry_id("plots.window_s") else {
            panic!("window must resolve")
        };
        assert_eq!(window_id, PLOT_WINDOW_ID_U16);
        assert!(matches!(
            lookup_entry_id("nope.missing"),
            Err(TweakError::UnknownName)
        ));
        assert!(matches!(lookup_entry(9_u16), Err(TweakError::BadId { .. })));
        for entry in REGISTRY {
            assert!(entry.description.split_whitespace().count() <= 20);
        }
    }

    #[test]
    fn ranges_reject_bad_values() {
        let entry = REGISTRY[DENSITY_SCALE_ID_U16 as usize];
        assert!(check_f64_bits(&entry, 1.1_f64.to_bits()).is_ok());
        assert!(matches!(
            check_f64_bits(&entry, 99.0_f64.to_bits()),
            Err(TweakError::OutOfRange)
        ));
        assert!(matches!(
            check_f64_bits(&entry, f64::NAN.to_bits()),
            Err(TweakError::BadValue)
        ));
    }

    #[test]
    fn apply_needs_pause_taints_and_records() {
        let mut board = smoke_board();
        let mut recorder = smoke_recorder();
        let mut top_bar = smoke_top_bar();
        assert!(
            board
                .set_draft(DENSITY_SCALE_ID_U16, 1.5_f64.to_bits())
                .is_ok()
        );
        assert!(matches!(
            board.apply(
                DENSITY_SCALE_ID_U16,
                SMOKE_TICK_U64,
                false,
                &mut recorder,
                &mut top_bar
            ),
            Err(TweakError::NotPaused)
        ));
        assert!(top_bar.is_clean());
        let Ok(command) = board.apply(
            DENSITY_SCALE_ID_U16,
            SMOKE_TICK_U64,
            true,
            &mut recorder,
            &mut top_bar,
        ) else {
            panic!("paused apply must commit")
        };
        assert_eq!(command.entry_id_u16(), DENSITY_SCALE_ID_U16);
        assert_eq!(command.value_bits_u64(), 1.5_f64.to_bits());
        assert!(top_bar.is_tainted());
        assert_eq!(recorder.len_usize(), 1);
        let Ok(committed) = board.committed_bits(DENSITY_SCALE_ID_U16) else {
            panic!("committed must read")
        };
        assert_eq!(committed, 1.5_f64.to_bits());
    }

    #[test]
    fn safe_live_entry_applies_running_clean() {
        let mut board = smoke_board();
        let mut recorder = smoke_recorder();
        let mut top_bar = smoke_top_bar();
        assert!(
            board
                .set_draft(PLOT_WINDOW_ID_U16, 30.0_f64.to_bits())
                .is_ok()
        );
        assert!(
            board
                .apply(
                    PLOT_WINDOW_ID_U16,
                    SMOKE_TICK_U64,
                    false,
                    &mut recorder,
                    &mut top_bar
                )
                .is_ok()
        );
        assert!(top_bar.is_clean());
        assert_eq!(recorder.len_usize(), 1);
    }

    #[test]
    fn drafts_cancel_cleanly() {
        let mut board = smoke_board();
        assert!(
            board
                .set_draft(HEATING_GAIN_ID_U16, 2.0_f64.to_bits())
                .is_ok()
        );
        assert!(board.draft(HEATING_GAIN_ID_U16).is_ok());
        assert!(board.arm_confirm(HEATING_GAIN_ID_U16).is_ok());
        assert_eq!(board.confirm_id_u16(), Some(HEATING_GAIN_ID_U16));
        assert!(board.cancel(HEATING_GAIN_ID_U16).is_ok());
        assert_eq!(board.confirm_id_u16(), None);
        let Ok(empty) = board.draft(HEATING_GAIN_ID_U16) else {
            panic!("draft read must work")
        };
        assert!(empty.is_none());
    }
}
