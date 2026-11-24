// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Finite-grid conformal risk control for non-monotone whole-schedule loss.
//!
//! Implements Theorem 1 of Aldirawi, Li & Guo, arXiv:2604.01502v3 (11 Nov 2026).
//! This is a deterministic synthetic research harness, not an operational certificate.

const LOSS_BOUND_MILLI: u16 = 1_000;
const TEST_SCHEDULES: usize = 5_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WholeScheduleLossGrid {
    /// One row per complete schedule; one bounded loss per candidate policy.
    pub schedule_losses_milli: Vec<Vec<u16>>,
}

impl WholeScheduleLossGrid {
    pub fn schedule_count(&self) -> usize {
        self.schedule_losses_milli.len()
    }

    pub fn parameter_count(&self) -> usize {
        self.schedule_losses_milli.first().map_or(0, Vec::len)
    }

    pub fn monotonicity_violations(&self) -> usize {
        self.schedule_losses_milli
            .iter()
            .map(|row| row.windows(2).filter(|pair| pair[1] > pair[0]).count())
            .sum()
    }

    fn validate(&self) -> Result<(), RiskControlError> {
        if self.schedule_losses_milli.is_empty() {
            return Err(RiskControlError::EmptyCalibration);
        }
        let width = self.parameter_count();
        if width == 0 {
            return Err(RiskControlError::EmptyParameterGrid);
        }
        if self
            .schedule_losses_milli
            .iter()
            .any(|row| row.len() != width)
        {
            return Err(RiskControlError::RaggedLossMatrix);
        }
        if self
            .schedule_losses_milli
            .iter()
            .flatten()
            .any(|loss| *loss > LOSS_BOUND_MILLI)
        {
            return Err(RiskControlError::LossExceedsBound);
        }
        Ok(())
    }
}

/// External population-risk assumption for the terminal (most conservative) policy.
/// It is a supplied evidence claim; calibration samples cannot prove it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EndpointPopulationRiskContract {
    pub scope: String,
    pub evidence_ref: String,
    /// Upper bound on E[L(lambda_max)], scaled to [0, 1000].
    pub expected_loss_upper_milli: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrozenRevisions {
    pub policy_revision: String,
    pub evaluator_revision: String,
    pub loss_contract_revision: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RiskControlError {
    EmptyCalibration,
    EmptyParameterGrid,
    RaggedLossMatrix,
    LossExceedsBound,
    InvalidTarget,
    MissingRevision,
    MissingEndpointPopulationRiskContract,
    NoPositiveAdjustedTarget {
        target_risk_milli: u16,
        excess_bound_milli: u32,
    },
    EndpointPopulationRiskExceedsAdjustedTarget {
        endpoint_risk_upper_milli: u16,
        adjusted_target_milli: u16,
    },
    InvalidExcessBound,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NonMonotoneRiskControlResult {
    pub selected_parameter_index: usize,
    pub parameter_count: usize,
    pub calibration_schedules: usize,
    pub target_risk_milli: u16,
    pub excess_bound_milli: u32,
    pub adjusted_risk_milli: u16,
    pub selected_calibration_loss_mean_milli: f64,
    pub monotonicity_violations: usize,
    pub empirical_selector_found_feasible_grid_point: bool,
    pub endpoint_population_risk_upper_milli: u16,
    pub endpoint_contract_scope: String,
    pub endpoint_evidence_ref: String,
    pub policy_revision: String,
    pub evaluator_revision: String,
    pub loss_contract_revision: String,
}

/// The finite-grid non-monotone correction:
///
/// D(m,n) = B * sqrt(log(2m)/(2n)) + B/(2*sqrt(2n*log(2m))).
///
/// Returns an upward-rounded milli-loss bound. Rounding upward avoids
/// understating the theorem's correction.
pub fn excess_bound_milli(parameter_count: usize, calibration_schedules: usize) -> Option<u32> {
    if parameter_count == 0 || calibration_schedules == 0 {
        return None;
    }
    let m = parameter_count as f64;
    let n = calibration_schedules as f64;
    let log_term = (2.0 * m).ln();
    let bound = f64::from(LOSS_BOUND_MILLI) * (log_term / (2.0 * n)).sqrt()
        + f64::from(LOSS_BOUND_MILLI) / (2.0 * (2.0 * n * log_term).sqrt());
    if !bound.is_finite() || bound < 0.0 || bound > f64::from(u32::MAX) {
        return None;
    }
    Some(bound.ceil() as u32)
}

fn validate_revisions(revisions: &FrozenRevisions) -> Result<(), RiskControlError> {
    if revisions.policy_revision.trim().is_empty()
        || revisions.evaluator_revision.trim().is_empty()
        || revisions.loss_contract_revision.trim().is_empty()
    {
        return Err(RiskControlError::MissingRevision);
    }
    Ok(())
}

/// Selects a candidate policy using the paper's finite-grid non-monotone CRC rule.
///
/// Assumptions:
/// - complete schedule-loss vectors are i.i.d. across calibration and future schedules;
/// - each candidate's schedule loss is bounded in [0, B];
/// - the endpoint policy has population expected loss no greater than the supplied
///   endpoint contract;
/// - policy grid, evaluator and loss definition were frozen before calibration.
///
/// If alpha' = alpha - D(m,n) is not positive, no adjusted expected-risk claim is
/// emitted. This theorem controls marginal expected loss, not a high-probability
/// bound conditional on the realized calibration set or subgroup-conditional risk.
pub fn calibrate(
    grid: &WholeScheduleLossGrid,
    target_risk_milli: u16,
    endpoint_contract: Option<&EndpointPopulationRiskContract>,
    revisions: &FrozenRevisions,
) -> Result<NonMonotoneRiskControlResult, RiskControlError> {
    grid.validate()?;
    validate_revisions(revisions)?;
    if target_risk_milli == 0 || target_risk_milli >= LOSS_BOUND_MILLI {
        return Err(RiskControlError::InvalidTarget);
    }

    let endpoint = endpoint_contract.ok_or(RiskControlError::MissingEndpointPopulationRiskContract)?;
    if endpoint.scope.trim().is_empty() || endpoint.evidence_ref.trim().is_empty() {
        return Err(RiskControlError::MissingEndpointPopulationRiskContract);
    }

    let n = grid.schedule_count();
    let m = grid.parameter_count();
    let correction = excess_bound_milli(m, n).ok_or(RiskControlError::InvalidExcessBound)?;
    if correction >= u32::from(target_risk_milli) {
        return Err(RiskControlError::NoPositiveAdjustedTarget {
            target_risk_milli,
            excess_bound_milli: correction,
        });
    }
    let adjusted_target = target_risk_milli - correction as u16;
    if endpoint.expected_loss_upper_milli > adjusted_target {
        return Err(RiskControlError::EndpointPopulationRiskExceedsAdjustedTarget {
            endpoint_risk_upper_milli: endpoint.expected_loss_upper_milli,
            adjusted_target_milli: adjusted_target,
        });
    }

    let last = m - 1;
    let mut selected = last;
    let mut selected_sum = grid
        .schedule_losses_milli
        .iter()
        .map(|row| u64::from(row[last]))
        .sum::<u64>();
    let mut found_feasible = false;
    let n_u64 = n as u64;

    for parameter_index in 0..m {
        let loss_sum = grid
            .schedule_losses_milli
            .iter()
            .map(|row| u64::from(row[parameter_index]))
            .sum::<u64>();

        // (n/(n+1))*empirical_mean + B/(n+1) <= alpha'
        // is exactly (sum(losses)+B) <= alpha'*(n+1).
        let lhs = loss_sum + u64::from(LOSS_BOUND_MILLI);
        let rhs = u64::from(adjusted_target) * (n_u64 + 1);
        if lhs <= rhs {
            selected = parameter_index;
            selected_sum = loss_sum;
            found_feasible = true;
            break;
        }
    }

    Ok(NonMonotoneRiskControlResult {
        selected_parameter_index: selected,
        parameter_count: m,
        calibration_schedules: n,
        target_risk_milli,
        excess_bound_milli: correction,
        adjusted_risk_milli: adjusted_target,
        selected_calibration_loss_mean_milli: selected_sum as f64 / n as f64,
        monotonicity_violations: grid.monotonicity_violations(),
        empirical_selector_found_feasible_grid_point: found_feasible,
        endpoint_population_risk_upper_milli: endpoint.expected_loss_upper_milli,
        endpoint_contract_scope: endpoint.scope.clone(),
        endpoint_evidence_ref: endpoint.evidence_ref.clone(),
        policy_revision: revisions.policy_revision.clone(),
        evaluator_revision: revisions.evaluator_revision.clone(),
        loss_contract_revision: revisions.loss_contract_revision.clone(),
    })
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

    fn below(&mut self, upper: u32) -> u32 {
        (self.next_u64() % u64::from(upper)) as u32
    }
}

/// Four deliberately non-monotone recourse alternatives. The terminal fallback
/// has zero synthetic failure loss by construction; this makes the endpoint
/// population-risk assumption explicit for this generated fixture only.
fn synthetic_non_monotone_grid(count: usize, seed: u64) -> WholeScheduleLossGrid {
    let mut rng = SplitMix64::new(seed);
    let schedule_losses_milli = (0..count)
        .map(|_| {
            let severity = rng.below(100) as u16;
            let deterministic_plan = if severity > 40 { 1_000 } else { 0 };
            let added_buffer = if severity > 60 {
                1_000
            } else if severity < 10 {
                250
            } else {
                0
            };
            let wider_recourse = if severity > 90 { 1_000 } else { 0 };
            let safe_abstention_fallback = 0;
            vec![
                deterministic_plan,
                added_buffer,
                wider_recourse,
                safe_abstention_fallback,
            ]
        })
        .collect();
    WholeScheduleLossGrid {
        schedule_losses_milli,
    }
}

fn synthetic_endpoint() -> EndpointPopulationRiskContract {
    EndpointPopulationRiskContract {
        scope: "SYNTHETIC_IID_WHOLE_SCHEDULES".to_owned(),
        evidence_ref: "terminal fallback zero-loss by construction".to_owned(),
        expected_loss_upper_milli: 0,
    }
}

fn synthetic_revisions(name: &str) -> FrozenRevisions {
    FrozenRevisions {
        policy_revision: format!("{name}-policy-v1"),
        evaluator_revision: format!("{name}-evaluator-v1"),
        loss_contract_revision: format!("{name}-loss-v1"),
    }
}

fn test_mean_loss(grid: &WholeScheduleLossGrid, parameter_index: usize) -> f64 {
    let total = grid
        .schedule_losses_milli
        .iter()
        .map(|row| f64::from(row[parameter_index]))
        .sum::<f64>();
    total / grid.schedule_count() as f64
}

/// Deterministic 16-column receipt with both finite-grid correction and selected policy.
pub fn render_csv() -> String {
    let mut output = String::from(
        "experiment,calibration_unit,method,calibration_schedules,parameter_count,target_risk_milli,excess_bound_milli,adjusted_risk_milli,selected_parameter_index,selected_calibration_loss_mean_milli,monotonicity_violations,endpoint_population_risk_upper_milli,endpoint_contract_scope,endpoint_evidence_ref,test_schedules,test_loss_mean_milli,status\n",
    );

    let experiments = [
        ("N1200_ALPHA200", 1_200, 200_u16, 0x4E4D_1200_001_u64),
        ("N1200_ALPHA100", 1_200, 100_u16, 0x4E4D_1200_002_u64),
        ("N50000_ALPHA100", 50_000, 100_u16, 0x4E4D_5000_003_u64),
    ];

    for (name, n, alpha, seed) in experiments {
        let calibration = synthetic_non_monotone_grid(n, seed);
        let test = synthetic_non_monotone_grid(TEST_SCHEDULES, seed.wrapping_add(1));
        let fit = calibrate(
            &calibration,
            alpha,
            Some(&synthetic_endpoint()),
            &synthetic_revisions(name),
        );

        match fit {
            Ok(result) => {
                let test_loss = test_mean_loss(&test, result.selected_parameter_index);
                output.push_str(&format!(
                    "{name},WHOLE_SCHEDULE,CRC_NM_FINITE_GRID,{},{},{},{},{},{},{:.2},{},{},{},{},{},{:.2},SYNTHETIC_ONLY\n",
                    result.calibration_schedules,
                    result.parameter_count,
                    result.target_risk_milli,
                    result.excess_bound_milli,
                    result.adjusted_risk_milli,
                    result.selected_parameter_index,
                    result.selected_calibration_loss_mean_milli,
                    result.monotonicity_violations,
                    result.endpoint_population_risk_upper_milli,
                    result.endpoint_contract_scope,
                    result.endpoint_evidence_ref,
                    TEST_SCHEDULES,
                    test_loss,
                ));
            }
            Err(RiskControlError::NoPositiveAdjustedTarget { excess_bound_milli, .. }) => {
                output.push_str(&format!(
                    "{name},WHOLE_SCHEDULE,CRC_NM_FINITE_GRID,{n},4,{alpha},{excess_bound_milli},NA,NA,NA,{},0,SYNTHETIC_IID_WHOLE_SCHEDULES,terminal fallback zero-loss by construction,{TEST_SCHEDULES},NA,NO_POSITIVE_ADJUSTED_TARGET\n",
                    calibration.monotonicity_violations()
                ));
            }
            Err(_) => {
                output.push_str(&format!(
                    "{name},WHOLE_SCHEDULE,CRC_NM_FINITE_GRID,{n},4,{alpha},NA,NA,NA,NA,{},0,SYNTHETIC_IID_WHOLE_SCHEDULES,terminal fallback zero-loss by construction,{TEST_SCHEDULES},NA,NO_RISK_CLAIM\n",
                    calibration.monotonicity_violations()
                ));
            }
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correction_decreases_as_schedule_calibration_size_increases() {
        let d120 = excess_bound_milli(4, 120).expect("D120");
        let d1200 = excess_bound_milli(4, 1_200).expect("D1200");
        let d50000 = excess_bound_milli(4, 50_000).expect("D50000");
        assert!(d1200 < d120);
        assert!(d50000 < d1200);
    }

    #[test]
    fn correction_increases_with_candidate_grid_resolution() {
        let d4 = excess_bound_milli(4, 10_000).expect("D4");
        let d16 = excess_bound_milli(16, 10_000).expect("D16");
        assert!(d16 > d4);
    }

    #[test]
    fn non_monotone_policy_family_is_not_silently_monotonized() {
        let grid = synthetic_non_monotone_grid(1_200, 91);
        assert!(grid.monotonicity_violations() > 0);
    }

    #[test]
    fn small_schedule_sample_can_consume_the_entire_ten_percent_budget() {
        let grid = synthetic_non_monotone_grid(120, 92);
        let result = calibrate(
            &grid,
            100,
            Some(&synthetic_endpoint()),
            &synthetic_revisions("small"),
        );
        assert!(matches!(
            result,
            Err(RiskControlError::NoPositiveAdjustedTarget {
                target_risk_milli: 100,
                ..
            })
        ));
    }

    #[test]
    fn selector_requires_an_explicit_population_risk_contract() {
        let grid = synthetic_non_monotone_grid(1_200, 93);
        assert_eq!(
            calibrate(&grid, 200, None, &synthetic_revisions("missing")),
            Err(RiskControlError::MissingEndpointPopulationRiskContract)
        );
    }

    #[test]
    fn endpoint_population_risk_must_fit_adjusted_target() {
        let grid = synthetic_non_monotone_grid(1_200, 94);
        let correction = excess_bound_milli(grid.parameter_count(), grid.schedule_count())
            .expect("finite correction");
        let adjusted = 200_u16 - correction as u16;
        let contract = EndpointPopulationRiskContract {
            scope: "bad endpoint".to_owned(),
            evidence_ref: "test".to_owned(),
            expected_loss_upper_milli: adjusted + 1,
        };
        assert!(matches!(
            calibrate(&grid, 200, Some(&contract), &synthetic_revisions("bad")),
            Err(RiskControlError::EndpointPopulationRiskExceedsAdjustedTarget { .. })
        ));
    }

    #[test]
    fn adjusted_selector_keeps_non_monotone_loss_curve_and_reports_excess() {
        let grid = synthetic_non_monotone_grid(1_200, 0x4E4D_1200_001);
        let result = calibrate(
            &grid,
            200,
            Some(&synthetic_endpoint()),
            &synthetic_revisions("nonmonotone"),
        )
        .expect("finite-grid selector");

        assert!(result.monotonicity_violations > 0);
        assert!(result.excess_bound_milli > 0);
        assert!(result.adjusted_risk_milli < result.target_risk_milli);
        assert_eq!(result.endpoint_population_risk_upper_milli, 0);
        assert_eq!(result.selected_parameter_index, 2);
    }

    #[test]
    fn fifty_thousand_schedule_units_can_make_the_ninety_percent_target_positive() {
        let grid = synthetic_non_monotone_grid(50_000, 0x4E4D_5000_003);
        let result = calibrate(
            &grid,
            100,
            Some(&synthetic_endpoint()),
            &synthetic_revisions("n50000"),
        )
        .expect("nonzero adjusted target at larger calibration size");

        assert!(result.excess_bound_milli < 100);
        assert!(result.adjusted_risk_milli > 0);
    }

    #[test]
    fn deterministic_receipt_is_byte_identical_and_has_three_rows() {
        let first = render_csv();
        let second = render_csv();
        assert_eq!(first, second);
        assert_eq!(first.lines().count(), 4);
        let width = first.lines().next().expect("header").split(',').count();
        assert_eq!(width, 17);
        for row in first.lines().skip(1) {
            assert_eq!(row.split(',').count(), width, "{row}");
        }
    }
}
