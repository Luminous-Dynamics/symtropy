use csv::{ReaderBuilder, StringRecord, Trim};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, env, fs, io::Read, path::Path, process::ExitCode};
use zip::ZipArchive;

const EXPECTED_INTERVALS: usize = 96;
const INTERVAL_HOURS: f64 = 0.25;
const CLAIM_CEILING: &str = "Integrity and descriptive audit of published modeled aggregate depot charging load profiles. These traces are 15-minute fleet-level simulation outputs, not measured individual charging sessions. The audit does not reproduce or validate the upstream controller, prove grid feasibility, compare controllers under matched constraints, or establish South African fleet performance.";

const FLEETS: [&str; 3] = [
    "fleet1-beverage-delivery",
    "fleet2-warehouse-delivery",
    "fleet3-food-delivery",
];
const FLEET_SIZES: [usize; 3] = [10, 50, 100];
const STRATEGIES: [&str; 3] = ["immediate", "delayed", "min_power"];
const PROFILE_CLASSES: [&str; 3] = ["average", "min_peak", "peak_peak"];

#[derive(Debug, Clone)]
struct ProfileMetadata {
    filename: String,
    fleet_id: String,
    fleet_size: usize,
    strategy: String,
    profile_class: String,
}

#[derive(Debug, Serialize)]
struct ProfileSummary {
    filename: String,
    internal_csv_sha256: String,
    fleet_id: String,
    fleet_size: usize,
    strategy: String,
    profile_class: String,
    resolution_minutes: u32,
    interval_count: usize,
    first_interval_start: String,
    last_interval_start: String,
    total_energy_kwh: f64,
    peak_power_kw: f64,
    mean_power_kw: f64,
    nonzero_interval_count: usize,
    zero_interval_count: usize,
    first_nonzero_interval_start: Option<String>,
    last_nonzero_interval_start: Option<String>,
}

#[derive(Debug, Serialize)]
struct AuditFinding {
    filename: String,
    reason: String,
}

#[derive(Debug, Serialize)]
struct Report {
    schema_version: String,
    tool: String,
    dataset_id: String,
    source_uri: String,
    source_version_declared_by_caller: String,
    evidence_class: String,
    source_zip_sha256: String,
    source_zip_bytes: u64,
    zip_entry_count: usize,
    csv_entry_count: usize,
    ignored_macos_sidecar_count: usize,
    expected_profile_count: usize,
    valid_profile_count: usize,
    invalid_profile_count: usize,
    unique_profile_key_count: usize,
    missing_profile_keys: Vec<String>,
    duplicate_profile_keys: Vec<String>,
    findings: Vec<AuditFinding>,
    profiles: Vec<ProfileSummary>,
    valid: bool,
    claim_ceiling: String,
    provenance_warning: String,
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn parse_profile_filename(filename: &str) -> Result<ProfileMetadata, String> {
    let basename = Path::new(filename)
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "filename_is_not_utf8_or_has_no_basename".to_string())?;
    let without_suffix = basename
        .strip_suffix(".csv")
        .ok_or_else(|| "profile_filename_must_end_in_csv".to_string())?;
    let without_resolution = without_suffix
        .strip_suffix("_15min")
        .ok_or_else(|| "profile_filename_must_use_15min_resolution".to_string())?;

    let (without_strategy, strategy) = [
        ("_min_power", "min_power"),
        ("_immediate", "immediate"),
        ("_delayed", "delayed"),
    ]
    .into_iter()
    .find_map(|(suffix, strategy)| {
        without_resolution
            .strip_suffix(suffix)
            .map(|prefix| (prefix, strategy))
    })
    .ok_or_else(|| "unsupported_or_missing_strategy_suffix".to_string())?;

    let parts: Vec<&str> = without_strategy.split('_').collect();
    if parts.len() != 3 {
        return Err("profile_filename_must_contain_fleet_size_and_profile_class".to_string());
    }
    let fleet_id = parts[0];
    if !FLEETS.contains(&fleet_id) {
        return Err(format!("unknown_fleet_id:{fleet_id}"));
    }
    let size_text = parts[1]
        .strip_suffix("vehs")
        .ok_or_else(|| "fleet_size_component_must_end_in_vehs".to_string())?;
    let fleet_size = size_text
        .parse::<usize>()
        .map_err(|_| "fleet_size_component_must_be_an_integer".to_string())?;
    if !FLEET_SIZES.contains(&fleet_size) {
        return Err(format!("unsupported_fleet_size:{fleet_size}"));
    }
    let profile_class = match parts[2] {
        "avg-prof" => "average",
        "min-prof" => "min_peak",
        "peak-prof" => "peak_peak",
        other => return Err(format!("unsupported_profile_class:{other}")),
    };
    if !STRATEGIES.contains(&strategy) {
        return Err(format!("unsupported_strategy:{strategy}"));
    }
    Ok(ProfileMetadata {
        filename: basename.to_string(),
        fleet_id: fleet_id.to_string(),
        fleet_size,
        strategy: strategy.to_string(),
        profile_class: profile_class.to_string(),
    })
}

fn profile_key(fleet: &str, size: usize, strategy: &str, profile_class: &str) -> String {
    format!("{fleet}|{size}|{strategy}|{profile_class}")
}

fn expected_profile_keys() -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for fleet in FLEETS {
        for size in FLEET_SIZES {
            for strategy in STRATEGIES {
                for profile_class in PROFILE_CLASSES {
                    keys.insert(profile_key(fleet, size, strategy, profile_class));
                }
            }
        }
    }
    keys
}

fn column(headers: &StringRecord, name: &str) -> Result<usize, String> {
    headers
        .iter()
        .position(|header| header.trim() == name)
        .ok_or_else(|| format!("required_column_missing:{name}"))
}

fn parse_power(text: &str) -> Result<f64, String> {
    let value = text
        .trim()
        .parse::<f64>()
        .map_err(|_| "power_value_not_numeric".to_string())?;
    if !value.is_finite() || value < 0.0 {
        return Err("power_value_must_be_finite_and_nonnegative".to_string());
    }
    Ok(value)
}

fn expected_time(index: usize) -> String {
    format!("{:02}:{:02}:00", index / 4, (index % 4) * 15)
}

fn audit_csv(
    filename: &str,
    bytes: &[u8],
    metadata: &ProfileMetadata,
) -> Result<ProfileSummary, String> {
    let mut reader = ReaderBuilder::new()
        .trim(Trim::All)
        .flexible(false)
        .from_reader(bytes);
    let headers = reader
        .headers()
        .map_err(|error| format!("csv_header_parse_error:{error}"))?
        .clone();
    let time_column = column(&headers, "time")?;
    let power_column = ["avg_power_kw", "power_kW", "power_kw"]
        .iter()
        .find_map(|name| headers.iter().position(|header| header == *name))
        .ok_or_else(|| "required_power_column_missing:avg_power_kw".to_string())?;

    let mut power_values = Vec::with_capacity(EXPECTED_INTERVALS);
    let mut time_values = Vec::with_capacity(EXPECTED_INTERVALS);
    for (index, row) in reader.records().enumerate() {
        let row = row.map_err(|error| format!("csv_row_{index}_parse_error:{error}"))?;
        let time = row
            .get(time_column)
            .ok_or_else(|| format!("row_{index}_time_missing"))?
            .to_string();
        let expected = expected_time(index);
        if time != expected {
            return Err(format!(
                "row_{index}_time_sequence_mismatch:expected={expected},actual={time}"
            ));
        }
        let power = parse_power(
            row.get(power_column)
                .ok_or_else(|| format!("row_{index}_power_missing"))?,
        )
        .map_err(|reason| format!("row_{index}:{reason}"))?;
        time_values.push(time);
        power_values.push(power);
    }
    if power_values.len() != EXPECTED_INTERVALS {
        return Err(format!(
            "expected_{EXPECTED_INTERVALS}_intervals_got_{}",
            power_values.len()
        ));
    }

    let total_energy_kwh = power_values.iter().sum::<f64>() * INTERVAL_HOURS;
    let peak_power_kw = power_values.iter().copied().fold(0.0_f64, f64::max);
    let mean_power_kw = power_values.iter().sum::<f64>() / power_values.len() as f64;
    let nonzero: Vec<usize> = power_values
        .iter()
        .enumerate()
        .filter(|(_, value)| **value > 1.0e-9)
        .map(|(index, _)| index)
        .collect();
    let first_nonzero = nonzero.first().map(|index| time_values[*index].clone());
    let last_nonzero = nonzero.last().map(|index| time_values[*index].clone());

    Ok(ProfileSummary {
        filename: filename.to_string(),
        internal_csv_sha256: sha256(bytes),
        fleet_id: metadata.fleet_id.clone(),
        fleet_size: metadata.fleet_size,
        strategy: metadata.strategy.clone(),
        profile_class: metadata.profile_class.clone(),
        resolution_minutes: 15,
        interval_count: power_values.len(),
        first_interval_start: time_values[0].clone(),
        last_interval_start: time_values[EXPECTED_INTERVALS - 1].clone(),
        total_energy_kwh,
        peak_power_kw,
        mean_power_kw,
        nonzero_interval_count: nonzero.len(),
        zero_interval_count: EXPECTED_INTERVALS - nonzero.len(),
        first_nonzero_interval_start: first_nonzero,
        last_nonzero_interval_start: last_nonzero,
    })
}

fn audit_zip(
    zip_bytes: &[u8],
    source_uri: String,
    source_version: String,
) -> Result<Report, String> {
    if !source_uri.starts_with("https://") || source_version.trim().is_empty() {
        return Err("https_source_uri_and_source_version_are_required".to_string());
    }
    let mut archive = ZipArchive::new(std::io::Cursor::new(zip_bytes))
        .map_err(|error| format!("invalid_zip_archive:{error}"))?;
    let zip_entry_count = archive.len();
    let mut csv_entry_count = 0_usize;
    let mut ignored_macos_sidecar_count = 0_usize;
    let mut profiles = Vec::<ProfileSummary>::new();
    let mut findings = Vec::<AuditFinding>::new();
    let mut seen_keys = BTreeSet::<String>::new();
    let mut duplicate_keys = BTreeSet::<String>::new();

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("cannot_open_zip_entry_{index}:{error}"))?;
        if entry.is_dir() || !entry.name().to_ascii_lowercase().ends_with(".csv") {
            continue;
        }
        let name = entry.name().to_string();
        let basename = Path::new(&name)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        // macOS archives often carry AppleDouble resource-fork files beside every CSV.
        // They are archive metadata, not independent load profiles.
        if name.split('/').any(|part| part == "__MACOSX") || basename.starts_with("._") {
            ignored_macos_sidecar_count += 1;
            continue;
        }
        csv_entry_count += 1;
        let mut bytes = Vec::new();
        if let Err(error) = entry.read_to_end(&mut bytes) {
            findings.push(AuditFinding {
                filename: name,
                reason: format!("cannot_read_zip_entry:{error}"),
            });
            continue;
        }
        drop(entry);

        let metadata = match parse_profile_filename(&name) {
            Ok(value) => value,
            Err(reason) => {
                findings.push(AuditFinding {
                    filename: name,
                    reason,
                });
                continue;
            }
        };
        let key = profile_key(
            &metadata.fleet_id,
            metadata.fleet_size,
            &metadata.strategy,
            &metadata.profile_class,
        );
        if !seen_keys.insert(key.clone()) {
            duplicate_keys.insert(key.clone());
            findings.push(AuditFinding {
                filename: name.clone(),
                reason: format!("duplicate_profile_key:{key}"),
            });
            continue;
        }
        match audit_csv(&metadata.filename, &bytes, &metadata) {
            Ok(profile) => profiles.push(profile),
            Err(reason) => findings.push(AuditFinding {
                filename: name,
                reason,
            }),
        }
    }

    let expected = expected_profile_keys();
    let missing_keys: Vec<String> = expected.difference(&seen_keys).cloned().collect();
    for key in &missing_keys {
        findings.push(AuditFinding {
            filename: String::new(),
            reason: format!("missing_expected_profile_key:{key}"),
        });
    }
    let duplicate_profile_keys: Vec<String> = duplicate_keys.into_iter().collect();
    let unique_profile_key_count = seen_keys.len();
    let valid = findings.is_empty()
        && profiles.len() == expected.len()
        && csv_entry_count == expected.len()
        && unique_profile_key_count == expected.len();
    profiles.sort_by(|a, b| {
        a.fleet_id
            .cmp(&b.fleet_id)
            .then_with(|| a.fleet_size.cmp(&b.fleet_size))
            .then_with(|| a.strategy.cmp(&b.strategy))
            .then_with(|| a.profile_class.cmp(&b.profile_class))
    });
    Ok(Report {
        schema_version: "0.1.0".to_string(),
        tool: "ev-depot-profile-audit".to_string(),
        dataset_id: "nlr-heavy-duty-depot-charging-load-profiles-2021".to_string(),
        source_uri,
        source_version_declared_by_caller: source_version,
        evidence_class: "published_modeled_aggregate_load_profiles".to_string(),
        source_zip_sha256: sha256(zip_bytes),
        source_zip_bytes: zip_bytes.len() as u64,
        zip_entry_count,
        csv_entry_count,
        ignored_macos_sidecar_count,
        expected_profile_count: expected.len(),
        valid_profile_count: profiles.len(),
        invalid_profile_count: csv_entry_count.saturating_sub(profiles.len()),
        unique_profile_key_count,
        missing_profile_keys: missing_keys,
        duplicate_profile_keys,
        findings,
        profiles,
        valid,
        claim_ceiling: CLAIM_CEILING.to_string(),
        provenance_warning: "The source URI and source version are caller-declared. SHA-256 identifies the archive bytes and each internal CSV byte sequence, but does not authenticate publisher identity. Profiles are upstream modeled fleet-level outputs, not metered charging records.".to_string(),
    })
}

fn run() -> Result<Report, String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        return Err(
            "usage: ev-depot-profile-audit <profile-zip> <source-uri-https> <source-version-label>"
                .to_string(),
        );
    }
    let zip_bytes = fs::read(&args[1])
        .map_err(|error| format!("cannot_read_profile_zip_{}:{error}", args[1]))?;
    audit_zip(&zip_bytes, args[2].clone(), args[3].clone())
}

fn main() -> ExitCode {
    match run() {
        Ok(report) => {
            let valid = report.valid;
            match serde_json::to_string_pretty(&report) {
                Ok(value) => {
                    println!("{value}");
                    if valid {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::from(2)
                    }
                }
                Err(error) => {
                    eprintln!("cannot_serialize_profile_audit_report:{error}");
                    ExitCode::from(2)
                }
            }
        }
        Err(error) => {
            eprintln!("ev-depot-profile-audit:{error}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expected_csv() -> Vec<u8> {
        let mut csv = String::from("time,avg_power_kw\n");
        for index in 0..EXPECTED_INTERVALS {
            csv.push_str(&format!(
                "{},{}\n",
                expected_time(index),
                if index < 4 { 100.0 } else { 0.0 }
            ));
        }
        csv.into_bytes()
    }

    #[test]
    fn parses_profile_filename_for_each_strategy_and_profile_class() {
        let average =
            parse_profile_filename("fleet1-beverage-delivery_10vehs_avg-prof_immediate_15min.csv")
                .unwrap();
        assert_eq!(average.fleet_id, "fleet1-beverage-delivery");
        assert_eq!(average.fleet_size, 10);
        assert_eq!(average.strategy, "immediate");
        assert_eq!(average.profile_class, "average");

        let minimum =
            parse_profile_filename("fleet3-food-delivery_100vehs_min-prof_min_power_15min.csv")
                .unwrap();
        assert_eq!(minimum.strategy, "min_power");
        assert_eq!(minimum.profile_class, "min_peak");

        let peak =
            parse_profile_filename("fleet2-warehouse-delivery_50vehs_peak-prof_delayed_15min.csv")
                .unwrap();
        assert_eq!(peak.strategy, "delayed");
        assert_eq!(peak.profile_class, "peak_peak");
    }

    #[test]
    fn rejects_unknown_fleet_resolution_and_strategy() {
        assert!(
            parse_profile_filename("fleet4-other_10vehs_avg-prof_immediate_15min.csv").is_err()
        );
        assert!(
            parse_profile_filename("fleet1-beverage-delivery_10vehs_avg-prof_immediate_1min.csv")
                .is_err()
        );
        assert!(
            parse_profile_filename("fleet1-beverage-delivery_10vehs_avg-prof_random_15min.csv")
                .is_err()
        );
    }

    #[test]
    fn audits_96_bins_and_conserves_daily_energy_integral() {
        let metadata =
            parse_profile_filename("fleet1-beverage-delivery_10vehs_avg-prof_immediate_15min.csv")
                .unwrap();
        let csv = expected_csv();
        let profile = audit_csv(&metadata.filename, &csv, &metadata).unwrap();
        assert_eq!(profile.interval_count, 96);
        assert!((profile.total_energy_kwh - 100.0).abs() < 1.0e-9);
        assert_eq!(profile.peak_power_kw, 100.0);
        assert_eq!(profile.mean_power_kw, 100.0 / 24.0);
        assert_eq!(profile.nonzero_interval_count, 4);
        assert_eq!(profile.zero_interval_count, 92);
        assert_eq!(
            profile.first_nonzero_interval_start.as_deref(),
            Some("00:00:00")
        );
        assert_eq!(
            profile.last_nonzero_interval_start.as_deref(),
            Some("00:45:00")
        );
    }

    #[test]
    fn rejects_time_gaps_and_malformed_power_values() {
        let metadata =
            parse_profile_filename("fleet1-beverage-delivery_10vehs_avg-prof_immediate_15min.csv")
                .unwrap();
        let csv = String::from_utf8(expected_csv())
            .unwrap()
            .replace("00:15:00,100", "00:30:00,100");
        assert!(
            audit_csv(&metadata.filename, csv.as_bytes(), &metadata)
                .unwrap_err()
                .contains("time_sequence_mismatch")
        );

        let csv = String::from_utf8(expected_csv())
            .unwrap()
            .replace("00:00:00,100", "00:00:00,NaN");
        assert!(
            audit_csv(&metadata.filename, csv.as_bytes(), &metadata)
                .unwrap_err()
                .contains("finite_and_nonnegative")
        );
    }

    #[test]
    fn rejects_wrong_interval_count_and_unknown_power_schema() {
        let metadata =
            parse_profile_filename("fleet1-beverage-delivery_10vehs_avg-prof_immediate_15min.csv")
                .unwrap();
        let too_short = "time,avg_power_kw\n00:00:00,1\n";
        assert!(
            audit_csv(&metadata.filename, too_short.as_bytes(), &metadata)
                .unwrap_err()
                .contains("expected_96_intervals")
        );
        let wrong_schema = "time,power\n00:00:00,1\n";
        assert!(
            audit_csv(&metadata.filename, wrong_schema.as_bytes(), &metadata)
                .unwrap_err()
                .contains("required_power_column_missing")
        );
    }

    #[test]
    fn expected_cross_product_contains_exactly_81_profiles() {
        let keys = expected_profile_keys();
        assert_eq!(keys.len(), 81);
    }
}
