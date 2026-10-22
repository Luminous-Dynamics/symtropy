// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic functional substitution search.
//!
//! The solver searches candidate role bindings against F7 functional
//! constraints. It does not spawn matter, choose a politically preferred
//! design, assign a hidden quality score, or authorize operation.

use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, error::Error, fmt};

use crate::{
    ConstraintOutcome, DesignBinding, DesignRoleId, EngineeringFactSet, FunctionalDesign,
    FunctionalEvaluation, FunctionalSubject, FunctionalSubjectKind, RoleBinding,
};

/// Candidate subjects that may satisfy one functional design role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleCandidateSet {
    pub role_id: DesignRoleId,
    pub candidates: Vec<FunctionalSubject>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubstitutionStatus {
    /// Every functional constraint has affirmative evidence.
    Verified,
    /// No known failure exists, but at least one constraint remains unknown.
    Conditional,
}

/// One non-failing substitution candidate. `Verified` is functional evidence
/// closure only and carries no civil/Device-Bus authorization semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubstitutionSolution {
    pub status: SubstitutionStatus,
    pub binding: DesignBinding,
    pub evaluation: FunctionalEvaluation,
}

/// Search result preserves uncertainty and search completeness explicitly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubstitutionResult {
    pub total_combinations: u64,
    pub explored_combinations: u64,
    pub rejected_combinations: u64,
    pub truncated: bool,
    pub verified: Vec<SubstitutionSolution>,
    pub conditional: Vec<SubstitutionSolution>,
}

impl SubstitutionResult {
    pub fn is_exhaustive(&self) -> bool {
        !self.truncated && self.explored_combinations == self.total_combinations
    }
}

/// Bounded deterministic solver. The bound is an execution-safety budget, not
/// a heuristic score cutoff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubstitutionSolver {
    pub max_combinations: u64,
}

impl SubstitutionSolver {
    pub fn new(max_combinations: u64) -> Result<Self, SubstitutionError> {
        if max_combinations == 0 {
            return Err(SubstitutionError::ZeroSearchBudget);
        }
        Ok(Self { max_combinations })
    }

    pub fn solve(
        &self,
        design: &FunctionalDesign,
        candidate_sets: &[RoleCandidateSet],
        facts: &EngineeringFactSet,
    ) -> Result<SubstitutionResult, SubstitutionError> {
        let normalized = normalize_candidate_sets(design, candidate_sets)?;
        let total_combinations = normalized.iter().fold(1_u64, |total, set| {
            total.saturating_mul(set.candidates.len() as u64)
        });
        let search_limit = total_combinations.min(self.max_combinations);

        let mut result = SubstitutionResult {
            total_combinations,
            explored_combinations: 0,
            rejected_combinations: 0,
            truncated: total_combinations > self.max_combinations,
            verified: Vec::new(),
            conditional: Vec::new(),
        };
        let mut assignments = Vec::with_capacity(normalized.len());
        visit_bindings(
            design,
            &normalized,
            facts,
            search_limit,
            0,
            &mut assignments,
            &mut result,
        );
        Ok(result)
    }
}

fn normalize_candidate_sets(
    design: &FunctionalDesign,
    candidate_sets: &[RoleCandidateSet],
) -> Result<Vec<RoleCandidateSet>, SubstitutionError> {
    if candidate_sets.len() != design.roles.len() {
        return Err(SubstitutionError::RoleCoverage {
            expected: design.roles.len(),
            provided: candidate_sets.len(),
        });
    }

    for (index, set) in candidate_sets.iter().enumerate() {
        if candidate_sets[..index]
            .iter()
            .any(|existing| existing.role_id == set.role_id)
        {
            return Err(SubstitutionError::DuplicateCandidateRole(set.role_id.clone()));
        }
        if design.role(&set.role_id).is_none() {
            return Err(SubstitutionError::UnknownRole(set.role_id.clone()));
        }
        if set.candidates.is_empty() {
            return Err(SubstitutionError::EmptyCandidateSet(set.role_id.clone()));
        }
        for (candidate_index, candidate) in set.candidates.iter().enumerate() {
            if set.candidates[..candidate_index].contains(candidate) {
                return Err(SubstitutionError::DuplicateCandidate {
                    role_id: set.role_id.clone(),
                    subject: candidate.clone(),
                });
            }
        }
    }

    for role in &design.roles {
        if !candidate_sets.iter().any(|set| set.role_id == role.id) {
            return Err(SubstitutionError::MissingRole(role.id.clone()));
        }
    }

    let mut normalized = candidate_sets.to_vec();
    normalized.sort_by(|left, right| {
        left.role_id
            .stable_id()
            .as_str()
            .cmp(right.role_id.stable_id().as_str())
    });
    for set in &mut normalized {
        set.candidates.sort_by(compare_subjects);
    }
    Ok(normalized)
}

fn compare_subjects(left: &FunctionalSubject, right: &FunctionalSubject) -> Ordering {
    subject_kind_rank(left.kind)
        .cmp(&subject_kind_rank(right.kind))
        .then_with(|| left.id.as_str().cmp(right.id.as_str()))
}

const fn subject_kind_rank(kind: FunctionalSubjectKind) -> u8 {
    match kind {
        FunctionalSubjectKind::Workpiece => 0,
        FunctionalSubjectKind::Interface => 1,
        FunctionalSubjectKind::Joint => 2,
        FunctionalSubjectKind::Assembly => 3,
        FunctionalSubjectKind::Process => 4,
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_bindings(
    design: &FunctionalDesign,
    candidate_sets: &[RoleCandidateSet],
    facts: &EngineeringFactSet,
    search_limit: u64,
    index: usize,
    assignments: &mut Vec<RoleBinding>,
    result: &mut SubstitutionResult,
) {
    if result.explored_combinations >= search_limit {
        return;
    }

    if index == candidate_sets.len() {
        result.explored_combinations += 1;
        let binding = DesignBinding::new(assignments.clone())
            .expect("normalized substitution roles are unique");
        let evaluation = design.evaluate(&binding, facts);

        if evaluation.all_satisfied() {
            result.verified.push(SubstitutionSolution {
                status: SubstitutionStatus::Verified,
                binding,
                evaluation,
            });
            return;
        }

        if evaluation
            .evaluations
            .iter()
            .any(|constraint| constraint.outcome == ConstraintOutcome::Unsatisfied)
        {
            result.rejected_combinations += 1;
            return;
        }

        result.conditional.push(SubstitutionSolution {
            status: SubstitutionStatus::Conditional,
            binding,
            evaluation,
        });
        return;
    }

    let set = &candidate_sets[index];
    for candidate in &set.candidates {
        if result.explored_combinations >= search_limit {
            break;
        }
        assignments.push(RoleBinding {
            role_id: set.role_id.clone(),
            subject: candidate.clone(),
        });
        visit_bindings(
            design,
            candidate_sets,
            facts,
            search_limit,
            index + 1,
            assignments,
            result,
        );
        assignments.pop();
    }
}

#[derive(Debug)]
pub enum SubstitutionError {
    ZeroSearchBudget,
    RoleCoverage { expected: usize, provided: usize },
    DuplicateCandidateRole(DesignRoleId),
    UnknownRole(DesignRoleId),
    MissingRole(DesignRoleId),
    EmptyCandidateSet(DesignRoleId),
    DuplicateCandidate {
        role_id: DesignRoleId,
        subject: FunctionalSubject,
    },
}

impl fmt::Display for SubstitutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroSearchBudget => write!(formatter, "substitution search budget must be non-zero"),
            Self::RoleCoverage { expected, provided } => write!(
                formatter,
                "substitution requires exactly one candidate set per design role: expected {expected}, got {provided}"
            ),
            Self::DuplicateCandidateRole(role) => {
                write!(formatter, "substitution repeats candidate set for role {role}")
            }
            Self::UnknownRole(role) => {
                write!(formatter, "substitution candidate set references unknown role {role}")
            }
            Self::MissingRole(role) => {
                write!(formatter, "substitution has no candidate set for role {role}")
            }
            Self::EmptyCandidateSet(role) => {
                write!(formatter, "substitution role {role} has no candidates")
            }
            Self::DuplicateCandidate { role_id, subject } => write!(
                formatter,
                "substitution role {role_id} repeats candidate {}",
                subject.id
            ),
        }
    }
}

impl Error for SubstitutionError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ConstraintPredicate, DesignRole, EngineeringEvidenceRef, EngineeringFact,
        EngineeringFactId, EngineeringFactValue, FunctionalConstraint, FunctionalConstraintId,
        FunctionalDesignId, MeasurementInterval,
    };
    use symtropy_game_state::StableId;

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn design() -> FunctionalDesign {
        FunctionalDesign::new(
            FunctionalDesignId::new(id("functional-design:conduit-repair")),
            1,
            vec![DesignRole {
                id: DesignRoleId::new(id("role:conduit")),
                subject_kind: FunctionalSubjectKind::Assembly,
            }],
            vec![FunctionalConstraint {
                id: FunctionalConstraintId::new(id("constraint:pressure-loss")),
                role_id: DesignRoleId::new(id("role:conduit")),
                dimension_id: id("engineering:pressure-loss-pa"),
                predicate: ConstraintPredicate::MeasurementWithin {
                    lower: 0,
                    upper: 500,
                },
            }],
        )
        .unwrap()
    }

    fn subject(name: &str) -> FunctionalSubject {
        FunctionalSubject {
            kind: FunctionalSubjectKind::Assembly,
            id: id(name),
        }
    }

    fn fact(subject: FunctionalSubject, lower: i64, upper: i64, suffix: &str) -> EngineeringFact {
        EngineeringFact {
            id: EngineeringFactId::new(id(&format!("fact:{suffix}"))),
            subject,
            dimension_id: id("engineering:pressure-loss-pa"),
            value: EngineeringFactValue::Measurement {
                interval: MeasurementInterval::new(lower, upper, 5).unwrap(),
            },
            evidence: EngineeringEvidenceRef::new(
                id("authority:pressure-test"),
                id(&format!("evidence:{suffix}")),
                1,
                format!("digest:{suffix}"),
            )
            .unwrap(),
        }
    }

    fn candidates(order: &[&str]) -> Vec<RoleCandidateSet> {
        vec![RoleCandidateSet {
            role_id: DesignRoleId::new(id("role:conduit")),
            candidates: order.iter().map(|name| subject(name)).collect(),
        }]
    }

    #[test]
    fn functional_substitute_can_pass_without_matching_an_authored_item_recipe() {
        let salvaged = subject("assembly:salvaged-copper-sleeve");
        let printed = subject("assembly:printed-standard-patch");
        let facts = EngineeringFactSet::new(vec![
            fact(salvaged.clone(), 100, 140, "salvaged"),
            fact(printed.clone(), 700, 760, "printed"),
        ])
        .unwrap();
        let result = SubstitutionSolver::new(16)
            .unwrap()
            .solve(
                &design(),
                &candidates(&[
                    "assembly:printed-standard-patch",
                    "assembly:salvaged-copper-sleeve",
                ]),
                &facts,
            )
            .unwrap();

        assert_eq!(result.verified.len(), 1);
        assert_eq!(
            result.verified[0].binding.assignments()[0].subject,
            salvaged
        );
        assert_eq!(result.rejected_combinations, 1);
    }

    #[test]
    fn missing_evidence_is_conditional_never_verified() {
        let facts = EngineeringFactSet::new(Vec::new()).unwrap();
        let result = SubstitutionSolver::new(4)
            .unwrap()
            .solve(&design(), &candidates(&["assembly:unknown-patch"]), &facts)
            .unwrap();
        assert!(result.verified.is_empty());
        assert_eq!(result.conditional.len(), 1);
    }

    #[test]
    fn candidate_input_order_does_not_change_solution_order() {
        let a = subject("assembly:a");
        let b = subject("assembly:b");
        let facts = EngineeringFactSet::new(vec![
            fact(a, 100, 120, "a"),
            fact(b, 120, 140, "b"),
        ])
        .unwrap();
        let solver = SubstitutionSolver::new(8).unwrap();
        let forward = solver
            .solve(&design(), &candidates(&["assembly:a", "assembly:b"]), &facts)
            .unwrap();
        let reverse = solver
            .solve(&design(), &candidates(&["assembly:b", "assembly:a"]), &facts)
            .unwrap();

        let forward_ids = forward
            .verified
            .iter()
            .map(|solution| solution.binding.assignments()[0].subject.id.clone())
            .collect::<Vec<_>>();
        let reverse_ids = reverse
            .verified
            .iter()
            .map(|solution| solution.binding.assignments()[0].subject.id.clone())
            .collect::<Vec<_>>();
        assert_eq!(forward_ids, reverse_ids);
    }

    #[test]
    fn bounded_search_reports_non_exhaustiveness() {
        let facts = EngineeringFactSet::new(Vec::new()).unwrap();
        let result = SubstitutionSolver::new(1)
            .unwrap()
            .solve(
                &design(),
                &candidates(&["assembly:a", "assembly:b", "assembly:c"]),
                &facts,
            )
            .unwrap();
        assert_eq!(result.total_combinations, 3);
        assert_eq!(result.explored_combinations, 1);
        assert!(result.truncated);
        assert!(!result.is_exhaustive());
    }

    #[test]
    fn result_has_no_hidden_rank_score_or_authority_claim() {
        let result = SubstitutionSolver::new(2)
            .unwrap()
            .solve(
                &design(),
                &candidates(&["assembly:a"]),
                &EngineeringFactSet::new(Vec::new()).unwrap(),
            )
            .unwrap();
        let value = serde_json::to_value(result).unwrap();
        assert!(value.get("score").is_none());
        assert!(value.get("rank").is_none());
        assert!(value.get("authorized").is_none());
        assert!(value.get("commissioned").is_none());
    }
}
