import Foundation

/// Flight session duration in seconds (15 minutes).
/// Source: docs/tech/quality.md thermal row (15-minute sustained session).
public let flightSessionDurationS: Double = 900.0

/// Flight capture cadence in seconds (1 Hz floor).
/// AC1 requires 5 channels plus tier at >= 1 Hz with no gaps > 5 s.
public let flightCaptureIntervalS: Double = 1.0

/// Largest allowed gap between consecutive CSV rows in seconds.
/// Any gap above this value fails AC1; the runner records it.
public let flightMaxGapS: Double = 5.0

/// CSV header in fixed column order, byte-identical to
/// `crates/debug/src/flight_log.rs` `FLIGHT_LOG_HEADER`.
public let flightCSVHeader: String =
    "tick_count_u64,elapsed_s_f64,frame_ms_f64,sim_avg_ms_f64," +
    "sim_p99_ms_f64,hitch_p95_ms_f64,resident_mb_f64," +
    "thermal_state,tier,warp_factor_f64,seed_u64,hash_u64\n"

/// One captured flight row in header order.
///
/// All channel values use the same units as `FlightSample`:
/// milliseconds for frame/sim/hitch, megabytes for resident, seconds
/// for elapsed, dimensionless warp/seed/hash. Thermal plus tier carry
/// the `FlightThermalState` and `FlightRenderTier` labels so the CSV
/// joins 1:1 with the Rust `format_row_csv` output.
public struct FlightRow {
    public var tickU64: UInt64
    public var elapsedS: Double
    public var frameMs: Double
    public var simAvgMs: Double
    public var simP99Ms: Double
    public var hitchMs: Double
    public var residentMb: Double
    public var thermal: FlightThermalState
    public var tier: FlightRenderTier
    public var warpFactor: Double
    public var seedU64: UInt64
    public var hashU64: UInt64

    public init(
        tickU64: UInt64,
        elapsedS: Double,
        frameMs: Double,
        simAvgMs: Double,
        simP99Ms: Double,
        hitchMs: Double,
        residentMb: Double,
        thermal: FlightThermalState,
        tier: FlightRenderTier,
        warpFactor: Double,
        seedU64: UInt64,
        hashU64: UInt64
    ) {
        self.tickU64 = tickU64
        self.elapsedS = elapsedS
        self.frameMs = frameMs
        self.simAvgMs = simAvgMs
        self.simP99Ms = simP99Ms
        self.hitchMs = hitchMs
        self.residentMb = residentMb
        self.thermal = thermal
        self.tier = tier
        self.warpFactor = warpFactor
        self.seedU64 = seedU64
        self.hashU64 = hashU64
    }

    /// Format one row in header order with the hash as 16-digit hex,
    /// matching `FlightLog::format_row_csv`.
    public func csvRow() -> String {
        let hashText = String(format: "%016llx", hashU64)
        return "\(tickU64),\(elapsedS),\(frameMs),\(simAvgMs)," +
            "\(simP99Ms),\(hitchMs),\(residentMb)," +
            "\(thermal.rawValue),\(tier.rawValue)," +
            "\(warpFactor),\(seedU64),\(hashText)\n"
    }
}

/// Append-only 15-minute flight session over the shared core.
///
/// The Rust core (`universe-debug` binary, same target as desktop)
/// owns the sim plus `FlightLog` ring; this runner owns the iOS shell
/// duties: 1 Hz row capture for 15 minutes, gap detection per AC1,
/// Serious-to-Low tier logging per AC3, and CSV plus `system.txt`
/// export per docs/tech/debug.md section 11. No new Rust dependency.
public final class FlightSession {

    /// Captured rows oldest-first.
    public private(set) var rows: [FlightRow] = []

    /// Tier-downgrade log lines (`elapsed_s`, old tier, new tier, state).
    public private(set) var tierLog: [String] = []

    /// Gap violations (`elapsed_s` plus gap length) for AC1 review.
    public private(set) var gapViolations: [String] = []

    /// Session start wall time; nil before `begin()`.
    public private(set) var startDate: Date?

    /// Reserve the 15-minute run at 1 Hz with margin, mirroring the
    /// `FLIGHT_LOG_CAPACITY_ENTRIES_USIZE = 2048` reservation.
    public init() {
        rows.reserveCapacity(1024)
    }

    /// Start a fresh session, clearing prior rows and logs.
    public func begin(now: Date = Date()) {
        rows.removeAll(keepingCapacity: true)
        tierLog.removeAll(keepingCapacity: true)
        gapViolations.removeAll(keepingCapacity: true)
        startDate = now
    }

    /// Record one row plus tier and gap bookkeeping.
    ///
    /// - Returns true when the row keeps the AC1 cadence (gap <= 5 s).
    @discardableResult
    public func record(_ row: FlightRow) -> Bool {
        if let previous = rows.last {
            let gapS = row.elapsedS - previous.elapsedS
            if gapS > flightMaxGapS {
                gapViolations.append(
                    "gap elapsed_s=\(row.elapsedS) gap_s=\(gapS) " +
                    "prev_tick=\(previous.tickU64) tick=\(row.tickU64)"
                )
                rows.append(row)
                return false
            }
            if row.tier != previous.tier {
                tierLog.append(
                    "tier elapsed_s=\(row.elapsedS) " +
                    "\(previous.tier.rawValue)->\(row.tier.rawValue) " +
                    "thermal=\(row.thermal.rawValue)"
                )
            }
        }
        rows.append(row)
        return true
    }

    /// Report whether the session holds the AC1 cadence so far.
    public var holdsCadenceBool: Bool {
        gapViolations.isEmpty
    }

    /// Report whether the 15-minute window is complete at 1 Hz.
    public var isCompleteBool: Bool {
        guard let first = rows.first, let last = rows.last else {
            return false
        }
        return (last.elapsedS - first.elapsedS) >= flightSessionDurationS
            && rows.count >= Int(flightSessionDurationS / flightCaptureIntervalS)
    }

    /// Format header plus rows oldest-first as append-only CSV.
    public func formatCSV() -> String {
        var text = flightCSVHeader
        for row in rows {
            text += row.csvRow()
        }
        return text
    }

    /// Format the section-11 `system.txt` companion for the bundle.
    ///
    /// Fields: device model, OS floor, backend, thermal tier summary,
    /// plus frame-time summary hooks the Rust `FlightLog` fractions
    /// fill from the named budgets (33.33/8/16/100/1024/5).
    public func formatSystemTxt(
        deviceModel: String,
        osVersion: String,
        backend: String = "metal",
        tier: FlightRenderTier
    ) -> String {
        let forced = tierLog.isEmpty ? "none" : tierLog.joined(separator: "; ")
        let gaps = gapViolations.isEmpty ? "none" : gapViolations.joined(separator: "; ")
        return "device_model = \"\(deviceModel)\"\n" +
            "os_floor = \"iOS 15\"\n" +
            "os_version = \"\(osVersion)\"\n" +
            "backend = \"\(backend)\"\n" +
            "tier = \"\(tier.rawValue)\"\n" +
            "thermal_poll_s = \(flightThermalPollIntervalS)\n" +
            "capture_hz = \(1.0 / flightCaptureIntervalS)\n" +
            "rows = \(rows.count)\n" +
            "tier_downgrades = \"\(forced)\"\n" +
            "gaps_over_5s = \"\(gaps)\"\n"
    }

    /// Write `flight.csv` plus `system.txt` atomically to a directory.
    public func writeExports(
        to directory: URL,
        deviceModel: String,
        osVersion: String,
        tier: FlightRenderTier,
        fileManager: FileManager = .default
    ) throws {
        try fileManager.createDirectory(
            at: directory,
            withIntermediateDirectories: true
        )
        let csvURL = directory.appendingPathComponent("flight.csv")
        let systemURL = directory.appendingPathComponent("system.txt")
        let csvText = formatCSV()
        let systemText = formatSystemTxt(
            deviceModel: deviceModel,
            osVersion: osVersion,
            tier: tier
        )
        try csvText.write(to: csvURL, atomically: true, encoding: .utf8)
        try systemText.write(to: systemURL, atomically: true, encoding: .utf8)
    }
}
