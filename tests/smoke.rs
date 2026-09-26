//! Headless sim smoke: fixed-step profile placeholder.
//!
//! Grows into the descent-profile gate in `docs/tech/quality.md`: same seed
//! and inputs yield the same state hash across runs and platforms.

#![forbid(unsafe_code)]

use engine::sim::Scheduler;

/// Ticks advanced by the smoke profile.
const TICK_COUNT: u64 = 8;

/// Fixed step in seconds for the smoke profile.
const SMOKE_STEP_S: f64 = 0.5;

/// Tolerance in seconds for accumulated floating-point drift.
const TIME_TOLERANCE_S: f64 = 1e-9;

/// Base seed for the determinism check.
const SEED: u64 = 0x1234_5678_9ABC_DEF0;

/// First salt for the determinism check.
const SALT_ONE: u64 = 1;

/// Second salt for the determinism check.
const SALT_TWO: u64 = 2;

/// Build a scheduler, failing the test on rejection.
fn new_scheduler(step_s: f64) -> Scheduler {
    match Scheduler::new(step_s) {
        Ok(scheduler) => scheduler,
        Err(error) => panic!("positive step rejected: {error}"),
    }
}

/// Scheduler advances exact multiples of the fixed step.
#[test]
fn fixed_step_advances_exact_time() {
    let mut scheduler = new_scheduler(SMOKE_STEP_S);
    for _ in 0..TICK_COUNT {
        scheduler.advance();
    }
    assert_eq!(scheduler.step_count(), TICK_COUNT);
    #[expect(
        clippy::cast_precision_loss,
        reason = "TICK_COUNT is 8, exactly representable in f64"
    )]
    let expected_s = TICK_COUNT as f64 * SMOKE_STEP_S;
    let drift_s = (scheduler.elapsed_s() - expected_s).abs();
    assert!(
        drift_s <= TIME_TOLERANCE_S,
        "drift {drift_s} exceeds {TIME_TOLERANCE_S}"
    );
}

/// Same seed and inputs yield the same state; distinct salts differ.
#[test]
fn seed_mixing_is_deterministic() {
    assert_eq!(
        engine::generation::mix_seed(SEED, SALT_ONE),
        engine::generation::mix_seed(SEED, SALT_ONE)
    );
    assert_ne!(
        engine::generation::mix_seed(SEED, SALT_ONE),
        engine::generation::mix_seed(SEED, SALT_TWO)
    );
}

/// Game ticks advance the scheduler and evolve the seed.
#[test]
fn game_tick_smoke() {
    let mut game = match game::Game::new(SMOKE_STEP_S, SEED) {
        Ok(game) => game,
        Err(error) => panic!("positive step rejected: {error}"),
    };
    let first_seed = game.seed();
    game.tick();
    assert_eq!(game.step_count(), 1);
    assert_ne!(game.seed(), first_seed);
}
