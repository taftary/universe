//! Continuity monitor with handoff before/after/delta markers.
//!
//! Records the seven `SimSnapshot` readout curves over time in a pre-sized
//! ring, emits one marker per orbit to atmosphere to surface regime change,
//! and draws decimated polylines with vertical handoff lines. Read-only for
//! the sim; the monitor never writes sim state.

#[cfg(feature = "dev-shell")]
use engine::inspect::SimSnapshot;

use crate::layout::PLOT_HISTORY_CAPACITY_ENTRIES_USIZE;

/// Plotted channels in fixed order, dimensionless indices.
pub const CHANNEL_COUNT_USIZE: usize = 7;

/// Altitude channel index, meters.
pub const CHANNEL_ALTITUDE_M_USIZE: usize = 0;

/// Corotating speed channel index, meters per second.
pub const CHANNEL_SPEED_MPS_USIZE: usize = 1;

/// Pressure channel index, pascals.
pub const CHANNEL_PRESSURE_PA_USIZE: usize = 2;

/// Temperature channel index, kelvin.
pub const CHANNEL_TEMPERATURE_K_USIZE: usize = 3;

/// Density channel index, kilograms per cubic meter.
pub const CHANNEL_DENSITY_KG_M3_USIZE: usize = 4;

/// Heating-proxy channel index, watts per square meter.
pub const CHANNEL_HEAT_W_M2_USIZE: usize = 5;

/// G-load channel index, g units.
pub const CHANNEL_G_LOAD_G_USIZE: usize = 6;

/// Relative handoff tolerance, dimensionless.
///
/// Source: issue #36 Step 3 design; flags deltas above float noise only.
pub const DELTA_REL_TOL_F64: f64 = 1e-9;

/// Absolute handoff floors per channel in channel units.
///
/// Near-vacuum zeros at the 120 km taper and rails-zero aero need floors;
/// `-0.0` is normalized before compare as in `snapshot_hash`.
/// Source: issue #36 Step 3 design.
pub const DELTA_ABS_FLOORS_F64: [f64; CHANNEL_COUNT_USIZE] = [
    1e-6,  // altitude_m
    1e-9,  // speed_mps
    1e-12, // pressure_pa
    1e-9,  // temperature_k
    1e-15, // density_kg_m3
    1e-12, // heat_flux_w_per_m2
    1e-12, // g_load_g
];

/// Channel short names with units for labels and tooltips.
///
/// Source: `docs/tech/debug.md` section 4.4 readout list.
pub const CHANNEL_LABELS: [&str; CHANNEL_COUNT_USIZE] = [
    "altitude_m",
    "velocity_m_s",
    "pressure_pa",
    "temperature_k",
    "density_kg_m3",
    "heating_proxy",
    "g_load",
];

/// Channel units for labels and tooltips.
///
/// Source: `docs/tech/debug.md` section 2 units rule.
pub const CHANNEL_UNITS: [&str; CHANNEL_COUNT_USIZE] =
    ["m", "m/s", "Pa", "K", "kg/m3", "W/m2", "g"];

/// Channel source modules for tooltips.
///
/// Names the sim module each readout derives from, per section 4.3.
/// Source: `docs/tech/debug.md` section 4.3 inspector sources.
pub const CHANNEL_SOURCES: [&str; CHANNEL_COUNT_USIZE] = [
    "trajectory",
    "trajectory",
    "atmo",
    "atmo",
    "atmo",
    "trajectory",
    "trajectory",
];

/// Orbit regime code, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `REGIME_ORBIT_U8`.
pub const CONTINUITY_REGIME_ORBIT_U8: u8 = 0;

/// Atmosphere regime code, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `REGIME_ATMOSPHERE_U8`.
pub const CONTINUITY_REGIME_ATMOSPHERE_U8: u8 = 1;

/// Surface regime code, dimensionless.
///
/// Source: `crates/engine/src/inspect.rs` `REGIME_SURFACE_U8`.
pub const CONTINUITY_REGIME_SURFACE_U8: u8 = 2;

/// Stored handoff markers cap, dimensionless.
///
/// Handoffs are rare (two per descent plus ascent); 32 keeps years of runs.
/// Source: issue #36 Step 3 design.
pub const MARKER_CAPACITY_ENTRIES_USIZE: usize = 32;

/// Default plot window in seconds.
///
/// Source: issue #36 Step 3 design; registry placeholder `plots.window_s`.
pub const PLOT_WINDOW_DEFAULT_S_F64: f64 = 60.0;

/// Minimum plot window in seconds.
///
/// Source: issue #36 Step 3 design.
pub const PLOT_WINDOW_MIN_S_F64: f64 = 5.0;

/// Maximum plot window in seconds.
///
/// Source: 2048 entries at 30 fps headless push rate, issue #36 Step 3.
pub const PLOT_WINDOW_MAX_S_F64: f64 = 68.0;

/// Minimum plot strip height in points.
///
/// Source: `docs/tech/debug.md` section 6.2 via layout `PLOT_MIN_HEIGHT_PT_F32`.
pub const PLOT_STRIP_HEIGHT_PT_F32: f32 = 96.0;

/// Continuity range-check failures.
///
/// Returned for non-finite samples, negative clocks, unknown regimes, and
/// out-of-range windows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ContinuityError {
    /// Sample or window value was non-finite.
    NonFinite {
        /// Rejected value.
        value_f64: f64,
    },
    /// Clock or window value was negative.
    Negative {
        /// Rejected value.
        value_f64: f64,
    },
    /// Regime code was outside `0` to `2`.
    InvalidRegime {
        /// Rejected code, dimensionless.
        code_u8: u8,
    },
    /// Window was outside `5.0` to `68.0` seconds.
    WindowOutOfRange {
        /// Rejected window in seconds.
        window_s_f64: f64,
    },
}

impl core::fmt::Display for ContinuityError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NonFinite { value_f64 } => {
                write!(formatter, "non-finite continuity value: {value_f64}")
            }
            Self::Negative { value_f64 } => {
                write!(formatter, "negative continuity value: {value_f64}")
            }
            Self::InvalidRegime { code_u8 } => {
                write!(formatter, "invalid regime code: {code_u8}")
            }
            Self::WindowOutOfRange { window_s_f64 } => {
                write!(formatter, "window out of range: {window_s_f64} s")
            }
        }
    }
}

impl std::error::Error for ContinuityError {}

/// One recorded sample with seven channels.
///
/// Plain data only; channel order follows `CHANNEL_*` indices.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlotSample {
    /// Tick count, dimensionless.
    tick_count_u64: u64,
    /// Elapsed sim time in seconds.
    elapsed_s_f64: f64,
    /// Channels in `CHANNEL_*` order with channel units.
    channels_f64: [f64; CHANNEL_COUNT_USIZE],
    /// Regime code, dimensionless `0` to `2`.
    regime_u8: u8,
}

impl PlotSample {
    /// Build a sample from tick, clock, channels, and regime.
    ///
    /// # Errors
    ///
    /// Returns [`ContinuityError`] for non-finite or negative clocks,
    /// non-finite channels, or a regime code above `2`.
    pub fn new(
        tick_count_u64: u64,
        elapsed_s_f64: f64,
        channels_f64: [f64; CHANNEL_COUNT_USIZE],
        regime_u8: u8,
    ) -> Result<Self, ContinuityError> {
        if !elapsed_s_f64.is_finite() {
            return Err(ContinuityError::NonFinite {
                value_f64: elapsed_s_f64,
            });
        }
        if elapsed_s_f64 < 0.0 {
            return Err(ContinuityError::Negative {
                value_f64: elapsed_s_f64,
            });
        }
        for value_f64 in channels_f64 {
            if !value_f64.is_finite() {
                return Err(ContinuityError::NonFinite { value_f64 });
            }
        }
        if regime_u8 > CONTINUITY_REGIME_SURFACE_U8 {
            return Err(ContinuityError::InvalidRegime { code_u8: regime_u8 });
        }
        Ok(Self {
            tick_count_u64,
            elapsed_s_f64,
            channels_f64: normalize_channels_f64(channels_f64),
            regime_u8,
        })
    }

    /// Return the tick count, dimensionless.
    #[must_use]
    pub const fn tick_count_u64(self) -> u64 {
        self.tick_count_u64
    }

    /// Return elapsed sim time in seconds.
    #[must_use]
    pub const fn elapsed_s_f64(self) -> f64 {
        self.elapsed_s_f64
    }

    /// Return the channels in `CHANNEL_*` order.
    #[must_use]
    pub const fn channels_f64(self) -> [f64; CHANNEL_COUNT_USIZE] {
        self.channels_f64
    }

    /// Return the regime code, dimensionless.
    #[must_use]
    pub const fn regime_u8(self) -> u8 {
        self.regime_u8
    }
}

/// One handoff marker between consecutive regimes.
///
/// Display classification only; carries before, after, and delta triples
/// with a red-flag bit for deltas above float noise.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HandoffMarker {
    /// Tick count at the after sample, dimensionless.
    tick_count_u64: u64,
    /// Elapsed sim time at the after sample in seconds.
    elapsed_s_f64: f64,
    /// Before values in `CHANNEL_*` order.
    before_f64: [f64; CHANNEL_COUNT_USIZE],
    /// After values in `CHANNEL_*` order.
    after_f64: [f64; CHANNEL_COUNT_USIZE],
    /// After-minus-before deltas in `CHANNEL_*` order.
    delta_f64: [f64; CHANNEL_COUNT_USIZE],
    /// True when any channel delta exceeds float noise.
    flagged_bool: bool,
}

impl HandoffMarker {
    /// Return the tick count, dimensionless.
    #[must_use]
    pub const fn tick_count_u64(self) -> u64 {
        self.tick_count_u64
    }

    /// Return elapsed sim time in seconds.
    #[must_use]
    pub const fn elapsed_s_f64(self) -> f64 {
        self.elapsed_s_f64
    }

    /// Return before values in channel order.
    #[must_use]
    pub const fn before_f64(self) -> [f64; CHANNEL_COUNT_USIZE] {
        self.before_f64
    }

    /// Return after values in channel order.
    #[must_use]
    pub const fn after_f64(self) -> [f64; CHANNEL_COUNT_USIZE] {
        self.after_f64
    }

    /// Return deltas in channel order.
    #[must_use]
    pub const fn delta_f64(self) -> [f64; CHANNEL_COUNT_USIZE] {
        self.delta_f64
    }

    /// Report whether the marker is red-flagged.
    #[must_use]
    pub const fn is_flagged(self) -> bool {
        self.flagged_bool
    }
}

/// Continuity monitor with a pre-sized sample ring and marker list.
///
/// Buffers allocate once at open and reuse after warmup; monitor state
/// never persists. Push once per frame from `Shell::observe_snapshot`,
/// not per sim sub-tick under warp.
#[derive(Debug, Clone)]
pub struct ContinuityMonitor {
    /// Pre-sized sample ring, oldest overwritten once full.
    samples: Vec<PlotSample>,
    /// Next write index into the ring, dimensionless.
    next_index_usize: usize,
    /// Stored handoff markers, oldest dropped past capacity.
    markers: Vec<HandoffMarker>,
    /// Display window in seconds.
    window_s_f64: f64,
}

impl ContinuityMonitor {
    /// Build an empty monitor with pre-sized buffers.
    ///
    /// Reserves the layout plot-history capacity plus the marker cap;
    /// no allocation happens after warmup.
    #[must_use]
    pub fn new() -> Self {
        Self {
            samples: Vec::with_capacity(PLOT_HISTORY_CAPACITY_ENTRIES_USIZE),
            next_index_usize: 0,
            markers: Vec::with_capacity(MARKER_CAPACITY_ENTRIES_USIZE),
            window_s_f64: PLOT_WINDOW_DEFAULT_S_F64,
        }
    }

    /// Return filled sample entries.
    #[must_use]
    pub fn len_usize(&self) -> usize {
        self.samples.len()
    }

    /// Report whether no samples are stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Return stored handoff markers.
    #[must_use]
    pub fn markers(&self) -> &[HandoffMarker] {
        &self.markers
    }

    /// Return the display window in seconds.
    #[must_use]
    pub const fn window_s_f64(&self) -> f64 {
        self.window_s_f64
    }

    /// Set the display window in seconds.
    ///
    /// Display crop only; history keeps recording.
    ///
    /// # Errors
    ///
    /// Returns [`ContinuityError`] for a non-finite window or one outside
    /// `5.0` to `68.0` seconds.
    pub fn set_window_s_f64(&mut self, window_s_f64: f64) -> Result<(), ContinuityError> {
        if !window_s_f64.is_finite() {
            return Err(ContinuityError::NonFinite {
                value_f64: window_s_f64,
            });
        }
        if !(PLOT_WINDOW_MIN_S_F64..=PLOT_WINDOW_MAX_S_F64).contains(&window_s_f64) {
            return Err(ContinuityError::WindowOutOfRange { window_s_f64 });
        }
        self.window_s_f64 = window_s_f64;
        Ok(())
    }

    /// Clear samples plus markers, keeping reservations for reuse.
    ///
    /// No allocation; called on fresh runs outside the frame loop.
    pub fn clear(&mut self) {
        self.samples.clear();
        self.markers.clear();
        self.next_index_usize = 0;
    }

    /// Push one sample, emitting a marker on regime change.
    ///
    /// Overwrites the oldest entry once full; no allocation after open.
    pub fn push_sample(&mut self, sample: PlotSample) {
        if let Some(previous) = self.latest()
            && previous.regime_u8() != sample.regime_u8()
        {
            self.push_marker(previous, sample);
        }
        if self.samples.len() < PLOT_HISTORY_CAPACITY_ENTRIES_USIZE {
            self.samples.push(sample);
        } else {
            self.samples[self.next_index_usize] = sample;
            self.next_index_usize =
                (self.next_index_usize + 1) % PLOT_HISTORY_CAPACITY_ENTRIES_USIZE;
        }
    }

    /// Push one snapshot by copy without writing sim state.
    ///
    /// Copies the seven readout curves plus clocks and regime from the
    /// snapshot; available only with the non-default `dev-shell` feature.
    ///
    /// # Errors
    ///
    /// Returns [`ContinuityError`] for bad clocks, non-finite channels,
    /// or an unknown regime code.
    #[cfg(feature = "dev-shell")]
    pub fn push_snapshot(&mut self, snapshot: &SimSnapshot) -> Result<(), ContinuityError> {
        let sample = PlotSample::new(
            snapshot.tick_count_u64,
            snapshot.elapsed_s_f64,
            [
                snapshot.altitude_m_f64,
                snapshot.speed_mps_f64,
                snapshot.pressure_pa_f64,
                snapshot.temperature_k_f64,
                snapshot.density_kg_m3_f64,
                snapshot.heat_flux_w_per_m2_f64,
                snapshot.g_load_g_f64,
            ],
            snapshot.regime_u8,
        )?;
        self.push_sample(sample);
        Ok(())
    }

    /// Return the latest sample when one exists.
    #[must_use]
    pub fn latest(&self) -> Option<PlotSample> {
        self.ordered().last().copied()
    }

    /// Iterate samples oldest-first without allocating.
    ///
    /// Splits the ring at the write index so a full ring reads oldest
    /// first and a filling ring reads insertion order; draw-time only.
    fn ordered(&self) -> impl DoubleEndedIterator<Item = &PlotSample> + '_ {
        let (head, tail) = self.samples.split_at(self.next_index_usize);
        tail.iter().chain(head.iter())
    }

    /// Draw readout curves with handoff markers in one bottom tab.
    ///
    /// Immediate-mode widgets only; creates no renderer. Decimates each
    /// curve to screen pixels; off-screen data keeps recording but skips
    /// draw. Available only with the non-default `dev-shell` feature.
    #[cfg(feature = "dev-shell")]
    pub fn draw(&self, ui: &mut egui::Ui) {
        if self.samples.is_empty() {
            ui.label("continuity: no samples yet");
            return;
        }
        let latest_elapsed_s_f64 = self
            .ordered()
            .last()
            .map_or(0.0, |sample| sample.elapsed_s_f64());
        let earliest_elapsed_s_f64 = latest_elapsed_s_f64 - self.window_s_f64;
        for (channel_usize, name) in CHANNEL_LABELS.iter().enumerate() {
            draw_channel_strip(
                ui,
                self,
                channel_usize,
                name,
                earliest_elapsed_s_f64,
                latest_elapsed_s_f64,
            );
        }
        ui.label(format!(
            "handoff markers={count} flagged={flagged}",
            count = self.markers.len(),
            flagged = self
                .markers
                .iter()
                .filter(|marker| marker.is_flagged())
                .count()
        ));
        for marker in &self.markers {
            ui.label(format!(
                "handoff tick={tick} elapsed_s={elapsed:.2} alt_m {before:.3}->{after:.3} delta={delta:.3}{flag}",
                tick = marker.tick_count_u64(),
                elapsed = marker.elapsed_s_f64(),
                before = marker.before_f64()[CHANNEL_ALTITUDE_M_USIZE],
                after = marker.after_f64()[CHANNEL_ALTITUDE_M_USIZE],
                delta = marker.delta_f64()[CHANNEL_ALTITUDE_M_USIZE],
                flag = if marker.is_flagged() {
                    " [flagged]"
                } else {
                    ""
                }
            ));
        }
    }

    /// Push a handoff marker, dropping the oldest past capacity.
    fn push_marker(&mut self, before: PlotSample, after: PlotSample) {
        let before_f64 = before.channels_f64();
        let after_f64 = after.channels_f64();
        let mut delta_f64 = [0.0; CHANNEL_COUNT_USIZE];
        let mut flagged_bool = false;
        for channel_usize in 0..CHANNEL_COUNT_USIZE {
            let delta = after_f64[channel_usize] - before_f64[channel_usize];
            delta_f64[channel_usize] = if delta == 0.0 { 0.0 } else { delta };
            let scale_f64 = before_f64[channel_usize]
                .abs()
                .max(after_f64[channel_usize].abs())
                .max(DELTA_ABS_FLOORS_F64[channel_usize]);
            if delta.abs() > DELTA_REL_TOL_F64 * scale_f64
                && delta.abs() > DELTA_ABS_FLOORS_F64[channel_usize]
            {
                flagged_bool = true;
            }
        }
        let marker = HandoffMarker {
            tick_count_u64: after.tick_count_u64(),
            elapsed_s_f64: after.elapsed_s_f64(),
            before_f64,
            after_f64,
            delta_f64,
            flagged_bool,
        };
        if self.markers.len() >= MARKER_CAPACITY_ENTRIES_USIZE {
            self.markers.remove(0);
        }
        self.markers.push(marker);
    }
}

impl Default for ContinuityMonitor {
    /// Default monitor with pre-sized buffers.
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a zero-to-one ratio to `f32` for egui points.
///
/// Callers pass time and value fractions already clamped to unit range by
/// construction; the cast cannot overflow the point space.
#[cfg(feature = "dev-shell")]
#[expect(
    clippy::cast_possible_truncation,
    reason = "unit ratios fit f32 point space"
)]
fn unit_ratio_to_f32(ratio_f64: f64) -> f32 {
    ratio_f64 as f32
}

/// Draw one channel strip with its handoff lines.
///
/// Decimates the curve to screen pixels and skips samples outside the
/// window. Iterates the monitor ring twice and allocates nothing.
/// Available only with the non-default `dev-shell` feature.
#[cfg(feature = "dev-shell")]
fn draw_channel_strip(
    ui: &mut egui::Ui,
    monitor: &ContinuityMonitor,
    channel_usize: usize,
    name: &str,
    earliest_elapsed_s_f64: f64,
    latest_elapsed_s_f64: f64,
) {
    use crate::theme::{BASE_TEXT_RGB_U8, BUDGET_OVER_RGB_U8};
    let latest_value_f64 = monitor
        .ordered()
        .last()
        .map_or(0.0, |sample| sample.channels_f64()[channel_usize]);
    let latest_tick_u64 = monitor
        .ordered()
        .last()
        .map_or(0, |sample| sample.tick_count_u64());
    ui.label(format!(
        "{name} latest={latest_value_f64:.6} tick={latest_tick_u64}"
    ))
    .on_hover_text(format!(
        "{name} in {unit}; source {module}; tick {tick}",
        unit = CHANNEL_UNITS[channel_usize],
        module = CHANNEL_SOURCES[channel_usize],
        tick = latest_tick_u64
    ));
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), PLOT_STRIP_HEIGHT_PT_F32),
        egui::Sense::hover(),
    );
    let painter = ui.painter();
    let text_color = egui::Color32::from_rgb(
        BASE_TEXT_RGB_U8.red_u8,
        BASE_TEXT_RGB_U8.green_u8,
        BASE_TEXT_RGB_U8.blue_u8,
    );
    let flag_color = egui::Color32::from_rgb(
        BUDGET_OVER_RGB_U8.red_u8,
        BUDGET_OVER_RGB_U8.green_u8,
        BUDGET_OVER_RGB_U8.blue_u8,
    );
    let (min_f64, max_f64) = channel_range_f64(monitor.ordered(), channel_usize);
    let span_f64 = (max_f64 - min_f64).max(1e-300);
    let time_span_f64 = (latest_elapsed_s_f64 - earliest_elapsed_s_f64).max(1e-9);
    let mut previous_point: Option<egui::Pos2> = None;
    let mut last_drawn_x_f32 = f32::NEG_INFINITY;
    for sample in monitor.ordered() {
        if sample.elapsed_s_f64() < earliest_elapsed_s_f64 {
            continue;
        }
        let x_ratio_f32 =
            unit_ratio_to_f32((sample.elapsed_s_f64() - earliest_elapsed_s_f64) / time_span_f64);
        if x_ratio_f32 - last_drawn_x_f32 < 1.0 / rect.width().max(1.0) {
            continue;
        }
        last_drawn_x_f32 = x_ratio_f32;
        let value_f64 = sample.channels_f64()[channel_usize];
        let y_ratio_f32 = unit_ratio_to_f32((value_f64 - min_f64) / span_f64);
        let point = egui::Pos2::new(
            rect.min.x + x_ratio_f32 * rect.width(),
            rect.max.y - y_ratio_f32 * rect.height(),
        );
        if let Some(previous) = previous_point {
            painter.line_segment([previous, point], egui::Stroke::new(1.0, text_color));
        }
        previous_point = Some(point);
    }
    for marker in monitor.markers() {
        if marker.elapsed_s_f64() < earliest_elapsed_s_f64
            || marker.elapsed_s_f64() > latest_elapsed_s_f64
        {
            continue;
        }
        let x_ratio_f32 =
            unit_ratio_to_f32((marker.elapsed_s_f64() - earliest_elapsed_s_f64) / time_span_f64);
        let x_pos_f32 = rect.min.x + x_ratio_f32 * rect.width();
        let color = if marker.is_flagged() {
            flag_color
        } else {
            text_color
        };
        painter.line_segment(
            [
                egui::Pos2::new(x_pos_f32, rect.min.y),
                egui::Pos2::new(x_pos_f32, rect.max.y),
            ],
            egui::Stroke::new(1.0, color),
        );
    }
}

/// Normalize `-0.0` to `+0.0` per the snapshot hash policy.
fn normalize_zero_f64(value_f64: f64) -> f64 {
    if value_f64 == 0.0 { 0.0 } else { value_f64 }
}

/// Normalize every channel of one sample.
fn normalize_channels_f64(channels_f64: [f64; CHANNEL_COUNT_USIZE]) -> [f64; CHANNEL_COUNT_USIZE] {
    let mut normalized_f64 = [0.0; CHANNEL_COUNT_USIZE];
    for channel_usize in 0..CHANNEL_COUNT_USIZE {
        normalized_f64[channel_usize] = normalize_zero_f64(channels_f64[channel_usize]);
    }
    normalized_f64
}

/// Range of one channel over oldest-first samples.
#[cfg(feature = "dev-shell")]
fn channel_range_f64<'a>(
    ordered: impl Iterator<Item = &'a PlotSample>,
    channel_usize: usize,
) -> (f64, f64) {
    let mut min_f64 = f64::INFINITY;
    let mut max_f64 = f64::NEG_INFINITY;
    for sample in ordered {
        let value_f64 = sample.channels_f64()[channel_usize];
        min_f64 = min_f64.min(value_f64);
        max_f64 = max_f64.max(value_f64);
    }
    if !min_f64.is_finite() || !max_f64.is_finite() {
        return (0.0, 1.0);
    }
    if (max_f64 - min_f64).abs() < 1e-300 {
        return (min_f64 - 0.5, max_f64 + 0.5);
    }
    (min_f64, max_f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMOKE_TICK_U64: u64 = 12;
    const SMOKE_ELAPSED_S_F64: f64 = 0.6;
    const SMOKE_CHANNELS_F64: [f64; CHANNEL_COUNT_USIZE] =
        [250_000.0, 3_400.0, 0.0, 210.0, 0.0, 0.0, 0.0];
    const FRACTION_TOL_F64: f64 = 1e-12;

    fn smoke_sample(regime_u8: u8) -> PlotSample {
        let Ok(sample) = PlotSample::new(
            SMOKE_TICK_U64,
            SMOKE_ELAPSED_S_F64,
            SMOKE_CHANNELS_F64,
            regime_u8,
        ) else {
            panic!("smoke sample must build")
        };
        sample
    }

    #[test]
    fn push_records_without_flag_on_same_regime() {
        let mut monitor = ContinuityMonitor::new();
        assert!(monitor.is_empty());
        assert_eq!(monitor.len_usize(), 0);
        monitor.push_sample(smoke_sample(CONTINUITY_REGIME_ORBIT_U8));
        monitor.push_sample(smoke_sample(CONTINUITY_REGIME_ORBIT_U8));
        assert_eq!(monitor.len_usize(), 2);
        assert!(monitor.markers().is_empty());
        let Some(latest) = monitor.latest() else {
            panic!("latest must exist")
        };
        assert_eq!(latest.tick_count_u64(), SMOKE_TICK_U64);
        assert!((latest.elapsed_s_f64() - SMOKE_ELAPSED_S_F64).abs() < FRACTION_TOL_F64);
    }

    #[test]
    fn regime_change_emits_unflagged_marker_on_identical_channels() {
        let mut monitor = ContinuityMonitor::new();
        monitor.push_sample(smoke_sample(CONTINUITY_REGIME_ORBIT_U8));
        monitor.push_sample(smoke_sample(CONTINUITY_REGIME_ATMOSPHERE_U8));
        assert_eq!(monitor.markers().len(), 1);
        let marker = monitor.markers()[0];
        assert!(!marker.is_flagged());
        assert_eq!(marker.tick_count_u64(), SMOKE_TICK_U64);
        for channel_usize in 0..CHANNEL_COUNT_USIZE {
            assert!((marker.delta_f64()[channel_usize]).abs() < FRACTION_TOL_F64);
        }
    }

    #[test]
    fn regime_change_flags_large_delta() {
        let mut monitor = ContinuityMonitor::new();
        monitor.push_sample(smoke_sample(CONTINUITY_REGIME_ORBIT_U8));
        let mut jumped_f64 = SMOKE_CHANNELS_F64;
        jumped_f64[CHANNEL_PRESSURE_PA_USIZE] = 610.0;
        let Ok(after) = PlotSample::new(
            SMOKE_TICK_U64 + 1,
            SMOKE_ELAPSED_S_F64 + 0.05,
            jumped_f64,
            CONTINUITY_REGIME_ATMOSPHERE_U8,
        ) else {
            panic!("jumped sample must build")
        };
        monitor.push_sample(after);
        assert_eq!(monitor.markers().len(), 1);
        assert!(monitor.markers()[0].is_flagged());
    }

    #[test]
    fn tiny_delta_stays_unflagged() {
        let mut monitor = ContinuityMonitor::new();
        monitor.push_sample(smoke_sample(CONTINUITY_REGIME_ATMOSPHERE_U8));
        let mut drifted_f64 = SMOKE_CHANNELS_F64;
        drifted_f64[CHANNEL_ALTITUDE_M_USIZE] += 1e-9;
        let Ok(after) = PlotSample::new(
            SMOKE_TICK_U64 + 1,
            SMOKE_ELAPSED_S_F64 + 0.05,
            drifted_f64,
            CONTINUITY_REGIME_SURFACE_U8,
        ) else {
            panic!("drifted sample must build")
        };
        monitor.push_sample(after);
        assert_eq!(monitor.markers().len(), 1);
        assert!(!monitor.markers()[0].is_flagged());
    }

    #[test]
    fn rejects_bad_samples_and_windows() {
        assert!(matches!(
            PlotSample::new(SMOKE_TICK_U64, f64::NAN, SMOKE_CHANNELS_F64, 0),
            Err(ContinuityError::NonFinite { .. })
        ));
        assert!(matches!(
            PlotSample::new(SMOKE_TICK_U64, -1.0, SMOKE_CHANNELS_F64, 0),
            Err(ContinuityError::Negative { .. })
        ));
        assert!(matches!(
            PlotSample::new(SMOKE_TICK_U64, SMOKE_ELAPSED_S_F64, SMOKE_CHANNELS_F64, 9),
            Err(ContinuityError::InvalidRegime { .. })
        ));
        let mut bad_channels_f64 = SMOKE_CHANNELS_F64;
        bad_channels_f64[0] = f64::INFINITY;
        assert!(matches!(
            PlotSample::new(SMOKE_TICK_U64, SMOKE_ELAPSED_S_F64, bad_channels_f64, 0),
            Err(ContinuityError::NonFinite { .. })
        ));
        let mut monitor = ContinuityMonitor::new();
        assert!(monitor.set_window_s_f64(30.0).is_ok());
        assert!((monitor.window_s_f64() - 30.0).abs() < FRACTION_TOL_F64);
        assert!(matches!(
            monitor.set_window_s_f64(10_000.0),
            Err(ContinuityError::WindowOutOfRange { .. })
        ));
        assert!(matches!(
            monitor.set_window_s_f64(f64::NAN),
            Err(ContinuityError::NonFinite { .. })
        ));
    }

    #[test]
    fn ring_reuses_buffer_at_capacity() {
        let mut monitor = ContinuityMonitor::new();
        for tick_u64 in 0..(PLOT_HISTORY_CAPACITY_ENTRIES_USIZE as u64 + 3) {
            let tick_mod_u64 = tick_u64 % 1_000_000;
            let Ok(tick_mod_u32) = u32::try_from(tick_mod_u64) else {
                panic!("tick mod must fit u32")
            };
            let Ok(sample) = PlotSample::new(
                tick_u64,
                f64::from(tick_mod_u32) * 0.05,
                SMOKE_CHANNELS_F64,
                CONTINUITY_REGIME_ORBIT_U8,
            ) else {
                panic!("ring sample must build")
            };
            monitor.push_sample(sample);
        }
        assert_eq!(monitor.len_usize(), PLOT_HISTORY_CAPACITY_ENTRIES_USIZE);
        let Some(latest) = monitor.latest() else {
            panic!("latest must exist")
        };
        assert_eq!(
            latest.tick_count_u64(),
            PLOT_HISTORY_CAPACITY_ENTRIES_USIZE as u64 + 2
        );
    }

    #[cfg(feature = "dev-shell")]
    #[test]
    fn snapshot_push_copies_seven_curves() {
        use crate::inspect_view::{
            INSPECT_DROP_NONE_U8, INSPECT_FRAME_DEPTH_M1_U8, INSPECT_FRAME_ORBIT_U8,
            INSPECT_MARK_SHIP_POINT_U8, INSPECT_REGIME_ORBIT_U8, INSPECT_WARP_X1_U8,
        };
        let snapshot = SimSnapshot {
            tick_count_u64: SMOKE_TICK_U64,
            elapsed_s_f64: SMOKE_ELAPSED_S_F64,
            ship_epoch_s_f64: SMOKE_ELAPSED_S_F64,
            master_seed_u64: 1,
            stream_seed_u64: 2,
            snapshot_hash_u64: 3,
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
            pick_altitude_m_f64: 0.0,
            pick_range_m_f64: 0.0,
            frame_body_id_u32: 1,
            parent_body_id_u32: 0,
            pick_body_id_u32: u32::MAX,
            pick_cell_x_i32: 0,
            pick_cell_y_i32: 0,
            warp_code_u8: INSPECT_WARP_X1_U8,
            drop_reason_u8: INSPECT_DROP_NONE_U8,
            warp_flags_u8: 3,
            regime_u8: INSPECT_REGIME_ORBIT_U8,
            frame_level_u8: INSPECT_FRAME_ORBIT_U8,
            frame_depth_u8: INSPECT_FRAME_DEPTH_M1_U8,
            elements_valid_u8: 1,
            pick_valid_u8: 0,
            mark_kind_u8: INSPECT_MARK_SHIP_POINT_U8,
            _pad_u8: [0_u8; 3],
        };
        let mut monitor = ContinuityMonitor::new();
        assert!(monitor.push_snapshot(&snapshot).is_ok());
        let Some(latest) = monitor.latest() else {
            panic!("snapshot sample must exist")
        };
        let curves_f64 = latest.channels_f64();
        assert!((curves_f64[CHANNEL_ALTITUDE_M_USIZE] - 250_000.0).abs() < FRACTION_TOL_F64);
        assert!((curves_f64[CHANNEL_SPEED_MPS_USIZE] - 3_400.0).abs() < FRACTION_TOL_F64);
        assert!((curves_f64[CHANNEL_PRESSURE_PA_USIZE] - 0.0).abs() < FRACTION_TOL_F64);
        assert!((curves_f64[CHANNEL_TEMPERATURE_K_USIZE] - 210.0).abs() < FRACTION_TOL_F64);
        assert!((curves_f64[CHANNEL_DENSITY_KG_M3_USIZE] - 0.0).abs() < FRACTION_TOL_F64);
        assert!((curves_f64[CHANNEL_HEAT_W_M2_USIZE] - 0.0).abs() < FRACTION_TOL_F64);
        assert!((curves_f64[CHANNEL_G_LOAD_G_USIZE] - 0.0).abs() < FRACTION_TOL_F64);
    }
}
