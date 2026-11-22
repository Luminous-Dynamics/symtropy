//! HBR-PGD bias-coupled trajectory-refinement (BCTR) objective and gradient.
//!
//! Implements the 2D single-tag local-window objective from the HBR-PGD paper:
//! - packet-level generalized adaptive robust loss over normalized residuals;
//! - bounded nonnegative anchor-channel structural bias states;
//! - velocity and acceleration smoothness;
//! - bias amplitude and temporal-drift regularization.
//!
//! This module evaluates the objective and analytic gradients and provides
//! one projected gradient step. It does not implement overlapping-window
//! construction, curvature-weighted window fusion, author-specific stopping
//! rules or calibration. Those remain separate reproduction tasks.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HbrRangeObservation2d {
    pub measurement_id: u64,
    pub time_index: usize,
    /// Stable anchor-channel group index in [0, group_count).
    pub bias_group_index: usize,
    pub anchor_id: u32,
    pub channel_id: u8,
    pub anchor_position_m: [f64; 2],
    pub measured_range_m: f64,
    pub heteroscedastic_scale_m: f64,
    pub robust_loss_shape_alpha: f64,
    pub nonnegative_correction_prior_m: f64,
    pub soft_gate_weight: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HbrBctrState2d {
    pub positions_m: Vec<[f64; 2]>,
    /// Row-major (time_index * group_count + bias_group_index).
    pub structural_bias_m: Vec<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HbrBctrConfig2d {
    pub group_count: usize,
    pub sigma_epsilon_m: f64,
    pub lambda_velocity: f64,
    pub lambda_acceleration: f64,
    pub lambda_bias: f64,
    pub bias_temporal_smoothness: f64,
    pub maximum_structural_bias_m: f64,
    pub position_min_m: [f64; 2],
    pub position_max_m: [f64; 2],
    pub position_step_size: f64,
    pub bias_step_size: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HbrBctrEvaluation2d {
    pub objective: f64,
    pub position_gradient: Vec<[f64; 2]>,
    pub structural_bias_gradient: Vec<f64>,
    pub packet_residual_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HbrBctrAlternatingStepReport2d {
    /// State after the projected X update, before the B update.
    pub state_after_position: HbrBctrState2d,
    /// State after the projected B update evaluated at state_after_position.
    pub state_after_bias: HbrBctrState2d,
    pub objective_before: f64,
    pub objective_after_position: f64,
    pub objective_after_bias: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HbrBctrError {
    InvalidConfiguration,
    EmptyTrajectory,
    EmptyObservationSet,
    PositionStateCountMismatch,
    BiasStateCountMismatch,
    InvalidStatePosition(usize),
    InvalidBiasState(usize),
    InvalidMeasurementIdentity,
    DuplicateMeasurementIdentity(u64),
    InvalidTimeIndex(u64),
    InvalidBiasGroupIndex(u64),
    InvalidAnchorIdentity(u64),
    InvalidChannelIdentity(u64),
    InvalidAnchorPosition(u64),
    InconsistentAnchorSurvey(u32),
    InconsistentBiasGroupIdentity(usize),
    AnchorChannelSplitAcrossBiasGroups { anchor_id: u32, channel_id: u8 },
    InvalidRange(u64),
    InvalidQualityParameter(u64),
    CoincidentPositionAndAnchor(u64),
    ArithmeticOverflow,
}

impl fmt::Display for HbrBctrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfiguration => write!(f, "HBR-BCTR configuration is invalid"),
            Self::EmptyTrajectory => write!(f, "trajectory must contain at least one position"),
            Self::EmptyObservationSet => write!(f, "at least one range observation is required"),
            Self::PositionStateCountMismatch => write!(f, "position state length does not match the trajectory"),
            Self::BiasStateCountMismatch => write!(f, "structural-bias state length does not equal time steps times anchor-channel groups"),
            Self::InvalidStatePosition(index) => write!(f, "trajectory state is invalid at time index {index}"),
            Self::InvalidBiasState(index) => write!(f, "structural-bias state is invalid at flat index {index}"),
            Self::InvalidMeasurementIdentity => write!(f, "measurement identity must be nonzero"),
            Self::DuplicateMeasurementIdentity(id) => write!(f, "duplicate measurement identity: {id}"),
            Self::InvalidTimeIndex(id) => write!(f, "measurement {id} has a time index outside the trajectory"),
            Self::InvalidBiasGroupIndex(id) => write!(f, "measurement {id} has a bias group index outside the configured group inventory"),
            Self::InvalidAnchorIdentity(id) => write!(f, "measurement {id} has an invalid anchor identity"),
            Self::InvalidChannelIdentity(id) => write!(f, "measurement {id} has an invalid channel identity"),
            Self::InvalidAnchorPosition(id) => write!(f, "measurement {id} has a non-finite anchor position"),
            Self::InconsistentAnchorSurvey(id) => write!(f, "anchor {id} has inconsistent surveyed coordinates across observations"),
            Self::InconsistentBiasGroupIdentity(index) => write!(f, "bias group {index} maps to more than one anchor/channel identity"),
            Self::AnchorChannelSplitAcrossBiasGroups { anchor_id, channel_id } => write!(f, "anchor/channel pair ({anchor_id}, {channel_id}) maps to multiple structural-bias states"),
            Self::InvalidRange(id) => write!(f, "measurement {id} has an invalid measured range"),
            Self::InvalidQualityParameter(id) => write!(f, "measurement {id} has invalid frozen HBR quality parameters"),
            Self::CoincidentPositionAndAnchor(id) => write!(f, "measurement {id} position coincides with its anchor; range gradient is undefined"),
            Self::ArithmeticOverflow => write!(f, "HBR-BCTR objective or gradient arithmetic overflowed"),
        }
    }
}

impl std::error::Error for HbrBctrError {}

/// Evaluate the HBR-BCTR local-window objective and analytic gradient.
pub fn evaluate_hbr_bctr_window_2d(
    state: &HbrBctrState2d,
    observations: &[HbrRangeObservation2d],
    config: HbrBctrConfig2d,
) -> Result<HbrBctrEvaluation2d, HbrBctrError> {
    validate_config(config)?;
    if state.positions_m.is_empty() {
        return Err(HbrBctrError::EmptyTrajectory);
    }
    if observations.is_empty() {
        return Err(HbrBctrError::EmptyObservationSet);
    }

    let time_steps = state.positions_m.len();
    let expected_bias_count = time_steps
        .checked_mul(config.group_count)
        .ok_or(HbrBctrError::ArithmeticOverflow)?;
    if state.structural_bias_m.len() != expected_bias_count {
        return Err(HbrBctrError::BiasStateCountMismatch);
    }

    for (index, position) in state.positions_m.iter().enumerate() {
        if !position.iter().all(|value| value.is_finite())
            || (0..2).any(|axis| {
                position[axis] < config.position_min_m[axis]
                    || position[axis] > config.position_max_m[axis]
            })
        {
            return Err(HbrBctrError::InvalidStatePosition(index));
        }
    }
    for (index, bias) in state.structural_bias_m.iter().enumerate() {
        if !bias.is_finite() || *bias < 0.0 || *bias > config.maximum_structural_bias_m {
            return Err(HbrBctrError::InvalidBiasState(index));
        }
    }

    let mut measurement_ids = BTreeSet::new();
    let mut anchor_positions = BTreeMap::<u32, [f64; 2]>::new();
    let mut bias_group_identities = BTreeMap::<usize, (u32, u8)>::new();
    let mut anchor_channel_bias_groups = BTreeMap::<(u32, u8), usize>::new();
    let mut objective = 0.0_f64;
    let mut position_gradient = vec![[0.0; 2]; time_steps];
    let mut structural_bias_gradient = vec![0.0; expected_bias_count];

    for observation in observations {
        if observation.measurement_id == 0 {
            return Err(HbrBctrError::InvalidMeasurementIdentity);
        }
        if !measurement_ids.insert(observation.measurement_id) {
            return Err(HbrBctrError::DuplicateMeasurementIdentity(
                observation.measurement_id,
            ));
        }
        if observation.time_index >= time_steps {
            return Err(HbrBctrError::InvalidTimeIndex(
                observation.measurement_id,
            ));
        }
        if observation.bias_group_index >= config.group_count {
            return Err(HbrBctrError::InvalidBiasGroupIndex(
                observation.measurement_id,
            ));
        }
        if observation.anchor_id == 0 {
            return Err(HbrBctrError::InvalidAnchorIdentity(
                observation.measurement_id,
            ));
        }
        if observation.channel_id == 0 {
            return Err(HbrBctrError::InvalidChannelIdentity(
                observation.measurement_id,
            ));
        }
        if !observation.anchor_position_m.iter().all(|value| value.is_finite()) {
            return Err(HbrBctrError::InvalidAnchorPosition(
                observation.measurement_id,
            ));
        }
        if let Some(previous_position) =
            anchor_positions.insert(observation.anchor_id, observation.anchor_position_m)
        {
            if previous_position != observation.anchor_position_m {
                return Err(HbrBctrError::InconsistentAnchorSurvey(observation.anchor_id));
            }
        }
        let anchor_channel_identity = (observation.anchor_id, observation.channel_id);
        if let Some(previous_identity) =
            bias_group_identities.insert(observation.bias_group_index, anchor_channel_identity)
        {
            if previous_identity != anchor_channel_identity {
                return Err(HbrBctrError::InconsistentBiasGroupIdentity(
                    observation.bias_group_index,
                ));
            }
        }
        if let Some(previous_group_index) =
            anchor_channel_bias_groups.insert(anchor_channel_identity, observation.bias_group_index)
        {
            if previous_group_index != observation.bias_group_index {
                return Err(HbrBctrError::AnchorChannelSplitAcrossBiasGroups {
                    anchor_id: observation.anchor_id,
                    channel_id: observation.channel_id,
                });
            }
        }
        if !observation.measured_range_m.is_finite() || observation.measured_range_m < 0.0 {
            return Err(HbrBctrError::InvalidRange(observation.measurement_id));
        }
        if !observation.heteroscedastic_scale_m.is_finite()
            || observation.heteroscedastic_scale_m <= 0.0
            || !observation.robust_loss_shape_alpha.is_finite()
            || observation.robust_loss_shape_alpha >= 2.0
            || !observation.nonnegative_correction_prior_m.is_finite()
            || observation.nonnegative_correction_prior_m < 0.0
            || !observation.soft_gate_weight.is_finite()
            || !(0.0..=1.0).contains(&observation.soft_gate_weight)
        {
            return Err(HbrBctrError::InvalidQualityParameter(
                observation.measurement_id,
            ));
        }

        let position = state.positions_m[observation.time_index];
        let delta_x = position[0] - observation.anchor_position_m[0];
        let delta_y = position[1] - observation.anchor_position_m[1];
        let distance = delta_x.hypot(delta_y);
        if !distance.is_finite() {
            return Err(HbrBctrError::ArithmeticOverflow);
        }
        if distance <= f64::EPSILON {
            return Err(HbrBctrError::CoincidentPositionAndAnchor(
                observation.measurement_id,
            ));
        }

        let bias_index =
            observation.time_index * config.group_count + observation.bias_group_index;
        let bias = state.structural_bias_m[bias_index];
        let corrected_residual = observation.measured_range_m
            - distance
            - bias
            - observation.soft_gate_weight
                * observation.nonnegative_correction_prior_m;
        let normalized_residual =
            corrected_residual / (observation.heteroscedastic_scale_m + config.sigma_epsilon_m);
        let (loss, loss_derivative) = barron_loss_and_derivative(
            normalized_residual,
            observation.robust_loss_shape_alpha,
        )?;
        let loss_scale = observation.heteroscedastic_scale_m + config.sigma_epsilon_m;
        objective += loss;
        if !objective.is_finite() {
            return Err(HbrBctrError::ArithmeticOverflow);
        }

        let residual_gradient = loss_derivative / loss_scale;
        let position_scale = -residual_gradient / distance;
        position_gradient[observation.time_index][0] += position_scale * delta_x;
        position_gradient[observation.time_index][1] += position_scale * delta_y;
        structural_bias_gradient[bias_index] -= residual_gradient;

        if !position_gradient[observation.time_index][0].is_finite()
            || !position_gradient[observation.time_index][1].is_finite()
            || !structural_bias_gradient[bias_index].is_finite()
        {
            return Err(HbrBctrError::ArithmeticOverflow);
        }
    }

    // Velocity smoothness: lambda_v * sum_t ||x_t - x_(t-1)||^2.
    for time in 1..time_steps {
        for axis in 0..2 {
            let difference =
                state.positions_m[time][axis] - state.positions_m[time - 1][axis];
            let term = config.lambda_velocity * difference * difference;
            objective += term;
            let gradient = 2.0 * config.lambda_velocity * difference;
            position_gradient[time][axis] += gradient;
            position_gradient[time - 1][axis] -= gradient;
        }
    }

    // Acceleration smoothness: lambda_a * sum_t ||x_t - 2x_(t-1) + x_(t-2)||^2.
    for time in 2..time_steps {
        for axis in 0..2 {
            let second_difference = state.positions_m[time][axis]
                - 2.0 * state.positions_m[time - 1][axis]
                + state.positions_m[time - 2][axis];
            let term =
                config.lambda_acceleration * second_difference * second_difference;
            objective += term;
            let gradient = 2.0 * config.lambda_acceleration * second_difference;
            position_gradient[time][axis] += gradient;
            position_gradient[time - 1][axis] -= 2.0 * gradient;
            position_gradient[time - 2][axis] += gradient;
        }
    }

    // Bias amplitude and temporal drift regularization.
    for (index, bias) in state.structural_bias_m.iter().enumerate() {
        objective += config.lambda_bias * bias * bias;
        structural_bias_gradient[index] += 2.0 * config.lambda_bias * bias;
    }
    for time in 1..time_steps {
        for group in 0..config.group_count {
            let current_index = time * config.group_count + group;
            let previous_index = (time - 1) * config.group_count + group;
            let difference = state.structural_bias_m[current_index]
                - state.structural_bias_m[previous_index];
            let weight = config.lambda_bias * config.bias_temporal_smoothness;
            objective += weight * difference * difference;
            let gradient = 2.0 * weight * difference;
            structural_bias_gradient[current_index] += gradient;
            structural_bias_gradient[previous_index] -= gradient;
        }
    }

    if !objective.is_finite()
        || !position_gradient.iter().flatten().all(|value| value.is_finite())
        || !structural_bias_gradient.iter().all(|value| value.is_finite())
    {
        return Err(HbrBctrError::ArithmeticOverflow);
    }

    Ok(HbrBctrEvaluation2d {
        objective,
        position_gradient,
        structural_bias_gradient,
        packet_residual_count: observations.len(),
    })
}

/// Convenience simultaneous projection of a precomputed position and
/// bias gradient. This is not the paper's alternating X-then-B iteration.
/// Use hbr_bctr_alternating_projected_step_2d for paper-ordered updates.
////
/// Position states are projected into a declared rectangular spatial domain;
/// structural biases are projected into [0, maximum_structural_bias_m].
pub fn hbr_bctr_projected_step_2d(
    state: &HbrBctrState2d,
    evaluation: &HbrBctrEvaluation2d,
    config: HbrBctrConfig2d,
) -> Result<HbrBctrState2d, HbrBctrError> {
    validate_config(config)?;
    if evaluation.position_gradient.len() != state.positions_m.len()
        || evaluation.structural_bias_gradient.len() != state.structural_bias_m.len()
    {
        return Err(HbrBctrError::PositionStateCountMismatch);
    }
    if !evaluation.objective.is_finite()
        || !evaluation.position_gradient.iter().flatten().all(|value| value.is_finite())
        || !evaluation.structural_bias_gradient.iter().all(|value| value.is_finite())
    {
        return Err(HbrBctrError::ArithmeticOverflow);
    }

    let positions_m = state
        .positions_m
        .iter()
        .zip(&evaluation.position_gradient)
        .map(|(position, gradient)| {
            [
                (position[0] - config.position_step_size * gradient[0])
                    .clamp(config.position_min_m[0], config.position_max_m[0]),
                (position[1] - config.position_step_size * gradient[1])
                    .clamp(config.position_min_m[1], config.position_max_m[1]),
            ]
        })
        .collect::<Vec<_>>();
    let structural_bias_m = state
        .structural_bias_m
        .iter()
        .zip(&evaluation.structural_bias_gradient)
        .map(|(bias, gradient)| {
            (bias - config.bias_step_size * gradient)
                .clamp(0.0, config.maximum_structural_bias_m)
        })
        .collect::<Vec<_>>();

    if !positions_m.iter().flatten().all(|value| value.is_finite())
        || !structural_bias_m.iter().all(|value| value.is_finite())
    {
        return Err(HbrBctrError::ArithmeticOverflow);
    }
    Ok(HbrBctrState2d {
        positions_m,
        structural_bias_m,
    })
}

fn validate_config(config: HbrBctrConfig2d) -> Result<(), HbrBctrError> {
    if config.group_count == 0
        || !config.sigma_epsilon_m.is_finite()
        || config.sigma_epsilon_m <= 0.0
        || !config.lambda_velocity.is_finite()
        || config.lambda_velocity < 0.0
        || !config.lambda_acceleration.is_finite()
        || config.lambda_acceleration < 0.0
        || !config.lambda_bias.is_finite()
        || config.lambda_bias < 0.0
        || !config.bias_temporal_smoothness.is_finite()
        || config.bias_temporal_smoothness < 0.0
        || !config.maximum_structural_bias_m.is_finite()
        || config.maximum_structural_bias_m < 0.0
        || !config.position_min_m.iter().all(|value| value.is_finite())
        || !config.position_max_m.iter().all(|value| value.is_finite())
        || (0..2).any(|axis| config.position_max_m[axis] < config.position_min_m[axis])
        || !config.position_step_size.is_finite()
        || config.position_step_size <= 0.0
        || !config.bias_step_size.is_finite()
        || config.bias_step_size <= 0.0
    {
        return Err(HbrBctrError::InvalidConfiguration);
    }
    Ok(())
}


/// Compute the exact 2x2 local position Hessian of the HBR-BCTR objective
/// with all other time positions and structural biases held fixed.
///
/// Includes packet robust-loss curvature plus diagonal contributions from
/// velocity and acceleration smoothness. The result may be indefinite for
/// nonconvex residuals; the window-fusion layer applies the paper's positive
/// eigenvalue clipping before computing tr_+.
pub fn evaluate_hbr_bctr_position_curvature_2d(
    state: &HbrBctrState2d,
    observations: &[HbrRangeObservation2d],
    config: HbrBctrConfig2d,
    time_index: usize,
) -> Result<[[f64; 2]; 2], HbrBctrError> {
    // Reuse the full objective validation so the Hessian sees the same input
    // contract, anchor/channel grouping and state bounds as the optimizer.
    let _ = evaluate_hbr_bctr_window_2d(state, observations, config)?;
    if time_index >= state.positions_m.len() {
        return Err(HbrBctrError::InvalidConfiguration);
    }

    let position = state.positions_m[time_index];
    let mut hessian = [[0.0_f64; 2]; 2];

    for observation in observations
        .iter()
        .filter(|observation| observation.time_index == time_index)
    {
        let dx = position[0] - observation.anchor_position_m[0];
        let dy = position[1] - observation.anchor_position_m[1];
        let distance = dx.hypot(dy);
        if !distance.is_finite() || distance <= f64::EPSILON {
            return Err(HbrBctrError::CoincidentPositionAndAnchor(
                observation.measurement_id,
            ));
        }
        let direction = [dx / distance, dy / distance];
        let scale = observation.heteroscedastic_scale_m + config.sigma_epsilon_m;
        let corrected_residual = observation.measured_range_m
            - distance
            - state.structural_bias_m[
                time_index * config.group_count + observation.bias_group_index
            ]
            - observation.soft_gate_weight
                * observation.nonnegative_correction_prior_m;
        let normalized_residual = corrected_residual / scale;
        let first_derivative = barron_loss_and_derivative(
            normalized_residual,
            observation.robust_loss_shape_alpha,
        )?
        .1;
        let second_derivative = barron_loss_second_derivative(
            normalized_residual,
            observation.robust_loss_shape_alpha,
        )?;
        let isotropic_term = first_derivative / (distance * scale);
        let radial_term = second_derivative / (scale * scale);
        if !isotropic_term.is_finite() || !radial_term.is_finite() {
            return Err(HbrBctrError::ArithmeticOverflow);
        }

        for row in 0..2 {
            for column in 0..2 {
                let identity = if row == column { 1.0 } else { 0.0 };
                hessian[row][column] += radial_term * direction[row] * direction[column]
                    - isotropic_term
                        * (identity - direction[row] * direction[column]);
            }
        }
    }

    // Diagonal block contribution from all adjacent velocity differences.
    let velocity_edge_count =
        usize::from(time_index > 0) + usize::from(time_index + 1 < state.positions_m.len());
    let velocity_diagonal =
        2.0 * config.lambda_velocity * velocity_edge_count as f64;
    hessian[0][0] += velocity_diagonal;
    hessian[1][1] += velocity_diagonal;

    // Diagonal block contribution from every second-difference factor that
    // contains this time index. Coefficients in (x_t - 2x_(t-1) + x_(t-2))
    // are 1, -2, 1 respectively.
    let time_steps = state.positions_m.len();
    let mut acceleration_coefficient_square_sum = 0.0_f64;
    for acceleration_end in 2..time_steps {
        let coefficient = if acceleration_end == time_index {
            1.0
        } else if acceleration_end == time_index + 1 {
            -2.0
        } else if acceleration_end == time_index + 2 {
            1.0
        } else {
            0.0
        };
        acceleration_coefficient_square_sum += coefficient * coefficient;
    }
    let acceleration_diagonal =
        2.0 * config.lambda_acceleration * acceleration_coefficient_square_sum;
    hessian[0][0] += acceleration_diagonal;
    hessian[1][1] += acceleration_diagonal;

    if !hessian.iter().flatten().all(|value| value.is_finite()) {
        return Err(HbrBctrError::ArithmeticOverflow);
    }
    Ok(hessian)
}

fn barron_loss_second_derivative(
    normalized_residual: f64,
    alpha: f64,
) -> Result<f64, HbrBctrError> {
    if !normalized_residual.is_finite() || !alpha.is_finite() || alpha >= 2.0 {
        return Err(HbrBctrError::ArithmeticOverflow);
    }
    let square = normalized_residual * normalized_residual;
    if !square.is_finite() {
        return Err(HbrBctrError::ArithmeticOverflow);
    }
    if alpha.abs() < 1e-8 {
        let denominator = 1.0 + 0.5 * square;
        let result = (1.0 - 0.5 * square) / (denominator * denominator);
        return result
            .is_finite()
            .then_some(result)
            .ok_or(HbrBctrError::ArithmeticOverflow);
    }
    let absolute_offset = (alpha - 2.0).abs();
    let base = 1.0 + square / absolute_offset;
    if !base.is_finite() || base <= 0.0 {
        return Err(HbrBctrError::ArithmeticOverflow);
    }
    let first = base.powf(alpha / 2.0 - 1.0);
    let second = square * base.powf(alpha / 2.0 - 2.0);
    let result = first - second;
    result
        .is_finite()
        .then_some(result)
        .ok_or(HbrBctrError::ArithmeticOverflow)
}

/// Apply one paper-ordered alternating HBR-BCTR update:
/// 1. Evaluate both gradients at (X^k, B^k).
/// 2. Project the position update X^(k+1) using that evaluation.
/// 3. Re-evaluate the full objective at (X^(k+1), B^k).
/// 4. Project the bias update B^(k+1) using the fresh bias gradient.
/// 5. Re-evaluate the objective at (X^(k+1), B^(k+1)).
///
/// This preserves the update order in the paper's equations (21)–(22).
/// It is a single PGD iteration only: convergence/stopping, line search,
/// overlapping windows and information-weighted fusion remain caller-owned.
pub fn hbr_bctr_alternating_projected_step_2d(
    state: &HbrBctrState2d,
    observations: &[HbrRangeObservation2d],
    config: HbrBctrConfig2d,
) -> Result<HbrBctrAlternatingStepReport2d, HbrBctrError> {
    let before = evaluate_hbr_bctr_window_2d(state, observations, config)?;
    let state_after_position =
        project_hbr_bctr_position_state_2d(state, &before, config)?;
    let after_position =
        evaluate_hbr_bctr_window_2d(&state_after_position, observations, config)?;
    let state_after_bias =
        project_hbr_bctr_bias_state_2d(&state_after_position, &after_position, config)?;
    let after_bias =
        evaluate_hbr_bctr_window_2d(&state_after_bias, observations, config)?;

    Ok(HbrBctrAlternatingStepReport2d {
        state_after_position,
        state_after_bias,
        objective_before: before.objective,
        objective_after_position: after_position.objective,
        objective_after_bias: after_bias.objective,
    })
}

fn project_hbr_bctr_position_state_2d(
    state: &HbrBctrState2d,
    evaluation: &HbrBctrEvaluation2d,
    config: HbrBctrConfig2d,
) -> Result<HbrBctrState2d, HbrBctrError> {
    validate_projected_step_inputs(state, evaluation, config)?;
    let positions_m = state
        .positions_m
        .iter()
        .zip(&evaluation.position_gradient)
        .map(|(position, gradient)| {
            [
                (position[0] - config.position_step_size * gradient[0])
                    .clamp(config.position_min_m[0], config.position_max_m[0]),
                (position[1] - config.position_step_size * gradient[1])
                    .clamp(config.position_min_m[1], config.position_max_m[1]),
            ]
        })
        .collect::<Vec<_>>();
    if !positions_m.iter().flatten().all(|value| value.is_finite()) {
        return Err(HbrBctrError::ArithmeticOverflow);
    }
    Ok(HbrBctrState2d {
        positions_m,
        structural_bias_m: state.structural_bias_m.clone(),
    })
}

fn project_hbr_bctr_bias_state_2d(
    state: &HbrBctrState2d,
    evaluation: &HbrBctrEvaluation2d,
    config: HbrBctrConfig2d,
) -> Result<HbrBctrState2d, HbrBctrError> {
    validate_projected_step_inputs(state, evaluation, config)?;
    let structural_bias_m = state
        .structural_bias_m
        .iter()
        .zip(&evaluation.structural_bias_gradient)
        .map(|(bias, gradient)| {
            (bias - config.bias_step_size * gradient)
                .clamp(0.0, config.maximum_structural_bias_m)
        })
        .collect::<Vec<_>>();
    if !structural_bias_m.iter().all(|value| value.is_finite()) {
        return Err(HbrBctrError::ArithmeticOverflow);
    }
    Ok(HbrBctrState2d {
        positions_m: state.positions_m.clone(),
        structural_bias_m,
    })
}

fn validate_projected_step_inputs(
    state: &HbrBctrState2d,
    evaluation: &HbrBctrEvaluation2d,
    config: HbrBctrConfig2d,
) -> Result<(), HbrBctrError> {
    validate_config(config)?;
    if evaluation.position_gradient.len() != state.positions_m.len() {
        return Err(HbrBctrError::PositionStateCountMismatch);
    }
    if evaluation.structural_bias_gradient.len() != state.structural_bias_m.len() {
        return Err(HbrBctrError::BiasStateCountMismatch);
    }
    if !evaluation.objective.is_finite()
        || !evaluation.position_gradient.iter().flatten().all(|value| value.is_finite())
        || !evaluation.structural_bias_gradient.iter().all(|value| value.is_finite())
    {
        return Err(HbrBctrError::ArithmeticOverflow);
    }
    Ok(())
}

/// Barron's generalized adaptive robust loss and derivative for alpha < 2.
///
/// The alpha=0 limit is evaluated as log(1 + x^2/2). The alpha=2 Gaussian
/// limit is outside the paper's configured range and is rejected.
fn barron_loss_and_derivative(
    normalized_residual: f64,
    alpha: f64,
) -> Result<(f64, f64), HbrBctrError> {
    if !normalized_residual.is_finite() || !alpha.is_finite() || alpha >= 2.0 {
        return Err(HbrBctrError::ArithmeticOverflow);
    }
    let square = normalized_residual * normalized_residual;
    if !square.is_finite() {
        return Err(HbrBctrError::ArithmeticOverflow);
    }

    if alpha.abs() < 1e-8 {
        // Continuous alpha -> 0 limit; avoiding cancellation in the
        // generalized expression near its logarithmic special case.
        let denominator = 1.0 + 0.5 * square;
        let loss = denominator.ln();
        let derivative = normalized_residual / denominator;
        if loss.is_finite() && derivative.is_finite() {
            return Ok((loss, derivative));
        }
        return Err(HbrBctrError::ArithmeticOverflow);
    }

    let absolute_offset = (alpha - 2.0).abs();
    let base = 1.0 + square / absolute_offset;
    if !base.is_finite() || base <= 0.0 {
        return Err(HbrBctrError::ArithmeticOverflow);
    }
    let power = base.powf(alpha / 2.0);
    let loss = (absolute_offset / alpha) * (power - 1.0);
    let derivative = normalized_residual * base.powf(alpha / 2.0 - 1.0);
    if loss.is_finite() && derivative.is_finite() {
        Ok((loss, derivative))
    } else {
        Err(HbrBctrError::ArithmeticOverflow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> HbrBctrConfig2d {
        HbrBctrConfig2d {
            group_count: 1,
            sigma_epsilon_m: 1e-6,
            lambda_velocity: 0.02,
            lambda_acceleration: 0.04,
            lambda_bias: 25.0,
            bias_temporal_smoothness: 0.12,
            maximum_structural_bias_m: 2.0,
            position_min_m: [-10.0, -10.0],
            position_max_m: [10.0, 10.0],
            position_step_size: 1e-4,
            bias_step_size: 1e-4,
        }
    }

    fn observation(id: u64, time: usize, range: f64) -> HbrRangeObservation2d {
        HbrRangeObservation2d {
            measurement_id: id,
            time_index: time,
            bias_group_index: 0,
            anchor_id: 1,
            channel_id: 5,
            anchor_position_m: [0.0, 0.0],
            measured_range_m: range,
            heteroscedastic_scale_m: 0.1,
            robust_loss_shape_alpha: 0.5,
            nonnegative_correction_prior_m: 0.2,
            soft_gate_weight: 0.5,
        }
    }

    fn state() -> HbrBctrState2d {
        HbrBctrState2d {
            positions_m: vec![[1.0, 0.0], [1.1, 0.0], [1.2, 0.0]],
            structural_bias_m: vec![0.1, 0.1, 0.1],
        }
    }

    #[test]
    fn computes_finite_hbr_objective_for_two_dimensional_trajectory_and_bias() {
        let result = evaluate_hbr_bctr_window_2d(
            &state(),
            &[observation(1, 0, 1.5), observation(2, 1, 1.6), observation(3, 2, 1.7)],
            config(),
        )
        .unwrap();
        assert!(result.objective.is_finite());
        assert_eq!(result.packet_residual_count, 3);
        assert_eq!(result.position_gradient.len(), 3);
        assert_eq!(result.structural_bias_gradient.len(), 3);
    }

    #[test]
    fn analytic_position_gradient_matches_finite_difference() {
        let current = state();
        let observations = [
            observation(1, 0, 1.5),
            observation(2, 1, 1.6),
            observation(3, 2, 1.7),
        ];
        let cfg = config();
        let result = evaluate_hbr_bctr_window_2d(&current, &observations, cfg).unwrap();
        let epsilon = 1e-6;
        let mut plus = current.clone();
        let mut minus = current.clone();
        plus.positions_m[1][0] += epsilon;
        minus.positions_m[1][0] -= epsilon;
        let plus_value =
            evaluate_hbr_bctr_window_2d(&plus, &observations, cfg).unwrap().objective;
        let minus_value =
            evaluate_hbr_bctr_window_2d(&minus, &observations, cfg).unwrap().objective;
        let finite_difference = (plus_value - minus_value) / (2.0 * epsilon);
        assert!((finite_difference - result.position_gradient[1][0]).abs() < 1e-4);
    }

    #[test]
    fn analytic_bias_gradient_matches_finite_difference() {
        let current = state();
        let observations = [
            observation(1, 0, 1.5),
            observation(2, 1, 1.6),
            observation(3, 2, 1.7),
        ];
        let cfg = config();
        let result = evaluate_hbr_bctr_window_2d(&current, &observations, cfg).unwrap();
        let epsilon = 1e-6;
        let mut plus = current.clone();
        let mut minus = current.clone();
        plus.structural_bias_m[1] += epsilon;
        minus.structural_bias_m[1] -= epsilon;
        let plus_value =
            evaluate_hbr_bctr_window_2d(&plus, &observations, cfg).unwrap().objective;
        let minus_value =
            evaluate_hbr_bctr_window_2d(&minus, &observations, cfg).unwrap().objective;
        let finite_difference = (plus_value - minus_value) / (2.0 * epsilon);
        assert!((finite_difference - result.structural_bias_gradient[1]).abs() < 1e-4);
    }

    #[test]
    fn analytic_local_position_curvature_matches_finite_difference_of_gradient() {
        let current = state();
        let observations = [
            observation(1, 0, 1.5),
            observation(2, 1, 1.6),
            observation(3, 1, 1.9),
        ];
        let cfg = config();
        let curvature =
            evaluate_hbr_bctr_position_curvature_2d(&current, &observations, cfg, 1).unwrap();
        let epsilon = 1e-5;
        let mut plus = current.clone();
        let mut minus = current.clone();
        plus.positions_m[1][0] += epsilon;
        minus.positions_m[1][0] -= epsilon;
        let plus_gradient =
            evaluate_hbr_bctr_window_2d(&plus, &observations, cfg).unwrap().position_gradient[1];
        let minus_gradient =
            evaluate_hbr_bctr_window_2d(&minus, &observations, cfg).unwrap().position_gradient[1];
        let finite_difference_x = (plus_gradient[0] - minus_gradient[0]) / (2.0 * epsilon);
        let finite_difference_y = (plus_gradient[1] - minus_gradient[1]) / (2.0 * epsilon);
        assert!((curvature[0][0] - finite_difference_x).abs() < 2e-3);
        assert!((curvature[1][0] - finite_difference_y).abs() < 2e-3);
        assert!((curvature[0][1] - curvature[1][0]).abs() < 1e-12);
    }

    #[test]
    fn barron_loss_supports_zero_and_negative_shape_limits() {
        let (zero_loss, zero_derivative) = barron_loss_and_derivative(2.0, 0.0).unwrap();
        assert!((zero_loss - (1.0 + 2.0_f64).ln()).abs() < 1e-12);
        assert!((zero_derivative - 2.0 / 3.0).abs() < 1e-12);
        let (negative_loss, negative_derivative) =
            barron_loss_and_derivative(2.0, -1.5).unwrap();
        assert!(negative_loss.is_finite() && negative_loss >= 0.0);
        assert!(negative_derivative.is_finite() && negative_derivative > 0.0);
    }

    #[test]
    fn barron_loss_is_continuous_near_zero_shape_parameter() {
        let (zero_loss, zero_derivative) = barron_loss_and_derivative(1.7, 0.0).unwrap();
        let (near_zero_loss, near_zero_derivative) =
            barron_loss_and_derivative(1.7, 1e-10).unwrap();
        assert!((zero_loss - near_zero_loss).abs() < 1e-12);
        assert!((zero_derivative - near_zero_derivative).abs() < 1e-12);
    }

    #[test]
    fn structural_bias_projection_clamps_to_physical_bounds() {
        let current = state();
        let evaluation = HbrBctrEvaluation2d {
            objective: 1.0,
            position_gradient: vec![[0.0, 0.0]; 3],
            structural_bias_gradient: vec![-1e6, 1e6, 0.0],
            packet_residual_count: 0,
        };
        let updated = hbr_bctr_projected_step_2d(&current, &evaluation, config()).unwrap();
        assert_eq!(updated.structural_bias_m[0], 2.0);
        assert_eq!(updated.structural_bias_m[1], 0.0);
        assert_eq!(updated.structural_bias_m[2], 0.1);
    }

    #[test]
    fn position_projection_respects_declared_spatial_box() {
        let current = state();
        let evaluation = HbrBctrEvaluation2d {
            objective: 1.0,
            position_gradient: vec![[-1e6, 1e6]; 3],
            structural_bias_gradient: vec![0.0; 3],
            packet_residual_count: 0,
        };
        let mut cfg = config();
        cfg.position_min_m = [-0.5, -0.5];
        cfg.position_max_m = [0.5, 0.5];
        let updated = hbr_bctr_projected_step_2d(&current, &evaluation, cfg).unwrap();
        assert!(updated.positions_m.iter().all(|point| point[0] == 0.5 && point[1] == -0.5));
    }

    #[test]
    fn rejects_duplicate_ids_out_of_range_indices_and_invalid_quality() {
        let current = state();
        assert_eq!(
            evaluate_hbr_bctr_window_2d(
                &current,
                &[observation(1, 0, 1.5), observation(1, 1, 1.6)],
                config()
            ),
            Err(HbrBctrError::DuplicateMeasurementIdentity(1))
        );
        let mut bad_time = observation(2, 9, 1.5);
        assert!(matches!(
            evaluate_hbr_bctr_window_2d(&current, &[bad_time], config()),
            Err(HbrBctrError::InvalidTimeIndex(2))
        ));
        bad_time = observation(3, 0, 1.5);
        bad_time.heteroscedastic_scale_m = 0.0;
        assert_eq!(
            evaluate_hbr_bctr_window_2d(&current, &[bad_time], config()),
            Err(HbrBctrError::InvalidQualityParameter(3))
        );
    }

    #[test]
    fn rejects_inconsistent_anchor_survey_or_bias_group_mapping() {
        let current = state();
        let mut second = observation(2, 1, 1.6);
        second.anchor_position_m = [0.1, 0.0];
        assert_eq!(
            evaluate_hbr_bctr_window_2d(
                &current,
                &[observation(1, 0, 1.5), second],
                config()
            ),
            Err(HbrBctrError::InconsistentAnchorSurvey(1))
        );

        let mut different_channel = observation(3, 1, 1.6);
        different_channel.channel_id = 9;
        assert_eq!(
            evaluate_hbr_bctr_window_2d(
                &current,
                &[observation(1, 0, 1.5), different_channel],
                config()
            ),
            Err(HbrBctrError::InconsistentBiasGroupIdentity(0))
        );
    }

    #[test]
    fn one_small_projected_step_reduces_the_local_objective() {
        let current = state();
        let observations = [
            observation(1, 0, 1.5),
            observation(2, 1, 1.6),
            observation(3, 2, 1.7),
        ];
        let cfg = config();
        let before = evaluate_hbr_bctr_window_2d(&current, &observations, cfg).unwrap();
        let updated = hbr_bctr_projected_step_2d(&current, &before, cfg).unwrap();
        let after = evaluate_hbr_bctr_window_2d(&updated, &observations, cfg).unwrap();
        assert!(after.objective < before.objective);
    }

    #[test]
    fn one_anchor_channel_cannot_be_split_across_multiple_bias_states() {
        let current = HbrBctrState2d {
            positions_m: vec![[1.0, 0.0], [1.1, 0.0]],
            structural_bias_m: vec![0.1, 0.1, 0.1, 0.1],
        };
        let mut second = observation(2, 1, 1.6);
        second.bias_group_index = 1;
        assert_eq!(
            evaluate_hbr_bctr_window_2d(
                &current,
                &[observation(1, 0, 1.5), second],
                HbrBctrConfig2d { group_count: 2, ..config() }
            ),
            Err(HbrBctrError::AnchorChannelSplitAcrossBiasGroups {
                anchor_id: 1,
                channel_id: 5
            })
        );
    }

    #[test]
    fn rejects_invalid_state_bias_bounds_and_coincident_anchor() {
        let mut current = state();
        current.structural_bias_m[0] = -0.1;
        assert_eq!(
            evaluate_hbr_bctr_window_2d(&current, &[observation(1, 0, 1.5)], config()),
            Err(HbrBctrError::InvalidBiasState(0))
        );
        current = state();
        current.positions_m[0] = [0.0, 0.0];
        assert_eq!(
            evaluate_hbr_bctr_window_2d(&current, &[observation(1, 0, 1.5)], config()),
            Err(HbrBctrError::CoincidentPositionAndAnchor(1))
        );
    }

    #[test]
    fn invalid_ranges_and_configuration_fail_closed() {
        let current = state();
        let mut bad = observation(1, 0, f64::NAN);
        assert_eq!(
            evaluate_hbr_bctr_window_2d(&current, &[bad], config()),
            Err(HbrBctrError::InvalidRange(1))
        );
        bad = observation(2, 0, 1.0);
        let mut cfg = config();
        cfg.lambda_velocity = -1.0;
        assert_eq!(
            evaluate_hbr_bctr_window_2d(&current, &[bad], cfg),
            Err(HbrBctrError::InvalidConfiguration)
        );
    }

    #[test]
    fn alternating_step_recomputes_bias_gradient_after_position_update() {
        let current = state();
        let observations = [
            observation(1, 0, 1.5),
            observation(2, 1, 1.6),
            observation(3, 2, 1.7),
        ];
        let cfg = config();

        let before = evaluate_hbr_bctr_window_2d(&current, &observations, cfg).unwrap();
        let expected_position =
            project_hbr_bctr_position_state_2d(&current, &before, cfg).unwrap();
        let expected_after_position =
            evaluate_hbr_bctr_window_2d(&expected_position, &observations, cfg).unwrap();
        let expected_bias =
            project_hbr_bctr_bias_state_2d(&expected_position, &expected_after_position, cfg)
                .unwrap();

        let report =
            hbr_bctr_alternating_projected_step_2d(&current, &observations, cfg).unwrap();
        assert_eq!(report.state_after_position, expected_position);
        assert_eq!(report.state_after_bias, expected_bias);
        assert_eq!(report.objective_before, before.objective);
        assert_eq!(report.objective_after_position, expected_after_position.objective);
        assert!(report.state_after_bias.positions_m == report.state_after_position.positions_m);
    }

    #[test]
    fn alternating_projected_iteration_decreases_objective_for_small_step() {
        let mut cfg = config();
        cfg.position_step_size = 1e-6;
        cfg.bias_step_size = 1e-6;
        let report = hbr_bctr_alternating_projected_step_2d(
            &state(),
            &[observation(1, 0, 1.5), observation(2, 1, 1.6), observation(3, 2, 1.7)],
            cfg,
        )
        .unwrap();
        assert!(report.objective_after_position <= report.objective_before);
        assert!(report.objective_after_bias <= report.objective_after_position);
    }

    #[test]
    fn rejects_bias_state_length_mismatch_and_overflow() {
        let mut current = state();
        current.structural_bias_m.pop();
        assert_eq!(
            evaluate_hbr_bctr_window_2d(&current, &[observation(1, 0, 1.5)], config()),
            Err(HbrBctrError::BiasStateCountMismatch)
        );
        current = state();
        current.positions_m[0] = [1e308, 0.0];
        let mut obs = observation(1, 0, 1.0);
        obs.anchor_position_m = [-1e308, 0.0];
        let mut overflow_config = config();
        overflow_config.position_min_m = [-1e308, -10.0];
        overflow_config.position_max_m = [1e308, 10.0];
        assert_eq!(
            evaluate_hbr_bctr_window_2d(&current, &[obs], overflow_config),
            Err(HbrBctrError::ArithmeticOverflow)
        );
    }
}
