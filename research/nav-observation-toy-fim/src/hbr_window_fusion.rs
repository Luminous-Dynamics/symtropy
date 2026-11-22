//! HBR-PGD deterministic overlapping windows and curvature-trace fusion.
//!
//! The window planner emits stable half-open time ranges with a fixed stride.
//! The fusion kernel implements the paper's weighted estimate:
//!
//!   x_hat_t = sum_w kappa_(w,t) * x_(w,t) / sum_w kappa_(w,t)
//!   kappa_(w,t) = tr_+(Lambda_(w,t)) + epsilon_kappa
//!
//! For the symmetric 2x2 local curvature matrix Lambda, tr_+ is the sum
//! of its nonnegative-clipped eigenvalues.
//!
//! These are estimator orchestration/aggregation primitives. Curvature is not
//! calibrated covariance or a protection level, and this module does not
//! certify local minima, convergence, or physical uncertainty coverage.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

const DIM: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HbrWindowPlanConfig {
    pub trajectory_length: usize,
    pub window_length: usize,
    /// Number of shared time indices between adjacent full windows.
    pub overlap_steps: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HbrLocalWindow {
    pub window_id: u64,
    pub start_index: usize,
    pub end_index_exclusive: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HbrWindowPlan {
    pub trajectory_length: usize,
    pub window_length: usize,
    pub overlap_steps: usize,
    pub stride: usize,
    /// Canonical order by increasing start_index and window_id.
    pub windows: Vec<HbrLocalWindow>,
    /// Canonical planned window IDs contributing to each global time index.
    pub contributors_by_time: Vec<Vec<u64>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HbrWindowPositionEstimate2d {
    pub window_id: u64,
    pub time_index: usize,
    pub position_m: [f64; DIM],
    /// Symmetric local objective curvature with respect to this position.
    pub curvature: [[f64; DIM]; DIM],
    /// Commitments to the converged local solution and local curvature artifact.
    pub local_solution_sha256: [u8; 32],
    pub local_curvature_sha256: [u8; 32],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HbrWindowFusionConfig {
    pub epsilon_kappa: f64,
    /// Relative tolerance for the symmetry check on off-diagonal curvature.
    pub symmetry_tolerance: f64,
    /// Commitment to the canonical overlapping-window plan.
    pub window_plan_sha256: [u8; 32],
    /// Commitment to the epsilon, symmetry tolerance and fusion policy.
    pub fusion_config_sha256: [u8; 32],
}

#[derive(Clone, Debug, PartialEq)]
pub struct HbrWindowContribution2d {
    pub window_id: u64,
    pub position_m: [f64; DIM],
    pub positive_curvature_trace: f64,
    /// Published weight before numerical rescaling: tr_+(Lambda) + epsilon.
    pub fusion_weight: f64,
    /// Weight divided by the maximum contributor weight for this time.
    pub normalized_fusion_weight: f64,
    pub local_solution_sha256: [u8; 32],
    pub local_curvature_sha256: [u8; 32],
}

#[derive(Clone, Debug, PartialEq)]
pub struct HbrFusedPosition2d {
    pub time_index: usize,
    pub position_m: [f64; DIM],
    pub contributors: Vec<HbrWindowContribution2d>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HbrWindowFusionReport2d {
    pub trajectory_length: usize,
    pub expected_window_position_count: usize,
    pub observed_window_position_count: usize,
    pub epsilon_kappa: f64,
    pub window_plan_sha256: [u8; 32],
    pub fusion_config_sha256: [u8; 32],
    pub positions: Vec<HbrFusedPosition2d>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HbrWindowFusionError {
    EmptyTrajectory,
    InvalidWindowConfiguration,
    InvalidWindowPlan,
    WindowIdentityOverflow,
    InvalidFusionConfiguration,
    MissingWindowPlanCommitment,
    MissingFusionConfigCommitment,
    MissingLocalSolutionCommitment { window_id: u64, time_index: usize },
    MissingLocalCurvatureCommitment { window_id: u64, time_index: usize },
    MissingEstimate { window_id: u64, time_index: usize },
    DuplicateEstimate { window_id: u64, time_index: usize },
    UnexpectedWindowIdentity(u64),
    EstimateOutsideWindow { window_id: u64, time_index: usize },
    InvalidPositionEstimate { window_id: u64, time_index: usize },
    InvalidCurvature { window_id: u64, time_index: usize },
    AsymmetricCurvature { window_id: u64, time_index: usize },
    CurvatureEigenvalueOverflow { window_id: u64, time_index: usize },
    FusionWeightOverflow { window_id: u64, time_index: usize },
    MissingTimeContributors(usize),
    ArithmeticOverflow(usize),
}

impl fmt::Display for HbrWindowFusionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTrajectory => write!(f, "trajectory length must be positive"),
            Self::InvalidWindowConfiguration => write!(f, "window length must be positive and overlap must be smaller than the window length"),
            Self::InvalidWindowPlan => write!(f, "window plan identities, bounds or contributor map are inconsistent"),
            Self::WindowIdentityOverflow => write!(f, "window identity overflowed"),
            Self::InvalidFusionConfiguration => write!(f, "fusion epsilon must be finite and positive and symmetry tolerance finite and nonnegative"),
            Self::MissingWindowPlanCommitment => write!(f, "window-plan SHA-256 commitment must be nonzero"),
            Self::MissingFusionConfigCommitment => write!(f, "fusion-configuration SHA-256 commitment must be nonzero"),
            Self::MissingLocalSolutionCommitment { window_id, time_index } => write!(f, "local solution commitment is missing for window {window_id}, time {time_index}"),
            Self::MissingLocalCurvatureCommitment { window_id, time_index } => write!(f, "local curvature commitment is missing for window {window_id}, time {time_index}"),
            Self::MissingEstimate { window_id, time_index } => write!(f, "window {window_id} is missing its local position estimate at time {time_index}"),
            Self::DuplicateEstimate { window_id, time_index } => write!(f, "window {window_id} has duplicate local position estimates at time {time_index}"),
            Self::UnexpectedWindowIdentity(id) => write!(f, "local estimate references an unknown window: {id}"),
            Self::EstimateOutsideWindow { window_id, time_index } => write!(f, "local estimate time {time_index} is outside window {window_id}"),
            Self::InvalidPositionEstimate { window_id, time_index } => write!(f, "position estimate is invalid for window {window_id}, time {time_index}"),
            Self::InvalidCurvature { window_id, time_index } => write!(f, "curvature matrix is invalid for window {window_id}, time {time_index}"),
            Self::AsymmetricCurvature { window_id, time_index } => write!(f, "curvature matrix is not symmetric within tolerance for window {window_id}, time {time_index}"),
            Self::CurvatureEigenvalueOverflow { window_id, time_index } => write!(f, "curvature eigenvalue arithmetic overflowed for window {window_id}, time {time_index}"),
            Self::FusionWeightOverflow { window_id, time_index } => write!(f, "curvature fusion weight overflowed for window {window_id}, time {time_index}"),
            Self::MissingTimeContributors(index) => write!(f, "no local window contributes to global time {index}"),
            Self::ArithmeticOverflow(index) => write!(f, "position fusion arithmetic overflowed at global time {index}"),
        }
    }
}

impl std::error::Error for HbrWindowFusionError {}

/// Build canonical overlapping time windows.
///
/// The stride is window_length - overlap_steps. The final window may be
/// shorter than window_length; it begins at the regular stride and ends at
/// trajectory_length. Every time index is covered, and no synthetic padding
/// or duplicated time index is introduced inside a window.
pub fn plan_hbr_overlapping_windows(
    config: HbrWindowPlanConfig,
) -> Result<HbrWindowPlan, HbrWindowFusionError> {
    if config.trajectory_length == 0 {
        return Err(HbrWindowFusionError::EmptyTrajectory);
    }
    if config.window_length == 0 || config.overlap_steps >= config.window_length {
        return Err(HbrWindowFusionError::InvalidWindowConfiguration);
    }
    let stride = config.window_length - config.overlap_steps;
    let mut windows = Vec::new();
    let mut contributors_by_time = vec![Vec::new(); config.trajectory_length];
    let mut start = 0_usize;

    loop {
        let end = start
            .checked_add(config.window_length)
            .unwrap_or(usize::MAX)
            .min(config.trajectory_length);
        if end <= start {
            return Err(HbrWindowFusionError::InvalidWindowConfiguration);
        }
        let window_id = u64::try_from(windows.len() + 1)
            .map_err(|_| HbrWindowFusionError::WindowIdentityOverflow)?;
        let window = HbrLocalWindow {
            window_id,
            start_index: start,
            end_index_exclusive: end,
        };
        for time_index in start..end {
            contributors_by_time[time_index].push(window_id);
        }
        windows.push(window);
        if end == config.trajectory_length {
            break;
        }
        start = start
            .checked_add(stride)
            .ok_or(HbrWindowFusionError::WindowIdentityOverflow)?;
    }

    if contributors_by_time.iter().any(Vec::is_empty) {
        return Err(HbrWindowFusionError::MissingTimeContributors(
            contributors_by_time
                .iter()
                .position(Vec::is_empty)
                .unwrap_or(0),
        ));
    }

    Ok(HbrWindowPlan {
        trajectory_length: config.trajectory_length,
        window_length: config.window_length,
        overlap_steps: config.overlap_steps,
        stride,
        windows,
        contributors_by_time,
    })
}

/// Compute trace of the positive-semidefinite part of a symmetric 2x2 matrix.
///
/// This is the sum of max(eigenvalue, 0), matching tr_+ in the HBR-PGD paper.
pub fn positive_curvature_trace_2x2(
    curvature: [[f64; DIM]; DIM],
    symmetry_tolerance: f64,
) -> Result<f64, HbrWindowFusionError> {
    if !symmetry_tolerance.is_finite() || symmetry_tolerance < 0.0 {
        return Err(HbrWindowFusionError::InvalidFusionConfiguration);
    }
    if !curvature.iter().flatten().all(|value| value.is_finite()) {
        return Err(HbrWindowFusionError::CurvatureEigenvalueOverflow {
            window_id: 0,
            time_index: 0,
        });
    }
    let a = curvature[0][0];
    let b_upper = curvature[0][1];
    let b_lower = curvature[1][0];
    let d = curvature[1][1];
    let scale = 1.0_f64
        .max(a.abs())
        .max(b_upper.abs())
        .max(b_lower.abs())
        .max(d.abs());
    if (b_upper - b_lower).abs() > symmetry_tolerance * scale {
        return Err(HbrWindowFusionError::AsymmetricCurvature {
            window_id: 0,
            time_index: 0,
        });
    }
    let b = b_upper / 2.0 + b_lower / 2.0;
    let a_scaled = a / scale;
    let b_scaled = b / scale;
    let d_scaled = d / scale;
    let half_trace = a_scaled / 2.0 + d_scaled / 2.0;
    let half_spread = 0.5 * (a_scaled - d_scaled).hypot(2.0 * b_scaled);
    let lambda_max_scaled = half_trace + half_spread;
    let lambda_min_scaled = half_trace - half_spread;
    let lambda_max = lambda_max_scaled * scale;
    let lambda_min = lambda_min_scaled * scale;
    if !lambda_max.is_finite() || !lambda_min.is_finite() {
        return Err(HbrWindowFusionError::CurvatureEigenvalueOverflow {
            window_id: 0,
            time_index: 0,
        });
    }
    let positive_trace = lambda_max.max(0.0) + lambda_min.max(0.0);
    positive_trace
        .is_finite()
        .then_some(positive_trace)
        .ok_or(HbrWindowFusionError::CurvatureEigenvalueOverflow {
            window_id: 0,
            time_index: 0,
        })
}

/// Fuse converged local position estimates using positive curvature trace.
///
/// Requires exactly one estimate for every time index inside every planned
/// window. Reports retain the raw curvature trace and fusion weight. Weight
/// scaling is local to each time index and avoids overflow in the weighted
/// numerator/denominator.
pub fn fuse_hbr_window_positions_2d(
    plan: &HbrWindowPlan,
    estimates: &[HbrWindowPositionEstimate2d],
    config: HbrWindowFusionConfig,
) -> Result<HbrWindowFusionReport2d, HbrWindowFusionError> {
    if !config.epsilon_kappa.is_finite()
        || config.epsilon_kappa <= 0.0
        || !config.symmetry_tolerance.is_finite()
        || config.symmetry_tolerance < 0.0
    {
        return Err(HbrWindowFusionError::InvalidFusionConfiguration);
    }
    if config.window_plan_sha256.iter().all(|byte| *byte == 0) {
        return Err(HbrWindowFusionError::MissingWindowPlanCommitment);
    }
    if config.fusion_config_sha256.iter().all(|byte| *byte == 0) {
        return Err(HbrWindowFusionError::MissingFusionConfigCommitment);
    }

    if plan.trajectory_length == 0
        || plan.windows.is_empty()
        || plan.contributors_by_time.len() != plan.trajectory_length
        || plan.window_length == 0
        || plan.overlap_steps >= plan.window_length
        || plan.stride != plan.window_length - plan.overlap_steps
    {
        return Err(HbrWindowFusionError::InvalidWindowPlan);
    }
    let mut windows_by_id = BTreeMap::new();
    let mut expected_contributors = vec![Vec::<u64>::new(); plan.trajectory_length];
    for window in &plan.windows {
        if window.window_id == 0
            || window.start_index >= window.end_index_exclusive
            || window.end_index_exclusive > plan.trajectory_length
            || windows_by_id.insert(window.window_id, *window).is_some()
        {
            return Err(HbrWindowFusionError::InvalidWindowPlan);
        }
        for time_index in window.start_index..window.end_index_exclusive {
            expected_contributors[time_index].push(window.window_id);
        }
    }
    for contributors in &mut expected_contributors {
        contributors.sort_unstable();
        if contributors.is_empty() {
            return Err(HbrWindowFusionError::InvalidWindowPlan);
        }
    }
    if expected_contributors != plan.contributors_by_time {
        return Err(HbrWindowFusionError::InvalidWindowPlan);
    }
    let mut estimates_by_key = BTreeMap::<(u64, usize), HbrWindowPositionEstimate2d>::new();
    let mut contributions_by_time = vec![Vec::<HbrWindowContribution2d>::new(); plan.trajectory_length];
    let mut expected_count = 0_usize;

    for window in &plan.windows {
        expected_count = expected_count
            .checked_add(window.end_index_exclusive - window.start_index)
            .ok_or(HbrWindowFusionError::ArithmeticOverflow(window.start_index))?;
    }

    for estimate in estimates {
        let window = windows_by_id
            .get(&estimate.window_id)
            .ok_or(HbrWindowFusionError::UnexpectedWindowIdentity(
                estimate.window_id,
            ))?;
        if estimate.time_index < window.start_index
            || estimate.time_index >= window.end_index_exclusive
        {
            return Err(HbrWindowFusionError::EstimateOutsideWindow {
                window_id: estimate.window_id,
                time_index: estimate.time_index,
            });
        }
        if estimates_by_key
            .insert((estimate.window_id, estimate.time_index), *estimate)
            .is_some()
        {
            return Err(HbrWindowFusionError::DuplicateEstimate {
                window_id: estimate.window_id,
                time_index: estimate.time_index,
            });
        }
        if !estimate.position_m.iter().all(|value| value.is_finite()) {
            return Err(HbrWindowFusionError::InvalidPositionEstimate {
                window_id: estimate.window_id,
                time_index: estimate.time_index,
            });
        }
        if estimate.local_solution_sha256.iter().all(|byte| *byte == 0) {
            return Err(HbrWindowFusionError::MissingLocalSolutionCommitment {
                window_id: estimate.window_id,
                time_index: estimate.time_index,
            });
        }
        if estimate.local_curvature_sha256.iter().all(|byte| *byte == 0) {
            return Err(HbrWindowFusionError::MissingLocalCurvatureCommitment {
                window_id: estimate.window_id,
                time_index: estimate.time_index,
            });
        }
        let trace = positive_curvature_trace_2x2(
            estimate.curvature,
            config.symmetry_tolerance,
        )
        .map_err(|error| match error {
            HbrWindowFusionError::AsymmetricCurvature { .. } => {
                HbrWindowFusionError::AsymmetricCurvature {
                    window_id: estimate.window_id,
                    time_index: estimate.time_index,
                }
            }
            HbrWindowFusionError::CurvatureEigenvalueOverflow { .. } => {
                HbrWindowFusionError::CurvatureEigenvalueOverflow {
                    window_id: estimate.window_id,
                    time_index: estimate.time_index,
                }
            }
            other => other,
        })?;
        let fusion_weight = trace + config.epsilon_kappa;
        if !fusion_weight.is_finite() || fusion_weight <= 0.0 {
            return Err(HbrWindowFusionError::FusionWeightOverflow {
                window_id: estimate.window_id,
                time_index: estimate.time_index,
            });
        }
        contributions_by_time[estimate.time_index].push(HbrWindowContribution2d {
            window_id: estimate.window_id,
            position_m: estimate.position_m,
            positive_curvature_trace: trace,
            fusion_weight,
            normalized_fusion_weight: 0.0,
            local_solution_sha256: estimate.local_solution_sha256,
            local_curvature_sha256: estimate.local_curvature_sha256,
        });
    }

    if estimates_by_key.len() != expected_count {
        for window in &plan.windows {
            for time_index in window.start_index..window.end_index_exclusive {
                if !estimates_by_key.contains_key(&(window.window_id, time_index)) {
                    return Err(HbrWindowFusionError::MissingEstimate {
                        window_id: window.window_id,
                        time_index,
                    });
                }
            }
        }
    }

    let mut positions = Vec::with_capacity(plan.trajectory_length);
    for (time_index, contributions) in contributions_by_time.iter_mut().enumerate() {
        if contributions.is_empty() {
            return Err(HbrWindowFusionError::MissingTimeContributors(time_index));
        }
        contributions.sort_by_key(|contribution| contribution.window_id);
        let maximum_weight = contributions
            .iter()
            .map(|contribution| contribution.fusion_weight)
            .fold(0.0_f64, f64::max);
        if !maximum_weight.is_finite() || maximum_weight <= 0.0 {
            return Err(HbrWindowFusionError::ArithmeticOverflow(time_index));
        }
        let mut normalized_total = 0.0_f64;
        let mut mean = [0.0_f64; DIM];
        for contribution in contributions.iter_mut() {
            let normalized_weight = contribution.fusion_weight / maximum_weight;
            if !normalized_weight.is_finite() || normalized_weight <= 0.0 {
                return Err(HbrWindowFusionError::ArithmeticOverflow(time_index));
            }
            let next_total = normalized_total + normalized_weight;
            if !next_total.is_finite() || next_total <= 0.0 {
                return Err(HbrWindowFusionError::ArithmeticOverflow(time_index));
            }
            let blend = normalized_weight / next_total;
            for axis in 0..DIM {
                mean[axis] = mean[axis] * (1.0 - blend)
                    + contribution.position_m[axis] * blend;
                if !mean[axis].is_finite() {
                    return Err(HbrWindowFusionError::ArithmeticOverflow(time_index));
                }
            }
            normalized_total = next_total;
            contribution.normalized_fusion_weight = normalized_weight;
        }
        let normalized_sum: f64 = contributions
            .iter()
            .map(|contribution| contribution.normalized_fusion_weight)
            .sum();
        if !normalized_sum.is_finite() || normalized_sum <= 0.0 {
            return Err(HbrWindowFusionError::ArithmeticOverflow(time_index));
        }
        for contribution in contributions {
            contribution.normalized_fusion_weight /= normalized_sum;
        }
        positions.push(HbrFusedPosition2d {
            time_index,
            position_m: mean,
            contributors: contributions.clone(),
        });
    }

    Ok(HbrWindowFusionReport2d {
        trajectory_length: plan.trajectory_length,
        expected_window_position_count: expected_count,
        observed_window_position_count: estimates_by_key.len(),
        epsilon_kappa: config.epsilon_kappa,
        window_plan_sha256: config.window_plan_sha256,
        fusion_config_sha256: config.fusion_config_sha256,
        positions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(length: usize, window: usize, overlap: usize) -> HbrWindowPlan {
        plan_hbr_overlapping_windows(HbrWindowPlanConfig {
            trajectory_length: length,
            window_length: window,
            overlap_steps: overlap,
        })
        .unwrap()
    }

    fn estimate(
        window_id: u64,
        time_index: usize,
        position_m: [f64; 2],
        curvature: [[f64; 2]; 2],
    ) -> HbrWindowPositionEstimate2d {
        HbrWindowPositionEstimate2d {
            window_id,
            time_index,
            position_m,
            curvature,
            local_solution_sha256: [0xA1; 32],
            local_curvature_sha256: [0xB2; 32],
        }
    }

    fn fusion_config() -> HbrWindowFusionConfig {
        HbrWindowFusionConfig {
            epsilon_kappa: 1e-6,
            symmetry_tolerance: 1e-12,
            window_plan_sha256: [0xC3; 32],
            fusion_config_sha256: [0xD4; 32],
        }
    }

    fn complete_estimates(window_plan: &HbrWindowPlan) -> Vec<HbrWindowPositionEstimate2d> {
        window_plan
            .windows
            .iter()
            .flat_map(|window| {
                (window.start_index..window.end_index_exclusive)
                    .map(move |time| estimate(window.window_id, time, [time as f64, 0.0], [[1.0, 0.0], [0.0, 1.0]]))
            })
            .collect()
    }

    #[test]
    fn planner_covers_every_time_with_fixed_stride_and_short_tail() {
        let result = plan(20, 11, 5);
        assert_eq!(result.stride, 6);
        assert_eq!(
            result.windows,
            vec![
                HbrLocalWindow { window_id: 1, start_index: 0, end_index_exclusive: 11 },
                HbrLocalWindow { window_id: 2, start_index: 6, end_index_exclusive: 17 },
                HbrLocalWindow { window_id: 3, start_index: 12, end_index_exclusive: 20 },
            ]
        );
        assert_eq!(result.contributors_by_time[0], vec![1]);
        assert_eq!(result.contributors_by_time[10], vec![1, 2]);
        assert_eq!(result.contributors_by_time[16], vec![2, 3]);
        assert_eq!(result.contributors_by_time[19], vec![3]);
    }

    #[test]
    fn planner_handles_shorter_than_window_and_exact_multiple_lengths() {
        let short = plan(7, 11, 5);
        assert_eq!(short.windows.len(), 1);
        assert_eq!(short.windows[0].end_index_exclusive, 7);
        let exact = plan(22, 11, 0);
        assert_eq!(exact.windows.len(), 2);
        assert_eq!(exact.windows[1].start_index, 11);
    }

    #[test]
    fn planner_rejects_zero_length_and_invalid_overlap() {
        assert_eq!(
            plan_hbr_overlapping_windows(HbrWindowPlanConfig {
                trajectory_length: 0,
                window_length: 11,
                overlap_steps: 5,
            }),
            Err(HbrWindowFusionError::EmptyTrajectory)
        );
        assert_eq!(
            plan_hbr_overlapping_windows(HbrWindowPlanConfig {
                trajectory_length: 10,
                window_length: 11,
                overlap_steps: 11,
            }),
            Err(HbrWindowFusionError::InvalidWindowConfiguration)
        );
        assert_eq!(
            plan_hbr_overlapping_windows(HbrWindowPlanConfig {
                trajectory_length: 10,
                window_length: 0,
                overlap_steps: 0,
            }),
            Err(HbrWindowFusionError::InvalidWindowConfiguration)
        );
    }

    #[test]
    fn fusion_rejects_malformed_or_internally_inconsistent_window_plans() {
        let mut malformed = plan(3, 2, 1);
        malformed.contributors_by_time[0].clear();
        let estimates = complete_estimates(&plan(3, 2, 1));
        assert_eq!(
            fuse_hbr_window_positions_2d(&malformed, &estimates, fusion_config()),
            Err(HbrWindowFusionError::InvalidWindowPlan)
        );

        malformed = plan(3, 2, 1);
        malformed.windows[1].window_id = malformed.windows[0].window_id;
        assert_eq!(
            fuse_hbr_window_positions_2d(&malformed, &estimates, fusion_config()),
            Err(HbrWindowFusionError::InvalidWindowPlan)
        );

        malformed = plan(3, 2, 1);
        malformed.windows[0].end_index_exclusive = 4;
        assert_eq!(
            fuse_hbr_window_positions_2d(&malformed, &estimates, fusion_config()),
            Err(HbrWindowFusionError::InvalidWindowPlan)
        );
    }

    #[test]
    fn positive_curvature_trace_clips_negative_eigenvalue() {
        assert_eq!(
            positive_curvature_trace_2x2([[4.0, 0.0], [0.0, -2.0]], 1e-12).unwrap(),
            4.0
        );
        assert_eq!(
            positive_curvature_trace_2x2([[3.0, 0.0], [0.0, 2.0]], 1e-12).unwrap(),
            5.0
        );
        assert_eq!(
            positive_curvature_trace_2x2([[0.0, 0.0], [0.0, -1.0]], 1e-12).unwrap(),
            0.0
        );
    }

    #[test]
    fn curvature_trace_uses_symmetric_eigenvalues_not_diagonal_sum() {
        let trace = positive_curvature_trace_2x2([[2.0, 1.0], [1.0, 2.0]], 1e-12).unwrap();
        assert!((trace - 4.0).abs() < 1e-12);
        let indefinite = positive_curvature_trace_2x2([[0.0, 2.0], [2.0, 0.0]], 1e-12).unwrap();
        assert!((indefinite - 2.0).abs() < 1e-12);
    }

    #[test]
    fn fusion_uses_curvature_trace_weights_and_records_receipts() {
        let window_plan = plan(3, 2, 1);
        // At t=1, window 1 has trace 2 and position 0; window 2 has trace 6 and position 4.
        let estimates = vec![
            estimate(1, 0, [0.0, 0.0], [[1.0, 0.0], [0.0, 1.0]]),
            estimate(1, 1, [0.0, 0.0], [[1.0, 0.0], [0.0, 1.0]]),
            estimate(2, 1, [4.0, 0.0], [[3.0, 0.0], [0.0, 3.0]]),
            estimate(2, 2, [2.0, 0.0], [[1.0, 0.0], [0.0, 1.0]]),
        ];
        let report =
            fuse_hbr_window_positions_2d(&window_plan, &estimates, fusion_config()).unwrap();
        assert!((report.positions[1].position_m[0] - 3.0).abs() < 1e-9);
        assert_eq!(report.positions[1].contributors.len(), 2);
        assert!((report.positions[1].contributors[0].normalized_fusion_weight - 0.25).abs() < 1e-9);
        assert!((report.positions[1].contributors[1].normalized_fusion_weight - 0.75).abs() < 1e-9);
        assert_eq!(report.expected_window_position_count, 4);
        assert_eq!(report.observed_window_position_count, 4);
    }

    #[test]
    fn fusion_is_permutation_invariant() {
        let window_plan = plan(3, 2, 1);
        let estimates = complete_estimates(&window_plan);
        let first = fuse_hbr_window_positions_2d(&window_plan, &estimates, fusion_config()).unwrap();
        let mut reversed = estimates.clone();
        reversed.reverse();
        let second = fuse_hbr_window_positions_2d(&window_plan, &reversed, fusion_config()).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn missing_duplicate_and_out_of_window_estimates_are_rejected() {
        let window_plan = plan(3, 2, 1);
        let mut estimates = complete_estimates(&window_plan);
        estimates.pop();
        assert!(matches!(
            fuse_hbr_window_positions_2d(&window_plan, &estimates, fusion_config()),
            Err(HbrWindowFusionError::MissingEstimate { .. })
        ));
        estimates = complete_estimates(&window_plan);
        estimates.push(estimates[0]);
        assert!(matches!(
            fuse_hbr_window_positions_2d(&window_plan, &estimates, fusion_config()),
            Err(HbrWindowFusionError::DuplicateEstimate { .. })
        ));
        estimates = complete_estimates(&window_plan);
        estimates.push(estimate(1, 2, [0.0, 0.0], [[1.0, 0.0], [0.0, 1.0]]));
        assert!(matches!(
            fuse_hbr_window_positions_2d(&window_plan, &estimates, fusion_config()),
            Err(HbrWindowFusionError::EstimateOutsideWindow { .. })
        ));
    }

    #[test]
    fn fusion_rejects_invalid_curvature_and_position() {
        let window_plan = plan(1, 1, 0);
        assert!(matches!(
            fuse_hbr_window_positions_2d(
                &window_plan,
                &[estimate(1, 0, [0.0, 0.0], [[1.0, 2.0], [0.0, 1.0]])],
                fusion_config()
            ),
            Err(HbrWindowFusionError::AsymmetricCurvature { .. })
        ));
        assert!(matches!(
            fuse_hbr_window_positions_2d(
                &window_plan,
                &[estimate(1, 0, [f64::NAN, 0.0], [[1.0, 0.0], [0.0, 1.0]])],
                fusion_config()
            ),
            Err(HbrWindowFusionError::InvalidPositionEstimate { .. })
        ));
    }

    #[test]
    fn fusion_requires_plan_local_solution_and_curvature_commitments() {
        let window_plan = plan(1, 1, 0);
        let valid_estimate = estimate(1, 0, [1.0, 2.0], [[1.0, 0.0], [0.0, 1.0]]);
        let mut invalid_config = fusion_config();
        invalid_config.window_plan_sha256 = [0; 32];
        assert_eq!(
            fuse_hbr_window_positions_2d(&window_plan, &[valid_estimate], invalid_config),
            Err(HbrWindowFusionError::MissingWindowPlanCommitment)
        );
        invalid_config = fusion_config();
        invalid_config.fusion_config_sha256 = [0; 32];
        assert_eq!(
            fuse_hbr_window_positions_2d(&window_plan, &[valid_estimate], invalid_config),
            Err(HbrWindowFusionError::MissingFusionConfigCommitment)
        );

        let mut invalid_estimate = valid_estimate;
        invalid_estimate.local_solution_sha256 = [0; 32];
        assert_eq!(
            fuse_hbr_window_positions_2d(&window_plan, &[invalid_estimate], fusion_config()),
            Err(HbrWindowFusionError::MissingLocalSolutionCommitment {
                window_id: 1,
                time_index: 0
            })
        );
        invalid_estimate = valid_estimate;
        invalid_estimate.local_curvature_sha256 = [0; 32];
        assert_eq!(
            fuse_hbr_window_positions_2d(&window_plan, &[invalid_estimate], fusion_config()),
            Err(HbrWindowFusionError::MissingLocalCurvatureCommitment {
                window_id: 1,
                time_index: 0
            })
        );
    }

    #[test]
    fn fusion_epsilon_gives_flat_curvature_a_small_positive_weight() {
        let window_plan = plan(1, 1, 0);
        let report = fuse_hbr_window_positions_2d(
            &window_plan,
            &[estimate(1, 0, [3.0, 4.0], [[-1.0, 0.0], [0.0, -2.0]])],
            fusion_config()
        )
        .unwrap();
        assert_eq!(report.positions[0].position_m, [3.0, 4.0]);
        assert_eq!(report.positions[0].contributors[0].positive_curvature_trace, 0.0);
        assert_eq!(report.positions[0].contributors[0].fusion_weight, 1e-6);
    }
}
