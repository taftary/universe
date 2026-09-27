//! Dev-only debug shell entry point. Never ships in release builds: the
//! binary requires the non-default `dev-shell` feature, and `debug` is not a
//! default workspace member.

#![forbid(unsafe_code)]

mod layout;
mod theme;

/// Demo tick count for the headless run.
const DEFAULT_TICK_COUNT: u64 = 4;

/// Smoke fractions spanning nominal, elevated, and over bands.
const SMOKE_FRACTIONS_F64: [f64; 3] = [0.25, 0.6, 0.9];

/// Smoke tick count shown on the dev tag.
const SMOKE_TICK_COUNT_U64: u64 = 42;

/// Smoke frame fraction shown on the dev tag.
const SMOKE_FRAME_FRACTION_F64: f64 = 0.1;

/// Smoke shell draw cost in milliseconds.
const SMOKE_DRAW_MS_F64: f64 = 0.4;

/// Unit draw-cost denominator in milliseconds for the smoke print.
const SMOKE_BUDGET_MS_F64: f64 = 1.0;

#[global_allocator]
static GLOBAL_ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// Run a short headless tick demo and print scheduler state.
///
/// # Errors
///
/// Returns the engine error when the demo step is rejected.
fn main() -> anyhow::Result<()> {
    let span = tracing::span!(tracing::Level::INFO, "debug_ticks");
    let _guard = span.enter();
    let mut scheduler = engine::sim::Scheduler::new(engine::sim::SIM_TICK_S)?;
    for _ in 0..DEFAULT_TICK_COUNT {
        scheduler.advance();
    }
    println!(
        "ticks={} elapsed_s={}",
        scheduler.step_count(),
        scheduler.elapsed().value()
    );
    print_theme_smoke();
    print_layout_smoke();
    Ok(())
}

/// Print the DevDark-Pro proposal with accents, fonts, and bands.
fn print_theme_smoke() {
    let proposal = theme::DevDarkProTheme::proposal();
    println!(
        "theme si_primary={si} tabular={tab} background={bg:?} text={fg:?}",
        si = proposal.si_primary(),
        tab = proposal.tabular_numerals(),
        bg = theme::DevDarkProTheme::background_rgb_u8().to_array_u8(),
        fg = theme::DevDarkProTheme::text_rgb_u8().to_array_u8()
    );
    for accent in theme::ThemeAccent::ALL {
        println!(
            "accent {name} rgb={rgb:?}",
            name = accent.label(),
            rgb = theme::DevDarkProTheme::accent_rgb_u8(accent).to_array_u8()
        );
    }
    for role in theme::FontRole::ALL {
        println!(
            "font {name} monospace={mono} tabular={tab}",
            name = role.label(),
            mono = role.monospace(),
            tab = role.tabular_numerals()
        );
    }
    for sample_ratio_f64 in SMOKE_FRACTIONS_F64 {
        match theme::BudgetStatus::new(sample_ratio_f64) {
            Ok(status) => println!(
                "budget fraction={ratio} percent={percent} band={band} rgb={rgb:?}",
                ratio = status.fraction_ratio_f64(),
                percent = status.fraction_percent_f64(),
                band = status.level().label(),
                rgb = status.color_rgb_u8().to_array_u8()
            ),
            Err(error) => println!("budget_error={error}"),
        }
    }
    match theme::BudgetLevel::classify(f64::NAN) {
        Ok(level) => println!("budget_nan_band={band}", band = level.label()),
        Err(error) => println!("budget_nan_error={error}"),
    }
    #[cfg(feature = "dev-shell")]
    println!(
        "visuals dark_mode={dark}",
        dark = theme::dev_dark_pro_visuals().dark_mode
    );
}

/// Print desktop presets, phone skeleton, dev tag, and shell cost.
fn print_layout_smoke() {
    for preset in layout::DesktopPreset::ALL {
        let visibility = layout::PanelVisibility::for_preset(preset);
        let direct = preset.visibility();
        println!(
            "preset {name} bits={bits} top={top} left={left} right={right} bottom={bottom} direct={same}",
            name = preset.label(),
            bits = visibility.bits_u8(),
            top = visibility.shows_top_bar(),
            left = visibility.shows_left_panel(),
            right = visibility.shows_right_panel(),
            bottom = visibility.shows_bottom_tabs(),
            same = (direct == visibility)
        );
    }
    for region in layout::DockRegion::ALL {
        println!("region {label}", label = region.label());
    }
    for tab in layout::PhoneTab::ALL {
        println!("phone_tab {label}", label = tab.label());
    }
    for chip in layout::ChipAction::ALL {
        println!("chip {label}", label = chip.label());
    }
    for detent in layout::BottomSheetDetent::ALL {
        println!(
            "detent {name} fraction={fraction}",
            name = detent.label(),
            fraction = detent.height_fraction_f64()
        );
    }
    let mode = layout::ShellInputMode::Passthrough.toggle();
    println!("input_mode {label}", label = mode.label());
    match layout::DevTag::new(SMOKE_TICK_COUNT_U64, SMOKE_FRAME_FRACTION_F64) {
        Ok(tag) => println!(
            "dev_tag anchor={anchor} tick={tick} fraction={fraction}",
            anchor = layout::DevTag::anchor(),
            tick = tag.tick_count_u64(),
            fraction = tag.frame_fraction_ratio_f64()
        ),
        Err(error) => println!("dev_tag_error={error}"),
    }
    match layout::ShellCostSample::new(SMOKE_DRAW_MS_F64) {
        Ok(sample) => match sample.fraction_of_budget(SMOKE_BUDGET_MS_F64) {
            Ok(fraction_ratio_f64) => println!(
                "shell draw_ms={ms} fraction={fraction_ratio_f64}",
                ms = sample.draw_ms_f64()
            ),
            Err(error) => println!("shell_budget_error={error}"),
        },
        Err(error) => println!("shell_sample_error={error}"),
    }
    let buffers = layout::BufferPlan::shell_default();
    println!(
        "buffers plot={plot} log={log} recorder={recorder} touch_pt={touch} plot_height_pt={height}",
        plot = buffers.plot_history_entries_usize(),
        log = buffers.log_history_entries_usize(),
        recorder = buffers.input_recorder_entries_usize(),
        touch = layout::MIN_TOUCH_TARGET_PT_F32,
        height = layout::PLOT_MIN_HEIGHT_PT_F32
    );
    #[cfg(feature = "dev-shell")]
    println!(
        "egui_wgpu_renderer={name}",
        name = layout::egui_wgpu_renderer_type_name()
    );
}
