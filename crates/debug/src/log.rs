//! Bounded tracing log with level and module filters.
//!
//! Keeps a pre-sized ring of recent entries in memory only, with draw-time
//! level and module-substring filters. Entries store fixed-size byte arrays
//! so no allocation happens after shell open; log state never persists. The
//! excerpt hook serves the future Phase C bug-bundle export.

use crate::layout::LOG_HISTORY_CAPACITY_ENTRIES_USIZE;

/// Module name cap in bytes.
///
/// Tracing module paths such as `universe_engine::trajectory` fit; longer
/// names are typed errors, never truncated silently.
/// Source: issue #36 Step 4 design.
pub const LOG_MODULE_CAP_BYTES_USIZE: usize = 32;

/// Message cap in bytes for display.
///
/// Longer messages truncate at a character boundary on push.
/// Source: issue #36 Step 4 design.
pub const LOG_MESSAGE_CAP_BYTES_USIZE: usize = 256;

/// Drawn rows cap per frame, dimensionless.
///
/// Bounds draw cost on the reference phone; history keeps recording.
/// Source: issue #36 Step 4 design.
pub const LOG_DRAW_ROWS_USIZE: usize = 50;

/// Log range-check and cap failures.
///
/// Returned for oversize modules; messages truncate instead of failing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogError {
    /// Module name exceeded 32 bytes.
    ModuleTooLong {
        /// Rejected byte length, dimensionless.
        len_usize: usize,
    },
}

impl core::fmt::Display for LogError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ModuleTooLong { len_usize } => {
                write!(formatter, "log module too long: {len_usize} bytes")
            }
        }
    }
}

impl std::error::Error for LogError {}

/// Log severity from trace through error.
///
/// Ordered so level filters keep entries at or above a threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    /// Finest tracing spans.
    Trace,
    /// Debug diagnostics.
    Debug,
    /// Normal events.
    Info,
    /// Recoverable problems.
    Warn,
    /// Failures.
    Error,
}

impl LogLevel {
    /// All levels from trace through error.
    pub const ALL: [Self; 5] = [
        Self::Trace,
        Self::Debug,
        Self::Info,
        Self::Warn,
        Self::Error,
    ];

    /// Return the short level label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Trace => "trace",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

/// One log entry with fixed-size storage.
///
/// Plain data only; module and message bytes are valid UTF-8 by
/// construction because pushes take `&str`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogEntry {
    /// Tick count at push, dimensionless.
    tick_count_u64: u64,
    /// Entry severity.
    level: LogLevel,
    /// Module name bytes, valid UTF-8.
    module_u8: [u8; LOG_MODULE_CAP_BYTES_USIZE],
    /// Module byte length, dimensionless.
    module_len_usize: usize,
    /// Message bytes truncated to cap, valid UTF-8.
    message_u8: [u8; LOG_MESSAGE_CAP_BYTES_USIZE],
    /// Message byte length, dimensionless.
    message_len_usize: usize,
}

impl LogEntry {
    /// Return the tick count, dimensionless.
    #[must_use]
    pub const fn tick_count_u64(self) -> u64 {
        self.tick_count_u64
    }

    /// Return the severity.
    #[must_use]
    pub const fn level(self) -> LogLevel {
        self.level
    }

    /// Return the module name.
    #[must_use]
    pub fn module_str(&self) -> &str {
        core::str::from_utf8(&self.module_u8[..self.module_len_usize]).unwrap_or_default()
    }

    /// Return the message.
    #[must_use]
    pub fn message_str(&self) -> &str {
        core::str::from_utf8(&self.message_u8[..self.message_len_usize]).unwrap_or_default()
    }
}

/// Bounded tracing log with a pre-sized ring.
///
/// Buffers allocate once at open and reuse after warmup; log state never
/// persists. Filters are draw-time predicates that copy nothing.
#[derive(Debug, Clone)]
pub struct TraceLog {
    /// Pre-sized entry ring, oldest overwritten once full.
    entries: Vec<LogEntry>,
    /// Next write index into the ring, dimensionless.
    next_index_usize: usize,
    /// Minimum severity shown.
    filter_level: LogLevel,
    /// Module substring filter bytes, valid UTF-8.
    filter_module_u8: [u8; LOG_MODULE_CAP_BYTES_USIZE],
    /// Module filter byte length, dimensionless.
    filter_module_len_usize: usize,
}

impl TraceLog {
    /// Build an empty log with pre-sized buffers.
    ///
    /// Reserves the layout log-history capacity; no allocation happens
    /// after warmup. The default filter shows info and above, all modules.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::with_capacity(LOG_HISTORY_CAPACITY_ENTRIES_USIZE),
            next_index_usize: 0,
            filter_level: LogLevel::Info,
            filter_module_u8: [0_u8; LOG_MODULE_CAP_BYTES_USIZE],
            filter_module_len_usize: 0,
        }
    }

    /// Return filled entry count.
    #[must_use]
    pub fn len_usize(&self) -> usize {
        self.entries.len()
    }

    /// Report whether no entries are stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Return the minimum severity shown.
    #[must_use]
    pub const fn filter_level(&self) -> LogLevel {
        self.filter_level
    }

    /// Set the minimum severity shown.
    ///
    /// Safe-only draw predicate; never touches sim state.
    pub const fn set_filter_level(&mut self, level: LogLevel) {
        self.filter_level = level;
    }

    /// Set the module substring filter.
    ///
    /// Empty shows all modules. Safe-only draw predicate.
    ///
    /// # Errors
    ///
    /// Returns [`LogError::ModuleTooLong`] when the filter exceeds 32 bytes.
    pub fn set_filter_module(&mut self, module: &str) -> Result<(), LogError> {
        if module.len() > LOG_MODULE_CAP_BYTES_USIZE {
            return Err(LogError::ModuleTooLong {
                len_usize: module.len(),
            });
        }
        self.filter_module_u8 = [0_u8; LOG_MODULE_CAP_BYTES_USIZE];
        self.filter_module_len_usize = module.len();
        self.filter_module_u8[..module.len()].copy_from_slice(module.as_bytes());
        Ok(())
    }

    /// Push one entry, truncating long messages at a boundary.
    ///
    /// Overwrites the oldest entry once full; no allocation after open.
    ///
    /// # Errors
    ///
    /// Returns [`LogError::ModuleTooLong`] when the module exceeds 32 bytes.
    pub fn push(
        &mut self,
        tick_count_u64: u64,
        level: LogLevel,
        module: &str,
        message: &str,
    ) -> Result<(), LogError> {
        if module.len() > LOG_MODULE_CAP_BYTES_USIZE {
            return Err(LogError::ModuleTooLong {
                len_usize: module.len(),
            });
        }
        let mut module_u8 = [0_u8; LOG_MODULE_CAP_BYTES_USIZE];
        module_u8[..module.len()].copy_from_slice(module.as_bytes());
        let mut message_u8 = [0_u8; LOG_MESSAGE_CAP_BYTES_USIZE];
        let mut message_len_usize = message.len().min(LOG_MESSAGE_CAP_BYTES_USIZE);
        while !message.is_char_boundary(message_len_usize) {
            message_len_usize -= 1;
        }
        message_u8[..message_len_usize].copy_from_slice(&message.as_bytes()[..message_len_usize]);
        let entry = LogEntry {
            tick_count_u64,
            level,
            module_u8,
            module_len_usize: module.len(),
            message_u8,
            message_len_usize,
        };
        if self.entries.len() < LOG_HISTORY_CAPACITY_ENTRIES_USIZE {
            self.entries.push(entry);
        } else {
            self.entries[self.next_index_usize] = entry;
            self.next_index_usize =
                (self.next_index_usize + 1) % LOG_HISTORY_CAPACITY_ENTRIES_USIZE;
        }
        Ok(())
    }

    /// Render 25 lines before plus 25 after a tick for bundle export.
    ///
    /// Allocates once per export; never called in the frame loop.
    /// Shell state stays in memory only; the caller writes the file.
    #[must_use]
    pub fn excerpt_around(&self, center_tick_u64: u64) -> Vec<String> {
        let (head, tail) = self.entries.split_at(self.next_index_usize);
        let mut ordered: Vec<&LogEntry> = Vec::with_capacity(self.entries.len());
        ordered.extend(tail.iter());
        ordered.extend(head.iter());
        let mut before: Vec<String> = Vec::new();
        let mut after: Vec<String> = Vec::new();
        for entry in ordered {
            let line = format!(
                "tick={tick} {level} {module}: {message}",
                tick = entry.tick_count_u64(),
                level = entry.level().label(),
                module = entry.module_str(),
                message = entry.message_str()
            );
            match entry.tick_count_u64().cmp(&center_tick_u64) {
                core::cmp::Ordering::Less => {
                    if before.len() >= 25 {
                        before.remove(0);
                    }
                    before.push(line);
                }
                core::cmp::Ordering::Greater => {
                    if after.len() < 25 {
                        after.push(line);
                    }
                }
                core::cmp::Ordering::Equal => {
                    before.push(line);
                }
            }
        }
        before.extend(after);
        before
    }

    /// Report whether an entry passes the current filters.
    #[must_use]
    pub fn passes_filter(&self, entry: &LogEntry) -> bool {
        if entry.level() < self.filter_level {
            return false;
        }
        if self.filter_module_len_usize == 0 {
            return true;
        }
        let Ok(filter) =
            core::str::from_utf8(&self.filter_module_u8[..self.filter_module_len_usize])
        else {
            return false;
        };
        entry.module_str().contains(filter)
    }

    /// Draw filter state plus the newest matching rows.
    ///
    /// Immediate-mode widgets only; creates no renderer. Shows at most 50
    /// newest matching rows to bound draw cost. Available only with the
    /// non-default `dev-shell` feature.
    #[cfg(feature = "dev-shell")]
    pub fn draw(&self, ui: &mut egui::Ui, tracy_connected_bool: bool) {
        let filter_module_str =
            core::str::from_utf8(&self.filter_module_u8[..self.filter_module_len_usize])
                .unwrap_or_default();
        ui.label(format!(
            "log filter={filter} module={module} tracy={tracy} entries={entries}",
            filter = self.filter_level.label(),
            module = if filter_module_str.is_empty() {
                "all"
            } else {
                filter_module_str
            },
            tracy = tracy_connected_bool,
            entries = self.entries.len()
        ))
        .on_hover_text("in-memory ring only; never persisted; excerpt hook serves Phase C export");
        let mut shown_usize = 0;
        if self.entries.len() < LOG_HISTORY_CAPACITY_ENTRIES_USIZE {
            for entry in self.entries.iter().rev() {
                if shown_usize >= LOG_DRAW_ROWS_USIZE {
                    break;
                }
                if !self.passes_filter(entry) {
                    continue;
                }
                ui.label(format!(
                    "tick={tick} {level} {module}: {message}",
                    tick = entry.tick_count_u64(),
                    level = entry.level().label(),
                    module = entry.module_str(),
                    message = entry.message_str()
                ));
                shown_usize += 1;
            }
        } else {
            for offset_usize in 0..LOG_HISTORY_CAPACITY_ENTRIES_USIZE {
                if shown_usize >= LOG_DRAW_ROWS_USIZE {
                    break;
                }
                let index_usize =
                    (self.next_index_usize + LOG_HISTORY_CAPACITY_ENTRIES_USIZE - 1 - offset_usize)
                        % LOG_HISTORY_CAPACITY_ENTRIES_USIZE;
                let entry = &self.entries[index_usize];
                if !self.passes_filter(entry) {
                    continue;
                }
                ui.label(format!(
                    "tick={tick} {level} {module}: {message}",
                    tick = entry.tick_count_u64(),
                    level = entry.level().label(),
                    module = entry.module_str(),
                    message = entry.message_str()
                ));
                shown_usize += 1;
            }
        }
    }
}

impl Default for TraceLog {
    /// Default empty log with pre-sized buffers.
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMOKE_TICK_U64: u64 = 7;

    #[test]
    fn push_filters_and_truncates() {
        let mut log = TraceLog::new();
        assert!(log.is_empty());
        assert_eq!(log.len_usize(), 0);
        assert!(
            log.push(SMOKE_TICK_U64, LogLevel::Info, "sim", "tick advanced")
                .is_ok()
        );
        assert!(
            log.push(SMOKE_TICK_U64, LogLevel::Debug, "render", "frame drawn")
                .is_ok()
        );
        assert_eq!(log.len_usize(), 2);
        assert_eq!(log.filter_level(), LogLevel::Info);
        let info_entry = LogEntry {
            tick_count_u64: SMOKE_TICK_U64,
            level: LogLevel::Info,
            module_u8: [0_u8; LOG_MODULE_CAP_BYTES_USIZE],
            module_len_usize: 0,
            message_u8: [0_u8; LOG_MESSAGE_CAP_BYTES_USIZE],
            message_len_usize: 0,
        };
        assert!(log.passes_filter(&info_entry));
        log.set_filter_level(LogLevel::Warn);
        assert!(!log.passes_filter(&info_entry));
        log.set_filter_level(LogLevel::Trace);
        assert!(
            log.push(SMOKE_TICK_U64, LogLevel::Trace, "sim", "fine")
                .is_ok()
        );
        assert!(log.set_filter_module("sim").is_ok());
        assert_eq!(LogLevel::ALL.len(), 5);
    }

    #[test]
    fn long_message_truncates_at_boundary() {
        let mut log = TraceLog::new();
        let long_message = "e".repeat(LOG_MESSAGE_CAP_BYTES_USIZE + 40);
        assert!(
            log.push(SMOKE_TICK_U64, LogLevel::Error, "sim", &long_message)
                .is_ok()
        );
        assert_eq!(log.len_usize(), 1);
    }

    #[test]
    fn rejects_oversize_module() {
        let mut log = TraceLog::new();
        let long_module = "m".repeat(LOG_MODULE_CAP_BYTES_USIZE + 1);
        assert!(matches!(
            log.push(SMOKE_TICK_U64, LogLevel::Info, &long_module, "msg"),
            Err(LogError::ModuleTooLong { .. })
        ));
        assert!(matches!(
            log.set_filter_module(&long_module),
            Err(LogError::ModuleTooLong { .. })
        ));
        assert!(log.is_empty());
    }

    #[test]
    fn ring_reuses_buffer_at_capacity() {
        let mut log = TraceLog::new();
        for tick_u64 in 0..(LOG_HISTORY_CAPACITY_ENTRIES_USIZE as u64 + 5) {
            assert!(
                log.push(tick_u64, LogLevel::Info, "sim", "tick").is_ok(),
                "ring push must succeed"
            );
        }
        assert_eq!(log.len_usize(), LOG_HISTORY_CAPACITY_ENTRIES_USIZE);
    }
}
