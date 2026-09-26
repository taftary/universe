# Mobile — phones first

**Status:** M0 locked. Floor and downgrade behavior are binding.

## OS floors

- Android API level 26 minimum. Built with `cargo-ndk` and `GameActivity`, `minSdk = 26`.
- iOS 15 minimum. Built with the iOS target check in [quality.md](quality.md).

No API below these floors is used without a runtime probe and a fallback path.

## Frame pacer

- Frame pacer plus 30 fps limiter. The limiter constant is `FRAME_BUDGET_MS = 33.33 ms` derived from 30 fps; source: project budget in [quality.md](quality.md).
- Never run an unbounded update loop. Sim advances by fixed steps from an accumulator; render interpolates. Excess time is dropped with a counter, never spiraled.
- Generation and texture uploads run on worker threads via `std::thread` plus `mpsc`; see [architecture.md](architecture.md).

## Thermal management

- iOS polls `ProcessInfo.thermalState`. Android polls the thermal status API. Poll interval is `THERMAL_POLL_S = 2.0 s`, source: project choice recorded here.
- Tier downgrade engages before hardware throttling: High to Medium, then Medium to Low. Downgrade reduces rendering scale and texture density; sim behavior is unchanged.
- Follow the Apple Serious-tier pattern: at Serious level, drop to the Low tier immediately and notify the player with an instrument-grade message, not a blocking dialog.
- A 15-minute sustained session must hold the floor without a throttle-induced drop; see [quality.md](quality.md).

## Textures and GPU set

- ETC2 is the default compressed format (universal GLES support). ASTC packs ship for devices that expose ASTC.
- Conservative GPU feature set: no bindless, no 1.3-only features. Runtime probing selects the backend and the texture pack; missing features disable paths, never crash.
- Driver caveats: Pixel 10 PowerVR-class GPUs have aborted GPU-driven culling paths upstream; vulkano has a reported Android segfault on some drivers. Both are reasons for the wgpu-plus-probing choice in [stack.md](stack.md).

## Build and threads

- Android: `cargo-ndk` with `GameActivity`, `minSdk = 26`. iOS: standard cargo Apple targets with `cargo check` gates in [quality.md](quality.md).
- Few-core guidance: one sim thread, one render thread, plus a small generation pool sized to `available_parallelism - 2` with a minimum of one worker. Never assume more than four usable cores on the Low tier.
- No async runtime in the frame loop. Blocking worker queues only.

Related: [../tech.md](../tech.md), [stack.md](stack.md), [quality.md](quality.md), [architecture.md](architecture.md).
