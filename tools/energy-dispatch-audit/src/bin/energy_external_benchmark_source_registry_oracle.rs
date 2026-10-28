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
const CITYLEARN_MANIFEST_BYTES: &[u8] =
    include_bytes!("../../fixtures/citylearn-challenge-2022-plus-evs-v3.0.2-source-manifest-v0.1.json");
const REGISTRY_SHA256: &str =
    "6481f1d4e4c2cc814582960350c2323b86aea8a388b97c83f7c516785f44783a";
const EXPERIMENT_SHA256: &str =
    "8c61231d68aef4badad3a61b362cb0348dc7177b329fa9f2bc8dc329c015416d";
const CITYLEARN_MANIFEST_SHA256: &str =
    "bd5086f66e47b400a1e37b4f6a6b5fffeabe02bf2c3a7a7c001c108813efe657";
const REGISTRY_SCHEMA: &str = "energy-external-benchmark-source-registry-v0.1";
const EXPERIMENT_SCHEMA: &str = "energy-external-benchmark-source-registry-experiment-v0.1";
const REGISTRY_ID: &str = "energy-external-benchmark-source-registry-2026-10-28";
const CITYLEARN_DATASET_ID: &str = "citylearn_challenge_2022_phase_all_plus_evs";
const CITYLEARN_SOURCE_COMMIT: &str = "834575c1a0194c8ae9d648ae858376a94dfceb78";
const CITYLEARN_SCHEMA_BLOB_SHA1: &str = "5d58d31fd75e4845d73d319c69df34d64223f533";

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit())
}

fn required_checks() -> [&'static str; 13] {
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
        "selected_citylearn_dataset_is_present_in_pinned_release",
        "citylearn_manifest_pins_all_schema_referenced_source_files",
        "forecast_columns_are_not_assumed_causal_without_issue_times",
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
    pinned_source_commit: Option<String>,
    selected_schema_path: Option<String>,
    selected_schema_git_blob_sha1: Option<String>,
    referenced_data_file_count: Option<usize>,
    simulator_step_count: Option<usize>,
    dataset_source_manifest_path: Option<String>,
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
    citylearn_source_manifest_path: String,
    citylearn_source_manifest_sha256: String,
    expected_coverage: ExpectedCoverage,
    known_answers: KnownAnswers,
    claim_ceiling: String,
    required_checks: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CityLearnManifest {
    schema_version: String,
    dataset_id: String,
    source_repository: String,
    source_release_tag: String,
    source_commit: String,
    source_commit_is_pinned: bool,
    schema_path: String,
    schema_git_blob_sha1: String,
    schema_size_bytes: usize,
    start_date: String,
    simulation_start_step: usize,
    simulation_end_step: usize,
    time_steps: usize,
    seconds_per_time_step: u32,
    buildings: usize,
    random_seed: u64,
    dataset_license_status: String,
    forecast_feature_names: Vec<String>,
    forecast_issue_time_status: String,
    external_raw_bytes_committed: bool,
    source_inventory_status: String,
    referenced_csv_count: usize,
    files: Vec<CityLearnFile>,
    required_checks: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CityLearnFile {
    path: String,
    git_blob_sha1: String,
    size_bytes: usize,
    data_rows: usize,
    immutable_source_url: String,
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
    citylearn_selected_dataset_id: String,
    citylearn_source_commit: String,
    citylearn_schema_blob_sha1: String,
    citylearn_referenced_csv_count: usize,
    citylearn_simulation_time_steps: usize,
    citylearn_building_count: usize,
    citylearn_schema_size_bytes: usize,
    citylearn_forecast_issue_times_verified: bool,
    citylearn_dataset_terms_cleared: bool,
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
    citylearn_manifest_sha256: String,
    citylearn_dataset_id: String,
    citylearn_source_commit: String,
    citylearn_schema_git_blob_sha1: String,
    citylearn_source_file_count: usize,
    citylearn_simulation_time_steps: usize,
    citylearn_forecast_issue_times_verified: bool,
    citylearn_dataset_terms_cleared: bool,
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
            || source.selected_dataset_id.as_deref() != Some(CITYLEARN_DATASET_ID)
            || source.pinned_source_commit.as_deref() != Some(CITYLEARN_SOURCE_COMMIT)
            || source.selected_schema_path.as_deref()
                != Some("data/datasets/citylearn_challenge_2022_phase_all_plus_evs/schema.json")
            || source.selected_schema_git_blob_sha1.as_deref() != Some(CITYLEARN_SCHEMA_BLOB_SHA1)
            || source.referenced_data_file_count != Some(30)
            || source.simulator_step_count != Some(8760)
            || source.dataset_source_manifest_path.as_deref()
                != Some("tools/energy-dispatch-audit/fixtures/citylearn-challenge-2022-plus-evs-v3.0.2-source-manifest-v0.1.json")
            || source.status != "reference_only_not_imported"
            || expected_removed.iter().any(|name| !source.removed_dataset_families.iter().any(|v| v == name))
            || !limitation_contains(source, "selected dataset schema is verified at the immutable v3.0.2 source commit")
            || !limitation_contains(source, "forecast issue/publication timestamp contract")
        {
            return Err("CityLearn dataset identity, source pin, removed dataset caveat, or unqualified timing state changed".to_owned());
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

fn validate_citylearn_manifest(
    manifest: &CityLearnManifest,
    registry: &Registry,
) -> Result<(), String> {
    if manifest.schema_version != "citylearn-source-tree-manifest-v0.1"
        || manifest.dataset_id != CITYLEARN_DATASET_ID
        || manifest.source_repository != "https://github.com/citylearn-project/CityLearn"
        || manifest.source_release_tag != "v3.0.2"
        || manifest.source_commit != CITYLEARN_SOURCE_COMMIT
        || !manifest.source_commit_is_pinned
        || manifest.schema_path != "data/datasets/citylearn_challenge_2022_phase_all_plus_evs/schema.json"
        || manifest.schema_git_blob_sha1 != CITYLEARN_SCHEMA_BLOB_SHA1
        || manifest.schema_size_bytes != 27644
        || manifest.start_date != "2024-08-01"
        || manifest.simulation_start_step != 0
        || manifest.simulation_end_step != 8759
        || manifest.time_steps != 8760
        || manifest.seconds_per_time_step != 3600
        || manifest.buildings != 17
        || manifest.random_seed != 2022
        || !manifest.dataset_license_status.contains("not_independently_cleared")
        || manifest.forecast_issue_time_status != "not_verified_from_schema_and_column_names"
        || manifest.external_raw_bytes_committed
        || !manifest.source_inventory_status.contains("remote_git_commit_and_blob_ids_are_inventoried")
        || manifest.referenced_csv_count != 30
        || manifest.files.len() != 30
    {
        return Err("CityLearn manifest identity, time coverage, rights state or claim ceiling changed".to_owned());
    }

    const MANIFEST_CHECKS: [&str; 8] = [
        "exact_source_commit_and_tag",
        "schema_blob_sha1_matches_selected_release",
        "all_schema_referenced_csv_files_are_listed",
        "all_git_blob_ids_are_40_hex",
        "nonempty_data_row_counts_and_file_sizes_are_recorded",
        "forecast_issue_time_is_not_assumed_from_column_name",
        "dataset_license_is_not_assumed_from_software_license",
        "national_or_simulated_data_are_not_claimed_as_south_african_field_evidence",
    ];
    if manifest.required_checks.len() != MANIFEST_CHECKS.len()
        || MANIFEST_CHECKS.iter().any(|required| {
            !manifest.required_checks.iter().any(|value| value.as_str() == *required)
        })
    {
        return Err("CityLearn manifest required-check registry changed".to_owned());
    }

    if manifest.forecast_feature_names.is_empty()
        || !manifest.forecast_feature_names.iter().any(|name| name.contains("predicted"))
    {
        return Err("forecast features are not explicitly registered for timing review".to_owned());
    }

    let expected_names: BTreeSet<String> = (1..=17)
        .map(|index| format!("Building_{index}.csv"))
        .chain([
            "weather.csv".to_owned(),
            "carbon_intensity.csv".to_owned(),
            "pricing.csv".to_owned(),
            "charger_1_1.csv".to_owned(),
            "charger_4_1.csv".to_owned(),
            "charger_5_1.csv".to_owned(),
            "charger_7_1.csv".to_owned(),
            "charger_10_1.csv".to_owned(),
            "charger_12_1.csv".to_owned(),
            "charger_15_1.csv".to_owned(),
            "charger_15_2.csv".to_owned(),
            "deferrable_appliance_1_cycle_profiles.csv".to_owned(),
            "deferrable_appliance_1_flexibility_schedule.csv".to_owned(),
        ])
        .collect();
    let root = "data/datasets/citylearn_challenge_2022_phase_all_plus_evs/";
    let mut paths = BTreeSet::new();
    let mut basenames: BTreeSet<String> = BTreeSet::new();
    for file in &manifest.files {
        if !file.path.starts_with(root)
            || file.path.contains("..")
            || !paths.insert(file.path.as_str())
        {
            return Err(format!("invalid or duplicate CityLearn file path {}", file.path));
        }
        let basename = file.path.strip_prefix(root).unwrap_or_default();
        if basename.contains('/')
            || !basenames.insert(basename.to_owned())
            || !valid_sha1(&file.git_blob_sha1)
            || file.size_bytes == 0
            || file.data_rows == 0
        {
            return Err(format!("invalid Git blob, size, row count or filename for {}", file.path));
        }
        let expected_url = format!(
            "https://github.com/citylearn-project/CityLearn/blob/{}/{}",
            CITYLEARN_SOURCE_COMMIT, file.path
        );
        if file.immutable_source_url != expected_url {
            return Err(format!("non-immutable source URL for {}", file.path));
        }
        let expected_rows = match basename {
            "weather.csv" | "carbon_intensity.csv" | "pricing.csv" => Some(8760),
            "charger_1_1.csv" | "charger_4_1.csv" => Some(9527),
            "charger_5_1.csv" | "charger_7_1.csv" | "charger_10_1.csv"
            | "charger_12_1.csv" | "charger_15_1.csv" | "charger_15_2.csv" => Some(35807),
            "deferrable_appliance_1_cycle_profiles.csv" => Some(315),
            "deferrable_appliance_1_flexibility_schedule.csv" => Some(365),
            name if name.starts_with("Building_") && name.ends_with(".csv") => Some(8760),
            _ => None,
        };
        if expected_rows != Some(file.data_rows) {
            return Err(format!("unexpected row count for {}", file.path));
        }
    }
    if basenames != expected_names {
        return Err("CityLearn manifest file inventory differs from the pinned schema references".to_owned());
    }

    let city = registry.sources.iter()
        .find(|source| source.source_id == "citylearn_official_v3_0_2")
        .ok_or_else(|| "CityLearn source entry missing from registry".to_owned())?;
    if city.selected_dataset_id.as_deref() != Some(manifest.dataset_id.as_str())
        || city.pinned_source_commit.as_deref() != Some(manifest.source_commit.as_str())
        || city.selected_schema_path.as_deref() != Some(manifest.schema_path.as_str())
        || city.selected_schema_git_blob_sha1.as_deref() != Some(manifest.schema_git_blob_sha1.as_str())
        || city.referenced_data_file_count != Some(manifest.referenced_csv_count)
        || city.simulator_step_count != Some(manifest.time_steps)
        || city.dataset_source_manifest_path.as_deref()
            != Some("tools/energy-dispatch-audit/fixtures/citylearn-challenge-2022-plus-evs-v3.0.2-source-manifest-v0.1.json")
        || city.raw_sha256.is_some()
        || city.snapshot_url.is_some()
        || city.status != "reference_only_not_imported"
    {
        return Err("CityLearn source registry and file manifest disagree or imply premature ingestion".to_owned());
    }
    Ok(())
}

fn valid_sha1(value: &str) -> bool {
    value.len() == 40 && value.chars().all(|c| c.is_ascii_hexdigit())
}

fn validate_experiment(
    experiment: &Experiment,
    registry: &Registry,
    registry_bytes: &[u8],
    manifest: &CityLearnManifest,
    manifest_bytes: &[u8],
) -> Result<(), String> {
    if experiment.schema_version != EXPERIMENT_SCHEMA || experiment.registry_id != registry.registry_id {
        return Err("unsupported source-registry experiment identity".to_owned());
    }
    if experiment.source_registry_path
        != "tools/energy-dispatch-audit/fixtures/energy-external-benchmark-source-registry-v0.1.json"
        || experiment.citylearn_source_manifest_path
            != "tools/energy-dispatch-audit/fixtures/citylearn-challenge-2022-plus-evs-v3.0.2-source-manifest-v0.1.json"
    {
        return Err("source registry or CityLearn manifest path changed".to_owned());
    }
    let actual_registry = sha256_hex(registry_bytes);
    if actual_registry != experiment.source_registry_sha256 {
        return Err(format!(
            "raw registry SHA-256 mismatch: expected {}, got {actual_registry}",
            experiment.source_registry_sha256
        ));
    }
    let actual_manifest = sha256_hex(manifest_bytes);
    if actual_manifest != experiment.citylearn_source_manifest_sha256 {
        return Err(format!(
            "raw CityLearn manifest SHA-256 mismatch: expected {}, got {actual_manifest}",
            experiment.citylearn_source_manifest_sha256
        ));
    }

    let status_count = |status: &str| registry.sources.iter().filter(|source| source.status == status).count();
    if experiment.expected_coverage.source_count != registry.sources.len()
        || experiment.expected_coverage.reference_only_not_imported_count != status_count("reference_only_not_imported")
        || experiment.expected_coverage.ingested_digest_pinned_count != status_count("ingested_digest_pinned")
        || experiment.expected_coverage.qualified_for_benchmark_count != status_count("qualified_for_benchmark")
        || !experiment.claim_ceiling.to_ascii_lowercase().contains("no external dataset")
        || experiment.required_checks.len() != required_checks().len()
        || required_checks().iter().any(|required| {
            !experiment.required_checks.iter().any(|value| value.as_str() == *required)
        })
    {
        return Err("registry status coverage, promotion claim or required check list changed".to_owned());
    }

    let sa = registry.sources.iter()
        .find(|source| source.source_id == "sa_power_supply_reliability_2018_2023")
        .ok_or_else(|| "South African source missing from registry".to_owned())?;
    let city = registry.sources.iter()
        .find(|source| source.source_id == "citylearn_official_v3_0_2")
        .ok_or_else(|| "CityLearn source missing from registry".to_owned())?;
    let eskom = registry.sources.iter()
        .find(|source| source.source_id == "eskom_rolling_demand_dashboard")
        .ok_or_else(|| "Eskom source missing from registry".to_owned())?;

    if experiment.known_answers.south_african_series_expected_hourly_rows != sa.expected_rows.unwrap_or(0)
        || experiment.known_answers.south_african_series_expected_cadence_seconds != sa.cadence_seconds.unwrap_or(0)
        || experiment.known_answers.citylearn_official_release != city.registry_release.as_deref().unwrap_or("")
        || experiment.known_answers.citylearn_public_dataset_count != 32
        || experiment.known_answers.removed_citylearn_families.iter().any(|name| {
            !city.removed_dataset_families.iter().any(|value| value == name)
        })
        || experiment.known_answers.eskom_rolling_feed_canonical_benchmark
        || experiment.known_answers.imported_external_bytes != 0
        || experiment.known_answers.citylearn_selected_dataset_id != manifest.dataset_id
        || experiment.known_answers.citylearn_source_commit != manifest.source_commit
        || experiment.known_answers.citylearn_schema_blob_sha1 != manifest.schema_git_blob_sha1
        || experiment.known_answers.citylearn_referenced_csv_count != manifest.referenced_csv_count
        || experiment.known_answers.citylearn_simulation_time_steps != manifest.time_steps
        || experiment.known_answers.citylearn_building_count != manifest.buildings
        || experiment.known_answers.citylearn_schema_size_bytes != manifest.schema_size_bytes
        || experiment.known_answers.citylearn_forecast_issue_times_verified
        || experiment.known_answers.citylearn_dataset_terms_cleared
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
    if sha256_hex(CITYLEARN_MANIFEST_BYTES) != CITYLEARN_MANIFEST_SHA256 {
        return Err("pinned raw CityLearn manifest bytes changed".to_owned());
    }
    let registry: Registry = serde_json::from_slice(REGISTRY_BYTES)
        .map_err(|error| format!("registry parse failed: {error}"))?;
    let experiment: Experiment = serde_json::from_slice(EXPERIMENT_BYTES)
        .map_err(|error| format!("experiment parse failed: {error}"))?;
    let manifest: CityLearnManifest = serde_json::from_slice(CITYLEARN_MANIFEST_BYTES)
        .map_err(|error| format!("CityLearn manifest parse failed: {error}"))?;

    validate_registry(&registry)?;
    validate_citylearn_manifest(&manifest, &registry)?;
    validate_experiment(&experiment, &registry, REGISTRY_BYTES, &manifest, CITYLEARN_MANIFEST_BYTES)?;

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
        status: "PASS_SOURCE_DISCOVERY_AND_PINNED_DATASET_MANIFEST_ONLY",
        claim_ceiling: registry.claim_ceiling,
        source_registry_sha256: sha256_hex(REGISTRY_BYTES),
        experiment_fixture_sha256: sha256_hex(EXPERIMENT_BYTES),
        citylearn_manifest_sha256: sha256_hex(CITYLEARN_MANIFEST_BYTES),
        citylearn_dataset_id: manifest.dataset_id,
        citylearn_source_commit: manifest.source_commit,
        citylearn_schema_git_blob_sha1: manifest.schema_git_blob_sha1,
        citylearn_source_file_count: manifest.files.len(),
        citylearn_simulation_time_steps: manifest.time_steps,
        citylearn_forecast_issue_times_verified: experiment.known_answers.citylearn_forecast_issue_times_verified,
        citylearn_dataset_terms_cleared: experiment.known_answers.citylearn_dataset_terms_cleared,
        source_count: registry.sources.len(),
        status_counts,
        source_results,
        imported_external_bytes: 0,
        qualified_sources: 0,
        promotion_gate: "RAW_FILE_SNAPSHOT_AND_LICENSE_REVIEW_REQUIRED_BEFORE_DATASET_INGESTION_OR_BENCHMARK_CLAIMS",
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

    fn manifest() -> CityLearnManifest {
        serde_json::from_slice(CITYLEARN_MANIFEST_BYTES).unwrap()
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
        assert_eq!(report.citylearn_dataset_id, CITYLEARN_DATASET_ID);
        assert_eq!(report.citylearn_source_commit, CITYLEARN_SOURCE_COMMIT);
        assert_eq!(report.citylearn_schema_git_blob_sha1, CITYLEARN_SCHEMA_BLOB_SHA1);
        assert_eq!(report.citylearn_source_file_count, 30);
        assert_eq!(report.citylearn_simulation_time_steps, 8760);
        assert!(!report.citylearn_forecast_issue_times_verified);
        assert!(!report.citylearn_dataset_terms_cleared);
        assert_eq!(report.status, "PASS_SOURCE_DISCOVERY_AND_PINNED_DATASET_MANIFEST_ONLY");
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
        let m = manifest();
        assert!(validate_experiment(&e, &r, b"{}", &m, CITYLEARN_MANIFEST_BYTES)
            .unwrap_err().contains("raw registry SHA-256 mismatch"));
    }


    #[test]
    fn validates_all_thirty_citylearn_source_file_identities() {
        let m = manifest();
        let r = registry();
        validate_citylearn_manifest(&m, &r).unwrap();
        assert_eq!(m.files.len(), 30);
        assert!(m.files.iter().all(|file| valid_sha1(&file.git_blob_sha1)));
    }

    #[test]
    fn rejects_changed_citylearn_schema_blob_or_missing_file() {
        let mut m = manifest();
        m.schema_git_blob_sha1 = "0000000000000000000000000000000000000000".to_owned();
        assert!(validate_citylearn_manifest(&m, &registry()).is_err());

        let mut m = manifest();
        m.files.pop();
        m.referenced_csv_count -= 1;
        assert!(validate_citylearn_manifest(&m, &registry()).is_err());
    }

    #[test]
    fn rejects_causal_forecast_claim_without_issue_time_evidence() {
        let mut m = manifest();
        m.forecast_issue_time_status = "verified".to_owned();
        assert!(validate_citylearn_manifest(&m, &registry()).is_err());
    }

    #[test]
    fn rejects_a_manifest_with_a_mismatched_immutable_source_url() {
        let mut m = manifest();
        m.files[0].immutable_source_url = "https://example.com/Building_1.csv".to_owned();
        assert!(validate_citylearn_manifest(&m, &registry()).is_err());
    }

    #[test]
    fn repeated_reports_are_deterministic() {
        let a = serde_json::to_vec(&run_fixture().unwrap()).unwrap();
        let b = serde_json::to_vec(&run_fixture().unwrap()).unwrap();
        assert_eq!(a, b);
    }
}
