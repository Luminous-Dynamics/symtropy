// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Contract and matched-output audit for the public NLR 15-minute depot-profile archive.
//!
//! The input is the unzipped operator-published archive. It is never presented as
//! metered charging. Candidate outputs are caller-supplied estimates or simulations.

use csv::ReaderBuilder;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

const SOURCE_REVISION: &str = "61530df51f6cd7d64f014ebbf8714cef24c8ca70";
const SOURCE_URI: &str = "https://github.com/NatLabRockies/hdev-depot-charging-2021";
const CATALOG_URI: &str = "https://data.nlr.gov/submissions/162";
const DATASET_DOI: &str = "10.7799/1787031";
const EXPECTED_INTERVALS: usize = 96;
const INTERVAL_HOURS: f64 = 0.25;
const POWER_TOLERANCE_KW: f64 = 1e-9;

const FLEETS: [&str; 3] = [
    "fleet1-beverage-delivery",
    "fleet2-warehouse-delivery",
    "fleet3-food-delivery",
];
const FLEET_SIZES: [usize; 3] = [10, 50, 100];
const STRATEGIES: [&str; 3] = ["immediate", "delayed", "min_power"];
const PROFILE_KINDS: [&str; 3] = ["avg-prof", "min-prof", "peak-prof"];

#[derive(Debug, Clone)]
struct Options {
    reference_dir: PathBuf,
    candidate_dir: Option<PathBuf>,
    output_path: Option<PathBuf>,
    source_revision: String,
    source_archive_sha256: Option<String>,
    candidate_evidence_class: String,
    candidate_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct ProfileKey {
    fleet_id: String,
    fleet_size: usize,
    strategy: String,
    profile_kind: String,
}

impl ProfileKey {
    fn file_name(&self) -> String {
        format!(
            "{}_{}vehs_{}_{}_15min.csv",
            self.fleet_id, self.fleet_size, self.profile_kind, self.strategy
        )
    }
}

#[derive(Debug, Clone)]
struct ProfileData {
    key: ProfileKey,
    values_kw: Vec<f64>,
    csv_sha256: String,
    minimum_kw: f64,
    mean_kw: f64,
    peak_kw: f64,
    energy_kwh: f64,
}

#[derive(Debug, Clone, Serialize)]
struct ProfileSummary {
    key: ProfileKey,
    file_name: String,
    csv_sha256: String,
    interval_count: usize,
    minimum_power_kw: f64,
    mean_power_kw: f64,
    peak_power_kw: f64,
    daily_energy_kwh: f64,
}

#[derive(Debug, Clone)]
struct AuditedArchive {
    profiles: BTreeMap<ProfileKey, ProfileData>,
    profile_summaries: Vec<ProfileSummary>,
    archive_content_sha256: String,
}

#[derive(Debug, Clone, Serialize)]
struct OrderingAudit {
    fleet_id: String,
    fleet_size: usize,
    strategy: String,
    average_peak_kw: f64,
    minimum_peak_kw: f64,
    maximum_peak_kw: f64,
    average_peak_not_above_maximum_profile: bool,
}

#[derive(Debug, Clone, Serialize)]
struct ProfileComparison {
    key: ProfileKey,
    interval_count: usize,
    candidate_csv_sha256: String,
    mean_absolute_error_kw: f64,
    root_mean_squared_error_kw: f64,
    p95_absolute_error_kw: f64,
    mean_bias_kw: f64,
    reference_daily_energy_kwh: f64,
    candidate_daily_energy_kwh: f64,
    daily_energy_absolute_error_kwh: f64,
    daily_energy_relative_error_percent: f64,
    reference_peak_kw: f64,
    candidate_peak_kw: f64,
    peak_absolute_error_kw: f64,
}

#[derive(Debug, Clone, Serialize)]
struct ComparisonSummary {
    candidate_id: String,
    candidate_evidence_class_declared_by_caller: String,
    profile_count: usize,
    interval_count: usize,
    mean_absolute_error_kw: f64,
    root_mean_squared_error_kw: f64,
    p95_absolute_error_kw: f64,
    mean_bias_kw: f64,
    total_reference_energy_kwh: f64,
    total_candidate_energy_kwh: f64,
    aggregate_energy_bias_kwh: f64,
    aggregate_energy_absolute_error_kwh: f64,
    aggregate_energy_relative_error_percent: f64,
    sum_profile_daily_energy_absolute_error_kwh: f64,
    sum_profile_daily_energy_relative_error_percent: f64,
    maximum_profile_peak_absolute_error_kw: f64,
    profile_comparisons: Vec<ProfileComparison>,
}

#[derive(Debug, Clone, Serialize)]
struct AuditReport {
    schema_version: String,
    tool: String,
    dataset_id: String,
    dataset_doi: String,
    catalog_uri: String,
    source_repository_uri: String,
    source_code_revision: String,
    source_archive_sha256_declared_by_caller: Option<String>,
    reference_archive_content_sha256: String,
    reference_profile_count: usize,
    expected_profile_count: usize,
    expected_intervals_per_profile: usize,
    expected_interval_minutes: u32,
    reference_total_interval_count: usize,
    fleets: Vec<String>,
    fleet_sizes: Vec<usize>,
    charging_strategies: Vec<String>,
    profile_kinds: Vec<String>,
    ordering_audits: Vec<OrderingAudit>,
    reference_profiles: Vec<ProfileSummary>,
    candidate_comparison: Option<ComparisonSummary>,
    provenance_warning: String,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize)]
struct AuditEnvelope {
    report: AuditReport,
    report_sha256: String,
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn expected_keys() -> Vec<ProfileKey> {
    let mut keys = Vec::with_capacity(81);
    for fleet_id in FLEETS {
        for fleet_size in FLEET_SIZES {
            for strategy in STRATEGIES {
                for profile_kind in PROFILE_KINDS {
                    keys.push(ProfileKey {
                        fleet_id: fleet_id.to_owned(),
                        fleet_size,
                        strategy: strategy.to_owned(),
                        profile_kind: profile_kind.to_owned(),
                    });
                }
            }
        }
    }
    keys.sort();
    keys
}

fn expected_timestamp(index: usize) -> String {
    let minute_of_day = index * 15;
    let hour = minute_of_day / 60;
    let minute = minute_of_day % 60;
    format!("{hour:02}:{minute:02}:00")
}

fn parse_nonnegative_power(text: &str, file_name: &str, row: usize) -> Result<f64, String> {
    let value = text
        .parse::<f64>()
        .map_err(|_| format!("{file_name}: row {row}: avg_power_kw is not numeric"))?;
    if !value.is_finite() || value < 0.0 {
        return Err(format!(
            "{file_name}: row {row}: avg_power_kw must be finite and non-negative"
        ));
    }
    Ok(value)
}

fn read_profile(directory: &Path, key: &ProfileKey) -> Result<ProfileData, String> {
    let file_name = key.file_name();
    let path = directory.join(&file_name);
    let bytes = fs::read(&path)
        .map_err(|error| format!("missing or unreadable profile {file_name}: {error}"))?;

    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .flexible(false)
        .from_reader(bytes.as_slice());

    let headers = reader
        .headers()
        .map_err(|error| format!("{file_name}: invalid CSV header: {error}"))?;
    if headers.len() != 2
        || headers.get(0) != Some("time")
        || headers.get(1) != Some("avg_power_kw")
    {
        return Err(format!(
            "{file_name}: expected exact CSV header time,avg_power_kw"
        ));
    }

    let mut values_kw = Vec::with_capacity(EXPECTED_INTERVALS);
    for (index, record) in reader.records().enumerate() {
        let row = index + 2;
        let record = record
            .map_err(|error| format!("{file_name}: row {row}: malformed CSV record: {error}"))?;
        if record.len() != 2 {
            return Err(format!("{file_name}: row {row}: expected two columns"));
        }
        let actual_time = record.get(0).unwrap_or_default();
        let expected_time = expected_timestamp(index);
        if actual_time != expected_time {
            return Err(format!(
                "{file_name}: row {row}: expected timestamp {expected_time}, found {actual_time}"
            ));
        }
        let power_text = record.get(1).unwrap_or_default();
        values_kw.push(parse_nonnegative_power(power_text, &file_name, row)?);
        if values_kw.len() > EXPECTED_INTERVALS {
            return Err(format!(
                "{file_name}: contains more than {EXPECTED_INTERVALS} intervals"
            ));
        }
    }

    if values_kw.len() != EXPECTED_INTERVALS {
        return Err(format!(
            "{file_name}: expected {EXPECTED_INTERVALS} intervals, found {}",
            values_kw.len()
        ));
    }

    let minimum_kw = values_kw.iter().copied().fold(f64::INFINITY, f64::min);
    let peak_kw = values_kw.iter().copied().fold(0.0_f64, f64::max);
    let energy_kwh = values_kw.iter().sum::<f64>() * INTERVAL_HOURS;
    let mean_kw = energy_kwh / (EXPECTED_INTERVALS as f64 * INTERVAL_HOURS);

    Ok(ProfileData {
        key: key.clone(),
        values_kw,
        csv_sha256: sha256_hex(&bytes),
        minimum_kw,
        mean_kw,
        peak_kw,
        energy_kwh,
    })
}

fn profile_summary(profile: &ProfileData) -> ProfileSummary {
    ProfileSummary {
        key: profile.key.clone(),
        file_name: profile.key.file_name(),
        csv_sha256: profile.csv_sha256.clone(),
        interval_count: profile.values_kw.len(),
        minimum_power_kw: profile.minimum_kw,
        mean_power_kw: profile.mean_kw,
        peak_power_kw: profile.peak_kw,
        daily_energy_kwh: profile.energy_kwh,
    }
}

fn archive_digest(profiles: &BTreeMap<ProfileKey, ProfileData>) -> String {
    let mut bytes = Vec::new();
    for profile in profiles.values() {
        bytes.extend_from_slice(profile.key.file_name().as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(profile.csv_sha256.as_bytes());
        bytes.push(b'\n');
    }
    sha256_hex(&bytes)
}

fn audit_archive(directory: &Path) -> Result<AuditedArchive, String> {
    if !directory.is_dir() {
        return Err(format!(
            "profile directory does not exist or is not a directory: {}",
            directory.display()
        ));
    }

    let mut profiles = BTreeMap::new();
    for key in expected_keys() {
        let profile = read_profile(directory, &key)?;
        if profiles.insert(key.clone(), profile).is_some() {
            return Err(format!(
                "duplicate expected profile key: {}",
                key.file_name()
            ));
        }
    }

    let expected_file_names = expected_keys()
        .into_iter()
        .map(|key| key.file_name())
        .collect::<std::collections::BTreeSet<_>>();

    let entries = fs::read_dir(directory)
        .map_err(|error| format!("cannot list profile directory: {error}"))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("cannot read directory entry: {error}"))?;
        let kind = entry
            .file_type()
            .map_err(|error| format!("cannot inspect directory entry: {error}"))?;
        if !kind.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.ends_with(".csv") && !expected_file_names.contains(&name) {
            return Err(format!("unexpected CSV profile in archive: {name}"));
        }
    }

    let mut ordering_audits = Vec::new();
    for fleet_id in FLEETS {
        for fleet_size in FLEET_SIZES {
            for strategy in STRATEGIES {
                let find = |profile_kind: &str| {
                    let key = ProfileKey {
                        fleet_id: fleet_id.to_owned(),
                        fleet_size,
                        strategy: strategy.to_owned(),
                        profile_kind: profile_kind.to_owned(),
                    };
                    profiles
                        .get(&key)
                        .ok_or_else(|| format!("internal missing profile {}", key.file_name()))
                };
                let average = find("avg-prof")?;
                let minimum = find("min-prof")?;
                let maximum = find("peak-prof")?;

                // The average profile is the pointwise mean across sampled
                // days; convexity of max() implies its peak cannot exceed the
                // largest peak among those days. The minimum-peak sample is
                // not required to sit below the mean profile at every time.
                let average_not_above_maximum =
                    average.peak_kw <= maximum.peak_kw + POWER_TOLERANCE_KW;
                if !average_not_above_maximum {
                    return Err(format!(
                        "{} / {} / {}: average profile peak {:.6} kW exceeds maximum-sample profile peak {:.6} kW",
                        fleet_id, fleet_size, strategy, average.peak_kw, maximum.peak_kw
                    ));
                }
                if minimum.peak_kw > maximum.peak_kw + POWER_TOLERANCE_KW {
                    return Err(format!(
                        "{} / {} / {}: minimum-sample profile peak exceeds maximum-sample profile peak",
                        fleet_id, fleet_size, strategy
                    ));
                }

                ordering_audits.push(OrderingAudit {
                    fleet_id: fleet_id.to_owned(),
                    fleet_size,
                    strategy: strategy.to_owned(),
                    average_peak_kw: average.peak_kw,
                    minimum_peak_kw: minimum.peak_kw,
                    maximum_peak_kw: maximum.peak_kw,
                    average_peak_not_above_maximum_profile: average_not_above_maximum,
                });
            }
        }
    }

    let profile_summaries = profiles.values().map(profile_summary).collect();
    let digest = archive_digest(&profiles);
    Ok(AuditedArchive {
        profiles,
        profile_summaries,
        archive_content_sha256: digest,
    })
}

fn percentile(values: &[f64], fraction: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let index = ((fraction * sorted.len() as f64).ceil() as usize)
        .saturating_sub(1)
        .min(sorted.len() - 1);
    Some(sorted[index])
}

fn compare_archives(
    reference: &AuditedArchive,
    candidate: &AuditedArchive,
    candidate_id: &str,
    candidate_evidence_class: &str,
) -> Result<ComparisonSummary, String> {
    let mut all_absolute_errors = Vec::with_capacity(81 * EXPECTED_INTERVALS);
    let mut all_biases = Vec::with_capacity(81 * EXPECTED_INTERVALS);
    let mut comparisons = Vec::with_capacity(81);
    let mut squared_error_sum = 0.0_f64;
    let mut energy_absolute_error_sum = 0.0_f64;
    let mut reference_energy_sum = 0.0_f64;
    let mut candidate_energy_sum = 0.0_f64;
    let mut maximum_peak_error = 0.0_f64;

    for (key, reference_profile) in &reference.profiles {
        let candidate_profile = candidate
            .profiles
            .get(key)
            .ok_or_else(|| format!("candidate missing profile {}", key.file_name()))?;
        if reference_profile.values_kw.len() != candidate_profile.values_kw.len() {
            return Err(format!(
                "profile interval count differs for {}",
                key.file_name()
            ));
        }

        let mut errors = Vec::with_capacity(EXPECTED_INTERVALS);
        let mut bias_sum = 0.0_f64;
        let mut squared_sum = 0.0_f64;
        for (reference_kw, candidate_kw) in reference_profile
            .values_kw
            .iter()
            .zip(candidate_profile.values_kw.iter())
        {
            let bias = candidate_kw - reference_kw;
            let absolute = bias.abs();
            errors.push(absolute);
            all_absolute_errors.push(absolute);
            all_biases.push(bias);
            bias_sum += bias;
            squared_sum += bias * bias;
            squared_error_sum += bias * bias;
        }

        let interval_count = EXPECTED_INTERVALS as f64;
        let mean_absolute_error = errors.iter().sum::<f64>() / interval_count;
        let root_mean_squared_error = (squared_sum / interval_count).sqrt();
        let p95_absolute_error = percentile(&errors, 0.95).unwrap_or(0.0);
        let mean_bias = bias_sum / interval_count;
        let energy_absolute_error =
            (candidate_profile.energy_kwh - reference_profile.energy_kwh).abs();
        let energy_relative_error = if reference_profile.energy_kwh > 0.0 {
            energy_absolute_error / reference_profile.energy_kwh * 100.0
        } else if energy_absolute_error == 0.0 {
            0.0
        } else {
            return Err(format!(
                "{}: non-zero candidate energy against a zero-energy reference",
                key.file_name()
            ));
        };
        let peak_absolute_error = (candidate_profile.peak_kw - reference_profile.peak_kw).abs();

        reference_energy_sum += reference_profile.energy_kwh;
        candidate_energy_sum += candidate_profile.energy_kwh;
        energy_absolute_error_sum += energy_absolute_error;
        maximum_peak_error = maximum_peak_error.max(peak_absolute_error);

        comparisons.push(ProfileComparison {
            key: key.clone(),
            interval_count: EXPECTED_INTERVALS,
            candidate_csv_sha256: candidate_profile.csv_sha256.clone(),
            mean_absolute_error_kw: mean_absolute_error,
            root_mean_squared_error_kw: root_mean_squared_error,
            p95_absolute_error_kw: p95_absolute_error,
            mean_bias_kw: mean_bias,
            reference_daily_energy_kwh: reference_profile.energy_kwh,
            candidate_daily_energy_kwh: candidate_profile.energy_kwh,
            daily_energy_absolute_error_kwh: energy_absolute_error,
            daily_energy_relative_error_percent: energy_relative_error,
            reference_peak_kw: reference_profile.peak_kw,
            candidate_peak_kw: candidate_profile.peak_kw,
            peak_absolute_error_kw: peak_absolute_error,
        });
    }

    let interval_count = all_absolute_errors.len() as f64;
    let mean_absolute_error = all_absolute_errors.iter().sum::<f64>() / interval_count;
    let rmse = (squared_error_sum / interval_count).sqrt();
    let mean_bias = all_biases.iter().sum::<f64>() / interval_count;
    let p95 = percentile(&all_absolute_errors, 0.95).unwrap_or(0.0);
    let aggregate_energy_bias = candidate_energy_sum - reference_energy_sum;
    let aggregate_energy_absolute_error = aggregate_energy_bias.abs();
    let aggregate_energy_relative_error = if reference_energy_sum > 0.0 {
        aggregate_energy_absolute_error / reference_energy_sum * 100.0
    } else if aggregate_energy_absolute_error == 0.0 {
        0.0
    } else {
        return Err("non-zero aggregate candidate energy against zero total reference energy".into());
    };
    let sum_profile_energy_relative_error = if reference_energy_sum > 0.0 {
        energy_absolute_error_sum / reference_energy_sum * 100.0
    } else if energy_absolute_error_sum == 0.0 {
        0.0
    } else {
        return Err("non-zero profile-level energy error against zero total reference energy".into());
    };

    Ok(ComparisonSummary {
        candidate_id: candidate_id.to_owned(),
        candidate_evidence_class_declared_by_caller: candidate_evidence_class.to_owned(),
        profile_count: comparisons.len(),
        interval_count: all_absolute_errors.len(),
        mean_absolute_error_kw: mean_absolute_error,
        root_mean_squared_error_kw: rmse,
        p95_absolute_error_kw: p95,
        mean_bias_kw: mean_bias,
        total_reference_energy_kwh: reference_energy_sum,
        total_candidate_energy_kwh: candidate_energy_sum,
        aggregate_energy_bias_kwh: aggregate_energy_bias,
        aggregate_energy_absolute_error_kwh: aggregate_energy_absolute_error,
        aggregate_energy_relative_error_percent: aggregate_energy_relative_error,
        sum_profile_daily_energy_absolute_error_kwh: energy_absolute_error_sum,
        sum_profile_daily_energy_relative_error_percent: sum_profile_energy_relative_error,
        maximum_profile_peak_absolute_error_kw: maximum_peak_error,
        profile_comparisons: comparisons,
    })
}

fn audit(options: &Options) -> Result<AuditEnvelope, String> {
    if options.source_revision.trim().is_empty() {
        return Err("source revision must not be empty".into());
    }
    if options
        .source_archive_sha256
        .as_deref()
        .is_some_and(|value| !is_sha256(value))
    {
        return Err("--source-archive-sha256 must be 64 lowercase hexadecimal characters".into());
    }

    let reference = audit_archive(&options.reference_dir)?;
    let candidate = match &options.candidate_dir {
        Some(directory) => Some(audit_archive(directory)?),
        None => None,
    };
    let comparison = match (&candidate, &options.candidate_dir) {
        (Some(archive), Some(_)) => Some(compare_archives(
            &reference,
            archive,
            &options.candidate_id,
            &options.candidate_evidence_class,
        )?),
        (None, None) => None,
        _ => return Err("candidate archive state is inconsistent".into()),
    };

    let report = AuditReport {
        schema_version: "ev-nlr-profile-audit/v0.1".into(),
        tool: "ev-nlr-profile-audit".into(),
        dataset_id: "NLR-DATA-CATALOG-162".into(),
        dataset_doi: DATASET_DOI.into(),
        catalog_uri: CATALOG_URI.into(),
        source_repository_uri: SOURCE_URI.into(),
        source_code_revision: options.source_revision.clone(),
        source_archive_sha256_declared_by_caller: options.source_archive_sha256.clone(),
        reference_archive_content_sha256: reference.archive_content_sha256,
        reference_profile_count: reference.profiles.len(),
        expected_profile_count: 81,
        expected_intervals_per_profile: EXPECTED_INTERVALS,
        expected_interval_minutes: 15,
        reference_total_interval_count: reference.profiles.len() * EXPECTED_INTERVALS,
        fleets: FLEETS.iter().map(|value| (*value).to_owned()).collect(),
        fleet_sizes: FLEET_SIZES.to_vec(),
        charging_strategies: STRATEGIES.iter().map(|value| (*value).to_owned()).collect(),
        profile_kinds: PROFILE_KINDS.iter().map(|value| (*value).to_owned()).collect(),
        ordering_audits: reference
            .profiles
            .keys()
            .filter(|key| key.profile_kind == "avg-prof")
            .map(|key| {
                let average = &reference.profiles[key];
                let min_key = ProfileKey {
                    profile_kind: "min-prof".into(),
                    ..key.clone()
                };
                let max_key = ProfileKey {
                    profile_kind: "peak-prof".into(),
                    ..key.clone()
                };
                let minimum = &reference.profiles[&min_key];
                let maximum = &reference.profiles[&max_key];
                OrderingAudit {
                    fleet_id: key.fleet_id.clone(),
                    fleet_size: key.fleet_size,
                    strategy: key.strategy.clone(),
                    average_peak_kw: average.peak_kw,
                    minimum_peak_kw: minimum.peak_kw,
                    maximum_peak_kw: maximum.peak_kw,
                    average_peak_not_above_maximum_profile: true,
                }
            })
            .collect(),
        reference_profiles: reference.profile_summaries,
        candidate_comparison: comparison,
        provenance_warning: "A SHA-256 proves byte identity, not publisher authenticity or physical truth. The NLR 15-minute profiles are modeled aggregate charging-load scenarios generated from operational schedules and declared energy/power assumptions; they are not measured individual charging sessions, metered depot loads, or a South African fleet benchmark. Candidate evidence class is declared by the caller.".into(),
        claim_ceiling: "Archive-contract validation and matched 15-minute aggregate-profile comparison only. This tool does not reproduce the upstream simulation unless the caller supplies outputs from an independently implemented or exact upstream generator with matched inputs and settings. It does not establish measured charger behaviour, battery performance, feeder feasibility, grid-code compliance or South African fleet performance.".into(),
    };

    let report_bytes = serde_json::to_vec(&report)
        .map_err(|error| format!("cannot serialize NLR profile audit report: {error}"))?;
    let report_sha256 = sha256_hex(&report_bytes);
    Ok(AuditEnvelope {
        report,
        report_sha256,
    })
}

fn parse_options(args: &[String]) -> Result<Options, String> {
    let mut reference_dir = None;
    let mut candidate_dir = None;
    let mut output_path = None;
    let mut source_revision = SOURCE_REVISION.to_owned();
    let mut source_archive_sha256 = None;
    let mut candidate_evidence_class = "estimate".to_owned();
    let mut candidate_id = "caller-supplied-candidate".to_owned();

    let mut index = 0;
    while index < args.len() {
        let name = args[index].as_str();
        if name == "--help" || name == "-h" {
            return Err("usage: ev-nlr-profile-audit --reference-dir PATH [--candidate-dir PATH] [--candidate-id ID] [--candidate-evidence-class estimate|simulated] [--source-revision SHA] [--source-archive-sha256 HEX] [--output PATH]".into());
        }
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("{name} requires a value"))?;
        match name {
            "--reference-dir" => reference_dir = Some(PathBuf::from(value)),
            "--candidate-dir" => candidate_dir = Some(PathBuf::from(value)),
            "--output" => output_path = Some(PathBuf::from(value)),
            "--source-revision" => source_revision = value.clone(),
            "--source-archive-sha256" => source_archive_sha256 = Some(value.clone()),
            "--candidate-evidence-class" => candidate_evidence_class = value.clone(),
            "--candidate-id" => candidate_id = value.clone(),
            _ => return Err(format!("unknown argument: {name}")),
        }
        index += 2;
    }

    if !["estimate", "simulated"].contains(&candidate_evidence_class.as_str()) {
        return Err("--candidate-evidence-class must be estimate or simulated".into());
    }
    if candidate_id.trim().is_empty() {
        return Err("--candidate-id must not be empty".into());
    }
    Ok(Options {
        reference_dir: reference_dir.ok_or_else(|| "--reference-dir is required".to_owned())?,
        candidate_dir,
        output_path,
        source_revision,
        source_archive_sha256,
        candidate_evidence_class,
        candidate_id,
    })
}

fn main() -> ExitCode {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let options = match parse_options(&args) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return if args.iter().any(|arg| arg == "--help" || arg == "-h") {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(2)
            };
        }
    };

    let envelope = match audit(&options) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("NLR profile audit failed: {error}");
            return ExitCode::from(2);
        }
    };
    let output = match serde_json::to_string_pretty(&envelope) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot serialize NLR profile audit envelope: {error}");
            return ExitCode::from(2);
        }
    };
    if let Some(path) = options.output_path {
        if let Err(error) = fs::write(&path, format!("{output}\n")) {
            eprintln!("cannot write audit report {}: {error}", path.display());
            return ExitCode::from(2);
        }
    } else {
        println!("{output}");
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn temporary_directory(label: &str) -> PathBuf {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = env::temp_dir().join(format!(
            "ev-nlr-profile-audit-{}-{label}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create test directory");
        path
    }

    fn write_fixture_archive(directory: &Path, additive_shift_kw: f64) {
        for (fleet_index, fleet_id) in FLEETS.iter().enumerate() {
            for (size_index, fleet_size) in FLEET_SIZES.iter().enumerate() {
                for (strategy_index, strategy) in STRATEGIES.iter().enumerate() {
                    for (kind_index, profile_kind) in PROFILE_KINDS.iter().enumerate() {
                        let key = ProfileKey {
                            fleet_id: (*fleet_id).to_owned(),
                            fleet_size: *fleet_size,
                            strategy: (*strategy).to_owned(),
                            profile_kind: (*profile_kind).to_owned(),
                        };
                        let path = directory.join(key.file_name());
                        let mut content = String::from("time,avg_power_kw\n");
                        for index in 0..EXPECTED_INTERVALS {
                            let base = 8.0
                                + fleet_index as f64 * 2.0
                                + size_index as f64 * 3.0
                                + strategy_index as f64
                                + kind_index as f64 * 1.0
                                + index as f64 * 0.01
                                + additive_shift_kw;
                            content.push_str(&format!("{},{}\n", expected_timestamp(index), base));
                        }
                        fs::write(path, content).expect("write profile fixture");
                    }
                }
            }
        }
    }

    #[test]
    fn accepts_exactly_81_profiles_with_96_quarter_hour_points_each() {
        let directory = temporary_directory("reference");
        write_fixture_archive(&directory, 0.0);
        let archive = audit_archive(&directory).expect("valid 81-profile fixture");
        assert_eq!(archive.profiles.len(), 81);
        assert_eq!(archive.profile_summaries.len(), 81);
        assert_eq!(archive.profiles.len() * EXPECTED_INTERVALS, 7_776);
        assert_eq!(archive.archive_content_sha256.len(), 64);
        fs::remove_dir_all(directory).expect("remove fixture directory");
    }

    #[test]
    fn identical_candidate_has_zero_error() {
        let reference_dir = temporary_directory("identical-reference");
        let candidate_dir = temporary_directory("identical-candidate");
        write_fixture_archive(&reference_dir, 0.0);
        write_fixture_archive(&candidate_dir, 0.0);
        let reference = audit_archive(&reference_dir).expect("reference fixture");
        let candidate = audit_archive(&candidate_dir).expect("candidate fixture");
        let report = compare_archives(&reference, &candidate, "identical", "simulated")
            .expect("compare identical profiles");
        assert_eq!(report.profile_count, 81);
        assert_eq!(report.interval_count, 7_776);
        assert!(report.mean_absolute_error_kw.abs() < 1e-12);
        assert!(report.root_mean_squared_error_kw.abs() < 1e-12);
        assert!(report.mean_bias_kw.abs() < 1e-12);
        assert!(report.sum_profile_daily_energy_absolute_error_kwh.abs() < 1e-12);
        assert!(report.aggregate_energy_absolute_error_kwh.abs() < 1e-12);
        assert!(report.aggregate_energy_bias_kwh.abs() < 1e-12);
        fs::remove_dir_all(reference_dir).expect("remove reference fixture");
        fs::remove_dir_all(candidate_dir).expect("remove candidate fixture");
    }

    #[test]
    fn constant_half_kw_bias_is_measured_exactly() {
        let reference_dir = temporary_directory("bias-reference");
        let candidate_dir = temporary_directory("bias-candidate");
        write_fixture_archive(&reference_dir, 0.0);
        write_fixture_archive(&candidate_dir, 0.5);
        let reference = audit_archive(&reference_dir).expect("reference fixture");
        let candidate = audit_archive(&candidate_dir).expect("candidate fixture");
        let report = compare_archives(&reference, &candidate, "biased", "estimate")
            .expect("compare biased profiles");
        assert!((report.mean_absolute_error_kw - 0.5).abs() < 1e-9);
        assert!((report.root_mean_squared_error_kw - 0.5).abs() < 1e-9);
        assert!((report.mean_bias_kw - 0.5).abs() < 1e-9);
        assert!((report.sum_profile_daily_energy_absolute_error_kwh - 81.0 * 12.0).abs() < 1e-9);
        assert!((report.aggregate_energy_absolute_error_kwh - 81.0 * 12.0).abs() < 1e-9);
        assert!((report.aggregate_energy_bias_kwh - 81.0 * 12.0).abs() < 1e-9);
        fs::remove_dir_all(reference_dir).expect("remove reference fixture");
        fs::remove_dir_all(candidate_dir).expect("remove candidate fixture");
    }

    #[test]
    fn rejects_timestamp_grid_drift_and_missing_profiles() {
        let directory = temporary_directory("invalid-grid");
        write_fixture_archive(&directory, 0.0);
        let key = expected_keys()
            .into_iter()
            .next()
            .expect("expected profile key");
        let path = directory.join(key.file_name());
        let source = fs::read_to_string(&path).expect("read test CSV");
        let changed = source.replacen("00:00:00,", "00:01:00,", 1);
        fs::write(&path, changed).expect("rewrite test CSV");
        assert!(audit_archive(&directory).is_err());
        fs::remove_file(path).expect("remove invalid profile");
        assert!(audit_archive(&directory).is_err());
        fs::remove_dir_all(directory).expect("remove fixture directory");
    }

    fn overwrite_first_power(directory: &Path, key: &ProfileKey, value: &str) {
        let path = directory.join(key.file_name());
        let source = fs::read_to_string(&path).expect("read source profile");
        let mut lines = source.lines().map(str::to_owned).collect::<Vec<_>>();
        let time = lines
            .get(1)
            .and_then(|line| line.split(',').next())
            .expect("first data timestamp")
            .to_owned();
        lines[1] = format!("{time},{value}");
        fs::write(path, lines.join("\n") + "\n").expect("rewrite source profile");
    }

    #[test]
    fn rejects_negative_and_non_finite_power_values() {
        let key = expected_keys()
            .into_iter()
            .next()
            .expect("expected profile key");

        let negative_dir = temporary_directory("negative-power");
        write_fixture_archive(&negative_dir, 0.0);
        overwrite_first_power(&negative_dir, &key, "-1.000000");
        assert!(audit_archive(&negative_dir).is_err());
        fs::remove_dir_all(negative_dir).expect("remove negative fixture");

        let non_finite_dir = temporary_directory("non-finite-power");
        write_fixture_archive(&non_finite_dir, 0.0);
        overwrite_first_power(&non_finite_dir, &key, "NaN");
        assert!(audit_archive(&non_finite_dir).is_err());
        fs::remove_dir_all(non_finite_dir).expect("remove non-finite fixture");
    }

    #[test]
    fn rejects_candidate_with_different_archive_shape() {
        let reference_dir = temporary_directory("shape-reference");
        let candidate_dir = temporary_directory("shape-candidate");
        write_fixture_archive(&reference_dir, 0.0);
        write_fixture_archive(&candidate_dir, 0.0);
        let key = expected_keys().remove(0);
        fs::remove_file(candidate_dir.join(key.file_name())).expect("remove candidate profile");
        let reference = audit_archive(&reference_dir).expect("reference fixture");
        assert!(audit_archive(&candidate_dir).is_err());
        assert!(
            compare_archives(
                &reference,
                &AuditedArchive {
                    profiles: BTreeMap::new(),
                    profile_summaries: Vec::new(),
                    archive_content_sha256: String::new(),
                },
                "missing",
                "estimate",
            )
            .is_err()
        );
        fs::remove_dir_all(reference_dir).expect("remove reference fixture");
        fs::remove_dir_all(candidate_dir).expect("remove candidate fixture");
    }

    #[test]
    fn rejects_average_profile_peak_above_maximum_sample_peak() {
        let directory = temporary_directory("ordering");
        write_fixture_archive(&directory, 0.0);
        let key = ProfileKey {
            fleet_id: FLEETS[0].into(),
            fleet_size: FLEET_SIZES[0],
            strategy: STRATEGIES[0].into(),
            profile_kind: "peak-prof".into(),
        };
        let path = directory.join(key.file_name());
        let mut content = String::from("time,avg_power_kw\n");
        for index in 0..EXPECTED_INTERVALS {
            content.push_str(&format!("{},{}\n", expected_timestamp(index), 1.0));
        }
        fs::write(path, content).expect("rewrite peak profile");
        assert!(audit_archive(&directory).is_err());
        fs::remove_dir_all(directory).expect("remove fixture directory");
    }
}
