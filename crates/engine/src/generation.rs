//! Pure deterministic generation helpers: same seed, same output.
//!
//! No wall clock, no thread-dependent order. Procedural content is never
//! saved; generation replays from the seed.

/// `SplitMix64` gamma. Source: Steele et al., "Fast splittable pseudorandom
/// number generators" (2014).
const SPLITMIX_GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;

/// `SplitMix64` multiplier one. Source: same as [`SPLITMIX_GAMMA`].
const SPLITMIX_MULT_ONE: u64 = 0xBF58_476D_1CE4_E5B9;

/// `SplitMix64` multiplier two. Source: same as [`SPLITMIX_GAMMA`].
const SPLITMIX_MULT_TWO: u64 = 0x94D0_49BB_1331_11EB;

/// Mix a seed with a salt into a new 64-bit value.
///
/// Deterministic across runs and platforms. One `SplitMix64` round over the
/// salted seed; callers chain outputs for longer streams.
#[must_use]
pub fn mix_seed(seed: u64, salt: u64) -> u64 {
    let mut state = seed.wrapping_add(salt).wrapping_add(SPLITMIX_GAMMA);
    state = (state ^ (state >> 30)).wrapping_mul(SPLITMIX_MULT_ONE);
    state = (state ^ (state >> 27)).wrapping_mul(SPLITMIX_MULT_TWO);
    state ^ (state >> 31)
}
