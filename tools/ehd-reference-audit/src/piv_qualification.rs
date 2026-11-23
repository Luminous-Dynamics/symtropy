// SPDX-License-Identifier: AGPL-3.0-or-later

//! PIV vector-validity, uncertainty-coverage and PIV/LPT comparison receipts.
//!
//! This module evaluates aggregate evidence supplied by a PIV pipeline. It does
//! not process raw images, generate velocity vectors, or establish physical
//! truth. Classifier labels, interval coverage and cross-diagnostic agreement
//! are deliberately reported as separate qualification dimensions.

use std::error::Error;
use std::fmt;

const WILSON_Z_95: f64 = 1.959_963_984_540_054;
const REQUIRED_REGION_COUNT: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FlowRegion {
    Bulk,
    NearWall,
    ShearLayer,
    Wake,
}

impl FlowRegion {
    const ALL: [Self; REQUIRED_REGION_COUNT] =
        [Self::Bulk, Self::NearWall, Self::ShearLayer, Self::Wake];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PivConfusionCounts {
    /// Invalid vectors correctly rejected by the post-processor (TP).
    pub invalid_vectors_rejected: u64,
    /// Valid vectors incorrectly rejected by the post-processor (FP).
    pub valid_vectors_rejected: u64,
    /// Invalid vectors incorrectly retained (FN).
    pub invalid_vectors_retained: u64,
    /// Valid vectors correctly retained (TN).
    pub valid_vectors_retained: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PivClassifierMetrics {
    pub precision_invalid_class: f64,
    pub recall_invalid_class: f64,
    pub f1_invalid_class: f64,
    /// Fraction of all reference-valid vectors removed by the post-processor.
    pub false_valid_vector_removal_rate: f64,
    pub valid_vector_retention_rate: f64,
    pub accuracy: f64,
    pub total_vectors: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PivClassifierGate {
    pub minimum_f1: f64,
    pub minimum_recall: f64,
    pub maximum_false_valid_vector_removal_rate: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassifierDisposition {
    MeetsDeclaredGates,
    F1BelowTarget,
    RecallBelowTarget,
    FalseValidVectorRemovalAboveTarget,
    MultipleGatesFailed,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PivClassifierReport {
    pub metrics: PivClassifierMetrics,
    pub gate: PivClassifierGate,
    pub disposition: ClassifierDisposition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegionalCoverageCounts {
    pub region: FlowRegion,
    pub sample_count: u64,
    pub covered_count: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoverageGate {
    pub nominal_coverage: f64,
    pub minimum_acceptable_coverage: f64,
    pub maximum_acceptable_coverage: f64,
    pub minimum_samples_per_region: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoverageDisposition {
    WithinTargetBand,
    Undercoverage,
    Overcoverage,
    InsufficientSamples,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RegionalCoverageReport {
    pub region: FlowRegion,
    pub sample_count: u64,
    pub covered_count: u64,
    pub observed_coverage: Option<f64>,
    pub wilson_95_lower: Option<f64>,
    pub wilson_95_upper: Option<f64>,
    pub wilson_interval_contains_nominal: Option<bool>,
    pub disposition: CoverageDisposition,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PivMeasurementQualityReport {
    pub classifier: PivClassifierReport,
    pub regional_coverage: Vec<RegionalCoverageReport>,
    /// True only when the classifier gates and every spatial-region coverage
    /// gate pass independently. No pooled-only pass is possible.
    pub overall_eligible: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticMethod {
    Piv,
    LptShakeTheBox,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VelocityVectorEstimate {
    pub method: DiagnosticMethod,
    pub condition_id: String,
    pub coordinate_frame_id: String,
    /// Acquisition identity is retained to expose shared raw-image/seeding
    /// errors; different IDs do not prove statistical independence.
    pub acquisition_id: String,
    pub evidence_id: String,
    pub velocity_m_s: [f64; 3],
    pub absolute_uncertainty_bound_m_s: [f64; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcquisitionDependence {
    SharedAcquisition,
    DistinctAcquisitionIdsButCorrelationUnquantified,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VectorComparisonDisposition {
    WithinConservativeBounds,
    OutsideConservativeBounds,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PivLptVelocityComparison {
    pub condition_id: String,
    pub coordinate_frame_id: String,
    pub acquisition_dependence: AcquisitionDependence,
    pub residual_m_s: [f64; 3],
    pub combined_absolute_bound_m_s: [f64; 3],
    pub within_bound_by_component: [bool; 3],
    pub disposition: VectorComparisonDisposition,
    pub uncertainty_policy: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PivQualificationError {
    EmptyConfusionMatrix,
    ConfusionCountOverflow,
    ZeroInvalidPredictionDenominator,
    ZeroInvalidReferenceDenominator,
    ZeroValidReferenceDenominator,
    InvalidClassifierGate,
    InvalidCoverageGate,
    CoverageCountExceedsSampleCount {
        region: FlowRegion,
    },
    DuplicateCoverageRegion {
        region: FlowRegion,
    },
    MissingCoverageRegion {
        region: FlowRegion,
    },
    NonFiniteVelocity {
        method: DiagnosticMethod,
        component: usize,
    },
    InvalidVelocityUncertainty {
        method: DiagnosticMethod,
        component: usize,
    },
    EmptyIdentity {
        field: &'static str,
    },
    DuplicateEvidenceIdentity,
    ConditionMismatch,
    CoordinateFrameMismatch,
    MethodMismatch,
    NonFiniteDerivedValue,
}

impl fmt::Display for PivQualificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyConfusionMatrix => write!(f, "PIV classifier confusion matrix is empty"),
            Self::ConfusionCountOverflow => write!(f, "PIV classifier confusion count overflow"),
            Self::ZeroInvalidPredictionDenominator => {
                write!(f, "PIV classifier has no vectors predicted invalid")
            }
            Self::ZeroInvalidReferenceDenominator => {
                write!(f, "PIV classifier has no reference-invalid vectors")
            }
            Self::ZeroValidReferenceDenominator => {
                write!(f, "PIV classifier has no reference-valid vectors")
            }
            Self::InvalidClassifierGate => {
                write!(
                    f,
                    "PIV classifier gate values must be finite probabilities in [0,1]"
                )
            }
            Self::InvalidCoverageGate => {
                write!(f, "PIV regional coverage gate is invalid")
            }
            Self::CoverageCountExceedsSampleCount { region } => {
                write!(f, "PIV coverage count exceeds sample count in {region:?}")
            }
            Self::DuplicateCoverageRegion { region } => {
                write!(f, "PIV coverage region {region:?} is duplicated")
            }
            Self::MissingCoverageRegion { region } => {
                write!(f, "PIV coverage region {region:?} is missing")
            }
            Self::NonFiniteVelocity { method, component } => {
                write!(f, "{method:?} velocity component {component} is non-finite")
            }
            Self::InvalidVelocityUncertainty { method, component } => {
                write!(f, "{method:?} velocity uncertainty {component} is invalid")
            }
            Self::EmptyIdentity { field } => {
                write!(f, "PIV/LPT comparison {field} must be non-empty")
            }
            Self::DuplicateEvidenceIdentity => {
                write!(f, "PIV and LPT evidence identities must be distinct")
            }
            Self::ConditionMismatch => write!(f, "PIV and LPT condition identities differ"),
            Self::CoordinateFrameMismatch => {
                write!(f, "PIV and LPT coordinate-frame identities differ")
            }
            Self::MethodMismatch => {
                write!(
                    f,
                    "cross-diagnostic comparison requires one PIV and one LPT estimate"
                )
            }
            Self::NonFiniteDerivedValue => {
                write!(
                    f,
                    "PIV qualification calculation produced a non-finite value"
                )
            }
        }
    }
}

impl Error for PivQualificationError {}

fn checked_add_counts(counts: PivConfusionCounts) -> Result<u64, PivQualificationError> {
    counts
        .invalid_vectors_rejected
        .checked_add(counts.valid_vectors_rejected)
        .and_then(|value| value.checked_add(counts.invalid_vectors_retained))
        .and_then(|value| value.checked_add(counts.valid_vectors_retained))
        .ok_or(PivQualificationError::ConfusionCountOverflow)
}

/// Compute confusion metrics with invalid vectors as the positive class.
/// False valid-vector removal is reported separately from F1.
pub fn compute_piv_classifier_metrics(
    counts: PivConfusionCounts,
) -> Result<PivClassifierMetrics, PivQualificationError> {
    let total = checked_add_counts(counts)?;
    if total == 0 {
        return Err(PivQualificationError::EmptyConfusionMatrix);
    }
    let predicted_invalid = counts
        .invalid_vectors_rejected
        .checked_add(counts.valid_vectors_rejected)
        .ok_or(PivQualificationError::ConfusionCountOverflow)?;
    if predicted_invalid == 0 {
        return Err(PivQualificationError::ZeroInvalidPredictionDenominator);
    }
    let reference_invalid = counts
        .invalid_vectors_rejected
        .checked_add(counts.invalid_vectors_retained)
        .ok_or(PivQualificationError::ConfusionCountOverflow)?;
    if reference_invalid == 0 {
        return Err(PivQualificationError::ZeroInvalidReferenceDenominator);
    }
    let reference_valid = counts
        .valid_vectors_rejected
        .checked_add(counts.valid_vectors_retained)
        .ok_or(PivQualificationError::ConfusionCountOverflow)?;
    if reference_valid == 0 {
        return Err(PivQualificationError::ZeroValidReferenceDenominator);
    }

    let precision = counts.invalid_vectors_rejected as f64 / predicted_invalid as f64;
    let recall = counts.invalid_vectors_rejected as f64 / reference_invalid as f64;
    let f1 = if precision + recall > 0.0 {
        2.0 * precision * recall / (precision + recall)
    } else {
        0.0
    };
    let false_removal = counts.valid_vectors_rejected as f64 / reference_valid as f64;
    let valid_retention = counts.valid_vectors_retained as f64 / reference_valid as f64;
    let accuracy =
        (counts.invalid_vectors_rejected + counts.valid_vectors_retained) as f64 / total as f64;

    if ![
        precision,
        recall,
        f1,
        false_removal,
        valid_retention,
        accuracy,
    ]
    .iter()
    .all(|value| value.is_finite())
    {
        return Err(PivQualificationError::NonFiniteDerivedValue);
    }

    Ok(PivClassifierMetrics {
        precision_invalid_class: precision,
        recall_invalid_class: recall,
        f1_invalid_class: f1,
        false_valid_vector_removal_rate: false_removal,
        valid_vector_retention_rate: valid_retention,
        accuracy,
        total_vectors: total,
    })
}

pub fn evaluate_piv_classifier(
    counts: PivConfusionCounts,
    gate: PivClassifierGate,
) -> Result<PivClassifierReport, PivQualificationError> {
    for threshold in [
        gate.minimum_f1,
        gate.minimum_recall,
        gate.maximum_false_valid_vector_removal_rate,
    ] {
        if !threshold.is_finite() || !(0.0..=1.0).contains(&threshold) {
            return Err(PivQualificationError::InvalidClassifierGate);
        }
    }
    let metrics = compute_piv_classifier_metrics(counts)?;
    let f1_failed = metrics.f1_invalid_class < gate.minimum_f1;
    let recall_failed = metrics.recall_invalid_class < gate.minimum_recall;
    let false_removal_failed =
        metrics.false_valid_vector_removal_rate > gate.maximum_false_valid_vector_removal_rate;
    let failure_count = [f1_failed, recall_failed, false_removal_failed]
        .iter()
        .filter(|failed| **failed)
        .count();
    let disposition = match failure_count {
        0 => ClassifierDisposition::MeetsDeclaredGates,
        1 if f1_failed => ClassifierDisposition::F1BelowTarget,
        1 if recall_failed => ClassifierDisposition::RecallBelowTarget,
        1 => ClassifierDisposition::FalseValidVectorRemovalAboveTarget,
        _ => ClassifierDisposition::MultipleGatesFailed,
    };
    Ok(PivClassifierReport {
        metrics,
        gate,
        disposition,
    })
}

fn wilson_interval(successes: u64, samples: u64) -> Result<(f64, f64), PivQualificationError> {
    if samples == 0 || successes > samples {
        return Err(PivQualificationError::NonFiniteDerivedValue);
    }
    let n = samples as f64;
    let p = successes as f64 / n;
    let z2 = WILSON_Z_95 * WILSON_Z_95;
    let denominator = 1.0 + z2 / n;
    let center = (p + z2 / (2.0 * n)) / denominator;
    let half_width = WILSON_Z_95 * ((p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt()) / denominator;
    let lower = (center - half_width).max(0.0);
    let upper = (center + half_width).min(1.0);
    if !lower.is_finite() || !upper.is_finite() {
        return Err(PivQualificationError::NonFiniteDerivedValue);
    }
    Ok((lower, upper))
}

/// Evaluate nominal 95% interval coverage per spatial region. The output
/// includes Wilson binomial confidence limits; regions are never pooled to
/// hide a weak near-wall, shear-layer or wake cohort.
pub fn evaluate_regional_coverage(
    counts: &[RegionalCoverageCounts],
    gate: CoverageGate,
) -> Result<Vec<RegionalCoverageReport>, PivQualificationError> {
    if !gate.nominal_coverage.is_finite()
        || !gate.minimum_acceptable_coverage.is_finite()
        || !gate.maximum_acceptable_coverage.is_finite()
        || gate.minimum_samples_per_region == 0
        || gate.nominal_coverage <= 0.0
        || gate.nominal_coverage >= 1.0
        || gate.minimum_acceptable_coverage < 0.0
        || gate.maximum_acceptable_coverage > 1.0
        || gate.minimum_acceptable_coverage >= gate.maximum_acceptable_coverage
        || gate.nominal_coverage < gate.minimum_acceptable_coverage
        || gate.nominal_coverage > gate.maximum_acceptable_coverage
    {
        return Err(PivQualificationError::InvalidCoverageGate);
    }

    let mut seen = [false; REQUIRED_REGION_COUNT];
    for item in counts {
        let index = match item.region {
            FlowRegion::Bulk => 0,
            FlowRegion::NearWall => 1,
            FlowRegion::ShearLayer => 2,
            FlowRegion::Wake => 3,
        };
        if seen[index] {
            return Err(PivQualificationError::DuplicateCoverageRegion {
                region: item.region,
            });
        }
        seen[index] = true;
        if item.covered_count > item.sample_count {
            return Err(PivQualificationError::CoverageCountExceedsSampleCount {
                region: item.region,
            });
        }
    }
    for region in FlowRegion::ALL {
        let index = match region {
            FlowRegion::Bulk => 0,
            FlowRegion::NearWall => 1,
            FlowRegion::ShearLayer => 2,
            FlowRegion::Wake => 3,
        };
        if !seen[index] {
            return Err(PivQualificationError::MissingCoverageRegion { region });
        }
    }

    let mut reports = Vec::with_capacity(REQUIRED_REGION_COUNT);
    for item in counts {
        let (observed, lower, upper, disposition) =
            if item.sample_count < gate.minimum_samples_per_region {
                (None, None, None, CoverageDisposition::InsufficientSamples)
            } else {
                let observed = item.covered_count as f64 / item.sample_count as f64;
                let (lower, upper) = wilson_interval(item.covered_count, item.sample_count)?;
                let disposition = if observed < gate.minimum_acceptable_coverage {
                    CoverageDisposition::Undercoverage
                } else if observed > gate.maximum_acceptable_coverage {
                    CoverageDisposition::Overcoverage
                } else {
                    CoverageDisposition::WithinTargetBand
                };
                (Some(observed), Some(lower), Some(upper), disposition)
            };
        reports.push(RegionalCoverageReport {
            region: item.region,
            sample_count: item.sample_count,
            covered_count: item.covered_count,
            observed_coverage: observed,
            wilson_95_lower: lower,
            wilson_95_upper: upper,
            wilson_interval_contains_nominal: match (lower, upper) {
                (Some(lo), Some(hi)) => {
                    Some(lo <= gate.nominal_coverage && gate.nominal_coverage <= hi)
                }
                _ => None,
            },
            disposition,
        });
    }
    reports.sort_by_key(|item| item.region);
    Ok(reports)
}

/// Evaluate classifier and coverage separately. Every spatial region is
/// required; an aggregate-only pass is impossible.
pub fn qualify_piv_measurement_quality(
    confusion: PivConfusionCounts,
    classifier_gate: PivClassifierGate,
    regional_counts: &[RegionalCoverageCounts],
    coverage_gate: CoverageGate,
) -> Result<PivMeasurementQualityReport, PivQualificationError> {
    let classifier = evaluate_piv_classifier(confusion, classifier_gate)?;
    let regional_coverage = evaluate_regional_coverage(regional_counts, coverage_gate)?;
    let all_regions_within_target = regional_coverage
        .iter()
        .all(|item| item.disposition == CoverageDisposition::WithinTargetBand);
    Ok(PivMeasurementQualityReport {
        overall_eligible: classifier.disposition == ClassifierDisposition::MeetsDeclaredGates
            && all_regions_within_target,
        classifier,
        regional_coverage,
    })
}

fn validate_velocity_estimate(
    estimate: &VelocityVectorEstimate,
) -> Result<(), PivQualificationError> {
    for (index, component) in estimate.velocity_m_s.iter().enumerate() {
        if !component.is_finite() {
            return Err(PivQualificationError::NonFiniteVelocity {
                method: estimate.method,
                component: index,
            });
        }
    }
    for (index, bound) in estimate.absolute_uncertainty_bound_m_s.iter().enumerate() {
        if !bound.is_finite() || *bound < 0.0 {
            return Err(PivQualificationError::InvalidVelocityUncertainty {
                method: estimate.method,
                component: index,
            });
        }
    }
    for (field, value) in [
        ("condition_id", estimate.condition_id.as_str()),
        ("coordinate_frame_id", estimate.coordinate_frame_id.as_str()),
        ("acquisition_id", estimate.acquisition_id.as_str()),
        ("evidence_id", estimate.evidence_id.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(PivQualificationError::EmptyIdentity { field });
        }
    }
    Ok(())
}

/// Compare a PIV vector with a corresponding LPT/STB vector using conservative
/// componentwise absolute bounds. The receipt discloses shared acquisition
/// identity and never assumes independent errors or combines bounds by RSS.
pub fn compare_piv_lpt_velocity(
    first: &VelocityVectorEstimate,
    second: &VelocityVectorEstimate,
) -> Result<PivLptVelocityComparison, PivQualificationError> {
    validate_velocity_estimate(first)?;
    validate_velocity_estimate(second)?;
    if first.method == second.method {
        return Err(PivQualificationError::MethodMismatch);
    }
    if first.condition_id != second.condition_id {
        return Err(PivQualificationError::ConditionMismatch);
    }
    if first.coordinate_frame_id != second.coordinate_frame_id {
        return Err(PivQualificationError::CoordinateFrameMismatch);
    }
    if first.evidence_id == second.evidence_id {
        return Err(PivQualificationError::DuplicateEvidenceIdentity);
    }

    let (piv, lpt) = if first.method == DiagnosticMethod::Piv {
        (first, second)
    } else {
        (second, first)
    };
    let mut residual = [0.0_f64; 3];
    let mut combined_bound = [0.0_f64; 3];
    let mut within = [false; 3];
    for component in 0..3 {
        residual[component] = piv.velocity_m_s[component] - lpt.velocity_m_s[component];
        combined_bound[component] = piv.absolute_uncertainty_bound_m_s[component]
            + lpt.absolute_uncertainty_bound_m_s[component];
        if !residual[component].is_finite() || !combined_bound[component].is_finite() {
            return Err(PivQualificationError::NonFiniteDerivedValue);
        }
        within[component] = residual[component].abs() <= combined_bound[component];
    }
    let acquisition_dependence = if piv.acquisition_id == lpt.acquisition_id {
        AcquisitionDependence::SharedAcquisition
    } else {
        AcquisitionDependence::DistinctAcquisitionIdsButCorrelationUnquantified
    };
    let disposition = if within.iter().all(|value| *value) {
        VectorComparisonDisposition::WithinConservativeBounds
    } else {
        VectorComparisonDisposition::OutsideConservativeBounds
    };
    Ok(PivLptVelocityComparison {
        condition_id: piv.condition_id.clone(),
        coordinate_frame_id: piv.coordinate_frame_id.clone(),
        acquisition_dependence,
        residual_m_s: residual,
        combined_absolute_bound_m_s: combined_bound,
        within_bound_by_component: within,
        disposition,
        uncertainty_policy: "sum absolute component bounds; no independence, covariance, or RSS assumption",
    })
}

/// Run a manufactured classifier, regional-coverage and PIV/LPT comparison
/// smoke check for the offline auditor. These are synthetic summaries, not
/// experimental PIV measurements or a physical cross-validation result.
pub fn reference_self_check() -> Result<(), PivQualificationError> {
    let classifier = evaluate_piv_classifier(
        PivConfusionCounts {
            invalid_vectors_rejected: 9_900,
            valid_vectors_rejected: 50,
            invalid_vectors_retained: 100,
            valid_vectors_retained: 9_950,
        },
        PivClassifierGate {
            minimum_f1: 0.95,
            minimum_recall: 0.90,
            maximum_false_valid_vector_removal_rate: 0.01,
        },
    )?;
    if classifier.disposition != ClassifierDisposition::MeetsDeclaredGates
        || (classifier.metrics.false_valid_vector_removal_rate - 0.005).abs() > 1.0e-12
    {
        return Err(PivQualificationError::NonFiniteDerivedValue);
    }

    let regional = [
        RegionalCoverageCounts {
            region: FlowRegion::Bulk,
            sample_count: 1_000,
            covered_count: 950,
        },
        RegionalCoverageCounts {
            region: FlowRegion::NearWall,
            sample_count: 1_000,
            covered_count: 945,
        },
        RegionalCoverageCounts {
            region: FlowRegion::ShearLayer,
            sample_count: 1_000,
            covered_count: 952,
        },
        RegionalCoverageCounts {
            region: FlowRegion::Wake,
            sample_count: 1_000,
            covered_count: 948,
        },
    ];
    let coverage = evaluate_regional_coverage(
        &regional,
        CoverageGate {
            nominal_coverage: 0.95,
            minimum_acceptable_coverage: 0.90,
            maximum_acceptable_coverage: 0.98,
            minimum_samples_per_region: 200,
        },
    )?;
    if coverage.len() != REQUIRED_REGION_COUNT
        || coverage.iter().any(|item| {
            item.disposition != CoverageDisposition::WithinTargetBand
                || item.wilson_interval_contains_nominal != Some(true)
        })
    {
        return Err(PivQualificationError::NonFiniteDerivedValue);
    }

    let joint_quality = qualify_piv_measurement_quality(
        PivConfusionCounts {
            invalid_vectors_rejected: 9_900,
            valid_vectors_rejected: 50,
            invalid_vectors_retained: 100,
            valid_vectors_retained: 9_950,
        },
        PivClassifierGate {
            minimum_f1: 0.95,
            minimum_recall: 0.90,
            maximum_false_valid_vector_removal_rate: 0.01,
        },
        &regional,
        CoverageGate {
            nominal_coverage: 0.95,
            minimum_acceptable_coverage: 0.90,
            maximum_acceptable_coverage: 0.98,
            minimum_samples_per_region: 200,
        },
    )?;
    if !joint_quality.overall_eligible
        || joint_quality.regional_coverage.len() != REQUIRED_REGION_COUNT
    {
        return Err(PivQualificationError::NonFiniteDerivedValue);
    }

    let piv = VelocityVectorEstimate {
        method: DiagnosticMethod::Piv,
        condition_id: "manufactured-condition".to_owned(),
        coordinate_frame_id: "manufactured-frame".to_owned(),
        acquisition_id: "shared-images".to_owned(),
        evidence_id: "piv-evidence".to_owned(),
        velocity_m_s: [1.0, 2.0, 3.0],
        absolute_uncertainty_bound_m_s: [0.05; 3],
    };
    let lpt = VelocityVectorEstimate {
        method: DiagnosticMethod::LptShakeTheBox,
        condition_id: "manufactured-condition".to_owned(),
        coordinate_frame_id: "manufactured-frame".to_owned(),
        acquisition_id: "shared-images".to_owned(),
        evidence_id: "lpt-evidence".to_owned(),
        velocity_m_s: [1.08, 1.95, 3.02],
        absolute_uncertainty_bound_m_s: [0.05; 3],
    };
    let comparison = compare_piv_lpt_velocity(&piv, &lpt)?;
    if comparison.disposition != VectorComparisonDisposition::WithinConservativeBounds
        || comparison.acquisition_dependence != AcquisitionDependence::SharedAcquisition
    {
        return Err(PivQualificationError::NonFiniteDerivedValue);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classifier_counts() -> PivConfusionCounts {
        PivConfusionCounts {
            invalid_vectors_rejected: 9_900,
            valid_vectors_rejected: 50,
            invalid_vectors_retained: 100,
            valid_vectors_retained: 9_950,
        }
    }

    fn regional_counts() -> [RegionalCoverageCounts; 4] {
        [
            RegionalCoverageCounts {
                region: FlowRegion::Bulk,
                sample_count: 1_000,
                covered_count: 950,
            },
            RegionalCoverageCounts {
                region: FlowRegion::NearWall,
                sample_count: 1_000,
                covered_count: 945,
            },
            RegionalCoverageCounts {
                region: FlowRegion::ShearLayer,
                sample_count: 1_000,
                covered_count: 952,
            },
            RegionalCoverageCounts {
                region: FlowRegion::Wake,
                sample_count: 1_000,
                covered_count: 948,
            },
        ]
    }

    fn classifier_gate() -> PivClassifierGate {
        PivClassifierGate {
            minimum_f1: 0.95,
            minimum_recall: 0.90,
            maximum_false_valid_vector_removal_rate: 0.01,
        }
    }

    fn coverage_gate() -> CoverageGate {
        CoverageGate {
            nominal_coverage: 0.95,
            minimum_acceptable_coverage: 0.90,
            maximum_acceptable_coverage: 0.98,
            minimum_samples_per_region: 200,
        }
    }

    fn vector(
        method: DiagnosticMethod,
        acquisition_id: &str,
        evidence_id: &str,
        value: [f64; 3],
    ) -> VelocityVectorEstimate {
        VelocityVectorEstimate {
            method,
            condition_id: "ehd-condition-22kv".to_owned(),
            coordinate_frame_id: "lab-frame-v1".to_owned(),
            acquisition_id: acquisition_id.to_owned(),
            evidence_id: evidence_id.to_owned(),
            velocity_m_s: value,
            absolute_uncertainty_bound_m_s: [0.05; 3],
        }
    }

    #[test]
    fn classifier_metrics_report_f1_and_false_valid_removal_separately() {
        let metrics = compute_piv_classifier_metrics(classifier_counts()).unwrap();
        assert!(metrics.f1_invalid_class > 0.98);
        assert!((metrics.false_valid_vector_removal_rate - 0.005).abs() < 1.0e-12);
        assert!((metrics.valid_vector_retention_rate - 0.995).abs() < 1.0e-12);
    }

    #[test]
    fn classifier_gate_rejects_high_false_removal_even_when_f1_is_high() {
        let mut counts = classifier_counts();
        counts.valid_vectors_rejected = 500;
        counts.valid_vectors_retained = 9_500;
        let report = evaluate_piv_classifier(counts, classifier_gate()).unwrap();
        assert!(report.metrics.f1_invalid_class > 0.95);
        assert_eq!(
            report.disposition,
            ClassifierDisposition::FalseValidVectorRemovalAboveTarget
        );
    }

    #[test]
    fn regional_coverage_is_reported_separately_with_wilson_intervals() {
        let report = evaluate_regional_coverage(&regional_counts(), coverage_gate()).unwrap();
        assert_eq!(report.len(), 4);
        assert!(report.iter().all(|item| {
            item.disposition == CoverageDisposition::WithinTargetBand
                && item.wilson_95_lower.is_some()
                && item.wilson_95_upper.is_some()
                && item.wilson_interval_contains_nominal == Some(true)
        }));
    }

    #[test]
    fn undercovered_near_wall_region_is_not_hidden_by_bulk_coverage() {
        let mut counts = regional_counts();
        counts[1].covered_count = 700;
        let report = evaluate_regional_coverage(&counts, coverage_gate()).unwrap();
        assert_eq!(
            report
                .iter()
                .find(|item| item.region == FlowRegion::NearWall)
                .unwrap()
                .disposition,
            CoverageDisposition::Undercoverage
        );
        assert!(
            report
                .iter()
                .filter(|item| item.region != FlowRegion::NearWall)
                .all(|item| item.disposition == CoverageDisposition::WithinTargetBand)
        );
    }

    #[test]
    fn missing_required_spatial_region_is_rejected() {
        let counts = regional_counts();
        assert_eq!(
            evaluate_regional_coverage(&counts[..3], coverage_gate()).unwrap_err(),
            PivQualificationError::MissingCoverageRegion {
                region: FlowRegion::Wake
            }
        );
    }

    #[test]
    fn small_region_sample_count_is_not_promoted_to_calibration() {
        let mut counts = regional_counts();
        counts[1].sample_count = 50;
        counts[1].covered_count = 48;
        let report = evaluate_regional_coverage(&counts, coverage_gate()).unwrap();
        assert_eq!(
            report
                .iter()
                .find(|item| item.region == FlowRegion::NearWall)
                .unwrap()
                .disposition,
            CoverageDisposition::InsufficientSamples
        );
    }

    #[test]
    fn full_piv_quality_requires_classifier_and_every_region_gate() {
        let report = qualify_piv_measurement_quality(
            classifier_counts(),
            classifier_gate(),
            &regional_counts(),
            coverage_gate(),
        )
        .unwrap();
        assert!(report.overall_eligible);
    }

    #[test]
    fn piv_lpt_velocity_comparison_uses_conservative_bounds() {
        let piv = vector(DiagnosticMethod::Piv, "acq-piv", "e-piv", [1.0, 2.0, 3.0]);
        let lpt = vector(
            DiagnosticMethod::LptShakeTheBox,
            "acq-lpt",
            "e-lpt",
            [1.08, 1.95, 3.02],
        );
        let report = compare_piv_lpt_velocity(&piv, &lpt).unwrap();
        assert_eq!(
            report.disposition,
            VectorComparisonDisposition::WithinConservativeBounds
        );
        assert_eq!(
            report.acquisition_dependence,
            AcquisitionDependence::DistinctAcquisitionIdsButCorrelationUnquantified
        );
        assert_eq!(report.combined_absolute_bound_m_s, [0.1; 3]);
    }

    #[test]
    fn shared_acquisition_is_not_reported_as_independent_cross_validation() {
        let piv = vector(
            DiagnosticMethod::Piv,
            "shared-images",
            "e-piv",
            [1.0, 2.0, 3.0],
        );
        let lpt = vector(
            DiagnosticMethod::LptShakeTheBox,
            "shared-images",
            "e-lpt",
            [1.0, 2.0, 3.0],
        );
        let report = compare_piv_lpt_velocity(&piv, &lpt).unwrap();
        assert_eq!(
            report.acquisition_dependence,
            AcquisitionDependence::SharedAcquisition
        );
    }

    #[test]
    fn mismatched_coordinate_frames_are_rejected() {
        let piv = vector(DiagnosticMethod::Piv, "acq-piv", "e-piv", [1.0, 2.0, 3.0]);
        let mut lpt = vector(
            DiagnosticMethod::LptShakeTheBox,
            "acq-lpt",
            "e-lpt",
            [1.0, 2.0, 3.0],
        );
        lpt.coordinate_frame_id = "other-frame".to_owned();
        assert_eq!(
            compare_piv_lpt_velocity(&piv, &lpt).unwrap_err(),
            PivQualificationError::CoordinateFrameMismatch
        );
    }

    #[test]
    fn discrepancy_above_combined_velocity_bounds_is_flagged() {
        let piv = vector(DiagnosticMethod::Piv, "acq-piv", "e-piv", [1.0, 2.0, 3.0]);
        let lpt = vector(
            DiagnosticMethod::LptShakeTheBox,
            "acq-lpt",
            "e-lpt",
            [1.3, 2.0, 3.0],
        );
        let report = compare_piv_lpt_velocity(&piv, &lpt).unwrap();
        assert_eq!(
            report.disposition,
            VectorComparisonDisposition::OutsideConservativeBounds
        );
        assert_eq!(report.within_bound_by_component, [false, true, true]);
    }

    #[test]
    fn invalid_coverage_counts_are_rejected() {
        let mut counts = regional_counts();
        counts[0].covered_count = counts[0].sample_count + 1;
        assert!(matches!(
            evaluate_regional_coverage(&counts, coverage_gate()),
            Err(PivQualificationError::CoverageCountExceedsSampleCount {
                region: FlowRegion::Bulk
            })
        ));
    }
}
