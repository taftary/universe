//! Universe simulation engine: fixed-step sim, deterministic generation,
//! render abstraction, and platform seams.
//!
//! `sim` is headless-testable: it depends only on `glam`, unit types, and the
//! project PRNG. No `winit`, no `wgpu`, no `egui`, no file IO.
//!
//! `unsafe` is allowed only in this crate, only with a `// SAFETY:` comment,
//! and only after techlead review. The workspace denies `unsafe_code`; each
//! justified use carries a per-item `expect`.

pub mod atmosphere;
pub mod body;
pub mod error;
pub mod generation;
pub mod hash;
pub mod orbit;
pub mod platform;
pub mod regime;
pub mod render;
pub mod rng;
pub mod sim;
pub mod surface;
pub mod trajectory;
pub mod units;
pub mod warp;
