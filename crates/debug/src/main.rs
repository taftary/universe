//! Dev-only debug shell entry point. Never ships in release builds: the
//! binary requires the non-default `dev-shell` feature, and `debug` is not a
//! default workspace member.

#![forbid(unsafe_code)]

mod bottom;
mod budget;
mod console;
mod continuity;
mod determinism;
mod export;
mod input;
mod inspect_view;
mod layout;
mod log;
#[cfg(feature = "dev-shell")]
mod os_window;
mod shell;
mod shell_cost;
mod theme;
mod top_bar;
mod tweak;

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

/// Smoke sim-tick average denominator in milliseconds.
///
/// Smoke value only; the gate lives in `docs/tech/quality.md`.
const SMOKE_SIM_AVG_BUDGET_MS_F64: f64 = 8.0;

/// Smoke sim-tick p99 denominator in milliseconds.
///
/// Smoke value only; the gate lives in `docs/tech/quality.md`.
const SMOKE_SIM_P99_BUDGET_MS_F64: f64 = 16.0;

/// Smoke surface-hitch p95 denominator in milliseconds.
///
/// Smoke value only; the gate lives in `docs/tech/quality.md`.
const SMOKE_HITCH_BUDGET_MS_F64: f64 = 100.0;

/// Smoke memory ceiling denominator in megabytes.
///
/// Smoke value only; the gate lives in `docs/tech/quality.md`.
const SMOKE_MEMORY_BUDGET_MB_F64: f64 = 1024.0;

/// Smoke cold-start denominator in seconds.
///
/// Smoke value only; the gate lives in `docs/tech/quality.md`.
const SMOKE_COLD_START_BUDGET_S_F64: f64 = 5.0;

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
    #[cfg(feature = "dev-shell")]
    {
        if run_dev_shell_window()? {
            return Ok(());
        }
    }
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
    print_input_smoke();
    print_inspect_smoke();
    print_shell_smoke();
    Ok(())
}

/// Open the OS window when flagged, else report headless.
///
/// Returns true when the window ran; false keeps the headless demo.
/// `--software` forces the fallback adapter with `--run-window`, else headless.
/// `--backend auto|vulkan|dx12|gl` restricts the wgpu backend with
/// `--run-window`, else headless; unknown values are typed errors.
/// Available only with the non-default `dev-shell` feature.
///
/// # Errors
///
/// Returns the OS window error when the flagged run fails.
#[cfg(feature = "dev-shell")]
fn run_dev_shell_window() -> anyhow::Result<bool> {
    let args: Vec<String> = std::env::args().collect();
    let software_mode_bool = os_window::software_requested(&args);
    let backend_selection = os_window::parse_backend_selection(&args)?;
    match os_window::decide_launch(&args) {
        os_window::LaunchDecision::OpenWindow => {
            os_window::run_window(software_mode_bool, backend_selection)?;
            Ok(true)
        }
        os_window::LaunchDecision::StayHeadless(reason) => {
            if software_mode_bool && reason == os_window::HeadlessReason::FlagMissing {
                println!(
                    "headless reason={reason}; note={} needs {}",
                    os_window::SOFTWARE_FLAG,
                    os_window::RUN_WINDOW_FLAG
                );
            } else {
                println!("headless reason={reason}");
            }
            Ok(false)
        }
    }
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
        match preset.default_bottom_tab() {
            Some(tab) => println!(
                "preset {name} bottom_tab={tab} bottom={bottom}",
                name = preset.label(),
                tab = tab.label(),
                bottom = bottom::BottomTab::default_for_preset(preset)
                    .map_or("none", bottom::BottomTab::label)
            ),
            None => println!("preset {name} bottom_tab=none", name = preset.label()),
        }
    }
    for tab in bottom::BottomTab::ALL {
        println!(
            "bottom_tab {label} phone={phone}",
            label = tab.label(),
            phone = tab.to_phone_tab().label()
        );
    }
    for region in layout::DockRegion::ALL {
        println!("region {label}", label = region.label());
    }
    for tab in layout::PhoneTab::ALL {
        println!(
            "phone_tab {label} phase_b={phase_b} bottom={bottom}",
            label = tab.label(),
            phase_b = tab.is_phase_b(),
            bottom =
                bottom::BottomTab::from_phone_tab(tab).map_or("none", bottom::BottomTab::label)
        );
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
        "buffers plot={plot} log={log} recorder={recorder} hash={hash} touch_pt={touch} plot_height_pt={height}",
        plot = buffers.plot_history_entries_usize(),
        log = buffers.log_history_entries_usize(),
        recorder = buffers.input_recorder_entries_usize(),
        hash = buffers.hash_history_entries_usize(),
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

/// Print input-router modes, gestures, routes, and pick tolerance.
fn print_input_smoke() {
    let mut router = input::InputRouter::new();
    println!(
        "input mode={label} passthrough={pass} tol_pt={tol}",
        label = router.mode().label(),
        pass = router.is_passthrough(),
        tol = router.pick_tolerance_pt_f32()
    );
    for key in input::RouterKey::ALL {
        let mode = router.on_key(key);
        println!(
            "input key={label} mode={mode}",
            label = key.label(),
            mode = mode.label()
        );
    }
    router.open_modal();
    println!(
        "input modal open={open} focused={focused}",
        open = router.is_modal_open(),
        focused = router.is_focused()
    );
    router.close_modal();
    println!("input modal open={open}", open = router.is_modal_open());
    match router.on_dev_tag_long_press(input::DEV_TAG_LONG_PRESS_S_F64) {
        Ok(mode) => println!("input long_press mode={mode}", mode = mode.label()),
        Err(error) => println!("input long_press_error={error}"),
    }
    match router.on_dev_tag_long_press(0.1) {
        Ok(mode) => println!("input short_press mode={mode}", mode = mode.label()),
        Err(error) => println!("input short_press_error={error}"),
    }
    match router.on_dev_tag_long_press(f64::NAN) {
        Ok(mode) => println!("input nan_press mode={mode}", mode = mode.label()),
        Err(error) => println!("input nan_press_error={error}"),
    }
    let tapped = router.on_multi_finger_tap(input::THREE_FINGER_TAP_COUNT_U8);
    println!("input three_finger mode={mode}", mode = tapped.label());
    let ignored = router.on_multi_finger_tap(2);
    println!("input two_finger mode={mode}", mode = ignored.label());
    for wants_pointer in [false, true] {
        for wants_keyboard in [false, true] {
            let route = router.route(wants_pointer, wants_keyboard);
            println!(
                "input route wants_pointer={pointer} wants_keyboard={keyboard} route={route} shell={shell} game_copy={copy}",
                pointer = wants_pointer,
                keyboard = wants_keyboard,
                route = route.label(),
                shell = route.shell_consumes(),
                copy = route.game_sees_copy()
            );
        }
    }
    match router.set_pick_tolerance(12.0) {
        Ok(()) => println!(
            "input tol set_pt={tol}",
            tol = router.pick_tolerance_pt_f32()
        ),
        Err(error) => println!("input tol_error={error}"),
    }
    match router.set_pick_tolerance(f32::NAN) {
        Ok(()) => println!("input tol_nan unexpected"),
        Err(error) => println!("input tol_nan_error={error}"),
    }
}

/// Print inspect-view scalars plus every snapshot code map.
fn print_inspect_smoke() {
    match inspect_view::InspectView::new(input::TAP_PICK_TOLERANCE_PT_F32) {
        Ok(view) => {
            println!(
                "inspect tick={tick} elapsed_s={elapsed} warp_factor={warp} drop={drop} regime={regime} frame={frame} elements={elements} pick={pick} tol_pt={tol}",
                tick = view.tick_count_u64(),
                elapsed = view.elapsed_s_f64(),
                warp = view.warp_factor_f64(),
                drop = view.drop_label(),
                regime = view.regime_label(),
                frame = view.frame_label(),
                elements = view.elements_valid(),
                pick = view.pick_valid(),
                tol = view.pick_tolerance_pt_f32()
            );
            println!(
                "inspect clocks ship_epoch_s={epoch} master_seed={master} stream_seed={stream} hash={hash}",
                epoch = view.ship_epoch_s_f64(),
                master = view.master_seed_u64(),
                stream = view.stream_seed_u64(),
                hash = view.snapshot_hash_u64()
            );
            println!(
                "inspect aero alt_m={alt} speed_mps={speed} p_pa={p} t_k={t} d_kg_m3={d} heat={heat} g={g}",
                alt = view.altitude_m_f64(),
                speed = view.speed_mps_f64(),
                p = view.pressure_pa_f64(),
                t = view.temperature_k_f64(),
                d = view.density_kg_m3_f64(),
                heat = view.heat_flux_w_per_m2_f64(),
                g = view.g_load_g_f64()
            );
            println!(
                "inspect elements axis_m={axis} ecc={ecc} incl_rad={incl} raan_rad={raan} arg_rad={arg} mean_rad={mean} mu={mu}",
                axis = view.semi_major_axis_m_f64(),
                ecc = view.eccentricity_f64(),
                incl = view.inclination_rad_f64(),
                raan = view.raan_rad_f64(),
                arg = view.arg_periapsis_rad_f64(),
                mean = view.mean_anomaly_rad_f64(),
                mu = view.mu_m3_s2_f64()
            );
            println!(
                "inspect frame warp_code={warp} regime={regime:?} body={body} parent={parent} depth={depth}",
                warp = view.warp_code_u8(),
                regime = view.regime(),
                body = view.frame_body_id_u32(),
                parent = view.parent_body_id_u32(),
                depth = view.frame_depth_u8()
            );
            println!(
                "inspect pick body={body} alt_m={alt} range_m={range} cell=({cx},{cy}) mark={mark}",
                body = view.pick_body_id_u32(),
                alt = view.pick_altitude_m_f64(),
                range = view.pick_range_m_f64(),
                cx = view.pick_cell().0,
                cy = view.pick_cell().1,
                mark = view.mark_label()
            );
            for code_u8 in [0_u8, 1, 2, 3, 4, 9] {
                match inspect_view::InspectView::warp_for(code_u8) {
                    Ok(warp) => println!(
                        "inspect warp code={code_u8} factor={factor}",
                        factor = warp.factor()
                    ),
                    Err(error) => println!("inspect warp code={code_u8} error={error}"),
                }
            }
            for code_u8 in [0_u8, 1, 2, 3, 9] {
                match inspect_view::InspectView::drop_label_for(code_u8) {
                    Ok(drop) => println!("inspect drop code={code_u8} reason={drop}"),
                    Err(error) => println!("inspect drop code={code_u8} error={error}"),
                }
            }
            for code_u8 in [0_u8, 1, 2, 9] {
                match inspect_view::InspectView::regime_label_for(code_u8) {
                    Ok(regime) => println!("inspect regime code={code_u8} regime={regime}"),
                    Err(error) => println!("inspect regime code={code_u8} error={error}"),
                }
            }
            for level_u8 in [5_u8, 6, 7, 4] {
                match inspect_view::InspectView::frame_label_for(level_u8) {
                    Ok(frame) => println!("inspect frame level={level_u8} frame={frame}"),
                    Err(error) => println!("inspect frame level={level_u8} error={error}"),
                }
            }
            for kind_u8 in [0_u8, 1, 2, 3, 4, 5, 6, 7, 9] {
                match inspect_view::InspectView::mark_label_for(kind_u8) {
                    Ok(mark) => println!("inspect mark kind={kind_u8} mark={mark}"),
                    Err(error) => println!("inspect mark kind={kind_u8} error={error}"),
                }
            }
        }
        Err(error) => println!("inspect_error={error}"),
    }
}

/// Print shell assembly, cost hook, close flag, and gated draw.
fn print_shell_smoke() {
    match shell::Shell::new() {
        Ok(mut shell) => {
            println!(
                "shell mode={mode} closed={closed} top={top} draw_ms={ms}",
                mode = shell.mode().label(),
                closed = shell.is_closed(),
                top = shell.visibility().shows_top_bar(),
                ms = shell.draw_cost_ms_f64()
            );
            let focused = shell.handle_key(input::RouterKey::F3);
            println!("shell f3 mode={mode}", mode = focused.label());
            println!(
                "shell passthrough={pass} focused={focused} tick={tick} meter_empty={empty} router={router}",
                pass = shell.is_passthrough(),
                focused = shell.is_focused(),
                tick = shell.top_bar().tick_count_u64(),
                empty = shell.meter().is_empty(),
                router = shell.router().mode().label()
            );
            shell.top_bar_mut().pause();
            println!(
                "shell paused={paused}",
                paused = shell.top_bar().is_paused()
            );
            shell.top_bar_mut().resume();
            match shell.handle_long_press(input::DEV_TAG_LONG_PRESS_S_F64) {
                Ok(mode) => println!("shell long_press mode={mode}", mode = mode.label()),
                Err(error) => println!("shell long_press_error={error}"),
            }
            let tapped = shell.handle_tap(input::THREE_FINGER_TAP_COUNT_U8);
            println!("shell tap mode={mode}", mode = tapped.label());
            shell.set_visibility(layout::PanelVisibility::for_preset(
                layout::DesktopPreset::Descent,
            ));
            println!(
                "shell descent left={left} bottom={bottom}",
                left = shell.visibility().shows_left_panel(),
                bottom = shell.visibility().shows_bottom_tabs()
            );
            shell.set_visibility(layout::PanelVisibility::for_preset(
                layout::DesktopPreset::TickerOnly,
            ));
            println!(
                "shell route game={game} shell={shell}",
                game = shell.route(false, false).label(),
                shell = shell.route(true, false).label()
            );
            match shell.record_draw_cost(SMOKE_DRAW_MS_F64) {
                Ok(()) => println!(
                    "shell draw_ms={ms} avg_ms={avg}",
                    ms = shell.draw_cost_ms_f64(),
                    avg = shell.average_draw_ms_f64()
                ),
                Err(error) => println!("shell_cost_error={error}"),
            }
            match shell.record_draw_cost(f64::NAN) {
                Ok(()) => println!("shell cost_nan unexpected"),
                Err(error) => println!("shell cost_nan_error={error}"),
            }
            shell.request_close();
            println!("shell closed={closed}", closed = shell.is_closed());
            shell.reopen();
            println!("shell reopened={closed}", closed = shell.is_closed());
            #[cfg(feature = "dev-shell")]
            print_dev_shell_snapshot_smoke(&mut shell);
        }
        Err(error) => println!("shell_error={error}"),
    }
}

/// Print channel, regime, level, and tier tables plus filter state.
///
/// Available only with the non-default `dev-shell` feature.
#[cfg(feature = "dev-shell")]
fn print_phase_b_tables(shell: &mut shell::Shell) {
    for (index_usize, name) in continuity::CHANNEL_LABELS.iter().enumerate() {
        println!("channel {name} index={index_usize}");
    }
    println!(
        "channels speed={speed} pressure={pressure} temperature={temperature} density={density} heat={heat} g={g}",
        speed = continuity::CHANNEL_SPEED_MPS_USIZE,
        pressure = continuity::CHANNEL_PRESSURE_PA_USIZE,
        temperature = continuity::CHANNEL_TEMPERATURE_K_USIZE,
        density = continuity::CHANNEL_DENSITY_KG_M3_USIZE,
        heat = continuity::CHANNEL_HEAT_W_M2_USIZE,
        g = continuity::CHANNEL_G_LOAD_G_USIZE
    );
    println!(
        "regimes orbit={orbit} atmosphere={atmo} surface={surface}",
        orbit = continuity::CONTINUITY_REGIME_ORBIT_U8,
        atmo = continuity::CONTINUITY_REGIME_ATMOSPHERE_U8,
        surface = continuity::CONTINUITY_REGIME_SURFACE_U8
    );
    for level in log::LogLevel::ALL {
        println!("log_level {label}", label = level.label());
    }
    for tier in budget::ThermalTier::ALL {
        println!("thermal_tier {label}", label = tier.label());
    }
    for kind in [
        tweak::TweakKind::Bool,
        tweak::TweakKind::I64,
        tweak::TweakKind::F64,
        tweak::TweakKind::Str,
        tweak::TweakKind::Enum,
    ] {
        println!("tweak_kind {label}", label = kind.label());
    }
    for entry in tweak::REGISTRY {
        println!(
            "registry {name} [{unit}]",
            name = entry.name,
            unit = entry.unit
        );
    }
    println!(
        "continuity empty={empty} samples={samples} log_empty={log_empty} log={log}",
        empty = shell.continuity().is_empty(),
        samples = shell.continuity().len_usize(),
        log_empty = shell.trace_log().is_empty(),
        log = shell.trace_log().len_usize()
    );
    shell.trace_log_mut().set_filter_level(log::LogLevel::Warn);
    match shell.trace_log_mut().set_filter_module("sim") {
        Ok(()) => println!(
            "trace_filter level={level} module=sim",
            level = shell.trace_log().filter_level().label()
        ),
        Err(error) => println!("trace_filter_error={error}"),
    }
    shell.trace_log_mut().set_filter_level(log::LogLevel::Info);
    match shell.trace_log_mut().set_filter_module("") {
        Ok(()) => println!("trace_filter reset=ok"),
        Err(error) => println!("trace_filter_error={error}"),
    }
}

/// Print desktop-tester readouts copied via the readouts-only API.
///
/// Available only with the non-default `dev-shell` feature.
#[cfg(feature = "dev-shell")]
fn print_desktop_tester_smoke(shell: &shell::Shell) {
    let readouts = shell.desktop_tester_readouts();
    println!(
        "desktop_tester tick={tick} regime={regime} alt_m={alt} speed_mps={speed} elements_valid={valid} warp={warp}x drop={drop} frame={frame}",
        tick = readouts.tick_count_u64,
        regime = readouts.regime_label,
        alt = readouts.altitude_m_f64,
        speed = readouts.speed_mps_f64,
        valid = readouts.elements_valid_bool,
        warp = readouts.warp_factor_f64,
        drop = readouts.drop_label,
        frame = readouts.frame_label
    );
}

/// Observe a smoke snapshot and run one headless shell draw.
///
/// Available only with the non-default `dev-shell` feature.
#[cfg(feature = "dev-shell")]
fn print_dev_shell_snapshot_smoke(shell: &mut shell::Shell) {
    print_phase_b_tables(shell);
    let identity = shell::BundleIdentity {
        created_utc: String::from("2026-09-27T00:00:00Z"),
        app_version: String::from("0.1.0-smoke"),
        platform: String::from("smoke-host"),
        tier: String::from("medium"),
    };
    match shell.export_bundle_to(std::path::Path::new(""), &identity) {
        Ok(_) => println!("export_unexpected_ok"),
        Err(error) => println!("export_no_snapshot_error={error}"),
    }
    shell.begin_run(SMOKE_SEED_U64);
    print_run_control_smoke(shell);
    let snapshot = dev_shell_smoke_snapshot();
    match shell.observe_snapshot(&snapshot) {
        Ok(()) => {
            println!(
                "shell snapshot tick={tick} regime={regime} mark={mark} pick_valid={pick}",
                tick = shell.inspect().tick_count_u64(),
                regime = shell.inspect().regime_label(),
                mark = shell.inspect().mark_label(),
                pick = shell.inspect().pick_valid()
            );
            print_desktop_tester_smoke(shell);
        }
        Err(error) => println!("shell_snapshot_error={error}"),
    }
    print_console_smoke(shell);
    print_replay_smoke(shell);
    let ctx = egui::Context::default();
    print_draw_export_smoke(shell, &ctx, &identity);
}

/// Draw the shell plus export plus replay-state smoke.
///
/// Available only with the non-default `dev-shell` feature.
#[cfg(feature = "dev-shell")]
fn print_draw_export_smoke(
    shell: &mut shell::Shell,
    ctx: &egui::Context,
    identity: &shell::BundleIdentity,
) {
    shell.set_visibility(layout::PanelVisibility::for_preset(
        layout::DesktopPreset::Descent,
    ));
    match shell.continuity_mut().set_window_s_f64(30.0) {
        Ok(()) => println!(
            "continuity window_s={window} samples={samples}",
            window = shell.continuity().window_s_f64(),
            samples = shell.continuity().len_usize()
        ),
        Err(error) => println!("continuity_window_error={error}"),
    }
    assert!(
        shell
            .budget_strip_mut()
            .set_frame_ms_f64(SMOKE_FRAME_MS_F64)
            .is_ok()
    );
    assert!(shell.budget_strip_mut().set_sim_avg_ms_f64(2.0).is_ok());
    assert!(shell.budget_strip_mut().set_sim_p99_ms_f64(4.0).is_ok());
    assert!(shell.budget_strip_mut().set_hitch_p95_ms_f64(10.0).is_ok());
    assert!(shell.budget_strip_mut().set_resident_mb_f64(256.0).is_ok());
    assert!(shell.budget_strip_mut().set_cold_start_s_f64(1.5).is_ok());
    assert!(
        shell
            .budget_strip_mut()
            .set_shell_ms_f64(SMOKE_DRAW_MS_F64)
            .is_ok()
    );
    shell
        .budget_strip_mut()
        .set_thermal_tier(budget::ThermalTier::Medium);
    match shell
        .trace_log_mut()
        .push(42, log::LogLevel::Info, "sim", "smoke entry")
    {
        Ok(()) => println!(
            "trace_log entries={entries}",
            entries = shell.trace_log().len_usize()
        ),
        Err(error) => println!("trace_log_error={error}"),
    }
    shell.bottom_tabs_mut().select(bottom::BottomTab::Console);
    shell.determinism_window_mut().open();
    println!(
        "window open={open} tab={tab}",
        open = shell.determinism_window().is_open(),
        tab = shell.bottom_tabs().selected().label()
    );
    match budget::BudgetDenominators::new(
        SMOKE_FRAME_BUDGET_MS_F64,
        SMOKE_SIM_AVG_BUDGET_MS_F64,
        SMOKE_SIM_P99_BUDGET_MS_F64,
        SMOKE_HITCH_BUDGET_MS_F64,
        SMOKE_MEMORY_BUDGET_MB_F64,
        SMOKE_COLD_START_BUDGET_S_F64,
    ) {
        Ok(budgets) => {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                let action = shell.draw(ctx, ui, SMOKE_FRAME_BUDGET_MS_F64, budgets, false);
                println!(
                    "draw export_requested={export}",
                    export = action.export_requested()
                );
            });
            // Headless smoke has no renderer; Step 5 applies texture deltas.
            output.textures_delta.clear();
            println!(
                "shell_draw=ok closed={closed} mode={mode} bottom={bottom} samples={samples} markers={markers} tier={tier} log={log} report={report}",
                closed = shell.is_closed(),
                mode = shell.mode().label(),
                bottom = shell.bottom_tabs().selected().label(),
                samples = shell.continuity().len_usize(),
                markers = shell.continuity().markers().len(),
                tier = shell.budget_strip().thermal_tier().label(),
                log = shell.trace_log().len_usize(),
                report = shell
                    .last_report()
                    .map_or(0, determinism::ReplayReport::ticks_compared_u64)
            );
        }
        Err(error) => println!("shell_budgets_error={error}"),
    }
    print_export_smoke(shell, identity);
    shell.determinism_window_mut().close();
    println!(
        "window open={open}",
        open = shell.determinism_window().is_open()
    );
    shell.top_bar_mut().begin_replay();
    println!(
        "replay health={health} clean={clean}",
        health = shell.top_bar().health_label(),
        clean = shell.top_bar().is_clean()
    );
}

/// Print run-control hooks with recorder states.
///
/// Available only with the non-default `dev-shell` feature.
#[cfg(feature = "dev-shell")]
fn print_run_control_smoke(shell: &mut shell::Shell) {
    use engine::warp::{Warp, WarpContext};
    println!(
        "run state={state} master={master:04x} clean={clean}",
        state = shell.recorder().state().label(),
        master = determinism::SeedTreeView::short_u16(shell.recorder().master_seed_u64()),
        clean = shell.top_bar().is_clean()
    );
    shell.pause();
    println!(
        "paused={paused} recorder={state} entries={entries}",
        paused = shell.top_bar().is_paused(),
        state = shell.recorder().state().label(),
        entries = shell.recorder().len_usize()
    );
    shell.resume();
    shell.request_step();
    match shell.request_warp(Warp::X100, WarpContext::cruise()) {
        Ok(granted) => println!("warp granted={factor}x", factor = granted.factor()),
        Err(error) => println!("warp_error={error}"),
    }
    println!(
        "hash empty={empty} len={len} seed={master:04x}/{star:04x} recorder_empty={rec_empty}",
        empty = shell.hash_ring().is_empty(),
        len = shell.hash_ring().len_usize(),
        master = determinism::SeedTreeView::short_u16(shell.seed_tree().master_seed_u64()),
        star = determinism::SeedTreeView::short_u16(shell.seed_tree().gen_star_u64()),
        rec_empty = shell.recorder().is_empty()
    );
    for kind in determinism::InputKind::ALL {
        println!(
            "input kind={label} code={code}",
            label = kind.label(),
            code = kind.code_u8()
        );
    }
    match determinism::InputKind::from_code(3_u8) {
        Ok(kind) => println!("input code 3={label}", label = kind.label()),
        Err(error) => println!("input_code_error={error}"),
    }
}

/// Run console lines and print the draft left staged.
///
/// Available only with the non-default `dev-shell` feature.
#[cfg(feature = "dev-shell")]
fn print_console_smoke(shell: &mut shell::Shell) {
    for line in [
        "get plots.window_s",
        "seed",
        "hash",
        "set plots.window_s 30",
        "set atmo.density_scale 1.5",
        "warp 2",
        "warp 9",
        "load bundle/",
        "replay log.csv",
        "frobnicate now",
    ] {
        match shell.execute_console_line(line) {
            Ok(()) => println!("console ok: {line}"),
            Err(error) => println!("console err: {line}: {error}"),
        }
    }
    match console::parse_command("get plots.window_s") {
        Ok(command) => println!("command safe={safe}", safe = command.is_safe_read()),
        Err(error) => println!("command_error={error}"),
    }
    match console::parse_command("set plots.window_s 30") {
        Ok(command) => println!("command safe={safe}", safe = command.is_safe_read()),
        Err(error) => println!("command_error={error}"),
    }
    shell.console_mut().push_history("get seed");
    shell.console_mut().push_history("hash");
    let history_len = shell.console_mut().history_len_usize();
    let older_opt = shell.console_mut().history_older().map(str::to_string);
    let newer_opt = shell.console_mut().history_newer().map(str::to_string);
    println!("history len={history_len} older={older_opt:?} newer={newer_opt:?}");
    match tweak::lookup_entry_id("plots.window_s") {
        Ok(id_u16) => match shell.tweak_board().draft(id_u16) {
            Ok(Some(draft)) => println!(
                "draft entry={entry} bits={bits:016x} confirm={confirm:?}",
                entry = draft.entry_id_u16(),
                bits = draft.pending_bits_u64(),
                confirm = shell.tweak_board().confirm_id_u16()
            ),
            Ok(None) => println!("draft none"),
            Err(error) => println!("draft_error={error}"),
        },
        Err(error) => println!("draft_error={error}"),
    }
    shell.console_mut().input_line_mut().push_str("hash");
    println!(
        "console staged={len} output={output}",
        len = shell.console_mut().input_line_mut().len(),
        output = shell.console().output().len()
    );
    shell.console_mut().clear_input();
    match tweak::lookup_entry_id("plots.window_s") {
        Ok(id_u16) => match shell.tweak_board().committed_f64(id_u16) {
            Ok(value_f64) => println!("committed window_s={value_f64}"),
            Err(error) => println!("committed_error={error}"),
        },
        Err(error) => println!("committed_error={error}"),
    }
    match shell.recorder_mut().record(determinism::InputEntry::new(
        99_u64,
        determinism::InputKind::StepTick,
        determinism::InputPayload::zero(),
    )) {
        Ok(()) => println!(
            "recorder entries={entries}",
            entries = shell.recorder().len_usize()
        ),
        Err(error) => println!("recorder_error={error}"),
    }
}

/// Replay clean plus bad-warp logs and store the divergence.
///
/// Available only with the non-default `dev-shell` feature.
#[cfg(feature = "dev-shell")]
fn print_replay_smoke(shell: &mut shell::Shell) {
    match determinism::replay(SMOKE_SEED_U64, SMOKE_SEED_U64, &[], &[]) {
        Ok(report) => {
            println!(
                "replay clean ticks={ticks} match={matched}",
                ticks = report.ticks_compared_u64(),
                matched = matches!(report.status(), determinism::ReplayStatus::Match)
            );
            shell.set_last_report(report);
        }
        Err(error) => println!("replay_error={error}"),
    }
    let bad_inputs = [determinism::InputEntry::new(
        2_u64,
        determinism::InputKind::WarpRequest,
        determinism::InputPayload::warp(9_u8, 0_u8),
    )];
    match determinism::replay(
        SMOKE_SEED_U64,
        SMOKE_SEED_U64,
        &bad_inputs,
        &[(2_u64, 0_u64)],
    ) {
        Ok(report) => {
            match report.status() {
                determinism::ReplayStatus::Diverged(record) => {
                    println!(
                        "replay diverged tick={tick} input={kind}",
                        tick = record.tick_count_u64(),
                        kind = record
                            .input_at_tick()
                            .map_or("none", |entry| entry.kind().label())
                    );
                }
                determinism::ReplayStatus::Match => println!("replay_unexpected_match"),
            }
            shell.set_last_report(report);
        }
        Err(error) => println!("replay_error={error}"),
    }
}

/// Export, verify, tamper, quarantine, and clean a smoke bundle.
///
/// Available only with the non-default `dev-shell` feature.
#[cfg(feature = "dev-shell")]
fn print_export_smoke(shell: &mut shell::Shell, identity: &shell::BundleIdentity) {
    let root = std::env::temp_dir().join(format!("universe-phasec-smoke-{}", std::process::id()));
    let bundle_dir = root.join("bundle");
    match shell.export_bundle_to(&bundle_dir, identity) {
        Ok(path) => {
            println!("export ok path={path}", path = path.display());
            for name in export::BUNDLE_FILE_NAMES {
                println!(
                    "bundle file {name} exists={exists}",
                    exists = bundle_dir.join(name).exists()
                );
            }
        }
        Err(error) => println!("export_error={error}"),
    }
    let inputs_path = bundle_dir.join(export::INPUTS_FILE_NAME);
    if let Ok(mut bytes) = std::fs::read(&inputs_path) {
        bytes.push(b'x');
        if std::fs::write(&inputs_path, &bytes).is_ok() {
            match export::verify_bundle_hashes(&bundle_dir) {
                Ok(()) => println!("verify_unexpected_ok"),
                Err(error) => println!("verify_tamper_error={error}"),
            }
            match export::quarantine_bundle(&bundle_dir, 1, 2) {
                Ok(path) => println!("quarantined={path}", path = path.display()),
                Err(error) => println!("quarantine_error={error}"),
            }
        }
    }
    if std::fs::remove_dir_all(&root).is_err() {
        println!("smoke_cleanup_missed");
    }
    shell.recorder_mut().stop_on_export();
    println!(
        "recorder state={state} entries={entries}",
        state = shell.recorder().state().label(),
        entries = shell.recorder().len_usize()
    );
}

/// Build a dev-shell smoke snapshot with orbit defaults.
///
/// Available only with the non-default `dev-shell` feature.
#[cfg(feature = "dev-shell")]
fn dev_shell_smoke_snapshot() -> engine::inspect::SimSnapshot {
    engine::inspect::SimSnapshot {
        tick_count_u64: SMOKE_TICK_COUNT_U64,
        elapsed_s_f64: SMOKE_ELAPSED_S_F64,
        ship_epoch_s_f64: SMOKE_ELAPSED_S_F64,
        master_seed_u64: SMOKE_SEED_U64,
        stream_seed_u64: SMOKE_SEED_U64,
        snapshot_hash_u64: SMOKE_HASH_U64,
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
        warp_code_u8: 0,
        drop_reason_u8: 0,
        warp_flags_u8: 3,
        regime_u8: 0,
        frame_level_u8: 5,
        frame_depth_u8: 2,
        elements_valid_u8: 1,
        pick_valid_u8: 1,
        mark_kind_u8: 6,
        _pad_u8: [0_u8; 3],
    }
}
