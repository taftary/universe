//! Gameplay wiring: owns the scheduler and seed, drives ticks.
//!
//! No GPU code lives here. Any draw call outside `engine::render` is a bug.

#![forbid(unsafe_code)]

use thiserror::Error;

/// Game-level errors.
#[derive(Debug, Error)]
pub enum GameError {
    /// Scheduler rejected the fixed step.
    #[error(transparent)]
    Engine(#[from] engine::error::EngineError),
}

/// Gameplay state: fixed-step scheduler plus generation seed.
#[derive(Debug)]
pub struct Game {
    /// Fixed-step scheduler.
    scheduler: engine::sim::Scheduler,
    /// Generation seed, mixed once per tick.
    seed: u64,
}

impl Game {
    /// Wire a game: fixed step in seconds plus generation seed.
    ///
    /// # Errors
    ///
    /// Returns the engine error when `step_s` is not positive.
    pub fn new(step_s: f64, seed: u64) -> Result<Self, GameError> {
        Ok(Self {
            scheduler: engine::sim::Scheduler::new(step_s)?,
            seed,
        })
    }

    /// Advance one fixed step and mix the generation seed.
    pub fn tick(&mut self) {
        self.scheduler.advance();
        self.seed = engine::generation::mix_seed(self.seed, self.scheduler.step_count());
    }

    /// Steps advanced so far.
    #[must_use]
    pub fn step_count(&self) -> u64 {
        self.scheduler.step_count()
    }

    /// Current generation seed.
    #[must_use]
    pub fn seed(&self) -> u64 {
        self.seed
    }
}
