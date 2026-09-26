//! Deterministic snapshot hashing with xxh3-64.
//!
//! Layouts are canonical little-endian; see [`scheduler_hash`].

use xxhash_rust::xxh3::xxh3_64;

/// Hash raw bytes with xxh3-64.
///
/// Same input bytes yield the same `u64` on every platform.
/// The hasher has no per-process seed, so digests are stable across runs.
///
/// # Example
///
/// ```
/// # use engine::hash::hash_bytes;
/// let first_u64 = hash_bytes(b"universe");
/// let second_u64 = hash_bytes(b"universe");
/// assert_eq!(first_u64, second_u64);
/// ```
///
/// # Example
///
/// One flipped bit changes the digest.
///
/// ```
/// # use engine::hash::hash_bytes;
/// let first_u64 = hash_bytes(&[0_u8, 1_u8]);
/// let second_u64 = hash_bytes(&[0_u8, 2_u8]);
/// assert_ne!(first_u64, second_u64);
/// ```
#[must_use]
pub fn hash_bytes(bytes: &[u8]) -> u64 {
    xxh3_64(bytes)
}

/// Hash scheduler state and seed deterministically.
///
/// Byte layout is fixed: `step_count` as `u64` little-endian (8 bytes),
/// then `step` bits as `u64` little-endian (8 bytes), then `elapsed`
/// bits as `u64` little-endian (8 bytes), then `seed` as `u64`
/// little-endian (8 bytes), for 32 bytes total hashed with xxh3-64.
/// `to_le_bytes` plus `to_bits` make the layout platform-independent;
/// float arithmetic cross-arch sameness is pinned by the `SimSnapshot`
/// golden test in `tests/smoke.rs`, which locks the `xxh3-64` digest over
/// `libm`-only stepping on `x86_64` and `AArch64`; this function pins
/// the scheduler byte layout only.
///
/// # Example
///
/// ```
/// # use engine::hash::scheduler_hash;
/// # use engine::sim::Scheduler;
/// let mut first = Scheduler::default();
/// let mut second = Scheduler::default();
/// first.advance();
/// second.advance();
/// let seed_u64 = 7_u64;
/// assert_eq!(
///     scheduler_hash(&first, seed_u64),
///     scheduler_hash(&second, seed_u64)
/// );
/// ```
#[must_use]
pub fn scheduler_hash(scheduler: &crate::sim::Scheduler, seed_u64: u64) -> u64 {
    let step_count_u64 = scheduler.step_count();
    let step_bits_u64 = scheduler.step().value().to_bits();
    let elapsed_bits_u64 = scheduler.elapsed().value().to_bits();
    let step_count_bytes = step_count_u64.to_le_bytes();
    let step_bits_bytes = step_bits_u64.to_le_bytes();
    let elapsed_bits_bytes = elapsed_bits_u64.to_le_bytes();
    let seed_bytes = seed_u64.to_le_bytes();
    let mut buffer_u8 = [0_u8; 32];
    buffer_u8[0..8].copy_from_slice(&step_count_bytes);
    buffer_u8[8..16].copy_from_slice(&step_bits_bytes);
    buffer_u8[16..24].copy_from_slice(&elapsed_bits_bytes);
    buffer_u8[24..32].copy_from_slice(&seed_bytes);
    xxh3_64(&buffer_u8)
}
