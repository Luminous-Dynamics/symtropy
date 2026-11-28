// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Finite ambiguity-set EVSI sensitivity and lower-envelope ranking.
//!
//! This module evaluates a finite, owner-approved set of coherent decision /
//! outcome-model scenarios. It is not a general distributionally robust policy
//! solver. It reports exact per-action value ranges and recommends only actions
//! whose lower net-benefit bound is strictly positive across every admitted
//! scenario. It does not authenticate source-owner signatures; the point-EVSI
//! kernel validates upstream receipt dispositions and exact scopes.

use super::evidence_action_evsi_v0_1::{
    EVSI_NUMERATOR_SCALE_V0_1, EvidenceActionPlannerDispositionV0_1,
    EvidenceActionPlannerInputV0_1, rank_evidence_actions_v0_1,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const ROBUST_EVSI_AMBIGUITY_SCHEMA_V0_1: &str = "AeroRobustEVSIAmbiguitySetV0_1";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RobustEVSIReceiptVerificationStatusV0_1 {
    #[serde(rename = "VERIFIED")]
    Verified,
    #[serde(rename = "UNVERIFIED")]
    Unverified,
    #[serde(rename = "FAILED")]
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RobustEVSIDispositionV0_1 {
    #[serde(rename = "ROBUST_RANKED_ACTIONS")]
    RobustRankedActions,
    #[serde(rename = "NO_ROBUST_POSITIVE_NET_BENEFIT")]
    NoRobustPositiveNetBenefit,
    #[serde(rename = "ABSTAIN_NO_COMMON_QUALIFIED_ACTIONS")]
    AbstainNoCommonQualifiedActions,
    #[serde(rename = "INVALID_INPUT")]
    InvalidInput,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EVSIAdmissibleScenarioV0_1 {
    pub scenario_id: String,
    pub input: EvidenceActionPlannerInputV0_1,
}

/// Owner/authority approval for one finite ambiguity set.
///
/// This receipt authenticates a scenario-set commitment and its intended claim
/// scope when independently verified upstream. The robust evaluator checks
/// exact scope and chronology but does not authenticate its signature.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EVSIAmbiguitySetApprovalReceiptV0_1 {
    pub verification_status: RobustEVSIReceiptVerificationStatusV0_1,
    pub owner_namespace: String,
    pub claim_id: String,
    pub decision_context_sha256: String,
    pub configuration_ref: String,
    pub scenario_set_sha256: String,
    pub scenario_count: u32,
    pub receipt_sha256: String,
    pub receiver_first_seen_at_unix_ms: u64,
    pub effective_from_unix_ms: u64,
    pub effective_until_unix_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RobustEvidenceActionPlannerInputV0_1 {
    pub schema: String,
    pub scenario_set_id: String,
    pub scenario_set_sha256: String,
    pub approval_receipt: Option<EVSIAmbiguitySetApprovalReceiptV0_1>,
    pub scenarios: Vec<EVSIAdmissibleScenarioV0_1>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RobustEvidenceActionBoundsV0_1 {
    pub rank: usize,
    pub action_id: String,
    pub owner_namespace: String,
    pub outcome_model_owner_namespace: String,
    pub target_obligation_ids: Vec<String>,
    pub action_catalogue_entry_sha256: String,
    pub authorization_receipt_sha256: String,
    pub calibration_receipt_sha256_by_scenario: Vec<(String, String)>,
    /// Lower bound of exact EVSI numerators over the finite scenario set.
    pub evsi_lower_scaled_numerator: String,
    /// Upper bound of exact EVSI numerators over the finite scenario set.
    pub evsi_upper_scaled_numerator: String,
    /// Lower bound of exact expected net-benefit numerators.
    pub net_benefit_lower_scaled_numerator: String,
    /// Upper bound of exact expected net-benefit numerators.
    pub net_benefit_upper_scaled_numerator: String,
    pub worst_case_scenario_id: String,
    pub best_case_scenario_id: String,
    pub positive_net_benefit_in_all_scenarios: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RobustEVSIScenarioActionResultV0_1 {
    pub scenario_id: String,
    pub action_id: String,
    pub evsi_scaled_numerator: String,
    pub net_benefit_scaled_numerator: String,
    pub calibration_receipt_sha256: String,
    pub outcome_likelihoods_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RobustEvidenceActionExclusionV0_1 {
    pub scenario_id: String,
    pub action_id: String,
    pub reason_code: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RobustEvidenceActionPlannerAssessmentV0_1 {
    pub disposition: RobustEVSIDispositionV0_1,
    pub scenario_set_id: String,
    pub scenario_set_sha256: String,
    pub approval_receipt_sha256: String,
    pub scenario_count: usize,
    pub claim_id: String,
    pub frozen_claim_disposition: String,
    pub configuration_ref: String,
    pub as_known_at_unix_ms: u64,
    pub decision_context_sha256: String,
    pub assurance_state_sha256: String,
    pub assurance_state_receipt_sha256: String,
    pub unresolved_obligation_manifest_sha256: String,
    pub action_catalogue_sha256: String,
    pub action_catalogue_receipt_sha256: String,
    pub scenario_action_results: Vec<RobustEVSIScenarioActionResultV0_1>,
    pub ranked_actions: Vec<RobustEvidenceActionBoundsV0_1>,
    pub excluded_actions: Vec<RobustEvidenceActionExclusionV0_1>,
    pub reason_codes: Vec<String>,
    pub selected_action_id: Option<String>,
    pub evsi_numerator_scale: i128,
    pub assurance_state_mutated: bool,
    pub physical_work_authorized: bool,
    pub engineering_acceptance_granted: bool,
}

#[derive(Serialize)]
struct ScenarioActionCalibrationCommitmentV0_1<'a> {
    action_id: &'a str,
    outcome_model_owner_namespace: &'a str,
    calibration_owner_namespace: Option<&'a str>,
    calibration_receipt_sha256: Option<&'a str>,
    calibration_profile_sha256: Option<&'a str>,
    outcome_likelihoods_sha256: Option<&'a str>,
}

#[derive(Serialize)]
struct ScenarioSetManifestEntryV0_1<'a> {
    scenario_id: &'a str,
    decision_model_sha256: &'a str,
    outcome_model_sha256: &'a str,
    action_catalogue_sha256: &'a str,
    unresolved_obligation_manifest_sha256: &'a str,
    action_calibration_commitments: Vec<ScenarioActionCalibrationCommitmentV0_1<'a>>,
}

#[derive(Serialize)]
struct ScenarioSetManifestEnvelopeV0_1<'a> {
    schema: &'static str,
    canonicalization_profile: &'static str,
    scenario_set_id: &'a str,
    entries: Vec<ScenarioSetManifestEntryV0_1<'a>>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

fn valid_identifier(value: &str) -> bool {
    !value.trim().is_empty()
        && value == value.trim()
        && value.bytes().all(|byte| byte.is_ascii_graphic())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence_action_evsi_v0_1::{
        EvidenceActionPlannerInputV0_1, compute_outcome_likelihoods_sha256,
    };
    use serde_json::Value;

    const FIXTURE: &str = include_str!(
        "../../../../docs/research/aviation/examples/aero_evidence_action_planner_v0_1.json"
    );

    fn point_input() -> EvidenceActionPlannerInputV0_1 {
        let fixture: Value = serde_json::from_str(FIXTURE).expect("EVSI fixture parses");
        serde_json::from_value(fixture["input"].clone())
            .expect("EVSI input matches the typed point-evaluator contract")
    }

    fn no_information_scenario() -> EvidenceActionPlannerInputV0_1 {
        let mut input = point_input();
        input.outcome_model_sha256 = "f".repeat(64);

        for outcome in &mut input.actions[0].outcome_likelihoods {
            outcome.likelihood_ppm = 500_000;
        }

        let model_sha = input.outcome_model_sha256.clone();
        for (index, action) in input.actions.iter_mut().enumerate() {
            let likelihood_sha = compute_outcome_likelihoods_sha256(action, &model_sha)
                .expect("exact likelihood-table commitment");
            let receipt = action
                .calibration_receipt
                .as_mut()
                .expect("fixture action has calibration receipt");
            receipt.outcome_model_sha256 = model_sha.clone();
            receipt.outcome_likelihoods_sha256 = likelihood_sha;
            receipt.calibration_report_sha256 = if index == 0 {
                "c".repeat(64)
            } else {
                "d".repeat(64)
            };
        }
        input
    }

    fn robust_input(
        scenario_set_id: &str,
        scenarios: Vec<EVSIAdmissibleScenarioV0_1>,
    ) -> RobustEvidenceActionPlannerInputV0_1 {
        let first = scenarios.first().expect("at least one scenario");
        let claim_id = first.input.claim_id.clone();
        let decision_context_sha256 = first.input.decision_context_sha256.clone();
        let configuration_ref = first.input.configuration_ref.clone();
        let as_known_at_unix_ms = first.input.as_known_at_unix_ms;
        let mut input = RobustEvidenceActionPlannerInputV0_1 {
            schema: ROBUST_EVSI_AMBIGUITY_SCHEMA_V0_1.to_owned(),
            scenario_set_id: scenario_set_id.to_owned(),
            scenario_set_sha256: String::new(),
            approval_receipt: None,
            scenarios,
        };
        input.scenario_set_sha256 =
            compute_scenario_set_sha256_v0_1(&input.scenario_set_id, &input.scenarios)
                .expect("scenario-set commitment");
        input.approval_receipt = Some(EVSIAmbiguitySetApprovalReceiptV0_1 {
            verification_status: RobustEVSIReceiptVerificationStatusV0_1::Verified,
            owner_namespace: "owner:approved-decision-model-board".to_owned(),
            claim_id,
            decision_context_sha256,
            configuration_ref,
            scenario_set_sha256: input.scenario_set_sha256.clone(),
            scenario_count: input.scenarios.len() as u32,
            receipt_sha256: "a".repeat(64),
            receiver_first_seen_at_unix_ms: as_known_at_unix_ms - 1_000,
            effective_from_unix_ms: as_known_at_unix_ms - 10_000,
            effective_until_unix_ms: as_known_at_unix_ms + 10_000,
        });
        input
    }

    fn scenario(id: &str, input: EvidenceActionPlannerInputV0_1) -> EVSIAdmissibleScenarioV0_1 {
        EVSIAdmissibleScenarioV0_1 {
            scenario_id: id.to_owned(),
            input,
        }
    }

    #[test]
    fn positive_lower_envelope_produces_robust_recommendation() {
        let base = point_input();
        let input = robust_input(
            "ambiguity-set:positive-control",
            vec![
                scenario("scenario:baseline-a", base.clone()),
                scenario("scenario:baseline-b", base),
            ],
        );
        let result = rank_robust_evidence_actions_v0_1(&input);
        assert_eq!(result.disposition, RobustEVSIDispositionV0_1::RobustRankedActions);
        assert_eq!(
            result.selected_action_id.as_deref(),
            Some("action:request-calibration-report")
        );
        assert!(result.ranked_actions[0].positive_net_benefit_in_all_scenarios);
        assert!(result.scenario_action_results.len() >= 2);
        assert!(!result.assurance_state_mutated);
        assert!(!result.physical_work_authorized);
        assert!(!result.engineering_acceptance_granted);
    }

    #[test]
    fn point_positive_but_zero_information_scenario_forces_abstention() {
        let base = point_input();
        let no_information = no_information_scenario();
        let input = robust_input(
            "ambiguity-set:ranking-instability",
            vec![
                scenario("scenario:informative", base),
                scenario("scenario:no-information", no_information),
            ],
        );
        let result = rank_robust_evidence_actions_v0_1(&input);
        assert_eq!(
            result.disposition,
            RobustEVSIDispositionV0_1::NoRobustPositiveNetBenefit
        );
        assert_eq!(result.selected_action_id, None);
        let lower = result.ranked_actions[0]
            .net_benefit_lower_scaled_numerator
            .parse::<i128>()
            .expect("lower exact net-benefit bound");
        assert!(lower <= 0);
        assert!(!result.ranked_actions[0].positive_net_benefit_in_all_scenarios);
    }

    #[test]
    fn late_ambiguity_set_approval_cannot_backfill_historical_knowledge() {
        let base = point_input();
        let mut input = robust_input(
            "ambiguity-set:late-approval",
            vec![
                scenario("scenario:a", base.clone()),
                scenario("scenario:b", base),
            ],
        );
        let cutoff = input.scenarios[0].input.as_known_at_unix_ms;
        input
            .approval_receipt
            .as_mut()
            .expect("approval receipt")
            .receiver_first_seen_at_unix_ms = cutoff + 1;
        let result = rank_robust_evidence_actions_v0_1(&input);
        assert_eq!(result.disposition, RobustEVSIDispositionV0_1::InvalidInput);
        assert!(result
            .reason_codes
            .contains(&"ROBUST_EVSI_SCENARIO_SET_APPROVAL_NOT_KNOWN_AT_CUTOFF".to_owned()));
    }

    #[test]
    fn scenario_manifest_mutation_requires_a_new_approval_commitment() {
        let base = point_input();
        let mut input = robust_input(
            "ambiguity-set:manifest-substitution",
            vec![
                scenario("scenario:a", base.clone()),
                scenario("scenario:b", base),
            ],
        );
        input.scenarios[1].input.outcome_model_sha256 = "f".repeat(64);
        let result = rank_robust_evidence_actions_v0_1(&input);
        assert_eq!(result.disposition, RobustEVSIDispositionV0_1::InvalidInput);
        assert!(result
            .reason_codes
            .contains(&"ROBUST_EVSI_SCENARIO_SET_COMMITMENT_MISMATCH".to_owned()));
    }

    #[test]
    fn scenario_order_permutation_preserves_complete_robust_result() {
        let base = point_input();
        let input = robust_input(
            "ambiguity-set:permutation",
            vec![
                scenario("scenario:a", base.clone()),
                scenario("scenario:b", base),
            ],
        );
        let baseline = rank_robust_evidence_actions_v0_1(&input);
        let mut permuted = input.clone();
        permuted.scenarios.reverse();
        let result = rank_robust_evidence_actions_v0_1(&permuted);
        assert_eq!(baseline, result);
    }
}

/// Recompute the finite ambiguity-set manifest over the exact coherent
/// scenario commitments. Scenario mutations remain inside their root-cause
/// family for benchmark statistics; this digest is not an independence claim.
pub fn compute_scenario_set_sha256_v0_1(
    scenario_set_id: &str,
    scenarios: &[EVSIAdmissibleScenarioV0_1],
) -> Result<String, String> {
    let mut ordered = scenarios.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));
    let entries = ordered
        .iter()
        .map(|scenario| {
            let mut actions = scenario.input.actions.iter().collect::<Vec<_>>();
            actions.sort_by(|left, right| left.action_id.cmp(&right.action_id));
            let action_calibration_commitments = actions
                .iter()
                .map(|action| {
                    let receipt = action.calibration_receipt.as_ref();
                    ScenarioActionCalibrationCommitmentV0_1 {
                        action_id: &action.action_id,
                        outcome_model_owner_namespace: &action.outcome_model_owner_namespace,
                        calibration_owner_namespace: receipt.map(|item| item.owner_namespace.as_str()),
                        calibration_receipt_sha256: receipt.map(|item| item.calibration_report_sha256.as_str()),
                        calibration_profile_sha256: receipt.map(|item| item.calibration_profile_sha256.as_str()),
                        outcome_likelihoods_sha256: receipt.map(|item| item.outcome_likelihoods_sha256.as_str()),
                    }
                })
                .collect();
            ScenarioSetManifestEntryV0_1 {
                scenario_id: &scenario.scenario_id,
                decision_model_sha256: &scenario.input.decision_model_sha256,
                outcome_model_sha256: &scenario.input.outcome_model_sha256,
                action_catalogue_sha256: &scenario.input.action_catalogue_sha256,
                unresolved_obligation_manifest_sha256: &scenario
                    .input
                    .unresolved_obligation_manifest_sha256,
                action_calibration_commitments,
            }
        })
        .collect();
    let envelope = ScenarioSetManifestEnvelopeV0_1 {
        schema: "AeroEVSIAmbiguityScenarioSetV0_1",
        canonicalization_profile: "rust-serde-json-struct-order-v1",
        scenario_set_id,
        entries,
    };
    serde_json::to_vec(&envelope)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|_| "ROBUST_EVSI_SCENARIO_SET_SERIALIZATION_FAILED".to_owned())
}

fn invalid_result(
    input: &RobustEvidenceActionPlannerInputV0_1,
    reasons: BTreeSet<String>,
) -> RobustEvidenceActionPlannerAssessmentV0_1 {
    let anchor = input.scenarios.first().map(|scenario| &scenario.input);
    let (claim_id, disposition, configuration, as_known, context, state, state_receipt, unresolved, catalogue, catalogue_receipt) =
        if let Some(anchor) = anchor {
            (
                anchor.claim_id.clone(),
                anchor.frozen_claim_disposition.clone(),
                anchor.configuration_ref.clone(),
                anchor.as_known_at_unix_ms,
                anchor.decision_context_sha256.clone(),
                anchor.assurance_state_sha256.clone(),
                anchor.assurance_state_receipt.as_ref().map(|r| r.receipt_sha256.clone()).unwrap_or_default(),
                anchor.unresolved_obligation_manifest_sha256.clone(),
                anchor.action_catalogue_sha256.clone(),
                anchor.action_catalogue_receipt.as_ref().map(|r| r.receipt_sha256.clone()).unwrap_or_default(),
            )
        } else {
            (String::new(), String::new(), String::new(), 0, String::new(), String::new(), String::new(), String::new(), String::new(), String::new())
        };
    RobustEvidenceActionPlannerAssessmentV0_1 {
        disposition: RobustEVSIDispositionV0_1::InvalidInput,
        scenario_set_id: input.scenario_set_id.clone(),
        scenario_set_sha256: input.scenario_set_sha256.clone(),
        approval_receipt_sha256: input.approval_receipt.as_ref().map(|r| r.receipt_sha256.clone()).unwrap_or_default(),
        scenario_count: input.scenarios.len(),
        claim_id,
        frozen_claim_disposition: disposition,
        configuration_ref: configuration,
        as_known_at_unix_ms: as_known,
        decision_context_sha256: context,
        assurance_state_sha256: state,
        assurance_state_receipt_sha256: state_receipt,
        unresolved_obligation_manifest_sha256: unresolved,
        action_catalogue_sha256: catalogue,
        action_catalogue_receipt_sha256: catalogue_receipt,
        scenario_action_results: Vec::new(),
        ranked_actions: Vec::new(),
        excluded_actions: Vec::new(),
        reason_codes: reasons.into_iter().collect(),
        selected_action_id: None,
        evsi_numerator_scale: EVSI_NUMERATOR_SCALE_V0_1,
        assurance_state_mutated: false,
        physical_work_authorized: false,
        engineering_acceptance_granted: false,
    }
}

fn validate_scenario_set(
    input: &RobustEvidenceActionPlannerInputV0_1,
) -> BTreeSet<String> {
    let mut errors = BTreeSet::new();
    if input.schema != ROBUST_EVSI_AMBIGUITY_SCHEMA_V0_1 || !valid_identifier(&input.scenario_set_id) {
        errors.insert("ROBUST_EVSI_SCHEMA_OR_SET_ID_INVALID".to_owned());
    }
    if input.scenarios.len() < 2 {
        errors.insert("ROBUST_EVSI_REQUIRES_AT_LEAST_TWO_SCENARIOS".to_owned());
    }
    if !valid_sha256(&input.scenario_set_sha256) {
        errors.insert("ROBUST_EVSI_SCENARIO_SET_DIGEST_INVALID".to_owned());
    }
    if compute_scenario_set_sha256_v0_1(&input.scenario_set_id, &input.scenarios).as_deref()
        != Ok(input.scenario_set_sha256.as_str())
    {
        errors.insert("ROBUST_EVSI_SCENARIO_SET_COMMITMENT_MISMATCH".to_owned());
    }

    let mut scenario_ids = BTreeSet::new();
    let Some(anchor_scenario) = input.scenarios.first() else {
        errors.insert("ROBUST_EVSI_SCENARIO_SET_EMPTY".to_owned());
        return errors;
    };
    let anchor = &anchor_scenario.input;
    for scenario in &input.scenarios {
        if !valid_identifier(&scenario.scenario_id) || !scenario_ids.insert(scenario.scenario_id.as_str()) {
            errors.insert("ROBUST_EVSI_SCENARIO_ID_INVALID_OR_DUPLICATE".to_owned());
        }
        let candidate = &scenario.input;
        if candidate.claim_id != anchor.claim_id
            || candidate.frozen_claim_disposition != anchor.frozen_claim_disposition
            || candidate.configuration_ref != anchor.configuration_ref
            || candidate.as_known_at_unix_ms != anchor.as_known_at_unix_ms
            || candidate.decision_context_sha256 != anchor.decision_context_sha256
            || candidate.assurance_state_sha256 != anchor.assurance_state_sha256
            || candidate.assurance_state_receipt != anchor.assurance_state_receipt
            || candidate.unresolved_obligation_manifest_sha256 != anchor.unresolved_obligation_manifest_sha256
            || candidate.action_catalogue_sha256 != anchor.action_catalogue_sha256
            || candidate.action_catalogue_receipt != anchor.action_catalogue_receipt
        {
            errors.insert("ROBUST_EVSI_SCENARIO_CLAIM_OR_CONTEXT_MISMATCH".to_owned());
        }
    }

    match &input.approval_receipt {
        None => {
            errors.insert("ROBUST_EVSI_SCENARIO_SET_APPROVAL_RECEIPT_MISSING".to_owned());
        }
        Some(receipt) => {
            if receipt.verification_status != RobustEVSIReceiptVerificationStatusV0_1::Verified {
                errors.insert("ROBUST_EVSI_SCENARIO_SET_APPROVAL_NOT_VERIFIED".to_owned());
            }
            if !valid_identifier(&receipt.owner_namespace)
                || !valid_sha256(&receipt.receipt_sha256)
            {
                errors.insert("ROBUST_EVSI_SCENARIO_SET_APPROVAL_RECEIPT_INVALID".to_owned());
            }
            if receipt.claim_id != anchor.claim_id
                || receipt.decision_context_sha256 != anchor.decision_context_sha256
                || receipt.configuration_ref != anchor.configuration_ref
                || receipt.scenario_set_sha256 != input.scenario_set_sha256
                || receipt.scenario_count as usize != input.scenarios.len()
            {
                errors.insert("ROBUST_EVSI_SCENARIO_SET_APPROVAL_SCOPE_MISMATCH".to_owned());
            }
            if receipt.receiver_first_seen_at_unix_ms > anchor.as_known_at_unix_ms {
                errors.insert("ROBUST_EVSI_SCENARIO_SET_APPROVAL_NOT_KNOWN_AT_CUTOFF".to_owned());
            }
            if receipt.effective_from_unix_ms >= receipt.effective_until_unix_ms
                || receipt.effective_from_unix_ms > anchor.as_known_at_unix_ms
                || receipt.effective_until_unix_ms <= anchor.as_known_at_unix_ms
            {
                errors.insert("ROBUST_EVSI_SCENARIO_SET_APPROVAL_NOT_EFFECTIVE_AT_CUTOFF".to_owned());
            }
        }
    }

    errors
}

/// Compute a finite lower/upper EVSI envelope and robust recommendation.
///
/// Every scenario is independently passed through the exact point-EVSI
/// evaluator. An action is eligible for robust ranking only if it is permitted
/// and calibrated in every admitted scenario. The lower net-benefit bound is
/// the minimum exact numerator over the finite scenario set. The chosen action
/// maximizes that lower bound, with deterministic tie-breaks. This is a finite
/// lower-envelope sensitivity policy, not a continuous distributionally robust
/// optimization theorem.
pub fn rank_robust_evidence_actions_v0_1(
    input: &RobustEvidenceActionPlannerInputV0_1,
) -> RobustEvidenceActionPlannerAssessmentV0_1 {
    let errors = validate_scenario_set(input);
    if !errors.is_empty() {
        return invalid_result(input, errors);
    }

    let mut evaluations = Vec::new();
    let mut scenario_action_results = Vec::new();
    let mut scenario_exclusions = Vec::new();
    for scenario in &input.scenarios {
        let result = rank_evidence_actions_v0_1(&scenario.input);
        if result.disposition == EvidenceActionPlannerDispositionV0_1::InvalidInput {
            let mut reasons = BTreeSet::new();
            reasons.insert(format!(
                "ROBUST_EVSI_SCENARIO_INPUT_INVALID:{}",
                scenario.scenario_id
            ));
            reasons.extend(result.reason_codes.iter().map(|code| {
                format!("SCENARIO:{}:{code}", scenario.scenario_id)
            }));
            return invalid_result(input, reasons);
        }
        for excluded_action in &result.excluded_actions {
            scenario_exclusions.push(RobustEvidenceActionExclusionV0_1 {
                scenario_id: scenario.scenario_id.clone(),
                action_id: excluded_action.action_id.clone(),
                reason_code: excluded_action.reason_code.clone(),
            });
        }
        let mut action_map = BTreeMap::new();
        for action in result.ranked_actions {
            let Ok(evsi) = action.evsi_scaled_numerator.parse::<i128>() else {
                return invalid_result(
                    input,
                    BTreeSet::from(["ROBUST_EVSI_EVSI_NUMERATOR_INVALID".to_owned()]),
                );
            };
            let Ok(net_benefit) = action.net_benefit_scaled_numerator.parse::<i128>() else {
                return invalid_result(
                    input,
                    BTreeSet::from(["ROBUST_EVSI_NET_BENEFIT_NUMERATOR_INVALID".to_owned()]),
                );
            };
            scenario_action_results.push(RobustEVSIScenarioActionResultV0_1 {
                scenario_id: scenario.scenario_id.clone(),
                action_id: action.action_id.clone(),
                evsi_scaled_numerator: evsi.to_string(),
                net_benefit_scaled_numerator: net_benefit.to_string(),
                calibration_receipt_sha256: action.calibration_receipt_sha256.clone(),
                outcome_likelihoods_sha256: action.outcome_likelihoods_sha256.clone(),
            });
            action_map.insert(action.action_id.clone(), (action, evsi, net_benefit));
        }
        evaluations.push((scenario.scenario_id.as_str(), action_map));
    }

    let mut all_action_ids = BTreeSet::new();
    for (_, action_map) in &evaluations {
        all_action_ids.extend(action_map.keys().cloned());
    }

    let mut bounds = Vec::new();
    let mut excluded: Vec<RobustEvidenceActionExclusionV0_1> = scenario_exclusions;
    for action_id in all_action_ids {
        let mut per_scenario = Vec::new();
        let mut missing = false;
        for (scenario_id, action_map) in &evaluations {
            match action_map.get(&action_id) {
                Some((action, evsi, net_benefit)) => {
                    per_scenario.push((*scenario_id, action, *evsi, *net_benefit));
                }
                None => missing = true,
            }
        }
        if missing || per_scenario.len() != evaluations.len() {
            for (scenario_id, action_map) in &evaluations {
                if !action_map.contains_key(&action_id) {
                    if !excluded.iter().any(|item| {
                        item.scenario_id.as_str() == *scenario_id && item.action_id.as_str() == action_id.as_str()
                    }) {
                        excluded.push(RobustEvidenceActionExclusionV0_1 {
                            scenario_id: scenario_id.to_string(),
                            action_id: action_id.clone(),
                            reason_code: "ACTION_NOT_QUALIFIED_IN_EVERY_APPROVED_SCENARIO".to_owned(),
                        });
                    }
                }
            }
            continue;
        }

        let mut evsi_min = per_scenario[0];
        let mut evsi_max = per_scenario[0];
        let mut net_min = per_scenario[0];
        let mut net_max = per_scenario[0];
        for item in &per_scenario[1..] {
            if item.2 < evsi_min.2 || (item.2 == evsi_min.2 && item.0 < evsi_min.0) {
                evsi_min = *item;
            }
            if item.2 > evsi_max.2 || (item.2 == evsi_max.2 && item.0 < evsi_max.0) {
                evsi_max = *item;
            }
            if item.3 < net_min.3 || (item.3 == net_min.3 && item.0 < net_min.0) {
                net_min = *item;
            }
            if item.3 > net_max.3 || (item.3 == net_max.3 && item.0 < net_max.0) {
                net_max = *item;
            }
        }
        let first = per_scenario[0].1;
        let mut calibration_receipt_sha256_by_scenario = per_scenario
            .iter()
            .map(|(scenario_id, action, _, _)| {
                (scenario_id.to_string(), action.calibration_receipt_sha256.clone())
            })
            .collect::<Vec<_>>();
        calibration_receipt_sha256_by_scenario.sort_by(|a, b| a.0.cmp(&b.0));
        bounds.push(RobustEvidenceActionBoundsV0_1 {
            rank: 0,
            action_id: action_id.clone(),
            owner_namespace: first.owner_namespace.clone(),
            outcome_model_owner_namespace: first.outcome_model_owner_namespace.clone(),
            target_obligation_ids: first.target_obligation_ids.clone(),
            action_catalogue_entry_sha256: first.action_catalogue_entry_sha256.clone(),
            authorization_receipt_sha256: first.authorization_receipt_sha256.clone(),
            calibration_receipt_sha256_by_scenario,
            evsi_lower_scaled_numerator: evsi_min.2.to_string(),
            evsi_upper_scaled_numerator: evsi_max.2.to_string(),
            net_benefit_lower_scaled_numerator: net_min.3.to_string(),
            net_benefit_upper_scaled_numerator: net_max.3.to_string(),
            worst_case_scenario_id: net_min.0.to_owned(),
            best_case_scenario_id: net_max.0.to_owned(),
            positive_net_benefit_in_all_scenarios: net_min.3 > 0,
        });
    }

    bounds.sort_by(|left, right| {
        let left_nb = left.net_benefit_lower_scaled_numerator.parse::<i128>().unwrap_or(i128::MIN);
        let right_nb = right.net_benefit_lower_scaled_numerator.parse::<i128>().unwrap_or(i128::MIN);
        right_nb.cmp(&left_nb).then_with(|| left.action_id.cmp(&right.action_id))
    });
    for (index, item) in bounds.iter_mut().enumerate() {
        item.rank = index + 1;
    }
    scenario_action_results.sort_by(|a, b| {
        a.scenario_id
            .cmp(&b.scenario_id)
            .then_with(|| a.action_id.cmp(&b.action_id))
    });
    excluded.sort_by(|a, b| {
        a.scenario_id
            .cmp(&b.scenario_id)
            .then_with(|| a.action_id.cmp(&b.action_id))
            .then_with(|| a.reason_code.cmp(&b.reason_code))
    });

    let selected_action_id = bounds
        .first()
        .filter(|item| item.positive_net_benefit_in_all_scenarios)
        .map(|item| item.action_id.clone());

    let disposition = if selected_action_id.is_some() {
        RobustEVSIDispositionV0_1::RobustRankedActions
    } else if bounds.is_empty() {
        RobustEVSIDispositionV0_1::AbstainNoCommonQualifiedActions
    } else {
        RobustEVSIDispositionV0_1::NoRobustPositiveNetBenefit
    };
    let mut reason_codes = BTreeSet::new();
    if selected_action_id.is_none() {
        reason_codes.insert(match disposition {
            RobustEVSIDispositionV0_1::AbstainNoCommonQualifiedActions => {
                "NO_ACTION_QUALIFIED_IN_EVERY_SCENARIO"
            }
            RobustEVSIDispositionV0_1::NoRobustPositiveNetBenefit => {
                "NO_ACTION_HAS_POSITIVE_LOWER_NET_BENEFIT"
            }
            _ => "NO_ROBUST_ACTION_SELECTED",
        }.to_owned());
    }

    let anchor = &input.scenarios[0].input;
    RobustEvidenceActionPlannerAssessmentV0_1 {
        disposition,
        scenario_set_id: input.scenario_set_id.clone(),
        scenario_set_sha256: input.scenario_set_sha256.clone(),
        approval_receipt_sha256: input.approval_receipt.as_ref().unwrap().receipt_sha256.clone(),
        scenario_count: input.scenarios.len(),
        claim_id: anchor.claim_id.clone(),
        frozen_claim_disposition: anchor.frozen_claim_disposition.clone(),
        configuration_ref: anchor.configuration_ref.clone(),
        as_known_at_unix_ms: anchor.as_known_at_unix_ms,
        decision_context_sha256: anchor.decision_context_sha256.clone(),
        assurance_state_sha256: anchor.assurance_state_sha256.clone(),
        assurance_state_receipt_sha256: anchor.assurance_state_receipt.as_ref().unwrap().receipt_sha256.clone(),
        unresolved_obligation_manifest_sha256: anchor.unresolved_obligation_manifest_sha256.clone(),
        action_catalogue_sha256: anchor.action_catalogue_sha256.clone(),
        action_catalogue_receipt_sha256: anchor.action_catalogue_receipt.as_ref().unwrap().receipt_sha256.clone(),
        scenario_action_results,
        ranked_actions: bounds,
        excluded_actions: excluded,
        reason_codes: reason_codes.into_iter().collect(),
        selected_action_id,
        evsi_numerator_scale: EVSI_NUMERATOR_SCALE_V0_1,
        assurance_state_mutated: false,
        physical_work_authorized: false,
        engineering_acceptance_granted: false,
    }
}
