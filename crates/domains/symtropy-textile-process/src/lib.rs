// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic, evidence-aware primitives for textile process assessment.
//!
//! This crate does not simulate a factory or calculate environmental impacts.
//! It validates already-declared objective observations and hard-gate evidence,
//! excludes blocked/incomplete candidates from ranking, and produces a
//! deterministic Pareto frontier. Values are integer ticks to avoid
//! floating-point ordering ambiguity; units/scales are declared and must match
//! the comparison contract exactly.
//!
//! A caller must not translate synthetic inputs into factory claims. All values
//! and evidence references supplied here are caller-owned assertions.

pub mod garment_profile;

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

/// Whether lower or higher values are preferred for one objective.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    Minimize,
    Maximize,
}

/// A declared objective contract shared by every candidate in a comparison.
///
/// `scale` is ticks per declared unit (for example, 1_000 ticks per kWh
/// represents millikWh). The evaluator performs no unit conversions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectiveSpec {
    pub id: String,
    pub unit: String,
    pub scale: u32,
    pub direction: Direction,
}

/// Result of checking a non-compensatory hard constraint.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GateOutcome {
    Pass { evidence_ref: String },
    Fail { evidence_ref: String },
    Unknown { reason: String },
}

/// One required constraint with a stable identifier.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GateCheck {
    pub id: String,
    pub outcome: GateOutcome,
}

/// One objective observation for a candidate.
///
/// `value_ticks` must use the exact unit and scale declared by the matching
/// `ObjectiveSpec`. Evidence references are identifiers, not authentication:
/// callers must independently validate their provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectiveValue {
    pub objective_id: String,
    pub value_ticks: i64,
    pub unit: String,
    pub scale: u32,
    pub evidence_ref: String,
}

/// A candidate plan/route that has already been assessed by upstream owners.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateEvaluation {
    pub id: String,
    pub gates: Vec<GateCheck>,
    pub values: Vec<ObjectiveValue>,
}

/// Why a candidate was not admitted to Pareto comparison.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Disposition {
    Feasible,
    Blocked { failed_gate_ids: Vec<String> },
    Incomplete { unknown_gate_ids: Vec<String> },
}

/// A stable, per-candidate disposition record in the result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateDisposition {
    pub candidate_id: String,
    pub disposition: Disposition,
}

/// A deterministic witness that one candidate dominates another.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct DominanceWitness {
    pub dominator_id: String,
    pub dominated_id: String,
}

/// The complete report. All vectors are returned in lexical order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComparisonReport {
    /// Stable identifier supplied by the caller for the objective/period/scope manifest.
    pub comparison_id: String,
    pub frontier: Vec<String>,
    pub dominated: Vec<DominanceWitness>,
    pub dispositions: Vec<CandidateDisposition>,
}

/// Input contract or evidence-shape failures that invalidate the comparison.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComparisonError {
    EmptyComparisonId,
    TooFewObjectives,
    EmptyObjectiveId,
    EmptyObjectiveUnit { objective_id: String },
    ZeroObjectiveScale { objective_id: String },
    DuplicateObjectiveId { objective_id: String },
    EmptyCandidateId,
    DuplicateCandidateId { candidate_id: String },
    NoHardGates { candidate_id: String },
    EmptyGateId { candidate_id: String },
    DuplicateGateId { candidate_id: String, gate_id: String },
    MissingGateEvidence { candidate_id: String, gate_id: String },
    EmptyUnknownReason { candidate_id: String, gate_id: String },
    EmptyValueEvidence { candidate_id: String, objective_id: String },
    UnknownObjective { candidate_id: String, objective_id: String },
    DuplicateObjectiveValue { candidate_id: String, objective_id: String },
    ObjectiveUnitMismatch { candidate_id: String, objective_id: String },
    ObjectiveScaleMismatch { candidate_id: String, objective_id: String },
    MissingObjectiveValue { candidate_id: String, objective_id: String },
}

impl fmt::Display for ComparisonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyComparisonId => write!(f, "comparison ID must not be empty"),
            Self::TooFewObjectives => {
                write!(f, "at least two objectives are required for Pareto comparison")
            }
            Self::EmptyObjectiveId => write!(f, "objective ID must not be empty"),
            Self::EmptyObjectiveUnit { objective_id } => {
                write!(f, "objective '{objective_id}' has an empty unit")
            }
            Self::ZeroObjectiveScale { objective_id } => {
                write!(f, "objective '{objective_id}' has zero scale")
            }
            Self::DuplicateObjectiveId { objective_id } => {
                write!(f, "duplicate objective ID '{objective_id}'")
            }
            Self::EmptyCandidateId => write!(f, "candidate ID must not be empty"),
            Self::DuplicateCandidateId { candidate_id } => {
                write!(f, "duplicate candidate ID '{candidate_id}'")
            }
            Self::NoHardGates { candidate_id } => {
                write!(f, "candidate '{candidate_id}' has no hard-gate evidence")
            }
            Self::EmptyGateId { candidate_id } => {
                write!(f, "candidate '{candidate_id}' has an empty gate ID")
            }
            Self::DuplicateGateId { candidate_id, gate_id } => {
                write!(f, "candidate '{candidate_id}' repeats gate '{gate_id}'")
            }
            Self::MissingGateEvidence { candidate_id, gate_id } => {
                write!(f, "candidate '{candidate_id}' gate '{gate_id}' lacks evidence")
            }
            Self::EmptyUnknownReason { candidate_id, gate_id } => {
                write!(f, "candidate '{candidate_id}' unknown gate '{gate_id}' lacks a reason")
            }
            Self::EmptyValueEvidence { candidate_id, objective_id } => {
                write!(f, "candidate '{candidate_id}' objective '{objective_id}' lacks evidence")
            }
            Self::UnknownObjective { candidate_id, objective_id } => {
                write!(f, "candidate '{candidate_id}' uses undeclared objective '{objective_id}'")
            }
            Self::DuplicateObjectiveValue { candidate_id, objective_id } => {
                write!(f, "candidate '{candidate_id}' repeats objective '{objective_id}'")
            }
            Self::ObjectiveUnitMismatch { candidate_id, objective_id } => {
                write!(
                    f,
                    "candidate '{candidate_id}' unit disagrees for objective '{objective_id}'"
                )
            }
            Self::ObjectiveScaleMismatch { candidate_id, objective_id } => {
                write!(
                    f,
                    "candidate '{candidate_id}' scale disagrees for objective '{objective_id}'"
                )
            }
            Self::MissingObjectiveValue { candidate_id, objective_id } => {
                write!(f, "feasible candidate '{candidate_id}' lacks objective '{objective_id}'")
            }
        }
    }
}

impl Error for ComparisonError {}

/// Validate, classify, and compare a fixed set of candidates deterministically.
///
/// A candidate is blocked if any gate fails. Otherwise it is incomplete if any
/// gate is unknown. Only candidates with all gates passing and a complete
/// objective vector can enter the frontier. Favorable objective values never
/// compensate for a failed or unknown hard gate.
pub fn compare_candidates(
    comparison_id: &str,
    objectives: &[ObjectiveSpec],
    candidates: &[CandidateEvaluation],
) -> Result<ComparisonReport, ComparisonError> {
    if comparison_id.trim().is_empty() {
        return Err(ComparisonError::EmptyComparisonId);
    }
    if objectives.len() < 2 {
        return Err(ComparisonError::TooFewObjectives);
    }

    let mut objective_by_id = BTreeMap::new();
    for objective in objectives {
        if objective.id.trim().is_empty() {
            return Err(ComparisonError::EmptyObjectiveId);
        }
        if objective.unit.trim().is_empty() {
            return Err(ComparisonError::EmptyObjectiveUnit {
                objective_id: objective.id.clone(),
            });
        }
        if objective.scale == 0 {
            return Err(ComparisonError::ZeroObjectiveScale {
                objective_id: objective.id.clone(),
            });
        }
        if objective_by_id.insert(objective.id.as_str(), objective).is_some() {
            return Err(ComparisonError::DuplicateObjectiveId {
                objective_id: objective.id.clone(),
            });
        }
    }

    let mut seen_candidates = BTreeSet::new();
    for candidate in candidates {
        if candidate.id.trim().is_empty() {
            return Err(ComparisonError::EmptyCandidateId);
        }
        if !seen_candidates.insert(candidate.id.as_str()) {
            return Err(ComparisonError::DuplicateCandidateId {
                candidate_id: candidate.id.clone(),
            });
        }
        if candidate.gates.is_empty() {
            return Err(ComparisonError::NoHardGates {
                candidate_id: candidate.id.clone(),
            });
        }

        let mut gate_ids = BTreeSet::new();
        for gate in &candidate.gates {
            if gate.id.trim().is_empty() {
                return Err(ComparisonError::EmptyGateId {
                    candidate_id: candidate.id.clone(),
                });
            }
            if !gate_ids.insert(gate.id.as_str()) {
                return Err(ComparisonError::DuplicateGateId {
                    candidate_id: candidate.id.clone(),
                    gate_id: gate.id.clone(),
                });
            }
            match &gate.outcome {
                GateOutcome::Pass { evidence_ref } | GateOutcome::Fail { evidence_ref }
                    if evidence_ref.trim().is_empty() =>
                {
                    return Err(ComparisonError::MissingGateEvidence {
                        candidate_id: candidate.id.clone(),
                        gate_id: gate.id.clone(),
                    });
                }
                GateOutcome::Unknown { reason } if reason.trim().is_empty() => {
                    return Err(ComparisonError::EmptyUnknownReason {
                        candidate_id: candidate.id.clone(),
                        gate_id: gate.id.clone(),
                    });
                }
                _ => {}
            }
        }

        let mut value_ids = BTreeSet::new();
        for value in &candidate.values {
            if value.evidence_ref.trim().is_empty() {
                return Err(ComparisonError::EmptyValueEvidence {
                    candidate_id: candidate.id.clone(),
                    objective_id: value.objective_id.clone(),
                });
            }
            let Some(spec) = objective_by_id.get(value.objective_id.as_str()) else {
                return Err(ComparisonError::UnknownObjective {
                    candidate_id: candidate.id.clone(),
                    objective_id: value.objective_id.clone(),
                });
            };
            if !value_ids.insert(value.objective_id.as_str()) {
                return Err(ComparisonError::DuplicateObjectiveValue {
                    candidate_id: candidate.id.clone(),
                    objective_id: value.objective_id.clone(),
                });
            }
            if value.unit != spec.unit {
                return Err(ComparisonError::ObjectiveUnitMismatch {
                    candidate_id: candidate.id.clone(),
                    objective_id: value.objective_id.clone(),
                });
            }
            if value.scale != spec.scale {
                return Err(ComparisonError::ObjectiveScaleMismatch {
                    candidate_id: candidate.id.clone(),
                    objective_id: value.objective_id.clone(),
                });
            }
        }
    }

    let mut dispositions = Vec::with_capacity(candidates.len());
    let mut eligible: BTreeMap<&str, BTreeMap<&str, i64>> = BTreeMap::new();

    for candidate in candidates {
        let failed: Vec<String> = candidate
            .gates
            .iter()
            .filter_map(|gate| match &gate.outcome {
                GateOutcome::Fail { .. } => Some(gate.id.clone()),
                _ => None,
            })
            .collect();
        let unknown: Vec<String> = candidate
            .gates
            .iter()
            .filter_map(|gate| match &gate.outcome {
                GateOutcome::Unknown { .. } => Some(gate.id.clone()),
                _ => None,
            })
            .collect();

        let disposition = if !failed.is_empty() {
            Disposition::Blocked {
                failed_gate_ids: failed,
            }
        } else if !unknown.is_empty() {
            Disposition::Incomplete {
                unknown_gate_ids: unknown,
            }
        } else {
            for objective in objectives {
                if !candidate
                    .values
                    .iter()
                    .any(|value| value.objective_id == objective.id)
                {
                    return Err(ComparisonError::MissingObjectiveValue {
                        candidate_id: candidate.id.clone(),
                        objective_id: objective.id.clone(),
                    });
                }
            }

            let values = candidate
                .values
                .iter()
                .map(|value| (value.objective_id.as_str(), value.value_ticks))
                .collect();
            eligible.insert(candidate.id.as_str(), values);
            Disposition::Feasible
        };

        dispositions.push(CandidateDisposition {
            candidate_id: candidate.id.clone(),
            disposition,
        });
    }

    let feasible_ids: Vec<&str> = eligible.keys().copied().collect();
    let mut frontier = Vec::new();
    let mut dominated = Vec::new();

    for target_id in &feasible_ids {
        let mut dominators: Vec<&str> = feasible_ids
            .iter()
            .copied()
            .filter(|other_id| {
                other_id != target_id
                    && dominates(
                        eligible.get(other_id).expect("eligible key exists"),
                        eligible.get(target_id).expect("eligible key exists"),
                        objectives,
                    )
            })
            .collect();
        dominators.sort_unstable();

        if let Some(dominator_id) = dominators.first() {
            dominated.push(DominanceWitness {
                dominator_id: (*dominator_id).to_owned(),
                dominated_id: (*target_id).to_owned(),
            });
        } else {
            frontier.push((*target_id).to_owned());
        }
    }

    dispositions.sort_by(|a, b| a.candidate_id.cmp(&b.candidate_id));
    frontier.sort();
    dominated.sort();

    Ok(ComparisonReport {
        comparison_id: comparison_id.to_owned(),
        frontier,
        dominated,
        dispositions,
    })
}

fn dominates(
    left: &BTreeMap<&str, i64>,
    right: &BTreeMap<&str, i64>,
    objectives: &[ObjectiveSpec],
) -> bool {
    let mut strictly_better = false;

    for objective in objectives {
        let left_value = left[objective.id.as_str()];
        let right_value = right[objective.id.as_str()];
        let (no_worse, strictly) = match objective.direction {
            Direction::Minimize => (left_value <= right_value, left_value < right_value),
            Direction::Maximize => (left_value >= right_value, left_value > right_value),
        };
        if !no_worse {
            return false;
        }
        strictly_better |= strictly;
    }

    strictly_better
}

/// Non-overlapping mass fates at one explicitly declared process boundary.
///
/// All fields use integer milligrams. The fates represent mutually exclusive
/// destinations for this boundary; material sent to rework must not also be
/// counted as accepted output at the same boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MassBalance {
    pub input_mg: u64,
    pub accepted_output_mg: u64,
    pub sent_to_rework_mg: u64,
    pub recovered_secondary_mg: u64,
    pub scrap_mg: u64,
    pub measured_process_loss_mg: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MassBalanceError {
    ArithmeticOverflow,
    Unbalanced { input_mg: u64, accounted_mg: u64 },
}

impl fmt::Display for MassBalanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArithmeticOverflow => write!(f, "mass-balance arithmetic overflow"),
            Self::Unbalanced { input_mg, accounted_mg } => {
                write!(f, "mass balance mismatch: {input_mg} mg input, {accounted_mg} mg accounted")
            }
        }
    }
}

impl Error for MassBalanceError {}

impl MassBalance {
    pub fn verify(self) -> Result<(), MassBalanceError> {
        let accounted_mg = [
            self.accepted_output_mg,
            self.sent_to_rework_mg,
            self.recovered_secondary_mg,
            self.scrap_mg,
            self.measured_process_loss_mg,
        ]
        .into_iter()
        .try_fold(0_u64, u64::checked_add)
        .ok_or(MassBalanceError::ArithmeticOverflow)?;

        if accounted_mg == self.input_mg {
            Ok(())
        } else {
            Err(MassBalanceError::Unbalanced {
                input_mg: self.input_mg,
                accounted_mg,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn objectives() -> Vec<ObjectiveSpec> {
        vec![
            ObjectiveSpec {
                id: "energy".into(),
                unit: "kWh/functional-unit".into(),
                scale: 1_000,
                direction: Direction::Minimize,
            },
            ObjectiveSpec {
                id: "accepted_output".into(),
                unit: "functional-units/hour".into(),
                scale: 100,
                direction: Direction::Maximize,
            },
            ObjectiveSpec {
                id: "water".into(),
                unit: "L/functional-unit".into(),
                scale: 1_000,
                direction: Direction::Minimize,
            },
        ]
    }

    fn candidate(id: &str, values: [i64; 3]) -> CandidateEvaluation {
        let specs = objectives();
        CandidateEvaluation {
            id: id.into(),
            gates: vec![GateCheck {
                id: "product_quality".into(),
                outcome: GateOutcome::Pass {
                    evidence_ref: format!("fixture:{id}:quality"),
                },
            }],
            values: specs
                .iter()
                .zip(values)
                .map(|(spec, value_ticks)| ObjectiveValue {
                    objective_id: spec.id.clone(),
                    value_ticks,
                    unit: spec.unit.clone(),
                    scale: spec.scale,
                    evidence_ref: format!("fixture:{id}:{}", spec.id),
                })
                .collect(),
        }
    }

    #[test]
    fn pareto_frontier_preserves_tradeoffs_and_emits_dominance_witness() {
        let report = compare_candidates(
            "fixture-1",
            &objectives(),
            &[
                candidate("balanced", [100, 100, 100]),
                candidate("fast", [130, 140, 120]),
                candidate("dominated", [150, 90, 130]),
            ],
        )
        .unwrap();

        assert_eq!(report.frontier, vec!["balanced", "fast"]);
        assert_eq!(
            report.dominated,
            vec![DominanceWitness {
                dominator_id: "balanced".into(),
                dominated_id: "dominated".into(),
            }]
        );
    }

    #[test]
    fn hard_failure_never_enters_frontier_even_with_best_objectives() {
        let mut blocked = candidate("best-looking", [1, 999, 1]);
        blocked.gates.push(GateCheck {
            id: "effluent_admission".into(),
            outcome: GateOutcome::Fail {
                evidence_ref: "fixture:failed-effluent-check".into(),
            },
        });

        let report = compare_candidates(
            "fixture-2",
            &objectives(),
            &[blocked, candidate("admissible", [10, 100, 10])],
        )
        .unwrap();

        assert_eq!(report.frontier, vec!["admissible"]);
        let blocked = report
            .dispositions
            .iter()
            .find(|item| item.candidate_id == "best-looking")
            .expect("blocked candidate is retained in the report");
        assert_eq!(
            &blocked.disposition,
            &Disposition::Blocked {
                failed_gate_ids: vec!["effluent_admission".into()]
            }
        );
    }

    #[test]
    fn unknown_gate_makes_candidate_incomplete_not_feasible() {
        let mut candidate = candidate("unknown", [1, 999, 1]);
        candidate.gates.push(GateCheck {
            id: "water_measurement".into(),
            outcome: GateOutcome::Unknown {
                reason: "meter calibration evidence missing".into(),
            },
        });
        let report =
            compare_candidates("fixture-3", &objectives(), &[candidate]).unwrap();

        assert!(report.frontier.is_empty());
        assert_eq!(
            report.dispositions[0].disposition,
            Disposition::Incomplete {
                unknown_gate_ids: vec!["water_measurement".into()]
            }
        );
    }

    #[test]
    fn unit_mismatch_invalidates_the_comparison() {
        let mut bad = candidate("bad-unit", [100, 100, 100]);
        bad.values[0].unit = "MJ/functional-unit".into();
        assert_eq!(
            compare_candidates("fixture-4", &objectives(), &[bad]),
            Err(ComparisonError::ObjectiveUnitMismatch {
                candidate_id: "bad-unit".into(),
                objective_id: "energy".into(),
            })
        );
    }

    #[test]
    fn feasible_candidate_must_have_every_objective() {
        let mut missing = candidate("missing", [100, 100, 100]);
        missing.values.pop();
        assert_eq!(
            compare_candidates("fixture-5", &objectives(), &[missing]),
            Err(ComparisonError::MissingObjectiveValue {
                candidate_id: "missing".into(),
                objective_id: "water".into(),
            })
        );
    }

    #[test]
    fn shuffled_candidate_input_has_identical_report() {
        let a = candidate("a", [100, 100, 100]);
        let b = candidate("b", [110, 100, 100]);
        let c = candidate("c", [80, 90, 80]);
        let left = compare_candidates(
            "fixture-6",
            &objectives(),
            &[a.clone(), b.clone(), c.clone()],
        )
            .unwrap();
        let right = compare_candidates("fixture-6", &objectives(), &[c, a, b]).unwrap();
        assert_eq!(left, right);
    }

    #[test]
    fn mass_balance_requires_exact_closure() {
        let balanced = MassBalance {
            input_mg: 1_000_000,
            accepted_output_mg: 800_000,
            sent_to_rework_mg: 80_000,
            recovered_secondary_mg: 50_000,
            scrap_mg: 40_000,
            measured_process_loss_mg: 30_000,
        };
        assert_eq!(balanced.verify(), Ok(()));

        let unbalanced = MassBalance {
            scrap_mg: 39_999,
            ..balanced
        };
        assert_eq!(
            unbalanced.verify(),
            Err(MassBalanceError::Unbalanced {
                input_mg: 1_000_000,
                accounted_mg: 999_999
            })
        );
    }

    #[test]
    fn arithmetic_overflow_is_not_treated_as_balanced() {
        let overflowing = MassBalance {
            input_mg: u64::MAX,
            accepted_output_mg: u64::MAX,
            sent_to_rework_mg: 1,
            recovered_secondary_mg: 0,
            scrap_mg: 0,
            measured_process_loss_mg: 0,
        };
        assert_eq!(overflowing.verify(), Err(MassBalanceError::ArithmeticOverflow));
    }

    #[test]
    fn duplicate_candidate_ids_are_rejected() {
        assert_eq!(
            compare_candidates(
                "fixture-7",
                &objectives(),
                &[candidate("same", [1, 1, 1]), candidate("same", [2, 2, 2])]
            ),
            Err(ComparisonError::DuplicateCandidateId {
                candidate_id: "same".into()
            })
        );
    }

    #[test]
    fn max_objective_direction_is_respected() {
        let specs = vec![
            ObjectiveSpec {
                id: "quality".into(),
                unit: "conformance-points".into(),
                scale: 1,
                direction: Direction::Maximize,
            },
            ObjectiveSpec {
                id: "energy".into(),
                unit: "kWh/unit".into(),
                scale: 1,
                direction: Direction::Minimize,
            },
        ];
        let make = |id: &str, quality: i64, energy: i64| CandidateEvaluation {
            id: id.into(),
            gates: vec![GateCheck {
                id: "hard-quality".into(),
                outcome: GateOutcome::Pass {
                    evidence_ref: format!("fixture:{id}:hard-quality"),
                },
            }],
            values: vec![
                ObjectiveValue {
                    objective_id: "quality".into(),
                    value_ticks: quality,
                    unit: "conformance-points".into(),
                    scale: 1,
                    evidence_ref: format!("fixture:{id}:quality"),
                },
                ObjectiveValue {
                    objective_id: "energy".into(),
                    value_ticks: energy,
                    unit: "kWh/unit".into(),
                    scale: 1,
                    evidence_ref: format!("fixture:{id}:energy"),
                },
            ],
        };
        let report = compare_candidates(
            "fixture-8",
            &specs,
            &[make("winner", 100, 10), make("worse", 90, 11)],
        )
        .unwrap();
        assert_eq!(report.frontier, vec!["winner"]);
        assert_eq!(
            report.dominated,
            vec![DominanceWitness {
                dominator_id: "winner".into(),
                dominated_id: "worse".into(),
            }]
        );
    }

    #[test]
    fn equal_objective_vectors_remain_distinct_frontier_alternatives() {
        let report = compare_candidates(
            "fixture-equal-vectors",
            &objectives(),
            &[
                candidate("alternative-a", [100, 100, 100]),
                candidate("alternative-b", [100, 100, 100]),
            ],
        )
        .unwrap();

        assert_eq!(report.frontier, vec!["alternative-a", "alternative-b"]);
        assert!(report.dominated.is_empty());
    }

    #[test]
    fn seeded_frontiers_match_independent_exhaustive_oracle() {
        let specs = vec![
            ObjectiveSpec {
                id: "energy".into(),
                unit: "kWh/unit".into(),
                scale: 100,
                direction: Direction::Minimize,
            },
            ObjectiveSpec {
                id: "throughput".into(),
                unit: "units/hour".into(),
                scale: 10,
                direction: Direction::Maximize,
            },
            ObjectiveSpec {
                id: "water".into(),
                unit: "L/unit".into(),
                scale: 100,
                direction: Direction::Minimize,
            },
            ObjectiveSpec {
                id: "yield".into(),
                unit: "fraction".into(),
                scale: 10_000,
                direction: Direction::Maximize,
            },
        ];

        // Fixed LCG: no external RNG dependency and identical corpus on every run.
        let mut state = 0x5eed_cafe_u64;
        for scenario in 0..100 {
            let vectors: Vec<[i64; 4]> = (0..16)
                .map(|_| {
                    std::array::from_fn(|_| {
                        state = state
                            .wrapping_mul(6_364_136_223_846_793_005)
                            .wrapping_add(1_442_695_040_888_963_407);
                        ((state >> 32) % 101) as i64
                    })
                })
                .collect();

            let candidates: Vec<CandidateEvaluation> = vectors
                .iter()
                .enumerate()
                .map(|(index, vector)| CandidateEvaluation {
                    id: format!("scenario-{scenario:03}-candidate-{index:02}"),
                    gates: vec![GateCheck {
                        id: "product-quality".into(),
                        outcome: GateOutcome::Pass {
                            evidence_ref: format!("oracle:scenario-{scenario}:quality-{index}"),
                        },
                    }],
                    values: specs
                        .iter()
                        .zip(vector)
                        .map(|(spec, value_ticks)| ObjectiveValue {
                            objective_id: spec.id.clone(),
                            value_ticks: *value_ticks,
                            unit: spec.unit.clone(),
                            scale: spec.scale,
                            evidence_ref: format!(
                                "oracle:scenario-{scenario}:candidate-{index}:{}",
                                spec.id
                            ),
                        })
                        .collect(),
                })
                .collect();

            // Independent exhaustive oracle: operate directly on [i64; 4] vectors
            // and explicit directions rather than calling production `dominates`.
            let oracle_dominates = |left: &[i64; 4], right: &[i64; 4]| {
                let mut strictly_better = false;
                for dimension in 0..4 {
                    let (no_worse, strictly) = match specs[dimension].direction {
                        Direction::Minimize => (
                            left[dimension] <= right[dimension],
                            left[dimension] < right[dimension],
                        ),
                        Direction::Maximize => (
                            left[dimension] >= right[dimension],
                            left[dimension] > right[dimension],
                        ),
                    };
                    if !no_worse {
                        return false;
                    }
                    strictly_better |= strictly;
                }
                strictly_better
            };

            let mut expected_frontier = Vec::new();
            let mut expected_dominated = Vec::new();
            for (target_index, target) in vectors.iter().enumerate() {
                let target_id = format!("scenario-{scenario:03}-candidate-{target_index:02}");
                let mut witnesses: Vec<String> = vectors
                    .iter()
                    .enumerate()
                    .filter(|(other_index, _)| *other_index != target_index)
                    .filter(|(_, other)| oracle_dominates(other, target))
                    .map(|(other_index, _)| {
                        format!("scenario-{scenario:03}-candidate-{other_index:02}")
                    })
                    .collect();
                witnesses.sort();

                if let Some(dominator_id) = witnesses.first() {
                    expected_dominated.push(DominanceWitness {
                        dominator_id: dominator_id.clone(),
                        dominated_id: target_id,
                    });
                } else {
                    expected_frontier.push(target_id);
                }
            }
            expected_frontier.sort();
            expected_dominated.sort();

            let actual = compare_candidates(
                &format!("oracle-scenario-{scenario:03}"),
                &specs,
                &candidates,
            )
            .unwrap();

            assert_eq!(actual.frontier, expected_frontier, "scenario {scenario}");
            assert_eq!(actual.dominated, expected_dominated, "scenario {scenario}");
            assert_eq!(
                actual.comparison_id,
                format!("oracle-scenario-{scenario:03}")
            );
        }
    }

    #[test]
    fn empty_hard_gates_are_not_implicitly_admitted() {
        let mut missing_gate = candidate("no-gates", [100, 100, 100]);
        missing_gate.gates.clear();
        assert_eq!(
            compare_candidates("fixture-9", &objectives(), &[missing_gate]),
            Err(ComparisonError::NoHardGates {
                candidate_id: "no-gates".into()
            })
        );
    }
}
