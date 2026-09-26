//! Universe workspace root: version constant and headless sim smoke tests.
//!
//! Simulation lives in the `engine` crate; gameplay wiring lives in `game`.
//! This package exists so `tests/` has a home at the workspace root.

#![forbid(unsafe_code)]

/// Workspace version, mirroring the package version.
pub const WORKSPACE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::WORKSPACE_VERSION;

    /// Version string is stamped and non-empty.
    #[test]
    fn workspace_version_is_stamped() {
        assert!(!WORKSPACE_VERSION.is_empty());
    }
}
