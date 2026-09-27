//! Debug shell assembly over top bar, cost meter, router, and inspect.
//!
//! Owns [`TopBarState`](crate::top_bar::TopBarState),
//! [`ShellCostMeter`](crate::shell_cost::ShellCostMeter),
//! [`InputRouter`](crate::input::InputRouter),
//! [`InspectView`](crate::inspect_view::InspectView), and dock visibility with
//! a `Passthrough` default. Records shell draw cost separately from sim and
//! render cost; never writes sim state; removed by close flag or by building
//! without `dev-shell`.

use crate::budget::{BudgetError, BudgetStrip};
use crate::console::{Console, ConsoleError};
use crate::continuity::{ContinuityError, ContinuityMonitor};
use crate::determinism::{
    DeterminismError, HashRing, InputEntry, InputKind, InputPayload, InputRecorder, SeedTreeView,
};
use crate::export::ExportError;
use crate::input::{
    InputRoute, InputRouter, InputRouterError, RouterKey, TAP_PICK_TOLERANCE_PT_F32,
};
use crate::inspect_view::{InspectView, InspectViewError};
use crate::layout::{DesktopPreset, PanelVisibility, ShellInputMode};
use crate::log::{LogError, TraceLog};
use crate::shell_cost::{ShellCostError, ShellCostMeter};
use crate::top_bar::{TopBarError, TopBarState};
use crate::tweak::{TweakBoard, TweakError};
use engine::warp::{Warp, WarpContext};

#[cfg(feature = "dev-shell")]
use crate::bottom::{BottomDraw, BottomTabs};
#[cfg(feature = "dev-shell")]
use crate::budget::BudgetDenominators;
#[cfg(feature = "dev-shell")]
use crate::determinism::{DeterminismDraw, DeterminismWindow, ReplayReport};
#[cfg(feature = "dev-shell")]
use engine::inspect::SimSnapshot;

/// Shell assembly failures from the owned parts.
///
/// Returned for bad draw costs, bad tolerances, bad snapshot codes, bad
/// snapshot clocks, bad continuity samples, bad budget samples, and bad
/// log pushes. Never a sim write failure; the shell is read-only.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShellError {
    /// Top-bar clock, warp, or cost check failed.
    TopBar(TopBarError),
    /// Shell-cost sample or budget check failed.
    ShellCost(ShellCostError),
    /// Input-router duration or tolerance check failed.
    Input(InputRouterError),
    /// Inspect code map or tolerance check failed.
    Inspect(InspectViewError),
    /// Continuity sample or window check failed.
    Continuity(ContinuityError),
    /// Budget sample or denominator check failed.
    Budget(BudgetError),
    /// Log module cap check failed.
    Log(LogError),
    /// Determinism sample or replay check failed.
    Determinism(DeterminismError),
    /// Bundle export or verify check failed.
    Export(ExportError),
    /// Tweak lookup, range, or pause check failed.
    Tweak(TweakError),
    /// Console parse failure.
    Console(ConsoleError),
}

impl core::fmt::Display for ShellError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TopBar(source) => write!(formatter, "shell top bar: {source}"),
            Self::ShellCost(source) => write!(formatter, "shell cost: {source}"),
            Self::Input(source) => write!(formatter, "shell input: {source}"),
            Self::Inspect(source) => write!(formatter, "shell inspect: {source}"),
            Self::Continuity(source) => write!(formatter, "shell continuity: {source}"),
            Self::Budget(source) => write!(formatter, "shell budget: {source}"),
            Self::Log(source) => write!(formatter, "shell log: {source}"),
            Self::Determinism(source) => write!(formatter, "shell determinism: {source}"),
            Self::Export(source) => write!(formatter, "shell export: {source}"),
            Self::Tweak(source) => write!(formatter, "shell tweak: {source}"),
            Self::Console(source) => write!(formatter, "shell console: {source}"),
        }
    }
}

impl std::error::Error for ShellError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::TopBar(source) => Some(source),
            Self::ShellCost(source) => Some(source),
            Self::Input(source) => Some(source),
            Self::Inspect(source) => Some(source),
            Self::Continuity(source) => Some(source),
            Self::Budget(source) => Some(source),
            Self::Log(source) => Some(source),
            Self::Determinism(source) => Some(source),
            Self::Export(source) => Some(source),
            Self::Tweak(source) => Some(source),
            Self::Console(source) => Some(source),
        }
    }
}

impl From<TopBarError> for ShellError {
    /// Convert a top-bar failure into a shell failure.
    fn from(source: TopBarError) -> Self {
        Self::TopBar(source)
    }
}

impl From<ShellCostError> for ShellError {
    /// Convert a shell-cost failure into a shell failure.
    fn from(source: ShellCostError) -> Self {
        Self::ShellCost(source)
    }
}

impl From<InputRouterError> for ShellError {
    /// Convert an input-router failure into a shell failure.
    fn from(source: InputRouterError) -> Self {
        Self::Input(source)
    }
}

impl From<InspectViewError> for ShellError {
    /// Convert an inspect-view failure into a shell failure.
    fn from(source: InspectViewError) -> Self {
        Self::Inspect(source)
    }
}

impl From<ContinuityError> for ShellError {
    /// Convert a continuity failure into a shell failure.
    fn from(source: ContinuityError) -> Self {
        Self::Continuity(source)
    }
}

impl From<BudgetError> for ShellError {
    /// Convert a budget failure into a shell failure.
    fn from(source: BudgetError) -> Self {
        Self::Budget(source)
    }
}

impl From<LogError> for ShellError {
    /// Convert a log failure into a shell failure.
    fn from(source: LogError) -> Self {
        Self::Log(source)
    }
}

impl From<DeterminismError> for ShellError {
    /// Convert a determinism failure into a shell failure.
    fn from(source: DeterminismError) -> Self {
        Self::Determinism(source)
    }
}

impl From<ExportError> for ShellError {
    /// Convert an export failure into a shell failure.
    fn from(source: ExportError) -> Self {
        Self::Export(source)
    }
}

impl From<TweakError> for ShellError {
    /// Convert a tweak failure into a shell failure.
    fn from(source: TweakError) -> Self {
        Self::Tweak(source)
    }
}

impl From<ConsoleError> for ShellError {
    /// Convert a console failure into a shell failure.
    fn from(source: ConsoleError) -> Self {
        Self::Console(source)
    }
}

/// One-frame shell action flags for the game loop.
///
/// Data only; the loop performs exports and modal confirms outside draw.
#[cfg(feature = "dev-shell")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellAction {
    /// True when the export-bundle button fired this frame.
    export_requested_bool: bool,
}

#[cfg(feature = "dev-shell")]
impl ShellAction {
    /// Return whether a bundle export was requested.
    #[must_use]
    pub const fn export_requested(self) -> bool {
        self.export_requested_bool
    }
}

/// Caller-provided bundle identity strings for export.
///
/// Device and build facts the shell cannot know itself; export-time
/// data only, never persisted by the shell.
#[cfg(feature = "dev-shell")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleIdentity {
    /// Creation timestamp text, e.g. ISO-8601 UTC.
    pub created_utc: String,
    /// Application version text.
    pub app_version: String,
    /// Platform text, e.g. device model plus OS floor.
    pub platform: String,
    /// Thermal tier label text.
    pub tier: String,
}

/// Debug shell assembly with run control, cost, input, and inspect.
///
/// Plain state only; observes snapshot scalars by copy and records its
/// own draw cost. Shell state only and never persists.
#[derive(Debug, Clone)]
pub struct Shell {
    /// Run control plus clocks and badges.
    top_bar: TopBarState,
    /// Shell draw-cost meter with close flag.
    meter: ShellCostMeter,
    /// Input router with passthrough default.
    router: InputRouter,
    /// Read-only inspect view over snapshot scalars.
    inspect: InspectView,
    /// Continuity monitor with handoff markers.
    continuity: ContinuityMonitor,
    /// Budget strip with latest samples.
    strip: BudgetStrip,
    /// Bounded tracing log with filters.
    log: TraceLog,
    /// Seed tree view with domain streams.
    seed_view: SeedTreeView,
    /// Per-tick hash ring.
    hash_ring: HashRing,
    /// Append-only input recorder.
    recorder: InputRecorder,
    /// Tweak board with drafts plus confirm state.
    tweaks: TweakBoard,
    /// Console with history plus output.
    console: Console,
    /// Last observed snapshot for bundle export, if any.
    #[cfg(feature = "dev-shell")]
    last_snapshot: Option<SimSnapshot>,
    /// Last replay report, if any.
    #[cfg(feature = "dev-shell")]
    last_report: Option<ReplayReport>,
    /// Bottom tab assembly with selection state.
    #[cfg(feature = "dev-shell")]
    bottom: BottomTabs,
    /// Determinism window with open state.
    #[cfg(feature = "dev-shell")]
    window: DeterminismWindow,
    /// Dock visibility for the current preset.
    visibility: PanelVisibility,
}

impl Shell {
    /// Build a passthrough shell with ticker-only visibility.
    ///
    /// Starts closed-never: badges only, game keeps all input, top bar
    /// alone for minimal occlusion.
    ///
    /// # Errors
    ///
    /// Returns [`ShellError`] when the default pick tolerance is rejected.
    pub fn new() -> Result<Self, ShellError> {
        Ok(Self {
            top_bar: TopBarState::new(),
            meter: ShellCostMeter::new(),
            router: InputRouter::new(),
            inspect: InspectView::new(TAP_PICK_TOLERANCE_PT_F32)?,
            continuity: ContinuityMonitor::new(),
            strip: BudgetStrip::new(),
            log: TraceLog::new(),
            seed_view: SeedTreeView::from_master(0),
            hash_ring: HashRing::new(),
            recorder: InputRecorder::new(),
            tweaks: TweakBoard::new(),
            console: Console::new(),
            #[cfg(feature = "dev-shell")]
            last_snapshot: None,
            #[cfg(feature = "dev-shell")]
            last_report: None,
            #[cfg(feature = "dev-shell")]
            bottom: BottomTabs::new(),
            #[cfg(feature = "dev-shell")]
            window: DeterminismWindow::new(),
            visibility: PanelVisibility::for_preset(DesktopPreset::TickerOnly),
        })
    }

    /// Return the current input-routing mode.
    #[must_use]
    pub const fn mode(&self) -> ShellInputMode {
        self.router.mode()
    }

    /// Report whether the game gets all input.
    #[must_use]
    pub const fn is_passthrough(&self) -> bool {
        self.router.is_passthrough()
    }

    /// Report whether shell widgets get input first.
    #[must_use]
    pub const fn is_focused(&self) -> bool {
        self.router.is_focused()
    }

    /// Report whether the close-shell button was pressed.
    #[must_use]
    pub const fn is_closed(&self) -> bool {
        self.meter.is_closed()
    }

    /// Return run control plus clocks and badges.
    #[must_use]
    pub const fn top_bar(&self) -> &TopBarState {
        &self.top_bar
    }

    /// Return run control for pause, step, and warp.
    ///
    /// Shell state only; run control never writes sim state directly.
    pub const fn top_bar_mut(&mut self) -> &mut TopBarState {
        &mut self.top_bar
    }

    /// Return the shell draw-cost meter.
    #[must_use]
    pub const fn meter(&self) -> &ShellCostMeter {
        &self.meter
    }

    /// Return the input router.
    #[must_use]
    pub const fn router(&self) -> &InputRouter {
        &self.router
    }

    /// Return the read-only inspect view.
    #[must_use]
    pub const fn inspect(&self) -> &InspectView {
        &self.inspect
    }

    /// Return the continuity monitor.
    #[must_use]
    pub const fn continuity(&self) -> &ContinuityMonitor {
        &self.continuity
    }

    /// Return the continuity monitor for snapshot pushes.
    ///
    /// Shell state only; pushes never write sim state.
    pub const fn continuity_mut(&mut self) -> &mut ContinuityMonitor {
        &mut self.continuity
    }

    /// Return the budget strip.
    #[must_use]
    pub const fn budget_strip(&self) -> &BudgetStrip {
        &self.strip
    }

    /// Return the budget strip for sample recording.
    ///
    /// Shell state only; samples never write sim state.
    pub const fn budget_strip_mut(&mut self) -> &mut BudgetStrip {
        &mut self.strip
    }

    /// Return the tracing log.
    #[must_use]
    pub const fn trace_log(&self) -> &TraceLog {
        &self.log
    }

    /// Return the tracing log for entry pushes.
    ///
    /// Shell state only; pushes never write sim state.
    pub const fn trace_log_mut(&mut self) -> &mut TraceLog {
        &mut self.log
    }

    /// Return the seed tree view.
    #[must_use]
    pub const fn seed_tree(&self) -> &SeedTreeView {
        &self.seed_view
    }

    /// Return the per-tick hash ring.
    #[must_use]
    pub const fn hash_ring(&self) -> &HashRing {
        &self.hash_ring
    }

    /// Return the input recorder.
    #[must_use]
    pub const fn recorder(&self) -> &InputRecorder {
        &self.recorder
    }

    /// Return the input recorder for run starts and event appends.
    ///
    /// Shell state only; appends never write sim state.
    pub const fn recorder_mut(&mut self) -> &mut InputRecorder {
        &mut self.recorder
    }

    /// Return the tweak board.
    #[must_use]
    pub const fn tweak_board(&self) -> &TweakBoard {
        &self.tweaks
    }

    /// Return the console.
    #[must_use]
    pub const fn console(&self) -> &Console {
        &self.console
    }

    /// Return the console for input plus history plus output.
    ///
    /// Shell state only and never persists.
    pub const fn console_mut(&mut self) -> &mut Console {
        &mut self.console
    }

    /// Return the bottom tab assembly.
    ///
    /// Available only with the non-default `dev-shell` feature.
    #[cfg(feature = "dev-shell")]
    #[must_use]
    pub const fn bottom_tabs(&self) -> &BottomTabs {
        &self.bottom
    }

    /// Return the bottom tabs for tab selection.
    ///
    /// Shell state only; selection never writes sim state. Available only
    /// with the non-default `dev-shell` feature.
    #[cfg(feature = "dev-shell")]
    pub const fn bottom_tabs_mut(&mut self) -> &mut BottomTabs {
        &mut self.bottom
    }

    /// Return the determinism window.
    ///
    /// Available only with the non-default `dev-shell` feature.
    #[cfg(feature = "dev-shell")]
    #[must_use]
    pub const fn determinism_window(&self) -> &DeterminismWindow {
        &self.window
    }

    /// Return the determinism window for open and close.
    ///
    /// Shell state only. Available only with the non-default `dev-shell`
    /// feature.
    #[cfg(feature = "dev-shell")]
    pub const fn determinism_window_mut(&mut self) -> &mut DeterminismWindow {
        &mut self.window
    }

    /// Return the last replay report, if any.
    ///
    /// Available only with the non-default `dev-shell` feature.
    #[cfg(feature = "dev-shell")]
    #[must_use]
    pub const fn last_report(&self) -> Option<ReplayReport> {
        self.last_report
    }

    /// Store the last replay report for the window.
    ///
    /// Shell state only. Available only with the non-default
    /// `dev-shell` feature.
    #[cfg(feature = "dev-shell")]
    pub const fn set_last_report(&mut self, report: ReplayReport) {
        self.last_report = Some(report);
    }

    /// Return dock visibility for the current preset.
    #[must_use]
    pub const fn visibility(&self) -> PanelVisibility {
        self.visibility
    }

    /// Switch dock visibility without changing content.
    pub const fn set_visibility(&mut self, visibility: PanelVisibility) {
        self.visibility = visibility;
    }

    /// Handle a desktop toggle key and return the new mode.
    pub const fn handle_key(&mut self, key: RouterKey) -> ShellInputMode {
        self.router.on_key(key)
    }

    /// Handle a dev-tag long-press and return the new mode.
    ///
    /// # Errors
    ///
    /// Returns [`ShellError`] when `duration_s_f64` is non-finite or negative.
    pub fn handle_long_press(&mut self, duration_s_f64: f64) -> Result<ShellInputMode, ShellError> {
        Ok(self.router.on_dev_tag_long_press(duration_s_f64)?)
    }

    /// Handle a multi-finger tap and return the new mode.
    pub const fn handle_tap(&mut self, finger_count_u8: u8) -> ShellInputMode {
        self.router.on_multi_finger_tap(finger_count_u8)
    }

    /// Decide one frame under the `wants` routing rule.
    #[must_use]
    pub const fn route(&self, wants_pointer_input: bool, wants_keyboard_input: bool) -> InputRoute {
        self.router.route(wants_pointer_input, wants_keyboard_input)
    }

    /// Record one shell draw sample as the cost timing hook.
    ///
    /// Updates the meter ring plus the top-bar shell badge. Call with the
    /// measured draw milliseconds after each shell draw; pass
    /// `FRAME_BUDGET_MS` at draw time for fractions, never stored here.
    ///
    /// # Errors
    ///
    /// Returns [`ShellError`] when `draw_ms_f64` is non-finite or negative.
    pub fn record_draw_cost(&mut self, draw_ms_f64: f64) -> Result<(), ShellError> {
        self.meter.record_sample(draw_ms_f64)?;
        self.top_bar.set_shell_cost(draw_ms_f64)?;
        Ok(())
    }

    /// Return the latest shell draw cost in milliseconds.
    #[must_use]
    pub const fn draw_cost_ms_f64(&self) -> f64 {
        self.meter.latest_draw_ms_f64()
    }

    /// Return the mean shell draw cost in milliseconds.
    #[must_use]
    pub fn average_draw_ms_f64(&self) -> f64 {
        self.meter.average_ms_f64()
    }

    /// Hold the scheduler and record the pause input.
    ///
    /// Run control under policy: recordable but never tainting. The
    /// recorder latches frozen; best-effort appends never fail pause.
    pub fn pause(&mut self) {
        self.top_bar.pause();
        let tick_count_u64 = self.top_bar.tick_count_u64();
        self.record_input(InputKind::Pause, InputPayload::zero(), tick_count_u64);
        self.recorder.stop_on_pause();
    }

    /// Release the scheduler and record the resume input.
    ///
    /// Reopens recorder appending after a pause; export and full
    /// freezes stay latched while the run continues.
    pub fn resume(&mut self) {
        self.top_bar.resume();
        let tick_count_u64 = self.top_bar.tick_count_u64();
        if self.recorder.resume(tick_count_u64).is_err() {
            // Recorder frozen by export or full; the run continues and
            // the gap is visible as a frozen recorder state.
        }
    }

    /// Request one fixed-step tick and record the step input.
    ///
    /// Sets a shell flag only; the scheduler consumes it outside.
    pub fn request_step(&mut self) {
        self.top_bar.request_step();
        let tick_count_u64 = self.top_bar.tick_count_u64();
        self.record_input(InputKind::StepTick, InputPayload::zero(), tick_count_u64);
    }

    /// Request a warp factor under the sim rules and record it.
    ///
    /// Higher factors need a ship in orbit or transit; denials leave
    /// state and record nothing.
    ///
    /// # Errors
    ///
    /// Returns [`ShellError::TopBar`] with `WarpDenied` when the rules
    /// reject the request.
    pub fn request_warp(&mut self, requested: Warp, ctx: WarpContext) -> Result<Warp, ShellError> {
        let granted = self.top_bar.request_warp(requested, ctx)?;
        let tick_count_u64 = self.top_bar.tick_count_u64();
        let mut flags_u8 = 0;
        if ctx.in_ship {
            flags_u8 |= 0x01;
        }
        if ctx.in_orbit_or_transit {
            flags_u8 |= 0x02;
        }
        if ctx.in_atmosphere {
            flags_u8 |= 0x04;
        }
        if ctx.approaching {
            flags_u8 |= 0x08;
        }
        if ctx.alarm {
            flags_u8 |= 0x10;
        }
        self.record_input(
            InputKind::WarpRequest,
            InputPayload::warp(self.top_bar.effective_warp_code_u8(), flags_u8),
            tick_count_u64,
        );
        Ok(granted)
    }

    /// Start a fresh run with clean determinism state.
    ///
    /// Clears taint plus ticker, restarts the recorder, refreshes the
    /// seed view, and clears plot plus hash histories. Shell state
    /// only and never persists.
    pub fn begin_run(&mut self, master_seed_u64: u64) {
        self.top_bar.begin_run();
        self.recorder.begin_run(master_seed_u64);
        self.seed_view = SeedTreeView::from_master(master_seed_u64);
        self.continuity.clear();
        self.hash_ring.clear();
    }

    /// Append one input entry on a best-effort basis.
    ///
    /// Full and frozen recorders refuse; run control continues either
    /// way and the frozen state shows the gap.
    fn record_input(&mut self, kind: InputKind, payload: InputPayload, tick_count_u64: u64) {
        if self
            .recorder
            .record(InputEntry::new(tick_count_u64, kind, payload))
            .is_err()
        {
            // Recorder full or frozen; run control continues.
        }
    }

    /// Observe a snapshot by copy without writing sim state.
    ///
    /// Forwards clocks plus warp, seed, and hash to the top bar, records
    /// the readout curves in the continuity monitor, and replaces the
    /// inspect view. Available only with `dev-shell`.
    ///
    /// # Errors
    ///
    /// Returns [`ShellError`] for bad snapshot clocks or unknown codes.
    #[cfg(feature = "dev-shell")]
    pub fn observe_snapshot(&mut self, snapshot: &SimSnapshot) -> Result<(), ShellError> {
        let view = InspectView::from_snapshot(snapshot, self.router.pick_tolerance_pt_f32())?;
        self.top_bar.observe_snapshot_view(
            snapshot.tick_count_u64,
            snapshot.elapsed_s_f64,
            snapshot.warp_code_u8,
            snapshot.drop_reason_u8,
            snapshot.master_seed_u64,
            snapshot.snapshot_hash_u64,
        )?;
        self.continuity.push_snapshot(snapshot)?;
        self.hash_ring
            .push(snapshot.tick_count_u64, snapshot.snapshot_hash_u64);
        if self.seed_view.master_seed_u64() != snapshot.master_seed_u64 {
            self.seed_view = SeedTreeView::from_master(snapshot.master_seed_u64);
        }
        self.last_snapshot = Some(*snapshot);
        self.inspect = view;
        Ok(())
    }

    /// Mark the shell closed via the close-shell button.
    pub const fn request_close(&mut self) {
        self.meter.request_close();
    }
    /// Reopen the shell after a close.
    pub const fn reopen(&mut self) {
        self.meter.reopen();
    }

    /// Execute one console line with safe-read versus tainting rules.
    ///
    /// Reads echo values without touching state; `set` stages a draft
    /// and applies on explicit Enter while paused; `warp` routes
    /// through the sim policy with snapshot-derived context;
    /// `load` and `replay` record plus taint and defer modals to the
    /// game loop. Available only with `dev-shell`.
    ///
    /// # Errors
    ///
    /// Returns [`ShellError`] for parse failures, unknown names, bad
    /// values, denied warps, and pause violations.
    #[cfg(feature = "dev-shell")]
    pub fn execute_console_line(&mut self, line: &str) -> Result<(), ShellError> {
        use crate::console::{Command, parse_command};
        let command = parse_command(line)?;
        let tick_count_u64 = self.top_bar.tick_count_u64();
        self.record_input(
            InputKind::ConsoleWrite,
            InputPayload::zero(),
            tick_count_u64,
        );
        match command {
            Command::Get(name) | Command::Watch(name) => self.exec_console_read(name),
            Command::Seed => {
                let view = self.seed_view;
                self.console.echo(
                    &format!(
                        "master={master:04x} star={star:04x} body={body:04x} terrain={terrain:04x}",
                        master = SeedTreeView::short_u16(view.master_seed_u64()),
                        star = SeedTreeView::short_u16(view.gen_star_u64()),
                        body = SeedTreeView::short_u16(view.gen_body_u64()),
                        terrain = SeedTreeView::short_u16(view.gen_terrain_u64())
                    ),
                    false,
                );
                Ok(())
            }
            Command::Hash => {
                match self.hash_ring.latest() {
                    Some((tick_u64, hash_u64)) => {
                        self.console.echo(
                            &format!(
                                "tick={tick} hash={short:04x} full={full:016x}",
                                tick = tick_u64,
                                short = SeedTreeView::short_u16(hash_u64),
                                full = hash_u64
                            ),
                            false,
                        );
                    }
                    None => {
                        self.console.echo("hash: no ticks yet", false);
                    }
                }
                Ok(())
            }
            Command::Set(name, value) => self.exec_console_set(name, value),
            Command::Warp(code) => self.exec_console_warp(code),
            Command::Load(path) => {
                self.top_bar.mark_tainted();
                self.console.echo(
                    &format!("load {path}: modal confirm runs in the game loop"),
                    true,
                );
                Ok(())
            }
            Command::Replay(source) => {
                self.top_bar.mark_tainted();
                self.console.echo(
                    &format!("replay {source}: modal confirm runs in the game loop"),
                    true,
                );
                Ok(())
            }
        }
    }

    /// Echo one registry value for console reads.
    #[cfg(feature = "dev-shell")]
    fn exec_console_read(&mut self, name: &str) -> Result<(), ShellError> {
        use crate::tweak::{TweakKind, lookup_entry, lookup_entry_id};
        let id_u16 = lookup_entry_id(name)?;
        let entry = lookup_entry(id_u16)?;
        let bits_u64 = self.tweaks.committed_bits(id_u16)?;
        let rendered = match entry.kind {
            TweakKind::F64 => {
                format!(
                    "{value:.4} {unit}",
                    value = f64::from_bits(bits_u64),
                    unit = entry.unit
                )
            }
            _ => format!(
                "bits={bits:016x} {unit}",
                bits = bits_u64,
                unit = entry.unit
            ),
        };
        self.console.echo(&format!("{name} = {rendered}"), false);
        Ok(())
    }

    /// Stage plus apply one registry write from console text.
    #[cfg(feature = "dev-shell")]
    fn exec_console_set(&mut self, name: &str, value: &str) -> Result<(), ShellError> {
        use crate::console::parse_value_bits;
        use crate::tweak::lookup_entry_id;
        let id_u16 = lookup_entry_id(name)?;
        let entry_kind = match crate::tweak::lookup_entry(id_u16) {
            Ok(entry) => entry.kind,
            Err(error) => return Err(ShellError::Tweak(error)),
        };
        let bits_u64 = parse_value_bits(value, entry_kind)?;
        self.tweaks.set_draft(id_u16, bits_u64)?;
        let tick_count_u64 = self.top_bar.tick_count_u64();
        let paused_bool = self.top_bar.is_paused();
        match self.tweaks.apply(
            id_u16,
            tick_count_u64,
            paused_bool,
            &mut self.recorder,
            &mut self.top_bar,
        ) {
            Ok(command) => {
                self.console.echo(
                    &format!(
                        "set {name}[{id}] = {value}",
                        id = command.entry_id_u16(),
                        value = command.value_bits_u64()
                    ),
                    true,
                );
                Ok(())
            }
            Err(crate::tweak::TweakError::NotPaused) => {
                self.console
                    .echo(&format!("draft staged for {name}; paused apply only"), true);
                Ok(())
            }
            Err(error) => Err(ShellError::Tweak(error)),
        }
    }

    /// Request a warp factor from console code text.
    #[cfg(feature = "dev-shell")]
    fn exec_console_warp(&mut self, code: &str) -> Result<(), ShellError> {
        let Ok(code_u8) = code.parse::<u8>() else {
            self.console.echo(&format!("bad warp code: {code}"), true);
            return Err(ShellError::Console(crate::console::ConsoleError::BadValue));
        };
        let Ok(warp) = TopBarState::warp_from_code(code_u8) else {
            self.console.echo(&format!("bad warp code: {code}"), true);
            return Err(ShellError::TopBar(TopBarError::InvalidWarpCode { code_u8 }));
        };
        let ctx = self.console_warp_context();
        let Ok(granted) = self.request_warp(warp, ctx) else {
            self.console.echo("warp denied by sim policy", true);
            return Err(ShellError::TopBar(TopBarError::WarpDenied {
                requested_factor_f64: warp.factor(),
            }));
        };
        self.console
            .echo(&format!("warp {factor}x", factor = granted.factor()), false);
        Ok(())
    }

    /// Build warp context from the last snapshot flags.
    ///
    /// Falls back to on-foot denial without sim context. Available
    /// only with `dev-shell`.
    #[cfg(feature = "dev-shell")]
    fn console_warp_context(&self) -> WarpContext {
        use engine::inspect::{
            WARP_FLAG_ALARM_U8, WARP_FLAG_APPROACHING_U8, WARP_FLAG_IN_ATMOSPHERE_U8,
            WARP_FLAG_IN_ORBIT_OR_TRANSIT_U8, WARP_FLAG_IN_SHIP_U8,
        };
        match self.last_snapshot {
            Some(snapshot) => WarpContext {
                in_ship: snapshot.warp_flags_u8 & WARP_FLAG_IN_SHIP_U8 != 0,
                in_orbit_or_transit: snapshot.warp_flags_u8 & WARP_FLAG_IN_ORBIT_OR_TRANSIT_U8 != 0,
                in_atmosphere: snapshot.warp_flags_u8 & WARP_FLAG_IN_ATMOSPHERE_U8 != 0,
                approaching: snapshot.warp_flags_u8 & WARP_FLAG_APPROACHING_U8 != 0,
                alarm: snapshot.warp_flags_u8 & WARP_FLAG_ALARM_U8 != 0,
            },
            None => WarpContext::on_foot(),
        }
    }

    /// Export the section-11 bundle for the current shell state.
    ///
    /// Assembles all eight files from recorder, hash ring, seed view,
    /// snapshot, tweak board, log excerpt, and meters; writes
    /// atomically, verifies checksums before parse, and quarantines
    /// on mismatch without retrying. Available only with `dev-shell`.
    ///
    /// # Errors
    ///
    /// Returns [`ShellError`] for missing snapshots, non-finite
    /// readouts, IO failures, or digest mismatches.
    #[cfg(feature = "dev-shell")]
    pub fn export_bundle_to(
        &self,
        bundle_dir: &std::path::Path,
        identity: &BundleIdentity,
    ) -> Result<std::path::PathBuf, ShellError> {
        use crate::export::{
            HASHES_FILE_NAME, INPUTS_FILE_NAME, META_FILE_NAME, export_bundle_files,
            format_config_toml, format_hashes_csv, format_inputs_csv, format_meta_toml,
            format_seed_tree_toml, format_snapshot_toml, quarantine_bundle, verify_bundle_hashes,
        };
        let Some(snapshot) = self.last_snapshot else {
            return Err(ShellError::Export(crate::export::ExportError::MissingField));
        };
        let inputs_csv = format_inputs_csv(self.recorder.entries());
        let hashes_csv = format_hashes_csv(&self.hash_ring.export_pairs());
        let meta_toml = format_meta_toml(
            &identity.created_utc,
            &identity.app_version,
            &identity.platform,
            &identity.tier,
            crate::export::content_hash_u64(inputs_csv.as_bytes()),
            crate::export::content_hash_u64(hashes_csv.as_bytes()),
        );
        let seed_view =
            crate::determinism::SeedTreeView::from_master(self.recorder.master_seed_u64());
        let seed_tree_toml = format_seed_tree_toml(
            seed_view.master_seed_u64(),
            seed_view.gen_star_u64(),
            seed_view.gen_body_u64(),
            seed_view.gen_terrain_u64(),
        );
        let snapshot_toml = format_snapshot_toml(&snapshot)?;
        let rendered_rows = Self::tweak_config_rows(&self.tweaks);
        let row_refs: Vec<(&str, &str)> = rendered_rows
            .iter()
            .map(|row| (row.0.as_str(), row.1.as_str()))
            .collect();
        let config_toml = format_config_toml(
            Self::preset_name_for_bits(self.visibility.bits_u8()),
            &row_refs,
        );
        let log_text = self
            .log
            .excerpt_around(self.top_bar.tick_count_u64())
            .join("\n");
        let system_text = format!(
            "platform = \"{platform}\"\ntier = \"{tier}\"\nframe_ms = {frame:.3}\nshell_avg_ms = {shell:.3}\ntick = {tick}\n",
            platform = identity.platform,
            tier = identity.tier,
            frame = self.top_bar.frame_ms_f64(),
            shell = self.meter.average_ms_f64(),
            tick = self.top_bar.tick_count_u64()
        );
        let files = [
            (META_FILE_NAME, meta_toml.as_str()),
            ("seed_tree.toml", seed_tree_toml.as_str()),
            (INPUTS_FILE_NAME, inputs_csv.as_str()),
            (HASHES_FILE_NAME, hashes_csv.as_str()),
            ("snapshot.toml", snapshot_toml.as_str()),
            ("config.toml", config_toml.as_str()),
            ("log_excerpt.txt", log_text.as_str()),
            ("system.txt", system_text.as_str()),
        ];
        export_bundle_files(bundle_dir, &files)?;
        match verify_bundle_hashes(bundle_dir) {
            Ok(()) => Ok(bundle_dir.to_path_buf()),
            Err(crate::export::ExportError::HashMismatch {
                expected_u64,
                actual_u64,
            }) => {
                let _ = quarantine_bundle(bundle_dir, expected_u64, actual_u64);
                Err(ShellError::Export(
                    crate::export::ExportError::HashMismatch {
                        expected_u64,
                        actual_u64,
                    },
                ))
            }
            Err(error) => Err(ShellError::Export(error)),
        }
    }

    /// Name the preset matching visibility bits for bundle config.
    fn preset_name_for_bits(bits_u8: u8) -> &'static str {
        use crate::layout::{
            BOTTOM_TABS_BIT_U8, LEFT_PANEL_BIT_U8, RIGHT_PANEL_BIT_U8, TOP_BAR_BIT_U8,
        };
        match bits_u8 {
            bits if bits == TOP_BAR_BIT_U8 | LEFT_PANEL_BIT_U8 | BOTTOM_TABS_BIT_U8 => "descent",
            bits if bits == TOP_BAR_BIT_U8 | BOTTOM_TABS_BIT_U8 => "determinism-or-budget",
            bits if bits == TOP_BAR_BIT_U8 => "ticker-only",
            other if other & RIGHT_PANEL_BIT_U8 != 0 => "inspector",
            _ => "custom",
        }
    }

    /// Render tweak committed values for bundle config.
    fn tweak_config_rows(tweaks: &TweakBoard) -> Vec<(String, String)> {
        use crate::tweak::{
            DENSITY_SCALE_ID_U16, HEATING_GAIN_ID_U16, PLOT_WINDOW_ID_U16, TweakKind, lookup_entry,
        };
        let mut rows = Vec::new();
        for id_u16 in [
            DENSITY_SCALE_ID_U16,
            HEATING_GAIN_ID_U16,
            PLOT_WINDOW_ID_U16,
        ] {
            let Ok(entry) = lookup_entry(id_u16) else {
                continue;
            };
            let rendered = match tweaks.committed_bits(id_u16) {
                Ok(bits_u64) if entry.kind == TweakKind::F64 => {
                    format!(
                        "{value:.4} {unit}",
                        value = f64::from_bits(bits_u64),
                        unit = entry.unit
                    )
                }
                Ok(bits_u64) => format!("bits={bits_u64:016x}"),
                Err(_) => String::from("unreadable"),
            };
            rows.push((String::from(entry.name), rendered));
        }
        rows
    }

    /// Draw top bar, router, inspector, bottom tabs, and window in one pass.
    ///
    /// Immediate-mode widgets only; creates no renderer. Skips everything
    /// when closed; skips the bottom tabs and right panel unless the
    /// current preset shows them; draws the determinism window when
    /// open. Returns frame action flags for the game loop. Available
    /// only with the non-default `dev-shell` feature.
    #[cfg(feature = "dev-shell")]
    pub fn draw(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        frame_budget_ms_f64: f64,
        budgets: BudgetDenominators,
        tracy_connected_bool: bool,
    ) -> ShellAction {
        if self.meter.is_closed() {
            return ShellAction {
                export_requested_bool: false,
            };
        }
        self.top_bar
            .draw(ctx, ui, &mut self.meter, frame_budget_ms_f64);
        self.router.draw(ui);
        self.inspect.draw(ui);
        if self.visibility.shows_bottom_tabs() {
            let content = BottomDraw {
                monitor: &self.continuity,
                strip: &self.strip,
                budgets,
                log: &self.log,
                tracy_connected_bool,
                tweaks: &mut self.tweaks,
                console: &mut self.console,
                paused_bool: self.top_bar.is_paused(),
            };
            self.bottom.draw(ui, content);
        }
        if self.visibility.shows_right_panel() {
            let paused_bool = self.top_bar.is_paused();
            self.tweaks.draw(ui, paused_bool);
        }
        let export_requested_bool = {
            let content = DeterminismDraw {
                seed: &self.seed_view,
                hashes: &self.hash_ring,
                recorder: &self.recorder,
                report: self.last_report,
            };
            self.window.draw(ctx, &content)
        };
        ShellAction {
            export_requested_bool,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::DesktopPreset;

    const SMOKE_DRAW_MS_F64: f64 = 0.4;
    const SMOKE_BUDGET_MS_F64: f64 = 32.0;
    const EXPECTED_FRACTION_F64: f64 = 0.0125;
    const FRACTION_TOL_F64: f64 = 1e-12;

    fn smoke_shell() -> Shell {
        let Ok(shell) = Shell::new() else {
            panic!("smoke shell must build")
        };
        shell
    }

    #[test]
    fn new_shell_defaults_to_passthrough_ticker() {
        let shell = smoke_shell();
        assert_eq!(shell.mode(), ShellInputMode::Passthrough);
        assert!(shell.is_passthrough());
        assert!(!shell.is_focused());
        assert!(!shell.is_closed());
        assert_eq!(
            shell.visibility(),
            PanelVisibility::for_preset(DesktopPreset::TickerOnly)
        );
        assert!(shell.visibility().shows_top_bar());
        assert!(!shell.visibility().shows_left_panel());
        assert_eq!(shell.top_bar().tick_count_u64(), 0);
        assert!(shell.meter().is_empty());
        assert_eq!(shell.router().mode(), ShellInputMode::Passthrough);
        assert_eq!(shell.inspect().tick_count_u64(), 0);
        assert!(shell.continuity().is_empty());
        assert_eq!(
            shell.budget_strip().thermal_tier(),
            crate::budget::ThermalTier::Medium
        );
        assert!(shell.trace_log().is_empty());
        assert!(shell.recorder().is_empty());
        assert!(shell.hash_ring().is_empty());
        assert_eq!(shell.tweak_board().confirm_id_u16(), None);
        assert!((shell.draw_cost_ms_f64() - 0.0).abs() < FRACTION_TOL_F64);
    }

    #[test]
    fn keys_and_gestures_route_through_shell() {
        let mut shell = smoke_shell();
        assert_eq!(shell.handle_key(RouterKey::F3), ShellInputMode::Focused);
        assert!(shell.is_focused());
        assert_eq!(shell.route(false, false), InputRoute::Game);
        assert_eq!(shell.route(true, false), InputRoute::Shell);
        let Ok(mode) = shell.handle_long_press(0.5) else {
            panic!("threshold press must route")
        };
        assert_eq!(mode, ShellInputMode::Passthrough);
        assert_eq!(shell.handle_tap(3), ShellInputMode::Focused);
        shell.top_bar_mut().pause();
        assert!(shell.top_bar().is_paused());
    }

    #[test]
    fn draw_cost_hook_updates_meter_and_badge() {
        let mut shell = smoke_shell();
        assert!(
            shell.record_draw_cost(SMOKE_DRAW_MS_F64).is_ok(),
            "smoke cost must record"
        );
        assert!((shell.draw_cost_ms_f64() - SMOKE_DRAW_MS_F64).abs() < FRACTION_TOL_F64);
        assert!((shell.average_draw_ms_f64() - SMOKE_DRAW_MS_F64).abs() < FRACTION_TOL_F64);
        assert!((shell.top_bar().shell_draw_ms_f64() - SMOKE_DRAW_MS_F64).abs() < FRACTION_TOL_F64);
        let Ok(fraction) = shell.meter().fraction_of_budget(SMOKE_BUDGET_MS_F64) else {
            panic!("smoke fraction must divide")
        };
        assert!((fraction - EXPECTED_FRACTION_F64).abs() < FRACTION_TOL_F64);
        assert!(matches!(
            shell.record_draw_cost(f64::NAN),
            Err(ShellError::ShellCost(_))
        ));
        assert!(matches!(
            shell.handle_long_press(f64::NAN),
            Err(ShellError::Input(_))
        ));
    }

    #[test]
    fn close_flag_removes_shell_and_reopen_restores() {
        let mut shell = smoke_shell();
        shell.request_close();
        assert!(shell.is_closed());
        shell.reopen();
        assert!(!shell.is_closed());
        shell.set_visibility(PanelVisibility::for_preset(DesktopPreset::Descent));
        assert!(shell.visibility().shows_left_panel());
        assert!(shell.visibility().shows_bottom_tabs());
    }

    #[test]
    fn shell_error_labels_each_part() {
        let top = ShellError::from(TopBarError::NonFinite { value_f64: 1.0 });
        assert!(format!("{top}").contains("top bar"));
        let cost = ShellError::from(ShellCostError::Negative { value_f64: 1.0 });
        assert!(format!("{cost}").contains("cost"));
        let input = ShellError::from(InputRouterError::Negative { value_f64: 1.0 });
        assert!(format!("{input}").contains("input"));
        let inspect = ShellError::from(InspectViewError::Negative { value_f64: 1.0 });
        assert!(format!("{inspect}").contains("inspect"));
        let continuity =
            ShellError::from(crate::continuity::ContinuityError::Negative { value_f64: 1.0 });
        assert!(format!("{continuity}").contains("continuity"));
        let budget = ShellError::from(crate::budget::BudgetError::Negative { value_f64: 1.0 });
        assert!(format!("{budget}").contains("budget"));
        let log = ShellError::from(crate::log::LogError::ModuleTooLong { len_usize: 40 });
        assert!(format!("{log}").contains("shell log"));
        let determinism = ShellError::from(crate::determinism::DeterminismError::RecorderFull);
        assert!(format!("{determinism}").contains("determinism"));
        let export = ShellError::from(crate::export::ExportError::MissingField);
        assert!(format!("{export}").contains("export"));
        let tweak = ShellError::from(crate::tweak::TweakError::UnknownName);
        assert!(format!("{tweak}").contains("tweak"));
        let console = ShellError::from(crate::console::ConsoleError::UnknownVerb);
        assert!(format!("{console}").contains("console"));
        assert!(std::error::Error::source(&top).is_some());
    }

    #[cfg(feature = "dev-shell")]
    #[test]
    fn observe_snapshot_replaces_view_and_clocks() {
        use crate::inspect_view::{
            INSPECT_DROP_NONE_U8, INSPECT_FRAME_DEPTH_M1_U8, INSPECT_FRAME_ORBIT_U8,
            INSPECT_MARK_SHIP_POINT_U8, INSPECT_REGIME_ORBIT_U8, INSPECT_WARP_X1_U8,
        };
        let mut shell = smoke_shell();
        let snapshot = SimSnapshot {
            tick_count_u64: 7,
            elapsed_s_f64: 0.35,
            ship_epoch_s_f64: 0.35,
            master_seed_u64: 0x1234_ABCD_5678_EF90,
            stream_seed_u64: 9,
            snapshot_hash_u64: 0xDEAD_BEEF_0000_4321,
            position_m_f64: [3_639_500.0, 0.0, 0.0],
            velocity_mps_f64: [0.0, 3_400.0, 0.0],
            drag_mps2_f64: [0.0, 0.0, 0.0],
            vel_dir_f64: [0.0, 1.0, 0.0],
            altitude_m_f64: 250_000.0,
            speed_mps_f64: 3_400.0,
            pressure_pa_f64: 0.0,
            temperature_k_f64: 210.0,
            density_kg_m3_f64: 0.0,
            heat_flux_w_per_m2_f64: 0.0,
            g_load_g_f64: 0.0,
            semi_major_axis_m_f64: 3_639_500.0,
            eccentricity_f64: 0.01,
            inclination_rad_f64: 0.3,
            raan_rad_f64: 0.7,
            arg_periapsis_rad_f64: 0.5,
            mean_anomaly_rad_f64: 1.0,
            mu_m3_s2_f64: 4.282_837e13,
            pick_altitude_m_f64: 249_000.0,
            pick_range_m_f64: 1_000.0,
            frame_body_id_u32: 1,
            parent_body_id_u32: 0,
            pick_body_id_u32: 1,
            pick_cell_x_i32: 3,
            pick_cell_y_i32: -2,
            warp_code_u8: INSPECT_WARP_X1_U8,
            drop_reason_u8: INSPECT_DROP_NONE_U8,
            warp_flags_u8: 3,
            regime_u8: INSPECT_REGIME_ORBIT_U8,
            frame_level_u8: INSPECT_FRAME_ORBIT_U8,
            frame_depth_u8: INSPECT_FRAME_DEPTH_M1_U8,
            elements_valid_u8: 1,
            pick_valid_u8: 1,
            mark_kind_u8: INSPECT_MARK_SHIP_POINT_U8,
            _pad_u8: [0_u8; 3],
        };
        assert!(
            shell.observe_snapshot(&snapshot).is_ok(),
            "smoke snapshot must observe"
        );
        assert_eq!(shell.inspect().tick_count_u64(), 7);
        assert_eq!(shell.continuity().len_usize(), 1);
        assert_eq!(shell.hash_ring().len_usize(), 1);
        assert_eq!(shell.seed_tree().master_seed_u64(), 0x1234_ABCD_5678_EF90);
        assert_eq!(shell.top_bar().tick_count_u64(), 7);
        assert_eq!(shell.inspect().mark_label(), "ship-point");
        assert!(shell.inspect().pick_valid());
        let mut bad = snapshot;
        bad.warp_code_u8 = 9;
        assert!(matches!(
            shell.observe_snapshot(&bad),
            Err(ShellError::Inspect(_))
        ));
    }
}
