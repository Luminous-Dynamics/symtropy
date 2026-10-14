// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Three-valued functional engineering constraints over externally supplied evidence.
//!
//! This layer evaluates design requirements without becoming a physics oracle,
//! commissioning authority, or civil permission system. `Satisfied` means the
//! supplied engineering evidence closes the stated constraint; `Unknown`
//! remains distinct from both pass and failure.

use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};
use symtropy_game_state::StableId;

use crate::{MeasurementInterval, ObservationState};

macro_rules! stable_id_type {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(StableId);

        impl $name {
            pub const fn new(id: StableId) -> Self {
                Self(id)
            }

            pub const fn stable_id(&self) -> &StableId {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

stable_id_type!(FunctionalDesignId);
stable_id_type!(DesignRoleId);
stable_id_type!(FunctionalConstraintId);
stable_id_type!(EngineeringFactId);

/// Broad subject class used by functional designs without coupling the
/// evaluator to ECS entities or a particular solver implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FunctionalSubjectKind {
    Workpiece,
    Interface,
    Joint,
    Assembly,
    Process,
}

/// Stable subject supplied by another domain authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionalSubject {
    pub kind: FunctionalSubjectKind,
    pub id: StableId,
}

/// Revisioned evidence supporting one engineering fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineeringEvidenceRef {
    pub authority_id: StableId,
    pub evidence_id: StableId,
    pub revision: u64,
    pub digest: String,
}

impl EngineeringEvidenceRef {
    pub fn new(
        authority_id: StableId,
        evidence_id: StableId,
        revision: u64,
        digest: impl Into<String>,
    ) -> Result<Self, ConstraintError> {
        let digest = digest.into();
        if digest.is_empty() || digest.len() > 256 {
            return Err(ConstraintError::InvalidEvidenceDigest(digest));
        }
        Ok(Self {
            authority_id,
            evidence_id,
            revision,
            digest,
        })
    }
}

/// Typed externally established fact. Fabrication stores the evidence reference
/// but does not invent the measurement or state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "fact_kind", rename_all = "snake_case")]
pub enum EngineeringFactValue {
    Measurement { interval: MeasurementInterval },
    Category { value_id: StableId },
    Predicate { state: ObservationState },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineeringFact {
    pub id: EngineeringFactId,
    pub subject: FunctionalSubject,
    pub dimension_id: StableId,
    pub value: EngineeringFactValue,
    pub evidence: EngineeringEvidenceRef,
}

/// Validated collection of facts. Multiple distinct facts may cover the same
/// subject/dimension; disagreement is surfaced by evaluation instead of being
/// silently resolved by latest-write-wins.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineeringFactSet {
    facts: Vec<EngineeringFact>,
}

impl EngineeringFactSet {
    pub fn new(facts: Vec<EngineeringFact>) -> Result<Self, ConstraintError> {
        for (index, fact) in facts.iter().enumerate() {
            if facts[..index].iter().any(|existing| existing.id == fact.id) {
                return Err(ConstraintError::DuplicateFact(fact.id.clone()));
            }
        }
        Ok(Self { facts })
    }

    pub fn facts(&self) -> &[EngineeringFact] {
        &self.facts
    }

    fn matching<'a>(
        &'a self,
        subject: &FunctionalSubject,
        dimension_id: &StableId,
    ) -> Vec<&'a EngineeringFact> {
        self.facts
            .iter()
            .filter(|fact| &fact.subject == subject && &fact.dimension_id == dimension_id)
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignRole {
    pub id: DesignRoleId,
    pub subject_kind: FunctionalSubjectKind,
}

/// One checkable requirement. No constraint carries an importance weight; a
/// design either exposes independent requirements or explicitly models an
/// alternative design branch at a higher layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionalConstraint {
    pub id: FunctionalConstraintId,
    pub role_id: DesignRoleId,
    pub dimension_id: StableId,
    pub predicate: ConstraintPredicate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "predicate", rename_all = "snake_case")]
pub enum ConstraintPredicate {
    MeasurementWithin { lower: i64, upper: i64 },
    CategoryOneOf { accepted: Vec<StableId> },
    PredicateIs { required: ObservationState },
}

impl ConstraintPredicate {
    fn validate(&self) -> Result<(), ConstraintError> {
        match self {
            Self::MeasurementWithin { lower, upper } if lower > upper => {
                Err(ConstraintError::InvalidConstraintInterval {
                    lower: *lower,
                    upper: *upper,
                })
            }
            Self::CategoryOneOf { accepted } if accepted.is_empty() => {
                Err(ConstraintError::EmptyAcceptedCategories)
            }
            Self::CategoryOneOf { accepted } => {
                for (index, value) in accepted.iter().enumerate() {
                    if accepted[..index].contains(value) {
                        return Err(ConstraintError::DuplicateAcceptedCategory(value.clone()));
                    }
                }
                Ok(())
            }
            Self::PredicateIs {
                required: ObservationState::Unknown,
            } => Err(ConstraintError::UnknownCannotBeRequired),
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionalDesign {
    pub id: FunctionalDesignId,
    pub revision: u64,
    pub roles: Vec<DesignRole>,
    pub constraints: Vec<FunctionalConstraint>,
}

impl FunctionalDesign {
    pub fn new(
        id: FunctionalDesignId,
        revision: u64,
        roles: Vec<DesignRole>,
        constraints: Vec<FunctionalConstraint>,
    ) -> Result<Self, ConstraintError> {
        if roles.is_empty() {
            return Err(ConstraintError::RoleRequired);
        }
        if constraints.is_empty() {
            return Err(ConstraintError::ConstraintRequired);
        }
        for (index, role) in roles.iter().enumerate() {
            if roles[..index].iter().any(|existing| existing.id == role.id) {
                return Err(ConstraintError::DuplicateRole(role.id.clone()));
            }
        }
        for (index, constraint) in constraints.iter().enumerate() {
            if constraints[..index]
                .iter()
                .any(|existing| existing.id == constraint.id)
            {
                return Err(ConstraintError::DuplicateConstraint(constraint.id.clone()));
            }
            if !roles.iter().any(|role| role.id == constraint.role_id) {
                return Err(ConstraintError::UnknownConstraintRole {
                    constraint_id: constraint.id.clone(),
                    role_id: constraint.role_id.clone(),
                });
            }
            constraint.predicate.validate()?;
        }
        Ok(Self {
            id,
            revision,
            roles,
            constraints,
        })
    }

    pub fn role(&self, role_id: &DesignRoleId) -> Option<&DesignRole> {
        self.roles.iter().find(|role| &role.id == role_id)
    }

    /// Evaluates supplied evidence only. `all_satisfied` is engineering evidence
    /// closure, not Device Bus initialization, legal permission, or commissioning.
    pub fn evaluate(
        &self,
        binding: &DesignBinding,
        facts: &EngineeringFactSet,
    ) -> FunctionalEvaluation {
        let evaluations = self
            .constraints
            .iter()
            .map(|constraint| self.evaluate_constraint(constraint, binding, facts))
            .collect();
        FunctionalEvaluation {
            design_id: self.id.clone(),
            design_revision: self.revision,
            evaluations,
        }
    }

    fn evaluate_constraint(
        &self,
        constraint: &FunctionalConstraint,
        binding: &DesignBinding,
        facts: &EngineeringFactSet,
    ) -> ConstraintEvaluation {
        let Some(role) = self.role(&constraint.role_id) else {
            return ConstraintEvaluation::unknown(
                constraint.id.clone(),
                ConstraintReason::MissingRoleDefinition,
                Vec::new(),
            );
        };
        let Some(subject) = binding.subject(&constraint.role_id) else {
            return ConstraintEvaluation::unknown(
                constraint.id.clone(),
                ConstraintReason::MissingBinding,
                Vec::new(),
            );
        };
        if subject.kind != role.subject_kind {
            return ConstraintEvaluation {
                constraint_id: constraint.id.clone(),
                outcome: ConstraintOutcome::Unsatisfied,
                reason: ConstraintReason::WrongSubjectKind {
                    required: role.subject_kind,
                    provided: subject.kind,
                },
                supporting_facts: Vec::new(),
            };
        }

        let matching = facts.matching(subject, &constraint.dimension_id);
        if matching.is_empty() {
            return ConstraintEvaluation::unknown(
                constraint.id.clone(),
                ConstraintReason::MissingEvidence,
                Vec::new(),
            );
        }

        let supporting_facts = matching.iter().map(|fact| fact.id.clone()).collect::<Vec<_>>();
        let individual = matching
            .iter()
            .map(|fact| evaluate_fact(&constraint.predicate, &fact.value))
            .collect::<Vec<_>>();

        if individual.iter().all(|result| result.outcome == ConstraintOutcome::Satisfied) {
            return ConstraintEvaluation {
                constraint_id: constraint.id.clone(),
                outcome: ConstraintOutcome::Satisfied,
                reason: ConstraintReason::EvidenceSupportsConstraint,
                supporting_facts,
            };
        }
        if individual
            .iter()
            .all(|result| result.outcome == ConstraintOutcome::Unsatisfied)
        {
            return ConstraintEvaluation {
                constraint_id: constraint.id.clone(),
                outcome: ConstraintOutcome::Unsatisfied,
                reason: common_reason(&individual).unwrap_or(ConstraintReason::AmbiguousEvidence),
                supporting_facts,
            };
        }
        if individual.len() == 1 && individual[0].outcome == ConstraintOutcome::Unknown {
            return ConstraintEvaluation::unknown(
                constraint.id.clone(),
                individual[0].reason.clone(),
                supporting_facts,
            );
        }

        ConstraintEvaluation::unknown(
            constraint.id.clone(),
            ConstraintReason::AmbiguousEvidence,
            supporting_facts,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleBinding {
    pub role_id: DesignRoleId,
    pub subject: FunctionalSubject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignBinding {
    assignments: Vec<RoleBinding>,
}

impl DesignBinding {
    pub fn new(assignments: Vec<RoleBinding>) -> Result<Self, ConstraintError> {
        for (index, assignment) in assignments.iter().enumerate() {
            if assignments[..index]
                .iter()
                .any(|existing| existing.role_id == assignment.role_id)
            {
                return Err(ConstraintError::DuplicateRoleBinding(
                    assignment.role_id.clone(),
                ));
            }
        }
        Ok(Self { assignments })
    }

    pub fn assignments(&self) -> &[RoleBinding] {
        &self.assignments
    }

    pub fn subject(&self, role_id: &DesignRoleId) -> Option<&FunctionalSubject> {
        self.assignments
            .iter()
            .find(|assignment| &assignment.role_id == role_id)
            .map(|assignment| &assignment.subject)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintOutcome {
    Satisfied,
    Unsatisfied,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum ConstraintReason {
    EvidenceSupportsConstraint,
    MissingRoleDefinition,
    MissingBinding,
    MissingEvidence,
    WrongSubjectKind {
        required: FunctionalSubjectKind,
        provided: FunctionalSubjectKind,
    },
    FactTypeMismatch,
    MeasurementOutsideLimit,
    MeasurementStraddlesLimit,
    CategoryRejected,
    PredicateRejected,
    PredicateUnknown,
    AmbiguousEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConstraintEvaluation {
    pub constraint_id: FunctionalConstraintId,
    pub outcome: ConstraintOutcome,
    pub reason: ConstraintReason,
    pub supporting_facts: Vec<EngineeringFactId>,
}

impl ConstraintEvaluation {
    fn unknown(
        constraint_id: FunctionalConstraintId,
        reason: ConstraintReason,
        supporting_facts: Vec<EngineeringFactId>,
    ) -> Self {
        Self {
            constraint_id,
            outcome: ConstraintOutcome::Unknown,
            reason,
            supporting_facts,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionalEvaluation {
    pub design_id: FunctionalDesignId,
    pub design_revision: u64,
    pub evaluations: Vec<ConstraintEvaluation>,
}

impl FunctionalEvaluation {
    /// True only when every constraint has affirmative evidence. This does not
    /// itself authorize or commission the resulting object.
    pub fn all_satisfied(&self) -> bool {
        !self.evaluations.is_empty()
            && self
                .evaluations
                .iter()
                .all(|evaluation| evaluation.outcome == ConstraintOutcome::Satisfied)
    }
}

#[derive(Debug, Clone)]
struct SingleFactEvaluation {
    outcome: ConstraintOutcome,
    reason: ConstraintReason,
}

fn common_reason(results: &[SingleFactEvaluation]) -> Option<ConstraintReason> {
    let first = results.first()?.reason.clone();
    results
        .iter()
        .all(|result| result.reason == first)
        .then_some(first)
}

fn evaluate_fact(
    predicate: &ConstraintPredicate,
    value: &EngineeringFactValue,
) -> SingleFactEvaluation {
    match (predicate, value) {
        (
            ConstraintPredicate::MeasurementWithin { lower, upper },
            EngineeringFactValue::Measurement { interval },
        ) => {
            if interval.lower >= *lower && interval.upper <= *upper {
                SingleFactEvaluation {
                    outcome: ConstraintOutcome::Satisfied,
                    reason: ConstraintReason::EvidenceSupportsConstraint,
                }
            } else if interval.upper < *lower || interval.lower > *upper {
                SingleFactEvaluation {
                    outcome: ConstraintOutcome::Unsatisfied,
                    reason: ConstraintReason::MeasurementOutsideLimit,
                }
            } else {
                SingleFactEvaluation {
                    outcome: ConstraintOutcome::Unknown,
                    reason: ConstraintReason::MeasurementStraddlesLimit,
                }
            }
        }
        (
            ConstraintPredicate::CategoryOneOf { accepted },
            EngineeringFactValue::Category { value_id },
        ) => SingleFactEvaluation {
            outcome: if accepted.contains(value_id) {
                ConstraintOutcome::Satisfied
            } else {
                ConstraintOutcome::Unsatisfied
            },
            reason: if accepted.contains(value_id) {
                ConstraintReason::EvidenceSupportsConstraint
            } else {
                ConstraintReason::CategoryRejected
            },
        },
        (
            ConstraintPredicate::PredicateIs { required },
            EngineeringFactValue::Predicate { state },
        ) => {
            if *state == ObservationState::Unknown {
                SingleFactEvaluation {
                    outcome: ConstraintOutcome::Unknown,
                    reason: ConstraintReason::PredicateUnknown,
                }
            } else if state == required {
                SingleFactEvaluation {
                    outcome: ConstraintOutcome::Satisfied,
                    reason: ConstraintReason::EvidenceSupportsConstraint,
                }
            } else {
                SingleFactEvaluation {
                    outcome: ConstraintOutcome::Unsatisfied,
                    reason: ConstraintReason::PredicateRejected,
                }
            }
        }
        _ => SingleFactEvaluation {
            outcome: ConstraintOutcome::Unknown,
            reason: ConstraintReason::FactTypeMismatch,
        },
    }
}

#[derive(Debug)]
pub enum ConstraintError {
    InvalidEvidenceDigest(String),
    DuplicateFact(EngineeringFactId),
    RoleRequired,
    ConstraintRequired,
    DuplicateRole(DesignRoleId),
    DuplicateConstraint(FunctionalConstraintId),
    UnknownConstraintRole {
        constraint_id: FunctionalConstraintId,
        role_id: DesignRoleId,
    },
    InvalidConstraintInterval { lower: i64, upper: i64 },
    EmptyAcceptedCategories,
    DuplicateAcceptedCategory(StableId),
    UnknownCannotBeRequired,
    DuplicateRoleBinding(DesignRoleId),
}

impl fmt::Display for ConstraintError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEvidenceDigest(digest) => write!(
                formatter,
                "engineering evidence digest must contain 1..=256 bytes, got {}",
                digest.len()
            ),
            Self::DuplicateFact(id) => write!(formatter, "engineering fact {id} is duplicated"),
            Self::RoleRequired => write!(formatter, "functional design requires at least one role"),
            Self::ConstraintRequired => {
                write!(formatter, "functional design requires at least one constraint")
            }
            Self::DuplicateRole(id) => write!(formatter, "functional design role {id} is duplicated"),
            Self::DuplicateConstraint(id) => {
                write!(formatter, "functional constraint {id} is duplicated")
            }
            Self::UnknownConstraintRole {
                constraint_id,
                role_id,
            } => write!(
                formatter,
                "functional constraint {constraint_id} references unknown role {role_id}"
            ),
            Self::InvalidConstraintInterval { lower, upper } => write!(
                formatter,
                "functional constraint interval is invalid: {lower}..{upper}"
            ),
            Self::EmptyAcceptedCategories => {
                write!(formatter, "category constraint requires at least one accepted value")
            }
            Self::DuplicateAcceptedCategory(value) => {
                write!(formatter, "category constraint repeats accepted value {value}")
            }
            Self::UnknownCannotBeRequired => {
                write!(formatter, "unknown cannot be a required predicate state")
            }
            Self::DuplicateRoleBinding(role) => {
                write!(formatter, "design role {role} is bound more than once")
            }
        }
    }
}

impl Error for ConstraintError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn evidence(name: &str) -> EngineeringEvidenceRef {
        EngineeringEvidenceRef::new(
            id("authority:engineering-test"),
            id(name),
            1,
            format!("digest:{name}"),
        )
        .unwrap()
    }

    fn subject() -> FunctionalSubject {
        FunctionalSubject {
            kind: FunctionalSubjectKind::Assembly,
            id: id("assembly:patch-conduit:1"),
        }
    }

    fn design() -> FunctionalDesign {
        FunctionalDesign::new(
            FunctionalDesignId::new(id("functional-design:patch-conduit")),
            2,
            vec![DesignRole {
                id: DesignRoleId::new(id("role:sealed-conduit")),
                subject_kind: FunctionalSubjectKind::Assembly,
            }],
            vec![FunctionalConstraint {
                id: FunctionalConstraintId::new(id("constraint:pressure-loss")),
                role_id: DesignRoleId::new(id("role:sealed-conduit")),
                dimension_id: id("engineering:pressure-loss-pa"),
                predicate: ConstraintPredicate::MeasurementWithin {
                    lower: 0,
                    upper: 500,
                },
            }],
        )
        .unwrap()
    }

    fn binding() -> DesignBinding {
        DesignBinding::new(vec![RoleBinding {
            role_id: DesignRoleId::new(id("role:sealed-conduit")),
            subject: subject(),
        }])
        .unwrap()
    }

    fn fact(interval: MeasurementInterval, suffix: &str) -> EngineeringFact {
        EngineeringFact {
            id: EngineeringFactId::new(id(&format!("fact:pressure-loss:{suffix}"))),
            subject: subject(),
            dimension_id: id("engineering:pressure-loss-pa"),
            value: EngineeringFactValue::Measurement { interval },
            evidence: evidence(&format!("evidence:pressure-loss:{suffix}")),
        }
    }

    #[test]
    fn empty_designs_are_rejected_before_evaluation() {
        let no_roles = FunctionalDesign::new(
            FunctionalDesignId::new(id("functional-design:empty")),
            1,
            Vec::new(),
            Vec::new(),
        );
        assert!(matches!(no_roles, Err(ConstraintError::RoleRequired)));

        let no_constraints = FunctionalDesign::new(
            FunctionalDesignId::new(id("functional-design:no-constraints")),
            1,
            vec![DesignRole {
                id: DesignRoleId::new(id("role:any")),
                subject_kind: FunctionalSubjectKind::Assembly,
            }],
            Vec::new(),
        );
        assert!(matches!(
            no_constraints,
            Err(ConstraintError::ConstraintRequired)
        ));
    }

    #[test]
    fn uncertain_measurement_straddling_limit_remains_unknown_with_specific_reason() {
        let facts = EngineeringFactSet::new(vec![fact(
            MeasurementInterval::new(480, 520, 10).unwrap(),
            "straddle",
        )])
        .unwrap();
        let result = design().evaluate(&binding(), &facts);
        assert_eq!(result.evaluations[0].outcome, ConstraintOutcome::Unknown);
        assert_eq!(
            result.evaluations[0].reason,
            ConstraintReason::MeasurementStraddlesLimit
        );
        assert!(!result.all_satisfied());
    }

    #[test]
    fn missing_evidence_is_unknown_not_failure() {
        let result = design().evaluate(
            &binding(),
            &EngineeringFactSet::new(Vec::new()).unwrap(),
        );
        assert_eq!(result.evaluations[0].outcome, ConstraintOutcome::Unknown);
        assert_eq!(result.evaluations[0].reason, ConstraintReason::MissingEvidence);
    }

    #[test]
    fn known_outside_measurement_is_unsatisfied() {
        let facts = EngineeringFactSet::new(vec![fact(
            MeasurementInterval::new(700, 730, 5).unwrap(),
            "fail",
        )])
        .unwrap();
        let result = design().evaluate(&binding(), &facts);
        assert_eq!(result.evaluations[0].outcome, ConstraintOutcome::Unsatisfied);
        assert_eq!(
            result.evaluations[0].reason,
            ConstraintReason::MeasurementOutsideLimit
        );
    }

    #[test]
    fn conflicting_independent_evidence_is_unknown() {
        let facts = EngineeringFactSet::new(vec![
            fact(MeasurementInterval::new(100, 120, 5).unwrap(), "pass"),
            fact(MeasurementInterval::new(700, 720, 5).unwrap(), "fail"),
        ])
        .unwrap();
        let result = design().evaluate(&binding(), &facts);
        assert_eq!(result.evaluations[0].outcome, ConstraintOutcome::Unknown);
        assert_eq!(
            result.evaluations[0].reason,
            ConstraintReason::AmbiguousEvidence
        );
    }

    #[test]
    fn all_satisfied_means_evidence_closure_not_authorization() {
        let facts = EngineeringFactSet::new(vec![fact(
            MeasurementInterval::new(100, 120, 5).unwrap(),
            "pass",
        )])
        .unwrap();
        let result = design().evaluate(&binding(), &facts);
        assert!(result.all_satisfied());
        let serialized = serde_json::to_value(result).unwrap();
        assert!(serialized.get("authorized").is_none());
        assert!(serialized.get("commissioned").is_none());
        assert!(serialized.get("score").is_none());
    }
}
