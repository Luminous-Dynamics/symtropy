//! Frozen Symthaea HDC-LTC temporal reservoir with a lightweight ridge readout.
//!
//! The reservoir and HDC encoder are initialized deterministically from a GenesisSeed
//! and remain frozen. Only a small ridge-regression readout is fit to causal residual
//! targets. This tests whether HDC/LTC temporal features help over the raw-state ridge
//! baseline; it is NOT end-to-end HDC/LTC training or a learned uncertainty model.

use symthaea_core::genesis::GenesisSeed;
use symthaea_core::hdc::{
    HDC_DIMENSION, HdcLtcUnifiedNetwork, UnifiedConfig, UnifiedNetworkConfig,
};
use symthaea_multirotor::{FlightState, QuadrotorCommand, QuadrotorHdcEncoder};

const PROJECTION_DIM: usize = 256;
const ACTION_FEATURES: usize = 4;
const DT_FEATURES: usize = 1;
const FEATURE_DIM: usize = 1 + PROJECTION_DIM + ACTION_FEATURES + DT_FEATURES;
const DT_SCALE_S: f64 = 0.01;
const ACTION_FEATURE_SCALE: f64 = 0.25;
const PROJECTION_SEED: u64 = 0x4844_434c_5443_0001;

/// Metadata needed to fit a readout. Features are already projected from the frozen
/// HDC-LTC representation and joined with the known action and interval duration.
#[derive(Debug, Clone)]
pub struct ReservoirTrainingSample {
    pub features: [f64; FEATURE_DIM],
    pub target_residual_acceleration_world_m_s2: [f64; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReservoirError {
    EmptyTrainingSet,
    InvalidRegularization,
    InvalidFeaturesOrTarget,
    SingularSystem,
}

/// Stateful, deterministic HDC-LTC reservoir. Reset between episodes, never between
/// adjacent steps within an episode.
pub struct HdcLtcResidualReservoir {
    encoder: QuadrotorHdcEncoder,
    network: HdcLtcUnifiedNetwork,
}

impl HdcLtcResidualReservoir {
    pub fn new(genesis_phrase: &str) -> Self {
        let genesis = GenesisSeed::from_phrase(genesis_phrase);
        let encoder = QuadrotorHdcEncoder::new(&genesis, 32);
        let neuron_config = UnifiedConfig {
            tau_base: 0.05,
            backbone_tau: 0.3,
            dimension: HDC_DIMENSION,
            learning_rate: 0.001,
            ..UnifiedConfig::default()
        };
        // Deliberately small frozen reservoir for a controlled first ablation.
        // The raw-state ridge remains the low-complexity baseline.
        let network_config = UnifiedNetworkConfig {
            layer_sizes: vec![2, 2],
            neuron_config,
            use_layer_binding: true,
            skip_connections: false,
        };
        let network = HdcLtcUnifiedNetwork::from_genesis(network_config, &genesis);
        Self { encoder, network }
    }

    /// Reset only recurrent/derivative state at episode boundaries. Genesis-derived
    /// encoder codebooks and frozen network weights remain unchanged.
    pub fn reset_episode(&mut self) {
        self.encoder.reset();
        self.network.reset();
    }

    /// Reset only the LTC recurrent state while preserving the HDC encoder's derivative
    /// history. Use this for the recurrent-memory ablation: it removes accumulated LTC
    /// state without changing how the encoder represents transitions within an episode.
    pub fn reset_reservoir_state(&mut self) {
        self.network.reset();
    }

    /// Encode causal state/action/time into a fixed-size readout feature vector.
    ///
    /// Call exactly once per transition, in chronological order within each episode.
    /// The output excludes all next-state and target data. Applied action and dt are
    /// included explicitly because the state encoder itself encodes state channels only.
    pub fn features(
        &mut self,
        state_t: &FlightState,
        applied_action: &QuadrotorCommand,
        dt_s: f64,
    ) -> Option<[f64; FEATURE_DIM]> {
        if !state_t.is_finite()
            || !state_t.timestamp.is_finite()
            || !dt_s.is_finite()
            || dt_s <= 0.0
            || !applied_action.to_ctrl().iter().all(|value| value.is_finite())
        {
            return None;
        }

        let quaternion_norm = state_t
            .quaternion
            .iter()
            .map(|value| value * value)
            .sum::<f64>()
            .sqrt();
        if !quaternion_norm.is_finite() || quaternion_norm < 1.0e-9 {
            return None;
        }

        let sensor_hv = self.encoder.encode(state_t);
        self.network.evolve_closed_form(dt_s as f32, &sensor_hv);
        let reservoir_output = self.network.output().normalize();
        let output_values = reservoir_output.as_slice();
        if output_values.len() != HDC_DIMENSION
            || !output_values.iter().all(|value| value.is_finite())
        {
            return None;
        }

        let mut features = [0.0_f64; FEATURE_DIM];
        features[0] = 1.0;
        project_hypervector(output_values, &mut features[1..1 + PROJECTION_DIM])?;

        let action = applied_action.clamped();
        let action_start = 1 + PROJECTION_DIM;
        features[action_start] =
            action.thrust as f64 / QuadrotorCommand::MAX_THRUST as f64 * ACTION_FEATURE_SCALE;
        features[action_start + 1] = action.roll_moment as f64
            / QuadrotorCommand::MAX_MOMENT_RP as f64
            * ACTION_FEATURE_SCALE;
        features[action_start + 2] = action.pitch_moment as f64
            / QuadrotorCommand::MAX_MOMENT_RP as f64
            * ACTION_FEATURE_SCALE;
        features[action_start + 3] = action.yaw_moment as f64
            / QuadrotorCommand::MAX_MOMENT_YAW as f64
            * ACTION_FEATURE_SCALE;
        features[action_start + ACTION_FEATURES] = dt_s / DT_SCALE_S * ACTION_FEATURE_SCALE;

        features.iter().all(|value| value.is_finite()).then_some(features)
    }
}

fn mix_u64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

/// Fixed CountSketch projection: bucket and sign depend only on source dimension and
/// a versioned seed; no fitted/test-derived projection state exists.
fn project_hypervector(source: &[f32], target: &mut [f64]) -> Option<()> {
    if source.is_empty()
        || target.is_empty()
        || !source.iter().all(|value| value.is_finite())
    {
        return None;
    }
    target.fill(0.0);
    for (index, value) in source.iter().enumerate() {
        let hash = mix_u64((index as u64) ^ PROJECTION_SEED);
        let bucket = (hash as usize) % target.len();
        let sign = if (hash >> 63) == 0 { 1.0 } else { -1.0 };
        target[bucket] += sign * (*value as f64);
    }
    target.iter().all(|value| value.is_finite()).then_some(())
}

#[derive(Debug, Clone)]
pub struct ReservoirRidgeReadout {
    weights: [[f64; 3]; FEATURE_DIM],
    ridge_lambda: f64,
    training_sample_count: usize,
}

impl ReservoirRidgeReadout {
    pub fn fit(
        samples: &[ReservoirTrainingSample],
        ridge_lambda: f64,
    ) -> Result<Self, ReservoirError> {
        if samples.is_empty() {
            return Err(ReservoirError::EmptyTrainingSet);
        }
        if !ridge_lambda.is_finite() || ridge_lambda <= 0.0 {
            return Err(ReservoirError::InvalidRegularization);
        }

        let mut gram = [[0.0_f64; FEATURE_DIM]; FEATURE_DIM];
        let mut cross = [[0.0_f64; 3]; FEATURE_DIM];
        let inverse_n = 1.0 / samples.len() as f64;
        for sample in samples {
            if !sample.features.iter().all(|value| value.is_finite())
                || !sample
                    .target_residual_acceleration_world_m_s2
                    .iter()
                    .all(|value| value.is_finite())
            {
                return Err(ReservoirError::InvalidFeaturesOrTarget);
            }
            for row in 0..FEATURE_DIM {
                for column in 0..FEATURE_DIM {
                    gram[row][column] +=
                        sample.features[row] * sample.features[column] * inverse_n;
                }
                for output in 0..3 {
                    cross[row][output] += sample.features[row]
                        * sample.target_residual_acceleration_world_m_s2[output]
                        * inverse_n;
                }
            }
        }

        // Do not penalize the fixed intercept.
        for feature in 1..FEATURE_DIM {
            gram[feature][feature] += ridge_lambda;
        }
        solve_pivoted(&mut gram, &mut cross).ok_or(ReservoirError::SingularSystem)?;

        let mut weights = [[0.0_f64; 3]; FEATURE_DIM];
        for feature in 0..FEATURE_DIM {
            for output in 0..3 {
                weights[feature][output] = cross[feature][output];
            }
        }
        if !weights.iter().flatten().all(|value| value.is_finite()) {
            return Err(ReservoirError::SingularSystem);
        }

        Ok(Self {
            weights,
            ridge_lambda,
            training_sample_count: samples.len(),
        })
    }

    pub fn predict(&self, features: &[f64; FEATURE_DIM]) -> Option<[f64; 3]> {
        if !features.iter().all(|value| value.is_finite()) {
            return None;
        }
        let mut output = [0.0; 3];
        for feature in 0..FEATURE_DIM {
            for axis in 0..3 {
                output[axis] += features[feature] * self.weights[feature][axis];
            }
        }
        output.iter().all(|value| value.is_finite()).then_some(output)
    }

    pub fn ridge_lambda(&self) -> f64 {
        self.ridge_lambda
    }

    pub fn training_sample_count(&self) -> usize {
        self.training_sample_count
    }
}

fn solve_pivoted(
    matrix: &mut [[f64; FEATURE_DIM]; FEATURE_DIM],
    rhs: &mut [[f64; 3]; FEATURE_DIM],
) -> Option<()> {
    for pivot_column in 0..FEATURE_DIM {
        let mut pivot_row = pivot_column;
        let mut pivot_abs = matrix[pivot_column][pivot_column].abs();
        for row in pivot_column + 1..FEATURE_DIM {
            let candidate = matrix[row][pivot_column].abs();
            if candidate > pivot_abs {
                pivot_abs = candidate;
                pivot_row = row;
            }
        }
        if !pivot_abs.is_finite() || pivot_abs < 1.0e-12 {
            return None;
        }
        if pivot_row != pivot_column {
            matrix.swap(pivot_row, pivot_column);
            rhs.swap(pivot_row, pivot_column);
        }

        let pivot = matrix[pivot_column][pivot_column];
        for column in pivot_column..FEATURE_DIM {
            matrix[pivot_column][column] /= pivot;
        }
        for output in 0..3 {
            rhs[pivot_column][output] /= pivot;
        }

        for row in 0..FEATURE_DIM {
            if row == pivot_column {
                continue;
            }
            let factor = matrix[row][pivot_column];
            if factor == 0.0 {
                continue;
            }
            for column in pivot_column..FEATURE_DIM {
                matrix[row][column] -= factor * matrix[pivot_column][column];
            }
            for output in 0..3 {
                rhs[row][output] -= factor * rhs[pivot_column][output];
            }
        }
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_projection_is_deterministic_and_preserves_finite_values() {
        let source = (0..HDC_DIMENSION)
            .map(|index| ((index as f32) * 0.001).sin())
            .collect::<Vec<_>>();
        let mut a = [0.0; PROJECTION_DIM];
        let mut b = [0.0; PROJECTION_DIM];
        assert!(project_hypervector(&source, &mut a).is_some());
        assert!(project_hypervector(&source, &mut b).is_some());
        assert_eq!(a, b);
        assert!(a.iter().any(|value| value.abs() > 0.0));
    }

    #[test]
    fn projection_rejects_non_finite_and_empty_inputs() {
        let mut projected = [0.0; PROJECTION_DIM];
        assert!(project_hypervector(&[], &mut projected).is_none());
        let mut source = vec![0.0_f32; HDC_DIMENSION];
        source[10] = f32::NAN;
        assert!(project_hypervector(&source, &mut projected).is_none());
        assert!(project_hypervector(&vec![0.0_f32; 4], &mut []).is_none());
    }

    fn synthetic_readout_sample(index: usize) -> ReservoirTrainingSample {
        let i = index as f64;
        let mut features = [0.0_f64; FEATURE_DIM];
        features[0] = 1.0;
        for feature in 1..FEATURE_DIM {
            let j = feature as f64;
            features[feature] =
                (i * (j + 3.0) * 0.037).sin() + ((i + 5.0) * (j + 1.0) * 0.011).cos();
        }
        let target = [
            0.02 + 0.05 * features[1] - 0.03 * features[17] + 0.04 * features[259],
            -0.01 + 0.02 * features[5] + 0.03 * features[91],
            0.03 - 0.04 * features[12] + 0.02 * features[256],
        ];
        ReservoirTrainingSample {
            features,
            target_residual_acceleration_world_m_s2: target,
        }
    }

    #[test]
    fn ridge_readout_recovers_known_linear_mapping() {
        let train = (0..320)
            .map(synthetic_readout_sample)
            .collect::<Vec<_>>();
        let heldout = (321..361)
            .map(synthetic_readout_sample)
            .collect::<Vec<_>>();
        let readout = ReservoirRidgeReadout::fit(&train, 1.0e-6)
            .expect("synthetic ridge readout fit should succeed");
        let mut squared_error = 0.0;
        let mut zero_squared_error = 0.0;
        let mut count = 0usize;
        for sample in heldout {
            let predicted = readout
                .predict(&sample.features)
                .expect("finite features should yield finite predictions");
            for axis in 0..3 {
                let target = sample.target_residual_acceleration_world_m_s2[axis];
                squared_error += (target - predicted[axis]).powi(2);
                zero_squared_error += target.powi(2);
                count += 1;
            }
        }
        let rmse = (squared_error / count as f64).sqrt();
        let zero_rmse = (zero_squared_error / count as f64).sqrt();
        assert!(rmse < 1.0e-3, "synthetic ridge RMSE was {rmse}");
        assert!(rmse < zero_rmse * 0.02, "ridge must beat zero baseline");
    }

    #[test]
    fn readout_fails_closed_on_invalid_training_samples() {
        assert_eq!(
            ReservoirRidgeReadout::fit(&[], 1.0e-3).err(),
            Some(ReservoirError::EmptyTrainingSet)
        );
        let mut features = [0.0; FEATURE_DIM];
        features[0] = 1.0;
        let mut sample = ReservoirTrainingSample {
            features,
            target_residual_acceleration_world_m_s2: [0.0; 3],
        };
        sample.target_residual_acceleration_world_m_s2[1] = f64::INFINITY;
        assert_eq!(
            ReservoirRidgeReadout::fit(&[sample], 1.0e-3).err(),
            Some(ReservoirError::InvalidFeaturesOrTarget)
        );
    }
}
