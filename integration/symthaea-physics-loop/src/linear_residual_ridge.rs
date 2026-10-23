//! Small, transparent linear baseline for one-step dynamics-residual prediction.
//!
//! This model is intentionally simpler than Symthaea HDC/LTC. It is a mandatory
//! baseline: the temporal HDC/LTC model should not be credited for gains that a
//! well-conditioned linear regressor can achieve at much lower complexity.
//!
//! Inputs are causal state-at-t, the applied/clamped command, and interval duration.
//! Labels are observed-minus-nominal world-frame acceleration residuals. No future
//! state, hidden disturbance force, scenario identifier, seed or split metadata enters
//! the feature vector.

use symthaea_multirotor::{FlightState, QuadrotorCommand};

use crate::transition_residual::{
    LinearTransitionResidual, NominalLinearDynamics, ResidualTargetError,
};

const INPUT_FEATURES: usize = 18;
const MODEL_DIM: usize = INPUT_FEATURES + 1; // unregularized intercept
const OUTPUT_DIM: usize = 3;
const POSITION_SCALE_M: f64 = 0.25;
const LINEAR_VELOCITY_SCALE_M_S: f64 = 5.0;
const ANGULAR_VELOCITY_SCALE_RAD_S: f64 = 5.0;
const DT_SCALE_S: f64 = 0.01;

/// One supervised training example. The state and action are known at decision time;
/// the residual target is derived from the subsequent observed transition.
#[derive(Debug, Clone)]
pub struct ResidualTrainingSample {
    pub state_t: FlightState,
    pub applied_action: QuadrotorCommand,
    pub dt_s: f64,
    pub target_residual_acceleration_world_m_s2: [f64; 3],
}

impl ResidualTrainingSample {
    pub fn from_transition(
        before: &FlightState,
        applied_action: &QuadrotorCommand,
        after: &FlightState,
        nominal: NominalLinearDynamics,
    ) -> Result<Self, ResidualTargetError> {
        let target =
            LinearTransitionResidual::from_transition(before, applied_action, after, nominal)?;
        Ok(Self {
            state_t: before.clone(),
            applied_action: applied_action.clamped(),
            dt_s: target.dt_s,
            target_residual_acceleration_world_m_s2: target.residual_acceleration_world_m_s2,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RidgeError {
    EmptyTrainingSet,
    InvalidRegularization,
    InvalidSample,
    SingularSystem,
}

/// Fixed-feature, three-output ridge regression solved via pivoted Gauss-Jordan
/// elimination. Feature scales are declared constants so evaluation does not estimate
/// normalization from held-out samples. Ridge lambda is applied to feature weights,
/// but not the intercept.
#[derive(Debug, Clone)]
pub struct LinearResidualRidge {
    weights: [[f64; OUTPUT_DIM]; MODEL_DIM],
    ridge_lambda: f64,
    training_sample_count: usize,
}

impl LinearResidualRidge {
    pub fn fit(samples: &[ResidualTrainingSample], ridge_lambda: f64) -> Result<Self, RidgeError> {
        if samples.is_empty() {
            return Err(RidgeError::EmptyTrainingSet);
        }
        if !ridge_lambda.is_finite() || ridge_lambda <= 0.0 {
            return Err(RidgeError::InvalidRegularization);
        }

        let mut gram = [[0.0_f64; MODEL_DIM]; MODEL_DIM];
        let mut cross = [[0.0_f64; OUTPUT_DIM]; MODEL_DIM];
        let inverse_n = 1.0 / samples.len() as f64;

        for sample in samples {
            let x = features(&sample.state_t, &sample.applied_action, sample.dt_s)
                .ok_or(RidgeError::InvalidSample)?;
            if !sample
                .target_residual_acceleration_world_m_s2
                .iter()
                .all(|value| value.is_finite())
            {
                return Err(RidgeError::InvalidSample);
            }

            for row in 0..MODEL_DIM {
                for column in 0..MODEL_DIM {
                    gram[row][column] += x[row] * x[column] * inverse_n;
                }
                for output in 0..OUTPUT_DIM {
                    cross[row][output] +=
                        x[row] * sample.target_residual_acceleration_world_m_s2[output]
                            * inverse_n;
                }
            }
        }

        for index in 1..MODEL_DIM {
            gram[index][index] += ridge_lambda;
        }
        solve_pivoted(&mut gram, &mut cross).ok_or(RidgeError::SingularSystem)?;

        let mut weights = [[0.0_f64; OUTPUT_DIM]; MODEL_DIM];
        for row in 0..MODEL_DIM {
            for output in 0..OUTPUT_DIM {
                weights[row][output] = cross[row][output];
            }
        }

        if !weights.iter().flatten().all(|value| value.is_finite()) {
            return Err(RidgeError::SingularSystem);
        }

        Ok(Self {
            weights,
            ridge_lambda,
            training_sample_count: samples.len(),
        })
    }

    pub fn predict(
        &self,
        state_t: &FlightState,
        applied_action: &QuadrotorCommand,
        dt_s: f64,
    ) -> Result<[f64; 3], RidgeError> {
        let x = features(state_t, applied_action, dt_s).ok_or(RidgeError::InvalidSample)?;
        let mut prediction = [0.0; OUTPUT_DIM];
        for feature in 0..MODEL_DIM {
            for output in 0..OUTPUT_DIM {
                prediction[output] += x[feature] * self.weights[feature][output];
            }
        }
        if !prediction.iter().all(|value| value.is_finite()) {
            return Err(RidgeError::InvalidSample);
        }
        Ok(prediction)
    }

    pub fn training_sample_count(&self) -> usize {
        self.training_sample_count
    }

    pub fn ridge_lambda(&self) -> f64 {
        self.ridge_lambda
    }
}

fn features(
    state: &FlightState,
    action: &QuadrotorCommand,
    dt_s: f64,
) -> Option<[f64; MODEL_DIM]> {
    if !state.timestamp.is_finite()
        || !state.position.iter().all(|value| value.is_finite())
        || !state.linear_velocity.iter().all(|value| value.is_finite())
        || !state.angular_velocity.iter().all(|value| value.is_finite())
        || !state.quaternion.iter().all(|value| value.is_finite())
        || !action.to_ctrl().iter().all(|value| value.is_finite())
        || !dt_s.is_finite()
        || dt_s <= 0.0
    {
        return None;
    }

    let quaternion_norm = state.quaternion.iter().map(|v| v * v).sum::<f64>().sqrt();
    if !quaternion_norm.is_finite() || quaternion_norm < 1.0e-9 {
        return None;
    }
    let q = state.quaternion.map(|value| value / quaternion_norm);
    let applied = action.clamped();

    let values = [
        1.0,
        state.position[0] / POSITION_SCALE_M,
        state.position[1] / POSITION_SCALE_M,
        state.position[2] / POSITION_SCALE_M,
        q[0],
        q[1],
        q[2],
        q[3],
        state.linear_velocity[0] / LINEAR_VELOCITY_SCALE_M_S,
        state.linear_velocity[1] / LINEAR_VELOCITY_SCALE_M_S,
        state.linear_velocity[2] / LINEAR_VELOCITY_SCALE_M_S,
        state.angular_velocity[0] / ANGULAR_VELOCITY_SCALE_RAD_S,
        state.angular_velocity[1] / ANGULAR_VELOCITY_SCALE_RAD_S,
        state.angular_velocity[2] / ANGULAR_VELOCITY_SCALE_RAD_S,
        applied.thrust as f64 / QuadrotorCommand::MAX_THRUST as f64,
        applied.roll_moment as f64 / QuadrotorCommand::MAX_MOMENT_RP as f64,
        applied.pitch_moment as f64 / QuadrotorCommand::MAX_MOMENT_RP as f64,
        applied.yaw_moment as f64 / QuadrotorCommand::MAX_MOMENT_YAW as f64,
        dt_s / DT_SCALE_S,
    ];

    if values.iter().all(|value| value.is_finite()) {
        Some(values)
    } else {
        None
    }
}

/// In-place Gauss-Jordan solve with partial pivoting and three right-hand sides.
fn solve_pivoted(
    matrix: &mut [[f64; MODEL_DIM]; MODEL_DIM],
    rhs: &mut [[f64; OUTPUT_DIM]; MODEL_DIM],
) -> Option<()> {
    for pivot_column in 0..MODEL_DIM {
        let mut pivot_row = pivot_column;
        let mut pivot_abs = matrix[pivot_row][pivot_column].abs();
        for row in (pivot_column + 1)..MODEL_DIM {
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
        for column in pivot_column..MODEL_DIM {
            matrix[pivot_column][column] /= pivot;
        }
        for output in 0..OUTPUT_DIM {
            rhs[pivot_column][output] /= pivot;
        }

        for row in 0..MODEL_DIM {
            if row == pivot_column {
                continue;
            }
            let factor = matrix[row][pivot_column];
            if factor == 0.0 {
                continue;
            }
            for column in pivot_column..MODEL_DIM {
                matrix[row][column] -= factor * matrix[pivot_column][column];
            }
            for output in 0..OUTPUT_DIM {
                rhs[row][output] -= factor * rhs[pivot_column][output];
            }
        }
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_sample(index: usize) -> ResidualTrainingSample {
        let i = index as f64;
        let roll = 0.35 * (i * 0.13).sin();
        let pitch = 0.3 * (i * 0.19).cos();
        let yaw = 0.4 * (i * 0.07).sin();
        let (sr, cr) = (0.5 * roll).sin_cos();
        let (sp, cp) = (0.5 * pitch).sin_cos();
        let (sy, cy) = (0.5 * yaw).sin_cos();
        let state_t = FlightState {
            position: [
                0.12 * (i * 0.17).sin(),
                0.1 * (i * 0.11).cos(),
                0.1 + 0.08 * (i * 0.09).sin(),
            ],
            quaternion: [
                cr * cp * cy + sr * sp * sy,
                sr * cp * cy - cr * sp * sy,
                cr * sp * cy + sr * cp * sy,
                cr * cp * sy - sr * sp * cy,
            ],
            linear_velocity: [
                2.0 * (i * 0.21).sin(),
                1.7 * (i * 0.14).cos(),
                1.2 * (i * 0.08).sin(),
            ],
            angular_velocity: [
                3.0 * (i * 0.1).sin(),
                2.5 * (i * 0.16).cos(),
                1.8 * (i * 0.23).sin(),
            ],
            timestamp: i * 0.002,
        };
        let action = QuadrotorCommand::from_raw([
            0.25 + 0.1 * (i * 0.12).sin() as f32,
            0.003 * (i * 0.15).sin() as f32,
            0.003 * (i * 0.18).cos() as f32,
            0.001 * (i * 0.24).sin() as f32,
        ]);
        let dt_s = 0.0015 + (index % 11) as f64 * 0.0001;
        let x = features(&state_t, &action, dt_s).expect("synthetic feature is valid");
        let mut target = [0.0; OUTPUT_DIM];
        for output in 0..OUTPUT_DIM {
            target[output] = 0.04 * (output as f64 + 1.0);
            for feature in 1..MODEL_DIM {
                let coefficient =
                    ((feature * 7 + output * 3) as f64).sin() * 0.012 / (feature as f64).sqrt();
                target[output] += coefficient * x[feature];
            }
        }
        ResidualTrainingSample {
            state_t,
            applied_action: action,
            dt_s,
            target_residual_acceleration_world_m_s2: target,
        }
    }

    #[test]
    fn ridge_model_recovers_a_synthetic_causal_linear_residual() {
        let train = (0..180).map(synthetic_sample).collect::<Vec<_>>();
        let heldout = (181..230).map(synthetic_sample).collect::<Vec<_>>();
        let model = LinearResidualRidge::fit(&train, 1.0e-8).expect("valid ridge fit");
        assert_eq!(model.training_sample_count(), train.len());

        let mut squared_error = 0.0;
        let mut zero_baseline_squared_error = 0.0;
        let mut count = 0usize;
        for sample in heldout {
            let prediction = model
                .predict(&sample.state_t, &sample.applied_action, sample.dt_s)
                .expect("valid prediction");
            for axis in 0..OUTPUT_DIM {
                squared_error += (prediction[axis]
                    - sample.target_residual_acceleration_world_m_s2[axis])
                    .powi(2);
                zero_baseline_squared_error +=
                    sample.target_residual_acceleration_world_m_s2[axis].powi(2);
                count += 1;
            }
        }
        let rmse = (squared_error / count as f64).sqrt();
        let zero_rmse = (zero_baseline_squared_error / count as f64).sqrt();
        assert!(rmse < 1.0e-4, "linear fit RMSE was {rmse}");
        assert!(
            rmse < zero_rmse * 0.02,
            "ridge RMSE {rmse} should strongly beat zero-residual baseline {zero_rmse}"
        );
    }

    #[test]
    fn invalid_training_data_and_regularization_fail_closed() {
        assert_eq!(
            LinearResidualRidge::fit(&[], 1.0e-3).err(),
            Some(RidgeError::EmptyTrainingSet)
        );
        let train = (0..40).map(synthetic_sample).collect::<Vec<_>>();
        assert_eq!(
            LinearResidualRidge::fit(&train, 0.0).err(),
            Some(RidgeError::InvalidRegularization)
        );

        let mut bad = synthetic_sample(3);
        bad.target_residual_acceleration_world_m_s2[1] = f64::NAN;
        let bad_set = vec![bad];
        assert_eq!(
            LinearResidualRidge::fit(&bad_set, 1.0e-3).err(),
            Some(RidgeError::InvalidSample)
        );
    }

    #[test]
    fn prediction_rejects_invalid_quaternion_and_time_interval() {
        let train = (0..60).map(synthetic_sample).collect::<Vec<_>>();
        let model = LinearResidualRidge::fit(&train, 1.0e-5).expect("valid ridge fit");
        let mut state = FlightState::hover(0.1);
        state.quaternion = [0.0; 4];
        assert_eq!(
            model.predict(&state, &QuadrotorCommand::hover(), 0.002).err(),
            Some(RidgeError::InvalidSample)
        );
        assert_eq!(
            model
                .predict(&FlightState::hover(0.1), &QuadrotorCommand::hover(), 0.0)
                .err(),
            Some(RidgeError::InvalidSample)
        );
    }
}
