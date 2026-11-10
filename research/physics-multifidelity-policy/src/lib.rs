//! Evidence-gated selection of an already-qualified physics model.
//!
//! This package does not execute a simulator, estimate physical truth, create
//! model credibility, or authorize actuation. It validates a finite declared
//! candidate set and chooses the least costly candidate that already satisfies
//! the caller's explicit admission contract. Unknown applicability abstains.
//!
//! The wire format deliberately avoids a weighted "fidelity score": semantic
//! equivalence, evidence class, validity envelope, hard constraints, error
//! bounds, and resource budgets are independent gates.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const SCHEMA_VERSION: u32 = 1;
const CLAIM_CEILING: &str =
    "finite-input-policy-conformance-only;does-not-qualify-a-physical-model-or-authorize-actuation";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PolicyInputV1 {
    pub schema_version: u32,
    pub decision_risk: DecisionRisk,
    pub scenario_id: String,
    pub scenario_manifest_sha256: String,
    pub requirement: QuantityRequirement,
    pub budget: ResourceBudget,
    pub candidates: Vec<ModelCandidate>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct QuantityRequirement {
    pub quantity_id: String,
    pub unit: String,
    pub reference_frame: String,
    pub max_residual_upper_bound: f64,
    pub max_uncertainty_upper_bound: f64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DecisionRisk {
    Exploratory,
    EngineeringDecision,
    SafetyCritical,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ResourceBudget {
    pub max_p95_wall_time_us: u64,
    pub max_peak_memory_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ModelCandidate {
    pub model_id: String,
    pub backend: String,
    pub backend_version: String,
    pub model_passport_sha256: String,
    pub capability_census_sha256: String,
    pub scenario_mapping_sha256: String,
    pub semantic_equivalence: SemanticEquivalence,
    pub evidence_class: EvidenceClass,
    /// Optional analytic/manufactured-solution anchor bound to its evidence receipt.
    pub analytic_anchor_sha256: Option<String>,
    /// Distinct independent validation receipts; syntax and uniqueness are checked here.
    pub independent_validation_receipt_sha256s: Vec<String>,
    pub validity: ValidityDisposition,
    pub horizon_covered: bool,
    pub hard_constraints: HardConstraintDisposition,
    pub residual_upper_bound: f64,
    pub uncertainty_upper_bound: f64,
    pub p95_wall_time_us: u64,
    pub peak_memory_bytes: u64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SemanticEquivalence {
    Matched,
    Mismatch,
    Unknown,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    AnalyticOracle,
    IndependentlyValidated,
    SyntheticOnly,
    Unqualified,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidityDisposition {
    InsideDeclaredEnvelope,
    OutsideDeclaredEnvelope,
    Unknown,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HardConstraintDisposition {
    WithinBounds,
    Violated,
    Unknown,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RejectionReason {
    SemanticMismatch,
    SemanticEquivalenceUnknown,
    EvidenceRiskFloorNotMet,
    AnalyticAnchorMissing,
    InsufficientIndependentValidationPaths,
    EvidenceSyntheticOnly,
    EvidenceUnqualified,
    ValidityOutsideEnvelope,
    ValidityUnknown,
    HorizonNotCovered,
    HardConstraintViolation,
    HardConstraintUnknown,
    ResidualBoundExceeded,
    UncertaintyBoundExceeded,
    WallTimeBudgetExceeded,
    MemoryBudgetExceeded,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CandidateEvaluation {
    pub model_id: String,
    pub admissible: bool,
    pub rejection_reasons: Vec<RejectionReason>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum PolicyDecision {
    Selected {
        model_id: String,
        model_passport_sha256: String,
    },
    Abstain {
        reason: AbstentionReason,
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AbstentionReason {
    NoCandidates,
    NoAdmissibleCandidate,
    ObservedValidityEnvelopeExit,
    ObservedHardConstraintViolation,
    ObservedHardConstraintUnknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PolicyReceiptV1 {
    pub schema_version: u32,
    pub input_sha256: String,
    pub scenario_id: String,
    pub decision: PolicyDecision,
    pub evaluations: Vec<CandidateEvaluation>,
    pub claim_ceiling: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RunObservation {
    /// Digest of the exact policy input/run configuration this observation belongs to.
    pub input_sha256: String,
    pub model_id: String,
    /// Exact model passport selected by the policy for this run.
    pub model_passport_sha256: String,
    /// Digest of the observation record produced by the measurement/replay owner.
    pub observation_receipt_sha256: String,
    pub observed_residual: f64,
    pub observed_uncertainty: f64,
    pub validity_inside_envelope: bool,
    pub hard_constraints: HardConstraintDisposition,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum MonitoringDecision {
    Continue {
        model_id: String,
        observation_receipt_sha256: String,
    },
    Escalate {
        from_model_id: String,
        to_model_id: String,
        next_input_sha256: String,
        observation_receipt_sha256: String,
    },
    Abstain {
        from_model_id: String,
        reason: AbstentionReason,
        trigger: MonitorTrigger,
        observation_receipt_sha256: String,
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MonitorTrigger {
    ResidualExceeded,
    UncertaintyExceeded,
    ValidityEnvelopeExited,
    HardConstraintViolation,
    HardConstraintUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyError {
    UnsupportedSchema(u32),
    EmptyField(&'static str),
    InvalidSha256(&'static str),
    NonFiniteOrNegative(&'static str),
    ZeroResourceBudget(&'static str),
    DuplicateModelId(String),
    ReceiptInputMismatch,
    ReceiptMismatch,
    ObservationInputMismatch,
    ObservationReceiptInvalid,
    ObservationModelMismatch,
    ObservationPassportMismatch,
    SelectedModelMissing,
    InvalidSelectedModel,
    Serialization(String),
}

impl std::fmt::Display for PolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSchema(v) => write!(f, "unsupported schema version: {v}"),
            Self::EmptyField(name) => write!(f, "required field is empty: {name}"),
            Self::InvalidSha256(name) => write!(f, "invalid SHA-256 identity: {name}"),
            Self::NonFiniteOrNegative(name) => {
                write!(f, "value must be finite and non-negative: {name}")
            }
            Self::ZeroResourceBudget(name) => write!(f, "resource budget must be non-zero: {name}"),
            Self::DuplicateModelId(id) => write!(f, "duplicate model id: {id}"),
            Self::ReceiptInputMismatch => write!(f, "receipt does not bind the supplied input"),
            Self::ReceiptMismatch => write!(f, "receipt differs from the canonical evaluation of the input"),
            Self::ObservationInputMismatch => write!(f, "observation does not bind the supplied policy input"),
            Self::ObservationReceiptInvalid => write!(f, "observation receipt digest is invalid"),
            Self::ObservationModelMismatch => {
                write!(f, "observation model does not match the selected model")
            }
            Self::ObservationPassportMismatch => write!(f, "observation model passport does not match the selected candidate"),
            Self::SelectedModelMissing => write!(f, "selected model is absent from the input"),
            Self::InvalidSelectedModel => write!(f, "receipt does not select an admissible model"),
            Self::Serialization(message) => write!(f, "serialization failure: {message}"),
        }
    }
}

impl std::error::Error for PolicyError {}

/// Evaluate a frozen candidate set and return a deterministic receipt.
///
/// Selection is lexicographic: p95 wall time, peak memory, then model id.
/// No weighted score can compensate for a failed semantic, evidence,
/// applicability, hard-constraint, error-bound, or resource gate.
pub fn evaluate(input: &PolicyInputV1) -> Result<PolicyReceiptV1, PolicyError> {
    validate_input(input)?;
    let input_sha256 = canonical_input_sha256(input)?;
    let mut candidates = input.candidates.clone();
    candidates.sort_by(|a, b| a.model_id.cmp(&b.model_id));

    let mut evaluations = Vec::with_capacity(candidates.len());
    let mut admissible: Vec<&ModelCandidate> = Vec::new();

    for candidate in &candidates {
        let reasons = rejection_reasons(input, candidate);
        let accepted = reasons.is_empty();
        if accepted {
            admissible.push(candidate);
        }
        evaluations.push(CandidateEvaluation {
            model_id: candidate.model_id.clone(),
            admissible: accepted,
            rejection_reasons: reasons,
        });
    }

    let decision = admissible
        .into_iter()
        .min_by(|a, b| {
            a.p95_wall_time_us
                .cmp(&b.p95_wall_time_us)
                .then_with(|| a.peak_memory_bytes.cmp(&b.peak_memory_bytes))
                .then_with(|| a.model_id.cmp(&b.model_id))
        })
        .map(|candidate| PolicyDecision::Selected {
            model_id: candidate.model_id.clone(),
            model_passport_sha256: candidate.model_passport_sha256.clone(),
        })
        .unwrap_or_else(|| PolicyDecision::Abstain {
            reason: if input.candidates.is_empty() {
                AbstentionReason::NoCandidates
            } else {
                AbstentionReason::NoAdmissibleCandidate
            },
        });

    Ok(PolicyReceiptV1 {
        schema_version: SCHEMA_VERSION,
        input_sha256,
        scenario_id: input.scenario_id.clone(),
        decision,
        evaluations,
        claim_ceiling: CLAIM_CEILING.to_owned(),
    })
}

/// Monitor an admitted run. A validity excursion, hard-constraint problem,
/// residual breach or uncertainty breach forces escalation to the next
/// independently admissible candidate, or abstention. Tolerances are never
/// relaxed during escalation.
pub fn monitor(
    input: &PolicyInputV1,
    receipt: &PolicyReceiptV1,
    observation: &RunObservation,
) -> Result<MonitoringDecision, PolicyError> {
    validate_input(input)?;
    let expected_receipt = evaluate(input)?;
    let expected_digest = expected_receipt.input_sha256.clone();

    if receipt.input_sha256 != expected_digest {
        return Err(PolicyError::ReceiptInputMismatch);
    }
    // A matching input digest alone is insufficient: the selected candidate,
    // evaluation dispositions and claim ceiling must also be canonical.
    if receipt != &expected_receipt {
        return Err(PolicyError::ReceiptMismatch);
    }
    validate_sha256(&observation.input_sha256, "observation.input_sha256")?;
    if observation.input_sha256 != expected_digest {
        return Err(PolicyError::ObservationInputMismatch);
    }
    validate_sha256(
        &observation.observation_receipt_sha256,
        "observation.observation_receipt_sha256",
    )
    .map_err(|_| PolicyError::ObservationReceiptInvalid)?;

    let selected_id = match &receipt.decision {
        PolicyDecision::Selected { model_id, .. } => model_id,
        PolicyDecision::Abstain { .. } => return Err(PolicyError::InvalidSelectedModel),
    };
    if selected_id != &observation.model_id {
        return Err(PolicyError::ObservationModelMismatch);
    }

    let selected = input
        .candidates
        .iter()
        .find(|candidate| &candidate.model_id == selected_id)
        .ok_or(PolicyError::SelectedModelMissing)?;

    if !rejection_reasons(input, selected).is_empty() {
        return Err(PolicyError::InvalidSelectedModel);
    }
    validate_sha256(
        &observation.model_passport_sha256,
        "observation.model_passport_sha256",
    )?;
    if observation.model_passport_sha256 != selected.model_passport_sha256 {
        return Err(PolicyError::ObservationPassportMismatch);
    }

    validate_nonnegative_finite(observation.observed_residual, "observed_residual")?;
    validate_nonnegative_finite(observation.observed_uncertainty, "observed_uncertainty")?;

    // These are stop conditions, not ordinary fidelity-escalation signals.
    // A hard-constraint problem or validity-domain exit requires a fresh
    // state-bound admission; the old scenario receipt cannot authorize another
    // model for a changed or unsafe state.
    let stop = match observation.hard_constraints {
        HardConstraintDisposition::Violated => Some((
            AbstentionReason::ObservedHardConstraintViolation,
            MonitorTrigger::HardConstraintViolation,
        )),
        HardConstraintDisposition::Unknown => Some((
            AbstentionReason::ObservedHardConstraintUnknown,
            MonitorTrigger::HardConstraintUnknown,
        )),
        HardConstraintDisposition::WithinBounds if !observation.validity_inside_envelope => Some((
            AbstentionReason::ObservedValidityEnvelopeExit,
            MonitorTrigger::ValidityEnvelopeExited,
        )),
        HardConstraintDisposition::WithinBounds => None,
    };
    if let Some((reason, trigger)) = stop {
        return Ok(MonitoringDecision::Abstain {
            from_model_id: selected_id.clone(),
            reason,
            trigger,
            observation_receipt_sha256: observation.observation_receipt_sha256.clone(),
        });
    }

    // Only quantitative residual/uncertainty excursions inside the declared
    // validity domain may request escalation to a pre-admitted peer.
    let trigger = if observation.observed_residual > selected.residual_upper_bound
        || observation.observed_residual > input.requirement.max_residual_upper_bound
    {
        Some(MonitorTrigger::ResidualExceeded)
    } else if observation.observed_uncertainty > selected.uncertainty_upper_bound
        || observation.observed_uncertainty > input.requirement.max_uncertainty_upper_bound
    {
        Some(MonitorTrigger::UncertaintyExceeded)
    } else {
        None
    };

    let Some(trigger) = trigger else {
        return Ok(MonitoringDecision::Continue {
            model_id: selected_id.clone(),
            observation_receipt_sha256: observation.observation_receipt_sha256.clone(),
        });
    };

    let mut remaining = input.clone();
    remaining
        .candidates
        .retain(|candidate| candidate.model_id != *selected_id);
    let next = evaluate(&remaining)?;

    match next.decision {
        PolicyDecision::Selected { model_id, .. } => Ok(MonitoringDecision::Escalate {
            from_model_id: selected_id.clone(),
            to_model_id: model_id,
            next_input_sha256: next.input_sha256,
            observation_receipt_sha256: observation.observation_receipt_sha256.clone(),
        }),
        PolicyDecision::Abstain { reason } => Ok(MonitoringDecision::Abstain {
            from_model_id: selected_id.clone(),
            reason,
            trigger,
            observation_receipt_sha256: observation.observation_receipt_sha256.clone(),
        }),
    }
}

fn validate_input(input: &PolicyInputV1) -> Result<(), PolicyError> {
    if input.schema_version != SCHEMA_VERSION {
        return Err(PolicyError::UnsupportedSchema(input.schema_version));
    }
    validate_nonempty(&input.scenario_id, "scenario_id")?;
    validate_sha256(&input.scenario_manifest_sha256, "scenario_manifest_sha256")?;
    validate_nonempty(&input.requirement.quantity_id, "requirement.quantity_id")?;
    validate_nonempty(&input.requirement.unit, "requirement.unit")?;
    validate_nonempty(&input.requirement.reference_frame, "requirement.reference_frame")?;
    validate_nonnegative_finite(
        input.requirement.max_residual_upper_bound,
        "max_residual_upper_bound",
    )?;
    validate_nonnegative_finite(
        input.requirement.max_uncertainty_upper_bound,
        "max_uncertainty_upper_bound",
    )?;
    if input.budget.max_p95_wall_time_us == 0 {
        return Err(PolicyError::ZeroResourceBudget("max_p95_wall_time_us"));
    }
    if input.budget.max_peak_memory_bytes == 0 {
        return Err(PolicyError::ZeroResourceBudget("max_peak_memory_bytes"));
    }

    let mut ids = BTreeSet::new();
    for candidate in &input.candidates {
        validate_nonempty(&candidate.model_id, "candidate.model_id")?;
        validate_nonempty(&candidate.backend, "candidate.backend")?;
        validate_nonempty(&candidate.backend_version, "candidate.backend_version")?;
        validate_sha256(&candidate.model_passport_sha256, "model_passport_sha256")?;
        validate_sha256(&candidate.capability_census_sha256, "capability_census_sha256")?;
        validate_sha256(&candidate.scenario_mapping_sha256, "scenario_mapping_sha256")?;
        if let Some(anchor) = &candidate.analytic_anchor_sha256 {
            validate_sha256(anchor, "candidate.analytic_anchor_sha256")?;
        }
        let mut validation_receipts = BTreeSet::new();
        for digest in &candidate.independent_validation_receipt_sha256s {
            validate_sha256(digest, "candidate.independent_validation_receipt_sha256s[]")?;
            if !validation_receipts.insert(digest) {
                return Err(PolicyError::InvalidSha256(
                    "duplicate candidate independent validation receipt",
                ));
            }
        }
        validate_nonnegative_finite(candidate.residual_upper_bound, "residual_upper_bound")?;
        validate_nonnegative_finite(
            candidate.uncertainty_upper_bound,
            "uncertainty_upper_bound",
        )?;
        if candidate.p95_wall_time_us == 0 {
            return Err(PolicyError::ZeroResourceBudget("candidate.p95_wall_time_us"));
        }
        if candidate.peak_memory_bytes == 0 {
            return Err(PolicyError::ZeroResourceBudget("candidate.peak_memory_bytes"));
        }
        if !ids.insert(candidate.model_id.clone()) {
            return Err(PolicyError::DuplicateModelId(candidate.model_id.clone()));
        }
    }
    Ok(())
}

fn rejection_reasons(
    input: &PolicyInputV1,
    candidate: &ModelCandidate,
) -> Vec<RejectionReason> {
    let mut reasons = Vec::new();

    match candidate.semantic_equivalence {
        SemanticEquivalence::Matched => {}
        SemanticEquivalence::Mismatch => reasons.push(RejectionReason::SemanticMismatch),
        SemanticEquivalence::Unknown => {
            reasons.push(RejectionReason::SemanticEquivalenceUnknown)
        }
    }

    match input.decision_risk {
        DecisionRisk::Exploratory => match candidate.evidence_class {
            EvidenceClass::AnalyticOracle
            | EvidenceClass::IndependentlyValidated
            | EvidenceClass::SyntheticOnly => {}
            EvidenceClass::Unqualified => {
                reasons.push(RejectionReason::EvidenceUnqualified)
            }
        },
        DecisionRisk::EngineeringDecision => match candidate.evidence_class {
            EvidenceClass::AnalyticOracle | EvidenceClass::IndependentlyValidated => {}
            EvidenceClass::SyntheticOnly => {
                reasons.push(RejectionReason::EvidenceSyntheticOnly)
            }
            EvidenceClass::Unqualified => {
                reasons.push(RejectionReason::EvidenceUnqualified)
            }
        },
        DecisionRisk::SafetyCritical => {
            if candidate.evidence_class != EvidenceClass::IndependentlyValidated {
                reasons.push(RejectionReason::EvidenceRiskFloorNotMet);
            }
            if candidate.analytic_anchor_sha256.is_none() {
                reasons.push(RejectionReason::AnalyticAnchorMissing);
            }
            if candidate.independent_validation_receipt_sha256s.len() < 2 {
                reasons.push(RejectionReason::InsufficientIndependentValidationPaths);
            }
        }
    }

    match candidate.validity {
        ValidityDisposition::InsideDeclaredEnvelope => {}
        ValidityDisposition::OutsideDeclaredEnvelope => {
            reasons.push(RejectionReason::ValidityOutsideEnvelope)
        }
        ValidityDisposition::Unknown => reasons.push(RejectionReason::ValidityUnknown),
    }

    if !candidate.horizon_covered {
        reasons.push(RejectionReason::HorizonNotCovered);
    }

    match candidate.hard_constraints {
        HardConstraintDisposition::WithinBounds => {}
        HardConstraintDisposition::Violated => {
            reasons.push(RejectionReason::HardConstraintViolation)
        }
        HardConstraintDisposition::Unknown => {
            reasons.push(RejectionReason::HardConstraintUnknown)
        }
    }

    if candidate.residual_upper_bound > input.requirement.max_residual_upper_bound {
        reasons.push(RejectionReason::ResidualBoundExceeded);
    }
    if candidate.uncertainty_upper_bound > input.requirement.max_uncertainty_upper_bound {
        reasons.push(RejectionReason::UncertaintyBoundExceeded);
    }
    if candidate.p95_wall_time_us > input.budget.max_p95_wall_time_us {
        reasons.push(RejectionReason::WallTimeBudgetExceeded);
    }
    if candidate.peak_memory_bytes > input.budget.max_peak_memory_bytes {
        reasons.push(RejectionReason::MemoryBudgetExceeded);
    }


    reasons
}

fn canonical_input_sha256(input: &PolicyInputV1) -> Result<String, PolicyError> {
    let mut canonical = input.clone();
    canonical
        .candidates
        .sort_by(|a, b| a.model_id.cmp(&b.model_id));
    let bytes = serde_json::to_vec(&canonical)
        .map_err(|error| PolicyError::Serialization(error.to_string()))?;
    Ok(hex_sha256(&bytes))
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

fn validate_nonempty(value: &str, field: &'static str) -> Result<(), PolicyError> {
    if value.trim().is_empty() {
        Err(PolicyError::EmptyField(field))
    } else {
        Ok(())
    }
}

fn validate_sha256(value: &str, field: &'static str) -> Result<(), PolicyError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(PolicyError::InvalidSha256(field));
    }
    Ok(())
}

fn validate_nonnegative_finite(value: f64, field: &'static str) -> Result<(), PolicyError> {
    if !value.is_finite() || value < 0.0 {
        return Err(PolicyError::NonFiniteOrNegative(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const H1: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const H2: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const H3: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    fn candidate(id: &str, runtime: u64, memory: u64) -> ModelCandidate {
        ModelCandidate {
            model_id: id.to_owned(),
            backend: "reference-solver".to_owned(),
            backend_version: "1.0.0".to_owned(),
            model_passport_sha256: H1.to_owned(),
            capability_census_sha256: H2.to_owned(),
            scenario_mapping_sha256: H3.to_owned(),
            semantic_equivalence: SemanticEquivalence::Matched,
            evidence_class: EvidenceClass::IndependentlyValidated,
            analytic_anchor_sha256: None,
            independent_validation_receipt_sha256s: vec![],
            validity: ValidityDisposition::InsideDeclaredEnvelope,
            horizon_covered: true,
            hard_constraints: HardConstraintDisposition::WithinBounds,
            residual_upper_bound: 0.01,
            uncertainty_upper_bound: 0.02,
            p95_wall_time_us: runtime,
            peak_memory_bytes: memory,
        }
    }

    fn input() -> PolicyInputV1 {
        PolicyInputV1 {
            schema_version: SCHEMA_VERSION,
            decision_risk: DecisionRisk::EngineeringDecision,
            scenario_id: "rigid-contact-known-answer-01".to_owned(),
            scenario_manifest_sha256: H1.to_owned(),
            requirement: QuantityRequirement {
                quantity_id: "contact_impulse".to_owned(),
                unit: "N*s".to_owned(),
                reference_frame: "world".to_owned(),
                max_residual_upper_bound: 0.1,
                max_uncertainty_upper_bound: 0.1,
            },
            budget: ResourceBudget {
                max_p95_wall_time_us: 10_000,
                max_peak_memory_bytes: 10_000_000,
            },
            candidates: vec![
                candidate("slow-qualified", 900, 2_000),
                candidate("fast-qualified", 400, 4_000),
            ],
        }
    }

    #[test]
    fn selects_least_runtime_candidate_inside_all_gates() {
        let receipt = evaluate(&input()).expect("valid fixture");
        assert_eq!(
            receipt.decision,
            PolicyDecision::Selected {
                model_id: "fast-qualified".to_owned(),
                model_passport_sha256: H1.to_owned(),
            }
        );
    }

    #[test]
    fn input_order_does_not_change_digest_or_decision() {
        let mut forward = input();
        let mut reverse = forward.clone();
        reverse.candidates.reverse();
        let a = evaluate(&forward).expect("forward");
        let b = evaluate(&reverse).expect("reverse");
        assert_eq!(a.input_sha256, b.input_sha256);
        assert_eq!(a.decision, b.decision);
        assert_eq!(a.evaluations, b.evaluations);
        forward.candidates.sort_by(|x, y| x.model_id.cmp(&y.model_id));
    }

    #[test]
    fn missing_candidate_changes_frozen_input_digest() {
        let mut reduced = input();
        let full = evaluate(&reduced).expect("full");
        reduced.candidates.pop();
        let partial = evaluate(&reduced).expect("partial");
        assert_ne!(full.input_sha256, partial.input_sha256);
    }

    #[test]
    fn synthetic_only_model_cannot_satisfy_engineering_gate() {
        let mut data = input();
        data.candidates[0].evidence_class = EvidenceClass::SyntheticOnly;
        let receipt = evaluate(&data).expect("synthetic candidate is a valid input");
        assert!(matches!(
            receipt.decision,
            PolicyDecision::Selected { ref model_id, .. } if model_id == "fast-qualified"
        ));
        assert!(receipt.evaluations.iter().any(|entry| {
            entry.model_id == "slow-qualified"
                && entry
                    .rejection_reasons
                    .contains(&RejectionReason::EvidenceSyntheticOnly)
        }));
    }

    #[test]
    fn safety_critical_risk_requires_independent_validation_and_analytic_anchor() {
        let mut data = input();
        data.decision_risk = DecisionRisk::SafetyCritical;
        data.candidates[0].evidence_class = EvidenceClass::IndependentlyValidated;
        data.candidates[0].independent_validation_receipt_sha256s = vec![H2.to_owned()];
        data.candidates[1].evidence_class = EvidenceClass::IndependentlyValidated;
        data.candidates[1].analytic_anchor_sha256 = Some(H2.to_owned());
        data.candidates[1].independent_validation_receipt_sha256s =
            vec![H1.to_owned(), H3.to_owned()];

        let receipt = evaluate(&data).expect("well-formed risk-tiered evidence");
        let fast = receipt.evaluations.iter().find(|item| item.model_id == "fast-qualified").expect("fast");
        assert!(fast.rejection_reasons.contains(&RejectionReason::AnalyticAnchorMissing));
        assert!(fast.rejection_reasons.contains(&RejectionReason::InsufficientIndependentValidationPaths));
        assert_eq!(
            receipt.decision,
            PolicyDecision::Selected {
                model_id: "slow-qualified".to_owned(),
                model_passport_sha256: H1.to_owned(),
            }
        );
    }

    #[test]
    fn synthetic_candidate_is_usable_only_when_request_explicitly_allows_it() {
        let mut data = input();
        data.candidates.truncate(1);
        data.candidates[0].evidence_class = EvidenceClass::SyntheticOnly;
        data.decision_risk = DecisionRisk::Exploratory;
        let receipt = evaluate(&data).expect("synthetic exploration input");
        assert!(matches!(
            receipt.decision,
            PolicyDecision::Selected { ref model_id, .. } if model_id == "slow-qualified"
        ));
    }

    #[test]
    fn unknown_validity_is_rejected_not_interpreted_as_low_confidence() {
        let mut data = input();
        data.candidates[0].validity = ValidityDisposition::Unknown;
        data.candidates[1].validity = ValidityDisposition::Unknown;
        let receipt = evaluate(&data).expect("unknown is a well-formed disposition");
        assert_eq!(
            receipt.decision,
            PolicyDecision::Abstain {
                reason: AbstentionReason::NoAdmissibleCandidate
            }
        );
    }

    #[test]
    fn hard_constraint_violation_cannot_be_compensated_by_speed() {
        let mut data = input();
        data.candidates[1].hard_constraints = HardConstraintDisposition::Violated;
        let receipt = evaluate(&data).expect("well-formed input");
        assert_eq!(
            receipt.decision,
            PolicyDecision::Selected {
                model_id: "slow-qualified".to_owned(),
                model_passport_sha256: H1.to_owned(),
            }
        );
        assert!(receipt.evaluations.iter().any(|entry| {
            entry.model_id == "fast-qualified"
                && entry
                    .rejection_reasons
                    .contains(&RejectionReason::HardConstraintViolation)
        }));
    }

    #[test]
    fn no_admissible_candidate_abstains_without_relaxing_limits() {
        let mut data = input();
        for candidate in &mut data.candidates {
            candidate.residual_upper_bound = 1.0;
        }
        let receipt = evaluate(&data).expect("well-formed input");
        assert_eq!(
            receipt.decision,
            PolicyDecision::Abstain {
                reason: AbstentionReason::NoAdmissibleCandidate
            }
        );
    }

    #[test]
    fn monitor_continues_when_observations_remain_inside_bounds() {
        let data = input();
        let receipt = evaluate(&data).expect("input");
        let observation = RunObservation {
            input_sha256: receipt.input_sha256.clone(),
            model_id: "fast-qualified".to_owned(),
            model_passport_sha256: H1.to_owned(),
            observation_receipt_sha256: H3.to_owned(),
            observed_residual: 0.005,
            observed_uncertainty: 0.01,
            validity_inside_envelope: true,
            hard_constraints: HardConstraintDisposition::WithinBounds,
        };
        assert_eq!(
            monitor(&data, &receipt, &observation).expect("monitor"),
            MonitoringDecision::Continue {
                model_id: "fast-qualified".to_owned(),
                observation_receipt_sha256: H3.to_owned(),
            }
        );
    }

    #[test]
    fn residual_breach_escalates_to_next_admissible_model() {
        let data = input();
        let receipt = evaluate(&data).expect("input");
        let observation = RunObservation {
            input_sha256: receipt.input_sha256.clone(),
            model_id: "fast-qualified".to_owned(),
            model_passport_sha256: H1.to_owned(),
            observation_receipt_sha256: H3.to_owned(),
            observed_residual: 0.5,
            observed_uncertainty: 0.05,
            validity_inside_envelope: true,
            hard_constraints: HardConstraintDisposition::WithinBounds,
        };
        assert_eq!(
            monitor(&data, &receipt, &observation).expect("monitor"),
            MonitoringDecision::Escalate {
                from_model_id: "fast-qualified".to_owned(),
                to_model_id: "slow-qualified".to_owned(),
                observation_receipt_sha256: H3.to_owned(),
                next_input_sha256: evaluate(&PolicyInputV1 {
                    candidates: vec![candidate("slow-qualified", 900, 2_000)],
                    ..data.clone()
                })
                .expect("remaining candidate")
                .input_sha256,
            }
        );
    }

    #[test]
    fn escalation_abstains_if_no_next_candidate_is_admissible() {
        let mut data = input();
        data.candidates.truncate(1);
        let receipt = evaluate(&data).expect("input");
        let observation = RunObservation {
            input_sha256: receipt.input_sha256.clone(),
            model_id: "slow-qualified".to_owned(),
            model_passport_sha256: H1.to_owned(),
            observation_receipt_sha256: H3.to_owned(),
            observed_residual: 0.5,
            observed_uncertainty: 0.05,
            validity_inside_envelope: true,
            hard_constraints: HardConstraintDisposition::WithinBounds,
        };
        assert_eq!(
            monitor(&data, &receipt, &observation).expect("monitor"),
            MonitoringDecision::Abstain {
                from_model_id: "slow-qualified".to_owned(),
                reason: AbstentionReason::NoCandidates,
                trigger: MonitorTrigger::ResidualExceeded,
                observation_receipt_sha256: H3.to_owned(),
            }
        );
    }

    #[test]
    fn observed_hard_constraint_violation_abstains_instead_of_switching_solvers() {
        let data = input();
        let receipt = evaluate(&data).expect("input");
        let observation = RunObservation {
            input_sha256: receipt.input_sha256.clone(),
            model_id: "fast-qualified".to_owned(),
            model_passport_sha256: H1.to_owned(),
            observation_receipt_sha256: H3.to_owned(),
            observed_residual: 0.01,
            observed_uncertainty: 0.01,
            validity_inside_envelope: true,
            hard_constraints: HardConstraintDisposition::Violated,
        };
        assert_eq!(
            monitor(&data, &receipt, &observation).expect("monitor"),
            MonitoringDecision::Abstain {
                from_model_id: "fast-qualified".to_owned(),
                reason: AbstentionReason::ObservedHardConstraintViolation,
                trigger: MonitorTrigger::HardConstraintViolation,
                observation_receipt_sha256: H3.to_owned(),
            }
        );
    }

    #[test]
    fn unknown_hard_constraint_abstains_instead_of_switching_solvers() {
        let data = input();
        let receipt = evaluate(&data).expect("input");
        let observation = RunObservation {
            input_sha256: receipt.input_sha256.clone(),
            model_id: "fast-qualified".to_owned(),
            model_passport_sha256: H1.to_owned(),
            observation_receipt_sha256: H3.to_owned(),
            observed_residual: 0.01,
            observed_uncertainty: 0.01,
            validity_inside_envelope: true,
            hard_constraints: HardConstraintDisposition::Unknown,
        };
        assert_eq!(
            monitor(&data, &receipt, &observation).expect("monitor"),
            MonitoringDecision::Abstain {
                from_model_id: "fast-qualified".to_owned(),
                reason: AbstentionReason::ObservedHardConstraintUnknown,
                trigger: MonitorTrigger::HardConstraintUnknown,
                observation_receipt_sha256: H3.to_owned(),
            }
        );
    }

    #[test]
    fn validity_envelope_exit_requires_fresh_state_bound_admission() {
        let data = input();
        let receipt = evaluate(&data).expect("input");
        let observation = RunObservation {
            input_sha256: receipt.input_sha256.clone(),
            model_id: "fast-qualified".to_owned(),
            model_passport_sha256: H1.to_owned(),
            observation_receipt_sha256: H3.to_owned(),
            observed_residual: 0.01,
            observed_uncertainty: 0.01,
            validity_inside_envelope: false,
            hard_constraints: HardConstraintDisposition::WithinBounds,
        };
        assert_eq!(
            monitor(&data, &receipt, &observation).expect("monitor"),
            MonitoringDecision::Abstain {
                from_model_id: "fast-qualified".to_owned(),
                reason: AbstentionReason::ObservedValidityEnvelopeExit,
                trigger: MonitorTrigger::ValidityEnvelopeExited,
                observation_receipt_sha256: H3.to_owned(),
            }
        );
    }

    #[test]
    fn exceeding_model_residual_bound_escalates_even_below_request_limit() {
        let data = input();
        let receipt = evaluate(&data).expect("input");
        let observation = RunObservation {
            input_sha256: receipt.input_sha256.clone(),
            model_id: "fast-qualified".to_owned(),
            model_passport_sha256: H1.to_owned(),
            observation_receipt_sha256: H3.to_owned(),
            observed_residual: 0.05,
            observed_uncertainty: 0.01,
            validity_inside_envelope: true,
            hard_constraints: HardConstraintDisposition::WithinBounds,
        };
        // The global request permits 0.1, but the selected model declared 0.01.
        assert!(observation.observed_residual < data.requirement.max_residual_upper_bound);
        assert_eq!(
            monitor(&data, &receipt, &observation).expect("monitor"),
            MonitoringDecision::Escalate {
                from_model_id: "fast-qualified".to_owned(),
                to_model_id: "slow-qualified".to_owned(),
                observation_receipt_sha256: H3.to_owned(),
                next_input_sha256: evaluate(&PolicyInputV1 {
                    candidates: vec![candidate("slow-qualified", 900, 2_000)],
                    ..data.clone()
                })
                .expect("remaining candidate")
                .input_sha256,
            }
        );
    }

    #[test]
    fn observation_must_bind_exact_input_and_model_passport() {
        let data = input();
        let receipt = evaluate(&data).expect("input");
        let mut observation = RunObservation {
            input_sha256: receipt.input_sha256.clone(),
            model_id: "fast-qualified".to_owned(),
            model_passport_sha256: H1.to_owned(),
            observation_receipt_sha256: H3.to_owned(),
            observed_residual: 0.005,
            observed_uncertainty: 0.01,
            validity_inside_envelope: true,
            hard_constraints: HardConstraintDisposition::WithinBounds,
        };
        observation.input_sha256 = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd".to_owned();
        assert_eq!(
            monitor(&data, &receipt, &observation).unwrap_err(),
            PolicyError::ObservationInputMismatch
        );

        observation.input_sha256 = receipt.input_sha256.clone();
        observation.model_passport_sha256 = H2.to_owned();
        assert_eq!(
            monitor(&data, &receipt, &observation).unwrap_err(),
            PolicyError::ObservationPassportMismatch
        );
    }

    #[test]
    fn mutated_receipt_is_rejected_even_when_input_digest_matches() {
        let data = input();
        let mut receipt = evaluate(&data).expect("input");
        receipt.decision = PolicyDecision::Selected {
            model_id: "slow-qualified".to_owned(),
            model_passport_sha256: H1.to_owned(),
        };
        let observation = RunObservation {
            input_sha256: receipt.input_sha256.clone(),
            model_id: "slow-qualified".to_owned(),
            model_passport_sha256: H1.to_owned(),
            observation_receipt_sha256: H3.to_owned(),
            observed_residual: 0.005,
            observed_uncertainty: 0.01,
            validity_inside_envelope: true,
            hard_constraints: HardConstraintDisposition::WithinBounds,
        };
        assert_eq!(
            monitor(&data, &receipt, &observation).unwrap_err(),
            PolicyError::ReceiptMismatch
        );
    }

    #[test]
    fn invalid_identity_fails_closed() {
        let mut data = input();
        data.candidates[0].model_passport_sha256 = "not-a-digest".to_owned();
        assert_eq!(
            evaluate(&data).unwrap_err(),
            PolicyError::InvalidSha256("model_passport_sha256")
        );
    }

    #[test]
    fn duplicate_candidate_identity_is_rejected() {
        let mut data = input();
        data.candidates[1].model_id = data.candidates[0].model_id.clone();
        assert!(matches!(
            evaluate(&data),
            Err(PolicyError::DuplicateModelId(_))
        ));
    }

    #[test]
    fn non_finite_observation_is_rejected() {
        let data = input();
        let receipt = evaluate(&data).expect("input");
        let observation = RunObservation {
            input_sha256: receipt.input_sha256.clone(),
            model_id: "fast-qualified".to_owned(),
            model_passport_sha256: H1.to_owned(),
            observation_receipt_sha256: H3.to_owned(),
            observed_residual: f64::NAN,
            observed_uncertainty: 0.01,
            validity_inside_envelope: true,
            hard_constraints: HardConstraintDisposition::WithinBounds,
        };
        assert_eq!(
            monitor(&data, &receipt, &observation).unwrap_err(),
            PolicyError::NonFiniteOrNegative("observed_residual")
        );
    }
}
