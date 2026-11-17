// SPDX-License-Identifier: AGPL-3.0-or-later
//! Analytic known-truth stability oracle for real, continuous-time 2x2 systems.
//!
//! This crate evaluates only the eigenvalues of x_dot = A x for a real 2x2
//! matrix. It is an executable benchmark control, not a WPT converter model,
//! MIMO impedance solver, generalized-Nyquist implementation, or safety case.

use std::error::Error;
use std::fmt;

/// A real continuous-time 2x2 state matrix.
///
/// The represented system is x_dot = A x. Matrix entries use one consistent
/// time unit; the caller is responsible for documenting the physical units
/// and the linearization point when using a matrix derived from a physical
/// system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix2 {
    pub a11: f64,
    pub a12: f64,
    pub a21: f64,
    pub a22: f64,
}

impl Matrix2 {
    /// Construct a real 2x2 matrix in row-major order.
    pub const fn new(a11: f64, a12: f64, a21: f64, a22: f64) -> Self {
        Self { a11, a12, a21, a22 }
    }

    fn is_finite(self) -> bool {
        self.a11.is_finite() && self.a12.is_finite() && self.a21.is_finite() && self.a22.is_finite()
    }
}

/// One eigenvalue represented by its real and imaginary components.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Eigenvalue {
    pub real: f64,
    pub imaginary: f64,
}

/// Classification under a caller-declared distance from the imaginary axis.
///
/// StableWithinMargin means every eigenvalue has real part strictly less
/// than -stability_margin. It does not imply robust stability under
/// parametric uncertainty or bounded perturbations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StabilityClass {
    StableWithinMargin,
    Unstable,
    MarginalOrUnresolved,
}

/// Recomputed analytic result for one declared matrix and margin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StabilityReport {
    pub trace: f64,
    pub determinant: f64,
    pub discriminant: f64,
    pub eigenvalues: [Eigenvalue; 2],
    pub maximum_real_part: f64,
    pub stability_margin: f64,
    pub classification: StabilityClass,
}

/// Fail-closed input or arithmetic errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StabilityError {
    NonFiniteMatrix,
    InvalidStabilityMargin,
    ArithmeticOverflow,
}

impl fmt::Display for StabilityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteMatrix => write!(formatter, "matrix entries must be finite"),
            Self::InvalidStabilityMargin => {
                write!(
                    formatter,
                    "stability margin must be finite and strictly positive"
                )
            }
            Self::ArithmeticOverflow => {
                write!(
                    formatter,
                    "matrix invariant or eigenvalue calculation overflowed"
                )
            }
        }
    }
}

impl Error for StabilityError {}

/// Classify the eigenvalues of a real continuous-time 2x2 matrix.
///
/// For characteristic polynomial lambda^2 - trace(A) * lambda + det(A),
/// the eigenvalues are computed analytically. A case is:
///
/// - StableWithinMargin when max(Re(lambda)) < -margin;
/// - Unstable when max(Re(lambda)) > margin;
/// - MarginalOrUnresolved otherwise.
///
/// A non-finite matrix, non-finite/non-positive margin, or non-finite
/// intermediate result returns an error. The function does not assess
/// transient amplification, uncertainty, nonlinear behaviour, sampled-data
/// delay, switching dynamics, or a physical WPT network.
pub fn classify(matrix: Matrix2, stability_margin: f64) -> Result<StabilityReport, StabilityError> {
    if !matrix.is_finite() {
        return Err(StabilityError::NonFiniteMatrix);
    }
    if !stability_margin.is_finite() || stability_margin <= 0.0 {
        return Err(StabilityError::InvalidStabilityMargin);
    }

    let trace = matrix.a11 + matrix.a22;
    let determinant = matrix.a11 * matrix.a22 - matrix.a12 * matrix.a21;
    let discriminant = trace * trace - 4.0 * determinant;

    if !trace.is_finite() || !determinant.is_finite() || !discriminant.is_finite() {
        return Err(StabilityError::ArithmeticOverflow);
    }

    let eigenvalues = if discriminant >= 0.0 {
        let root = discriminant.sqrt();
        // Use the cancellation-resistant quadratic formula. Computing both
        // roots as (trace +/- root) / 2 can erase the smaller root when the
        // eigenvalues differ greatly in magnitude.
        let q = 0.5 * (trace + root.copysign(trace));
        let mut real_roots = if q == 0.0 {
            [0.0, 0.0]
        } else {
            [q, determinant / q]
        };
        real_roots.sort_by(f64::total_cmp);
        [
            Eigenvalue {
                real: real_roots[1],
                imaginary: 0.0,
            },
            Eigenvalue {
                real: real_roots[0],
                imaginary: 0.0,
            },
        ]
    } else {
        let real = trace / 2.0;
        let imaginary = (-discriminant).sqrt() / 2.0;
        [
            Eigenvalue { real, imaginary },
            Eigenvalue {
                real,
                imaginary: -imaginary,
            },
        ]
    };

    if !eigenvalues
        .iter()
        .all(|eigenvalue| eigenvalue.real.is_finite() && eigenvalue.imaginary.is_finite())
    {
        return Err(StabilityError::ArithmeticOverflow);
    }

    let maximum_real_part = eigenvalues[0].real.max(eigenvalues[1].real);
    let classification = if maximum_real_part < -stability_margin {
        StabilityClass::StableWithinMargin
    } else if maximum_real_part > stability_margin {
        StabilityClass::Unstable
    } else {
        StabilityClass::MarginalOrUnresolved
    };

    Ok(StabilityReport {
        trace,
        determinant,
        discriminant,
        eigenvalues,
        maximum_real_part,
        stability_margin,
        classification,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MARGIN: f64 = 0.05;

    #[test]
    fn uncoupled_stable_subsystems_are_classified_stable() {
        let matrix = Matrix2::new(-1.0, 0.0, 0.0, -1.0);
        let report = classify(matrix, MARGIN).unwrap();

        assert_eq!(report.classification, StabilityClass::StableWithinMargin);
        assert_eq!(report.eigenvalues[0].real, -1.0);
        assert_eq!(report.eigenvalues[1].real, -1.0);
    }

    #[test]
    fn coupling_can_make_individually_stable_subsystems_unstable() {
        // Each isolated diagonal subsystem has pole -1. Coupling both
        // directions by 3 creates the full-system eigenvalues +2 and -4.
        let coupled = Matrix2::new(-1.0, 3.0, 3.0, -1.0);
        let report = classify(coupled, MARGIN).unwrap();

        assert_eq!(report.classification, StabilityClass::Unstable);
        assert!((report.eigenvalues[0].real - 2.0).abs() < 1e-12);
        assert!((report.eigenvalues[1].real + 4.0).abs() < 1e-12);
    }

    #[test]
    fn stable_coupled_real_poles_are_classified_stable() {
        let matrix = Matrix2::new(-1.0, 0.2, 0.3, -2.0);
        let report = classify(matrix, MARGIN).unwrap();

        assert_eq!(report.classification, StabilityClass::StableWithinMargin);
        assert!(report.maximum_real_part < -MARGIN);
        assert!(
            report
                .eigenvalues
                .iter()
                .all(|value| value.imaginary == 0.0)
        );
    }

    #[test]
    fn widely_separated_real_poles_avoid_quadratic_cancellation() {
        let matrix = Matrix2::new(-1.0e16, 0.0, 0.0, -1.0);
        let report = classify(matrix, 0.05).unwrap();

        assert_eq!(report.classification, StabilityClass::StableWithinMargin);
        assert!((report.eigenvalues[0].real + 1.0).abs() < 1e-12);
        assert!((report.eigenvalues[1].real + 1.0e16).abs() < 1.0);
    }

    #[test]
    fn stable_complex_pole_pair_is_classified_stable() {
        let matrix = Matrix2::new(-1.0, -2.0, 2.0, -1.0);
        let report = classify(matrix, MARGIN).unwrap();

        assert_eq!(report.classification, StabilityClass::StableWithinMargin);
        assert!((report.eigenvalues[0].real + 1.0).abs() < 1e-12);
        assert!((report.eigenvalues[0].imaginary - 2.0).abs() < 1e-12);
        assert!((report.eigenvalues[1].imaginary + 2.0).abs() < 1e-12);
    }

    #[test]
    fn near_axis_pole_is_not_accepted_as_stable() {
        let matrix = Matrix2::new(-1.0, 0.999, 0.999, -1.0);
        let report = classify(matrix, 0.01).unwrap();

        assert_eq!(report.classification, StabilityClass::MarginalOrUnresolved);
        assert!((report.maximum_real_part + 0.001).abs() < 1e-9);
    }

    #[test]
    fn non_normal_stable_matrix_does_not_claim_transient_bound() {
        let matrix = Matrix2::new(-1.0, 100.0, 0.0, -2.0);
        let report = classify(matrix, MARGIN).unwrap();

        assert_eq!(report.classification, StabilityClass::StableWithinMargin);
        assert_eq!(report.eigenvalues[0].real, -1.0);
        assert_eq!(report.eigenvalues[1].real, -2.0);
        // Large off-diagonal coupling can cause transient amplification;
        // this eigenvalue-only oracle intentionally does not certify it.
    }

    #[test]
    fn exact_axis_pole_is_marginal_or_unresolved() {
        let matrix = Matrix2::new(0.0, 0.0, 0.0, -1.0);
        let report = classify(matrix, MARGIN).unwrap();

        assert_eq!(report.classification, StabilityClass::MarginalOrUnresolved);
        assert_eq!(report.maximum_real_part, 0.0);
    }

    #[test]
    fn stable_margin_matches_independent_shifted_routh_hurwitz_test() {
        let cases = [
            (Matrix2::new(-1.0, 0.0, 0.0, -1.0), 0.05),
            (Matrix2::new(-1.0, 0.2, 0.3, -2.0), 0.05),
            (Matrix2::new(-1.0, -2.0, 2.0, -1.0), 0.05),
            (Matrix2::new(-1.0, 3.0, 3.0, -1.0), 0.05),
            (Matrix2::new(-1.0, 0.999, 0.999, -1.0), 0.01),
            (Matrix2::new(-1.0, 100.0, 0.0, -2.0), 0.05),
            (Matrix2::new(-1.0e16, 0.0, 0.0, -1.0), 0.05),
        ];

        for (matrix, margin) in cases {
            let report = classify(matrix, margin).unwrap();
            let shifted_trace = matrix.a11 + matrix.a22 + 2.0 * margin;
            let shifted_determinant =
                (matrix.a11 + margin) * (matrix.a22 + margin) - matrix.a12 * matrix.a21;
            let independently_stable = shifted_trace < 0.0 && shifted_determinant > 0.0;

            assert_eq!(
                report.classification == StabilityClass::StableWithinMargin,
                independently_stable
            );
        }
    }

    #[test]
    fn rejects_non_finite_matrix_entries() {
        let matrix = Matrix2::new(-1.0, f64::NAN, 0.0, -1.0);
        assert_eq!(
            classify(matrix, MARGIN),
            Err(StabilityError::NonFiniteMatrix)
        );
    }

    #[test]
    fn rejects_invalid_stability_margins() {
        let matrix = Matrix2::new(-1.0, 0.0, 0.0, -1.0);
        assert_eq!(
            classify(matrix, 0.0),
            Err(StabilityError::InvalidStabilityMargin)
        );
        assert_eq!(
            classify(matrix, -0.1),
            Err(StabilityError::InvalidStabilityMargin)
        );
        assert_eq!(
            classify(matrix, f64::INFINITY),
            Err(StabilityError::InvalidStabilityMargin)
        );
    }

    #[test]
    fn rejects_intermediate_overflow() {
        let matrix = Matrix2::new(f64::MAX, f64::MAX, f64::MAX, f64::MAX);
        assert_eq!(
            classify(matrix, MARGIN),
            Err(StabilityError::ArithmeticOverflow)
        );
    }
}
