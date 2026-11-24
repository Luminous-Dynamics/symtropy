// SPDX-License-Identifier: AGPL-3.0-or-later

//! Structure-preservation scorecard for manufactured 2D PIV vector fields.
//!
//! The kernel compares a reconstructed field with a declared reference field.
//! It keeps vector coverage, global error, vorticity fidelity and weak-structure
//! retention separate so that deleting difficult vectors cannot improve a
//! single aggregate score without exposing the loss of coverage or structure.
//!
//! This is a metric kernel, not a PIV image processor or a physical vortex
//! detector. Thresholds define benchmark estimands and must be frozen before
//! evaluating candidate methods.

#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct VectorField2D {
    pub width: usize,
    pub height: usize,
    pub spacing_x: f64,
    pub spacing_y: f64,
    pub u: Vec<f64>,
    pub v: Vec<f64>,
    /// A false entry means the vector is absent, not a zero-valued vector.
    pub valid: Vec<bool>,
}

impl VectorField2D {
    pub fn len(&self) -> usize {
        self.width.saturating_mul(self.height)
    }

    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructureConfig {
    /// Minimum absolute reference vorticity that defines a weak structure.
    pub weak_vorticity_min: f64,
    /// Upper weak-band boundary; stronger reference structures are not counted
    /// as weak structures or background false-support opportunities.
    pub weak_vorticity_max: f64,
    /// Minimum predicted/reference absolute-vorticity ratio for retention.
    pub minimum_retained_amplitude_fraction: f64,
    /// Minimum fraction of declared domain vectors that must be reconstructed.
    pub minimum_vector_coverage_fraction: f64,
    /// Minimum fraction of reference derivative sites with a valid prediction.
    pub minimum_derivative_coverage_fraction: f64,
    /// Minimum truth-defined weak-structure cells for a qualified weak score.
    pub minimum_weak_structure_cells: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InsufficientSupport {
    VectorCoverageBelowMinimum,
    DerivativeCoverageBelowMinimum,
    TooFewWeakStructureCells,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructureScorecard {
    pub domain_cells: usize,
    pub reconstructed_vector_cells: usize,
    pub vector_coverage_fraction: f64,
    /// Error is computed only on cells with both reference and reconstructed
    /// vectors. Missingness remains visible in vector_coverage_fraction.
    pub relative_l2_on_available_vectors: Option<f64>,
    pub normalized_rmse_on_available_vectors: Option<f64>,
    pub derivative_reference_cells: usize,
    pub derivative_reconstructed_cells: usize,
    pub derivative_coverage_fraction: f64,
    pub relative_vorticity_l2_on_available_cells: Option<f64>,
    pub weak_structure_cells: usize,
    /// Retained cells have a present prediction, correct vorticity sign and
    /// at least the configured fraction of reference magnitude.
    pub weak_structure_retained_cells: usize,
    pub predicted_weak_support_cells: usize,
    pub false_positive_weak_support_cells: usize,
    pub weak_structure_recall: Option<f64>,
    /// Precision counts incorrect predicted support both in weak-reference cells
    /// (wrong sign or inadequate amplitude) and in reference-defined background.
    pub weak_structure_precision: Option<f64>,
    pub weak_structure_dice: Option<f64>,
    pub weak_structure_sign_agreement: Option<f64>,
    /// Signed, reference-magnitude-weighted predicted/reference ratio.
    /// Negative values indicate net sign reversal; values below one indicate
    /// attenuation and values above one indicate amplification.
    pub weak_structure_signed_amplitude_ratio: Option<f64>,
    pub weak_structure_signed_amplitude_bias: Option<f64>,
    pub background_cells: usize,
    pub background_prediction_available_cells: usize,
    pub false_support_cells: usize,
    pub false_support_rate_in_background: Option<f64>,
    pub disposition: Result<(), InsufficientSupport>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldMetricError {
    GridTooSmall,
    ShapeMismatch {
        field: &'static str,
        expected: usize,
        actual: usize,
    },
    GridMetadataMismatch,
    InvalidSpacing {
        field: &'static str,
    },
    NonFiniteReference {
        index: usize,
    },
    NonFiniteReconstruction {
        index: usize,
    },
    ReferenceVectorMissing {
        index: usize,
    },
    EmptyEvaluationDomain,
    NoDerivativeSites,
    InvalidConfiguration {
        field: &'static str,
    },
}

impl fmt::Display for FieldMetricError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid structure-preservation input: {self:?}")
    }
}

impl Error for FieldMetricError {}

pub fn evaluate_structure_preservation(
    reference: &VectorField2D,
    reconstruction: &VectorField2D,
    evaluation_domain: &[bool],
    config: &StructureConfig,
) -> Result<StructureScorecard, FieldMetricError> {
    validate_field(reference, "reference")?;
    validate_field(reconstruction, "reconstruction")?;
    validate_config(config)?;

    if reference.width != reconstruction.width || reference.height != reconstruction.height {
        return Err(FieldMetricError::GridMetadataMismatch);
    }
    if !close_enough(reference.spacing_x, reconstruction.spacing_x)
        || !close_enough(reference.spacing_y, reconstruction.spacing_y)
    {
        return Err(FieldMetricError::GridMetadataMismatch);
    }

    let cell_count = reference.len();
    if evaluation_domain.len() != cell_count {
        return Err(FieldMetricError::ShapeMismatch {
            field: "evaluation_domain",
            expected: cell_count,
            actual: evaluation_domain.len(),
        });
    }
    let domain_cells = evaluation_domain
        .iter()
        .filter(|included| **included)
        .count();
    if domain_cells == 0 {
        return Err(FieldMetricError::EmptyEvaluationDomain);
    }

    for (index, included) in evaluation_domain.iter().enumerate() {
        if !included {
            continue;
        }
        if !reference.valid[index] {
            return Err(FieldMetricError::ReferenceVectorMissing { index });
        }
        if !reference.u[index].is_finite() || !reference.v[index].is_finite() {
            return Err(FieldMetricError::NonFiniteReference { index });
        }
        if reconstruction.valid[index]
            && (!reconstruction.u[index].is_finite() || !reconstruction.v[index].is_finite())
        {
            return Err(FieldMetricError::NonFiniteReconstruction { index });
        }
    }

    let mut reconstructed_vector_cells = 0usize;
    let mut vector_error_squared = 0.0;
    let mut reference_vector_squared = 0.0;
    for (index, included) in evaluation_domain.iter().enumerate() {
        if !included || !reconstruction.valid[index] {
            continue;
        }
        reconstructed_vector_cells += 1;
        let du = reconstruction.u[index] - reference.u[index];
        let dv = reconstruction.v[index] - reference.v[index];
        vector_error_squared += du * du + dv * dv;
        reference_vector_squared +=
            reference.u[index] * reference.u[index] + reference.v[index] * reference.v[index];
    }

    let vector_coverage_fraction = reconstructed_vector_cells as f64 / domain_cells as f64;
    let relative_l2_on_available_vectors =
        safe_ratio(vector_error_squared.sqrt(), reference_vector_squared.sqrt());
    let normalized_rmse_on_available_vectors = if reconstructed_vector_cells == 0 {
        None
    } else {
        let rmse = (vector_error_squared / (2.0 * reconstructed_vector_cells as f64)).sqrt();
        let reference_rms =
            (reference_vector_squared / (2.0 * reconstructed_vector_cells as f64)).sqrt();
        safe_ratio(rmse, reference_rms)
    };

    let mut derivative_reference_cells = 0usize;
    let mut derivative_reconstructed_cells = 0usize;
    let mut vorticity_error_squared = 0.0;
    let mut reference_vorticity_squared = 0.0;
    let mut weak_structure_cells = 0usize;
    let mut weak_structure_retained_cells = 0usize;
    let mut predicted_weak_support_cells = 0usize;
    let mut false_positive_weak_support_cells = 0usize;
    let mut weak_structure_sign_agreement_cells = 0usize;
    let mut weak_reference_magnitude_sum = 0.0;
    let mut weak_signed_prediction_sum = 0.0;
    let mut background_cells = 0usize;
    let mut background_prediction_available_cells = 0usize;
    let mut false_support_cells = 0usize;

    for y in 1..(reference.height - 1) {
        for x in 1..(reference.width - 1) {
            let center = y * reference.width + x;
            let west = center - 1;
            let east = center + 1;
            let south = center - reference.width;
            let north = center + reference.width;
            let stencil = [center, west, east, south, north];
            if !stencil.iter().all(|index| evaluation_domain[*index]) {
                continue;
            }

            derivative_reference_cells += 1;
            let omega_reference = vorticity(reference, west, east, south, north);
            if !omega_reference.is_finite() {
                return Err(FieldMetricError::NonFiniteReference { index: center });
            }

            let reconstruction_has_stencil =
                stencil.iter().all(|index| reconstruction.valid[*index]);
            let omega_reconstruction = if reconstruction_has_stencil {
                derivative_reconstructed_cells += 1;
                let value = vorticity(reconstruction, west, east, south, north);
                if !value.is_finite() {
                    return Err(FieldMetricError::NonFiniteReconstruction { index: center });
                }
                vorticity_error_squared += (value - omega_reference) * (value - omega_reference);
                reference_vorticity_squared += omega_reference * omega_reference;
                Some(value)
            } else {
                None
            };

            let reference_magnitude = omega_reference.abs();
            if reference_magnitude >= config.weak_vorticity_min
                && reference_magnitude <= config.weak_vorticity_max
            {
                weak_structure_cells += 1;
                weak_reference_magnitude_sum += reference_magnitude;
                if let Some(omega) = omega_reconstruction {
                    let signed_prediction = omega
                        * if omega_reference.is_sign_negative() {
                            -1.0
                        } else {
                            1.0
                        };
                    weak_signed_prediction_sum += signed_prediction;
                    let predicts_structure = omega.abs() >= config.weak_vorticity_min;
                    if predicts_structure {
                        predicted_weak_support_cells += 1;
                    }
                    if omega.signum() == omega_reference.signum() {
                        weak_structure_sign_agreement_cells += 1;
                        if omega.abs()
                            >= config.minimum_retained_amplitude_fraction * reference_magnitude
                        {
                            weak_structure_retained_cells += 1;
                        } else if predicts_structure {
                            false_positive_weak_support_cells += 1;
                        }
                    } else if predicts_structure {
                        false_positive_weak_support_cells += 1;
                    }
                }
            } else if reference_magnitude < config.weak_vorticity_min {
                background_cells += 1;
                if let Some(omega) = omega_reconstruction {
                    background_prediction_available_cells += 1;
                    if omega.abs() >= config.weak_vorticity_min {
                        false_support_cells += 1;
                        predicted_weak_support_cells += 1;
                        false_positive_weak_support_cells += 1;
                    }
                }
            }
        }
    }

    if derivative_reference_cells == 0 {
        return Err(FieldMetricError::NoDerivativeSites);
    }

    let derivative_coverage_fraction =
        derivative_reconstructed_cells as f64 / derivative_reference_cells as f64;
    let weak_structure_recall = ratio(weak_structure_retained_cells, weak_structure_cells);
    let weak_structure_sign_agreement =
        ratio(weak_structure_sign_agreement_cells, weak_structure_cells);
    let weak_structure_precision = safe_ratio(
        weak_structure_retained_cells as f64,
        predicted_weak_support_cells as f64,
    );
    let weak_structure_dice = safe_ratio(
        2.0 * weak_structure_retained_cells as f64,
        (2 * weak_structure_retained_cells
            + (weak_structure_cells - weak_structure_retained_cells)
            + false_positive_weak_support_cells) as f64,
    );
    let weak_structure_signed_amplitude_ratio =
        safe_ratio(weak_signed_prediction_sum, weak_reference_magnitude_sum);
    let weak_structure_signed_amplitude_bias =
        weak_structure_signed_amplitude_ratio.map(|ratio| ratio - 1.0);
    let false_support_rate_in_background =
        ratio(false_support_cells, background_prediction_available_cells);
    let relative_vorticity_l2_on_available_cells = safe_ratio(
        vorticity_error_squared.sqrt(),
        reference_vorticity_squared.sqrt(),
    );

    let disposition = if vector_coverage_fraction < config.minimum_vector_coverage_fraction {
        Err(InsufficientSupport::VectorCoverageBelowMinimum)
    } else if derivative_coverage_fraction < config.minimum_derivative_coverage_fraction {
        Err(InsufficientSupport::DerivativeCoverageBelowMinimum)
    } else if weak_structure_cells < config.minimum_weak_structure_cells {
        Err(InsufficientSupport::TooFewWeakStructureCells)
    } else {
        Ok(())
    };

    Ok(StructureScorecard {
        domain_cells,
        reconstructed_vector_cells,
        vector_coverage_fraction,
        relative_l2_on_available_vectors,
        normalized_rmse_on_available_vectors,
        derivative_reference_cells,
        derivative_reconstructed_cells,
        derivative_coverage_fraction,
        relative_vorticity_l2_on_available_cells,
        weak_structure_cells,
        weak_structure_retained_cells,
        predicted_weak_support_cells,
        false_positive_weak_support_cells,
        weak_structure_recall,
        weak_structure_precision,
        weak_structure_dice,
        weak_structure_sign_agreement,
        weak_structure_signed_amplitude_ratio,
        weak_structure_signed_amplitude_bias,
        background_cells,
        background_prediction_available_cells,
        false_support_cells,
        false_support_rate_in_background,
        disposition,
    })
}

fn validate_field(field: &VectorField2D, name: &'static str) -> Result<(), FieldMetricError> {
    if field.width < 3 || field.height < 3 {
        return Err(FieldMetricError::GridTooSmall);
    }
    if !field.spacing_x.is_finite() || field.spacing_x <= 0.0 {
        return Err(FieldMetricError::InvalidSpacing { field: "spacing_x" });
    }
    if !field.spacing_y.is_finite() || field.spacing_y <= 0.0 {
        return Err(FieldMetricError::InvalidSpacing { field: "spacing_y" });
    }
    let expected = field.len();
    for (field_name, actual) in [
        ("u", field.u.len()),
        ("v", field.v.len()),
        ("valid", field.valid.len()),
    ] {
        if actual != expected {
            return Err(FieldMetricError::ShapeMismatch {
                field: field_name,
                expected,
                actual,
            });
        }
    }
    let _ = name;
    Ok(())
}

fn validate_config(config: &StructureConfig) -> Result<(), FieldMetricError> {
    if !config.weak_vorticity_min.is_finite()
        || config.weak_vorticity_min <= 0.0
        || !config.weak_vorticity_max.is_finite()
        || config.weak_vorticity_max <= config.weak_vorticity_min
    {
        return Err(FieldMetricError::InvalidConfiguration {
            field: "weak_vorticity_band",
        });
    }
    if !config.minimum_retained_amplitude_fraction.is_finite()
        || !(0.0..=1.0).contains(&config.minimum_retained_amplitude_fraction)
        || config.minimum_retained_amplitude_fraction == 0.0
    {
        return Err(FieldMetricError::InvalidConfiguration {
            field: "minimum_retained_amplitude_fraction",
        });
    }
    for (field, value) in [
        (
            "minimum_vector_coverage_fraction",
            config.minimum_vector_coverage_fraction,
        ),
        (
            "minimum_derivative_coverage_fraction",
            config.minimum_derivative_coverage_fraction,
        ),
    ] {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(FieldMetricError::InvalidConfiguration { field });
        }
    }
    if config.minimum_weak_structure_cells == 0 {
        return Err(FieldMetricError::InvalidConfiguration {
            field: "minimum_weak_structure_cells",
        });
    }
    Ok(())
}

fn vorticity(field: &VectorField2D, west: usize, east: usize, south: usize, north: usize) -> f64 {
    let dv_dx = (field.v[east] - field.v[west]) / (2.0 * field.spacing_x);
    let du_dy = (field.u[north] - field.u[south]) / (2.0 * field.spacing_y);
    dv_dx - du_dy
}

fn ratio(numerator: usize, denominator: usize) -> Option<f64> {
    if denominator == 0 {
        None
    } else {
        Some(numerator as f64 / denominator as f64)
    }
}

fn safe_ratio(numerator: f64, denominator: f64) -> Option<f64> {
    if denominator == 0.0 || !denominator.is_finite() {
        None
    } else {
        Some(numerator / denominator)
    }
}

fn close_enough(left: f64, right: f64) -> bool {
    (left - right).abs() <= 1.0e-12 * left.abs().max(right.abs()).max(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> StructureConfig {
        StructureConfig {
            weak_vorticity_min: 0.01,
            weak_vorticity_max: 0.20,
            minimum_retained_amplitude_fraction: 0.50,
            minimum_vector_coverage_fraction: 0.95,
            minimum_derivative_coverage_fraction: 0.90,
            minimum_weak_structure_cells: 4,
        }
    }

    fn field(width: usize, height: usize, vortex_amplitude: f64) -> VectorField2D {
        let spacing = 0.1;
        let mut u = Vec::with_capacity(width * height);
        let mut v = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                let x_position = (x as f64 - (width as f64 - 1.0) * 0.5) * spacing;
                let y_position = (y as f64 - (height as f64 - 1.0) * 0.5) * spacing;
                let radius_squared = x_position * x_position + y_position * y_position;
                let envelope = (-radius_squared / (2.0 * 0.6 * 0.6)).exp();
                u.push(5.0 - vortex_amplitude * y_position * envelope);
                v.push(vortex_amplitude * x_position * envelope);
            }
        }
        VectorField2D {
            width,
            height,
            spacing_x: spacing,
            spacing_y: spacing,
            valid: vec![true; width * height],
            u,
            v,
        }
    }

    fn domain(width: usize, height: usize) -> Vec<bool> {
        vec![true; width * height]
    }

    #[test]
    fn exact_reconstruction_preserves_weak_vorticity() {
        let reference = field(41, 41, 0.05);
        let report =
            evaluate_structure_preservation(&reference, &reference, &domain(41, 41), &config())
                .expect("valid exact fixture");
        assert_eq!(report.disposition, Ok(()));
        assert_eq!(report.weak_structure_recall, Some(1.0));
        assert_eq!(report.weak_structure_precision, Some(1.0));
        assert!((report.weak_structure_signed_amplitude_ratio.unwrap() - 1.0).abs() < 1.0e-12);
        assert_eq!(report.vector_coverage_fraction, 1.0);
    }

    #[test]
    fn deleting_a_weak_vortex_is_visible_even_when_global_l2_is_small() {
        let reference = field(41, 41, 0.05);
        let reconstruction = field(41, 41, 0.0);
        let report = evaluate_structure_preservation(
            &reference,
            &reconstruction,
            &domain(41, 41),
            &config(),
        )
        .expect("valid erased-vortex fixture");
        assert!(report.relative_l2_on_available_vectors.unwrap() < 0.02);
        assert_eq!(report.weak_structure_recall, Some(0.0));
        assert_eq!(report.weak_structure_signed_amplitude_ratio, Some(0.0));
    }

    #[test]
    fn attenuation_is_reported_as_a_structure_amplitude_loss() {
        let reference = field(41, 41, 0.05);
        let reconstruction = field(41, 41, 0.02);
        let report = evaluate_structure_preservation(
            &reference,
            &reconstruction,
            &domain(41, 41),
            &config(),
        )
        .expect("valid attenuated fixture");
        let ratio = report.weak_structure_signed_amplitude_ratio.unwrap();
        assert!((ratio - 0.4).abs() < 0.08);
        assert!(report.weak_structure_recall.unwrap() < 1.0);
    }

    #[test]
    fn reversed_vorticity_sign_is_not_counted_as_preserved_structure() {
        let reference = field(41, 41, 0.05);
        let reconstruction = field(41, 41, -0.05);
        let report = evaluate_structure_preservation(
            &reference,
            &reconstruction,
            &domain(41, 41),
            &config(),
        )
        .expect("valid sign-reversal fixture");
        assert_eq!(report.weak_structure_recall, Some(0.0));
        assert_eq!(report.weak_structure_sign_agreement, Some(0.0));
        assert!(report.weak_structure_signed_amplitude_ratio.unwrap() < 0.0);
    }

    #[test]
    fn invented_vortex_is_counted_as_false_support_in_background() {
        let reference = field(41, 41, 0.0);
        let reconstruction = field(41, 41, 0.05);
        let report = evaluate_structure_preservation(
            &reference,
            &reconstruction,
            &domain(41, 41),
            &config(),
        )
        .expect("valid hallucinated-vortex fixture");
        assert!(report.false_support_cells > 0);
        assert!(report.false_support_rate_in_background.unwrap() > 0.0);
        assert_eq!(
            report.disposition,
            Err(InsufficientSupport::TooFewWeakStructureCells)
        );
    }

    #[test]
    fn missing_vectors_reduce_coverage_and_weak_structure_recall() {
        let reference = field(41, 41, 0.05);
        let mut reconstruction = reference.clone();
        let center = 20 * 41 + 20;
        for y in 18..=22 {
            for x in 18..=22 {
                reconstruction.valid[y * 41 + x] = false;
            }
        }
        let report = evaluate_structure_preservation(
            &reference,
            &reconstruction,
            &domain(41, 41),
            &config(),
        )
        .expect("valid missing-vector fixture");
        assert!(report.vector_coverage_fraction < 1.0);
        assert!(report.derivative_coverage_fraction < 1.0);
        assert!(report.weak_structure_recall.unwrap() < 1.0);
        assert!(center < 41 * 41);
    }

    #[test]
    fn irregular_domain_boundary_does_not_create_cross_mask_derivatives() {
        let reference = field(41, 41, 0.05);
        let reconstruction = reference.clone();
        let mut mask = domain(41, 41);
        for y in 0..41 {
            mask[y * 41 + 20] = false;
        }
        let report = evaluate_structure_preservation(&reference, &reconstruction, &mask, &config())
            .expect("valid masked-domain fixture");
        assert!(report.derivative_reference_cells < 39 * 39);
        assert_eq!(report.weak_structure_recall, Some(1.0));
    }

    #[test]
    fn reference_holes_are_rejected_instead_of_silently_omitted() {
        let mut reference = field(41, 41, 0.05);
        reference.valid[20 * 41 + 20] = false;
        let reconstruction = field(41, 41, 0.05);
        let result = evaluate_structure_preservation(
            &reference,
            &reconstruction,
            &domain(41, 41),
            &config(),
        );
        assert_eq!(
            result,
            Err(FieldMetricError::ReferenceVectorMissing {
                index: 20 * 41 + 20
            })
        );
    }

    #[test]
    fn non_finite_reconstructed_vectors_are_rejected_when_marked_valid() {
        let reference = field(41, 41, 0.05);
        let mut reconstruction = reference.clone();
        reconstruction.u[100] = f64::NAN;
        let result = evaluate_structure_preservation(
            &reference,
            &reconstruction,
            &domain(41, 41),
            &config(),
        );
        assert_eq!(
            result,
            Err(FieldMetricError::NonFiniteReconstruction { index: 100 })
        );
    }

    #[test]
    fn invalid_grid_and_structure_thresholds_are_rejected() {
        let reference = field(41, 41, 0.05);
        let mut invalid = config();
        invalid.weak_vorticity_max = invalid.weak_vorticity_min;
        assert_eq!(
            evaluate_structure_preservation(&reference, &reference, &domain(41, 41), &invalid),
            Err(FieldMetricError::InvalidConfiguration {
                field: "weak_vorticity_band"
            })
        );
    }

    #[test]
    fn insufficient_prediction_support_is_explicit() {
        let reference = field(41, 41, 0.05);
        let mut reconstruction = reference.clone();
        for y in 0..41 {
            for x in 0..41 {
                if x < 10 {
                    reconstruction.valid[y * 41 + x] = false;
                }
            }
        }
        let report = evaluate_structure_preservation(
            &reference,
            &reconstruction,
            &domain(41, 41),
            &config(),
        )
        .expect("valid low-coverage fixture");
        assert_eq!(
            report.disposition,
            Err(InsufficientSupport::VectorCoverageBelowMinimum)
        );
    }
}
