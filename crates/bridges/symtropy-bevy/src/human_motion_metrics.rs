// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Dependency-light metrics for validating human motion traces.
//!
//! Metrics operate on world-space positions in metres. Samples compared by trajectory
//! index must be time-aligned by the caller. These functions measure geometric error;
//! they do not establish that a motion looks human or is safe in every context.

use std::error::Error;
use std::fmt;

/// Invalid or insufficient input to a motion-quality metric.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricInputError {
    EmptySequence,
    LengthMismatch,
    NonFiniteValue,
    NonMonotonicTime,
    TooFewSamples,
    NoContactSamples,
}

impl fmt::Display for MetricInputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptySequence => "motion sequence is empty",
            Self::LengthMismatch => "motion sequences have different lengths",
            Self::NonFiniteValue => "input contains a non-finite value",
            Self::NonMonotonicTime => "timestamps must be strictly increasing",
            Self::TooFewSamples => "at least four samples are needed for jerk estimation",
            Self::NoContactSamples => "no samples were marked as stance/contact",
        };
        f.write_str(message)
    }
}

impl Error for MetricInputError {}

/// Pointwise root-trajectory error after aligning predicted and reference samples.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrajectoryErrors {
    /// Number of aligned sample pairs scored.
    pub samples: usize,
    /// Mean Euclidean displacement error, in metres.
    pub mean_displacement_error_m: f64,
    /// Euclidean error at the final aligned sample, in metres.
    pub final_displacement_error_m: f64,
    /// Largest Euclidean displacement error, in metres.
    pub max_displacement_error_m: f64,
}

/// Foot drift during stance, measured relative to the first point of each contact run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StanceFootDrift {
    /// Number of contact samples, including each stance run's zero-drift anchor.
    pub samples_in_contact: usize,
    /// Root-mean-square world-space drift from the stance anchor, in metres.
    pub rms_drift_m: f64,
    /// Largest world-space drift from a stance anchor, in metres.
    pub max_drift_m: f64,
}

/// Compare time-aligned predicted and reference 3D root trajectories.
///
/// Both slices must contain the same non-zero number of samples. Interpolate trajectories
/// to a shared timestamp grid before calling this function if source sample rates differ.
pub fn trajectory_errors(
    predicted: &[[f64; 3]],
    reference: &[[f64; 3]],
) -> Result<TrajectoryErrors, MetricInputError> {
    validate_positions(predicted)?;
    validate_positions(reference)?;
    if predicted.len() != reference.len() {
        return Err(MetricInputError::LengthMismatch);
    }

    let mut sum_error = 0.0;
    let mut max_error: f64 = 0.0;
    let mut final_error = 0.0;

    for (predicted_position, reference_position) in predicted.iter().zip(reference) {
        let error = distance(*predicted_position, *reference_position);
        if !error.is_finite() {
            return Err(MetricInputError::NonFiniteValue);
        }
        sum_error += error;
        max_error = max_error.max(error);
        final_error = error;
    }

    if !sum_error.is_finite() {
        return Err(MetricInputError::NonFiniteValue);
    }

    Ok(TrajectoryErrors {
        samples: predicted.len(),
        mean_displacement_error_m: sum_error / predicted.len() as f64,
        final_displacement_error_m: final_error,
        max_displacement_error_m: max_error,
    })
}

/// Measure world-space foot drift within consecutive stance/contact intervals.
///
/// A false contact sample ends the current stance interval. Each new contact interval
/// establishes a fresh anchor. The metric reports the RMS and maximum deviation from
/// those anchors, not foot movement caused by the character's root trajectory.
pub fn stance_foot_drift(
    world_positions: &[[f64; 3]],
    in_contact: &[bool],
) -> Result<StanceFootDrift, MetricInputError> {
    validate_positions(world_positions)?;
    if world_positions.len() != in_contact.len() {
        return Err(MetricInputError::LengthMismatch);
    }

    let mut anchor: Option<[f64; 3]> = None;
    let mut sum_squared = 0.0;
    let mut sample_count = 0usize;
    let mut max_drift: f64 = 0.0;

    for (position, contact) in world_positions.iter().zip(in_contact) {
        if !contact {
            anchor = None;
            continue;
        }

        let anchor_position = *anchor.get_or_insert(*position);
        let drift = distance(*position, anchor_position);
        if !drift.is_finite() {
            return Err(MetricInputError::NonFiniteValue);
        }
        sum_squared += drift * drift;
        if !sum_squared.is_finite() {
            return Err(MetricInputError::NonFiniteValue);
        }
        sample_count += 1;
        max_drift = max_drift.max(drift);
    }

    if sample_count == 0 {
        return Err(MetricInputError::NoContactSamples);
    }

    Ok(StanceFootDrift {
        samples_in_contact: sample_count,
        rms_drift_m: (sum_squared / sample_count as f64).sqrt(),
        max_drift_m: max_drift,
    })
}

/// Estimate mean-squared 3D jerk (m²/s⁶) from position samples and timestamps.
///
/// Uses third divided differences over sliding four-sample windows. This is exact for
/// cubic position curves sampled at distinct times, but remains a noise-sensitive discrete
/// estimate; smooth or filter measurements only under a documented, fixed protocol.
pub fn mean_squared_jerk(
    positions: &[[f64; 3]],
    timestamps_sec: &[f64],
) -> Result<f64, MetricInputError> {
    validate_positions(positions)?;
    if positions.len() != timestamps_sec.len() {
        return Err(MetricInputError::LengthMismatch);
    }
    if positions.len() < 4 {
        return Err(MetricInputError::TooFewSamples);
    }
    if timestamps_sec.iter().any(|time| !time.is_finite()) {
        return Err(MetricInputError::NonFiniteValue);
    }
    if timestamps_sec.windows(2).any(|pair| pair[1] <= pair[0]) {
        return Err(MetricInputError::NonMonotonicTime);
    }
    if timestamps_sec
        .windows(2)
        .any(|pair| !(pair[1] - pair[0]).is_finite())
    {
        return Err(MetricInputError::NonFiniteValue);
    }

    let mut squared_jerk_sum = 0.0;
    let mut windows = 0usize;

    for index in 0..positions.len() - 3 {
        let p0 = positions[index];
        let p1 = positions[index + 1];
        let p2 = positions[index + 2];
        let p3 = positions[index + 3];

        let t0 = timestamps_sec[index];
        let t1 = timestamps_sec[index + 1];
        let t2 = timestamps_sec[index + 2];
        let t3 = timestamps_sec[index + 3];

        let dt01 = t1 - t0;
        let dt12 = t2 - t1;
        let dt23 = t3 - t2;
        let dt02 = t2 - t0;
        let dt13 = t3 - t1;
        let dt03 = t3 - t0;
        if [dt01, dt12, dt23, dt02, dt13, dt03]
            .iter()
            .any(|delta| !delta.is_finite() || *delta <= 0.0)
        {
            return Err(MetricInputError::NonFiniteValue);
        }

        let d01 = scale(subtract(p1, p0), 1.0 / dt01);
        let d12 = scale(subtract(p2, p1), 1.0 / dt12);
        let d23 = scale(subtract(p3, p2), 1.0 / dt23);

        let dd012 = scale(subtract(d12, d01), 1.0 / dt02);
        let dd123 = scale(subtract(d23, d12), 1.0 / dt13);
        let ddd0123 = scale(subtract(dd123, dd012), 1.0 / dt03);
        let jerk = scale(ddd0123, 6.0);
        let squared_jerk = dot(jerk, jerk);
        if jerk.iter().any(|value| !value.is_finite()) || !squared_jerk.is_finite() {
            return Err(MetricInputError::NonFiniteValue);
        }

        squared_jerk_sum += squared_jerk;
        if !squared_jerk_sum.is_finite() {
            return Err(MetricInputError::NonFiniteValue);
        }
        windows += 1;
    }

    Ok(squared_jerk_sum / windows as f64)
}

fn validate_positions(positions: &[[f64; 3]]) -> Result<(), MetricInputError> {
    if positions.is_empty() {
        return Err(MetricInputError::EmptySequence);
    }
    if positions.iter().flatten().any(|value| !value.is_finite()) {
        return Err(MetricInputError::NonFiniteValue);
    }
    Ok(())
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    dot(subtract(a, b), subtract(a, b)).sqrt()
}

fn subtract(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(vector: [f64; 3], factor: f64) -> [f64; 3] {
    [vector[0] * factor, vector[1] * factor, vector[2] * factor]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_trajectories_have_zero_error() {
        let positions = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 1.0, 0.0]];
        let result = trajectory_errors(&positions, &positions).unwrap();
        assert_eq!(result.samples, 3);
        assert_eq!(result.mean_displacement_error_m, 0.0);
        assert_eq!(result.final_displacement_error_m, 0.0);
        assert_eq!(result.max_displacement_error_m, 0.0);
    }

    #[test]
    fn trajectory_error_reports_mean_final_and_maximum() {
        let predicted = [[1.0, 0.0, 0.0], [2.0, 0.0, 0.0], [3.0, 0.0, 0.0]];
        let reference = [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [1.0, 0.0, 0.0]];
        let result = trajectory_errors(&predicted, &reference).unwrap();
        assert!((result.mean_displacement_error_m - 1.0).abs() < 1e-12);
        assert!((result.final_displacement_error_m - 2.0).abs() < 1e-12);
        assert!((result.max_displacement_error_m - 2.0).abs() < 1e-12);
    }

    #[test]
    fn stance_foot_drift_resets_between_contact_runs() {
        let positions = [
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            [0.01, 0.0, 0.0],
            [0.5, 0.0, 0.0],
            [0.51, 0.0, 0.0],
        ];
        let contacts = [false, true, true, false, true];
        let result = stance_foot_drift(&positions, &contacts).unwrap();
        assert_eq!(result.samples_in_contact, 3);
        assert!((result.rms_drift_m - (0.0001_f64 / 3.0).sqrt()).abs() < 1e-12);
        assert!((result.max_drift_m - 0.01).abs() < 1e-12);
    }

    #[test]
    fn stance_foot_drift_rejects_no_contact() {
        let positions = [[0.0, 0.0, 0.0]; 2];
        assert_eq!(
            stance_foot_drift(&positions, &[false, false]),
            Err(MetricInputError::NoContactSamples)
        );
    }

    #[test]
    fn cubic_path_has_expected_constant_jerk() {
        let times = [0.0, 1.0, 2.0, 3.0, 4.0];
        let positions: Vec<[f64; 3]> = times
            .iter()
            .map(|time| [time * time * time, 0.0, 0.0])
            .collect();
        let result = mean_squared_jerk(&positions, &times).unwrap();
        assert!((result - 36.0).abs() < 1e-9);
    }

    #[test]
    fn metric_inputs_fail_closed_on_mismatch_or_non_finite_values() {
        assert_eq!(
            trajectory_errors(&[[0.0, 0.0, 0.0]], &[]),
            Err(MetricInputError::EmptySequence)
        );
        assert_eq!(
            trajectory_errors(&[[0.0, 0.0, 0.0]], &[[0.0, 0.0, 0.0]; 2]),
            Err(MetricInputError::LengthMismatch)
        );
        assert_eq!(
            trajectory_errors(&[[f64::NAN, 0.0, 0.0]], &[[0.0, 0.0, 0.0]]),
            Err(MetricInputError::NonFiniteValue)
        );
        assert_eq!(
            mean_squared_jerk(&[[0.0, 0.0, 0.0]; 4], &[0.0, 1.0, 1.0, 2.0]),
            Err(MetricInputError::NonMonotonicTime)
        );
    }
}
