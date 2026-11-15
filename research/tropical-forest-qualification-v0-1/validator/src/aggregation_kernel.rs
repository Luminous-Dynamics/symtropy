// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Variable-aware aggregation over bounded rectangular observation supports.
//!
//! The functions consume already-qualified, same-CRS rectangles. They are not a
//! replacement for geospatial transforms, survey-design validation, or source adapters.
//! Every spatial aggregate reports observed support and never silently extrapolates.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::support_kernel::{
    CoverageDisposition, Rect, SupportError, SupportSample, qualify_rectangular_support,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AggregationError {
    Support(SupportError),
    EmptyUnit,
    DuplicateSampleIdentity,
    UnitMismatch,
    InvalidValue,
    InvalidInclusionProbability,
    InvalidSamplingFrame,
    NonProbabilityDesign,
    EmptyCategory,
}

impl fmt::Display for AggregationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Support(error) => write!(formatter, "support error: {error}"),
            other => write!(formatter, "{other:?}"),
        }
    }
}

impl std::error::Error for AggregationError {}

impl From<SupportError> for AggregationError {
    fn from(error: SupportError) -> Self {
        Self::Support(error)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct IntensiveSample {
    pub support: SupportSample,
    pub value: f64,
    pub unit: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IntensiveAggregate {
    /// Mean over the valid observed portion only. None means no valid observations.
    pub observed_mean: Option<f64>,
    pub unit: String,
    pub target_area_m2: f64,
    pub geometric_coverage_fraction: f64,
    pub valid_data_coverage_fraction: f64,
    pub valid_observed_area_m2: f64,
    pub disposition: CoverageDisposition,
}

/// Area-weighted mean for an intensive variable (for example, AGB density in Mg/ha).
///
/// The result is explicitly the mean over valid observed support, not an estimate for
/// unobserved parts of the target. Units must match exactly.
pub fn aggregate_intensive_mean(
    target_crs: &str,
    target: Rect,
    samples: &[IntensiveSample],
    expected_unit: &str,
) -> Result<IntensiveAggregate, AggregationError> {
    if expected_unit.trim().is_empty() {
        return Err(AggregationError::UnitMismatch);
    }

    let mut identities = BTreeSet::new();
    for sample in samples {
        if sample.support.sample_id.trim().is_empty() {
            return Err(AggregationError::EmptyUnit);
        }
        if !identities.insert(sample.support.sample_id.as_str()) {
            return Err(AggregationError::DuplicateSampleIdentity);
        }
        if sample.unit != expected_unit {
            return Err(AggregationError::UnitMismatch);
        }
        if !sample.value.is_finite() {
            return Err(AggregationError::InvalidValue);
        }
    }

    let support_samples: Vec<SupportSample> = samples
        .iter()
        .map(|sample| sample.support.clone())
        .collect();
    let support_report = qualify_rectangular_support(target_crs, target, &support_samples)?;

    let mut ordered: Vec<&IntensiveSample> = samples.iter().collect();
    ordered.sort_by(|left, right| left.support.sample_id.cmp(&right.support.sample_id));

    let mut weighted_sum = 0.0;
    let mut compensation = 0.0;
    let mut valid_observed_area_m2 = 0.0;
    for sample in ordered {
        if !sample.support.valid {
            continue;
        }
        let overlap = target.intersection_area(sample.support.bounds);
        if overlap <= 0.0 {
            continue;
        }
        let term = sample.value * overlap;
        let adjusted = term - compensation;
        let next = weighted_sum + adjusted;
        compensation = (next - weighted_sum) - adjusted;
        weighted_sum = next;
        valid_observed_area_m2 += overlap;
    }

    let observed_mean = if valid_observed_area_m2 > 0.0 {
        Some(weighted_sum / valid_observed_area_m2)
    } else {
        None
    };

    Ok(IntensiveAggregate {
        observed_mean,
        unit: expected_unit.to_string(),
        target_area_m2: support_report.target_area,
        geometric_coverage_fraction: support_report.geometric_coverage_fraction,
        valid_data_coverage_fraction: support_report.valid_data_coverage_fraction,
        valid_observed_area_m2,
        disposition: support_report.disposition,
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct DensitySample {
    pub support: SupportSample,
    pub value_mg_per_ha: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IntegratedStock {
    /// Stock inside valid observed overlap only; no value is imputed outside it.
    pub observed_stock_mg: f64,
    pub target_area_m2: f64,
    pub geometric_coverage_fraction: f64,
    pub valid_data_coverage_fraction: f64,
    pub valid_observed_area_m2: f64,
    pub disposition: CoverageDisposition,
}

/// Integrate an intensive biomass density into an extensive stock over valid observed area.
///
/// 1 hectare = 10,000 m². Partial support yields the stock only over the observed overlap;
/// this function does not scale it up to the full target polygon.
pub fn integrate_density_to_observed_stock(
    target_crs: &str,
    target: Rect,
    samples: &[DensitySample],
) -> Result<IntegratedStock, AggregationError> {
    let mut identities = BTreeSet::new();
    for sample in samples {
        if sample.support.sample_id.trim().is_empty() {
            return Err(AggregationError::EmptyUnit);
        }
        if !identities.insert(sample.support.sample_id.as_str()) {
            return Err(AggregationError::DuplicateSampleIdentity);
        }
        if !sample.value_mg_per_ha.is_finite() || sample.value_mg_per_ha < 0.0 {
            return Err(AggregationError::InvalidValue);
        }
    }

    let support_samples: Vec<SupportSample> = samples
        .iter()
        .map(|sample| sample.support.clone())
        .collect();
    let support_report = qualify_rectangular_support(target_crs, target, &support_samples)?;

    let mut ordered: Vec<&DensitySample> = samples.iter().collect();
    ordered.sort_by(|left, right| left.support.sample_id.cmp(&right.support.sample_id));

    let mut observed_stock_mg = 0.0;
    let mut compensation = 0.0;
    let mut valid_observed_area_m2 = 0.0;
    for sample in ordered {
        if !sample.support.valid {
            continue;
        }
        let overlap = target.intersection_area(sample.support.bounds);
        if overlap <= 0.0 {
            continue;
        }
        let stock = sample.value_mg_per_ha * overlap / 10_000.0;
        let adjusted = stock - compensation;
        let next = observed_stock_mg + adjusted;
        compensation = (next - observed_stock_mg) - adjusted;
        observed_stock_mg = next;
        valid_observed_area_m2 += overlap;
    }

    Ok(IntegratedStock {
        observed_stock_mg,
        target_area_m2: support_report.target_area,
        geometric_coverage_fraction: support_report.geometric_coverage_fraction,
        valid_data_coverage_fraction: support_report.valid_data_coverage_fraction,
        valid_observed_area_m2,
        disposition: support_report.disposition,
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct CountSample {
    pub sample_unit_id: String,
    pub sampling_frame_id: String,
    pub sampling_design_id: String,
    pub design_kind: String,
    pub observed_count: u64,
    /// First-order probability that this sample unit was selected by the design.
    pub inclusion_probability: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CountEstimate {
    pub horvitz_thompson_total: f64,
    pub sampling_frame_id: String,
    pub sampling_design_id: String,
    pub population_unit_count: usize,
    pub sampled_unit_count: usize,
    /// Variance is deliberately not fabricated without second-order inclusion probabilities.
    pub uncertainty_status: &'static str,
}

/// Horvitz–Thompson total for an explicitly declared probability sample.
///
/// The estimate requires positive first-order inclusion probabilities. It does not return
/// a standard error: that requires design-specific joint inclusion probabilities or another
/// independently justified variance estimator.
pub fn estimate_probability_sample_count_total(
    sampling_frame_id: &str,
    sampling_design_id: &str,
    population_unit_count: usize,
    samples: &[CountSample],
) -> Result<CountEstimate, AggregationError> {
    if sampling_frame_id.trim().is_empty()
        || sampling_design_id.trim().is_empty()
        || population_unit_count == 0
        || samples.is_empty()
        || samples.len() > population_unit_count
    {
        return Err(AggregationError::InvalidSamplingFrame);
    }

    let mut unit_ids = BTreeSet::new();
    let mut ordered: Vec<&CountSample> = samples.iter().collect();
    for sample in &ordered {
        if sample.sample_unit_id.trim().is_empty() {
            return Err(AggregationError::EmptyUnit);
        }
        if !unit_ids.insert(sample.sample_unit_id.as_str()) {
            return Err(AggregationError::DuplicateSampleIdentity);
        }
        if sample.sampling_frame_id != sampling_frame_id
            || sample.sampling_design_id != sampling_design_id
        {
            return Err(AggregationError::InvalidSamplingFrame);
        }
        if sample.design_kind != "probability_sample" {
            return Err(AggregationError::NonProbabilityDesign);
        }
        if !sample.inclusion_probability.is_finite()
            || sample.inclusion_probability <= 0.0
            || sample.inclusion_probability > 1.0
        {
            return Err(AggregationError::InvalidInclusionProbability);
        }
    }

    ordered.sort_by(|left, right| left.sample_unit_id.cmp(&right.sample_unit_id));
    let mut total = 0.0;
    let mut compensation = 0.0;
    for sample in ordered {
        let term = sample.observed_count as f64 / sample.inclusion_probability;
        let adjusted = term - compensation;
        let next = total + adjusted;
        compensation = (next - total) - adjusted;
        total = next;
    }

    Ok(CountEstimate {
        horvitz_thompson_total: total,
        sampling_frame_id: sampling_frame_id.to_string(),
        sampling_design_id: sampling_design_id.to_string(),
        population_unit_count,
        sampled_unit_count: samples.len(),
        uncertainty_status: "NOT_ESTIMATED_REQUIRES_SECOND_ORDER_INCLUSION_PROBABILITIES",
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct CategoricalSample {
    pub support: SupportSample,
    pub class_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClassArea {
    pub class_id: String,
    pub valid_area_m2: f64,
    pub fraction_of_valid_observed_area: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CategoricalAggregate {
    pub target_area_m2: f64,
    pub geometric_coverage_fraction: f64,
    pub valid_data_coverage_fraction: f64,
    pub valid_observed_area_m2: f64,
    pub classes: Vec<ClassArea>,
    pub disposition: CoverageDisposition,
}

/// Area composition for categorical observations. Class IDs remain labels; codes are never averaged.
pub fn aggregate_categorical_areas(
    target_crs: &str,
    target: Rect,
    samples: &[CategoricalSample],
) -> Result<CategoricalAggregate, AggregationError> {
    let mut identities = BTreeSet::new();
    for sample in samples {
        if sample.support.sample_id.trim().is_empty() {
            return Err(AggregationError::EmptyUnit);
        }
        if !identities.insert(sample.support.sample_id.as_str()) {
            return Err(AggregationError::DuplicateSampleIdentity);
        }
        if sample.class_id.trim().is_empty() {
            return Err(AggregationError::EmptyCategory);
        }
    }

    let support_samples: Vec<SupportSample> = samples
        .iter()
        .map(|sample| sample.support.clone())
        .collect();
    let support_report = qualify_rectangular_support(target_crs, target, &support_samples)?;

    let mut ordered: Vec<&CategoricalSample> = samples.iter().collect();
    ordered.sort_by(|left, right| left.support.sample_id.cmp(&right.support.sample_id));

    let mut areas: BTreeMap<String, f64> = BTreeMap::new();
    for sample in ordered {
        if !sample.support.valid {
            continue;
        }
        let overlap = target.intersection_area(sample.support.bounds);
        if overlap > 0.0 {
            *areas.entry(sample.class_id.clone()).or_default() += overlap;
        }
    }

    let valid_observed_area_m2: f64 = areas.values().sum();
    let classes = areas
        .into_iter()
        .map(|(class_id, valid_area_m2)| ClassArea {
            class_id,
            valid_area_m2,
            fraction_of_valid_observed_area: if valid_observed_area_m2 > 0.0 {
                valid_area_m2 / valid_observed_area_m2
            } else {
                0.0
            },
        })
        .collect();

    Ok(CategoricalAggregate {
        target_area_m2: support_report.target_area,
        geometric_coverage_fraction: support_report.geometric_coverage_fraction,
        valid_data_coverage_fraction: support_report.valid_data_coverage_fraction,
        valid_observed_area_m2,
        classes,
        disposition: support_report.disposition,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Rect {
        Rect::new(min_x, min_y, max_x, max_y).expect("test rectangle is valid")
    }

    fn support(id: &str, bounds: Rect, valid: bool) -> SupportSample {
        SupportSample {
            sample_id: id.to_string(),
            crs: "LOCAL_METRE_GRID_A".to_string(),
            bounds,
            valid,
        }
    }

    #[test]
    fn intensive_mean_is_area_weighted() {
        let target = rect(0.0, 0.0, 10.0, 10.0);
        let samples = [
            IntensiveSample {
                support: support("a", rect(0.0, 0.0, 5.0, 10.0), true),
                value: 10.0,
                unit: "Mg/ha".to_string(),
            },
            IntensiveSample {
                support: support("b", rect(5.0, 0.0, 10.0, 10.0), true),
                value: 20.0,
                unit: "Mg/ha".to_string(),
            },
        ];
        let result = aggregate_intensive_mean("LOCAL_METRE_GRID_A", target, &samples, "Mg/ha")
            .expect("weighted mean");
        assert_eq!(result.observed_mean, Some(15.0));
        assert_eq!(result.valid_data_coverage_fraction, 1.0);
    }

    #[test]
    fn intensive_mean_does_not_extrapolate_partial_coverage() {
        let result = aggregate_intensive_mean(
            "LOCAL_METRE_GRID_A",
            rect(0.0, 0.0, 10.0, 10.0),
            &[IntensiveSample {
                support: support("half", rect(0.0, 0.0, 5.0, 10.0), true),
                value: 18.0,
                unit: "Mg/ha".to_string(),
            }],
            "Mg/ha",
        )
        .expect("partial mean");
        assert_eq!(result.observed_mean, Some(18.0));
        assert_eq!(result.valid_data_coverage_fraction, 0.5);
        assert_eq!(result.valid_observed_area_m2, 50.0);
    }

    #[test]
    fn intensive_mean_skips_invalid_coverage_not_zero_fills() {
        let result = aggregate_intensive_mean(
            "LOCAL_METRE_GRID_A",
            rect(0.0, 0.0, 10.0, 10.0),
            &[IntensiveSample {
                support: support("cloud", rect(0.0, 0.0, 10.0, 10.0), false),
                value: 0.0,
                unit: "Mg/ha".to_string(),
            }],
            "Mg/ha",
        )
        .expect("masked sample");
        assert_eq!(result.observed_mean, None);
        assert_eq!(result.geometric_coverage_fraction, 1.0);
        assert_eq!(result.valid_data_coverage_fraction, 0.0);
    }

    #[test]
    fn intensive_mean_rejects_unit_mismatch() {
        let result = aggregate_intensive_mean(
            "LOCAL_METRE_GRID_A",
            rect(0.0, 0.0, 10.0, 10.0),
            &[IntensiveSample {
                support: support("a", rect(0.0, 0.0, 10.0, 10.0), true),
                value: 4.0,
                unit: "kg".to_string(),
            }],
            "Mg/ha",
        );
        assert_eq!(result, Err(AggregationError::UnitMismatch));
    }

    #[test]
    fn intensive_mean_rejects_non_finite_values() {
        let result = aggregate_intensive_mean(
            "LOCAL_METRE_GRID_A",
            rect(0.0, 0.0, 10.0, 10.0),
            &[IntensiveSample {
                support: support("a", rect(0.0, 0.0, 10.0, 10.0), true),
                value: f64::NAN,
                unit: "Mg/ha".to_string(),
            }],
            "Mg/ha",
        );
        assert_eq!(result, Err(AggregationError::InvalidValue));
    }

    #[test]
    fn intensive_results_are_order_independent() {
        let target = rect(0.0, 0.0, 10.0, 10.0);
        let a = IntensiveSample {
            support: support("a", rect(0.0, 0.0, 5.0, 10.0), true),
            value: 12.0,
            unit: "Mg/ha".to_string(),
        };
        let b = IntensiveSample {
            support: support("b", rect(5.0, 0.0, 10.0, 10.0), true),
            value: 18.0,
            unit: "Mg/ha".to_string(),
        };
        let left = aggregate_intensive_mean(
            "LOCAL_METRE_GRID_A",
            target,
            &[a.clone(), b.clone()],
            "Mg/ha",
        )
        .unwrap();
        let right =
            aggregate_intensive_mean("LOCAL_METRE_GRID_A", target, &[b, a], "Mg/ha").unwrap();
        assert_eq!(left, right);
    }

    #[test]
    fn density_integrates_to_extensive_stock_in_correct_units() {
        let result = integrate_density_to_observed_stock(
            "LOCAL_METRE_GRID_A",
            rect(0.0, 0.0, 10.0, 10.0),
            &[
                DensitySample {
                    support: support("a", rect(0.0, 0.0, 5.0, 10.0), true),
                    value_mg_per_ha: 20.0,
                },
                DensitySample {
                    support: support("b", rect(5.0, 0.0, 10.0, 10.0), true),
                    value_mg_per_ha: 10.0,
                },
            ],
        )
        .expect("integrated stock");
        assert!((result.observed_stock_mg - 0.15).abs() < 1e-12);
        assert_eq!(result.valid_data_coverage_fraction, 1.0);
    }

    #[test]
    fn density_stock_reports_partial_observed_total_without_target_extrapolation() {
        let result = integrate_density_to_observed_stock(
            "LOCAL_METRE_GRID_A",
            rect(0.0, 0.0, 10.0, 10.0),
            &[DensitySample {
                support: support("half", rect(0.0, 0.0, 5.0, 10.0), true),
                value_mg_per_ha: 20.0,
            }],
        )
        .expect("partial integrated stock");
        assert!((result.observed_stock_mg - 0.1).abs() < 1e-12);
        assert_eq!(result.valid_data_coverage_fraction, 0.5);
        assert_eq!(result.valid_observed_area_m2, 50.0);
    }

    #[test]
    fn density_stock_rejects_negative_density() {
        let result = integrate_density_to_observed_stock(
            "LOCAL_METRE_GRID_A",
            rect(0.0, 0.0, 10.0, 10.0),
            &[DensitySample {
                support: support("negative", rect(0.0, 0.0, 10.0, 10.0), true),
                value_mg_per_ha: -1.0,
            }],
        );
        assert_eq!(result, Err(AggregationError::InvalidValue));
    }

    #[test]
    fn horvitz_thompson_uses_first_order_inclusion_probabilities() {
        let samples = [
            CountSample {
                sample_unit_id: "a".to_string(),
                sampling_frame_id: "frame".to_string(),
                sampling_design_id: "design".to_string(),
                design_kind: "probability_sample".to_string(),
                observed_count: 4,
                inclusion_probability: 0.5,
            },
            CountSample {
                sample_unit_id: "b".to_string(),
                sampling_frame_id: "frame".to_string(),
                sampling_design_id: "design".to_string(),
                design_kind: "probability_sample".to_string(),
                observed_count: 5,
                inclusion_probability: 0.25,
            },
        ];
        let result = estimate_probability_sample_count_total("frame", "design", 10, &samples)
            .expect("probability sample total");
        assert_eq!(result.horvitz_thompson_total, 28.0);
        assert_eq!(
            result.uncertainty_status,
            "NOT_ESTIMATED_REQUIRES_SECOND_ORDER_INCLUSION_PROBABILITIES"
        );
    }

    #[test]
    fn horvitz_thompson_rejects_zero_inclusion_probability() {
        let result = estimate_probability_sample_count_total(
            "frame",
            "design",
            10,
            &[CountSample {
                sample_unit_id: "a".to_string(),
                sampling_frame_id: "frame".to_string(),
                sampling_design_id: "design".to_string(),
                design_kind: "probability_sample".to_string(),
                observed_count: 4,
                inclusion_probability: 0.0,
            }],
        );
        assert_eq!(result, Err(AggregationError::InvalidInclusionProbability));
    }

    #[test]
    fn horvitz_thompson_rejects_non_probability_design() {
        let result = estimate_probability_sample_count_total(
            "frame",
            "design",
            10,
            &[CountSample {
                sample_unit_id: "a".to_string(),
                sampling_frame_id: "frame".to_string(),
                sampling_design_id: "design".to_string(),
                design_kind: "convenience_sample".to_string(),
                observed_count: 4,
                inclusion_probability: 0.5,
            }],
        );
        assert_eq!(result, Err(AggregationError::NonProbabilityDesign));
    }

    #[test]
    fn horvitz_thompson_rejects_duplicate_and_mixed_frame_units() {
        let duplicated = CountSample {
            sample_unit_id: "a".to_string(),
            sampling_frame_id: "frame".to_string(),
            sampling_design_id: "design".to_string(),
            design_kind: "probability_sample".to_string(),
            observed_count: 4,
            inclusion_probability: 0.5,
        };
        let duplicate_result = estimate_probability_sample_count_total(
            "frame",
            "design",
            10,
            &[duplicated.clone(), duplicated.clone()],
        );
        assert_eq!(
            duplicate_result,
            Err(AggregationError::DuplicateSampleIdentity)
        );

        let wrong_frame = CountSample {
            sample_unit_id: "b".to_string(),
            sampling_frame_id: "other".to_string(),
            sampling_design_id: "design".to_string(),
            design_kind: "probability_sample".to_string(),
            observed_count: 5,
            inclusion_probability: 0.5,
        };
        let mixed_result = estimate_probability_sample_count_total(
            "frame",
            "design",
            10,
            &[duplicated, wrong_frame],
        );
        assert_eq!(mixed_result, Err(AggregationError::InvalidSamplingFrame));
    }

    #[test]
    fn categorical_aggregation_preserves_class_identity_and_area_fraction() {
        let result = aggregate_categorical_areas(
            "LOCAL_METRE_GRID_A",
            rect(0.0, 0.0, 10.0, 10.0),
            &[
                CategoricalSample {
                    support: support("a", rect(0.0, 0.0, 2.0, 10.0), true),
                    class_id: "forest".to_string(),
                },
                CategoricalSample {
                    support: support("b", rect(2.0, 0.0, 10.0, 10.0), true),
                    class_id: "degraded_forest".to_string(),
                },
            ],
        )
        .expect("categorical composition");
        assert_eq!(result.classes.len(), 2);
        assert_eq!(result.classes[0].class_id, "degraded_forest");
        assert_eq!(result.classes[0].fraction_of_valid_observed_area, 0.8);
        assert_eq!(result.classes[1].class_id, "forest");
        assert_eq!(result.classes[1].fraction_of_valid_observed_area, 0.2);
    }

    #[test]
    fn categorical_aggregation_keeps_missing_class_pixels_out_of_valid_area() {
        let result = aggregate_categorical_areas(
            "LOCAL_METRE_GRID_A",
            rect(0.0, 0.0, 10.0, 10.0),
            &[
                CategoricalSample {
                    support: support("valid", rect(0.0, 0.0, 5.0, 10.0), true),
                    class_id: "forest".to_string(),
                },
                CategoricalSample {
                    support: support("masked", rect(5.0, 0.0, 10.0, 10.0), false),
                    class_id: "unknown".to_string(),
                },
            ],
        )
        .expect("masked categorical composition");
        assert_eq!(result.geometric_coverage_fraction, 1.0);
        assert_eq!(result.valid_data_coverage_fraction, 0.5);
        assert_eq!(result.classes.len(), 1);
        assert_eq!(result.classes[0].class_id, "forest");
        assert_eq!(result.classes[0].fraction_of_valid_observed_area, 1.0);
    }

    #[test]
    fn categorical_aggregation_rejects_empty_label_and_overlapping_support() {
        let empty = aggregate_categorical_areas(
            "LOCAL_METRE_GRID_A",
            rect(0.0, 0.0, 10.0, 10.0),
            &[CategoricalSample {
                support: support("a", rect(0.0, 0.0, 10.0, 10.0), true),
                class_id: " ".to_string(),
            }],
        );
        assert_eq!(empty, Err(AggregationError::EmptyCategory));

        let overlap = aggregate_categorical_areas(
            "LOCAL_METRE_GRID_A",
            rect(0.0, 0.0, 10.0, 10.0),
            &[
                CategoricalSample {
                    support: support("a", rect(0.0, 0.0, 6.0, 10.0), true),
                    class_id: "forest".to_string(),
                },
                CategoricalSample {
                    support: support("b", rect(4.0, 0.0, 10.0, 10.0), true),
                    class_id: "water".to_string(),
                },
            ],
        );
        assert_eq!(
            overlap,
            Err(AggregationError::Support(
                SupportError::OverlappingSourceSupports
            ))
        );
    }

    #[test]
    fn categorical_aggregation_handles_no_valid_observations_without_class_guess() {
        let result = aggregate_categorical_areas(
            "LOCAL_METRE_GRID_A",
            rect(0.0, 0.0, 10.0, 10.0),
            &[CategoricalSample {
                support: support("masked", rect(0.0, 0.0, 10.0, 10.0), false),
                class_id: "unknown".to_string(),
            }],
        )
        .expect("all masked");
        assert!(result.classes.is_empty());
        assert_eq!(result.valid_data_coverage_fraction, 0.0);
        assert_eq!(
            result.disposition,
            CoverageDisposition::GeometricallyCompleteButDataIncomplete
        );
    }
}
