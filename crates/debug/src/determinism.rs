//! Determinism panel: seed tree, per-tick hash ring, input recorder.
//!
//! Read-only views over snapshot copies plus an append-only recorder and
//! a headless replay comparator. The shell never writes sim state; the
//! replay harness below drives `engine` stepping in tests and in the
//! headless replay action only.

#[cfg(feature = "dev-shell")]
use engine::inspect::snapshot_hash;

use crate::layout::{HASH_HISTORY_CAPACITY_ENTRIES_USIZE, INPUT_RECORDER_CAPACITY_ENTRIES_USIZE};

/// Input payload size in bytes, fixed.
///
/// Source: issue #38 Step 2 design; 1024 entries fit ~42 KB.
pub const INPUT_PAYLOAD_CAP_BYTES_USIZE: usize = 32;

/// Low-16-bit mask for short hash and seed display, dimensionless.
///
/// Same truncation rule as `TopBarState::hash_short_u16`.
/// Source: `crates/debug/src/top_bar.rs` `SHORT_DISPLAY_MASK_U64`.
pub const SHORT_DISPLAY_MASK_U64: u64 = 0xFFFF;

/// Hex digits per payload byte in CSV form, dimensionless.
const HEX_DIGITS_PER_BYTE_USIZE: usize = 2;

/// Payload hex buffer length in bytes for 32 payload bytes.
const PAYLOAD_HEX_CAP_BYTES_USIZE: usize =
    INPUT_PAYLOAD_CAP_BYTES_USIZE * HEX_DIGITS_PER_BYTE_USIZE;

/// Lowercase hex alphabet for payload encoding.
const HEX_ALPHABET_U8: [u8; 16] = *b"0123456789abcdef";

/// Determinism range-check and replay failures.
///
/// Returned for unknown input kinds, bad warp codes, and full recorders.
/// Hash mismatches surface as `ReplayReport` status, never as errors.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeterminismError {
    /// Input kind code was outside `0` to `5`.
    UnknownKind {
        /// Rejected code, dimensionless.
        code_u8: u8,
    },
    /// Warp code was outside `0` to `4`.
    BadWarpCode {
        /// Rejected code, dimensionless.
        code_u8: u8,
    },
    /// Recorder holds no more entries.
    RecorderFull,
    /// Recorder is frozen and rejects appends.
    RecorderFrozen,
    /// Replay fixture could not be built from locked presets.
    FixtureRejected,
}

impl core::fmt::Display for DeterminismError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnknownKind { code_u8 } => {
                write!(formatter, "unknown input kind: {code_u8}")
            }
            Self::BadWarpCode { code_u8 } => {
                write!(formatter, "bad warp code: {code_u8}")
            }
            Self::RecorderFull => write!(formatter, "input recorder is full"),
            Self::RecorderFrozen => write!(formatter, "input recorder is frozen"),
            Self::FixtureRejected => write!(formatter, "replay fixture rejected"),
        }
    }
}

impl std::error::Error for DeterminismError {}

/// Recordable input kinds in the headless stage.
///
/// Run-control under policy plus tweak and console writes. The game has
/// no inputs yet; adding one is a new variant plus a payload
/// constructor, never a recorder rewrite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum InputKind {
    /// Scheduler hold requested.
    Pause = 0,
    /// Scheduler release requested.
    Resume = 1,
    /// Single-step tick requested.
    StepTick = 2,
    /// Warp factor requested under policy.
    WarpRequest = 3,
    /// Tweakable applied while paused.
    TweakApply = 4,
    /// Console write command run.
    ConsoleWrite = 5,
}

impl InputKind {
    /// All input kinds in code order.
    pub const ALL: [Self; 6] = [
        Self::Pause,
        Self::Resume,
        Self::StepTick,
        Self::WarpRequest,
        Self::TweakApply,
        Self::ConsoleWrite,
    ];

    /// Return the short kind label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Pause => "pause",
            Self::Resume => "resume",
            Self::StepTick => "step",
            Self::WarpRequest => "warp",
            Self::TweakApply => "tweak",
            Self::ConsoleWrite => "console",
        }
    }

    /// Return the kind code, dimensionless.
    #[must_use]
    pub const fn code_u8(self) -> u8 {
        self as u8
    }

    /// Map a code onto a kind.
    ///
    /// # Errors
    ///
    /// Returns [`DeterminismError::UnknownKind`] above `5`.
    pub const fn from_code(code_u8: u8) -> Result<Self, DeterminismError> {
        match code_u8 {
            0 => Ok(Self::Pause),
            1 => Ok(Self::Resume),
            2 => Ok(Self::StepTick),
            3 => Ok(Self::WarpRequest),
            4 => Ok(Self::TweakApply),
            5 => Ok(Self::ConsoleWrite),
            _ => Err(DeterminismError::UnknownKind { code_u8 }),
        }
    }
}

/// Fixed-size input payload with typed constructors.
///
/// Pause, resume, and step payloads stay zeroed. Warp packs the warp
/// code plus context flags; tweak and console packs pack the registry
/// entry id plus value bits plus a kind tag. No allocation, ever.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputPayload([u8; INPUT_PAYLOAD_CAP_BYTES_USIZE]);

impl InputPayload {
    /// Build a zeroed payload for pause, resume, and step entries.
    #[must_use]
    pub const fn zero() -> Self {
        Self([0_u8; INPUT_PAYLOAD_CAP_BYTES_USIZE])
    }

    /// Pack a warp request with code plus context flag bits.
    #[must_use]
    pub const fn warp(warp_code_u8: u8, context_flags_u8: u8) -> Self {
        let mut bytes_u8 = [0_u8; INPUT_PAYLOAD_CAP_BYTES_USIZE];
        bytes_u8[0] = warp_code_u8;
        bytes_u8[1] = context_flags_u8;
        Self(bytes_u8)
    }

    /// Pack a tweak or console write with entry id plus value bits.
    #[must_use]
    pub const fn setting(entry_id_u16: u16, value_bits_u64: u64, tag_u8: u8) -> Self {
        let mut bytes_u8 = [0_u8; INPUT_PAYLOAD_CAP_BYTES_USIZE];
        let id_bytes = entry_id_u16.to_le_bytes();
        bytes_u8[0] = id_bytes[0];
        bytes_u8[1] = id_bytes[1];
        let value_bytes = value_bits_u64.to_le_bytes();
        bytes_u8[2] = value_bytes[0];
        bytes_u8[3] = value_bytes[1];
        bytes_u8[4] = value_bytes[2];
        bytes_u8[5] = value_bytes[3];
        bytes_u8[6] = value_bytes[4];
        bytes_u8[7] = value_bytes[5];
        bytes_u8[8] = value_bytes[6];
        bytes_u8[9] = value_bytes[7];
        bytes_u8[10] = tag_u8;
        Self(bytes_u8)
    }

    /// Return the raw payload bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; INPUT_PAYLOAD_CAP_BYTES_USIZE] {
        &self.0
    }

    /// Encode the payload as lowercase hex into a fixed buffer.
    ///
    /// Returns the buffer plus the used length; CSV form never allocates
    /// in the frame loop.
    #[must_use]
    pub const fn hex_bytes(&self) -> ([u8; PAYLOAD_HEX_CAP_BYTES_USIZE], usize) {
        let mut out_u8 = [0_u8; PAYLOAD_HEX_CAP_BYTES_USIZE];
        let mut index_usize = 0;
        while index_usize < INPUT_PAYLOAD_CAP_BYTES_USIZE {
            let byte_u8 = self.0[index_usize];
            out_u8[index_usize * HEX_DIGITS_PER_BYTE_USIZE] =
                HEX_ALPHABET_U8[(byte_u8 >> 4) as usize];
            out_u8[index_usize * HEX_DIGITS_PER_BYTE_USIZE + 1] =
                HEX_ALPHABET_U8[(byte_u8 & 0x0F) as usize];
            index_usize += 1;
        }
        (out_u8, PAYLOAD_HEX_CAP_BYTES_USIZE)
    }
}

/// One recorded input with tick, kind, and fixed payload.
///
/// Plain data, 41 bytes; the recorder pre-sizes 1024 entries (~42 KB).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputEntry {
    /// Tick count at input time, dimensionless.
    tick_count_u64: u64,
    /// Input kind.
    kind: InputKind,
    /// Fixed-size payload with units per kind.
    payload: InputPayload,
}

impl InputEntry {
    /// Build an entry from tick, kind, and payload.
    #[must_use]
    pub const fn new(tick_count_u64: u64, kind: InputKind, payload: InputPayload) -> Self {
        Self {
            tick_count_u64,
            kind,
            payload,
        }
    }

    /// Return the tick count, dimensionless.
    #[must_use]
    pub const fn tick_count_u64(self) -> u64 {
        self.tick_count_u64
    }

    /// Return the input kind.
    #[must_use]
    pub const fn kind(self) -> InputKind {
        self.kind
    }

    /// Return the payload.
    #[must_use]
    pub const fn payload(self) -> InputPayload {
        self.payload
    }
}

/// Recorder lifecycle states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecorderState {
    /// Accepting appends.
    Recording,
    /// Latched on scheduler hold; resume reopens.
    FrozenPause,
    /// Latched on export; a new run reopens.
    FrozenExport,
    /// Latched when the 1024-entry cap fills; evidence is never dropped.
    FrozenFull,
}

impl RecorderState {
    /// Return the short state label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Recording => "recording",
            Self::FrozenPause => "frozen-pause",
            Self::FrozenExport => "frozen-export",
            Self::FrozenFull => "frozen-full",
        }
    }
}

/// Append-only input recorder with a pre-sized buffer.
///
/// Starts clean on run start via `begin_run`, stops on pause or export.
/// Full latches instead of overwriting evidence. Shell state only and
/// never persists; exports serialize it explicitly.
#[derive(Debug, Clone)]
pub struct InputRecorder {
    /// Pre-sized entry buffer, append-only.
    entries: Vec<InputEntry>,
    /// Lifecycle state.
    state: RecorderState,
    /// Master seed of the recorded run, dimensionless.
    master_seed_u64: u64,
}

impl InputRecorder {
    /// Build an empty recorder with pre-sized buffers.
    ///
    /// Reserves the layout recorder capacity; no allocation after open.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::with_capacity(INPUT_RECORDER_CAPACITY_ENTRIES_USIZE),
            state: RecorderState::Recording,
            master_seed_u64: 0,
        }
    }

    /// Start a clean run under a master seed.
    ///
    /// Clears entries, sets recording plus clean; the caller clears taint
    /// through the top bar alongside.
    pub fn begin_run(&mut self, master_seed_u64: u64) {
        self.entries.clear();
        self.state = RecorderState::Recording;
        self.master_seed_u64 = master_seed_u64;
    }

    /// Return recorded entry count.
    #[must_use]
    pub fn len_usize(&self) -> usize {
        self.entries.len()
    }

    /// Report whether no entries are recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Return the lifecycle state.
    #[must_use]
    pub const fn state(&self) -> RecorderState {
        self.state
    }

    /// Return the run master seed, dimensionless.
    #[must_use]
    pub const fn master_seed_u64(&self) -> u64 {
        self.master_seed_u64
    }

    /// Return recorded entries oldest-first.
    #[must_use]
    pub fn entries(&self) -> &[InputEntry] {
        &self.entries
    }

    /// Append one entry while recording.
    ///
    /// Latches full instead of dropping evidence at the cap.
    ///
    /// # Errors
    ///
    /// Returns [`DeterminismError::RecorderFrozen`] while frozen, or
    /// [`DeterminismError::RecorderFull`] at the cap.
    pub fn record(&mut self, entry: InputEntry) -> Result<(), DeterminismError> {
        if self.state != RecorderState::Recording {
            return Err(DeterminismError::RecorderFrozen);
        }
        if self.entries.len() >= INPUT_RECORDER_CAPACITY_ENTRIES_USIZE {
            self.state = RecorderState::FrozenFull;
            return Err(DeterminismError::RecorderFull);
        }
        self.entries.push(entry);
        Ok(())
    }

    /// Latch frozen on scheduler hold.
    ///
    /// The pause transition itself is recorded by the caller as the last
    /// entry before this call.
    pub const fn stop_on_pause(&mut self) {
        if matches!(self.state, RecorderState::Recording) {
            self.state = RecorderState::FrozenPause;
        }
    }

    /// Latch frozen on bundle export.
    pub const fn stop_on_export(&mut self) {
        if matches!(self.state, RecorderState::Recording) {
            self.state = RecorderState::FrozenExport;
        }
    }

    /// Reopen appending after a pause and record the resume event.
    ///
    /// # Errors
    ///
    /// Returns [`DeterminismError`] when the resume entry cannot append
    /// (frozen export, frozen full, or a full buffer).
    pub fn resume(&mut self, tick_count_u64: u64) -> Result<(), DeterminismError> {
        if matches!(
            self.state,
            RecorderState::FrozenExport | RecorderState::FrozenFull
        ) {
            return Err(DeterminismError::RecorderFrozen);
        }
        self.state = RecorderState::Recording;
        self.record(InputEntry::new(
            tick_count_u64,
            InputKind::Resume,
            InputPayload::zero(),
        ))
    }
}

impl Default for InputRecorder {
    /// Default empty recorder with pre-sized buffers.
    fn default() -> Self {
        Self::new()
    }
}

/// Seed-tree view with master plus three domain streams.
///
/// Plain copy of `engine::rng::SeedTree` values for display; per-domain
/// rows recompute `split_domain` live to prove tag independence.
#[expect(
    clippy::struct_field_names,
    reason = "unit suffixes are required by the naming rule"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeedTreeView {
    /// Master seed, dimensionless.
    master_seed_u64: u64,
    /// Star-domain stream, dimensionless.
    gen_star_u64: u64,
    /// Body-domain stream, dimensionless.
    gen_body_u64: u64,
    /// Terrain-domain stream, dimensionless.
    gen_terrain_u64: u64,
}

impl SeedTreeView {
    /// Derive all domain streams from one master seed.
    ///
    /// Pure and infallible; reuses `engine::rng::SeedTree` as-is.
    #[must_use]
    pub fn from_master(master_seed_u64: u64) -> Self {
        let tree = engine::rng::SeedTree::new(master_seed_u64);
        Self {
            master_seed_u64: tree.master_seed_u64,
            gen_star_u64: tree.gen_star_u64,
            gen_body_u64: tree.gen_body_u64,
            gen_terrain_u64: tree.gen_terrain_u64,
        }
    }

    /// Return the master seed, dimensionless.
    #[must_use]
    pub const fn master_seed_u64(self) -> u64 {
        self.master_seed_u64
    }

    /// Return the star-domain stream, dimensionless.
    #[must_use]
    pub const fn gen_star_u64(self) -> u64 {
        self.gen_star_u64
    }

    /// Return the body-domain stream, dimensionless.
    #[must_use]
    pub const fn gen_body_u64(self) -> u64 {
        self.gen_body_u64
    }

    /// Return the terrain-domain stream, dimensionless.
    #[must_use]
    pub const fn gen_terrain_u64(self) -> u64 {
        self.gen_terrain_u64
    }

    /// Return the low 16 bits for short display, dimensionless.
    #[must_use]
    pub const fn short_u16(value_u64: u64) -> u16 {
        (value_u64 & SHORT_DISPLAY_MASK_U64) as u16
    }

    /// Draw the seed tree with per-domain rehash rows.
    ///
    /// Immediate-mode widgets only; creates no renderer. Available only
    /// with the non-default `dev-shell` feature.
    #[cfg(feature = "dev-shell")]
    pub fn draw(&self, ui: &mut egui::Ui) {
        ui.label(format!(
            "master_seed={seed:04x} ({seed})",
            seed = Self::short_u16(self.master_seed_u64),
        ))
        .on_hover_text(format!(
            "master seed full hex {full:016x} decimal {full}",
            full = self.master_seed_u64
        ));
        for (name, stream_u64, tag_u64) in [
            ("gen_star", self.gen_star_u64, engine::rng::GEN_STAR_TAG_U64),
            ("gen_body", self.gen_body_u64, engine::rng::GEN_BODY_TAG_U64),
            (
                "gen_terrain",
                self.gen_terrain_u64,
                engine::rng::GEN_TERRAIN_TAG_U64,
            ),
        ] {
            let rehashed_u64 = engine::rng::split_domain(self.master_seed_u64, tag_u64);
            ui.label(format!(
                "{name}={short:04x} rehash={same}",
                short = Self::short_u16(stream_u64),
                same = if rehashed_u64 == stream_u64 {
                    "match"
                } else {
                    "MISMATCH"
                }
            ))
            .on_hover_text(format!(
                "{name} full hex {stream_u64:016x} tag {tag_u64:016x}; one tag never moves the other rows"
            ));
        }
    }
}

/// Per-tick hash ring with short display.
///
/// Copies `snapshot_hash_u64` per observed frame into a pre-sized ring;
/// never hashes itself. Shell state only and never persists.
#[derive(Debug, Clone)]
pub struct HashRing {
    /// Pre-sized ring of tick plus hash pairs.
    pairs: Vec<(u64, u64)>,
    /// Next write index into the ring, dimensionless.
    next_index_usize: usize,
}

impl HashRing {
    /// Build an empty ring with pre-sized buffers.
    ///
    /// Reserves the hash-history capacity; no allocation after open.
    #[must_use]
    pub fn new() -> Self {
        Self {
            pairs: Vec::with_capacity(HASH_HISTORY_CAPACITY_ENTRIES_USIZE),
            next_index_usize: 0,
        }
    }

    /// Return filled entry count.
    #[must_use]
    pub fn len_usize(&self) -> usize {
        self.pairs.len()
    }

    /// Clear all pairs, keeping the reservation for reuse.
    ///
    /// No allocation; called on fresh runs outside the frame loop.
    pub fn clear(&mut self) {
        self.pairs.clear();
        self.next_index_usize = 0;
    }

    /// Report whether no hashes are stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    /// Push one tick plus hash pair, overwriting oldest when full.
    pub fn push(&mut self, tick_count_u64: u64, hash_u64: u64) {
        if self.pairs.len() < HASH_HISTORY_CAPACITY_ENTRIES_USIZE {
            self.pairs.push((tick_count_u64, hash_u64));
        } else {
            self.pairs[self.next_index_usize] = (tick_count_u64, hash_u64);
            self.next_index_usize =
                (self.next_index_usize + 1) % HASH_HISTORY_CAPACITY_ENTRIES_USIZE;
        }
    }

    /// Return the latest pair when one exists.
    #[must_use]
    pub fn latest(&self) -> Option<(u64, u64)> {
        if self.pairs.is_empty() {
            return None;
        }
        if self.pairs.len() < HASH_HISTORY_CAPACITY_ENTRIES_USIZE {
            return self.pairs.last().copied();
        }
        let newest_index_usize = (self.next_index_usize + HASH_HISTORY_CAPACITY_ENTRIES_USIZE - 1)
            % HASH_HISTORY_CAPACITY_ENTRIES_USIZE;
        self.pairs.get(newest_index_usize).copied()
    }

    /// Draw the current hash plus the newest rows and any divergence.
    ///
    /// Immediate-mode widgets only; creates no renderer. Shows at most
    /// 25 newest rows to bound draw cost. Available only with the
    /// non-default `dev-shell` feature.
    #[cfg(feature = "dev-shell")]
    pub fn draw(&self, ui: &mut egui::Ui, divergence: Option<&DivergenceRecord>) {
        match self.latest() {
            Some((tick_count_u64, hash_u64)) => {
                ui.label(format!(
                    "hash tick={tick_count_u64} {short:04x}",
                    short = SeedTreeView::short_u16(hash_u64)
                ))
                .on_hover_text(format!(
                    "per-tick hash full hex {hash_u64:016x} tick {tick_count_u64}"
                ));
            }
            None => {
                ui.label("hash: no ticks yet");
            }
        }
        if let Some(record) = divergence {
            ui.label(format!(
                "DIVERGED tick={tick} expected={expected:04x} actual={actual:04x}",
                tick = record.tick_count_u64,
                expected = SeedTreeView::short_u16(record.expected_hash_u64),
                actual = SeedTreeView::short_u16(record.actual_hash_u64)
            ))
            .on_hover_text(format!(
                "first divergence tick {} expected {:016x} actual {:016x}",
                record.tick_count_u64, record.expected_hash_u64, record.actual_hash_u64
            ));
        }
        for (tick_count_u64, hash_u64) in self.ordered().rev().take(25) {
            ui.label(format!(
                "tick={tick_count_u64} {short:04x}",
                short = SeedTreeView::short_u16(*hash_u64)
            ))
            .on_hover_text(format!("tick {tick_count_u64} full hex {hash_u64:016x}"));
        }
    }

    /// Iterate pairs oldest-first without allocating.
    #[cfg(feature = "dev-shell")]
    fn ordered(&self) -> impl DoubleEndedIterator<Item = &(u64, u64)> + '_ {
        let (head, tail) = self.pairs.split_at(self.next_index_usize);
        tail.iter().chain(head.iter())
    }

    /// Collect pairs oldest-first for bundle export.
    ///
    /// Allocates once per export; never called in the frame loop.
    #[must_use]
    pub fn export_pairs(&self) -> Vec<(u64, u64)> {
        let (head, tail) = self.pairs.split_at(self.next_index_usize);
        let mut ordered = Vec::with_capacity(self.pairs.len());
        ordered.extend_from_slice(tail);
        ordered.extend_from_slice(head);
        ordered
    }
}

impl Default for HashRing {
    /// Default empty ring with pre-sized buffers.
    fn default() -> Self {
        Self::new()
    }
}

/// Draw inputs borrowed for one determinism window frame.
///
/// Available only with the non-default `dev-shell` feature.
#[cfg(feature = "dev-shell")]
pub struct DeterminismDraw<'a> {
    /// Seed tree view.
    pub seed: &'a SeedTreeView,
    /// Per-tick hash ring.
    pub hashes: &'a HashRing,
    /// Input recorder.
    pub recorder: &'a InputRecorder,
    /// Last replay report, if any.
    pub report: Option<ReplayReport>,
}

/// Determinism window with open state.
///
/// Floating `Window`-layer panel opened from the determinism badge,
/// showing seed tree, hashes, recorder, loader, and export. Shell
/// state only and never persists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeterminismWindow {
    /// True while the window draws.
    open_bool: bool,
}

impl DeterminismWindow {
    /// Build a closed window.
    #[must_use]
    pub const fn new() -> Self {
        Self { open_bool: false }
    }

    /// Report whether the window draws.
    #[must_use]
    pub const fn is_open(self) -> bool {
        self.open_bool
    }

    /// Open the window from the determinism badge.
    pub const fn open(&mut self) {
        self.open_bool = true;
    }

    /// Close the window.
    pub const fn close(&mut self) {
        self.open_bool = false;
    }

    /// Draw the window sections when open, returning export demand.
    ///
    /// Immediate-mode widgets only; creates no renderer. Returns true
    /// when the export-bundle button fires; the caller performs the
    /// filesystem export outside draw. Available only with the
    /// non-default `dev-shell` feature.
    #[cfg(feature = "dev-shell")]
    pub fn draw(&mut self, ctx: &egui::Context, content: &DeterminismDraw<'_>) -> bool {
        if !self.open_bool {
            return false;
        }
        let mut export_requested_bool = false;
        let mut close_requested_bool = false;
        egui::Window::new("universe-determinism").show(ctx, |ui| {
            content.seed.draw(ui);
            content.hashes.draw(
                ui,
                content
                    .report
                    .and_then(|report| match report.status() {
                        ReplayStatus::Diverged(record) => Some(record),
                        ReplayStatus::Match => None,
                    })
                    .as_ref(),
            );
            ui.label(format!(
                "recorder {state} entries={entries}",
                state = content.recorder.state().label(),
                entries = content.recorder.len_usize()
            ));
            match content.report {
                Some(report) => report.draw(ui),
                None => {
                    ui.label("replay: no compare yet");
                }
            }
            if ui.button("export bundle").clicked() {
                export_requested_bool = true;
            }
            if ui.button("close").clicked() {
                close_requested_bool = true;
            }
        });
        if close_requested_bool {
            self.open_bool = false;
        }
        export_requested_bool
    }
}

impl Default for DeterminismWindow {
    /// Default closed window.
    fn default() -> Self {
        Self::new()
    }
}

/// First-divergence record from a replay compare.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DivergenceRecord {
    /// Tick count of divergence, dimensionless.
    tick_count_u64: u64,
    /// Expected hash, dimensionless.
    expected_hash_u64: u64,
    /// Actual hash, dimensionless.
    actual_hash_u64: u64,
    /// Input recorded at the diverging tick, if any.
    input_at_tick: Option<InputEntry>,
}

impl DivergenceRecord {
    /// Return the diverging tick, dimensionless.
    #[must_use]
    pub const fn tick_count_u64(self) -> u64 {
        self.tick_count_u64
    }

    /// Return the expected hash, dimensionless.
    #[must_use]
    pub const fn expected_hash_u64(self) -> u64 {
        self.expected_hash_u64
    }

    /// Return the actual hash, dimensionless.
    #[must_use]
    pub const fn actual_hash_u64(self) -> u64 {
        self.actual_hash_u64
    }

    /// Return the input at the diverging tick, if any.
    #[must_use]
    pub const fn input_at_tick(self) -> Option<InputEntry> {
        self.input_at_tick
    }
}

/// Replay compare outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayStatus {
    /// Every compared tick matched.
    Match,
    /// Compare stopped at the first divergence.
    Diverged(DivergenceRecord),
}

/// Replay report with compared tick count plus status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplayReport {
    /// Compared ticks, dimensionless.
    ticks_compared_u64: u64,
    /// Match or first divergence.
    status: ReplayStatus,
}

impl ReplayReport {
    /// Return compared ticks, dimensionless.
    #[must_use]
    pub const fn ticks_compared_u64(self) -> u64 {
        self.ticks_compared_u64
    }

    /// Return the compare status.
    #[must_use]
    pub const fn status(self) -> ReplayStatus {
        self.status
    }

    /// Draw the report line.
    ///
    /// Immediate-mode widgets only; creates no renderer. Available only
    /// with the non-default `dev-shell` feature.
    #[cfg(feature = "dev-shell")]
    pub fn draw(&self, ui: &mut egui::Ui) {
        match self.status {
            ReplayStatus::Match => {
                ui.label(format!(
                    "replay match ticks={ticks}",
                    ticks = self.ticks_compared_u64
                ));
            }
            ReplayStatus::Diverged(record) => {
                ui.label(format!(
                    "replay DIVERGED tick={tick} expected={expected:04x} actual={actual:04x}",
                    tick = record.tick_count_u64(),
                    expected = SeedTreeView::short_u16(record.expected_hash_u64()),
                    actual = SeedTreeView::short_u16(record.actual_hash_u64())
                ));
            }
        }
    }
}

/// Replay one seed plus input log headlessly and compare hashes.
///
/// Drives the canonical M1 fixture (250 km circular orbit, Mars-like
/// body and atmosphere, preset vehicle) with `SIM_TICK_S` steps:
/// applies warp requests at their ticks, advances the scheduler, steps
/// the point ship, captures the snapshot, and hashes. Pause, resume,
/// step, tweak, and console inputs ride along as markers with no
/// stepping effect; tweak and console writes never touch engine
/// constants. The stream seed evolves by `mix_seed` per tick exactly
/// like the record side, so equal inputs yield equal hashes on every
/// platform with the locked `libm`, `rand_xoshiro`, and `xxhash-rust`
/// versions. Available only with `dev-shell`.
///
/// # Errors
///
/// Returns [`DeterminismError::BadWarpCode`] for a warp code above `4`.
#[cfg(feature = "dev-shell")]
pub fn replay(
    master_seed_u64: u64,
    stream_seed_u64: u64,
    inputs: &[InputEntry],
    expected_hashes: &[(u64, u64)],
) -> Result<ReplayReport, DeterminismError> {
    use engine::atmosphere::AtmosphereParams;
    use engine::body::BodyParams;
    use engine::orbit::Mu;
    use engine::trajectory::VehicleParams;

    let body = BodyParams::mars_like();
    let Ok(atmosphere) = AtmosphereParams::mars_like() else {
        return Err(DeterminismError::FixtureRejected);
    };
    let vehicle = VehicleParams::preset();
    let Ok(mu) = Mu::new(body.gravitational_parameter_m3_s2()) else {
        return Err(DeterminismError::FixtureRejected);
    };
    Ok(replay_with_environment(
        master_seed_u64,
        stream_seed_u64,
        inputs,
        expected_hashes,
        &body,
        &atmosphere,
        &vehicle,
        mu,
    ))
}

/// Replay driver over caller-built Mars-like environment parts.
///
/// Splits environment construction (fallible) from the tick loop so the
/// loop stays focused. Available only with `dev-shell`.
#[cfg(feature = "dev-shell")]
#[expect(
    clippy::too_many_arguments,
    reason = "replay environment arrives as one fixture bundle"
)]
fn replay_with_environment(
    master_seed_u64: u64,
    stream_seed_u64: u64,
    inputs: &[InputEntry],
    expected_hashes: &[(u64, u64)],
    body: &engine::body::BodyParams,
    atmosphere: &engine::atmosphere::AtmosphereParams,
    vehicle: &engine::trajectory::VehicleParams,
    mu: engine::orbit::Mu,
) -> ReplayReport {
    use engine::sim::Scheduler;
    use engine::trajectory::{StateVector, step_point_ship};
    use engine::units::Seconds;
    use engine::warp::Warp;
    use glam::DVec3;

    let radius_m_f64 = body.radius_m().value() + 250_000.0;
    let start = StateVector::new(
        DVec3::new(radius_m_f64, 0.0, 0.0),
        DVec3::new(0.0, libm::sqrt(mu.value() / radius_m_f64), 0.0),
        Seconds::new(0.0),
    );
    let Ok(mut current) = start else {
        return diverged_report(0, 0, 0, 0, None);
    };
    let mut requested = Warp::X1;
    let mut stream_u64 = stream_seed_u64;
    let mut scheduler = Scheduler::default();
    let mut compared_u64 = 0;
    for (tick_u64, expected_u64) in expected_hashes.iter().copied() {
        while scheduler.step_count() < tick_u64 {
            scheduler.advance();
        }
        for entry in inputs
            .iter()
            .filter(|entry| entry.tick_count_u64() == tick_u64)
        {
            if entry.kind() == InputKind::WarpRequest {
                let code_u8 = entry.payload().as_bytes()[0];
                let Ok(warp) = warp_from_code(code_u8) else {
                    return diverged_report(compared_u64, tick_u64, expected_u64, 0, Some(*entry));
                };
                requested = warp;
            }
        }
        let stepped = step_point_ship(
            &current,
            engine::sim::SIM_TICK_S,
            body,
            atmosphere,
            vehicle,
            mu,
        );
        let Ok(sample) = stepped else {
            return diverged_report(
                compared_u64,
                tick_u64,
                expected_u64,
                0,
                input_recorded_at(inputs, tick_u64),
            );
        };
        current = sample.state;
        scheduler.advance();
        stream_u64 = engine::generation::mix_seed(stream_u64, scheduler.step_count());
        let captured = engine::inspect::capture_snapshot(
            &scheduler,
            &current,
            body,
            atmosphere,
            vehicle,
            master_seed_u64,
            stream_u64,
            requested,
            true,
            false,
            false,
        );
        let Ok(snapshot) = captured else {
            return diverged_report(
                compared_u64,
                tick_u64,
                expected_u64,
                0,
                input_recorded_at(inputs, tick_u64),
            );
        };
        let actual_u64 = snapshot_hash(&snapshot);
        if actual_u64 != expected_u64 {
            return diverged_report(
                compared_u64,
                tick_u64,
                expected_u64,
                actual_u64,
                input_recorded_at(inputs, tick_u64),
            );
        }
        compared_u64 += 1;
    }
    ReplayReport {
        ticks_compared_u64: compared_u64,
        status: ReplayStatus::Match,
    }
}

/// Find the first input recorded at a tick, if any.
///
/// Available only with the non-default `dev-shell` feature.
#[cfg(feature = "dev-shell")]
fn input_recorded_at(inputs: &[InputEntry], tick_count_u64: u64) -> Option<InputEntry> {
    inputs
        .iter()
        .find(|entry| entry.tick_count_u64() == tick_count_u64)
        .copied()
}

/// Build a diverged replay report for one tick.
///
/// Available only with the non-default `dev-shell` feature.
#[cfg(feature = "dev-shell")]
fn diverged_report(
    compared_u64: u64,
    tick_count_u64: u64,
    expected_u64: u64,
    actual_u64: u64,
    input_at_tick: Option<InputEntry>,
) -> ReplayReport {
    ReplayReport {
        ticks_compared_u64: compared_u64,
        status: ReplayStatus::Diverged(DivergenceRecord {
            tick_count_u64,
            expected_hash_u64: expected_u64,
            actual_hash_u64: actual_u64,
            input_at_tick,
        }),
    }
}

/// Map a warp code onto a warp factor.
///
/// Codes follow the inspect warp constants from `0` to `4`.
///
/// # Errors
///
/// Returns [`DeterminismError::BadWarpCode`] above `4`.
#[cfg(feature = "dev-shell")]
fn warp_from_code(code_u8: u8) -> Result<engine::warp::Warp, DeterminismError> {
    use engine::warp::Warp;
    match code_u8 {
        0 => Ok(Warp::X1),
        1 => Ok(Warp::X10),
        2 => Ok(Warp::X100),
        3 => Ok(Warp::X1000),
        4 => Ok(Warp::X10000),
        _ => Err(DeterminismError::BadWarpCode { code_u8 }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMOKE_MASTER_SEED_U64: u64 = 0x243F_6A88_85A3_08D3;
    const SMOKE_TICK_U64: u64 = 9;

    #[test]
    fn input_kinds_map_all_codes() {
        assert_eq!(InputKind::ALL.len(), 6);
        for (index_usize, kind) in InputKind::ALL.iter().enumerate() {
            let Ok(code_u8) = u8::try_from(index_usize) else {
                panic!("kind index must fit u8")
            };
            let Ok(roundtrip) = InputKind::from_code(code_u8) else {
                panic!("kind code must map")
            };
            assert_eq!(*kind, roundtrip);
            assert_eq!(kind.code_u8(), code_u8);
        }
        assert!(matches!(
            InputKind::from_code(9_u8),
            Err(DeterminismError::UnknownKind { .. })
        ));
    }

    #[test]
    fn payload_constructors_pack_fields() {
        let warp = InputPayload::warp(2_u8, 0x03);
        assert_eq!(warp.as_bytes()[0], 2_u8);
        assert_eq!(warp.as_bytes()[1], 0x03);
        let setting = InputPayload::setting(7_u16, 0x1234_5678_9ABC_DEF0, 1_u8);
        assert_eq!(setting.as_bytes()[0], 7_u8);
        assert_eq!(setting.as_bytes()[1], 0_u8);
        assert_eq!(setting.as_bytes()[10], 1_u8);
        let (hex_u8, len_usize) = InputPayload::zero().hex_bytes();
        assert_eq!(len_usize, PAYLOAD_HEX_CAP_BYTES_USIZE);
        assert!(hex_u8.iter().all(|byte_u8| *byte_u8 == b'0'));
        let entry = InputEntry::new(SMOKE_TICK_U64, InputKind::StepTick, InputPayload::zero());
        assert_eq!(entry.tick_count_u64(), SMOKE_TICK_U64);
        assert_eq!(entry.kind(), InputKind::StepTick);
    }

    #[test]
    fn recorder_freezes_instead_of_dropping() {
        let mut recorder = InputRecorder::new();
        recorder.begin_run(SMOKE_MASTER_SEED_U64);
        assert_eq!(recorder.master_seed_u64(), SMOKE_MASTER_SEED_U64);
        assert!(recorder.is_empty());
        assert_eq!(recorder.state(), RecorderState::Recording);
        assert!(
            recorder
                .record(InputEntry::new(
                    SMOKE_TICK_U64,
                    InputKind::Pause,
                    InputPayload::zero()
                ))
                .is_ok()
        );
        assert_eq!(recorder.len_usize(), 1);
        recorder.stop_on_pause();
        assert_eq!(recorder.state(), RecorderState::FrozenPause);
        assert!(matches!(
            recorder.record(InputEntry::new(
                SMOKE_TICK_U64,
                InputKind::StepTick,
                InputPayload::zero()
            )),
            Err(DeterminismError::RecorderFrozen)
        ));
        assert!(recorder.resume(SMOKE_TICK_U64 + 1).is_ok());
        assert_eq!(recorder.state(), RecorderState::Recording);
        assert_eq!(recorder.len_usize(), 2);
        recorder.stop_on_export();
        assert_eq!(recorder.state(), RecorderState::FrozenExport);
        assert!(matches!(
            recorder.resume(SMOKE_TICK_U64 + 2),
            Err(DeterminismError::RecorderFrozen)
        ));
    }

    #[test]
    fn determinism_errors_label_each_variant() {
        assert!(format!("{}", DeterminismError::UnknownKind { code_u8: 9 }).contains("unknown"));
        assert!(format!("{}", DeterminismError::BadWarpCode { code_u8: 9 }).contains("warp"));
        assert!(format!("{}", DeterminismError::RecorderFull).contains("full"));
        assert!(format!("{}", DeterminismError::RecorderFrozen).contains("frozen"));
        assert!(format!("{}", DeterminismError::FixtureRejected).contains("fixture"));
    }

    #[test]
    fn seed_tree_domains_split_independently() {
        let tree = SeedTreeView::from_master(SMOKE_MASTER_SEED_U64);
        assert_eq!(tree.master_seed_u64(), SMOKE_MASTER_SEED_U64);
        assert_eq!(
            tree.gen_star_u64(),
            engine::rng::split_domain(SMOKE_MASTER_SEED_U64, engine::rng::GEN_STAR_TAG_U64)
        );
        assert_eq!(
            tree.gen_body_u64(),
            engine::rng::split_domain(SMOKE_MASTER_SEED_U64, engine::rng::GEN_BODY_TAG_U64)
        );
        assert_eq!(
            tree.gen_terrain_u64(),
            engine::rng::split_domain(SMOKE_MASTER_SEED_U64, engine::rng::GEN_TERRAIN_TAG_U64)
        );
        assert_ne!(tree.gen_star_u64(), tree.gen_body_u64());
        assert_eq!(SeedTreeView::short_u16(u64::MAX), 0xFFFF);
    }

    #[test]
    fn hash_ring_reuses_buffer_at_capacity() {
        let mut ring = HashRing::new();
        assert!(ring.is_empty());
        for tick_u64 in 0..(HASH_HISTORY_CAPACITY_ENTRIES_USIZE as u64 + 4) {
            ring.push(tick_u64, tick_u64.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        }
        assert_eq!(ring.len_usize(), HASH_HISTORY_CAPACITY_ENTRIES_USIZE);
        let Some((latest_tick_u64, _)) = ring.latest() else {
            panic!("latest hash must exist")
        };
        assert_eq!(
            latest_tick_u64,
            HASH_HISTORY_CAPACITY_ENTRIES_USIZE as u64 + 3
        );
    }

    /// Record one hash stream over the canonical fixture for replay tests.
    ///
    /// Mirrors the `replay` tick loop exactly (advance, step, advance,
    /// mix, capture) so record and replay agree tick for tick. Test
    /// helper only.
    #[cfg(feature = "dev-shell")]
    fn record_stream(
        master_seed_u64: u64,
        stream_seed_u64: u64,
        inputs: &[InputEntry],
        tick_count_u64: u64,
    ) -> Result<Vec<(u64, u64)>, String> {
        use engine::atmosphere::AtmosphereParams;
        use engine::body::BodyParams;
        use engine::orbit::Mu;
        use engine::sim::{SIM_TICK_S, Scheduler};
        use engine::trajectory::{StateVector, VehicleParams, step_point_ship};
        use engine::units::Seconds;
        use engine::warp::Warp;
        use glam::DVec3;

        let body = BodyParams::mars_like();
        let atmosphere = AtmosphereParams::mars_like()
            .map_err(|error| format!("test atmosphere must validate: {error}"))?;
        let vehicle = VehicleParams::preset();
        let mu = Mu::new(body.gravitational_parameter_m3_s2())
            .map_err(|error| format!("test mu must validate: {error}"))?;
        let radius_m_f64 = body.radius_m().value() + 250_000.0;
        let mut current = StateVector::new(
            DVec3::new(radius_m_f64, 0.0, 0.0),
            DVec3::new(0.0, libm::sqrt(mu.value() / radius_m_f64), 0.0),
            Seconds::new(0.0),
        )
        .map_err(|error| format!("test state must validate: {error}"))?;
        let mut requested = Warp::X1;
        let mut stream_u64 = stream_seed_u64;
        let mut scheduler = Scheduler::default();
        let mut hashes = Vec::new();
        for tick_u64 in 0..tick_count_u64 {
            for entry in inputs
                .iter()
                .filter(|entry| entry.tick_count_u64() == tick_u64)
            {
                if entry.kind() == InputKind::WarpRequest {
                    requested = warp_from_code(entry.payload().as_bytes()[0])
                        .map_err(|error| format!("test warp must map: {error}"))?;
                }
            }
            let sample = step_point_ship(&current, SIM_TICK_S, &body, &atmosphere, &vehicle, mu)
                .map_err(|error| format!("test step must succeed: {error}"))?;
            current = sample.state;
            scheduler.advance();
            stream_u64 = engine::generation::mix_seed(stream_u64, scheduler.step_count());
            let snapshot = engine::inspect::capture_snapshot(
                &scheduler,
                &current,
                &body,
                &atmosphere,
                &vehicle,
                master_seed_u64,
                stream_u64,
                requested,
                true,
                false,
                false,
            )
            .map_err(|error| format!("test capture must succeed: {error}"))?;
            hashes.push((tick_u64, snapshot_hash(&snapshot)));
        }
        Ok(hashes)
    }

    /// Scripted inputs exercising every kind as recordable markers.
    #[cfg(feature = "dev-shell")]
    fn smoke_inputs() -> Vec<InputEntry> {
        vec![
            InputEntry::new(1_u64, InputKind::Pause, InputPayload::zero()),
            InputEntry::new(2_u64, InputKind::Resume, InputPayload::zero()),
            InputEntry::new(3_u64, InputKind::StepTick, InputPayload::zero()),
            InputEntry::new(
                4_u64,
                InputKind::WarpRequest,
                InputPayload::warp(2_u8, 0x03),
            ),
            InputEntry::new(
                5_u64,
                InputKind::TweakApply,
                InputPayload::setting(2_u16, 1_u64, 0_u8),
            ),
            InputEntry::new(6_u64, InputKind::ConsoleWrite, InputPayload::zero()),
        ]
    }

    #[cfg(feature = "dev-shell")]
    #[test]
    fn replay_reproduces_recorded_hashes() {
        let inputs = smoke_inputs();
        let Ok(expected) =
            record_stream(SMOKE_MASTER_SEED_U64, SMOKE_MASTER_SEED_U64, &inputs, 8_u64)
        else {
            panic!("record must succeed")
        };
        assert_eq!(expected.len(), 8);
        let Ok(report) = replay(
            SMOKE_MASTER_SEED_U64,
            SMOKE_MASTER_SEED_U64,
            &inputs,
            &expected,
        ) else {
            panic!("replay must run")
        };
        assert_eq!(report.ticks_compared_u64(), 8_u64);
        assert_eq!(report.status(), ReplayStatus::Match);
    }

    #[cfg(feature = "dev-shell")]
    #[test]
    fn replay_reports_first_divergence() {
        let inputs = smoke_inputs();
        let Ok(mut expected) =
            record_stream(SMOKE_MASTER_SEED_U64, SMOKE_MASTER_SEED_U64, &inputs, 8_u64)
        else {
            panic!("record must succeed")
        };
        expected[5].1 ^= 0xFF;
        let Ok(report) = replay(
            SMOKE_MASTER_SEED_U64,
            SMOKE_MASTER_SEED_U64,
            &inputs,
            &expected,
        ) else {
            panic!("replay must run")
        };
        assert_eq!(report.ticks_compared_u64(), 5_u64);
        let ReplayStatus::Diverged(record) = report.status() else {
            panic!("replay must diverge")
        };
        assert_eq!(record.tick_count_u64(), 5_u64);
        assert_ne!(record.expected_hash_u64(), record.actual_hash_u64());
    }

    #[cfg(feature = "dev-shell")]
    #[test]
    fn replay_rejects_bad_warp_code() {
        let bad_inputs = [InputEntry::new(
            2_u64,
            InputKind::WarpRequest,
            InputPayload::warp(9_u8, 0_u8),
        )];
        let Ok(report) = replay(
            SMOKE_MASTER_SEED_U64,
            SMOKE_MASTER_SEED_U64,
            &bad_inputs,
            &[(2_u64, 0_u64)],
        ) else {
            panic!("replay must run")
        };
        let ReplayStatus::Diverged(record) = report.status() else {
            panic!("bad warp must diverge")
        };
        assert_eq!(record.tick_count_u64(), 2_u64);
        let Some(entry) = record.input_at_tick() else {
            panic!("divergence must carry the input")
        };
        assert_eq!(entry.kind(), InputKind::WarpRequest);
    }
}
