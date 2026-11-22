//! Offline risk-coverage evaluator for navigation service decisions.
//!
//! Confidence thresholds convert low-confidence claims into explicit
//! abstentions. Metrics retain false-safe and false-degraded outcomes
//! separately and report both record- and exposure-time coverage.
//!
//! The Wilson upper bound is a per-record Bernoulli approximation. It does
//! not account for clustered samples or common-cause dependence; do not use it
//! as an integrity certificate.

use std::collections::BTreeSet;
use std::fmt;

const SCALE_PPM: u128 = 1_000_000;
const WILSON_Z_95: f64 = 1.959_963_984_540_054;

/// Evaluator-side record for one controller service decision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluationCase {
    pub record_id: String,
    /// Controller-provided confidence in the service-state claim, in ppm.
    pub confidence_ppm: u32,
    /// true means the controller claims StateOne (service safe/in envelope).
    pub claim_state_one: bool,
    /// Evaluator-only truth. Never pass this field to the controller.
    pub truth_state_one: bool,
    /// Duration represented by this decision, used for exposure accounting.
    pub exposure_seconds: u64,
}

/// Application-specific cost of a false-safe and false-degraded service claim.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServiceLoss {
    /// Cost per exposed second when StateOne is claimed but StateZero is true.
    pub false_safe_loss_micros_per_second: u64,
    /// Cost per exposed second when StateZero is claimed but StateOne is true.
    pub false_degraded_loss_micros_per_second: u64,
}

impl ServiceLoss {
    pub fn validate(self) -> Result<(), RiskCoverageError> {
        if self.false_safe_loss_micros_per_second == 0
            && self.false_degraded_loss_micros_per_second == 0
        {
            return Err(RiskCoverageError::InvalidLoss);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RiskCoverageError {
    EmptyCaseSet,
    EmptyRecordId,
    DuplicateRecordId(String),
    InvalidConfidence(String),
    ZeroExposure(String),
    InvalidLoss,
}

impl fmt::Display for RiskCoverageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCaseSet => write!(f, "at least one evaluation case is required"),
            Self::EmptyRecordId => write!(f, "record ID must not be empty"),
            Self::DuplicateRecordId(id) => write!(f, "duplicate record ID: {id}"),
            Self::InvalidConfidence(id) => {
                write!(f, "confidence must be within 0..=1,000,000 ppm: {id}")
            }
            Self::ZeroExposure(id) => write!(f, "exposure duration must be positive: {id}"),
            Self::InvalidLoss => write!(f, "false-safe and false-degraded loss cannot both be zero"),
        }
    }
}
impl std::error::Error for RiskCoverageError {}

/// Complete risk/coverage record at one confidence threshold.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RiskCoveragePoint {
    pub threshold_ppm: u32,
    pub total_records: usize,
    pub covered_records: usize,
    pub abstained_records: usize,
    /// Covered records / all records, scaled to parts per million.
    pub record_coverage_ppm: u32,
    pub total_exposure_seconds: u128,
    pub covered_exposure_seconds: u128,
    /// Covered exposure seconds / total exposure seconds, ppm.
    pub exposure_coverage_ppm: u32,
    pub false_safe_records: usize,
    pub false_degraded_records: usize,
    pub false_safe_exposure_seconds: u128,
    pub false_degraded_exposure_seconds: u128,
    /// Unweighted errors among covered records.
    pub selective_error_records: usize,
    pub selective_risk_ppm: Option<u32>,
    /// False-safe cases / covered cases; None when no cases are covered.
    pub false_safe_rate_ppm: Option<u32>,
    /// One-sided 95% Wilson upper bound for false-safe record rate.
    /// Assumes independent Bernoulli records; clustered samples need a
    /// separate cluster bootstrap or hierarchical model.
    pub false_safe_wilson_upper_95_ppm: Option<u32>,
    /// Sum(loss-per-second × exposed seconds) for false-safe and false-degraded decisions.
    pub weighted_exposure_loss_micros_seconds: u128,
    /// Weighted exposure loss per covered hour; None when coverage is zero.
    pub weighted_loss_per_covered_hour_micros: Option<u128>,
}

fn ratio_ppm(numerator: u128, denominator: u128) -> u32 {
    if denominator == 0 {
        return 0;
    }
    ((numerator.saturating_mul(SCALE_PPM) / denominator).min(SCALE_PPM)) as u32
}

/// One-sided Wilson-score upper bound for a binomial event rate.
pub fn wilson_upper_95_ppm(events: usize, trials: usize) -> Option<u32> {
    if trials == 0 || events > trials {
        return None;
    }
    let n = trials as f64;
    let p = events as f64 / n;
    let z2 = WILSON_Z_95 * WILSON_Z_95;
    let denominator = 1.0 + z2 / n;
    let centre = p + z2 / (2.0 * n);
    let radius = WILSON_Z_95
        * ((p * (1.0 - p) / n) + z2 / (4.0 * n * n)).sqrt();
    let upper = ((centre + radius) / denominator).clamp(0.0, 1.0);
    Some((upper * SCALE_PPM as f64).ceil().min(SCALE_PPM as f64) as u32)
}

fn validate_cases(cases: &[EvaluationCase], loss: ServiceLoss) -> Result<(), RiskCoverageError> {
    if cases.is_empty() {
        return Err(RiskCoverageError::EmptyCaseSet);
    }
    loss.validate()?;
    let mut ids = BTreeSet::new();
    for case in cases {
        if case.record_id.trim().is_empty() {
            return Err(RiskCoverageError::EmptyRecordId);
        }
        if !ids.insert(case.record_id.clone()) {
            return Err(RiskCoverageError::DuplicateRecordId(case.record_id.clone()));
        }
        if case.confidence_ppm > SCALE_PPM as u32 {
            return Err(RiskCoverageError::InvalidConfidence(
                case.record_id.clone(),
            ));
        }
        if case.exposure_seconds == 0 {
            return Err(RiskCoverageError::ZeroExposure(case.record_id.clone()));
        }
    }
    Ok(())
}

/// Evaluate one confidence threshold. Cases below the threshold abstain;
/// they are not counted as correct, wrong, or covered.
pub fn evaluate_at_threshold(
    cases: &[EvaluationCase],
    threshold_ppm: u32,
    loss: ServiceLoss,
) -> Result<RiskCoveragePoint, RiskCoverageError> {
    validate_cases(cases, loss)?;
    if threshold_ppm > SCALE_PPM as u32 + 1 {
        return Err(RiskCoverageError::InvalidConfidence(
            "threshold".to_owned(),
        ));
    }

    let mut covered_records = 0_usize;
    let mut false_safe_records = 0_usize;
    let mut false_degraded_records = 0_usize;
    let mut false_safe_exposure_seconds = 0_u128;
    let mut false_degraded_exposure_seconds = 0_u128;
    let mut total_exposure_seconds = 0_u128;
    let mut covered_exposure_seconds = 0_u128;

    for case in cases {
        total_exposure_seconds =
            total_exposure_seconds.saturating_add(case.exposure_seconds as u128);
        if case.confidence_ppm < threshold_ppm {
            continue;
        }

        covered_records += 1;
        covered_exposure_seconds =
            covered_exposure_seconds.saturating_add(case.exposure_seconds as u128);
        match (case.claim_state_one, case.truth_state_one) {
            (true, false) => {
                false_safe_records += 1;
                false_safe_exposure_seconds =
                    false_safe_exposure_seconds.saturating_add(case.exposure_seconds as u128);
            }
            (false, true) => {
                false_degraded_records += 1;
                false_degraded_exposure_seconds =
                    false_degraded_exposure_seconds.saturating_add(case.exposure_seconds as u128);
            }
            _ => {}
        }
    }

    let abstained_records = cases.len() - covered_records;
    let selective_error_records = false_safe_records + false_degraded_records;
    let selective_risk_ppm = (covered_records > 0).then(|| {
        ratio_ppm(
            selective_error_records as u128,
            covered_records as u128,
        )
    });
    let false_safe_rate_ppm = (covered_records > 0).then(|| {
        ratio_ppm(
            false_safe_records as u128,
            covered_records as u128,
        )
    });
    let false_safe_wilson_upper_95_ppm =
        wilson_upper_95_ppm(false_safe_records, covered_records);

    let weighted_exposure_loss_micros_seconds = (loss
        .false_safe_loss_micros_per_second as u128)
        .saturating_mul(false_safe_exposure_seconds)
        .saturating_add(
            (loss.false_degraded_loss_micros_per_second as u128)
                .saturating_mul(false_degraded_exposure_seconds),
        );
    let weighted_loss_per_covered_hour_micros = (covered_exposure_seconds > 0).then(|| {
        weighted_exposure_loss_micros_seconds
            .saturating_mul(3600)
            / covered_exposure_seconds
    });

    Ok(RiskCoveragePoint {
        threshold_ppm,
        total_records: cases.len(),
        covered_records,
        abstained_records,
        record_coverage_ppm: ratio_ppm(
            covered_records as u128,
            cases.len() as u128,
        ),
        total_exposure_seconds,
        covered_exposure_seconds,
        exposure_coverage_ppm: ratio_ppm(
            covered_exposure_seconds,
            total_exposure_seconds,
        ),
        false_safe_records,
        false_degraded_records,
        false_safe_exposure_seconds,
        false_degraded_exposure_seconds,
        selective_error_records,
        selective_risk_ppm,
        false_safe_rate_ppm,
        false_safe_wilson_upper_95_ppm,
        weighted_exposure_loss_micros_seconds,
        weighted_loss_per_covered_hour_micros,
    })
}

/// Evaluate a deterministic risk–coverage curve at threshold 0, every
/// observed confidence value, and 1,000,001 ppm (explicit all-abstain point).
/// Thresholds are returned in ascending order; coverage is nonincreasing.
pub fn risk_coverage_curve(
    cases: &[EvaluationCase],
    loss: ServiceLoss,
) -> Result<Vec<RiskCoveragePoint>, RiskCoverageError> {
    validate_cases(cases, loss)?;
    let mut thresholds = BTreeSet::from([0_u32, 1_000_001_u32]);
    for case in cases {
        thresholds.insert(case.confidence_ppm);
    }
    thresholds
        .into_iter()
        .map(|threshold| evaluate_at_threshold(cases, threshold, loss))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cases() -> Vec<EvaluationCase> {
        vec![
            EvaluationCase {
                record_id: "a".to_owned(),
                confidence_ppm: 950_000,
                claim_state_one: true,
                truth_state_one: true,
                exposure_seconds: 3600,
            },
            EvaluationCase {
                record_id: "b".to_owned(),
                confidence_ppm: 900_000,
                claim_state_one: true,
                truth_state_one: false,
                exposure_seconds: 1800,
            },
            EvaluationCase {
                record_id: "c".to_owned(),
                confidence_ppm: 650_000,
                claim_state_one: false,
                truth_state_one: true,
                exposure_seconds: 900,
            },
            EvaluationCase {
                record_id: "d".to_owned(),
                confidence_ppm: 200_000,
                claim_state_one: false,
                truth_state_one: false,
                exposure_seconds: 900,
            },
        ]
    }

    fn loss() -> ServiceLoss {
        ServiceLoss {
            false_safe_loss_micros_per_second: 10,
            false_degraded_loss_micros_per_second: 1,
        }
    }

    #[test]
    fn zero_threshold_measures_full_coverage_and_both_error_types() {
        let point = evaluate_at_threshold(&cases(), 0, loss()).unwrap();
        assert_eq!(point.total_records, 4);
        assert_eq!(point.covered_records, 4);
        assert_eq!(point.abstained_records, 0);
        assert_eq!(point.record_coverage_ppm, 1_000_000);
        assert_eq!(point.false_safe_records, 1);
        assert_eq!(point.false_degraded_records, 1);
        assert_eq!(point.false_safe_exposure_seconds, 1800);
        assert_eq!(point.false_degraded_exposure_seconds, 900);
        assert_eq!(point.selective_risk_ppm, Some(500_000));
        assert_eq!(point.false_safe_rate_ppm, Some(250_000));
        assert!(point.false_safe_wilson_upper_95_ppm.unwrap() >= 250_000);
    }

    #[test]
    fn raising_confidence_threshold_can_reduce_risk_and_coverage() {
        let low = evaluate_at_threshold(&cases(), 0, loss()).unwrap();
        let high = evaluate_at_threshold(&cases(), 925_000, loss()).unwrap();
        assert!(high.record_coverage_ppm < low.record_coverage_ppm);
        assert_eq!(high.covered_records, 1);
        assert_eq!(high.false_safe_records, 0);
        assert_eq!(high.false_degraded_records, 0);
        assert_eq!(high.selective_risk_ppm, Some(0));
    }

    #[test]
    fn all_abstention_is_zero_coverage_not_a_zero_risk_claim() {
        let point = evaluate_at_threshold(&cases(), 1_000_001, loss()).unwrap();
        assert_eq!(point.covered_records, 0);
        assert_eq!(point.abstained_records, 4);
        assert_eq!(point.record_coverage_ppm, 0);
        assert_eq!(point.exposure_coverage_ppm, 0);
        assert_eq!(point.selective_risk_ppm, None);
        assert_eq!(point.false_safe_rate_ppm, None);
        assert_eq!(point.false_safe_wilson_upper_95_ppm, None);
        assert_eq!(point.weighted_loss_per_covered_hour_micros, None);
    }

    #[test]
    fn asymmetric_loss_and_exposure_are_reported_explicitly() {
        let point = evaluate_at_threshold(&cases(), 0, loss()).unwrap();
        // false-safe: 10 * 1800; false-degraded: 1 * 900.
        assert_eq!(point.weighted_exposure_loss_micros_seconds, 18_900);
        // covered time is 7200 seconds (2 hours).
        assert_eq!(point.weighted_loss_per_covered_hour_micros, Some(9_450));
    }

    #[test]
    fn risk_coverage_curve_contains_full_and_zero_coverage_endpoints() {
        let curve = risk_coverage_curve(&cases(), loss()).unwrap();
        assert_eq!(curve.first().unwrap().threshold_ppm, 0);
        assert_eq!(curve.last().unwrap().threshold_ppm, 1_000_001);
        assert_eq!(curve.first().unwrap().record_coverage_ppm, 1_000_000);
        assert_eq!(curve.last().unwrap().record_coverage_ppm, 0);
        assert!(curve.windows(2).all(|window| {
            window[0].record_coverage_ppm >= window[1].record_coverage_ppm
        }));
    }

    #[test]
    fn evaluation_is_invariant_to_input_order() {
        let mut forward = cases();
        let mut reversed = cases();
        reversed.reverse();
        assert_eq!(
            risk_coverage_curve(&forward, loss()).unwrap(),
            risk_coverage_curve(&reversed, loss()).unwrap()
        );
        forward.reverse();
    }

    #[test]
    fn invalid_inputs_fail_closed() {
        assert_eq!(
            evaluate_at_threshold(&[], 0, loss()),
            Err(RiskCoverageError::EmptyCaseSet)
        );
        assert_eq!(
            evaluate_at_threshold(&cases(), 0, ServiceLoss {
                false_safe_loss_micros_per_second: 0,
                false_degraded_loss_micros_per_second: 0,
            }),
            Err(RiskCoverageError::InvalidLoss)
        );

        let mut invalid = cases();
        invalid[0].confidence_ppm = 1_000_001;
        assert_eq!(
            evaluate_at_threshold(&invalid, 0, loss()),
            Err(RiskCoverageError::InvalidConfidence("a".to_owned()))
        );

        let mut zero_exposure = cases();
        zero_exposure[0].exposure_seconds = 0;
        assert_eq!(
            evaluate_at_threshold(&zero_exposure, 0, loss()),
            Err(RiskCoverageError::ZeroExposure("a".to_owned()))
        );

        let mut duplicate = cases();
        duplicate[1].record_id = "a".to_owned();
        assert_eq!(
            evaluate_at_threshold(&duplicate, 0, loss()),
            Err(RiskCoverageError::DuplicateRecordId("a".to_owned()))
        );
    }

    #[test]
    fn wilson_bound_is_absent_for_zero_trials_and_above_observed_rate() {
        assert_eq!(wilson_upper_95_ppm(0, 0), None);
        assert_eq!(wilson_upper_95_ppm(2, 1), None);
        let bound = wilson_upper_95_ppm(1, 100).unwrap();
        assert!(bound > 10_000);
        let larger_sample = wilson_upper_95_ppm(10, 1000).unwrap();
        assert!(larger_sample < bound);
    }
}
