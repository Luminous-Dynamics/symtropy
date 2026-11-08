//! Reduced causal source-history controls for dynamical QNM activation.
//!
//! This crate evaluates a retarded single-pole kernel
//!
//!     g_R(t) = theta(t) exp(-i omega t)
//!
//! against piecewise-constant source histories. It returns the history-driven
//! convolution ("activation") separately from an instantaneous direct channel.
//! This is an analytic reduced-model control, not the published Schwarzschild
//! EMRI activation formula and not a black-hole Green-function solver.

use core::fmt;
use num_complex::Complex64;

/// A constant source segment on the half-open interval [start, end).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourceSegment {
    pub start: f64,
    pub end: f64,
    pub amplitude: Complex64,
}

impl SourceSegment {
    pub fn new(start: f64, end: f64, amplitude: Complex64) -> Result<Self, SourceError> {
        if !start.is_finite() || !end.is_finite() || !finite(amplitude) {
            return Err(SourceError::NonFiniteInput);
        }
        if start >= end {
            return Err(SourceError::InvalidSegment);
        }
        Ok(Self {
            start,
            end,
            amplitude,
        })
    }
}

/// A validated, sorted, non-overlapping source history.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceHistory {
    segments: Vec<SourceSegment>,
}

/// Stable retarded pole with a separate direct/feedthrough channel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RetardedPole {
    omega: Complex64,
    direct_gain: Complex64,
}

/// The three explicitly separated outputs of one source-history evaluation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DynamicResponse {
    /// Retarded convolution of the full source history with exp(-i omega t).
    pub activation: Complex64,
    /// Instantaneous source/direct channel at the observation time.
    pub direct: Complex64,
    /// Sum of activation and direct channels.
    pub total: Complex64,
}

/// Errors for invalid source histories and unstable/non-finite pole data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceError {
    NonFiniteInput,
    InvalidSegment,
    OverlappingOrUnsortedSegments,
    UnstablePole,
    NonFiniteResult,
}

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteInput => write!(f, "source, time and pole inputs must be finite"),
            Self::InvalidSegment => write!(f, "source segment must have start < end"),
            Self::OverlappingOrUnsortedSegments => {
                write!(f, "source segments must be sorted and non-overlapping")
            }
            Self::UnstablePole => write!(f, "reduced retarded pole must not grow in forward time"),
            Self::NonFiniteResult => write!(f, "retarded convolution produced a non-finite result"),
        }
    }
}

impl std::error::Error for SourceError {}

fn finite(value: Complex64) -> bool {
    value.re.is_finite() && value.im.is_finite()
}

impl SourceHistory {
    /// Construct a sorted, non-overlapping source history.
    ///
    /// Adjacent segments may share an endpoint. At the shared endpoint the
    /// earlier segment is inactive and the later segment is active.
    pub fn new(segments: Vec<SourceSegment>) -> Result<Self, SourceError> {
        for segment in &segments {
            if !segment.start.is_finite() || !segment.end.is_finite() || !finite(segment.amplitude)
            {
                return Err(SourceError::NonFiniteInput);
            }
            if segment.start >= segment.end {
                return Err(SourceError::InvalidSegment);
            }
        }
        for pair in segments.windows(2) {
            if pair[1].start < pair[0].end {
                return Err(SourceError::OverlappingOrUnsortedSegments);
            }
        }
        Ok(Self { segments })
    }

    pub fn segments(&self) -> &[SourceSegment] {
        &self.segments
    }

    /// Evaluate the instantaneous source on the half-open segment convention.
    pub fn value_at(&self, time: f64) -> Result<Complex64, SourceError> {
        if !time.is_finite() {
            return Err(SourceError::NonFiniteInput);
        }
        Ok(self
            .segments
            .iter()
            .find(|segment| time >= segment.start && time < segment.end)
            .map_or(Complex64::new(0.0, 0.0), |segment| segment.amplitude))
    }
}

impl RetardedPole {
    /// Construct a damped or marginal pole. For exp(-i omega t), Im(omega)>0
    /// would grow in forward time and is rejected by this stable-control crate.
    pub fn new(omega: Complex64, direct_gain: Complex64) -> Result<Self, SourceError> {
        if !finite(omega) || !finite(direct_gain) {
            return Err(SourceError::NonFiniteInput);
        }
        if omega.im > 0.0 {
            return Err(SourceError::UnstablePole);
        }
        Ok(Self { omega, direct_gain })
    }

    pub const fn omega(self) -> Complex64 {
        self.omega
    }

    pub const fn direct_gain(self) -> Complex64 {
        self.direct_gain
    }

    /// Stable evaluation of integral_0^duration exp(-i omega u) du.
    ///
    /// The short-argument branch avoids cancellation when omega*duration is
    /// close to zero. The fifth-order Taylor truncation is used only for
    /// |omega*duration| < 1e-4.
    fn integrated_kernel(self, duration: f64) -> Complex64 {
        if duration <= 0.0 {
            return Complex64::new(0.0, 0.0);
        }
        let z = Complex64::new(0.0, -1.0) * self.omega * duration;
        let integral_factor = if z.norm() < 1.0e-4 {
            let z2 = z * z;
            let z3 = z2 * z;
            let z4 = z3 * z;
            Complex64::new(1.0, 0.0) + z * 0.5 + z2 / 6.0 + z3 / 24.0 + z4 / 120.0
        } else {
            (Complex64::new(1.0, 0.0) - z.exp()) / (-z)
        };
        Complex64::new(duration, 0.0) * integral_factor
    }

    /// Evaluate history activation, direct channel and their sum at time t.
    ///
    /// For each active source segment [a,b), the activation contribution is
    ///
    ///     integral_a^min(b,t) exp(-i omega (t-tau)) S(tau) d tau.
    ///
    /// No contribution is included from future source support.
    pub fn evaluate(
        self,
        history: &SourceHistory,
        time: f64,
    ) -> Result<DynamicResponse, SourceError> {
        if !time.is_finite() {
            return Err(SourceError::NonFiniteInput);
        }

        let mut activation = Complex64::new(0.0, 0.0);
        for segment in &history.segments {
            if time <= segment.start {
                continue;
            }
            let upper = segment.end.min(time);
            let duration = upper - segment.start;
            if duration <= 0.0 {
                continue;
            }

            let delay = time - upper;
            let phase = (Complex64::new(0.0, -1.0) * self.omega * delay).exp();
            activation += segment.amplitude * phase * self.integrated_kernel(duration);
        }

        let source_now = history.value_at(time)?;
        let direct = self.direct_gain * source_now;
        let total = activation + direct;
        if finite(activation) && finite(direct) && finite(total) {
            Ok(DynamicResponse {
                activation,
                direct,
                total,
            })
        } else {
            Err(SourceError::NonFiniteResult)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn z(re: f64, im: f64) -> Complex64 {
        Complex64::new(re, im)
    }

    fn segment(start: f64, end: f64, amplitude: Complex64) -> SourceSegment {
        SourceSegment::new(start, end, amplitude).unwrap()
    }

    fn pole() -> RetardedPole {
        RetardedPole::new(z(1.2, -0.15), z(0.4, 0.2)).unwrap()
    }

    #[test]
    fn no_response_precedes_source_support() {
        let history = SourceHistory::new(vec![segment(2.0, 3.0, z(1.0, 0.0))]).unwrap();
        let response = pole().evaluate(&history, 1.999).unwrap();
        assert_eq!(response.activation, z(0.0, 0.0));
        assert_eq!(response.direct, z(0.0, 0.0));
        assert_eq!(response.total, z(0.0, 0.0));
    }

    #[test]
    fn rectangular_source_matches_exact_activation_inside_and_after_pulse() {
        let omega = z(1.0, -0.25);
        let retarded = RetardedPole::new(omega, z(0.0, 0.0)).unwrap();
        let source = z(1.3, -0.2);
        let history = SourceHistory::new(vec![segment(1.0, 2.5, source)]).unwrap();

        let inside_time = 1.8;
        let inside_duration = inside_time - 1.0;
        let expected_inside = source
            * (z(1.0, 0.0) - (z(0.0, -1.0) * omega * inside_duration).exp())
            / (z(0.0, 1.0) * omega);
        let inside = retarded.evaluate(&history, inside_time).unwrap();
        assert!((inside.activation - expected_inside).norm() < 2.0e-14);

        let after_time = 3.1;
        let delay_after_segment = after_time - 2.5;
        let duration = 2.5 - 1.0;
        let expected_after = source
            * (z(0.0, -1.0) * omega * delay_after_segment).exp()
            * (z(1.0, 0.0) - (z(0.0, -1.0) * omega * duration).exp())
            / (z(0.0, 1.0) * omega);
        let after = retarded.evaluate(&history, after_time).unwrap();
        assert!((after.activation - expected_after).norm() < 2.0e-14);
        assert_eq!(after.direct, z(0.0, 0.0));
    }

    #[test]
    fn instantaneous_direct_channel_is_separate_from_history_activation() {
        let history = SourceHistory::new(vec![segment(0.0, 2.0, z(2.0, 1.0))]).unwrap();
        let response = pole().evaluate(&history, 0.5).unwrap();
        assert_eq!(response.direct, z(0.4, 0.2) * z(2.0, 1.0));
        assert!(response.activation.norm() > 0.0);
        assert!((response.total - response.activation - response.direct).norm() < 1e-14);
    }

    #[test]
    fn time_translation_equivariance_preserves_activation() {
        let shift = 7.25;
        let history = SourceHistory::new(vec![
            segment(0.5, 1.0, z(1.0, 0.2)),
            segment(1.5, 2.0, z(-0.3, 0.8)),
        ])
        .unwrap();
        let shifted = SourceHistory::new(vec![
            segment(0.5 + shift, 1.0 + shift, z(1.0, 0.2)),
            segment(1.5 + shift, 2.0 + shift, z(-0.3, 0.8)),
        ])
        .unwrap();
        let a = pole().evaluate(&history, 3.0).unwrap();
        let b = pole().evaluate(&shifted, 3.0 + shift).unwrap();
        assert!((a.activation - b.activation).norm() < 2.0e-14);
        assert!((a.direct - b.direct).norm() < 1.0e-14);
    }

    #[test]
    fn source_scaling_scales_activation_and_direct_linearly() {
        let history = SourceHistory::new(vec![segment(0.0, 1.0, z(0.5, -0.2))]).unwrap();
        let scale = z(-1.3, 0.7);
        let scaled_history =
            SourceHistory::new(vec![segment(0.0, 1.0, scale * z(0.5, -0.2))]).unwrap();
        let base = pole().evaluate(&history, 0.8).unwrap();
        let scaled = pole().evaluate(&scaled_history, 0.8).unwrap();
        assert!((scaled.activation - scale * base.activation).norm() < 2.0e-14);
        assert!((scaled.direct - scale * base.direct).norm() < 2.0e-14);
    }

    #[test]
    fn identical_instantaneous_source_can_have_different_history_activation() {
        let earlier = SourceHistory::new(vec![segment(0.0, 1.0, z(1.0, 0.0))]).unwrap();
        let later = SourceHistory::new(vec![segment(0.5, 1.0, z(1.0, 0.0))]).unwrap();
        let time = 0.75;
        assert_eq!(
            earlier.value_at(time).unwrap(),
            later.value_at(time).unwrap()
        );
        let a = pole().evaluate(&earlier, time).unwrap();
        let b = pole().evaluate(&later, time).unwrap();
        assert_eq!(a.direct, b.direct);
        assert!((a.activation - b.activation).norm() > 1.0e-6);
    }

    #[test]
    fn zero_frequency_limit_is_the_integrated_source_duration() {
        let retarded = RetardedPole::new(z(0.0, 0.0), z(0.0, 0.0)).unwrap();
        let history = SourceHistory::new(vec![segment(1.0, 3.0, z(2.0, -1.0))]).unwrap();
        let response = retarded.evaluate(&history, 4.0).unwrap();
        assert!((response.activation - z(4.0, -2.0)).norm() < 1.0e-14);
    }

    #[test]
    fn adjacent_segments_and_gaps_are_supported() {
        let history = SourceHistory::new(vec![
            segment(0.0, 1.0, z(1.0, 0.0)),
            segment(1.0, 2.0, z(2.0, 0.0)),
            segment(3.0, 4.0, z(3.0, 0.0)),
        ])
        .unwrap();
        assert_eq!(history.value_at(1.0).unwrap(), z(2.0, 0.0));
        assert_eq!(history.value_at(2.5).unwrap(), z(0.0, 0.0));
        assert_eq!(history.value_at(3.0).unwrap(), z(3.0, 0.0));
    }

    #[test]
    fn overlapping_unsorted_and_invalid_segments_are_rejected() {
        assert_eq!(
            SourceSegment::new(1.0, 1.0, z(1.0, 0.0)),
            Err(SourceError::InvalidSegment)
        );
        assert_eq!(
            SourceHistory::new(vec![
                segment(0.0, 2.0, z(1.0, 0.0)),
                segment(1.0, 3.0, z(1.0, 0.0)),
            ]),
            Err(SourceError::OverlappingOrUnsortedSegments)
        );
        assert_eq!(
            SourceHistory::new(vec![SourceSegment {
                start: f64::NAN,
                end: 1.0,
                amplitude: z(1.0, 0.0),
            }]),
            Err(SourceError::NonFiniteInput)
        );
        assert_eq!(
            SourceHistory::new(vec![
                segment(2.0, 3.0, z(1.0, 0.0)),
                segment(0.0, 1.0, z(1.0, 0.0)),
            ]),
            Err(SourceError::OverlappingOrUnsortedSegments)
        );
    }

    #[test]
    fn unstable_non_finite_and_non_finite_time_inputs_fail_closed() {
        assert_eq!(
            RetardedPole::new(z(1.0, 0.1), z(0.0, 0.0)),
            Err(SourceError::UnstablePole)
        );
        assert_eq!(
            RetardedPole::new(z(f64::NAN, 0.0), z(0.0, 0.0)),
            Err(SourceError::NonFiniteInput)
        );
        let history = SourceHistory::new(vec![]).unwrap();
        assert_eq!(
            pole().evaluate(&history, f64::INFINITY),
            Err(SourceError::NonFiniteInput)
        );
    }
}
