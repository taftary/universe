//! Fixed-step simulation scheduler and floating origin.

use glam::DVec3;

use crate::error::EngineError;

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

/// Fixed-step scheduler: advances sim time by a constant step.
///
/// # Example
///
/// ```
/// # use engine::sim::Scheduler;
/// let scheduler = Scheduler::new(0.5);
/// assert!(scheduler.is_ok());
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scheduler {
    /// Fixed step in seconds.
    step_s: f64,
    /// Elapsed sim time in seconds.
    elapsed_s: f64,
    /// Steps advanced so far.
    step_count: u64,
}

impl Scheduler {
    /// Fixed step in seconds; must be positive and finite.
    ///
    /// # Errors
    ///
    /// Returns `EngineError::InvalidStep` when `step_s` is not positive.
    pub fn new(step_s: f64) -> Result<Self, EngineError> {
        if step_s.is_finite() && step_s > 0.0 {
            Ok(Self {
                step_s,
                elapsed_s: 0.0,
                step_count: 0,
            })
        } else {
            Err(EngineError::InvalidStep { step_s })
        }
    }

    /// Advance one fixed step.
    #[tracing::instrument(skip(self))]
    pub fn advance(&mut self) {
        self.elapsed_s += self.step_s;
        self.step_count += 1;
    }

    /// Fixed step in seconds.
    #[must_use]
    pub fn step_s(&self) -> f64 {
        self.step_s
    }

    /// Elapsed sim time in seconds.
    #[must_use]
    pub fn elapsed_s(&self) -> f64 {
        self.elapsed_s
    }

    /// Steps advanced so far.
    #[must_use]
    pub fn step_count(&self) -> u64 {
        self.step_count
    }
}
