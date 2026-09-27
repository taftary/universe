//! Console parser with history and completion over the registry.
//!
//! Borrows the input line without allocating: commands split the line
//! into verbs plus arguments, reads resolve against the tweak
//! registry, and writes route through the tweak draft/apply path or
//! the warp policy in Step 5 shell wiring. History and output rings
//! pre-size at open; the input buffer reuses one 128-byte capacity.
//! Shell state only and never persists.

use crate::tweak::{REGISTRY, TweakKind};

/// Console history reservation in entries at shell open.
///
/// Source: `docs/tech/debug.md` section 8 console history.
pub const CONSOLE_HISTORY_ENTRIES_USIZE: usize = 64;

/// Console line cap in bytes for history storage.
///
/// Longer lines truncate at a character boundary on push.
/// Source: issue #38 Step 4 design.
pub const CONSOLE_LINE_CAP_BYTES_USIZE: usize = 128;

/// Console output cap in lines for the scroll view.
///
/// Bounds draw cost; history keeps no more rows.
/// Source: issue #38 Step 4 design.
pub const CONSOLE_OUTPUT_CAP_LINES_USIZE: usize = 128;

/// Console parse failures.
///
/// Returned for unknown verbs, missing arguments, and bad values.
/// Name resolution against the registry happens at execution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConsoleError {
    /// Verb is outside the command set.
    UnknownVerb,
    /// Required argument is missing.
    MissingArg,
    /// Value text does not parse.
    BadValue,
}

impl core::fmt::Display for ConsoleError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnknownVerb => write!(formatter, "unknown console verb"),
            Self::MissingArg => write!(formatter, "console argument missing"),
            Self::BadValue => write!(formatter, "bad console value"),
        }
    }
}

impl std::error::Error for ConsoleError {}

/// Parsed console command borrowing the input line.
///
/// Reads (`Get`, `Watch`, `Seed`, `Hash`) are safe running or paused.
/// Writes (`Set`, `Warp`, `Load`, `Replay`) follow the tweak taint
/// rules at execution in shell wiring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command<'a> {
    /// Read one registry value by dotted-path name.
    Get(&'a str),
    /// Read one registry value on every draw (sticky read).
    Watch(&'a str),
    /// Show the master seed plus domain streams.
    Seed,
    /// Show the current per-tick hash.
    Hash,
    /// Stage a registry write with raw value text.
    Set(&'a str, &'a str),
    /// Request a warp factor by code text.
    Warp(&'a str),
    /// Open the bundle-load modal for a path.
    Load(&'a str),
    /// Open the replay modal for a seed plus log source.
    Replay(&'a str),
}

impl Command<'_> {
    /// Report whether the command reads without tainting.
    #[must_use]
    pub const fn is_safe_read(self) -> bool {
        matches!(
            self,
            Self::Get(_) | Self::Watch(_) | Self::Seed | Self::Hash
        )
    }
}

/// Built-in verb names for completion.
pub const VERBS: [&str; 8] = [
    "get", "watch", "seed", "hash", "set", "warp", "load", "replay",
];

/// Parse one input line into a command.
///
/// Splits on whitespace; empty lines are missing arguments.
///
/// # Errors
///
/// Returns [`ConsoleError`] for unknown verbs, missing arguments, or
/// empty lines.
pub fn parse_command(line: &str) -> Result<Command<'_>, ConsoleError> {
    let mut words = line.split_whitespace();
    let Some(verb) = words.next() else {
        return Err(ConsoleError::MissingArg);
    };
    match verb {
        "get" => {
            let Some(name) = words.next() else {
                return Err(ConsoleError::MissingArg);
            };
            Ok(Command::Get(name))
        }
        "watch" => {
            let Some(name) = words.next() else {
                return Err(ConsoleError::MissingArg);
            };
            Ok(Command::Watch(name))
        }
        "seed" => Ok(Command::Seed),
        "hash" => Ok(Command::Hash),
        "set" => {
            let Some(name) = words.next() else {
                return Err(ConsoleError::MissingArg);
            };
            let Some(value) = words.next() else {
                return Err(ConsoleError::MissingArg);
            };
            Ok(Command::Set(name, value))
        }
        "warp" => {
            let Some(code) = words.next() else {
                return Err(ConsoleError::MissingArg);
            };
            Ok(Command::Warp(code))
        }
        "load" => {
            let Some(path) = words.next() else {
                return Err(ConsoleError::MissingArg);
            };
            Ok(Command::Load(path))
        }
        "replay" => {
            let Some(source) = words.next() else {
                return Err(ConsoleError::MissingArg);
            };
            Ok(Command::Replay(source))
        }
        _ => Err(ConsoleError::UnknownVerb),
    }
}

/// Parse value text for a registry kind into bits.
///
/// # Errors
///
/// Returns [`ConsoleError::BadValue`] for unparseable or non-finite
/// floats, bad integers, or unknown booleans.
pub fn parse_value_bits(text: &str, kind: TweakKind) -> Result<u64, ConsoleError> {
    match kind {
        TweakKind::F64 => {
            let Ok(value_f64) = text.parse::<f64>() else {
                return Err(ConsoleError::BadValue);
            };
            if !value_f64.is_finite() {
                return Err(ConsoleError::BadValue);
            }
            Ok(value_f64.to_bits())
        }
        TweakKind::I64 => {
            let Ok(value_i64) = text.parse::<i64>() else {
                return Err(ConsoleError::BadValue);
            };
            Ok(value_i64.cast_unsigned())
        }
        TweakKind::Bool => match text {
            "true" | "1" => Ok(1_u64),
            "false" | "0" => Ok(0_u64),
            _ => Err(ConsoleError::BadValue),
        },
        TweakKind::Str | TweakKind::Enum => Err(ConsoleError::BadValue),
    }
}

/// Complete a prefix against verbs plus registry names.
///
/// Returns the first match without allocating; callers accept on tap
/// or Tab. Empty prefix completes nothing.
pub fn complete_prefix(prefix: &str) -> Option<&'static str> {
    if prefix.is_empty() {
        return None;
    }
    for verb in VERBS {
        if verb.starts_with(prefix) {
            return Some(verb);
        }
    }
    for entry in REGISTRY {
        if entry.name.starts_with(prefix) {
            return Some(entry.name);
        }
    }
    None
}

/// One console output line with a taint marker bit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputLine {
    /// Rendered text, truncated to the line cap.
    text: String,
    /// True for tainting command echoes.
    tainted_bool: bool,
}

impl OutputLine {
    /// Return the text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Report whether the line echoes a tainting command.
    #[must_use]
    pub const fn is_tainted(&self) -> bool {
        self.tainted_bool
    }
}

/// Console with input buffer, history ring, and output scroll.
///
/// The input buffer reuses one 128-byte capacity; history stores
/// fixed 128-byte arrays; output truncates to the line cap and drops
/// oldest past 128 lines. Shell state only and never persists.
#[derive(Debug, Clone)]
pub struct Console {
    /// Reused input buffer with 128-byte capacity.
    input_line: String,
    /// History lines as fixed arrays plus lengths.
    history: Vec<([u8; CONSOLE_LINE_CAP_BYTES_USIZE], usize)>,
    /// History cursor for Up/Down, dimensionless.
    history_index_usize: usize,
    /// Bounded output scroll, newest last.
    output: Vec<OutputLine>,
}

impl Console {
    /// Build an empty console with pre-sized buffers.
    ///
    /// Reserves history capacity and the input buffer once; the output
    /// scroll grows to its cap only on command echoes.
    #[must_use]
    pub fn new() -> Self {
        Self {
            input_line: String::with_capacity(CONSOLE_LINE_CAP_BYTES_USIZE),
            history: Vec::with_capacity(CONSOLE_HISTORY_ENTRIES_USIZE),
            history_index_usize: 0,
            output: Vec::new(),
        }
    }

    /// Return the input buffer for the input line widget.
    pub fn input_line_mut(&mut self) -> &mut String {
        &mut self.input_line
    }

    /// Clear the input buffer after running a command.
    pub fn clear_input(&mut self) {
        self.input_line.clear();
    }

    /// Return output lines oldest-first.
    #[must_use]
    pub fn output(&self) -> &[OutputLine] {
        &self.output
    }

    /// Push one history line, truncating at a boundary past the cap.
    pub fn push_history(&mut self, line: &str) {
        let mut bytes_u8 = [0_u8; CONSOLE_LINE_CAP_BYTES_USIZE];
        let mut len_usize = line.len().min(CONSOLE_LINE_CAP_BYTES_USIZE);
        while !line.is_char_boundary(len_usize) {
            len_usize -= 1;
        }
        bytes_u8[..len_usize].copy_from_slice(&line.as_bytes()[..len_usize]);
        if self.history.len() >= CONSOLE_HISTORY_ENTRIES_USIZE {
            self.history.remove(0);
        }
        self.history.push((bytes_u8, len_usize));
        self.history_index_usize = self.history.len();
    }

    /// Return history line count.
    #[must_use]
    pub fn history_len_usize(&self) -> usize {
        self.history.len()
    }

    /// Step the history cursor toward older lines.
    pub fn history_older(&mut self) -> Option<&str> {
        if self.history.is_empty() {
            return None;
        }
        if self.history_index_usize > 0 {
            self.history_index_usize -= 1;
        }
        let (bytes_u8, len_usize) = &self.history[self.history_index_usize];
        core::str::from_utf8(&bytes_u8[..*len_usize]).ok()
    }

    /// Step the history cursor toward newer lines.
    pub fn history_newer(&mut self) -> Option<&str> {
        if self.history.is_empty() {
            return None;
        }
        if self.history_index_usize + 1 < self.history.len() {
            self.history_index_usize += 1;
            let (bytes_u8, len_usize) = &self.history[self.history_index_usize];
            core::str::from_utf8(&bytes_u8[..*len_usize]).ok()
        } else {
            self.history_index_usize = self.history.len();
            Some("")
        }
    }

    /// Echo one output line with truncation plus oldest-drop past cap.
    pub fn echo(&mut self, text: &str, tainted_bool: bool) {
        let mut end_usize = text.len().min(CONSOLE_LINE_CAP_BYTES_USIZE * 2);
        while !text.is_char_boundary(end_usize) {
            end_usize -= 1;
        }
        if self.output.len() >= CONSOLE_OUTPUT_CAP_LINES_USIZE {
            self.output.remove(0);
        }
        self.output.push(OutputLine {
            text: String::from(&text[..end_usize]),
            tainted_bool,
        });
    }

    /// Draw output scroll, completion, and input line in one tab.
    ///
    /// Immediate-mode widgets only; creates no renderer. The input
    /// buffer reuses its capacity across frames. Available only with
    /// the non-default `dev-shell` feature.
    #[cfg(feature = "dev-shell")]
    pub fn draw(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            for line in &self.output {
                if line.is_tainted() {
                    ui.label(format!("! {text}", text = line.text()));
                } else {
                    ui.label(line.text());
                }
            }
        });
        let completion_opt = complete_prefix(self.input_line.trim());
        if let Some(completion) = completion_opt
            && ui.button(format!("complete: {completion}")).clicked()
        {
            self.input_line.clear();
            self.input_line.push_str(completion);
            self.input_line.push(' ');
        }
        ui.horizontal(|ui| {
            ui.label(">");
            ui.text_edit_singleline(&mut self.input_line);
        });
    }
}

impl Default for Console {
    /// Default empty console with pre-sized buffers.
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verbs_parse_with_args() {
        assert!(matches!(
            parse_command("get plots.window_s"),
            Ok(Command::Get("plots.window_s"))
        ));
        assert!(matches!(
            parse_command("watch atmo.density_scale"),
            Ok(Command::Watch(_))
        ));
        assert!(matches!(parse_command("seed"), Ok(Command::Seed)));
        assert!(matches!(parse_command("hash"), Ok(Command::Hash)));
        assert!(matches!(
            parse_command("set plots.window_s 30"),
            Ok(Command::Set(_, _))
        ));
        assert!(matches!(parse_command("warp 2"), Ok(Command::Warp("2"))));
        assert!(matches!(
            parse_command("load bundle/"),
            Ok(Command::Load(_))
        ));
        assert!(matches!(
            parse_command("replay log.csv"),
            Ok(Command::Replay(_))
        ));
        assert!(Command::Get("x").is_safe_read());
        assert!(Command::Seed.is_safe_read());
        assert!(!Command::Set("x", "1").is_safe_read());
        assert!(!Command::Warp("1").is_safe_read());
    }

    #[test]
    fn parse_rejects_bad_lines() {
        assert!(matches!(parse_command(""), Err(ConsoleError::MissingArg)));
        assert!(matches!(
            parse_command("frobnicate x"),
            Err(ConsoleError::UnknownVerb)
        ));
        assert!(matches!(
            parse_command("get"),
            Err(ConsoleError::MissingArg)
        ));
        assert!(matches!(
            parse_command("set plots.window_s"),
            Err(ConsoleError::MissingArg)
        ));
    }

    #[test]
    fn values_parse_per_kind() {
        let Ok(bits) = parse_value_bits("1.5", TweakKind::F64) else {
            panic!("f64 must parse")
        };
        assert_eq!(bits, 1.5_f64.to_bits());
        assert!(matches!(
            parse_value_bits("nan", TweakKind::F64),
            Err(ConsoleError::BadValue)
        ));
        assert!(matches!(
            parse_value_bits("abc", TweakKind::F64),
            Err(ConsoleError::BadValue)
        ));
        let Ok(int_bits) = parse_value_bits("-7", TweakKind::I64) else {
            panic!("i64 must parse")
        };
        assert_eq!(int_bits, (-7_i64).cast_unsigned());
        let Ok(true_bits) = parse_value_bits("true", TweakKind::Bool) else {
            panic!("bool must parse")
        };
        assert_eq!(true_bits, 1_u64);
        assert!(matches!(
            parse_value_bits("maybe", TweakKind::Bool),
            Err(ConsoleError::BadValue)
        ));
        assert!(matches!(
            parse_value_bits("x", TweakKind::Str),
            Err(ConsoleError::BadValue)
        ));
    }

    #[test]
    fn completion_matches_verbs_and_names() {
        assert_eq!(complete_prefix(""), None);
        assert_eq!(complete_prefix("ge"), Some("get"));
        assert_eq!(complete_prefix("wa"), Some("watch"));
        assert_eq!(complete_prefix("war"), Some("warp"));
        assert_eq!(complete_prefix("plots."), Some("plots.window_s"));
        assert_eq!(complete_prefix("atmo."), Some("atmo.density_scale"));
        assert_eq!(complete_prefix("zzz"), None);
    }

    #[test]
    fn history_navigates_and_caps() {
        let mut console = Console::new();
        assert_eq!(console.history_len_usize(), 0);
        assert!(console.history_older().is_none());
        console.push_history("get seed");
        console.push_history("hash");
        assert_eq!(console.history_len_usize(), 2);
        assert_eq!(console.history_older(), Some("hash"));
        assert_eq!(console.history_older(), Some("get seed"));
        assert_eq!(console.history_older(), Some("get seed"));
        assert_eq!(console.history_newer(), Some("hash"));
        assert_eq!(console.history_newer(), Some(""));
        for _ in 0..(CONSOLE_HISTORY_ENTRIES_USIZE + 5) {
            console.push_history("warp 1");
        }
        assert_eq!(console.history_len_usize(), CONSOLE_HISTORY_ENTRIES_USIZE);
    }

    #[test]
    fn output_truncates_and_drops_oldest() {
        let mut console = Console::new();
        console.echo("clean line", false);
        console.echo("tainted line", true);
        assert_eq!(console.output().len(), 2);
        assert!(!console.output()[0].is_tainted());
        assert!(console.output()[1].is_tainted());
        for _ in 0..(CONSOLE_OUTPUT_CAP_LINES_USIZE + 3) {
            console.echo("filler", false);
        }
        assert_eq!(console.output().len(), CONSOLE_OUTPUT_CAP_LINES_USIZE);
        console.clear_input();
    }
}
