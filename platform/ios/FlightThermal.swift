import Foundation

/// Thermal poll interval in seconds.
/// Source: docs/tech/mobile.md thermal management (`THERMAL_POLL_S = 2.0 s`).
public let flightThermalPollIntervalS: Double = 2.0

/// Portable thermal state with CSV labels matching
/// `crates/debug/src/flight_log.rs` `ThermalState::label`.
public enum FlightThermalState: String, Equatable, CaseIterable {
    case nominal = "nominal"
    case fair = "fair"
    case serious = "serious"
    case critical = "critical"

    /// Map a `ProcessInfo.ThermalState` to the portable state.
    /// `ProcessInfo.thermalState` exists since iOS 11, so it is
    /// guaranteed on the iOS 15 floor with no probe needed.
    public init(processInfoState: ProcessInfo.ThermalState) {
        switch processInfoState {
        case .nominal:
            self = .nominal
        case .fair:
            self = .fair
        case .serious:
            self = .serious
        case .critical:
            self = .critical
        @unknown default:
            // Future states fail safe toward the hotter band so the
            // tier downgrade engages before hardware throttling.
            self = .serious
        }
    }
}

/// Render-only thermal tier with CSV labels matching
/// `crates/debug/src/budget.rs` `ThermalTier::label`.
/// Tier downgrade is render-only; sim behavior is identical across
/// tiers per docs/tech/quality.md and docs/tech/mobile.md.
public enum FlightRenderTier: String, Equatable, CaseIterable {
    case high = "high"
    case medium = "medium"
    case low = "low"
}

/// Map a thermal state plus current tier to the next render tier.
///
/// - Serious or Critical drops to Low immediately with an
///   instrument-grade notice, never a blocking dialog.
/// - Fair steps down one tier (High to Medium, Medium to Low).
/// - Nominal holds the current tier; upgrades are manual only so the
///   15-minute run never oscillates.
/// Source: docs/tech/mobile.md thermal management.
public func flightTier(
    for state: FlightThermalState,
    current: FlightRenderTier
) -> FlightRenderTier {
    switch state {
    case .serious, .critical:
        return .low
    case .fair:
        switch current {
        case .high:
            return .medium
        case .medium, .low:
            return .low
        }
    case .nominal:
        return current
    }
}

/// Read the current thermal state with the iOS 15 floor guard.
///
/// Uses only APIs guaranteed on iOS 15. Any newer API must add a
/// `#available` probe plus a fallback to `.nominal` per
/// docs/tech/mobile.md (no API below the floor without probe plus
/// fallback). On the simulator or when the state is unavailable the
/// fallback is `.nominal` with no tier change.
public func flightCurrentThermalState() -> FlightThermalState {
    if #available(iOS 15, *) {
        return FlightThermalState(
            processInfoState: ProcessInfo.processInfo.thermalState
        )
    } else {
        return .nominal
    }
}

/// Poll `ProcessInfo.thermalState` every `flightThermalPollIntervalS`
/// and report state plus tier.
///
/// Owned by the flight shell on the main thread. The timer never
/// drives sim state; it only feeds `FlightLog` samples and the
/// render-only tier. Invalidate on session end.
public final class FlightThermalPoller: NSObject {

    /// Latest observed thermal state.
    public private(set) var state: FlightThermalState = .nominal

    /// Latest render-only tier, starting at Medium (reference phone).
    public private(set) var tier: FlightRenderTier = .medium

    /// True once a Serious or Critical state forced Low this session.
    public private(set) var didForceLowBool: Bool = false

    /// Called on every poll with the new state, tier, and a flag that
    /// is true only on the poll that forced Low from Serious/Critical.
    public var onPoll: ((FlightThermalState, FlightRenderTier, Bool) -> Void)?

    private var timer: Timer?

    /// Start polling. Safe to call once; restarts after `stop()`.
    public func start() {
        stop()
        // Probe the floor before scheduling; below iOS 15 the poller
        // stays idle at nominal so the 15-minute CSV still records.
        guard #available(iOS 15, *) else {
            state = .nominal
            return
        }
        poll()
        timer = Timer.scheduledTimer(
            withTimeInterval: flightThermalPollIntervalS,
            repeats: true
        ) { [weak self] _ in
            self?.poll()
        }
    }

    /// Stop polling and release the timer.
    public func stop() {
        timer?.invalidate()
        timer = nil
    }

    private func poll() {
        let next = flightCurrentThermalState()
        let previousTier = tier
        let nextTier = flightTier(for: next, current: previousTier)
        let forcedLow = (next == .serious || next == .critical) && nextTier == .low
            && previousTier != .low
        if forcedLow {
            didForceLowBool = true
        }
        state = next
        tier = nextTier
        onPoll?(next, nextTier, forcedLow)
    }
}
