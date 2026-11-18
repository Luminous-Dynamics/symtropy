//! Synthetic ensemble calibration of model-family residual thresholds.
//!
//! This module estimates a deterministic adequacy threshold on one synthetic
//! Gaussian-noise seed and evaluates it on a distinct held-out seed. It reports
//! finite-sample counts/rates only; no asymptotic chi-square calibration or
//! real-world false-alarm guarantee is implied.

use crate::experiment_design::{
    ExcitationDesignError, ExperimentDesignScenario, InputProfile, evaluate_input_profile,
};
use crate::network::{NetworkScenario, SpectralOracle};
use crate::robust_discrimination::{
    ModelFamilyAdequacyDisposition, ModelFamilyAdequacyPolicy, ObservedSensorTrace,
    ProfiledDiscriminationError, ProfiledModelFamilyScenario, evaluate_model_family_adequacy,
};
use crate::sensor_design::principal_submatrix;
use std::fmt;

const SPLITMIX_INCREMENT: u64 = 0x9E37_79B9_7F4A_7C15;

/// Independent deterministic Monte Carlo streams for threshold fitting and holdout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdequacyCalibrationConfig {
    pub calibration_seed: u64,
    pub validation_seed: u64,
    /// Number of independent observation traces per candidate truth variant.
    pub replicates_per_model: usize,
    /// Target empirical training-set coverage used to select the threshold.
    pub target_in_family_coverage: f64,
}

/// One held-out out-of-family graph truth, never inserted into the candidate family.
#[derive(Debug, Clone, PartialEq)]
pub struct HiddenOutOfFamilyTruth {
    pub evaluator_id: u32,
    pub network: NetworkScenario,
}

/// Finite-ensemble threshold calibration and held-out adequacy results.
#[derive(Debug, Clone, PartialEq)]
pub struct AdequacyCalibrationReport {
    pub threshold: f64,
    pub target_in_family_coverage: f64,
    pub calibration_samples: usize,
    pub calibration_in_family_accepts: usize,
    pub calibration_in_family_coverage: f64,
    pub validation_samples: usize,
    pub validation_in_family_accepts: usize,
    pub validation_in_family_coverage: f64,
    pub validation_false_out_of_model_set_count: usize,
    pub validation_false_out_of_model_set_rate: f64,
    pub validation_wrong_family_preference_count: usize,
    pub validation_wrong_family_preference_rate: f64,
    pub validation_ambiguous_count: usize,
    pub validation_ambiguous_rate: f64,
    pub out_of_family_samples: usize,
    pub out_of_family_rejections: usize,
    pub out_of_family_rejection_rate: f64,
    pub calibration_seed: u64,
    pub validation_seed: u64,
    pub assumptions: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdequacyCalibrationError {
    Profiled(ProfiledDiscriminationError),
    Experiment(ExcitationDesignError),
    InvalidReplicateCount,
    InvalidTargetCoverage,
    SeedsMustDiffer,
    CovarianceNotPositiveDefinite,
    UnknownSelectedSensor(u32),
    IncompatibleHiddenTruth { evaluator_id: u32 },
    MissingModelPrediction(u32),
    MissingTrueFamily(u32),
    MissingFamilyResidual(u32),
    EmptyCalibrationSet,
    NonFiniteGeneratedValue,
}

impl fmt::Display for AdequacyCalibrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Profiled(error) => write!(f, "profiled model-family scenario invalid: {error}"),
            Self::Experiment(error) => write!(f, "base experiment invalid: {error}"),
            Self::InvalidReplicateCount => {
                write!(f, "replicates per model must be positive")
            }
            Self::InvalidTargetCoverage => {
                write!(f, "target in-family coverage must be finite and strictly between zero and one")
            }
            Self::SeedsMustDiffer => {
                write!(f, "calibration and validation seeds must differ")
            }
            Self::CovarianceNotPositiveDefinite => {
                write!(f, "selected observation covariance must be strictly positive definite")
            }
            Self::UnknownSelectedSensor(id) => {
                write!(f, "selected sensor ID {id} is absent from the experiment manifest")
            }
            Self::IncompatibleHiddenTruth { evaluator_id } => {
                write!(f, "hidden truth {evaluator_id} does not share the declared state basis and horizon")
            }
            Self::MissingModelPrediction(id) => {
                write!(f, "candidate model {id} has no predicted observation trace")
            }
            Self::MissingTrueFamily(id) => {
                write!(f, "candidate model {id} has no declared structural family")
            }
            Self::MissingFamilyResidual(id) => {
                write!(f, "candidate family {id} has no residual in the adequacy report")
            }
            Self::EmptyCalibrationSet => {
                write!(f, "no in-family calibration samples were generated")
            }
            Self::NonFiniteGeneratedValue => {
                write!(f, "synthetic Gaussian sampling generated a non-finite value")
            }
        }
    }
}

impl std::error::Error for AdequacyCalibrationError {}

/// Small deterministic PRNG. Intended only for repeatable synthetic test
/// ensembles, never for secrets, cryptography or production random sampling.
#[derive(Debug, Clone, Copy)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(SPLITMIX_INCREMENT);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn uniform_open(&mut self) -> f64 {
        let mantissa = self.next_u64() >> 11;
        (mantissa as f64 + 0.5) / 9_007_199_254_740_992.0
    }

    fn standard_normal(&mut self) -> f64 {
        let radius = (-2.0 * self.uniform_open().ln()).sqrt();
        let angle = std::f64::consts::TAU * self.uniform_open();
        radius * angle.cos()
    }
}

/// Calibrate a family-adequacy residual threshold using synthetic Gaussian noise.
///
/// Candidate truth variants are all included in the calibration/validation
/// ensembles. Hidden out-of-family truths are evaluated only on the holdout
/// seed. The threshold is the empirical nearest-rank quantile of the minimum
/// residual to the true family under the calibration seed.
pub fn calibrate_model_family_adequacy(
    scenario: &ProfiledModelFamilyScenario,
    profile: &InputProfile,
    selected_sensor_ids: &[u32],
    hidden_out_of_family_truths: &[HiddenOutOfFamilyTruth],
    config: AdequacyCalibrationConfig,
) -> Result<AdequacyCalibrationReport, AdequacyCalibrationError> {
    scenario
        .validate()
        .map_err(AdequacyCalibrationError::Profiled)?;
    if config.replicates_per_model == 0 {
        return Err(AdequacyCalibrationError::InvalidReplicateCount);
    }
    if !config.target_in_family_coverage.is_finite()
        || config.target_in_family_coverage <= 0.0
        || config.target_in_family_coverage >= 1.0
    {
        return Err(AdequacyCalibrationError::InvalidTargetCoverage);
    }
    if config.calibration_seed == config.validation_seed {
        return Err(AdequacyCalibrationError::SeedsMustDiffer);
    }

    let base_score = evaluate_input_profile(
        &scenario.experiment,
        profile,
        selected_sensor_ids,
    )
    .map_err(AdequacyCalibrationError::Experiment)?;
    let canonical_sensor_ids = base_score.selected_sensor_ids.clone();
    let selected_indices = selected_covariance_indices(
        &scenario.experiment,
        &canonical_sensor_ids,
    )?;
    let covariance = principal_submatrix(
        &scenario.experiment.noise_covariance,
        &selected_indices,
    );
    let cholesky = cholesky_lower(&covariance)?;
    let family_by_model: std::collections::HashMap<u32, u32> = scenario
        .family_assignments
        .iter()
        .map(|assignment| (assignment.model_id, assignment.family_id))
        .collect();

    for hidden in hidden_out_of_family_truths {
        if hidden.network.capacities != scenario.experiment.models[0].network.capacities
            || hidden.network.initial != scenario.experiment.models[0].network.initial
            || hidden.network.horizon != scenario.experiment.horizon()
        {
            return Err(AdequacyCalibrationError::IncompatibleHiddenTruth {
                evaluator_id: hidden.evaluator_id,
            });
        }
    }

    let predictions: std::collections::HashMap<u32, Vec<Vec<f64>>> = base_score
        .model_observation_means
        .iter()
        .map(|model| (model.model_id, model.observations.clone()))
        .collect();

    let mut calibration_rng = SplitMix64::new(config.calibration_seed);
    let mut calibration_residuals = Vec::with_capacity(
        scenario.experiment.models.len() * config.replicates_per_model,
    );
    for model in &scenario.experiment.models {
        let family_id = *family_by_model
            .get(&model.id)
            .ok_or(AdequacyCalibrationError::MissingTrueFamily(model.id))?;
        let means = predictions
            .get(&model.id)
            .ok_or(AdequacyCalibrationError::MissingModelPrediction(model.id))?;

        for _ in 0..config.replicates_per_model {
            let trace = sample_observation_trace(
                &canonical_sensor_ids,
                &scenario.experiment.observation_times,
                means,
                &cholesky,
                &mut calibration_rng,
            )?;
            let adequacy = evaluate_model_family_adequacy(
                scenario,
                profile,
                &canonical_sensor_ids,
                &trace,
                ModelFamilyAdequacyPolicy {
                    maximum_gaussian_residual: f64::MAX,
                },
            )
            .map_err(AdequacyCalibrationError::Profiled)?;
            let residual = adequacy
                .family_fits
                .iter()
                .find(|fit| fit.family_id == family_id)
                .map(|fit| fit.minimum_gaussian_residual)
                .ok_or(AdequacyCalibrationError::MissingFamilyResidual(family_id))?;
            calibration_residuals.push(residual);
        }
    }
    if calibration_residuals.is_empty() {
        return Err(AdequacyCalibrationError::EmptyCalibrationSet);
    }
    calibration_residuals.sort_by(f64::total_cmp);
    let rank = ((config.target_in_family_coverage * calibration_residuals.len() as f64).ceil()
        as usize)
        .clamp(1, calibration_residuals.len());
    let threshold = calibration_residuals[rank - 1];

    let mut validation_rng = SplitMix64::new(config.validation_seed);
    let mut validation_samples = 0_usize;
    let mut validation_in_family_accepts = 0_usize;
    let mut validation_false_out_of_model_set_count = 0_usize;
    let mut validation_wrong_family_preference_count = 0_usize;
    let mut validation_ambiguous_count = 0_usize;

    for model in &scenario.experiment.models {
        let true_family_id = *family_by_model
            .get(&model.id)
            .ok_or(AdequacyCalibrationError::MissingTrueFamily(model.id))?;
        let means = predictions
            .get(&model.id)
            .ok_or(AdequacyCalibrationError::MissingModelPrediction(model.id))?;

        for _ in 0..config.replicates_per_model {
            validation_samples += 1;
            let trace = sample_observation_trace(
                &canonical_sensor_ids,
                &scenario.experiment.observation_times,
                means,
                &cholesky,
                &mut validation_rng,
            )?;
            let adequacy = evaluate_model_family_adequacy(
                scenario,
                profile,
                &canonical_sensor_ids,
                &trace,
                ModelFamilyAdequacyPolicy {
                    maximum_gaussian_residual: threshold,
                },
            )
            .map_err(AdequacyCalibrationError::Profiled)?;

            match &adequacy.disposition {
                ModelFamilyAdequacyDisposition::OutOfModelSet => {
                    validation_false_out_of_model_set_count += 1;
                }
                ModelFamilyAdequacyDisposition::InFamilyPreferred { family_id, .. } => {
                    if *family_id == true_family_id {
                        validation_in_family_accepts += 1;
                    } else {
                        validation_wrong_family_preference_count += 1;
                    }
                }
                ModelFamilyAdequacyDisposition::ParameterNotIdentified { family_id, .. } => {
                    if *family_id == true_family_id {
                        validation_in_family_accepts += 1;
                    } else {
                        validation_wrong_family_preference_count += 1;
                    }
                    validation_ambiguous_count += 1;
                }
                ModelFamilyAdequacyDisposition::InFamilyUnresolved {
                    compatible_family_ids,
                } => {
                    if compatible_family_ids.contains(&true_family_id) {
                        validation_in_family_accepts += 1;
                    } else {
                        validation_wrong_family_preference_count += 1;
                    }
                    validation_ambiguous_count += 1;
                }
            }
        }
    }

    let mut out_of_family_samples = 0_usize;
    let mut out_of_family_rejections = 0_usize;
    for hidden in hidden_out_of_family_truths {
        let means = predict_hidden_truth(
            &hidden.network,
            &scenario.experiment,
            profile,
            &canonical_sensor_ids,
        )?;
        for _ in 0..config.replicates_per_model {
            out_of_family_samples += 1;
            let trace = sample_observation_trace(
                &canonical_sensor_ids,
                &scenario.experiment.observation_times,
                &means,
                &cholesky,
                &mut validation_rng,
            )?;
            let adequacy = evaluate_model_family_adequacy(
                scenario,
                profile,
                &canonical_sensor_ids,
                &trace,
                ModelFamilyAdequacyPolicy {
                    maximum_gaussian_residual: threshold,
                },
            )
            .map_err(AdequacyCalibrationError::Profiled)?;
            if adequacy.disposition == ModelFamilyAdequacyDisposition::OutOfModelSet {
                out_of_family_rejections += 1;
            }
        }
    }

    let calibration_accepts = calibration_residuals
        .iter()
        .filter(|residual| **residual <= threshold)
        .count();
    let calibration_samples = calibration_residuals.len();
    let validation_in_family_coverage = rate(
        validation_in_family_accepts,
        validation_samples,
    );
    Ok(AdequacyCalibrationReport {
        threshold,
        target_in_family_coverage: config.target_in_family_coverage,
        calibration_samples,
        calibration_in_family_accepts: calibration_accepts,
        calibration_in_family_coverage: rate(calibration_accepts, calibration_samples),
        validation_samples,
        validation_in_family_accepts,
        validation_in_family_coverage,
        validation_false_out_of_model_set_count,
        validation_false_out_of_model_set_rate: rate(
            validation_false_out_of_model_set_count,
            validation_samples,
        ),
        validation_wrong_family_preference_count,
        validation_wrong_family_preference_rate: rate(
            validation_wrong_family_preference_count,
            validation_samples,
        ),
        validation_ambiguous_count,
        validation_ambiguous_rate: rate(validation_ambiguous_count, validation_samples),
        out_of_family_samples,
        out_of_family_rejections,
        out_of_family_rejection_rate: rate(out_of_family_rejections, out_of_family_samples),
        calibration_seed: config.calibration_seed,
        validation_seed: config.validation_seed,
        assumptions: vec![
            "additive zero-mean Gaussian noise",
            "known full covariance, shared by all candidate models",
            "observation-noise blocks are independent across sample times",
            "candidate model parameters and observation operators are fixed within a run",
            "finite synthetic sample counts; reported rates are empirical, not guarantees",
        ],
    })
}

fn rate(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn selected_covariance_indices(
    scenario: &ExperimentDesignScenario,
    selected_sensor_ids: &[u32],
) -> Result<Vec<usize>, AdequacyCalibrationError> {
    let mut indices = Vec::with_capacity(selected_sensor_ids.len());
    for id in selected_sensor_ids {
        let index = scenario
            .sensors
            .iter()
            .position(|sensor| sensor.id == *id)
            .ok_or(AdequacyCalibrationError::UnknownSelectedSensor(*id))?;
        indices.push(index);
    }
    Ok(indices)
}

fn cholesky_lower(matrix: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, AdequacyCalibrationError> {
    let n = matrix.len();
    let mut lower = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let mut value = matrix[i][j];
            for k in 0..j {
                value -= lower[i][k] * lower[j][k];
            }
            if i == j {
                if !value.is_finite() || value <= 0.0 {
                    return Err(AdequacyCalibrationError::CovarianceNotPositiveDefinite);
                }
                lower[i][j] = value.sqrt();
            } else {
                lower[i][j] = value / lower[j][j];
            }
        }
    }
    Ok(lower)
}

fn sample_observation_trace(
    sensor_ids: &[u32],
    times: &[f64],
    means: &[Vec<f64>],
    cholesky: &[Vec<f64>],
    rng: &mut SplitMix64,
) -> Result<ObservedSensorTrace, AdequacyCalibrationError> {
    let sensor_count = sensor_ids.len();
    let mut values = Vec::with_capacity(times.len());
    for (time_index, _) in times.iter().enumerate() {
        let standard: Vec<f64> = (0..sensor_count).map(|_| rng.standard_normal()).collect();
        let mut noise = vec![0.0; sensor_count];
        for row in 0..sensor_count {
            for column in 0..=row {
                noise[row] += cholesky[row][column] * standard[column];
            }
        }
        let row: Vec<f64> = means[time_index]
            .iter()
            .zip(noise.iter())
            .map(|(mean, perturbation)| mean + perturbation)
            .collect();
        if row.iter().any(|value| !value.is_finite()) {
            return Err(AdequacyCalibrationError::NonFiniteGeneratedValue);
        }
        values.push(row);
    }
    Ok(ObservedSensorTrace {
        sensor_ids: sensor_ids.to_vec(),
        times: times.to_vec(),
        values,
    })
}

fn predict_hidden_truth(
    network: &NetworkScenario,
    experiment: &ExperimentDesignScenario,
    profile: &InputProfile,
    selected_sensor_ids: &[u32],
) -> Result<Vec<Vec<f64>>, AdequacyCalibrationError> {
    let oracle: SpectralOracle = network
        .spectral_oracle()
        .map_err(|error| {
            AdequacyCalibrationError::Profiled(
                ProfiledDiscriminationError::Experiment(
                    ExcitationDesignError::Network(error),
                ),
            )
        })?;
    let mut observations = Vec::with_capacity(experiment.observation_times.len());
    for time in &experiment.observation_times {
        let state = forced_state_at_calibration(&oracle, &network.initial, profile, *time)?;
        let mut row = Vec::with_capacity(selected_sensor_ids.len());
        for sensor_id in selected_sensor_ids {
            let sensor = experiment
                .sensors
                .iter()
                .find(|sensor| sensor.id == *sensor_id)
                .ok_or(AdequacyCalibrationError::CovarianceNotPositiveDefinite)?;
            row.push(state[sensor.node]);
        }
        observations.push(row);
    }
    Ok(observations)
}

fn forced_state_at_calibration(
    oracle: &SpectralOracle,
    initial_state: &[f64],
    profile: &InputProfile,
    time: f64,
) -> Result<Vec<f64>, AdequacyCalibrationError> {
    let mut state = initial_state.to_vec();
    for segment in &profile.segments {
        if time <= segment.start_time {
            break;
        }
        let elapsed = time.min(segment.end_time) - segment.start_time;
        if elapsed > 0.0 {
            state = oracle
                .state_after_constant_forcing(&state, &segment.forcing, elapsed)
                .map_err(|error| {
                    AdequacyCalibrationError::Profiled(
                        ProfiledDiscriminationError::Experiment(
                            ExcitationDesignError::Network(error),
                        ),
                    )
                })?;
        }
        if time <= segment.end_time {
            break;
        }
    }
    Ok(state)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::experiment_design::{ForcingSegment, ModelCandidate};
    use crate::network::Edge;
    use crate::robust_discrimination::{ModelFamilyAssignment, ProfiledModelFamilyScenario};
    use crate::sensor_design::SensorCandidate;

    fn candidate(id: u32, label: &'static str, bypass: f64) -> ModelCandidate {
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
        if bypass > 0.0 {
            edges.push(Edge {
                left: 0,
                right: 2,
                conductance: bypass,
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
                name: "synthetic_adequacy_noise_calibration",
                models: vec![
                    candidate(1, "chain", 0.0),
                    candidate(2, "bypass_half", 0.5),
                    candidate(3, "bypass_zero_limit", 0.0),
                ],
                sensors: vec![SensorCandidate {
                    id: 30,
                    node: 2,
                    cost: 1.0,
                    common_cause_group: None,
                }],
                noise_covariance: vec![vec![1.0e-5]],
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

    fn config() -> AdequacyCalibrationConfig {
        AdequacyCalibrationConfig {
            calibration_seed: 0xCA11_BA7E,
            validation_seed: 0xB01D_5EED,
            replicates_per_model: 128,
            target_in_family_coverage: 0.95,
        }
    }

    #[test]
    fn threshold_calibration_uses_disjoint_seeds_and_reports_empirical_rates() {
        let problem = scenario();
        let hidden = HiddenOutOfFamilyTruth {
            evaluator_id: 900,
            network: candidate(900, "held_out_bypass_three_quarters", 0.75).network,
        };
        let report = calibrate_model_family_adequacy(
            &problem,
            &pulse(),
            &[30],
            &[hidden],
            config(),
        )
        .unwrap();

        assert!(report.threshold.is_finite());
        assert!(report.threshold > 0.0);
        assert_eq!(report.calibration_samples, 3 * 128);
        assert_eq!(report.validation_samples, 3 * 128);
        assert!(report.calibration_in_family_coverage >= 0.95);
        assert!(report.validation_in_family_coverage >= 0.90);
        assert!(report.validation_false_out_of_model_set_rate <= 0.10);
        assert!(report.out_of_family_rejection_rate >= 0.95);
        assert_eq!(report.calibration_seed, config().calibration_seed);
        assert_eq!(report.validation_seed, config().validation_seed);
    }

    #[test]
    fn threshold_calibration_is_reproducible_for_fixed_seeds() {
        let problem = scenario();
        let hidden = HiddenOutOfFamilyTruth {
            evaluator_id: 900,
            network: candidate(900, "held_out_bypass_three_quarters", 0.75).network,
        };
        let first = calibrate_model_family_adequacy(
            &problem,
            &pulse(),
            &[30],
            &[hidden.clone()],
            config(),
        )
        .unwrap();
        let second = calibrate_model_family_adequacy(
            &problem,
            &pulse(),
            &[30],
            &[hidden],
            config(),
        )
        .unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn calibration_rejects_reused_seeds_and_invalid_coverage_targets() {
        let problem = scenario();
        let mut invalid = config();
        invalid.validation_seed = invalid.calibration_seed;
        assert_eq!(
            calibrate_model_family_adequacy(&problem, &pulse(), &[30], &[], invalid),
            Err(AdequacyCalibrationError::SeedsMustDiffer)
        );

        let mut invalid = config();
        invalid.target_in_family_coverage = 1.0;
        assert_eq!(
            calibrate_model_family_adequacy(&problem, &pulse(), &[30], &[], invalid),
            Err(AdequacyCalibrationError::InvalidTargetCoverage)
        );
    }
}
