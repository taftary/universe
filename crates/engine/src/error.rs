//! Typed errors for the engine crate.

use thiserror::Error;

/// Engine-wide error enum.
#[derive(Debug, Error)]
pub enum EngineError {
    /// Fixed step was not positive and finite.
    #[error("invalid fixed step in seconds: {step_s}")]
    InvalidStep {
        /// Rejected step in seconds.
        step_s: f64,
    },
    /// Platform IO failed.
    #[error("platform IO failed: {0}")]
    Io(#[from] std::io::Error),
}
