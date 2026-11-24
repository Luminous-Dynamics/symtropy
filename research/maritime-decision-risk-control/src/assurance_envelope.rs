// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Composes finite adversarial-scenario evidence with a statistical risk claim.
//!
//! Finite scenarios ask whether every declared hard case remained physically
//! feasible and within participant authority. Statistical risk asks whether a
//! named method provides its declared guarantee over an operating distribution.
//! These evidence classes are never pooled or substituted for one another.
//!
//! Provenance and independence references are caller declarations. This module
//! does not authenticate them, prove the scenario universe is exhaustive for
//! reality, verify theorem assumptions, or grant operational authority.

use std::collections::BTreeSet;

use crate::oracle_adapter::{
    AuthorityDisposition, PhysicalDisposition, PolicyActionClass,
};

const MAX_LOSS_MILLI: u16 = 1_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FiniteScenarioSource {
    /// A model-generated fixture or simulator output, not independent truth.
    SyntheticModel,
    /// A caller-declared independently adjudicated finite scenario oracle.
    IndependentOracle,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FiniteScenarioContract {
    pub scenario_set_id: String,
    pub scenario_set_revision: String,
    pub policy_id: String,
    pub policy_revision: String,
    pub evaluator_revision: String,
    pub loss_contract_revision: String,
    pub operating_regime_id: String,
    /// Exact, predeclared universe of stress-scenario identifiers.
    pub expected_scenario_ids: Vec<String>,
    pub source: FiniteScenarioSource,
    pub source_evidence_ref: String,
    /// Required for IndependentOracle; caller assertion, not verified here.
    pub independence_evidence_ref: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FiniteScenarioOutcome {
    pub scenario_id: String,
    pub policy_id: String,
    pub policy_revision: String,
    pub evaluator_revision: String,
    pub loss_contract_revision: String,
    pub action_class: PolicyActionClass,
    pub authority: AuthorityDisposition,
    pub physical: PhysicalDisposition,
    pub service_loss_milli: u16,
    /// Participant outcomes stay separate; no hidden fairness weighting.
    pub participant_a_service_units: u32,
    pub participant_b_service_units: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FiniteScenarioError {
    MissingContractField(&'static str),
    EmptyScenarioUniverse,
    DuplicateExpectedScenario(String),
    DuplicateObservedScenario(String),
    UnexpectedScenario(String),
    PolicyIdentityMismatch,
    EvaluatorRevisionMismatch,
    LossContractRevisionMismatch,
    ServiceLossOutsideBound(u16),
    MissingIndependenceEvidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FiniteScenarioAudit {
    pub scenario_set_id: String,
    pub scenario_set_revision: String,
    pub policy_id: String,
    pub policy_revision: String,
    pub evaluator_revision: String,
    pub loss_contract_revision: String,
    pub operating_regime_id: String,
    pub source: FiniteScenarioSource,
    pub source_evidence_ref: String,
    pub independence_evidence_ref: Option<String>,
    pub expected_scenario_count: usize,
    pub observed_scenario_count: usize,
    pub missing_scenario_ids: Vec<String>,
    pub physical_failure_count: usize,
    pub unknown_physical_count: usize,
    pub authority_violation_count: usize,
    pub maximum_service_loss_milli: Option<u16>,
    pub minimum_participant_service_units: Option<u32>,
}

impl FiniteScenarioAudit {
    pub fn scenario_universe_complete(&self) -> bool {
        self.missing_scenario_ids.is_empty()
            && self.observed_scenario_count == self.expected_scenario_count
    }
}

/// Compiles a finite stress set without turning its rows into i.i.d. samples.
///
/// IndependentOracle is only a provenance declaration. A caller must
/// separately verify oracle independence and that the declared scenario
/// universe is appropriate for the claim being made.
pub fn compile_finite_scenario_audit(
    contract: &FiniteScenarioContract,
    outcomes: &[FiniteScenarioOutcome],
) -> Result<FiniteScenarioAudit, FiniteScenarioError> {
    for (value, field) in [
        (contract.scenario_set_id.as_str(), "scenario_set_id"),
        (contract.scenario_set_revision.as_str(), "scenario_set_revision"),
        (contract.policy_id.as_str(), "policy_id"),
        (contract.policy_revision.as_str(), "policy_revision"),
        (contract.evaluator_revision.as_str(), "evaluator_revision"),
        (contract.loss_contract_revision.as_str(), "loss_contract_revision"),
        (contract.operating_regime_id.as_str(), "operating_regime_id"),
        (contract.source_evidence_ref.as_str(), "source_evidence_ref"),
    ] {
        if value.trim().is_empty() {
            return Err(FiniteScenarioError::MissingContractField(field));
        }
    }

    if contract.expected_scenario_ids.is_empty() {
        return Err(FiniteScenarioError::EmptyScenarioUniverse);
    }
    if contract.source == FiniteScenarioSource::IndependentOracle
        && contract
            .independence_evidence_ref
            .as_deref()
            .is_none_or(str::is_empty)
    {
        return Err(FiniteScenarioError::MissingIndependenceEvidence);
    }

    let mut expected = BTreeSet::new();
    for scenario_id in &contract.expected_scenario_ids {
        if scenario_id.trim().is_empty() {
            return Err(FiniteScenarioError::MissingContractField(
                "expected_scenario_id",
            ));
        }
        if !expected.insert(scenario_id.clone()) {
            return Err(FiniteScenarioError::DuplicateExpectedScenario(
                scenario_id.clone(),
            ));
        }
    }

    let mut observed = BTreeSet::new();
    let mut physical_failure_count = 0_usize;
    let mut unknown_physical_count = 0_usize;
    let mut authority_violation_count = 0_usize;
    let mut maximum_service_loss_milli = None;
    let mut minimum_participant_service_units = None;

    for outcome in outcomes {
        if !expected.contains(&outcome.scenario_id) {
            return Err(FiniteScenarioError::UnexpectedScenario(
                outcome.scenario_id.clone(),
            ));
        }
        if !observed.insert(outcome.scenario_id.clone()) {
            return Err(FiniteScenarioError::DuplicateObservedScenario(
                outcome.scenario_id.clone(),
            ));
        }
        if outcome.policy_id != contract.policy_id
            || outcome.policy_revision != contract.policy_revision
        {
            return Err(FiniteScenarioError::PolicyIdentityMismatch);
        }
        if outcome.evaluator_revision != contract.evaluator_revision {
            return Err(FiniteScenarioError::EvaluatorRevisionMismatch);
        }
        if outcome.loss_contract_revision != contract.loss_contract_revision {
            return Err(FiniteScenarioError::LossContractRevisionMismatch);
        }
        if outcome.service_loss_milli > MAX_LOSS_MILLI {
            return Err(FiniteScenarioError::ServiceLossOutsideBound(
                outcome.service_loss_milli,
            ));
        }

        match outcome.physical {
            PhysicalDisposition::Feasible => {}
            PhysicalDisposition::Infeasible => physical_failure_count += 1,
            PhysicalDisposition::Unknown => unknown_physical_count += 1,
        }

        let authority_is_valid = matches!(
            (outcome.action_class, outcome.authority),
            (PolicyActionClass::NoMutation, AuthorityDisposition::NotRequired)
                | (PolicyActionClass::Mutating, AuthorityDisposition::Authorized)
        );
        if !authority_is_valid {
            authority_violation_count += 1;
        }

        maximum_service_loss_milli = Some(
            maximum_service_loss_milli.map_or(outcome.service_loss_milli, |value: u16| {
                value.max(outcome.service_loss_milli)
            }),
        );

        let participant_min = outcome
            .participant_a_service_units
            .min(outcome.participant_b_service_units);
        minimum_participant_service_units = Some(
            minimum_participant_service_units.map_or(participant_min, |value: u32| {
                value.min(participant_min)
            }),
        );
    }

    let missing_scenario_ids = expected.difference(&observed).cloned().collect();

    Ok(FiniteScenarioAudit {
        scenario_set_id: contract.scenario_set_id.clone(),
        scenario_set_revision: contract.scenario_set_revision.clone(),
        policy_id: contract.policy_id.clone(),
        policy_revision: contract.policy_revision.clone(),
        evaluator_revision: contract.evaluator_revision.clone(),
        loss_contract_revision: contract.loss_contract_revision.clone(),
        operating_regime_id: contract.operating_regime_id.clone(),
        source: contract.source,
        source_evidence_ref: contract.source_evidence_ref.clone(),
        independence_evidence_ref: contract.independence_evidence_ref.clone(),
        expected_scenario_count: expected.len(),
        observed_scenario_count: observed.len(),
        missing_scenario_ids,
        physical_failure_count,
        unknown_physical_count,
        authority_violation_count,
        maximum_service_loss_milli,
        minimum_participant_service_units,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StatisticalRiskAxis {
    HardPhysicalFailure,
    ServiceLoss,
    ParticipantService,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StatisticalGuarantee {
    /// Marginal expected risk over calibration and a future draw.
    MarginalExpected,
    /// Sample-conditional risk control with a declared failure probability.
    HighProbability {
        failure_probability_ppm: u32,
    },
    /// Time-uniform risk control over the declared monitoring sequence.
    TimeUniform {
        failure_probability_ppm: u32,
    },
    /// Descriptive metric only; never sufficient for this composition gate.
    DiagnosticOnly,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatisticalRiskEvidenceClaim {
    pub method_id: String,
    pub method_result_ref: String,
    pub assumptions_scope_ref: String,
    pub policy_id: String,
    pub policy_revision: String,
    pub evaluator_revision: String,
    pub loss_contract_revision: String,
    pub operating_regime_id: String,
    pub risk_axis: StatisticalRiskAxis,
    pub guarantee: StatisticalGuarantee,
    pub target_risk_milli: u16,
    /// Upper risk bound emitted by the named method; may exceed 1,000.
    pub risk_upper_bound_milli: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssuranceBlocker {
    ScenarioSourceNotIndependent,
    MissingIndependenceEvidence,
    ScenarioUniverseIncomplete,
    UnknownPhysicalScenarioOutcome,
    PhysicalScenarioFailure,
    AuthorityViolation,
    PolicyIdentityMismatch,
    EvaluatorRevisionMismatch,
    LossContractRevisionMismatch,
    OperatingRegimeMismatch,
    WrongRiskAxis,
    InvalidRiskClaim,
    DiagnosticIsNotRiskControl,
    RiskUpperBoundExceedsTarget,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssuranceDecision {
    /// Compositional admissibility within the two declared evidence scopes.
    /// This is not an operational permission or a proof of source truth.
    pub admissible_within_declared_scope: bool,
    pub blockers: Vec<AssuranceBlocker>,
    pub guarantee: StatisticalGuarantee,
    pub target_risk_milli: u16,
    pub risk_upper_bound_milli: u32,
    pub scenario_count: usize,
    pub physical_failure_count: usize,
    pub unknown_physical_count: usize,
    pub authority_violation_count: usize,
}

/// Applies a strict conjunction: finite hard-case assurance AND statistical
/// risk assurance. Guarantee classes and risk bounds remain visible separately.
pub fn compose_assurance(
    scenarios: &FiniteScenarioAudit,
    risk: &StatisticalRiskEvidenceClaim,
) -> AssuranceDecision {
    let mut blockers = Vec::new();

    if scenarios.source != FiniteScenarioSource::IndependentOracle {
        blockers.push(AssuranceBlocker::ScenarioSourceNotIndependent);
    }
    if scenarios.source == FiniteScenarioSource::IndependentOracle
        && scenarios
            .independence_evidence_ref
            .as_deref()
            .is_none_or(str::is_empty)
    {
        blockers.push(AssuranceBlocker::MissingIndependenceEvidence);
    }
    if !scenarios.scenario_universe_complete() {
        blockers.push(AssuranceBlocker::ScenarioUniverseIncomplete);
    }
    if scenarios.unknown_physical_count > 0 {
        blockers.push(AssuranceBlocker::UnknownPhysicalScenarioOutcome);
    }
    if scenarios.physical_failure_count > 0 {
        blockers.push(AssuranceBlocker::PhysicalScenarioFailure);
    }
    if scenarios.authority_violation_count > 0 {
        blockers.push(AssuranceBlocker::AuthorityViolation);
    }

    if risk.policy_id != scenarios.policy_id || risk.policy_revision != scenarios.policy_revision
    {
        blockers.push(AssuranceBlocker::PolicyIdentityMismatch);
    }
    if risk.evaluator_revision != scenarios.evaluator_revision {
        blockers.push(AssuranceBlocker::EvaluatorRevisionMismatch);
    }
    if risk.loss_contract_revision != scenarios.loss_contract_revision {
        blockers.push(AssuranceBlocker::LossContractRevisionMismatch);
    }
    if risk.operating_regime_id != scenarios.operating_regime_id {
        blockers.push(AssuranceBlocker::OperatingRegimeMismatch);
    }
    if risk.risk_axis != StatisticalRiskAxis::HardPhysicalFailure {
        blockers.push(AssuranceBlocker::WrongRiskAxis);
    }

    let references_present = !risk.method_id.trim().is_empty()
        && !risk.method_result_ref.trim().is_empty()
        && !risk.assumptions_scope_ref.trim().is_empty();
    let target_valid = risk.target_risk_milli > 0 && risk.target_risk_milli < MAX_LOSS_MILLI;
    let delta_valid = match risk.guarantee {
        StatisticalGuarantee::HighProbability {
            failure_probability_ppm,
        }
        | StatisticalGuarantee::TimeUniform {
            failure_probability_ppm,
        } => (1..1_000_000).contains(&failure_probability_ppm),
        StatisticalGuarantee::MarginalExpected | StatisticalGuarantee::DiagnosticOnly => true,
    };
    if !references_present || !target_valid || !delta_valid {
        blockers.push(AssuranceBlocker::InvalidRiskClaim);
    }
    if risk.guarantee == StatisticalGuarantee::DiagnosticOnly {
        blockers.push(AssuranceBlocker::DiagnosticIsNotRiskControl);
    }
    if risk.risk_upper_bound_milli > u32::from(risk.target_risk_milli) {
        blockers.push(AssuranceBlocker::RiskUpperBoundExceedsTarget);
    }

    AssuranceDecision {
        admissible_within_declared_scope: blockers.is_empty(),
        blockers,
        guarantee: risk.guarantee,
        target_risk_milli: risk.target_risk_milli,
        risk_upper_bound_milli: risk.risk_upper_bound_milli,
        scenario_count: scenarios.observed_scenario_count,
        physical_failure_count: scenarios.physical_failure_count,
        unknown_physical_count: scenarios.unknown_physical_count,
        authority_violation_count: scenarios.authority_violation_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract(source: FiniteScenarioSource) -> FiniteScenarioContract {
        FiniteScenarioContract {
            scenario_set_id: "reserve-recovery-six-worlds".to_owned(),
            scenario_set_revision: "scenario-grid-v1".to_owned(),
            policy_id: "independent-reserve-inspection".to_owned(),
            policy_revision: "policy-v1".to_owned(),
            evaluator_revision: "evaluator-v1".to_owned(),
            loss_contract_revision: "loss-v1".to_owned(),
            operating_regime_id: "synthetic-reserve-regime".to_owned(),
            expected_scenario_ids: vec![
                "a-critical".to_owned(),
                "b-critical".to_owned(),
                "reserve-outage".to_owned(),
            ],
            source,
            source_evidence_ref: "test-fixture-source".to_owned(),
            independence_evidence_ref: (source == FiniteScenarioSource::IndependentOracle)
                .then(|| "test-fixture-independence-claim".to_owned()),
        }
    }

    fn outcomes() -> Vec<FiniteScenarioOutcome> {
        ["a-critical", "b-critical", "reserve-outage"]
            .into_iter()
            .map(|scenario_id| FiniteScenarioOutcome {
                scenario_id: scenario_id.to_owned(),
                policy_id: "independent-reserve-inspection".to_owned(),
                policy_revision: "policy-v1".to_owned(),
                evaluator_revision: "evaluator-v1".to_owned(),
                loss_contract_revision: "loss-v1".to_owned(),
                action_class: PolicyActionClass::NoMutation,
                authority: AuthorityDisposition::NotRequired,
                physical: PhysicalDisposition::Feasible,
                service_loss_milli: 100,
                participant_a_service_units: 30,
                participant_b_service_units: 30,
            })
            .collect()
    }

    fn risk_claim() -> StatisticalRiskEvidenceClaim {
        StatisticalRiskEvidenceClaim {
            method_id: "simultaneous-hoeffding".to_owned(),
            method_result_ref: "synthetic-risk-method-receipt".to_owned(),
            assumptions_scope_ref: "iid-bounded-frozen-grid".to_owned(),
            policy_id: "independent-reserve-inspection".to_owned(),
            policy_revision: "policy-v1".to_owned(),
            evaluator_revision: "evaluator-v1".to_owned(),
            loss_contract_revision: "loss-v1".to_owned(),
            operating_regime_id: "synthetic-reserve-regime".to_owned(),
            risk_axis: StatisticalRiskAxis::HardPhysicalFailure,
            guarantee: StatisticalGuarantee::HighProbability {
                failure_probability_ppm: 50_000,
            },
            target_risk_milli: 100,
            risk_upper_bound_milli: 75,
        }
    }

    fn compile(source: FiniteScenarioSource) -> FiniteScenarioAudit {
        compile_finite_scenario_audit(&contract(source), &outcomes())
            .expect("structurally valid finite-scenario fixture")
    }

    #[test]
    fn valid_finite_and_statistical_evidence_compose_without_erasing_guarantee_type() {
        let decision = compose_assurance(
            &compile(FiniteScenarioSource::IndependentOracle),
            &risk_claim(),
        );

        assert!(decision.admissible_within_declared_scope);
        assert!(decision.blockers.is_empty());
        assert_eq!(
            decision.guarantee,
            StatisticalGuarantee::HighProbability {
                failure_probability_ppm: 50_000
            }
        );
        assert_eq!(decision.scenario_count, 3);
        assert_eq!(decision.risk_upper_bound_milli, 75);
    }

    #[test]
    fn model_generated_scenarios_cannot_masquerade_as_independent_truth() {
        let decision =
            compose_assurance(&compile(FiniteScenarioSource::SyntheticModel), &risk_claim());

        assert!(!decision.admissible_within_declared_scope);
        assert!(decision
            .blockers
            .contains(&AssuranceBlocker::ScenarioSourceNotIndependent));
    }

    #[test]
    fn a_passing_statistical_claim_cannot_override_a_physical_scenario_failure() {
        let mut values = outcomes();
        values[1].physical = PhysicalDisposition::Infeasible;
        let audit = compile_finite_scenario_audit(
            &contract(FiniteScenarioSource::IndependentOracle),
            &values,
        )
        .expect("known physical failure remains a valid audit input");

        let decision = compose_assurance(&audit, &risk_claim());
        assert!(!decision.admissible_within_declared_scope);
        assert!(decision
            .blockers
            .contains(&AssuranceBlocker::PhysicalScenarioFailure));
    }

    #[test]
    fn authority_failure_is_a_hard_gate_not_a_statistical_loss_tradeoff() {
        let mut values = outcomes();
        values[0].action_class = PolicyActionClass::Mutating;
        values[0].authority = AuthorityDisposition::Denied;
        let audit = compile_finite_scenario_audit(
            &contract(FiniteScenarioSource::IndependentOracle),
            &values,
        )
        .expect("authority violation is retained in the audit");

        let decision = compose_assurance(&audit, &risk_claim());
        assert!(!decision.admissible_within_declared_scope);
        assert!(decision
            .blockers
            .contains(&AssuranceBlocker::AuthorityViolation));
    }

    #[test]
    fn unknown_physical_outcomes_block_composition() {
        let mut values = outcomes();
        values[2].physical = PhysicalDisposition::Unknown;
        let audit = compile_finite_scenario_audit(
            &contract(FiniteScenarioSource::IndependentOracle),
            &values,
        )
        .expect("unknown state remains explicit");

        let decision = compose_assurance(&audit, &risk_claim());
        assert!(decision
            .blockers
            .contains(&AssuranceBlocker::UnknownPhysicalScenarioOutcome));
    }

    #[test]
    fn missing_scenario_prevents_a_complete_finite_set_claim() {
        let mut values = outcomes();
        values.pop();
        let audit = compile_finite_scenario_audit(
            &contract(FiniteScenarioSource::IndependentOracle),
            &values,
        )
        .expect("partial scenario evidence is retained");

        assert!(!audit.scenario_universe_complete());
        let decision = compose_assurance(&audit, &risk_claim());
        assert!(decision
            .blockers
            .contains(&AssuranceBlocker::ScenarioUniverseIncomplete));
    }

    #[test]
    fn empirical_tail_diagnostics_do_not_count_as_statistical_risk_control() {
        let mut claim = risk_claim();
        claim.guarantee = StatisticalGuarantee::DiagnosticOnly;
        let decision = compose_assurance(
            &compile(FiniteScenarioSource::IndependentOracle),
            &claim,
        );

        assert!(decision
            .blockers
            .contains(&AssuranceBlocker::DiagnosticIsNotRiskControl));
        assert!(!decision.admissible_within_declared_scope);
    }

    #[test]
    fn risk_bound_above_target_blocks_even_when_finite_scenarios_pass() {
        let mut claim = risk_claim();
        claim.risk_upper_bound_milli = 101;
        let decision = compose_assurance(
            &compile(FiniteScenarioSource::IndependentOracle),
            &claim,
        );

        assert!(decision
            .blockers
            .contains(&AssuranceBlocker::RiskUpperBoundExceedsTarget));
    }

    #[test]
    fn policy_and_evaluator_revision_drift_blocks_composition() {
        let mut claim = risk_claim();
        claim.policy_revision = "policy-v2".to_owned();
        claim.evaluator_revision = "evaluator-v2".to_owned();
        let decision = compose_assurance(
            &compile(FiniteScenarioSource::IndependentOracle),
            &claim,
        );

        assert!(decision
            .blockers
            .contains(&AssuranceBlocker::PolicyIdentityMismatch));
        assert!(decision
            .blockers
            .contains(&AssuranceBlocker::EvaluatorRevisionMismatch));
    }

    #[test]
    fn service_loss_claim_cannot_stand_in_for_hard_physical_risk() {
        let mut claim = risk_claim();
        claim.risk_axis = StatisticalRiskAxis::ServiceLoss;
        let decision = compose_assurance(
            &compile(FiniteScenarioSource::IndependentOracle),
            &claim,
        );

        assert!(decision.blockers.contains(&AssuranceBlocker::WrongRiskAxis));
    }

    #[test]
    fn invalid_anytime_failure_probability_is_rejected() {
        let mut claim = risk_claim();
        claim.guarantee = StatisticalGuarantee::TimeUniform {
            failure_probability_ppm: 0,
        };
        let decision = compose_assurance(
            &compile(FiniteScenarioSource::IndependentOracle),
            &claim,
        );

        assert!(decision.blockers.contains(&AssuranceBlocker::InvalidRiskClaim));
    }

    #[test]
    fn independent_source_requires_a_declared_independence_reference() {
        let mut fixture = contract(FiniteScenarioSource::IndependentOracle);
        fixture.independence_evidence_ref = None;
        assert_eq!(
            compile_finite_scenario_audit(&fixture, &outcomes()),
            Err(FiniteScenarioError::MissingIndependenceEvidence)
        );
    }
}
