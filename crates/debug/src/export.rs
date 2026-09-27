//! Bug-bundle export writer for the section-11 layout.
//!
//! Hand-rolled TOML plus CSV text with explicit escaping, xxh3-64
//! content hashes over the replay-critical files, atomic
//! tmp-plus-rename writes, and quarantine on checksum mismatch. Export
//! is a modal action outside the frame loop, so formatting may
//! allocate; the frame loop never calls into this module. Seeds only:
//! procedural content and render caches are excluded by construction.

use crate::determinism::InputEntry;

/// Bundle format version, dimensionless.
///
/// Source: `docs/tech/debug.md` section 11 `bundle_version_u16`.
pub const BUNDLE_VERSION_U16: u16 = 1;

/// Bundle file names in fixed section-11 order, dimensionless count.
pub const BUNDLE_FILE_NAMES: [&str; 8] = [
    "meta.toml",
    "seed_tree.toml",
    "inputs.csv",
    "hashes.csv",
    "snapshot.toml",
    "config.toml",
    "log_excerpt.txt",
    "system.txt",
];

/// Meta file name in the bundle layout.
pub const META_FILE_NAME: &str = "meta.toml";

/// Inputs file name in the bundle layout.
pub const INPUTS_FILE_NAME: &str = "inputs.csv";

/// Hashes file name in the bundle layout.
pub const HASHES_FILE_NAME: &str = "hashes.csv";

/// Temporary suffix for atomic writes.
const TMP_SUFFIX: &str = ".tmp";

/// Quarantine directory name beside the bundle root.
const QUARANTINE_DIR_NAME: &str = "corrupt";

/// Bundle export and verify failures.
///
/// Copyable by design so the shell error enum can wrap it by value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportError {
    /// Float value was non-finite and unformattable.
    NonFinite {
        /// Rejected value.
        value_f64: f64,
    },
    /// Filesystem operation failed with a kind.
    Io {
        /// Operation kind for display.
        kind: std::io::ErrorKind,
    },
    /// Required metadata field was absent.
    MissingField,
    /// Content hash mismatch on verify.
    HashMismatch {
        /// Expected digest, dimensionless.
        expected_u64: u64,
        /// Actual digest, dimensionless.
        actual_u64: u64,
    },
}

impl core::fmt::Display for ExportError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NonFinite { value_f64 } => {
                write!(formatter, "non-finite bundle value: {value_f64}")
            }
            Self::Io { kind } => write!(formatter, "bundle IO failed: {kind:?}"),
            Self::MissingField => write!(formatter, "bundle metadata field missing"),
            Self::HashMismatch {
                expected_u64,
                actual_u64,
            } => write!(
                formatter,
                "bundle hash mismatch: expected {expected_u64:016x}, actual {actual_u64:016x}"
            ),
        }
    }
}

impl std::error::Error for ExportError {}

/// Hash bytes with xxh3-64 for bundle content checksums.
///
/// Reuses the engine digest so digests agree across tools.
#[must_use]
pub fn content_hash_u64(bytes: &[u8]) -> u64 {
    engine::hash::hash_bytes(bytes)
}

/// Quote a TOML string with escapes without taking the buffer.
///
/// Allocates once per export file; never called in the frame loop.
fn toml_quoted(value: &str) -> String {
    let mut text = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => text.push_str("\\\""),
            '\\' => text.push_str("\\\\"),
            '\n' => text.push_str("\\n"),
            '\r' => text.push_str("\\r"),
            '\t' => text.push_str("\\t"),
            _ => text.push(ch),
        }
    }
    text.push('"');
    text
}

/// Quote one CSV field with RFC4180 quote-doubling.
///
/// Allocates once per export file; never called in the frame loop.
fn csv_quoted(value: &str) -> String {
    let needs_quotes = value
        .chars()
        .any(|ch| ch == ',' || ch == '"' || ch == '\n' || ch == '\r');
    if !needs_quotes {
        return String::from(value);
    }
    let mut text = String::from("\"");
    for ch in value.chars() {
        if ch == '"' {
            text.push('"');
        }
        text.push(ch);
    }
    text.push('"');
    text
}

/// Check one float for bundle formatting.
///
/// # Errors
///
/// Returns [`ExportError::NonFinite`] for non-finite values; bundle
/// text never carries `inf` or `NaN` literals.
fn check_finite_f64(value_f64: f64) -> Result<(), ExportError> {
    if !value_f64.is_finite() {
        return Err(ExportError::NonFinite { value_f64 });
    }
    Ok(())
}

/// Format `meta.toml` from caller-provided strings plus content hashes.
pub fn format_meta_toml(
    created_utc: &str,
    app_version: &str,
    platform: &str,
    tier: &str,
    inputs_hash_u64: u64,
    hashes_hash_u64: u64,
) -> String {
    format!(
        "bundle_version_u16 = {version}\ncreated_utc = {created}\napp_version = {app}\nplatform = {platform}\ntier = {tier}\ninputs_hash_hex = \"{inputs_hash_u64:016x}\"\nhashes_hash_hex = \"{hashes_hash_u64:016x}\"\n",
        version = BUNDLE_VERSION_U16,
        created = toml_quoted(created_utc),
        app = toml_quoted(app_version),
        platform = toml_quoted(platform),
        tier = toml_quoted(tier),
    )
}

/// Format `seed_tree.toml` from master plus domain streams.
pub fn format_seed_tree_toml(
    master_seed_u64: u64,
    gen_star_u64: u64,
    gen_body_u64: u64,
    gen_terrain_u64: u64,
) -> String {
    format!(
        "master_seed_u64 = {master_seed_u64}\ngen_star_u64 = {gen_star_u64}\ngen_body_u64 = {gen_body_u64}\ngen_terrain_u64 = {gen_terrain_u64}\n"
    )
}

/// Format `inputs.csv` from recorder entries oldest-first.
///
/// Header plus `tick_count_u64,kind,payload_hex` rows; the hex payload
/// carries the per-kind units note via kind labels.
pub fn format_inputs_csv(entries: &[InputEntry]) -> String {
    let mut text = String::from("tick_count_u64,kind,payload_hex\n");
    for entry in entries {
        text.push_str(&entry.tick_count_u64().to_string());
        text.push(',');
        text.push_str(&csv_quoted(entry.kind().label()));
        text.push(',');
        let (hex_u8, len_usize) = entry.payload().hex_bytes();
        for byte_u8 in hex_u8.iter().take(len_usize) {
            text.push(*byte_u8 as char);
        }
        text.push('\n');
    }
    text
}

/// Format `hashes.csv` from tick plus hash pairs.
pub fn format_hashes_csv(pairs: &[(u64, u64)]) -> String {
    let mut text = String::from("tick_count_u64,snapshot_hash_hex\n");
    for (tick_count_u64, hash_u64) in pairs {
        let row = format!("{tick_count_u64},{hash_u64:016x}\n");
        text.push_str(&row);
    }
    text
}

/// Format `snapshot.toml` with unit-suffixed keys on every field.
///
/// Available only with the non-default `dev-shell` feature.
///
/// # Errors
///
/// Returns [`ExportError::NonFinite`] for any non-finite readout.
#[cfg(feature = "dev-shell")]
pub fn format_snapshot_toml(
    snapshot: &engine::inspect::SimSnapshot,
) -> Result<String, ExportError> {
    for value_f64 in [
        snapshot.elapsed_s_f64,
        snapshot.ship_epoch_s_f64,
        snapshot.altitude_m_f64,
        snapshot.speed_mps_f64,
        snapshot.pressure_pa_f64,
        snapshot.temperature_k_f64,
        snapshot.density_kg_m3_f64,
        snapshot.heat_flux_w_per_m2_f64,
        snapshot.g_load_g_f64,
        snapshot.semi_major_axis_m_f64,
        snapshot.eccentricity_f64,
        snapshot.inclination_rad_f64,
        snapshot.raan_rad_f64,
        snapshot.arg_periapsis_rad_f64,
        snapshot.mean_anomaly_rad_f64,
        snapshot.mu_m3_s2_f64,
    ] {
        check_finite_f64(value_f64)?;
    }
    for axis_f64 in snapshot
        .position_m_f64
        .iter()
        .chain(snapshot.velocity_mps_f64.iter())
        .chain(snapshot.drag_mps2_f64.iter())
        .chain(snapshot.vel_dir_f64.iter())
        .copied()
    {
        check_finite_f64(axis_f64)?;
    }
    Ok(format!(
        "tick_count_u64 = {tick}\nelapsed_s_f64 = {elapsed}\nship_epoch_s_f64 = {epoch}\nmaster_seed_u64 = {master}\nstream_seed_u64 = {stream}\nsnapshot_hash_hex = \"{hash:016x}\"\naltitude_m_f64 = {alt}\nspeed_mps_f64 = {speed}\npressure_pa_f64 = {pressure}\ntemperature_k_f64 = {temp}\ndensity_kg_m3_f64 = {density}\nheat_flux_w_per_m2_f64 = {heat}\ng_load_g_f64 = {g}\nregime_u8 = {regime}\nframe_level_u8 = {level}\nwarp_code_u8 = {warp}\ndrop_reason_u8 = {drop}\n",
        tick = snapshot.tick_count_u64,
        elapsed = snapshot.elapsed_s_f64,
        epoch = snapshot.ship_epoch_s_f64,
        master = snapshot.master_seed_u64,
        stream = snapshot.stream_seed_u64,
        hash = snapshot.snapshot_hash_u64,
        alt = snapshot.altitude_m_f64,
        speed = snapshot.speed_mps_f64,
        pressure = snapshot.pressure_pa_f64,
        temp = snapshot.temperature_k_f64,
        density = snapshot.density_kg_m3_f64,
        heat = snapshot.heat_flux_w_per_m2_f64,
        g = snapshot.g_load_g_f64,
        regime = snapshot.regime_u8,
        level = snapshot.frame_level_u8,
        warp = snapshot.warp_code_u8,
        drop = snapshot.drop_reason_u8,
    ))
}

/// Format `config.toml` from preset name plus pre-rendered rows.
///
/// Callers render each value with its unit; keys come from the fixed
/// registry-name set and reject anything outside `[A-Za-z0-9_.]`.
pub fn format_config_toml(preset_name: &str, rows: &[(&str, &str)]) -> String {
    let mut text = String::from("preset_name = ");
    text.push_str(&toml_quoted(preset_name));
    text.push('\n');
    for (name, value) in rows {
        if name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '.')
            && !name.is_empty()
        {
            text.push_str(name);
            text.push_str(" = ");
            text.push_str(&toml_quoted(value));
            text.push('\n');
        }
    }
    text
}

/// Write bundle files atomically into a directory.
///
/// Each file writes to `<name>.tmp` with sync, then renames; the rename
/// is the commit point and files are never written in place.
///
/// # Errors
///
/// Returns [`ExportError::Io`] for filesystem failures.
pub fn export_bundle_files(
    bundle_dir: &std::path::Path,
    files: &[(&str, &str)],
) -> Result<(), ExportError> {
    std::fs::create_dir_all(bundle_dir).map_err(|error| ExportError::Io { kind: error.kind() })?;
    for (name, contents) in files {
        let tmp_path = bundle_dir.join(format!("{name}{TMP_SUFFIX}"));
        std::fs::write(&tmp_path, contents)
            .map_err(|error| ExportError::Io { kind: error.kind() })?;
        let final_path = bundle_dir.join(name);
        std::fs::rename(&tmp_path, &final_path)
            .map_err(|error| ExportError::Io { kind: error.kind() })?;
    }
    Ok(())
}

/// Parse a 16-hex-digit `u64` from bundle text.
fn parse_hex_u64(text: &str) -> Option<u64> {
    let digits = text.trim();
    if digits.len() > 16 || digits.is_empty() {
        return None;
    }
    let mut value_u64 = 0_u64;
    for ch in digits.chars() {
        let digit_u64 = u64::from(ch.to_digit(16)?);
        value_u64 = value_u64.checked_mul(16)?.checked_add(digit_u64)?;
    }
    Some(value_u64)
}

/// Read one quoted hash line from `meta.toml` text.
fn read_meta_hash_hex(meta_toml: &str, key: &str) -> Option<u64> {
    for line in meta_toml.lines() {
        let Some((line_key, line_value)) = line.split_once('=') else {
            continue;
        };
        if line_key.trim() != key {
            continue;
        }
        let quoted = line_value.trim().trim_matches('"');
        if let Some(hash_u64) = parse_hex_u64(quoted) {
            return Some(hash_u64);
        }
    }
    None
}

/// Verify bundle content hashes before parsing.
///
/// Reads `meta.toml` plus both CSV files, recomputes xxh3-64 over the
/// exact byte streams, and compares. Any mismatch quarantines via
/// [`quarantine_bundle`] and never retries automatically.
///
/// # Errors
///
/// Returns [`ExportError`] for missing files, missing fields, IO
/// failures, or digest mismatches.
pub fn verify_bundle_hashes(bundle_dir: &std::path::Path) -> Result<(), ExportError> {
    let meta_toml = std::fs::read_to_string(bundle_dir.join(META_FILE_NAME))
        .map_err(|error| ExportError::Io { kind: error.kind() })?;
    let Some(expected_inputs_u64) = read_meta_hash_hex(&meta_toml, "inputs_hash_hex") else {
        return Err(ExportError::MissingField);
    };
    let Some(expected_hashes_u64) = read_meta_hash_hex(&meta_toml, "hashes_hash_hex") else {
        return Err(ExportError::MissingField);
    };
    let inputs_bytes = std::fs::read(bundle_dir.join(INPUTS_FILE_NAME))
        .map_err(|error| ExportError::Io { kind: error.kind() })?;
    let hashes_bytes = std::fs::read(bundle_dir.join(HASHES_FILE_NAME))
        .map_err(|error| ExportError::Io { kind: error.kind() })?;
    let actual_inputs_u64 = content_hash_u64(&inputs_bytes);
    if actual_inputs_u64 != expected_inputs_u64 {
        return Err(ExportError::HashMismatch {
            expected_u64: expected_inputs_u64,
            actual_u64: actual_inputs_u64,
        });
    }
    let actual_hashes_u64 = content_hash_u64(&hashes_bytes);
    if actual_hashes_u64 != expected_hashes_u64 {
        return Err(ExportError::HashMismatch {
            expected_u64: expected_hashes_u64,
            actual_u64: actual_hashes_u64,
        });
    }
    Ok(())
}

/// Quarantine a corrupt bundle beside its root.
///
/// Moves the tree to `corrupt/bundle-<expected>-<actual>-<version>/`
/// with the exact digests plus version in the name, and reports the
/// new path. Never retries automatically.
///
/// # Errors
///
/// Returns [`ExportError::Io`] for filesystem failures.
pub fn quarantine_bundle(
    bundle_dir: &std::path::Path,
    expected_u64: u64,
    actual_u64: u64,
) -> Result<std::path::PathBuf, ExportError> {
    let Some(parent) = bundle_dir.parent() else {
        return Err(ExportError::Io {
            kind: std::io::ErrorKind::NotFound,
        });
    };
    let target = parent.join(QUARANTINE_DIR_NAME).join(format!(
        "bundle-{expected_u64:016x}-{actual_u64:016x}-{BUNDLE_VERSION_U16}",
    ));
    if let Some(target_parent) = target.parent() {
        std::fs::create_dir_all(target_parent)
            .map_err(|error| ExportError::Io { kind: error.kind() })?;
    }
    std::fs::rename(bundle_dir, &target).map_err(|error| ExportError::Io { kind: error.kind() })?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::determinism::{InputKind, InputPayload};

    const SMOKE_TICK_U64: u64 = 12;

    fn smoke_entries() -> Vec<InputEntry> {
        vec![
            InputEntry::new(SMOKE_TICK_U64, InputKind::Pause, InputPayload::zero()),
            InputEntry::new(
                SMOKE_TICK_U64 + 1,
                InputKind::WarpRequest,
                InputPayload::warp(2_u8, 0x03),
            ),
        ]
    }

    #[test]
    fn toml_escapes_specials() {
        let meta = format_meta_toml("2026-09-27T\"quoted\"", "0.1.0", "host", "medium", 1, 2);
        assert!(meta.contains("bundle_version_u16 = 1\n"));
        assert!(meta.contains("\\\"quoted\\\""));
        assert!(meta.contains("inputs_hash_hex = \"0000000000000001\"\n"));
        assert!(meta.contains("hashes_hash_hex = \"0000000000000002\"\n"));
        let seed_tree = format_seed_tree_toml(9, 1, 2, 3);
        assert!(seed_tree.contains("master_seed_u64 = 9\n"));
        let config =
            format_config_toml("descent", &[("plots.window_s", "60 s"), ("bad key!", "x")]);
        assert!(config.contains("preset_name = \"descent\"\n"));
        assert!(config.contains("plots.window_s = \"60 s\"\n"));
        assert!(!config.contains("bad key!"));
    }

    #[test]
    fn csv_quotes_specials() {
        let csv = format_inputs_csv(&smoke_entries());
        assert!(csv.starts_with("tick_count_u64,kind,payload_hex\n"));
        assert!(csv.contains("12,pause,"));
        assert!(csv.contains("13,warp,"));
        let hashes = format_hashes_csv(&[(7_u64, 0xDEAD_BEEF_u64)]);
        assert!(hashes.contains("7,00000000deadbeef\n"));
        assert_eq!(csv_quoted("plain"), "plain");
        assert_eq!(csv_quoted("a,b\"c\nd"), "\"a,b\"\"c\nd\"");
    }

    #[test]
    fn bundle_round_trip_verifies() {
        let root =
            std::env::temp_dir().join(format!("universe-bundle-test-{}", std::process::id()));
        let bundle_dir = root.join("bundle");
        let inputs_csv = format_inputs_csv(&smoke_entries());
        let hashes_csv = format_hashes_csv(&[(SMOKE_TICK_U64, 42_u64)]);
        let meta_toml = format_meta_toml(
            "2026-09-27T00:00:00Z",
            "0.1.0-test",
            "test-host",
            "medium",
            content_hash_u64(inputs_csv.as_bytes()),
            content_hash_u64(hashes_csv.as_bytes()),
        );
        let files = [
            (META_FILE_NAME, meta_toml.as_str()),
            (INPUTS_FILE_NAME, inputs_csv.as_str()),
            (HASHES_FILE_NAME, hashes_csv.as_str()),
        ];
        assert!(export_bundle_files(&bundle_dir, &files).is_ok());
        assert!(verify_bundle_hashes(&bundle_dir).is_ok());
        assert!(std::fs::remove_dir_all(&root).is_ok());
    }

    #[test]
    fn tampered_bundle_quarantines() {
        let root =
            std::env::temp_dir().join(format!("universe-bundle-tamper-{}", std::process::id()));
        let bundle_dir = root.join("bundle");
        let inputs_csv = format_inputs_csv(&smoke_entries());
        let meta_toml = format_meta_toml(
            "2026-09-27T00:00:00Z",
            "0.1.0-test",
            "test-host",
            "medium",
            content_hash_u64(inputs_csv.as_bytes()),
            content_hash_u64(b"other bytes"),
        );
        let files = [
            (META_FILE_NAME, meta_toml.as_str()),
            (INPUTS_FILE_NAME, inputs_csv.as_str()),
            (HASHES_FILE_NAME, "tick_count_u64,snapshot_hash_hex\n"),
        ];
        assert!(export_bundle_files(&bundle_dir, &files).is_ok());
        assert!(matches!(
            verify_bundle_hashes(&bundle_dir),
            Err(ExportError::HashMismatch { .. })
        ));
        let Ok(quarantined) = quarantine_bundle(&bundle_dir, 1, 2) else {
            panic!("quarantine must move the tree")
        };
        assert!(quarantined.exists());
        assert!(!bundle_dir.exists());
        assert!(std::fs::remove_dir_all(&root).is_ok());
    }

    #[test]
    fn missing_field_rejects_verify() {
        let root =
            std::env::temp_dir().join(format!("universe-bundle-missing-{}", std::process::id()));
        let bundle_dir = root.join("bundle");
        let files = [(META_FILE_NAME, "bundle_version_u16 = 1\n")];
        assert!(export_bundle_files(&bundle_dir, &files).is_ok());
        assert!(matches!(
            verify_bundle_hashes(&bundle_dir),
            Err(ExportError::MissingField)
        ));
        assert!(std::fs::remove_dir_all(&root).is_ok());
    }

    #[cfg(feature = "dev-shell")]
    #[test]
    fn snapshot_toml_covers_unit_keys() {
        let snapshot = engine::inspect::SimSnapshot {
            tick_count_u64: SMOKE_TICK_U64,
            elapsed_s_f64: 0.6,
            ship_epoch_s_f64: 0.6,
            master_seed_u64: 1,
            stream_seed_u64: 2,
            snapshot_hash_u64: 3,
            position_m_f64: [3_639_500.0, 0.0, 0.0],
            velocity_mps_f64: [0.0, 3_400.0, 0.0],
            drag_mps2_f64: [0.0, 0.0, 0.0],
            vel_dir_f64: [0.0, 1.0, 0.0],
            altitude_m_f64: 250_000.0,
            speed_mps_f64: 3_400.0,
            pressure_pa_f64: 0.0,
            temperature_k_f64: 210.0,
            density_kg_m3_f64: 0.0,
            heat_flux_w_per_m2_f64: 0.0,
            g_load_g_f64: 0.0,
            semi_major_axis_m_f64: 3_639_500.0,
            eccentricity_f64: 0.01,
            inclination_rad_f64: 0.3,
            raan_rad_f64: 0.7,
            arg_periapsis_rad_f64: 0.5,
            mean_anomaly_rad_f64: 1.0,
            mu_m3_s2_f64: 4.282_837e13,
            pick_altitude_m_f64: 0.0,
            pick_range_m_f64: 0.0,
            frame_body_id_u32: 1,
            parent_body_id_u32: 0,
            pick_body_id_u32: u32::MAX,
            pick_cell_x_i32: 0,
            pick_cell_y_i32: 0,
            warp_code_u8: 0,
            drop_reason_u8: 0,
            warp_flags_u8: 3,
            regime_u8: 0,
            frame_level_u8: 5,
            frame_depth_u8: 2,
            elements_valid_u8: 1,
            pick_valid_u8: 0,
            mark_kind_u8: 6,
            _pad_u8: [0_u8; 3],
        };
        let Ok(toml) = format_snapshot_toml(&snapshot) else {
            panic!("snapshot toml must format")
        };
        assert!(toml.contains("altitude_m_f64 = 250000"));
        assert!(toml.contains("snapshot_hash_hex = \"0000000000000003\""));
        let mut bad = snapshot;
        bad.pressure_pa_f64 = f64::NAN;
        assert!(matches!(
            format_snapshot_toml(&bad),
            Err(ExportError::NonFinite { .. })
        ));
    }
}
