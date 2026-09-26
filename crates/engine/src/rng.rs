//! Project-owned deterministic RNG over `xoshiro256**` with domain-split seeds.
//!
//! Simulation and generation code never touches device randomness. [`SeedTree`]
//! splits one dimensionless `master_seed_u64` into three independent streams
//! (star, body, terrain) via [`split_domain`]; each stream seeds a
//! [`ProjectRng`]. See `docs/tech/simulation.md` for the determinism rules.

use core::convert::Infallible;

use rand_core::{SeedableRng, TryRng};
use rand_xoshiro::Xoshiro256PlusPlus;

/// Star generation domain tag, dimensionless.
///
/// Source: domain separation tags, project-chosen, fixed.
pub const GEN_STAR_TAG_U64: u64 = 0x8A2F_9C4B_1D3E_5F07;

/// Body generation domain tag, dimensionless.
///
/// Source: domain separation tags, project-chosen, fixed.
pub const GEN_BODY_TAG_U64: u64 = 0x3C7D_1A9E_6B4F_82D5;

/// Terrain generation domain tag, dimensionless.
///
/// Source: domain separation tags, project-chosen, fixed.
pub const GEN_TERRAIN_TAG_U64: u64 = 0xF159_2E6C_4A8B_D0C3;

/// Split a master seed into one domain stream.
///
/// Thin wrapper over [`crate::generation::mix_seed`]; dimensionless `u64` in
/// and out. Each domain mixes independently, so changing one tag never changes
/// another domain's stream.
#[must_use]
pub fn split_domain(master_seed_u64: u64, tag_u64: u64) -> u64 {
    crate::generation::mix_seed(master_seed_u64, tag_u64)
}

/// Hierarchical seed tree with three independent domain streams.
///
/// Each stream derives from `master_seed_u64` by one independent
/// [`split_domain`] call, so changing one tag never changes another stream.
/// All values are dimensionless `u64`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeedTree {
    /// Master seed every stream derives from, dimensionless.
    pub master_seed_u64: u64,
    /// Star-domain stream seed, dimensionless.
    pub gen_star_u64: u64,
    /// Body-domain stream seed, dimensionless.
    pub gen_body_u64: u64,
    /// Terrain-domain stream seed, dimensionless.
    pub gen_terrain_u64: u64,
}

impl SeedTree {
    /// Derive all three domain streams from one master seed.
    ///
    /// Deterministic: the same dimensionless `master_seed_u64` always yields
    /// the same tree on every platform.
    #[must_use]
    pub fn new(master_seed_u64: u64) -> Self {
        Self {
            master_seed_u64,
            gen_star_u64: split_domain(master_seed_u64, GEN_STAR_TAG_U64),
            gen_body_u64: split_domain(master_seed_u64, GEN_BODY_TAG_U64),
            gen_terrain_u64: split_domain(master_seed_u64, GEN_TERRAIN_TAG_U64),
        }
    }

    /// Build the star-domain RNG from this tree.
    ///
    /// Reads only `gen_star_u64`; other streams stay untouched.
    #[must_use]
    pub fn star_rng(&self) -> ProjectRng {
        ProjectRng::from_seed_u64(self.gen_star_u64)
    }

    /// Build the body-domain RNG from this tree.
    ///
    /// Reads only `gen_body_u64`; other streams stay untouched.
    #[must_use]
    pub fn body_rng(&self) -> ProjectRng {
        ProjectRng::from_seed_u64(self.gen_body_u64)
    }

    /// Build the terrain-domain RNG from this tree.
    ///
    /// Reads only `gen_terrain_u64`; other streams stay untouched.
    #[must_use]
    pub fn terrain_rng(&self) -> ProjectRng {
        ProjectRng::from_seed_u64(self.gen_terrain_u64)
    }
}

/// Project deterministic RNG over `xoshiro256**`.
///
/// Seeded only via [`SeedTree`] or [`ProjectRng::from_seed_u64`], never from
/// device randomness. Stream construction allocates nothing beyond fixed-size
/// state and never depends on thread order.
///
/// Implements [`TryRng`] with an infallible error type, so the infallible
/// [`Rng`](rand_core::Rng) methods (`next_u32`, `next_u64`, `fill_bytes`) apply
/// automatically via the blanket implementation. `rand_core::RngCore` is the
/// deprecated alias for `Rng` and is never used directly.
#[derive(Debug, Clone)]
pub struct ProjectRng(Xoshiro256PlusPlus);

impl ProjectRng {
    /// Create a deterministic RNG from one seed value.
    ///
    /// Deterministic for the locked `rand_xoshiro` version; the seed is a
    /// dimensionless `u64`.
    #[must_use]
    pub fn from_seed_u64(seed_u64: u64) -> Self {
        Self(Xoshiro256PlusPlus::seed_from_u64(seed_u64))
    }
}

impl TryRng for ProjectRng {
    type Error = Infallible;

    fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
        self.0.try_next_u32()
    }

    fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
        self.0.try_next_u64()
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Self::Error> {
        self.0.try_fill_bytes(dest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::{Rng, TryRng};

    #[test]
    fn same_master_yields_equal_trees() {
        let first = SeedTree::new(1234);
        let second = SeedTree::new(1234);
        assert_eq!(first, second);
        assert_eq!(first.master_seed_u64, 1234);
    }

    #[test]
    fn different_masters_yield_different_trees() {
        let first = SeedTree::new(1234);
        let second = SeedTree::new(5678);
        assert_ne!(first, second);
        assert_ne!(first.gen_star_u64, second.gen_star_u64);
        assert_ne!(first.gen_body_u64, second.gen_body_u64);
        assert_ne!(first.gen_terrain_u64, second.gen_terrain_u64);
    }

    #[test]
    fn domain_tags_are_distinct() {
        assert_ne!(GEN_STAR_TAG_U64, GEN_BODY_TAG_U64);
        assert_ne!(GEN_STAR_TAG_U64, GEN_TERRAIN_TAG_U64);
        assert_ne!(GEN_BODY_TAG_U64, GEN_TERRAIN_TAG_U64);
    }

    #[test]
    fn domains_split_independently() {
        let master_seed_u64 = 99;
        let tree = SeedTree::new(master_seed_u64);
        assert_eq!(
            split_domain(master_seed_u64, GEN_STAR_TAG_U64),
            tree.gen_star_u64
        );
        assert_eq!(
            split_domain(master_seed_u64, GEN_BODY_TAG_U64),
            tree.gen_body_u64
        );
        assert_eq!(
            split_domain(master_seed_u64, GEN_TERRAIN_TAG_U64),
            tree.gen_terrain_u64
        );
        let modified_body_u64 = split_domain(master_seed_u64, GEN_BODY_TAG_U64 ^ 0xFFFF);
        assert_ne!(modified_body_u64, tree.gen_body_u64);
        assert_eq!(
            split_domain(master_seed_u64, GEN_STAR_TAG_U64),
            tree.gen_star_u64
        );
        assert_eq!(
            split_domain(master_seed_u64, GEN_TERRAIN_TAG_U64),
            tree.gen_terrain_u64
        );
    }

    #[test]
    fn same_seed_yields_equal_streams() {
        let mut first = ProjectRng::from_seed_u64(7);
        let mut second = ProjectRng::from_seed_u64(7);
        for _ in 0..4 {
            assert_eq!(first.next_u64(), second.next_u64());
        }
    }

    #[test]
    fn different_seeds_yield_different_streams() {
        let mut first = ProjectRng::from_seed_u64(7);
        let mut second = ProjectRng::from_seed_u64(8);
        assert_ne!(first.next_u64(), second.next_u64());
    }

    #[test]
    fn tree_conveniences_match_direct_seeding() {
        let tree = SeedTree::new(42);
        let mut via_tree = tree.star_rng();
        let mut direct = ProjectRng::from_seed_u64(tree.gen_star_u64);
        assert_eq!(via_tree.next_u64(), direct.next_u64());
        let mut via_tree = tree.body_rng();
        let mut direct = ProjectRng::from_seed_u64(tree.gen_body_u64);
        assert_eq!(via_tree.next_u64(), direct.next_u64());
        let mut via_tree = tree.terrain_rng();
        let mut direct = ProjectRng::from_seed_u64(tree.gen_terrain_u64);
        assert_eq!(via_tree.next_u64(), direct.next_u64());
    }

    #[test]
    fn fill_bytes_matches_between_equal_rngs() {
        let mut first = ProjectRng::from_seed_u64(11);
        let mut second = ProjectRng::from_seed_u64(11);
        let mut first_bytes = [0_u8; 32];
        let mut second_bytes = [0_u8; 32];
        first.fill_bytes(&mut first_bytes);
        second.fill_bytes(&mut second_bytes);
        assert_eq!(first_bytes, second_bytes);
        assert!(second.try_fill_bytes(&mut second_bytes).is_ok());
        assert_ne!(first_bytes, second_bytes);
    }
}
