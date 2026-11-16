//! Non-uniform subterranean profile/morphometry reference.
//!
//! The input is a one-dimensional sequence of surveyed or explicitly
//! unobserved profile intervals. Each observed interval carries a
//! representative width and height. The output is a piecewise-constant,
//! length-weighted geometric summary.
//!
//! This is not a 3D reconstruction, connectivity solver, geomechanics model,
//! roof-stability prediction, clearance certification or habitat assessment.
//! Width and height values describe the surveyed profile only; an unobserved
//! interval must be represented explicitly with `cross_section: None`.

use crate::{combined_uncertainty, Measurement, MorphometryError};

/// Representative cross-section observation for one profile interval.
#[derive(Clone, Debug, PartialEq)]
pub struct CrossSectionObservation {
    /// Measured span across the profile, metres.
    pub width_m: Measurement,
    /// Measured vertical dimension, metres.
    pub height_m: Measurement,
    /// Optional measured roof thickness at the representative station, metres.
    /// Roof thickness is not a stability metric by itself.
    pub roof_thickness_m: Option<Measurement>,
}

/// One contiguous station interval along a single survey profile.
#[derive(Clone, Debug, PartialEq)]
pub struct ProfileSegment {
    pub segment_id: String,
    /// Along-profile start station, metres.
    pub start_m: f64,
    /// Along-profile end station, metres.
    pub end_m: f64,
    /// None means the interval is explicitly unobserved.
    pub cross_section: Option<CrossSectionObservation>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubsurfaceProfile {
    pub profile_id: String,
    pub total_length_m: f64,
    /// Segments must be ordered, non-overlapping, contiguous, and cover
    /// [0, total_length_m]. Gaps must be represented as unobserved segments.
    pub segments: Vec<ProfileSegment>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileCoverageStatus {
    FullyObserved,
    PartiallyObserved,
    Unquantified,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MorphometrySummary {
    pub total_length_m: f64,
    pub observed_length_m: f64,
    pub coverage_fraction: f64,
    pub coverage_status: ProfileCoverageStatus,
    /// Length-weighted nominal dimensions over observed intervals.
    pub mean_width_m: Option<f64>,
    pub mean_width_standard_uncertainty_m: Option<f64>,
    pub mean_height_m: Option<f64>,
    pub mean_height_standard_uncertainty_m: Option<f64>,
    /// Minimum and maximum values among observed representative sections only.
    pub min_observed_width_m: Option<f64>,
    pub max_observed_width_m: Option<f64>,
    pub min_observed_height_m: Option<f64>,
    pub max_observed_height_m: Option<f64>,
    /// Weighted population coefficient of variation over observed intervals.
    pub width_coefficient_of_variation: Option<f64>,
    pub height_coefficient_of_variation: Option<f64>,
    /// Fraction of the entire profile with an explicit roof-thickness measure.
    pub roof_thickness_coverage_fraction: f64,
}

const STATION_TOLERANCE_M: f64 = 1e-6;

fn finite(value: f64, context: &str) -> Result<f64, MorphometryError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(MorphometryError::NumericalOverflow(context.to_string()))
    }
}

fn weighted_cv(
    observations: &[(f64, f64)],
    total_weight: f64,
    mean: f64,
    context: &str,
) -> Result<Option<f64>, MorphometryError> {
    if observations.is_empty() || mean <= 0.0 || total_weight <= 0.0 {
        return Ok(None);
    }

    let mut weighted_squared_deviation = 0.0_f64;
    for (value, weight) in observations {
        let delta = value - mean;
        let term = delta * delta * weight;
        weighted_squared_deviation += term;
        finite(weighted_squared_deviation, context)?;
    }

    let variance = weighted_squared_deviation / total_weight;
    finite(variance, context)?;
    let cv = variance.sqrt() / mean;
    finite(cv, context).map(Some)
}

/// Summarize a single non-branching profile.
///
/// Length-weighted averages assume each representative section applies
/// piecewise-constantly across its interval. This is a declared discretization
/// approximation; it is not a full interior surface reconstruction.
pub fn summarize_profile(
    profile: &SubsurfaceProfile,
) -> Result<MorphometrySummary, MorphometryError> {
    let mut errors = Vec::<String>::new();

    if profile.profile_id.trim().is_empty() {
        errors.push("profile_id must not be empty".to_string());
    }
    if !profile.total_length_m.is_finite() || profile.total_length_m <= 0.0 {
        errors.push("total_length_m must be finite and > 0".to_string());
    }
    if profile.segments.is_empty() {
        errors.push("profile must contain at least one segment".to_string());
    }
    if !errors.is_empty() {
        return Err(MorphometryError::InvalidProfile(errors));
    }

    let mut seen_ids = std::collections::BTreeSet::new();
    let mut previous_end = 0.0_f64;
    let mut observed_length = 0.0_f64;
    let mut roof_length = 0.0_f64;
    let mut width_sum = 0.0_f64;
    let mut height_sum = 0.0_f64;
    let mut width_observations = Vec::<(f64, f64)>::new();
    let mut height_observations = Vec::<(f64, f64)>::new();
    let mut width_terms = Vec::<(f64, &Measurement)>::new();
    let mut height_terms = Vec::<(f64, &Measurement)>::new();
    let mut min_width = f64::INFINITY;
    let mut max_width = f64::NEG_INFINITY;
    let mut min_height = f64::INFINITY;
    let mut max_height = f64::NEG_INFINITY;

    for (index, segment) in profile.segments.iter().enumerate() {
        let label = format!("segments[{index}]");

        if segment.segment_id.trim().is_empty() {
            errors.push(format!("{label}.segment_id must not be empty"));
        }
        if !seen_ids.insert(segment.segment_id.as_str()) {
            errors.push(format!("{label} duplicates segment_id {}", segment.segment_id));
        }
        if !segment.start_m.is_finite()
            || !segment.end_m.is_finite()
            || segment.start_m < 0.0
            || segment.end_m <= segment.start_m
            || segment.end_m > profile.total_length_m + STATION_TOLERANCE_M
        {
            errors.push(format!("{label} has invalid station bounds"));
            continue;
        }
        if (segment.start_m - previous_end).abs() > STATION_TOLERANCE_M {
            errors.push(format!(
                "{label} is not contiguous with the previous interval; encode gaps explicitly as unobserved segments"
            ));
        }
        previous_end = segment.end_m;

        let interval_length = segment.end_m - segment.start_m;
        if !interval_length.is_finite() || interval_length <= 0.0 {
            errors.push(format!("{label} has invalid interval length"));
            continue;
        }

        let Some(section) = &segment.cross_section else {
            continue;
        };

        let mut measurement_errors = Vec::<String>::new();
        crate::validate_measurement(
            &section.width_m,
            &format!("{label}.width_m"),
            &mut measurement_errors,
        );
        crate::validate_measurement(
            &section.height_m,
            &format!("{label}.height_m"),
            &mut measurement_errors,
        );
        if let Some(roof) = &section.roof_thickness_m {
            crate::validate_measurement(
                roof,
                &format!("{label}.roof_thickness_m"),
                &mut measurement_errors,
            );
            if roof.value <= 0.0 {
                measurement_errors.push(format!("{label}.roof_thickness_m must be > 0"));
            }
        }
        if section.width_m.value <= 0.0 {
            measurement_errors.push(format!("{label}.width_m must be > 0"));
        }
        if section.height_m.value <= 0.0 {
            measurement_errors.push(format!("{label}.height_m must be > 0"));
        }
        if !measurement_errors.is_empty() {
            errors.extend(measurement_errors);
            continue;
        }

        observed_length += interval_length;
        width_sum += section.width_m.value * interval_length;
        height_sum += section.height_m.value * interval_length;
        finite(observed_length, "observed profile length")?;
        finite(width_sum, "weighted profile width sum")?;
        finite(height_sum, "weighted profile height sum")?;

        width_observations.push((section.width_m.value, interval_length));
        height_observations.push((section.height_m.value, interval_length));
        min_width = min_width.min(section.width_m.value);
        max_width = max_width.max(section.width_m.value);
        min_height = min_height.min(section.height_m.value);
        max_height = max_height.max(section.height_m.value);

        if let Some(roof) = &section.roof_thickness_m {
            roof_length += interval_length;
            finite(roof_length, "roof-thickness coverage length")?;
        }
    }

    if (previous_end - profile.total_length_m).abs() > STATION_TOLERANCE_M {
        errors.push("profile intervals do not cover the declared total_length_m".to_string());
    }
    if observed_length > profile.total_length_m + STATION_TOLERANCE_M {
        errors.push("observed interval length exceeds total profile length".to_string());
    }
    if !errors.is_empty() {
        return Err(MorphometryError::InvalidProfile(errors));
    }

    let coverage_fraction = (observed_length / profile.total_length_m).clamp(0.0, 1.0);
    let coverage_status = if observed_length <= 0.0 {
        ProfileCoverageStatus::Unquantified
    } else if coverage_fraction >= 1.0 - (STATION_TOLERANCE_M / profile.total_length_m) {
        ProfileCoverageStatus::FullyObserved
    } else {
        ProfileCoverageStatus::PartiallyObserved
    };

    let (mean_width, mean_width_sigma, mean_height, mean_height_sigma) =
        if observed_length > 0.0 {
            let width_mean = width_sum / observed_length;
            let height_mean = height_sum / observed_length;
            finite(width_mean, "mean profile width")?;
            finite(height_mean, "mean profile height")?;

            for segment in &profile.segments {
                if let Some(section) = &segment.cross_section {
                    let weight =
                        (segment.end_m - segment.start_m) / observed_length;
                    width_terms.push((weight, &section.width_m));
                    height_terms.push((weight, &section.height_m));
                }
            }

            let width_sigma = combined_uncertainty(
                &width_terms,
                "mean profile width uncertainty",
            )?;
            let height_sigma = combined_uncertainty(
                &height_terms,
                "mean profile height uncertainty",
            )?;

            (
                Some(width_mean),
                width_sigma,
                Some(height_mean),
                height_sigma,
            )
        } else {
            (None, None, None, None)
        };

    let width_cv = match mean_width {
        Some(mean) => weighted_cv(
            &width_observations,
            observed_length,
            mean,
            "width coefficient of variation",
        )?,
        None => None,
    };
    let height_cv = match mean_height {
        Some(mean) => weighted_cv(
            &height_observations,
            observed_length,
            mean,
            "height coefficient of variation",
        )?,
        None => None,
    };

    Ok(MorphometrySummary {
        total_length_m: profile.total_length_m,
        observed_length_m: observed_length,
        coverage_fraction,
        coverage_status,
        mean_width_m: mean_width,
        mean_width_standard_uncertainty_m: mean_width_sigma,
        mean_height_m: mean_height,
        mean_height_standard_uncertainty_m: mean_height_sigma,
        min_observed_width_m: if observed_length > 0.0 { Some(min_width) } else { None },
        max_observed_width_m: if observed_length > 0.0 { Some(max_width) } else { None },
        min_observed_height_m: if observed_length > 0.0 { Some(min_height) } else { None },
        max_observed_height_m: if observed_length > 0.0 { Some(max_height) } else { None },
        width_coefficient_of_variation: width_cv,
        height_coefficient_of_variation: height_cv,
        roof_thickness_coverage_fraction: (roof_length / profile.total_length_m).clamp(0.0, 1.0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measurement(value: f64, sigma: f64) -> Measurement {
        Measurement::measured(value, sigma, None)
    }

    fn segment(
        id: &str,
        start_m: f64,
        end_m: f64,
        width_m: Option<f64>,
        height_m: Option<f64>,
        roof_m: Option<f64>,
    ) -> ProfileSegment {
        let cross_section = match (width_m, height_m) {
            (Some(width), Some(height)) => Some(CrossSectionObservation {
                width_m: measurement(width, 0.01),
                height_m: measurement(height, 0.02),
                roof_thickness_m: roof_m.map(|value| measurement(value, 0.05)),
            }),
            (None, None) => None,
            _ => panic!("width and height must either both be present or both be absent"),
        };

        ProfileSegment {
            segment_id: id.to_string(),
            start_m,
            end_m,
            cross_section,
        }
    }

    #[test]
    fn complete_nonuniform_profile_uses_length_weighted_dimensions() {
        let profile = SubsurfaceProfile {
            profile_id: "synthetic-profile".to_string(),
            total_length_m: 30.0,
            segments: vec![
                segment("a", 0.0, 10.0, Some(8.0), Some(3.0), Some(2.0)),
                segment("b", 10.0, 20.0, Some(12.0), Some(5.0), Some(3.0)),
                segment("c", 20.0, 30.0, Some(10.0), Some(4.0), None),
            ],
        };

        let result = summarize_profile(&profile).unwrap();
        assert_eq!(result.coverage_status, ProfileCoverageStatus::FullyObserved);
        assert!((result.coverage_fraction - 1.0).abs() < 1e-12);
        assert!((result.mean_width_m.unwrap() - 10.0).abs() < 1e-12);
        assert!((result.mean_height_m.unwrap() - 4.0).abs() < 1e-12);
        assert_eq!(result.min_observed_width_m, Some(8.0));
        assert_eq!(result.max_observed_width_m, Some(12.0));
        assert!((result.roof_thickness_coverage_fraction - 2.0 / 3.0).abs() < 1e-12);
        assert!(result.mean_width_standard_uncertainty_m.is_some());
    }

    #[test]
    fn unobserved_interval_reduces_coverage_without_becoming_zero_width() {
        let profile = SubsurfaceProfile {
            profile_id: "partial-profile".to_string(),
            total_length_m: 30.0,
            segments: vec![
                segment("a", 0.0, 10.0, Some(8.0), Some(3.0), None),
                segment("unknown", 10.0, 20.0, None, None, None),
                segment("c", 20.0, 30.0, Some(10.0), Some(4.0), None),
            ],
        };

        let result = summarize_profile(&profile).unwrap();
        assert_eq!(result.coverage_status, ProfileCoverageStatus::PartiallyObserved);
        assert!((result.coverage_fraction - 2.0 / 3.0).abs() < 1e-12);
        assert!((result.mean_width_m.unwrap() - 9.0).abs() < 1e-12);
        assert_eq!(result.min_observed_width_m, Some(8.0));
    }

    #[test]
    fn wholly_unobserved_profile_is_unquantified() {
        let profile = SubsurfaceProfile {
            profile_id: "unknown-profile".to_string(),
            total_length_m: 10.0,
            segments: vec![segment("unknown", 0.0, 10.0, None, None, None)],
        };

        let result = summarize_profile(&profile).unwrap();
        assert_eq!(result.coverage_status, ProfileCoverageStatus::Unquantified);
        assert_eq!(result.coverage_fraction, 0.0);
        assert!(result.mean_width_m.is_none());
        assert!(result.min_observed_height_m.is_none());
    }

    #[test]
    fn missing_profile_interval_must_be_explicit() {
        let profile = SubsurfaceProfile {
            profile_id: "gap-profile".to_string(),
            total_length_m: 20.0,
            segments: vec![
                segment("a", 0.0, 8.0, Some(8.0), Some(3.0), None),
                segment("b", 12.0, 20.0, Some(9.0), Some(4.0), None),
            ],
        };

        assert!(summarize_profile(&profile).is_err());
    }

    #[test]
    fn duplicate_segment_ids_are_rejected() {
        let profile = SubsurfaceProfile {
            profile_id: "duplicate-profile".to_string(),
            total_length_m: 20.0,
            segments: vec![
                segment("same", 0.0, 10.0, Some(8.0), Some(3.0), None),
                segment("same", 10.0, 20.0, Some(9.0), Some(4.0), None),
            ],
        };

        assert!(summarize_profile(&profile).is_err());
    }

    #[test]
    fn zero_width_is_rejected() {
        let profile = SubsurfaceProfile {
            profile_id: "zero-width".to_string(),
            total_length_m: 10.0,
            segments: vec![segment("a", 0.0, 10.0, Some(0.0), Some(3.0), None)],
        };

        assert!(summarize_profile(&profile).is_err());
    }

    #[test]
    fn negative_geometry_uncertainty_is_rejected() {
        let mut section = segment("a", 0.0, 10.0, Some(8.0), Some(3.0), None);
        section.cross_section.as_mut().unwrap().width_m.standard_uncertainty = Some(-0.1);
        let profile = SubsurfaceProfile {
            profile_id: "negative-sigma".to_string(),
            total_length_m: 10.0,
            segments: vec![section],
        };

        assert!(summarize_profile(&profile).is_err());
    }

    #[test]
    fn nonfinite_geometry_is_rejected() {
        let profile = SubsurfaceProfile {
            profile_id: "nan-width".to_string(),
            total_length_m: 10.0,
            segments: vec![segment("a", 0.0, 10.0, Some(f64::NAN), Some(3.0), None)],
        };

        assert!(summarize_profile(&profile).is_err());
    }

    #[test]
    fn noncontiguous_or_reversed_segments_are_rejected() {
        let profile = SubsurfaceProfile {
            profile_id: "bad-order".to_string(),
            total_length_m: 20.0,
            segments: vec![
                segment("a", 0.0, 12.0, Some(8.0), Some(3.0), None),
                segment("b", 10.0, 20.0, Some(9.0), Some(4.0), None),
            ],
        };

        assert!(summarize_profile(&profile).is_err());
    }

    #[test]
    fn width_variation_is_reported_without_assuming_a_cylinder() {
        let profile = SubsurfaceProfile {
            profile_id: "variable-width".to_string(),
            total_length_m: 20.0,
            segments: vec![
                segment("a", 0.0, 10.0, Some(4.0), Some(3.0), None),
                segment("b", 10.0, 20.0, Some(12.0), Some(3.0), None),
            ],
        };

        let result = summarize_profile(&profile).unwrap();
        assert!((result.mean_width_m.unwrap() - 8.0).abs() < 1e-12);
        assert!(result.width_coefficient_of_variation.unwrap() > 0.0);
        assert_eq!(result.height_coefficient_of_variation, Some(0.0));
    }

    #[test]
    fn overflow_in_weighted_dimensions_is_rejected() {
        let profile = SubsurfaceProfile {
            profile_id: "overflow".to_string(),
            total_length_m: 1.0e200,
            segments: vec![segment(
                "a",
                0.0,
                1.0e200,
                Some(1.0e200),
                Some(1.0),
                None,
            )],
        };

        assert!(matches!(
            summarize_profile(&profile),
            Err(MorphometryError::NumericalOverflow(_))
        ));
    }
}
