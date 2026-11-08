//! Analytic frequency-derivative controls for Pöschl–Teller QNMs.
//!
//! This uses the width convention from Lestingi, Sberna & Green:
//! V(x, zeta) = sech²(zeta x), with
//! omega_n^±(zeta) = ±sqrt(1-zeta²/4) - i zeta (n+1/2).
//!
//! This is an analytic known-answer reference only. It does not compute the
//! bilinear form, perturbed mode shape, QNM projection sum, continuum term,
//! Green function or a physical black-hole waveform.

use core::fmt;
use num_complex::Complex64;

/// Choice of the two frequency branches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Branch {
    Plus,
    Minus,
}

/// Errors for invalid perturbation parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerturbationError {
    NonFiniteWidthParameter,
    NonPositiveWidthParameter,
    ThresholdSingularity,
    NonFiniteResult,
}

impl fmt::Display for PerturbationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteWidthParameter => write!(f, "width parameter zeta must be finite"),
            Self::NonPositiveWidthParameter => {
                write!(f, "width parameter zeta must be strictly positive")
            }
            Self::ThresholdSingularity => write!(f, "frequency derivative is singular at zeta = 2"),
            Self::NonFiniteResult => write!(f, "frequency derivative produced a non-finite result"),
        }
    }
}

impl std::error::Error for PerturbationError {}

/// Analytic frequency and derivatives for the exact Pöschl–Teller spectrum.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoschlTellerQnm {
    zeta: f64,
    overtone: u32,
    branch: Branch,
}

fn finite(value: Complex64) -> bool {
    value.re.is_finite() && value.im.is_finite()
}

impl PoschlTellerQnm {
    /// Construct an exact Pöschl–Teller mode.
    ///
    /// zeta is the inverse-width parameter in V(x,zeta)=sech²(zeta*x).
    /// Values above 2 are accepted using the principal complex square root.
    /// Exactly zeta=2 is rejected by derivative methods because the branch
    /// derivative is singular there.
    pub fn new(zeta: f64, overtone: u32, branch: Branch) -> Result<Self, PerturbationError> {
        if !zeta.is_finite() {
            return Err(PerturbationError::NonFiniteWidthParameter);
        }
        if zeta <= 0.0 {
            return Err(PerturbationError::NonPositiveWidthParameter);
        }
        Ok(Self {
            zeta,
            overtone,
            branch,
        })
    }

    pub const fn zeta(self) -> f64 {
        self.zeta
    }

    pub const fn overtone(self) -> u32 {
        self.overtone
    }

    pub const fn branch(self) -> Branch {
        self.branch
    }

    fn branch_sign(self) -> f64 {
        match self.branch {
            Branch::Plus => 1.0,
            Branch::Minus => -1.0,
        }
    }

    fn radicand(self) -> f64 {
        1.0 - self.zeta * self.zeta / 4.0
    }

    fn validate_derivative_domain(self) -> Result<(), PerturbationError> {
        if !self.radicand().is_finite() {
            return Err(PerturbationError::NonFiniteWidthParameter);
        }
        if self.zeta == 2.0 {
            return Err(PerturbationError::ThresholdSingularity);
        }
        Ok(())
    }

    /// Exact complex QNM frequency.
    pub fn frequency(self) -> Result<Complex64, PerturbationError> {
        let root = Complex64::new(self.radicand(), 0.0).sqrt();
        let real_branch = root * self.branch_sign();
        let damping = Complex64::new(0.0, -self.zeta * (f64::from(self.overtone) + 0.5));
        let result = real_branch + damping;
        if finite(result) {
            Ok(result)
        } else {
            Err(PerturbationError::NonFiniteResult)
        }
    }

    /// Exact first derivative d omega / d zeta.
    ///
    /// d/dzeta sqrt(1-zeta²/4) = -zeta / (4 sqrt(1-zeta²/4)).
    pub fn first_derivative(self) -> Result<Complex64, PerturbationError> {
        self.validate_derivative_domain()?;
        let root = Complex64::new(self.radicand(), 0.0).sqrt();
        let branch = Complex64::new(-self.branch_sign() * self.zeta / 4.0, 0.0) / root;
        let damping = Complex64::new(0.0, -(f64::from(self.overtone) + 0.5));
        let result = branch + damping;
        if finite(result) {
            Ok(result)
        } else {
            Err(PerturbationError::NonFiniteResult)
        }
    }

    /// Exact second derivative d² omega / d zeta².
    ///
    /// d²/dzeta² sqrt(1-zeta²/4)
    /// = -1/(4 sqrt(1-zeta²/4))
    ///   - zeta²/(16 (1-zeta²/4)^(3/2)).
    pub fn second_derivative(self) -> Result<Complex64, PerturbationError> {
        self.validate_derivative_domain()?;
        let g = Complex64::new(self.radicand(), 0.0);
        let root = g.sqrt();
        let first = Complex64::new(-1.0 / 4.0, 0.0) / root;
        let second = Complex64::new(-self.zeta * self.zeta / 16.0, 0.0) / (g * root);
        let result = (first + second) * self.branch_sign();
        if finite(result) {
            Ok(result)
        } else {
            Err(PerturbationError::NonFiniteResult)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: Complex64, expected: Complex64, tolerance: f64) {
        assert!(
            (actual - expected).norm() <= tolerance,
            "actual={actual:?}, expected={expected:?}, tolerance={tolerance:e}"
        );
    }

    fn plus(zeta: f64, n: u32) -> PoschlTellerQnm {
        PoschlTellerQnm::new(zeta, n, Branch::Plus).unwrap()
    }

    #[test]
    fn exact_frequency_matches_published_fundamental_fixture() {
        let mode = plus(1.0, 0);
        close(
            mode.frequency().unwrap(),
            Complex64::new(3.0_f64.sqrt() / 2.0, -0.5),
            1.0e-14,
        );
    }

    #[test]
    fn first_derivative_matches_published_width_shift_for_multiple_overtones() {
        for n in [0, 1, 2, 5, 10, 15, 30, 40, 50] {
            let expected = Complex64::new(-1.0 / (2.0 * 3.0_f64.sqrt()), -(f64::from(n) + 0.5));
            close(plus(1.0, n).first_derivative().unwrap(), expected, 1.0e-14);
        }
    }

    #[test]
    fn second_derivative_matches_published_fundamental_fixture() {
        let expected = Complex64::new(-2.0 / (3.0 * 3.0_f64.sqrt()), 0.0);
        close(plus(1.0, 0).second_derivative().unwrap(), expected, 1.0e-14);
    }

    #[test]
    fn branch_reversal_flips_real_derivatives_but_not_damping_derivative() {
        let plus = PoschlTellerQnm::new(1.0, 3, Branch::Plus).unwrap();
        let minus = PoschlTellerQnm::new(1.0, 3, Branch::Minus).unwrap();
        close(
            plus.first_derivative().unwrap(),
            Complex64::new(-1.0 / (2.0 * 3.0_f64.sqrt()), -3.5),
            1.0e-14,
        );
        close(
            minus.first_derivative().unwrap(),
            Complex64::new(1.0 / (2.0 * 3.0_f64.sqrt()), -3.5),
            1.0e-14,
        );
        close(
            minus.second_derivative().unwrap(),
            -plus.second_derivative().unwrap(),
            1.0e-14,
        );
    }

    #[test]
    fn analytic_first_derivative_agrees_with_finite_difference_sweep() {
        let zeta = 1.0;
        let mode = plus(zeta, 4);
        let exact = mode.first_derivative().unwrap();
        let mut previous_error = f64::INFINITY;
        for h in [1.0e-2, 5.0e-3, 2.5e-3, 1.25e-3] {
            let upper = plus(zeta + h, 4).frequency().unwrap();
            let lower = plus(zeta - h, 4).frequency().unwrap();
            let estimate = (upper - lower) / (2.0 * h);
            let error = (estimate - exact).norm();
            assert!(error < previous_error);
            previous_error = error;
        }
        assert!(previous_error < 2.0e-7);
    }

    #[test]
    fn analytic_second_derivative_agrees_with_finite_difference_sweep() {
        let zeta = 1.0;
        let mode = plus(zeta, 0);
        let exact = mode.second_derivative().unwrap();
        let mut previous_error = f64::INFINITY;
        for h in [2.0e-2, 1.0e-2, 5.0e-3, 2.5e-3] {
            let upper = plus(zeta + h, 0).frequency().unwrap();
            let center = mode.frequency().unwrap();
            let lower = plus(zeta - h, 0).frequency().unwrap();
            let estimate = (upper - 2.0 * center + lower) / (h * h);
            let error = (estimate - exact).norm();
            assert!(error < previous_error);
            previous_error = error;
        }
        assert!(previous_error < 2.0e-6);
    }

    #[test]
    fn derivative_respects_principal_complex_continuation_above_threshold() {
        let mode = plus(2.5, 0);
        assert!(finite(mode.frequency().unwrap()));
        assert!(finite(mode.first_derivative().unwrap()));
        assert!(finite(mode.second_derivative().unwrap()));
    }

    #[test]
    fn singular_threshold_and_invalid_parameters_fail_closed() {
        assert_eq!(
            PoschlTellerQnm::new(2.0, 0, Branch::Plus)
                .unwrap()
                .first_derivative(),
            Err(PerturbationError::ThresholdSingularity)
        );
        assert_eq!(
            PoschlTellerQnm::new(0.0, 0, Branch::Plus),
            Err(PerturbationError::NonPositiveWidthParameter)
        );
        assert_eq!(
            PoschlTellerQnm::new(f64::NAN, 0, Branch::Plus),
            Err(PerturbationError::NonFiniteWidthParameter)
        );
        assert_eq!(
            PoschlTellerQnm::new(f64::INFINITY, 0, Branch::Plus),
            Err(PerturbationError::NonFiniteWidthParameter)
        );
    }
}
