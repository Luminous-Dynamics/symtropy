// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Authority-bounded expected-value-of-sample-information (EVSI) ranking.
//!
//! This module is a synthetic, deterministic decision-analysis kernel. It
//! ranks only actions whose permission and outcome-model calibration are
//! represented as verified upstream dispositions. It does not authenticate
//! those dispositions, authorize execution, mutate an AERO claim, or establish
//! engineering / physical truth.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const EVSI_PROBABILITY_SCALE_PPM_V0_1: u32 = 1_000_000;
pub const EVSI_NUMERATOR_SCALE_V0_1: i128 = 1_000_000_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ActionAuthorizationStatusV0_1 {
    #[serde(rename = "VERIFIED_PERMITTED")]
    VerifiedPermitted,
    #[serde(rename = "DENIED")]
    Denied,
    #[serde(rename = "UNKNOWN")]
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum OutcomeModelCalibrationStatusV0_1 {
    #[serde(rename = "QUALIFIED_FOR_DECLARED_PURPOSE")]
    QualifiedForDeclaredPurpose,
    #[serde(rename = "UNVERIFIED")]
    Unverified,
    #[serde(rename = "OUT_OF_DOMAIN")]
    OutOfDomain,
    #[serde(rename = "NOT_ESTIMABLE")]
    NotEstimable,
}

/// Owner-issued permission receipt bound to one exact claim/action context.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActionAuthorizationReceiptV0_1 {
    pub owner_namespace: String,
    pub claim_id: String,
    pub decision_context_sha256: String,
    pub configuration_ref: String,
    pub action_id: String,
    pub action_catalogue_entry_sha256: String,
    pub receipt_sha256: String,
    pub receiver_first_seen_at_unix_ms: u64,
    pub effective_from_unix_ms: u64,
    pub effective_until_unix_ms: u64,
}

/// Owner-issued calibration receipt for the exact action outcome model/use.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OutcomeModelCalibrationReceiptV0_1 {
    pub owner_namespace: String,
    pub claim_id: String,
    pub decision_context_sha256: String,
    pub configuration_ref: String,
    pub action_id: String,
    pub outcome_model_sha256: String,
    pub outcome_likelihoods_sha256: String,
    pub calibration_profile_sha256: String,
    pub calibration_report_sha256: String,
    pub receiver_first_seen_at_unix_ms: u64,
    pub effective_from_unix_ms: u64,
    pub effective_until_unix_ms: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EvidenceActionPlannerDispositionV0_1 {
    #[serde(rename = "RANKED_ACTIONS")]
    RankedActions,
    #[serde(rename = "NO_POSITIVE_NET_BENEFIT")]
    NoPositiveNetBenefit,
    #[serde(rename = "ABSTAIN_NO_QUALIFIED_PERMITTED_ACTIONS")]
    AbstainNoQualifiedPermittedActions,
    #[serde(rename = "INVALID_INPUT")]
    InvalidInput,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DecisionAlternativeV0_1 {
    pub decision_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorldStateV0_1 {
    pub state_id: String,
    pub prior_probability_ppm: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StateUtilityV0_1 {
    pub decision_id: String,
    pub state_id: String,
    /// Decision utility in an explicitly declared integer unit.
    pub utility_units: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OutcomeLikelihoodV0_1 {
    pub state_id: String,
    pub outcome_id: String,
    /// P(outcome | state), expressed in parts per million.
    pub likelihood_ppm: u32,
}

/// One candidate action from an owner-controlled action catalogue.
///
/// The authorization and calibration fields are consumed dispositions from
/// their upstream owners; this planner does not verify their signatures or
/// receipts. A verified-permitted action must carry an authorization receipt
/// digest. Every action carries a catalogue-entry and expected-artifact
/// contract digest so the recommendation has a defined output boundary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceAcquisitionActionV0_1 {
    pub action_id: String,
    pub owner_namespace: String,
    pub action_catalogue_entry_sha256: String,
    pub authorization_status: ActionAuthorizationStatusV0_1,
    pub authorization_receipt: Option<ActionAuthorizationReceiptV0_1>,
    /// Model owner namespace declared by the action catalogue.
    pub outcome_model_owner_namespace: String,
    pub calibration_status: OutcomeModelCalibrationStatusV0_1,
    pub calibration_receipt: Option<OutcomeModelCalibrationReceiptV0_1>,
    pub expected_artifact_contract_sha256: String,
    pub target_obligation_ids: Vec<String>,
    /// Direct acquisition cost in the declared decision-utility unit.
    pub expected_cost_units: i64,
    /// Delay / interruption / opportunity cost in the same declared unit.
    pub expected_delay_cost_units: i64,
    /// Complete P(outcome | state) table for this action.
    pub outcome_likelihoods: Vec<OutcomeLikelihoodV0_1>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceActionPlannerInputV0_1 {
    pub claim_id: String,
    /// Passed through unchanged; the planner never recalculates or upgrades it.
    pub frozen_claim_disposition: String,
    pub configuration_ref: String,
    pub as_known_at_unix_ms: u64,
    /// Exact unresolved obligation identities exposed by the source-bound state.
    pub unresolved_obligation_ids: Vec<String>,
    pub unresolved_obligation_manifest_sha256: String,
    pub decision_context_sha256: String,
    pub assurance_state_sha256: String,
    pub action_catalogue_sha256: String,
    pub decision_model_sha256: String,
    pub outcome_model_sha256: String,
    pub decision_alternatives: Vec<DecisionAlternativeV0_1>,
    pub world_states: Vec<WorldStateV0_1>,
    /// Complete Cartesian mapping of decision × world-state utility.
    pub utilities: Vec<StateUtilityV0_1>,
    pub actions: Vec<EvidenceAcquisitionActionV0_1>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExcludedEvidenceActionV0_1 {
    pub action_id: String,
    pub reason_code: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RankedEvidenceActionV0_1 {
    pub rank: usize,
    pub action_id: String,
    pub owner_namespace: String,
    pub outcome_model_owner_namespace: String,
    pub target_obligation_ids: Vec<String>,
    pub action_catalogue_entry_sha256: String,
    pub authorization_receipt_sha256: String,
    pub calibration_receipt_sha256: String,
    pub outcome_likelihoods_sha256: String,
    pub calibration_profile_sha256: String,
    pub expected_artifact_contract_sha256: String,
    /// Exact numerator; divide by EVSI_NUMERATOR_SCALE_V0_1 for utility units.
    pub evsi_scaled_numerator: String,
    /// Direct cost and delay cost remain separately visible.
    pub expected_cost_units: i64,
    pub expected_delay_cost_units: i64,
    /// Exact net benefit numerator at the same scale as EVSI.
    pub net_benefit_scaled_numerator: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceActionPlannerAssessmentV0_1 {
    pub disposition: EvidenceActionPlannerDispositionV0_1,
    pub claim_id: String,
    pub frozen_claim_disposition: String,
    pub configuration_ref: String,
    pub as_known_at_unix_ms: u64,
    pub decision_context_sha256: String,
    pub assurance_state_sha256: String,
    pub unresolved_obligation_manifest_sha256: String,
    pub action_catalogue_sha256: String,
    pub decision_model_sha256: String,
    pub outcome_model_sha256: String,
    pub current_best_decision_id: Option<String>,
    pub selected_action_id: Option<String>,
    pub ranked_actions: Vec<RankedEvidenceActionV0_1>,
    pub excluded_actions: Vec<ExcludedEvidenceActionV0_1>,
    pub reason_codes: Vec<String>,
    pub evsi_numerator_scale: i128,
    pub assurance_state_mutated: bool,
    pub physical_work_authorized: bool,
    pub engineering_acceptance_granted: bool,
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

fn invalid_assessment(
    input: &EvidenceActionPlannerInputV0_1,
    reasons: BTreeSet<String>,
) -> EvidenceActionPlannerAssessmentV0_1 {
    EvidenceActionPlannerAssessmentV0_1 {
        disposition: EvidenceActionPlannerDispositionV0_1::InvalidInput,
        claim_id: input.claim_id.clone(),
        frozen_claim_disposition: input.frozen_claim_disposition.clone(),
        configuration_ref: input.configuration_ref.clone(),
        as_known_at_unix_ms: input.as_known_at_unix_ms,
        decision_context_sha256: input.decision_context_sha256.clone(),
        assurance_state_sha256: input.assurance_state_sha256.clone(),
        unresolved_obligation_manifest_sha256: input.unresolved_obligation_manifest_sha256.clone(),
        action_catalogue_sha256: input.action_catalogue_sha256.clone(),
        decision_model_sha256: input.decision_model_sha256.clone(),
        outcome_model_sha256: input.outcome_model_sha256.clone(),
        current_best_decision_id: None,
        selected_action_id: None,
        ranked_actions: Vec::new(),
        excluded_actions: Vec::new(),
        reason_codes: reasons.into_iter().collect(),
        evsi_numerator_scale: EVSI_NUMERATOR_SCALE_V0_1,
        assurance_state_mutated: false,
        physical_work_authorized: false,
        engineering_acceptance_granted: false,
    }
}

fn utility_map(
    input: &EvidenceActionPlannerInputV0_1,
) -> BTreeMap<(&str, &str), i64> {
    input
        .utilities
        .iter()
        .map(|item| {
            (
                (item.decision_id.as_str(), item.state_id.as_str()),
                item.utility_units,
            )
        })
        .collect()
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

#[derive(Serialize)]
struct DecisionModelCommitmentV0_1<'a> {
    schema: &'static str,
    canonicalization_profile: &'static str,
    decision_alternatives: Vec<&'a DecisionAlternativeV0_1>,
    world_states: Vec<&'a WorldStateV0_1>,
    utilities: Vec<&'a StateUtilityV0_1>,
}

fn compute_decision_model_sha256(
    input: &EvidenceActionPlannerInputV0_1,
) -> Result<String, String> {
    let mut decisions = input.decision_alternatives.iter().collect::<Vec<_>>();
    decisions.sort_by(|left, right| left.decision_id.cmp(&right.decision_id));
    let mut states = input.world_states.iter().collect::<Vec<_>>();
    states.sort_by(|left, right| left.state_id.cmp(&right.state_id));
    let mut utilities = input.utilities.iter().collect::<Vec<_>>();
    utilities.sort_by(|left, right| {
        (&left.decision_id, &left.state_id).cmp(&(&right.decision_id, &right.state_id))
    });
    let envelope = DecisionModelCommitmentV0_1 {
        schema: "AeroEvidenceDecisionModelV0_1",
        canonicalization_profile: "rust-serde-json-struct-order-v1",
        decision_alternatives: decisions,
        world_states: states,
        utilities,
    };
    serde_json::to_vec(&envelope)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|_| "EVSI_DECISION_MODEL_SERIALIZATION_FAILED".to_owned())
}

#[derive(Serialize)]
struct ActionCatalogueEntryEnvelopeV0_1<'a> {
    schema: &'static str,
    canonicalization_profile: &'static str,
    action_id: &'a str,
    owner_namespace: &'a str,
    outcome_model_owner_namespace: &'a str,
    expected_artifact_contract_sha256: &'a str,
    target_obligation_ids: Vec<&'a str>,
    expected_cost_units: i64,
    expected_delay_cost_units: i64,
}

fn compute_action_catalogue_entry_sha256(
    action: &EvidenceAcquisitionActionV0_1,
) -> Result<String, String> {
    let mut targets = action
        .target_obligation_ids
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    targets.sort_unstable();
    let envelope = ActionCatalogueEntryEnvelopeV0_1 {
        schema: "AeroEvidenceActionCatalogueEntryV0_1",
        canonicalization_profile: "rust-serde-json-struct-order-v1",
        action_id: &action.action_id,
        owner_namespace: &action.owner_namespace,
        outcome_model_owner_namespace: &action.outcome_model_owner_namespace,
        expected_artifact_contract_sha256: &action.expected_artifact_contract_sha256,
        target_obligation_ids: targets,
        expected_cost_units: action.expected_cost_units,
        expected_delay_cost_units: action.expected_delay_cost_units,
    };
    serde_json::to_vec(&envelope)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|_| "EVSI_ACTION_CATALOGUE_ENTRY_SERIALIZATION_FAILED".to_owned())
}

#[derive(Serialize)]
struct ActionCatalogueCutEntryV0_1<'a> {
    action_id: &'a str,
    owner_namespace: &'a str,
    outcome_model_owner_namespace: &'a str,
    action_catalogue_entry_sha256: &'a str,
}

#[derive(Serialize)]
struct ActionCatalogueCutEnvelopeV0_1<'a> {
    schema: &'static str,
    canonicalization_profile: &'static str,
    entries: Vec<ActionCatalogueCutEntryV0_1<'a>>,
}

fn compute_action_catalogue_cut_sha256(
    input: &EvidenceActionPlannerInputV0_1,
) -> Result<String, String> {
    let mut actions = input.actions.iter().collect::<Vec<_>>();
    actions.sort_by(|left, right| left.action_id.cmp(&right.action_id));
    let entries = actions
        .iter()
        .map(|action| ActionCatalogueCutEntryV0_1 {
            action_id: &action.action_id,
            owner_namespace: &action.owner_namespace,
            outcome_model_owner_namespace: &action.outcome_model_owner_namespace,
            action_catalogue_entry_sha256: &action.action_catalogue_entry_sha256,
        })
        .collect();
    let envelope = ActionCatalogueCutEnvelopeV0_1 {
        schema: "AeroEvidenceActionCatalogueCutV0_1",
        canonicalization_profile: "rust-serde-json-struct-order-v1",
        entries,
    };
    serde_json::to_vec(&envelope)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|_| "EVSI_ACTION_CATALOGUE_SERIALIZATION_FAILED".to_owned())
}

#[derive(Serialize)]
struct OutcomeLikelihoodCommitmentV0_1<'a> {
    schema: &'static str,
    canonicalization_profile: &'static str,
    action_id: &'a str,
    outcome_model_sha256: &'a str,
    likelihoods: Vec<&'a OutcomeLikelihoodV0_1>,
}

fn compute_outcome_likelihoods_sha256(
    action: &EvidenceAcquisitionActionV0_1,
    outcome_model_sha256: &str,
) -> Result<String, String> {
    let mut likelihoods = action.outcome_likelihoods.iter().collect::<Vec<_>>();
    likelihoods.sort_by(|left, right| {
        (&left.state_id, &left.outcome_id).cmp(&(&right.state_id, &right.outcome_id))
    });
    let envelope = OutcomeLikelihoodCommitmentV0_1 {
        schema: "AeroOutcomeLikelihoodTableV0_1",
        canonicalization_profile: "rust-serde-json-struct-order-v1",
        action_id: &action.action_id,
        outcome_model_sha256,
        likelihoods,
    };
    serde_json::to_vec(&envelope)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|_| "EVSI_OUTCOME_LIKELIHOOD_SERIALIZATION_FAILED".to_owned())
}

#[derive(Serialize)]
struct UnresolvedObligationManifestV0_1<'a> {
    schema: &'static str,
    canonicalization_profile: &'static str,
    obligation_ids: Vec<&'a str>,
}

fn compute_unresolved_obligation_manifest_sha256(
    obligation_ids: &[String],
) -> Result<String, String> {
    let mut ids = obligation_ids.iter().map(String::as_str).collect::<Vec<_>>();
    ids.sort_unstable();
    let envelope = UnresolvedObligationManifestV0_1 {
        schema: "AeroUnresolvedObligationManifestV0_1",
        canonicalization_profile: "sorted-unique-identifiers-v1",
        obligation_ids: ids,
    };
    serde_json::to_vec(&envelope)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|_| "EVSI_UNRESOLVED_OBLIGATION_SERIALIZATION_FAILED".to_owned())
}

fn validate_input(input: &EvidenceActionPlannerInputV0_1) -> BTreeSet<String> {
    let mut errors = BTreeSet::new();
    for (code, value) in [
        ("CLAIM_ID", input.claim_id.as_str()),
        ("CLAIM_DISPOSITION", input.frozen_claim_disposition.as_str()),
        ("CONFIGURATION_REF", input.configuration_ref.as_str()),
    ] {
        if !valid_identifier(value) {
            errors.insert(format!("EVSI_{code}_INVALID"));
        }
    }
    for (code, value) in [
        ("DECISION_CONTEXT", input.decision_context_sha256.as_str()),
        ("ASSURANCE_STATE", input.assurance_state_sha256.as_str()),
        ("UNRESOLVED_OBLIGATION_MANIFEST", input.unresolved_obligation_manifest_sha256.as_str()),
        ("ACTION_CATALOGUE", input.action_catalogue_sha256.as_str()),
        ("DECISION_MODEL", input.decision_model_sha256.as_str()),
        ("OUTCOME_MODEL", input.outcome_model_sha256.as_str()),
    ] {
        if !valid_sha256(value) {
            errors.insert(format!("EVSI_{code}_DIGEST_INVALID"));
        }
    }

    if input.decision_alternatives.len() < 2 {
        errors.insert("EVSI_REQUIRES_AT_LEAST_TWO_DECISION_ALTERNATIVES".to_owned());
    }
    if input.world_states.is_empty() {
        errors.insert("EVSI_WORLD_STATE_SET_EMPTY".to_owned());
    }
    let mut unresolved_ids = BTreeSet::new();
    for id in &input.unresolved_obligation_ids {
        if !valid_identifier(id) || !unresolved_ids.insert(id.as_str()) {
            errors.insert("EVSI_UNRESOLVED_OBLIGATION_ID_INVALID_OR_DUPLICATE".to_owned());
        }
    }
    if unresolved_ids.is_empty() {
        errors.insert("EVSI_UNRESOLVED_OBLIGATION_SET_EMPTY".to_owned());
    }
    if compute_unresolved_obligation_manifest_sha256(&input.unresolved_obligation_ids)
        .as_deref()
        != Ok(input.unresolved_obligation_manifest_sha256.as_str())
    {
        errors.insert("EVSI_UNRESOLVED_OBLIGATION_MANIFEST_MISMATCH".to_owned());
    }
    if compute_decision_model_sha256(input).as_deref()
        != Ok(input.decision_model_sha256.as_str())
    {
        errors.insert("EVSI_DECISION_MODEL_COMMITMENT_MISMATCH".to_owned());
    }
    if compute_action_catalogue_cut_sha256(input).as_deref()
        != Ok(input.action_catalogue_sha256.as_str())
    {
        errors.insert("EVSI_ACTION_CATALOGUE_COMMITMENT_MISMATCH".to_owned());
    }

    let mut decision_ids = BTreeSet::new();
    for item in &input.decision_alternatives {
        if !valid_identifier(&item.decision_id) || !decision_ids.insert(item.decision_id.as_str()) {
            errors.insert("EVSI_DECISION_ID_INVALID_OR_DUPLICATE".to_owned());
        }
    }

    let mut state_ids = BTreeSet::new();
    let mut prior_total = 0_u64;
    for item in &input.world_states {
        if !valid_identifier(&item.state_id) || !state_ids.insert(item.state_id.as_str()) {
            errors.insert("EVSI_WORLD_STATE_ID_INVALID_OR_DUPLICATE".to_owned());
        }
        if item.prior_probability_ppm > EVSI_PROBABILITY_SCALE_PPM_V0_1 {
            errors.insert("EVSI_PRIOR_PROBABILITY_OUT_OF_RANGE".to_owned());
        }
        prior_total += u64::from(item.prior_probability_ppm);
    }
    if prior_total != u64::from(EVSI_PROBABILITY_SCALE_PPM_V0_1) {
        errors.insert("EVSI_PRIOR_PROBABILITIES_DO_NOT_SUM_TO_ONE".to_owned());
    }

    let expected_utility_rows =
        input.decision_alternatives.len().saturating_mul(input.world_states.len());
    if input.utilities.len() != expected_utility_rows {
        errors.insert("EVSI_UTILITY_MATRIX_INCOMPLETE_OR_OVERSPECIFIED".to_owned());
    }
    let mut utility_keys = BTreeSet::new();
    for item in &input.utilities {
        if !decision_ids.contains(item.decision_id.as_str())
            || !state_ids.contains(item.state_id.as_str())
        {
            errors.insert("EVSI_UTILITY_REFERENCES_UNKNOWN_ID".to_owned());
        }
        if !utility_keys.insert((item.decision_id.as_str(), item.state_id.as_str())) {
            errors.insert("EVSI_UTILITY_CELL_DUPLICATED".to_owned());
        }
    }

    let mut action_ids = BTreeSet::new();
    for action in &input.actions {
        if !valid_identifier(&action.action_id)
            || !action_ids.insert(action.action_id.as_str())
        {
            errors.insert("EVSI_ACTION_ID_INVALID_OR_DUPLICATE".to_owned());
        }
        if !valid_identifier(&action.owner_namespace)
            || !valid_identifier(&action.outcome_model_owner_namespace)
        {
            errors.insert("EVSI_ACTION_OWNER_INVALID".to_owned());
        }
        if !valid_sha256(&action.action_catalogue_entry_sha256)
            || !valid_sha256(&action.expected_artifact_contract_sha256)
        {
            errors.insert("EVSI_ACTION_CATALOGUE_OR_ARTIFACT_DIGEST_INVALID".to_owned());
        }
        if compute_action_catalogue_entry_sha256(action).as_deref()
            != Ok(action.action_catalogue_entry_sha256.as_str())
        {
            errors.insert("EVSI_ACTION_CATALOGUE_ENTRY_COMMITMENT_MISMATCH".to_owned());
        }
        if action.expected_cost_units < 0 || action.expected_delay_cost_units < 0 {
            errors.insert("EVSI_ACTION_COST_NEGATIVE".to_owned());
        }
        if action.target_obligation_ids.is_empty() {
            errors.insert("EVSI_ACTION_HAS_NO_TARGET_OBLIGATIONS".to_owned());
        }
        let mut target_ids = BTreeSet::new();
        for id in &action.target_obligation_ids {
            if !valid_identifier(id) || !target_ids.insert(id.as_str()) {
                errors.insert("EVSI_ACTION_TARGET_OBLIGATION_INVALID_OR_DUPLICATE".to_owned());
            } else if !unresolved_ids.contains(id.as_str()) {
                errors.insert("EVSI_ACTION_TARGET_IS_NOT_A_DECLARED_UNRESOLVED_OBLIGATION".to_owned());
            }
        }
        if action.authorization_status == ActionAuthorizationStatusV0_1::VerifiedPermitted
            && action.authorization_receipt.is_none()
        {
            errors.insert("EVSI_VERIFIED_ACTION_MISSING_AUTHORIZATION_RECEIPT".to_owned());
        }
        if let Some(receipt) = &action.authorization_receipt {
            if !valid_identifier(&receipt.owner_namespace)
                || !valid_identifier(&receipt.claim_id)
                || !valid_identifier(&receipt.configuration_ref)
                || !valid_identifier(&receipt.action_id)
                || !valid_sha256(&receipt.decision_context_sha256)
                || !valid_sha256(&receipt.action_catalogue_entry_sha256)
                || !valid_sha256(&receipt.receipt_sha256)
            {
                errors.insert("EVSI_ACTION_AUTHORIZATION_RECEIPT_INVALID".to_owned());
            }
            if receipt.effective_from_unix_ms >= receipt.effective_until_unix_ms {
                errors.insert("EVSI_ACTION_AUTHORIZATION_INTERVAL_INVALID".to_owned());
            }
        }
        if action.calibration_status
            == OutcomeModelCalibrationStatusV0_1::QualifiedForDeclaredPurpose
            && action.calibration_receipt.is_none()
        {
            errors.insert("EVSI_QUALIFIED_OUTCOME_MODEL_MISSING_CALIBRATION_RECEIPT".to_owned());
        }
        if let Some(receipt) = &action.calibration_receipt {
            if !valid_identifier(&receipt.owner_namespace)
                || !valid_identifier(&receipt.claim_id)
                || !valid_identifier(&receipt.configuration_ref)
                || !valid_identifier(&receipt.action_id)
                || !valid_sha256(&receipt.decision_context_sha256)
                || !valid_sha256(&receipt.outcome_model_sha256)
                || !valid_sha256(&receipt.outcome_likelihoods_sha256)
                || !valid_sha256(&receipt.calibration_profile_sha256)
                || !valid_sha256(&receipt.calibration_report_sha256)
            {
                errors.insert("EVSI_ACTION_CALIBRATION_RECEIPT_INVALID".to_owned());
            }
            if receipt.effective_from_unix_ms >= receipt.effective_until_unix_ms {
                errors.insert("EVSI_ACTION_CALIBRATION_INTERVAL_INVALID".to_owned());
            }
        }

        let mut outcomes = BTreeSet::new();
        let mut likelihood_keys = BTreeSet::new();
        let mut sums_by_state: BTreeMap<&str, u64> = BTreeMap::new();
        for likelihood in &action.outcome_likelihoods {
            if !state_ids.contains(likelihood.state_id.as_str())
                || !valid_identifier(&likelihood.outcome_id)
            {
                errors.insert("EVSI_ACTION_OUTCOME_REFERENCES_UNKNOWN_OR_INVALID_ID".to_owned());
            }
            if likelihood.likelihood_ppm > EVSI_PROBABILITY_SCALE_PPM_V0_1 {
                errors.insert("EVSI_ACTION_OUTCOME_PROBABILITY_OUT_OF_RANGE".to_owned());
            }
            outcomes.insert(likelihood.outcome_id.as_str());
            if !likelihood_keys.insert((likelihood.state_id.as_str(), likelihood.outcome_id.as_str())) {
                errors.insert("EVSI_ACTION_OUTCOME_CELL_DUPLICATED".to_owned());
            }
            *sums_by_state.entry(likelihood.state_id.as_str()).or_default() +=
                u64::from(likelihood.likelihood_ppm);
        }
        if action.outcome_likelihoods.is_empty() {
            errors.insert("EVSI_ACTION_OUTCOME_TABLE_EMPTY".to_owned());
        }
        if action.outcome_likelihoods.len() != state_ids.len().saturating_mul(outcomes.len()) {
            errors.insert("EVSI_ACTION_OUTCOME_MATRIX_INCOMPLETE".to_owned());
        }
        for state_id in &state_ids {
            if sums_by_state.get(state_id).copied() != Some(u64::from(EVSI_PROBABILITY_SCALE_PPM_V0_1)) {
                errors.insert("EVSI_ACTION_OUTCOME_LIKELIHOODS_DO_NOT_SUM_TO_ONE_PER_STATE".to_owned());
            }
        }
    }

    errors
}

/// Rank permitted, calibrated acquisition actions by exact EVSI minus cost.
///
/// Probabilities are integer parts-per-million and expected utility is
/// represented by integer `utility_units`. For each action:
///
/// `EVSI = sum_outcome max_decision E[utility | outcome] - max_decision E[utility]`.
///
/// The returned scaled numerator has denominator 10^12 (PPM × PPM). Costs are
/// multiplied by that denominator before comparison, so ranking has no
/// floating-point rounding. Ties are resolved by net benefit, EVSI, cost,
/// delay cost, then canonical action ID.
///
/// Permission and calibration are owner-provided dispositions. Unknown,
/// denied, unverified, out-of-domain or non-estimable actions are excluded.
/// This function only emits recommendations; the frozen claim disposition and
/// assurance-state digest are copied unchanged into the output.
pub fn rank_evidence_actions_v0_1(
    input: &EvidenceActionPlannerInputV0_1,
) -> EvidenceActionPlannerAssessmentV0_1 {
    let validation_errors = validate_input(input);
    if !validation_errors.is_empty() {
        return invalid_assessment(input, validation_errors);
    }

    let decisions = input
        .decision_alternatives
        .iter()
        .map(|item| item.decision_id.as_str())
        .collect::<Vec<_>>();
    let states = input
        .world_states
        .iter()
        .map(|item| item.state_id.as_str())
        .collect::<Vec<_>>();
    let priors = input
        .world_states
        .iter()
        .map(|item| (item.state_id.as_str(), item.prior_probability_ppm))
        .collect::<BTreeMap<_, _>>();
    let utilities = utility_map(input);

    let mut prior_scores = BTreeMap::new();
    for decision in &decisions {
        let mut score = 0_i128;
        for state in &states {
            let prior = i128::from(priors[state]);
            let utility = i128::from(utilities[&(*decision, *state)]);
            let Some(term) = prior.checked_mul(utility) else {
                return invalid_assessment(
                    input,
                    BTreeSet::from(["EVSI_ARITHMETIC_OVERFLOW".to_owned()]),
                );
            };
            let Some(next) = score.checked_add(term) else {
                return invalid_assessment(
                    input,
                    BTreeSet::from(["EVSI_ARITHMETIC_OVERFLOW".to_owned()]),
                );
            };
            score = next;
        }
        prior_scores.insert(*decision, score);
    }
    let current_best_decision = prior_scores
        .iter()
        .max_by(|(left_id, left_score), (right_id, right_score)| {
            left_score.cmp(right_score).then_with(|| right_id.cmp(left_id))
        })
        .map(|(id, _)| *id);
    let Some(current_best_decision) = current_best_decision else {
        return invalid_assessment(
            input,
            BTreeSet::from(["EVSI_CURRENT_DECISION_NOT_ESTIMABLE".to_owned()]),
        );
    };
    let current_best_score = prior_scores[current_best_decision];

    let mut excluded_actions = Vec::new();
    let mut ranked_internal: Vec<(i128, i128, i64, i64, &EvidenceAcquisitionActionV0_1)> =
        Vec::new();

    for action in &input.actions {
        let authorization_reason = match action.authorization_status {
            ActionAuthorizationStatusV0_1::Denied => Some("ACTION_AUTHORITY_DENIED"),
            ActionAuthorizationStatusV0_1::Unknown => Some("ACTION_AUTHORITY_UNKNOWN"),
            ActionAuthorizationStatusV0_1::VerifiedPermitted => {
                let receipt = action.authorization_receipt.as_ref().expect(
                    "verified-permitted action receipt was checked during input validation",
                );
                if receipt.owner_namespace != action.owner_namespace
                    || receipt.claim_id != input.claim_id
                    || receipt.decision_context_sha256 != input.decision_context_sha256
                    || receipt.configuration_ref != input.configuration_ref
                    || receipt.action_id != action.action_id
                    || receipt.action_catalogue_entry_sha256
                        != action.action_catalogue_entry_sha256
                {
                    Some("ACTION_AUTHORIZATION_SCOPE_MISMATCH")
                } else if receipt.receiver_first_seen_at_unix_ms > input.as_known_at_unix_ms {
                    Some("ACTION_AUTHORIZATION_NOT_KNOWN_AT_CUTOFF")
                } else if receipt.effective_from_unix_ms > input.as_known_at_unix_ms
                    || receipt.effective_until_unix_ms <= input.as_known_at_unix_ms
                {
                    Some("ACTION_AUTHORIZATION_NOT_EFFECTIVE_AT_CUTOFF")
                } else {
                    None
                }
            }
        };
        let calibration_reason = match action.calibration_status {
            OutcomeModelCalibrationStatusV0_1::QualifiedForDeclaredPurpose => {
                let receipt = action.calibration_receipt.as_ref().expect(
                    "qualified outcome-model receipt was checked during input validation",
                );
                if receipt.owner_namespace != action.outcome_model_owner_namespace
                    || receipt.claim_id != input.claim_id
                    || receipt.decision_context_sha256 != input.decision_context_sha256
                    || receipt.configuration_ref != input.configuration_ref
                    || receipt.action_id != action.action_id
                    || receipt.outcome_model_sha256 != input.outcome_model_sha256
                    || compute_outcome_likelihoods_sha256(action, &input.outcome_model_sha256)
                        .as_deref()
                        != Ok(receipt.outcome_likelihoods_sha256.as_str())
                {
                    Some("ACTION_CALIBRATION_SCOPE_MISMATCH")
                } else if receipt.receiver_first_seen_at_unix_ms > input.as_known_at_unix_ms {
                    Some("ACTION_CALIBRATION_NOT_KNOWN_AT_CUTOFF")
                } else if receipt.effective_from_unix_ms > input.as_known_at_unix_ms
                    || receipt.effective_until_unix_ms <= input.as_known_at_unix_ms
                {
                    Some("ACTION_CALIBRATION_NOT_EFFECTIVE_AT_CUTOFF")
                } else {
                    None
                }
            }
            OutcomeModelCalibrationStatusV0_1::Unverified => {
                Some("ACTION_OUTCOME_MODEL_UNVERIFIED")
            }
            OutcomeModelCalibrationStatusV0_1::OutOfDomain => {
                Some("ACTION_OUTCOME_MODEL_OUT_OF_DOMAIN")
            }
            OutcomeModelCalibrationStatusV0_1::NotEstimable => {
                Some("ACTION_OUTCOME_MODEL_NOT_ESTIMABLE")
            }
        };
        let exclusion_reason = authorization_reason.or(calibration_reason);
        if let Some(reason_code) = exclusion_reason {
            excluded_actions.push(ExcludedEvidenceActionV0_1 {
                action_id: action.action_id.clone(),
                reason_code: reason_code.to_owned(),
            });
            continue;
        }

        let outcome_ids = action
            .outcome_likelihoods
            .iter()
            .map(|item| item.outcome_id.as_str())
            .collect::<BTreeSet<_>>();
        let mut posterior_value_scaled = 0_i128;
        for outcome_id in outcome_ids {
            let mut best_outcome_score = None;
            for decision in &decisions {
                let mut score = 0_i128;
                for state in &states {
                    let likelihood = action
                        .outcome_likelihoods
                        .iter()
                        .find(|item| {
                            item.state_id == *state && item.outcome_id == outcome_id
                        })
                        .map(|item| item.likelihood_ppm)
                        .unwrap_or(0);
                    let prior = i128::from(priors[state]);
                    let probability = i128::from(likelihood);
                    let utility = i128::from(utilities[&(*decision, *state)]);
                    let Some(term) = prior
                        .checked_mul(probability)
                        .and_then(|value| value.checked_mul(utility))
                    else {
                        return invalid_assessment(
                            input,
                            BTreeSet::from(["EVSI_ARITHMETIC_OVERFLOW".to_owned()]),
                        );
                    };
                    let Some(next) = score.checked_add(term) else {
                        return invalid_assessment(
                            input,
                            BTreeSet::from(["EVSI_ARITHMETIC_OVERFLOW".to_owned()]),
                        );
                    };
                    score = next;
                }
                best_outcome_score = Some(best_outcome_score.map_or(score, |best: i128| best.max(score)));
            }
            let Some(best) = best_outcome_score else {
                return invalid_assessment(
                    input,
                    BTreeSet::from(["EVSI_OUTCOME_NOT_ESTIMABLE".to_owned()]),
                );
            };
            let Some(next) = posterior_value_scaled.checked_add(best) else {
                return invalid_assessment(
                    input,
                    BTreeSet::from(["EVSI_ARITHMETIC_OVERFLOW".to_owned()]),
                );
            };
            posterior_value_scaled = next;
        }

        let Some(current_scaled) = current_best_score.checked_mul(EVSI_NUMERATOR_SCALE_V0_1 / 1_000_000) else {
            return invalid_assessment(
                input,
                BTreeSet::from(["EVSI_ARITHMETIC_OVERFLOW".to_owned()]),
            );
        };
        let Some(evsi_scaled) = posterior_value_scaled.checked_sub(current_scaled) else {
            return invalid_assessment(
                input,
                BTreeSet::from(["EVSI_ARITHMETIC_OVERFLOW".to_owned()]),
            );
        };
        if evsi_scaled < 0 {
            return invalid_assessment(
                input,
                BTreeSet::from(["EVSI_NEGATIVE_INFORMATION_VALUE"].to_owned()),
            );
        }

        let total_cost = i128::from(action.expected_cost_units)
            + i128::from(action.expected_delay_cost_units);
        let Some(cost_scaled) = total_cost.checked_mul(EVSI_NUMERATOR_SCALE_V0_1) else {
            return invalid_assessment(
                input,
                BTreeSet::from(["EVSI_ARITHMETIC_OVERFLOW".to_owned()]),
            );
        };
        let Some(net_benefit_scaled) = evsi_scaled.checked_sub(cost_scaled) else {
            return invalid_assessment(
                input,
                BTreeSet::from(["EVSI_ARITHMETIC_OVERFLOW".to_owned()]),
            );
        };
        ranked_internal.push((
            net_benefit_scaled,
            evsi_scaled,
            action.expected_cost_units,
            action.expected_delay_cost_units,
            action,
        ));
    }

    excluded_actions.sort_by(|left, right| left.action_id.cmp(&right.action_id));
    ranked_internal.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| right.1.cmp(&left.1))
            .then_with(|| left.2.cmp(&right.2))
            .then_with(|| left.3.cmp(&right.3))
            .then_with(|| left.4.action_id.cmp(&right.4.action_id))
    });

    let mut ranked_actions = Vec::new();
    for (index, (net_benefit, evsi, cost, delay_cost, action)) in
        ranked_internal.iter().enumerate()
    {
        ranked_actions.push(RankedEvidenceActionV0_1 {
            rank: index + 1,
            action_id: action.action_id.clone(),
            owner_namespace: action.owner_namespace.clone(),
            outcome_model_owner_namespace: action.outcome_model_owner_namespace.clone(),
            target_obligation_ids: action.target_obligation_ids.clone(),
            action_catalogue_entry_sha256: action.action_catalogue_entry_sha256.clone(),
            authorization_receipt_sha256: action
                .authorization_receipt
                .as_ref()
                .map(|receipt| receipt.receipt_sha256.clone())
                .unwrap_or_default(),
            calibration_receipt_sha256: action
                .calibration_receipt
                .as_ref()
                .map(|receipt| receipt.calibration_report_sha256.clone())
                .unwrap_or_default(),
            outcome_likelihoods_sha256: action
                .calibration_receipt
                .as_ref()
                .map(|receipt| receipt.outcome_likelihoods_sha256.clone())
                .unwrap_or_default(),
            calibration_profile_sha256: action
                .calibration_receipt
                .as_ref()
                .map(|receipt| receipt.calibration_profile_sha256.clone())
                .unwrap_or_default(),
            expected_artifact_contract_sha256: action.expected_artifact_contract_sha256.clone(),
            evsi_scaled_numerator: evsi.to_string(),
            expected_cost_units: *cost,
            expected_delay_cost_units: *delay_cost,
            net_benefit_scaled_numerator: net_benefit.to_string(),
        });
    }

    let selected_action_id = ranked_internal
        .first()
        .filter(|item| item.0 > 0)
        .map(|item| item.4.action_id.clone());

    let disposition = if selected_action_id.is_some() {
        EvidenceActionPlannerDispositionV0_1::RankedActions
    } else if ranked_internal.is_empty() {
        EvidenceActionPlannerDispositionV0_1::AbstainNoQualifiedPermittedActions
    } else {
        EvidenceActionPlannerDispositionV0_1::NoPositiveNetBenefit
    };

    let mut reason_codes = BTreeSet::new();
    if selected_action_id.is_none() {
        reason_codes.insert(match disposition {
            EvidenceActionPlannerDispositionV0_1::AbstainNoQualifiedPermittedActions => {
                "NO_QUALIFIED_PERMITTED_EVIDENCE_ACTION"
            }
            EvidenceActionPlannerDispositionV0_1::NoPositiveNetBenefit => {
                "NO_ACTION_HAS_POSITIVE_EXPECTED_NET_BENEFIT"
            }
            _ => "NO_ACTION_SELECTED",
        }
        .to_owned());
    }

    EvidenceActionPlannerAssessmentV0_1 {
        disposition,
        claim_id: input.claim_id.clone(),
        frozen_claim_disposition: input.frozen_claim_disposition.clone(),
        configuration_ref: input.configuration_ref.clone(),
        as_known_at_unix_ms: input.as_known_at_unix_ms,
        decision_context_sha256: input.decision_context_sha256.clone(),
        assurance_state_sha256: input.assurance_state_sha256.clone(),
        unresolved_obligation_manifest_sha256: input.unresolved_obligation_manifest_sha256.clone(),
        action_catalogue_sha256: input.action_catalogue_sha256.clone(),
        decision_model_sha256: input.decision_model_sha256.clone(),
        outcome_model_sha256: input.outcome_model_sha256.clone(),
        current_best_decision_id: Some(current_best_decision.to_owned()),
        selected_action_id,
        ranked_actions,
        excluded_actions,
        reason_codes: reason_codes.into_iter().collect(),
        evsi_numerator_scale: EVSI_NUMERATOR_SCALE_V0_1,
        assurance_state_mutated: false,
        physical_work_authorized: false,
        engineering_acceptance_granted: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn base_input() -> EvidenceActionPlannerInputV0_1 {
        let action = EvidenceAcquisitionActionV0_1 {
            action_id: "action:request-calibration-report".to_owned(),
            owner_namespace: "owner:metrology-lab".to_owned(),
            outcome_model_owner_namespace: "owner:forecast-model-team".to_owned(),
            action_catalogue_entry_sha256: digest('1'),
            authorization_status: ActionAuthorizationStatusV0_1::VerifiedPermitted,
            authorization_receipt: Some(ActionAuthorizationReceiptV0_1 {
                owner_namespace: "owner:metrology-lab".to_owned(),
                claim_id: "claim:wing-panel-proof-load".to_owned(),
                decision_context_sha256: digest('a'),
                configuration_ref: "configuration:wing-r17".to_owned(),
                action_id: "action:request-calibration-report".to_owned(),
                action_catalogue_entry_sha256: digest('1'),
                receipt_sha256: digest('2'),
                receiver_first_seen_at_unix_ms: 1_795_516_000_000,
                effective_from_unix_ms: 1_795_500_000_000,
                effective_until_unix_ms: 1_798_000_000_000,
            }),
            calibration_status: OutcomeModelCalibrationStatusV0_1::QualifiedForDeclaredPurpose,
            calibration_receipt: Some(OutcomeModelCalibrationReceiptV0_1 {
                owner_namespace: "owner:forecast-model-team".to_owned(),
                claim_id: "claim:wing-panel-proof-load".to_owned(),
                decision_context_sha256: digest('a'),
                configuration_ref: "configuration:wing-r17".to_owned(),
                action_id: "action:request-calibration-report".to_owned(),
                outcome_model_sha256: digest('e'),
                outcome_likelihoods_sha256: "PLACEHOLDER_LIKELIHOOD_DIGEST".to_owned(),
                calibration_profile_sha256: digest('6'),
                calibration_report_sha256: digest('7'),
                receiver_first_seen_at_unix_ms: 1_795_516_000_000,
                effective_from_unix_ms: 1_795_500_000_000,
                effective_until_unix_ms: 1_798_000_000_000,
            }),
            expected_artifact_contract_sha256: digest('3'),
            target_obligation_ids: vec!["obligation:calibration".to_owned()],
            expected_cost_units: 1,
            expected_delay_cost_units: 2,
            outcome_likelihoods: vec![
                OutcomeLikelihoodV0_1 {
                    state_id: "state:bad".to_owned(),
                    outcome_id: "outcome:positive".to_owned(),
                    likelihood_ppm: 100_000,
                },
                OutcomeLikelihoodV0_1 {
                    state_id: "state:bad".to_owned(),
                    outcome_id: "outcome:negative".to_owned(),
                    likelihood_ppm: 900_000,
                },
                OutcomeLikelihoodV0_1 {
                    state_id: "state:good".to_owned(),
                    outcome_id: "outcome:positive".to_owned(),
                    likelihood_ppm: 900_000,
                },
                OutcomeLikelihoodV0_1 {
                    state_id: "state:good".to_owned(),
                    outcome_id: "outcome:negative".to_owned(),
                    likelihood_ppm: 100_000,
                },
            ],
        };
        let mut input = EvidenceActionPlannerInputV0_1 {
            claim_id: "claim:wing-panel-proof-load".to_owned(),
            frozen_claim_disposition: "INCOMPLETE".to_owned(),
            configuration_ref: "configuration:wing-r17".to_owned(),
            as_known_at_unix_ms: 1_795_517_000_000,
            decision_context_sha256: digest('a'),
            assurance_state_sha256: digest('b'),
            action_catalogue_sha256: digest('c'),
            decision_model_sha256: digest('d'),
            outcome_model_sha256: digest('e'),
            unresolved_obligation_ids: vec!["obligation:calibration".to_owned()],
            unresolved_obligation_manifest_sha256: digest('f'),
            decision_alternatives: vec![
                DecisionAlternativeV0_1 {
                    decision_id: "decision:hold".to_owned(),
                },
                DecisionAlternativeV0_1 {
                    decision_id: "decision:proceed".to_owned(),
                },
            ],
            world_states: vec![
                WorldStateV0_1 {
                    state_id: "state:bad".to_owned(),
                    prior_probability_ppm: 500_000,
                },
                WorldStateV0_1 {
                    state_id: "state:good".to_owned(),
                    prior_probability_ppm: 500_000,
                },
            ],
            utilities: vec![
                StateUtilityV0_1 {
                    decision_id: "decision:hold".to_owned(),
                    state_id: "state:bad".to_owned(),
                    utility_units: 0,
                },
                StateUtilityV0_1 {
                    decision_id: "decision:hold".to_owned(),
                    state_id: "state:good".to_owned(),
                    utility_units: 0,
                },
                StateUtilityV0_1 {
                    decision_id: "decision:proceed".to_owned(),
                    state_id: "state:bad".to_owned(),
                    utility_units: -100,
                },
                StateUtilityV0_1 {
                    decision_id: "decision:proceed".to_owned(),
                    state_id: "state:good".to_owned(),
                    utility_units: 100,
                },
            ],
            actions: vec![action],
        };
        input.decision_model_sha256 =
            compute_decision_model_sha256(&input).expect("decision model commitment");
        for action in &mut input.actions {
            action.action_catalogue_entry_sha256 =
                compute_action_catalogue_entry_sha256(action)
                    .expect("action catalogue entry commitment");
        }
        let first_entry_digest = input.actions[0].action_catalogue_entry_sha256.clone();
        if let Some(receipt) = input.actions[0].authorization_receipt.as_mut() {
            receipt.action_catalogue_entry_sha256 = first_entry_digest;
        }
        input.action_catalogue_sha256 =
            compute_action_catalogue_cut_sha256(&input).expect("action catalogue cut commitment");
        input.unresolved_obligation_manifest_sha256 =
            compute_unresolved_obligation_manifest_sha256(&input.unresolved_obligation_ids)
                .expect("unresolved obligation commitment");
        let likelihood_digest = compute_outcome_likelihoods_sha256(
            &input.actions[0],
            &input.outcome_model_sha256,
        )
        .expect("outcome likelihood commitment");
        input.actions[0]
            .calibration_receipt
            .as_mut()
            .expect("calibration receipt")
            .outcome_likelihoods_sha256 = likelihood_digest;
        input
    }

    #[test]
    fn decision_relevant_information_has_positive_evsi_after_cost() {
        let input = base_input();
        let result = rank_evidence_actions_v0_1(&input);
        assert_eq!(
            result.disposition,
            EvidenceActionPlannerDispositionV0_1::RankedActions
        );
        assert_eq!(
            result.selected_action_id.as_deref(),
            Some("action:request-calibration-report")
        );
        assert_eq!(result.ranked_actions[0].evsi_scaled_numerator, "40000000000000");
        assert_eq!(
            result.ranked_actions[0].net_benefit_scaled_numerator,
            "37000000000000"
        );
        assert_eq!(result.frozen_claim_disposition, input.frozen_claim_disposition);
        assert_eq!(result.assurance_state_sha256, input.assurance_state_sha256);
        assert!(!result.assurance_state_mutated);
        assert!(!result.physical_work_authorized);
        assert!(!result.engineering_acceptance_granted);
    }

    #[test]
    fn high_information_with_identical_decision_utilities_has_zero_decision_value() {
        let mut input = base_input();
        for utility in &mut input.utilities {
            utility.utility_units = 0;
        }
        input.decision_model_sha256 =
            compute_decision_model_sha256(&input).expect("zero-utility decision commitment");
        let result = rank_evidence_actions_v0_1(&input);
        assert_eq!(
            result.disposition,
            EvidenceActionPlannerDispositionV0_1::NoPositiveNetBenefit
        );
        assert_eq!(result.ranked_actions[0].evsi_scaled_numerator, "0");
        assert_eq!(result.selected_action_id, None);
    }

    #[test]
    fn denied_actions_are_excluded_and_never_become_recommendations() {
        let mut input = base_input();
        input.actions[0].authorization_status = ActionAuthorizationStatusV0_1::Denied;
        let result = rank_evidence_actions_v0_1(&input);
        assert_eq!(
            result.disposition,
            EvidenceActionPlannerDispositionV0_1::AbstainNoQualifiedPermittedActions
        );
        assert!(result.ranked_actions.is_empty());
        assert_eq!(
            result.excluded_actions[0].reason_code,
            "ACTION_AUTHORITY_DENIED"
        );
        assert!(!result.physical_work_authorized);
    }

    #[test]
    fn uncalibrated_action_model_is_excluded() {
        let mut input = base_input();
        input.actions[0].calibration_status = OutcomeModelCalibrationStatusV0_1::Unverified;
        let result = rank_evidence_actions_v0_1(&input);
        assert_eq!(
            result.disposition,
            EvidenceActionPlannerDispositionV0_1::AbstainNoQualifiedPermittedActions
        );
        assert_eq!(
            result.excluded_actions[0].reason_code,
            "ACTION_OUTCOME_MODEL_UNVERIFIED"
        );
    }

    #[test]
    fn permission_for_another_configuration_is_not_reused() {
        let mut input = base_input();
        input.actions[0]
            .authorization_receipt
            .as_mut()
            .expect("permission receipt")
            .configuration_ref = "configuration:other-aircraft".to_owned();
        let result = rank_evidence_actions_v0_1(&input);
        assert_eq!(
            result.disposition,
            EvidenceActionPlannerDispositionV0_1::AbstainNoQualifiedPermittedActions
        );
        assert_eq!(
            result.excluded_actions[0].reason_code,
            "ACTION_AUTHORIZATION_SCOPE_MISMATCH"
        );
    }

    #[test]
    fn permission_received_after_the_selected_cutoff_is_not_used() {
        let mut input = base_input();
        input.actions[0]
            .authorization_receipt
            .as_mut()
            .expect("permission receipt")
            .receiver_first_seen_at_unix_ms = input.as_known_at_unix_ms + 1;
        let result = rank_evidence_actions_v0_1(&input);
        assert_eq!(
            result.disposition,
            EvidenceActionPlannerDispositionV0_1::AbstainNoQualifiedPermittedActions
        );
        assert_eq!(
            result.excluded_actions[0].reason_code,
            "ACTION_AUTHORIZATION_NOT_KNOWN_AT_CUTOFF"
        );
    }

    #[test]
    fn calibration_for_another_outcome_model_is_not_reused() {
        let mut input = base_input();
        input.actions[0]
            .calibration_receipt
            .as_mut()
            .expect("calibration receipt")
            .outcome_model_sha256 = digest('f');
        let result = rank_evidence_actions_v0_1(&input);
        assert_eq!(
            result.disposition,
            EvidenceActionPlannerDispositionV0_1::AbstainNoQualifiedPermittedActions
        );
        assert_eq!(
            result.excluded_actions[0].reason_code,
            "ACTION_CALIBRATION_SCOPE_MISMATCH"
        );
    }

    #[test]
    fn verified_permission_without_receipt_is_invalid_input() {
        let mut input = base_input();
        input.actions[0].authorization_receipt = None;
        let result = rank_evidence_actions_v0_1(&input);
        assert_eq!(
            result.disposition,
            EvidenceActionPlannerDispositionV0_1::InvalidInput
        );
        assert!(result
            .reason_codes
            .contains(&"EVSI_VERIFIED_ACTION_MISSING_AUTHORIZATION_RECEIPT".to_owned()));
    }

    #[test]
    fn changed_probability_table_invalidates_owner_calibration_receipt() {
        let mut input = base_input();
        input.actions[0].outcome_likelihoods[0].likelihood_ppm -= 1;
        input.actions[0].outcome_likelihoods[1].likelihood_ppm += 1;
        let result = rank_evidence_actions_v0_1(&input);
        assert_eq!(
            result.disposition,
            EvidenceActionPlannerDispositionV0_1::AbstainNoQualifiedPermittedActions
        );
        assert_eq!(
            result.excluded_actions[0].reason_code,
            "ACTION_CALIBRATION_SCOPE_MISMATCH"
        );
    }

    #[test]
    fn changed_utility_matrix_invalidates_decision_model_commitment() {
        let mut input = base_input();
        input.utilities[0].utility_units = 1;
        let result = rank_evidence_actions_v0_1(&input);
        assert_eq!(
            result.disposition,
            EvidenceActionPlannerDispositionV0_1::InvalidInput
        );
        assert!(result
            .reason_codes
            .contains(&"EVSI_DECISION_MODEL_COMMITMENT_MISMATCH".to_owned()));
    }

    #[test]
    fn changed_action_cost_invalidates_owner_catalogue_entry_commitment() {
        let mut input = base_input();
        input.actions[0].expected_cost_units = 0;
        let result = rank_evidence_actions_v0_1(&input);
        assert_eq!(
            result.disposition,
            EvidenceActionPlannerDispositionV0_1::InvalidInput
        );
        assert!(result.reason_codes.iter().any(|code| {
            code == "EVSI_ACTION_CATALOGUE_ENTRY_COMMITMENT_MISMATCH"
        }));
    }

    #[test]
    fn changed_expected_artifact_contract_invalidates_catalogue_entry_commitment() {
        let mut input = base_input();
        input.actions[0].expected_artifact_contract_sha256 = digest('8');
        let result = rank_evidence_actions_v0_1(&input);
        assert_eq!(
            result.disposition,
            EvidenceActionPlannerDispositionV0_1::InvalidInput
        );
        assert!(result.reason_codes.iter().any(|code| {
            code == "EVSI_ACTION_CATALOGUE_ENTRY_COMMITMENT_MISMATCH"
        }));
    }

    #[test]
    fn changed_candidate_action_set_invalidates_catalogue_cut_commitment() {
        let mut input = base_input();
        input.actions[0].action_id = "action:substituted".to_owned();
        let result = rank_evidence_actions_v0_1(&input);
        assert_eq!(
            result.disposition,
            EvidenceActionPlannerDispositionV0_1::InvalidInput
        );
        assert!(result
            .reason_codes
            .contains(&"EVSI_ACTION_CATALOGUE_COMMITMENT_MISMATCH".to_owned()));
    }

    #[test]
    fn action_cannot_target_an_obligation_outside_the_unresolved_manifest() {
        let mut input = base_input();
        input.actions[0].target_obligation_ids = vec!["obligation:not-open".to_owned()];
        let result = rank_evidence_actions_v0_1(&input);
        assert_eq!(
            result.disposition,
            EvidenceActionPlannerDispositionV0_1::InvalidInput
        );
        assert!(result.reason_codes.iter().any(|code| {
            code == "EVSI_ACTION_TARGET_IS_NOT_A_DECLARED_UNRESOLVED_OBLIGATION"
        }));
    }

    #[test]
    fn malformed_probability_table_is_rejected_not_normalized() {
        let mut input = base_input();
        input.world_states[0].prior_probability_ppm = 400_000;
        let result = rank_evidence_actions_v0_1(&input);
        assert_eq!(
            result.disposition,
            EvidenceActionPlannerDispositionV0_1::InvalidInput
        );
        assert!(result
            .reason_codes
            .contains(&"EVSI_PRIOR_PROBABILITIES_DO_NOT_SUM_TO_ONE".to_owned()));
    }

    #[test]
    fn published_synthetic_fixture_matches_exact_evsi_and_authority_boundary() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../docs/research/aviation/examples/aero_evidence_action_planner_v0_1.json"
        ))
        .expect("published EVSI fixture parses");
        let input: EvidenceActionPlannerInputV0_1 =
            serde_json::from_value(fixture["input"].clone())
                .expect("published EVSI input matches the typed contract");
        let result = rank_evidence_actions_v0_1(&input);
        let expected = &fixture["expected"];

        assert_eq!(
            result.disposition,
            EvidenceActionPlannerDispositionV0_1::RankedActions
        );
        assert_eq!(
            result.disposition,
            serde_json::from_value(expected["disposition"].clone())
                .expect("expected disposition is a valid enum value")
        );
        assert_eq!(
            result.current_best_decision_id.as_deref(),
            expected["current_best_decision_id"].as_str()
        );
        assert_eq!(
            result.selected_action_id.as_deref(),
            expected["selected_action_id"].as_str()
        );
        assert_eq!(
            result.ranked_actions[0].evsi_scaled_numerator,
            expected["ranked_actions"][0]["evsi_scaled_numerator"]
                .as_str()
                .expect("EVSI numerator")
        );
        assert_eq!(
            result.ranked_actions[0].net_benefit_scaled_numerator,
            expected["ranked_actions"][0]["net_benefit_scaled_numerator"]
                .as_str()
                .expect("net benefit numerator")
        );
        assert_eq!(
            result.excluded_actions[0].reason_code,
            expected["excluded_actions"][0]["reason_code"]
                .as_str()
                .expect("exclusion reason")
        );
        assert_eq!(result.claim_id, "claim:wing-panel-proof-load");
        assert_eq!(
            result.decision_context_sha256,
            expected["decision_context_sha256"].as_str().expect("decision context")
        );
        assert_eq!(
            result.assurance_state_sha256,
            expected["assurance_state_sha256"].as_str().expect("assurance state")
        );
        assert_eq!(
            result.action_catalogue_sha256,
            expected["action_catalogue_sha256"].as_str().expect("action catalogue cut")
        );
        assert_eq!(
            result.decision_model_sha256,
            expected["decision_model_sha256"].as_str().expect("decision model")
        );
        assert_eq!(
            result.outcome_model_sha256,
            expected["outcome_model_sha256"].as_str().expect("outcome model")
        );
        assert_eq!(result.frozen_claim_disposition, "INCOMPLETE");
        assert_eq!(
            result.configuration_ref,
            expected["configuration_ref"].as_str().expect("configuration")
        );
        assert_eq!(
            result.as_known_at_unix_ms,
            expected["as_known_at_unix_ms"].as_u64().expect("as-known cutoff")
        );
        assert_eq!(
            result.unresolved_obligation_manifest_sha256,
            expected["unresolved_obligation_manifest_sha256"]
                .as_str()
                .expect("unresolved obligation manifest")
        );
        assert_eq!(
            result.ranked_actions[0].outcome_model_owner_namespace,
            expected["ranked_actions"][0]["outcome_model_owner_namespace"]
                .as_str()
                .expect("outcome model owner")
        );
        assert_eq!(
            result.ranked_actions[0].authorization_receipt_sha256,
            expected["ranked_actions"][0]["authorization_receipt_sha256"]
                .as_str()
                .expect("authorization receipt")
        );
        assert_eq!(
            result.ranked_actions[0].calibration_receipt_sha256,
            expected["ranked_actions"][0]["calibration_receipt_sha256"]
                .as_str()
                .expect("calibration receipt")
        );
        assert_eq!(
            result.ranked_actions[0].outcome_likelihoods_sha256,
            expected["ranked_actions"][0]["outcome_likelihoods_sha256"]
                .as_str()
                .expect("outcome likelihood commitment")
        );
        let expected_reasons: Vec<String> =
            serde_json::from_value(expected["reason_codes"].clone())
                .expect("expected reason-code vector");
        assert_eq!(result.reason_codes, expected_reasons);
        assert!(!result.assurance_state_mutated);
        assert!(!result.physical_work_authorized);
        assert!(!result.engineering_acceptance_granted);
    }

    #[test]
    fn a_deny_all_action_catalogue_does_not_pass_as_success() {
        let mut input = base_input();
        input.actions[0].authorization_status = ActionAuthorizationStatusV0_1::Unknown;
        let result = rank_evidence_actions_v0_1(&input);
        assert_eq!(
            result.disposition,
            EvidenceActionPlannerDispositionV0_1::AbstainNoQualifiedPermittedActions
        );
        assert_eq!(result.selected_action_id, None);
        assert_eq!(result.frozen_claim_disposition, "INCOMPLETE");
    }
}
