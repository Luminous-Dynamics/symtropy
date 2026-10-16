// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic robust path selection over paired, evidence-bounded scenarios.
//!
//! Every candidate path is evaluated over the same frozen scenario set. The
//! selector reports both the nominal-fastest candidate and the minimax candidate
//! that minimizes its worst arrival time across the declared finite scenario set.
//! Scenarios are not probability-weighted: stress cases must never be presented
//! as frequency estimates or empirical likelihoods.
//!
//! This is exact only over the caller-supplied candidate paths and scenario set.
//! It does not prove candidate-path completeness, estimate real-world scenario
//! probabilities, authorize dispatch, or establish empirical generalization.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use super::{
    FifoPathEvaluationError, FifoPathEstimateV1, FifoPathLegV1,
    evaluate_fifo_path,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FifoPathScenarioKindV1 {
    /// The declared reference conditions. Exactly one scenario set member must use this kind.
    Nominal,
    /// A deterministic adversarial/stress condition; no probability is implied.
    Stress,
    /// A recorded event replay admitted with its own evidence manifest.
    ObservedReplay,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FifoPathScenarioDefinitionV1 {
    pub scenario_id: String,
    pub kind: FifoPathScenarioKindV1,
}

/// Immutable caller-declared scenario inventory. The digest binds the external
/// manifest; this module validates its shape but does not recompute source bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FifoPathScenarioSetV1 {
    pub scenario_set_id: String,
    pub manifest_sha256: String,
    pub time_profile_id: String,
    pub scenarios: Vec<FifoPathScenarioDefinitionV1>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FifoScenarioCandidatePathV1 {
    pub scenario_id: String,
    pub legs: Vec<FifoPathLegV1>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FifoRobustCandidatePathV1 {
    pub path_id: String,
    /// Same physical/logical path topology under each scenario, but edge
    /// arrival functions and explicitly modeled delays may vary by scenario.
    pub scenario_paths: Vec<FifoScenarioCandidatePathV1>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FifoRobustSelectionError {
    InvalidScenarioSet(&'static str),
    DuplicateScenarioId(String),
    DuplicateCandidatePathId(String),
    EmptyCandidatePaths,
    EmptyCandidatePathId,
    DuplicateCandidateScenario { path_id: String, scenario_id: String },
    MissingCandidateScenario { path_id: String, scenario_id: String },
    UnexpectedCandidateScenario { path_id: String, scenario_id: String },
    TopologyChangedAcrossScenarios { path_id: String, scenario_id: String },
    CandidateEndpointsDiffer {
        path_id: String,
        expected_origin_node_id: String,
        expected_destination_node_id: String,
        actual_origin_node_id: String,
        actual_destination_node_id: String,
    },
    NoQualifiedCandidatePath,
    PathEvaluation {
        path_id: String,
        scenario_id: String,
        source: FifoPathEvaluationError,
    },
}

impl fmt::Display for FifoRobustSelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidScenarioSet(reason) => {
                write!(formatter, "invalid FIFO path scenario set: {reason}")
            }
            Self::DuplicateScenarioId(id) => write!(formatter, "duplicate scenario ID {id}"),
            Self::DuplicateCandidatePathId(id) => {
                write!(formatter, "duplicate candidate path ID {id}")
            }
            Self::EmptyCandidatePaths => formatter.write_str("candidate path set is empty"),
            Self::EmptyCandidatePathId => formatter.write_str("candidate path ID is blank"),
            Self::DuplicateCandidateScenario { path_id, scenario_id } => {
                write!(formatter, "candidate path {path_id} repeats scenario {scenario_id}")
            }
            Self::MissingCandidateScenario { path_id, scenario_id } => {
                write!(formatter, "candidate path {path_id} omits scenario {scenario_id}")
            }
            Self::UnexpectedCandidateScenario { path_id, scenario_id } => {
                write!(formatter, "candidate path {path_id} includes undeclared scenario {scenario_id}")
            }
            Self::TopologyChangedAcrossScenarios { path_id, scenario_id } => write!(
                formatter,
                "candidate path {path_id} changes edge/node topology under scenario {scenario_id}"
            ),
            Self::CandidateEndpointsDiffer {
                path_id,
                expected_origin_node_id,
                expected_destination_node_id,
                actual_origin_node_id,
                actual_destination_node_id,
            } => write!(
                formatter,
                "candidate path {path_id} connects {actual_origin_node_id} to {actual_destination_node_id}, expected {expected_origin_node_id} to {expected_destination_node_id}"
            ),
            Self::NoQualifiedCandidatePath => {
                formatter.write_str("no candidate is feasible and evidence-qualified across every scenario")
            }
            Self::PathEvaluation { path_id, scenario_id, source } => {
                write!(formatter, "candidate path {path_id} failed in scenario {scenario_id}: {source}")
            }
        }
    }
}

impl std::error::Error for FifoRobustSelectionError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FifoScenarioPathObservationV1 {
    pub scenario_id: String,
    pub kind: FifoPathScenarioKindV1,
    pub final_arrival_epoch_seconds: u64,
    pub elapsed_seconds: u64,
    pub path_estimate: FifoPathEstimateV1,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FifoRobustPathScoreV1 {
    pub path_id: String,
    pub nominal_arrival_epoch_seconds: u64,
    pub worst_case_arrival_epoch_seconds: u64,
    pub worst_case_scenario_id: String,
    pub observations: Vec<FifoScenarioPathObservationV1>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FifoRejectedPathScenarioV1 {
    pub path_id: String,
    pub scenario_id: String,
    pub reason: FifoPathEvaluationError,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FifoRobustPathSelectionReportV1 {
    pub scenario_set_id: String,
    pub scenario_manifest_sha256: String,
    pub time_profile_id: String,
    pub initial_departure_epoch_seconds: u64,
    pub knowledge_cutoff_epoch_seconds: u64,
    /// Fastest path under the single scenario explicitly marked Nominal.
    pub nominal_best_path_id: String,
    /// Minimax path: minimum worst-case arrival among qualified supplied paths.
    pub robust_best_path_id: String,
    /// Explicitly limited to candidate paths supplied to this function.
    pub optimality_scope: &'static str,
    pub candidate_path_count: usize,
    pub qualified_path_count: usize,
    pub scores: Vec<FifoRobustPathScoreV1>,
    /// Failed candidates remain visible; they are not silently dropped.
    pub rejected_path_scenarios: Vec<FifoRejectedPathScenarioV1>,
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_scenario_set(
    scenario_set: &FifoPathScenarioSetV1,
) -> Result<(BTreeMap<String, FifoPathScenarioKindV1>, String), FifoRobustSelectionError> {
    if scenario_set.scenario_set_id.trim().is_empty() {
        return Err(FifoRobustSelectionError::InvalidScenarioSet(
            "scenario-set ID is blank",
        ));
    }
    if !valid_sha256(&scenario_set.manifest_sha256) {
        return Err(FifoRobustSelectionError::InvalidScenarioSet(
            "scenario manifest must provide a 64-character SHA-256 digest",
        ));
    }
    if scenario_set.time_profile_id.trim().is_empty() {
        return Err(FifoRobustSelectionError::InvalidScenarioSet(
            "time profile ID is blank",
        ));
    }
    if scenario_set.scenarios.is_empty() {
        return Err(FifoRobustSelectionError::InvalidScenarioSet(
            "scenario inventory is empty",
        ));
    }

    let mut scenarios = BTreeMap::new();
    let mut nominal_count = 0_usize;
    for scenario in &scenario_set.scenarios {
        if scenario.scenario_id.trim().is_empty() {
            return Err(FifoRobustSelectionError::InvalidScenarioSet(
                "scenario ID is blank",
            ));
        }
        if scenario.kind == FifoPathScenarioKindV1::Nominal {
            nominal_count += 1;
        }
        if scenarios
            .insert(scenario.scenario_id.clone(), scenario.kind)
            .is_some()
        {
            return Err(FifoRobustSelectionError::DuplicateScenarioId(
                scenario.scenario_id.clone(),
            ));
        }
    }
    if nominal_count != 1 {
        return Err(FifoRobustSelectionError::InvalidScenarioSet(
            "exactly one scenario must be marked Nominal",
        ));
    }

    let nominal_id = scenarios
        .iter()
        .find_map(|(id, kind)| (*kind == FifoPathScenarioKindV1::Nominal).then(|| id.clone()))
        .ok_or(FifoRobustSelectionError::InvalidScenarioSet(
            "nominal scenario is missing",
        ))?;
    Ok((scenarios, nominal_id))
}

fn path_topology(legs: &[FifoPathLegV1]) -> Vec<(String, String, String)> {
    legs.iter()
        .map(|leg| (
            leg.edge_id.clone(),
            leg.from_node_id.clone(),
            leg.to_node_id.clone(),
        ))
        .collect()
}

/// Evaluate and select a robust path over a finite, paired scenario set.
///
/// The nominal choice minimizes the nominal scenario's final arrival. The
/// robust choice minimizes worst-case final arrival across all declared
/// scenarios, including Nominal, Stress, and ObservedReplay. Ties are broken
/// lexicographically by stable path ID. No scenario frequencies or probabilities
/// are inferred. A path is qualified only if every scenario path evaluates
/// successfully; rejected path/scenario pairs remain in the report.
///
/// Exactness means exact minimax selection over the supplied paths and scenarios
/// only. An omitted path or an incomplete scenario set can change the answer.
pub fn select_robust_fifo_path(
    scenario_set: &FifoPathScenarioSetV1,
    candidate_paths: &[FifoRobustCandidatePathV1],
    initial_departure_epoch_seconds: u64,
    knowledge_cutoff_epoch_seconds: u64,
) -> Result<FifoRobustPathSelectionReportV1, FifoRobustSelectionError> {
    let (scenario_kinds, nominal_scenario_id) = validate_scenario_set(scenario_set)?;
    if candidate_paths.is_empty() {
        return Err(FifoRobustSelectionError::EmptyCandidatePaths);
    }

    let mut candidate_ids = BTreeSet::new();
    for path in candidate_paths {
        if path.path_id.trim().is_empty() {
            return Err(FifoRobustSelectionError::EmptyCandidatePathId);
        }
        if !candidate_ids.insert(path.path_id.clone()) {
            return Err(FifoRobustSelectionError::DuplicateCandidatePathId(
                path.path_id.clone(),
            ));
        }
    }

    let mut sorted_paths = candidate_paths.iter().collect::<Vec<_>>();
    sorted_paths.sort_by(|a, b| a.path_id.cmp(&b.path_id));

    let mut scores = Vec::new();
    let mut rejected_path_scenarios = Vec::new();
    let mut common_endpoints: Option<(String, String)> = None;

    for candidate in sorted_paths {
        let mut candidate_by_scenario = BTreeMap::new();
        for scenario_path in &candidate.scenario_paths {
            if !scenario_kinds.contains_key(&scenario_path.scenario_id) {
                return Err(FifoRobustSelectionError::UnexpectedCandidateScenario {
                    path_id: candidate.path_id.clone(),
                    scenario_id: scenario_path.scenario_id.clone(),
                });
            }
            if candidate_by_scenario
                .insert(scenario_path.scenario_id.clone(), scenario_path)
                .is_some()
            {
                return Err(FifoRobustSelectionError::DuplicateCandidateScenario {
                    path_id: candidate.path_id.clone(),
                    scenario_id: scenario_path.scenario_id.clone(),
                });
            }
        }

        for scenario_id in scenario_kinds.keys() {
            if !candidate_by_scenario.contains_key(scenario_id) {
                return Err(FifoRobustSelectionError::MissingCandidateScenario {
                    path_id: candidate.path_id.clone(),
                    scenario_id: scenario_id.clone(),
                });
            }
        }

        let mut previous_topology: Option<Vec<(String, String, String)>> = None;
        for scenario_id in scenario_kinds.keys() {
            let scenario_path = candidate_by_scenario[scenario_id];
            let topology = path_topology(&scenario_path.legs);
            if let Some(expected) = &previous_topology {
                if &topology != expected {
                    return Err(FifoRobustSelectionError::TopologyChangedAcrossScenarios {
                        path_id: candidate.path_id.clone(),
                        scenario_id: scenario_id.clone(),
                    });
                }
            } else {
                previous_topology = Some(topology);
            }
        }

        // Candidate paths are compared for one common OD pair. Without this
        // check a "robust" result could merely terminate at a closer destination.
        if let Some(first_scenario_id) = scenario_kinds.keys().next() {
            let first_path = candidate_by_scenario[first_scenario_id];
            if let (Some(first_leg), Some(last_leg)) =
                (first_path.legs.first(), first_path.legs.last())
            {
                let actual = (first_leg.from_node_id.clone(), last_leg.to_node_id.clone());
                if let Some((expected_origin, expected_destination)) = &common_endpoints {
                    if actual.0.as_str() != expected_origin.as_str()
                        || actual.1.as_str() != expected_destination.as_str()
                    {
                        return Err(FifoRobustSelectionError::CandidateEndpointsDiffer {
                            path_id: candidate.path_id.clone(),
                            expected_origin_node_id: expected_origin.clone(),
                            expected_destination_node_id: expected_destination.clone(),
                            actual_origin_node_id: actual.0,
                            actual_destination_node_id: actual.1,
                        });
                    }
                } else {
                    common_endpoints = Some(actual);
                }
            }
        }

        let mut observations = Vec::new();
        for (scenario_id, kind) in &scenario_kinds {
            let scenario_path = candidate_by_scenario[scenario_id];
            match evaluate_fifo_path(
                &scenario_path.legs,
                initial_departure_epoch_seconds,
                knowledge_cutoff_epoch_seconds,
                &scenario_set.time_profile_id,
            ) {
                Ok(path_estimate) => observations.push(FifoScenarioPathObservationV1 {
                    scenario_id: scenario_id.clone(),
                    kind: *kind,
                    final_arrival_epoch_seconds: path_estimate.final_arrival_epoch_seconds,
                    elapsed_seconds: path_estimate.total_elapsed_seconds,
                    path_estimate,
                }),
                Err(source) => rejected_path_scenarios.push(FifoRejectedPathScenarioV1 {
                    path_id: candidate.path_id.clone(),
                    scenario_id: scenario_id.clone(),
                    reason: source,
                }),
            }
        }

        if observations.len() != scenario_kinds.len() {
            continue;
        }

        let nominal_arrival_epoch_seconds = observations
            .iter()
            .find(|observation| observation.scenario_id == nominal_scenario_id)
            .map(|observation| observation.final_arrival_epoch_seconds)
            .ok_or(FifoRobustSelectionError::InvalidScenarioSet(
                "evaluated nominal scenario is missing",
            ))?;

        let worst = observations
            .iter()
            .max_by(|a, b| {
                a.final_arrival_epoch_seconds
                    .cmp(&b.final_arrival_epoch_seconds)
                    // On equal arrivals, lexicographically smaller scenario ID wins.
                    .then_with(|| b.scenario_id.cmp(&a.scenario_id))
            })
            .ok_or(FifoRobustSelectionError::InvalidScenarioSet(
                "candidate has no scenario observations",
            ))?;

        let worst_case_arrival_epoch_seconds = worst.final_arrival_epoch_seconds;
        let worst_case_scenario_id = worst.scenario_id.clone();
        observations.sort_by(|a, b| a.scenario_id.cmp(&b.scenario_id));
        scores.push(FifoRobustPathScoreV1 {
            path_id: candidate.path_id.clone(),
            nominal_arrival_epoch_seconds,
            worst_case_arrival_epoch_seconds,
            worst_case_scenario_id,
            observations,
        });
    }

    if scores.is_empty() {
        return Err(FifoRobustSelectionError::NoQualifiedCandidatePath);
    }
    scores.sort_by(|a, b| a.path_id.cmp(&b.path_id));

    let nominal_best_path_id = scores
        .iter()
        .min_by(|a, b| {
            a.nominal_arrival_epoch_seconds
                .cmp(&b.nominal_arrival_epoch_seconds)
                .then_with(|| a.path_id.cmp(&b.path_id))
        })
        .map(|score| score.path_id.clone())
        .ok_or(FifoRobustSelectionError::NoQualifiedCandidatePath)?;

    let robust_best_path_id = scores
        .iter()
        .min_by(|a, b| {
            a.worst_case_arrival_epoch_seconds
                .cmp(&b.worst_case_arrival_epoch_seconds)
                .then_with(|| a.path_id.cmp(&b.path_id))
        })
        .map(|score| score.path_id.clone())
        .ok_or(FifoRobustSelectionError::NoQualifiedCandidatePath)?;

    rejected_path_scenarios.sort_by(|a, b| {
        (&a.path_id, &a.scenario_id).cmp(&(&b.path_id, &b.scenario_id))
    });

    Ok(FifoRobustPathSelectionReportV1 {
        scenario_set_id: scenario_set.scenario_set_id.clone(),
        scenario_manifest_sha256: scenario_set.manifest_sha256.clone(),
        time_profile_id: scenario_set.time_profile_id.clone(),
        initial_departure_epoch_seconds,
        knowledge_cutoff_epoch_seconds,
        nominal_best_path_id,
        robust_best_path_id,
        optimality_scope: "ExactMinimaxOverSuppliedCandidatePathsAndDeclaredScenarios",
        candidate_path_count: candidate_paths.len(),
        qualified_path_count: scores.len(),
        scores,
        rejected_path_scenarios,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ArrivalFunctionEvidenceStatusV1, ArrivalFunctionEvidenceV1,
        ArrivalTimeBreakpointV1, FifoArrivalTimeFunctionV1,
    };

    const PROFILE: &str = "UTC_UNIX_SECONDS_V1";

    fn function(
        function_id: &str,
        base_travel: u64,
        knowledge_time: u64,
    ) -> FifoArrivalTimeFunctionV1 {
        let departure_start = 100;
        let departure_end = 10_000;
        FifoArrivalTimeFunctionV1::new(
            ArrivalFunctionEvidenceV1 {
                function_id: function_id.into(),
                time_profile_id: PROFILE.into(),
                source_record_id: format!("source-{function_id}"),
                source_record_sha256: Some("b".repeat(64)),
                source_published_at_epoch_seconds: Some(knowledge_time),
                repository_recorded_at_epoch_seconds: knowledge_time,
                source_observed_at_epoch_seconds: knowledge_time,
                valid_from_epoch_seconds: departure_start,
                valid_until_epoch_seconds: departure_end,
                source_locator: Some(format!("manifest.json#/functions/{function_id}")),
                status: ArrivalFunctionEvidenceStatusV1::Qualified,
            },
            vec![
                ArrivalTimeBreakpointV1 {
                    departure_epoch_seconds: departure_start,
                    arrival_epoch_seconds: departure_start + base_travel,
                },
                ArrivalTimeBreakpointV1 {
                    departure_epoch_seconds: departure_end - 1,
                    arrival_epoch_seconds: departure_end - 1 + base_travel,
                },
            ],
        )
        .expect("valid synthetic FIFO function")
    }

    fn leg(edge: &str, from: &str, to: &str, function: FifoArrivalTimeFunctionV1) -> FifoPathLegV1 {
        FifoPathLegV1 {
            edge_id: edge.into(),
            from_node_id: from.into(),
            to_node_id: to.into(),
            pre_leg_delay_seconds: 0,
            arrival_function: function,
        }
    }

    fn scenario_set() -> FifoPathScenarioSetV1 {
        FifoPathScenarioSetV1 {
            scenario_set_id: "toy-stress-set-v1".into(),
            manifest_sha256: "c".repeat(64),
            time_profile_id: PROFILE.into(),
            scenarios: vec![
                FifoPathScenarioDefinitionV1 {
                    scenario_id: "nominal".into(),
                    kind: FifoPathScenarioKindV1::Nominal,
                },
                FifoPathScenarioDefinitionV1 {
                    scenario_id: "corridor-a-disrupted".into(),
                    kind: FifoPathScenarioKindV1::Stress,
                },
                FifoPathScenarioDefinitionV1 {
                    scenario_id: "corridor-b-disrupted".into(),
                    kind: FifoPathScenarioKindV1::Stress,
                },
            ],
        }
    }

    fn candidate(
        path_id: &str,
        nominal_travel: u64,
        a_disrupted_travel: u64,
        b_disrupted_travel: u64,
        knowledge_time: u64,
    ) -> FifoRobustCandidatePathV1 {
        let edge = format!("corridor-{}", path_id.to_ascii_lowercase());
        let (from, to) = ("origin", "destination");
        FifoRobustCandidatePathV1 {
            path_id: path_id.into(),
            scenario_paths: vec![
                FifoScenarioCandidatePathV1 {
                    scenario_id: "nominal".into(),
                    legs: vec![leg(&edge, from, to, function(
                        &format!("{path_id}-nominal"), nominal_travel, knowledge_time
                    ))],
                },
                FifoScenarioCandidatePathV1 {
                    scenario_id: "corridor-a-disrupted".into(),
                    legs: vec![leg(&edge, from, to, function(
                        &format!("{path_id}-a-disrupted"), a_disrupted_travel, knowledge_time
                    ))],
                },
                FifoScenarioCandidatePathV1 {
                    scenario_id: "corridor-b-disrupted".into(),
                    legs: vec![leg(&edge, from, to, function(
                        &format!("{path_id}-b-disrupted"), b_disrupted_travel, knowledge_time
                    ))],
                },
            ],
        }
    }

    #[test]
    fn minimax_selection_can_prefer_a_robust_route_to_the_nominal_fastest_route() {
        // Path A wins nominally but collapses under the A-corridor outage.
        // Path B is slightly slower nominally and has the better worst case.
        let candidates = vec![
            candidate("A", 100, 500, 120, 50),
            candidate("B", 130, 180, 200, 50),
        ];
        let report = select_robust_fifo_path(&scenario_set(), &candidates, 100, 50).unwrap();

        assert_eq!(report.nominal_best_path_id, "A");
        assert_eq!(report.robust_best_path_id, "B");
        assert_eq!(report.candidate_path_count, 2);
        assert_eq!(report.qualified_path_count, 2);
        let a = report.scores.iter().find(|score| score.path_id == "A").unwrap();
        let b = report.scores.iter().find(|score| score.path_id == "B").unwrap();
        assert_eq!(a.worst_case_arrival_epoch_seconds, 600);
        assert_eq!(b.worst_case_arrival_epoch_seconds, 300);
        assert_eq!(a.worst_case_scenario_id, "corridor-a-disrupted");
    }

    #[test]
    fn candidate_and_scenario_input_order_do_not_change_the_selection() {
        let forward_candidates = vec![
            candidate("A", 100, 500, 120, 50),
            candidate("B", 130, 180, 200, 50),
        ];
        let mut reversed_candidates = forward_candidates.clone();
        reversed_candidates.reverse();
        let first = select_robust_fifo_path(&scenario_set(), &forward_candidates, 100, 50).unwrap();

        let mut reversed_set = scenario_set();
        reversed_set.scenarios.reverse();
        let second = select_robust_fifo_path(&reversed_set, &reversed_candidates, 100, 50).unwrap();

        assert_eq!(first, second);
    }

    #[test]
    fn missing_or_undeclared_scenarios_cannot_enter_a_robust_comparison() {
        let mut candidate = candidate("A", 100, 500, 120, 50);
        candidate.scenario_paths.pop();
        assert!(matches!(
            select_robust_fifo_path(&scenario_set(), &[candidate], 100, 50),
            Err(FifoRobustSelectionError::MissingCandidateScenario { .. })
        ));

        let mut candidate = candidate("A", 100, 500, 120, 50);
        candidate.scenario_paths.push(FifoScenarioCandidatePathV1 {
            scenario_id: "secret-fourth-scenario".into(),
            legs: vec![leg("a", "origin", "destination", function("fourth", 1, 50))],
        });
        assert!(matches!(
            select_robust_fifo_path(&scenario_set(), &[candidate], 100, 50),
            Err(FifoRobustSelectionError::UnexpectedCandidateScenario { .. })
        ));
    }

    #[test]
    fn candidates_that_fail_any_scenario_are_retained_as_rejections() {
        let mut candidate = candidate("A", 100, 500, 120, 50);
        let stress = candidate.scenario_paths.iter_mut()
            .find(|scenario| scenario.scenario_id == "corridor-a-disrupted")
            .unwrap();
        stress.legs[0].arrival_function = function("late-observed-profile", 1, 75);

        let robust = candidate("B", 130, 180, 200, 50);
        let report = select_robust_fifo_path(&scenario_set(), &[candidate, robust], 100, 50).unwrap();

        assert_eq!(report.qualified_path_count, 1);
        assert_eq!(report.robust_best_path_id, "B");
        assert_eq!(report.rejected_path_scenarios.len(), 1);
        assert_eq!(report.rejected_path_scenarios[0].path_id, "A");
        assert_eq!(report.rejected_path_scenarios[0].scenario_id, "corridor-a-disrupted");
        assert_eq!(
            report.rejected_path_scenarios[0].reason,
            FifoPathEvaluationError::ArrivalFunction {
                edge_id: "corridor-a".into(),
                source: ArrivalFunctionError::NotKnownAtCutoff,
            }
        );
    }

    #[test]
    fn all_scenario_paths_must_share_the_same_topology() {
        let mut candidate = candidate("A", 100, 500, 120, 50);
        let second = candidate.scenario_paths.iter_mut()
            .find(|scenario| scenario.scenario_id == "corridor-a-disrupted")
            .unwrap();
        second.legs[0].edge_id = "different-edge".into();

        assert!(matches!(
            select_robust_fifo_path(&scenario_set(), &[candidate], 100, 50),
            Err(FifoRobustSelectionError::TopologyChangedAcrossScenarios { .. })
        ));
    }

    #[test]
    fn minimax_and_nominal_choices_match_independent_exhaustive_reference() {
        let mut candidates = Vec::new();
        let mut reference = Vec::new();

        for nominal in 1_u64..=4 {
            for a_disrupted in 1_u64..=4 {
                for b_disrupted in 1_u64..=4 {
                    let path_id = format!("p-{nominal}-{a_disrupted}-{b_disrupted}");
                    let worst = nominal.max(a_disrupted).max(b_disrupted);
                    candidates.push(candidate(
                        &path_id, nominal, a_disrupted, b_disrupted, 50
                    ));
                    reference.push((path_id, nominal + 100, worst + 100));
                }
            }
        }

        // Independent, deliberately slow score table; do not call production
        // scenario resolution or score helpers.
        let nominal_best = reference.iter()
            .min_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)))
            .expect("reference candidates exist");
        let minimax_best = reference.iter()
            .min_by(|a, b| a.2.cmp(&b.2).then_with(|| a.0.cmp(&b.0)))
            .expect("reference candidates exist");
        let report = select_robust_fifo_path(
            &scenario_set(), &candidates, 100, 50
        ).unwrap();

        assert_eq!(report.candidate_path_count, 64);
        assert_eq!(report.qualified_path_count, 64);
        assert_eq!(report.nominal_best_path_id, nominal_best.0);
        assert_eq!(report.robust_best_path_id, minimax_best.0);
        for score in &report.scores {
            let expected = reference.iter()
                .find(|row| row.0 == score.path_id)
                .expect("score maps to reference");
            assert_eq!(score.nominal_arrival_epoch_seconds, expected.1);
            assert_eq!(score.worst_case_arrival_epoch_seconds, expected.2);
        }
    }

    #[test]
    fn candidates_for_different_destinations_cannot_be_compared() {
        let path_a = candidate("A", 100, 500, 120, 50);
        let mut path_b = candidate("B", 130, 180, 200, 50);
        for scenario in &mut path_b.scenario_paths {
            scenario.legs[0].to_node_id = "other-destination".into();
        }

        assert!(matches!(
            select_robust_fifo_path(&scenario_set(), &[path_a, path_b], 100, 50),
            Err(FifoRobustSelectionError::CandidateEndpointsDiffer { .. })
        ));
    }

    #[test]
    fn one_nominal_scenario_and_manifest_digest_are_mandatory() {
        let mut set = scenario_set();
        set.scenarios[0].kind = FifoPathScenarioKindV1::Stress;
        assert!(matches!(
            select_robust_fifo_path(&set, &[candidate("A", 100, 500, 120, 50)], 100, 50),
            Err(FifoRobustSelectionError::InvalidScenarioSet(_))
        ));

        let mut set = scenario_set();
        set.manifest_sha256 = "not-a-digest".into();
        assert!(matches!(
            select_robust_fifo_path(&set, &[candidate("A", 100, 500, 120, 50)], 100, 50),
            Err(FifoRobustSelectionError::InvalidScenarioSet(_))
        ));
    }

    #[test]
    fn minimax_exactness_claim_is_limited_to_the_supplied_candidate_pool() {
        let report = select_robust_fifo_path(
            &scenario_set(),
            &[candidate("A", 100, 500, 120, 50), candidate("B", 130, 180, 200, 50)],
            100,
            50,
        )
        .unwrap();
        assert_eq!(
            report.optimality_scope,
            "ExactMinimaxOverSuppliedCandidatePathsAndDeclaredScenarios"
        );
    }
}
