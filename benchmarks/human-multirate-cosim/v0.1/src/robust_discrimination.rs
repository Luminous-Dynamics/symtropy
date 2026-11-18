//! Finite-grid least-favourable model-family discrimination.
//!
//! This module composes the fixed-parameter Gaussian-KL experiment evaluator.
//! It groups declared parameter variants into structural model families and
//! reports the minimum cross-family separation over every admitted variant
//! pair. It is an exact finite-grid nuisance-profile reference, not a continuous
//! profile-likelihood or Pontryagin optimal-control implementation.

use crate::experiment_design::{
    ExcitationDesignError, ExperimentDesignScenario, ExperimentDesignScore, InputProfile,
    evaluate_input_profile,
};
use std::collections::{HashMap, HashSet};
use std::fmt;

const TIE_TOLERANCE: f64 = 1.0e-12;
const OVERLAP_TOLERANCE: f64 = 1.0e-12;

/// Assigns one fixed-parameter model variant to a structural/model family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelFamilyAssignment {
    pub model_id: u32,
    pub family_id: u32,
}

/// Least-favourable cross-family variant pair under one design.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProfiledFamilyPairSeparation {
    pub left_family_id: u32,
    pub right_family_id: u32,
    pub least_favourable_left_model_id: u32,
    pub least_favourable_right_model_id: u32,
    pub least_favourable_gaussian_kl: f64,
    pub parameter_pair_count: usize,
}

/// Declared grouping of candidate model parameter variants.
#[derive(Debug, Clone, PartialEq)]
pub struct ProfiledModelFamilyScenario {
    pub experiment: ExperimentDesignScenario,
    /// Exactly one assignment is required for every candidate model variant.
    pub family_assignments: Vec<ModelFamilyAssignment>,
}

/// Worst-case model-family separation after finite-grid nuisance profiling.
#[derive(Debug, Clone, PartialEq)]
pub struct ProfiledDiscriminationScore {
    pub base_score: ExperimentDesignScore,
    pub family_pair_separations: Vec<ProfiledFamilyPairSeparation>,
    pub minimum_profiled_family_separation: f64,
    pub mean_profiled_family_separation: f64,
    pub worst_case_family_overlap: bool,
}

/// Tournament over a frozen catalogue of input profiles.
#[derive(Debug, Clone, PartialEq)]
pub struct ProfiledDiscriminationTournament {
    pub scores: Vec<ProfiledDiscriminationScore>,
    pub best_worst_case: Option<ProfiledDiscriminationScore>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfiledDiscriminationError {
    Experiment(ExcitationDesignError),
    MissingFamilyAssignments,
    DuplicateModelFamilyAssignment(u32),
    UnknownModelId(u32),
    MissingModelFamilyAssignment(u32),
    InsufficientModelFamilies,
    MissingVariantPair { left_family: u32, right_family: u32 },
    NoProfiles,
    DuplicateProfileId(u32),
}

impl fmt::Display for ProfiledDiscriminationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Experiment(error) => write!(f, "base experiment invalid: {error}"),
            Self::MissingFamilyAssignments => {
                write!(f, "at least one model-family assignment is required")
            }
            Self::DuplicateModelFamilyAssignment(id) => {
                write!(f, "model ID {id} has more than one family assignment")
            }
            Self::UnknownModelId(id) => {
                write!(f, "family assignment references unknown model ID {id}")
            }
            Self::MissingModelFamilyAssignment(id) => {
                write!(f, "model ID {id} has no family assignment")
            }
            Self::InsufficientModelFamilies => {
                write!(f, "at least two distinct model families are required")
            }
            Self::MissingVariantPair {
                left_family,
                right_family,
            } => write!(
                f,
                "no model-variant pair exists for families {left_family} and {right_family}"
            ),
            Self::NoProfiles => write!(f, "at least one input profile is required"),
            Self::DuplicateProfileId(id) => write!(f, "duplicate input profile ID {id}"),
        }
    }
}

impl std::error::Error for ProfiledDiscriminationError {}

impl ProfiledModelFamilyScenario {
    pub fn validate(&self) -> Result<(), ProfiledDiscriminationError> {
        self.experiment
            .validate()
            .map_err(ProfiledDiscriminationError::Experiment)?;
        if self.family_assignments.is_empty() {
            return Err(ProfiledDiscriminationError::MissingFamilyAssignments);
        }

        let model_ids: HashSet<u32> = self
            .experiment
            .models
            .iter()
            .map(|model| model.id)
            .collect();
        let mut assigned_model_ids = HashSet::new();
        let mut family_ids = HashSet::new();

        for assignment in &self.family_assignments {
            if !model_ids.contains(&assignment.model_id) {
                return Err(ProfiledDiscriminationError::UnknownModelId(
                    assignment.model_id,
                ));
            }
            if !assigned_model_ids.insert(assignment.model_id) {
                return Err(
                    ProfiledDiscriminationError::DuplicateModelFamilyAssignment(
                        assignment.model_id,
                    ),
                );
            }
            family_ids.insert(assignment.family_id);
        }

        for model_id in model_ids {
            if !assigned_model_ids.contains(&model_id) {
                return Err(ProfiledDiscriminationError::MissingModelFamilyAssignment(
                    model_id,
                ));
            }
        }
        if family_ids.len() < 2 {
            return Err(ProfiledDiscriminationError::InsufficientModelFamilies);
        }
        Ok(())
    }
}

/// Evaluate every family pair using the least-favourable parameter-variant
/// combination admitted by the frozen finite grid.
pub fn evaluate_profiled_input(
    scenario: &ProfiledModelFamilyScenario,
    profile: &InputProfile,
    selected_sensor_ids: &[u32],
) -> Result<ProfiledDiscriminationScore, ProfiledDiscriminationError> {
    scenario.validate()?;
    let base_score = evaluate_input_profile(
        &scenario.experiment,
        profile,
        selected_sensor_ids,
    )
    .map_err(ProfiledDiscriminationError::Experiment)?;

    let family_by_model: HashMap<u32, u32> = scenario
        .family_assignments
        .iter()
        .map(|assignment| (assignment.model_id, assignment.family_id))
        .collect();
    let mut families: Vec<u32> = family_by_model.values().copied().collect();
    families.sort_unstable();
    families.dedup();

    let mut family_pair_separations = Vec::new();
    for left_index in 0..families.len() {
        for right_index in (left_index + 1)..families.len() {
            let left_family = families[left_index];
            let right_family = families[right_index];
            let mut least_favourable: Option<ProfiledFamilyPairSeparation> = None;
            let mut parameter_pair_count = 0_usize;

            for pair in &base_score.pairwise_separations {
                let Some(pair_left_family) = family_by_model.get(&pair.left_model_id) else {
                    continue;
                };
                let Some(pair_right_family) = family_by_model.get(&pair.right_model_id) else {
                    continue;
                };

                let matches = (*pair_left_family == left_family
                    && *pair_right_family == right_family)
                    || (*pair_left_family == right_family
                        && *pair_right_family == left_family);
                if !matches {
                    continue;
                }
                parameter_pair_count += 1;

                let (left_model_id, right_model_id) =
                    if *pair_left_family == left_family {
                        (pair.left_model_id, pair.right_model_id)
                    } else {
                        (pair.right_model_id, pair.left_model_id)
                    };

                let candidate = ProfiledFamilyPairSeparation {
                    left_family_id: left_family,
                    right_family_id: right_family,
                    least_favourable_left_model_id: left_model_id,
                    least_favourable_right_model_id: right_model_id,
                    least_favourable_gaussian_kl: pair.gaussian_kl_separation,
                    parameter_pair_count: 0,
                };
                if least_favourable.as_ref().is_none_or(|current| {
                    candidate.least_favourable_gaussian_kl
                        < current.least_favourable_gaussian_kl - TIE_TOLERANCE
                        || ((candidate.least_favourable_gaussian_kl
                            - current.least_favourable_gaussian_kl)
                            .abs()
                            <= TIE_TOLERANCE
                            && (candidate.least_favourable_left_model_id,
                                candidate.least_favourable_right_model_id)
                                < (
                                    current.least_favourable_left_model_id,
                                    current.least_favourable_right_model_id,
                                ))
                }) {
                    least_favourable = Some(candidate);
                }
            }

            let mut least_favourable = least_favourable.ok_or(
                ProfiledDiscriminationError::MissingVariantPair {
                    left_family,
                    right_family,
                },
            )?;
            least_favourable.parameter_pair_count = parameter_pair_count;
            family_pair_separations.push(least_favourable);
        }
    }

    let minimum_profiled_family_separation = family_pair_separations
        .iter()
        .map(|pair| pair.least_favourable_gaussian_kl)
        .fold(f64::INFINITY, f64::min);
    let mean_profiled_family_separation = family_pair_separations
        .iter()
        .map(|pair| pair.least_favourable_gaussian_kl)
        .sum::<f64>()
        / family_pair_separations.len() as f64;

    Ok(ProfiledDiscriminationScore {
        base_score,
        family_pair_separations,
        minimum_profiled_family_separation,
        mean_profiled_family_separation,
        worst_case_family_overlap: minimum_profiled_family_separation <= OVERLAP_TOLERANCE,
    })
}

/// Select the design maximizing the minimum least-favourable family separation.
/// Ties prefer lower input energy, then lower peak amplitude, then lower profile ID.
pub fn evaluate_profiled_tournament(
    scenario: &ProfiledModelFamilyScenario,
    profiles: &[InputProfile],
    selected_sensor_ids: &[u32],
) -> Result<ProfiledDiscriminationTournament, ProfiledDiscriminationError> {
    if profiles.is_empty() {
        return Err(ProfiledDiscriminationError::NoProfiles);
    }
    let mut profile_ids = HashSet::new();
    for profile in profiles {
        if !profile_ids.insert(profile.id) {
            return Err(ProfiledDiscriminationError::DuplicateProfileId(profile.id));
        }
    }

    let mut scores = Vec::with_capacity(profiles.len());
    for profile in profiles {
        scores.push(evaluate_profiled_input(
            scenario,
            profile,
            selected_sensor_ids,
        )?);
    }

    let mut best: Option<ProfiledDiscriminationScore> = None;
    for score in &scores {
        let replace = best.as_ref().is_none_or(|current| {
            score.minimum_profiled_family_separation
                > current.minimum_profiled_family_separation + TIE_TOLERANCE
                || ((score.minimum_profiled_family_separation
                    - current.minimum_profiled_family_separation)
                    .abs()
                    <= TIE_TOLERANCE
                    && (score.base_score.integrated_squared_input
                        < current.base_score.integrated_squared_input - TIE_TOLERANCE
                        || ((score.base_score.integrated_squared_input
                            - current.base_score.integrated_squared_input)
                            .abs()
                            <= TIE_TOLERANCE
                            && (score.base_score.maximum_abs_forcing
                                < current.base_score.maximum_abs_forcing - TIE_TOLERANCE
                                || ((score.base_score.maximum_abs_forcing
                                    - current.base_score.maximum_abs_forcing)
                                    .abs()
                                    <= TIE_TOLERANCE
                                    && score.base_score.profile_id
                                        < current.base_score.profile_id)))))
        });
        if replace {
            best = Some(score.clone());
        }
    }

    Ok(ProfiledDiscriminationTournament {
        scores,
        best_worst_case: best,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::experiment_design::{ForcingSegment, ModelCandidate};
    use crate::network::{Edge, NetworkScenario};
    use crate::sensor_design::SensorCandidate;

    fn candidate(id: u32, label: &'static str, bypass_conductance: f64) -> ModelCandidate {
        let mut edges = vec![
            Edge {
                left: 0,
                right: 1,
                conductance: 1.0,
            },
            Edge {
                left: 1,
                right: 2,
                conductance: 1.0,
            },
        ];
        if bypass_conductance > 0.0 {
            edges.push(Edge {
                left: 0,
                right: 2,
                conductance: bypass_conductance,
            });
        }
        ModelCandidate {
            id,
            label,
            network: NetworkScenario {
                name: label,
                capacities: vec![1.0, 1.0, 1.0],
                initial: vec![0.0, 0.0, 0.0],
                edges,
                horizon: 2.0,
            },
        }
    }

    fn scenario() -> ProfiledModelFamilyScenario {
        ProfiledModelFamilyScenario {
            experiment: ExperimentDesignScenario {
                name: "chain_vs_bypass_nuisance_profile",
                models: vec![
                    candidate(1, "chain_nominal", 0.0),
                    candidate(2, "bypass_nominal", 0.5),
                    candidate(3, "bypass_zero_limit", 0.0),
                ],
                sensors: vec![SensorCandidate {
                    id: 30,
                    node: 2,
                    cost: 1.0,
                    common_cause_group: None,
                }],
                noise_covariance: vec![vec![0.01]],
                observation_times: vec![0.25, 0.5, 1.0, 2.0],
                sensor_budget: 1.0,
                max_abs_forcing: 1.0,
                input_energy_budget: 0.5,
            },
            family_assignments: vec![
                ModelFamilyAssignment {
                    model_id: 1,
                    family_id: 100,
                },
                ModelFamilyAssignment {
                    model_id: 2,
                    family_id: 200,
                },
                ModelFamilyAssignment {
                    model_id: 3,
                    family_id: 200,
                },
            ],
        }
    }

    fn pulse() -> InputProfile {
        InputProfile {
            id: 2,
            label: "bounded_node_zero_pulse",
            segments: vec![
                ForcingSegment {
                    start_time: 0.0,
                    end_time: 0.5,
                    forcing: vec![1.0, 0.0, 0.0],
                },
                ForcingSegment {
                    start_time: 0.5,
                    end_time: 2.0,
                    forcing: vec![0.0, 0.0, 0.0],
                },
            ],
        }
    }

    #[test]
    fn nuisance_profiling_detects_a_family_that_contains_the_chain_limit() {
        let score = evaluate_profiled_input(&scenario(), &pulse(), &[30]).unwrap();
        let nominal_pair = score
            .base_score
            .pairwise_separations
            .iter()
            .find(|pair| pair.left_model_id == 1 && pair.right_model_id == 2)
            .unwrap();
        assert!(nominal_pair.gaussian_kl_separation > 0.3);
        assert_eq!(score.family_pair_separations.len(), 1);
        assert_eq!(score.family_pair_separations[0].parameter_pair_count, 2);
        assert_eq!(
            score.family_pair_separations[0].least_favourable_left_model_id,
            1
        );
        assert_eq!(
            score.family_pair_separations[0].least_favourable_right_model_id,
            3
        );
        assert!(score.minimum_profiled_family_separation < 1.0e-12);
        assert!(score.worst_case_family_overlap);
    }

    #[test]
    fn family_assignment_must_cover_each_candidate_exactly_once() {
        let mut problem = scenario();
        problem.family_assignments.pop();
        assert_eq!(
            problem.validate(),
            Err(ProfiledDiscriminationError::MissingModelFamilyAssignment(3))
        );

        let mut duplicate = scenario();
        duplicate.family_assignments.push(duplicate.family_assignments[0]);
        assert_eq!(
            duplicate.validate(),
            Err(ProfiledDiscriminationError::DuplicateModelFamilyAssignment(1))
        );
    }

    #[test]
    fn within_family_parameter_variants_are_not_mistaken_for_structural_alternatives() {
        let mut problem = scenario();
        problem.family_assignments = vec![
            ModelFamilyAssignment {
                model_id: 1,
                family_id: 100,
            },
            ModelFamilyAssignment {
                model_id: 2,
                family_id: 100,
            },
            ModelFamilyAssignment {
                model_id: 3,
                family_id: 200,
            },
        ];
        let score = evaluate_profiled_input(&problem, &pulse(), &[30]).unwrap();
        assert_eq!(score.family_pair_separations.len(), 1);
        assert_eq!(
            score.family_pair_separations[0].parameter_pair_count,
            2
        );
    }
}
