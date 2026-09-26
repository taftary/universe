//! Fixed-step simulation scheduler and floating origin.

use glam::DVec3;

use crate::error::EngineError;
use crate::units::Seconds;

/// Origin of the camera-relative frame in meters.
///
/// The sim runs in `f64` world space; rendering converts once, here, to `f32`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatingOrigin {
    /// Frame center in meters.
    center_m: DVec3,
}

impl FloatingOrigin {
    /// Zero origin: world and camera frames coincide.
    #[must_use]
    pub fn zero() -> Self {
        Self {
            center_m: DVec3::ZERO,
        }
    }

    /// Origin shifted to a body center in meters.
    #[must_use]
    pub fn shifted(center_m: DVec3) -> Self {
        Self { center_m }
    }

    /// Absolute position minus the origin, in meters.
    ///
    /// # Example
    ///
    /// ```
    /// # use engine::sim::FloatingOrigin;
    /// # use glam::DVec3;
    /// let origin = FloatingOrigin::shifted(DVec3::new(100.0, 0.0, 0.0));
    /// assert_eq!(
    ///     origin.relative_position_m(DVec3::new(101.0, 0.0, 0.0)),
    ///     DVec3::new(1.0, 0.0, 0.0)
    /// );
    /// ```
    #[must_use]
    pub fn relative_position_m(&self, absolute_m: DVec3) -> DVec3 {
        absolute_m - self.center_m
    }
}

/// Minimum fixed sim tick; never derived from frame time.
///
/// Source: D-012, locked in `docs/tech.md`; see `docs/tech/simulation.md`.
pub const SIM_TICK_S: Seconds = Seconds::new(0.05);

/// Fixed-step scheduler: advances sim time by a constant step.
///
/// # Example
///
/// ```
/// # use engine::sim::Scheduler;
/// # use engine::units::Seconds;
/// let scheduler = Scheduler::new(Seconds::new(0.5));
/// assert!(scheduler.is_ok());
/// if let Ok(scheduler) = scheduler {
///     assert!(scheduler.step().value() == 0.5);
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scheduler {
    /// Fixed step in seconds.
    step: Seconds,
    /// Elapsed sim time in seconds.
    elapsed: Seconds,
    /// Steps advanced so far.
    step_count: u64,
}

impl Scheduler {
    /// Create a scheduler with the given fixed step.
    ///
    /// # Errors
    ///
    /// Returns `EngineError::InvalidStep` when `step` is not positive and finite.
    pub fn new(step: Seconds) -> Result<Self, EngineError> {
        if step.value().is_finite() && step.value() > 0.0 {
            Ok(Self {
                step,
                elapsed: Seconds::new(0.0),
                step_count: 0,
            })
        } else {
            Err(EngineError::InvalidStep {
                step_s: step.value(),
            })
        }
    }

    /// Advance one fixed step.
    #[tracing::instrument(skip(self))]
    pub fn advance(&mut self) {
        self.elapsed += self.step;
        self.step_count += 1;
    }

    /// Fixed step in seconds.
    #[must_use]
    pub fn step(&self) -> Seconds {
        self.step
    }

    /// Elapsed sim time in seconds.
    #[must_use]
    pub fn elapsed(&self) -> Seconds {
        self.elapsed
    }

    /// Steps advanced so far.
    #[must_use]
    pub fn step_count(&self) -> u64 {
        self.step_count
    }
}

impl Default for Scheduler {
    /// Default scheduler at the minimum fixed tick.
    ///
    /// Uses [`SIM_TICK_S`], which is valid by construction and never fails.
    ///
    /// # Example
    ///
    /// ```
    /// # use engine::sim::Scheduler;
    /// let scheduler = Scheduler::default();
    /// assert!(scheduler.step().value() == engine::sim::SIM_TICK_S.value());
    /// ```
    fn default() -> Self {
        Self {
            step: SIM_TICK_S,
            elapsed: Seconds::new(0.0),
            step_count: 0,
        }
    }
}
