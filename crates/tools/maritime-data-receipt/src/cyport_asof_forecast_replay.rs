// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

//! Strict, fail-closed policy validator for the CyPort as-of forecast replay contract.
//! It validates source and time-boundary constraints only; it does not fetch archives,
//! certify feature availability, run a historical replay, or fit a model.

use super::{
    JsonValue, array, exact_keys, expect_text, field, integer, parse_strict, require_nonempty_text,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;

const MAX_CONTRACT_BYTES: usize = 128 * 1024;
const SCHEMA_ID: &str = "luminous.symtropy.maritime.cyport-asof-forecast-replay";
const SCHEMA_VERSION: &str = "0.1.0";
const SOURCE_REVISION: &str = "5cb1d8778d3968c5e50d0b8cb8213f2414d692d3";
const SOURCE_PATH: &str = "Tropical_cyclone_interaction/interaction_data.csv";
const SOURCE_BLOB_SHA1: &str = "21beb5b7673918143a0ab108b03acc3cccaeb330";
const SOURCE_BYTES: u64 = 643_047;
const SPLIT_PATH: &str = "docs/tech/CYPORT_EPISODE_SPLIT_V0_1.json";
const SPLIT_SCHEMA_ID: &str = "luminous.symtropy.maritime.cyport-episode-split";
const REQUIRED_FEATURE_FIELDS: &[&str] = &[
    "interaction_id",
    "storm_sid",
    "port_pid",
    "feature_name",
    "feature_value_or_missing_reason",
    "units",
    "source_uri",
    "source_revision_or_product_id",
    "source_artifact_sha256",
    "source_artifact_byte_length",
    "source_publication_time_utc",
    "source_retrieval_time_utc",
    "forecast_valid_start_utc",
    "forecast_valid_end_utc",
    "feature_available_at_utc",
    "source_kind",
    "derivation_id",
    "join_method",
    "storm_crosswalk_evidence_id",
    "target_date_semantics_review_id",
];
const REQUIRED_PROMOTION_GATES: &[&str] = &[
    "PIN_SOURCE_AND_REVIEW_REUSE_TERMS",
    "AUDIT_STORM_ID_CROSSWALK_WITH_AMBIGUOUS_AND_UNMATCHED_COUNTS",
    "VERIFY_TARGET_DATE_TIMEZONE_AND_EVENT_TIME_SEMANTICS",
    "CAPTURE_IMMUTABLE_ARCHIVED_FORECAST_PRODUCTS_AND_SHA256",
    "PROVE_EACH_FEATURE_WAS_PUBLISHED_AND_AVAILABLE_BY_ISSUE_CUTOFF",
    "EXCLUDE_RETROSPECTIVE_BEST_TRACK_AND_POST_EVENT_NETWORK_WINDOWS",
    "LOCK_TRAIN_CALIBRATION_TEST_SPLIT_BEFORE_FIT",
    "REPORT_STORM_CLUSTERED_UNCERTAINTY_AND_POSITIVE_STORM_COUNTS",
    "REQUIRE_INDEPENDENT_EXTERNAL_REPLAY_BEFORE_OPERATIONAL_CLAIMS",
];
const REQUIRED_PRODUCTS: &[&str] = &[
    "Tropical Cyclone Advisory Archive",
    "Marine & Advisory Text Products",
    "Marine Graphical Products",
];
const REQUIRED_LEAD_HOURS: &[u64] = &[24, 48, 72];

#[derive(Debug, Serialize)]
struct ReplayContractReport {
    status: &'static str,
    contract_sha256: String,
    schema_id: &'static str,
    source_identity_bound: bool,
    target_date_precision: &'static str,
    target_timezone_verified: bool,
    archive_candidate_recorded: bool,
    archive_coverage_audited: bool,
    feature_whitelist_count: usize,
    exact_lead_time_claim_authorized: bool,
    calendar_day_lead_claim_authorized: bool,
    replay_performed: bool,
    model_fit_performed: bool,
    operational_authority: bool,
    promotion_gate_count: usize,
    claim_ceiling: &'static str,
}

fn require_string_set(value: &JsonValue, path: &str, expected: &[&str]) -> Result<(), String> {
    let values = array(value, path)?;
    let mut actual = BTreeSet::new();
    for (index, item) in values.iter().enumerate() {
        let item = require_nonempty_text(item, &format!("{path}[{index}]"))?;
        if !actual.insert(item.to_owned()) {
            return Err(format!("{path}: duplicate entry {item:?}"));
        }
    }
    let expected_set: BTreeSet<&str> = expected.iter().copied().collect();
    let actual_set: BTreeSet<&str> = actual.iter().map(String::as_str).collect();
    if actual_set != expected_set {
        return Err(format!(
            "{path}: required set mismatch; missing={:?}, unknown={:?}",
            expected_set
                .difference(&actual_set)
                .copied()
                .collect::<Vec<_>>(),
            actual_set
                .difference(&expected_set)
                .copied()
                .collect::<Vec<_>>()
        ));
    }
    Ok(())
}

fn expect_bool(value: &JsonValue, path: &str, expected: bool) -> Result<(), String> {
    match value {
        JsonValue::Bool(actual) if *actual == expected => Ok(()),
        _ => Err(format!("{path}: expected boolean {expected}")),
    }
}

fn require_integer_list(value: &JsonValue, path: &str, expected: &[u64]) -> Result<(), String> {
    let values = array(value, path)?;
    let actual = values
        .iter()
        .enumerate()
        .map(|(index, value)| integer(value, &format!("{path}[{index}]")))
        .collect::<Result<Vec<_>, _>>()?;
    if actual != expected {
        return Err(format!("{path}: expected {expected:?}, found {actual:?}"));
    }
    Ok(())
}

fn expect_false(value: &JsonValue, path: &str) -> Result<(), String> {
    expect_bool(value, path, false)
}

fn expect_true(value: &JsonValue, path: &str) -> Result<(), String> {
    expect_bool(value, path, true)
}

fn validate_contract(bytes: &[u8]) -> Result<ReplayContractReport, String> {
    if bytes.len() > MAX_CONTRACT_BYTES {
        return Err(format!("contract exceeds {MAX_CONTRACT_BYTES} bytes"));
    }
    let root = parse_strict(bytes)?;
    exact_keys(
        &root,
        &[
            "schema_id",
            "schema_version",
            "status",
            "source",
            "split_manifest",
            "archive_candidate",
            "replay_policy",
            "required_feature_receipt_fields",
            "promotion_gates",
            "future_lead_time_targets",
            "claim_ceiling",
        ],
        "root",
    )?;
    expect_text(
        field(&root, "schema_id", "root")?,
        "root.schema_id",
        SCHEMA_ID,
    )?;
    expect_text(
        field(&root, "schema_version", "root")?,
        "root.schema_version",
        SCHEMA_VERSION,
    )?;
    expect_text(
        field(&root, "status", "root")?,
        "root.status",
        "POLICY_ONLY_NO_REPLAY_EXECUTED",
    )?;

    let source = field(&root, "source", "root")?;
    exact_keys(
        source,
        &[
            "repository",
            "revision",
            "resource_path",
            "git_blob_sha1",
            "byte_length",
            "target_date_field",
            "target_date_precision",
            "target_timezone_status",
        ],
        "source",
    )?;
    expect_text(
        field(source, "repository", "source")?,
        "source.repository",
        "https://github.com/ChenchenMobility/Maritime-Data-CyPort",
    )?;
    expect_text(
        field(source, "revision", "source")?,
        "source.revision",
        SOURCE_REVISION,
    )?;
    expect_text(
        field(source, "resource_path", "source")?,
        "source.resource_path",
        SOURCE_PATH,
    )?;
    expect_text(
        field(source, "git_blob_sha1", "source")?,
        "source.git_blob_sha1",
        SOURCE_BLOB_SHA1,
    )?;
    if integer(
        field(source, "byte_length", "source")?,
        "source.byte_length",
    )? != SOURCE_BYTES
    {
        return Err("source.byte_length: does not match pinned CyPort source".to_owned());
    }
    expect_text(
        field(source, "target_date_field", "source")?,
        "source.target_date_field",
        "start_date",
    )?;
    expect_text(
        field(source, "target_date_precision", "source")?,
        "source.target_date_precision",
        "DATE_ONLY",
    )?;
    expect_text(
        field(source, "target_timezone_status", "source")?,
        "source.target_timezone_status",
        "UNVERIFIED",
    )?;

    let split = field(&root, "split_manifest", "root")?;
    exact_keys(
        split,
        &[
            "path",
            "schema_id",
            "schema_version",
            "split_unit",
            "test_period",
        ],
        "split_manifest",
    )?;
    expect_text(
        field(split, "path", "split_manifest")?,
        "split_manifest.path",
        SPLIT_PATH,
    )?;
    expect_text(
        field(split, "schema_id", "split_manifest")?,
        "split_manifest.schema_id",
        SPLIT_SCHEMA_ID,
    )?;
    expect_text(
        field(split, "schema_version", "split_manifest")?,
        "split_manifest.schema_version",
        "0.1.0",
    )?;
    expect_text(
        field(split, "split_unit", "split_manifest")?,
        "split_manifest.split_unit",
        "SID",
    )?;
    expect_text(
        field(split, "test_period", "split_manifest")?,
        "split_manifest.test_period",
        "2022-2023",
    )?;

    let archive = field(&root, "archive_candidate", "root")?;
    exact_keys(
        archive,
        &[
            "provider",
            "archive_url",
            "product_families",
            "status",
            "completeness_claimed",
            "forecast_archive_may_supply_asof_features",
            "retrospective_best_track_allowed_as_predictor",
            "storm_crosswalk_status",
            "coverage_note",
        ],
        "archive_candidate",
    )?;
    expect_text(
        field(archive, "provider", "archive_candidate")?,
        "archive_candidate.provider",
        "NOAA National Hurricane Center",
    )?;
    expect_text(
        field(archive, "archive_url", "archive_candidate")?,
        "archive_candidate.archive_url",
        "https://www.nhc.noaa.gov/data/",
    )?;
    require_string_set(
        field(archive, "product_families", "archive_candidate")?,
        "archive_candidate.product_families",
        REQUIRED_PRODUCTS,
    )?;
    expect_text(
        field(archive, "status", "archive_candidate")?,
        "archive_candidate.status",
        "CANDIDATE_NOT_AUDITED_ROW_BY_ROW",
    )?;
    expect_false(
        field(archive, "completeness_claimed", "archive_candidate")?,
        "archive_candidate.completeness_claimed",
    )?;
    expect_true(
        field(
            archive,
            "forecast_archive_may_supply_asof_features",
            "archive_candidate",
        )?,
        "archive_candidate.forecast_archive_may_supply_asof_features",
    )?;
    expect_false(
        field(
            archive,
            "retrospective_best_track_allowed_as_predictor",
            "archive_candidate",
        )?,
        "archive_candidate.retrospective_best_track_allowed_as_predictor",
    )?;
    expect_text(
        field(archive, "storm_crosswalk_status", "archive_candidate")?,
        "archive_candidate.storm_crosswalk_status",
        "NOT_BUILT_OR_VERIFIED",
    )?;
    let coverage_note = require_nonempty_text(
        field(archive, "coverage_note", "archive_candidate")?,
        "archive_candidate.coverage_note",
    )?;
    if !coverage_note.to_ascii_lowercase().contains("completeness") {
        return Err("coverage_note must retain its completeness caveat".to_owned());
    }

    let policy = field(&root, "replay_policy", "root")?;
    exact_keys(
        policy,
        &[
            "forecast_issue_time_required",
            "feature_availability_at_cutoff_required",
            "immutable_source_artifact_hash_required",
            "source_publication_time_required",
            "retrieval_time_required",
            "forecast_valid_time_required",
            "units_and_transformation_lineage_required",
            "feature_whitelist",
            "exact_lead_time_claim_allowed",
            "calendar_day_lead_claim_allowed",
            "replay_run_allowed",
            "blocked_reason",
        ],
        "replay_policy",
    )?;
    for name in [
        "forecast_issue_time_required",
        "feature_availability_at_cutoff_required",
        "immutable_source_artifact_hash_required",
        "source_publication_time_required",
        "retrieval_time_required",
        "forecast_valid_time_required",
        "units_and_transformation_lineage_required",
    ] {
        expect_true(
            field(policy, name, "replay_policy")?,
            &format!("replay_policy.{name}"),
        )?;
    }
    let feature_whitelist = field(policy, "feature_whitelist", "replay_policy")?;
    if !array(feature_whitelist, "replay_policy.feature_whitelist")?.is_empty() {
        return Err(
            "replay_policy.feature_whitelist must stay empty until field evidence is reviewed"
                .to_owned(),
        );
    }
    expect_false(
        field(policy, "exact_lead_time_claim_allowed", "replay_policy")?,
        "replay_policy.exact_lead_time_claim_allowed",
    )?;
    expect_false(
        field(policy, "calendar_day_lead_claim_allowed", "replay_policy")?,
        "replay_policy.calendar_day_lead_claim_allowed",
    )?;
    expect_false(
        field(policy, "replay_run_allowed", "replay_policy")?,
        "replay_policy.replay_run_allowed",
    )?;
    let blocked_reason = require_nonempty_text(
        field(policy, "blocked_reason", "replay_policy")?,
        "replay_policy.blocked_reason",
    )?;
    if !blocked_reason.contains("date-only") && !blocked_reason.contains("date only") {
        return Err("blocked_reason must state the date-only target-time limit".to_owned());
    }

    require_string_set(
        field(&root, "required_feature_receipt_fields", "root")?,
        "required_feature_receipt_fields",
        REQUIRED_FEATURE_FIELDS,
    )?;
    require_string_set(
        field(&root, "promotion_gates", "root")?,
        "promotion_gates",
        REQUIRED_PROMOTION_GATES,
    )?;

    let lead_targets = field(&root, "future_lead_time_targets", "root")?;
    exact_keys(
        lead_targets,
        &[
            "status",
            "candidate_lead_hours",
            "requires_timestamped_targets",
            "requires_issue_time_and_valid_time",
        ],
        "future_lead_time_targets",
    )?;
    expect_text(
        field(lead_targets, "status", "future_lead_time_targets")?,
        "future_lead_time_targets.status",
        "FUTURE_TARGET_ONLY_NOT_CURRENTLY_EVALUABLE",
    )?;
    require_integer_list(
        field(
            lead_targets,
            "candidate_lead_hours",
            "future_lead_time_targets",
        )?,
        "future_lead_time_targets.candidate_lead_hours",
        REQUIRED_LEAD_HOURS,
    )?;
    expect_true(
        field(
            lead_targets,
            "requires_timestamped_targets",
            "future_lead_time_targets",
        )?,
        "future_lead_time_targets.requires_timestamped_targets",
    )?;
    expect_true(
        field(
            lead_targets,
            "requires_issue_time_and_valid_time",
            "future_lead_time_targets",
        )?,
        "future_lead_time_targets.requires_issue_time_and_valid_time",
    )?;

    let claims = array(field(&root, "claim_ceiling", "root")?, "claim_ceiling")?;
    if claims.len() < 3 {
        return Err("claim_ceiling must enumerate replay and operational non-claims".to_owned());
    }
    for (index, claim) in claims.iter().enumerate() {
        require_nonempty_text(claim, &format!("claim_ceiling[{index}]"))?;
    }

    let digest = Sha256::digest(bytes);
    let contract_sha256: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(ReplayContractReport {
        status: "ASOF_REPLAY_POLICY_VALIDATED_NOT_REPLAYED",
        contract_sha256,
        schema_id: SCHEMA_ID,
        source_identity_bound: true,
        target_date_precision: "DATE_ONLY",
        target_timezone_verified: false,
        archive_candidate_recorded: true,
        archive_coverage_audited: false,
        feature_whitelist_count: 0,
        exact_lead_time_claim_authorized: false,
        calendar_day_lead_claim_authorized: false,
        replay_performed: false,
        model_fit_performed: false,
        operational_authority: false,
        promotion_gate_count: REQUIRED_PROMOTION_GATES.len(),
        claim_ceiling: "Policy validation only; no model fit, replay, or operational authority.",
    })
}

pub fn run_cli(path: &str) -> Result<(), String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("cannot stat as-of replay contract {path}: {error}"))?;
    if metadata.len() > MAX_CONTRACT_BYTES as u64 {
        return Err(format!(
            "as-of replay contract exceeds {MAX_CONTRACT_BYTES} bytes"
        ));
    }
    let bytes = fs::read(path)
        .map_err(|error| format!("cannot read as-of replay contract {path}: {error}"))?;
    let report = validate_contract(&bytes)?;
    println!(
        "{}",
        serde_json::to_string(&report)
            .map_err(|error| format!("cannot serialize as-of replay contract report: {error}"))?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const CONTRACT: &[u8] =
        include_bytes!("../../../../docs/tech/CYPORT_ASOF_FORECAST_REPLAY_V0_1.json");

    #[test]
    fn contract_validates_without_claiming_that_replay_ran() {
        let report = validate_contract(CONTRACT).expect("as-of replay contract should validate");
        assert_eq!(report.status, "ASOF_REPLAY_POLICY_VALIDATED_NOT_REPLAYED");
        assert!(report.source_identity_bound);
        assert!(report.archive_candidate_recorded);
        assert!(!report.archive_coverage_audited);
        assert_eq!(report.feature_whitelist_count, 0);
        assert!(!report.exact_lead_time_claim_authorized);
        assert!(!report.replay_performed);
        assert!(!report.model_fit_performed);
        assert!(!report.operational_authority);
    }

    #[test]
    fn a_predictor_cannot_be_enabled_without_a_new_reviewed_contract() {
        let mut value: Value = serde_json::from_slice(CONTRACT).expect("valid contract JSON");
        value["replay_policy"]["feature_whitelist"] = serde_json::json!(["WIND"]);
        let changed = serde_json::to_vec(&value).expect("serialize mutated contract");
        assert!(
            validate_contract(&changed)
                .unwrap_err()
                .contains("feature_whitelist")
        );
    }

    #[test]
    fn exact_hour_lead_claims_fail_while_target_time_is_date_only() {
        let mut value: Value = serde_json::from_slice(CONTRACT).expect("valid contract JSON");
        value["replay_policy"]["exact_lead_time_claim_allowed"] = serde_json::json!(true);
        let changed = serde_json::to_vec(&value).expect("serialize mutated contract");
        assert!(
            validate_contract(&changed)
                .unwrap_err()
                .contains("exact_lead_time_claim_allowed")
        );
    }

    #[test]
    fn post_storm_best_track_cannot_be_promoted_to_forecast_predictor() {
        let mut value: Value = serde_json::from_slice(CONTRACT).expect("valid contract JSON");
        value["archive_candidate"]["retrospective_best_track_allowed_as_predictor"] =
            serde_json::json!(true);
        let changed = serde_json::to_vec(&value).expect("serialize mutated contract");
        assert!(
            validate_contract(&changed)
                .unwrap_err()
                .contains("retrospective_best_track_allowed_as_predictor")
        );
    }

    #[test]
    fn exact_pinned_source_identity_is_required() {
        let mut value: Value = serde_json::from_slice(CONTRACT).expect("valid contract JSON");
        value["source"]["revision"] = serde_json::json!("main");
        let changed = serde_json::to_vec(&value).expect("serialize mutated contract");
        assert!(
            validate_contract(&changed)
                .unwrap_err()
                .contains("source.revision")
        );
    }
}
