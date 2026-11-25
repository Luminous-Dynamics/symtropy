use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const CORPUS_SCHEMA: &str = "luminous.orbital-inspection-two-key-admission.corpus.v0.1";
pub const RECEIPT_SCHEMA: &str = "luminous.orbital-inspection-two-key-admission.receipt.v0.1";
pub const BENCHMARK_ID: &str = "ORBITAL-INSPECTION-TWO-KEY-ADMISSION-001";
pub const CLAIM_CEILING: &str = "synthetic_non_compensatory_action_admission_composition_only";
const MAX_COST: u64 = 1_000_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GateState {
    Pass,
    Fail,
    Unknown,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActionClass {
    Inspection,
    Recovery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Disposition {
    EligibleInspection,
    EligibleRecovery,
    RejectedSafety,
    RejectedPortfolio,
    RejectedEvidence,
    OutOfScope,
    Indeterminate,
    InvalidInput,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionScope {
    pub action_id: String,
    pub vehicle_id: String,
    pub trajectory_revision: String,
    pub horizon_id: String,
    pub acquisition_id: Option<String>,
    pub camera_id: Option<String>,
    pub asset_id: Option<String>,
    pub feature_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicalSafetyBinding {
    pub state: GateState,
    pub action_id: String,
    pub vehicle_id: String,
    pub trajectory_revision: String,
    pub horizon_id: String,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PortfolioBinding {
    pub state: GateState,
    pub event_set_complete: bool,
    pub action_id: String,
    pub vehicle_id: String,
    pub trajectory_revision: String,
    pub horizon_id: String,
    pub event_set_revision: String,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MeasurementBinding {
    pub acquisition_state: GateState,
    pub scope_coherence_state: GateState,
    pub action_id: String,
    pub vehicle_id: String,
    pub trajectory_revision: String,
    pub horizon_id: String,
    pub acquisition_id: String,
    pub camera_id: String,
    pub asset_id: String,
    pub feature_id: String,
    pub measurement_receipt_sha256: String,
    pub scope_receipt_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateAction {
    pub class: ActionClass,
    pub scope: ActionScope,
    pub physical_safety: Option<PhysicalSafetyBinding>,
    pub post_action_portfolio: Option<PortfolioBinding>,
    pub measurement: Option<MeasurementBinding>,
    pub delta_v_milli_m_s: u64,
    pub duration_ms: u64,
    pub compute_us: u64,
    pub decision_value_milli: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub case_id: String,
    pub candidate: CandidateAction,
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
pub struct CaseResult {
    pub case_id: String,
    pub disposition: Disposition,
    pub reason_codes: Vec<String>,
    pub delta_v_milli_m_s: u64,
    pub duration_ms: u64,
    pub compute_us: u64,
    pub decision_value_milli: u64,
    pub physical_safety_receipt_sha256: Option<String>,
    pub portfolio_receipt_sha256: Option<String>,
    pub measurement_receipt_sha256: Option<String>,
    pub scope_receipt_sha256: Option<String>,
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

fn valid_optional_text(value: &Option<String>) -> bool {
    value.as_ref().is_none_or(|item| valid_text(item))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn checked_text_fields(scope: &ActionScope) -> bool {
    valid_text(&scope.action_id)
        && valid_text(&scope.vehicle_id)
        && valid_text(&scope.trajectory_revision)
        && valid_text(&scope.horizon_id)
        && valid_optional_text(&scope.acquisition_id)
        && valid_optional_text(&scope.camera_id)
        && valid_optional_text(&scope.asset_id)
        && valid_optional_text(&scope.feature_id)
}

fn valid_costs(candidate: &CandidateAction) -> bool {
    [
        candidate.delta_v_milli_m_s,
        candidate.duration_ms,
        candidate.compute_us,
        candidate.decision_value_milli,
    ]
    .iter()
    .all(|value| *value <= MAX_COST)
}

fn same(required: &str, supplied: &str) -> bool {
    required == supplied
}

fn evaluate_case(case: &Case) -> CaseResult {
    let candidate = &case.candidate;
    let scope = &candidate.scope;
    let mut reasons = BTreeSet::new();

    let safety_digest = candidate
        .physical_safety
        .as_ref()
        .map(|binding| binding.receipt_sha256.clone());
    let portfolio_digest = candidate
        .post_action_portfolio
        .as_ref()
        .map(|binding| binding.receipt_sha256.clone());
    let measurement_digest = candidate
        .measurement
        .as_ref()
        .map(|binding| binding.measurement_receipt_sha256.clone());
    let scope_digest = candidate
        .measurement
        .as_ref()
        .map(|binding| binding.scope_receipt_sha256.clone());

    let result = |disposition, reason_codes: Vec<String>| CaseResult {
        case_id: case.case_id.clone(),
        disposition,
        reason_codes,
        delta_v_milli_m_s: candidate.delta_v_milli_m_s,
        duration_ms: candidate.duration_ms,
        compute_us: candidate.compute_us,
        decision_value_milli: candidate.decision_value_milli,
        physical_safety_receipt_sha256: safety_digest.clone(),
        portfolio_receipt_sha256: portfolio_digest.clone(),
        measurement_receipt_sha256: measurement_digest.clone(),
        scope_receipt_sha256: scope_digest.clone(),
    };

    if !valid_text(&case.case_id) || !checked_text_fields(scope) || !valid_costs(candidate) {
        return result(Disposition::InvalidInput, vec!["CASE_SCOPE_OR_COST_INVALID".to_owned()]);
    }

    if candidate.class == ActionClass::Recovery
        && (candidate.post_action_portfolio.is_some()
            || candidate.measurement.is_some()
            || scope.acquisition_id.is_some()
            || scope.camera_id.is_some()
            || scope.asset_id.is_some()
            || scope.feature_id.is_some())
    {
        return result(
            Disposition::InvalidInput,
            vec!["RECOVERY_ACTION_MUST_NOT_CLAIM_INSPECTION".to_owned()],
        );
    }

    if let Some(binding) = &candidate.physical_safety {
        if !valid_text(&binding.action_id)
            || !valid_text(&binding.vehicle_id)
            || !valid_text(&binding.trajectory_revision)
            || !valid_text(&binding.horizon_id)
            || !valid_sha256(&binding.receipt_sha256)
        {
            return result(Disposition::InvalidInput, vec!["PHYSICAL_SAFETY_BINDING_INVALID".to_owned()]);
        }
        if !same(&scope.action_id, &binding.action_id)
            || !same(&scope.vehicle_id, &binding.vehicle_id)
            || !same(&scope.trajectory_revision, &binding.trajectory_revision)
            || !same(&scope.horizon_id, &binding.horizon_id)
        {
            return result(Disposition::OutOfScope, vec!["PHYSICAL_SAFETY_SCOPE_MISMATCH".to_owned()]);
        }
    } else {
        return result(Disposition::Indeterminate, vec!["PHYSICAL_SAFETY_EVIDENCE_MISSING".to_owned()]);
    }

    if let Some(binding) = &candidate.post_action_portfolio {
        if !valid_text(&binding.action_id)
            || !valid_text(&binding.vehicle_id)
            || !valid_text(&binding.trajectory_revision)
            || !valid_text(&binding.horizon_id)
            || !valid_text(&binding.event_set_revision)
            || !valid_sha256(&binding.receipt_sha256)
        {
            return result(Disposition::InvalidInput, vec!["PORTFOLIO_BINDING_INVALID".to_owned()]);
        }
        if !same(&scope.action_id, &binding.action_id)
            || !same(&scope.vehicle_id, &binding.vehicle_id)
            || !same(&scope.trajectory_revision, &binding.trajectory_revision)
            || !same(&scope.horizon_id, &binding.horizon_id)
        {
            return result(Disposition::OutOfScope, vec!["POST_ACTION_PORTFOLIO_SCOPE_MISMATCH".to_owned()]);
        }
    }

    if let Some(binding) = &candidate.measurement {
        if !valid_text(&binding.action_id)
            || !valid_text(&binding.vehicle_id)
            || !valid_text(&binding.trajectory_revision)
            || !valid_text(&binding.horizon_id)
            || !valid_text(&binding.acquisition_id)
            || !valid_text(&binding.camera_id)
            || !valid_text(&binding.asset_id)
            || !valid_text(&binding.feature_id)
            || !valid_sha256(&binding.measurement_receipt_sha256)
            || !valid_sha256(&binding.scope_receipt_sha256)
        {
            return result(Disposition::InvalidInput, vec!["MEASUREMENT_BINDING_INVALID".to_owned()]);
        }
        let Some(acquisition_id) = scope.acquisition_id.as_deref() else {
            return result(Disposition::Indeterminate, vec!["ACQUISITION_SCOPE_MISSING".to_owned()]);
        };
        let Some(camera_id) = scope.camera_id.as_deref() else {
            return result(Disposition::Indeterminate, vec!["CAMERA_SCOPE_MISSING".to_owned()]);
        };
        let Some(asset_id) = scope.asset_id.as_deref() else {
            return result(Disposition::Indeterminate, vec!["ASSET_SCOPE_MISSING".to_owned()]);
        };
        let Some(feature_id) = scope.feature_id.as_deref() else {
            return result(Disposition::Indeterminate, vec!["FEATURE_SCOPE_MISSING".to_owned()]);
        };
        if !same(&scope.action_id, &binding.action_id)
            || !same(&scope.vehicle_id, &binding.vehicle_id)
            || !same(&scope.trajectory_revision, &binding.trajectory_revision)
            || !same(&scope.horizon_id, &binding.horizon_id)
            || !same(acquisition_id, &binding.acquisition_id)
            || !same(camera_id, &binding.camera_id)
            || !same(asset_id, &binding.asset_id)
            || !same(feature_id, &binding.feature_id)
        {
            return result(Disposition::OutOfScope, vec!["MEASUREMENT_SCOPE_MISMATCH".to_owned()]);
        }
    }

    let safety = candidate.physical_safety.as_ref().expect("checked above");
    match safety.state {
        GateState::Pass => {}
        GateState::Fail => {
            reasons.insert("PHYSICAL_SAFETY_FAIL".to_owned());
            return result(Disposition::RejectedSafety, reasons.into_iter().collect());
        }
        GateState::Unknown => {
            reasons.insert("PHYSICAL_SAFETY_UNKNOWN".to_owned());
            return result(Disposition::Indeterminate, reasons.into_iter().collect());
        }
        GateState::Stale => {
            reasons.insert("PHYSICAL_SAFETY_STALE".to_owned());
            return result(Disposition::Indeterminate, reasons.into_iter().collect());
        }
    }

    match candidate.class {
        ActionClass::Recovery => {
            if candidate.post_action_portfolio.is_some() || candidate.measurement.is_some() {
                return result(
                    Disposition::InvalidInput,
                    vec!["RECOVERY_ACTION_MUST_NOT_CLAIM_INSPECTION".to_owned()],
                );
            }
            if scope.acquisition_id.is_some()
                || scope.camera_id.is_some()
                || scope.asset_id.is_some()
                || scope.feature_id.is_some()
            {
                return result(
                    Disposition::InvalidInput,
                    vec!["RECOVERY_SCOPE_MUST_NOT_CLAIM_MEASUREMENT".to_owned()],
                );
            }
            result(Disposition::EligibleRecovery, Vec::new())
        }
        ActionClass::Inspection => {
            let Some(portfolio) = candidate.post_action_portfolio.as_ref() else {
                return result(Disposition::Indeterminate, vec!["POST_ACTION_PORTFOLIO_MISSING".to_owned()]);
            };
            if !portfolio.event_set_complete {
                return result(Disposition::Indeterminate, vec!["POST_ACTION_EVENT_SET_INCOMPLETE".to_owned()]);
            }
            match portfolio.state {
                GateState::Pass => {}
                GateState::Fail => {
                    return result(Disposition::RejectedPortfolio, vec!["POST_ACTION_PORTFOLIO_FAIL".to_owned()]);
                }
                GateState::Unknown => {
                    return result(Disposition::Indeterminate, vec!["POST_ACTION_PORTFOLIO_UNKNOWN".to_owned()]);
                }
                GateState::Stale => {
                    return result(Disposition::Indeterminate, vec!["POST_ACTION_PORTFOLIO_STALE".to_owned()]);
                }
            }

            let Some(measurement) = candidate.measurement.as_ref() else {
                return result(Disposition::Indeterminate, vec!["MEASUREMENT_EVIDENCE_MISSING".to_owned()]);
            };
            match measurement.acquisition_state {
                GateState::Pass => {}
                GateState::Fail => {
                    return result(Disposition::RejectedEvidence, vec!["MEASUREMENT_ACQUISITION_FAIL".to_owned()]);
                }
                GateState::Unknown => {
                    return result(Disposition::Indeterminate, vec!["MEASUREMENT_ACQUISITION_UNKNOWN".to_owned()]);
                }
                GateState::Stale => {
                    return result(Disposition::Indeterminate, vec!["MEASUREMENT_ACQUISITION_STALE".to_owned()]);
                }
            }
            match measurement.scope_coherence_state {
                GateState::Pass => {}
                GateState::Fail => {
                    return result(Disposition::RejectedEvidence, vec!["MEASUREMENT_SCOPE_COHERENCE_FAIL".to_owned()]);
                }
                GateState::Unknown => {
                    return result(Disposition::Indeterminate, vec!["MEASUREMENT_SCOPE_COHERENCE_UNKNOWN".to_owned()]);
                }
                GateState::Stale => {
                    return result(Disposition::Indeterminate, vec!["MEASUREMENT_SCOPE_COHERENCE_STALE".to_owned()]);
                }
            }
            result(Disposition::EligibleInspection, Vec::new())
        }
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
        include_bytes!("../../../docs/research/orbital-truss/fixtures/two-key-admission-v0_1.json")
    }

    fn corpus() -> Corpus {
        serde_json::from_slice(corpus_bytes()).expect("fixture parses")
    }

    #[test]
    fn frozen_corpus_matches_every_expected_disposition() {
        let receipt = audit_corpus(corpus_bytes(), "0123456789abcdef0123456789abcdef01234567")
            .expect("corpus audits");
        assert_eq!(receipt.case_count, 23);
        assert_eq!(receipt.expected_disposition_matches, 23);
        assert_eq!(receipt.expected_disposition_mismatches, 0);
        assert!(!receipt.authority_granted);
        assert!(!receipt.source_bytes_authenticated);
    }

    #[test]
    fn both_independent_keys_are_required_for_inspection() {
        let input = corpus();
        let eligible = input
            .cases
            .iter()
            .find(|case| case.case_id == "TK-001-BOTH-PASS")
            .unwrap();
        assert_eq!(evaluate_case(eligible).disposition, Disposition::EligibleInspection);

        let safety_only = input
            .cases
            .iter()
            .find(|case| case.case_id == "TK-004-EVIDENCE-FAIL")
            .unwrap();
        let unsafe_only = input
            .cases
            .iter()
            .find(|case| case.case_id == "TK-002-SAFETY-FAIL")
            .unwrap();
        assert_eq!(evaluate_case(safety_only).disposition, Disposition::RejectedEvidence);
        assert_eq!(evaluate_case(unsafe_only).disposition, Disposition::RejectedSafety);
    }

    #[test]
    fn recovery_action_can_be_admitted_without_measurement_claim() {
        let input = corpus();
        let case = input
            .cases
            .iter()
            .find(|case| case.case_id == "TK-011-RECOVERY-SAFE")
            .unwrap();
        assert_eq!(evaluate_case(case).disposition, Disposition::EligibleRecovery);
    }

    #[test]
    fn high_information_value_never_overrides_failed_safety() {
        let input = corpus();
        let case = input
            .cases
            .iter()
            .find(|case| case.case_id == "TK-013-HIGH-VALUE-UNSAFE")
            .unwrap();
        assert!(case.candidate.decision_value_milli > 1_000_000_000);
        assert_eq!(evaluate_case(case).disposition, Disposition::RejectedSafety);
    }

    #[test]
    fn exact_scope_mismatch_is_out_of_scope() {
        let input = corpus();
        for id in [
            "TK-006-SCOPE-MISMATCH",
            "TK-009-TRAJECTORY-MISMATCH",
            "TK-010-ACQUISITION-MISMATCH",
            "TK-021-PORTFOLIO-TRAJECTORY-MISMATCH",
            "TK-022-MEASUREMENT-VEHICLE-MISMATCH",
            "TK-023-MEASUREMENT-HORIZON-MISMATCH",
        ] {
            let case = input.cases.iter().find(|case| case.case_id == id).unwrap();
            assert_eq!(evaluate_case(case).disposition, Disposition::OutOfScope);
        }
    }

    #[test]
    fn unknown_stale_or_incomplete_evidence_never_admits_inspection() {
        let input = corpus();
        for id in [
            "TK-003-SAFETY-UNKNOWN",
            "TK-005-EVIDENCE-UNKNOWN",
            "TK-007-PORTFOLIO-INCOMPLETE",
            "TK-008-MEASUREMENT-STALE",
            "TK-015-SAFETY-MISSING",
            "TK-016-PORTFOLIO-MISSING",
            "TK-017-MEASUREMENT-MISSING",
        ] {
            let case = input.cases.iter().find(|case| case.case_id == id).unwrap();
            assert_eq!(evaluate_case(case).disposition, Disposition::Indeterminate);
        }
    }

    #[test]
    fn malformed_digest_and_recovery_measurement_claim_are_invalid() {
        let input = corpus();
        let malformed = input
            .cases
            .iter()
            .find(|case| case.case_id == "TK-018-MALFORMED-DIGEST")
            .unwrap();
        assert_eq!(
            evaluate_case(malformed).disposition,
            Disposition::InvalidInput
        );

        let mut recovery = input
            .cases
            .iter()
            .find(|case| case.case_id == "TK-011-RECOVERY-SAFE")
            .unwrap()
            .clone();
        recovery.candidate.measurement = Some(
            input
                .cases
                .iter()
                .find(|case| case.case_id == "TK-001-BOTH-PASS")
                .unwrap()
                .candidate
                .measurement
                .clone()
                .unwrap(),
        );
        assert_eq!(
            evaluate_case(&recovery).disposition,
            Disposition::InvalidInput
        );
    }

    #[test]
    fn action_order_and_utility_do_not_change_hard_gate_disposition() {
        let input = corpus();
        let original = audit_corpus(corpus_bytes(), "0123456789abcdef0123456789abcdef01234567").unwrap();
        let mut permuted = input;
        permuted.cases.reverse();
        let bytes = serde_json::to_vec(&permuted).unwrap();
        let replay = audit_corpus(&bytes, "0123456789abcdef0123456789abcdef01234567").unwrap();
        assert_eq!(original.cases, replay.cases);

        let mut candidate = corpus()
            .cases
            .into_iter()
            .find(|case| case.case_id == "TK-001-BOTH-PASS")
            .unwrap();
        candidate.candidate.decision_value_milli = MAX_COST;
        assert_eq!(evaluate_case(&candidate).disposition, Disposition::EligibleInspection);
    }
}
