// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Technical commissioning evidence for fabricated assemblies and devices.
//!
//! Commissioning here means technical evidence closure for one exact subject
//! state. It does not grant civic permission, initialize a Device Bus node,
//! register ownership, or make any claim about lawful/authorized operation.

use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, error::Error, fmt};
use symtropy_game_state::StableId;

use crate::{
    ConstraintOutcome, FunctionalConstraintId, FunctionalDesign, FunctionalDesignId,
    FunctionalEvaluation, FunctionalSubject, FunctionalSubjectKind,
};

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

stable_id_type!(CommissioningPlanId);
stable_id_type!(CommissioningRequirementId);

/// Exact artifact state against which commissioning evidence is valid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommissioningSubjectState {
    pub subject: FunctionalSubject,
    pub revision: u64,
    pub digest: String,
}

impl CommissioningSubjectState {
    pub fn new(
        subject: FunctionalSubject,
        revision: u64,
        digest: impl Into<String>,
    ) -> Result<Self, CommissioningError> {
        let digest = digest.into();
        validate_digest(&digest)?;
        Ok(Self {
            subject,
            revision,
            digest,
        })
    }
}

/// Provenance envelope binding a functional evaluation to the exact subject
/// state for which an engineering authority emitted it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundFunctionalEvaluation {
    pub subject_state: CommissioningSubjectState,
    pub authority_id: StableId,
    pub evidence_id: StableId,
    pub evidence_revision: u64,
    pub evidence_digest: String,
    pub evaluation: FunctionalEvaluation,
}

impl BoundFunctionalEvaluation {
    pub fn new(
        subject_state: CommissioningSubjectState,
        authority_id: StableId,
        evidence_id: StableId,
        evidence_revision: u64,
        evidence_digest: impl Into<String>,
        evaluation: FunctionalEvaluation,
    ) -> Result<Self, CommissioningError> {
        let evidence_digest = evidence_digest.into();
        validate_digest(&evidence_digest)?;
        Ok(Self {
            subject_state,
            authority_id,
            evidence_id,
            evidence_revision,
            evidence_digest,
            evaluation,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "requirement_kind", rename_all = "snake_case")]
pub enum CommissioningRequirementKind {
    FunctionalConstraintSatisfied {
        constraint_id: FunctionalConstraintId,
    },
    EvidenceKindPresent {
        evidence_kind: StableId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommissioningRequirement {
    pub id: CommissioningRequirementId,
    pub kind: CommissioningRequirementKind,
}

/// Reusable technical commissioning specification bound to one exact functional
/// design revision and one subject class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommissioningPlan {
    pub id: CommissioningPlanId,
    pub revision: u64,
    pub subject_kind: FunctionalSubjectKind,
    pub design_id: FunctionalDesignId,
    pub design_revision: u64,
    design_constraints: Vec<FunctionalConstraintId>,
    requirements: Vec<CommissioningRequirement>,
}

impl CommissioningPlan {
    pub fn new(
        id: CommissioningPlanId,
        revision: u64,
        subject_kind: FunctionalSubjectKind,
        design: &FunctionalDesign,
        mut requirements: Vec<CommissioningRequirement>,
    ) -> Result<Self, CommissioningError> {
        if requirements.is_empty() {
            return Err(CommissioningError::RequirementRequired);
        }

        let mut design_constraints = design
            .constraints
            .iter()
            .map(|constraint| constraint.id.clone())
            .collect::<Vec<_>>();
        design_constraints.sort();

        requirements.sort_by(|left, right| left.id.cmp(&right.id));
        for pair in requirements.windows(2) {
            if pair[0].id == pair[1].id {
                return Err(CommissioningError::DuplicateRequirement(pair[0].id.clone()));
            }
        }

        for requirement in &requirements {
            if let CommissioningRequirementKind::FunctionalConstraintSatisfied { constraint_id } =
                &requirement.kind
            {
                if !design_constraints.contains(constraint_id) {
                    return Err(CommissioningError::UnknownConstraintRequirement {
                        requirement_id: requirement.id.clone(),
                        constraint_id: constraint_id.clone(),
                    });
                }
            }
        }

        Ok(Self {
            id,
            revision,
            subject_kind,
            design_id: design.id.clone(),
            design_revision: design.revision,
            design_constraints,
            requirements,
        })
    }

    pub fn requirements(&self) -> &[CommissioningRequirement] {
        &self.requirements
    }

    /// Evaluates technical evidence for one exact subject state only.
    pub fn assess(
        &self,
        dossier: &CommissioningDossier,
        functional: &BoundFunctionalEvaluation,
    ) -> Result<CommissioningAssessment, CommissioningError> {
        if dossier.plan_id != self.id || dossier.plan_revision != self.revision {
            return Err(CommissioningError::DossierPlanMismatch {
                expected_id: self.id.clone(),
                expected_revision: self.revision,
                actual_id: dossier.plan_id.clone(),
                actual_revision: dossier.plan_revision,
            });
        }
        if dossier.subject_state.subject.kind != self.subject_kind {
            return Err(CommissioningError::WrongSubjectKind {
                required: self.subject_kind,
                provided: dossier.subject_state.subject.kind,
            });
        }
        if functional.subject_state != dossier.subject_state {
            return Err(CommissioningError::FunctionalSubjectStateMismatch);
        }
        if functional.evaluation.design_id != self.design_id
            || functional.evaluation.design_revision != self.design_revision
        {
            return Err(CommissioningError::FunctionalDesignMismatch {
                expected_id: self.design_id.clone(),
                expected_revision: self.design_revision,
                actual_id: functional.evaluation.design_id.clone(),
                actual_revision: functional.evaluation.design_revision,
            });
        }

        let mut by_constraint = BTreeMap::new();
        for evaluation in &functional.evaluation.evaluations {
            if by_constraint
                .insert(evaluation.constraint_id.clone(), evaluation.outcome)
                .is_some()
            {
                return Err(CommissioningError::DuplicateConstraintEvaluation(
                    evaluation.constraint_id.clone(),
                ));
            }
        }
        for expected in &self.design_constraints {
            if !by_constraint.contains_key(expected) {
                return Err(CommissioningError::MissingConstraintEvaluation(expected.clone()));
            }
        }
        if let Some(unexpected) = by_constraint
            .keys()
            .find(|constraint_id| !self.design_constraints.contains(constraint_id))
        {
            return Err(CommissioningError::UnexpectedConstraintEvaluation(
                unexpected.clone(),
            ));
        }

        let evaluations = self
            .requirements
            .iter()
            .map(|requirement| match &requirement.kind {
                CommissioningRequirementKind::FunctionalConstraintSatisfied { constraint_id } => {
                    evaluate_functional_requirement(requirement, constraint_id, &by_constraint)
                }
                CommissioningRequirementKind::EvidenceKindPresent { evidence_kind } => {
                    evaluate_evidence_requirement(requirement, evidence_kind, dossier)
                }
            })
            .collect();

        Ok(CommissioningAssessment {
            plan_id: self.id.clone(),
            plan_revision: self.revision,
            subject_state: dossier.subject_state.clone(),
            functional_evaluation_identity: CommissioningEvidenceIdentity {
                authority_id: functional.authority_id.clone(),
                evidence_id: functional.evidence_id.clone(),
            },
            evaluations,
        })
    }
}

fn evaluate_functional_requirement(
    requirement: &CommissioningRequirement,
    constraint_id: &FunctionalConstraintId,
    by_constraint: &BTreeMap<FunctionalConstraintId, ConstraintOutcome>,
) -> RequirementEvaluation {
    let outcome = *by_constraint
        .get(constraint_id)
        .expect("commissioning plan validates functional constraint membership");
    match outcome {
        ConstraintOutcome::Satisfied => RequirementEvaluation {
            requirement_id: requirement.id.clone(),
            outcome: CommissioningRequirementOutcome::Satisfied,
            reason: CommissioningReason::FunctionalConstraintSatisfied,
            supporting_evidence: Vec::new(),
        },
        ConstraintOutcome::Unsatisfied => RequirementEvaluation {
            requirement_id: requirement.id.clone(),
            outcome: CommissioningRequirementOutcome::Unsatisfied,
            reason: CommissioningReason::FunctionalConstraintUnsatisfied,
            supporting_evidence: Vec::new(),
        },
        ConstraintOutcome::Unknown => RequirementEvaluation {
            requirement_id: requirement.id.clone(),
            outcome: CommissioningRequirementOutcome::Unknown,
            reason: CommissioningReason::FunctionalConstraintUnknown,
            supporting_evidence: Vec::new(),
        },
    }
}

fn evaluate_evidence_requirement(
    requirement: &CommissioningRequirement,
    evidence_kind: &StableId,
    dossier: &CommissioningDossier,
) -> RequirementEvaluation {
    let supporting_evidence = dossier
        .evidence
        .iter()
        .filter(|evidence| &evidence.evidence_kind == evidence_kind)
        .map(CommissioningEvidenceRef::identity)
        .collect::<Vec<_>>();
    if supporting_evidence.is_empty() {
        RequirementEvaluation {
            requirement_id: requirement.id.clone(),
            outcome: CommissioningRequirementOutcome::Unknown,
            reason: CommissioningReason::RequiredEvidenceMissing,
            supporting_evidence,
        }
    } else {
        RequirementEvaluation {
            requirement_id: requirement.id.clone(),
            outcome: CommissioningRequirementOutcome::Satisfied,
            reason: CommissioningReason::RequiredEvidencePresent,
            supporting_evidence,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommissioningEvidenceIdentity {
    pub authority_id: StableId,
    pub evidence_id: StableId,
}

/// External evidence bound to one exact commissioning subject state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommissioningEvidenceRef {
    pub authority_id: StableId,
    pub evidence_id: StableId,
    pub revision: u64,
    pub digest: String,
    pub evidence_kind: StableId,
    pub subject_state: CommissioningSubjectState,
}

impl CommissioningEvidenceRef {
    pub fn new(
        authority_id: StableId,
        evidence_id: StableId,
        revision: u64,
        digest: impl Into<String>,
        evidence_kind: StableId,
        subject_state: CommissioningSubjectState,
    ) -> Result<Self, CommissioningError> {
        let digest = digest.into();
        validate_digest(&digest)?;
        Ok(Self {
            authority_id,
            evidence_id,
            revision,
            digest,
            evidence_kind,
            subject_state,
        })
    }

    fn identity(&self) -> CommissioningEvidenceIdentity {
        CommissioningEvidenceIdentity {
            authority_id: self.authority_id.clone(),
            evidence_id: self.evidence_id.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommissioningDossier {
    pub plan_id: CommissioningPlanId,
    pub plan_revision: u64,
    pub subject_state: CommissioningSubjectState,
    evidence: Vec<CommissioningEvidenceRef>,
}

impl CommissioningDossier {
    pub fn new(
        plan_id: CommissioningPlanId,
        plan_revision: u64,
        subject_state: CommissioningSubjectState,
        mut evidence: Vec<CommissioningEvidenceRef>,
    ) -> Result<Self, CommissioningError> {
        evidence.sort_by(|left, right| {
            left.authority_id
                .cmp(&right.authority_id)
                .then_with(|| left.evidence_id.cmp(&right.evidence_id))
        });
        for item in &evidence {
            if item.subject_state != subject_state {
                return Err(CommissioningError::EvidenceSubjectStateMismatch {
                    authority_id: item.authority_id.clone(),
                    evidence_id: item.evidence_id.clone(),
                });
            }
        }
        for pair in evidence.windows(2) {
            if pair[0].authority_id == pair[1].authority_id
                && pair[0].evidence_id == pair[1].evidence_id
            {
                return Err(CommissioningError::DuplicateEvidenceIdentity(
                    pair[0].identity(),
                ));
            }
        }
        Ok(Self {
            plan_id,
            plan_revision,
            subject_state,
            evidence,
        })
    }

    pub fn evidence(&self) -> &[CommissioningEvidenceRef] {
        &self.evidence
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommissioningRequirementOutcome {
    Satisfied,
    Unsatisfied,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommissioningReason {
    FunctionalConstraintSatisfied,
    FunctionalConstraintUnsatisfied,
    FunctionalConstraintUnknown,
    RequiredEvidencePresent,
    RequiredEvidenceMissing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequirementEvaluation {
    pub requirement_id: CommissioningRequirementId,
    pub outcome: CommissioningRequirementOutcome,
    pub reason: CommissioningReason,
    pub supporting_evidence: Vec<CommissioningEvidenceIdentity>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommissioningAssessment {
    pub plan_id: CommissioningPlanId,
    pub plan_revision: u64,
    pub subject_state: CommissioningSubjectState,
    pub functional_evaluation_identity: CommissioningEvidenceIdentity,
    pub evaluations: Vec<RequirementEvaluation>,
}

impl CommissioningAssessment {
    pub fn technical_ready(&self) -> bool {
        !self.evaluations.is_empty()
            && self.evaluations.iter().all(|evaluation| {
                evaluation.outcome == CommissioningRequirementOutcome::Satisfied
            })
    }
}

fn validate_digest(digest: &str) -> Result<(), CommissioningError> {
    if digest.is_empty() || digest.len() > 256 {
        return Err(CommissioningError::InvalidEvidenceDigest(digest.to_owned()));
    }
    Ok(())
}

#[derive(Debug)]
pub enum CommissioningError {
    RequirementRequired,
    DuplicateRequirement(CommissioningRequirementId),
    UnknownConstraintRequirement {
        requirement_id: CommissioningRequirementId,
        constraint_id: FunctionalConstraintId,
    },
    InvalidEvidenceDigest(String),
    EvidenceSubjectStateMismatch {
        authority_id: StableId,
        evidence_id: StableId,
    },
    DuplicateEvidenceIdentity(CommissioningEvidenceIdentity),
    DossierPlanMismatch {
        expected_id: CommissioningPlanId,
        expected_revision: u64,
        actual_id: CommissioningPlanId,
        actual_revision: u64,
    },
    WrongSubjectKind {
        required: FunctionalSubjectKind,
        provided: FunctionalSubjectKind,
    },
    FunctionalSubjectStateMismatch,
    FunctionalDesignMismatch {
        expected_id: FunctionalDesignId,
        expected_revision: u64,
        actual_id: FunctionalDesignId,
        actual_revision: u64,
    },
    DuplicateConstraintEvaluation(FunctionalConstraintId),
    MissingConstraintEvaluation(FunctionalConstraintId),
    UnexpectedConstraintEvaluation(FunctionalConstraintId),
}

impl fmt::Display for CommissioningError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RequirementRequired => {
                write!(formatter, "commissioning plan requires at least one requirement")
            }
            Self::DuplicateRequirement(id) => {
                write!(formatter, "commissioning plan repeats requirement {id}")
            }
            Self::UnknownConstraintRequirement {
                requirement_id,
                constraint_id,
            } => write!(
                formatter,
                "commissioning requirement {requirement_id} references unknown constraint {constraint_id}"
            ),
            Self::InvalidEvidenceDigest(digest) => write!(
                formatter,
                "commissioning evidence digest must contain 1..=256 bytes, got {}",
                digest.len()
            ),
            Self::EvidenceSubjectStateMismatch {
                authority_id,
                evidence_id,
            } => write!(
                formatter,
                "commissioning evidence {authority_id}/{evidence_id} targets a different subject state"
            ),
            Self::DuplicateEvidenceIdentity(identity) => write!(
                formatter,
                "commissioning evidence {}/{} is duplicated",
                identity.authority_id, identity.evidence_id
            ),
            Self::DossierPlanMismatch {
                expected_id,
                expected_revision,
                actual_id,
                actual_revision,
            } => write!(
                formatter,
                "commissioning dossier expects plan {expected_id}@{expected_revision}, got {actual_id}@{actual_revision}"
            ),
            Self::WrongSubjectKind { required, provided } => write!(
                formatter,
                "commissioning subject kind mismatch: required {required:?}, provided {provided:?}"
            ),
            Self::FunctionalSubjectStateMismatch => write!(
                formatter,
                "functional evaluation was emitted for a different commissioning subject state"
            ),
            Self::FunctionalDesignMismatch {
                expected_id,
                expected_revision,
                actual_id,
                actual_revision,
            } => write!(
                formatter,
                "commissioning expects design {expected_id}@{expected_revision}, got {actual_id}@{actual_revision}"
            ),
            Self::DuplicateConstraintEvaluation(id) => {
                write!(formatter, "functional evaluation repeats constraint {id}")
            }
            Self::MissingConstraintEvaluation(id) => {
                write!(formatter, "functional evaluation omits required constraint {id}")
            }
            Self::UnexpectedConstraintEvaluation(id) => {
                write!(formatter, "functional evaluation contains unexpected constraint {id}")
            }
        }
    }
}

impl Error for CommissioningError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ConstraintEvaluation, ConstraintPredicate, ConstraintReason, DesignRole, DesignRoleId,
        FunctionalConstraint,
    };

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn subject() -> FunctionalSubject {
        FunctionalSubject {
            kind: FunctionalSubjectKind::Assembly,
            id: id("assembly:patch-conduit"),
        }
    }

    fn subject_state(revision: u64) -> CommissioningSubjectState {
        CommissioningSubjectState::new(
            subject(),
            revision,
            format!("digest:assembly:patch-conduit:{revision}"),
        )
        .unwrap()
    }

    fn constraint_id() -> FunctionalConstraintId {
        FunctionalConstraintId::new(id("constraint:pressure-loss"))
    }

    fn design() -> FunctionalDesign {
        let role_id = DesignRoleId::new(id("role:conduit"));
        FunctionalDesign::new(
            FunctionalDesignId::new(id("functional-design:patch-conduit")),
            4,
            vec![DesignRole {
                id: role_id.clone(),
                subject_kind: FunctionalSubjectKind::Assembly,
            }],
            vec![FunctionalConstraint {
                id: constraint_id(),
                role_id,
                dimension_id: id("engineering:pressure-loss-pa"),
                predicate: ConstraintPredicate::MeasurementWithin {
                    lower: 0,
                    upper: 500,
                },
            }],
        )
        .unwrap()
    }

    fn plan() -> CommissioningPlan {
        CommissioningPlan::new(
            CommissioningPlanId::new(id("commissioning-plan:patch-conduit")),
            3,
            FunctionalSubjectKind::Assembly,
            &design(),
            vec![
                CommissioningRequirement {
                    id: CommissioningRequirementId::new(id("commissioning:req:pressure")),
                    kind: CommissioningRequirementKind::FunctionalConstraintSatisfied {
                        constraint_id: constraint_id(),
                    },
                },
                CommissioningRequirement {
                    id: CommissioningRequirementId::new(id("commissioning:req:pressure-test")),
                    kind: CommissioningRequirementKind::EvidenceKindPresent {
                        evidence_kind: id("evidence-kind:pressure-test"),
                    },
                },
            ],
        )
        .unwrap()
    }

    fn evidence(
        authority: &str,
        evidence_name: &str,
        state: CommissioningSubjectState,
    ) -> CommissioningEvidenceRef {
        CommissioningEvidenceRef::new(
            id(authority),
            id(evidence_name),
            1,
            format!("digest:{evidence_name}"),
            id("evidence-kind:pressure-test"),
            state,
        )
        .unwrap()
    }

    fn dossier(
        state: CommissioningSubjectState,
        evidence: Vec<CommissioningEvidenceRef>,
    ) -> CommissioningDossier {
        CommissioningDossier::new(
            CommissioningPlanId::new(id("commissioning-plan:patch-conduit")),
            3,
            state,
            evidence,
        )
        .unwrap()
    }

    fn functional(
        state: CommissioningSubjectState,
        outcome: ConstraintOutcome,
    ) -> BoundFunctionalEvaluation {
        BoundFunctionalEvaluation::new(
            state,
            id("authority:functional-engineering"),
            id("evidence:functional-evaluation"),
            2,
            "digest:functional-evaluation",
            FunctionalEvaluation {
                design_id: FunctionalDesignId::new(id("functional-design:patch-conduit")),
                design_revision: 4,
                evaluations: vec![ConstraintEvaluation {
                    constraint_id: constraint_id(),
                    outcome,
                    reason: ConstraintReason::AmbiguousEvidence,
                    supporting_facts: Vec::new(),
                }],
            },
        )
        .unwrap()
    }

    #[test]
    fn positive_evidence_yields_technical_readiness_only() {
        let state = subject_state(9);
        let assessment = plan()
            .assess(
                &dossier(
                    state.clone(),
                    vec![evidence(
                        "authority:test-rig",
                        "evidence:pressure-test",
                        state.clone(),
                    )],
                ),
                &functional(state, ConstraintOutcome::Satisfied),
            )
            .unwrap();
        assert!(assessment.technical_ready());
        let value = serde_json::to_value(assessment).unwrap();
        for forbidden in [
            "authorized",
            "registered",
            "device_bus",
            "lawful",
            "ownership",
            "operator_permission",
        ] {
            assert!(value.get(forbidden).is_none(), "unexpected field {forbidden}");
        }
    }

    #[test]
    fn stale_test_evidence_cannot_enter_current_dossier() {
        let current = subject_state(10);
        let stale = subject_state(9);
        assert!(matches!(
            CommissioningDossier::new(
                CommissioningPlanId::new(id("commissioning-plan:patch-conduit")),
                3,
                current,
                vec![evidence(
                    "authority:test-rig",
                    "evidence:pressure-test:stale",
                    stale,
                )],
            ),
            Err(CommissioningError::EvidenceSubjectStateMismatch { .. })
        ));
    }

    #[test]
    fn stale_functional_evaluation_cannot_commission_changed_subject() {
        let current = subject_state(10);
        let stale = subject_state(9);
        assert!(matches!(
            plan().assess(
                &dossier(current, Vec::new()),
                &functional(stale, ConstraintOutcome::Satisfied),
            ),
            Err(CommissioningError::FunctionalSubjectStateMismatch)
        ));
    }

    #[test]
    fn missing_test_evidence_is_unknown_not_failure() {
        let state = subject_state(9);
        let assessment = plan()
            .assess(
                &dossier(state.clone(), Vec::new()),
                &functional(state, ConstraintOutcome::Satisfied),
            )
            .unwrap();
        assert!(!assessment.technical_ready());
        assert!(assessment.evaluations.iter().any(|evaluation| {
            evaluation.reason == CommissioningReason::RequiredEvidenceMissing
                && evaluation.outcome == CommissioningRequirementOutcome::Unknown
        }));
    }

    #[test]
    fn known_functional_failure_blocks_technical_readiness() {
        let state = subject_state(9);
        let assessment = plan()
            .assess(
                &dossier(
                    state.clone(),
                    vec![evidence(
                        "authority:test-rig",
                        "evidence:pressure-test",
                        state.clone(),
                    )],
                ),
                &functional(state, ConstraintOutcome::Unsatisfied),
            )
            .unwrap();
        assert!(!assessment.technical_ready());
        assert!(assessment.evaluations.iter().any(|evaluation| {
            evaluation.reason == CommissioningReason::FunctionalConstraintUnsatisfied
                && evaluation.outcome == CommissioningRequirementOutcome::Unsatisfied
        }));
    }

    #[test]
    fn evidence_identity_is_scoped_by_authority() {
        let state = subject_state(9);
        let dossier = dossier(
            state.clone(),
            vec![
                evidence("authority:test-rig:a", "evidence:shared", state.clone()),
                evidence("authority:test-rig:b", "evidence:shared", state),
            ],
        );
        assert_eq!(dossier.evidence().len(), 2);
    }

    #[test]
    fn duplicate_evidence_identity_is_rejected() {
        let state = subject_state(9);
        let item = evidence(
            "authority:test-rig",
            "evidence:duplicate",
            state.clone(),
        );
        assert!(matches!(
            CommissioningDossier::new(
                CommissioningPlanId::new(id("commissioning-plan:patch-conduit")),
                3,
                state,
                vec![item.clone(), item],
            ),
            Err(CommissioningError::DuplicateEvidenceIdentity(_))
        ));
    }

    #[test]
    fn exact_design_revision_is_required() {
        let state = subject_state(9);
        let mut wrong = functional(state.clone(), ConstraintOutcome::Satisfied);
        wrong.evaluation.design_revision = 5;
        assert!(matches!(
            plan().assess(&dossier(state, Vec::new()), &wrong),
            Err(CommissioningError::FunctionalDesignMismatch { .. })
        ));
    }
}
