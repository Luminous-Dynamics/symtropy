//! Robust decision-directed measurement selection.
//!
//! This finite oracle compares candidate tests by worst-case residual minimax
//! regret. It does not assign probabilities to hypotheses or experiment
//! outcomes. Cost and intervention risk remain independent hard constraints
//! and Pareto dimensions; they are not silently converted into utility.

use std::cmp::Ordering;
use std::collections::BTreeSet;

use crate::StableId;
use crate::decision_closure::{
    DecisionCertificate, DecisionError, DecisionManifest, DecisionStatus, ExactRational,
};

const DOMAIN: &[u8] = b"symtropy.road-decision-measurement.v0.1\0";

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ExperimentModality {
    PassiveObservation,
    ControlledIntervention,
    ClosedCourseIntervention,
    SimulationChallenge,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RiskAuthorization {
    Approved {
        authorization_id: StableId,
        safety_case_digest: [u8; 32],
    },
    NotApproved {
        reason_id: StableId,
    },
    Unassessed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExperimentOutcome {
    pub outcome_id: StableId,
    /// All model hypotheses consistent with this possible observation.
    /// Multiple outcomes may contain the same hypothesis when the test is noisy.
    pub compatible_hypotheses: BTreeSet<StableId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateExperiment {
    pub experiment_id: StableId,
    pub modality: ExperimentModality,
    pub protocol_digest: [u8; 32],
    pub preregistration_digest: [u8; 32],
    pub risk_assessment_digest: [u8; 32],
    pub cost_scale_id: StableId,
    pub acquisition_cost: ExactRational,
    pub risk_scale_id: StableId,
    /// Conservative upper bound, not a probability-weighted expected risk.
    pub risk_upper_bound: ExactRational,
    pub authorization: RiskAuthorization,
    pub outcomes: Vec<ExperimentOutcome>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExperimentConstraints {
    pub cost_scale_id: StableId,
    pub maximum_cost: ExactRational,
    pub risk_scale_id: StableId,
    pub maximum_risk: ExactRational,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MeasurementSelectionManifest {
    pub selection_id: StableId,
    pub version: StableId,
    pub baseline: DecisionManifest,
    pub constraints: ExperimentConstraints,
    pub candidates: Vec<CandidateExperiment>,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateDisposition {
    ParetoEligible,
    Dominated,
    NoDecisionRegretReduction,
    OverBudget,
    RiskLimitExceeded,
    NotApproved,
    Unassessed,
    Incomplete,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateInvalidReason {
    MissingProtocolDigest,
    MissingPreregistrationDigest,
    MissingRiskAssessmentDigest,
    NegativeAcquisitionCost,
    NegativeRiskBound,
    ZeroCostDenominator,
    ZeroRiskDenominator,
    CostScaleMismatch,
    RiskScaleMismatch,
    NoPossibleOutcomes,
    DuplicateOutcomeId,
    EmptyCompatibleHypothesisSet,
    UnknownCompatibleHypothesis,
    IncompleteHypothesisCoverage,
    MissingSafetyCaseDigest,
    WorstCaseRegretIncreased,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConditionalOutcomeEvaluation {
    pub outcome_id: StableId,
    pub compatible_hypotheses: Vec<StableId>,
    pub minimax_regret: ExactRational,
    pub decision_status: DecisionStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateEvaluation {
    pub experiment_id: StableId,
    pub modality: ExperimentModality,
    pub disposition: CandidateDisposition,
    pub invalid_reason: Option<CandidateInvalidReason>,
    pub acquisition_cost: ExactRational,
    pub risk_upper_bound: ExactRational,
    pub baseline_minimax_regret: Option<ExactRational>,
    pub worst_case_residual_regret: Option<ExactRational>,
    pub regret_reduction: Option<ExactRational>,
    pub outcome_evaluations: Vec<ConditionalOutcomeEvaluation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MeasurementSelectionCertificate {
    pub schema_id: &'static str,
    pub manifest_digest: [u8; 32],
    pub baseline_decision_certificate: DecisionCertificate,
    pub baseline_minimax_regret: ExactRational,
    pub candidate_evaluations: Vec<CandidateEvaluation>,
    /// Non-dominated candidates that are within budget/risk and reduce regret.
    pub pareto_experiment_ids: Vec<StableId>,
    pub receipt_digest: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MeasurementSelectionError {
    BaselineInvalid(DecisionError),
    BaselineNotFullySupported,
    InvalidConstraints(CandidateInvalidReason),
    DuplicateCandidateId,
    ArithmeticOverflow,
    ReceiptMismatch,
}

impl MeasurementSelectionManifest {
    pub fn validate(&self) -> Result<(), MeasurementSelectionError> {
        self.baseline
            .validate()
            .map_err(MeasurementSelectionError::BaselineInvalid)?;

        let mut candidate_ids = BTreeSet::new();
        for candidate in &self.candidates {
            if !candidate_ids.insert(candidate.experiment_id.clone()) {
                return Err(MeasurementSelectionError::DuplicateCandidateId);
            }
        }

        validate_nonnegative(
            &self.constraints.maximum_cost,
            CandidateInvalidReason::ZeroCostDenominator,
            CandidateInvalidReason::NegativeAcquisitionCost,
        )
        .map_err(MeasurementSelectionError::InvalidConstraints)?;
        validate_nonnegative(
            &self.constraints.maximum_risk,
            CandidateInvalidReason::ZeroRiskDenominator,
            CandidateInvalidReason::NegativeRiskBound,
        )
        .map_err(MeasurementSelectionError::InvalidConstraints)?;
        Ok(())
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, MeasurementSelectionError> {
        self.validate()?;
        let baseline_digest = self
            .baseline
            .digest()
            .map_err(MeasurementSelectionError::BaselineInvalid)?;

        let mut out = Vec::new();
        out.extend_from_slice(DOMAIN);
        push_id(&mut out, &self.selection_id);
        push_id(&mut out, &self.version);
        out.extend_from_slice(&baseline_digest);

        push_id(&mut out, &self.constraints.cost_scale_id);
        push_rational(&mut out, &self.constraints.maximum_cost)?;
        push_id(&mut out, &self.constraints.risk_scale_id);
        push_rational(&mut out, &self.constraints.maximum_risk)?;

        let mut candidates = self.candidates.iter().collect::<Vec<_>>();
        candidates.sort_by(|a, b| a.experiment_id.cmp(&b.experiment_id));
        push_len(&mut out, candidates.len());
        for candidate in candidates {
            push_candidate(&mut out, candidate)?;
        }
        Ok(out)
    }

    pub fn digest(&self) -> Result<[u8; 32], MeasurementSelectionError> {
        Ok(*blake3::hash(&self.canonical_bytes()?).as_bytes())
    }

    pub fn evaluate(&self) -> Result<MeasurementSelectionCertificate, MeasurementSelectionError> {
        self.validate()?;
        let baseline_decision_certificate = self
            .baseline
            .evaluate()
            .map_err(MeasurementSelectionError::BaselineInvalid)?;
        if matches!(
            baseline_decision_certificate.status,
            DecisionStatus::Unsupported | DecisionStatus::Incomplete
        ) {
            return Err(MeasurementSelectionError::BaselineNotFullySupported);
        }
        let baseline_minimax_regret =
            minimum_action_regret(&baseline_decision_certificate.action_regrets)?;
        let manifest_digest = self.digest()?;

        let mut candidate_evaluations = self
            .candidates
            .iter()
            .map(|candidate| self.evaluate_candidate(candidate, &baseline_minimax_regret))
            .collect::<Result<Vec<_>, _>>()?;
        candidate_evaluations.sort_by(|a, b| a.experiment_id.cmp(&b.experiment_id));

        mark_pareto_frontier(&mut candidate_evaluations)?;

        let pareto_experiment_ids = candidate_evaluations
            .iter()
            .filter(|evaluation| evaluation.disposition == CandidateDisposition::ParetoEligible)
            .map(|evaluation| evaluation.experiment_id.clone())
            .collect::<Vec<_>>();

        let mut certificate = MeasurementSelectionCertificate {
            schema_id: "symtropy.road-decision-measurement.certificate.v0.1",
            manifest_digest,
            baseline_decision_certificate,
            baseline_minimax_regret,
            candidate_evaluations,
            pareto_experiment_ids,
            receipt_digest: [0; 32],
        };
        certificate.receipt_digest = *blake3::hash(&certificate_bytes(&certificate)).as_bytes();
        Ok(certificate)
    }

    fn evaluate_candidate(
        &self,
        candidate: &CandidateExperiment,
        baseline_minimax_regret: &ExactRational,
    ) -> Result<CandidateEvaluation, MeasurementSelectionError> {
        let mut evaluation = empty_evaluation(candidate);

        if let Err(reason) = validate_candidate(candidate, &self.baseline, &self.constraints) {
            evaluation.disposition = CandidateDisposition::Incomplete;
            evaluation.invalid_reason = Some(reason);
            return Ok(evaluation);
        }

        match &candidate.authorization {
            RiskAuthorization::Approved { .. } => {}
            RiskAuthorization::NotApproved { .. } => {
                evaluation.disposition = CandidateDisposition::NotApproved;
                return Ok(evaluation);
            }
            RiskAuthorization::Unassessed => {
                evaluation.disposition = CandidateDisposition::Unassessed;
                return Ok(evaluation);
            }
        }

        if candidate
            .acquisition_cost
            .checked_cmp(&self.constraints.maximum_cost)
            .map_err(|_| MeasurementSelectionError::ArithmeticOverflow)?
            == Ordering::Greater
        {
            evaluation.disposition = CandidateDisposition::OverBudget;
            return Ok(evaluation);
        }
        if candidate
            .risk_upper_bound
            .checked_cmp(&self.constraints.maximum_risk)
            .map_err(|_| MeasurementSelectionError::ArithmeticOverflow)?
            == Ordering::Greater
        {
            evaluation.disposition = CandidateDisposition::RiskLimitExceeded;
            return Ok(evaluation);
        }

        let mut outcome_evaluations = Vec::with_capacity(candidate.outcomes.len());
        let mut worst_case_residual =
            ExactRational::new(0, 1).map_err(|_| MeasurementSelectionError::ArithmeticOverflow)?;

        for outcome in &candidate.outcomes {
            let compatible = &outcome.compatible_hypotheses;
            let mut conditional_manifest = self.baseline.clone();
            conditional_manifest
                .hypotheses
                .retain(|hypothesis| compatible.contains(&hypothesis.hypothesis_id));
            let conditional_certificate = conditional_manifest
                .evaluate()
                .map_err(MeasurementSelectionError::BaselineInvalid)?;
            let residual = minimum_action_regret(&conditional_certificate.action_regrets)?;
            if residual
                .checked_cmp(&worst_case_residual)
                .map_err(|_| MeasurementSelectionError::ArithmeticOverflow)?
                == Ordering::Greater
            {
                worst_case_residual = residual.clone();
            }
            outcome_evaluations.push(ConditionalOutcomeEvaluation {
                outcome_id: outcome.outcome_id.clone(),
                compatible_hypotheses: compatible.iter().cloned().collect(),
                minimax_regret: residual,
                decision_status: conditional_certificate.status,
            });
        }
        outcome_evaluations.sort_by(|a, b| a.outcome_id.cmp(&b.outcome_id));

        let regret_reduction = baseline_minimax_regret
            .checked_sub(&worst_case_residual)
            .map_err(|_| MeasurementSelectionError::ArithmeticOverflow)?;
        if regret_reduction.numerator < 0 {
            evaluation.disposition = CandidateDisposition::Incomplete;
            evaluation.invalid_reason = Some(CandidateInvalidReason::WorstCaseRegretIncreased);
            return Ok(evaluation);
        }

        evaluation.baseline_minimax_regret = Some(baseline_minimax_regret.clone());
        evaluation.worst_case_residual_regret = Some(worst_case_residual);
        evaluation.regret_reduction = Some(regret_reduction.clone());
        evaluation.outcome_evaluations = outcome_evaluations;
        evaluation.disposition = if regret_reduction.numerator == 0 {
            CandidateDisposition::NoDecisionRegretReduction
        } else {
            CandidateDisposition::ParetoEligible
        };
        Ok(evaluation)
    }
}

impl MeasurementSelectionCertificate {
    /// Unkeyed checksum only; exact replay is the verification path.
    pub fn integrity_check(&self) -> bool {
        *blake3::hash(&certificate_bytes(self)).as_bytes() == self.receipt_digest
    }

    pub fn verify_against(
        &self,
        manifest: &MeasurementSelectionManifest,
    ) -> Result<(), MeasurementSelectionError> {
        let recomputed = manifest.evaluate()?;
        if recomputed == *self {
            Ok(())
        } else {
            Err(MeasurementSelectionError::ReceiptMismatch)
        }
    }
}

fn empty_evaluation(candidate: &CandidateExperiment) -> CandidateEvaluation {
    CandidateEvaluation {
        experiment_id: candidate.experiment_id.clone(),
        modality: candidate.modality,
        disposition: CandidateDisposition::Incomplete,
        invalid_reason: None,
        acquisition_cost: normalize_or_original(&candidate.acquisition_cost),
        risk_upper_bound: normalize_or_original(&candidate.risk_upper_bound),
        baseline_minimax_regret: None,
        worst_case_residual_regret: None,
        regret_reduction: None,
        outcome_evaluations: Vec::new(),
    }
}

fn normalize_or_original(value: &ExactRational) -> ExactRational {
    ExactRational::new(value.numerator, value.denominator).unwrap_or_else(|_| value.clone())
}

fn validate_candidate(
    candidate: &CandidateExperiment,
    baseline: &DecisionManifest,
    constraints: &ExperimentConstraints,
) -> Result<(), CandidateInvalidReason> {
    if candidate.protocol_digest == [0; 32] {
        return Err(CandidateInvalidReason::MissingProtocolDigest);
    }
    if candidate.preregistration_digest == [0; 32] {
        return Err(CandidateInvalidReason::MissingPreregistrationDigest);
    }
    if candidate.risk_assessment_digest == [0; 32] {
        return Err(CandidateInvalidReason::MissingRiskAssessmentDigest);
    }
    validate_nonnegative(
        &candidate.acquisition_cost,
        CandidateInvalidReason::ZeroCostDenominator,
        CandidateInvalidReason::NegativeAcquisitionCost,
    )?;
    validate_nonnegative(
        &candidate.risk_upper_bound,
        CandidateInvalidReason::ZeroRiskDenominator,
        CandidateInvalidReason::NegativeRiskBound,
    )?;
    if candidate.cost_scale_id != constraints.cost_scale_id {
        return Err(CandidateInvalidReason::CostScaleMismatch);
    }
    if candidate.risk_scale_id != constraints.risk_scale_id {
        return Err(CandidateInvalidReason::RiskScaleMismatch);
    }
    if candidate.outcomes.is_empty() {
        return Err(CandidateInvalidReason::NoPossibleOutcomes);
    }

    let known_hypotheses = baseline
        .hypotheses
        .iter()
        .map(|hypothesis| hypothesis.hypothesis_id.clone())
        .collect::<BTreeSet<_>>();
    let mut outcome_ids = BTreeSet::new();
    let mut covered_hypotheses = BTreeSet::new();

    for outcome in &candidate.outcomes {
        if !outcome_ids.insert(outcome.outcome_id.clone()) {
            return Err(CandidateInvalidReason::DuplicateOutcomeId);
        }
        if outcome.compatible_hypotheses.is_empty() {
            return Err(CandidateInvalidReason::EmptyCompatibleHypothesisSet);
        }
        for hypothesis_id in &outcome.compatible_hypotheses {
            if !known_hypotheses.contains(hypothesis_id) {
                return Err(CandidateInvalidReason::UnknownCompatibleHypothesis);
            }
            covered_hypotheses.insert(hypothesis_id.clone());
        }
    }
    if covered_hypotheses != known_hypotheses {
        return Err(CandidateInvalidReason::IncompleteHypothesisCoverage);
    }

    match &candidate.authorization {
        RiskAuthorization::Approved {
            authorization_id,
            safety_case_digest,
        } => {
            let _authorization_reference = authorization_id;
            if *safety_case_digest == [0; 32] {
                return Err(CandidateInvalidReason::MissingSafetyCaseDigest);
            }
        }
        RiskAuthorization::NotApproved { .. } | RiskAuthorization::Unassessed => {}
    }
    Ok(())
}

fn validate_nonnegative(
    value: &ExactRational,
    zero_denominator: CandidateInvalidReason,
    negative_value: CandidateInvalidReason,
) -> Result<(), CandidateInvalidReason> {
    if value.denominator == 0 {
        return Err(zero_denominator);
    }
    if value.numerator < 0 {
        return Err(negative_value);
    }
    Ok(())
}

fn minimum_action_regret(
    action_regrets: &[crate::decision_closure::ActionRegret],
) -> Result<ExactRational, MeasurementSelectionError> {
    let first = action_regrets
        .first()
        .ok_or(MeasurementSelectionError::BaselineNotFullySupported)?;
    let mut minimum = first.worst_case_regret.clone();
    for action in action_regrets.iter().skip(1) {
        if action
            .worst_case_regret
            .checked_cmp(&minimum)
            .map_err(|_| MeasurementSelectionError::ArithmeticOverflow)?
            == Ordering::Less
        {
            minimum = action.worst_case_regret.clone();
        }
    }
    Ok(minimum)
}

fn mark_pareto_frontier(
    evaluations: &mut [CandidateEvaluation],
) -> Result<(), MeasurementSelectionError> {
    let eligible = evaluations
        .iter()
        .filter(|evaluation| evaluation.disposition == CandidateDisposition::ParetoEligible)
        .cloned()
        .collect::<Vec<_>>();

    for evaluation in evaluations.iter_mut() {
        if evaluation.disposition != CandidateDisposition::ParetoEligible {
            continue;
        }
        let mut dominated = false;
        for competitor in &eligible {
            if competitor.experiment_id == evaluation.experiment_id {
                continue;
            }
            if dominates(competitor, evaluation)? {
                dominated = true;
                break;
            }
        }
        if dominated {
            evaluation.disposition = CandidateDisposition::Dominated;
        }
    }
    Ok(())
}

fn dominates(
    left: &CandidateEvaluation,
    right: &CandidateEvaluation,
) -> Result<bool, MeasurementSelectionError> {
    let left_cost = &left.acquisition_cost;
    let right_cost = &right.acquisition_cost;
    let left_risk = &left.risk_upper_bound;
    let right_risk = &right.risk_upper_bound;
    let (Some(left_residual), Some(right_residual)) = (
        left.worst_case_residual_regret.as_ref(),
        right.worst_case_residual_regret.as_ref(),
    ) else {
        return Ok(false);
    };

    let cost_order = left_cost
        .checked_cmp(right_cost)
        .map_err(|_| MeasurementSelectionError::ArithmeticOverflow)?;
    let risk_order = left_risk
        .checked_cmp(right_risk)
        .map_err(|_| MeasurementSelectionError::ArithmeticOverflow)?;
    let regret_order = left_residual
        .checked_cmp(right_residual)
        .map_err(|_| MeasurementSelectionError::ArithmeticOverflow)?;

    Ok(cost_order != Ordering::Greater
        && risk_order != Ordering::Greater
        && regret_order != Ordering::Greater
        && (cost_order == Ordering::Less
            || risk_order == Ordering::Less
            || regret_order == Ordering::Less))
}

fn push_candidate(
    out: &mut Vec<u8>,
    candidate: &CandidateExperiment,
) -> Result<(), MeasurementSelectionError> {
    push_id(out, &candidate.experiment_id);
    out.push(candidate.modality as u8);
    out.extend_from_slice(&candidate.protocol_digest);
    out.extend_from_slice(&candidate.preregistration_digest);
    out.extend_from_slice(&candidate.risk_assessment_digest);
    push_id(out, &candidate.cost_scale_id);
    push_rational(out, &candidate.acquisition_cost)?;
    push_id(out, &candidate.risk_scale_id);
    push_rational(out, &candidate.risk_upper_bound)?;

    match &candidate.authorization {
        RiskAuthorization::Approved {
            authorization_id,
            safety_case_digest,
        } => {
            out.push(1);
            push_id(out, authorization_id);
            out.extend_from_slice(safety_case_digest);
        }
        RiskAuthorization::NotApproved { reason_id } => {
            out.push(2);
            push_id(out, reason_id);
        }
        RiskAuthorization::Unassessed => out.push(3),
    }

    let mut outcomes = candidate.outcomes.iter().collect::<Vec<_>>();
    outcomes.sort_by(|a, b| a.outcome_id.cmp(&b.outcome_id));
    push_len(out, outcomes.len());
    for outcome in outcomes {
        push_id(out, &outcome.outcome_id);
        push_len(out, outcome.compatible_hypotheses.len());
        for hypothesis_id in &outcome.compatible_hypotheses {
            push_id(out, hypothesis_id);
        }
    }
    Ok(())
}

fn push_rational(
    out: &mut Vec<u8>,
    value: &ExactRational,
) -> Result<(), MeasurementSelectionError> {
    let normalized = ExactRational::new(value.numerator, value.denominator)
        .map_err(|_| MeasurementSelectionError::ArithmeticOverflow)?;
    out.extend_from_slice(&normalized.numerator.to_le_bytes());
    out.extend_from_slice(&normalized.denominator.to_le_bytes());
    Ok(())
}

fn certificate_bytes(certificate: &MeasurementSelectionCertificate) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"symtropy.road-decision-measurement.certificate.v0.1\0");
    push_bytes(&mut out, certificate.schema_id.as_bytes());
    out.extend_from_slice(&certificate.manifest_digest);
    out.extend_from_slice(&certificate.baseline_decision_certificate.receipt_digest);
    out.extend_from_slice(&certificate.baseline_minimax_regret.numerator.to_le_bytes());
    out.extend_from_slice(
        &certificate
            .baseline_minimax_regret
            .denominator
            .to_le_bytes(),
    );

    push_len(&mut out, certificate.candidate_evaluations.len());
    for evaluation in &certificate.candidate_evaluations {
        push_id(&mut out, &evaluation.experiment_id);
        out.push(evaluation.modality as u8);
        out.push(evaluation.disposition as u8);
        out.push(
            evaluation
                .invalid_reason
                .map(|reason| reason as u8)
                .unwrap_or(0),
        );
        out.extend_from_slice(&evaluation.acquisition_cost.numerator.to_le_bytes());
        out.extend_from_slice(&evaluation.acquisition_cost.denominator.to_le_bytes());
        out.extend_from_slice(&evaluation.risk_upper_bound.numerator.to_le_bytes());
        out.extend_from_slice(&evaluation.risk_upper_bound.denominator.to_le_bytes());
        push_option_rational(&mut out, evaluation.baseline_minimax_regret.as_ref());
        push_option_rational(&mut out, evaluation.worst_case_residual_regret.as_ref());
        push_option_rational(&mut out, evaluation.regret_reduction.as_ref());
        push_len(&mut out, evaluation.outcome_evaluations.len());
        for outcome in &evaluation.outcome_evaluations {
            push_id(&mut out, &outcome.outcome_id);
            out.push(outcome.decision_status as u8);
            push_rational_bytes(&mut out, &outcome.minimax_regret);
            push_len(&mut out, outcome.compatible_hypotheses.len());
            for hypothesis_id in &outcome.compatible_hypotheses {
                push_id(&mut out, hypothesis_id);
            }
        }
    }

    push_len(&mut out, certificate.pareto_experiment_ids.len());
    for experiment_id in &certificate.pareto_experiment_ids {
        push_id(&mut out, experiment_id);
    }
    out
}

fn push_option_rational(out: &mut Vec<u8>, value: Option<&ExactRational>) {
    match value {
        Some(value) => {
            out.push(1);
            push_rational_bytes(out, value);
        }
        None => out.push(0),
    }
}

fn push_rational_bytes(out: &mut Vec<u8>, value: &ExactRational) {
    out.extend_from_slice(&value.numerator.to_le_bytes());
    out.extend_from_slice(&value.denominator.to_le_bytes());
}

fn push_id(out: &mut Vec<u8>, id: &StableId) {
    let bytes = id.as_str().as_bytes();
    let length = u32::try_from(bytes.len()).expect("StableId length is bounded");
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(bytes);
}

fn push_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    let length = u64::try_from(bytes.len()).expect("schema byte length must fit u64");
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(bytes);
}

fn push_len(out: &mut Vec<u8>, length: usize) {
    let length = u64::try_from(length).expect("collection length must fit u64");
    out.extend_from_slice(&length.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision_closure::{DecisionHypothesis, HypothesisActionOutcome, OutcomeSupport};

    fn id(value: &str) -> StableId {
        StableId::new(value).expect("fixture ID")
    }

    fn outcome(action: &str, utility: i128) -> HypothesisActionOutcome {
        HypothesisActionOutcome {
            action_id: id(action),
            support: OutcomeSupport::Supported {
                utility: ExactRational::new(utility, 1).unwrap(),
                evidence_digest: *blake3::hash(action.as_bytes()).as_bytes(),
            },
        }
    }

    fn hypothesis(name: &str, a: i128, b: i128) -> DecisionHypothesis {
        DecisionHypothesis {
            hypothesis_id: id(name),
            reference_manifest_digest: *blake3::hash(name.as_bytes()).as_bytes(),
            outcomes: vec![outcome("action-a", a), outcome("action-b", b)],
        }
    }

    fn baseline() -> DecisionManifest {
        DecisionManifest {
            decision_id: id("measurement-selection-crossing-decision"),
            version: id("v0.1"),
            utility_scale_id: id("net-safety-benefit-scale-v1"),
            utility_semantics_id: id("higher-is-better-v1"),
            actions: vec![id("action-a"), id("action-b")],
            hypotheses: vec![hypothesis("model-h1", 10, 0), hypothesis("model-h2", 0, 10)],
        }
    }

    fn candidate(
        name: &str,
        cost: i128,
        risk: i128,
        outcomes: Vec<ExperimentOutcome>,
    ) -> CandidateExperiment {
        CandidateExperiment {
            experiment_id: id(name),
            modality: ExperimentModality::PassiveObservation,
            protocol_digest: [1; 32],
            preregistration_digest: [2; 32],
            risk_assessment_digest: [3; 32],
            cost_scale_id: id("field-hours-v1"),
            acquisition_cost: ExactRational::new(cost, 1).unwrap(),
            risk_scale_id: id("normalized-risk-v1"),
            risk_upper_bound: ExactRational::new(risk, 1).unwrap(),
            authorization: RiskAuthorization::Approved {
                authorization_id: id("independent-risk-review"),
                safety_case_digest: [4; 32],
            },
            outcomes,
        }
    }

    fn singleton_outcomes() -> Vec<ExperimentOutcome> {
        vec![
            ExperimentOutcome {
                outcome_id: id("sensor-signature-h1"),
                compatible_hypotheses: BTreeSet::from([id("model-h1")]),
            },
            ExperimentOutcome {
                outcome_id: id("sensor-signature-h2"),
                compatible_hypotheses: BTreeSet::from([id("model-h2")]),
            },
        ]
    }

    fn constraints() -> ExperimentConstraints {
        ExperimentConstraints {
            cost_scale_id: id("field-hours-v1"),
            maximum_cost: ExactRational::new(20, 1).unwrap(),
            risk_scale_id: id("normalized-risk-v1"),
            maximum_risk: ExactRational::new(10, 1).unwrap(),
        }
    }

    fn manifest(candidates: Vec<CandidateExperiment>) -> MeasurementSelectionManifest {
        MeasurementSelectionManifest {
            selection_id: id("next-safe-road-measurement"),
            version: id("v0.1"),
            baseline: baseline(),
            constraints: constraints(),
            candidates,
        }
    }

    #[test]
    fn perfect_test_reduces_worst_case_regret_to_zero() {
        let certificate = manifest(vec![candidate("perfect-test", 5, 2, singleton_outcomes())])
            .evaluate()
            .unwrap();
        assert_eq!(
            certificate.baseline_minimax_regret,
            ExactRational::new(10, 1).unwrap()
        );
        let evaluation = &certificate.candidate_evaluations[0];
        assert_eq!(
            evaluation.worst_case_residual_regret,
            Some(ExactRational::new(0, 1).unwrap())
        );
        assert_eq!(
            evaluation.regret_reduction,
            Some(ExactRational::new(10, 1).unwrap())
        );
        assert_eq!(evaluation.disposition, CandidateDisposition::ParetoEligible);
    }

    #[test]
    fn uninformative_test_has_zero_decision_regret_reduction() {
        let all_hypotheses = BTreeSet::from([id("model-h1"), id("model-h2")]);
        let candidate = candidate(
            "uninformative-test",
            1,
            0,
            vec![ExperimentOutcome {
                outcome_id: id("same-reading"),
                compatible_hypotheses: all_hypotheses,
            }],
        );
        let evaluation = &manifest(vec![candidate])
            .evaluate()
            .unwrap()
            .candidate_evaluations[0];
        assert_eq!(
            evaluation.worst_case_residual_regret,
            Some(ExactRational::new(10, 1).unwrap())
        );
        assert_eq!(
            evaluation.disposition,
            CandidateDisposition::NoDecisionRegretReduction
        );
    }

    #[test]
    fn noisy_overlapping_outcomes_use_worst_compatible_hypothesis() {
        let candidate = candidate(
            "noisy-test",
            1,
            0,
            vec![
                ExperimentOutcome {
                    outcome_id: id("ambiguous-reading"),
                    compatible_hypotheses: BTreeSet::from([id("model-h1"), id("model-h2")]),
                },
                ExperimentOutcome {
                    outcome_id: id("specific-reading"),
                    compatible_hypotheses: BTreeSet::from([id("model-h2")]),
                },
            ],
        );
        let evaluation = &manifest(vec![candidate])
            .evaluate()
            .unwrap()
            .candidate_evaluations[0];
        assert_eq!(
            evaluation.worst_case_residual_regret,
            Some(ExactRational::new(10, 1).unwrap())
        );
        assert_eq!(
            evaluation.disposition,
            CandidateDisposition::NoDecisionRegretReduction
        );
    }

    #[test]
    fn incomplete_outcome_coverage_is_not_credited_as_information() {
        let candidate = candidate(
            "incomplete-test",
            1,
            0,
            vec![ExperimentOutcome {
                outcome_id: id("only-h1"),
                compatible_hypotheses: BTreeSet::from([id("model-h1")]),
            }],
        );
        let evaluation = &manifest(vec![candidate])
            .evaluate()
            .unwrap()
            .candidate_evaluations[0];
        assert_eq!(evaluation.disposition, CandidateDisposition::Incomplete);
        assert_eq!(
            evaluation.invalid_reason,
            Some(CandidateInvalidReason::IncompleteHypothesisCoverage)
        );
        assert_eq!(evaluation.regret_reduction, None);
    }

    #[test]
    fn unknown_or_empty_compatible_hypotheses_reject() {
        let unknown = candidate(
            "unknown-test",
            1,
            0,
            vec![ExperimentOutcome {
                outcome_id: id("unknown-reading"),
                compatible_hypotheses: BTreeSet::from([id("model-unknown")]),
            }],
        );
        let empty = candidate(
            "empty-test",
            1,
            0,
            vec![ExperimentOutcome {
                outcome_id: id("empty-reading"),
                compatible_hypotheses: BTreeSet::new(),
            }],
        );
        let evaluations = manifest(vec![unknown, empty]).evaluate().unwrap();
        assert_eq!(
            evaluations.candidate_evaluations[0].invalid_reason,
            Some(CandidateInvalidReason::EmptyCompatibleHypothesisSet)
        );
        assert_eq!(
            evaluations.candidate_evaluations[1].invalid_reason,
            Some(CandidateInvalidReason::UnknownCompatibleHypothesis)
        );
    }

    #[test]
    fn budget_and_risk_are_hard_constraints_not_utility_penalties() {
        let over_budget = candidate("expensive", 21, 1, singleton_outcomes());
        let over_risk = candidate("risky", 1, 11, singleton_outcomes());
        let evaluations = manifest(vec![over_budget, over_risk]).evaluate().unwrap();
        assert_eq!(
            evaluations.candidate_evaluations[0].disposition,
            CandidateDisposition::OverBudget
        );
        assert_eq!(
            evaluations.candidate_evaluations[1].disposition,
            CandidateDisposition::RiskLimitExceeded
        );
        assert!(evaluations.pareto_experiment_ids.is_empty());
    }

    #[test]
    fn unapproved_and_unassessed_experiments_cannot_enter_frontier() {
        let mut unapproved = candidate("unapproved", 1, 1, singleton_outcomes());
        unapproved.authorization = RiskAuthorization::NotApproved {
            reason_id: id("safety-case-pending"),
        };
        let mut unassessed = candidate("unassessed", 1, 1, singleton_outcomes());
        unassessed.authorization = RiskAuthorization::Unassessed;
        let evaluations = manifest(vec![unapproved, unassessed]).evaluate().unwrap();
        assert_eq!(
            evaluations.candidate_evaluations[0].disposition,
            CandidateDisposition::NotApproved
        );
        assert_eq!(
            evaluations.candidate_evaluations[1].disposition,
            CandidateDisposition::Unassessed
        );
    }

    #[test]
    fn incompatible_cost_or_risk_scales_are_not_compared() {
        let mut candidate = candidate("wrong-cost-scale", 1, 1, singleton_outcomes());
        candidate.cost_scale_id = id("vehicle-hours-v1");
        let evaluation = &manifest(vec![candidate])
            .evaluate()
            .unwrap()
            .candidate_evaluations[0];
        assert_eq!(
            evaluation.invalid_reason,
            Some(CandidateInvalidReason::CostScaleMismatch)
        );
    }

    #[test]
    fn protocol_preregistration_and_risk_assessment_are_required() {
        for missing in 0..3 {
            let mut candidate = candidate("missing-digest", 1, 1, singleton_outcomes());
            match missing {
                0 => candidate.protocol_digest = [0; 32],
                1 => candidate.preregistration_digest = [0; 32],
                2 => candidate.risk_assessment_digest = [0; 32],
                _ => unreachable!(),
            }
            let evaluation = &manifest(vec![candidate])
                .evaluate()
                .unwrap()
                .candidate_evaluations[0];
            assert_eq!(evaluation.disposition, CandidateDisposition::Incomplete);
        }
    }

    #[test]
    fn pareto_frontier_keeps_cost_risk_tradeoffs_and_removes_dominated_plan() {
        let low_cost_higher_risk = candidate("low-cost", 5, 6, singleton_outcomes());
        let high_cost_low_risk = candidate("low-risk", 8, 4, singleton_outcomes());
        let dominated = candidate("dominated", 8, 8, singleton_outcomes());
        let certificate = manifest(vec![low_cost_higher_risk, high_cost_low_risk, dominated])
            .evaluate()
            .unwrap();
        assert_eq!(
            certificate.pareto_experiment_ids,
            vec![id("low-cost"), id("low-risk")]
        );
        let dominated_evaluation = certificate
            .candidate_evaluations
            .iter()
            .find(|evaluation| evaluation.experiment_id == id("dominated"))
            .expect("dominated candidate remains in the report");
        assert_eq!(
            dominated_evaluation.disposition,
            CandidateDisposition::Dominated
        );
    }

    #[test]
    fn declaration_order_does_not_change_manifest_or_certificate() {
        let first = manifest(vec![
            candidate("test-a", 5, 2, singleton_outcomes()),
            candidate("test-b", 8, 4, singleton_outcomes()),
        ]);
        let mut second = first.clone();
        second.candidates.reverse();
        for candidate in &mut second.candidates {
            candidate.outcomes.reverse();
            for outcome in &mut candidate.outcomes {
                outcome.compatible_hypotheses = outcome
                    .compatible_hypotheses
                    .iter()
                    .cloned()
                    .rev()
                    .collect();
            }
        }
        assert_eq!(first.digest().unwrap(), second.digest().unwrap());
        assert_eq!(first.evaluate().unwrap(), second.evaluate().unwrap());
    }

    #[test]
    fn receipt_tampering_and_frozen_manifest_mutation_are_detected() {
        let first = manifest(vec![candidate("test-a", 5, 2, singleton_outcomes())]);
        let certificate = first.evaluate().unwrap();
        assert!(certificate.integrity_check());
        assert_eq!(certificate.verify_against(&first), Ok(()));

        let mut changed = first;
        changed.candidates[0].protocol_digest[0] ^= 1;
        assert_eq!(
            certificate.verify_against(&changed),
            Err(MeasurementSelectionError::ReceiptMismatch)
        );

        let mut tampered = certificate;
        tampered.manifest_digest[0] ^= 1;
        assert!(!tampered.integrity_check());
    }

    #[test]
    fn baseline_without_complete_action_hypothesis_support_is_rejected() {
        let mut invalid = baseline();
        invalid.hypotheses[0].outcomes[0].support = OutcomeSupport::NotIdentified {
            evidence_ref: id("not-supported"),
        };
        let manifest = MeasurementSelectionManifest {
            selection_id: id("invalid-baseline"),
            version: id("v0.1"),
            baseline: invalid,
            constraints: constraints(),
            candidates: vec![candidate("test", 1, 1, singleton_outcomes())],
        };
        assert_eq!(
            manifest.evaluate(),
            Err(MeasurementSelectionError::BaselineNotFullySupported)
        );
    }

    #[test]
    fn equivalent_cost_and_risk_rationals_have_identical_manifest_and_certificate() {
        let first = manifest(vec![candidate("same-test", 5, 2, singleton_outcomes())]);
        let mut second = first.clone();
        second.candidates[0].acquisition_cost = ExactRational {
            numerator: 10,
            denominator: 2,
        };
        second.candidates[0].risk_upper_bound = ExactRational {
            numerator: 4,
            denominator: 2,
        };
        assert_eq!(first.digest().unwrap(), second.digest().unwrap());
        assert_eq!(first.evaluate().unwrap(), second.evaluate().unwrap());
    }

    #[test]
    fn declared_approval_without_safety_case_digest_is_incomplete() {
        let mut candidate = candidate("no-safety-case", 1, 1, singleton_outcomes());
        candidate.authorization = RiskAuthorization::Approved {
            authorization_id: id("approval-ref"),
            safety_case_digest: [0; 32],
        };
        let evaluation = &manifest(vec![candidate])
            .evaluate()
            .unwrap()
            .candidate_evaluations[0];
        assert_eq!(evaluation.disposition, CandidateDisposition::Incomplete);
        assert_eq!(
            evaluation.invalid_reason,
            Some(CandidateInvalidReason::MissingSafetyCaseDigest)
        );
    }

    #[test]
    fn simulation_challenge_is_a_distinct_non_empirical_modality() {
        let mut candidate = candidate("simulation-only", 1, 1, singleton_outcomes());
        candidate.modality = ExperimentModality::SimulationChallenge;
        let evaluation = &manifest(vec![candidate])
            .evaluate()
            .unwrap()
            .candidate_evaluations[0];
        assert_eq!(evaluation.modality, ExperimentModality::SimulationChallenge);
        assert_eq!(evaluation.disposition, CandidateDisposition::ParetoEligible);
        // Eligibility here only means an informative, authorized challenge plan.
        // It does not promote a simulator result to real-world evidence.
    }
}
