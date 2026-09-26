# References — accepted and rejected sources

**Status:** M0 locked. Sources below informed the D-001..D-008 decisions in [../tech.md](../tech.md).

## Accepted

- The Rust Book, chapter 20 (final project and patterns). Baseline for workspace organization and testing habits.
- The Rustonomicon. Unsafe and layout rules behind [standards.md](standards.md).
- The Rust Performance Book. Allocation, layout, and profiling guidance.
- Canonical Rust best practices (official organization guides). Naming and module structure.
- Microsoft Rust guidelines. Error handling and API design input for D-007.
- Corgea 2026 Rust analysis. Dependency and unsafe-trend context.
- Bielefeld University Rust slides. Teaching reference for ownership and concurrency basics.
- dasifefe Rust resource list. Index used to cross-check the sources above.
- bevy.org documentation. Compared for the Bevy 0.19 evaluation in [stack.md](stack.md).
- Bevy GitHub issues #20998, #23754, #19358. Mobile support, f32 Transform limits, and release-cadence evidence cited in [stack.md](stack.md).
- Rustunit mobile framerate study. Sustained-rate evidence for the 30 fps floor in [quality.md](quality.md).
- Arm ASTC guide. Texture format choice in [mobile.md](mobile.md).
- Android texture compression documentation. ETC2 default rationale in [mobile.md](mobile.md).
- kvark wgpu notes (gfx-rs author). Backend behavior and GLES fallback context for D-003.

## Rejected (one-line reason each)

- nanotechinsight source 1: fabricated content, no verifiable provenance.
- nanotechinsight source 2: fabricated content, no verifiable provenance.
- w3reference: beginner-level summaries, insufficient depth for engine decisions.
- codelessgenie snippet set: contains undefined-behavior examples, unsafe to follow.
- webreference snippet set: contains undefined-behavior examples, unsafe to follow.
- Medium paywalled essays: not openly verifiable, cannot serve as a durable reference.
- General game-dev blogs: stale APIs with no mobile coverage, misleading for this budget.

Rule: a rejected source is never cited as evidence. If new evidence is needed, open a tech issue and propose a replacement source.

Related: [../tech.md](../tech.md), [stack.md](stack.md), [quality.md](quality.md).
