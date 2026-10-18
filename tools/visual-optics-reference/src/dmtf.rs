#![deny(unsafe_code)]
// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Independent-method comparison for dynamic modulation transfer functions.
//!
//! This module compares paired outputs sampled on a preregistered common
//! frequency × view-angle × depth grid. It does not derive MTF from optical
//! captures, model a physical display, or qualify instruments/calibration.

use core::fmt;

/// Paired DMTF estimates for one point on the declared common grid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DmtfSample {
    /// Spatial frequency in cycles per degree of visual angle.
    pub spatial_frequency_cycles_per_degree: f64,
    /// Viewing angle in degrees; zero is the declared center view.
    pub view_angle_degrees: f64,
    /// Image-plane depth coordinate in millimetres for the protocol-defined origin.
    pub depth_mm: f64,
    /// DMTF from the predeclared method-A path, commonly static MTF × temporal response.
    pub method_a_mtf: f64,
    /// DMTF from the independently implemented method-B path, commonly high-speed capture.
    pub method_b_mtf: f64,
    /// Combined absolute uncertainty bound for the difference at this point, in MTF units.
    ///
    /// This value must be justified by the measurement protocol. It is not
    /// inferred by this function and is not automatically a confidence interval.
    pub combined_uncertainty_abs: f64,
}

/// Rules frozen before comparing independent DMTF methods.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DmtfComparisonConfig {
    /// Values below this method-A MTF are outside the declared comparison band.
    pub minimum_reference_mtf: f64,
    /// Maximum allowed normalized RMS disagreement, as a percentage.
    pub max_normalized_rms_percent: f64,
    /// Minimum number of valid in-band grid points required for a comparison.
    pub minimum_points: usize,
}

/// Results of comparing two methods on the common valid grid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DmtfComparisonReport {
    /// Number of points included in the normalized RMS calculation.
    pub points_evaluated: usize,
    /// Number of supplied points excluded because method-A MTF is below the declared floor.
    pub points_below_reference_floor: usize,
    /// Root-mean-square of (method A - method B) / method-A, as a percentage.
    pub normalized_rms_error_percent: f64,
    /// Largest absolute MTF difference over evaluated points.
    pub max_absolute_difference: f64,
    /// Largest pointwise relative disagreement over evaluated points, as a percentage.
    pub max_normalized_error_percent: f64,
    /// Number of evaluated points where the difference is within the supplied bound.
    pub points_within_uncertainty: usize,
    /// Number of evaluated points where the difference exceeds the supplied bound.
    pub points_outside_uncertainty: usize,
    /// True only when both the normalized-RMS threshold and every uncertainty check pass.
    pub passed: bool,
}

/// Invalid paired data, comparison contract or insufficient valid grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DmtfError {
    /// No samples were supplied.
    EmptySamples,
    /// Comparison thresholds or minimum sample count are invalid.
    InvalidConfig,
    /// A frequency, angle or depth coordinate is invalid.
    InvalidCoordinate,
    /// The common grid contains a repeated frequency/angle/depth coordinate.
    DuplicateGridPoint,
    /// An MTF value is non-finite or negative.
    InvalidMtf,
    /// The combined uncertainty bound is non-finite or negative.
    InvalidUncertainty,
    /// Too few points remain above the preregistered reference floor.
    InsufficientInBandPoints,
    /// Arithmetic produced a non-finite report value.
    NonFiniteResult,
}

impl fmt::Display for DmtfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySamples => f.write_str("DMTF comparison requires samples"),
            Self::InvalidConfig => {
                f.write_str("DMTF comparison thresholds and minimum points must be valid")
            }
            Self::InvalidCoordinate => {
                f.write_str("DMTF frequency, view angle or depth coordinate is invalid")
            }
            Self::DuplicateGridPoint => {
                f.write_str("DMTF common grid contains a duplicate coordinate")
            }
            Self::InvalidMtf => f.write_str("DMTF values must be finite and nonnegative"),
            Self::InvalidUncertainty => {
                f.write_str("combined DMTF uncertainty must be finite and nonnegative")
            }
            Self::InsufficientInBandPoints => {
                f.write_str("too few DMTF points remain above the reference floor")
            }
            Self::NonFiniteResult => {
                f.write_str("DMTF comparison produced a non-finite result")
            }
        }
    }
}

impl std::error::Error for DmtfError {}

fn validate_config(config: DmtfComparisonConfig) -> Result<(), DmtfError> {
    if !config.minimum_reference_mtf.is_finite()
        || config.minimum_reference_mtf <= 0.0
        || !config.max_normalized_rms_percent.is_finite()
        || config.max_normalized_rms_percent < 0.0
        || config.minimum_points < 2
    {
        return Err(DmtfError::InvalidConfig);
    }
    Ok(())
}

fn validate_sample(sample: DmtfSample) -> Result<(), DmtfError> {
    if !sample.spatial_frequency_cycles_per_degree.is_finite()
        || sample.spatial_frequency_cycles_per_degree < 0.0
        || !sample.view_angle_degrees.is_finite()
        || !(-90.0..=90.0).contains(&sample.view_angle_degrees)
        || !sample.depth_mm.is_finite()
        || sample.depth_mm < 0.0
    {
        return Err(DmtfError::InvalidCoordinate);
    }
    if !sample.method_a_mtf.is_finite()
        || sample.method_a_mtf < 0.0
        || !sample.method_b_mtf.is_finite()
        || sample.method_b_mtf < 0.0
    {
        return Err(DmtfError::InvalidMtf);
    }
    if !sample.combined_uncertainty_abs.is_finite() || sample.combined_uncertainty_abs < 0.0 {
        return Err(DmtfError::InvalidUncertainty);
    }
    Ok(())
}

fn canonical_bits(value: f64) -> u64 {
    if value == 0.0 { 0 } else { value.to_bits() }
}

/// Compares two DMTF methods over a frozen common grid.
///
/// All input samples must share the same units and predeclared comparison
/// method. Grid coordinates are validated for duplicates; all MTF values and
/// uncertainty bounds are validated, including points outside the comparison
/// band. Method-A values below the configured floor are counted but excluded
/// from normalized RMS scoring. If every in-band difference is not covered by
/// its supplied uncertainty bound, the report cannot pass even when RMS is low.
///
/// The normalized error is relative to method A because it is the predeclared
/// reference path in this comparison. Swapping methods changes the denominator
/// and therefore is not an equivalent test; the reference direction must be
/// fixed before examining results.
pub fn compare_dmtf_methods(
    samples: &[DmtfSample],
    config: DmtfComparisonConfig,
) -> Result<DmtfComparisonReport, DmtfError> {
    validate_config(config)?;
    if samples.is_empty() {
        return Err(DmtfError::EmptySamples);
    }

    let mut grid_keys = Vec::with_capacity(samples.len());
    for sample in samples {
        validate_sample(*sample)?;
        grid_keys.push((
            canonical_bits(sample.spatial_frequency_cycles_per_degree),
            canonical_bits(sample.view_angle_degrees),
            canonical_bits(sample.depth_mm),
        ));
    }
    grid_keys.sort_unstable();
    if grid_keys.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(DmtfError::DuplicateGridPoint);
    }

    let mut points_evaluated = 0usize;
    let mut points_below_reference_floor = 0usize;
    let mut squared_normalized_sum = 0.0;
    let mut max_absolute_difference: f64 = 0.0;
    let mut max_normalized_error_percent: f64 = 0.0;
    let mut points_within_uncertainty = 0usize;
    let mut points_outside_uncertainty = 0usize;

    for sample in samples {
        if sample.method_a_mtf < config.minimum_reference_mtf {
            points_below_reference_floor += 1;
            continue;
        }

        let difference = (sample.method_a_mtf - sample.method_b_mtf).abs();
        let normalized_error = difference / sample.method_a_mtf;
        let normalized_percent = normalized_error * 100.0;
        squared_normalized_sum += normalized_error * normalized_error;
        max_absolute_difference = max_absolute_difference.max(difference);
        max_normalized_error_percent = max_normalized_error_percent.max(normalized_percent);
        points_evaluated += 1;

        if difference <= sample.combined_uncertainty_abs {
            points_within_uncertainty += 1;
        } else {
            points_outside_uncertainty += 1;
        }
    }

    if points_evaluated < config.minimum_points {
        return Err(DmtfError::InsufficientInBandPoints);
    }

    let normalized_rms_error_percent =
        (squared_normalized_sum / points_evaluated as f64).sqrt() * 100.0;
    if !normalized_rms_error_percent.is_finite()
        || !max_absolute_difference.is_finite()
        || !max_normalized_error_percent.is_finite()
    {
        return Err(DmtfError::NonFiniteResult);
    }

    let rms_passed = normalized_rms_error_percent <= config.max_normalized_rms_percent;
    let uncertainty_passed = points_outside_uncertainty == 0;

    Ok(DmtfComparisonReport {
        points_evaluated,
        points_below_reference_floor,
        normalized_rms_error_percent,
        max_absolute_difference,
        max_normalized_error_percent,
        points_within_uncertainty,
        points_outside_uncertainty,
        passed: rms_passed && uncertainty_passed,
    })
}
