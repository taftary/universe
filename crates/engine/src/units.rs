//! SI unit newtypes over `f64` for gameplay quantities.
//!
//! Raw `f64` never crosses a module boundary; these types carry the unit.
//! See `crate::sim` for consumers and `crate::error` for engine errors.
//!
//! # Example
//!
//! Same-type addition and subtraction preserve the unit.
//!
//! ```
//! # use engine::units::Meters;
//! let first_m = Meters::new(3.0);
//! let second_m = Meters::new(2.0);
//! assert!((first_m + second_m).value() == 5.0);
//! assert!((first_m - second_m).value() == 1.0);
//! ```
//!
//! Scalar scaling preserves the unit.
//!
//! ```
//! # use engine::units::Meters;
//! let distance_m = Meters::new(4.0);
//! assert!((distance_m * 2.0).value() == 8.0);
//! assert!((distance_m / 2.0).value() == 2.0);
//! ```
//!
//! Heterogeneous division converts distance over time into velocity.
//!
//! ```
//! # use engine::units::{Meters, Seconds};
//! let distance_m = Meters::new(100.0);
//! let duration_s = Seconds::new(10.0);
//! let speed_mps = distance_m / duration_s;
//! assert!(speed_mps.value() == 10.0);
//! ```
//!
//! Heterogeneous division converts velocity over time into acceleration.
//!
//! ```
//! # use engine::units::{MetersPerSecond, Seconds};
//! let speed_mps = MetersPerSecond::new(20.0);
//! let duration_s = Seconds::new(4.0);
//! let accel_mps2 = speed_mps / duration_s;
//! assert!(accel_mps2.value() == 5.0);
//! ```
//!
//! Ranged construction rejects values below absolute zero.
//!
//! ```
//! # use engine::units::{Kelvin, UnitError};
//! assert!(matches!(
//!     Kelvin::new(-1.0),
//!     Err(UnitError::BelowAbsoluteZero { .. })
//! ));
//! match Kelvin::new(-1.0) {
//!     Err(UnitError::BelowAbsoluteZero { value_kelvin_f64 }) => {
//!         assert!(value_kelvin_f64 == -1.0);
//!     }
//!     Ok(_) => assert!(false, "negative kelvin must fail"),
//! }
//! ```
//!
//! Valid temperatures round-trip through the accessor.
//!
//! ```
//! # use engine::units::Kelvin;
//! match Kelvin::new(273.15) {
//!     Ok(temp_k) => assert!(temp_k.value() == 273.15),
//!     Err(_) => assert!(false, "273.15 K must succeed"),
//! }
//! ```

use std::ops::{Add, AddAssign, Div, Mul, Sub, SubAssign};

use thiserror::Error;

/// Unit constructor failure for ranged SI quantities.
///
/// Currently only [`Kelvin`] is ranged; other units accept any magnitude.
#[derive(Debug, Error)]
pub enum UnitError {
    /// Kelvin value was negative and below absolute zero.
    #[error("temperature below absolute zero: {value_kelvin_f64} K")]
    BelowAbsoluteZero {
        /// Rejected temperature in kelvin.
        value_kelvin_f64: f64,
    },
}

/// Generate SI newtype boilerplate with typed operators.
///
/// Emits the struct, constructor, accessor, same-type arithmetic, scalar
/// scaling, and (via `@div` arms) explicit heterogeneous division. All
/// per-type impls flow through this macro; no hand-written impls exist.
macro_rules! impl_units {
    (@plain $(#[$type_meta:meta])* $Unit:ident($param:ident)) => {
        $(#[$type_meta])*
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        pub struct $Unit(f64);

        impl $Unit {
            /// Create a value from its SI magnitude; range is unbounded and construction never fails.
            ///
            /// Wraps the raw SI magnitude without validation.
            #[must_use]
            pub const fn new($param: f64) -> Self {
                Self($param)
            }

            /// Return the inner SI magnitude; range matches construction and never fails.
            ///
            /// Unwraps the newtype back to raw `f64` for solver internals.
            #[must_use]
            pub const fn value(self) -> f64 {
                self.0
            }
        }

        impl Add for $Unit {
            type Output = Self;
            fn add(self, rhs: Self) -> Self {
                Self(self.0 + rhs.0)
            }
        }

        impl Sub for $Unit {
            type Output = Self;
            fn sub(self, rhs: Self) -> Self {
                Self(self.0 - rhs.0)
            }
        }

        impl AddAssign for $Unit {
            fn add_assign(&mut self, rhs: Self) {
                self.0 += rhs.0;
            }
        }

        impl SubAssign for $Unit {
            fn sub_assign(&mut self, rhs: Self) {
                self.0 -= rhs.0;
            }
        }

        impl Mul<f64> for $Unit {
            type Output = Self;
            fn mul(self, factor: f64) -> Self {
                Self(self.0 * factor)
            }
        }

        impl Div<f64> for $Unit {
            type Output = Self;
            fn div(self, factor: f64) -> Self {
                Self(self.0 / factor)
            }
        }
    };
    (@ranged $(#[$type_meta:meta])* $Unit:ident($param:ident) const $(#[$const_meta:meta])* $Const:ident) => {
        $(#[$type_meta])*
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
        pub struct $Unit(f64);

        $(#[$const_meta])*
        pub const $Const: $Unit = $Unit(0.0);

        impl $Unit {
            /// Create kelvin from raw value; rejects negatives below absolute zero.
            ///
            /// # Errors
            ///
            /// Returns [`UnitError::BelowAbsoluteZero`] when `value_kelvin_f64` is negative.
            ///
            /// Compares against [`ABSOLUTE_ZERO_K`] so no literal appears outside the constant.
            pub fn new($param: f64) -> Result<Self, UnitError> {
                if $param < $Const.value() {
                    Err(UnitError::BelowAbsoluteZero {
                        value_kelvin_f64: $param,
                    })
                } else {
                    Ok(Self($param))
                }
            }

            /// Return the inner SI magnitude; range matches construction and never fails.
            ///
            /// Unwraps the newtype back to raw `f64` for solver internals.
            #[must_use]
            pub const fn value(self) -> f64 {
                self.0
            }
        }

        impl Add for $Unit {
            type Output = Self;
            fn add(self, rhs: Self) -> Self {
                Self(self.0 + rhs.0)
            }
        }

        impl Sub for $Unit {
            type Output = Self;
            fn sub(self, rhs: Self) -> Self {
                Self(self.0 - rhs.0)
            }
        }

        impl AddAssign for $Unit {
            fn add_assign(&mut self, rhs: Self) {
                self.0 += rhs.0;
            }
        }

        impl SubAssign for $Unit {
            fn sub_assign(&mut self, rhs: Self) {
                self.0 -= rhs.0;
            }
        }

        impl Mul<f64> for $Unit {
            type Output = Self;
            fn mul(self, factor: f64) -> Self {
                Self(self.0 * factor)
            }
        }

        impl Div<f64> for $Unit {
            type Output = Self;
            fn div(self, factor: f64) -> Self {
                Self(self.0 / factor)
            }
        }
    };
    (@div $Lhs:ident / $Rhs:ident -> $Out:ident) => {
        impl Div<$Rhs> for $Lhs {
            type Output = $Out;
            fn div(self, rhs: $Rhs) -> $Out {
                $Out(self.0 / rhs.0)
            }
        }
    };
}

impl_units!(@plain
    /// Distance in meters; range is unbounded and construction never fails.
    Meters(value_m)
);

impl_units!(@plain
    /// Duration in seconds; range is unbounded and construction never fails.
    Seconds(value_s)
);

impl_units!(@plain
    /// Mass in kilograms; range is unbounded and construction never fails.
    Kilograms(value_kg)
);

impl_units!(@ranged
    /// Temperature in kelvin; rejects negatives below absolute zero.
    Kelvin(value_kelvin_f64)
    const
    /// Absolute zero at 0 kelvin; lower values fail construction. Source: SI definition.
    ABSOLUTE_ZERO_K
);

impl_units!(@plain
    /// Pressure in pascals; range is unbounded and construction never fails.
    Pascals(value_pa)
);

impl_units!(@plain
    /// Velocity in meters per second; range is unbounded and construction never fails.
    MetersPerSecond(value_mps)
);

impl_units!(@plain
    /// Acceleration in meters per second squared; range is unbounded and construction never fails.
    MetersPerSecondSquared(value_mps2)
);

impl_units!(@div Meters / Seconds -> MetersPerSecond);

impl_units!(@div MetersPerSecond / Seconds -> MetersPerSecondSquared);
