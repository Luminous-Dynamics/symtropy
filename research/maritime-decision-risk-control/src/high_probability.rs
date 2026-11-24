// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Finite-grid simultaneous Hoeffding upper-confidence risk control.
//!
//! This is a transparent high-probability baseline, not an implementation of
//! Learn-Then-Test or RCPS. A union bound produces simultaneous upper risk
//! confidence bounds for every frozen policy; a service-efficient policy may
//! then be selected from the simultaneously certified subset.

use crate::non_monotone::WholeScheduleLossGrid;

const LOSS_BOUND_MILLI: u16 = 1_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HighProbabilityError {
    EmptyRiskSample,
    EmptyServiceSample,
    ScheduleCountMismatch,
    PolicyGridMismatch,
    RaggedRiskGrid,
    RaggedServiceGrid,
    RiskLossOutsideBound,
    ServiceLossOutsideBound,
    InvalidTarget,
    InvalidDelta,
    NoPolicyMeetsTarget,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SimultaneousHoeffdingResult {
    pub calibration_schedules: usize,
    pub parameter_count: usize,
    pub target_risk_milli: u16,
    pub delta: f64,
    pub simultaneous_radius_milli: u32,
    pub candidate_risk_upper_bounds_milli: Vec<u32>,
    pub candidate_service_means_milli: Vec<f64>,
    pub eligible_policy_indices: Vec<usize>,
    pub selected_policy_index: usize,
    pub selected_risk_upper_bound_milli: u32,
    pub selected_service_mean_milli: f64,
    pub confidence_level: f64,
}

/// Upper confidence radius for a finite grid of m bounded losses in [0,B].
///
/// For each policy, one-sided Hoeffding gives exp(-2 n eps^2 / B^2).
/// A union bound over m policies sets the simultaneous failure probability
/// to delta. The result is rounded upward in milli-loss units.
pub fn simultaneous_hoeffding_radius_milli(
    parameter_count: usize,
    calibration_schedules: usize,
    delta: f64,
) -> Result<u32, HighProbabilityError> {
    if parameter_count == 0 || calibration_schedules == 0 {
        return Err(HighProbabilityError::EmptyRiskSample);
    }
    if !delta.is_finite() || delta <= 0.0 || delta >= 1.0 {
        return Err(HighProbabilityError::InvalidDelta);
    }

    let m = parameter_count as f64;
    let n = calibration_schedules as f64;
    let radius = f64::from(LOSS_BOUND_MILLI) * ((m / delta).ln() / (2.0 * n)).sqrt();
    if !radius.is_finite() || radius < 0.0 || radius > f64::from(u32::MAX) {
        return Err(HighProbabilityError::InvalidDelta);
    }
    Ok(radius.ceil() as u32)
}

fn validate_grid(
    grid: &WholeScheduleLossGrid,
    empty: HighProbabilityError,
    ragged: HighProbabilityError,
    outside: HighProbabilityError,
) -> Result<(), HighProbabilityError> {
    if grid.schedule_losses_milli.is_empty() {
        return Err(empty);
    }
    let width = grid.parameter_count();
    if width == 0
        || grid
            .schedule_losses_milli
            .iter()
            .any(|row| row.len() != width)
    {
        return Err(ragged);
    }
    if grid
        .schedule_losses_milli
        .iter()
        .flatten()
        .any(|loss| *loss > LOSS_BOUND_MILLI)
    {
        return Err(outside);
    }
    Ok(())
}

/// Selects the lowest empirical service-loss policy among those whose
/// simultaneous one-sided Hoeffding risk upper bound is at most the target.
///
/// Under i.i.d. complete-schedule vectors, fixed candidate policies and
/// bounded hard-risk losses, the selected policy's population expected risk
/// is at most target_risk_milli/1000 with probability at least 1-delta.
/// Service loss is optimized only within the simultaneously certified set.
/// No monotonicity or terminal safe-policy assumption is needed; if no policy
/// qualifies, the method returns NoPolicyMeetsTarget rather than guessing.
pub fn select_simultaneous_hoeffding(
    hard_failure: &WholeScheduleLossGrid,
    service_loss: &WholeScheduleLossGrid,
    target_risk_milli: u16,
    delta: f64,
) -> Result<SimultaneousHoeffdingResult, HighProbabilityError> {
    validate_grid(
        hard_failure,
        HighProbabilityError::EmptyRiskSample,
        HighProbabilityError::RaggedRiskGrid,
        HighProbabilityError::RiskLossOutsideBound,
    )?;
    validate_grid(
        service_loss,
        HighProbabilityError::EmptyServiceSample,
        HighProbabilityError::RaggedServiceGrid,
        HighProbabilityError::ServiceLossOutsideBound,
    )?;
    if hard_failure.schedule_count() != service_loss.schedule_count() {
        return Err(HighProbabilityError::ScheduleCountMismatch);
    }
    if hard_failure.parameter_count() != service_loss.parameter_count() {
        return Err(HighProbabilityError::PolicyGridMismatch);
    }
    if target_risk_milli == 0 || target_risk_milli >= LOSS_BOUND_MILLI {
        return Err(HighProbabilityError::InvalidTarget);
    }

    let n = hard_failure.schedule_count();
    let m = hard_failure.parameter_count();
    let radius = simultaneous_hoeffding_radius_milli(m, n, delta)?;

    let mut candidate_risk_upper_bounds_milli = Vec::with_capacity(m);
    let mut candidate_service_means_milli = Vec::with_capacity(m);
    let mut eligible_policy_indices = Vec::new();

    for policy_index in 0..m {
        let risk_sum = hard_failure
            .schedule_losses_milli
            .iter()
            .map(|row| u64::from(row[policy_index]))
            .sum::<u64>();
        let service_sum = service_loss
            .schedule_losses_milli
            .iter()
            .map(|row| u64::from(row[policy_index]))
            .sum::<u64>();

        let empirical_risk_mean = risk_sum as f64 / n as f64;
        let risk_upper = (empirical_risk_mean + f64::from(radius)).ceil() as u32;
        let service_mean = service_sum as f64 / n as f64;

        candidate_risk_upper_bounds_milli.push(risk_upper);
        candidate_service_means_milli.push(service_mean);
        if risk_upper <= u32::from(target_risk_milli) {
            eligible_policy_indices.push(policy_index);
        }
    }

    let selected_policy_index = eligible_policy_indices
        .iter()
        .copied()
        .min_by(|left, right| {
            candidate_service_means_milli[*left]
                .total_cmp(&candidate_service_means_milli[*right])
                .then_with(|| left.cmp(right))
        })
        .ok_or(HighProbabilityError::NoPolicyMeetsTarget)?;

    Ok(SimultaneousHoeffdingResult {
        calibration_schedules: n,
        parameter_count: m,
        target_risk_milli,
        delta,
        simultaneous_radius_milli: radius,
        selected_risk_upper_bound_milli: candidate_risk_upper_bounds_milli[selected_policy_index],
        selected_service_mean_milli: candidate_service_means_milli[selected_policy_index],
        candidate_risk_upper_bounds_milli,
        candidate_service_means_milli,
        eligible_policy_indices,
        selected_policy_index,
        confidence_level: 1.0 - delta,
    })
}

fn bernoulli_schedule_grid(
    count: usize,
    seed: u64,
    probabilities: &[f64],
) -> WholeScheduleLossGrid {
    let mut rng = SplitMix64::new(seed);
    let schedules = (0..count)
        .map(|_| {
            let draw = (rng.next_u64() >> 11) as f64 / ((1_u64 << 53) as f64);
            probabilities
                .iter()
                .map(|probability| {
                    if draw < *probability {
                        LOSS_BOUND_MILLI
                    } else {
                        0
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect();
    WholeScheduleLossGrid {
        schedule_losses_milli: schedules,
    }
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
}

/// Deterministic receipt for the direct high-probability finite-grid baseline.
/// The fixed service burden is separate from the hard-failure risk axis.
pub fn render_csv() -> String {
    let mut output = String::from(
        "calibration_schedules,parameter_count,target_risk_milli,delta,simultaneous_radius_milli,selected_policy_index,selected_risk_upper_bound_milli,selected_service_mean_milli,eligible_policy_count,status\n",
    );
    let hard_probabilities = [0.25, 0.08, 0.13, 0.02];
    let service_costs = [100_u16, 300, 150, 450];

    for (n, seed) in [
        (120, 0x4849_5042_0120_u64),
        (1_200, 0x4849_5042_1200_u64),
        (50_000, 0x4849_5042_5000_u64),
    ] {
        let hard = bernoulli_schedule_grid(n, seed, &hard_probabilities);
        let service = WholeScheduleLossGrid {
            schedule_losses_milli: (0..n).map(|_| service_costs.to_vec()).collect(),
        };
        match select_simultaneous_hoeffding(&hard, &service, 100, 0.05) {
            Ok(result) => output.push_str(&format!(
                "{},{},{},{:.3},{},{},{},{:.3},{},SIMULTANEOUS_UCB_SYNTHETIC\n",
                result.calibration_schedules,
                result.parameter_count,
                result.target_risk_milli,
                result.delta,
                result.simultaneous_radius_milli,
                result.selected_policy_index,
                result.selected_risk_upper_bound_milli,
                result.selected_service_mean_milli,
                result.eligible_policy_indices.len(),
            )),
            Err(HighProbabilityError::NoPolicyMeetsTarget) => output.push_str(&format!(
                "{n},4,200,0.050,NA,NA,NA,NA,0,NO_POLICY_MEETS_TARGET\n"
            )),
            Err(_) => output.push_str(&format!("{n},4,200,0.050,NA,NA,NA,NA,0,NO_RISK_CLAIM\n")),
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simultaneous_radius_decreases_with_more_calibration_schedules() {
        let small = simultaneous_hoeffding_radius_milli(4, 120, 0.05).expect("small radius");
        let large = simultaneous_hoeffding_radius_milli(4, 1_200, 0.05).expect("large radius");
        assert!(large < small);
    }

    #[test]
    fn smaller_delta_increases_simultaneous_radius() {
        let lenient = simultaneous_hoeffding_radius_milli(4, 1_200, 0.10).expect("delta .1");
        let strict = simultaneous_hoeffding_radius_milli(4, 1_200, 0.01).expect("delta .01");
        assert!(strict > lenient);
    }

    #[test]
    fn selector_picks_lowest_service_cost_among_simultaneously_safe_policies() {
        let hard = bernoulli_schedule_grid(50_000, 0x4849_5053, &[0.25, 0.08, 0.13, 0.04]);
        let service = WholeScheduleLossGrid {
            schedule_losses_milli: (0..50_000).map(|_| vec![100, 300, 150, 450]).collect(),
        };
        let result = select_simultaneous_hoeffding(&hard, &service, 200, 0.05)
            .expect("one or more certified policies");

        assert_eq!(result.selected_policy_index, 2);
        assert_eq!(result.selected_service_mean_milli, 150.0);
        assert!(result.selected_risk_upper_bound_milli <= 200);
        assert!(result.eligible_policy_indices.contains(&2));
    }

    #[test]
    fn returns_no_policy_instead_of_guessing_when_bound_is_too_wide() {
        let hard = bernoulli_schedule_grid(120, 0x4849_5053, &[0.25, 0.08, 0.13, 0.04]);
        let service = WholeScheduleLossGrid {
            schedule_losses_milli: (0..120).map(|_| vec![100, 300, 150, 450]).collect(),
        };
        assert_eq!(
            select_simultaneous_hoeffding(&hard, &service, 100, 0.05),
            Err(HighProbabilityError::NoPolicyMeetsTarget)
        );
    }

    #[test]
    fn selector_rejects_mismatched_rows_and_candidate_grids() {
        let hard = bernoulli_schedule_grid(10, 7, &[0.1, 0.2]);
        let service = WholeScheduleLossGrid {
            schedule_losses_milli: vec![vec![100, 200]],
        };
        assert_eq!(
            select_simultaneous_hoeffding(&hard, &service, 200, 0.05),
            Err(HighProbabilityError::ScheduleCountMismatch)
        );

        let service = WholeScheduleLossGrid {
            schedule_losses_milli: vec![vec![100]; 10],
        };
        assert_eq!(
            select_simultaneous_hoeffding(&hard, &service, 200, 0.05),
            Err(HighProbabilityError::PolicyGridMismatch)
        );
    }

    #[test]
    fn selector_rejects_invalid_confidence_and_unbounded_losses() {
        let hard = bernoulli_schedule_grid(10, 8, &[0.1, 0.2]);
        let service = WholeScheduleLossGrid {
            schedule_losses_milli: vec![vec![100, 200]; 10],
        };
        assert_eq!(
            select_simultaneous_hoeffding(&hard, &service, 200, 1.0),
            Err(HighProbabilityError::InvalidDelta)
        );
        let service = WholeScheduleLossGrid {
            schedule_losses_milli: vec![vec![100, 1_001]; 10],
        };
        assert_eq!(
            select_simultaneous_hoeffding(&hard, &service, 200, 0.05),
            Err(HighProbabilityError::ServiceLossOutsideBound)
        );
    }

    #[test]
    fn deterministic_receipt_replays_with_three_sample_sizes() {
        let first = render_csv();
        assert_eq!(first, render_csv());
        assert_eq!(first.lines().count(), 4);
        let width = first.lines().next().expect("header").split(',').count();
        assert_eq!(width, 10);
        for row in first.lines().skip(1) {
            assert_eq!(row.split(',').count(), width);
        }
    }
}
