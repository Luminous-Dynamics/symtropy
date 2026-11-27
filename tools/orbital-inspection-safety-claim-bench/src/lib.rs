use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const CORPUS_SCHEMA: &str = "luminous.orbital-inspection-safety-claim.corpus.v0.1";
pub const RECEIPT_SCHEMA: &str = "luminous.orbital-inspection-safety-claim.receipt.v0.1";
pub const BENCHMARK_ID: &str = "ORBITAL-INSPECTION-SAFETY-CLAIM-001";
pub const CLAIM_CEILING: &str = "synthetic_safety_claim_scope_and_component_composition_only";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClaimComponent {
    FiniteHorizonKeepOut,
    SafeContinuation,
    StatisticalRisk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AssuranceClass {
    RobustTrajectoryEnclosure,
    PassiveInvariantSet,
    AbortReachableSet,
    ProbabilisticRiskBound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SourceState {
    Pass,
    Fail,
    Unknown,
    Stale,
    Revoked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Disposition {
    SafetyClaimsComplete,
    SafetyClaimFailed,
    SafetyClaimIncomplete,
    RiskOnly,
    OutOfScope,
    Indeterminate,
    InvalidInput,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ActionScope {
    pub action_id: String,
    pub action_revision: String,
    pub vehicle_id: String,
    pub vehicle_config_revision: String,
    pub trajectory_revision: String,
    pub horizon_id: String,
    pub protected_object_id: String,
    pub protected_object_revision: String,
    pub keepout_profile_id: String,
    pub keepout_geometry_revision: String,
    pub reference_frame_id: String,
    pub frame_revision: String,
    pub transform_convention: String,
    pub dynamics_revision: String,
    pub initial_state_revision: String,
    pub uncertainty_set_revision: String,
    pub time_basis_id: String,
    pub action_start_tai_ns: u64,
    pub action_end_tai_ns: u64,
    pub required_clearance_um: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ValidityInterval {
    pub time_basis_id: String,
    pub start_tai_ns: u64,
    pub end_tai_ns: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SafetyClaim {
    pub evidence_id: String,
    pub component: ClaimComponent,
    pub assurance_class: AssuranceClass,
    pub state: SourceState,
    pub scope: ActionScope,
    pub validity: ValidityInterval,
    pub receipt_sha256: String,
    pub temporal_enclosure_complete: bool,
    pub minimum_clearance_lower_um: Option<u64>,
    pub required_clearance_um: Option<u64>,
    pub terminal_set_id: Option<String>,
    pub terminal_set_revision: Option<String>,
    pub terminal_epoch_tai_ns: Option<u64>,
    pub terminal_invariant_proof_sha256: Option<String>,
    pub risk_measure: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub case_id: String,
    pub action_scope: ActionScope,
    pub claims: Vec<SafetyClaim>,
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
pub struct ClaimResult {
    pub evidence_id: String,
    pub component: ClaimComponent,
    pub assurance_class: AssuranceClass,
    pub disposition: Disposition,
    pub reason_codes: Vec<String>,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub disposition: Disposition,
    pub finite_horizon_claim_present: bool,
    pub safe_continuation_claim_present: bool,
    pub risk_only_claim_present: bool,
    pub claim_results: Vec<ClaimResult>,
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
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn valid_text(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_scope(scope: &ActionScope) -> Result<(), String> {
    let values = [
        &scope.action_id,
        &scope.action_revision,
        &scope.vehicle_id,
        &scope.vehicle_config_revision,
        &scope.trajectory_revision,
        &scope.horizon_id,
        &scope.protected_object_id,
        &scope.protected_object_revision,
        &scope.keepout_profile_id,
        &scope.keepout_geometry_revision,
        &scope.reference_frame_id,
        &scope.frame_revision,
        &scope.transform_convention,
        &scope.dynamics_revision,
        &scope.initial_state_revision,
        &scope.uncertainty_set_revision,
        &scope.time_basis_id,
    ];
    if values.iter().any(|value| !valid_text(value)) {
        return Err("ACTION_SCOPE_FIELD_EMPTY".to_owned());
    }
    if scope.action_start_tai_ns >= scope.action_end_tai_ns {
        return Err("ACTION_INTERVAL_INVALID".to_owned());
    }
    if scope.required_clearance_um == 0 {
        return Err("REQUIRED_CLEARANCE_MUST_BE_POSITIVE".to_owned());
    }
    Ok(())
}

fn validate_validity(interval: &ValidityInterval) -> Result<(), String> {
    if !valid_text(&interval.time_basis_id) || interval.start_tai_ns >= interval.end_tai_ns {
        return Err("CLAIM_VALIDITY_INTERVAL_INVALID".to_owned());
    }
    Ok(())
}

fn same_scope(expected: &ActionScope, actual: &ActionScope) -> bool {
    expected == actual
}

fn validate_claim_shape(claim: &SafetyClaim) -> Result<(), String> {
    if !valid_text(&claim.evidence_id) || !valid_sha256(&claim.receipt_sha256) {
        return Err("CLAIM_ID_OR_RECEIPT_INVALID".to_owned());
    }
    validate_scope(&claim.scope)?;
    validate_validity(&claim.validity)?;

    match claim.component {
        ClaimComponent::FiniteHorizonKeepOut => {
            if claim.assurance_class != AssuranceClass::RobustTrajectoryEnclosure
                || !claim.temporal_enclosure_complete
                || claim.minimum_clearance_lower_um.is_none()
                || claim.required_clearance_um != Some(claim.scope.required_clearance_um)
                || claim.terminal_set_id.is_some()
                || claim.terminal_set_revision.is_some()
                || claim.terminal_epoch_tai_ns.is_some()
                || claim.terminal_invariant_proof_sha256.is_some()
                || claim.risk_measure.is_some()
            {
                return Err("FINITE_HORIZON_CLAIM_SHAPE_INVALID".to_owned());
            }
        }
        ClaimComponent::SafeContinuation => {
            if !matches!(
                claim.assurance_class,
                AssuranceClass::PassiveInvariantSet | AssuranceClass::AbortReachableSet
            ) || !claim.temporal_enclosure_complete
                || !claim.terminal_set_id.as_deref().is_some_and(valid_text)
                || !claim
                    .terminal_set_revision
                    .as_deref()
                    .is_some_and(valid_text)
                || claim.terminal_epoch_tai_ns.is_none()
                || !claim
                    .terminal_invariant_proof_sha256
                    .as_deref()
                    .is_some_and(valid_sha256)
                || claim.minimum_clearance_lower_um.is_some()
                || claim.required_clearance_um.is_some()
                || claim.risk_measure.is_some()
            {
                return Err("SAFE_CONTINUATION_CLAIM_SHAPE_INVALID".to_owned());
            }
        }
        ClaimComponent::StatisticalRisk => {
            if claim.assurance_class != AssuranceClass::ProbabilisticRiskBound
                || !claim.risk_measure.as_deref().is_some_and(valid_text)
                || claim.minimum_clearance_lower_um.is_some()
                || claim.required_clearance_um.is_some()
                || claim.terminal_set_id.is_some()
                || claim.terminal_set_revision.is_some()
                || claim.terminal_epoch_tai_ns.is_some()
                || claim.terminal_invariant_proof_sha256.is_some()
            {
                return Err("STATISTICAL_RISK_CLAIM_SHAPE_INVALID".to_owned());
            }
        }
    }
    Ok(())
}

fn evaluate_claim(claim: &SafetyClaim, action: &ActionScope) -> ClaimResult {
    let result = |disposition, reason_codes: Vec<String>| ClaimResult {
        evidence_id: claim.evidence_id.clone(),
        component: claim.component,
        assurance_class: claim.assurance_class,
        disposition,
        reason_codes,
        receipt_sha256: claim.receipt_sha256.clone(),
    };

    if let Err(reason) = validate_claim_shape(claim) {
        return result(Disposition::InvalidInput, vec![reason]);
    }
    if !same_scope(action, &claim.scope) {
        return result(
            Disposition::OutOfScope,
            vec!["ACTION_SCOPE_MISMATCH".to_owned()],
        );
    }
    if claim.validity.time_basis_id != action.time_basis_id {
        return result(
            Disposition::OutOfScope,
            vec!["TIME_BASIS_MISMATCH".to_owned()],
        );
    }

    match claim.component {
        ClaimComponent::FiniteHorizonKeepOut => {
            if claim.validity.start_tai_ns > action.action_start_tai_ns
                || claim.validity.end_tai_ns < action.action_end_tai_ns
            {
                return result(
                    Disposition::OutOfScope,
                    vec!["KEEP_OUT_CLAIM_DOES_NOT_COVER_ACTION_HORIZON".to_owned()],
                );
            }
            let lower = claim.minimum_clearance_lower_um.unwrap();
            let required = claim.required_clearance_um.unwrap();
            if lower < required {
                return result(
                    Disposition::SafetyClaimFailed,
                    vec!["ROBUST_CLEARANCE_BELOW_REQUIRED_MARGIN".to_owned()],
                );
            }
        }
        ClaimComponent::SafeContinuation => {
            if claim.terminal_epoch_tai_ns != Some(action.action_end_tai_ns)
                || claim.validity.start_tai_ns > action.action_end_tai_ns
                || claim.validity.end_tai_ns <= action.action_end_tai_ns
            {
                return result(
                    Disposition::OutOfScope,
                    vec!["TERMINAL_CLAIM_NOT_BOUND_TO_ACTION_END".to_owned()],
                );
            }
        }
        ClaimComponent::StatisticalRisk => {
            if claim.validity.start_tai_ns > action.action_start_tai_ns
                || claim.validity.end_tai_ns < action.action_end_tai_ns
            {
                return result(
                    Disposition::OutOfScope,
                    vec!["RISK_CLAIM_DOES_NOT_COVER_ACTION_HORIZON".to_owned()],
                );
            }
            return match claim.state {
                SourceState::Pass => result(
                    Disposition::RiskOnly,
                    vec!["STATISTICAL_RISK_IS_NOT_A_DETERMINISTIC_SHIELD".to_owned()],
                ),
                SourceState::Fail => result(
                    Disposition::SafetyClaimFailed,
                    vec!["STATISTICAL_RISK_GATE_FAIL".to_owned()],
                ),
                SourceState::Unknown | SourceState::Stale | SourceState::Revoked => result(
                    Disposition::Indeterminate,
                    vec!["STATISTICAL_RISK_SOURCE_UNRESOLVED".to_owned()],
                ),
            };
        }
    }

    match claim.state {
        SourceState::Pass => result(Disposition::SafetyClaimsComplete, Vec::new()),
        SourceState::Fail => result(
            Disposition::SafetyClaimFailed,
            vec!["SAFETY_COMPONENT_FAIL".to_owned()],
        ),
        SourceState::Unknown => result(
            Disposition::Indeterminate,
            vec!["SAFETY_COMPONENT_UNKNOWN".to_owned()],
        ),
        SourceState::Stale => result(
            Disposition::Indeterminate,
            vec!["SAFETY_COMPONENT_STALE".to_owned()],
        ),
        SourceState::Revoked => result(
            Disposition::Indeterminate,
            vec!["SAFETY_COMPONENT_REVOKED".to_owned()],
        ),
    }
}

fn evaluate_case(case: &Case) -> CaseResult {
    let invalid = |reason: String| CaseResult {
        case_id: case.case_id.clone(),
        disposition: Disposition::InvalidInput,
        finite_horizon_claim_present: false,
        safe_continuation_claim_present: false,
        risk_only_claim_present: false,
        claim_results: Vec::new(),
        reason_codes: vec![reason],
    };

    if !valid_text(&case.case_id) {
        return invalid("CASE_ID_EMPTY".to_owned());
    }
    if let Err(reason) = validate_scope(&case.action_scope) {
        return invalid(reason);
    }

    let mut evidence_ids = BTreeSet::new();
    let mut components = BTreeSet::new();
    for claim in &case.claims {
        if !evidence_ids.insert(claim.evidence_id.clone()) {
            return invalid("EVIDENCE_ID_DUPLICATE".to_owned());
        }
        if !components.insert(claim.component) {
            return invalid(format!("CLAIM_COMPONENT_DUPLICATE:{:?}", claim.component));
        }
    }

    let mut claim_results: Vec<ClaimResult> = case
        .claims
        .iter()
        .map(|claim| evaluate_claim(claim, &case.action_scope))
        .collect();
    claim_results.sort_by(|left, right| left.evidence_id.cmp(&right.evidence_id));

    let finite_present = components.contains(&ClaimComponent::FiniteHorizonKeepOut);
    let continuation_present = components.contains(&ClaimComponent::SafeContinuation);
    let risk_present = components.contains(&ClaimComponent::StatisticalRisk);
    let mut reasons = BTreeSet::new();
    for claim in &claim_results {
        reasons.extend(claim.reason_codes.iter().cloned());
    }

    let has_invalid = claim_results
        .iter()
        .any(|claim| claim.disposition == Disposition::InvalidInput);
    let has_out_of_scope = claim_results
        .iter()
        .any(|claim| claim.disposition == Disposition::OutOfScope);
    let has_failed = claim_results
        .iter()
        .any(|claim| claim.disposition == Disposition::SafetyClaimFailed);
    let has_indeterminate = claim_results
        .iter()
        .any(|claim| claim.disposition == Disposition::Indeterminate);

    let disposition = if has_invalid {
        Disposition::InvalidInput
    } else if has_out_of_scope {
        Disposition::OutOfScope
    } else if has_failed {
        Disposition::SafetyClaimFailed
    } else if has_indeterminate {
        Disposition::Indeterminate
    } else if finite_present && continuation_present {
        Disposition::SafetyClaimsComplete
    } else if risk_present && !finite_present && !continuation_present {
        Disposition::RiskOnly
    } else {
        Disposition::SafetyClaimIncomplete
    };

    if !finite_present {
        reasons.insert("FINITE_HORIZON_KEEP_OUT_CLAIM_MISSING".to_owned());
    }
    if !continuation_present {
        reasons.insert("SAFE_CONTINUATION_CLAIM_MISSING".to_owned());
    }

    CaseResult {
        case_id: case.case_id.clone(),
        disposition,
        finite_horizon_claim_present: finite_present,
        safe_continuation_claim_present: continuation_present,
        risk_only_claim_present: risk_present,
        claim_results,
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

    let mut ids = BTreeSet::new();
    let mut cases = Vec::new();
    let mut matches = 0_usize;
    for case in &corpus.cases {
        if !ids.insert(&case.case_id) {
            return Err("CASE_ID_DUPLICATE".to_owned());
        }
        let result = evaluate_case(case);
        if result.disposition == case.expected_disposition {
            matches += 1;
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
        expected_disposition_matches: matches,
        expected_disposition_mismatches: cases.len() - matches,
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
        include_bytes!(
            "../../../docs/research/orbital-truss/fixtures/inspection-safety-claim-v0_1.json"
        )
    }

    fn corpus() -> Corpus {
        serde_json::from_slice(corpus_bytes()).expect("fixture parses")
    }

    #[test]
    fn frozen_safety_claim_corpus_matches_expected_dispositions() {
        let receipt = audit_corpus(corpus_bytes(), "0123456789abcdef0123456789abcdef01234567")
            .expect("safety corpus audits");
        assert_eq!(receipt.case_count, 26);
        assert_eq!(receipt.expected_disposition_matches, 26);
        assert_eq!(receipt.expected_disposition_mismatches, 0);
        assert!(!receipt.source_bytes_authenticated);
        assert!(!receipt.authority_granted);
    }

    #[test]
    fn complete_composite_requires_both_independent_components() {
        let input = corpus();
        let case = input
            .cases
            .iter()
            .find(|case| case.case_id == "SA-001-COMPOSITE-PASS")
            .unwrap();
        assert_eq!(
            evaluate_case(case).disposition,
            Disposition::SafetyClaimsComplete
        );
    }

    #[test]
    fn finite_horizon_and_terminal_safety_do_not_substitute_for_each_other() {
        let input = corpus();
        let finite_only = input
            .cases
            .iter()
            .find(|case| case.case_id == "SA-002-FINITE-ONLY")
            .unwrap();
        let terminal_only = input
            .cases
            .iter()
            .find(|case| case.case_id == "SA-003-TERMINAL-ONLY")
            .unwrap();
        assert_eq!(
            evaluate_case(finite_only).disposition,
            Disposition::SafetyClaimIncomplete
        );
        assert_eq!(
            evaluate_case(terminal_only).disposition,
            Disposition::SafetyClaimIncomplete
        );
    }

    #[test]
    fn statistical_risk_is_never_a_deterministic_shield() {
        let input = corpus();
        for id in ["SA-014-PC-ONLY", "SA-015-NC-ONLY"] {
            let case = input.cases.iter().find(|case| case.case_id == id).unwrap();
            assert_eq!(evaluate_case(case).disposition, Disposition::RiskOnly);
        }
    }

    #[test]
    fn action_horizon_must_be_fully_covered_by_keepout_claim() {
        let input = corpus();
        for id in ["SA-004-KEEP_OUT-START-LATE", "SA-005-KEEP_OUT-END-EARLY"] {
            let case = input.cases.iter().find(|case| case.case_id == id).unwrap();
            assert_eq!(evaluate_case(case).disposition, Disposition::OutOfScope);
        }
    }

    #[test]
    fn safety_scope_mismatches_are_out_of_scope() {
        let input = corpus();
        for id in [
            "SA-006-VEHICLE-MISMATCH",
            "SA-007-PROTECTED-OBJECT-MISMATCH",
            "SA-008-KEEPOUT-PROFILE-MISMATCH",
            "SA-009-FRAME-MISMATCH",
            "SA-010-DYNAMICS-MISMATCH",
            "SA-011-INITIAL-STATE-MISMATCH",
            "SA-012-UNCERTAINTY-MISMATCH",
            "SA-013-TIME-BASIS-MISMATCH",
        ] {
            let case = input.cases.iter().find(|case| case.case_id == id).unwrap();
            assert_eq!(evaluate_case(case).disposition, Disposition::OutOfScope);
        }
    }

    #[test]
    fn unknown_stale_and_revoked_claims_fail_closed() {
        let input = corpus();
        for id in [
            "SA-016-UNKNOWN-FINITE",
            "SA-017-STALE-TERMINAL",
            "SA-018-REVOKED-FINITE",
        ] {
            let case = input.cases.iter().find(|case| case.case_id == id).unwrap();
            assert_eq!(evaluate_case(case).disposition, Disposition::Indeterminate);
        }
    }

    #[test]
    fn margin_and_terminal_epoch_are_enforced() {
        let input = corpus();
        let short = input
            .cases
            .iter()
            .find(|case| case.case_id == "SA-019-CLEARANCE-TOO-SMALL")
            .unwrap();
        let wrong_terminal = input
            .cases
            .iter()
            .find(|case| case.case_id == "SA-020-TERMINAL-EPOCH-MISMATCH")
            .unwrap();
        assert_eq!(
            evaluate_case(short).disposition,
            Disposition::SafetyClaimFailed
        );
        assert_eq!(
            evaluate_case(wrong_terminal).disposition,
            Disposition::OutOfScope
        );
    }

    #[test]
    fn malformed_component_class_or_digest_is_invalid() {
        let input = corpus();
        for id in [
            "SA-021-WRONG-ASSURANCE-CLASS",
            "SA-022-DIGEST-MALFORMED",
            "SA-023-MISSING-TERMINAL-PROOF",
        ] {
            let case = input.cases.iter().find(|case| case.case_id == id).unwrap();
            assert_eq!(evaluate_case(case).disposition, Disposition::InvalidInput);
        }
    }

    #[test]
    fn half_open_validity_boundaries_distinguish_action_horizon_from_terminal_epoch() {
        let input = corpus();
        let finite = input
            .cases
            .iter()
            .find(|case| case.case_id == "SA-025-FINITE-VALID-UNTIL-EQUALS-ACTION-END")
            .unwrap();
        let terminal = input
            .cases
            .iter()
            .find(|case| case.case_id == "SA-026-TERMINAL-VALID-UNTIL-EQUALS-EPOCH")
            .unwrap();

        assert_eq!(
            evaluate_case(finite).disposition,
            Disposition::SafetyClaimsComplete
        );
        assert_eq!(evaluate_case(terminal).disposition, Disposition::OutOfScope);
    }

    #[test]
    fn case_order_does_not_change_canonical_receipt_order() {
        let mut input = corpus();
        let source = "0123456789abcdef0123456789abcdef01234567";
        let original = audit_corpus(corpus_bytes(), source).unwrap();
        input.cases.reverse();
        let bytes = serde_json::to_vec(&input).unwrap();
        let replay = audit_corpus(&bytes, source).unwrap();
        assert_eq!(original.cases, replay.cases);
    }
}
