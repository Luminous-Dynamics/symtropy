//! Decision-time split-conformal calibration and conservative interval gates.
//!
//! This module implements finite-sample absolute-residual split-conformal rank
//! arithmetic with explicit model/calibration/decision clocks. It does not
//! establish exchangeability, conditional coverage, or live schedule safety.

use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoverageApplicability {
    ExchangeabilitySupported,
    KnownDistributionShift,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoverageClaim {
    MarginalSplitConformalUnderDeclaredExchangeability,
    NoFormalGuaranteeKnownDistributionShift,
    NoFormalGuaranteeApplicabilityUnknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecisionGate {
    Feasible,
    Infeasible,
    Indeterminate,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CalibrationContract {
    pub model_fit_cutoff: u64,
    pub calibration_window_start: u64,
    pub calibration_window_end: u64,
    pub decision_cutoff: u64,
    /// Miscoverage probability in parts per million: 100_000 means 10%.
    pub alpha_ppm: u32,
    pub model_sha256: String,
    pub feature_schema_sha256: String,
    pub calibration_data_sha256: String,
    pub applicability: CoverageApplicability,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CalibrationObservation {
    pub observation_id: String,
    pub prediction_issued_at: u64,
    pub label_available_at: u64,
    pub predicted_value: u64,
    pub observed_value: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CalibrationReport {
    pub sample_count: u64,
    /// One-based finite-sample order-statistic rank.
    pub quantile_rank_1based: u64,
    /// None means the finite-sample rank exceeds n: the interval is unbounded.
    pub radius: Option<u64>,
    pub alpha_ppm: u32,
    pub nominal_coverage_claim_ppm: Option<u32>,
    pub coverage_claim: CoverageClaim,
    pub model_fit_cutoff: u64,
    pub calibration_window_start: u64,
    pub calibration_window_end: u64,
    pub model_sha256: String,
    pub feature_schema_sha256: String,
    pub calibration_data_sha256: String,
    pub decision_cutoff: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConformalInterval {
    pub point_prediction: u64,
    pub lower: u64,
    pub upper: u64,
    pub radius: u64,
    pub sample_count: u64,
    pub quantile_rank_1based: u64,
    pub coverage_claim: CoverageClaim,
    pub nominal_coverage_claim_ppm: Option<u32>,
    pub model_fit_cutoff: u64,
    pub calibration_window_start: u64,
    pub calibration_window_end: u64,
    pub model_sha256: String,
    pub feature_schema_sha256: String,
    pub calibration_data_sha256: String,
    pub decision_cutoff: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CoverageObservation {
    pub covered: bool,
}

fn is_sha256_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn coverage_claim(applicability: CoverageApplicability) -> CoverageClaim {
    match applicability {
        CoverageApplicability::ExchangeabilitySupported => {
            CoverageClaim::MarginalSplitConformalUnderDeclaredExchangeability
        }
        CoverageApplicability::KnownDistributionShift => {
            CoverageClaim::NoFormalGuaranteeKnownDistributionShift
        }
        CoverageApplicability::Unknown => CoverageClaim::NoFormalGuaranteeApplicabilityUnknown,
    }
}

fn validate_contract(contract: &CalibrationContract) -> Result<(), String> {
    if contract.alpha_ppm == 0 || contract.alpha_ppm >= 1_000_000 {
        return Err("alpha_ppm must be strictly between 0 and 1,000,000".to_owned());
    }
    if !is_sha256_digest(&contract.model_sha256)
        || !is_sha256_digest(&contract.feature_schema_sha256)
        || !is_sha256_digest(&contract.calibration_data_sha256)
    {
        return Err(
            "model, feature-schema and calibration-data digests must be lowercase SHA-256 hex"
                .to_owned(),
        );
    }
    if contract.model_fit_cutoff >= contract.calibration_window_start {
        return Err("calibration window must begin strictly after model-fit cutoff".to_owned());
    }
    if contract.calibration_window_start > contract.calibration_window_end {
        return Err("calibration window start must not exceed its end".to_owned());
    }
    if contract.calibration_window_end > contract.decision_cutoff {
        return Err("calibration window must end by the decision cutoff".to_owned());
    }
    Ok(())
}

fn expected_rank_1based(sample_count: u64, alpha_ppm: u32) -> Option<u64> {
    if sample_count == 0 || alpha_ppm == 0 || alpha_ppm >= 1_000_000 {
        return None;
    }
    let numerator = (u128::from(sample_count) + 1)
        .checked_mul(u128::from(1_000_000 - alpha_ppm))?;
    u64::try_from(numerator.div_ceil(1_000_000)).ok()
}

fn valid_coverage_claim(
    coverage_claim: CoverageClaim,
    nominal_coverage_claim_ppm: Option<u32>,
    alpha_ppm: u32,
) -> bool {
    match coverage_claim {
        CoverageClaim::MarginalSplitConformalUnderDeclaredExchangeability => {
            nominal_coverage_claim_ppm == Some(1_000_000 - alpha_ppm)
        }
        CoverageClaim::NoFormalGuaranteeKnownDistributionShift
        | CoverageClaim::NoFormalGuaranteeApplicabilityUnknown => {
            nominal_coverage_claim_ppm.is_none()
        }
    }
}

fn valid_calibration_identity(
    model_sha256: &str,
    feature_schema_sha256: &str,
    calibration_data_sha256: &str,
    model_fit_cutoff: u64,
    calibration_window_start: u64,
    calibration_window_end: u64,
    decision_cutoff: u64,
) -> bool {
    is_sha256_digest(model_sha256)
        && is_sha256_digest(feature_schema_sha256)
        && is_sha256_digest(calibration_data_sha256)
        && model_fit_cutoff < calibration_window_start
        && calibration_window_start <= calibration_window_end
        && calibration_window_end <= decision_cutoff
}

/// Validate the exact finite-sample rank, provenance digests, calibration
/// chronology, claim semantics and endpoint arithmetic of a finite interval.
/// This validates the declared contract, not authenticity of the committed bytes.
pub fn validate_interval(interval: &ConformalInterval) -> Result<(), String> {
    let expected_rank = expected_rank_1based(interval.sample_count, interval.alpha_ppm)
        .ok_or_else(|| "interval has invalid alpha or zero calibration sample count".to_owned())?;
    if expected_rank != interval.quantile_rank_1based {
        return Err("interval rank does not match the exact finite-sample conformal rank".to_owned());
    }
    if expected_rank > interval.sample_count {
        return Err("finite interval supplied for an unbounded conformal rank".to_owned());
    }
    if !valid_calibration_identity(
        &interval.model_sha256,
        &interval.feature_schema_sha256,
        &interval.calibration_data_sha256,
        interval.model_fit_cutoff,
        interval.calibration_window_start,
        interval.calibration_window_end,
        interval.decision_cutoff,
    ) {
        return Err("interval calibration provenance or chronology is invalid".to_owned());
    }
    if !valid_coverage_claim(
        interval.coverage_claim,
        interval.nominal_coverage_claim_ppm,
        interval.alpha_ppm,
    ) {
        return Err("interval nominal coverage claim is inconsistent with its applicability state".to_owned());
    }
    if interval.point_prediction.saturating_sub(interval.radius) != interval.lower
        || interval.point_prediction.checked_add(interval.radius) != Some(interval.upper)
    {
        return Err("interval endpoints do not match point prediction and radius".to_owned());
    }
    Ok(())
}

fn validate_calibration_report(report: &CalibrationReport) -> Result<(), String> {
    let expected_rank = expected_rank_1based(report.sample_count, report.alpha_ppm)
        .ok_or_else(|| "calibration report has invalid alpha or zero sample count".to_owned())?;
    if expected_rank != report.quantile_rank_1based {
        return Err("calibration report rank does not match exact finite-sample rank".to_owned());
    }
    if !valid_calibration_identity(
        &report.model_sha256,
        &report.feature_schema_sha256,
        &report.calibration_data_sha256,
        report.model_fit_cutoff,
        report.calibration_window_start,
        report.calibration_window_end,
        report.decision_cutoff,
    ) {
        return Err("calibration report provenance or chronology is invalid".to_owned());
    }
    if !valid_coverage_claim(
        report.coverage_claim,
        report.nominal_coverage_claim_ppm,
        report.alpha_ppm,
    ) {
        return Err("calibration report coverage claim is inconsistent".to_owned());
    }
    match (report.radius, expected_rank <= report.sample_count) {
        (Some(_), true) | (None, false) => Ok(()),
        (Some(_), false) => Err("finite radius supplied for an unbounded conformal rank".to_owned()),
        (None, true) => Err("unbounded radius supplied despite an available finite conformal rank".to_owned()),
    }
}

/// Calibrate the absolute-residual split-conformal radius.
///
/// Uses the exact finite-sample rank ceil((n + 1) * (1 - alpha)). If that rank
/// exceeds n, returns radius=None rather than inventing a finite quantile.
/// All labels must have become available by the decision cutoff; all calibration
/// predictions must occur strictly after model fit and inside the calibration
/// window.
pub fn calibrate_absolute_residuals(
    contract: &CalibrationContract,
    observations: &[CalibrationObservation],
) -> Result<CalibrationReport, String> {
    validate_contract(contract)?;
    if observations.is_empty() {
        return Err("calibration set must contain at least one observation".to_owned());
    }

    let mut ids = BTreeSet::new();
    let mut residuals = Vec::with_capacity(observations.len());
    for observation in observations {
        if observation.observation_id.trim().is_empty()
            || !ids.insert(observation.observation_id.as_str())
        {
            return Err("calibration observation IDs must be non-empty and unique".to_owned());
        }
        if observation.prediction_issued_at <= contract.model_fit_cutoff
            || observation.prediction_issued_at < contract.calibration_window_start
            || observation.prediction_issued_at > contract.calibration_window_end
        {
            return Err(format!(
                "{}: calibration prediction time is outside the post-fit calibration window",
                observation.observation_id
            ));
        }
        if observation.label_available_at < observation.prediction_issued_at {
            return Err(format!(
                "{}: label cannot become available before its prediction was issued",
                observation.observation_id
            ));
        }
        if observation.label_available_at > contract.decision_cutoff {
            return Err(format!(
                "{}: calibration label was not available by the decision cutoff",
                observation.observation_id
            ));
        }
        residuals.push(
            observation
                .predicted_value
                .abs_diff(observation.observed_value),
        );
    }

    residuals.sort_unstable();
    let sample_count = u64::try_from(residuals.len())
        .map_err(|_| "calibration sample count exceeds u64".to_owned())?;
    let rank_numerator = (u128::from(sample_count) + 1)
        .checked_mul(u128::from(1_000_000 - contract.alpha_ppm))
        .ok_or_else(|| "conformal rank arithmetic overflow".to_owned())?;
    let rank = rank_numerator.div_ceil(1_000_000);
    let quantile_rank_1based =
        u64::try_from(rank).map_err(|_| "conformal rank exceeds u64".to_owned())?;
    let radius = if quantile_rank_1based > sample_count {
        None
    } else {
        Some(residuals[(quantile_rank_1based - 1) as usize])
    };

    let claim = coverage_claim(contract.applicability);
    let nominal_coverage_claim_ppm =
        if contract.applicability == CoverageApplicability::ExchangeabilitySupported {
            Some(1_000_000 - contract.alpha_ppm)
        } else {
            None
        };

    Ok(CalibrationReport {
        sample_count,
        quantile_rank_1based,
        radius,
        alpha_ppm: contract.alpha_ppm,
        nominal_coverage_claim_ppm,
        coverage_claim: claim,
        model_fit_cutoff: contract.model_fit_cutoff,
        calibration_window_start: contract.calibration_window_start,
        calibration_window_end: contract.calibration_window_end,
        model_sha256: contract.model_sha256.clone(),
        feature_schema_sha256: contract.feature_schema_sha256.clone(),
        calibration_data_sha256: contract.calibration_data_sha256.clone(),
        decision_cutoff: contract.decision_cutoff,
    })
}

/// Construct a finite interval around a point prediction. Returns None when
/// the finite-sample conformal rank is unbounded; endpoint overflow is an error.
pub fn build_interval(
    report: &CalibrationReport,
    point_prediction: u64,
) -> Result<Option<ConformalInterval>, String> {
    validate_calibration_report(report)?;
    let Some(radius) = report.radius else {
        return Ok(None);
    };
    let upper = point_prediction
        .checked_add(radius)
        .ok_or_else(|| "conformal upper endpoint overflow".to_owned())?;
    let lower = point_prediction.saturating_sub(radius);

    Ok(Some(ConformalInterval {
        point_prediction,
        lower,
        upper,
        radius,
        sample_count: report.sample_count,
        quantile_rank_1based: report.quantile_rank_1based,
        coverage_claim: report.coverage_claim,
        nominal_coverage_claim_ppm: report.nominal_coverage_claim_ppm,
        model_fit_cutoff: report.model_fit_cutoff,
        calibration_window_start: report.calibration_window_start,
        calibration_window_end: report.calibration_window_end,
        model_sha256: report.model_sha256.clone(),
        feature_schema_sha256: report.feature_schema_sha256.clone(),
        calibration_data_sha256: report.calibration_data_sha256.clone(),
        decision_cutoff: report.decision_cutoff,
    }))
}

/// Check whether a subsequently observed outcome fell inside the interval.
/// This is one observation, not an aggregate coverage estimate.
pub fn assess_coverage(interval: &ConformalInterval, observed_value: u64) -> CoverageObservation {
    CoverageObservation {
        covered: interval.lower <= observed_value && observed_value <= interval.upper,
    }
}

/// Conservative single-deadline gate: latest plausible arrival plus the upper
/// handling-duration bound must meet the deadline. If calibration is unbounded
/// or exchangeability applicability is unsupported, return Indeterminate.
///
/// This checks one deadline only. It does not model berth conflicts, crane
/// capacity, tidal windows, yard resources or cumulative service state.
pub fn assess_deadline_feasibility(
    interval: &ConformalInterval,
    handling_duration_upper: u64,
    deadline: u64,
) -> DecisionGate {
    if validate_interval(interval).is_err()
        || interval.coverage_claim
            != CoverageClaim::MarginalSplitConformalUnderDeclaredExchangeability
    {
        return DecisionGate::Indeterminate;
    }
    let Some(worst_case_finish) = interval.upper.checked_add(handling_duration_upper) else {
        return DecisionGate::Indeterminate;
    };
    if worst_case_finish <= deadline {
        DecisionGate::Feasible
    } else {
        DecisionGate::Infeasible
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(character: char) -> String {
        std::iter::repeat_n(character, 64).collect()
    }

    fn contract(applicability: CoverageApplicability) -> CalibrationContract {
        CalibrationContract {
            model_fit_cutoff: 10,
            calibration_window_start: 11,
            calibration_window_end: 40,
            decision_cutoff: 50,
            alpha_ppm: 100_000,
            model_sha256: digest('a'),
            feature_schema_sha256: digest('b'),
            calibration_data_sha256: digest('c'),
            applicability,
        }
    }

    fn observations(count: usize) -> Vec<CalibrationObservation> {
        (0..count)
            .map(|index| CalibrationObservation {
                observation_id: format!("CALL-{index:03}"),
                prediction_issued_at: 12 + (index as u64 % 20),
                label_available_at: 35,
                predicted_value: 100,
                observed_value: 101 + index as u64,
            })
            .collect()
    }

    #[test]
    fn finite_sample_rank_uses_exact_ceiling_rule() {
        let report = calibrate_absolute_residuals(
            &contract(CoverageApplicability::ExchangeabilitySupported),
            &observations(9),
        )
        .expect("valid calibration");
        assert_eq!(report.sample_count, 9);
        assert_eq!(report.quantile_rank_1based, 9);
        assert_eq!(report.radius, Some(9));
        assert_eq!(report.nominal_coverage_claim_ppm, Some(900_000));
        let interval = build_interval(&report, 100)
            .expect("interval")
            .expect("finite");
        assert_eq!(interval.model_sha256, report.model_sha256);
        assert_eq!(interval.feature_schema_sha256, report.feature_schema_sha256);
        assert_eq!(interval.calibration_data_sha256, report.calibration_data_sha256);
        assert_eq!(interval.model_fit_cutoff, 10);
        assert_eq!(interval.calibration_window_start, 11);
        assert_eq!(interval.calibration_window_end, 40);
    }

    #[test]
    fn insufficient_calibration_sample_produces_unbounded_radius() {
        let report = calibrate_absolute_residuals(
            &contract(CoverageApplicability::ExchangeabilitySupported),
            &observations(8),
        )
        .expect("valid calibration");
        assert_eq!(report.quantile_rank_1based, 9);
        assert_eq!(report.radius, None);
        assert!(
            build_interval(&report, 100)
                .expect("unbounded interval")
                .is_none()
        );
    }

    #[test]
    fn calibration_quantile_is_invariant_to_input_order() {
        let contract = contract(CoverageApplicability::ExchangeabilitySupported);
        let forward = observations(12);
        let mut reverse = forward.clone();
        reverse.reverse();
        assert_eq!(
            calibrate_absolute_residuals(&contract, &forward).expect("forward"),
            calibrate_absolute_residuals(&contract, &reverse).expect("reverse")
        );
    }

    #[test]
    fn calibration_ids_must_be_unique() {
        let mut values = observations(9);
        values[1].observation_id = values[0].observation_id.clone();
        assert!(
            calibrate_absolute_residuals(
                &contract(CoverageApplicability::ExchangeabilitySupported),
                &values
            )
            .is_err()
        );
    }

    #[test]
    fn label_not_available_by_cutoff_is_rejected() {
        let mut values = observations(9);
        values[0].label_available_at = 51;
        assert!(
            calibrate_absolute_residuals(
                &contract(CoverageApplicability::ExchangeabilitySupported),
                &values
            )
            .unwrap_err()
            .contains("not available by the decision cutoff")
        );
    }

    #[test]
    fn fit_and_calibration_chronology_are_enforced() {
        let mut values = observations(9);
        values[0].prediction_issued_at = 10;
        assert!(
            calibrate_absolute_residuals(
                &contract(CoverageApplicability::ExchangeabilitySupported),
                &values
            )
            .unwrap_err()
            .contains("post-fit calibration window")
        );

        let mut values = observations(9);
        values[0].label_available_at = values[0].prediction_issued_at - 1;
        assert!(
            calibrate_absolute_residuals(
                &contract(CoverageApplicability::ExchangeabilitySupported),
                &values
            )
            .unwrap_err()
            .contains("before its prediction")
        );
    }

    #[test]
    fn malformed_digests_and_invalid_alpha_are_rejected() {
        let mut invalid = contract(CoverageApplicability::ExchangeabilitySupported);
        invalid.model_sha256 = "not-a-digest".to_owned();
        assert!(calibrate_absolute_residuals(&invalid, &observations(9)).is_err());

        let mut invalid = contract(CoverageApplicability::ExchangeabilitySupported);
        invalid.alpha_ppm = 0;
        assert!(calibrate_absolute_residuals(&invalid, &observations(9)).is_err());
    }

    #[test]
    fn known_shift_and_unknown_applicability_make_no_nominal_claim_and_cannot_pass_gate() {
        for applicability in [
            CoverageApplicability::KnownDistributionShift,
            CoverageApplicability::Unknown,
        ] {
            let report = calibrate_absolute_residuals(&contract(applicability), &observations(9))
                .expect("calibration arithmetic remains available");
            assert_eq!(report.nominal_coverage_claim_ppm, None);
            let interval = build_interval(&report, 100)
                .expect("build")
                .expect("finite");
            assert_eq!(
                assess_deadline_feasibility(&interval, 5, 200),
                DecisionGate::Indeterminate
            );
        }
    }

    #[test]
    fn robust_deadline_gate_uses_the_upper_prediction_bound() {
        let report = calibrate_absolute_residuals(
            &contract(CoverageApplicability::ExchangeabilitySupported),
            &observations(9),
        )
        .expect("calibration");
        let interval = build_interval(&report, 100)
            .expect("build")
            .expect("finite");
        assert_eq!(interval.lower, 91);
        assert_eq!(interval.upper, 109);
        assert_eq!(
            assess_deadline_feasibility(&interval, 10, 119),
            DecisionGate::Feasible
        );
        assert_eq!(
            assess_deadline_feasibility(&interval, 10, 118),
            DecisionGate::Infeasible
        );
    }

    #[test]
    fn outcome_coverage_and_interval_endpoint_overflow_are_explicit() {
        let report = calibrate_absolute_residuals(
            &contract(CoverageApplicability::ExchangeabilitySupported),
            &observations(9),
        )
        .expect("calibration");
        let interval = build_interval(&report, 100).expect("build").expect("finite");
        assert!(assess_coverage(&interval, 109).covered);
        assert!(!assess_coverage(&interval, 110).covered);
        assert!(build_interval(&report, u64::MAX).is_err());
    }
}
