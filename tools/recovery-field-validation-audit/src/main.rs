use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const EMBEDDED_PACKET: &str =
    include_str!("../../../docs/tech/recovery-field-validation-packet-v0.1.json");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Packet {
    schema_version: String,
    protocol_id: String,
    status: String,
    evaluation_day: u64,
    decision_profile: DecisionProfile,
    protocol_contract: ProtocolContract,
    claim_ceiling: ClaimCeiling,
    evidence_records: Vec<EvidenceRecord>,
    records: Vec<FieldRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct DecisionProfile {
    profile_id: String,
    domain: String,
    taxonomy_version: String,
    criteria_regime: String,
    covered_failure_families: Vec<String>,
    required_decision_evidence_tags: Vec<String>,
    max_evidence_age_days: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ProtocolContract {
    decision_inputs_exclude_post_decision_outcomes: bool,
    zero_is_distinct_from_unmeasured: bool,
    no_action_and_censored_cases_retained: bool,
    source_authenticity_must_be_unverified_for_synthetic_records: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ClaimCeiling {
    all_records_synthetic: bool,
    real_field_observations: bool,
    legal_authority_verified: bool,
    professional_qualifications_verified: bool,
    expected_actuation: bool,
    execution_authority: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct EvidenceRecord {
    evidence_id: String,
    evidence_class: String,
    observed_day: u64,
    available_day: u64,
    source_ref: String,
    source_authenticity_status: String,
    evidence_tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct FieldRecord {
    case_id: String,
    asset_pseudonym: String,
    domain: String,
    profile_version: String,
    taxonomy_version: String,
    criteria_regime: String,
    observed_failure_families: Vec<String>,
    family_classification: String,
    classification_review_status: String,
    decision_day: u64,
    decision_manifest_sha256: String,
    decision_manifest_payload_json: String,
    episode_status: String,
    decision_input_evidence_refs: Vec<String>,
    post_decision_outcome_evidence_refs: Vec<String>,
    data_rights: DataRights,
    authority_gates: Vec<AuthorityGate>,
    required_metric_ids: Vec<String>,
    measurements: Vec<Measurement>,
    ground_truth: GroundTruth,
    expected_audit_disposition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct DecisionManifest {
    manifest_version: String,
    case_id: String,
    profile_version: String,
    taxonomy_version: String,
    criteria_regime: String,
    decision_day: u64,
    decision_input_evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct DataRights {
    use_status: String,
    license_ref: Option<String>,
    allowed_purposes: Vec<String>,
    retention_until_day: u64,
    public_location_disclosed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct AuthorityGate {
    gate_id: String,
    status: String,
    evidence_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Measurement {
    metric_id: String,
    measurement_state: String,
    value_scaled: Option<i64>,
    scale: u64,
    unit: String,
    observed_day: Option<u64>,
    available_day: Option<u64>,
    evidence_ref: Option<String>,
    uncertainty_lower_scaled: Option<i64>,
    uncertainty_upper_scaled: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct GroundTruth {
    adjudication_status: String,
    censoring_status: String,
    outcome_day: Option<u64>,
    outcome_evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct CaseResult {
    case_id: String,
    profile_scope_status: String,
    data_rights_status: String,
    authority_status: String,
    evidence_status: String,
    disposition: String,
    episode_status: String,
    manifest_integrity_status: String,
    manifest_semantic_status: String,
    synthetic_review_candidate: bool,
    actuation_authorized: bool,
    expected_disposition_matches: bool,
}

#[derive(Debug, Serialize)]
struct AuditReceipt {
    result: String,
    schema_version: String,
    protocol_id: String,
    canonical_packet_sha256: String,
    source_commit: Option<String>,
    case_count: usize,
    evidence_record_count: usize,
    decision_input_reference_count: usize,
    post_decision_outcome_reference_count: usize,
    manifest_match_count: usize,
    profile_scope_counts: BTreeMap<String, usize>,
    data_rights_counts: BTreeMap<String, usize>,
    authority_counts: BTreeMap<String, usize>,
    evidence_counts: BTreeMap<String, usize>,
    disposition_counts: BTreeMap<String, usize>,
    episode_status_counts: BTreeMap<String, usize>,
    measurement_state_counts: BTreeMap<String, usize>,
    decision_time_future_evidence_violation_count: usize,
    outcome_before_decision_violation_count: usize,
    missing_evidence_reference_count: usize,
    duplicate_identity_count: usize,
    expected_dispositions_match: bool,
    all_records_synthetic: bool,
    all_actuation_disabled: bool,
    synthetic_review_candidate_count: usize,
    case_results: Vec<CaseResult>,
    errors: Vec<String>,
}

const REQUIRED_GATE_IDS: [&str; 7] = [
    "legal_authority",
    "site_access",
    "owner_consent",
    "ecological_review",
    "safety_review",
    "qualified_operator",
    "transport_disposal",
];

const ALLOWED_METRIC_STATES: [&str; 7] = [
    "MEASURED",
    "ESTIMATED",
    "NOT_MEASURED",
    "INACCESSIBLE",
    "CENSORED",
    "CONTRADICTORY",
    "NOT_APPLICABLE",
];

const EXPECTED_DISPOSITIONS: [&str; 7] = [
    "REVIEW_ONLY_CANDIDATE",
    "REASSESS_SCOPE",
    "BLOCKED_DATA_RIGHTS",
    "HOLD_DATA_RIGHTS",
    "BLOCKED_AUTHORITY",
    "HOLD_AUTHORITY",
    "HOLD_EVIDENCE",
];

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

fn is_nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_commit_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn sorted(values: &[String]) -> Vec<String> {
    let mut output = values.to_vec();
    output.sort();
    output
}

fn inc(map: &mut BTreeMap<String, usize>, key: &str) {
    *map.entry(key.to_string()).or_insert(0) += 1;
}

fn canonical_packet_sha256(packet: &Packet) -> String {
    let mut canonical = packet.clone();
    canonical
        .evidence_records
        .sort_by(|a, b| a.evidence_id.cmp(&b.evidence_id));
    for evidence in &mut canonical.evidence_records {
        evidence.evidence_tags.sort();
    }
    canonical.records.sort_by(|a, b| a.case_id.cmp(&b.case_id));
    for record in &mut canonical.records {
        record.observed_failure_families.sort();
        record.decision_input_evidence_refs.sort();
        record.post_decision_outcome_evidence_refs.sort();
        record.required_metric_ids.sort();
        record.data_rights.allowed_purposes.sort();
        record
            .authority_gates
            .sort_by(|a, b| a.gate_id.cmp(&b.gate_id));
        record
            .measurements
            .sort_by(|a, b| a.metric_id.cmp(&b.metric_id));
        record.ground_truth.outcome_evidence_refs.sort();
    }
    serde_json::to_vec(&canonical)
        .map(|bytes| sha256_hex(&bytes))
        .unwrap_or_default()
}

fn validate_packet(packet: &Packet, source_commit: Option<&str>, errors: &mut Vec<String>) {
    if packet.schema_version != "recovery-field-validation-packet-v0.1" {
        errors.push("SCHEMA_VERSION_UNSUPPORTED".to_string());
    }
    if packet.protocol_id != "rights-aware-recovery-field-validation-v0.1" {
        errors.push("PROTOCOL_ID_UNSUPPORTED".to_string());
    }
    if packet.status != "synthetic-protocol-fixture" {
        errors.push("PACKET_STATUS_UNEXPECTED".to_string());
    }
    if !packet.claim_ceiling.all_records_synthetic
        || packet.claim_ceiling.real_field_observations
        || packet.claim_ceiling.legal_authority_verified
        || packet.claim_ceiling.professional_qualifications_verified
    {
        errors.push("SYNTHETIC_CLAIM_CEILING_VIOLATED".to_string());
    }
    if packet.claim_ceiling.expected_actuation
        || packet.claim_ceiling.execution_authority != "NO_ACTUATION_AUTHORIZED"
    {
        errors.push("ACTUATION_CLAIM_CEILING_VIOLATED".to_string());
    }
    if !packet
        .protocol_contract
        .decision_inputs_exclude_post_decision_outcomes
        || !packet.protocol_contract.zero_is_distinct_from_unmeasured
        || !packet
            .protocol_contract
            .no_action_and_censored_cases_retained
        || !packet
            .protocol_contract
            .source_authenticity_must_be_unverified_for_synthetic_records
    {
        errors.push("PROTOCOL_CONTRACT_DISABLED".to_string());
    }
    if let Some(commit) = source_commit {
        if !is_commit_sha(commit) {
            errors.push("SOURCE_COMMIT_INVALID".to_string());
        }
    }

    for value in [
        packet.decision_profile.profile_id.as_str(),
        packet.decision_profile.domain.as_str(),
        packet.decision_profile.taxonomy_version.as_str(),
        packet.decision_profile.criteria_regime.as_str(),
    ] {
        if !is_nonempty(value) {
            errors.push("DECISION_PROFILE_IDENTITY_EMPTY".to_string());
        }
    }
    if packet.decision_profile.covered_failure_families.is_empty() {
        errors.push("COVERED_FAILURE_FAMILIES_EMPTY".to_string());
    }
    if packet
        .decision_profile
        .required_decision_evidence_tags
        .is_empty()
    {
        errors.push("REQUIRED_DECISION_EVIDENCE_TAGS_EMPTY".to_string());
    }
    if packet.evidence_records.is_empty() || packet.records.is_empty() {
        errors.push("PACKET_COLLECTION_EMPTY".to_string());
    }

    let mut evidence_ids = BTreeSet::new();
    for evidence in &packet.evidence_records {
        if !is_nonempty(&evidence.evidence_id) || !is_nonempty(&evidence.source_ref) {
            errors.push("EVIDENCE_IDENTITY_EMPTY".to_string());
        }
        if !evidence_ids.insert(&evidence.evidence_id) {
            errors.push(format!("DUPLICATE_EVIDENCE_ID:{}", evidence.evidence_id));
        }
        if !matches!(
            evidence.evidence_class.as_str(),
            "DECISION_TIME" | "POST_DECISION_OUTCOME"
        ) {
            errors.push(format!("EVIDENCE_CLASS_INVALID:{}", evidence.evidence_id));
        }
        if !evidence.evidence_id.starts_with("synthetic://")
            || !evidence.source_ref.starts_with("synthetic://")
            || evidence.source_authenticity_status != "UNVERIFIED_SYNTHETIC"
        {
            errors.push(format!(
                "EVIDENCE_AUTHENTICITY_CEILING_VIOLATED:{}",
                evidence.evidence_id
            ));
        }
        if evidence.available_day < evidence.observed_day {
            errors.push(format!(
                "EVIDENCE_AVAILABLE_BEFORE_OBSERVED:{}",
                evidence.evidence_id
            ));
        }
        if evidence.evidence_tags.is_empty()
            || evidence.evidence_tags.iter().any(|tag| !is_nonempty(tag))
        {
            errors.push(format!("EVIDENCE_TAGS_INVALID:{}", evidence.evidence_id));
        }
    }

    let mut case_ids = BTreeSet::new();
    for record in &packet.records {
        if !is_nonempty(&record.case_id) || !is_nonempty(&record.asset_pseudonym) {
            errors.push("CASE_IDENTITY_EMPTY".to_string());
        }
        if !case_ids.insert(&record.case_id) {
            errors.push(format!("DUPLICATE_CASE_ID:{}", record.case_id));
        }
        if !is_sha256(&record.decision_manifest_sha256) {
            errors.push(format!(
                "DECISION_MANIFEST_SHA256_INVALID:{}",
                record.case_id
            ));
        }
        if !is_nonempty(&record.decision_manifest_payload_json) {
            errors.push(format!(
                "DECISION_MANIFEST_PAYLOAD_EMPTY:{}",
                record.case_id
            ));
        }
        if !matches!(
            record.family_classification.as_str(),
            "COMPLETE" | "INCOMPLETE" | "UNKNOWN"
        ) {
            errors.push(format!("FAMILY_CLASSIFICATION_INVALID:{}", record.case_id));
        }
        if !matches!(
            record.classification_review_status.as_str(),
            "REVIEWED" | "UNREVIEWED" | "UNKNOWN"
        ) {
            errors.push(format!(
                "CLASSIFICATION_REVIEW_STATUS_INVALID:{}",
                record.case_id
            ));
        }
        if !matches!(
            record.episode_status.as_str(),
            "NO_ACTION" | "REFERRED" | "FOLLOW_UP_CENSORED"
        ) {
            errors.push(format!("EPISODE_STATUS_INVALID:{}", record.case_id));
        }
        if !EXPECTED_DISPOSITIONS.contains(&record.expected_audit_disposition.as_str()) {
            errors.push(format!("EXPECTED_DISPOSITION_INVALID:{}", record.case_id));
        }
        if record.decision_day > packet.evaluation_day {
            errors.push(format!(
                "DECISION_DAY_AFTER_EVALUATION_DAY:{}",
                record.case_id
            ));
        }
        if record.observed_failure_families.is_empty()
            || record.decision_input_evidence_refs.is_empty()
            || record.required_metric_ids.is_empty()
        {
            errors.push(format!("REQUIRED_CASE_LIST_EMPTY:{}", record.case_id));
        }
        for metric_id in &record.required_metric_ids {
            if !is_nonempty(metric_id) {
                errors.push(format!("REQUIRED_METRIC_ID_EMPTY:{}", record.case_id));
            }
        }
    }
}

fn evidence_by_id<'a>(packet: &'a Packet) -> BTreeMap<&'a str, &'a EvidenceRecord> {
    packet
        .evidence_records
        .iter()
        .map(|evidence| (evidence.evidence_id.as_str(), evidence))
        .collect()
}

fn validate_manifest(record: &FieldRecord, errors: &mut Vec<String>) -> (String, bool) {
    let start = errors.len();
    let manifest: DecisionManifest =
        match serde_json::from_str(&record.decision_manifest_payload_json) {
            Ok(value) => value,
            Err(error) => {
                errors.push(format!(
                    "DECISION_MANIFEST_JSON_INVALID:{}:{error}",
                    record.case_id
                ));
                return ("INVALID".to_string(), false);
            }
        };
    let canonical = match serde_json::to_string(&manifest) {
        Ok(value) => value,
        Err(error) => {
            errors.push(format!(
                "DECISION_MANIFEST_SERIALIZE_FAILED:{}:{error}",
                record.case_id
            ));
            return ("INVALID".to_string(), false);
        }
    };
    if canonical != record.decision_manifest_payload_json {
        errors.push(format!(
            "DECISION_MANIFEST_NOT_CANONICAL:{}",
            record.case_id
        ));
    }
    let digest = sha256_hex(record.decision_manifest_payload_json.as_bytes());
    if digest != record.decision_manifest_sha256.to_ascii_lowercase() {
        errors.push(format!(
            "DECISION_MANIFEST_DIGEST_MISMATCH:{}",
            record.case_id
        ));
    }
    let refs_are_sorted_unique = sorted(&record.decision_input_evidence_refs)
        == record.decision_input_evidence_refs
        && record
            .decision_input_evidence_refs
            .iter()
            .collect::<BTreeSet<_>>()
            .len()
            == record.decision_input_evidence_refs.len();
    let semantic_match = manifest.manifest_version == "recovery-decision-manifest-v0.1"
        && manifest.case_id == record.case_id
        && manifest.profile_version == record.profile_version
        && manifest.taxonomy_version == record.taxonomy_version
        && manifest.criteria_regime == record.criteria_regime
        && manifest.decision_day == record.decision_day
        && manifest.decision_input_evidence_refs == record.decision_input_evidence_refs
        && refs_are_sorted_unique;
    if !semantic_match {
        errors.push(format!(
            "DECISION_MANIFEST_SEMANTIC_MISMATCH:{}",
            record.case_id
        ));
    }
    let status = if errors.len() == start && semantic_match {
        "MATCH"
    } else {
        "MISMATCH"
    };
    (status.to_string(), errors.len() == start && semantic_match)
}

fn validate_measurement(
    packet: &Packet,
    record: &FieldRecord,
    measurement: &Measurement,
    evidence: &BTreeMap<&str, &EvidenceRecord>,
    errors: &mut Vec<String>,
) -> bool {
    let start = errors.len();
    if !is_nonempty(&measurement.metric_id) || !is_nonempty(&measurement.unit) {
        errors.push(format!(
            "MEASUREMENT_ID_OR_UNIT_EMPTY:{}:{}",
            record.case_id, measurement.metric_id
        ));
    }
    if measurement.scale == 0 {
        errors.push(format!(
            "MEASUREMENT_SCALE_ZERO:{}:{}",
            record.case_id, measurement.metric_id
        ));
    }
    if !ALLOWED_METRIC_STATES.contains(&measurement.measurement_state.as_str()) {
        errors.push(format!(
            "MEASUREMENT_STATE_INVALID:{}:{}",
            record.case_id, measurement.metric_id
        ));
        return false;
    }

    if matches!(
        measurement.measurement_state.as_str(),
        "MEASURED" | "ESTIMATED"
    ) {
        if measurement.value_scaled.is_none()
            || measurement.observed_day.is_none()
            || measurement.available_day.is_none()
            || measurement.evidence_ref.is_none()
        {
            errors.push(format!(
                "QUANTIFIED_MEASUREMENT_MISSING_VALUE_OR_PROVENANCE:{}:{}",
                record.case_id, measurement.metric_id
            ));
        }
        if let (Some(lower), Some(value), Some(upper)) = (
            measurement.uncertainty_lower_scaled,
            measurement.value_scaled,
            measurement.uncertainty_upper_scaled,
        ) {
            if lower > value || value > upper {
                errors.push(format!(
                    "MEASUREMENT_UNCERTAINTY_DOES_NOT_ENCLOSE_VALUE:{}:{}",
                    record.case_id, measurement.metric_id
                ));
            }
        } else if measurement.uncertainty_lower_scaled.is_some()
            || measurement.uncertainty_upper_scaled.is_some()
        {
            errors.push(format!(
                "MEASUREMENT_UNCERTAINTY_PARTIAL:{}:{}",
                record.case_id, measurement.metric_id
            ));
        }
        if let (Some(observed), Some(available)) =
            (measurement.observed_day, measurement.available_day)
        {
            if observed > available
                || available > record.decision_day
                || available > packet.evaluation_day
            {
                errors.push(format!(
                    "MEASUREMENT_TIME_INVALID:{}:{}",
                    record.case_id, measurement.metric_id
                ));
            }
        }
        if let Some(reference) = measurement.evidence_ref.as_deref() {
            match evidence.get(reference) {
                None => errors.push(format!(
                    "MEASUREMENT_EVIDENCE_MISSING:{}:{}",
                    record.case_id, measurement.metric_id
                )),
                Some(item)
                    if item.evidence_class != "DECISION_TIME"
                        || item.available_day > record.decision_day
                        || item.observed_day > record.decision_day
                        || !record
                            .decision_input_evidence_refs
                            .iter()
                            .any(|v| v == reference) =>
                {
                    errors.push(format!(
                        "MEASUREMENT_EVIDENCE_NOT_DECISION_INPUT:{}:{}",
                        record.case_id, measurement.metric_id
                    ));
                }
                Some(_) => {}
            }
        }
    } else {
        // Missing, inaccessible, censored, contradictory and not-applicable values are never numeric zero.
        if measurement.value_scaled.is_some()
            || measurement.uncertainty_lower_scaled.is_some()
            || measurement.uncertainty_upper_scaled.is_some()
        {
            errors.push(format!(
                "UNQUANTIFIED_STATE_HAS_NUMERIC_VALUE:{}:{}",
                record.case_id, measurement.metric_id
            ));
        }
        if measurement.evidence_ref.is_some()
            || measurement.observed_day.is_some()
            || measurement.available_day.is_some()
        {
            errors.push(format!(
                "UNQUANTIFIED_STATE_HAS_MEASUREMENT_PROVENANCE:{}:{}",
                record.case_id, measurement.metric_id
            ));
        }
    }
    errors.len() == start
}

fn validate_record(
    packet: &Packet,
    record: &FieldRecord,
    evidence: &BTreeMap<&str, &EvidenceRecord>,
    global_errors: &mut Vec<String>,
    duplicate_identity_count: &mut usize,
    decision_ref_count: &mut usize,
    outcome_ref_count: &mut usize,
    future_leakage_count: &mut usize,
    outcome_before_decision_count: &mut usize,
    missing_ref_count: &mut usize,
    measurement_counts: &mut BTreeMap<String, usize>,
) -> CaseResult {
    let error_start = global_errors.len();
    let (manifest_status, manifest_matches) = validate_manifest(record, global_errors);

    let scope_ok = record.domain == packet.decision_profile.domain
        && record.profile_version == packet.decision_profile.profile_id
        && record.taxonomy_version == packet.decision_profile.taxonomy_version
        && record.criteria_regime == packet.decision_profile.criteria_regime
        && record.family_classification == "COMPLETE"
        && record.classification_review_status == "REVIEWED"
        && !record.observed_failure_families.is_empty()
        && record.observed_failure_families.iter().all(|family| {
            packet
                .decision_profile
                .covered_failure_families
                .contains(family)
        });
    let profile_scope_status = if scope_ok { "IN_SCOPE" } else { "OUT_OF_SCOPE" };

    let mut refs_valid = true;
    let mut seen_decision_refs = BTreeSet::new();
    let mut observed_tags = BTreeSet::new();
    for reference in &record.decision_input_evidence_refs {
        *decision_ref_count += 1;
        if !seen_decision_refs.insert(reference) {
            *duplicate_identity_count += 1;
            global_errors.push(format!(
                "DECISION_EVIDENCE_REFERENCE_DUPLICATE:{}:{}",
                record.case_id, reference
            ));
            refs_valid = false;
        }
        if record
            .post_decision_outcome_evidence_refs
            .contains(reference)
        {
            global_errors.push(format!(
                "OUTCOME_REFERENCE_USED_AS_DECISION_INPUT:{}:{}",
                record.case_id, reference
            ));
            refs_valid = false;
        }
        match evidence.get(reference.as_str()) {
            None => {
                *missing_ref_count += 1;
                global_errors.push(format!(
                    "DECISION_EVIDENCE_REFERENCE_MISSING:{}:{}",
                    record.case_id, reference
                ));
                refs_valid = false;
            }
            Some(item) => {
                if item.evidence_class != "DECISION_TIME" {
                    global_errors.push(format!(
                        "NON_DECISION_EVIDENCE_USED_AS_INPUT:{}:{}",
                        record.case_id, reference
                    ));
                    refs_valid = false;
                }
                if item.observed_day > record.decision_day
                    || item.available_day > record.decision_day
                {
                    *future_leakage_count += 1;
                    global_errors.push(format!(
                        "FUTURE_EVIDENCE_IN_DECISION_INPUT:{}:{}",
                        record.case_id, reference
                    ));
                    refs_valid = false;
                }
                if record.decision_day.saturating_sub(item.available_day)
                    > packet.decision_profile.max_evidence_age_days
                {
                    global_errors.push(format!(
                        "STALE_DECISION_EVIDENCE:{}:{}",
                        record.case_id, reference
                    ));
                    refs_valid = false;
                }
                for tag in &item.evidence_tags {
                    observed_tags.insert(tag.as_str());
                }
            }
        }
    }
    for required_tag in &packet.decision_profile.required_decision_evidence_tags {
        if !observed_tags.contains(required_tag.as_str()) {
            global_errors.push(format!(
                "REQUIRED_DECISION_EVIDENCE_TAG_MISSING:{}:{}",
                record.case_id, required_tag
            ));
            refs_valid = false;
        }
    }

    let expected_gate_ids: BTreeSet<&str> = REQUIRED_GATE_IDS.into_iter().collect();
    let mut gate_ids = BTreeSet::new();
    let mut blocked_gate = false;
    let mut unknown_gate = false;
    for gate in &record.authority_gates {
        if !gate_ids.insert(gate.gate_id.as_str()) {
            *duplicate_identity_count += 1;
            global_errors.push(format!(
                "AUTHORITY_GATE_DUPLICATE:{}:{}",
                record.case_id, gate.gate_id
            ));
        }
        if !expected_gate_ids.contains(gate.gate_id.as_str()) {
            global_errors.push(format!(
                "AUTHORITY_GATE_UNSUPPORTED:{}:{}",
                record.case_id, gate.gate_id
            ));
        }
        if !matches!(gate.status.as_str(), "PASS" | "BLOCK" | "UNKNOWN") {
            global_errors.push(format!(
                "AUTHORITY_GATE_STATUS_INVALID:{}:{}",
                record.case_id, gate.gate_id
            ));
        }
        match gate.status.as_str() {
            "BLOCK" => blocked_gate = true,
            "UNKNOWN" => unknown_gate = true,
            _ => {}
        }
        if !record
            .decision_input_evidence_refs
            .contains(&gate.evidence_ref)
        {
            global_errors.push(format!(
                "AUTHORITY_GATE_EVIDENCE_NOT_INPUT:{}:{}",
                record.case_id, gate.gate_id
            ));
        }
        match evidence.get(gate.evidence_ref.as_str()) {
            None => {
                *missing_ref_count += 1;
                global_errors.push(format!(
                    "AUTHORITY_GATE_EVIDENCE_MISSING:{}:{}",
                    record.case_id, gate.gate_id
                ));
            }
            Some(item)
                if item.evidence_class != "DECISION_TIME"
                    || !item.evidence_tags.contains(&gate.gate_id) =>
            {
                global_errors.push(format!(
                    "AUTHORITY_GATE_EVIDENCE_TAG_MISMATCH:{}:{}",
                    record.case_id, gate.gate_id
                ));
            }
            Some(_) => {}
        }
    }
    if gate_ids != expected_gate_ids {
        global_errors.push(format!("AUTHORITY_GATE_SET_INCOMPLETE:{}", record.case_id));
    }
    let authority_status = if blocked_gate {
        "BLOCK"
    } else if unknown_gate {
        "UNKNOWN"
    } else {
        "PASS"
    };

    let mut data_rights_status = "ALLOWED";
    let rights_valid = match record.data_rights.use_status.as_str() {
        "BLOCKED" => {
            data_rights_status = "BLOCKED";
            false
        }
        "UNKNOWN" => {
            data_rights_status = "UNKNOWN";
            false
        }
        "ALLOWED" => {
            let license_valid = record
                .data_rights
                .license_ref
                .as_deref()
                .map(|value| value.starts_with("synthetic://license/"))
                .unwrap_or(false);
            let purposes_valid = record
                .data_rights
                .allowed_purposes
                .contains(&"research_validation".to_string())
                && record
                    .data_rights
                    .allowed_purposes
                    .contains(&"benchmarking".to_string());
            if record.data_rights.retention_until_day < packet.evaluation_day {
                data_rights_status = "EXPIRED";
                false
            } else if !license_valid || !purposes_valid {
                data_rights_status = "INSUFFICIENT";
                false
            } else {
                true
            }
        }
        _ => {
            data_rights_status = "INVALID";
            false
        }
    };
    if record.data_rights.public_location_disclosed {
        global_errors.push(format!(
            "PUBLIC_LOCATION_DISCLOSURE_FORBIDDEN:{}",
            record.case_id
        ));
    }

    let mut metric_ids = BTreeSet::new();
    let mut measurements_valid = true;
    let mut measured_metric_ids = BTreeSet::new();
    for measurement in &record.measurements {
        inc(measurement_counts, &measurement.measurement_state);
        if !metric_ids.insert(measurement.metric_id.as_str()) {
            *duplicate_identity_count += 1;
            global_errors.push(format!(
                "MEASUREMENT_METRIC_ID_DUPLICATE:{}:{}",
                record.case_id, measurement.metric_id
            ));
            measurements_valid = false;
        }
        let valid = validate_measurement(packet, record, measurement, evidence, global_errors);
        if measurement.measurement_state == "MEASURED"
            && measurement.value_scaled.is_some()
            && valid
        {
            measured_metric_ids.insert(measurement.metric_id.as_str());
        }
        if !valid {
            measurements_valid = false;
        }
    }
    let missing_required_metrics: Vec<&str> = record
        .required_metric_ids
        .iter()
        .filter(|metric_id| !measured_metric_ids.contains(metric_id.as_str()))
        .map(String::as_str)
        .collect();
    let required_metrics_measured = missing_required_metrics.is_empty();
    if !measurements_valid {
        global_errors.push(format!("MEASUREMENT_VALIDATION_FAILED:{}", record.case_id));
    }

    for reference in &record.post_decision_outcome_evidence_refs {
        *outcome_ref_count += 1;
        if record.decision_input_evidence_refs.contains(reference) {
            global_errors.push(format!(
                "POST_DECISION_OUTCOME_USED_AS_INPUT:{}:{}",
                record.case_id, reference
            ));
        }
        match evidence.get(reference.as_str()) {
            None => {
                *missing_ref_count += 1;
                global_errors.push(format!(
                    "POST_DECISION_OUTCOME_REFERENCE_MISSING:{}:{}",
                    record.case_id, reference
                ));
            }
            Some(item) => {
                if item.evidence_class != "POST_DECISION_OUTCOME" {
                    global_errors.push(format!(
                        "OUTCOME_REFERENCE_CLASS_INVALID:{}:{}",
                        record.case_id, reference
                    ));
                }
                if item.observed_day <= record.decision_day
                    || item.available_day <= record.decision_day
                {
                    *outcome_before_decision_count += 1;
                    global_errors.push(format!(
                        "OUTCOME_NOT_STRICTLY_POST_DECISION:{}:{}",
                        record.case_id, reference
                    ));
                }
                if record
                    .ground_truth
                    .outcome_day
                    .map(|day| item.observed_day <= day)
                    .unwrap_or(false)
                    == false
                {
                    global_errors.push(format!(
                        "OUTCOME_EVIDENCE_AFTER_OR_MISSING_OUTCOME_DAY:{}:{}",
                        record.case_id, reference
                    ));
                }
            }
        }
    }
    if record
        .ground_truth
        .outcome_day
        .map(|day| day <= record.decision_day)
        .unwrap_or(true)
    {
        global_errors.push(format!(
            "GROUND_TRUTH_OUTCOME_DAY_NOT_POST_DECISION:{}",
            record.case_id
        ));
    }
    if record.ground_truth.outcome_evidence_refs.is_empty()
        || record
            .ground_truth
            .outcome_evidence_refs
            .iter()
            .any(|reference| {
                !record
                    .post_decision_outcome_evidence_refs
                    .contains(reference)
            })
    {
        global_errors.push(format!(
            "GROUND_TRUTH_OUTCOME_REFERENCES_INVALID:{}",
            record.case_id
        ));
    }
    if !matches!(
        record.ground_truth.adjudication_status.as_str(),
        "SYNTHETIC_DEMO_ONLY" | "NOT_ADJUDICATED" | "DISAGREEMENT" | "PENDING"
    ) {
        global_errors.push(format!(
            "GROUND_TRUTH_ADJUDICATION_STATUS_INVALID:{}",
            record.case_id
        ));
    }
    if !matches!(
        record.ground_truth.censoring_status.as_str(),
        "COMPLETE" | "FOLLOW_UP_CENSORED" | "UNKNOWN" | "NOT_APPLICABLE"
    ) {
        global_errors.push(format!(
            "GROUND_TRUTH_CENSORING_STATUS_INVALID:{}",
            record.case_id
        ));
    }
    if (record.ground_truth.censoring_status == "FOLLOW_UP_CENSORED")
        != (record.episode_status == "FOLLOW_UP_CENSORED")
    {
        global_errors.push(format!(
            "CENSORING_EPISODE_STATUS_MISMATCH:{}",
            record.case_id
        ));
    }

    let evidence_ok = refs_valid && measurements_valid && required_metrics_measured;
    let disposition = if !scope_ok {
        "REASSESS_SCOPE"
    } else if !rights_valid && (data_rights_status == "BLOCKED" || data_rights_status == "EXPIRED")
    {
        "BLOCKED_DATA_RIGHTS"
    } else if !rights_valid {
        "HOLD_DATA_RIGHTS"
    } else if blocked_gate {
        "BLOCKED_AUTHORITY"
    } else if unknown_gate {
        "HOLD_AUTHORITY"
    } else if !evidence_ok {
        "HOLD_EVIDENCE"
    } else {
        "REVIEW_ONLY_CANDIDATE"
    };

    let expected_matches = disposition == record.expected_audit_disposition;
    if !expected_matches {
        global_errors.push(format!("EXPECTED_DISPOSITION_MISMATCH:{}", record.case_id));
    }

    CaseResult {
        case_id: record.case_id.clone(),
        profile_scope_status: profile_scope_status.to_string(),
        data_rights_status: data_rights_status.to_string(),
        authority_status: authority_status.to_string(),
        evidence_status: if evidence_ok {
            "SUFFICIENT"
        } else {
            "INSUFFICIENT"
        }
        .to_string(),
        disposition: disposition.to_string(),
        episode_status: record.episode_status.clone(),
        manifest_integrity_status: manifest_status,
        manifest_semantic_status: if manifest_matches {
            "MATCH".to_string()
        } else {
            "MISMATCH".to_string()
        },
        synthetic_review_candidate: disposition == "REVIEW_ONLY_CANDIDATE"
            && global_errors.len() == error_start,
        actuation_authorized: false,
        expected_disposition_matches: expected_matches,
    }
}

fn audit(packet: &Packet, source_commit: Option<String>, mut errors: Vec<String>) -> AuditReceipt {
    validate_packet(packet, source_commit.as_deref(), &mut errors);
    let evidence = evidence_by_id(packet);
    let mut decision_input_reference_count = 0usize;
    let mut post_decision_outcome_reference_count = 0usize;
    let mut future_leakage_count = 0usize;
    let mut outcome_before_decision_count = 0usize;
    let mut missing_ref_count = 0usize;
    let mut duplicate_identity_count = 0usize;
    let mut measurement_state_counts = BTreeMap::new();

    let mut case_results = Vec::new();
    for record in &packet.records {
        case_results.push(validate_record(
            packet,
            record,
            &evidence,
            &mut errors,
            &mut duplicate_identity_count,
            &mut decision_input_reference_count,
            &mut post_decision_outcome_reference_count,
            &mut future_leakage_count,
            &mut outcome_before_decision_count,
            &mut missing_ref_count,
            &mut measurement_state_counts,
        ));
    }
    case_results.sort_by(|a, b| a.case_id.cmp(&b.case_id));

    // Packet-wide failure suppresses all candidate promotion, even for otherwise valid records.
    if !errors.is_empty() {
        for result in &mut case_results {
            result.synthetic_review_candidate = false;
        }
    }

    let mut profile_scope_counts = BTreeMap::new();
    let mut data_rights_counts = BTreeMap::new();
    let mut authority_counts = BTreeMap::new();
    let mut evidence_counts = BTreeMap::new();
    let mut disposition_counts = BTreeMap::new();
    let mut episode_status_counts = BTreeMap::new();
    for result in &case_results {
        inc(&mut profile_scope_counts, &result.profile_scope_status);
        inc(&mut data_rights_counts, &result.data_rights_status);
        inc(&mut authority_counts, &result.authority_status);
        inc(&mut evidence_counts, &result.evidence_status);
        inc(&mut disposition_counts, &result.disposition);
        inc(&mut episode_status_counts, &result.episode_status);
        if result.actuation_authorized {
            errors.push(format!("ACTUATION_AUTHORIZED_OUTPUT:{}", result.case_id));
        }
    }

    let expected_dispositions_match = case_results
        .iter()
        .all(|result| result.expected_disposition_matches);
    let all_records_synthetic = packet.claim_ceiling.all_records_synthetic
        && !packet.claim_ceiling.real_field_observations
        && packet
            .evidence_records
            .iter()
            .all(|item| item.source_authenticity_status == "UNVERIFIED_SYNTHETIC");
    let all_actuation_disabled = !packet.claim_ceiling.expected_actuation
        && packet.claim_ceiling.execution_authority == "NO_ACTUATION_AUTHORIZED"
        && case_results
            .iter()
            .all(|result| !result.actuation_authorized);
    let synthetic_review_candidate_count = case_results
        .iter()
        .filter(|result| result.synthetic_review_candidate)
        .count();

    if !all_records_synthetic {
        errors.push("SYNTHETIC_CLAIM_CEILING_VIOLATED".to_string());
    }
    if !all_actuation_disabled {
        errors.push("ACTUATION_CLAIM_CEILING_VIOLATED".to_string());
    }
    errors.sort();
    errors.dedup();

    let manifest_match_count = case_results
        .iter()
        .filter(|result| {
            result.manifest_integrity_status == "MATCH"
                && result.manifest_semantic_status == "MATCH"
        })
        .count();
    let passed = errors.is_empty()
        && expected_dispositions_match
        && all_records_synthetic
        && all_actuation_disabled
        && future_leakage_count == 0
        && outcome_before_decision_count == 0
        && missing_ref_count == 0
        && manifest_match_count == packet.records.len()
        && synthetic_review_candidate_count
            == case_results
                .iter()
                .filter(|result| result.disposition == "REVIEW_ONLY_CANDIDATE")
                .count();

    AuditReceipt {
        result: if passed { "pass" } else { "fail" }.to_string(),
        schema_version: packet.schema_version.clone(),
        protocol_id: packet.protocol_id.clone(),
        canonical_packet_sha256: canonical_packet_sha256(packet),
        source_commit,
        case_count: packet.records.len(),
        evidence_record_count: packet.evidence_records.len(),
        decision_input_reference_count,
        post_decision_outcome_reference_count,
        manifest_match_count,
        profile_scope_counts,
        data_rights_counts,
        authority_counts,
        evidence_counts,
        disposition_counts,
        episode_status_counts,
        measurement_state_counts,
        decision_time_future_evidence_violation_count: future_leakage_count,
        outcome_before_decision_violation_count: outcome_before_decision_count,
        missing_evidence_reference_count: missing_ref_count,
        duplicate_identity_count,
        expected_dispositions_match,
        all_records_synthetic,
        all_actuation_disabled,
        synthetic_review_candidate_count,
        case_results,
        errors,
    }
}

fn usage_error(message: &str) -> ExitCode {
    eprintln!("{message}");
    ExitCode::FAILURE
}

fn main() -> ExitCode {
    let mut packet_path: Option<String> = None;
    let mut source_commit: Option<String> = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--source-commit" {
            if source_commit.is_some() {
                return usage_error("--source-commit may only be provided once");
            }
            match args.next() {
                Some(value) => source_commit = Some(value),
                None => return usage_error("--source-commit requires a full 40-character SHA"),
            }
        } else if arg.starts_with("--") {
            return usage_error("unsupported command-line option");
        } else if packet_path.is_none() {
            packet_path = Some(arg);
        } else {
            return usage_error("provide at most one packet path");
        }
    }

    let raw = match packet_path {
        Some(path) => match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) => return usage_error(&format!("could not read packet: {error}")),
        },
        None => EMBEDDED_PACKET.as_bytes().to_vec(),
    };
    let packet: Packet = match serde_json::from_slice(&raw) {
        Ok(value) => value,
        Err(error) => return usage_error(&format!("FIELD_VALIDATION_PACKET_JSON_INVALID:{error}")),
    };
    let receipt = audit(&packet, source_commit, Vec::new());
    match serde_json::to_string_pretty(&receipt) {
        Ok(output) => println!("{output}"),
        Err(error) => return usage_error(&format!("RECEIPT_SERIALIZATION_FAILED:{error}")),
    }
    if receipt.result == "pass" {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet() -> Packet {
        serde_json::from_str(EMBEDDED_PACKET).expect("embedded packet should parse")
    }

    fn run(packet: &Packet) -> AuditReceipt {
        audit(packet, Some("a".repeat(40)), Vec::new())
    }

    fn record_mut<'a>(packet: &'a mut Packet, case_id: &str) -> &'a mut FieldRecord {
        packet
            .records
            .iter_mut()
            .find(|record| record.case_id == case_id)
            .expect("record should exist")
    }

    fn result_for<'a>(receipt: &'a AuditReceipt, case_id: &str) -> &'a CaseResult {
        receipt
            .case_results
            .iter()
            .find(|result| result.case_id == case_id)
            .expect("case result exists")
    }

    fn rehash_manifest(record: &mut FieldRecord, manifest: &DecisionManifest) {
        record.decision_manifest_payload_json =
            serde_json::to_string(manifest).expect("manifest serializes");
        record.decision_manifest_sha256 =
            sha256_hex(record.decision_manifest_payload_json.as_bytes());
    }

    #[test]
    fn frozen_synthetic_packet_matches_expected_dispositions() {
        let receipt = run(&packet());
        assert_eq!(receipt.result, "pass", "{:?}", receipt.errors);
        assert_eq!(receipt.case_count, 7);
        assert_eq!(receipt.evidence_record_count, 11);
        assert_eq!(receipt.manifest_match_count, 7);
        assert_eq!(receipt.synthetic_review_candidate_count, 1);
        assert_eq!(
            receipt.disposition_counts.get("REVIEW_ONLY_CANDIDATE"),
            Some(&1)
        );
        assert_eq!(receipt.disposition_counts.get("REASSESS_SCOPE"), Some(&1));
        assert_eq!(
            receipt.disposition_counts.get("BLOCKED_DATA_RIGHTS"),
            Some(&1)
        );
        assert_eq!(receipt.disposition_counts.get("HOLD_DATA_RIGHTS"), Some(&1));
        assert_eq!(
            receipt.disposition_counts.get("BLOCKED_AUTHORITY"),
            Some(&1)
        );
        assert_eq!(receipt.disposition_counts.get("HOLD_AUTHORITY"), Some(&1));
        assert_eq!(receipt.disposition_counts.get("HOLD_EVIDENCE"), Some(&1));
        assert!(receipt.all_records_synthetic);
        assert!(receipt.all_actuation_disabled);
        assert_eq!(receipt.decision_time_future_evidence_violation_count, 0);
        assert_eq!(receipt.outcome_before_decision_violation_count, 0);
    }

    #[test]
    fn measured_zero_is_not_missing_data() {
        let receipt = run(&packet());
        let record = result_for(&receipt, "field-valid-zero-measurement");
        assert_eq!(record.disposition, "REVIEW_ONLY_CANDIDATE");
        assert_eq!(record.evidence_status, "SUFFICIENT");
        assert_eq!(
            packet()
                .records
                .iter()
                .find(|r| r.case_id == "field-valid-zero-measurement")
                .unwrap()
                .measurements[0]
                .value_scaled,
            Some(0)
        );
    }

    #[test]
    fn unmeasured_required_metric_holds_even_if_other_metric_is_zero() {
        let receipt = run(&packet());
        let record = result_for(&receipt, "field-required-metric-not-measured");
        assert_eq!(record.disposition, "HOLD_EVIDENCE");
        assert_eq!(record.evidence_status, "INSUFFICIENT");
    }

    #[test]
    fn future_decision_input_is_rejected() {
        let mut value = packet();
        let case = record_mut(&mut value, "field-valid-zero-measurement");
        case.decision_input_evidence_refs
            .push("synthetic://field-evidence/outcome-follow-up-complete".to_string());
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert!(receipt.decision_time_future_evidence_violation_count > 0);
        assert_eq!(receipt.synthetic_review_candidate_count, 0);
    }

    #[test]
    fn data_rights_block_precedes_other_gates() {
        let receipt = run(&packet());
        assert_eq!(
            result_for(&receipt, "field-blocked-data-rights").disposition,
            "BLOCKED_DATA_RIGHTS"
        );
    }

    #[test]
    fn unknown_rights_hold_and_do_not_become_allowed() {
        let receipt = run(&packet());
        assert_eq!(
            result_for(&receipt, "field-unknown-data-rights").disposition,
            "HOLD_DATA_RIGHTS"
        );
    }

    #[test]
    fn authority_block_is_non_compensatory() {
        let receipt = run(&packet());
        assert_eq!(
            result_for(&receipt, "field-ecological-review-block").disposition,
            "BLOCKED_AUTHORITY"
        );
    }

    #[test]
    fn unknown_legal_authority_requires_hold() {
        let receipt = run(&packet());
        assert_eq!(
            result_for(&receipt, "field-unknown-legal-authority").disposition,
            "HOLD_AUTHORITY"
        );
    }

    #[test]
    fn incomplete_hazard_classification_reassesses_scope() {
        let receipt = run(&packet());
        assert_eq!(
            result_for(&receipt, "field-incomplete-hazard-classification").disposition,
            "REASSESS_SCOPE"
        );
    }

    #[test]
    fn expired_data_retention_blocks_use() {
        let mut value = packet();
        record_mut(&mut value, "field-valid-zero-measurement")
            .data_rights
            .retention_until_day = 11;
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert_eq!(
            result_for(&receipt, "field-valid-zero-measurement").disposition,
            "BLOCKED_DATA_RIGHTS"
        );
        assert_eq!(receipt.synthetic_review_candidate_count, 0);
    }

    #[test]
    fn missing_license_with_allowed_status_holds_rights() {
        let mut value = packet();
        record_mut(&mut value, "field-valid-zero-measurement")
            .data_rights
            .license_ref = None;
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert_eq!(
            result_for(&receipt, "field-valid-zero-measurement").disposition,
            "HOLD_DATA_RIGHTS"
        );
    }

    #[test]
    fn decision_manifest_digest_tampering_is_detected() {
        let mut value = packet();
        record_mut(&mut value, "field-valid-zero-measurement")
            .decision_manifest_payload_json
            .push(' ');
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert!(
            receipt
                .errors
                .iter()
                .any(|error| error.starts_with("DECISION_MANIFEST_DIGEST_MISMATCH"))
        );
        assert_eq!(receipt.synthetic_review_candidate_count, 0);
    }

    #[test]
    fn rehashed_but_semantically_changed_manifest_is_detected() {
        let mut value = packet();
        let case = record_mut(&mut value, "field-valid-zero-measurement");
        let mut manifest: DecisionManifest =
            serde_json::from_str(&case.decision_manifest_payload_json).unwrap();
        manifest.taxonomy_version = "forest-hazard-taxonomy-v2".to_string();
        rehash_manifest(case, &manifest);
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert!(
            receipt
                .errors
                .iter()
                .any(|error| error.starts_with("DECISION_MANIFEST_SEMANTIC_MISMATCH"))
        );
    }

    #[test]
    fn missing_authority_gate_is_rejected() {
        let mut value = packet();
        record_mut(&mut value, "field-valid-zero-measurement")
            .authority_gates
            .pop();
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert!(
            receipt
                .errors
                .iter()
                .any(|error| error.starts_with("AUTHORITY_GATE_SET_INCOMPLETE"))
        );
    }

    #[test]
    fn missing_required_decision_evidence_tag_is_rejected() {
        let mut value = packet();
        let record = record_mut(&mut value, "field-valid-zero-measurement");
        record
            .decision_input_evidence_refs
            .retain(|reference| reference != "synthetic://field-evidence/data-rights-record");
        let mut manifest: DecisionManifest =
            serde_json::from_str(&record.decision_manifest_payload_json).unwrap();
        manifest.decision_input_evidence_refs = record.decision_input_evidence_refs.clone();
        rehash_manifest(record, &manifest);
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert!(
            receipt
                .errors
                .iter()
                .any(|error| error.contains("REQUIRED_DECISION_EVIDENCE_TAG_MISSING"))
        );
    }

    #[test]
    fn post_decision_outcomes_must_be_strictly_later() {
        let mut value = packet();
        let outcome = value
            .evidence_records
            .iter_mut()
            .find(|e| e.evidence_id == "synthetic://field-evidence/outcome-follow-up-complete")
            .unwrap();
        outcome.observed_day = 2;
        outcome.available_day = 2;
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert!(receipt.outcome_before_decision_violation_count > 0);
    }

    #[test]
    fn post_decision_outcomes_cannot_be_used_as_decision_input() {
        let mut value = packet();
        let record = record_mut(&mut value, "field-valid-zero-measurement");
        record
            .decision_input_evidence_refs
            .push("synthetic://field-evidence/outcome-follow-up-complete".to_string());
        let mut manifest: DecisionManifest =
            serde_json::from_str(&record.decision_manifest_payload_json).unwrap();
        manifest.decision_input_evidence_refs = record.decision_input_evidence_refs.clone();
        rehash_manifest(record, &manifest);
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert!(
            receipt
                .errors
                .iter()
                .any(|error| error.starts_with("OUTCOME_REFERENCE_USED_AS_DECISION_INPUT"))
        );
    }

    #[test]
    fn measured_value_requires_linked_decision_time_evidence() {
        let mut value = packet();
        let measurement =
            &mut record_mut(&mut value, "field-valid-zero-measurement").measurements[0];
        measurement.evidence_ref =
            Some("synthetic://field-evidence/outcome-follow-up-complete".to_string());
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert!(
            receipt
                .errors
                .iter()
                .any(|error| error.starts_with("MEASUREMENT_EVIDENCE_NOT_DECISION_INPUT"))
        );
    }

    #[test]
    fn unmeasured_state_cannot_carry_numeric_zero() {
        let mut value = packet();
        let measurement = record_mut(&mut value, "field-required-metric-not-measured")
            .measurements
            .iter_mut()
            .find(|m| m.metric_id == "retained_deadwood_fraction")
            .unwrap();
        measurement.value_scaled = Some(0);
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert!(
            receipt
                .errors
                .iter()
                .any(|error| error.starts_with("UNQUANTIFIED_STATE_HAS_NUMERIC_VALUE"))
        );
    }

    #[test]
    fn public_location_disclosure_is_rejected() {
        let mut value = packet();
        record_mut(&mut value, "field-valid-zero-measurement")
            .data_rights
            .public_location_disclosed = true;
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert!(
            receipt
                .errors
                .iter()
                .any(|error| error.starts_with("PUBLIC_LOCATION_DISCLOSURE_FORBIDDEN"))
        );
    }

    #[test]
    fn row_order_does_not_change_canonical_packet_digest_or_receipt() {
        let original = packet();
        let mut permuted = original.clone();
        permuted.records.reverse();
        permuted.evidence_records.reverse();
        for record in &mut permuted.records {
            // Decision-input references are required to remain canonical because their order is hash-bound in the manifest.
            record.post_decision_outcome_evidence_refs.reverse();
            record.required_metric_ids.reverse();
            record.data_rights.allowed_purposes.reverse();
            record.authority_gates.reverse();
            record.measurements.reverse();
            record.ground_truth.outcome_evidence_refs.reverse();
        }
        for evidence in &mut permuted.evidence_records {
            evidence.evidence_tags.reverse();
        }
        let first = run(&original);
        let second = run(&permuted);
        assert_eq!(
            first.canonical_packet_sha256,
            second.canonical_packet_sha256
        );
        assert_eq!(
            serde_json::to_vec(&first).expect("receipt serializes"),
            serde_json::to_vec(&second).expect("receipt serializes")
        );
    }

    #[test]
    fn duplicate_evidence_ids_fail_closed() {
        let mut value = packet();
        let duplicate = value.evidence_records[0].clone();
        value.evidence_records.push(duplicate);
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert_eq!(receipt.synthetic_review_candidate_count, 0);
        assert!(
            receipt
                .errors
                .iter()
                .any(|error| error.starts_with("DUPLICATE_EVIDENCE_ID"))
        );
    }

    #[test]
    fn referral_and_censored_episodes_remain_in_denominator() {
        let receipt = run(&packet());
        assert_eq!(receipt.episode_status_counts.get("NO_ACTION"), Some(&3));
        assert_eq!(receipt.episode_status_counts.get("REFERRED"), Some(&3));
        assert_eq!(
            receipt.episode_status_counts.get("FOLLOW_UP_CENSORED"),
            Some(&1)
        );
        assert_eq!(receipt.case_count, 7);
    }

    #[test]
    fn invalid_source_commit_fails_closed() {
        let receipt = audit(&packet(), Some("bad".to_string()), Vec::new());
        assert_eq!(receipt.result, "fail");
        assert!(
            receipt
                .errors
                .iter()
                .any(|error| error == "SOURCE_COMMIT_INVALID")
        );
    }

    #[test]
    fn no_synthetic_case_can_authorize_actuation() {
        let receipt = run(&packet());
        assert!(receipt.all_actuation_disabled);
        assert!(
            receipt
                .case_results
                .iter()
                .all(|result| !result.actuation_authorized)
        );
    }
}
