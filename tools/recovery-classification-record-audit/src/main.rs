use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const EMBEDDED_PACKET: &str =
    include_str!("../../../docs/tech/recovery-classification-record-packet-v0.1.json");
const EMBEDDED_ENVELOPE: &str =
    include_str!("../../../docs/tech/recovery-scenario-envelope-corpus-v0.1.json");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ClaimCeiling {
    all_records_synthetic: bool,
    authenticity_verified: bool,
    professional_authority_verified: bool,
    expected_actuation: bool,
    execution_authority: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Packet {
    schema_version: String,
    algorithm_id: String,
    status: String,
    decision_day: u64,
    claim_ceiling: ClaimCeiling,
    records: Vec<StoredRecord>,
    case_bindings: Vec<CaseBinding>,
    revocation_entries: Vec<RevocationEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct StoredRecord {
    record_id: String,
    canonical_payload_json: String,
    sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ClassificationRecordPayload {
    record_id: String,
    case_id: String,
    profile_id: String,
    domain: String,
    profile_version: String,
    criteria_regime: String,
    taxonomy_version: String,
    classification_status: String,
    classification_review_status: String,
    classified_failure_families: Vec<String>,
    evidence_tags: Vec<String>,
    issued_day: u64,
    valid_from_day: u64,
    valid_until_day: u64,
    revocation_policy_id: String,
    source_authenticity_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct CaseBinding {
    case_id: String,
    record_id: String,
    sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct RevocationEntry {
    record_id: String,
    status: String,
    recorded_day: u64,
    policy_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct EnvelopeCorpus {
    profiles: Vec<EnvelopeProfile>,
    cases: Vec<EnvelopeCase>,
}

#[derive(Debug, Clone, Deserialize)]
struct EnvelopeProfile {
    profile_id: String,
    domain: String,
    profile_version: String,
    criteria_regime: String,
    family_taxonomy_version: String,
    covered_failure_families: Vec<String>,
    required_evidence_tags: Vec<String>,
    max_evidence_age_days: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct EnvelopeCase {
    case_id: String,
    profile_id: String,
    domain: String,
    #[serde(default)]
    evidence_age_days: Option<u64>,
    hard_gate_status: String,
    profile_version: String,
    criteria_regime: String,
    family_taxonomy_version: String,
    classification_review_status: String,
    classification_record_ref: Option<String>,
    family_classification: String,
    observed_failure_families: Vec<String>,
    observed_evidence_tags: Vec<String>,
    expected_disposition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct CaseResult {
    case_id: String,
    record_id: Option<String>,
    record_integrity_status: String,
    case_binding_status: String,
    profile_scope_status: String,
    lifecycle_status: String,
    hard_gate_status: String,
    evidence_sufficient: bool,
    authenticity_status: String,
    synthetic_review_candidate: bool,
    actuation_authorized: bool,
}

#[derive(Debug, Serialize)]
struct AuditReceipt {
    result: String,
    schema_version: String,
    algorithm_id: String,
    canonical_packet_sha256: String,
    source_commit: Option<String>,
    record_count: usize,
    binding_count: usize,
    revocation_entry_count: usize,
    case_count: usize,
    integrity_counts: BTreeMap<String, usize>,
    binding_counts: BTreeMap<String, usize>,
    scope_counts: BTreeMap<String, usize>,
    lifecycle_counts: BTreeMap<String, usize>,
    synthetic_review_candidate_count: usize,
    all_authenticity_unverified: bool,
    all_actuation_disabled: bool,
    case_results: Vec<CaseResult>,
    errors: Vec<String>,
}

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

fn increment(map: &mut BTreeMap<String, usize>, key: &str) {
    *map.entry(key.to_string()).or_insert(0) += 1;
}

fn sorted(values: &[String]) -> Vec<String> {
    let mut result = values.to_vec();
    result.sort();
    result
}

fn validate_packet(packet: &Packet, source_commit: Option<&str>, errors: &mut Vec<String>) {
    if packet.schema_version != "recovery-classification-record-packet-v0.1" {
        errors.push("SCHEMA_VERSION_UNSUPPORTED".to_string());
    }
    if packet.algorithm_id != "content-bound-classification-record-audit-v0.1" {
        errors.push("ALGORITHM_ID_UNSUPPORTED".to_string());
    }
    if packet.status != "synthetic-integrity-and-binding-fixtures" {
        errors.push("PACKET_STATUS_UNEXPECTED".to_string());
    }
    if !packet.claim_ceiling.all_records_synthetic {
        errors.push("ALL_RECORDS_MUST_BE_DECLARED_SYNTHETIC".to_string());
    }
    if packet.claim_ceiling.authenticity_verified {
        errors.push("AUTHENTICITY_MUST_REMAIN_UNVERIFIED".to_string());
    }
    if packet.claim_ceiling.professional_authority_verified {
        errors.push("PROFESSIONAL_AUTHORITY_MUST_NOT_BE_CLAIMED".to_string());
    }
    if packet.claim_ceiling.expected_actuation {
        errors.push("ACTUATION_CEILING_MUST_BE_FALSE".to_string());
    }
    if packet.claim_ceiling.execution_authority != "NO_ACTUATION_AUTHORIZED" {
        errors.push("EXECUTION_AUTHORITY_INVALID".to_string());
    }
    if let Some(commit) = source_commit {
        if commit.len() != 40 || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            errors.push("SOURCE_COMMIT_INVALID".to_string());
        }
    }
    if packet.records.is_empty() {
        errors.push("RECORDS_MISSING_OR_EMPTY".to_string());
    }
    if packet.case_bindings.is_empty() {
        errors.push("CASE_BINDINGS_MISSING_OR_EMPTY".to_string());
    }
    if packet.revocation_entries.is_empty() {
        errors.push("REVOCATION_ENTRIES_MISSING_OR_EMPTY".to_string());
    }

    let mut record_ids = BTreeSet::new();
    for record in &packet.records {
        if !is_nonempty(&record.record_id) {
            errors.push("RECORD_ID_EMPTY".to_string());
        } else if !record_ids.insert(&record.record_id) {
            errors.push(format!("DUPLICATE_RECORD_ID:{}", record.record_id));
        }
        if !is_sha256(&record.sha256) {
            errors.push(format!("RECORD_SHA256_INVALID:{}", record.record_id));
        }
    }

    let mut binding_cases = BTreeSet::new();
    for binding in &packet.case_bindings {
        if !is_nonempty(&binding.case_id) || !is_nonempty(&binding.record_id) {
            errors.push("BINDING_IDENTITY_EMPTY".to_string());
        }
        if !binding_cases.insert(&binding.case_id) {
            errors.push(format!("DUPLICATE_CASE_BINDING:{}", binding.case_id));
        }
        if !is_sha256(&binding.sha256) {
            errors.push(format!("BINDING_SHA256_INVALID:{}", binding.case_id));
        }
    }

    let mut revocation_ids = BTreeSet::new();
    for entry in &packet.revocation_entries {
        if !is_nonempty(&entry.record_id) {
            errors.push("REVOCATION_RECORD_ID_EMPTY".to_string());
        }
        if !is_nonempty(&entry.policy_id) {
            errors.push(format!("REVOCATION_POLICY_ID_EMPTY:{}", entry.record_id));
        }
        if !revocation_ids.insert(&entry.record_id) {
            errors.push(format!("DUPLICATE_REVOCATION_ENTRY:{}", entry.record_id));
        }
        if !matches!(entry.status.as_str(), "ACTIVE" | "REVOKED" | "UNKNOWN") {
            errors.push(format!("REVOCATION_STATUS_INVALID:{}", entry.record_id));
        }
    }
}

fn canonical_packet_digest(packet: &Packet) -> String {
    let mut canonical = packet.clone();
    canonical
        .records
        .sort_by(|a, b| a.record_id.cmp(&b.record_id));
    canonical
        .case_bindings
        .sort_by(|a, b| a.case_id.cmp(&b.case_id));
    canonical
        .revocation_entries
        .sort_by(|a, b| a.record_id.cmp(&b.record_id));
    match serde_json::to_vec(&canonical) {
        Ok(bytes) => sha256_hex(&bytes),
        Err(_) => String::new(),
    }
}

fn payload_for(
    record: &StoredRecord,
    errors: &mut Vec<String>,
) -> Option<ClassificationRecordPayload> {
    let payload: ClassificationRecordPayload =
        match serde_json::from_str(&record.canonical_payload_json) {
            Ok(value) => value,
            Err(error) => {
                errors.push(format!(
                    "RECORD_PAYLOAD_JSON_INVALID:{}:{error}",
                    record.record_id
                ));
                return None;
            }
        };

    let canonical = match serde_json::to_string(&payload) {
        Ok(value) => value,
        Err(error) => {
            errors.push(format!(
                "RECORD_CANONICAL_SERIALIZATION_FAILED:{}:{error}",
                record.record_id
            ));
            return None;
        }
    };
    if canonical != record.canonical_payload_json {
        errors.push(format!("RECORD_PAYLOAD_NOT_CANONICAL:{}", record.record_id));
    }
    let computed = sha256_hex(record.canonical_payload_json.as_bytes());
    if computed != record.sha256 {
        errors.push(format!("RECORD_DIGEST_MISMATCH:{}", record.record_id));
    }
    if payload.record_id != record.record_id {
        errors.push(format!("RECORD_OUTER_ID_MISMATCH:{}", record.record_id));
    }
    if payload.source_authenticity_status != "UNVERIFIED_SYNTHETIC" {
        errors.push(format!(
            "RECORD_AUTHENTICITY_CEILING_VIOLATION:{}",
            record.record_id
        ));
    }
    if payload.issued_day > payload.valid_from_day
        || payload.valid_from_day > payload.valid_until_day
    {
        errors.push(format!(
            "RECORD_VALIDITY_RANGE_INVALID:{}",
            record.record_id
        ));
    }
    if !is_nonempty(&payload.revocation_policy_id) {
        errors.push(format!("REVOCATION_POLICY_ID_MISSING:{}", record.record_id));
    } else if payload.revocation_policy_id != "synthetic-revocation-policy-v1" {
        errors.push(format!(
            "REVOCATION_POLICY_UNSUPPORTED:{}",
            record.record_id
        ));
    }
    if payload.classified_failure_families != sorted(&payload.classified_failure_families) {
        errors.push(format!(
            "CLASSIFIED_FAMILIES_NOT_CANONICAL:{}",
            record.record_id
        ));
    }
    if payload.evidence_tags != sorted(&payload.evidence_tags) {
        errors.push(format!("EVIDENCE_TAGS_NOT_CANONICAL:{}", record.record_id));
    }
    if !matches!(
        payload.classification_status.as_str(),
        "COMPLETE" | "INCOMPLETE" | "UNKNOWN"
    ) {
        errors.push(format!(
            "CLASSIFICATION_STATUS_INVALID:{}",
            record.record_id
        ));
    }
    if !matches!(
        payload.classification_review_status.as_str(),
        "REVIEWED" | "UNREVIEWED" | "UNKNOWN"
    ) {
        errors.push(format!(
            "CLASSIFICATION_REVIEW_STATUS_INVALID:{}",
            record.record_id
        ));
    }

    Some(payload)
}

fn evaluate_case(
    case: &EnvelopeCase,
    profiles: &[EnvelopeProfile],
    bindings: &[CaseBinding],
    records: &[StoredRecord],
    revocations: &[RevocationEntry],
    decision_day: u64,
    errors: &mut Vec<String>,
) -> CaseResult {
    let case_error_start = errors.len();
    let matching_bindings: Vec<&CaseBinding> = bindings
        .iter()
        .filter(|binding| binding.case_id == case.case_id)
        .collect();
    let binding = matching_bindings.first().copied();
    let mut result = CaseResult {
        case_id: case.case_id.clone(),
        record_id: binding.map(|value| value.record_id.clone()),
        record_integrity_status: "MISSING".to_string(),
        case_binding_status: "MISSING".to_string(),
        profile_scope_status: "OUT_OF_SCOPE".to_string(),
        lifecycle_status: "UNKNOWN".to_string(),
        hard_gate_status: case.hard_gate_status.clone(),
        evidence_sufficient: false,
        authenticity_status: "UNVERIFIED_SYNTHETIC".to_string(),
        synthetic_review_candidate: false,
        actuation_authorized: false,
    };

    if matching_bindings.len() != 1 {
        result.case_binding_status = "BINDING_CARDINALITY_INVALID".to_string();
        errors.push(format!("CASE_BINDING_CARDINALITY_INVALID:{}", case.case_id));
        return result;
    }
    let binding = binding.expect("one binding was checked above");
    if case.classification_record_ref.as_deref() != Some(binding.record_id.as_str()) {
        result.case_binding_status = "CASE_REFERENCE_MISMATCH".to_string();
        errors.push(format!("CASE_REFERENCE_MISMATCH:{}", case.case_id));
    }

    let matching_records: Vec<&StoredRecord> = records
        .iter()
        .filter(|record| record.record_id == binding.record_id)
        .collect();
    if matching_records.len() != 1 {
        result.record_integrity_status = if matching_records.is_empty() {
            "MISSING".to_string()
        } else {
            "RECORD_CARDINALITY_INVALID".to_string()
        };
        errors.push(format!("BOUND_RECORD_CARDINALITY_INVALID:{}", case.case_id));
        return result;
    }
    let record = matching_records[0];
    let before_errors = errors.len();
    let payload = payload_for(record, errors);
    let Some(payload) = payload else {
        result.record_integrity_status = "INVALID".to_string();
        result.authenticity_status = "UNAVAILABLE".to_string();
        return result;
    };
    result.authenticity_status = payload.source_authenticity_status.clone();

    let record_digest_matches =
        sha256_hex(record.canonical_payload_json.as_bytes()) == record.sha256;
    let binding_digest_matches = binding.sha256 == record.sha256;
    if record_digest_matches && binding_digest_matches && errors.len() == before_errors {
        result.record_integrity_status = "MATCH".to_string();
    } else {
        result.record_integrity_status = "MISMATCH".to_string();
        if !binding_digest_matches {
            errors.push(format!("CASE_BINDING_DIGEST_MISMATCH:{}", case.case_id));
        }
    }

    let semantic_match = payload.record_id == binding.record_id
        && payload.case_id == case.case_id
        && payload.profile_id == case.profile_id
        && payload.domain == case.domain
        && payload.profile_version == case.profile_version
        && payload.criteria_regime == case.criteria_regime
        && payload.taxonomy_version == case.family_taxonomy_version
        && payload.classification_status == case.family_classification
        && payload.classification_review_status == case.classification_review_status
        && sorted(&payload.classified_failure_families) == sorted(&case.observed_failure_families)
        && sorted(&payload.evidence_tags) == sorted(&case.observed_evidence_tags);
    if !semantic_match {
        result.case_binding_status = "SEMANTIC_MISMATCH".to_string();
        errors.push(format!("RECORD_CASE_SEMANTIC_MISMATCH:{}", case.case_id));
    } else if case.classification_review_status != "REVIEWED" {
        result.case_binding_status = "REVIEW_NOT_COMPLETE".to_string();
    } else {
        result.case_binding_status = "MATCH".to_string();
    }

    if let Some(profile) = profiles
        .iter()
        .find(|profile| profile.profile_id == case.profile_id)
    {
        let in_scope = case.domain == profile.domain
            && case.profile_version == profile.profile_version
            && case.criteria_regime == profile.criteria_regime
            && case.family_taxonomy_version == profile.family_taxonomy_version
            && case.family_classification == "COMPLETE"
            && case.classification_review_status == "REVIEWED"
            && !case.observed_failure_families.is_empty()
            && case
                .observed_failure_families
                .iter()
                .all(|family| profile.covered_failure_families.contains(family));
        result.profile_scope_status = if in_scope {
            "IN_SCOPE".to_string()
        } else {
            "OUT_OF_SCOPE".to_string()
        };
        result.evidence_sufficient = case
            .evidence_age_days
            .map(|age| age <= profile.max_evidence_age_days)
            .unwrap_or(false)
            && profile
                .required_evidence_tags
                .iter()
                .all(|required| case.observed_evidence_tags.contains(required));
    }

    let matching_revocations: Vec<&RevocationEntry> = revocations
        .iter()
        .filter(|entry| entry.record_id == binding.record_id)
        .collect();
    if matching_revocations.len() != 1 {
        result.lifecycle_status = "REVOCATION_CARDINALITY_INVALID".to_string();
        errors.push(format!("REVOCATION_CARDINALITY_INVALID:{}", case.case_id));
    } else {
        let revocation = matching_revocations[0];
        if revocation.policy_id != payload.revocation_policy_id {
            result.lifecycle_status = "REVOCATION_POLICY_MISMATCH".to_string();
            errors.push(format!("REVOCATION_POLICY_MISMATCH:{}", case.case_id));
        } else if revocation.status == "REVOKED" {
            result.lifecycle_status = "REVOKED".to_string();
            errors.push(format!("RECORD_REVOKED:{}", case.case_id));
        } else if revocation.status != "ACTIVE" {
            result.lifecycle_status = "REVOCATION_UNKNOWN".to_string();
            errors.push(format!("RECORD_REVOCATION_UNKNOWN:{}", case.case_id));
        } else if payload.valid_until_day < decision_day {
            result.lifecycle_status = "EXPIRED".to_string();
            errors.push(format!("RECORD_EXPIRED:{}", case.case_id));
        } else if payload.valid_from_day > decision_day || payload.issued_day > decision_day {
            result.lifecycle_status = "NOT_YET_VALID".to_string();
            errors.push(format!("RECORD_NOT_YET_VALID:{}", case.case_id));
        } else if revocation.recorded_day > decision_day {
            result.lifecycle_status = "REVOCATION_TIME_INVALID".to_string();
            errors.push(format!("REVOCATION_ENTRY_FROM_FUTURE:{}", case.case_id));
        } else {
            result.lifecycle_status = "VALID_ACTIVE".to_string();
        }
    }

    if !matches!(case.hard_gate_status.as_str(), "PASS" | "BLOCK" | "UNKNOWN") {
        errors.push(format!("CASE_HARD_GATE_STATUS_INVALID:{}", case.case_id));
    }
    result.synthetic_review_candidate = result.record_integrity_status == "MATCH"
        && result.case_binding_status == "MATCH"
        && result.profile_scope_status == "IN_SCOPE"
        && result.evidence_sufficient
        && result.hard_gate_status == "PASS"
        && result.lifecycle_status == "VALID_ACTIVE"
        && case.expected_disposition == "REVIEW_ONLY_CANDIDATE"
        && errors.len() == case_error_start;
    result
}

fn audit(
    packet: &Packet,
    envelope: &EnvelopeCorpus,
    source_commit: Option<String>,
    mut errors: Vec<String>,
) -> AuditReceipt {
    validate_packet(packet, source_commit.as_deref(), &mut errors);

    let mut record_ids = BTreeSet::new();
    for record in &packet.records {
        record_ids.insert(record.record_id.clone());
    }
    let mut binding_case_ids = BTreeSet::new();
    for binding in &packet.case_bindings {
        binding_case_ids.insert(binding.case_id.clone());
        if !envelope
            .cases
            .iter()
            .any(|case| case.case_id == binding.case_id)
        {
            errors.push(format!("ORPHAN_CASE_BINDING:{}", binding.case_id));
        }
        if !record_ids.contains(&binding.record_id) {
            errors.push(format!("BINDING_RECORD_MISSING:{}", binding.case_id));
        }
    }
    for case in &envelope.cases {
        if !binding_case_ids.contains(&case.case_id) {
            errors.push(format!("CASE_BINDING_MISSING:{}", case.case_id));
        }
    }
    for record in &packet.records {
        if !packet
            .case_bindings
            .iter()
            .any(|binding| binding.record_id == record.record_id)
        {
            errors.push(format!("ORPHAN_RECORD:{}", record.record_id));
        }
        if !packet
            .revocation_entries
            .iter()
            .any(|entry| entry.record_id == record.record_id)
        {
            errors.push(format!("REVOCATION_ENTRY_MISSING:{}", record.record_id));
        }
    }
    for entry in &packet.revocation_entries {
        if !record_ids.contains(&entry.record_id) {
            errors.push(format!("ORPHAN_REVOCATION_ENTRY:{}", entry.record_id));
        }
    }

    let mut case_results = Vec::new();
    for case in &envelope.cases {
        case_results.push(evaluate_case(
            case,
            &envelope.profiles,
            &packet.case_bindings,
            &packet.records,
            &packet.revocation_entries,
            packet.decision_day,
            &mut errors,
        ));
    }
    case_results.sort_by(|a, b| a.case_id.cmp(&b.case_id));

    if !errors.is_empty() {
        for result in &mut case_results {
            result.synthetic_review_candidate = false;
        }
    }

    let mut integrity_counts = BTreeMap::new();
    let mut binding_counts = BTreeMap::new();
    let mut scope_counts = BTreeMap::new();
    let mut lifecycle_counts = BTreeMap::new();
    for result in &case_results {
        increment(&mut integrity_counts, &result.record_integrity_status);
        increment(&mut binding_counts, &result.case_binding_status);
        increment(&mut scope_counts, &result.profile_scope_status);
        increment(&mut lifecycle_counts, &result.lifecycle_status);
        if result.actuation_authorized {
            errors.push(format!("ACTUATION_AUTHORIZED_OUTPUT:{}", result.case_id));
        }
    }

    if case_results.len() != envelope.cases.len() {
        errors.push("CASE_RESULT_COUNT_MISMATCH".to_string());
    }
    let synthetic_review_candidate_count = case_results
        .iter()
        .filter(|result| result.synthetic_review_candidate)
        .count();
    let all_authenticity_unverified = !packet.claim_ceiling.authenticity_verified
        && case_results.iter().all(|result| {
            result.authenticity_status == "UNVERIFIED_SYNTHETIC"
                && result.record_integrity_status == "MATCH"
                && result.case_binding_status == "MATCH"
        });
    let all_actuation_disabled = !packet.claim_ceiling.expected_actuation
        && case_results
            .iter()
            .all(|result| !result.actuation_authorized);
    if !all_authenticity_unverified {
        errors.push("AUTHENTICITY_CEILING_VIOLATED".to_string());
    }
    if !all_actuation_disabled {
        errors.push("ACTUATION_CEILING_VIOLATED".to_string());
    }

    errors.sort();
    errors.dedup();
    let passed = errors.is_empty()
        && case_results.len() == envelope.cases.len()
        && integrity_counts.get("MATCH") == Some(&envelope.cases.len())
        && binding_counts.get("MATCH") == Some(&envelope.cases.len())
        && lifecycle_counts.get("VALID_ACTIVE") == Some(&envelope.cases.len())
        && all_authenticity_unverified
        && all_actuation_disabled;

    AuditReceipt {
        result: if passed { "pass" } else { "fail" }.to_string(),
        schema_version: packet.schema_version.clone(),
        algorithm_id: packet.algorithm_id.clone(),
        canonical_packet_sha256: canonical_packet_digest(packet),
        source_commit,
        record_count: packet.records.len(),
        binding_count: packet.case_bindings.len(),
        revocation_entry_count: packet.revocation_entries.len(),
        case_count: case_results.len(),
        integrity_counts,
        binding_counts,
        scope_counts,
        lifecycle_counts,
        synthetic_review_candidate_count,
        all_authenticity_unverified,
        all_actuation_disabled,
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
                return usage_error("--source-commit may be supplied only once");
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
            return usage_error("provide at most one record-packet path");
        }
    }

    let raw = match packet_path {
        Some(path) => match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) => return usage_error(&format!("could not read record packet: {error}")),
        },
        None => EMBEDDED_PACKET.as_bytes().to_vec(),
    };
    let packet: Packet = match serde_json::from_slice(&raw) {
        Ok(value) => value,
        Err(error) => return usage_error(&format!("RECORD_PACKET_JSON_INVALID: {error}")),
    };
    let envelope: EnvelopeCorpus = match serde_json::from_str(EMBEDDED_ENVELOPE) {
        Ok(value) => value,
        Err(error) => return usage_error(&format!("EMBEDDED_ENVELOPE_JSON_INVALID: {error}")),
    };

    let receipt = audit(&packet, &envelope, source_commit, Vec::new());
    match serde_json::to_string_pretty(&receipt) {
        Ok(output) => println!("{output}"),
        Err(error) => return usage_error(&format!("RECEIPT_SERIALIZATION_FAILED: {error}")),
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

    fn envelope() -> EnvelopeCorpus {
        serde_json::from_str(EMBEDDED_ENVELOPE).expect("embedded envelope should parse")
    }

    fn run(packet: &Packet) -> AuditReceipt {
        audit(packet, &envelope(), Some("a".repeat(40)), Vec::new())
    }

    fn record_mut<'a>(packet: &'a mut Packet, record_id: &str) -> &'a mut StoredRecord {
        packet
            .records
            .iter_mut()
            .find(|record| record.record_id == record_id)
            .expect("test record should exist")
    }

    fn payload_of(record: &StoredRecord) -> ClassificationRecordPayload {
        serde_json::from_str(&record.canonical_payload_json).expect("payload should parse")
    }

    fn update_payload(
        packet: &mut Packet,
        record_id: &str,
        mutate: impl FnOnce(&mut ClassificationRecordPayload),
    ) {
        let record = record_mut(packet, record_id);
        let mut payload = payload_of(record);
        mutate(&mut payload);
        record.canonical_payload_json =
            serde_json::to_string(&payload).expect("payload serializes");
        record.sha256 = sha256_hex(record.canonical_payload_json.as_bytes());
        let digest = record.sha256.clone();
        for binding in &mut packet.case_bindings {
            if binding.record_id == record_id {
                binding.sha256 = digest.clone();
            }
        }
    }

    fn result_for<'a>(receipt: &'a AuditReceipt, case_id: &str) -> &'a CaseResult {
        receipt
            .case_results
            .iter()
            .find(|result| result.case_id == case_id)
            .expect("case result should exist")
    }

    #[test]
    fn frozen_packet_matches_expected_summary() {
        let receipt = run(&packet());
        assert_eq!(receipt.result, "pass");
        assert_eq!(receipt.record_count, 17);
        assert_eq!(receipt.binding_count, 17);
        assert_eq!(receipt.revocation_entry_count, 17);
        assert_eq!(receipt.case_count, 17);
        assert_eq!(receipt.integrity_counts.get("MATCH"), Some(&17));
        assert_eq!(receipt.binding_counts.get("MATCH"), Some(&17));
        assert_eq!(receipt.scope_counts.get("IN_SCOPE"), Some(&10));
        assert_eq!(receipt.scope_counts.get("OUT_OF_SCOPE"), Some(&7));
        assert_eq!(receipt.lifecycle_counts.get("VALID_ACTIVE"), Some(&17));
        assert_eq!(receipt.synthetic_review_candidate_count, 3);
        assert!(receipt.all_authenticity_unverified);
        assert!(receipt.all_actuation_disabled);
    }

    #[test]
    fn payload_tampering_without_digest_update_is_detected() {
        let mut value = packet();
        value.records[0].canonical_payload_json.push(' ');
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert_eq!(receipt.synthetic_review_candidate_count, 0);
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.starts_with("RECORD_DIGEST_MISMATCH")));
        assert!(
            !result_for(&receipt, "forest-known-family-complete-evidence")
                .synthetic_review_candidate
        );
    }

    #[test]
    fn binding_digest_mismatch_is_detected() {
        let mut value = packet();
        value.case_bindings[0].sha256 = "0".repeat(64);
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert_eq!(receipt.synthetic_review_candidate_count, 0);
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.starts_with("CASE_BINDING_DIGEST_MISMATCH")));
    }

    #[test]
    fn semantic_mutation_rehashed_still_fails_case_binding() {
        let mut value = packet();
        update_payload(
            &mut value,
            "synthetic://classification-review/forest-known-family-complete-evidence",
            |p| {
                p.criteria_regime = "unrelated-regime".to_string();
            },
        );
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert_eq!(receipt.synthetic_review_candidate_count, 0);
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.contains("RECORD_CASE_SEMANTIC_MISMATCH")));
    }

    #[test]
    fn taxonomy_mutation_rehashed_fails_semantic_and_scope_checks() {
        let mut value = packet();
        update_payload(
            &mut value,
            "synthetic://classification-review/forest-known-family-complete-evidence",
            |p| {
                p.taxonomy_version = "forest-hazard-taxonomy-v2".to_string();
            },
        );
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert_eq!(receipt.synthetic_review_candidate_count, 0);
        let result = result_for(&receipt, "forest-known-family-complete-evidence");
        assert_eq!(result.case_binding_status, "SEMANTIC_MISMATCH");
        assert!(!result.synthetic_review_candidate);
    }

    #[test]
    fn cross_case_record_replay_is_detected() {
        let mut value = packet();
        let a = "forest-known-family-complete-evidence";
        let b = "pv-known-family-complete-evidence";
        let a_binding = value
            .case_bindings
            .iter()
            .find(|x| x.case_id == a)
            .unwrap()
            .clone();
        let b_binding = value
            .case_bindings
            .iter()
            .find(|x| x.case_id == b)
            .unwrap()
            .clone();
        for binding in &mut value.case_bindings {
            if binding.case_id == a {
                binding.record_id = b_binding.record_id.clone();
                binding.sha256 = b_binding.sha256.clone();
            } else if binding.case_id == b {
                binding.record_id = a_binding.record_id.clone();
                binding.sha256 = a_binding.sha256.clone();
            }
        }
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert_eq!(receipt.synthetic_review_candidate_count, 0);
        assert!(!result_for(&receipt, a).synthetic_review_candidate);
        assert!(!result_for(&receipt, b).synthetic_review_candidate);
    }

    #[test]
    fn expired_record_is_rejected_even_with_consistent_digest() {
        let mut value = packet();
        let id = "synthetic://classification-review/forest-known-family-complete-evidence";
        update_payload(&mut value, id, |p| p.valid_until_day = 19999);
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert_eq!(receipt.synthetic_review_candidate_count, 0);
        assert_eq!(
            result_for(&receipt, "forest-known-family-complete-evidence").lifecycle_status,
            "EXPIRED"
        );
        assert!(
            !result_for(&receipt, "forest-known-family-complete-evidence")
                .synthetic_review_candidate
        );
    }

    #[test]
    fn revoked_record_is_rejected() {
        let mut value = packet();
        value.revocation_entries[0].status = "REVOKED".to_string();
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert_eq!(receipt.synthetic_review_candidate_count, 0);
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.starts_with("RECORD_REVOKED")));
    }

    #[test]
    fn unknown_revocation_state_fails_closed() {
        let mut value = packet();
        value.revocation_entries[0].status = "UNKNOWN".to_string();
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert_eq!(receipt.synthetic_review_candidate_count, 0);
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.starts_with("RECORD_REVOCATION_UNKNOWN")));
    }

    #[test]
    fn missing_binding_is_detected() {
        let mut value = packet();
        value
            .case_bindings
            .retain(|binding| binding.case_id != "forest-known-family-complete-evidence");
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert_eq!(receipt.synthetic_review_candidate_count, 0);
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.contains("CASE_BINDING_MISSING")));
    }

    #[test]
    fn duplicate_bindings_are_detected() {
        let mut value = packet();
        let duplicate = value.case_bindings[0].clone();
        value.case_bindings.push(duplicate);
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert_eq!(receipt.synthetic_review_candidate_count, 0);
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.starts_with("DUPLICATE_CASE_BINDING")));
    }

    #[test]
    fn duplicate_record_ids_are_detected() {
        let mut value = packet();
        let duplicate = value.records[0].clone();
        value.records.push(duplicate);
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert_eq!(receipt.synthetic_review_candidate_count, 0);
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.starts_with("DUPLICATE_RECORD_ID")));
    }

    #[test]
    fn orphan_record_is_detected() {
        let mut value = packet();
        let mut orphan = value.records[0].clone();
        let mut payload = payload_of(&orphan);
        payload.record_id = "synthetic://classification-review/orphan".to_string();
        payload.case_id = "orphan-case".to_string();
        orphan.record_id = payload.record_id.clone();
        orphan.canonical_payload_json = serde_json::to_string(&payload).unwrap();
        orphan.sha256 = sha256_hex(orphan.canonical_payload_json.as_bytes());
        value.records.push(orphan);
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert_eq!(receipt.synthetic_review_candidate_count, 0);
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.starts_with("ORPHAN_RECORD")));
    }

    #[test]
    fn invalid_authenticity_claim_fails_closed() {
        let mut value = packet();
        value.claim_ceiling.authenticity_verified = true;
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert_eq!(receipt.synthetic_review_candidate_count, 0);
        assert!(!receipt.all_authenticity_unverified);
    }

    #[test]
    fn payload_cannot_claim_verified_source_authenticity() {
        let mut value = packet();
        update_payload(
            &mut value,
            "synthetic://classification-review/forest-known-family-complete-evidence",
            |p| {
                p.source_authenticity_status = "AUTHENTICATED".to_string();
            },
        );
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert_eq!(receipt.synthetic_review_candidate_count, 0);
        assert!(!receipt.all_authenticity_unverified);
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.contains("RECORD_AUTHENTICITY_CEILING_VIOLATION")));
    }

    #[test]
    fn invalid_source_commit_fails_closed() {
        let receipt = audit(
            &packet(),
            &envelope(),
            Some("bad-sha".to_string()),
            Vec::new(),
        );
        assert_eq!(receipt.result, "fail");
        assert_eq!(receipt.synthetic_review_candidate_count, 0);
        assert!(receipt
            .errors
            .iter()
            .any(|error| error == "SOURCE_COMMIT_INVALID"));
    }

    #[test]
    fn row_order_does_not_change_canonical_packet_digest_or_results() {
        let original = packet();
        let mut reordered = original.clone();
        reordered.records.reverse();
        reordered.case_bindings.reverse();
        reordered.revocation_entries.reverse();
        let first = run(&original);
        let second = run(&reordered);
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
    fn unknown_profile_never_becomes_a_review_candidate() {
        let receipt = run(&packet());
        assert_eq!(
            result_for(&receipt, "unknown-profile-id").profile_scope_status,
            "OUT_OF_SCOPE"
        );
        assert!(!result_for(&receipt, "unknown-profile-id").synthetic_review_candidate);
    }

    #[test]
    fn every_case_result_is_actuation_disabled_and_authenticity_unverified() {
        let receipt = run(&packet());
        assert!(receipt.all_actuation_disabled);
        assert!(receipt.all_authenticity_unverified);
        assert!(receipt
            .case_results
            .iter()
            .all(|result| !result.actuation_authorized));
        assert!(receipt
            .case_results
            .iter()
            .all(|result| result.authenticity_status == "UNVERIFIED_SYNTHETIC"));
    }

    #[test]
    fn revocation_policy_must_match_the_record_policy() {
        let mut value = packet();
        value.revocation_entries[0].policy_id = "different-policy-v9".to_string();
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert_eq!(receipt.synthetic_review_candidate_count, 0);
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.starts_with("REVOCATION_POLICY_MISMATCH")));
    }

    #[test]
    fn arbitrary_rehashed_revocation_policy_is_rejected() {
        let mut value = packet();
        let id = "synthetic://classification-review/forest-known-family-complete-evidence";
        update_payload(&mut value, id, |p| {
            p.revocation_policy_id = "attacker-policy-v9".to_string();
        });
        value.revocation_entries[0].policy_id = "attacker-policy-v9".to_string();
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert_eq!(receipt.synthetic_review_candidate_count, 0);
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.starts_with("REVOCATION_POLICY_UNSUPPORTED")));
    }

    #[test]
    fn expected_candidate_label_cannot_override_a_block_gate() {
        let packet = packet();
        let envelope = envelope();
        let mut case = envelope
            .cases
            .iter()
            .find(|case| case.case_id == "forest-habitat-hard-gate-block")
            .expect("blocked case exists")
            .clone();
        case.expected_disposition = "REVIEW_ONLY_CANDIDATE".to_string();
        let mut errors = Vec::new();
        let result = evaluate_case(
            &case,
            &envelope.profiles,
            &packet.case_bindings,
            &packet.records,
            &packet.revocation_entries,
            packet.decision_day,
            &mut errors,
        );
        assert_eq!(result.hard_gate_status, "BLOCK");
        assert!(!result.synthetic_review_candidate);
    }

    #[test]
    fn expected_candidate_label_cannot_override_stale_evidence() {
        let packet = packet();
        let envelope = envelope();
        let mut case = envelope
            .cases
            .iter()
            .find(|case| case.case_id == "forest-stale-evidence")
            .expect("stale case exists")
            .clone();
        case.expected_disposition = "REVIEW_ONLY_CANDIDATE".to_string();
        let mut errors = Vec::new();
        let result = evaluate_case(
            &case,
            &envelope.profiles,
            &packet.case_bindings,
            &packet.records,
            &packet.revocation_entries,
            packet.decision_day,
            &mut errors,
        );
        assert!(!result.evidence_sufficient);
        assert!(!result.synthetic_review_candidate);
    }

    #[test]
    fn future_revocation_entry_is_invalid() {
        let mut value = packet();
        let future_day = value.decision_day + 1;
        value.revocation_entries[0].recorded_day = future_day;
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert_eq!(receipt.synthetic_review_candidate_count, 0);
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.starts_with("REVOCATION_ENTRY_FROM_FUTURE")));
    }
    #[test]
    fn missing_revocation_entry_is_rejected() {
        let mut value = packet();
        let record_id = value.records[0].record_id.clone();
        value
            .revocation_entries
            .retain(|entry| entry.record_id != record_id);
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert_eq!(receipt.synthetic_review_candidate_count, 0);
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.starts_with("REVOCATION_ENTRY_MISSING")));
    }

    #[test]
    fn duplicate_revocation_entries_are_rejected() {
        let mut value = packet();
        let duplicate = value.revocation_entries[0].clone();
        value.revocation_entries.push(duplicate);
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert_eq!(receipt.synthetic_review_candidate_count, 0);
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.starts_with("DUPLICATE_REVOCATION_ENTRY")));
    }

    #[test]
    fn invalid_validity_window_is_rejected_even_after_rehash() {
        let mut value = packet();
        let id = "synthetic://classification-review/forest-known-family-complete-evidence";
        update_payload(&mut value, id, |p| {
            p.valid_from_day = 20010;
            p.valid_until_day = 20005;
        });
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert_eq!(receipt.synthetic_review_candidate_count, 0);
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.starts_with("RECORD_VALIDITY_RANGE_INVALID")));
    }

    #[test]
    fn unsorted_set_fields_are_rejected_even_after_rehash() {
        let mut value = packet();
        let id = "synthetic://classification-review/forest-known-family-complete-evidence";
        update_payload(&mut value, id, |p| {
            p.classified_failure_families.reverse();
        });
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert_eq!(receipt.synthetic_review_candidate_count, 0);
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.starts_with("CLASSIFIED_FAMILIES_NOT_CANONICAL")));
    }
}
