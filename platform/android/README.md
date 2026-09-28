# Android flight shell (Step 2a)

On-device `universe-debug` host for the 15-minute 5-channel run in issue
#56. Headless-testable policy lives in `crates/debug/src/android.rs`;
this folder holds only packaging plus the build script.

## Build

```sh
crates/debug/build-android.sh
```

The script runs `cargo ndk` for `arm64-v8a` at platform 26 with the
`dev-shell` feature, then `cargo check` for the `aarch64-linux-android`
target. Packaging into an APK is a Gradle step outside this repo:

```sh
gradle assembleDebug
```

## Runtime map

| Brief item | Code | Notes |
| --- | --- | --- |
| cargo-ndk + GameActivity, minSdk 26 | `AndroidManifest.xml`, `build.gradle` | `androidx.games.activity.GameActivity`, lib `universe_debug` |
| Frame pacer + 30 fps limiter 33.33 ms | `FramePacer` in `android.rs` | Sleep clamps at zero; excess time drops with a counter |
| 1 sim + 1 render + pool avail-2 min 1 | `ThreadPlan` in `android.rs` | Totals above 4 exceed the Low tier assumption |
| PowerManager poll 2.0 s | `ThermalPoll` in `android.rs` | `OnThermalStatusChangedListener` where events beat polling |
| 15-minute CSV capture at 1 Hz | `FlightCapture` + `flight_log.rs` ring | 900 samples fit the 2048-entry ring; gaps above 5 s fail |
| No API below 26 without probe + fallback | `check_sdk`, `thermal_api_available` | Thermal status needs API 29; 26-28 use the temperature fallback |

Tier downgrade is render-only and identical sim behavior holds across
tiers: Serious plus Critical drop to Low immediately per
`docs/tech/mobile.md`; High is headroom only, never thermal-selected.

## JNI seam (Step 2c)

The `GameActivity` event loop plus the JNI clock, thermal, and file
bridge land in Step 2c (`crates/debug/src/os_window.rs` follow-up).
This step proves the policy headlessly; no new Rust dependencies were
added.
