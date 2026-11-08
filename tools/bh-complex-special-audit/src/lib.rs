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
    0.999_999_999_999_809_93,
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
}

impl fmt::Display for SpecialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteInput => write!(f, "special-function inputs must be finite"),
            Self::GammaPole => write!(f, "gamma function has a pole at this input"),
            Self::HypergeometricDenominatorPole => {
                write!(f, "hypergeometric denominator parameter is a non-positive integer")
            }
            Self::ResonantContinuationUnsupported => {
                write!(f, "connection formula is resonant when c-a-b is an integer")
            }
            Self::BranchCutUnsupported => {
                write!(f, "the requested point lies on the principal ₂F₁ branch cut [1, infinity)")
            }
            Self::ContinuationRegionUnsupported => {
                write!(f, "input is outside the explicitly supported ₂F₁ continuation regions")
            }
            Self::NoConvergence => write!(f, "hypergeometric series did not meet its error criterion"),
            Self::NonFiniteResult => write!(f, "special-function calculation produced a non-finite result"),
            Self::InvalidConfiguration => write!(f, "series configuration is invalid"),
            Self::AssociatedLegendreDomainUnsupported => {
                write!(f, "Ferrers associated-Legendre control requires real z strictly between -1 and 1")
            }
            Self::InvalidPoschlTellerParameters => {
                write!(f, "Pöschl–Teller Jost parameters require finite omega, b>0 and lambda>=0")
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
        sum += Complex64::new(*coefficient, 0.0)
            / (shifted + Complex64::new(index as f64, 0.0));
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
    /// Conservative geometric-tail estimate from the final series terms.
    /// This is an a-posteriori estimate, not a rigorous global bound.
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
/// Convergence is accepted only after an estimated geometric tail falls below
/// the configured tolerance. The returned estimate is diagnostic and not a
/// proof of a rigorous tail bound for arbitrary complex parameters.
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

        term *= (a + Complex64::new(n_minus_one, 0.0))
            * (b + Complex64::new(n_minus_one, 0.0))
            * z
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

        let n_complex = Complex64::new(n as f64, 0.0);
        let next_denominator = c + n_complex;
        if next_denominator == Complex64::new(0.0, 0.0) {
            return Err(SpecialError::HypergeometricDenominatorPole);
        }
        let next_ratio = ((a + n_complex) * (b + n_complex) * z
            / (next_denominator * (n as f64 + 1.0)))
        .norm();

        // The ratio tends to |z|. Using the larger of the local ratio and |z|
        // avoids an optimistic estimate when the asymptotic ratio is approached
        // from below. A 1% cushion is retained for the diagnostic tail estimate.
        let ratio_bound = (next_ratio.max(z.norm()) * 1.01).min(0.999_999_999_999);
        let estimated_tail = if ratio_bound < 1.0 {
            term.norm() * ratio_bound / (1.0 - ratio_bound)
        } else {
            f64::INFINITY
        };
        let tolerance =
            config.absolute_tolerance + config.relative_tolerance * sum.norm();

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
    let series_one = hyp2f1_series(
        a,
        b,
        Complex64::new(1.0, 0.0) - delta,
        one_minus_z,
        config,
    )?;
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
    let coefficient_two =
        gamma_c * gamma(-delta)? * reciprocal_gamma(a)? * reciprocal_gamma(b)?;
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
/// - Pfaff transform when |z/(z-1)| < 1 and the transformed series converges.
///
/// The principal branch cut [1, infinity) is rejected. Resonant near-one
/// continuation (c-a-b integer) is explicitly unsupported; a logarithmic
/// continuation is required there and is not silently approximated.
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
        return connection_near_one(a, b, c, z, config);
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
    if !width.is_finite()
        || !lambda.is_finite()
        || !x.is_finite()
        || width <= 0.0
        || lambda < 0.0
    {
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

    let degree = Complex64::new(-0.5, 0.0)
        + Complex64::new(0.25 - lambda, 0.0).sqrt();
    let order = Complex64::new(0.0, 1.0) * width * omega;
    let legendre = associated_legendre_ferrers(degree, order, argument, config)?;
    let normalization = gamma(Complex64::new(1.0, 0.0) - order)?;

    let sech_squared = 1.0 - z * z;
    let argument_x_derivative = side_sign * sech_squared / width;
    let argument_x_second_derivative =
        -2.0 * side_sign * z * sech_squared / (width * width);

    let value = normalization * legendre.value;
    let first_x_derivative =
        normalization * legendre.first_derivative * argument_x_derivative;
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
    let value = left.value * right.first_x_derivative
        - left.first_x_derivative * right.value;
    if finite(value) {
        Ok(value)
    } else {
        Err(SpecialError::NonFiniteResult)
    }
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

    #[test]
    fn gamma_matches_independent_80_digit_fixture_one() {
        let actual = gamma(z(0.3, 1.2)).unwrap();
        let expected = z(
            0.107_075_474_962_553_64,
            -0.353_143_987_729_088_6,
        );
        close(actual, expected, 2.0e-14);
    }

    #[test]
    fn gamma_reflection_path_matches_independent_80_digit_fixture() {
        let actual = gamma(z(-1.2, 0.7)).unwrap();
        let expected = z(
            0.174_095_916_181_767_3,
            0.706_909_450_271_453_4,
        );
        close(actual, expected, 3.0e-14);
    }

    #[test]
    fn gamma_recurrence_and_conjugation_identities_hold() {
        let value = z(0.4, 1.1);
        let lhs = gamma(value + z(1.0, 0.0)).unwrap();
        let rhs = value * gamma(value).unwrap();
        close(lhs, rhs, 2.0e-13);
        close(gamma(value.conj()).unwrap(), gamma(value).unwrap().conj(), 2.0e-13);
    }

    #[test]
    fn gamma_reflection_identity_holds() {
        let value = z(0.37, 0.82);
        let lhs = gamma(value).unwrap() * gamma(z(1.0, 0.0) - value).unwrap();
        let rhs = z(core::f64::consts::PI, 0.0)
            / (z(core::f64::consts::PI, 0.0) * value).sin();
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
    fn branch_cut_and_resonant_continuation_are_rejected() {
        assert_eq!(
            hyp2f1(z(0.5, 0.0), z(0.7, 0.0), z(1.4, 0.0), z(1.2, 0.0), SeriesConfig::default()),
            Err(SpecialError::BranchCutUnsupported)
        );
        assert_eq!(
            hyp2f1(z(0.6, 0.0), z(0.8, 0.0), z(2.4, 0.0), z(0.95, 0.0), SeriesConfig::default()),
            Err(SpecialError::ResonantContinuationUnsupported)
        );
    }

    #[test]
    fn denominator_poles_and_unsupported_continuation_fail_closed() {
        assert_eq!(
            hyp2f1(z(0.5, 0.0), z(0.7, 0.0), z(-2.0, 0.0), z(0.2, 0.0), SeriesConfig::default()),
            Err(SpecialError::HypergeometricDenominatorPole)
        );
        assert_eq!(
            hyp2f1(z(0.4, 0.2), z(1.1, -0.3), z(2.2, 0.1), z(0.5, 1.0), SeriesConfig::default()),
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
        let upper = hyp2f1(a, b, c, point + z(h, 0.0), SeriesConfig::default()).unwrap().value;
        let lower = hyp2f1(a, b, c, point - z(h, 0.0), SeriesConfig::default()).unwrap().value;
        let numeric = (upper - lower) / (2.0 * h);
        let analytic = a * b / c
            * hyp2f1(a + z(1.0, 0.0), b + z(1.0, 0.0), c + z(1.0, 0.0), point, SeriesConfig::default()).unwrap().value;
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
            let result = associated_legendre_ferrers(
                degree,
                order,
                z(y, 0.0),
                SeriesConfig::default(),
            )
            .unwrap();
            let one_minus_y2 = 1.0 - y * y;
            let residual = one_minus_y2 * result.second_derivative
                - 2.0 * y * result.first_derivative
                + (degree * (degree + z(1.0, 0.0))
                    - order * order / one_minus_y2)
                    * result.value;
            assert!(
                residual.norm() < 3.0e-10,
                "z={y}, residual={residual:?}"
            );
        }
    }

    #[test]
    fn associated_legendre_first_derivative_matches_central_difference() {
        let degree = z(-0.5, 0.7);
        let order = z(0.35, -0.2);
        let y = 0.23;
        let h = 1.0e-5;
        let base = associated_legendre_ferrers(
            degree,
            order,
            z(y, 0.0),
            SeriesConfig::default(),
        )
        .unwrap();
        let upper = associated_legendre_ferrers(
            degree,
            order,
            z(y + h, 0.0),
            SeriesConfig::default(),
        )
        .unwrap()
        .value;
        let lower = associated_legendre_ferrers(
            degree,
            order,
            z(y - h, 0.0),
            SeriesConfig::default(),
        )
        .unwrap()
        .value;
        close(
            base.first_derivative,
            (upper - lower) / (2.0 * h),
            2.0e-8,
        );
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
        close(left.first_x_derivative, -z(0.0, 1.0) * omega * left.value, 2.0e-13);
        close(right.first_x_derivative, z(0.0, 1.0) * omega * right.value, 2.0e-13);
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
                let solution = poschl_teller_jost(
                    width,
                    lambda,
                    omega,
                    x,
                    side,
                    SeriesConfig::default(),
                )
                .unwrap();
                let sech_squared = 1.0 - (x / width).tanh().powi(2);
                let residual = solution.second_x_derivative
                    + (omega * omega - lambda * sech_squared / (width * width))
                        * solution.value;
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
}
