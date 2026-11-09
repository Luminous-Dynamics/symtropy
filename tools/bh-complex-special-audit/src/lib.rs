//! Bounded complex special-function reference kernels.
//!
//! Implements:
//! - complex Gamma via Lanczos plus the reflection formula;
//! - complex-parameter Gauss ₂F₁ in a declared set of continuation regions:
//!   direct power series, Pfaff transform, and non-resonant connection near z=1.
//!
//! This crate is a research reference kernel. It does not claim a universal
//! branch-continuation implementation, and returns typed errors outside its
//! validated regions (including the resonant connection cases c-a-b ∈ Z).

use core::fmt;
use num_complex::Complex64;

const LANCZOS_G: f64 = 7.0;
const LANCZOS_COEFFICIENTS: [f64; 9] = [
    0.999_999_999_999_809_9,
    676.520_368_121_885_1,
    -1_259.139_216_722_402_8,
    771.323_428_777_653_1,
    -176.615_029_162_140_6,
    12.507_343_278_686_905,
    -0.138_571_095_265_720_12,
    9.984_369_578_019_572e-6,
    1.505_632_735_149_311_6e-7,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialError {
    NonFiniteInput,
    GammaPole,
    HypergeometricDenominatorPole,
    ResonantContinuationUnsupported,
    BranchCutUnsupported,
    ContinuationRegionUnsupported,
    NoConvergence,
    NonFiniteResult,
    InvalidConfiguration,
    AssociatedLegendreDomainUnsupported,
    InvalidPoschlTellerParameters,
    InvalidCausalRegionParameters,
}

impl fmt::Display for SpecialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteInput => write!(f, "special-function inputs must be finite"),
            Self::GammaPole => write!(f, "gamma function has a pole at this input"),
            Self::HypergeometricDenominatorPole => {
                write!(
                    f,
                    "hypergeometric denominator parameter is a non-positive integer"
                )
            }
            Self::ResonantContinuationUnsupported => {
                write!(f, "connection formula is resonant when c-a-b is an integer")
            }
            Self::BranchCutUnsupported => {
                write!(
                    f,
                    "the requested point lies on the principal ₂F₁ branch cut [1, infinity)"
                )
            }
            Self::ContinuationRegionUnsupported => {
                write!(
                    f,
                    "input is outside the explicitly supported ₂F₁ continuation regions"
                )
            }
            Self::NoConvergence => {
                write!(f, "hypergeometric series did not meet its error criterion")
            }
            Self::NonFiniteResult => write!(
                f,
                "special-function calculation produced a non-finite result"
            ),
            Self::InvalidConfiguration => write!(f, "series configuration is invalid"),
            Self::AssociatedLegendreDomainUnsupported => {
                write!(
                    f,
                    "Ferrers associated-Legendre control requires real z strictly between -1 and 1"
                )
            }
            Self::InvalidPoschlTellerParameters => {
                write!(
                    f,
                    "Pöschl–Teller Jost parameters require finite omega, b>0 and lambda>=0"
                )
            }
            Self::InvalidCausalRegionParameters => {
                write!(f, "Pöschl–Teller causal-region parameters are invalid")
            }
        }
    }
}

impl std::error::Error for SpecialError {}

fn finite(value: Complex64) -> bool {
    value.re.is_finite() && value.im.is_finite()
}

fn is_real_nonpositive_integer(value: Complex64) -> bool {
    value.im == 0.0 && value.re <= 0.0 && value.re.fract() == 0.0
}

fn is_near_integer(value: Complex64, tolerance: f64) -> bool {
    value.im.abs() <= tolerance && (value.re - value.re.round()).abs() <= tolerance
}

fn validate_complex(values: &[Complex64]) -> Result<(), SpecialError> {
    if values.iter().all(|value| finite(*value)) {
        Ok(())
    } else {
        Err(SpecialError::NonFiniteInput)
    }
}

/// Complex Gamma function using a nine-coefficient Lanczos approximation.
///
/// Reflection is used for Re(z) < 1/2. At exact non-positive integers the
/// function returns GammaPole rather than a NaN/Inf sentinel.
pub fn gamma(z: Complex64) -> Result<Complex64, SpecialError> {
    validate_complex(&[z])?;
    if is_real_nonpositive_integer(z) {
        return Err(SpecialError::GammaPole);
    }

    let result = if z.re < 0.5 {
        let pi_z = Complex64::new(core::f64::consts::PI, 0.0) * z;
        let sine = pi_z.sin();
        let reflected = gamma(Complex64::new(1.0, 0.0) - z)?;
        Complex64::new(core::f64::consts::PI, 0.0) / (sine * reflected)
    } else {
        lanczos_gamma(z)
    };

    if finite(result) {
        Ok(result)
    } else {
        Err(SpecialError::NonFiniteResult)
    }
}

fn lanczos_gamma(z: Complex64) -> Complex64 {
    let shifted = z - Complex64::new(1.0, 0.0);
    let mut sum = Complex64::new(LANCZOS_COEFFICIENTS[0], 0.0);
    for (index, coefficient) in LANCZOS_COEFFICIENTS.iter().enumerate().skip(1) {
        sum += Complex64::new(*coefficient, 0.0) / (shifted + Complex64::new(index as f64, 0.0));
    }

    let t = shifted + Complex64::new(LANCZOS_G + 0.5, 0.0);
    let log_gamma = Complex64::new(0.5 * (2.0 * core::f64::consts::PI).ln(), 0.0)
        + (shifted + Complex64::new(0.5, 0.0)) * t.ln()
        - t
        + sum.ln();
    log_gamma.exp()
}

fn reciprocal_gamma(z: Complex64) -> Result<Complex64, SpecialError> {
    if is_real_nonpositive_integer(z) {
        return Ok(Complex64::new(0.0, 0.0));
    }
    Ok(Complex64::new(1.0, 0.0) / gamma(z)?)
}

/// Configuration for bounded power-series evaluations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeriesConfig {
    pub max_terms: usize,
    pub absolute_tolerance: f64,
    pub relative_tolerance: f64,
}

impl Default for SeriesConfig {
    fn default() -> Self {
        Self {
            max_terms: 100_000,
            absolute_tolerance: 2.0e-15,
            relative_tolerance: 2.0e-14,
        }
    }
}

impl SeriesConfig {
    fn validate(self) -> Result<(), SpecialError> {
        if self.max_terms == 0
            || !self.absolute_tolerance.is_finite()
            || !self.relative_tolerance.is_finite()
            || self.absolute_tolerance <= 0.0
            || self.relative_tolerance <= 0.0
        {
            return Err(SpecialError::InvalidConfiguration);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvaluationMethod {
    TerminatingPolynomial,
    DirectSeries,
    PfaffContinuation,
    ConnectionNearOne,
}

/// Result of one complex-parameter Gauss hypergeometric evaluation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HypergeometricEvaluation {
    pub value: Complex64,
    pub method: EvaluationMethod,
    pub terms: usize,
    /// Geometric majorant for the series truncation tail when available.
    /// This does not include floating-point roundoff and is not an interval-certified total error.
    pub estimated_absolute_error: f64,
}

/// Return the degree when a or b is an exact non-positive integer.
fn terminating_degree(value: Complex64) -> Option<usize> {
    if is_real_nonpositive_integer(value) {
        Some((-value.re) as usize)
    } else {
        None
    }
}

fn validate_hypergeometric_denominator(c: Complex64) -> Result<(), SpecialError> {
    if is_real_nonpositive_integer(c) {
        Err(SpecialError::HypergeometricDenominatorPole)
    } else {
        Ok(())
    }
}

/// Evaluate the defining series for |z| < 1, or a terminating polynomial.
///
/// Uses the recurrence
/// term_n = term_(n-1) * (a+n-1)(b+n-1) z / ((c+n-1)n).
/// Convergence is accepted only after a global geometric ratio majorant for
/// the remaining series terms falls below the configured tolerance. The
/// resulting tail bound excludes floating-point roundoff.
pub fn hyp2f1_series(
    a: Complex64,
    b: Complex64,
    c: Complex64,
    z: Complex64,
    config: SeriesConfig,
) -> Result<HypergeometricEvaluation, SpecialError> {
    config.validate()?;
    validate_complex(&[a, b, c, z])?;
    validate_hypergeometric_denominator(c)?;

    let degree = terminating_degree(a).or_else(|| terminating_degree(b));
    if z.norm() >= 1.0 && degree.is_none() {
        return Err(SpecialError::ContinuationRegionUnsupported);
    }

    if degree == Some(0) || z == Complex64::new(0.0, 0.0) {
        return Ok(HypergeometricEvaluation {
            value: Complex64::new(1.0, 0.0),
            method: if degree.is_some() {
                EvaluationMethod::TerminatingPolynomial
            } else {
                EvaluationMethod::DirectSeries
            },
            terms: 1,
            estimated_absolute_error: 0.0,
        });
    }

    let mut term = Complex64::new(1.0, 0.0);
    let mut sum = term;
    let mut consecutive_small = 0_usize;

    for n in 1..=config.max_terms {
        let n_minus_one = (n - 1) as f64;
        let denominator_parameter = c + Complex64::new(n_minus_one, 0.0);
        if denominator_parameter == Complex64::new(0.0, 0.0) {
            return Err(SpecialError::HypergeometricDenominatorPole);
        }

        term *= (a + Complex64::new(n_minus_one, 0.0)) * (b + Complex64::new(n_minus_one, 0.0)) * z
            / (denominator_parameter * n as f64);
        sum += term;

        if !finite(term) || !finite(sum) {
            return Err(SpecialError::NonFiniteResult);
        }

        if degree.is_some_and(|max_degree| n >= max_degree) {
            return Ok(HypergeometricEvaluation {
                value: sum,
                method: EvaluationMethod::TerminatingPolynomial,
                terms: n + 1,
                estimated_absolute_error: 0.0,
            });
        }

        // For every future k >= n, triangle inequalities give
        // |a+k| <= k+|a|, |b+k| <= k+|b|, and |c+k| >= k-|c|.
        // Thus, once n>|c|, every subsequent term ratio is bounded by this
        // uniform majorant. Unlike a local-ratio extrapolation, this controls
        // the whole remaining tail in exact arithmetic.
        let n_radius = n as f64;
        let c_radius = c.norm();
        let ratio_bound = if n_radius > c_radius {
            z.norm() * (1.0 + a.norm() / n_radius) * (1.0 + b.norm() / n_radius)
                / (1.0 - c_radius / n_radius)
                * (1.0 + 64.0 * f64::EPSILON)
        } else {
            f64::INFINITY
        };
        let estimated_tail = if ratio_bound < 1.0 {
            term.norm() * ratio_bound / (1.0 - ratio_bound)
        } else {
            f64::INFINITY
        };
        let tolerance = config.absolute_tolerance + config.relative_tolerance * sum.norm();

        if n >= 4 && estimated_tail <= tolerance {
            consecutive_small += 1;
        } else {
            consecutive_small = 0;
        }
        if consecutive_small >= 3 {
            return Ok(HypergeometricEvaluation {
                value: sum,
                method: EvaluationMethod::DirectSeries,
                terms: n + 1,
                estimated_absolute_error: estimated_tail,
            });
        }
    }

    Err(SpecialError::NoConvergence)
}

fn on_principal_branch_cut(z: Complex64) -> bool {
    z.im == 0.0 && z.re >= 1.0
}

fn near_resonant_exponent(delta: Complex64) -> bool {
    is_near_integer(delta, 1.0e-10)
}

fn connection_near_one(
    a: Complex64,
    b: Complex64,
    c: Complex64,
    z: Complex64,
    config: SeriesConfig,
) -> Result<HypergeometricEvaluation, SpecialError> {
    let delta = c - a - b;
    if near_resonant_exponent(delta) {
        return Err(SpecialError::ResonantContinuationUnsupported);
    }

    let one_minus_z = Complex64::new(1.0, 0.0) - z;
    let series_one = hyp2f1_series(a, b, Complex64::new(1.0, 0.0) - delta, one_minus_z, config)?;
    let series_two = hyp2f1_series(
        c - a,
        c - b,
        Complex64::new(1.0, 0.0) + delta,
        one_minus_z,
        config,
    )?;

    let gamma_c = gamma(c)?;
    let coefficient_one =
        gamma_c * gamma(delta)? * reciprocal_gamma(c - a)? * reciprocal_gamma(c - b)?;
    let coefficient_two = gamma_c * gamma(-delta)? * reciprocal_gamma(a)? * reciprocal_gamma(b)?;
    let power = (delta * one_minus_z.ln()).exp();

    let value = coefficient_one * series_one.value + coefficient_two * power * series_two.value;
    if !finite(value) {
        return Err(SpecialError::NonFiniteResult);
    }

    let error = coefficient_one.norm() * series_one.estimated_absolute_error
        + (coefficient_two * power).norm() * series_two.estimated_absolute_error;

    Ok(HypergeometricEvaluation {
        value,
        method: EvaluationMethod::ConnectionNearOne,
        terms: series_one.terms + series_two.terms,
        estimated_absolute_error: error,
    })
}

fn pfaff_continuation(
    a: Complex64,
    b: Complex64,
    c: Complex64,
    z: Complex64,
    config: SeriesConfig,
) -> Result<HypergeometricEvaluation, SpecialError> {
    let one_minus_z = Complex64::new(1.0, 0.0) - z;
    let transformed_z = z / (z - Complex64::new(1.0, 0.0));
    let inner = hyp2f1_series(a, c - b, c, transformed_z, config)?;
    let prefactor = (-a * one_minus_z.ln()).exp();
    let value = prefactor * inner.value;

    if !finite(value) {
        return Err(SpecialError::NonFiniteResult);
    }

    Ok(HypergeometricEvaluation {
        value,
        method: EvaluationMethod::PfaffContinuation,
        terms: inner.terms,
        estimated_absolute_error: prefactor.norm() * inner.estimated_absolute_error,
    })
}

/// Evaluate ₂F₁(a,b;c;z) for complex a,b,c and complex z.
///
/// Supported regions:
/// - direct series / terminating polynomial when |z| < 1;
/// - connection formula when |1-z| < 0.5 and c-a-b is non-resonant;
/// - direct-series fallback for resonant near-one points that remain inside
///   the unit disk, using a global geometric ratio majorant for the tail;
/// - Pfaff transform when |z/(z-1)| < 1 and the transformed series converges.
///
/// The principal branch cut [1, infinity) is rejected. If the resonant
/// fallback cannot meet its tail criterion within max_terms, it returns
/// NoConvergence rather than silently accepting an unqualified value.
pub fn hyp2f1(
    a: Complex64,
    b: Complex64,
    c: Complex64,
    z: Complex64,
    config: SeriesConfig,
) -> Result<HypergeometricEvaluation, SpecialError> {
    config.validate()?;
    validate_complex(&[a, b, c, z])?;
    validate_hypergeometric_denominator(c)?;

    if terminating_degree(a).is_some() || terminating_degree(b).is_some() {
        return hyp2f1_series(a, b, c, z, config);
    }

    if on_principal_branch_cut(z) {
        return Err(SpecialError::BranchCutUnsupported);
    }

    if z.norm() < 0.85 {
        return hyp2f1_series(a, b, c, z, config);
    }

    if (Complex64::new(1.0, 0.0) - z).norm() < 0.5 {
        return match connection_near_one(a, b, c, z, config) {
            Err(SpecialError::ResonantContinuationUnsupported) if z.norm() < 1.0 => {
                hyp2f1_series(a, b, c, z, config)
            }
            result => result,
        };
    }

    let transformed_z = z / (z - Complex64::new(1.0, 0.0));
    if transformed_z.norm() < 0.85 {
        return pfaff_continuation(a, b, c, z, config);
    }

    Err(SpecialError::ContinuationRegionUnsupported)
}

/// Value and first two derivatives of the Ferrers associated-Legendre function.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AssociatedLegendreEvaluation {
    pub value: Complex64,
    pub first_derivative: Complex64,
    pub second_derivative: Complex64,
    pub value_method: EvaluationMethod,
    pub first_derivative_method: EvaluationMethod,
    pub second_derivative_method: EvaluationMethod,
}

/// Evaluate the Ferrers function P_degree^order(z) and its first two
/// derivatives with respect to the real argument z in (-1,1).
///
/// Uses
///
/// P_l^mu(z) = exp(mu atanh(z)) / Gamma(1-mu)
///             * 2F1(-l, l+1; 1-mu; (1-z)/2).
///
/// This interval is the physical tanh(x/b) coordinate for the real
/// Pöschl–Teller tortoise line. Complex degree and order are supported.
/// Integer/resonant cases that make the underlying 2F1 denominator singular,
/// or require logarithmic continuation, are returned as typed errors.
///
/// Derivatives are computed from contiguous 2F1 identities, not by
/// differentiating the ODE. This allows an independent ODE residual check.
pub fn associated_legendre_ferrers(
    degree: Complex64,
    order: Complex64,
    z: Complex64,
    config: SeriesConfig,
) -> Result<AssociatedLegendreEvaluation, SpecialError> {
    config.validate()?;
    validate_complex(&[degree, order, z])?;
    if z.im != 0.0 || z.re <= -1.0 || z.re >= 1.0 {
        return Err(SpecialError::AssociatedLegendreDomainUnsupported);
    }

    let one = Complex64::new(1.0, 0.0);
    let two = Complex64::new(2.0, 0.0);
    let a = -degree;
    let b = degree + one;
    let c = one - order;
    let argument = (one - z) / two;

    let f0 = hyp2f1(a, b, c, argument, config)?;
    let f1 = hyp2f1(a + one, b + one, c + one, argument, config)?;
    let f2 = hyp2f1(a + two, b + two, c + two, argument, config)?;

    let first_factor = a * b / c;
    let second_factor = a * (a + one) * b * (b + one) / (c * (c + one));

    let q = order / (one - z * z);
    let q_prime = two * order * z / ((one - z * z) * (one - z * z));
    let prefactor = (order * z.re.atanh()).exp() * reciprocal_gamma(c)?;

    let value = prefactor * f0.value;
    let first_derivative = prefactor * (q * f0.value - 0.5 * first_factor * f1.value);
    let second_derivative = prefactor
        * ((q * q + q_prime) * f0.value - q * first_factor * f1.value
            + 0.25 * second_factor * f2.value);

    if !finite(value) || !finite(first_derivative) || !finite(second_derivative) {
        return Err(SpecialError::NonFiniteResult);
    }

    Ok(AssociatedLegendreEvaluation {
        value,
        first_derivative,
        second_derivative,
        value_method: f0.method,
        first_derivative_method: f1.method,
        second_derivative_method: f2.method,
    })
}

/// Direction of the outgoing Jost boundary condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JostSide {
    /// Unit outgoing wave exp(-i omega x) as x -> -infinity.
    Left,
    /// Unit outgoing wave exp(+i omega x) as x -> +infinity.
    Right,
}

/// Unit-normalized Pöschl–Teller Jost solution and x derivatives.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoschlTellerJostEvaluation {
    pub value: Complex64,
    pub first_x_derivative: Complex64,
    pub second_x_derivative: Complex64,
    pub degree: Complex64,
    pub order: Complex64,
}

/// Evaluate one outgoing Pöschl–Teller Jost solution for
///
///     u''(x) + [omega² - lambda/b² sech²(x/b)] u(x) = 0.
///
/// With z=tanh(x/b), degree l=-1/2+sqrt(1/4-lambda), and order mu=i b omega,
/// the right-outgoing solution is Gamma(1-mu) P_l^mu(z), while the
/// left-outgoing solution is Gamma(1-mu) P_l^mu(-z).
///
/// The unit normalization follows from P_l^mu(z) ~ exp(mu atanh(z)) /
/// Gamma(1-mu) as z -> +1. This is a homogeneous Jost solution only; it does
/// not compute the scattering matrix, Green function, Wronskian residues,
/// or contour prescription.
pub fn poschl_teller_jost(
    width: f64,
    lambda: f64,
    omega: Complex64,
    x: f64,
    side: JostSide,
    config: SeriesConfig,
) -> Result<PoschlTellerJostEvaluation, SpecialError> {
    config.validate()?;
    validate_complex(&[omega])?;
    if !width.is_finite() || !lambda.is_finite() || !x.is_finite() || width <= 0.0 || lambda < 0.0 {
        return Err(SpecialError::InvalidPoschlTellerParameters);
    }

    let z = (x / width).tanh();
    let side_sign = match side {
        JostSide::Right => 1.0,
        JostSide::Left => -1.0,
    };
    let argument = Complex64::new(side_sign * z, 0.0);
    if argument.re <= -1.0 || argument.re >= 1.0 {
        return Err(SpecialError::AssociatedLegendreDomainUnsupported);
    }

    let degree = Complex64::new(-0.5, 0.0) + Complex64::new(0.25 - lambda, 0.0).sqrt();
    let order = Complex64::new(0.0, 1.0) * width * omega;
    let legendre = associated_legendre_ferrers(degree, order, argument, config)?;
    let normalization = gamma(Complex64::new(1.0, 0.0) - order)?;

    let sech_squared = 1.0 - z * z;
    let argument_x_derivative = side_sign * sech_squared / width;
    let argument_x_second_derivative = -2.0 * side_sign * z * sech_squared / (width * width);

    let value = normalization * legendre.value;
    let first_x_derivative = normalization * legendre.first_derivative * argument_x_derivative;
    let second_x_derivative = normalization
        * (legendre.second_derivative * argument_x_derivative.powi(2)
            + legendre.first_derivative * argument_x_second_derivative);

    if !finite(value) || !finite(first_x_derivative) || !finite(second_x_derivative) {
        return Err(SpecialError::NonFiniteResult);
    }

    Ok(PoschlTellerJostEvaluation {
        value,
        first_x_derivative,
        second_x_derivative,
        degree,
        order,
    })
}

/// Compute W[f_left, f_right] at one tortoise coordinate using unit-normalized
/// Pöschl–Teller Jost solutions.
pub fn poschl_teller_jost_wronskian(
    width: f64,
    lambda: f64,
    omega: Complex64,
    x: f64,
    config: SeriesConfig,
) -> Result<Complex64, SpecialError> {
    let left = poschl_teller_jost(width, lambda, omega, x, JostSide::Left, config)?;
    let right = poschl_teller_jost(width, lambda, omega, x, JostSide::Right, config)?;
    let value = left.value * right.first_x_derivative - left.first_x_derivative * right.value;
    if finite(value) {
        Ok(value)
    } else {
        Err(SpecialError::NonFiniteResult)
    }
}

/// Evaluate the analytic connection-formula Wronskian for the unit-normalized
/// Pöschl–Teller left/right Jost pair.
///
/// With ell = -1/2 + sqrt(1/4-lambda) and mu = i*width*omega,
///
/// W[f_left,f_right] = -(2/width) * Gamma(1-mu)^2
///                     / (Gamma(1-mu+ell) Gamma(-ell-mu)).
///
/// This connection coefficient is independent of tortoise position and is
/// intentionally separate from the direct Jost/Wronskian evaluation above.
/// It provides an analytic cross-check and a pole locator. Points where the
/// Gamma representation itself is singular outside its removable domain
/// return a typed error rather than being regularized heuristically.
pub fn poschl_teller_jost_wronskian_connection(
    width: f64,
    lambda: f64,
    omega: Complex64,
) -> Result<Complex64, SpecialError> {
    validate_complex(&[omega])?;
    if !width.is_finite() || !lambda.is_finite() || width <= 0.0 || lambda < 0.0 {
        return Err(SpecialError::InvalidPoschlTellerParameters);
    }

    let one = Complex64::new(1.0, 0.0);
    let degree = Complex64::new(-0.5, 0.0) + Complex64::new(0.25 - lambda, 0.0).sqrt();
    let order = Complex64::new(0.0, 1.0) * width * omega;
    let gamma_factor = gamma(one - order)?;
    let inverse_left_connection = reciprocal_gamma(one - order + degree)?;
    let inverse_right_connection = reciprocal_gamma(-degree - order)?;
    let value = (-2.0 / width)
        * gamma_factor
        * gamma_factor
        * inverse_left_connection
        * inverse_right_connection;

    if finite(value) {
        Ok(value)
    } else {
        Err(SpecialError::NonFiniteResult)
    }
}

/// Frequency-domain retarded Green-kernel evaluation for the time-domain
/// equation (d_t^2 - d_x^2 + V) Psi = S with inverse Fourier factor e^{-i omega t}.
///
/// The homogeneous radial operator is L_omega = d_x^2 + omega^2 - V.
/// The stated time-domain PDE transforms to -L_omega Psi_hat = S_hat, so
/// this kernel includes the required overall minus sign.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoschlTellerRetardedGreenEvaluation {
    /// Green kernel of the Fourier-transformed time-domain PDE.
    pub value: Complex64,
    /// Direct W[f_left, f_right] = f_left f_right' - f_left' f_right.
    pub wronskian: Complex64,
    /// Independent analytic connection-formula Wronskian.
    pub connection_wronskian: Complex64,
    /// Left Jost value evaluated at min(observer_x, source_x).
    pub left_jost_value: Complex64,
    /// Right Jost value evaluated at max(observer_x, source_x).
    pub right_jost_value: Complex64,
}

/// Evaluate the outgoing frequency-domain Green kernel for
///
///     (d_t^2 - d_x^2 + V(x)) G_R(t,x;x') = delta(t) delta(x-x')
///
/// under the e^{-i omega t} convention, with
/// V(x) = lambda / width^2 * sech^2(x/width).
///
/// The returned value is -f_left(x_<) f_right(x_>) / W. It therefore
/// inverts -L_omega, not L_omega, and has the source derivative jump -1.
/// The retarded Fourier transform is selected by analyticity for Im(omega)>0;
/// evaluations below the real axis are its meromorphic continuation for
/// resonance calculations, not themselves a retarded time-domain transform.
/// The Jost functions use unit asymptotic normalizations. A small finite
/// Wronskian is exposed rather than hidden behind an arbitrary pole threshold.
pub fn poschl_teller_retarded_green(
    width: f64,
    lambda: f64,
    omega: Complex64,
    observer_x: f64,
    source_x: f64,
    config: SeriesConfig,
) -> Result<PoschlTellerRetardedGreenEvaluation, SpecialError> {
    if !observer_x.is_finite() || !source_x.is_finite() {
        return Err(SpecialError::InvalidPoschlTellerParameters);
    }

    let x_left = observer_x.min(source_x);
    let x_right = observer_x.max(source_x);
    let left = poschl_teller_jost(width, lambda, omega, x_left, JostSide::Left, config)?;
    let right = poschl_teller_jost(width, lambda, omega, x_right, JostSide::Right, config)?;
    let wronskian = poschl_teller_jost_wronskian(width, lambda, omega, 0.0, config)?;
    let connection_wronskian = poschl_teller_jost_wronskian_connection(width, lambda, omega)?;
    if !finite(wronskian) || wronskian == Complex64::new(0.0, 0.0) || !finite(connection_wronskian)
    {
        return Err(SpecialError::NonFiniteResult);
    }
    let value = -(left.value * right.value) / wronskian;

    if !finite(value) {
        return Err(SpecialError::NonFiniteResult);
    }

    Ok(PoschlTellerRetardedGreenEvaluation {
        value,
        wronskian,
        connection_wronskian,
        left_jost_value: left.value,
        right_jost_value: right.value,
    })
}

/// Region in which a Pöschl–Teller retarded response has a qualified
/// mode-sum representation under the Arnaudo–Carballo–Withers split.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoschlTellerModeSumRegion {
    /// Both split frequency-space terms close in the lower half-plane;
    /// their Matsubara residues cancel and the QNM sum converges.
    LateQnm,
    /// The QNM-bearing term closes in the upper half-plane and the other in
    /// the lower half-plane; the response is represented by Matsubara modes.
    EarlyMatsubara,
    /// Both terms close in the upper half-plane; the retarded Green function
    /// vanishes by causality.
    CausallyZero,
    /// The point lies within the caller-declared uncertainty band around a
    /// direct or potential-reflected light cone. No series is selected.
    TransitionUnresolved,
}

/// Dimensionless geometry and signed distances to the two Pöschl–Teller
/// light-cone boundaries. Positive margins are on the post-front side.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoschlTellerModeSumRegionEvaluation {
    pub region: PoschlTellerModeSumRegion,
    pub dimensionless_time: f64,
    pub direct_lightcone_margin: f64,
    pub reflected_lightcone_margin: f64,
    pub boundary_tolerance: f64,
}

/// Classify the causal mode-sum region for
/// V(x)=lambda/width^2 * sech^2(x/width), with its peak at x=0.
///
/// This is the exact Pöschl–Teller region geometry in
/// Arnaudo, Carballo & Withers, Phys. Rev. D 114, 024025 (2026),
/// DOI 10.1103/1bcl-q79x. In dimensionless coordinates
///
///     T = (t-t')/width, X = observer_x/width, X' = source_x/width,
///
/// the direct front is T=|X-X'| and the front scattered from the potential
/// peak is T=|X+X'|. Region I is QNM-only when both margins are positive;
/// Region II is Matsubara-only when the direct margin is positive and the
/// reflected margin is negative; Region III is causally zero before the
/// direct front. A positive boundary tolerance creates an explicit abstention
/// band instead of making a numerically fragile choice at either front.
///
/// The formula is a geometric domain selector, not a convergence certificate.
/// The selected expansion must still establish its own residue-sum remainder
/// or return unresolved.
pub fn classify_poschl_teller_mode_sum_region(
    width: f64,
    lambda: f64,
    delta_time: f64,
    observer_x: f64,
    source_x: f64,
    boundary_tolerance: f64,
) -> Result<PoschlTellerModeSumRegionEvaluation, SpecialError> {
    if !width.is_finite()
        || !lambda.is_finite()
        || !delta_time.is_finite()
        || !observer_x.is_finite()
        || !source_x.is_finite()
        || width <= 0.0
        || lambda <= 0.0
        || !boundary_tolerance.is_finite()
        || boundary_tolerance <= 0.0
    {
        return Err(SpecialError::InvalidCausalRegionParameters);
    }

    let dimensionless_time = delta_time / width;
    let observer = observer_x / width;
    let source = source_x / width;
    let direct_lightcone_margin = dimensionless_time - (observer - source).abs();
    let reflected_lightcone_margin = dimensionless_time - (observer + source).abs();

    let region = if direct_lightcone_margin.abs() <= boundary_tolerance
        || reflected_lightcone_margin.abs() <= boundary_tolerance
    {
        PoschlTellerModeSumRegion::TransitionUnresolved
    } else if direct_lightcone_margin > 0.0 && reflected_lightcone_margin > 0.0 {
        PoschlTellerModeSumRegion::LateQnm
    } else if direct_lightcone_margin > 0.0 && reflected_lightcone_margin < 0.0 {
        PoschlTellerModeSumRegion::EarlyMatsubara
    } else {
        PoschlTellerModeSumRegion::CausallyZero
    };

    Ok(PoschlTellerModeSumRegionEvaluation {
        region,
        dimensionless_time,
        direct_lightcone_margin,
        reflected_lightcone_margin,
        boundary_tolerance,
    })
}

/// Asymptotic residue-ratio limits for the region-appropriate Pöschl–Teller
/// mode sum. These are ratio-test limits, not finite-truncation tail bounds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoschlTellerModeSumAsymptoticRatios {
    pub region: PoschlTellerModeSumRegion,
    /// Present only in the late-time QNM region:
    /// exp(-T + |X| + |X'|).
    pub qnm_ratio_limit: Option<f64>,
    /// Present only in the early-time Matsubara region:
    /// exp(T - |X| - |X'|).
    pub matsubara_positive_ratio_limit: Option<f64>,
    /// Present only in the early-time Matsubara region:
    /// exp(-T - |X| - |X'|).
    pub matsubara_negative_ratio_limit: Option<f64>,
}

/// Return the asymptotic ratio-test limits derived for the Pöschl–Teller
/// residue sums in Arnaudo, Carballo & Withers, PRD 114, 024025 (2026),
/// Eqs. (31), (35) and (36). The values are asymptotic successive-term
/// ratios; they establish a ratio-test convergence region when below one,
/// but do not certify a finite-N remainder or account for floating-point
/// cancellation. Those must be measured separately.
pub fn poschl_teller_mode_sum_asymptotic_ratios(
    width: f64,
    lambda: f64,
    delta_time: f64,
    observer_x: f64,
    source_x: f64,
    boundary_tolerance: f64,
) -> Result<PoschlTellerModeSumAsymptoticRatios, SpecialError> {
    let geometry = classify_poschl_teller_mode_sum_region(
        width,
        lambda,
        delta_time,
        observer_x,
        source_x,
        boundary_tolerance,
    )?;
    let dimensionless_sum = (observer_x / width).abs() + (source_x / width).abs();

    let (qnm_ratio_limit, matsubara_positive_ratio_limit, matsubara_negative_ratio_limit) =
        match geometry.region {
            PoschlTellerModeSumRegion::LateQnm => (
                Some((-geometry.dimensionless_time + dimensionless_sum).exp()),
                None,
                None,
            ),
            PoschlTellerModeSumRegion::EarlyMatsubara => (
                None,
                Some((geometry.dimensionless_time - dimensionless_sum).exp()),
                Some((-geometry.dimensionless_time - dimensionless_sum).exp()),
            ),
            PoschlTellerModeSumRegion::CausallyZero
            | PoschlTellerModeSumRegion::TransitionUnresolved => (None, None, None),
        };

    Ok(PoschlTellerModeSumAsymptoticRatios {
        region: geometry.region,
        qnm_ratio_limit,
        matsubara_positive_ratio_limit,
        matsubara_negative_ratio_limit,
    })
}

/// Winding-number result for the analytic Pöschl–Teller connection Wronskian.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoschlTellerWindingEvaluation {
    pub winding_number: i32,
    /// Number of contour intervals in the final accepted refinement.
    pub samples: usize,
    /// Largest adjacent sampled phase increment on the final contour.
    pub max_phase_step: f64,
    /// Number of nested contour refinements performed after the requested grid.
    pub refinements: usize,
    /// Number of consecutive equal winding estimates at the end of refinement.
    pub stable_refinements: usize,
}

/// Sample the analytic connection Wronskian along a circular contour and
/// estimate its argument-principle winding from adjacent phase increments.
fn sample_poschl_teller_wronskian_winding(
    width: f64,
    lambda: f64,
    center: Complex64,
    radius: f64,
    samples: usize,
) -> Result<(i32, f64), SpecialError> {
    let mut previous: Option<Complex64> = None;
    let mut total_phase = 0.0;
    let mut max_phase_step: f64 = 0.0;

    for index in 0..=samples {
        let angle = core::f64::consts::TAU * index as f64 / samples as f64;
        let point = center + Complex64::new(radius * angle.cos(), radius * angle.sin());
        let value = poschl_teller_jost_wronskian_connection(width, lambda, point)?;
        if !finite(value) || value == Complex64::new(0.0, 0.0) {
            return Err(SpecialError::NoConvergence);
        }

        if let Some(previous_value) = previous {
            let phase_step = (value * previous_value.conj()).arg();
            max_phase_step = max_phase_step.max(phase_step.abs());
            // Keep each sampled increment comfortably away from the branch
            // ambiguity at +/-pi. Refinement below is still required.
            if phase_step.abs() >= core::f64::consts::FRAC_PI_4 {
                return Err(SpecialError::NoConvergence);
            }
            total_phase += phase_step;
        }
        previous = Some(value);
    }

    let turns = total_phase / core::f64::consts::TAU;
    let rounded = turns.round();
    if !turns.is_finite() || (turns - rounded).abs() > 0.02 {
        return Err(SpecialError::NoConvergence);
    }
    Ok((rounded as i32, max_phase_step))
}

/// Evaluate the argument-principle winding number of W around a circular
/// frequency contour using nested trapezoidal phase sampling.
///
/// The requested grid is doubled until the winding estimate is stable across
/// three consecutive grids and the final adjacent phase increment is below
/// pi/8. A sampled zero, phase jump, winding disagreement or exhausted
/// refinement budget returns NoConvergence.
///
/// This is a numerical refinement qualification, not a rigorous interval
/// proof that the phase curve cannot oscillate between samples. It counts
/// zeros minus poles of the meromorphic connection Wronskian; it is not
/// automatically the number of Green-function poles if W has poles inside
/// the contour.
pub fn poschl_teller_wronskian_winding_number(
    width: f64,
    lambda: f64,
    center: Complex64,
    radius: f64,
    samples: usize,
) -> Result<PoschlTellerWindingEvaluation, SpecialError> {
    validate_complex(&[center])?;
    if !width.is_finite()
        || !lambda.is_finite()
        || !radius.is_finite()
        || radius <= 0.0
        || width <= 0.0
        || lambda < 0.0
        || samples < 16
    {
        return Err(SpecialError::InvalidConfiguration);
    }

    let mut current_samples = samples;
    let mut previous_winding: Option<i32> = None;
    let mut stable_refinements = 0_usize;
    let mut refinements = 0_usize;

    for _ in 0..8 {
        let (winding_number, max_phase_step) =
            sample_poschl_teller_wronskian_winding(width, lambda, center, radius, current_samples)?;
        if max_phase_step >= core::f64::consts::FRAC_PI_8 {
            stable_refinements = 0;
        } else if previous_winding == Some(winding_number) {
            stable_refinements += 1;
        } else {
            stable_refinements = 0;
        }

        if stable_refinements >= 2 {
            return Ok(PoschlTellerWindingEvaluation {
                winding_number,
                samples: current_samples,
                max_phase_step,
                refinements,
                stable_refinements: stable_refinements + 1,
            });
        }

        previous_winding = Some(winding_number);
        current_samples = current_samples
            .checked_mul(2)
            .ok_or(SpecialError::InvalidConfiguration)?;
        refinements += 1;
    }

    Err(SpecialError::NoConvergence)
}

/// Inputs that define a Green-function frequency contour and its source/observer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoschlTellerGreenContourConfig {
    pub radius: f64,
    pub observer_x: f64,
    pub source_x: f64,
    pub samples: usize,
}

const GREEN_LAURENT_QUADRATURE_TOLERANCE: f64 = 2.0e-9;
const MAX_GREEN_LAURENT_REFINEMENTS: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq)]
struct GreenLaurentGridEvaluation {
    c_minus_one: Complex64,
    c_minus_two: Complex64,
    max_relative_wronskian_disagreement: f64,
}

/// Evaluate both Laurent contour integrals on one periodic trapezoidal grid.
fn sample_retarded_green_laurent_grid(
    width: f64,
    lambda: f64,
    center: Complex64,
    radius: f64,
    samples: usize,
    observer_x: f64,
    source_x: f64,
    config: SeriesConfig,
) -> Result<GreenLaurentGridEvaluation, SpecialError> {
    let mut c_minus_one = Complex64::new(0.0, 0.0);
    let mut c_minus_two = Complex64::new(0.0, 0.0);
    let mut max_relative_wronskian_disagreement: f64 = 0.0;

    for index in 0..samples {
        let angle = core::f64::consts::TAU * index as f64 / samples as f64;
        let displacement = Complex64::new(radius * angle.cos(), radius * angle.sin());
        let omega = center + displacement;
        let evaluation =
            poschl_teller_retarded_green(width, lambda, omega, observer_x, source_x, config)?;
        let denominator = evaluation
            .wronskian
            .norm()
            .max(evaluation.connection_wronskian.norm());
        if !denominator.is_finite() || denominator == 0.0 {
            return Err(SpecialError::NoConvergence);
        }
        let disagreement =
            (evaluation.wronskian - evaluation.connection_wronskian).norm() / denominator;
        max_relative_wronskian_disagreement = max_relative_wronskian_disagreement.max(disagreement);
        c_minus_one += displacement * evaluation.value;
        c_minus_two += displacement * displacement * evaluation.value;
    }

    c_minus_one /= samples as f64;
    c_minus_two /= samples as f64;
    if !finite(c_minus_one)
        || !finite(c_minus_two)
        || !max_relative_wronskian_disagreement.is_finite()
    {
        return Err(SpecialError::NonFiniteResult);
    }

    Ok(GreenLaurentGridEvaluation {
        c_minus_one,
        c_minus_two,
        max_relative_wronskian_disagreement,
    })
}

/// Laurent coefficients of the retarded Green kernel about a circularly
/// enclosed pole cluster. The periodic trapezoidal identities are
///
///   C_-1 = (1/2πi) ∮ G(ω) dω
///   C_-2 = (1/2πi) ∮ (ω-center) G(ω) dω.
///
/// For a simple pole C_-1 is the residue and C_-2 should vanish. For a
/// second-order pole C_-2 is the leading Laurent coefficient. The Wronskian
/// winding and the Laurent quadrature are refined independently: winding
/// stability does not certify the coefficient integrals. The reported
/// coefficient discrepancy is an a-posteriori nested-grid diagnostic, not
/// an interval-arithmetic remainder proof.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoschlTellerGreenLaurentEvaluation {
    pub center: Complex64,
    pub radius: f64,
    /// Final accepted contour grid for the Laurent integrals.
    pub samples: usize,
    pub winding_number: i32,
    pub max_phase_step: f64,
    pub c_minus_one: Complex64,
    pub c_minus_two: Complex64,
    pub max_relative_wronskian_disagreement: f64,
    /// Number of contour-grid doublings used after the initial Laurent grid.
    pub quadrature_refinements: usize,
    /// Consecutive nested-grid comparisons satisfying the coefficient tolerance.
    pub stable_quadrature_refinements: usize,
    /// Maximum scaled difference between the last two Laurent coefficient grids.
    pub scaled_quadrature_difference: f64,
}

pub fn poschl_teller_retarded_green_laurent(
    width: f64,
    lambda: f64,
    center: Complex64,
    contour: PoschlTellerGreenContourConfig,
    config: SeriesConfig,
) -> Result<PoschlTellerGreenLaurentEvaluation, SpecialError> {
    if !contour.observer_x.is_finite() || !contour.source_x.is_finite() {
        return Err(SpecialError::InvalidPoschlTellerParameters);
    }
    let winding = poschl_teller_wronskian_winding_number(
        width,
        lambda,
        center,
        contour.radius,
        contour.samples,
    )?;

    // Winding and Laurent integrations both start from the refined winding
    // grid, but the coefficient quadrature has its own nested-grid acceptance.
    let mut samples = winding.samples.max(contour.samples);
    let mut previous = sample_retarded_green_laurent_grid(
        width,
        lambda,
        center,
        contour.radius,
        samples,
        contour.observer_x,
        contour.source_x,
        config,
    )?;
    let mut stable_quadrature_refinements = 0_usize;
    let mut scaled_quadrature_difference = f64::INFINITY;

    for quadrature_refinements in 1..=MAX_GREEN_LAURENT_REFINEMENTS {
        samples = samples
            .checked_mul(2)
            .ok_or(SpecialError::InvalidConfiguration)?;
        let current = sample_retarded_green_laurent_grid(
            width,
            lambda,
            center,
            contour.radius,
            samples,
            contour.observer_x,
            contour.source_x,
            config,
        )?;
        let c1_scale = current.c_minus_one.norm().max(1.0);
        let c2_scale = current.c_minus_two.norm().max(1.0);
        scaled_quadrature_difference = ((current.c_minus_one - previous.c_minus_one).norm()
            / c1_scale)
            .max((current.c_minus_two - previous.c_minus_two).norm() / c2_scale);

        if scaled_quadrature_difference <= GREEN_LAURENT_QUADRATURE_TOLERANCE {
            stable_quadrature_refinements += 1;
        } else {
            stable_quadrature_refinements = 0;
        }

        if stable_quadrature_refinements >= 2 {
            return Ok(PoschlTellerGreenLaurentEvaluation {
                center,
                radius: contour.radius,
                samples,
                winding_number: winding.winding_number,
                max_phase_step: winding.max_phase_step,
                c_minus_one: current.c_minus_one,
                c_minus_two: current.c_minus_two,
                max_relative_wronskian_disagreement: current.max_relative_wronskian_disagreement,
                quadrature_refinements,
                stable_quadrature_refinements,
                scaled_quadrature_difference,
            });
        }

        previous = current;
    }

    Err(SpecialError::NoConvergence)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn z(re: f64, im: f64) -> Complex64 {
        Complex64::new(re, im)
    }

    fn close(actual: Complex64, expected: Complex64, tolerance: f64) {
        assert!(
            (actual - expected).norm() <= tolerance,
            "actual={actual:?}, expected={expected:?}, tolerance={tolerance:e}"
        );
    }

    fn integrate_jost_rk4(
        width: f64,
        lambda: f64,
        omega: Complex64,
        target_x: f64,
        side: JostSide,
        max_step: f64,
    ) -> (Complex64, Complex64) {
        let (initial_x, direction) = match side {
            JostSide::Left => (-18.0, -1.0),
            JostSide::Right => (18.0, 1.0),
        };
        let imaginary_direction = Complex64::new(0.0, direction);
        let mut value = (imaginary_direction * omega * initial_x).exp();
        let mut derivative = imaginary_direction * omega * value;
        let steps = (((target_x - initial_x).abs() / max_step).ceil() as usize).max(1);
        let step = (target_x - initial_x) / steps as f64;
        let potential = |x: f64| lambda / (width * width) / (x / width).cosh().powi(2);
        let rhs = |x: f64, y: Complex64, dy: Complex64| (dy, (potential(x) - omega * omega) * y);

        for index in 0..steps {
            let x = initial_x + index as f64 * step;
            let (k1_y, k1_dy) = rhs(x, value, derivative);
            let (k2_y, k2_dy) = rhs(
                x + step / 2.0,
                value + step * k1_y / 2.0,
                derivative + step * k1_dy / 2.0,
            );
            let (k3_y, k3_dy) = rhs(
                x + step / 2.0,
                value + step * k2_y / 2.0,
                derivative + step * k2_dy / 2.0,
            );
            let (k4_y, k4_dy) = rhs(x + step, value + step * k3_y, derivative + step * k3_dy);
            value += step * (k1_y + 2.0 * k2_y + 2.0 * k3_y + k4_y) / 6.0;
            derivative += step * (k1_dy + 2.0 * k2_dy + 2.0 * k3_dy + k4_dy) / 6.0;
        }

        (value, derivative)
    }

    #[test]
    fn gamma_matches_independent_80_digit_fixture_one() {
        let actual = gamma(z(0.3, 1.2)).unwrap();
        let expected = z(0.107_075_474_962_553_64, -0.353_143_987_729_088_6);
        close(actual, expected, 2.0e-14);
    }

    #[test]
    fn gamma_reflection_path_matches_independent_80_digit_fixture() {
        let actual = gamma(z(-1.2, 0.7)).unwrap();
        let expected = z(0.174_095_916_181_767_3, 0.706_909_450_271_453_4);
        close(actual, expected, 3.0e-14);
    }

    #[test]
    fn gamma_recurrence_and_conjugation_identities_hold() {
        let value = z(0.4, 1.1);
        let lhs = gamma(value + z(1.0, 0.0)).unwrap();
        let rhs = value * gamma(value).unwrap();
        close(lhs, rhs, 2.0e-13);
        close(
            gamma(value.conj()).unwrap(),
            gamma(value).unwrap().conj(),
            2.0e-13,
        );
    }

    #[test]
    fn gamma_reflection_identity_holds() {
        let value = z(0.37, 0.82);
        let lhs = gamma(value).unwrap() * gamma(z(1.0, 0.0) - value).unwrap();
        let rhs = z(core::f64::consts::PI, 0.0) / (z(core::f64::consts::PI, 0.0) * value).sin();
        close(lhs, rhs, 2.0e-13);
    }

    #[test]
    fn gamma_poles_and_nonfinite_inputs_fail_closed() {
        assert_eq!(gamma(z(0.0, 0.0)), Err(SpecialError::GammaPole));
        assert_eq!(gamma(z(-3.0, 0.0)), Err(SpecialError::GammaPole));
        assert_eq!(gamma(z(f64::NAN, 0.0)), Err(SpecialError::NonFiniteInput));
    }

    #[test]
    fn hyp2f1_direct_series_matches_independent_80_digit_fixture() {
        let result = hyp2f1(
            z(0.7, 0.3),
            z(1.2, -0.4),
            z(2.1, 0.2),
            z(0.4, 0.1),
            SeriesConfig::default(),
        )
        .unwrap();
        assert_eq!(result.method, EvaluationMethod::DirectSeries);
        close(
            result.value,
            z(1.243_404_961_188_053_7, 0.075_688_576_668_042_22),
            5.0e-13,
        );
    }

    #[test]
    fn hyp2f1_near_one_connection_matches_independent_80_digit_fixture() {
        let result = hyp2f1(
            z(0.6, 0.2),
            z(1.3, -0.1),
            z(2.1, 0.3),
            z(0.92, 0.03),
            SeriesConfig::default(),
        )
        .unwrap();
        assert_eq!(result.method, EvaluationMethod::ConnectionNearOne);
        close(
            result.value,
            z(2.092_272_092_930_979_4, 0.204_304_434_046_581_6),
            2.0e-12,
        );
    }

    #[test]
    fn hyp2f1_pfaff_continuation_matches_independent_80_digit_fixture() {
        let result = hyp2f1(
            z(0.4, 0.2),
            z(1.1, -0.3),
            z(2.2, 0.1),
            z(-1.5, 0.2),
            SeriesConfig::default(),
        )
        .unwrap();
        assert_eq!(result.method, EvaluationMethod::PfaffContinuation);
        close(
            result.value,
            z(0.784_859_038_431_843_2, -0.016_248_502_085_529_596),
            2.0e-12,
        );
    }

    #[test]
    fn hyp2f1_complex_resonant_case_matches_independent_100_digit_fixture() {
        let result = hyp2f1(
            z(0.6, 0.2),
            z(0.8, -0.1),
            z(2.4, 0.1),
            z(0.95, 0.0),
            SeriesConfig::default(),
        )
        .unwrap();
        assert_eq!(result.method, EvaluationMethod::DirectSeries);
        close(
            result.value,
            z(1.417_466_195_157_772_6, 0.079_463_109_250_766_79),
            5.0e-13,
        );
        assert!(result.estimated_absolute_error < 1.0e-12);
    }

    #[test]
    fn hyp2f1_resonant_fallback_abstains_when_term_budget_is_insufficient() {
        let config = SeriesConfig {
            max_terms: 16,
            absolute_tolerance: 2.0e-15,
            relative_tolerance: 2.0e-14,
        };
        assert_eq!(
            hyp2f1(z(0.6, 0.2), z(0.8, -0.1), z(2.4, 0.1), z(0.95, 0.0), config,),
            Err(SpecialError::NoConvergence)
        );
    }

    #[test]
    fn hyp2f1_is_symmetric_in_a_and_b() {
        let (a, b, c, z) = (z(0.8, 0.1), z(1.2, -0.2), z(2.3, 0.4), z(0.72, 0.08));
        let forward = hyp2f1(a, b, c, z, SeriesConfig::default()).unwrap();
        let reverse = hyp2f1(b, a, c, z, SeriesConfig::default()).unwrap();
        close(forward.value, reverse.value, 2.0e-13);
    }

    #[test]
    fn terminating_series_is_exact_polynomial_even_outside_unit_disk() {
        let result = hyp2f1(
            z(-3.0, 0.0),
            z(0.7, 0.2),
            z(1.4, -0.1),
            z(1.3, 0.0),
            SeriesConfig::default(),
        )
        .unwrap();
        assert_eq!(result.method, EvaluationMethod::TerminatingPolynomial);
        assert!(finite(result.value));
    }

    #[test]
    fn branch_cut_is_rejected_and_resonance_uses_direct_series_fallback() {
        assert_eq!(
            hyp2f1(
                z(0.5, 0.0),
                z(0.7, 0.0),
                z(1.4, 0.0),
                z(1.2, 0.0),
                SeriesConfig::default()
            ),
            Err(SpecialError::BranchCutUnsupported)
        );

        let result = hyp2f1(
            z(0.6, 0.0),
            z(0.8, 0.0),
            z(2.4, 0.0),
            z(0.95, 0.0),
            SeriesConfig::default(),
        )
        .unwrap();
        assert_eq!(result.method, EvaluationMethod::DirectSeries);
        close(result.value, z(1.397_999_809_791_163_2, 0.0), 2.0e-13);
        assert!(result.estimated_absolute_error < 1.0e-12);
    }

    #[test]
    fn denominator_poles_and_unsupported_continuation_fail_closed() {
        assert_eq!(
            hyp2f1(
                z(0.5, 0.0),
                z(0.7, 0.0),
                z(-2.0, 0.0),
                z(0.2, 0.0),
                SeriesConfig::default()
            ),
            Err(SpecialError::HypergeometricDenominatorPole)
        );
        assert_eq!(
            hyp2f1(
                z(0.4, 0.2),
                z(1.1, -0.3),
                z(2.2, 0.1),
                z(0.5, 1.0),
                SeriesConfig::default()
            ),
            Err(SpecialError::ContinuationRegionUnsupported)
        );
    }

    #[test]
    fn derivative_identity_matches_central_difference() {
        let a = z(0.7, 0.2);
        let b = z(1.1, -0.3);
        let c = z(2.0, 0.1);
        let point = z(0.35, 0.08);
        let h = 1.0e-5;
        let upper = hyp2f1(a, b, c, point + z(h, 0.0), SeriesConfig::default())
            .unwrap()
            .value;
        let lower = hyp2f1(a, b, c, point - z(h, 0.0), SeriesConfig::default())
            .unwrap()
            .value;
        let numeric = (upper - lower) / (2.0 * h);
        let analytic = a * b / c
            * hyp2f1(
                a + z(1.0, 0.0),
                b + z(1.0, 0.0),
                c + z(1.0, 0.0),
                point,
                SeriesConfig::default(),
            )
            .unwrap()
            .value;
        close(numeric, analytic, 2.0e-8);
    }

    #[test]
    fn associated_legendre_reduces_to_exact_legendre_polynomial() {
        let y = 0.31;
        let result = associated_legendre_ferrers(
            z(2.0, 0.0),
            z(0.0, 0.0),
            z(y, 0.0),
            SeriesConfig::default(),
        )
        .unwrap();
        close(result.value, z((3.0 * y * y - 1.0) / 2.0, 0.0), 2.0e-14);
        close(result.first_derivative, z(3.0 * y, 0.0), 2.0e-13);
        close(result.second_derivative, z(3.0, 0.0), 2.0e-12);
    }

    #[test]
    fn associated_legendre_satisfies_ode_in_direct_and_near_one_regions() {
        let degree = z(-0.5, 0.7);
        let order = z(0.35, -0.2);
        for y in [0.23, -0.9] {
            let result =
                associated_legendre_ferrers(degree, order, z(y, 0.0), SeriesConfig::default())
                    .unwrap();
            let one_minus_y2 = 1.0 - y * y;
            let residual = one_minus_y2 * result.second_derivative
                - 2.0 * y * result.first_derivative
                + (degree * (degree + z(1.0, 0.0)) - order * order / one_minus_y2) * result.value;
            assert!(residual.norm() < 3.0e-10, "z={y}, residual={residual:?}");
        }
    }

    #[test]
    fn associated_legendre_first_derivative_matches_central_difference() {
        let degree = z(-0.5, 0.7);
        let order = z(0.35, -0.2);
        let y = 0.23;
        let h = 1.0e-5;
        let base =
            associated_legendre_ferrers(degree, order, z(y, 0.0), SeriesConfig::default()).unwrap();
        let upper =
            associated_legendre_ferrers(degree, order, z(y + h, 0.0), SeriesConfig::default())
                .unwrap()
                .value;
        let lower =
            associated_legendre_ferrers(degree, order, z(y - h, 0.0), SeriesConfig::default())
                .unwrap()
                .value;
        close(base.first_derivative, (upper - lower) / (2.0 * h), 2.0e-8);
    }

    #[test]
    fn associated_legendre_rejects_endpoints_and_complex_argument() {
        for argument in [z(-1.0, 0.0), z(1.0, 0.0), z(0.2, 0.01)] {
            assert_eq!(
                associated_legendre_ferrers(
                    z(-0.5, 0.7),
                    z(0.35, -0.2),
                    argument,
                    SeriesConfig::default(),
                ),
                Err(SpecialError::AssociatedLegendreDomainUnsupported)
            );
        }
    }

    #[test]
    fn free_potential_jost_solutions_are_exact_plane_waves() {
        let omega = z(0.9, -0.12);
        let x = 0.73;
        let config = SeriesConfig::default();
        let left = poschl_teller_jost(1.4, 0.0, omega, x, JostSide::Left, config).unwrap();
        let right = poschl_teller_jost(1.4, 0.0, omega, x, JostSide::Right, config).unwrap();
        close(left.value, (-z(0.0, 1.0) * omega * x).exp(), 2.0e-13);
        close(right.value, (z(0.0, 1.0) * omega * x).exp(), 2.0e-13);
        close(
            left.first_x_derivative,
            -z(0.0, 1.0) * omega * left.value,
            2.0e-13,
        );
        close(
            right.first_x_derivative,
            z(0.0, 1.0) * omega * right.value,
            2.0e-13,
        );
        close(
            left.value * right.first_x_derivative - left.first_x_derivative * right.value,
            z(0.0, 2.0) * omega,
            5.0e-13,
        );
    }

    #[test]
    fn barrier_jost_solutions_satisfy_the_potential_ode() {
        let width = 1.3;
        let lambda = 0.75;
        let omega = z(0.9, -0.12);
        for x in [-1.4, -0.2, 0.8, 1.5] {
            for side in [JostSide::Left, JostSide::Right] {
                let solution =
                    poschl_teller_jost(width, lambda, omega, x, side, SeriesConfig::default())
                        .unwrap();
                let sech_squared = 1.0 - (x / width).tanh().powi(2);
                let residual = solution.second_x_derivative
                    + (omega * omega - lambda * sech_squared / (width * width)) * solution.value;
                assert!(
                    residual.norm() < 2.0e-9,
                    "x={x}, side={side:?}, residual={residual:?}"
                );
            }
        }
    }

    #[test]
    fn barrier_jost_wronskian_is_position_independent() {
        let width = 1.0;
        let lambda = 0.75;
        let omega = z(0.9, -0.12);
        let config = SeriesConfig::default();
        let reference = poschl_teller_jost_wronskian(width, lambda, omega, -1.2, config).unwrap();
        for x in [-0.7, 0.0, 0.65, 1.1] {
            let value = poschl_teller_jost_wronskian(width, lambda, omega, x, config).unwrap();
            close(value, reference, 2.0e-9);
        }
    }

    #[test]
    fn jost_parameters_and_saturated_coordinates_fail_closed() {
        let config = SeriesConfig::default();
        assert_eq!(
            poschl_teller_jost(0.0, 0.75, z(0.9, -0.1), 0.0, JostSide::Right, config),
            Err(SpecialError::InvalidPoschlTellerParameters)
        );
        assert_eq!(
            poschl_teller_jost(1.0, -0.1, z(0.9, -0.1), 0.0, JostSide::Right, config),
            Err(SpecialError::InvalidPoschlTellerParameters)
        );
        assert_eq!(
            poschl_teller_jost(1.0, 0.75, z(0.9, -0.1), 30.0, JostSide::Right, config),
            Err(SpecialError::AssociatedLegendreDomainUnsupported)
        );
    }

    #[test]
    fn invalid_series_configuration_is_rejected() {
        let config = SeriesConfig {
            max_terms: 0,
            ..SeriesConfig::default()
        };
        assert_eq!(
            hyp2f1(z(0.5, 0.0), z(0.7, 0.0), z(1.4, 0.0), z(0.2, 0.0), config),
            Err(SpecialError::InvalidConfiguration)
        );
    }
    #[test]
    fn jost_matsubara_resonance_uses_bounded_direct_series_fallback() {
        let config = SeriesConfig::default();
        for n in [0_u32, 1, 2] {
            let omega = z(0.0, f64::from(n));
            for x in [1.0, 2.0, 3.0] {
                assert!(
                    poschl_teller_jost(1.0, 0.75, omega, x, JostSide::Left, config).is_ok(),
                    "left Jost Matsubara evaluation should converge at x={x}, n={n}"
                );
                assert!(
                    poschl_teller_jost(1.0, 0.75, omega, -x, JostSide::Right, config).is_ok(),
                    "right Jost Matsubara evaluation should converge at x={x}, n={n}"
                );
            }
        }
    }

    #[test]
    fn connection_wronskian_matches_direct_jost_wronskian() {
        let config = SeriesConfig::default();
        let cases = [
            (1.3, 0.75, z(0.9, -0.12)),
            (0.8, 0.1, z(0.47, 0.19)),
            (1.7, 1.4, z(1.1, 0.08)),
            (1.0, 0.75, z(0.0, 0.0)),
        ];
        for (width, lambda, omega) in cases {
            let analytic = poschl_teller_jost_wronskian_connection(width, lambda, omega).unwrap();
            for x in [-1.1, -0.2, 0.0, 0.65, 1.3] {
                let direct = poschl_teller_jost_wronskian(width, lambda, omega, x, config).unwrap();
                close(direct, analytic, 2.0e-9 * (1.0 + analytic.norm()));
            }
        }
    }

    #[test]
    fn connection_wronskian_free_limit_is_two_i_omega() {
        for (width, omega) in [(0.7, z(0.9, -0.12)), (1.3, z(0.0, 0.0)), (2.1, z(0.3, 0.2))] {
            let actual = poschl_teller_jost_wronskian_connection(width, 0.0, omega).unwrap();
            close(actual, z(0.0, 2.0) * omega, 5.0e-13);
        }
    }

    #[test]
    fn connection_wronskian_reproduces_principal_root_poles_on_both_sides_of_threshold() {
        let width = 1.0;
        for (lambda, expected_plus_sign) in [(0.75, 1.0), (0.1, 0.0)] {
            for n in [0_u32, 1, 2, 5] {
                let damping = f64::from(n) + 0.5;
                let split = Complex64::new(lambda - 0.25, 0.0).sqrt();
                let plus = z(0.0, -damping) + split;
                let minus = z(0.0, -damping) - split;
                let plus_w = poschl_teller_jost_wronskian_connection(width, lambda, plus).unwrap();
                let minus_w =
                    poschl_teller_jost_wronskian_connection(width, lambda, minus).unwrap();
                assert!(
                    plus_w.norm() < 2.0e-10,
                    "plus branch lambda={lambda}, n={n}, W={plus_w:?}"
                );
                assert!(
                    minus_w.norm() < 2.0e-10,
                    "minus branch lambda={lambda}, n={n}, W={minus_w:?}"
                );

                if expected_plus_sign == 1.0 {
                    assert!(plus.re > 0.0 && minus.re < 0.0);
                } else {
                    assert!(plus.re == 0.0 && minus.re == 0.0);
                    assert!(plus.im > minus.im, "plus branch must be less damped");
                }
            }
        }
    }

    #[test]
    fn threshold_coalescence_is_a_second_order_wronskian_zero() {
        let lambda = 0.25;
        let n = 0_u32;
        let pole = z(0.0, -(f64::from(n) + 0.5));
        let exact = poschl_teller_jost_wronskian_connection(1.0, lambda, pole).unwrap();
        close(exact, z(0.0, 0.0), 0.0);

        let h = 1.0e-3;
        let at_h = poschl_teller_jost_wronskian_connection(1.0, lambda, pole + z(h, 0.0)).unwrap();
        let at_half =
            poschl_teller_jost_wronskian_connection(1.0, lambda, pole + z(h / 2.0, 0.0)).unwrap();
        let scaled_h = at_h.norm() / h.powi(2);
        let scaled_half = at_half.norm() / (h / 2.0).powi(2);
        assert!(scaled_h.is_finite() && scaled_half.is_finite());
        assert!(
            (scaled_h - scaled_half).abs() / scaled_h < 2.0e-3,
            "scaled second-order zero did not stabilize: h={scaled_h:e}, h/2={scaled_half:e}"
        );
        assert!(
            (at_h.norm() / at_half.norm() - 4.0).abs() < 8.0e-3,
            "threshold local scaling is not quadratic"
        );
    }

    #[test]
    fn connection_wronskian_rejects_invalid_parameters() {
        assert_eq!(
            poschl_teller_jost_wronskian_connection(0.0, 0.75, z(0.9, -0.1)),
            Err(SpecialError::InvalidPoschlTellerParameters)
        );
        assert_eq!(
            poschl_teller_jost_wronskian_connection(1.0, -0.1, z(0.9, -0.1)),
            Err(SpecialError::InvalidPoschlTellerParameters)
        );
        assert_eq!(
            poschl_teller_jost_wronskian_connection(1.0, 0.75, z(f64::NAN, 0.0)),
            Err(SpecialError::NonFiniteInput)
        );
    }
    #[test]
    fn retarded_green_free_limit_matches_outgoing_kernel() {
        let omega = z(0.9, -0.12);
        let config = SeriesConfig::default();
        for (observer_x, source_x) in [(-0.8, 0.3), (0.0, 0.0), (1.1, -0.4)] {
            let actual =
                poschl_teller_retarded_green(1.3, 0.0, omega, observer_x, source_x, config)
                    .unwrap();
            let distance = (observer_x - source_x).abs();
            let expected =
                z(0.0, 1.0) / (z(2.0, 0.0) * omega) * (z(0.0, 1.0) * omega * distance).exp();
            close(actual.value, expected, 5.0e-13);
        }
    }

    #[test]
    fn retarded_green_has_the_signed_source_derivative_jump() {
        let width = 1.3;
        let lambda = 0.75;
        let omega = z(0.9, -0.12);
        let source_x = 0.27;
        let config = SeriesConfig::default();
        let left =
            poschl_teller_jost(width, lambda, omega, source_x, JostSide::Left, config).unwrap();
        let right =
            poschl_teller_jost(width, lambda, omega, source_x, JostSide::Right, config).unwrap();
        let wronskian = poschl_teller_jost_wronskian(width, lambda, omega, 0.0, config).unwrap();

        let derivative_from_left = -(left.first_x_derivative * right.value) / wronskian;
        let derivative_from_right = -(left.value * right.first_x_derivative) / wronskian;

        close(
            derivative_from_right - derivative_from_left,
            z(-1.0, 0.0),
            2.0e-12,
        );

        let at_source =
            poschl_teller_retarded_green(width, lambda, omega, source_x, source_x, config).unwrap();
        let epsilon = 1.0e-7;
        let from_left = poschl_teller_retarded_green(
            width,
            lambda,
            omega,
            source_x - epsilon,
            source_x,
            config,
        )
        .unwrap();
        let from_right = poschl_teller_retarded_green(
            width,
            lambda,
            omega,
            source_x + epsilon,
            source_x,
            config,
        )
        .unwrap();
        assert!(
            (from_left.value - at_source.value).norm() < 2.0e-7,
            "left-limit continuity residual: {:?}",
            from_left.value - at_source.value
        );
        assert!(
            (from_right.value - at_source.value).norm() < 2.0e-7,
            "right-limit continuity residual: {:?}",
            from_right.value - at_source.value
        );
    }

    #[test]
    fn retarded_green_is_reciprocal_for_the_real_scalar_barrier() {
        let config = SeriesConfig::default();
        let omega = z(1.1, 0.08);
        let forward = poschl_teller_retarded_green(1.0, 0.75, omega, 0.8, -0.4, config).unwrap();
        let reverse = poschl_teller_retarded_green(1.0, 0.75, omega, -0.4, 0.8, config).unwrap();
        close(forward.value, reverse.value, 2.0e-12);
        close(forward.wronskian, reverse.wronskian, 2.0e-12);
    }

    #[test]
    fn retarded_green_exposes_the_wronskian_and_source_sign() {
        let config = SeriesConfig::default();
        let omega = z(0.9, -0.12);
        let observer_x = 0.73;
        let source_x = -0.21;
        let actual =
            poschl_teller_retarded_green(1.4, 0.0, omega, observer_x, source_x, config).unwrap();
        close(actual.wronskian, z(0.0, 2.0) * omega, 5.0e-13);

        let radial_operator_inverse =
            actual.left_jost_value * actual.right_jost_value / actual.wronskian;
        close(actual.value, -radial_operator_inverse, 1.0e-14);
        let reversed_wronskian_value =
            -(actual.left_jost_value * actual.right_jost_value) / -actual.wronskian;
        close(reversed_wronskian_value, -actual.value, 1.0e-14);
    }

    #[test]
    fn retarded_green_rejects_nonfinite_coordinates_and_invalid_width() {
        let config = SeriesConfig::default();
        let omega = z(0.9, -0.12);
        assert_eq!(
            poschl_teller_retarded_green(1.0, 0.75, omega, f64::NAN, 0.0, config,),
            Err(SpecialError::InvalidPoschlTellerParameters)
        );
        assert_eq!(
            poschl_teller_retarded_green(0.0, 0.75, omega, 0.2, 0.0, config,),
            Err(SpecialError::InvalidPoschlTellerParameters)
        );
    }

    #[test]
    fn retarded_green_matches_independent_100_digit_jost_fixture() {
        // mpmath 1.3.0, 100 decimal digits:
        // width=1.3, lambda=0.75, omega=0.9-0.12i,
        // left Jost at x=-0.21, right Jost at x=0.73.
        let config = SeriesConfig::default();
        let omega = z(0.9, -0.12);
        let left = poschl_teller_jost(1.3, 0.75, omega, -0.21, JostSide::Left, config).unwrap();
        let right = poschl_teller_jost(1.3, 0.75, omega, 0.73, JostSide::Right, config).unwrap();
        close(
            left.value,
            z(1.077_691_453_714_667_6, 0.475_573_043_669_987_6),
            5.0e-11,
        );
        close(
            right.value,
            z(0.843_603_430_198_184_2, 0.826_004_257_644_994_8),
            5.0e-11,
        );

        let green = poschl_teller_retarded_green(1.3, 0.75, omega, 0.73, -0.21, config).unwrap();
        close(
            green.wronskian,
            z(-0.924_255_829_850_433_7, 1.347_984_031_935_372_3),
            5.0e-11,
        );
        close(
            green.value,
            z(-0.473_003_612_861_573_6, 0.707_349_028_100_136_9),
            5.0e-11,
        );
    }

    #[test]
    fn retarded_green_obeys_real_potential_frequency_conjugation_symmetry() {
        let config = SeriesConfig::default();
        let omega = z(0.83, 0.17);
        let conjugate_frequency = -omega.conj();
        let value = poschl_teller_retarded_green(1.0, 0.75, omega, -0.8, 0.7, config).unwrap();
        let conjugate =
            poschl_teller_retarded_green(1.0, 0.75, conjugate_frequency, -0.8, 0.7, config)
                .unwrap();

        close(conjugate.value, value.value.conj(), 5.0e-11);
        close(conjugate.wronskian, value.wronskian.conj(), 5.0e-11);
    }

    #[test]
    fn independent_rk4_jost_integration_reproduces_green_fixture() {
        let width = 1.3;
        let lambda = 0.75;
        let omega = z(0.9, -0.12);
        let observer_x = -0.21;
        let source_x = 0.73;
        let expected_left = z(1.077_691_453_714_667_6, 0.475_573_043_669_987_6);
        let expected_right = z(0.843_603_430_198_184_2, 0.826_004_257_644_994_8);
        let expected_wronskian = z(-0.924_255_829_850_433_7, 1.347_984_031_935_372_3);
        let expected_green = z(-0.473_003_612_861_573_6, 0.707_349_028_100_136_9);

        let mut previous_left_error = f64::INFINITY;
        let mut previous_right_error = f64::INFINITY;
        for max_step in [0.02, 0.01, 0.005] {
            let left =
                integrate_jost_rk4(width, lambda, omega, observer_x, JostSide::Left, max_step);
            let right =
                integrate_jost_rk4(width, lambda, omega, source_x, JostSide::Right, max_step);
            let left_at_center =
                integrate_jost_rk4(width, lambda, omega, 0.0, JostSide::Left, max_step);
            let right_at_center =
                integrate_jost_rk4(width, lambda, omega, 0.0, JostSide::Right, max_step);
            let wronskian =
                left_at_center.0 * right_at_center.1 - left_at_center.1 * right_at_center.0;
            let green = -(left.0 * right.0) / wronskian;

            let left_error = (left.0 - expected_left).norm();
            let right_error = (right.0 - expected_right).norm();
            assert!(left_error < previous_left_error);
            assert!(right_error < previous_right_error);
            assert!(left_error < 3.0e-8, "left ODE error={left_error:e}");
            assert!(right_error < 3.0e-8, "right ODE error={right_error:e}");
            close(wronskian, expected_wronskian, 1.0e-7);
            close(green, expected_green, 1.0e-8);
            previous_left_error = left_error;
            previous_right_error = right_error;
        }

        let w_left = integrate_jost_rk4(width, lambda, omega, -0.5, JostSide::Left, 0.01);
        let w_right = integrate_jost_rk4(width, lambda, omega, -0.5, JostSide::Right, 0.01);
        let w_at_minus_half = w_left.0 * w_right.1 - w_left.1 * w_right.0;
        let w_left = integrate_jost_rk4(width, lambda, omega, 0.65, JostSide::Left, 0.01);
        let w_right = integrate_jost_rk4(width, lambda, omega, 0.65, JostSide::Right, 0.01);
        let w_at_plus_point = w_left.0 * w_right.1 - w_left.1 * w_right.0;
        close(w_at_minus_half, w_at_plus_point, 1.0e-9);
    }

    #[test]
    fn source_observer_interventions_change_response_not_homogeneous_wronskian() {
        let config = SeriesConfig::default();
        let omega = z(0.83, 0.17);
        let first = poschl_teller_retarded_green(1.0, 0.75, omega, -0.8, 0.7, config).unwrap();
        let second = poschl_teller_retarded_green(1.0, 0.75, omega, 0.9, -0.15, config).unwrap();

        close(first.wronskian, second.wronskian, 2.0e-13);
        assert!(
            (first.value - second.value).norm() > 1.0e-3,
            "moving source/observer positions should alter the response"
        );
    }

    #[test]
    fn green_exposes_direct_and_connection_wronskians() {
        let config = SeriesConfig::default();
        let omega = z(0.9, -0.12);
        let result = poschl_teller_retarded_green(1.3, 0.75, omega, 0.73, -0.21, config).unwrap();
        close(result.wronskian, result.connection_wronskian, 2.0e-9);
    }

    #[test]
    fn argument_principle_counts_simple_and_threshold_double_zeros() {
        let q = (0.75_f64 - 0.25).sqrt();
        let simple_center = z(q, -0.5);
        let simple =
            poschl_teller_wronskian_winding_number(1.0, 0.75, simple_center, 0.05, 128).unwrap();
        assert_eq!(simple.winding_number, 1);

        let threshold =
            poschl_teller_wronskian_winding_number(1.0, 0.25, z(0.0, -0.5), 0.05, 128).unwrap();
        assert_eq!(threshold.winding_number, 2);
    }

    #[test]
    fn contour_residue_matches_simple_pole_derivative_formula() {
        let config = SeriesConfig::default();
        let q = (0.75_f64 - 0.25).sqrt();
        let pole = z(q, -0.5);
        let contour = poschl_teller_retarded_green_laurent(
            1.0,
            0.75,
            pole,
            PoschlTellerGreenContourConfig {
                radius: 0.03,
                observer_x: 0.4,
                source_x: -0.3,
                samples: 128,
            },
            config,
        )
        .unwrap();
        assert_eq!(contour.winding_number, 1);
        assert!(contour.c_minus_two.norm() < 1.0e-8);
        assert!(contour.max_relative_wronskian_disagreement < 2.0e-8);
        assert!(contour.quadrature_refinements >= 3);
        assert!(contour.stable_quadrature_refinements >= 2);
        assert!(contour.scaled_quadrature_difference <= GREEN_LAURENT_QUADRATURE_TOLERANCE);

        let left = poschl_teller_jost(1.0, 0.75, pole, -0.3, JostSide::Left, config).unwrap();
        let right = poschl_teller_jost(1.0, 0.75, pole, 0.4, JostSide::Right, config).unwrap();
        let h = 1.0e-4;
        let upper = poschl_teller_jost_wronskian_connection(1.0, 0.75, pole + z(h, 0.0)).unwrap();
        let lower = poschl_teller_jost_wronskian_connection(1.0, 0.75, pole - z(h, 0.0)).unwrap();
        let derivative = (upper - lower) / (2.0 * h);
        let expected_residue = -(left.value * right.value) / derivative;
        close(contour.c_minus_one, expected_residue, 2.0e-5);
    }

    #[test]
    fn threshold_contour_extracts_second_order_laurent_coefficient() {
        let config = SeriesConfig::default();
        let center = z(0.0, -0.5);
        let coarse = poschl_teller_retarded_green_laurent(
            1.0,
            0.25,
            center,
            PoschlTellerGreenContourConfig {
                radius: 0.05,
                observer_x: 0.4,
                source_x: -0.3,
                samples: 128,
            },
            config,
        )
        .unwrap();
        let fine = poschl_teller_retarded_green_laurent(
            1.0,
            0.25,
            center,
            PoschlTellerGreenContourConfig {
                radius: 0.025,
                observer_x: 0.4,
                source_x: -0.3,
                samples: 256,
            },
            config,
        )
        .unwrap();
        assert_eq!(coarse.winding_number, 2);
        assert_eq!(fine.winding_number, 2);
        close(coarse.c_minus_two, fine.c_minus_two, 2.0e-7);
        close(coarse.c_minus_one, fine.c_minus_one, 2.0e-7);
        assert!(coarse.stable_quadrature_refinements >= 2);
        assert!(fine.stable_quadrature_refinements >= 2);
        assert!(coarse.scaled_quadrature_difference <= GREEN_LAURENT_QUADRATURE_TOLERANCE);
        assert!(fine.scaled_quadrature_difference <= GREEN_LAURENT_QUADRATURE_TOLERANCE);
        assert!(fine.c_minus_two.norm() > 1.0e-3);
        assert!(fine.max_relative_wronskian_disagreement < 2.0e-8);
    }

    #[test]
    fn argument_principle_winding_is_stable_under_nested_refinement() {
        let simple = poschl_teller_wronskian_winding_number(
            1.0,
            0.75,
            z((0.75_f64 - 0.25).sqrt(), -0.5),
            0.05,
            32,
        )
        .unwrap();
        assert_eq!(simple.winding_number, 1);
        assert!(simple.samples >= 128);
        assert!(simple.refinements >= 2);
        assert!(simple.stable_refinements >= 3);
        assert!(simple.max_phase_step < core::f64::consts::FRAC_PI_8);

        let double =
            poschl_teller_wronskian_winding_number(1.0, 0.25, z(0.0, -0.5), 0.05, 32).unwrap();
        assert_eq!(double.winding_number, 2);
        assert!(double.samples >= 128);
        assert!(double.refinements >= 2);
        assert!(double.stable_refinements >= 3);
        assert!(double.max_phase_step < core::f64::consts::FRAC_PI_8);
    }

    #[test]
    fn winding_number_rejects_invalid_contours() {
        assert_eq!(
            poschl_teller_wronskian_winding_number(1.0, 0.75, z(0.5, -0.5), 0.0, 128,),
            Err(SpecialError::InvalidConfiguration)
        );
        assert_eq!(
            poschl_teller_wronskian_winding_number(1.0, 0.75, z(0.5, -0.5), 0.05, 8,),
            Err(SpecialError::InvalidConfiguration)
        );
    }

    #[test]
    fn causal_region_selector_reproduces_all_three_exact_domains() {
        let late =
            classify_poschl_teller_mode_sum_region(2.0, 0.75, 8.0, 4.0, 2.0, 1.0e-10).unwrap();
        assert_eq!(late.region, PoschlTellerModeSumRegion::LateQnm);
        assert!(late.direct_lightcone_margin > 0.0);
        assert!(late.reflected_lightcone_margin > 0.0);

        let early =
            classify_poschl_teller_mode_sum_region(2.0, 0.75, 4.0, 4.0, 2.0, 1.0e-10).unwrap();
        assert_eq!(early.region, PoschlTellerModeSumRegion::EarlyMatsubara);
        assert!(early.direct_lightcone_margin > 0.0);
        assert!(early.reflected_lightcone_margin < 0.0);

        let zero =
            classify_poschl_teller_mode_sum_region(2.0, 0.75, 1.0, 4.0, 2.0, 1.0e-10).unwrap();
        assert_eq!(zero.region, PoschlTellerModeSumRegion::CausallyZero);
        assert!(zero.direct_lightcone_margin < 0.0);
    }

    #[test]
    fn causal_region_selector_scales_with_width_and_handles_opposite_sides() {
        let narrow =
            classify_poschl_teller_mode_sum_region(2.0, 0.75, 4.0, 4.0, 2.0, 1.0e-10).unwrap();
        let scaled =
            classify_poschl_teller_mode_sum_region(4.0, 0.75, 8.0, 8.0, 4.0, 1.0e-10).unwrap();
        assert_eq!(narrow.region, scaled.region);
        assert_eq!(narrow.dimensionless_time, scaled.dimensionless_time);
        assert_eq!(
            narrow.direct_lightcone_margin,
            scaled.direct_lightcone_margin
        );
        assert_eq!(
            narrow.reflected_lightcone_margin,
            scaled.reflected_lightcone_margin
        );

        let opposite_sides =
            classify_poschl_teller_mode_sum_region(2.0, 0.75, 4.0, 4.0, -2.0, 1.0e-10).unwrap();
        assert_eq!(
            opposite_sides.region,
            PoschlTellerModeSumRegion::CausallyZero
        );
    }

    #[test]
    fn causal_region_selector_abstains_at_both_wavefronts() {
        let direct =
            classify_poschl_teller_mode_sum_region(1.0, 0.75, 1.0, 2.0, 1.0, 1.0e-8).unwrap();
        assert_eq!(
            direct.region,
            PoschlTellerModeSumRegion::TransitionUnresolved
        );

        let reflected =
            classify_poschl_teller_mode_sum_region(1.0, 0.75, 3.0, 2.0, 1.0, 1.0e-8).unwrap();
        assert_eq!(
            reflected.region,
            PoschlTellerModeSumRegion::TransitionUnresolved
        );
    }

    #[test]
    fn causal_region_selector_fails_closed_on_invalid_inputs() {
        for (width, lambda, time, observer, source, tolerance) in [
            (0.0, 0.75, 1.0, 2.0, 1.0, 1.0e-8),
            (1.0, 0.0, 1.0, 2.0, 1.0, 1.0e-8),
            (1.0, 0.75, f64::NAN, 2.0, 1.0, 1.0e-8),
            (1.0, 0.75, 1.0, 2.0, 1.0, 0.0),
        ] {
            assert_eq!(
                classify_poschl_teller_mode_sum_region(
                    width, lambda, time, observer, source, tolerance
                ),
                Err(SpecialError::InvalidCausalRegionParameters)
            );
        }
    }

    #[test]
    fn asymptotic_ratio_limits_match_the_published_region_tests() {
        let late =
            poschl_teller_mode_sum_asymptotic_ratios(2.0, 0.75, 8.0, 4.0, 2.0, 1.0e-10).unwrap();
        assert_eq!(late.region, PoschlTellerModeSumRegion::LateQnm);
        close(
            z(late.qnm_ratio_limit.unwrap(), 0.0),
            z((-1.0_f64).exp(), 0.0),
            1.0e-14,
        );
        assert!(late.matsubara_positive_ratio_limit.is_none());
        assert!(late.matsubara_negative_ratio_limit.is_none());

        let early =
            poschl_teller_mode_sum_asymptotic_ratios(2.0, 0.75, 4.0, 4.0, 2.0, 1.0e-10).unwrap();
        assert_eq!(early.region, PoschlTellerModeSumRegion::EarlyMatsubara);
        close(
            z(early.matsubara_positive_ratio_limit.unwrap(), 0.0),
            z((-1.0_f64).exp(), 0.0),
            1.0e-14,
        );
        close(
            z(early.matsubara_negative_ratio_limit.unwrap(), 0.0),
            z((-5.0_f64).exp(), 0.0),
            1.0e-14,
        );
        assert!(early.qnm_ratio_limit.is_none());
    }

    #[test]
    fn asymptotic_ratio_limits_are_absent_outside_sum_regions() {
        for (time, expected) in [
            (1.0, PoschlTellerModeSumRegion::CausallyZero),
            (6.0, PoschlTellerModeSumRegion::TransitionUnresolved),
        ] {
            let result =
                poschl_teller_mode_sum_asymptotic_ratios(2.0, 0.75, time, 4.0, 2.0, 1.0e-8)
                    .unwrap();
            assert_eq!(result.region, expected);
            assert!(result.qnm_ratio_limit.is_none());
            assert!(result.matsubara_positive_ratio_limit.is_none());
            assert!(result.matsubara_negative_ratio_limit.is_none());
        }
    }
}
