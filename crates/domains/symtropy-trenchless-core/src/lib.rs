//! Evidence-qualified screening for pipeline construction and renewal methods.
//!
//! This crate evaluates declared constraints. It does not calculate ground
//! response, hydraulic capacity, pipe stress, jacking force, settlement,
//! machine performance, lifecycle cost, safety, or construction authorization.
//! Those inputs must come from separately qualified owners and retain their
//! source references and evidence classes.
//!
//! A method is eligible only when every supplied hard constraint is known and
//! satisfied. A known violation makes it ineligible. Missing, stale, conflicting,
//! out-of-domain, or not-applicable evidence never creates a pass.
//!
//! No method ranking is produced. Economic comparison belongs downstream and
//! must consume independently accepted functional output and matched lifecycle
//! cost boundaries.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fmt;

/// A method family, not a claim that every machine in the family has the same
/// operating envelope.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum MethodKind {
    OpenCutReplacement,
    CippStructuralLining,
    CloseFitLining,
    Sliplining,
    PipeBursting,
    HorizontalDirectionalDrilling,
    AugerBoring,
    SlurryMicrotunnelling,
    PipeRamming,
    DirectPipe,
}

/// Describes the origin of a numeric fact. It is not an authentication result.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum EvidenceClass {
    Measured,
    Derived,
    Modelled,
    DesignSpecified,
    VendorDeclared,
    OperatorReported,
    Other,
}

/// A typed reason why a required input is not presently known.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum UnknownReason {
    NotCollected,
    NotSurveyed,
    NotProvided,
    Withheld,
    Conflicting,
    Stale,
    Unsupported,
    OutOfDomain,
}

/// Explicit physical units supported by the first screening contract.
///
/// The evaluator rejects mixed units instead of guessing or silently
/// normalizing them. Extend this enum deliberately when a new quantity is
/// introduced.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Unit {
    Millimetre,
    Metre,
    Kilonewton,
    Megapascal,
    CubicMetrePerSecond,
    KilowattHourPerMetre,
    Degree,
    Dimensionless,
}

/// A known numeric fact with a source reference and evidence classification.
///
/// The source_ref is a locator supplied by the caller. This crate does not
/// verify signatures, retrieve the source, or authenticate the issuing organization.
#[derive(Clone, Debug, PartialEq)]
pub struct NumericFact {
    pub value: f64,
    pub unit: Unit,
    pub source_ref: String,
    pub evidence_class: EvidenceClass,
}

/// Evidence state for one side of a numeric constraint.
#[derive(Clone, Debug, PartialEq)]
pub enum NumericEvidence {
    Known(NumericFact),
    Unknown(UnknownReason),
    /// A required comparison was explicitly declared not applicable.
    ///
    /// This is retained as unresolved rather than treated as a pass. A caller
    /// should normally omit a genuinely irrelevant constraint from the
    /// method-specific constraint set.
    NotApplicable(String),
}

/// The relationship that a method prediction/capability must satisfy.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ConstraintRelation {
    /// The candidate's available capability must be >= the requirement.
    AvailableAtLeastRequired,
    /// The candidate's predicted impact/consumption must be <= the allowed limit.
    PredictedAtMostAllowed,
}

/// One evidence-bearing, method-specific hard constraint.
///
/// Examples:
/// - supported maximum drive length >= proposed drive length;
/// - predicted settlement <= project settlement limit;
/// - available jacking capacity >= qualified required jacking load.
///
/// The semantics of candidate and requirement are fixed by relation.
#[derive(Clone, Debug, PartialEq)]
pub struct NumericConstraint {
    pub id: String,
    pub relation: ConstraintRelation,
    pub candidate: NumericEvidence,
    pub requirement: NumericEvidence,
}

/// One candidate method/configuration and its required hard constraints.
#[derive(Clone, Debug, PartialEq)]
pub struct MethodCandidate {
    /// Stable identity for the exact method + equipment + tool + configuration.
    pub method_id: String,
    pub method: MethodKind,
    pub constraints: Vec<NumericConstraint>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvidenceGap {
    Unknown(UnknownReason),
    NotApplicable(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConstraintOutcome {
    Pass,
    Fail,
    Unresolved {
        candidate: Option<EvidenceGap>,
        requirement: Option<EvidenceGap>,
    },
}

/// Result for a single constraint. Values and source references are copied
/// into the result so an exported report can show exactly what was compared.
#[derive(Clone, Debug, PartialEq)]
pub struct ConstraintResult {
    pub id: String,
    pub relation: ConstraintRelation,
    pub unit: Option<Unit>,
    pub candidate_value: Option<f64>,
    pub requirement_value: Option<f64>,
    pub candidate_source_ref: Option<String>,
    pub requirement_source_ref: Option<String>,
    pub outcome: ConstraintOutcome,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Eligibility {
    Eligible,
    Ineligible,
    Indeterminate,
}

/// Assessment for one exact method candidate.
#[derive(Clone, Debug, PartialEq)]
pub struct MethodAssessment {
    pub method_id: String,
    pub method: MethodKind,
    pub eligibility: Eligibility,
    /// Sorted by constraint ID for deterministic serialization and review.
    pub constraints: Vec<ConstraintResult>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvaluationError {
    EmptyPortfolio,
    EmptyMethodId,
    DuplicateMethodId(String),
    NoConstraints(String),
    EmptyConstraintId(String),
    DuplicateConstraintId { method_id: String, constraint_id: String },
    EmptySourceReference { method_id: String, constraint_id: String, side: &'static str },
    EmptyNotApplicableReason { method_id: String, constraint_id: String, side: &'static str },
    NonFiniteValue { method_id: String, constraint_id: String, side: &'static str },
    UnitMismatch { method_id: String, constraint_id: String },
}

impl fmt::Display for EvaluationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPortfolio => write!(f, "portfolio contains no method candidates"),
            Self::EmptyMethodId => write!(f, "method candidate has an empty method ID"),
            Self::DuplicateMethodId(id) => write!(f, "duplicate method ID: {id}"),
            Self::NoConstraints(id) => write!(f, "method candidate {id} has no hard constraints"),
            Self::EmptyConstraintId(method_id) => {
                write!(f, "method candidate {method_id} has an empty constraint ID")
            }
            Self::DuplicateConstraintId { method_id, constraint_id } => {
                write!(f, "method candidate {method_id} repeats constraint {constraint_id}")
            }
            Self::EmptySourceReference { method_id, constraint_id, side } => {
                write!(f, "{method_id}/{constraint_id}: {side} source reference is empty")
            }
            Self::EmptyNotApplicableReason { method_id, constraint_id, side } => {
                write!(f, "{method_id}/{constraint_id}: {side} not-applicable reason is empty")
            }
            Self::NonFiniteValue { method_id, constraint_id, side } => {
                write!(f, "{method_id}/{constraint_id}: {side} numeric value is not finite")
            }
            Self::UnitMismatch { method_id, constraint_id } => {
                write!(f, "{method_id}/{constraint_id}: candidate and requirement units differ")
            }
        }
    }
}

impl std::error::Error for EvaluationError {}

/// Evaluate a portfolio without ranking it.
///
/// Output is sorted by stable method_id, and each candidate's constraints
/// are sorted by constraint ID. A candidate is Eligible only when it has at
/// least one constraint and every constraint is known and satisfied.
///
/// Status precedence is deliberately conservative:
/// - any known failed constraint => Ineligible;
/// - otherwise any unresolved constraint => Indeterminate;
/// - otherwise => Eligible.
pub fn assess_portfolio(
    candidates: &[MethodCandidate],
) -> Result<Vec<MethodAssessment>, EvaluationError> {
    if candidates.is_empty() {
        return Err(EvaluationError::EmptyPortfolio);
    }

    let mut method_ids = BTreeSet::new();
    for candidate in candidates {
        if candidate.method_id.trim().is_empty() {
            return Err(EvaluationError::EmptyMethodId);
        }
        if !method_ids.insert(candidate.method_id.as_str()) {
            return Err(EvaluationError::DuplicateMethodId(candidate.method_id.clone()));
        }
    }

    let mut ordered: Vec<&MethodCandidate> = candidates.iter().collect();
    ordered.sort_by(|a, b| a.method_id.cmp(&b.method_id));

    ordered
        .into_iter()
        .map(assess_candidate)
        .collect()
}

fn assess_candidate(candidate: &MethodCandidate) -> Result<MethodAssessment, EvaluationError> {
    if candidate.constraints.is_empty() {
        return Err(EvaluationError::NoConstraints(candidate.method_id.clone()));
    }

    let mut constraint_ids = BTreeSet::new();
    let mut results = Vec::with_capacity(candidate.constraints.len());

    for constraint in &candidate.constraints {
        if constraint.id.trim().is_empty() {
            return Err(EvaluationError::EmptyConstraintId(candidate.method_id.clone()));
        }
        if !constraint_ids.insert(constraint.id.as_str()) {
            return Err(EvaluationError::DuplicateConstraintId {
                method_id: candidate.method_id.clone(),
                constraint_id: constraint.id.clone(),
            });
        }

        validate_evidence(
            &candidate.method_id,
            &constraint.id,
            "candidate",
            &constraint.candidate,
        )?;
        validate_evidence(
            &candidate.method_id,
            &constraint.id,
            "requirement",
            &constraint.requirement,
        )?;

        results.push(evaluate_constraint(candidate, constraint)?);
    }

    results.sort_by(|a, b| a.id.cmp(&b.id));

    let has_failure = results.iter().any(|r| r.outcome == ConstraintOutcome::Fail);
    let has_unresolved = results
        .iter()
        .any(|r| matches!(r.outcome, ConstraintOutcome::Unresolved { .. }));

    let eligibility = if has_failure {
        Eligibility::Ineligible
    } else if has_unresolved {
        Eligibility::Indeterminate
    } else {
        Eligibility::Eligible
    };

    Ok(MethodAssessment {
        method_id: candidate.method_id.clone(),
        method: candidate.method,
        eligibility,
        constraints: results,
    })
}

fn validate_evidence(
    method_id: &str,
    constraint_id: &str,
    side: &'static str,
    evidence: &NumericEvidence,
) -> Result<(), EvaluationError> {
    match evidence {
        NumericEvidence::Known(fact) => {
            if !fact.value.is_finite() {
                return Err(EvaluationError::NonFiniteValue {
                    method_id: method_id.to_owned(),
                    constraint_id: constraint_id.to_owned(),
                    side,
                });
            }
            if fact.source_ref.trim().is_empty() {
                return Err(EvaluationError::EmptySourceReference {
                    method_id: method_id.to_owned(),
                    constraint_id: constraint_id.to_owned(),
                    side,
                });
            }
        }
        NumericEvidence::Unknown(_) => {}
        NumericEvidence::NotApplicable(reason) => {
            if reason.trim().is_empty() {
                return Err(EvaluationError::EmptyNotApplicableReason {
                    method_id: method_id.to_owned(),
                    constraint_id: constraint_id.to_owned(),
                    side,
                });
            }
        }
    }
    Ok(())
}

fn evidence_gap(evidence: &NumericEvidence) -> Option<EvidenceGap> {
    match evidence {
        NumericEvidence::Known(_) => None,
        NumericEvidence::Unknown(reason) => Some(EvidenceGap::Unknown(*reason)),
        NumericEvidence::NotApplicable(reason) => {
            Some(EvidenceGap::NotApplicable(reason.clone()))
        }
    }
}

fn known_fact(evidence: &NumericEvidence) -> Option<&NumericFact> {
    match evidence {
        NumericEvidence::Known(fact) => Some(fact),
        NumericEvidence::Unknown(_) | NumericEvidence::NotApplicable(_) => None,
    }
}

fn evaluate_constraint(
    candidate: &MethodCandidate,
    constraint: &NumericConstraint,
) -> Result<ConstraintResult, EvaluationError> {
    let candidate_fact = known_fact(&constraint.candidate);
    let requirement_fact = known_fact(&constraint.requirement);

    if let (Some(left), Some(right)) = (candidate_fact, requirement_fact) {
        if left.unit != right.unit {
            return Err(EvaluationError::UnitMismatch {
                method_id: candidate.method_id.clone(),
                constraint_id: constraint.id.clone(),
            });
        }
    }

    let unit = candidate_fact
        .map(|fact| fact.unit)
        .or_else(|| requirement_fact.map(|fact| fact.unit));

    let outcome = match (candidate_fact, requirement_fact) {
        (Some(left), Some(right)) => {
            let satisfied = match constraint.relation {
                ConstraintRelation::AvailableAtLeastRequired => left.value >= right.value,
                ConstraintRelation::PredictedAtMostAllowed => left.value <= right.value,
            };
            if satisfied {
                ConstraintOutcome::Pass
            } else {
                ConstraintOutcome::Fail
            }
        }
        _ => ConstraintOutcome::Unresolved {
            candidate: evidence_gap(&constraint.candidate),
            requirement: evidence_gap(&constraint.requirement),
        },
    };

    Ok(ConstraintResult {
        id: constraint.id.clone(),
        relation: constraint.relation,
        unit,
        candidate_value: candidate_fact.map(|fact| fact.value),
        requirement_value: requirement_fact.map(|fact| fact.value),
        candidate_source_ref: candidate_fact.map(|fact| fact.source_ref.clone()),
        requirement_source_ref: requirement_fact.map(|fact| fact.source_ref.clone()),
        outcome,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fact(value: f64, unit: Unit, source_ref: &str, class: EvidenceClass) -> NumericEvidence {
        NumericEvidence::Known(NumericFact {
            value,
            unit,
            source_ref: source_ref.to_owned(),
            evidence_class: class,
        })
    }

    fn constraint(
        id: &str,
        relation: ConstraintRelation,
        candidate: NumericEvidence,
        requirement: NumericEvidence,
    ) -> NumericConstraint {
        NumericConstraint {
            id: id.to_owned(),
            relation,
            candidate,
            requirement,
        }
    }

    fn candidate(method_id: &str, constraints: Vec<NumericConstraint>) -> MethodCandidate {
        MethodCandidate {
            method_id: method_id.to_owned(),
            method: MethodKind::SlurryMicrotunnelling,
            constraints,
        }
    }

    #[test]
    fn all_known_constraints_satisfied_is_eligible() {
        let input = candidate(
            "mtbm-a-config-7",
            vec![
                constraint(
                    "drive-length",
                    ConstraintRelation::AvailableAtLeastRequired,
                    fact(180.0, Unit::Metre, "machine-envelope:v7", EvidenceClass::VendorDeclared),
                    fact(150.0, Unit::Metre, "design:drive-01", EvidenceClass::DesignSpecified),
                ),
                constraint(
                    "settlement",
                    ConstraintRelation::PredictedAtMostAllowed,
                    fact(3.0, Unit::Millimetre, "model:run-42", EvidenceClass::Modelled),
                    fact(5.0, Unit::Millimetre, "design:settlement-limit", EvidenceClass::DesignSpecified),
                ),
            ],
        );

        let report = assess_portfolio(&[input]).unwrap();
        assert_eq!(report[0].eligibility, Eligibility::Eligible);
        assert_eq!(report[0].constraints.len(), 2);
    }

    #[test]
    fn known_hard_limit_violation_is_ineligible() {
        let input = candidate(
            "hdd-small-rig",
            vec![constraint(
                "product-diameter",
                ConstraintRelation::AvailableAtLeastRequired,
                fact(250.0, Unit::Millimetre, "rig-envelope:v2", EvidenceClass::VendorDeclared),
                fact(315.0, Unit::Millimetre, "design:pipe-315", EvidenceClass::DesignSpecified),
            )],
        );

        let report = assess_portfolio(&[input]).unwrap();
        assert_eq!(report[0].eligibility, Eligibility::Ineligible);
        assert_eq!(report[0].constraints[0].outcome, ConstraintOutcome::Fail);
    }

    #[test]
    fn missing_required_fact_is_indeterminate_not_eligible() {
        let input = candidate(
            "pipe-bursting-config-a",
            vec![constraint(
                "existing-pipe-condition",
                ConstraintRelation::AvailableAtLeastRequired,
                NumericEvidence::Unknown(UnknownReason::NotSurveyed),
                fact(1.0, Unit::Dimensionless, "design:host-pipe-condition-minimum", EvidenceClass::DesignSpecified),
            )],
        );

        let report = assess_portfolio(&[input]).unwrap();
        assert_eq!(report[0].eligibility, Eligibility::Indeterminate);
        assert!(matches!(
            report[0].constraints[0].outcome,
            ConstraintOutcome::Unresolved { .. }
        ));
    }

    #[test]
    fn a_known_failure_is_not_hidden_by_an_unresolved_constraint() {
        let input = candidate(
            "mixed-evidence-option",
            vec![
                constraint(
                    "capacity",
                    ConstraintRelation::AvailableAtLeastRequired,
                    fact(100.0, Unit::Kilonewton, "machine:capacity", EvidenceClass::VendorDeclared),
                    fact(125.0, Unit::Kilonewton, "design:required-jacking-load", EvidenceClass::Modelled),
                ),
                constraint(
                    "utility-clearance",
                    ConstraintRelation::AvailableAtLeastRequired,
                    NumericEvidence::Unknown(UnknownReason::NotSurveyed),
                    fact(1.0, Unit::Dimensionless, "design:clearance-required", EvidenceClass::DesignSpecified),
                ),
            ],
        );

        let report = assess_portfolio(&[input]).unwrap();
        assert_eq!(report[0].eligibility, Eligibility::Ineligible);
        assert_eq!(report[0].constraints[0].outcome, ConstraintOutcome::Fail);
        assert!(matches!(
            report[0].constraints[1].outcome,
            ConstraintOutcome::Unresolved { .. }
        ));
    }

    #[test]
    fn mixed_units_are_rejected() {
        let input = candidate(
            "unit-mismatch",
            vec![constraint(
                "length",
                ConstraintRelation::AvailableAtLeastRequired,
                fact(100.0, Unit::Metre, "machine:envelope", EvidenceClass::VendorDeclared),
                fact(100.0, Unit::Millimetre, "design:length", EvidenceClass::DesignSpecified),
            )],
        );

        assert_eq!(
            assess_portfolio(&[input]).unwrap_err(),
            EvaluationError::UnitMismatch {
                method_id: "unit-mismatch".to_owned(),
                constraint_id: "length".to_owned(),
            }
        );
    }

    #[test]
    fn duplicate_constraint_ids_are_rejected() {
        let duplicate = constraint(
            "same",
            ConstraintRelation::AvailableAtLeastRequired,
            fact(2.0, Unit::Metre, "machine:a", EvidenceClass::VendorDeclared),
            fact(1.0, Unit::Metre, "design:a", EvidenceClass::DesignSpecified),
        );
        let input = candidate("duplicate-constraint", vec![duplicate.clone(), duplicate]);

        assert!(matches!(
            assess_portfolio(&[input]),
            Err(EvaluationError::DuplicateConstraintId { .. })
        ));
    }

    #[test]
    fn non_finite_values_are_rejected() {
        let input = candidate(
            "nan-input",
            vec![constraint(
                "capacity",
                ConstraintRelation::AvailableAtLeastRequired,
                fact(f64::NAN, Unit::Kilonewton, "machine:a", EvidenceClass::VendorDeclared),
                fact(1.0, Unit::Kilonewton, "design:a", EvidenceClass::DesignSpecified),
            )],
        );

        assert!(matches!(
            assess_portfolio(&[input]),
            Err(EvaluationError::NonFiniteValue { .. })
        ));
    }

    #[test]
    fn empty_source_reference_is_rejected() {
        let input = candidate(
            "missing-source",
            vec![constraint(
                "capacity",
                ConstraintRelation::AvailableAtLeastRequired,
                fact(2.0, Unit::Kilonewton, " ", EvidenceClass::VendorDeclared),
                fact(1.0, Unit::Kilonewton, "design:a", EvidenceClass::DesignSpecified),
            )],
        );

        assert!(matches!(
            assess_portfolio(&[input]),
            Err(EvaluationError::EmptySourceReference { .. })
        ));
    }

    #[test]
    fn not_applicable_does_not_create_a_pass() {
        let input = candidate(
            "not-applicable",
            vec![constraint(
                "groundwater-envelope",
                ConstraintRelation::AvailableAtLeastRequired,
                NumericEvidence::NotApplicable("no verified groundwater assessment".to_owned()),
                fact(0.0, Unit::Metre, "design:assumed-head", EvidenceClass::DesignSpecified),
            )],
        );

        let report = assess_portfolio(&[input]).unwrap();
        assert_eq!(report[0].eligibility, Eligibility::Indeterminate);
    }

    #[test]
    fn empty_constraint_set_is_rejected() {
        let input = candidate("no-evidence", vec![]);
        assert_eq!(
            assess_portfolio(&[input]).unwrap_err(),
            EvaluationError::NoConstraints("no-evidence".to_owned())
        );
    }

    #[test]
    fn output_order_is_deterministic_and_does_not_rank_methods() {
        let a = candidate(
            "z-option",
            vec![constraint(
                "z-check",
                ConstraintRelation::AvailableAtLeastRequired,
                fact(2.0, Unit::Metre, "z:cap", EvidenceClass::VendorDeclared),
                fact(1.0, Unit::Metre, "z:req", EvidenceClass::DesignSpecified),
            )],
        );
        let b = candidate(
            "a-option",
            vec![constraint(
                "b-check",
                ConstraintRelation::AvailableAtLeastRequired,
                fact(2.0, Unit::Metre, "a:cap", EvidenceClass::VendorDeclared),
                fact(1.0, Unit::Metre, "a:req", EvidenceClass::DesignSpecified),
            )],
        );

        let report = assess_portfolio(&[a, b]).unwrap();
        assert_eq!(report[0].method_id, "a-option");
        assert_eq!(report[1].method_id, "z-option");
    }

    #[test]
    fn duplicate_method_ids_are_rejected() {
        let make = || candidate(
            "same-id",
            vec![constraint(
                "capacity",
                ConstraintRelation::AvailableAtLeastRequired,
                fact(2.0, Unit::Metre, "machine:a", EvidenceClass::VendorDeclared),
                fact(1.0, Unit::Metre, "design:a", EvidenceClass::DesignSpecified),
            )],
        );

        assert_eq!(
            assess_portfolio(&[make(), make()]).unwrap_err(),
            EvaluationError::DuplicateMethodId("same-id".to_owned())
        );
    }

    #[test]
    fn empty_portfolio_is_rejected() {
        assert_eq!(
            assess_portfolio(&[]).unwrap_err(),
            EvaluationError::EmptyPortfolio
        );
    }
}
