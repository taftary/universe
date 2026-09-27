//! Dev-only debug shell entry point. Never ships in release builds: the
//! binary requires the non-default `dev-shell` feature, and `debug` is not a
//! default workspace member.

#![forbid(unsafe_code)]

mod input;
mod inspect_view;
mod layout;
mod shell;
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
    print_input_smoke();
    print_inspect_smoke();
    print_shell_smoke();
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

/// Observe a smoke snapshot and run one headless shell draw.
///
/// Available only with the non-default `dev-shell` feature.
#[cfg(feature = "dev-shell")]
fn print_dev_shell_snapshot_smoke(shell: &mut shell::Shell) {
    let snapshot = dev_shell_smoke_snapshot();
    match shell.observe_snapshot(&snapshot) {
        Ok(()) => println!(
            "shell snapshot tick={tick} regime={regime} mark={mark} pick_valid={pick}",
            tick = shell.inspect().tick_count_u64(),
            regime = shell.inspect().regime_label(),
            mark = shell.inspect().mark_label(),
            pick = shell.inspect().pick_valid()
        ),
        Err(error) => println!("shell_snapshot_error={error}"),
    }
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        shell.draw(&ctx, ui, SMOKE_FRAME_BUDGET_MS_F64);
    });
    // Headless smoke has no renderer; Step 5 applies texture deltas.
    output.textures_delta.clear();
    println!(
        "shell_draw=ok closed={closed} mode={mode}",
        closed = shell.is_closed(),
        mode = shell.mode().label()
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
