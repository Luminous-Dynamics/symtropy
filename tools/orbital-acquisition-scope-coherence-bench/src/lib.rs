use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const CORPUS_SCHEMA: &str = "luminous.orbital-acquisition-scope-coherence.corpus.v0.1";
pub const RECEIPT_SCHEMA: &str = "luminous.orbital-acquisition-scope-coherence.receipt.v0.1";
pub const BENCHMARK_ID: &str = "ORBITAL-ACQUISITION-SCOPE-COHERENCE-001";
pub const CLAIM_CEILING: &str = "synthetic_exact_identity_and_admitted_interval_composition_only";

const CANONICAL_SCOPE_FIELDS: &[&str] = &[
    "acquisition_id",
    "camera_id",
    "sensor_mode_id",
    "intrinsics_revision",
    "robot_id",
    "mount_revision",
    "asset_id",
    "asset_revision",
    "camera_frame_id",
    "asset_frame_id",
    "transform_convention",
    "feature_id",
    "support_id",
    "motion_model_revision",
    "calibration_scope_id",
    "clock_map_id",
    "time_basis_id",
    "exposure_profile_id",
    "projector_profile_id",
    "pattern_sequence_id",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EvidenceKind {
    Motion,
    Support,
    Calibration,
    Timing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SourceState {
    Current,
    Stale,
    Revoked,
    Contradicted,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Disposition {
    ScopeCoherent,
    OutOfScope,
    Indeterminate,
    InvalidInput,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScopeClaim {
    pub field: String,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TimeInterval {
    pub time_basis_id: String,
    pub start_tai_ns: u64,
    pub end_tai_ns: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceBinding {
    pub evidence_id: String,
    pub kind: EvidenceKind,
    pub source_state: SourceState,
    pub receipt_sha256: String,
    pub validity_interval: TimeInterval,
    pub scope_claims: Vec<ScopeClaim>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub case_id: String,
    pub acquisition_interval: TimeInterval,
    pub acquisition_scope: Vec<ScopeClaim>,
    pub evidence: Vec<EvidenceBinding>,
    pub expected_disposition: Disposition,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    pub benchmark_id: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<Case>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BindingResult {
    pub evidence_id: String,
    pub kind: EvidenceKind,
    pub disposition: Disposition,
    pub missing_fields: Vec<String>,
    pub mismatched_fields: Vec<String>,
    pub reason_codes: Vec<String>,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub disposition: Disposition,
    pub checked_binding_count: usize,
    pub matched_binding_count: usize,
    pub missing_evidence_kinds: Vec<EvidenceKind>,
    pub binding_results: Vec<BindingResult>,
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Receipt {
    pub schema: String,
    pub benchmark_id: String,
    pub source_commit: String,
    pub input_sha256: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub case_count: usize,
    pub expected_disposition_matches: usize,
    pub expected_disposition_mismatches: usize,
    pub source_bytes_authenticated: bool,
    pub authority_granted: bool,
    pub physical_validation: String,
    pub cases: Vec<CaseResult>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn valid_text(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn required_fields(kind: EvidenceKind) -> &'static [&'static str] {
    match kind {
        EvidenceKind::Motion => &[
            "acquisition_id",
            "camera_id",
            "sensor_mode_id",
            "robot_id",
            "mount_revision",
            "asset_id",
            "asset_revision",
            "camera_frame_id",
            "asset_frame_id",
            "transform_convention",
            "feature_id",
            "support_id",
            "motion_model_revision",
            "clock_map_id",
            "time_basis_id",
        ],
        EvidenceKind::Support => &[
            "asset_id",
            "asset_revision",
            "camera_frame_id",
            "asset_frame_id",
            "transform_convention",
            "feature_id",
            "support_id",
        ],
        EvidenceKind::Calibration => &[
            "camera_id",
            "sensor_mode_id",
            "intrinsics_revision",
            "robot_id",
            "mount_revision",
            "asset_id",
            "asset_revision",
            "camera_frame_id",
            "asset_frame_id",
            "calibration_scope_id",
            "clock_map_id",
            "time_basis_id",
        ],
        EvidenceKind::Timing => &[
            "acquisition_id",
            "camera_id",
            "sensor_mode_id",
            "clock_map_id",
            "time_basis_id",
            "exposure_profile_id",
            "projector_profile_id",
            "pattern_sequence_id",
        ],
    }
}

fn claims_to_map(claims: &[ScopeClaim]) -> Result<BTreeMap<String, String>, String> {
    let mut values = BTreeMap::new();
    for claim in claims {
        if !valid_text(&claim.field) || !valid_text(&claim.value) {
            return Err("SCOPE_CLAIM_EMPTY".to_owned());
        }
        if !CANONICAL_SCOPE_FIELDS.contains(&claim.field.as_str()) {
            return Err(format!("SCOPE_FIELD_UNSUPPORTED:{}", claim.field));
        }
        if values.insert(claim.field.clone(), claim.value.clone()).is_some() {
            return Err(format!("SCOPE_FIELD_DUPLICATE:{}", claim.field));
        }
    }
    Ok(values)
}

fn validate_interval(interval: &TimeInterval) -> Result<(), String> {
    if !valid_text(&interval.time_basis_id) || interval.start_tai_ns >= interval.end_tai_ns {
        return Err("TIME_INTERVAL_INVALID".to_owned());
    }
    Ok(())
}

fn evaluate_binding(
    binding: &EvidenceBinding,
    acquisition_interval: &TimeInterval,
    acquisition_scope: &BTreeMap<String, String>,
) -> BindingResult {
    let mut reason_codes = BTreeSet::new();
    let mut missing_fields = BTreeSet::new();
    let mut mismatched_fields = BTreeSet::new();
    let mut validity_boundary_indeterminate = false;

    if !valid_text(&binding.evidence_id) || !valid_sha256(&binding.receipt_sha256) {
        reason_codes.insert("BINDING_ID_OR_RECEIPT_INVALID".to_owned());
        return BindingResult {
            evidence_id: binding.evidence_id.clone(),
            kind: binding.kind,
            disposition: Disposition::InvalidInput,
            missing_fields: Vec::new(),
            mismatched_fields: Vec::new(),
            reason_codes: reason_codes.into_iter().collect(),
            receipt_sha256: binding.receipt_sha256.clone(),
        };
    }
    if let Err(reason) = validate_interval(&binding.validity_interval) {
        reason_codes.insert(reason);
        return BindingResult {
            evidence_id: binding.evidence_id.clone(),
            kind: binding.kind,
            disposition: Disposition::InvalidInput,
            missing_fields: Vec::new(),
            mismatched_fields: Vec::new(),
            reason_codes: reason_codes.into_iter().collect(),
            receipt_sha256: binding.receipt_sha256.clone(),
        };
    }

    let evidence_scope = match claims_to_map(&binding.scope_claims) {
        Ok(scope) => scope,
        Err(reason) => {
            reason_codes.insert(reason);
            return BindingResult {
                evidence_id: binding.evidence_id.clone(),
                kind: binding.kind,
                disposition: Disposition::InvalidInput,
                missing_fields: Vec::new(),
                mismatched_fields: Vec::new(),
                reason_codes: reason_codes.into_iter().collect(),
                receipt_sha256: binding.receipt_sha256.clone(),
            };
        }
    };

    for field in required_fields(binding.kind) {
        let Some(expected) = acquisition_scope.get(*field) else {
            missing_fields.insert((*field).to_owned());
            reason_codes.insert(format!("ACQUISITION_SCOPE_FIELD_MISSING:{field}"));
            continue;
        };
        match evidence_scope.get(*field) {
            Some(actual) if actual == expected => {}
            Some(_) => {
                mismatched_fields.insert((*field).to_owned());
                reason_codes.insert(format!("SCOPE_MISMATCH:{field}"));
            }
            None => {
                missing_fields.insert((*field).to_owned());
                reason_codes.insert(format!("EVIDENCE_SCOPE_FIELD_MISSING:{field}"));
            }
        }
    }

    // An owner may assert optional scope dimensions beyond its minimum contract.
    // Never ignore an extra assertion: every supplied identity must still match
    // the canonical acquisition descriptor.
    for (field, actual) in &evidence_scope {
        if required_fields(binding.kind).contains(&field.as_str()) {
            continue;
        }
        match acquisition_scope.get(field) {
            Some(expected) if expected == actual => {}
            Some(_) => {
                mismatched_fields.insert(field.clone());
                reason_codes.insert(format!("SCOPE_MISMATCH:{field}"));
            }
            None => {
                missing_fields.insert(field.clone());
                reason_codes.insert(format!("ACQUISITION_SCOPE_FIELD_MISSING:{field}"));
            }
        }
    }

    if evidence_scope
        .get("time_basis_id")
        .is_some_and(|value| value != &acquisition_interval.time_basis_id)
        || binding.validity_interval.time_basis_id != acquisition_interval.time_basis_id
    {
        mismatched_fields.insert("time_basis_id".to_owned());
        reason_codes.insert("TIME_BASIS_MISMATCH".to_owned());
    }

    // All validity intervals are half-open [start, end). Require the complete
    // acquisition interval to be strictly contained; equality at the right
    // boundary is not silently promoted to valid.
    if acquisition_interval.end_tai_ns < binding.validity_interval.start_tai_ns
        || acquisition_interval.start_tai_ns >= binding.validity_interval.end_tai_ns
    {
        mismatched_fields.insert("validity_interval".to_owned());
        reason_codes.insert("EVIDENCE_VALIDITY_DISJOINT".to_owned());
    } else if acquisition_interval.start_tai_ns < binding.validity_interval.start_tai_ns
        || acquisition_interval.end_tai_ns >= binding.validity_interval.end_tai_ns
    {
        validity_boundary_indeterminate = true;
        reason_codes.insert("EVIDENCE_VALIDITY_BOUNDARY_OVERLAP".to_owned());
    }

    if binding.source_state != SourceState::Current {
        reason_codes.insert(format!("SOURCE_NOT_CURRENT:{:?}", binding.source_state).to_ascii_uppercase());
    }

    let disposition = if !missing_fields.is_empty() && mismatched_fields.is_empty()
        && binding.source_state == SourceState::Current
    {
        Disposition::Indeterminate
    } else if !mismatched_fields.is_empty() {
        Disposition::OutOfScope
    } else if !missing_fields.is_empty()
        || validity_boundary_indeterminate
        || binding.source_state != SourceState::Current
    {
        Disposition::Indeterminate
    } else {
        Disposition::ScopeCoherent
    };

    BindingResult {
        evidence_id: binding.evidence_id.clone(),
        kind: binding.kind,
        disposition,
        missing_fields: missing_fields.into_iter().collect(),
        mismatched_fields: mismatched_fields.into_iter().collect(),
        reason_codes: reason_codes.into_iter().collect(),
        receipt_sha256: binding.receipt_sha256.clone(),
    }
}

fn evaluate_case(case: &Case) -> CaseResult {
    let invalid = |reason: String| CaseResult {
        case_id: case.case_id.clone(),
        disposition: Disposition::InvalidInput,
        checked_binding_count: 0,
        matched_binding_count: 0,
        missing_evidence_kinds: Vec::new(),
        binding_results: Vec::new(),
        reason_codes: vec![reason],
    };

    if !valid_text(&case.case_id) {
        return invalid("CASE_ID_EMPTY".to_owned());
    }
    if let Err(reason) = validate_interval(&case.acquisition_interval) {
        return invalid(reason);
    }

    let acquisition_scope = match claims_to_map(&case.acquisition_scope) {
        Ok(scope) => scope,
        Err(reason) => return invalid(reason),
    };
    let mut acquisition_missing = BTreeSet::new();
    for field in CANONICAL_SCOPE_FIELDS {
        if !acquisition_scope.contains_key(*field) {
            acquisition_missing.insert((*field).to_owned());
        }
    }

    let mut evidence_ids = BTreeSet::new();
    let mut evidence_kinds = BTreeSet::new();
    for binding in &case.evidence {
        if !evidence_ids.insert(binding.evidence_id.clone()) {
            return invalid("EVIDENCE_ID_DUPLICATE".to_owned());
        }
        if !evidence_kinds.insert(binding.kind) {
            return invalid(format!("EVIDENCE_KIND_DUPLICATE:{:?}", binding.kind));
        }
    }

    let required_kinds = [
        EvidenceKind::Motion,
        EvidenceKind::Support,
        EvidenceKind::Calibration,
        EvidenceKind::Timing,
    ];
    let missing_evidence_kinds: Vec<EvidenceKind> = required_kinds
        .iter()
        .copied()
        .filter(|kind| !evidence_kinds.contains(kind))
        .collect();

    let mut binding_results: Vec<BindingResult> = case
        .evidence
        .iter()
        .map(|binding| {
            evaluate_binding(binding, &case.acquisition_interval, &acquisition_scope)
        })
        .collect();
    binding_results.sort_by(|left, right| left.evidence_id.cmp(&right.evidence_id));

    let mut reasons = BTreeSet::new();
    for field in &acquisition_missing {
        reasons.insert(format!("ACQUISITION_SCOPE_FIELD_MISSING:{field}"));
    }
    for kind in &missing_evidence_kinds {
        reasons.insert(format!("REQUIRED_EVIDENCE_MISSING:{kind:?}").to_ascii_uppercase());
    }
    for binding in &binding_results {
        reasons.extend(binding.reason_codes.iter().cloned());
    }

    let has_invalid = binding_results
        .iter()
        .any(|binding| binding.disposition == Disposition::InvalidInput);
    let has_out_of_scope = binding_results
        .iter()
        .any(|binding| binding.disposition == Disposition::OutOfScope);
    let has_indeterminate = binding_results
        .iter()
        .any(|binding| binding.disposition == Disposition::Indeterminate);

    let disposition = if has_invalid {
        Disposition::InvalidInput
    } else if has_out_of_scope {
        Disposition::OutOfScope
    } else if has_indeterminate || !acquisition_missing.is_empty() || !missing_evidence_kinds.is_empty() {
        Disposition::Indeterminate
    } else {
        Disposition::ScopeCoherent
    };

    let matched_binding_count = binding_results
        .iter()
        .filter(|binding| binding.disposition == Disposition::ScopeCoherent)
        .count();

    CaseResult {
        case_id: case.case_id.clone(),
        disposition,
        checked_binding_count: binding_results.len(),
        matched_binding_count,
        missing_evidence_kinds,
        binding_results,
        reason_codes: reasons.into_iter().collect(),
    }
}

pub fn audit_corpus(bytes: &[u8], source_commit: &str) -> Result<Receipt, String> {
    if source_commit.len() != 40 || !source_commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS".to_owned());
    }
    let corpus: Corpus =
        serde_json::from_slice(bytes).map_err(|error| format!("CORPUS_JSON_INVALID: {error}"))?;
    if corpus.schema != CORPUS_SCHEMA
        || corpus.benchmark_id != BENCHMARK_ID
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != CLAIM_CEILING
        || corpus.cases.is_empty()
        || corpus.cases.len() > 128
    {
        return Err("CORPUS_IDENTITY_OR_CLAIM_CEILING_INVALID".to_owned());
    }

    let mut case_ids = BTreeSet::new();
    let mut cases = Vec::new();
    let mut expected_disposition_matches = 0_usize;
    for case in &corpus.cases {
        if !case_ids.insert(&case.case_id) {
            return Err("CASE_ID_DUPLICATE".to_owned());
        }
        let result = evaluate_case(case);
        if result.disposition == case.expected_disposition {
            expected_disposition_matches += 1;
        }
        cases.push(result);
    }
    cases.sort_by(|left, right| left.case_id.cmp(&right.case_id));

    Ok(Receipt {
        schema: RECEIPT_SCHEMA.to_owned(),
        benchmark_id: BENCHMARK_ID.to_owned(),
        source_commit: source_commit.to_ascii_lowercase(),
        input_sha256: sha256_hex(bytes),
        evidence_class: corpus.evidence_class,
        claim_ceiling: corpus.claim_ceiling,
        case_count: cases.len(),
        expected_disposition_matches,
        expected_disposition_mismatches: cases.len() - expected_disposition_matches,
        source_bytes_authenticated: false,
        authority_granted: false,
        physical_validation: "NOT_ASSESSED".to_owned(),
        cases,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus_bytes() -> &'static [u8] {
        include_bytes!("../../../docs/research/orbital-truss/fixtures/acquisition-scope-coherence-v0_1.json")
    }

    fn corpus() -> Corpus {
        serde_json::from_slice(corpus_bytes()).expect("fixture parses")
    }

    #[test]
    fn frozen_scope_corpus_matches_all_expected_dispositions() {
        let receipt = audit_corpus(
            corpus_bytes(),
            "0123456789abcdef0123456789abcdef01234567",
        )
        .expect("scope corpus audits");
        assert_eq!(receipt.case_count, 24);
        assert_eq!(receipt.expected_disposition_matches, 24);
        assert_eq!(receipt.expected_disposition_mismatches, 0);
        assert!(!receipt.source_bytes_authenticated);
        assert!(!receipt.authority_granted);
    }

    #[test]
    fn coherent_scope_requires_all_four_owner_bindings() {
        let input = corpus();
        let case = input
            .cases
            .iter()
            .find(|case| case.case_id == "SC-001-ALL-MATCH")
            .unwrap();
        let result = evaluate_case(case);
        assert_eq!(result.disposition, Disposition::ScopeCoherent);
        assert_eq!(result.matched_binding_count, 4);
    }

    #[test]
    fn changed_camera_asset_frame_or_feature_is_out_of_scope() {
        let input = corpus();
        for id in [
            "SC-002-CAMERA-MISMATCH",
            "SC-005-ASSET-REVISION-MISMATCH",
            "SC-006-FRAME-MISMATCH",
            "SC-008-FEATURE-MISMATCH",
        ] {
            let case = input.cases.iter().find(|case| case.case_id == id).unwrap();
            assert_eq!(evaluate_case(case).disposition, Disposition::OutOfScope);
        }
    }

    #[test]
    fn extra_scope_claims_are_checked_not_ignored() {
        let input = corpus();
        let case = input
            .cases
            .iter()
            .find(|case| case.case_id == "SC-024-EXTRA-SCOPE-CLAIM-MISMATCH")
            .unwrap();
        let result = evaluate_case(case);
        assert_eq!(result.disposition, Disposition::OutOfScope);
        assert!(result.reason_codes.contains(&"SCOPE_MISMATCH:camera_id".to_owned()));
    }

    #[test]
    fn clock_map_mismatch_never_compares_unaligned_raw_clocks() {
        let input = corpus();
        let case = input
            .cases
            .iter()
            .find(|case| case.case_id == "SC-011-CLOCK-MAP-MISMATCH")
            .unwrap();
        let result = evaluate_case(case);
        assert_eq!(result.disposition, Disposition::OutOfScope);
        assert!(result.reason_codes.contains(&"SCOPE_MISMATCH:clock_map_id".to_owned())
            || result.reason_codes.contains(&"TIME_BASIS_MISMATCH".to_owned()));
    }

    #[test]
    fn half_open_validity_boundary_is_not_promoted() {
        let input = corpus();
        let case = input
            .cases
            .iter()
            .find(|case| case.case_id == "SC-016-TOUCHES-VALID-UNTIL")
            .unwrap();
        let result = evaluate_case(case);
        assert_eq!(result.disposition, Disposition::OutOfScope);
        assert!(result.reason_codes.contains(&"EVIDENCE_VALIDITY_DOES_NOT_COVER_ACQUISITION".to_owned()));
    }

    #[test]
    fn missing_scope_or_unknown_source_is_indeterminate() {
        let input = corpus();
        for id in [
            "SC-017-MISSING-SCOPE",
            "SC-021-REVOKED-SOURCE",
        ] {
            let case = input.cases.iter().find(|case| case.case_id == id).unwrap();
            assert_eq!(evaluate_case(case).disposition, Disposition::Indeterminate);
        }
    }

    #[test]
    fn missing_required_evidence_owner_is_indeterminate() {
        let input = corpus();
        let case = input
            .cases
            .iter()
            .find(|case| case.case_id == "SC-023-MISSING-TIMING-EVIDENCE")
            .unwrap();
        let result = evaluate_case(case);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert_eq!(result.missing_evidence_kinds, vec![EvidenceKind::Timing]);
    }

    #[test]
    fn malformed_and_duplicate_bindings_are_invalid() {
        let input = corpus();
        for id in [
            "SC-019-DUPLICATE-SCOPE-FIELD",
            "SC-020-SOURCE-DIGEST-SHAPE-INVALID",
        ] {
            let case = input.cases.iter().find(|case| case.case_id == id).unwrap();
            assert_eq!(evaluate_case(case).disposition, Disposition::InvalidInput);
        }
    }

    #[test]
    fn input_order_and_claim_order_do_not_change_canonical_case_result() {
        let input = corpus();
        let source = "0123456789abcdef0123456789abcdef01234567";
        let original = audit_corpus(corpus_bytes(), source).unwrap();
        let mut permuted = input;
        permuted.cases.reverse();
        for case in &mut permuted.cases {
            case.acquisition_scope.reverse();
            case.evidence.reverse();
            for binding in &mut case.evidence {
                binding.scope_claims.reverse();
            }
        }
        let bytes = serde_json::to_vec(&permuted).unwrap();
        let replay = audit_corpus(&bytes, source).unwrap();
        assert_eq!(original.cases, replay.cases);
    }

    #[test]
    fn digest_shape_is_not_authentication_or_authority() {
        let input = corpus();
        let case = input.cases.iter().find(|case| case.case_id == "SC-022-DIGEST-NOT-AUTH").unwrap();
        assert_eq!(evaluate_case(case).disposition, Disposition::ScopeCoherent);
        let receipt = audit_corpus(corpus_bytes(), "0123456789abcdef0123456789abcdef01234567").unwrap();
        assert!(!receipt.source_bytes_authenticated);
        assert!(!receipt.authority_granted);
    }
}
