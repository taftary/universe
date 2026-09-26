# References — accepted and rejected sources

**Status:** M0 locked. Sources below informed the D-001..D-008 decisions in [../tech.md](../tech.md).

## Accepted

- The Rust Book, chapter 20 "Advanced Features" (https://doc.rust-lang.org/book/ch20-00-advanced-features.html). Unsafe, advanced traits/types/functions/closures, and macros. Underpins the unit-newtype rule in [simulation.md](simulation.md) and the SAFETY-comment rule in [standards.md](standards.md).
- The Little Book of Rust Macros (https://danielkeep.github.io/tlborm/book/). Macro-authoring reference for the `impl_units!` boilerplate rule in [simulation.md](simulation.md).
- The Rustonomicon (https://doc.rust-lang.org/nomicon/). Unsafe and layout rules behind [standards.md](standards.md).
- The Rust Performance Book (https://nnethercote.github.io/perf-book/). Allocation, layout, and profiling guidance.
- Rust API Guidelines (https://rust-lang.github.io/api-guidelines/). Naming and module structure.
- Microsoft Rust guidelines (https://github.com/microsoft/rust-guidelines). Error handling and API design input for D-007.
- Corgea "Rust Best Practices 2026: Security, Idioms and Error Handling" (https://corgea.com/learn/rust-security-best-practices). Best-practices article: lints declared in `Cargo.toml`, `cargo audit` / `cargo deny` policy, unwrap policy. Context for [standards.md](standards.md) and [quality.md](quality.md).
- Bielefeld University Rust slides. Teaching reference for ownership and concurrency basics. No stable public URL found; retained pending a durable link (propose one via a tech issue).
- dasifefe Rust game-development frameworks list (https://github.com/dasifefe/rust-game-development-frameworks). Index used to cross-check the sources above.
- bevy.org documentation (https://bevy.org/learn/). Compared for the Bevy 0.19 evaluation in [stack.md](stack.md); source of the f32 `Transform` limit and the roughly 3-month release cadence cited there.
- Bevy GitHub issues #20998 (https://github.com/bevyengine/bevy/issues/20998, mobile support under-staffed), #23754 (https://github.com/bevyengine/bevy/issues/23754, Pixel 10 PowerVR-class GPU-driven-culling crash), #19358 (https://github.com/bevyengine/bevy/issues/19358, iOS 60 fps cap). Mobile-risk evidence cited in [stack.md](stack.md) and [mobile.md](mobile.md).
- Bevy PRs #23708 (https://github.com/bevyengine/bevy/pull/23708) and #23491 (https://github.com/bevyengine/bevy/pull/23491). Android activity feature work behind the `minSdk = 26` floor in [mobile.md](mobile.md).
- bevy_framepace (https://github.com/aevyrie/bevy_framepace). Frame pacing and frame limiting reference for the frame pacer in [mobile.md](mobile.md).
- bevy_ios_toolkit (https://docs.rs/bevy_ios_toolkit). Thermal state (`ProcessInfo.ThermalState`) plus Low Power Mode as a polled resource; reference for the Serious-tier downgrade pattern in [mobile.md](mobile.md).
- Rustunit "Bevy Efficiency on Mobile" (https://rustunit.com/blog/2025/01-02-bevy-mobile-framerate/). Default engine settings run an unbounded update loop (observed at unbounded fps with 200% CPU); evidence for the "never run an unbounded update loop" rule in [mobile.md](mobile.md).
- Android `PowerManager` thermal API docs (https://developer.android.com/reference/android/os/PowerManager#getCurrentThermalStatus()). `getCurrentThermalStatus` / `OnThermalStatusChangedListener` behind the Android thermal poll in [mobile.md](mobile.md).
- Apple `ProcessInfo` thermal state docs (https://developer.apple.com/documentation/foundation/processinfo/thermalstate). iOS thermal levels behind the tier downgrade in [mobile.md](mobile.md).
- Arm ASTC guide (https://developer.arm.com/documentation/102162/latest/). Texture format choice in [mobile.md](mobile.md).
- Android texture compression docs (https://developer.android.com/games/optimize/textures). ETC2 default rationale in [mobile.md](mobile.md).
- kvark wgpu notes (https://hackmd.io/@kvark/rust-gfx). Backend behavior and GLES fallback context for D-003.

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
