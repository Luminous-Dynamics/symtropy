//! Deterministic synthetic benchmark generator for physical-vs-measurement
//! attribution.
//!
//! The generator separates model-facing observations/designs from latent truth.
//! It supports orthogonal, partially confounded and exactly aliased mechanism
//! signatures. Exact aliases are deliberate negative-qualification cases:
//! the correct scientific response is non-identifiability, not a forced label.
//!
//! These are abstract synthetic data, not nuclear measurements or an operating
//! procedure.

use crate::horseshoe_hmc::SplitMix64;
use crate::ModelError;

/// Geometry between the physical-mechanism and measurement-effect signatures.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AttributionConfounding {
    Orthogonal,
    /// Correlation between the normalized physical and measurement energy bases.
    Partial {
        correlation: f64,
    },
    /// Identical energy signatures and constant experiment feature: only the sum
    /// of the two amplitudes is observable.
    ExactAlias,
}

/// Frozen generator configuration. Input vectors are part of the benchmark
/// manifest and should be hashed before model fitting.
#[derive(Debug, Clone, PartialEq)]
pub struct BlindAttributionConfig {
    pub energy_grid_mev: Vec<f64>,
    pub experiment_feature_values: Vec<f64>,
    pub physical_amplitude: f64,
    pub measurement_amplitude: f64,
    pub log_noise_standard_deviation: f64,
    pub confounding: AttributionConfounding,
    pub seed: u64,
}

/// Model-facing data. This struct intentionally contains no latent mechanism
/// labels or amplitudes.
#[derive(Debug, Clone, PartialEq)]
pub struct BlindAttributionPublicData {
    pub energy_grid_mev: Vec<f64>,
    pub experiment_feature_values: Vec<f64>,
    /// Row-major order: experiment-major, then energy bin.
    pub observations: Vec<f64>,
    pub baseline_mean: Vec<f64>,
    /// One physical-mechanism basis column per observation.
    pub physical_design: Vec<Vec<f64>>,
    /// One feature-conditioned measurement-bias basis column per observation.
    pub measurement_design: Vec<Vec<f64>>,
    /// Experiment index corresponding to each observation row.
    pub experiment_index_by_observation: Vec<usize>,
    pub observation_count: usize,
}

/// Latent truth. Store this separately from model input in blind benchmark
/// execution; do not reveal it to fitting, feature selection or model selection.
#[derive(Debug, Clone, PartialEq)]
pub struct BlindAttributionTruth {
    pub physical_amplitude: f64,
    pub measurement_amplitude: f64,
    pub log_noise_standard_deviation: f64,
    pub confounding: AttributionConfounding,
    pub exact_observational_alias: bool,
    pub seed: u64,
}

/// Complete deterministic fixture. The `public_data` and `truth` fields
/// should be materialized to separate artifacts before running blind evaluation.
#[derive(Debug, Clone, PartialEq)]
pub struct BlindAttributionDataset {
    pub public_data: BlindAttributionPublicData,
    pub truth: BlindAttributionTruth,
}

/// Generate a positive synthetic spectrum with controlled physical and
/// measurement contributions and lognormal observation noise.
pub fn generate_blind_attribution_dataset(
    config: &BlindAttributionConfig,
) -> Result<BlindAttributionDataset, ModelError> {
    let energy_count = config.energy_grid_mev.len();
    let experiment_count = config.experiment_feature_values.len();
    if energy_count < 2 || experiment_count < 2 {
        return Err(ModelError::MismatchedDataLengths);
    }
    if config
        .energy_grid_mev
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
        || config
            .energy_grid_mev
            .windows(2)
            .any(|pair| pair[1] <= pair[0])
    {
        return Err(ModelError::InvalidParameter(
            "blind_attribution_energy_grid_must_be_positive_strictly_increasing",
        ));
    }
    if config
        .experiment_feature_values
        .iter()
        .any(|value| !value.is_finite())
    {
        return Err(ModelError::NonFiniteValue(
            "blind_attribution_experiment_features",
        ));
    }
    for (value, name) in [
        (config.physical_amplitude, "blind_physical_amplitude"),
        (config.measurement_amplitude, "blind_measurement_amplitude"),
        (
            config.log_noise_standard_deviation,
            "blind_log_noise_standard_deviation",
        ),
    ] {
        if !value.is_finite() {
            return Err(ModelError::NonFiniteValue(name));
        }
    }
    if config.log_noise_standard_deviation <= 0.0 {
        return Err(ModelError::NonPositiveValue(
            "blind_log_noise_standard_deviation",
        ));
    }
    if let AttributionConfounding::Partial { correlation } = config.confounding {
        if !correlation.is_finite() {
            return Err(ModelError::NonFiniteValue(
                "blind_attribution_basis_correlation",
            ));
        }
        if !(-1.0..=1.0).contains(&correlation) {
            return Err(ModelError::InvalidParameter(
                "blind_attribution_basis_correlation_out_of_range",
            ));
        }
        if correlation.abs() >= 1.0 {
            return Err(ModelError::InvalidParameter(
                "partial_confounding_requires_abs_correlation_below_one",
            ));
        }
    }
    if config.confounding == AttributionConfounding::ExactAlias
        && config
            .experiment_feature_values
            .iter()
            .any(|value| (*value - 1.0).abs() > 1.0e-12)
    {
        return Err(ModelError::InvalidParameter(
            "exact_alias_requires_constant_unit_experiment_features",
        ));
    }

    let physical_basis = normalized_centered_log_energy_basis(&config.energy_grid_mev)?;
    let orthogonal_basis = orthogonal_energy_basis(&physical_basis)?;
    let measurement_basis = match config.confounding {
        AttributionConfounding::Orthogonal => orthogonal_basis,
        AttributionConfounding::Partial { correlation } => physical_basis
            .iter()
            .zip(orthogonal_basis.iter())
            .map(|(physical, orthogonal)| {
                correlation * physical + (1.0 - correlation * correlation).sqrt() * orthogonal
            })
            .collect(),
        AttributionConfounding::ExactAlias => physical_basis.clone(),
    };

    let observation_count =
        energy_count
            .checked_mul(experiment_count)
            .ok_or(ModelError::InvalidParameter(
                "blind_attribution_observation_count_overflow",
            ))?;
    let mut observations = Vec::with_capacity(observation_count);
    let mut baseline_mean = Vec::with_capacity(observation_count);
    let mut physical_design = Vec::with_capacity(observation_count);
    let mut measurement_design = Vec::with_capacity(observation_count);
    let mut experiment_index_by_observation = Vec::with_capacity(observation_count);

    let mut rng = SplitMix64::new(config.seed);
    let sigma = config.log_noise_standard_deviation;
    // The -0.5*sigma^2 correction makes the multiplicative lognormal noise
    // have unit expectation and keeps the declared noiseless mean interpretable.
    let lognormal_mean_correction = -0.5 * sigma * sigma;

    for (experiment_index, feature_value) in config.experiment_feature_values.iter().enumerate() {
        for (energy_index, energy) in config.energy_grid_mev.iter().enumerate() {
            let baseline = energy.sqrt() * (-energy / 1.42).exp();
            if !baseline.is_finite() || baseline <= 0.0 {
                return Err(ModelError::InvalidParameter(
                    "blind_attribution_baseline_mean_invalid",
                ));
            }
            let physical_component = config.physical_amplitude * physical_basis[energy_index];
            let measurement_component =
                config.measurement_amplitude * feature_value * measurement_basis[energy_index];
            let expected_mean = baseline * (physical_component + measurement_component).exp();
            let noise = sigma * rng.normal() + lognormal_mean_correction;
            let observed = expected_mean * noise.exp();
            if !expected_mean.is_finite()
                || expected_mean <= 0.0
                || !observed.is_finite()
                || observed <= 0.0
            {
                return Err(ModelError::InvalidParameter(
                    "blind_attribution_observation_nonfinite_or_nonpositive",
                ));
            }

            observations.push(observed);
            baseline_mean.push(baseline);
            physical_design.push(vec![physical_basis[energy_index]]);
            measurement_design.push(vec![feature_value * measurement_basis[energy_index]]);
            experiment_index_by_observation.push(experiment_index);
        }
    }

    Ok(BlindAttributionDataset {
        public_data: BlindAttributionPublicData {
            energy_grid_mev: config.energy_grid_mev.clone(),
            experiment_feature_values: config.experiment_feature_values.clone(),
            observations,
            baseline_mean,
            physical_design,
            measurement_design,
            experiment_index_by_observation,
            observation_count,
        },
        truth: BlindAttributionTruth {
            physical_amplitude: config.physical_amplitude,
            measurement_amplitude: config.measurement_amplitude,
            log_noise_standard_deviation: config.log_noise_standard_deviation,
            confounding: config.confounding,
            exact_observational_alias: config.confounding == AttributionConfounding::ExactAlias,
            seed: config.seed,
        },
    })
}

fn normalized_centered_log_energy_basis(energies: &[f64]) -> Result<Vec<f64>, ModelError> {
    let logs: Vec<f64> = energies.iter().map(|energy| energy.ln()).collect();
    let mean = logs.iter().sum::<f64>() / logs.len() as f64;
    let mut basis: Vec<f64> = logs.iter().map(|value| value - mean).collect();
    normalize_basis(&mut basis, "physical_log_energy_basis")?;
    Ok(basis)
}

fn orthogonal_energy_basis(physical: &[f64]) -> Result<Vec<f64>, ModelError> {
    let mut candidate: Vec<f64> = (0..physical.len())
        .map(|index| ((index + 1) as f64 * 1.618_033_988_749_895).sin())
        .collect();
    let mean = candidate.iter().sum::<f64>() / candidate.len() as f64;
    for value in &mut candidate {
        *value -= mean;
    }
    let projection = candidate
        .iter()
        .zip(physical.iter())
        .map(|(left, right)| left * right)
        .sum::<f64>();
    for (value, direction) in candidate.iter_mut().zip(physical.iter()) {
        *value -= projection * direction;
    }
    normalize_basis(&mut candidate, "orthogonal_measurement_energy_basis")?;
    Ok(candidate)
}

fn normalize_basis(values: &mut [f64], name: &'static str) -> Result<(), ModelError> {
    let norm = values.iter().map(|value| value * value).sum::<f64>().sqrt();
    if !norm.is_finite() || norm <= 1.0e-12 {
        return Err(ModelError::InvalidParameter(name));
    }
    for value in values {
        *value /= norm;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(confounding: AttributionConfounding) -> BlindAttributionConfig {
        BlindAttributionConfig {
            energy_grid_mev: vec![0.2, 0.4, 0.8, 1.6, 3.2, 6.4],
            experiment_feature_values: if confounding == AttributionConfounding::ExactAlias {
                vec![1.0, 1.0, 1.0]
            } else {
                vec![-1.0, 0.0, 1.0]
            },
            physical_amplitude: 0.12,
            measurement_amplitude: 0.08,
            log_noise_standard_deviation: 0.01,
            confounding,
            seed: 0xC0FFEE,
        }
    }

    #[test]
    fn fixed_seed_reproduces_identical_public_observations_and_truth() {
        let config = fixture(AttributionConfounding::Orthogonal);
        let first = generate_blind_attribution_dataset(&config).expect("first fixture");
        let second = generate_blind_attribution_dataset(&config).expect("second fixture");
        assert_eq!(first, second);
        assert_eq!(first.public_data.observation_count, 18);
        assert!(first
            .public_data
            .observations
            .iter()
            .all(|value| *value > 0.0));
    }

    #[test]
    fn orthogonal_design_has_zero_basis_inner_product() {
        let data = generate_blind_attribution_dataset(&fixture(AttributionConfounding::Orthogonal))
            .expect("orthogonal fixture");
        let inner: f64 = data
            .public_data
            .physical_design
            .iter()
            .zip(data.public_data.measurement_design.iter())
            .filter(|(physical, measurement)| physical[0].abs() > 0.0 && measurement[0] != 0.0)
            .map(|(physical, measurement)| physical[0] * measurement[0])
            .sum();
        // The three experiment features (-1,0,+1) cancel in the full stacked
        // inner product; within an experiment, the energy bases are orthogonal.
        assert!(inner.abs() < 1.0e-10);
    }

    #[test]
    fn partial_confounding_has_the_requested_energy_basis_correlation() {
        let rho = 0.6;
        let data = generate_blind_attribution_dataset(&fixture(AttributionConfounding::Partial {
            correlation: rho,
        }))
        .expect("partial fixture");
        let physical: Vec<f64> = data.public_data.physical_design
            [..data.public_data.energy_grid_mev.len()]
            .iter()
            .map(|row| row[0])
            .collect();
        let measurement: Vec<f64> = data.public_data.measurement_design
            [..data.public_data.energy_grid_mev.len()]
            .iter()
            .map(|row| row[0])
            .collect();
        let inner: f64 = physical
            .iter()
            .zip(measurement.iter())
            .map(|(left, right)| left * right)
            .sum();
        let feature = data.public_data.experiment_feature_values[0];
        assert!((inner - rho * feature).abs() < 1.0e-10);
    }

    #[test]
    fn exact_alias_emits_identical_public_design_columns() {
        let data = generate_blind_attribution_dataset(&fixture(AttributionConfounding::ExactAlias))
            .expect("exact-alias fixture");
        assert!(data.truth.exact_observational_alias);
        assert_eq!(
            data.public_data.physical_design,
            data.public_data.measurement_design
        );
    }

    #[test]
    fn exact_alias_requires_constant_unit_experiment_features() {
        let mut config = fixture(AttributionConfounding::ExactAlias);
        config.experiment_feature_values[1] = 0.0;
        assert_eq!(
            generate_blind_attribution_dataset(&config),
            Err(ModelError::InvalidParameter(
                "exact_alias_requires_constant_unit_experiment_features",
            ))
        );
    }
}
