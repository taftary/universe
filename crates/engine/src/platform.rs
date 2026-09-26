//! Platform seams: clock, filesystem, thermal. Callers use these traits;
//! concrete implementations are injected and tests inject fakes.

/// Wall-clock source.
pub trait PlatformClock {
    /// Current time in seconds; arbitrary epoch, monotonic.
    fn now_s(&self) -> f64;
}

/// File reader.
pub trait PlatformFs {
    /// Read a file into memory.
    ///
    /// # Errors
    ///
    /// Returns the platform IO error unchanged.
    fn read_bytes(&self, path: &str) -> Result<Vec<u8>, std::io::Error>;
}

/// Thermal source.
pub trait PlatformThermal {
    /// Device temperature in degrees Celsius, when available.
    fn temperature_c(&self) -> Option<f32>;
}
