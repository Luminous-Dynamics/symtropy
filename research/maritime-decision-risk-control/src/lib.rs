pub mod aggregate_cargo_status;
pub mod assurance_envelope;
pub mod clustered_dependence;
pub mod cross_standard_mapping;
pub mod dcsa_maturity;
pub mod high_probability;
pub mod lipschitz;
pub mod oracle_adapter;
pub mod repeated_sampling;
pub mod service_variability;
pub mod source_semantics;
pub mod weighted_shift;

pub mod epcis_corrections;
pub mod non_monotone;

// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

const LOSS_MAX_MILLI: u16 = 1_000;
const TARGET_RISK_MILLI: u16 = 100;
const MONOTONE_GRID: [u16; 5] = [40, 60, 80, 95, 100];
const TEST_SCHEDULES: usize = 1_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LossMatrix {
    /// One row per complete schedule; one column per policy parameter.
    pub schedules: Vec<Vec<u16>>,
}

impl LossMatrix {
    pub fn schedule_count(&self) -> usize {
        self.schedules.len()
    }

    pub fn parameter_count(&self) -> usize {
        self.schedules.first().map_or(0, Vec::len)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MonotonicityMode {
    RequireMonotone,
    UseUpperEnvelope,
}

impl MonotonicityMode {
    fn id(self) -> &'static str {
        match self {
            Self::RequireMonotone => "REQUIRE_MONOTONE",
            Self::UseUpperEnvelope => "MONOTONE_ENVELOPE",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaxParameterContract {
    /// Explicit domain over which the maximum-parameter loss bound is claimed.
    pub scope: String,
    /// Evidence reference for the structural/exhaustive argument; not fetched or verified here.
    pub evidence_ref: String,
    /// Declared upper bound on the loss at the maximum parameter, in milli-loss units.
    pub declared_loss_bound_milli: u16,
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
    MissingMaximumParameterContract,
    MaximumParameterContractExceedsTarget,
    MaximumParameterObservedLossExceedsContract,
    NonMonotoneLoss { violations: usize },
}

#[derive(Clone, Debug, PartialEq)]
pub struct RiskControlResult {
    pub selected_parameter_index: usize,
    pub parameter_count: usize,
    pub calibration_schedules: usize,
    pub target_risk_milli: u16,
    pub observed_loss_sum_milli: u64,
    pub observed_loss_mean_milli: f64,
    pub monotonicity_violations: usize,
    pub monotone_envelope_applied: bool,
    pub maximum_parameter_observed_loss_milli: u16,
    pub maximum_parameter_contract_scope: String,
    pub maximum_parameter_evidence_ref: String,
    pub revisions: FrozenRevisions,
}

fn validate_matrix(matrix: &LossMatrix) -> Result<(), RiskControlError> {
    if matrix.schedules.is_empty() {
        return Err(RiskControlError::EmptyCalibration);
    }
    let width = matrix.parameter_count();
    if width == 0 {
        return Err(RiskControlError::EmptyParameterGrid);
    }
    if matrix.schedules.iter().any(|row| row.len() != width) {
        return Err(RiskControlError::RaggedLossMatrix);
    }
    if matrix
        .schedules
        .iter()
        .flatten()
        .any(|loss| *loss > LOSS_MAX_MILLI)
    {
        return Err(RiskControlError::LossExceedsBound);
    }
    Ok(())
}

/// Counts adjacent increases in loss as the policy parameter becomes more conservative.
pub fn monotonicity_violations(matrix: &LossMatrix) -> usize {
    matrix
        .schedules
        .iter()
        .map(|row| row.windows(2).filter(|pair| pair[1] > pair[0]).count())
        .sum()
}

/// Constructs L_tilde_i(lambda) = sup_{lambda' >= lambda} L_i(lambda').
///
/// The result is non-increasing in the parameter and upper-bounds the original
/// loss at every grid point. This is the monotone-envelope route described in
/// the Conformal Risk Control paper for non-monotone losses.
pub fn upper_monotone_envelope(matrix: &LossMatrix) -> LossMatrix {
    let schedules = matrix
        .schedules
        .iter()
        .map(|row| {
            let mut envelope = row.clone();
            let mut suffix_max = 0_u16;
            for index in (0..row.len()).rev() {
                suffix_max = suffix_max.max(row[index]);
                envelope[index] = suffix_max;
            }
            envelope
        })
        .collect();

    LossMatrix { schedules }
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

/// Fits CRC on whole-schedule losses.
///
/// The guarantee is marginal expected bounded loss over the joint randomness
/// of the exchangeable calibration schedules and the future schedule. It is
/// not a conditional high-probability bound given the realized calibration set.
pub fn calibrate(
    matrix: &LossMatrix,
    target_risk_milli: u16,
    mode: MonotonicityMode,
    maximum_parameter_contract: Option<&MaxParameterContract>,
    revisions: &FrozenRevisions,
) -> Result<RiskControlResult, RiskControlError> {
    validate_matrix(matrix)?;
    validate_revisions(revisions)?;

    if target_risk_milli == 0 || target_risk_milli >= LOSS_MAX_MILLI {
        return Err(RiskControlError::InvalidTarget);
    }

    let contract =
        maximum_parameter_contract.ok_or(RiskControlError::MissingMaximumParameterContract)?;

    if contract.scope.trim().is_empty() || contract.evidence_ref.trim().is_empty() {
        return Err(RiskControlError::MissingMaximumParameterContract);
    }
    if contract.declared_loss_bound_milli > target_risk_milli {
        return Err(RiskControlError::MaximumParameterContractExceedsTarget);
    }

    let violations = monotonicity_violations(matrix);
    let calibrated_matrix = match mode {
        MonotonicityMode::RequireMonotone if violations > 0 => {
            return Err(RiskControlError::NonMonotoneLoss { violations });
        }
        MonotonicityMode::RequireMonotone => matrix.clone(),
        MonotonicityMode::UseUpperEnvelope => upper_monotone_envelope(matrix),
    };

    let last_column = calibrated_matrix.parameter_count() - 1;
    let maximum_parameter_observed_loss = calibrated_matrix
        .schedules
        .iter()
        .map(|row| row[last_column])
        .max()
        .unwrap_or(0);

    // This is a falsification check against the caller's by-design contract,
    // not proof that the contract covers every possible future schedule.
    if maximum_parameter_observed_loss > contract.declared_loss_bound_milli {
        return Err(RiskControlError::MaximumParameterObservedLossExceedsContract);
    }

    let n = calibrated_matrix.schedule_count() as u64;
    let denominator = n + 1;
    let mut selected_parameter_index = last_column;
    let mut selected_sum = 0_u64;

    for parameter_index in 0..calibrated_matrix.parameter_count() {
        let loss_sum = calibrated_matrix
            .schedules
            .iter()
            .map(|row| u64::from(row[parameter_index]))
            .sum::<u64>();

        // CRC condition: (n/(n+1))*mean_loss + B/(n+1) <= alpha.
        // With milli-loss units this is exactly:
        // loss_sum + B <= alpha * (n + 1).
        let lhs = loss_sum + u64::from(LOSS_MAX_MILLI);
        let rhs = u64::from(target_risk_milli) * denominator;

        if lhs <= rhs {
            selected_parameter_index = parameter_index;
            selected_sum = loss_sum;
            break;
        }
    }

    // If no parameter passes the empirical inequality, CRC returns lambda_max.
    // Its expected-risk theorem depends on the maximum-parameter contract above.
    if selected_parameter_index == last_column {
        selected_sum = calibrated_matrix
            .schedules
            .iter()
            .map(|row| u64::from(row[last_column]))
            .sum();
    }

    let observed_loss_mean_milli = selected_sum as f64 / n as f64;

    Ok(RiskControlResult {
        selected_parameter_index,
        parameter_count: calibrated_matrix.parameter_count(),
        calibration_schedules: calibrated_matrix.schedule_count(),
        target_risk_milli,
        observed_loss_sum_milli: selected_sum,
        observed_loss_mean_milli,
        monotonicity_violations: violations,
        monotone_envelope_applied: mode == MonotonicityMode::UseUpperEnvelope,
        maximum_parameter_observed_loss_milli: maximum_parameter_observed_loss,
        maximum_parameter_contract_scope: contract.scope.clone(),
        maximum_parameter_evidence_ref: contract.evidence_ref.clone(),
        revisions: revisions.clone(),
    })
}

fn monotone_fixture(count: usize) -> LossMatrix {
    let schedules = (0..count)
        .map(|index| {
            let severity = (index % 100) as u16;
            MONOTONE_GRID
                .iter()
                .map(|threshold| {
                    if severity > *threshold {
                        LOSS_MAX_MILLI
                    } else {
                        0
                    }
                })
                .collect()
        })
        .collect();

    LossMatrix { schedules }
}

fn non_monotone_fixture(count: usize) -> LossMatrix {
    let schedules = (0..count)
        .map(|index| {
            let severity = (index % 100) as u16;
            let baseline = if severity > 40 { LOSS_MAX_MILLI } else { 0 };
            let extra_buffer = if severity > 60 {
                LOSS_MAX_MILLI
            } else if severity < 10 {
                250
            } else {
                0
            };
            let wider_recourse = if severity > 90 { LOSS_MAX_MILLI } else { 0 };
            let fail_closed_fallback = 0;
            vec![baseline, extra_buffer, wider_recourse, fail_closed_fallback]
        })
        .collect();

    LossMatrix { schedules }
}

fn test_fixture(matrix: &LossMatrix, parameter_index: usize) -> f64 {
    if matrix.schedules.is_empty() {
        return 0.0;
    }

    let sum = matrix
        .schedules
        .iter()
        .map(|row| f64::from(row[parameter_index]))
        .sum::<f64>();

    sum / matrix.schedule_count() as f64
}

fn contract(scope: &str) -> MaxParameterContract {
    MaxParameterContract {
        scope: scope.to_owned(),
        evidence_ref: "synthetic-fixture-by-construction".to_owned(),
        declared_loss_bound_milli: 0,
    }
}

fn revisions(name: &str) -> FrozenRevisions {
    FrozenRevisions {
        policy_revision: format!("{name}-policy-v1"),
        evaluator_revision: format!("{name}-evaluator-v1"),
        loss_contract_revision: format!("{name}-loss-v1"),
    }
}

/// Deterministic research receipt; values are synthetic and are not DCT estimates.
pub fn render_csv() -> String {
    let mut output = String::from(
        "experiment,calibration_unit,monotonicity_mode,policy_revision,evaluator_revision,loss_contract_revision,calibration_schedules,parameter_count,target_risk_milli,selected_parameter_index,selected_calibration_loss_mean_milli,monotonicity_violations,envelope_applied,max_parameter_observed_loss_milli,max_parameter_contract_scope,max_parameter_evidence_ref,test_schedules,test_original_loss_mean_milli,claim_type,status\n",
    );

    let monotone = monotone_fixture(120);
    let monotone_test = monotone_fixture(TEST_SCHEDULES);
    let monotone_fit = calibrate(
        &monotone,
        TARGET_RISK_MILLI,
        MonotonicityMode::RequireMonotone,
        Some(&contract("SYNTHETIC_MONOTONE_POLICY")),
        &revisions("monotone"),
    )
    .expect("valid monotone synthetic fixture");

    let non_monotone = non_monotone_fixture(120);
    let non_monotone_test = non_monotone_fixture(TEST_SCHEDULES);
    let non_monotone_fit = calibrate(
        &non_monotone,
        TARGET_RISK_MILLI,
        MonotonicityMode::UseUpperEnvelope,
        Some(&contract("SYNTHETIC_ENVELOPE_POLICY")),
        &revisions("envelope"),
    )
    .expect("valid monotone-envelope synthetic fixture");

    let rows = [
        (
            "MONOTONE_HARD_FAILURE",
            MonotonicityMode::RequireMonotone,
            &monotone_fit,
            &monotone_test,
        ),
        (
            "NON_MONOTONE_WITH_ENVELOPE",
            MonotonicityMode::UseUpperEnvelope,
            &non_monotone_fit,
            &non_monotone_test,
        ),
    ];

    for (name, mode, fit, test) in rows {
        let test_loss = test_fixture(test, fit.selected_parameter_index);
        output.push_str(&format!(
            "{name},WHOLE_SCHEDULE,{},{},{},{},{},{},{},{},{:.3},{},{},{},{},{},{},{:.3},MARGINAL_EXPECTED_BOUNDED_LOSS,SYNTHETIC_ONLY\n",
            mode.id(),
            fit.revisions.policy_revision,
            fit.revisions.evaluator_revision,
            fit.revisions.loss_contract_revision,
            fit.calibration_schedules,
            fit.parameter_count,
            fit.target_risk_milli,
            fit.selected_parameter_index,
            fit.observed_loss_mean_milli,
            fit.monotonicity_violations,
            fit.monotone_envelope_applied,
            fit.maximum_parameter_observed_loss_milli,
            fit.maximum_parameter_contract_scope,
            fit.maximum_parameter_evidence_ref,
            TEST_SCHEDULES,
            test_loss
        ));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_contract() -> MaxParameterContract {
        contract("SYNTHETIC_TEST_DOMAIN")
    }

    fn valid_revisions() -> FrozenRevisions {
        revisions("test")
    }

    #[test]
    fn monotone_crc_selects_smallest_parameter_passing_finite_sample_condition() {
        let matrix = monotone_fixture(120);
        let result = calibrate(
            &matrix,
            TARGET_RISK_MILLI,
            MonotonicityMode::RequireMonotone,
            Some(&valid_contract()),
            &valid_revisions(),
        )
        .expect("calibration succeeds");

        assert_eq!(result.selected_parameter_index, 3);
        assert_eq!(result.monotonicity_violations, 0);
        assert!(!result.monotone_envelope_applied);
    }

    #[test]
    fn non_monotone_policy_is_rejected_without_envelope() {
        let matrix = non_monotone_fixture(120);
        let result = calibrate(
            &matrix,
            TARGET_RISK_MILLI,
            MonotonicityMode::RequireMonotone,
            Some(&valid_contract()),
            &valid_revisions(),
        );

        assert!(matches!(
            result,
            Err(RiskControlError::NonMonotoneLoss { violations }) if violations > 0
        ));
    }

    #[test]
    fn upper_envelope_is_monotone_and_dominates_original_loss() {
        let matrix = non_monotone_fixture(120);
        let envelope = upper_monotone_envelope(&matrix);

        assert_eq!(monotonicity_violations(&envelope), 0);
        for (original, upper) in matrix.schedules.iter().zip(envelope.schedules.iter()) {
            for (actual, bound) in original.iter().zip(upper.iter()) {
                assert!(bound >= actual);
            }
        }
    }

    #[test]
    fn monotone_envelope_allows_crc_only_after_transforming_loss_family() {
        let matrix = non_monotone_fixture(120);
        let result = calibrate(
            &matrix,
            TARGET_RISK_MILLI,
            MonotonicityMode::UseUpperEnvelope,
            Some(&valid_contract()),
            &valid_revisions(),
        )
        .expect("envelope calibration succeeds");

        assert!(result.monotonicity_violations > 0);
        assert!(result.monotone_envelope_applied);
        assert_eq!(result.selected_parameter_index, 2);
    }

    #[test]
    fn selected_parameter_uses_whole_schedules_as_calibration_units() {
        let matrix = monotone_fixture(120);
        let result = calibrate(
            &matrix,
            TARGET_RISK_MILLI,
            MonotonicityMode::RequireMonotone,
            Some(&valid_contract()),
            &valid_revisions(),
        )
        .expect("calibration succeeds");

        assert_eq!(result.calibration_schedules, 120);
        assert!((result.observed_loss_mean_milli - (4_000.0 / 120.0)).abs() < 1e-9);
    }

    #[test]
    fn maximum_parameter_contract_is_required() {
        let result = calibrate(
            &monotone_fixture(120),
            TARGET_RISK_MILLI,
            MonotonicityMode::RequireMonotone,
            None,
            &valid_revisions(),
        );

        assert_eq!(
            result,
            Err(RiskControlError::MissingMaximumParameterContract)
        );
    }

    #[test]
    fn maximum_parameter_contract_must_meet_target_by_design() {
        let mut bad_contract = valid_contract();
        bad_contract.declared_loss_bound_milli = TARGET_RISK_MILLI + 1;

        let result = calibrate(
            &monotone_fixture(120),
            TARGET_RISK_MILLI,
            MonotonicityMode::RequireMonotone,
            Some(&bad_contract),
            &valid_revisions(),
        );

        assert_eq!(
            result,
            Err(RiskControlError::MaximumParameterContractExceedsTarget)
        );
    }

    #[test]
    fn calibration_data_can_falsify_declared_maximum_parameter_contract() {
        let mut matrix = monotone_fixture(120);
        matrix.schedules[0] = vec![10; 5];

        let result = calibrate(
            &matrix,
            TARGET_RISK_MILLI,
            MonotonicityMode::RequireMonotone,
            Some(&valid_contract()),
            &valid_revisions(),
        );

        assert_eq!(
            result,
            Err(RiskControlError::MaximumParameterObservedLossExceedsContract)
        );
    }

    #[test]
    fn bounded_loss_contract_rejects_values_above_one() {
        let mut matrix = monotone_fixture(120);
        matrix.schedules[0][0] = LOSS_MAX_MILLI + 1;

        assert_eq!(
            calibrate(
                &matrix,
                TARGET_RISK_MILLI,
                MonotonicityMode::RequireMonotone,
                Some(&valid_contract()),
                &valid_revisions()
            ),
            Err(RiskControlError::LossExceedsBound)
        );
    }

    #[test]
    fn missing_policy_evaluator_or_loss_revision_is_rejected() {
        let mut ids = valid_revisions();
        ids.evaluator_revision.clear();

        assert_eq!(
            calibrate(
                &monotone_fixture(120),
                TARGET_RISK_MILLI,
                MonotonicityMode::RequireMonotone,
                Some(&valid_contract()),
                &ids
            ),
            Err(RiskControlError::MissingRevision)
        );
    }

    #[test]
    fn crc_empirical_threshold_matches_paper_formula() {
        let matrix = LossMatrix {
            schedules: vec![
                vec![1_000, 0],
                vec![0, 0],
                vec![0, 0],
                vec![0, 0],
                vec![0, 0],
            ],
        };
        let result = calibrate(
            &matrix,
            250,
            MonotonicityMode::RequireMonotone,
            Some(&MaxParameterContract {
                scope: "small fixture".to_owned(),
                evidence_ref: "constructed fallback".to_owned(),
                declared_loss_bound_milli: 0,
            }),
            &valid_revisions(),
        )
        .expect("fallback contract");

        // n=5, B=1, alpha=.25: the empirical mean .20 fails
        // .25 - (.75 / 5) = .10; lambda_max is the first passing grid point.
        assert_eq!(result.selected_parameter_index, 1);
        assert_eq!(result.observed_loss_mean_milli, 0.0);
    }

    #[test]
    fn deterministic_receipt_is_byte_identical() {
        assert_eq!(render_csv(), render_csv());
    }

    #[test]
    fn csv_schema_has_twenty_columns_and_two_synthetic_rows() {
        let output = render_csv();
        let mut lines = output.lines();
        let header = lines.next().expect("CSV header");
        let width = header.split(',').count();

        assert_eq!(width, 20);
        let rows = lines.collect::<Vec<_>>();
        assert_eq!(rows.len(), 2);
        for row in rows {
            assert_eq!(row.split(',').count(), width, "{row}");
        }
    }
}
