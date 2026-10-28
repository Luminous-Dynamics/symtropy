// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

//! Validates the prospective historical validation-source registry.
//!
//! This checker validates source metadata and corpus design only. It does not
//! download source data, establish licence rights, import observations, or run
//! a historical model evaluation.

use serde::Serialize;
use std::collections::BTreeSet;
use std::fs;

use super::JsonValue;

const MAX_MANIFEST_BYTES: usize = 500_000;
const ROOT_KEYS: &[&str] = &[
    "schema_id",
    "schema_version",
    "registry_status",
    "claim_ceiling",
    "data_capture_performed",
    "model_validation_performed",
    "sources",
    "corpus_design",
    "episode_record_contract",
    "capture_gates",
];
const SOURCE_KEYS: &[&str] = &[
    "source_id",
    "source_kind",
    "publisher",
    "landing_uri",
    "metadata_uri",
    "immutable_revision",
    "resource_path",
    "resource_identifier",
    "source_git_blob_sha1",
    "declared_resource_byte_length",
    "last_modified_utc",
    "last_modified_date",
    "access_level",
    "license_status",
    "license_uri",
    "capture_status",
    "artifact_byte_length",
    "artifact_sha256",
    "captured_at_utc",
    "intended_role",
    "described_content",
    "known_limitations",
    "use_decision",
    "required_capture_fields",
];
const EXPECTED_EPISODE_FIELDS: &[&str] = &[
    "episode_id",
    "source_references",
    "affected_entity_ids",
    "control_entity_ids_or_unavailable_reason",
    "hazard_class",
    "event_time_start",
    "event_time_end_or_unknown_reason",
    "decision_time_cutoff",
    "impact_definition",
    "recovery_definition",
    "observation_refs",
    "label_quality",
    "split_assignment",
];

#[derive(Debug, Serialize, PartialEq, Eq)]
struct HistoricalSourceSummary {
    source_id: String,
    source_kind: String,
    immutable_revision: Option<String>,
    source_git_blob_sha1: Option<String>,
    declared_resource_byte_length: Option<u64>,
    license_status: String,
    capture_status: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct HistoricalCorpusReport {
    status: &'static str,
    schema_id: &'static str,
    schema_version: &'static str,
    manifest_sha256: String,
    manifest_byte_length: usize,
    source_count: usize,
    immutable_revision_pinned_count: usize,
    captured_artifact_count: usize,
    sources: Vec<HistoricalSourceSummary>,
    pilot_episode_target: u64,
    pilot_disruption_class_target: u64,
    broader_claim_episode_target: u64,
    data_capture_performed: bool,
    model_validation_performed: bool,
    scored_corpus_admitted: bool,
    claim_ceiling: &'static str,
}

fn field_text<'a>(root: &'a JsonValue, key: &str, path: &str) -> Result<&'a str, String> {
    super::text(super::field(root, key, path)?, &format!("{path}.{key}"))
}

fn nonempty_text<'a>(value: &'a JsonValue, path: &str) -> Result<&'a str, String> {
    let text = super::text(value, path)?;
    if text.trim().is_empty() {
        return Err(format!("{path}: empty string is forbidden"));
    }
    Ok(text)
}

fn nonempty_array(value: &JsonValue, path: &str) -> Result<Vec<String>, String> {
    let array = super::array(value, path)?;
    if array.is_empty() {
        return Err(format!("{path}: array must not be empty"));
    }
    let mut seen = BTreeSet::new();
    let mut items = Vec::with_capacity(array.len());
    for (index, raw) in array.iter().enumerate() {
        let item = nonempty_text(raw, &format!("{path}[{index}]"))?.to_owned();
        if !seen.insert(item.clone()) {
            return Err(format!("{path}[{index}]: duplicate item {item:?}"));
        }
        items.push(item);
    }
    Ok(items)
}

fn expect_array_contains_all(
    values: &[String],
    expected: &[&str],
    path: &str,
) -> Result<(), String> {
    let actual: BTreeSet<&str> = values.iter().map(String::as_str).collect();
    let missing: Vec<&str> = expected
        .iter()
        .copied()
        .filter(|value| !actual.contains(value))
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!("{path}: required entries missing {missing:?}"))
    }
}

fn is_lower_hex(value: &str, expected_len: usize) -> bool {
    value.len() == expected_len
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_url(value: &JsonValue, path: &str, allow_http: bool) -> Result<(), String> {
    let uri = nonempty_text(value, path)?;
    let valid_scheme = uri.starts_with("https://") || (allow_http && uri.starts_with("http://"));
    if !valid_scheme || uri.ends_with("://") {
        return Err(format!(
            "{path}: expected a non-empty HTTP(S) URI allowed by policy"
        ));
    }
    Ok(())
}

fn validate_date(value: &JsonValue, path: &str) -> Result<(), String> {
    match value {
        JsonValue::Null => Ok(()),
        JsonValue::String(date) => {
            let bytes = date.as_bytes();
            if bytes.len() != 10
                || bytes.get(4) != Some(&b'-')
                || bytes.get(7) != Some(&b'-')
                || !bytes
                    .iter()
                    .enumerate()
                    .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
            {
                return Err(format!("{path}: expected YYYY-MM-DD or null"));
            }
            let year: u32 = date[0..4]
                .parse()
                .map_err(|_| format!("{path}: invalid year"))?;
            let month: u32 = date[5..7]
                .parse()
                .map_err(|_| format!("{path}: invalid month"))?;
            let day: u32 = date[8..10]
                .parse()
                .map_err(|_| format!("{path}: invalid day"))?;
            let leap =
                year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
            let max_day = match month {
                1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
                4 | 6 | 9 | 11 => 30,
                2 if leap => 29,
                2 => 28,
                _ => 0,
            };
            if day == 0 || day > max_day {
                return Err(format!("{path}: invalid calendar date {date:?}"));
            }
            Ok(())
        }
        _ => Err(format!("{path}: expected date string or null")),
    }
}

fn validate_uncaptured_artifact(root: &JsonValue, path: &str) -> Result<(), String> {
    let capture_status = field_text(root, "capture_status", path)?;
    if capture_status != "not_captured" {
        return Err(format!(
            "{path}.capture_status: only not_captured is allowed in registry v0.1"
        ));
    }
    for (key, expected_path) in [
        ("artifact_byte_length", "artifact_byte_length"),
        ("artifact_sha256", "artifact_sha256"),
        ("captured_at_utc", "captured_at_utc"),
    ] {
        if !matches!(super::field(root, key, path)?, JsonValue::Null) {
            return Err(format!(
                "{path}.{expected_path}: captured artifact metadata cannot be populated before capture"
            ));
        }
    }
    Ok(())
}

fn validate_source(source: &JsonValue, index: usize) -> Result<HistoricalSourceSummary, String> {
    let path = format!("manifest.sources[{index}]");
    super::exact_keys(source, SOURCE_KEYS, &path)?;

    let source_id = field_text(source, "source_id", &path)?.to_owned();
    let source_kind = field_text(source, "source_kind", &path)?.to_owned();
    nonempty_text(
        super::field(source, "publisher", &path)?,
        &format!("{path}.publisher"),
    )?;
    let source_git_blob_sha1 = match super::field(source, "source_git_blob_sha1", &path)? {
        JsonValue::Null => None,
        value => {
            let hash = nonempty_text(value, &format!("{path}.source_git_blob_sha1"))?.to_owned();
            if !is_lower_hex(&hash, 40) {
                return Err(format!(
                    "{path}.source_git_blob_sha1: expected lowercase 40-character Git blob SHA-1 or null"
                ));
            }
            Some(hash)
        }
    };
    let declared_resource_byte_length =
        match super::field(source, "declared_resource_byte_length", &path)? {
            JsonValue::Null => None,
            value => Some(super::integer(
                value,
                &format!("{path}.declared_resource_byte_length"),
            )?),
        };
    if source_git_blob_sha1.is_some() != declared_resource_byte_length.is_some() {
        return Err(format!(
            "{path}: source Git blob SHA-1 and declared resource byte length must be jointly present or absent"
        ));
    }
    validate_url(
        super::field(source, "landing_uri", &path)?,
        &format!("{path}.landing_uri"),
        false,
    )?;
    validate_url(
        super::field(source, "metadata_uri", &path)?,
        &format!("{path}.metadata_uri"),
        false,
    )?;
    nonempty_text(
        super::field(source, "resource_path", &path)?,
        &format!("{path}.resource_path"),
    )?;
    nonempty_text(
        super::field(source, "resource_identifier", &path)?,
        &format!("{path}.resource_identifier"),
    )?;
    validate_date(
        super::field(source, "last_modified_date", &path)?,
        &format!("{path}.last_modified_date"),
    )?;
    match super::field(source, "last_modified_utc", &path)? {
        JsonValue::Null => {}
        JsonValue::String(timestamp) if super::valid_utc_timestamp(timestamp) => {}
        _ => {
            return Err(format!(
                "{path}.last_modified_utc: expected a valid RFC3339 UTC timestamp or null"
            ));
        }
    }
    nonempty_text(
        super::field(source, "access_level", &path)?,
        &format!("{path}.access_level"),
    )?;
    nonempty_text(
        super::field(source, "intended_role", &path)?,
        &format!("{path}.intended_role"),
    )?;
    nonempty_text(
        super::field(source, "use_decision", &path)?,
        &format!("{path}.use_decision"),
    )?;

    let described = nonempty_array(
        super::field(source, "described_content", &path)?,
        &format!("{path}.described_content"),
    )?;
    let limitations = nonempty_array(
        super::field(source, "known_limitations", &path)?,
        &format!("{path}.known_limitations"),
    )?;
    let capture_fields = nonempty_array(
        super::field(source, "required_capture_fields", &path)?,
        &format!("{path}.required_capture_fields"),
    )?;
    expect_array_contains_all(
        &capture_fields,
        &["retrieval_timestamp_utc"],
        &format!("{path}.required_capture_fields"),
    )?;
    if described.is_empty() || limitations.is_empty() {
        return Err(format!(
            "{path}: content description and limitations are required"
        ));
    }

    let license_status = field_text(source, "license_status", &path)?.to_owned();
    let license_uri = super::field(source, "license_uri", &path)?;
    match source_id.as_str() {
        "port_disruption_database" => {
            if source_kind != "research_incident_dataset" {
                return Err(format!(
                    "{path}.source_kind: unexpected kind for research incident candidate"
                ));
            }
            let revision = nonempty_text(
                super::field(source, "immutable_revision", &path)?,
                &format!("{path}.immutable_revision"),
            )?;
            if !is_lower_hex(revision, 40) {
                return Err(format!(
                    "{path}.immutable_revision: expected pinned 40-character lowercase Git commit"
                ));
            }
            if source_git_blob_sha1.is_some() || declared_resource_byte_length.is_some() {
                return Err(format!(
                    "{path}: uninspected workbook resource metadata cannot be invented"
                ));
            }
            if license_status != "unverified" || !matches!(license_uri, JsonValue::Null) {
                return Err(format!(
                    "{path}: research workbook rights must remain explicitly unverified"
                ));
            }
            if !field_text(source, "metadata_uri", &path)?.contains(revision) {
                return Err(format!(
                    "{path}.metadata_uri: must pin the same immutable revision as immutable_revision"
                ));
            }
            if !field_text(source, "resource_identifier", &path)?.contains(revision) {
                return Err(format!(
                    "{path}.resource_identifier: must bind resources to the immutable revision"
                ));
            }
            if field_text(source, "use_decision", &path)?
                != "metadata_inspection_only_until_rights_and_terms_are_confirmed"
            {
                return Err(format!(
                    "{path}.use_decision: unverified rights must block data use"
                ));
            }
            if field_text(source, "access_level", &path)? != "public_repository" {
                return Err(format!("{path}.access_level: expected public_repository"));
            }
            expect_array_contains_all(
                &capture_fields,
                &[
                    "exact_resource_byte_length",
                    "exact_resource_sha256",
                    "rights_or_license_evidence",
                ],
                &format!("{path}.required_capture_fields"),
            )?;
        }
        "cyport_us_port_tropical_cyclone_resilience" => {
            if source_kind != "research_incident_dataset" {
                return Err(format!(
                    "{path}.source_kind: unexpected kind for CyPort research candidate"
                ));
            }
            let revision = nonempty_text(
                super::field(source, "immutable_revision", &path)?,
                &format!("{path}.immutable_revision"),
            )?;
            if !is_lower_hex(revision, 40) {
                return Err(format!(
                    "{path}.immutable_revision: expected pinned 40-character lowercase Git commit"
                ));
            }
            if field_text(source, "resource_path", &path)?
                != "Tropical_cyclone_interaction/interaction_data.csv"
            {
                return Err(format!(
                    "{path}.resource_path: only the pinned interaction table is admitted by this profiler"
                ));
            }
            if source_git_blob_sha1.as_deref() != Some("21beb5b7673918143a0ab108b03acc3cccaeb330")
                || declared_resource_byte_length != Some(643_047)
            {
                return Err(format!(
                    "{path}: pinned interaction CSV Git blob ID and byte length do not match the audited source tree"
                ));
            }
            let resource_identifier = field_text(source, "resource_identifier", &path)?;
            for identity_component in [
                "Tropical_cyclone_interaction/interaction_data.csv",
                "git_blob_sha1=21beb5b7673918143a0ab108b03acc3cccaeb330",
                "bytes=643047",
            ] {
                if !resource_identifier.contains(identity_component) {
                    return Err(format!(
                        "{path}.resource_identifier: missing pinned source identity component {identity_component:?}"
                    ));
                }
            }
            if license_status != "readme_declares_cc_by_4_0" {
                return Err(format!(
                    "{path}.license_status: declared CC BY 4.0 status must remain explicit"
                ));
            }
            super::expect_text(
                license_uri,
                &format!("{path}.license_uri"),
                "https://creativecommons.org/licenses/by/4.0/",
            )?;
            if !field_text(source, "metadata_uri", &path)?.contains(revision) {
                return Err(format!(
                    "{path}.metadata_uri: must pin the same immutable revision as immutable_revision"
                ));
            }
            if !field_text(source, "resource_identifier", &path)?.contains(revision) {
                return Err(format!(
                    "{path}.resource_identifier: must bind resources to the immutable revision"
                ));
            }
            if field_text(source, "use_decision", &path)?
                != "primary_candidate_after_component_rights_and_schema_review"
            {
                return Err(format!(
                    "{path}.use_decision: component-level rights and schema review are required"
                ));
            }
            if field_text(source, "access_level", &path)? != "public_repository" {
                return Err(format!("{path}.access_level: expected public_repository"));
            }
            expect_array_contains_all(
                &capture_fields,
                &[
                    "exact_resource_byte_length",
                    "exact_resource_sha256",
                    "rights_or_license_evidence",
                    "component_level_terms_snapshot",
                    "raw_vs_derived_field_map",
                    "source_git_blob_sha1",
                    "declared_resource_byte_length",
                    "git_blob_sha1_recomputation",
                ],
                &format!("{path}.required_capture_fields"),
            )?;
        }
        "bts_port_throughput_metrics" => {
            if source_kind != "government_port_performance_dataset" {
                return Err(format!(
                    "{path}.source_kind: unexpected kind for BTS candidate"
                ));
            }
            if !matches!(
                super::field(source, "immutable_revision", &path)?,
                JsonValue::Null
            ) {
                return Err(format!(
                    "{path}.immutable_revision: mutable API view must not be treated as a content revision"
                ));
            }
            if source_git_blob_sha1.is_some() || declared_resource_byte_length.is_some() {
                return Err(format!(
                    "{path}: mutable API view cannot claim an immutable Git blob identity"
                ));
            }
            if license_status != "catalog_declares_public_domain_label" {
                return Err(format!(
                    "{path}.license_status: source catalogue's declared label must be explicit"
                ));
            }
            validate_url(license_uri, &format!("{path}.license_uri"), true)?;
            if field_text(source, "use_decision", &path)?
                != "candidate_for_baseline_context_after_capture_and_schema_review"
            {
                return Err(format!(
                    "{path}.use_decision: BTS summary is not admitted as incident truth"
                ));
            }
            if field_text(source, "access_level", &path)? != "public" {
                return Err(format!("{path}.access_level: expected public"));
            }
            expect_array_contains_all(
                &capture_fields,
                &[
                    "exact_response_byte_length",
                    "exact_response_sha256",
                    "license_metadata_snapshot",
                ],
                &format!("{path}.required_capture_fields"),
            )?;
        }
        _ => {
            return Err(format!(
                "{path}.source_id: unknown source identifier {source_id:?}"
            ));
        }
    }

    validate_uncaptured_artifact(source, &path)?;

    Ok(HistoricalSourceSummary {
        source_id,
        source_kind,
        immutable_revision: match super::field(source, "immutable_revision", &path)? {
            JsonValue::String(value) => Some(value.clone()),
            JsonValue::Null => None,
            _ => {
                return Err(format!(
                    "{path}.immutable_revision: expected string or null"
                ));
            }
        },
        source_git_blob_sha1,
        declared_resource_byte_length,
        license_status,
        capture_status: "not_captured".to_owned(),
    })
}

fn validate_corpus_design(root: &JsonValue) -> Result<(u64, u64, u64), String> {
    let design = super::field(root, "corpus_design", "manifest")?;
    super::exact_keys(
        design,
        &[
            "split_unit",
            "leakage_policy",
            "holdout_policy",
            "minimum_pilot_targets",
            "broader_claim_targets",
            "baseline_models",
            "reported_metrics",
            "forbidden_inferences",
        ],
        "manifest.corpus_design",
    )?;
    if field_text(design, "split_unit", "manifest.corpus_design")?
        != "independent_disruption_episode"
    {
        return Err(
            "corpus_design.split_unit: split by independent episode to prevent leakage".to_owned(),
        );
    }
    for key in ["leakage_policy", "holdout_policy"] {
        nonempty_text(
            super::field(design, key, "manifest.corpus_design")?,
            &format!("manifest.corpus_design.{key}"),
        )?;
    }

    let pilot = super::field(design, "minimum_pilot_targets", "manifest.corpus_design")?;
    super::exact_keys(
        pilot,
        &[
            "independent_episodes",
            "distinct_disruption_classes",
            "separate_control_port_series_when_available",
        ],
        "manifest.corpus_design.minimum_pilot_targets",
    )?;
    let pilot_episodes = super::integer(
        super::field(pilot, "independent_episodes", "minimum_pilot_targets")?,
        "minimum_pilot_targets.independent_episodes",
    )?;
    let pilot_classes = super::integer(
        super::field(
            pilot,
            "distinct_disruption_classes",
            "minimum_pilot_targets",
        )?,
        "minimum_pilot_targets.distinct_disruption_classes",
    )?;
    if pilot_episodes < 10 || pilot_classes < 3 {
        return Err(
            "pilot targets must include at least 10 episodes and 3 disruption classes".to_owned(),
        );
    }
    if !super::bool_value(
        super::field(
            pilot,
            "separate_control_port_series_when_available",
            "minimum_pilot_targets",
        )?,
        "minimum_pilot_targets.separate_control_port_series_when_available",
    )? {
        return Err("pilot must preserve unaffected/control series where available".to_owned());
    }

    let broad = super::field(design, "broader_claim_targets", "manifest.corpus_design")?;
    super::exact_keys(
        broad,
        &[
            "independent_episodes",
            "temporally_held_out_evaluation_required",
            "leave_one_port_out_when_supported",
        ],
        "manifest.corpus_design.broader_claim_targets",
    )?;
    let broad_episodes = super::integer(
        super::field(broad, "independent_episodes", "broader_claim_targets")?,
        "broader_claim_targets.independent_episodes",
    )?;
    if broad_episodes < 30 {
        return Err("broader empirical claims require a declared target of at least 30 independent episodes".to_owned());
    }
    for key in [
        "temporally_held_out_evaluation_required",
        "leave_one_port_out_when_supported",
    ] {
        if !super::bool_value(
            super::field(broad, key, "broader_claim_targets")?,
            &format!("broader_claim_targets.{key}"),
        )? {
            return Err(format!(
                "broader_claim_targets.{key}: required evaluation control must remain enabled"
            ));
        }
    }

    let baselines = nonempty_array(
        super::field(design, "baseline_models", "manifest.corpus_design")?,
        "corpus_design.baseline_models",
    )?;
    expect_array_contains_all(
        &baselines,
        &[
            "persistence_no_new_information",
            "transparent_static_capacity_loss_and_recovery",
        ],
        "corpus_design.baseline_models",
    )?;

    let metrics = nonempty_array(
        super::field(design, "reported_metrics", "manifest.corpus_design")?,
        "corpus_design.reported_metrics",
    )?;
    expect_array_contains_all(
        &metrics,
        &[
            "per_service_trajectory_error",
            "integrated_unmet_service_error",
            "observed_recovery_time_error",
            "right_censored_recovery_count",
            "missingness_and_revision_profile",
        ],
        "corpus_design.reported_metrics",
    )?;

    let prohibited = nonempty_array(
        super::field(design, "forbidden_inferences", "manifest.corpus_design")?,
        "corpus_design.forbidden_inferences",
    )?;
    expect_array_contains_all(
        &prohibited,
        &[
            "publicly accessible means redistributable",
            "vessel activity equals cargo throughput",
            "missing observation equals zero service",
            "no observed recovery equals zero recovery time",
            "synthetic test success proves historical predictive validity",
        ],
        "corpus_design.forbidden_inferences",
    )?;
    Ok((pilot_episodes, pilot_classes, broad_episodes))
}

fn validate_episode_record_contract(root: &JsonValue) -> Result<(), String> {
    let contract = super::field(root, "episode_record_contract", "manifest")?;
    super::exact_keys(
        contract,
        &[
            "required_fields",
            "time_semantics",
            "censoring_semantics",
            "counterfactual_semantics",
        ],
        "manifest.episode_record_contract",
    )?;
    let fields = nonempty_array(
        super::field(contract, "required_fields", "episode_record_contract")?,
        "episode_record_contract.required_fields",
    )?;
    expect_array_contains_all(
        &fields,
        EXPECTED_EPISODE_FIELDS,
        "episode_record_contract.required_fields",
    )?;
    for key in [
        "time_semantics",
        "censoring_semantics",
        "counterfactual_semantics",
    ] {
        nonempty_text(
            super::field(contract, key, "episode_record_contract")?,
            &format!("episode_record_contract.{key}"),
        )?;
    }
    let censoring = field_text(contract, "censoring_semantics", "episode_record_contract")?;
    if !censoring.contains("right_censored") || !censoring.contains("zero") {
        return Err("episode_record_contract.censoring_semantics: right-censoring and zero-time confusion must be explicitly prevented".to_owned());
    }
    Ok(())
}

fn evaluate_bytes(bytes: &[u8]) -> Result<HistoricalCorpusReport, String> {
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err(format!(
            "historical corpus manifest exceeds {MAX_MANIFEST_BYTES}-byte size bound"
        ));
    }
    let root = super::parse_strict(bytes)?;
    super::exact_keys(&root, ROOT_KEYS, "manifest")?;
    super::expect_text(
        super::field(&root, "schema_id", "manifest")?,
        "manifest.schema_id",
        "luminous.symtropy.maritime-historical-validation-corpus",
    )?;
    super::expect_text(
        super::field(&root, "schema_version", "manifest")?,
        "manifest.schema_version",
        "0.1.0",
    )?;
    super::expect_text(
        super::field(&root, "registry_status", "manifest")?,
        "manifest.registry_status",
        "prospective_sources_not_captured",
    )?;
    super::expect_text(
        super::field(&root, "claim_ceiling", "manifest")?,
        "manifest.claim_ceiling",
        "DATASET_DISCOVERY_AND_CORPUS_DESIGN_ONLY",
    )?;
    if super::bool_value(
        super::field(&root, "data_capture_performed", "manifest")?,
        "manifest.data_capture_performed",
    )? {
        return Err(
            "manifest.data_capture_performed: no capture is claimed by this registry".to_owned(),
        );
    }
    if super::bool_value(
        super::field(&root, "model_validation_performed", "manifest")?,
        "manifest.model_validation_performed",
    )? {
        return Err(
            "manifest.model_validation_performed: source registry does not validate a model"
                .to_owned(),
        );
    }

    let sources = super::array(
        super::field(&root, "sources", "manifest")?,
        "manifest.sources",
    )?;
    if sources.len() != 3 {
        return Err(format!(
            "manifest.sources: expected three source candidates, found {}",
            sources.len()
        ));
    }
    let mut summaries = Vec::with_capacity(sources.len());
    let mut ids = BTreeSet::new();
    let mut pinned_count = 0usize;
    for (index, source) in sources.iter().enumerate() {
        let summary = validate_source(source, index)?;
        if !ids.insert(summary.source_id.clone()) {
            return Err(format!(
                "manifest.sources[{index}].source_id: duplicate source ID"
            ));
        }
        if summary.immutable_revision.is_some() {
            pinned_count += 1;
        }
        summaries.push(summary);
    }
    if !ids.contains("port_disruption_database")
        || !ids.contains("cyport_us_port_tropical_cyclone_resilience")
        || !ids.contains("bts_port_throughput_metrics")
    {
        return Err("manifest.sources: CyPort, legacy research candidate and BTS baseline source must all be present".to_owned());
    }

    let (pilot_episodes, pilot_classes, broad_episodes) = validate_corpus_design(&root)?;
    validate_episode_record_contract(&root)?;
    let gates = nonempty_array(
        super::field(&root, "capture_gates", "manifest")?,
        "manifest.capture_gates",
    )?;
    if gates.len() < 7 {
        return Err(
            "manifest.capture_gates: expected at least seven explicit evidence gates".to_owned(),
        );
    }

    Ok(HistoricalCorpusReport {
        status: "SOURCE_REGISTRY_VALID_NOT_CAPTURED",
        schema_id: "luminous.symtropy.maritime-historical-validation-corpus",
        schema_version: "0.1.0",
        manifest_sha256: super::sha256_bytes(bytes),
        manifest_byte_length: bytes.len(),
        source_count: summaries.len(),
        immutable_revision_pinned_count: pinned_count,
        captured_artifact_count: 0,
        sources: summaries,
        pilot_episode_target: pilot_episodes,
        pilot_disruption_class_target: pilot_classes,
        broader_claim_episode_target: broad_episodes,
        data_capture_performed: false,
        model_validation_performed: false,
        scored_corpus_admitted: false,
        claim_ceiling: "Source-discovery and corpus-design contract only. No source capture, licence clearance, observation import, model validation, or operational claim.",
    })
}

pub(super) fn run_cli(manifest_path: &str) -> Result<(), String> {
    let metadata = fs::metadata(manifest_path).map_err(|error| {
        format!("cannot stat historical corpus manifest {manifest_path}: {error}")
    })?;
    if metadata.len() > MAX_MANIFEST_BYTES as u64 {
        return Err(format!(
            "historical corpus manifest exceeds {MAX_MANIFEST_BYTES}-byte size bound"
        ));
    }
    let bytes = fs::read(manifest_path).map_err(|error| {
        format!("cannot read historical corpus manifest {manifest_path}: {error}")
    })?;
    let report = evaluate_bytes(&bytes)?;
    let output = serde_json::to_string(&report)
        .map_err(|error| format!("cannot serialize historical corpus report: {error}"))?;
    println!("{output}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const FIXTURE: &[u8] =
        include_bytes!("../../../../docs/tech/MARITIME_HISTORICAL_VALIDATION_CORPUS_V0_1.json");

    #[test]
    fn prospective_source_registry_is_valid_but_not_admitted() {
        let report = evaluate_bytes(FIXTURE).expect("registry contract should validate");
        assert_eq!(report.status, "SOURCE_REGISTRY_VALID_NOT_CAPTURED");
        assert_eq!(report.source_count, 3);
        assert_eq!(report.immutable_revision_pinned_count, 2);
        assert_eq!(
            report.sources[1].source_git_blob_sha1.as_deref(),
            Some("21beb5b7673918143a0ab108b03acc3cccaeb330")
        );
        assert_eq!(
            report.sources[1].declared_resource_byte_length,
            Some(643_047)
        );
        assert_eq!(report.captured_artifact_count, 0);
        assert!(!report.data_capture_performed);
        assert!(!report.model_validation_performed);
        assert!(!report.scored_corpus_admitted);
    }

    #[test]
    fn manifest_digest_binds_exact_bytes_and_report_is_deterministic() {
        let first = evaluate_bytes(FIXTURE).unwrap();
        let second = evaluate_bytes(FIXTURE).unwrap();
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );
        assert_eq!(first.manifest_sha256, super::super::sha256_bytes(FIXTURE));
        let changed = [FIXTURE, &b"\n"[..]].concat();
        assert_ne!(
            evaluate_bytes(&changed).unwrap().manifest_sha256,
            first.manifest_sha256
        );
    }

    #[test]
    fn duplicate_json_keys_fail_closed() {
        assert!(super::super::parse_strict(br#"{"schema_id":"a","schema_id":"b"}"#).is_err());
    }

    #[test]
    fn unknown_root_fields_fail_closed() {
        let raw = String::from_utf8(FIXTURE.to_vec()).unwrap();
        let changed = raw.replacen(
            "\"registry_status\": \"prospective_sources_not_captured\",",
            "\"registry_status\": \"prospective_sources_not_captured\",\n  \"surprise\": true,",
            1,
        );
        assert_ne!(changed, raw);
        assert!(evaluate_bytes(changed.as_bytes()).is_err());
    }

    #[test]
    fn capture_claim_without_bytes_is_rejected() {
        let raw = String::from_utf8(FIXTURE.to_vec()).unwrap();
        let changed = raw.replacen(
            "\"data_capture_performed\": false",
            "\"data_capture_performed\": true",
            1,
        );
        assert_ne!(changed, raw);
        assert!(
            evaluate_bytes(changed.as_bytes())
                .unwrap_err()
                .contains("no capture is claimed")
        );
    }

    #[test]
    fn populating_digest_without_capture_is_rejected() {
        let raw = String::from_utf8(FIXTURE.to_vec()).unwrap();
        let changed = raw.replacen(
            "\"artifact_sha256\": null",
            "\"artifact_sha256\": \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"",
            1,
        );
        assert_ne!(changed, raw);
        assert!(
            evaluate_bytes(changed.as_bytes())
                .unwrap_err()
                .contains("cannot be populated before capture")
        );
    }

    #[test]
    fn pinned_research_metadata_must_match_source_revision() {
        let mut root: Value = serde_json::from_slice(FIXTURE).unwrap();
        root["sources"][1]["metadata_uri"] = Value::String(
            "https://github.com/ChenchenMobility/Maritime-Data-CyPort/blob/main/README.md"
                .to_owned(),
        );
        let changed = serde_json::to_vec(&root).unwrap();
        assert!(
            evaluate_bytes(&changed)
                .unwrap_err()
                .contains("must pin the same immutable revision")
        );
    }

    #[test]
    fn mutable_research_revision_is_rejected() {
        let raw = String::from_utf8(FIXTURE.to_vec()).unwrap();
        let changed = raw.replacen(
            "\"immutable_revision\": \"29cfb82b68bce25bc93df44a240913a6c3420ef1\"",
            "\"immutable_revision\": \"master\"",
            1,
        );
        assert_ne!(changed, raw);
        assert!(
            evaluate_bytes(changed.as_bytes())
                .unwrap_err()
                .contains("pinned 40-character lowercase Git commit")
        );
    }

    #[test]
    fn unresolved_research_rights_cannot_be_marked_cleared() {
        let raw = String::from_utf8(FIXTURE.to_vec()).unwrap();
        let changed = raw.replacen(
            "\"license_status\": \"unverified\"",
            "\"license_status\": \"approved\"",
            1,
        );
        assert_ne!(changed, raw);
        assert!(evaluate_bytes(changed.as_bytes()).is_err());
    }

    #[test]
    fn cyport_component_license_and_revision_are_explicit() {
        let mut root: Value = serde_json::from_slice(FIXTURE).unwrap();
        assert_eq!(
            root["sources"][1]["license_status"],
            "readme_declares_cc_by_4_0"
        );
        root["sources"][1]["license_status"] =
            Value::String("all_components_unrestricted".to_owned());
        let changed = serde_json::to_vec(&root).unwrap();
        assert!(
            evaluate_bytes(&changed)
                .unwrap_err()
                .contains("declared CC BY 4.0 status must remain explicit")
        );
    }

    #[test]
    fn cyport_resource_identity_is_pinned_to_blob_and_byte_length() {
        let mut root: Value = serde_json::from_slice(FIXTURE).unwrap();
        root["sources"][1]["source_git_blob_sha1"] =
            Value::String("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned());
        let changed = serde_json::to_vec(&root).unwrap();
        assert!(
            evaluate_bytes(&changed)
                .unwrap_err()
                .contains("Git blob ID and byte length do not match")
        );
    }

    #[test]
    fn cyport_resource_identifier_must_bind_blob_and_size() {
        let mut root: Value = serde_json::from_slice(FIXTURE).unwrap();
        root["sources"][1]["resource_identifier"] = Value::String(
            "Maritime-Data-CyPort@5cb1d8778d3968c5e50d0b8cb8213f2414d692d3:Tropical_cyclone_interaction/interaction_data.csv".to_owned()
        );
        let changed = serde_json::to_vec(&root).unwrap();
        assert!(
            evaluate_bytes(&changed)
                .unwrap_err()
                .contains("missing pinned source identity component")
        );
    }

    #[test]
    fn cyport_declared_file_size_cannot_drift() {
        let mut root: Value = serde_json::from_slice(FIXTURE).unwrap();
        root["sources"][1]["declared_resource_byte_length"] =
            Value::Number(serde_json::Number::from(643_048));
        let changed = serde_json::to_vec(&root).unwrap();
        assert!(
            evaluate_bytes(&changed)
                .unwrap_err()
                .contains("Git blob ID and byte length do not match")
        );
    }

    #[test]
    fn mutable_bts_view_cannot_be_mistaken_for_immutable_revision() {
        let mut root: Value = serde_json::from_slice(FIXTURE).unwrap();
        root["sources"][2]["immutable_revision"] = Value::String("latest".to_owned());
        let changed = serde_json::to_vec(&root).unwrap();
        assert!(
            evaluate_bytes(&changed)
                .unwrap_err()
                .contains("mutable API view must not be treated as a content revision")
        );
    }

    #[test]
    fn temporal_holdout_is_mandatory_for_broader_claims() {
        let mut root: Value = serde_json::from_slice(FIXTURE).unwrap();
        root["corpus_design"]["broader_claim_targets"]["temporally_held_out_evaluation_required"] =
            Value::Bool(false);
        let changed = serde_json::to_vec(&root).unwrap();
        assert!(evaluate_bytes(&changed).is_err());
    }

    #[test]
    fn episode_split_cannot_be_changed_to_row_randomization() {
        let mut root: Value = serde_json::from_slice(FIXTURE).unwrap();
        root["corpus_design"]["split_unit"] = Value::String("random_rows".to_owned());
        let changed = serde_json::to_vec(&root).unwrap();
        assert!(
            evaluate_bytes(&changed)
                .unwrap_err()
                .contains("split by independent episode")
        );
    }

    #[test]
    fn censoring_semantics_must_not_collapse_to_zero() {
        let raw = String::from_utf8(FIXTURE.to_vec()).unwrap();
        let changed = raw.replacen(
            "record right_censored or unknown; never encode it as a recovery time of zero",
            "encode missing recovery as zero",
            1,
        );
        assert_ne!(changed, raw);
        assert!(
            evaluate_bytes(changed.as_bytes())
                .unwrap_err()
                .contains("right-censoring and zero-time confusion")
        );
    }

    #[test]
    fn duplicate_source_ids_are_rejected() {
        let mut root: Value = serde_json::from_slice(FIXTURE).unwrap();
        let first_source = root["sources"][0].clone();
        root["sources"][1] = first_source;
        let changed = serde_json::to_vec(&root).unwrap();
        assert!(
            evaluate_bytes(&changed)
                .unwrap_err()
                .contains("duplicate source ID")
        );
    }

    #[test]
    fn malformed_modification_timestamp_is_rejected() {
        let mut root: Value = serde_json::from_slice(FIXTURE).unwrap();
        root["sources"][1]["last_modified_utc"] = Value::String("2026-02-31T00:00:00Z".to_owned());
        let changed = serde_json::to_vec(&root).unwrap();
        assert!(
            evaluate_bytes(&changed)
                .unwrap_err()
                .contains("valid RFC3339 UTC timestamp")
        );
    }

    #[test]
    fn invalid_calendar_date_is_rejected() {
        let mut root: Value = serde_json::from_slice(FIXTURE).unwrap();
        root["sources"][2]["last_modified_date"] = Value::String("2026-02-31".to_owned());
        let changed = serde_json::to_vec(&root).unwrap();
        assert!(
            evaluate_bytes(&changed)
                .unwrap_err()
                .contains("invalid calendar date")
        );
    }
}
