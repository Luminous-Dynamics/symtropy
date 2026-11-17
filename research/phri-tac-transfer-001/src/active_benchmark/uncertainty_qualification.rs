//! Group-conditional split-conformal prediction-interval coverage oracle.
//!
//! This module qualifies empirical predictive interval coverage by physical
//! context strata. Calibration and evaluation sample identities must be disjoint.
//! Reference-unresolved and out-of-domain calibration samples are excluded;
//! evaluation samples in those states are abstentions, not scored misses or passes.
//! No global threshold is silently borrowed by an under-sampled stratum.
//!
//! This validates predictive intervals only. It is not a traceable Type A/Type B
//! metrological uncertainty budget, physical reference certificate, or proof
//! of coverage under arbitrary distribution shift.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum ReferenceEvidenceDisposition {
    IndependentlyQualified,
    SharedCommonCause,
    Unresolved,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CalibrationObservation {
    pub sample_id: String,
    pub stratum_id: String,
    pub context_digest: String,
    /// Absolute evaluator residual in the declared output unit.
    pub absolute_error: f64,
    /// Positive scale predicted without using this sample's held-out target.
    pub predicted_scale: f64,
    pub in_qualified_domain: bool,
    pub reference_evidence: ReferenceEvidenceDisposition,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvaluationObservation {
    pub sample_id: String,
    pub stratum_id: String,
    pub context_digest: String,
    /// Absolute evaluator residual; this value is never an input to fitting.
    pub absolute_error: f64,
    pub predicted_scale: f64,
    pub in_qualified_domain: bool,
    pub reference_evidence: ReferenceEvidenceDisposition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum ConformalFitDisposition {
    Fitted,
    NoAdmissibleCalibrationData,
    InsufficientCalibrationSamples,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct StratumConformalThreshold {
    pub stratum_id: String,
    pub context_digest: String,
    pub calibration_sample_count: usize,
    pub rank_one_based: Option<usize>,
    pub score_quantile: Option<f64>,
    pub disposition: ConformalFitDisposition,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct SplitConformalModel {
    pub nominal_coverage: f64,
    pub minimum_calibration_samples: usize,
    pub groups: Vec<StratumConformalThreshold>,
    /// Stored to prevent calibration/evaluation identity leakage.
    pub calibration_sample_ids: Vec<String>,
    pub excluded_out_of_domain_calibration_count: usize,
    pub excluded_reference_unresolved_calibration_count: usize,
    pub claim_class: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum IntervalEvaluationDisposition {
    Covered,
    Missed,
    AbstainedOutOfDomain,
    AbstainedReferenceUnresolved,
    NoCalibratorForStratum,
    InsufficientStratumCalibration,
    InvalidPredictionScale,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct SampleIntervalResult {
    pub sample_id: String,
    pub stratum_id: String,
    pub disposition: IntervalEvaluationDisposition,
    pub interval_width: Option<f64>,
    pub normalized_absolute_error: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct StratumCoverageReport {
    pub stratum_id: String,
    pub context_digest: String,
    pub nominal_coverage: f64,
    pub evaluated_count: usize,
    pub covered_count: usize,
    pub empirical_coverage: Option<f64>,
    pub wilson_95_lower: Option<f64>,
    pub wilson_95_upper: Option<f64>,
    pub mean_interval_width: Option<f64>,
    pub abstained_out_of_domain_count: usize,
    pub abstained_reference_unresolved_count: usize,
    pub no_calibrator_count: usize,
    pub insufficient_calibration_count: usize,
    pub invalid_prediction_scale_count: usize,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct SplitConformalEvaluationReport {
    pub nominal_coverage: f64,
    pub sample_count: usize,
    pub scored_count: usize,
    pub covered_count: usize,
    pub missed_count: usize,
    pub overall_empirical_coverage: Option<f64>,
    pub overall_wilson_95_lower: Option<f64>,
    pub overall_wilson_95_upper: Option<f64>,
    pub abstained_out_of_domain_count: usize,
    pub abstained_reference_unresolved_count: usize,
    pub no_calibrator_count: usize,
    pub insufficient_calibration_count: usize,
    pub invalid_prediction_scale_count: usize,
    pub strata: Vec<StratumCoverageReport>,
    pub sample_results: Vec<SampleIntervalResult>,
    pub claim_class: &'static str,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct SyntheticConformalExample {
    pub model: SplitConformalModel,
    pub evaluation: SplitConformalEvaluationReport,
    pub claim_class: &'static str,
}

#[derive(Clone, Debug, Default)]
struct MutableStratumCoverage {
    evaluated_count: usize,
    covered_count: usize,
    interval_width_sum: f64,
    abstained_out_of_domain_count: usize,
    abstained_reference_unresolved_count: usize,
    no_calibrator_count: usize,
    insufficient_calibration_count: usize,
    invalid_prediction_scale_count: usize,
}

fn validate_sample_identity(
    sample_id: &str,
    stratum_id: &str,
    context_digest: &str,
) -> Result<(), String> {
    if sample_id.trim().is_empty() || stratum_id.trim().is_empty() || context_digest.trim().is_empty() {
        return Err("sample, stratum and context identities must be nonempty".to_owned());
    }
    Ok(())
}

fn validate_error_scale(absolute_error: f64, predicted_scale: f64) -> Result<(), String> {
    if !absolute_error.is_finite() || absolute_error < 0.0 {
        return Err("absolute calibration/evaluation error must be finite and nonnegative".to_owned());
    }
    if !predicted_scale.is_finite() || predicted_scale <= 0.0 {
        return Err("predicted scale must be finite and strictly positive".to_owned());
    }
    Ok(())
}

fn finite_sample_rank(sample_count: usize, nominal_coverage: f64) -> Result<usize, String> {
    let adjusted_count = sample_count
        .checked_add(1)
        .ok_or_else(|| "finite-sample conformal rank overflows usize".to_owned())?;
    let rank = (adjusted_count as f64 * nominal_coverage).ceil();
    if !rank.is_finite() || rank < 1.0 || rank > usize::MAX as f64 {
        return Err("finite-sample conformal rank is outside usize range".to_owned());
    }
    Ok(rank as usize)
}

/// Fit a separate normalized absolute-residual quantile for each exact
/// (stratum, context) pair. Scores are |error| / predicted_scale. A group with
/// too few valid calibration samples receives no finite threshold.
pub fn fit_groupwise_split_conformal(
    observations: &[CalibrationObservation],
    nominal_coverage: f64,
    minimum_calibration_samples: usize,
) -> Result<SplitConformalModel, String> {
    if !nominal_coverage.is_finite() || !(0.0..1.0).contains(&nominal_coverage) {
        return Err("nominal conformal coverage must lie strictly between zero and one".to_owned());
    }
    if minimum_calibration_samples == 0 {
        return Err("minimum calibration sample count must be positive".to_owned());
    }

    let mut seen_ids = BTreeSet::new();
    let mut scores: BTreeMap<(String, String), Vec<f64>> = BTreeMap::new();
    let mut all_groups: BTreeSet<(String, String)> = BTreeSet::new();
    let mut excluded_out_of_domain = 0;
    let mut excluded_reference_unresolved = 0;

    for observation in observations {
        validate_sample_identity(
            &observation.sample_id,
            &observation.stratum_id,
            &observation.context_digest,
        )?;
        validate_error_scale(observation.absolute_error, observation.predicted_scale)?;
        if !seen_ids.insert(observation.sample_id.clone()) {
            return Err(format!("duplicate calibration sample ID {}", observation.sample_id));
        }
        let key = (
            observation.stratum_id.clone(),
            observation.context_digest.clone(),
        );
        all_groups.insert(key.clone());

        if !observation.in_qualified_domain {
            excluded_out_of_domain += 1;
            continue;
        }
        if observation.reference_evidence != ReferenceEvidenceDisposition::IndependentlyQualified {
            excluded_reference_unresolved += 1;
            continue;
        }

        let score = observation.absolute_error / observation.predicted_scale;
        if !score.is_finite() {
            return Err("normalized calibration score is non-finite".to_owned());
        }
        scores.entry(key).or_default().push(score);
    }

    let mut groups = Vec::with_capacity(all_groups.len());
    for (stratum_id, context_digest) in all_groups {
        let mut group_scores = scores
            .remove(&(stratum_id.clone(), context_digest.clone()))
            .unwrap_or_default();
        let calibration_sample_count = group_scores.len();
        let rank = if calibration_sample_count >= minimum_calibration_samples {
            Some(finite_sample_rank(calibration_sample_count, nominal_coverage)?)
        } else {
            None
        };

        let threshold = match rank {
            Some(rank) if rank <= calibration_sample_count => {
                group_scores.sort_by(f64::total_cmp);
                Some(group_scores[rank - 1])
            }
            _ => None,
        };
        let disposition = if calibration_sample_count == 0 {
            ConformalFitDisposition::NoAdmissibleCalibrationData
        } else if threshold.is_some() {
            ConformalFitDisposition::Fitted
        } else {
            ConformalFitDisposition::InsufficientCalibrationSamples
        };
        groups.push(StratumConformalThreshold {
            stratum_id,
            context_digest,
            calibration_sample_count,
            rank_one_based: rank.filter(|rank| *rank <= calibration_sample_count),
            score_quantile: threshold,
            disposition,
        });
    }

    Ok(SplitConformalModel {
        nominal_coverage,
        minimum_calibration_samples,
        groups,
        calibration_sample_ids: seen_ids.into_iter().collect(),
        excluded_out_of_domain_calibration_count: excluded_out_of_domain,
        excluded_reference_unresolved_calibration_count: excluded_reference_unresolved,
        claim_class: "synthetic_groupwise_split_conformal_prediction_only",
    })
}

fn wilson_interval(successes: usize, trials: usize) -> (Option<f64>, Option<f64>) {
    if trials == 0 {
        return (None, None);
    }
    let z = 1.959_963_984_540_054_f64;
    let n = trials as f64;
    let p = successes as f64 / n;
    let z_squared = z * z;
    let denominator = 1.0 + z_squared / n;
    let center = p + z_squared / (2.0 * n);
    let half_width = z
        * (p * (1.0 - p) / n + z_squared / (4.0 * n * n))
            .sqrt();
    (
        Some(((center - half_width) / denominator).max(0.0)),
        Some(((center + half_width) / denominator).min(1.0)),
    )
}

/// Evaluate only on a disjoint held-out sample set. Samples outside the
/// qualified domain, with unresolved references, or without a stratum-specific
/// threshold are abstentions and are counted separately from interval misses.
pub fn evaluate_groupwise_split_conformal(
    model: &SplitConformalModel,
    observations: &[EvaluationObservation],
) -> Result<SplitConformalEvaluationReport, String> {
    let calibration_ids: BTreeSet<_> = model.calibration_sample_ids.iter().cloned().collect();
    let mut evaluation_ids = BTreeSet::new();
    let mut mutable: BTreeMap<(String, String), MutableStratumCoverage> = BTreeMap::new();
    let mut sample_results = Vec::with_capacity(observations.len());
    let mut total_scored = 0;
    let mut total_covered = 0;
    let mut total_missed = 0;
    let mut total_out_of_domain = 0;
    let mut total_reference_unresolved = 0;
    let mut total_no_calibrator = 0;
    let mut total_insufficient = 0;
    let mut total_invalid_scale = 0;

    for observation in observations {
        validate_sample_identity(
            &observation.sample_id,
            &observation.stratum_id,
            &observation.context_digest,
        )?;
        validate_error_scale(observation.absolute_error, observation.predicted_scale)?;
        if !evaluation_ids.insert(observation.sample_id.clone()) {
            return Err(format!("duplicate evaluation sample ID {}", observation.sample_id));
        }
        if calibration_ids.contains(&observation.sample_id) {
            return Err(format!(
                "calibration/evaluation sample leakage for ID {}",
                observation.sample_id
            ));
        }

        let key = (
            observation.stratum_id.clone(),
            observation.context_digest.clone(),
        );
        let group = mutable.entry(key.clone()).or_default();

        let disposition = if !observation.in_qualified_domain {
            group.abstained_out_of_domain_count += 1;
            total_out_of_domain += 1;
            IntervalEvaluationDisposition::AbstainedOutOfDomain
        } else if observation.reference_evidence
            != ReferenceEvidenceDisposition::IndependentlyQualified
        {
            group.abstained_reference_unresolved_count += 1;
            total_reference_unresolved += 1;
            IntervalEvaluationDisposition::AbstainedReferenceUnresolved
        } else {
            match model.groups.iter().find(|candidate| {
                candidate.stratum_id == observation.stratum_id
                    && candidate.context_digest == observation.context_digest
            }) {
                None => {
                    group.no_calibrator_count += 1;
                    total_no_calibrator += 1;
                    IntervalEvaluationDisposition::NoCalibratorForStratum
                }
                Some(threshold)
                    if threshold.disposition != ConformalFitDisposition::Fitted
                        || threshold.score_quantile.is_none() =>
                {
                    group.insufficient_calibration_count += 1;
                    total_insufficient += 1;
                    IntervalEvaluationDisposition::InsufficientStratumCalibration
                }
                Some(threshold) => {
                    let quantile = threshold.score_quantile.expect("fitted threshold has quantile");
                    let radius = quantile * observation.predicted_scale;
                    let normalized_error = observation.absolute_error / observation.predicted_scale;
                    if !radius.is_finite() || !normalized_error.is_finite() {
                        group.invalid_prediction_scale_count += 1;
                        total_invalid_scale += 1;
                        IntervalEvaluationDisposition::InvalidPredictionScale
                    } else {
                        group.evaluated_count += 1;
                        total_scored += 1;
                        group.interval_width_sum += 2.0 * radius;
                        if normalized_error <= quantile {
                            group.covered_count += 1;
                            total_covered += 1;
                            IntervalEvaluationDisposition::Covered
                        } else {
                            total_missed += 1;
                            IntervalEvaluationDisposition::Missed
                        }
                    }
                }
            }
        };

        let interval_width = if matches!(
            disposition,
            IntervalEvaluationDisposition::Covered | IntervalEvaluationDisposition::Missed
        ) {
            model
                .groups
                .iter()
                .find(|candidate| {
                    candidate.stratum_id == observation.stratum_id
                        && candidate.context_digest == observation.context_digest
                        && candidate.disposition == ConformalFitDisposition::Fitted
                })
                .and_then(|threshold| threshold.score_quantile)
                .map(|quantile| 2.0 * quantile * observation.predicted_scale)
                .filter(|width| width.is_finite())
        } else {
            None
        };
        let normalized_absolute_error = interval_width
            .map(|_| observation.absolute_error / observation.predicted_scale);

        sample_results.push(SampleIntervalResult {
            sample_id: observation.sample_id.clone(),
            stratum_id: observation.stratum_id.clone(),
            disposition,
            interval_width,
            normalized_absolute_error,
        });
    }

    let strata = mutable
        .into_iter()
        .map(|((stratum_id, context_digest), report)| {
            let (wilson_95_lower, wilson_95_upper) =
                wilson_interval(report.covered_count, report.evaluated_count);
            StratumCoverageReport {
                stratum_id,
                context_digest,
                nominal_coverage: model.nominal_coverage,
                evaluated_count: report.evaluated_count,
                covered_count: report.covered_count,
                empirical_coverage: (report.evaluated_count > 0).then(|| {
                    report.covered_count as f64 / report.evaluated_count as f64
                }),
                wilson_95_lower,
                wilson_95_upper,
                mean_interval_width: (report.evaluated_count > 0).then(|| {
                    report.interval_width_sum / report.evaluated_count as f64
                }),
                abstained_out_of_domain_count: report.abstained_out_of_domain_count,
                abstained_reference_unresolved_count: report.abstained_reference_unresolved_count,
                no_calibrator_count: report.no_calibrator_count,
                insufficient_calibration_count: report.insufficient_calibration_count,
                invalid_prediction_scale_count: report.invalid_prediction_scale_count,
            }
        })
        .collect();

    let (overall_wilson_95_lower, overall_wilson_95_upper) =
        wilson_interval(total_covered, total_scored);
    Ok(SplitConformalEvaluationReport {
        nominal_coverage: model.nominal_coverage,
        sample_count: observations.len(),
        scored_count: total_scored,
        covered_count: total_covered,
        missed_count: total_missed,
        overall_empirical_coverage: (total_scored > 0).then(|| {
            total_covered as f64 / total_scored as f64
        }),
        overall_wilson_95_lower,
        overall_wilson_95_upper,
        abstained_out_of_domain_count: total_out_of_domain,
        abstained_reference_unresolved_count: total_reference_unresolved,
        no_calibrator_count: total_no_calibrator,
        insufficient_calibration_count: total_insufficient,
        invalid_prediction_scale_count: total_invalid_scale,
        strata,
        sample_results,
        claim_class: "synthetic_split_conformal_coverage_evaluation_only",
    })
}

/// Deterministic two-stratum example. Calibration and evaluation samples are
/// distinct; OOD and common-cause evaluation cases are reported as abstentions.
pub fn run_synthetic_conformal_example() -> Result<SyntheticConformalExample, String> {
    let mut calibration = Vec::new();
    for (stratum, context, scale) in [
        ("magnetic", "sha256:magnetic-host-a", 0.1),
        ("piezoresistive", "sha256:piezoresistive-host-b", 0.2),
    ] {
        for index in 0..40 {
            let normalized_error = 0.2 + index as f64 * 0.035;
            calibration.push(CalibrationObservation {
                sample_id: format!("cal-{stratum}-{index:03}"),
                stratum_id: stratum.to_owned(),
                context_digest: context.to_owned(),
                absolute_error: normalized_error * scale,
                predicted_scale: scale,
                in_qualified_domain: true,
                reference_evidence: ReferenceEvidenceDisposition::IndependentlyQualified,
            });
        }
    }
    let model = fit_groupwise_split_conformal(&calibration, 0.9, 30)?;
    let mut evaluation = Vec::new();
    for (stratum, context, scale) in [
        ("magnetic", "sha256:magnetic-host-a", 0.1),
        ("piezoresistive", "sha256:piezoresistive-host-b", 0.2),
    ] {
        for index in 0..20 {
            let normalized_error = 0.15 + index as f64 * 0.06;
            evaluation.push(EvaluationObservation {
                sample_id: format!("eval-{stratum}-{index:03}"),
                stratum_id: stratum.to_owned(),
                context_digest: context.to_owned(),
                absolute_error: normalized_error * scale,
                predicted_scale: scale,
                in_qualified_domain: true,
                reference_evidence: ReferenceEvidenceDisposition::IndependentlyQualified,
            });
        }
    }
    evaluation.push(EvaluationObservation {
        sample_id: "eval-magnetic-ood".to_owned(),
        stratum_id: "magnetic".to_owned(),
        context_digest: "sha256:magnetic-host-a".to_owned(),
        absolute_error: 0.1,
        predicted_scale: 0.1,
        in_qualified_domain: false,
        reference_evidence: ReferenceEvidenceDisposition::IndependentlyQualified,
    });
    evaluation.push(EvaluationObservation {
        sample_id: "eval-piezo-common-cause".to_owned(),
        stratum_id: "piezoresistive".to_owned(),
        context_digest: "sha256:piezoresistive-host-b".to_owned(),
        absolute_error: 0.1,
        predicted_scale: 0.2,
        in_qualified_domain: true,
        reference_evidence: ReferenceEvidenceDisposition::SharedCommonCause,
    });
    let report = evaluate_groupwise_split_conformal(&model, &evaluation)?;
    Ok(SyntheticConformalExample {
        model,
        evaluation: report,
        claim_class: "synthetic_groupwise_conformal_example_only",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calibration_sample(
        sample_id: &str,
        stratum: &str,
        context: &str,
        normalized_error: f64,
    ) -> CalibrationObservation {
        CalibrationObservation {
            sample_id: sample_id.to_owned(),
            stratum_id: stratum.to_owned(),
            context_digest: context.to_owned(),
            absolute_error: normalized_error * 0.1,
            predicted_scale: 0.1,
            in_qualified_domain: true,
            reference_evidence: ReferenceEvidenceDisposition::IndependentlyQualified,
        }
    }

    fn evaluation_sample(
        sample_id: &str,
        stratum: &str,
        context: &str,
        normalized_error: f64,
    ) -> EvaluationObservation {
        EvaluationObservation {
            sample_id: sample_id.to_owned(),
            stratum_id: stratum.to_owned(),
            context_digest: context.to_owned(),
            absolute_error: normalized_error * 0.1,
            predicted_scale: 0.1,
            in_qualified_domain: true,
            reference_evidence: ReferenceEvidenceDisposition::IndependentlyQualified,
        }
    }

    #[test]
    fn groupwise_finite_sample_quantiles_are_fit_separately() {
        let mut calibration = Vec::new();
        for index in 0..40 {
            calibration.push(calibration_sample(
                &format!("cal-a-{index}"),
                "sensor-a",
                "sha256:context-a",
                0.2 + index as f64 * 0.02,
            ));
            calibration.push(calibration_sample(
                &format!("cal-b-{index}"),
                "sensor-b",
                "sha256:context-b",
                1.0 + index as f64 * 0.05,
            ));
        }
        let model = fit_groupwise_split_conformal(&calibration, 0.9, 30).expect("conformal fit");
        assert_eq!(model.groups.len(), 2);
        assert!(model.groups.iter().all(|group| {
            group.disposition == ConformalFitDisposition::Fitted
                && group.score_quantile.is_some()
        }));
        let a = model.groups.iter().find(|group| group.stratum_id == "sensor-a").unwrap();
        let b = model.groups.iter().find(|group| group.stratum_id == "sensor-b").unwrap();
        assert!(b.score_quantile.unwrap() > a.score_quantile.unwrap());
    }

    #[test]
    fn under_sampled_stratum_does_not_borrow_the_global_threshold() {
        let calibration = (0..40)
            .map(|index| calibration_sample(
                &format!("cal-a-{index}"),
                "sensor-a",
                "sha256:context-a",
                0.5,
            ))
            .chain((0..5).map(|index| calibration_sample(
                &format!("cal-b-{index}"),
                "sensor-b",
                "sha256:context-b",
                0.2,
            )))
            .collect::<Vec<_>>();
        let model = fit_groupwise_split_conformal(&calibration, 0.9, 30).expect("fit");
        let b = model.groups.iter().find(|group| group.stratum_id == "sensor-b").unwrap();
        assert_eq!(
            b.disposition,
            ConformalFitDisposition::InsufficientCalibrationSamples
        );
        assert_eq!(b.score_quantile, None);

        let report = evaluate_groupwise_split_conformal(
            &model,
            &[evaluation_sample("eval-b-1", "sensor-b", "sha256:context-b", 0.1)],
        )
        .expect("evaluation");
        assert_eq!(report.insufficient_calibration_count, 1);
        assert_eq!(report.scored_count, 0);
    }

    #[test]
    fn evaluation_rejects_reused_calibration_ids_and_duplicate_holdout_ids() {
        let calibration = (0..30)
            .map(|index| calibration_sample(
                &format!("cal-{index}"),
                "sensor-a",
                "sha256:context-a",
                0.5,
            ))
            .collect::<Vec<_>>();
        let model = fit_groupwise_split_conformal(&calibration, 0.9, 20).expect("fit");
        assert!(evaluate_groupwise_split_conformal(
            &model,
            &[evaluation_sample("cal-0", "sensor-a", "sha256:context-a", 0.2)],
        )
        .is_err());

        let duplicate = evaluation_sample("eval-dup", "sensor-a", "sha256:context-a", 0.2);
        assert!(evaluate_groupwise_split_conformal(&model, &[duplicate.clone(), duplicate]).is_err());
    }

    #[test]
    fn ood_and_shared_reference_samples_are_abstentions_not_coverage() {
        let calibration = (0..30)
            .map(|index| calibration_sample(
                &format!("cal-{index}"),
                "sensor-a",
                "sha256:context-a",
                0.5,
            ))
            .collect::<Vec<_>>();
        let model = fit_groupwise_split_conformal(&calibration, 0.9, 20).expect("fit");
        let mut ood = evaluation_sample("eval-ood", "sensor-a", "sha256:context-a", 0.1);
        ood.in_qualified_domain = false;
        let mut shared = evaluation_sample("eval-shared", "sensor-a", "sha256:context-a", 0.1);
        shared.reference_evidence = ReferenceEvidenceDisposition::SharedCommonCause;
        let report = evaluate_groupwise_split_conformal(&model, &[ood, shared]).expect("evaluate");
        assert_eq!(report.scored_count, 0);
        assert_eq!(report.abstained_out_of_domain_count, 1);
        assert_eq!(report.abstained_reference_unresolved_count, 1);
    }

    #[test]
    fn shifted_holdout_is_reported_as_undercoverage_not_hidden_by_calibration_fit() {
        let calibration = (0..40)
            .map(|index| calibration_sample(
                &format!("cal-{index}"),
                "sensor-a",
                "sha256:context-a",
                0.5,
            ))
            .collect::<Vec<_>>();
        let model = fit_groupwise_split_conformal(&calibration, 0.9, 30).expect("fit");
        let shifted = (0..20)
            .map(|index| evaluation_sample(
                &format!("shifted-{index}"),
                "sensor-a",
                "sha256:context-a",
                3.0 + index as f64 * 0.1,
            ))
            .collect::<Vec<_>>();
        let report = evaluate_groupwise_split_conformal(&model, &shifted).expect("evaluate");
        assert_eq!(report.scored_count, 20);
        assert_eq!(report.covered_count, 0);
        assert_eq!(report.overall_empirical_coverage, Some(0.0));
        assert!(report.overall_wilson_95_upper.unwrap() < 0.2);
    }

    #[test]
    fn invalid_nominal_coverage_identity_and_scale_fail_closed() {
        assert!(fit_groupwise_split_conformal(&[], 1.0, 10).is_err());
        assert!(fit_groupwise_split_conformal(&[], 0.9, 0).is_err());
        let mut invalid = calibration_sample("cal-1", "sensor-a", "sha256:ctx", 0.5);
        invalid.predicted_scale = 0.0;
        assert!(fit_groupwise_split_conformal(&[invalid], 0.9, 1).is_err());

        let duplicate = calibration_sample("cal-dup", "sensor-a", "sha256:ctx", 0.5);
        assert!(fit_groupwise_split_conformal(
            &[duplicate.clone(), duplicate],
            0.9,
            1
        )
        .is_err());
    }

    #[test]
    fn synthetic_example_keeps_domain_and_reference_abstentions_separate() {
        let example = run_synthetic_conformal_example().expect("synthetic conformal example");
        assert_eq!(example.model.groups.len(), 2);
        assert_eq!(example.evaluation.abstained_out_of_domain_count, 1);
        assert_eq!(example.evaluation.abstained_reference_unresolved_count, 1);
        assert_eq!(example.evaluation.scored_count, 40);
        assert_eq!(example.claim_class, "synthetic_groupwise_conformal_example_only");
    }
}
