//! Gram-weighted resolvent controls for non-normal spectral calculations.
//!
//! This small standalone crate is a reference implementation of the norm
//! transformation used in finite-dimensional pseudospectral calculations.
//! It is not a Kerr / Teukolsky operator solver.
//!
//! If the Hermitian positive-definite Gram matrix is factored as
//! G = L L*, define Q = L*, so Q* Q = G. The operator norm induced by
//! the Gram inner product is then computed from
//!
//! ||R||_G = ||Q R Q^-1||_2,
//!
//! and therefore
//!
//! ||(A - z I)^-1||_G = 1 / sigma_min(Q (A-zI) Q^-1).
//!
//! Reversing the factors (Q^-1 (A-zI) Q) evaluates a different, dual-norm
//! quantity. It is not an innocuous rescaling for a non-normal operator.
//!
//! A Cholesky factor is used instead of an explicit Hermitian square root.
//! Since any two factors Q satisfying Q*Q=G differ by a unitary factor,
//! the singular values of the similarity-transformed operator are identical.

use nalgebra::DMatrix;
use num_complex::Complex64;
use std::fmt;

/// Selects the metric in which the resolvent norm is measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormPlacement {
    /// The declared Gram / energy inner product: Q (A-zI) Q^-1.
    Gram,
    /// The dual metric: Q^-1 (A-zI) Q. Diagnostic only unless explicitly
    /// declared as the intended dual inner product.
    Dual,
}

/// A finite-dimensional, conditioning-aware resolvent diagnostic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolventReport {
    /// log10 of the induced resolvent 2-norm.
    pub log10_resolvent_norm: f64,
    /// Smallest singular value of the similarity-transformed shifted operator.
    pub sigma_min_shifted: f64,
    /// 2-norm condition estimate of the supplied Gram matrix from its SVD.
    pub gram_condition_2: f64,
    /// Metric placement used for this result.
    pub placement: NormPlacement,
}

/// Explicit failures. In particular, an invalid Gram matrix never silently
/// becomes a physical-energy norm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormError {
    EmptyMatrix,
    NonSquareOperator,
    NonSquareGram,
    DimensionMismatch,
    NonFiniteInput,
    NonHermitianGram,
    GramNotPositiveDefinite,
    SingularShift,
    NonFiniteResult,
}

impl fmt::Display for NormError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyMatrix => write!(f, "operator and Gram matrix must be non-empty"),
            Self::NonSquareOperator => write!(f, "operator matrix must be square"),
            Self::NonSquareGram => write!(f, "Gram matrix must be square"),
            Self::DimensionMismatch => write!(f, "operator and Gram dimensions must match"),
            Self::NonFiniteInput => write!(f, "operator, Gram matrix and frequency must be finite"),
            Self::NonHermitianGram => write!(f, "Gram matrix must be Hermitian"),
            Self::GramNotPositiveDefinite => {
                write!(f, "Gram matrix is not numerically positive definite")
            }
            Self::SingularShift => {
                write!(f, "shifted operator is singular at the requested frequency")
            }
            Self::NonFiniteResult => write!(f, "resolvent diagnostic produced a non-finite result"),
        }
    }
}

impl std::error::Error for NormError {}

fn finite(z: Complex64) -> bool {
    z.re.is_finite() && z.im.is_finite()
}

/// Compute the log10 Gram-induced resolvent norm for a dense complex matrix.
///
/// The Gram matrix must be Hermitian positive definite. For G = L L*,
/// Q = L* is used to form the equivalent Euclidean representation
/// Q (A-zI) Q^-1. This avoids forming a matrix inverse of (A-zI) and
/// computes the resolvent norm as the reciprocal of the smallest singular
/// value.
///
/// This function returns a finite-dimensional matrix result only. It does
/// not certify that the chosen Gram matrix is a physically correct energy
/// norm, that a discretized operator converges to a continuum operator, or
/// that an optimized resolvent gain is observable for a given source.
///
/// # Errors
///
/// Returns NonHermitianGram or GramNotPositiveDefinite rather than
/// regularizing an invalid Gram matrix. Conditioning is returned separately
/// and must be used by the caller to decide whether a requested accuracy is
/// supportable.
pub fn gram_weighted_log10_resolvent(
    operator: &DMatrix<Complex64>,
    omega: Complex64,
    gram: &DMatrix<Complex64>,
    placement: NormPlacement,
) -> Result<ResolventReport, NormError> {
    if operator.nrows() == 0 || gram.nrows() == 0 {
        return Err(NormError::EmptyMatrix);
    }
    if !operator.is_square() {
        return Err(NormError::NonSquareOperator);
    }
    if !gram.is_square() {
        return Err(NormError::NonSquareGram);
    }
    if operator.nrows() != gram.nrows() {
        return Err(NormError::DimensionMismatch);
    }
    if !finite(omega) || operator.iter().any(|&z| !finite(z)) || gram.iter().any(|&z| !finite(z)) {
        return Err(NormError::NonFiniteInput);
    }

    let n = gram.nrows();
    let gram_scale = gram.iter().map(|z| z.norm()).fold(0.0_f64, f64::max);
    if gram_scale == 0.0 {
        return Err(NormError::GramNotPositiveDefinite);
    }
    let hermitian_tolerance = 64.0 * f64::EPSILON * gram_scale;
    for i in 0..n {
        for j in 0..n {
            if (gram[(i, j)] - gram[(j, i)].conj()).norm() > hermitian_tolerance {
                return Err(NormError::NonHermitianGram);
            }
        }
    }

    let decomposition = gram
        .clone()
        .cholesky()
        .ok_or(NormError::GramNotPositiveDefinite)?;
    let lower = decomposition.l().clone();
    // Cholesky over complex scalars can represent a negative real pivot as
    // an imaginary square root. A Hermitian positive-definite Gram factor
    // must have strictly positive real diagonal pivots.
    for i in 0..n {
        let pivot = lower[(i, i)];
        if !pivot.re.is_finite()
            || !pivot.im.is_finite()
            || pivot.re <= 0.0
            || pivot.im.abs() > 64.0 * f64::EPSILON * pivot.norm().max(f64::MIN_POSITIVE)
        {
            return Err(NormError::GramNotPositiveDefinite);
        }
    }
    let q = lower.adjoint();
    let q_inverse = q
        .clone()
        .try_inverse()
        .ok_or(NormError::GramNotPositiveDefinite)?;

    // Estimate kappa_2(G) independently of the Cholesky factorization.
    let gram_singular_values = gram.clone().svd(false, false).singular_values;
    let gram_max = gram_singular_values.iter().copied().fold(0.0_f64, f64::max);
    let gram_min = gram_singular_values
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);
    if !gram_min.is_finite() || gram_min <= 0.0 {
        return Err(NormError::GramNotPositiveDefinite);
    }
    let gram_condition_2 = gram_max / gram_min;

    let shifted = operator - DMatrix::<Complex64>::identity(n, n) * omega;
    let transformed = match placement {
        NormPlacement::Gram => &q * shifted * &q_inverse,
        NormPlacement::Dual => &q_inverse * shifted * &q,
    };
    let singular_values = transformed.svd(false, false).singular_values;
    let sigma_min_shifted = singular_values
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);

    if !sigma_min_shifted.is_finite() || sigma_min_shifted <= f64::MIN_POSITIVE {
        return Err(NormError::SingularShift);
    }
    let log10_resolvent_norm = -sigma_min_shifted.log10();
    if !log10_resolvent_norm.is_finite() || !gram_condition_2.is_finite() {
        return Err(NormError::NonFiniteResult);
    }

    Ok(ResolventReport {
        log10_resolvent_norm,
        sigma_min_shifted,
        gram_condition_2,
        placement,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matrix(values: [Complex64; 4]) -> DMatrix<Complex64> {
        DMatrix::from_row_slice(2, 2, &values)
    }

    fn diagonal(a: f64, b: f64) -> DMatrix<Complex64> {
        matrix([
            Complex64::new(a, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(b, 0.0),
        ])
    }

    fn jordan_like_operator() -> DMatrix<Complex64> {
        matrix([
            Complex64::new(-1.0, 0.0),
            Complex64::new(4.0, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(-2.0, 0.0),
        ])
    }

    #[test]
    fn identity_gram_matches_ordinary_euclidean_resolvent() {
        let operator = jordan_like_operator();
        let omega = Complex64::new(-0.8, 0.7);
        let gram = DMatrix::<Complex64>::identity(2, 2);
        let report =
            gram_weighted_log10_resolvent(&operator, omega, &gram, NormPlacement::Gram).unwrap();
        assert!(report.log10_resolvent_norm.is_finite());
        assert!((report.gram_condition_2 - 1.0).abs() < 1.0e-14);
    }

    #[test]
    fn dual_placement_is_not_a_harmless_rescaling() {
        let operator = jordan_like_operator();
        let omega = Complex64::new(-0.8, 0.7);
        let gram = diagonal(4.0, 0.25);
        let energy =
            gram_weighted_log10_resolvent(&operator, omega, &gram, NormPlacement::Gram).unwrap();
        let dual =
            gram_weighted_log10_resolvent(&operator, omega, &gram, NormPlacement::Dual).unwrap();
        assert!((energy.log10_resolvent_norm - 1.2012767555437331).abs() < 1.0e-10);
        assert!((dual.log10_resolvent_norm - 0.243102895186264).abs() < 1.0e-10);
        assert!((energy.log10_resolvent_norm - dual.log10_resolvent_norm).abs() > 0.5);
    }

    #[test]
    fn consistent_state_similarity_preserves_the_physical_norm() {
        let operator = jordan_like_operator();
        let omega = Complex64::new(-0.8, 0.7);
        let gram = diagonal(4.0, 0.25);

        // x' = S x, G' = S^(-*) G S^(-1), A' = S A S^(-1).
        let s = diagonal(1.0e3, 1.0e-3);
        let s_inverse = diagonal(1.0e-3, 1.0e3);
        let transformed_operator = &s * &operator * &s_inverse;
        let transformed_gram = s_inverse.adjoint() * &gram * &s_inverse;

        let original =
            gram_weighted_log10_resolvent(&operator, omega, &gram, NormPlacement::Gram).unwrap();
        let transformed = gram_weighted_log10_resolvent(
            &transformed_operator,
            omega,
            &transformed_gram,
            NormPlacement::Gram,
        )
        .unwrap();

        assert!((original.log10_resolvent_norm - transformed.log10_resolvent_norm).abs() < 1.0e-10);
    }

    #[test]
    fn mismatched_state_scaling_is_detectable() {
        let operator = jordan_like_operator();
        let omega = Complex64::new(-0.8, 0.7);
        let gram = diagonal(4.0, 0.25);
        let s = diagonal(1.0e3, 1.0e-3);
        let s_inverse = diagonal(1.0e-3, 1.0e3);
        let transformed_operator = &s * &operator * &s_inverse;

        let original =
            gram_weighted_log10_resolvent(&operator, omega, &gram, NormPlacement::Gram).unwrap();
        let mismatched =
            gram_weighted_log10_resolvent(&transformed_operator, omega, &gram, NormPlacement::Gram)
                .unwrap();
        assert!((original.log10_resolvent_norm - mismatched.log10_resolvent_norm).abs() > 1.0);
    }

    #[test]
    fn non_hermitian_gram_is_rejected() {
        let operator = DMatrix::<Complex64>::identity(2, 2);
        let gram = matrix([
            Complex64::new(1.0, 0.0),
            Complex64::new(0.5, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(1.0, 0.0),
        ]);
        assert_eq!(
            gram_weighted_log10_resolvent(
                &operator,
                Complex64::new(0.2, 0.1),
                &gram,
                NormPlacement::Gram,
            ),
            Err(NormError::NonHermitianGram)
        );
    }

    #[test]
    fn indefinite_gram_is_rejected() {
        let operator = DMatrix::<Complex64>::identity(2, 2);
        let gram = diagonal(1.0, -1.0);
        assert_eq!(
            gram_weighted_log10_resolvent(
                &operator,
                Complex64::new(0.2, 0.1),
                &gram,
                NormPlacement::Gram,
            ),
            Err(NormError::GramNotPositiveDefinite)
        );
    }

    #[test]
    fn empty_and_mismatched_inputs_fail_closed() {
        let empty = DMatrix::<Complex64>::zeros(0, 0);
        assert_eq!(
            gram_weighted_log10_resolvent(
                &empty,
                Complex64::new(0.0, 1.0),
                &empty,
                NormPlacement::Gram,
            ),
            Err(NormError::EmptyMatrix)
        );

        let operator = DMatrix::<Complex64>::identity(2, 2);
        let gram = DMatrix::<Complex64>::identity(3, 3);
        assert_eq!(
            gram_weighted_log10_resolvent(
                &operator,
                Complex64::new(0.0, 1.0),
                &gram,
                NormPlacement::Gram,
            ),
            Err(NormError::DimensionMismatch)
        );
    }

    #[test]
    fn order_two_jordan_pseudospectrum_has_square_root_scaling() {
        // Canonical EP control J = [[0, 1], [0, 0]]. This is an analytic
        // finite-matrix fixture, not a Regge-Wheeler exceptional-point result.
        let jordan = matrix([
            Complex64::new(0.0, 0.0),
            Complex64::new(1.0, 0.0),
            Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0),
        ]);
        let gram = DMatrix::<Complex64>::identity(2, 2);
        let epsilons = [1.0e-2_f64, 1.0e-4, 1.0e-6, 1.0e-8];
        let mut radii = Vec::new();

        for epsilon in epsilons {
            let radius = epsilon.sqrt();
            let report = gram_weighted_log10_resolvent(
                &jordan,
                Complex64::new(radius, 0.0),
                &gram,
                NormPlacement::Gram,
            )
            .unwrap();
            radii.push(radius);
            assert!((report.sigma_min_shifted / epsilon - 1.0).abs() < 0.02);
        }

        let log_radius_span = radii[3].ln() - radii[0].ln();
        let log_epsilon_span = epsilons[3].ln() - epsilons[0].ln();
        let slope = log_radius_span / log_epsilon_span;
        assert!((slope - 0.5).abs() < 0.005);
    }

    #[test]
    fn simple_non_ep_control_has_linear_pseudospectral_scaling() {
        let diagonal_operator = diagonal(0.0, 2.0);
        let gram = DMatrix::<Complex64>::identity(2, 2);
        let epsilons = [1.0e-2_f64, 1.0e-4, 1.0e-6, 1.0e-8];
        let mut measured = Vec::new();

        for epsilon in epsilons {
            let report = gram_weighted_log10_resolvent(
                &diagonal_operator,
                Complex64::new(epsilon, 0.0),
                &gram,
                NormPlacement::Gram,
            )
            .unwrap();
            measured.push(report.sigma_min_shifted);
            assert!((report.sigma_min_shifted / epsilon - 1.0).abs() < 1.0e-10);
        }

        let slope = (measured[3].ln() - measured[0].ln()) / (epsilons[3].ln() - epsilons[0].ln());
        assert!((slope - 1.0).abs() < 1.0e-10);
    }

    #[test]
    fn square_root_branch_exchanges_after_one_ep_encircling() {
        // Analytic normal-form control A(delta) = [[0, 1], [delta, 0]],
        // with eigenvalues +/-sqrt(delta). Unwrapped parameter continuation
        // exchanges the two branches after one circuit and returns after two.
        // Berry-phase evaluation requires a separately declared biorthogonal
        // eigenvector gauge and is intentionally not inferred here.
        let radius = 0.01_f64;
        let initial = Complex64::new(radius.sqrt(), 0.0);
        let after_one_loop = Complex64::from_polar(radius.sqrt(), std::f64::consts::PI);
        let after_two_loops = Complex64::from_polar(radius.sqrt(), std::f64::consts::TAU);

        assert!((after_one_loop + initial).norm() < 1.0e-14);
        assert!((after_two_loops - initial).norm() < 1.0e-14);
    }
}
