#![forbid(unsafe_code)]

//! Exact-grid comparison of predicted and observed magnetic vector-field maps.
//! Metrics are deterministic and unweighted. No interpolation, solver, or
//! physical qualification is performed.

use std::collections::HashSet;
use std::fmt;

/// Position in a right-handed Cartesian frame, in metres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PositionMeters(pub [f64; 3]);

/// Magnetic flux-density vector, in tesla.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldVectorTesla(pub [f64; 3]);

/// One identified sample in a frozen field-map grid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldSample {
    pub id: u64,
    pub position: PositionMeters,
    pub field: FieldVectorTesla,
}

/// A field map bound to caller-defined frame and grid identities.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldMap {
    pub frame_id: String,
    pub grid_id: String,
    pub samples: Vec<FieldSample>,
}

/// Thresholds are explicit inputs, never inferred from the compared maps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ComparisonPolicy {
    pub absolute_vector_error_tolerance_t: f64,
    pub direction_magnitude_floor_t: f64,
}

impl ComparisonPolicy {
    #[must_use]
    pub const fn new(
        absolute_vector_error_tolerance_t: f64,
        direction_magnitude_floor_t: f64,
    ) -> Self {
        Self {
            absolute_vector_error_tolerance_t,
            direction_magnitude_floor_t,
        }
    }
}

/// Independent scores. No composite or hidden weighted score is emitted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldMapMetrics {
    pub sample_count: usize,
    pub component_mae_t: f64,
    pub vector_mae_t: f64,
    pub vector_rmse_t: f64,
    pub reference_rms_t: f64,
    /// Undefined when the reference map has zero RMS field.
    pub normalized_vector_rmse: Option<f64>,
    pub max_vector_error_t: f64,
    /// Nearest-rank 95th percentile of per-sample vector error.
    pub p95_vector_error_t: f64,
    pub worst_sample_id: u64,
    pub within_absolute_tolerance_fraction: f64,
    /// Applies only to the supplied exact grid; it is not component qualification.
    pub all_samples_within_absolute_tolerance: bool,
    pub directional_sample_count: usize,
    pub skipped_directional_sample_count: usize,
    pub mean_direction_error_rad: Option<f64>,
    pub max_direction_error_rad: Option<f64>,
    pub direction_reversal_fraction: Option<f64>,
    pub reversed_directional_sample_count: usize,
}

/// Invalid input, identity mismatch, or unrepresentable arithmetic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldMapError {
    EmptyFrameId,
    EmptyGridId,
    EmptySamples,
    DuplicateSampleId(u64),
    NonFinitePosition(u64),
    NonFiniteField(u64),
    FrameMismatch,
    GridMismatch,
    SampleCountMismatch,
    SampleIdMismatch { index: usize },
    PositionMismatch { sample_id: u64 },
    InvalidPolicy,
    NonFiniteArithmetic { sample_id: u64 },
    MetricOverflow,
}

impl fmt::Display for FieldMapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyFrameId => f.write_str("coordinate frame identifier is empty"),
            Self::EmptyGridId => f.write_str("grid identifier is empty"),
            Self::EmptySamples => f.write_str("field map contains no samples"),
            Self::DuplicateSampleId(id) => write!(f, "duplicate sample identifier: {id}"),
            Self::NonFinitePosition(id) => write!(f, "sample {id} has a non-finite position"),
            Self::NonFiniteField(id) => write!(f, "sample {id} has a non-finite field"),
            Self::FrameMismatch => f.write_str("coordinate frame identifiers differ"),
            Self::GridMismatch => f.write_str("grid identifiers differ"),
            Self::SampleCountMismatch => f.write_str("field maps have different sample counts"),
            Self::SampleIdMismatch { index } => {
                write!(f, "sample identifiers differ at index {index}")
            }
            Self::PositionMismatch { sample_id } => {
                write!(f, "sample {sample_id} positions differ")
            }
            Self::InvalidPolicy => f.write_str("comparison policy contains an invalid threshold"),
            Self::NonFiniteArithmetic { sample_id } => {
                write!(
                    f,
                    "comparison arithmetic is non-finite at sample {sample_id}"
                )
            }
            Self::MetricOverflow => f.write_str("a field-map metric is outside the f64 range"),
        }
    }
}

impl std::error::Error for FieldMapError {}

/// Compare two maps without interpolation, reordering, or silent sample loss.
///
/// Matching requires identical frame ID, grid ID, sample count, ordered sample
/// IDs and exact sample positions. The tolerance flag applies only to these
/// supplied points. It says nothing about unsampled space or component fitness.
///
/// # Errors
///
/// Rejects empty/invalid maps, duplicate IDs, non-finite values, frame/grid
/// mismatch, different sample identity/order/positions, invalid thresholds,
/// and arithmetic that cannot be represented with f64.
pub fn compare_field_maps(
    reference: &FieldMap,
    predicted: &FieldMap,
    policy: ComparisonPolicy,
) -> Result<FieldMapMetrics, FieldMapError> {
    validate_map(reference)?;
    validate_map(predicted)?;
    validate_policy(policy)?;

    if reference.frame_id != predicted.frame_id {
        return Err(FieldMapError::FrameMismatch);
    }
    if reference.grid_id != predicted.grid_id {
        return Err(FieldMapError::GridMismatch);
    }
    if reference.samples.len() != predicted.samples.len() {
        return Err(FieldMapError::SampleCountMismatch);
    }

    let count = reference.samples.len();
    let mut component_mean = [0.0_f64; 3];
    let mut vector_mae = 0.0_f64;
    let mut error_rms = RmsAccumulator::default();
    let mut reference_rms = RmsAccumulator::default();
    let mut errors = Vec::with_capacity(count);
    let mut within_tolerance_count = 0_usize;
    let mut max_vector_error_t = -1.0_f64;
    let mut worst_sample_id = reference.samples[0].id;

    let mut directional_sample_count = 0_usize;
    let mut skipped_directional_sample_count = 0_usize;
    let mut mean_direction_error_rad = 0.0_f64;
    let mut max_direction_error_rad = 0.0_f64;
    let mut reversed_directional_sample_count = 0_usize;

    for (index, (r, p)) in reference.samples.iter().zip(&predicted.samples).enumerate() {
        if r.id != p.id {
            return Err(FieldMapError::SampleIdMismatch { index });
        }
        if r.position != p.position {
            return Err(FieldMapError::PositionMismatch { sample_id: r.id });
        }

        let error = subtract(p.field.0, r.field.0);
        if !finite(error) {
            return Err(FieldMapError::NonFiniteArithmetic { sample_id: r.id });
        }
        let error_norm = magnitude(error);
        let reference_norm = magnitude(r.field.0);
        let predicted_norm = magnitude(p.field.0);
        if !error_norm.is_finite() || !reference_norm.is_finite() || !predicted_norm.is_finite() {
            return Err(FieldMapError::NonFiniteArithmetic { sample_id: r.id });
        }

        let divisor = (index + 1) as f64;
        for axis in 0..3 {
            let value = error[axis].abs();
            component_mean[axis] += (value - component_mean[axis]) / divisor;
        }
        vector_mae += (error_norm - vector_mae) / divisor;
        error_rms.push(error_norm);
        reference_rms.push(reference_norm);
        errors.push(error_norm);

        if error_norm <= policy.absolute_vector_error_tolerance_t {
            within_tolerance_count += 1;
        }
        if error_norm > max_vector_error_t {
            max_vector_error_t = error_norm;
            worst_sample_id = r.id;
        }

        if reference_norm > policy.direction_magnitude_floor_t
            && predicted_norm > policy.direction_magnitude_floor_t
        {
            let angle = direction_angle(r.field.0, p.field.0, reference_norm, predicted_norm);
            let next_count = (directional_sample_count + 1) as f64;
            mean_direction_error_rad += (angle - mean_direction_error_rad) / next_count;
            max_direction_error_rad = max_direction_error_rad.max(angle);
            let cosine = dot(
                normalize(r.field.0, reference_norm),
                normalize(p.field.0, predicted_norm),
            );
            if cosine < 0.0 {
                reversed_directional_sample_count += 1;
            }
            directional_sample_count += 1;
        } else {
            skipped_directional_sample_count += 1;
        }
    }

    let vector_rmse_t = error_rms.value().ok_or(FieldMapError::MetricOverflow)?;
    let reference_rms_t = reference_rms.value().ok_or(FieldMapError::MetricOverflow)?;
    let component_mae_t =
        component_mean[0] / 3.0 + component_mean[1] / 3.0 + component_mean[2] / 3.0;
    let normalized_vector_rmse = if reference_rms_t > 0.0 {
        let value = vector_rmse_t / reference_rms_t;
        if !value.is_finite() {
            return Err(FieldMapError::MetricOverflow);
        }
        Some(value)
    } else {
        None
    };

    errors.sort_by(f64::total_cmp);
    let p95_rank = (errors.len() * 95).div_ceil(100);
    let p95_vector_error_t = errors[p95_rank - 1];
    let within_absolute_tolerance_fraction = within_tolerance_count as f64 / count as f64;

    let (mean_angle, max_angle, reversal_fraction) = if directional_sample_count > 0 {
        (
            Some(mean_direction_error_rad),
            Some(max_direction_error_rad),
            Some(reversed_directional_sample_count as f64 / directional_sample_count as f64),
        )
    } else {
        (None, None, None)
    };

    let metrics = FieldMapMetrics {
        sample_count: count,
        component_mae_t,
        vector_mae_t: vector_mae,
        vector_rmse_t,
        reference_rms_t,
        normalized_vector_rmse,
        max_vector_error_t,
        p95_vector_error_t,
        worst_sample_id,
        within_absolute_tolerance_fraction,
        all_samples_within_absolute_tolerance: within_tolerance_count == count,
        directional_sample_count,
        skipped_directional_sample_count,
        mean_direction_error_rad: mean_angle,
        max_direction_error_rad: max_angle,
        direction_reversal_fraction: reversal_fraction,
        reversed_directional_sample_count,
    };

    if ![
        metrics.component_mae_t,
        metrics.vector_mae_t,
        metrics.vector_rmse_t,
        metrics.reference_rms_t,
        metrics.max_vector_error_t,
        metrics.p95_vector_error_t,
        metrics.within_absolute_tolerance_fraction,
    ]
    .iter()
    .all(|value| value.is_finite())
    {
        return Err(FieldMapError::MetricOverflow);
    }
    Ok(metrics)
}

fn validate_map(map: &FieldMap) -> Result<(), FieldMapError> {
    if map.frame_id.trim().is_empty() {
        return Err(FieldMapError::EmptyFrameId);
    }
    if map.grid_id.trim().is_empty() {
        return Err(FieldMapError::EmptyGridId);
    }
    if map.samples.is_empty() {
        return Err(FieldMapError::EmptySamples);
    }
    let mut seen = HashSet::with_capacity(map.samples.len());
    for sample in &map.samples {
        if !seen.insert(sample.id) {
            return Err(FieldMapError::DuplicateSampleId(sample.id));
        }
        if !finite(sample.position.0) {
            return Err(FieldMapError::NonFinitePosition(sample.id));
        }
        if !finite(sample.field.0) {
            return Err(FieldMapError::NonFiniteField(sample.id));
        }
    }
    Ok(())
}

fn validate_policy(policy: ComparisonPolicy) -> Result<(), FieldMapError> {
    if !policy.absolute_vector_error_tolerance_t.is_finite()
        || policy.absolute_vector_error_tolerance_t < 0.0
        || !policy.direction_magnitude_floor_t.is_finite()
        || policy.direction_magnitude_floor_t < 0.0
    {
        return Err(FieldMapError::InvalidPolicy);
    }
    Ok(())
}

#[derive(Default)]
struct RmsAccumulator {
    scale: f64,
    sum_squares: f64,
    count: usize,
}

impl RmsAccumulator {
    fn push(&mut self, value: f64) {
        let value = value.abs();
        self.count += 1;
        if value == 0.0 {
            return;
        }
        if self.scale < value {
            let ratio = self.scale / value;
            self.sum_squares = 1.0 + self.sum_squares * ratio * ratio;
            self.scale = value;
        } else {
            let ratio = value / self.scale;
            self.sum_squares += ratio * ratio;
        }
    }

    fn value(&self) -> Option<f64> {
        if self.count == 0 {
            return None;
        }
        let value = self.scale * (self.sum_squares / self.count as f64).sqrt();
        value.is_finite().then_some(value)
    }
}

fn finite(vector: [f64; 3]) -> bool {
    vector.iter().all(|value| value.is_finite())
}

fn magnitude(vector: [f64; 3]) -> f64 {
    vector[0].hypot(vector[1]).hypot(vector[2])
}

fn subtract(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn normalize(vector: [f64; 3], norm: f64) -> [f64; 3] {
    [vector[0] / norm, vector[1] / norm, vector[2] / norm]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn direction_angle(reference: [f64; 3], predicted: [f64; 3], r_norm: f64, p_norm: f64) -> f64 {
    let a = normalize(reference, r_norm);
    let b = normalize(predicted, p_norm);
    magnitude(cross(a, b)).atan2(dot(a, b).clamp(-1.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(id: u64, position: [f64; 3], field: [f64; 3]) -> FieldSample {
        FieldSample {
            id,
            position: PositionMeters(position),
            field: FieldVectorTesla(field),
        }
    }

    fn map(samples: Vec<FieldSample>) -> FieldMap {
        FieldMap {
            frame_id: "component-frame-rh".to_owned(),
            grid_id: "grid-3d-v1".to_owned(),
            samples,
        }
    }

    fn policy(tolerance: f64, floor: f64) -> ComparisonPolicy {
        ComparisonPolicy::new(tolerance, floor)
    }

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual {actual}, expected {expected}, tolerance {tolerance}"
        );
    }

    #[test]
    fn identical_maps_have_zero_error_and_full_coverage() {
        let reference = map(vec![
            sample(1, [0.0; 3], [0.0, 0.0, 2.0]),
            sample(2, [0.0, 0.0, 0.1], [1.0, 0.0, 0.0]),
        ]);
        let result = compare_field_maps(&reference, &reference, policy(0.0, 1.0e-9)).unwrap();
        assert_eq!(result.sample_count, 2);
        assert_eq!(result.component_mae_t, 0.0);
        assert_eq!(result.vector_mae_t, 0.0);
        assert_eq!(result.vector_rmse_t, 0.0);
        assert_eq!(result.normalized_vector_rmse, Some(0.0));
        assert_eq!(result.within_absolute_tolerance_fraction, 1.0);
        assert!(result.all_samples_within_absolute_tolerance);
        assert_eq!(result.direction_reversal_fraction, Some(0.0));
    }

    #[test]
    fn known_answer_metrics_are_computed_independently() {
        let reference = map(vec![
            sample(7, [0.0; 3], [0.0, 0.0, 2.0]),
            sample(8, [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
        ]);
        let predicted = map(vec![
            sample(7, [0.0; 3], [0.0, 0.0, 1.0]),
            sample(8, [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
        ]);
        let result = compare_field_maps(&reference, &predicted, policy(0.5, 0.01)).unwrap();
        close(result.component_mae_t, 1.0 / 6.0, 1.0e-15);
        close(result.vector_mae_t, 0.5, 1.0e-15);
        close(result.vector_rmse_t, 0.5_f64.sqrt(), 1.0e-15);
        close(result.reference_rms_t, 2.5_f64.sqrt(), 1.0e-15);
        close(
            result.normalized_vector_rmse.unwrap(),
            0.2_f64.sqrt(),
            1.0e-15,
        );
        close(result.max_vector_error_t, 1.0, 1.0e-15);
        close(result.p95_vector_error_t, 1.0, 1.0e-15);
        assert_eq!(result.worst_sample_id, 7);
        close(result.within_absolute_tolerance_fraction, 0.5, 1.0e-15);
        assert!(!result.all_samples_within_absolute_tolerance);
        assert_eq!(result.directional_sample_count, 2);
        assert_eq!(result.direction_reversal_fraction, Some(0.0));
    }

    #[test]
    fn field_reversal_is_not_hidden_by_a_permissive_magnitude_tolerance() {
        let reference = map(vec![sample(42, [0.0; 3], [0.0, 0.0, 1.0])]);
        let predicted = map(vec![sample(42, [0.0; 3], [0.0, 0.0, -1.0])]);
        let result = compare_field_maps(&reference, &predicted, policy(2.0, 0.01)).unwrap();
        close(result.max_vector_error_t, 2.0, 1.0e-15);
        close(
            result.mean_direction_error_rad.unwrap(),
            std::f64::consts::PI,
            1.0e-12,
        );
        assert_eq!(result.direction_reversal_fraction, Some(1.0));
        assert_eq!(result.reversed_directional_sample_count, 1);
        assert!(result.all_samples_within_absolute_tolerance);
    }

    #[test]
    fn zero_reference_makes_normalized_and_directional_metrics_undefined() {
        let reference = map(vec![sample(1, [0.0; 3], [0.0; 3])]);
        let predicted = map(vec![sample(1, [0.0; 3], [0.1, 0.0, 0.0])]);
        let result = compare_field_maps(&reference, &predicted, policy(0.2, 0.01)).unwrap();
        assert_eq!(result.reference_rms_t, 0.0);
        assert_eq!(result.normalized_vector_rmse, None);
        assert_eq!(result.directional_sample_count, 0);
        assert_eq!(result.skipped_directional_sample_count, 1);
        assert_eq!(result.mean_direction_error_rad, None);
        assert_eq!(result.direction_reversal_fraction, None);
    }

    #[test]
    fn weak_fields_are_excluded_from_directional_metrics() {
        let reference = map(vec![sample(1, [0.0; 3], [0.0, 0.0, 1.0e-5])]);
        let predicted = map(vec![sample(1, [0.0; 3], [0.0, 0.0, -1.0e-5])]);
        let result = compare_field_maps(&reference, &predicted, policy(1.0, 1.0e-4)).unwrap();
        assert_eq!(result.directional_sample_count, 0);
        assert_eq!(result.skipped_directional_sample_count, 1);
        assert_eq!(result.direction_reversal_fraction, None);
    }

    #[test]
    fn common_rotation_preserves_scalar_scores() {
        let reference = map(vec![
            sample(1, [0.0; 3], [0.2, 0.3, 1.0]),
            sample(2, [0.1, 0.0, 0.0], [-0.1, 0.4, 0.7]),
        ]);
        let predicted = map(vec![
            sample(1, [0.0; 3], [0.1, 0.3, 0.9]),
            sample(2, [0.1, 0.0, 0.0], [-0.2, 0.4, 0.6]),
        ]);
        let original = compare_field_maps(&reference, &predicted, policy(0.2, 0.01)).unwrap();
        let rotate = |v: [f64; 3]| [-v[1], v[0], v[2]];
        let rotate_map = |input: &FieldMap| {
            map(input
                .samples
                .iter()
                .map(|s| sample(s.id, rotate(s.position.0), rotate(s.field.0)))
                .collect())
        };
        let rotated = compare_field_maps(
            &rotate_map(&reference),
            &rotate_map(&predicted),
            policy(0.2, 0.01),
        )
        .unwrap();
        close(rotated.vector_mae_t, original.vector_mae_t, 1.0e-14);
        close(rotated.vector_rmse_t, original.vector_rmse_t, 1.0e-14);
        close(
            rotated.mean_direction_error_rad.unwrap(),
            original.mean_direction_error_rad.unwrap(),
            1.0e-12,
        );
        assert_eq!(rotated.worst_sample_id, original.worst_sample_id);
    }

    #[test]
    fn common_field_scaling_preserves_normalized_error() {
        let r = map(vec![sample(1, [0.0; 3], [0.0, 0.0, 2.0])]);
        let p = map(vec![sample(1, [0.0; 3], [0.0, 0.0, 1.0])]);
        let base = compare_field_maps(&r, &p, policy(2.0, 0.01)).unwrap();
        let rs = map(vec![sample(1, [0.0; 3], [0.0, 0.0, 6.0])]);
        let ps = map(vec![sample(1, [0.0; 3], [0.0, 0.0, 3.0])]);
        let scaled = compare_field_maps(&rs, &ps, policy(6.0, 0.01)).unwrap();
        close(
            scaled.normalized_vector_rmse.unwrap(),
            base.normalized_vector_rmse.unwrap(),
            1.0e-15,
        );
        close(scaled.vector_rmse_t, base.vector_rmse_t * 3.0, 1.0e-15);
    }

    #[test]
    fn identity_order_and_position_mismatches_fail_closed() {
        let reference = map(vec![sample(1, [0.0; 3], [0.0, 0.0, 1.0])]);
        let mut other_frame = reference.clone();
        other_frame.frame_id = "other-frame".to_owned();
        assert_eq!(
            compare_field_maps(&reference, &other_frame, policy(1.0, 0.0)),
            Err(FieldMapError::FrameMismatch)
        );
        let mut other_grid = reference.clone();
        other_grid.grid_id = "other-grid".to_owned();
        assert_eq!(
            compare_field_maps(&reference, &other_grid, policy(1.0, 0.0)),
            Err(FieldMapError::GridMismatch)
        );
        let other_id = map(vec![sample(2, [0.0; 3], [0.0, 0.0, 1.0])]);
        assert_eq!(
            compare_field_maps(&reference, &other_id, policy(1.0, 0.0)),
            Err(FieldMapError::SampleIdMismatch { index: 0 })
        );
        let other_position = map(vec![sample(1, [1.0, 0.0, 0.0], [0.0, 0.0, 1.0])]);
        assert_eq!(
            compare_field_maps(&reference, &other_position, policy(1.0, 0.0)),
            Err(FieldMapError::PositionMismatch { sample_id: 1 })
        );
    }

    #[test]
    fn empty_duplicate_and_count_mismatch_inputs_are_rejected() {
        let reference = map(vec![sample(1, [0.0; 3], [0.0, 0.0, 1.0])]);
        let empty = map(vec![]);
        assert_eq!(
            compare_field_maps(&reference, &empty, policy(1.0, 0.0)),
            Err(FieldMapError::EmptySamples)
        );
        let more = map(vec![
            sample(1, [0.0; 3], [0.0, 0.0, 1.0]),
            sample(2, [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
        ]);
        assert_eq!(
            compare_field_maps(&reference, &more, policy(1.0, 0.0)),
            Err(FieldMapError::SampleCountMismatch)
        );
        let duplicate = map(vec![
            sample(1, [0.0; 3], [0.0, 0.0, 1.0]),
            sample(1, [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
        ]);
        assert_eq!(
            compare_field_maps(&duplicate, &duplicate, policy(1.0, 0.0)),
            Err(FieldMapError::DuplicateSampleId(1))
        );
    }

    #[test]
    fn non_finite_values_and_invalid_policies_are_rejected() {
        let reference = map(vec![sample(1, [0.0; 3], [0.0, 0.0, 1.0])]);
        let nan_field = map(vec![sample(1, [0.0; 3], [f64::NAN, 0.0, 1.0])]);
        assert_eq!(
            compare_field_maps(&reference, &nan_field, policy(1.0, 0.0)),
            Err(FieldMapError::NonFiniteField(1))
        );
        let nan_position = map(vec![sample(1, [f64::INFINITY, 0.0, 0.0], [0.0, 0.0, 1.0])]);
        assert_eq!(
            compare_field_maps(&reference, &nan_position, policy(1.0, 0.0)),
            Err(FieldMapError::NonFinitePosition(1))
        );
        assert_eq!(
            compare_field_maps(&reference, &reference, policy(f64::INFINITY, 0.0)),
            Err(FieldMapError::InvalidPolicy)
        );
        assert_eq!(
            compare_field_maps(&reference, &reference, policy(1.0, -1.0)),
            Err(FieldMapError::InvalidPolicy)
        );
    }

    #[test]
    fn stable_norm_handles_large_components_without_intermediate_square_overflow() {
        let reference = map(vec![sample(1, [0.0; 3], [0.0; 3])]);
        let predicted = map(vec![sample(1, [0.0; 3], [1.0e200, 1.0e200, 1.0e200])]);
        let result = compare_field_maps(&reference, &predicted, policy(1.0e200, 0.0)).unwrap();
        assert!(result.vector_rmse_t.is_finite());
        assert!(result.max_vector_error_t.is_finite());
        assert_eq!(result.within_absolute_tolerance_fraction, 0.0);
    }
}
