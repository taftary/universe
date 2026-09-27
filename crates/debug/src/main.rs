//! Dev-only debug shell entry point. Never ships in release builds: the
//! binary requires the non-default `dev-shell` feature, and `debug` is not a
//! default workspace member.

#![forbid(unsafe_code)]

mod layout;
mod shell_cost;
mod theme;
mod top_bar;

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

/// Smoke elapsed sim time in seconds.
const SMOKE_ELAPSED_S_F64: f64 = 0.6;

/// Smoke frame time in milliseconds.
const SMOKE_FRAME_MS_F64: f64 = 8.0;

/// Smoke frame budget denominator in milliseconds.
const SMOKE_FRAME_BUDGET_MS_F64: f64 = 32.0;

/// Smoke master seed, dimensionless.
const SMOKE_SEED_U64: u64 = 0x1234_ABCD_5678_EF90;

/// Smoke snapshot hash, dimensionless.
const SMOKE_HASH_U64: u64 = 0xDEAD_BEEF_0000_4321;

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
    print_top_bar_smoke();
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

/// Print top-bar run control, clocks, badges, and shell cost.
fn print_top_bar_smoke() {
    use engine::warp::{Warp, WarpContext};

    let mut bar = top_bar::TopBarState::new();
    bar.pause();
    bar.request_step();
    let paused = bar.is_paused();
    let pending = bar.take_step();
    match bar.set_clocks(SMOKE_TICK_COUNT_U64, SMOKE_ELAPSED_S_F64) {
        Ok(()) => println!(
            "top_bar clocks tick={tick} elapsed_s={elapsed} step_s={step} paused={paused} step={pending}",
            tick = bar.tick_count_u64(),
            elapsed = bar.elapsed_s_f64(),
            step = top_bar::TopBarState::tick_step_s_f64()
        ),
        Err(error) => println!("top_bar_clocks_error={error}"),
    }
    match bar.request_warp(Warp::X100, WarpContext::cruise()) {
        Ok(warp) => println!(
            "top_bar warp requested={factor}x drop={drop}",
            factor = warp.factor(),
            drop = bar.auto_drop_label()
        ),
        Err(error) => println!("top_bar_warp_error={error}"),
    }
    match bar.request_warp(Warp::X10, WarpContext::on_foot()) {
        Ok(warp) => println!("top_bar_warp_unexpected={factor}x", factor = warp.factor()),
        Err(error) => println!("top_bar_warp_deny={error}"),
    }
    let entry_ctx = WarpContext::new(true, true, true, false, false);
    println!(
        "top_bar entry_auto_drop={drop}",
        drop = entry_ctx.should_auto_drop()
    );
    match bar.observe_snapshot_view(
        SMOKE_TICK_COUNT_U64,
        SMOKE_ELAPSED_S_F64,
        top_bar::TOP_BAR_WARP_CODE_X1_U8,
        top_bar::TOP_BAR_DROP_ENTRY_U8,
        SMOKE_SEED_U64,
        SMOKE_HASH_U64,
    ) {
        Ok(()) => println!(
            "top_bar snapshot warp={warp}x drop={drop} health={health} seed={seed:04x} hash={hash:04x}",
            warp = bar.effective_factor_f64(),
            drop = bar.auto_drop_label(),
            health = bar.health_label(),
            seed = bar.seed_short_u16(),
            hash = bar.hash_short_u16()
        ),
        Err(error) => println!("top_bar_snapshot_error={error}"),
    }
    match bar.set_frame(SMOKE_FRAME_MS_F64) {
        Ok(()) => match bar.frame_fraction_of_budget(SMOKE_FRAME_BUDGET_MS_F64) {
            Ok(fraction_ratio_f64) => println!(
                "top_bar frame ms={ms} fps={fps:.1} fraction={fraction_ratio_f64}",
                ms = bar.frame_ms_f64(),
                fps = bar.frame_fps_f64()
            ),
            Err(error) => println!("top_bar_frame_budget_error={error}"),
        },
        Err(error) => println!("top_bar_frame_error={error}"),
    }
    match bar.set_shell_cost(SMOKE_DRAW_MS_F64) {
        Ok(()) => match bar.shell_fraction_of_budget(SMOKE_FRAME_BUDGET_MS_F64) {
            Ok(fraction_ratio_f64) => println!(
                "top_bar shell draw_ms={ms} fraction={fraction_ratio_f64}",
                ms = bar.shell_draw_ms_f64()
            ),
            Err(error) => println!("top_bar_shell_budget_error={error}"),
        },
        Err(error) => println!("top_bar_shell_error={error}"),
    }
    bar.resume();
    println!("top_bar paused={paused}", paused = bar.is_paused());
    print_top_bar_coverage(&mut bar);
    print_shell_meter_smoke(&mut bar);
}

/// Print shell-meter samples, fractions, close flag, and headless draw.
fn print_shell_meter_smoke(bar: &mut top_bar::TopBarState) {
    let mut meter = shell_cost::ShellCostMeter::new();
    match meter.record_sample(SMOKE_DRAW_MS_F64) {
        Ok(()) => match meter.budget_status(SMOKE_FRAME_BUDGET_MS_F64) {
            Ok(status) => println!(
                "shell_meter draw_ms={ms} avg_ms={avg} fraction={fraction} band={band} closed={closed}",
                ms = meter.latest_draw_ms_f64(),
                avg = meter.average_ms_f64(),
                fraction = status.fraction_ratio_f64(),
                band = status.level().label(),
                closed = meter.is_closed()
            ),
            Err(error) => println!("shell_meter_budget_error={error}"),
        },
        Err(error) => println!("shell_meter_error={error}"),
    }
    match meter.fraction_of_budget(SMOKE_FRAME_BUDGET_MS_F64) {
        Ok(fraction_ratio_f64) => println!(
            "shell_meter direct fraction={fraction_ratio_f64} cap={cap} len={len} empty={empty}",
            cap = shell_cost::ShellCostMeter::capacity_usize(),
            len = meter.len_usize(),
            empty = meter.is_empty()
        ),
        Err(error) => println!("shell_meter_direct_error={error}"),
    }
    let mut closed_meter = shell_cost::ShellCostMeter::default();
    closed_meter.request_close();
    println!(
        "shell_meter closed={closed}",
        closed = closed_meter.is_closed()
    );
    closed_meter.reopen();
    println!(
        "shell_meter reopened={closed}",
        closed = closed_meter.is_closed()
    );
    #[cfg(feature = "dev-shell")]
    {
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            bar.draw(&ctx, ui, &mut meter, SMOKE_FRAME_BUDGET_MS_F64);
        });
        // Headless smoke has no renderer; Step 5 applies texture deltas.
        output.textures_delta.clear();
        println!(
            "top_bar_draw=ok shell_closed={closed} paused={paused}",
            closed = meter.is_closed(),
            paused = bar.is_paused()
        );
    }
}

/// Exercise every remaining top-bar accessor for bin dead-code coverage.
fn print_top_bar_coverage(bar: &mut top_bar::TopBarState) {
    for reason in top_bar::AutoDropReason::ALL {
        println!(
            "auto_drop {label} auto={auto}",
            label = reason.label(),
            auto = reason.is_auto()
        );
    }
    for health in top_bar::HealthStatus::ALL {
        bar.set_health(health);
        println!(
            "health {label} current={current}",
            label = health.label(),
            current = bar.health().label()
        );
    }
    bar.set_determinism(SMOKE_SEED_U64, SMOKE_HASH_U64, true);
    println!(
        "determinism seed={seed} hash={hash} clean={clean} tainted={tainted} pending={pending}",
        seed = bar.seed_u64(),
        hash = bar.snapshot_hash_u64(),
        clean = bar.is_clean(),
        tainted = bar.is_tainted(),
        pending = bar.step_pending()
    );
    bar.mark_tainted();
    println!(
        "warps requested={req}x effective={eff}x req_factor={factor} code={code} reason={reason} health={health}",
        req = bar.requested_warp().factor(),
        eff = bar.effective_warp().factor(),
        factor = bar.requested_factor_f64(),
        code = bar.effective_warp_code_u8(),
        reason = bar.auto_drop_reason().label(),
        health = bar.health_label()
    );
    match bar.frame_budget_status(SMOKE_FRAME_BUDGET_MS_F64) {
        Ok(status) => println!(
            "top_bar frame band={band} percent={percent}",
            band = status.level().label(),
            percent = status.fraction_percent_f64()
        ),
        Err(error) => println!("top_bar_frame_status_error={error}"),
    }
    let default_bar = top_bar::TopBarState::default();
    println!(
        "default_bar tick={tick} warp={warp}x",
        tick = default_bar.tick_count_u64(),
        warp = default_bar.effective_factor_f64()
    );
}
