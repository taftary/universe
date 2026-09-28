# iOS flight shell (Step 2b, issue #56)

Same `universe-debug` core as desktop, plus an iOS shell that polls
`ProcessInfo.thermalState` and captures the 15-minute flight CSV.

## Floor

iOS 15 minimum (`MinimumOSVersion 15.0` in `Info.plist`).
No API below the floor is used without a runtime probe plus fallback:
only `Foundation` APIs guaranteed on iOS 15 (`ProcessInfo.thermalState`
since iOS 11, `Timer`, `FileManager`) are used. Any newer API must add
a `#available` probe with a fallback to `nominal` plus no tier change.
`flightCurrentThermalState()` shows the pattern.

## Files

- `Info.plist`: bundle identity plus `MinimumOSVersion 15.0` plus Metal
  capability (wgpu backend on iOS per `docs/tech/stack.md`).
- `FlightThermal.swift`: `flightThermalPollIntervalS = 2.0 s` poll
  (source `docs/tech/mobile.md`), `FlightThermalState` labels matching
  `crates/debug/src/flight_log.rs`, `FlightRenderTier` labels matching
  `crates/debug/src/budget.rs`, Serious/Critical to Low immediately,
  Fair steps down one tier, Nominal holds. `FlightThermalPoller`
  schedules the `Timer` on the main thread and is idle at nominal
  below iOS 15 so the CSV still records.
- `FlightRunner.swift`: 15-minute (`900.0 s`) session at 1 Hz
  (`flightCaptureIntervalS = 1.0 s`), gap limit `5.0 s` per AC1,
  header byte-identical to `FLIGHT_LOG_HEADER`, `system.txt` companion
  per `docs/tech/debug.md` section 11 (device, OS floor, backend,
  tier, poll cadence, downgrade plus gap logs).
- `crates/debug/build-ios.sh`: checks the shared core for
  `aarch64-apple-ios` (see below).

## Thermal and tier contract

Tier downgrade is render-only; sim behavior is identical across tiers
per `docs/tech/quality.md` and `docs/tech/mobile.md`. At Serious the
shell drops to Low immediately and posts an instrument-grade notice,
never a blocking dialog. Downgrades append to `tierLog`; gaps over
5 s append to `gapViolations`. Both ship in `system.txt`.

Fractions reuse the named budgets only: frame `33.33 ms`, sim avg
`8.0 ms`, sim p99 `16.0 ms`, hitch p95 `100.0 ms`, resident
`1024.0 MB`, cold start `5.0 s`. Gates live in `docs/tech/quality.md`.

## Build

```sh
./crates/debug/build-ios.sh
```

The script runs `cargo check -p universe-debug --features dev-shell
--target aarch64-apple-ios` (plus the default-member gate check).
It probes for the installed target and the Xcode SDK and prints the
fix when either is missing. No new Rust dependencies; `Cargo.lock`
is unchanged.

## Run

1. Build the core plus open this folder as the Xcode shell target.
2. Start the poller, then `begin()` the session on run start.
3. Push one `FlightRow` per second from the Rust `FlightLog::latest`
   sample (frame, sim avg, sim p99, hitch, resident, warp, seed, hash)
   with the poller state plus tier.
4. After 15 minutes (or on export) call `writeExports(to:)` for
   `flight.csv` plus `system.txt` into the debug bundle directory.

Live iOS run stays UNPROVEN on hosts without Xcode plus a device;
the contract above plus the Rust core tests are the gate here.
