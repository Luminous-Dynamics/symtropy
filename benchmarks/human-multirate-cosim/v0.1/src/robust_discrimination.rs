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
use crate::sensor_design::{invert_spd, principal_submatrix};
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


/// Candidate-visible observation history used for a declared model-family adequacy check.
/// Channel order follows sensor_ids; time order follows times.
#[derive(Debug, Clone, PartialEq)]
pub struct ObservedSensorTrace {
    pub sensor_ids: Vec<u32>,
    pub times: Vec<f64>,
    pub values: Vec<Vec<f64>>,
}

/// Explicit residual threshold for the deterministic Gaussian score.
/// It is not automatically a chi-square threshold or a calibrated false-alarm rate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelFamilyAdequacyPolicy {
    pub maximum_gaussian_residual: f64,
}

/// Best finite-grid fit and all parameter variants within the declared threshold.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelFamilyFitScore {
    pub family_id: u32,
    pub best_fit_model_id: u32,
    pub minimum_gaussian_residual: f64,
    pub compatible_model_ids: Vec<u32>,
}

/// Interpretation of candidate-family adequacy under the declared residual policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelFamilyAdequacyDisposition {
    InFamilyPreferred {
        family_id: u32,
        model_id: u32,
    },
    ParameterNotIdentified {
        family_id: u32,
        compatible_model_ids: Vec<u32>,
    },
    InFamilyUnresolved {
        compatible_family_ids: Vec<u32>,
    },
    OutOfModelSet,
}

/// Model-family adequacy result. OUT_OF_MODEL_SET means no registered finite-grid
/// variant met the explicit threshold; it does not identify the omitted cause.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelFamilyAdequacyScore {
    pub profile_id: u32,
    pub maximum_gaussian_residual: f64,
    pub family_fits: Vec<ModelFamilyFitScore>,
    pub disposition: ModelFamilyAdequacyDisposition,
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
    InvalidAdequacyThreshold,
    TraceSensorOrderMismatch,
    TraceTimeScheduleMismatch,
    TraceShapeMismatch,
    NonFiniteObservedValue,
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
            Self::InvalidAdequacyThreshold => {
                write!(f, "maximum Gaussian residual threshold must be finite and nonnegative")
            }
            Self::TraceSensorOrderMismatch => {
                write!(f, "observation trace sensor IDs/order do not match the selected sensors")
            }
            Self::TraceTimeScheduleMismatch => {
                write!(f, "observation trace times do not match the declared experiment schedule")
            }
            Self::TraceShapeMismatch => {
                write!(f, "observation trace shape does not match the time and sensor dimensions")
            }
            Self::NonFiniteObservedValue => {
                write!(f, "observation trace contains a non-finite value")
            }
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

/// Compare a candidate-visible observation trace with every finite-grid model variant.
///
/// The declared threshold is applied to the summed equal-covariance Gaussian
/// residual score. No automatic threshold calibration, posterior inference,
/// model-discrepancy distribution or hidden-truth access is performed.
pub fn evaluate_model_family_adequacy(
    scenario: &ProfiledModelFamilyScenario,
    profile: &InputProfile,
    selected_sensor_ids: &[u32],
    observations: &ObservedSensorTrace,
    policy: ModelFamilyAdequacyPolicy,
) -> Result<ModelFamilyAdequacyScore, ProfiledDiscriminationError> {
    scenario.validate()?;
    if !policy.maximum_gaussian_residual.is_finite()
        || policy.maximum_gaussian_residual < 0.0
    {
        return Err(ProfiledDiscriminationError::InvalidAdequacyThreshold);
    }

    let base_score = evaluate_input_profile(
        &scenario.experiment,
        profile,
        selected_sensor_ids,
    )
    .map_err(ProfiledDiscriminationError::Experiment)?;

    if observations.sensor_ids != base_score.selected_sensor_ids {
        return Err(ProfiledDiscriminationError::TraceSensorOrderMismatch);
    }
    if observations.times.len() != scenario.experiment.observation_times.len()
        || observations
            .times
            .iter()
            .zip(scenario.experiment.observation_times.iter())
            .any(|(observed, expected)| {
                !observed.is_finite() || (observed - expected).abs() > TIE_TOLERANCE
            })
    {
        return Err(ProfiledDiscriminationError::TraceTimeScheduleMismatch);
    }
    if observations.values.len() != observations.times.len()
        || observations
            .values
            .iter()
            .any(|row| row.len() != observations.sensor_ids.len())
    {
        return Err(ProfiledDiscriminationError::TraceShapeMismatch);
    }
    if observations.values.iter().flatten().any(|value| !value.is_finite()) {
        return Err(ProfiledDiscriminationError::NonFiniteObservedValue);
    }

    let sensor_lookup: HashMap<u32, usize> = scenario
        .experiment
        .sensors
        .iter()
        .enumerate()
        .map(|(index, sensor)| (sensor.id, index))
        .collect();
    let selected_indices: Vec<usize> = base_score
        .selected_sensor_ids
        .iter()
        .map(|id| {
            sensor_lookup
                .get(id)
                .copied()
                .ok_or(ProfiledDiscriminationError::TraceSensorOrderMismatch)
        })
        .collect::<Result<_, _>>()?;
    let covariance = principal_submatrix(
        &scenario.experiment.noise_covariance,
        &selected_indices,
    );
    let covariance_inverse = invert_spd(&covariance).map_err(|error| {
        ProfiledDiscriminationError::Experiment(
            ExcitationDesignError::SensorCovariance(error),
        )
    })?;

    let family_by_model: HashMap<u32, u32> = scenario
        .family_assignments
        .iter()
        .map(|assignment| (assignment.model_id, assignment.family_id))
        .collect();
    let mut model_residuals: HashMap<u32, f64> = HashMap::new();
    for prediction in &base_score.model_observation_means {
        let mut residual_score = 0.0;
        for time_index in 0..observations.times.len() {
            let residual: Vec<f64> = observations.values[time_index]
                .iter()
                .zip(prediction.observations[time_index].iter())
                .map(|(observed, predicted)| observed - predicted)
                .collect();
            let weighted = matrix_vector_product(&covariance_inverse, &residual);
            residual_score += 0.5
                * residual
                    .iter()
                    .zip(weighted.iter())
                    .map(|(value, weighted_value)| value * weighted_value)
                    .sum::<f64>();
        }
        model_residuals.insert(prediction.model_id, residual_score.max(0.0));
    }

    let mut ordered_families: Vec<u32> = family_by_model.values().copied().collect();
    ordered_families.sort_unstable();
    ordered_families.dedup();
    let mut family_fits = Vec::with_capacity(ordered_families.len());

    for family_id in ordered_families {
        let mut variants: Vec<(u32, f64)> = family_by_model
            .iter()
            .filter(|(_, candidate_family)| **candidate_family == family_id)
            .filter_map(|(model_id, _)| {
                model_residuals
                    .get(model_id)
                    .copied()
                    .map(|residual| (*model_id, residual))
            })
            .collect();
        variants.sort_by_key(|(model_id, _)| *model_id);
        if variants.is_empty() {
            return Err(ProfiledDiscriminationError::MissingVariantPair {
                left_family: family_id,
                right_family: family_id,
            });
        }

        let minimum_gaussian_residual = variants
            .iter()
            .map(|(_, residual)| *residual)
            .fold(f64::INFINITY, f64::min);
        let best_fit_model_id = variants
            .iter()
            .find(|(_, residual)| {
                (*residual - minimum_gaussian_residual).abs() <= TIE_TOLERANCE
            })
            .map(|(model_id, _)| *model_id)
            .ok_or(ProfiledDiscriminationError::MissingFamilyAssignments)?;
        let compatible_model_ids = variants
            .iter()
            .filter(|(_, residual)| {
                *residual <= policy.maximum_gaussian_residual + TIE_TOLERANCE
            })
            .map(|(model_id, _)| *model_id)
            .collect();

        family_fits.push(ModelFamilyFitScore {
            family_id,
            best_fit_model_id,
            minimum_gaussian_residual,
            compatible_model_ids,
        });
    }

    let compatible_families: Vec<&ModelFamilyFitScore> = family_fits
        .iter()
        .filter(|family| !family.compatible_model_ids.is_empty())
        .collect();
    let disposition = match compatible_families.as_slice() {
        [] => ModelFamilyAdequacyDisposition::OutOfModelSet,
        [family] if family.compatible_model_ids.len() == 1 => {
            ModelFamilyAdequacyDisposition::InFamilyPreferred {
                family_id: family.family_id,
                model_id: family.compatible_model_ids[0],
            }
        }
        [family] => ModelFamilyAdequacyDisposition::ParameterNotIdentified {
            family_id: family.family_id,
            compatible_model_ids: family.compatible_model_ids.clone(),
        },
        families => ModelFamilyAdequacyDisposition::InFamilyUnresolved {
            compatible_family_ids: families.iter().map(|family| family.family_id).collect(),
        },
    };

    Ok(ModelFamilyAdequacyScore {
        profile_id: profile.id,
        maximum_gaussian_residual: policy.maximum_gaussian_residual,
        family_fits,
        disposition,
    })
}

fn matrix_vector_product(matrix: &[Vec<f64>], vector: &[f64]) -> Vec<f64> {
    matrix
        .iter()
        .map(|row| row.iter().zip(vector.iter()).map(|(a, b)| a * b).sum())
        .collect()
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
