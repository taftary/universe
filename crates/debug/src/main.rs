//! Dev-only debug shell entry point. Never ships in release builds: the
//! binary requires the non-default `dev-shell` feature, and `debug` is not a
//! default workspace member.

#![forbid(unsafe_code)]

/// Demo tick count for the headless run.
const DEFAULT_TICK_COUNT: u64 = 4;

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
    Ok(())
}
