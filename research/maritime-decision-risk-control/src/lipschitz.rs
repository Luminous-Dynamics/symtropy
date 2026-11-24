// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Lipschitz-structured non-monotone CRC correction.
//!
//! Implements Proposition 2 of Aldirawi, Li & Guo, arXiv:2604.01502v3.
//! The synthetic loss is a smooth service-cost surrogate, not a hard feasibility
//! indicator. The uniform Lipschitz constant is analytically bounded by the
//! maximum derivative of the constructed sinusoidal loss family.

const LOSS_BOUND: f64 = 1.0;
const TARGET_RISK: f64 = 0.20;
const POLICY_INTERVAL_LENGTH: f64 = 1.0;
const LOSS_CENTER: f64 = 0.15;
const LOSS_AMPLITUDE: f64 = 0.02;
const LIPSCHITZ_K: f64 = 0.126;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LipschitzError {
    EmptyPopulation,
    InvalidGridSize,
    InvalidCalibrationSize,
    InvalidLipschitzBound,
    EndpointPopulationRiskExceedsTarget,
    SearchFailed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LipschitzBoundResult {
    pub calibration_schedules: usize,
    pub policy_grid_size: usize,
    pub target_risk: f64,
    pub loss_bound: f64,
    pub lipschitz_k: f64,
    pub interval_length: f64,
    pub grid_correction: f64,
    pub lipschitz_correction: f64,
    pub auxiliary_grid_size: usize,
    pub selected_correction: f64,
    pub endpoint_population_risk: f64,
    pub claim_type: &'static str,
}

fn d_bound(grid_size: usize, calibration_schedules: usize) -> Result<f64, LipschitzError> {
    if grid_size < 2 {
        return Err(LipschitzError::InvalidGridSize);
    }
    if calibration_schedules == 0 {
        return Err(LipschitzError::InvalidCalibrationSize);
    }

    let m = grid_size as f64;
    let n = calibration_schedules as f64;
    let log_term = (2.0 * m).ln();
    Ok(LOSS_BOUND * (log_term / (2.0 * n)).sqrt()
        + LOSS_BOUND / (2.0 * (2.0 * n * log_term).sqrt()))
}

/// Computes D_e(n,K,b-a) = inf_{N>=2} { D(N,n) + K(b-a)/(N-1) }.
///
/// The search stops exactly once D(N,n) is no smaller than the best objective:
/// D(N,n) is increasing in N and the discretization term is nonnegative, so
/// no later auxiliary grid can improve the current minimum.
pub fn lipschitz_correction(
    calibration_schedules: usize,
    lipschitz_k: f64,
    interval_length: f64,
) -> Result<(f64, usize), LipschitzError> {
    if calibration_schedules == 0 {
        return Err(LipschitzError::InvalidCalibrationSize);
    }
    if !lipschitz_k.is_finite() || lipschitz_k < 0.0 {
        return Err(LipschitzError::InvalidLipschitzBound);
    }
    if !interval_length.is_finite() || interval_length <= 0.0 {
        return Err(LipschitzError::InvalidLipschitzBound);
    }

    let mut best = f64::INFINITY;
    let mut best_n = 0_usize;
    let mut auxiliary_grid = 2_usize;

    loop {
        let concentration = d_bound(auxiliary_grid, calibration_schedules)?;
        if concentration >= best {
            break;
        }
        let discretization = lipschitz_k * interval_length / (auxiliary_grid - 1) as f64;
        let objective = concentration + discretization;
        if objective < best {
            best = objective;
            best_n = auxiliary_grid;
        }
        auxiliary_grid = auxiliary_grid.saturating_add(1);
        if auxiliary_grid == usize::MAX {
            return Err(LipschitzError::SearchFailed);
        }
    }

    if !best.is_finite() || best_n < 2 {
        return Err(LipschitzError::SearchFailed);
    }
    Ok((best, best_n))
}

#[derive(Clone, Copy, Debug)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn unit_interval(&mut self) -> f64 {
        let mantissa = self.next_u64() >> 11;
        mantissa as f64 / ((1_u64 << 53) as f64)
    }
}

/// Loss for one complete schedule. Its derivative magnitude is at most
/// 2*pi*LOSS_AMPLITUDE, strictly below the declared K=0.126 on [0,1].
fn schedule_loss(phase: f64, lambda: f64) -> f64 {
    LOSS_CENTER + LOSS_AMPLITUDE * (std::f64::consts::TAU * (lambda + phase)).sin()
}

fn sample_phases(count: usize, seed: u64) -> Vec<f64> {
    let mut rng = SplitMix64::new(seed);
    (0..count).map(|_| rng.unit_interval()).collect()
}

fn mean_loss(phases: &[f64], lambda: f64) -> f64 {
    phases
        .iter()
        .copied()
        .map(|phase| schedule_loss(phase, lambda))
        .sum::<f64>()
        / phases.len() as f64
}

pub fn evaluate_grid(
    calibration_schedules: usize,
    policy_grid_size: usize,
    seed: u64,
) -> Result<LipschitzBoundResult, LipschitzError> {
    if calibration_schedules == 0 {
        return Err(LipschitzError::InvalidCalibrationSize);
    }
    if policy_grid_size < 2 {
        return Err(LipschitzError::InvalidGridSize);
    }
    let max_loss = LOSS_CENTER + LOSS_AMPLITUDE;
    if max_loss > TARGET_RISK {
        return Err(LipschitzError::EndpointPopulationRiskExceedsTarget);
    }

    let phases = sample_phases(calibration_schedules, seed);
    let endpoint_population_risk = max_loss;
    let endpoint_calibration_loss = mean_loss(&phases, POLICY_INTERVAL_LENGTH);
    if endpoint_calibration_loss > endpoint_population_risk {
        return Err(LipschitzError::EndpointPopulationRiskExceedsTarget);
    }

    let grid_correction = d_bound(policy_grid_size, calibration_schedules)?;
    let (lipschitz_correction, auxiliary_grid_size) =
        lipschitz_correction(calibration_schedules, LIPSCHITZ_K, POLICY_INTERVAL_LENGTH)?;
    let selected_correction = grid_correction.min(lipschitz_correction);

    Ok(LipschitzBoundResult {
        calibration_schedules,
        policy_grid_size,
        target_risk: TARGET_RISK,
        loss_bound: LOSS_BOUND,
        lipschitz_k: LIPSCHITZ_K,
        interval_length: POLICY_INTERVAL_LENGTH,
        grid_correction,
        lipschitz_correction,
        auxiliary_grid_size,
        selected_correction,
        endpoint_population_risk,
        claim_type: "EXPECTED_NON_MONOTONE_LIPSCHITZ",
    })
}

/// Deterministic 11-column comparator receipt for a smooth service-cost loss.
pub fn render_csv() -> String {
    let mut output = String::from(
        "calibration_schedules,policy_grid_size,target_risk,loss_bound,lipschitz_k,interval_length,grid_correction,lipschitz_correction,auxiliary_grid_size,selected_correction,endpoint_population_risk,claim_type\n",
    );

    for (n, seed) in [
        (1_200, 0x4C49_5053_4348_1200_u64),
        (10_000, 0x4C49_5053_4348_1000_u64),
        (50_000, 0x4C49_5053_4348_5000_u64),
    ] {
        for m in [64_usize, 10_000_usize] {
            let result = evaluate_grid(n, m, seed).expect("valid Lipschitz synthetic fixture");
            output.push_str(&format!(
                "{},{},{:.3},{:.3},{:.3},{:.3},{:.6},{:.6},{},{:.6},{:.3},{}\n",
                result.calibration_schedules,
                result.policy_grid_size,
                result.target_risk,
                result.loss_bound,
                result.lipschitz_k,
                result.interval_length,
                result.grid_correction,
                result.lipschitz_correction,
                result.auxiliary_grid_size,
                result.selected_correction,
                result.endpoint_population_risk,
                result.claim_type,
            ));
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declared_lipschitz_constant_bounds_analytic_derivative() {
        let analytic_k = std::f64::consts::TAU * LOSS_AMPLITUDE;
        assert!(analytic_k <= LIPSCHITZ_K);
    }

    #[test]
    fn synthetic_loss_is_bounded_and_endpoint_population_risk_is_safe() {
        let phases = sample_phases(1_000, 17);
        for phase in phases {
            for step in 0..=100 {
                let lambda = step as f64 / 100.0;
                let loss = schedule_loss(phase, lambda);
                assert!((0.0..=LOSS_BOUND).contains(&loss));
                assert!(loss <= TARGET_RISK);
            }
        }
    }

    #[test]
    fn finite_grid_correction_grows_with_grid_resolution() {
        let coarse = d_bound(64, 1_200).expect("coarse bound");
        let fine = d_bound(10_000, 1_200).expect("fine bound");
        assert!(fine > coarse);
    }

    #[test]
    fn lipschitz_correction_does_not_depend_on_original_grid_size() {
        let (first, first_n) = lipschitz_correction(1_200, LIPSCHITZ_K, 1.0).expect("De");
        let (second, second_n) = lipschitz_correction(1_200, LIPSCHITZ_K, 1.0).expect("De");
        assert_eq!(first, second);
        assert_eq!(first_n, second_n);
    }

    #[test]
    fn lipschitz_bound_can_improve_over_large_finite_grid_correction() {
        let result = evaluate_grid(1_200, 10_000, 99).expect("fine-grid result");
        assert!(result.lipschitz_correction < result.grid_correction);
        assert_eq!(result.selected_correction, result.lipschitz_correction);
    }

    #[test]
    fn fine_grid_and_lipschitz_bounds_are_both_reported() {
        let result = evaluate_grid(10_000, 10_000, 100).expect("large sample");
        assert!(result.grid_correction > 0.0);
        assert!(result.lipschitz_correction > 0.0);
        assert!(result.selected_correction <= result.grid_correction);
    }

    #[test]
    fn deterministic_receipt_is_byte_identical_and_has_six_rows() {
        let first = render_csv();
        assert_eq!(first, render_csv());
        assert_eq!(first.lines().count(), 7);
        let width = first.lines().next().expect("header").split(',').count();
        assert_eq!(width, 12);
        for row in first.lines().skip(1) {
            assert_eq!(row.split(',').count(), width);
        }
    }
}
