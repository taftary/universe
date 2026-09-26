//! Headless sim smoke: fixed-step profile plus snapshot golden.
//!
//! Grows into the descent-profile gate in `docs/tech/quality.md`: same seed
//! and inputs yield the same state hash across runs and platforms. Snapshot
//! golden tests need the `dev-shell` feature for `engine::inspect`; the
//! remaining tests run without features.

#![forbid(unsafe_code)]

use engine::sim::Scheduler;
use engine::units::Seconds;

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

/// Ticks advanced by the golden-hash profile.
#[cfg(feature = "dev-shell")]
const GOLDEN_TICK_COUNT: u64 = 100;

/// Seed for the golden-hash profile. Source: fractional hex digits of pi.
/// Fixed project constant for the determinism gate.
#[cfg(feature = "dev-shell")]
const GOLDEN_SEED_U64: u64 = 0x243F_6A88_85A3_08D3;

/// Expected xxh3-64 digest after 100 golden ticks.
///
/// Snapshot golden for the headless M1 cruise profile documented in
/// `golden_hash_after_ticks`; relocked in issue 32 step 3.
#[cfg(feature = "dev-shell")]
const GOLDEN_100_TICK_HASH_U64: u64 = 17_172_072_447_561_828_286;

/// Cruise altitude in meters for the golden profile.
///
/// Circular Mars-like orbit above the rails cutoff; source: M1 cruise
/// profile in issue 32 step 3, matching the 250 km low-orbit reference.
#[cfg(feature = "dev-shell")]
const GOLDEN_ALTITUDE_M: f64 = 250_000.0;

/// Negative fixed step rejected by the scheduler.
const NEGATIVE_STEP_S: f64 = -0.5;

/// Build a scheduler, failing the test on rejection.
fn new_scheduler(step_s: f64) -> Scheduler {
    match Scheduler::new(Seconds::new(step_s)) {
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
    let drift_s = (scheduler.elapsed().value() - expected_s).abs();
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
    let mut game = match game::Game::new(Seconds::new(SMOKE_STEP_S), SEED) {
        Ok(game) => game,
        Err(error) => panic!("positive step rejected: {error}"),
    };
    let first_seed = game.seed();
    game.tick();
    assert_eq!(game.step_count(), 1);
    assert_ne!(game.seed(), first_seed);
}

/// Run the golden profile for a tick count and seed.
///
/// Steps the headless M1 cruise profile with the default scheduler at
/// `SIM_TICK_S`: Mars-like body and atmosphere, preset point-ship from a
/// 250 km circular orbit at `mu/r` speed, `Warp::X1` cruise inside the ship.
/// Each tick advances the scheduler, steps with `step_point_ship`, mixes the
/// stream seed with `mix_seed`, and captures a `SimSnapshot`; the digest is
/// `snapshot_hash` of the last snapshot. Intermediate ticks pin transitively
/// through deterministic stepping. Deterministic `libm`-only math, no IO.
#[cfg(feature = "dev-shell")]
fn golden_hash_after_ticks(tick_count_u64: u64, seed_u64: u64) -> u64 {
    use engine::atmosphere::AtmosphereParams;
    use engine::body::BodyParams;
    use engine::inspect::{capture_snapshot, snapshot_hash};
    use engine::orbit::Mu;
    use engine::sim::{SIM_TICK_S, Scheduler};
    use engine::trajectory::{StateVector, VehicleParams, step_point_ship};
    use engine::units::Seconds;
    use engine::warp::Warp;
    use glam::DVec3;

    let body = BodyParams::mars_like();
    let atmosphere = match AtmosphereParams::mars_like() {
        Ok(atmosphere) => atmosphere,
        Err(error) => panic!("golden atmosphere must validate: {error}"),
    };
    let vehicle = VehicleParams::preset();
    let mu = match Mu::new(body.gravitational_parameter_m3_s2()) {
        Ok(mu) => mu,
        Err(error) => panic!("golden mu must validate: {error}"),
    };
    let radius_m_f64 = body.radius_m().value() + GOLDEN_ALTITUDE_M;
    let speed_mps_f64 = libm::sqrt(mu.value() / radius_m_f64);
    let mut state = match StateVector::new(
        DVec3::new(radius_m_f64, 0.0, 0.0),
        DVec3::new(0.0, speed_mps_f64, 0.0),
        Seconds::new(0.0),
    ) {
        Ok(state) => state,
        Err(error) => panic!("golden state must validate: {error}"),
    };
    let mut scheduler = Scheduler::default();
    let mut stream_seed_u64 = seed_u64;
    let mut digest_u64 = 0_u64;
    for _ in 0..tick_count_u64 {
        scheduler.advance();
        let sample = match step_point_ship(&state, SIM_TICK_S, &body, &atmosphere, &vehicle, mu) {
            Ok(sample) => sample,
            Err(error) => panic!("golden step must succeed: {error}"),
        };
        state = sample.state;
        stream_seed_u64 = engine::generation::mix_seed(stream_seed_u64, scheduler.step_count());
        let snapshot = match capture_snapshot(
            &scheduler,
            &state,
            &body,
            &atmosphere,
            &vehicle,
            seed_u64,
            stream_seed_u64,
            Warp::X1,
            true,
            false,
            false,
        ) {
            Ok(snapshot) => snapshot,
            Err(error) => panic!("golden capture must succeed: {error}"),
        };
        digest_u64 = snapshot_hash(&snapshot);
    }
    digest_u64
}

/// Golden digest pins 100 snapshot ticks across platforms.
///
/// Headless M1 cruise profile hashed via `SimSnapshot`; `libm`-only math
/// keeps `x86_64` and `AArch64` in agreement.
#[cfg(feature = "dev-shell")]
#[test]
fn golden_hash_100_ticks() {
    let hash_u64 = golden_hash_after_ticks(GOLDEN_TICK_COUNT, GOLDEN_SEED_U64);
    assert_eq!(hash_u64, GOLDEN_100_TICK_HASH_U64);
}

/// Perturbed seed or truncated run changes the digest.
#[cfg(feature = "dev-shell")]
#[test]
fn hash_perturbation_changes_digest() {
    let baseline_u64 = golden_hash_after_ticks(GOLDEN_TICK_COUNT, GOLDEN_SEED_U64);
    let perturbed_u64 = golden_hash_after_ticks(GOLDEN_TICK_COUNT, GOLDEN_SEED_U64 ^ 1);
    assert_ne!(baseline_u64, perturbed_u64);
    let short_u64 = golden_hash_after_ticks(GOLDEN_TICK_COUNT - 1, GOLDEN_SEED_U64);
    assert_ne!(baseline_u64, short_u64);
}

/// Zero and negative steps are rejected as invalid.
#[test]
fn scheduler_rejects_non_positive_step() {
    assert!(matches!(
        Scheduler::new(Seconds::new(0.0)),
        Err(engine::error::EngineError::InvalidStep { .. })
    ));
    assert!(matches!(
        Scheduler::new(Seconds::new(NEGATIVE_STEP_S)),
        Err(engine::error::EngineError::InvalidStep { .. })
    ));
}

/// Non-finite steps are rejected as invalid.
#[test]
fn scheduler_rejects_non_finite_step() {
    assert!(matches!(
        Scheduler::new(Seconds::new(f64::NAN)),
        Err(engine::error::EngineError::InvalidStep { .. })
    ));
    assert!(matches!(
        Scheduler::new(Seconds::new(f64::INFINITY)),
        Err(engine::error::EngineError::InvalidStep { .. })
    ));
    assert!(matches!(
        Scheduler::new(Seconds::new(f64::NEG_INFINITY)),
        Err(engine::error::EngineError::InvalidStep { .. })
    ));
}
