//! Render backend seam. The concrete backend (`wgpu`, D-003) implements
//! [`Renderer`]; gameplay code never touches GPU types.

/// Render backend handle.
pub trait Renderer {
    /// Backend name for logs and diagnostics.
    fn name(&self) -> &'static str;
}
