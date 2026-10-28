//! External energy-dataset source registry gate.
//!
//! This validates provenance and promotion metadata, not the remote datasets.
//! No raw external data is bundled and no source is qualified for benchmarking.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::{env, fs};

const REGISTRY_BYTES: &[u8] =
    include_bytes!("../../fixtures/energy-external-benchmark-source-registry-v0.1.json");
const EXPERIMENT_BYTES: &[u8] =
    include_bytes!("../../fixtures/energy-external-benchmark-source-registry-experiment-v0.1.json");
const REGISTRY_SHA256: &str =
    "d53b6f0f247b8c277ad853566c4ab2f4621b92314022d43ad00b10043fc631a1";
const EXPERIMENT_SHA256: &str =
    "669308da2193ae7e78b240d66c998a3858270c06e27469aafacccc909287a6e9";
const REGISTRY_SCHEMA: &str = "energy-external-benchmark-source-registry-v0.1";
const EXPERIMENT_SCHEMA: &str = "energy-external-benchmark-source-registry-experiment-v0.1";
const REGISTRY_ID: &str = "energy-external-benchmark-source-registry-2026-10-28";

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit())
}

fn required_checks() -> [&'static str; 10] {
    [
        "source_ids_are_unique_and_nonempty",
        "source_urls_are_canonical_http_or_https",
        "source_status_and_snapshot_fields_are_consistent",
        "qualified_sources_require_raw_sha256_and_snapshot_url",
        "unresolved_data_rights_block_benchmark_qualification",
        "national_system_data_cannot_claim_site_dispatch_validation",
        "reported_windows_and_cadence_are_consistent",
        "mutable_rolling_sources_cannot_be_canonical_benchmarks_without_snapshot",
        "removed_citylearn_dataset_families_are_not_selected",
        "registry_claim_ceiling_remains_discovery_only",
    ]
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    schema_version: String,
    registry_id: String,
    claim_ceiling: String,
    as_of_date: String,
    sources: Vec<Source>,
    required_checks: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    source_id: String,
    title: String,
    landing_page_url: String,
    publication_doi: Option<String>,
    published_on: Option<String>,
    registry_release: Option<String>,
    benchmark_role: String,
    status: String,
    geography: String,
    cadence_seconds: Option<u32>,
    start_time: Option<String>,
    end_exclusive: Option<String>,
    expected_rows: Option<u64>,
    reported_variable_count: Option<u32>,
    source_timezone_status: String,
    license_status: String,
    snapshot_url: Option<String>,
    raw_sha256: Option<String>,
    selected_dataset_id: Option<String>,
    removed_dataset_families: Vec<String>,
    window_semantics: String,
    limitations: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Experiment {
    schema_version: String,
    registry_id: String,
    source_registry_path: String,
    source_registry_sha256: String,
    expected_coverage: ExpectedCoverage,
    known_answers: KnownAnswers,
    claim_ceiling: String,
    required_checks: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedCoverage {
    source_count: usize,
    reference_only_not_imported_count: usize,
    ingested_digest_pinned_count: usize,
    qualified_for_benchmark_count: usize,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct KnownAnswers {
    south_african_series_expected_hourly_rows: u64,
    south_african_series_expected_cadence_seconds: u32,
    citylearn_official_release: String,
    citylearn_public_dataset_count: usize,
    removed_citylearn_families: Vec<String>,
    eskom_rolling_feed_canonical_benchmark: bool,
    imported_external_bytes: usize,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct SourceResult {
    source_id: String,
    status: String,
    benchmark_role: String,
    expected_rows: Option<u64>,
    cadence_seconds: Option<u32>,
    raw_sha256: Option<String>,
    license_status: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct Report {
    schema_version: &'static str,
    registry_id: String,
    status: &'static str,
    claim_ceiling: String,
    source_registry_sha256: String,
    experiment_fixture_sha256: String,
    source_count: usize,
    status_counts: BTreeMap<String, usize>,
    source_results: Vec<SourceResult>,
    imported_external_bytes: usize,
    qualified_sources: usize,
    promotion_gate: &'static str,
}

fn https_url(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with("http://")
}

fn limitation_contains(source: &Source, phrase: &str) -> bool {
    source.limitations.iter().any(|line| line.to_ascii_lowercase().contains(&phrase.to_ascii_lowercase()))
}

fn validate_source(source: &Source) -> Result<(), String> {
    if source.source_id.trim().is_empty() || source.title.trim().is_empty() {
        return Err("source ID and title must be nonempty".to_owned());
    }
    if !https_url(&source.landing_page_url) {
        return Err(format!("{}: landing page must be HTTP(S)", source.source_id));
    }
    if let Some(url) = &source.snapshot_url {
        if !https_url(url) {
            return Err(format!("{}: snapshot URL must be HTTP(S)", source.source_id));
        }
    }
    if let Some(hash) = &source.raw_sha256 {
        if !valid_sha256(hash) {
            return Err(format!("{}: raw snapshot SHA-256 must be 64 hex chars", source.source_id));
        }
    }

    match source.status.as_str() {
        "reference_only_not_imported" => {
            if source.raw_sha256.is_some() || source.snapshot_url.is_some() {
                return Err(format!("{}: reference-only source cannot claim a captured snapshot", source.source_id));
            }
        }
        "ingested_digest_pinned" => {
            if source.raw_sha256.is_none() || source.snapshot_url.is_none() || source.expected_rows.unwrap_or(0) == 0 {
                return Err(format!("{}: ingested status requires snapshot URL, SHA-256 and a positive row count", source.source_id));
            }
        }
        "license_review_required" => {
            if source.license_status == "rights_reviewed_and_cleared" {
                return Err(format!("{}: status contradicts cleared rights", source.source_id));
            }
        }
        "qualified_for_benchmark" => {
            if source.raw_sha256.is_none()
                || source.snapshot_url.is_none()
                || source.expected_rows.unwrap_or(0) == 0
                || source.selected_dataset_id.is_none()
                || source.license_status != "rights_reviewed_and_cleared"
                || source.source_timezone_status != "verified_and_utc_normalization_documented"
            {
                return Err(format!("{}: qualification requires pinned bytes, row count, dataset identity, cleared rights and verified time semantics", source.source_id));
            }
        }
        status => return Err(format!("{}: unsupported source status {status}", source.source_id)),
    }

    if source.benchmark_role == "south_africa_system_level_forecast_and_regime_robustness" {
        if source.geography != "South Africa national power system"
            || source.cadence_seconds != Some(3600)
            || source.start_time.as_deref() != Some("2018-07-01T00:00:00")
            || source.end_exclusive.as_deref() != Some("2023-07-01T00:00:00")
            || source.expected_rows != Some(43824)
            || source.reported_variable_count != Some(13)
            || !source.license_status.contains("underlying_eskom_data_terms_unverified")
            || !limitation_contains(source, "not a building or site-meter trace")
            || !limitation_contains(source, "not sufficient alone to assert battery dispatch feasibility")
        {
            return Err("South African reliability series scope, timing, coverage or rights boundary changed".to_owned());
        }
        if source.status == "qualified_for_benchmark" {
            return Err("national system reliability series cannot qualify site-level dispatch claims".to_owned());
        }
    }

    if source.source_id == "citylearn_official_v3_0_2" {
        let expected_removed = ["rec_2023_*", "EC_Ermesinde", "ALADI_*"];
        if source.registry_release.as_deref() != Some("v3.0.2")
            || source.selected_dataset_id.is_some()
            || expected_removed.iter().any(|name| !source.removed_dataset_families.iter().any(|v| v == name))
            || !limitation_contains(source, "select an actual dataset present in the exact official tag")
        {
            return Err("CityLearn tag, removed dataset caveat, or dataset-pinning rule changed".to_owned());
        }
    }

    if source.source_id == "eskom_rolling_demand_dashboard" {
        if source.benchmark_role != "ingestion_smoke_test_only"
            || !source.window_semantics.contains("mutable_rolling_recent_week")
            || source.status == "qualified_for_benchmark"
            || source.status == "ingested_digest_pinned"
        {
            return Err("mutable Eskom rolling feed cannot be a canonical benchmark without an archival snapshot contract".to_owned());
        }
    }
    Ok(())
}

fn validate_registry(registry: &Registry) -> Result<(), String> {
    if registry.schema_version != REGISTRY_SCHEMA
        || registry.registry_id != REGISTRY_ID
        || registry.as_of_date != "2026-10-28"
        || !registry.claim_ceiling.to_ascii_lowercase().contains("no external data snapshot")
        || registry.sources.len() != 3
    {
        return Err("registry identity, claim ceiling or expected source count changed".to_owned());
    }
    if registry.required_checks.len() != required_checks().len()
        || required_checks().iter().any(|required| !registry.required_checks.iter().any(|value| value.as_str() == *required))
    {
        return Err("required source-registry integrity check list changed".to_owned());
    }
    let mut ids = BTreeSet::new();
    for source in &registry.sources {
        if !ids.insert(source.source_id.as_str()) {
            return Err(format!("duplicate source ID {}", source.source_id));
        }
        validate_source(source)?;
    }
    let expected_ids = [
        "sa_power_supply_reliability_2018_2023",
        "citylearn_official_v3_0_2",
        "eskom_rolling_demand_dashboard",
    ];
    if registry.sources.iter().map(|s| s.source_id.as_str()).collect::<Vec<_>>() != expected_ids {
        return Err("source registry order or expected identity set changed".to_owned());
    }
    Ok(())
}

fn validate_experiment(experiment: &Experiment, registry: &Registry, bytes: &[u8]) -> Result<(), String> {
    if experiment.schema_version != EXPERIMENT_SCHEMA || experiment.registry_id != registry.registry_id {
        return Err("unsupported source-registry experiment identity".to_owned());
    }
    if experiment.source_registry_path
        != "tools/energy-dispatch-audit/fixtures/energy-external-benchmark-source-registry-v0.1.json"
    {
        return Err("source-registry fixture path changed".to_owned());
    }
    let actual = sha256_hex(bytes);
    if actual != experiment.source_registry_sha256 {
        return Err(format!("raw registry SHA-256 mismatch: expected {}, got {actual}", experiment.source_registry_sha256));
    }
    let status_count = |status: &str| registry.sources.iter().filter(|source| source.status == status).count();
    if experiment.expected_coverage.source_count != registry.sources.len()
        || experiment.expected_coverage.reference_only_not_imported_count != status_count("reference_only_not_imported")
        || experiment.expected_coverage.ingested_digest_pinned_count != status_count("ingested_digest_pinned")
        || experiment.expected_coverage.qualified_for_benchmark_count != status_count("qualified_for_benchmark")
        || !experiment.claim_ceiling.to_ascii_lowercase().contains("no external dataset")
        || experiment.required_checks.len() != required_checks().len()
        || required_checks().iter().any(|required| !experiment.required_checks.iter().any(|value| value.as_str() == *required))
    {
        return Err("registry status coverage, promotion claim or required check list changed".to_owned());
    }

    let sa = registry.sources.iter().find(|s| s.source_id == "sa_power_supply_reliability_2018_2023").unwrap();
    let city = registry.sources.iter().find(|s| s.source_id == "citylearn_official_v3_0_2").unwrap();
    let eskom = registry.sources.iter().find(|s| s.source_id == "eskom_rolling_demand_dashboard").unwrap();
    if experiment.known_answers.south_african_series_expected_hourly_rows != sa.expected_rows.unwrap_or(0)
        || experiment.known_answers.south_african_series_expected_cadence_seconds != sa.cadence_seconds.unwrap_or(0)
        || experiment.known_answers.citylearn_official_release != city.registry_release.as_deref().unwrap_or("")
        || experiment.known_answers.citylearn_public_dataset_count != 32
        || experiment.known_answers.removed_citylearn_families.iter().any(|name| !city.removed_dataset_families.iter().any(|v| v == name))
        || experiment.known_answers.eskom_rolling_feed_canonical_benchmark
        || experiment.known_answers.imported_external_bytes != 0
        || registry.sources.iter().any(|source| source.raw_sha256.is_some() || source.snapshot_url.is_some())
        || registry.sources.iter().any(|source| source.status != "reference_only_not_imported")
        || !limitation_contains(eskom, "rolling export is mutable")
    {
        return Err("source-registry known answers changed or premature qualification was introduced".to_owned());
    }
    Ok(())
}

fn run_fixture() -> Result<Report, String> {
    if sha256_hex(REGISTRY_BYTES) != REGISTRY_SHA256 {
        return Err("pinned raw source registry bytes changed".to_owned());
    }
    if sha256_hex(EXPERIMENT_BYTES) != EXPERIMENT_SHA256 {
        return Err("pinned raw experiment bytes changed".to_owned());
    }
    let registry: Registry = serde_json::from_slice(REGISTRY_BYTES)
        .map_err(|error| format!("registry parse failed: {error}"))?;
    let experiment: Experiment = serde_json::from_slice(EXPERIMENT_BYTES)
        .map_err(|error| format!("experiment parse failed: {error}"))?;
    validate_registry(&registry)?;
    validate_experiment(&experiment, &registry, REGISTRY_BYTES)?;

    let mut status_counts = BTreeMap::new();
    let source_results = registry.sources.iter().map(|source| {
        *status_counts.entry(source.status.clone()).or_insert(0usize) += 1;
        SourceResult {
            source_id: source.source_id.clone(),
            status: source.status.clone(),
            benchmark_role: source.benchmark_role.clone(),
            expected_rows: source.expected_rows,
            cadence_seconds: source.cadence_seconds,
            raw_sha256: source.raw_sha256.clone(),
            license_status: source.license_status.clone(),
        }
    }).collect();

    Ok(Report {
        schema_version: "energy-external-benchmark-source-registry-report-v0.1",
        registry_id: registry.registry_id,
        status: "PASS_SOURCE_DISCOVERY_CONTRACT_ONLY",
        claim_ceiling: registry.claim_ceiling,
        source_registry_sha256: sha256_hex(REGISTRY_BYTES),
        experiment_fixture_sha256: sha256_hex(EXPERIMENT_BYTES),
        source_count: registry.sources.len(),
        status_counts,
        source_results,
        imported_external_bytes: 0,
        qualified_sources: 0,
        promotion_gate: "DOWNLOAD_AND_LICENSE_REVIEW_REQUIRED_BEFORE_INGESTION_OR_BENCHMARK_CLAIMS",
    })
}

fn main() {
    let output_path = env::args().nth(1);
    match run_fixture() {
        Ok(report) => {
            let serialized = match serde_json::to_vec_pretty(&report) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("FAIL report serialization: {error}");
                    std::process::exit(2);
                }
            };
            if let Some(path) = output_path {
                if let Err(error) = fs::write(&path, &serialized) {
                    eprintln!("FAIL writing report {}: {}", path, error);
                    std::process::exit(2);
                }
            } else {
                println!("{}", String::from_utf8_lossy(&serialized));
            }
            println!("PASS source discovery contract only");
            println!("registry SHA-256: {}", report.source_registry_sha256);
            println!("experiment SHA-256: {}", report.experiment_fixture_sha256);
            println!("sources inventoried: {}", report.source_count);
            println!("external bytes imported: {}", report.imported_external_bytes);
            println!("qualified sources: {}", report.qualified_sources);
        }
        Err(error) => {
            eprintln!("FAIL source registry contract: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> Registry {
        serde_json::from_slice(REGISTRY_BYTES).unwrap()
    }

    #[test]
    fn frozen_source_registry_known_answers() {
        let report = run_fixture().unwrap();
        assert_eq!(report.source_count, 3);
        assert_eq!(report.imported_external_bytes, 0);
        assert_eq!(report.qualified_sources, 0);
        assert_eq!(report.status_counts["reference_only_not_imported"], 3);
        assert_eq!(report.source_results[0].expected_rows, Some(43824));
        assert_eq!(report.source_results[1].status, "reference_only_not_imported");
    }

    #[test]
    fn rejects_duplicate_source_ids() {
        let mut r = registry();
        r.sources[1].source_id = r.sources[0].source_id.clone();
        assert!(validate_registry(&r).unwrap_err().contains("duplicate source ID"));
    }

    #[test]
    fn rejects_qualified_source_without_digest_and_snapshot() {
        let mut r = registry();
        let source = r.sources.iter_mut().find(|s| s.source_id == "citylearn_official_v3_0_2").unwrap();
        source.status = "qualified_for_benchmark".to_owned();
        source.selected_dataset_id = Some("some-dataset".to_owned());
        source.license_status = "rights_reviewed_and_cleared".to_owned();
        source.source_timezone_status = "verified_and_utc_normalization_documented".to_owned();
        assert!(validate_registry(&r).is_err());
    }

    #[test]
    fn rejects_south_african_series_relabelled_as_site_dispatch_evidence() {
        let mut r = registry();
        let source = r.sources.iter_mut().find(|s| s.source_id == "sa_power_supply_reliability_2018_2023").unwrap();
        source.benchmark_role = "site_dispatch_validation".to_owned();
        assert!(validate_registry(&r).is_err());
    }

    #[test]
    fn rejects_selecting_removed_citylearn_dataset_family() {
        let mut r = registry();
        let source = r.sources.iter_mut().find(|s| s.source_id == "citylearn_official_v3_0_2").unwrap();
        source.selected_dataset_id = Some("rec_2023_CORE_30".to_owned());
        assert!(validate_registry(&r).is_err());
    }

    #[test]
    fn rejects_a_rolling_feed_as_canonical_benchmark() {
        let mut r = registry();
        let source = r.sources.iter_mut().find(|s| s.source_id == "eskom_rolling_demand_dashboard").unwrap();
        source.status = "qualified_for_benchmark".to_owned();
        assert!(validate_registry(&r).is_err());
    }

    #[test]
    fn rejects_registry_digest_tampering() {
        let r = registry();
        let e: Experiment = serde_json::from_slice(EXPERIMENT_BYTES).unwrap();
        assert!(validate_experiment(&e, &r, b"{}").unwrap_err().contains("raw registry SHA-256 mismatch"));
    }

    #[test]
    fn repeated_reports_are_deterministic() {
        let a = serde_json::to_vec(&run_fixture().unwrap()).unwrap();
        let b = serde_json::to_vec(&run_fixture().unwrap()).unwrap();
        assert_eq!(a, b);
    }
}
