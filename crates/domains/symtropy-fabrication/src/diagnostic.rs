// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Evidence-preserving diagnostic cases over functional evaluations.
//!
//! Diagnostics organize competing explanations and candidate evidence-gathering
//! probes. They do not become a physics oracle, assign probabilistic confidence,
//! select a single "true" diagnosis, authorize operation, or commission devices.

use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};
use symtropy_game_state::StableId;

use crate::{
    ConstraintOutcome, FunctionalConstraintId, FunctionalDesign, FunctionalDesignId,
    FunctionalEvaluation, ProcessSpecId,
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

stable_id_type!(DiagnosticCaseId);
stable_id_type!(DiagnosticHypothesisId);
stable_id_type!(DiagnosticProbeId);

/// A diagnostic hypothesis may require a constraint to be known satisfied or
/// known unsatisfied. Unknown evidence can never count as affirmative support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnownConstraintState {
    Satisfied,
    Unsatisfied,
}

impl KnownConstraintState {
    const fn compare(self, actual: ConstraintOutcome) -> Option<bool> {
        match actual {
            ConstraintOutcome::Unknown => None,
            ConstraintOutcome::Satisfied => Some(matches!(self, Self::Satisfied)),
            ConstraintOutcome::Unsatisfied => Some(matches!(self, Self::Unsatisfied)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticCriterion {
    pub constraint_id: FunctionalConstraintId,
    pub required: KnownConstraintState,
}

/// One candidate explanation. No weight, probability, or rank is attached.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticHypothesis {
    pub id: DiagnosticHypothesisId,
    criteria: Vec<DiagnosticCriterion>,
}

impl DiagnosticHypothesis {
    pub fn new(
        id: DiagnosticHypothesisId,
        mut criteria: Vec<DiagnosticCriterion>,
    ) -> Result<Self, DiagnosticError> {
        if criteria.is_empty() {
            return Err(DiagnosticError::HypothesisCriterionRequired(id));
        }
        criteria.sort_by(|left, right| left.constraint_id.cmp(&right.constraint_id));
        for pair in criteria.windows(2) {
            if pair[0].constraint_id == pair[1].constraint_id {
                return Err(DiagnosticError::DuplicateHypothesisCriterion {
                    hypothesis_id: id,
                    constraint_id: pair[0].constraint_id.clone(),
                });
            }
        }
        Ok(Self { id, criteria })
    }

    pub fn criteria(&self) -> &[DiagnosticCriterion] {
        &self.criteria
    }
}

/// Candidate evidence-gathering operation for one or more constraints.
/// Referencing a process does not prove the probe is executable or sufficient.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticProbe {
    pub id: DiagnosticProbeId,
    pub process_spec_id: ProcessSpecId,
    pub process_spec_revision: u64,
    target_constraints: Vec<FunctionalConstraintId>,
    expected_evidence_kinds: Vec<StableId>,
}

impl DiagnosticProbe {
    pub fn new(
        id: DiagnosticProbeId,
        process_spec_id: ProcessSpecId,
        process_spec_revision: u64,
        mut target_constraints: Vec<FunctionalConstraintId>,
        mut expected_evidence_kinds: Vec<StableId>,
    ) -> Result<Self, DiagnosticError> {
        if target_constraints.is_empty() {
            return Err(DiagnosticError::ProbeTargetRequired(id));
        }
        if expected_evidence_kinds.is_empty() {
            return Err(DiagnosticError::ProbeEvidenceKindRequired(id));
        }

        target_constraints.sort();
        for pair in target_constraints.windows(2) {
            if pair[0] == pair[1] {
                return Err(DiagnosticError::DuplicateProbeTarget {
                    probe_id: id,
                    constraint_id: pair[0].clone(),
                });
            }
        }

        expected_evidence_kinds.sort();
        for pair in expected_evidence_kinds.windows(2) {
            if pair[0] == pair[1] {
                return Err(DiagnosticError::DuplicateProbeEvidenceKind {
                    probe_id: id,
                    evidence_kind: pair[0].clone(),
                });
            }
        }

        Ok(Self {
            id,
            process_spec_id,
            process_spec_revision,
            target_constraints,
            expected_evidence_kinds,
        })
    }

    pub fn target_constraints(&self) -> &[FunctionalConstraintId] {
        &self.target_constraints
    }

    pub fn expected_evidence_kinds(&self) -> &[StableId] {
        &self.expected_evidence_kinds
    }
}

/// Immutable diagnostic knowledge tied to one exact functional design revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticCase {
    pub id: DiagnosticCaseId,
    pub design_id: FunctionalDesignId,
    pub design_revision: u64,
    design_constraints: Vec<FunctionalConstraintId>,
    hypotheses: Vec<DiagnosticHypothesis>,
    probes: Vec<DiagnosticProbe>,
}

impl DiagnosticCase {
    pub fn new(
        id: DiagnosticCaseId,
        design: &FunctionalDesign,
        mut hypotheses: Vec<DiagnosticHypothesis>,
        mut probes: Vec<DiagnosticProbe>,
    ) -> Result<Self, DiagnosticError> {
        if hypotheses.is_empty() {
            return Err(DiagnosticError::HypothesisRequired);
        }

        let mut design_constraints = design
            .constraints
            .iter()
            .map(|constraint| constraint.id.clone())
            .collect::<Vec<_>>();
        design_constraints.sort();
        let known_constraints = design_constraints.iter().cloned().collect::<BTreeSet<_>>();

        hypotheses.sort_by(|left, right| left.id.cmp(&right.id));
        for pair in hypotheses.windows(2) {
            if pair[0].id == pair[1].id {
                return Err(DiagnosticError::DuplicateHypothesis(pair[0].id.clone()));
            }
        }
        for hypothesis in &hypotheses {
            for criterion in &hypothesis.criteria {
                if !known_constraints.contains(&criterion.constraint_id) {
                    return Err(DiagnosticError::UnknownConstraintInHypothesis {
                        hypothesis_id: hypothesis.id.clone(),
                        constraint_id: criterion.constraint_id.clone(),
                    });
                }
            }
        }

        probes.sort_by(|left, right| left.id.cmp(&right.id));
        for pair in probes.windows(2) {
            if pair[0].id == pair[1].id {
                return Err(DiagnosticError::DuplicateProbe(pair[0].id.clone()));
            }
        }
        for probe in &probes {
            for constraint_id in &probe.target_constraints {
                if !known_constraints.contains(constraint_id) {
                    return Err(DiagnosticError::UnknownConstraintInProbe {
                        probe_id: probe.id.clone(),
                        constraint_id: constraint_id.clone(),
                    });
                }
            }
        }

        Ok(Self {
            id,
            design_id: design.id.clone(),
            design_revision: design.revision,
            design_constraints,
            hypotheses,
            probes,
        })
    }

    pub fn hypotheses(&self) -> &[DiagnosticHypothesis] {
        &self.hypotheses
    }

    pub fn probes(&self) -> &[DiagnosticProbe] {
        &self.probes
    }

    /// Evaluates all hypotheses without ranking or selecting a winner.
    pub fn assess(
        &self,
        evaluation: &FunctionalEvaluation,
    ) -> Result<DiagnosticAssessment, DiagnosticError> {
        if evaluation.design_id != self.design_id || evaluation.design_revision != self.design_revision {
            return Err(DiagnosticError::EvaluationDesignMismatch {
                expected_id: self.design_id.clone(),
                expected_revision: self.design_revision,
                actual_id: evaluation.design_id.clone(),
                actual_revision: evaluation.design_revision,
            });
        }

        let mut by_constraint = BTreeMap::new();
        for constraint in &evaluation.evaluations {
            if by_constraint
                .insert(constraint.constraint_id.clone(), constraint.outcome)
                .is_some()
            {
                return Err(DiagnosticError::DuplicateConstraintEvaluation(
                    constraint.constraint_id.clone(),
                ));
            }
        }

        for expected in &self.design_constraints {
            if !by_constraint.contains_key(expected) {
                return Err(DiagnosticError::MissingConstraintEvaluation(expected.clone()));
            }
        }
        if let Some(unexpected) = by_constraint
            .keys()
            .find(|constraint_id| !self.design_constraints.contains(constraint_id))
        {
            return Err(DiagnosticError::UnexpectedConstraintEvaluation(
                unexpected.clone(),
            ));
        }

        let hypotheses = self
            .hypotheses
            .iter()
            .map(|hypothesis| evaluate_hypothesis(hypothesis, &by_constraint))
            .collect::<Vec<_>>();

        let unknown_constraints = by_constraint
            .iter()
            .filter_map(|(constraint_id, outcome)| {
                (*outcome == ConstraintOutcome::Unknown).then_some(constraint_id.clone())
            })
            .collect::<BTreeSet<_>>();

        let suggested_probes = self
            .probes
            .iter()
            .filter(|probe| {
                probe
                    .target_constraints
                    .iter()
                    .any(|constraint_id| unknown_constraints.contains(constraint_id))
            })
            .map(|probe| probe.id.clone())
            .collect();

        Ok(DiagnosticAssessment {
            case_id: self.id.clone(),
            design_id: self.design_id.clone(),
            design_revision: self.design_revision,
            hypotheses,
            suggested_probes,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HypothesisOutcome {
    Supported,
    Contradicted,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HypothesisAssessment {
    pub hypothesis_id: DiagnosticHypothesisId,
    pub outcome: HypothesisOutcome,
    pub matching_constraints: Vec<FunctionalConstraintId>,
    pub contradicting_constraints: Vec<FunctionalConstraintId>,
    pub unresolved_constraints: Vec<FunctionalConstraintId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticAssessment {
    pub case_id: DiagnosticCaseId,
    pub design_id: FunctionalDesignId,
    pub design_revision: u64,
    pub hypotheses: Vec<HypothesisAssessment>,
    pub suggested_probes: Vec<DiagnosticProbeId>,
}

impl DiagnosticAssessment {
    pub fn supported_hypotheses(&self) -> Vec<&HypothesisAssessment> {
        self.hypotheses
            .iter()
            .filter(|hypothesis| hypothesis.outcome == HypothesisOutcome::Supported)
            .collect()
    }

    pub fn unresolved_hypotheses(&self) -> Vec<&HypothesisAssessment> {
        self.hypotheses
            .iter()
            .filter(|hypothesis| hypothesis.outcome == HypothesisOutcome::Unresolved)
            .collect()
    }
}

fn evaluate_hypothesis(
    hypothesis: &DiagnosticHypothesis,
    by_constraint: &BTreeMap<FunctionalConstraintId, ConstraintOutcome>,
) -> HypothesisAssessment {
    let mut matching_constraints = Vec::new();
    let mut contradicting_constraints = Vec::new();
    let mut unresolved_constraints = Vec::new();

    for criterion in &hypothesis.criteria {
        let actual = *by_constraint
            .get(&criterion.constraint_id)
            .expect("diagnostic case validates complete evaluation coverage");
        match criterion.required.compare(actual) {
            Some(true) => matching_constraints.push(criterion.constraint_id.clone()),
            Some(false) => contradicting_constraints.push(criterion.constraint_id.clone()),
            None => unresolved_constraints.push(criterion.constraint_id.clone()),
        }
    }

    let outcome = if !contradicting_constraints.is_empty() {
        HypothesisOutcome::Contradicted
    } else if !unresolved_constraints.is_empty() {
        HypothesisOutcome::Unresolved
    } else {
        HypothesisOutcome::Supported
    };

    HypothesisAssessment {
        hypothesis_id: hypothesis.id.clone(),
        outcome,
        matching_constraints,
        contradicting_constraints,
        unresolved_constraints,
    }
}

#[derive(Debug)]
pub enum DiagnosticError {
    HypothesisRequired,
    HypothesisCriterionRequired(DiagnosticHypothesisId),
    DuplicateHypothesisCriterion {
        hypothesis_id: DiagnosticHypothesisId,
        constraint_id: FunctionalConstraintId,
    },
    ProbeTargetRequired(DiagnosticProbeId),
    ProbeEvidenceKindRequired(DiagnosticProbeId),
    DuplicateProbeTarget {
        probe_id: DiagnosticProbeId,
        constraint_id: FunctionalConstraintId,
    },
    DuplicateProbeEvidenceKind {
        probe_id: DiagnosticProbeId,
        evidence_kind: StableId,
    },
    DuplicateHypothesis(DiagnosticHypothesisId),
    UnknownConstraintInHypothesis {
        hypothesis_id: DiagnosticHypothesisId,
        constraint_id: FunctionalConstraintId,
    },
    DuplicateProbe(DiagnosticProbeId),
    UnknownConstraintInProbe {
        probe_id: DiagnosticProbeId,
        constraint_id: FunctionalConstraintId,
    },
    EvaluationDesignMismatch {
        expected_id: FunctionalDesignId,
        expected_revision: u64,
        actual_id: FunctionalDesignId,
        actual_revision: u64,
    },
    DuplicateConstraintEvaluation(FunctionalConstraintId),
    MissingConstraintEvaluation(FunctionalConstraintId),
    UnexpectedConstraintEvaluation(FunctionalConstraintId),
}

impl fmt::Display for DiagnosticError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HypothesisRequired => {
                write!(formatter, "diagnostic case requires at least one hypothesis")
            }
            Self::HypothesisCriterionRequired(id) => {
                write!(formatter, "diagnostic hypothesis {id} requires at least one criterion")
            }
            Self::DuplicateHypothesisCriterion {
                hypothesis_id,
                constraint_id,
            } => write!(
                formatter,
                "diagnostic hypothesis {hypothesis_id} repeats constraint {constraint_id}"
            ),
            Self::ProbeTargetRequired(id) => {
                write!(formatter, "diagnostic probe {id} requires at least one target constraint")
            }
            Self::ProbeEvidenceKindRequired(id) => {
                write!(formatter, "diagnostic probe {id} requires at least one expected evidence kind")
            }
            Self::DuplicateProbeTarget {
                probe_id,
                constraint_id,
            } => write!(
                formatter,
                "diagnostic probe {probe_id} repeats target constraint {constraint_id}"
            ),
            Self::DuplicateProbeEvidenceKind {
                probe_id,
                evidence_kind,
            } => write!(
                formatter,
                "diagnostic probe {probe_id} repeats expected evidence kind {evidence_kind}"
            ),
            Self::DuplicateHypothesis(id) => {
                write!(formatter, "diagnostic case repeats hypothesis {id}")
            }
            Self::UnknownConstraintInHypothesis {
                hypothesis_id,
                constraint_id,
            } => write!(
                formatter,
                "diagnostic hypothesis {hypothesis_id} references unknown constraint {constraint_id}"
            ),
            Self::DuplicateProbe(id) => write!(formatter, "diagnostic case repeats probe {id}"),
            Self::UnknownConstraintInProbe {
                probe_id,
                constraint_id,
            } => write!(
                formatter,
                "diagnostic probe {probe_id} references unknown constraint {constraint_id}"
            ),
            Self::EvaluationDesignMismatch {
                expected_id,
                expected_revision,
                actual_id,
                actual_revision,
            } => write!(
                formatter,
                "diagnostic case expects design {expected_id}@{expected_revision}, got {actual_id}@{actual_revision}"
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

impl Error for DiagnosticError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ConstraintEvaluation, ConstraintPredicate, ConstraintReason, DesignRole, DesignRoleId,
        FunctionalConstraint, FunctionalSubjectKind,
    };

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn constraint_id(name: &str) -> FunctionalConstraintId {
        FunctionalConstraintId::new(id(&format!("constraint:{name}")))
    }

    fn design() -> FunctionalDesign {
        let role_id = DesignRoleId::new(id("role:pump"));
        FunctionalDesign::new(
            FunctionalDesignId::new(id("functional-design:pump-diagnostic")),
            7,
            vec![DesignRole {
                id: role_id.clone(),
                subject_kind: FunctionalSubjectKind::Assembly,
            }],
            vec![
                FunctionalConstraint {
                    id: constraint_id("alignment"),
                    role_id: role_id.clone(),
                    dimension_id: id("engineering:alignment"),
                    predicate: ConstraintPredicate::MeasurementWithin {
                        lower: -50,
                        upper: 50,
                    },
                },
                FunctionalConstraint {
                    id: constraint_id("bearing-temperature"),
                    role_id,
                    dimension_id: id("engineering:bearing-temperature"),
                    predicate: ConstraintPredicate::MeasurementWithin {
                        lower: 0,
                        upper: 90,
                    },
                },
            ],
        )
        .unwrap()
    }

    fn hypothesis(
        name: &str,
        criteria: Vec<(&str, KnownConstraintState)>,
    ) -> DiagnosticHypothesis {
        DiagnosticHypothesis::new(
            DiagnosticHypothesisId::new(id(&format!("hypothesis:{name}"))),
            criteria
                .into_iter()
                .map(|(constraint, required)| DiagnosticCriterion {
                    constraint_id: constraint_id(constraint),
                    required,
                })
                .collect(),
        )
        .unwrap()
    }

    fn probe() -> DiagnosticProbe {
        DiagnosticProbe::new(
            DiagnosticProbeId::new(id("probe:laser-alignment")),
            ProcessSpecId::new(id("process-spec:inspect-alignment")),
            2,
            vec![constraint_id("alignment")],
            vec![id("evidence-kind:alignment-measurement")],
        )
        .unwrap()
    }

    fn case() -> DiagnosticCase {
        DiagnosticCase::new(
            DiagnosticCaseId::new(id("diagnostic-case:pump")),
            &design(),
            vec![
                hypothesis(
                    "misalignment",
                    vec![("alignment", KnownConstraintState::Unsatisfied)],
                ),
                hypothesis(
                    "bearing-overheat",
                    vec![("bearing-temperature", KnownConstraintState::Unsatisfied)],
                ),
            ],
            vec![probe()],
        )
        .unwrap()
    }

    fn evaluation(
        alignment: ConstraintOutcome,
        temperature: ConstraintOutcome,
    ) -> FunctionalEvaluation {
        FunctionalEvaluation {
            design_id: FunctionalDesignId::new(id("functional-design:pump-diagnostic")),
            design_revision: 7,
            evaluations: vec![
                ConstraintEvaluation {
                    constraint_id: constraint_id("alignment"),
                    outcome: alignment,
                    reason: ConstraintReason::AmbiguousEvidence,
                    supporting_facts: Vec::new(),
                },
                ConstraintEvaluation {
                    constraint_id: constraint_id("bearing-temperature"),
                    outcome: temperature,
                    reason: ConstraintReason::AmbiguousEvidence,
                    supporting_facts: Vec::new(),
                },
            ],
        }
    }

    fn assessment_for<'a>(
        assessment: &'a DiagnosticAssessment,
        hypothesis_name: &str,
    ) -> &'a HypothesisAssessment {
        let expected = DiagnosticHypothesisId::new(id(&format!("hypothesis:{hypothesis_name}")));
        assessment
            .hypotheses
            .iter()
            .find(|candidate| candidate.hypothesis_id == expected)
            .unwrap()
    }

    #[test]
    fn unknown_evidence_preserves_competing_hypothesis_as_unresolved() {
        let assessment = case()
            .assess(&evaluation(
                ConstraintOutcome::Unknown,
                ConstraintOutcome::Satisfied,
            ))
            .unwrap();

        assert_eq!(
            assessment_for(&assessment, "misalignment").outcome,
            HypothesisOutcome::Unresolved
        );
        assert_eq!(
            assessment_for(&assessment, "bearing-overheat").outcome,
            HypothesisOutcome::Contradicted
        );
        assert_eq!(
            assessment.suggested_probes,
            vec![DiagnosticProbeId::new(id("probe:laser-alignment"))]
        );
    }

    #[test]
    fn multiple_hypotheses_may_remain_supported_without_ranking() {
        let assessment = case()
            .assess(&evaluation(
                ConstraintOutcome::Unsatisfied,
                ConstraintOutcome::Unsatisfied,
            ))
            .unwrap();
        assert_eq!(assessment.supported_hypotheses().len(), 2);
        assert!(assessment.unresolved_hypotheses().is_empty());
    }

    #[test]
    fn contradicted_criterion_overrides_matching_criteria() {
        let combined = DiagnosticHypothesis::new(
            DiagnosticHypothesisId::new(id("hypothesis:combined")),
            vec![
                DiagnosticCriterion {
                    constraint_id: constraint_id("alignment"),
                    required: KnownConstraintState::Unsatisfied,
                },
                DiagnosticCriterion {
                    constraint_id: constraint_id("bearing-temperature"),
                    required: KnownConstraintState::Unsatisfied,
                },
            ],
        )
        .unwrap();
        let diagnostic = DiagnosticCase::new(
            DiagnosticCaseId::new(id("diagnostic-case:combined")),
            &design(),
            vec![combined],
            Vec::new(),
        )
        .unwrap();
        let assessment = diagnostic
            .assess(&evaluation(
                ConstraintOutcome::Unsatisfied,
                ConstraintOutcome::Satisfied,
            ))
            .unwrap();
        assert_eq!(assessment.hypotheses[0].outcome, HypothesisOutcome::Contradicted);
        assert_eq!(assessment.hypotheses[0].matching_constraints.len(), 1);
        assert_eq!(assessment.hypotheses[0].contradicting_constraints.len(), 1);
    }

    #[test]
    fn exact_design_revision_is_required() {
        let mut wrong = evaluation(ConstraintOutcome::Unknown, ConstraintOutcome::Unknown);
        wrong.design_revision = 8;
        assert!(matches!(
            case().assess(&wrong),
            Err(DiagnosticError::EvaluationDesignMismatch { .. })
        ));
    }

    #[test]
    fn unknown_constraint_references_are_rejected_at_case_construction() {
        let invalid = hypothesis(
            "invalid",
            vec![("not-in-design", KnownConstraintState::Unsatisfied)],
        );
        assert!(matches!(
            DiagnosticCase::new(
                DiagnosticCaseId::new(id("diagnostic-case:invalid")),
                &design(),
                vec![invalid],
                Vec::new(),
            ),
            Err(DiagnosticError::UnknownConstraintInHypothesis { .. })
        ));
    }

    #[test]
    fn serialized_assessment_contains_no_confidence_rank_or_authority_claim() {
        let assessment = case()
            .assess(&evaluation(
                ConstraintOutcome::Unsatisfied,
                ConstraintOutcome::Unknown,
            ))
            .unwrap();
        let value = serde_json::to_value(assessment).unwrap();
        for forbidden in [
            "confidence",
            "probability",
            "score",
            "rank",
            "diagnosis",
            "authorized",
            "commissioned",
        ] {
            assert!(value.get(forbidden).is_none(), "unexpected field {forbidden}");
        }
    }

    #[test]
    fn diagnostic_case_requires_complete_functional_evaluation() {
        let incomplete = FunctionalEvaluation {
            design_id: FunctionalDesignId::new(id("functional-design:pump-diagnostic")),
            design_revision: 7,
            evaluations: vec![ConstraintEvaluation {
                constraint_id: constraint_id("alignment"),
                outcome: ConstraintOutcome::Unknown,
                reason: ConstraintReason::MissingEvidence,
                supporting_facts: Vec::new(),
            }],
        };
        assert!(matches!(
            case().assess(&incomplete),
            Err(DiagnosticError::MissingConstraintEvaluation(_))
        ));
    }

    #[test]
    fn diagnostic_case_serialization_carries_no_world_binding_or_runtime_state() {
        let value = serde_json::to_value(case()).unwrap();
        for forbidden in ["binding", "world_state", "progress", "selected_diagnosis"] {
            assert!(value.get(forbidden).is_none(), "unexpected field {forbidden}");
        }
    }
}
