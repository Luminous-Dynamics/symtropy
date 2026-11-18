//! Action-derived five-dimensional equal-spin Einstein-Gauss-Bonnet ODEs.
//!
//! The effective radial Lagrangian is transcribed from Brihaye et al.
//! (2010), Eqs. (2.10)-(2.12). A second-order multivariate jet evaluates
//! its gradient and Hessian. The Euler-Lagrange residual is then
//!
//!   E_q = dL/dq - sum_j (d^2 L / dq' dx_j) * x'_j,
//!
//! for x=(b,f,g,h,w,b',f',g',h',w'). The metric gauge g=r^2 is imposed
//! through the sample constructor only; the E_g equation is still returned.
//! This module is a cohomogeneity-one action oracle, not a generic 5D tensor
//! curvature implementation.
//!
//! The fourth Einstein-Hilbert term is b*g'*h'/h (not /g). This is pinned by
//! exact Boulware-Deser and Myers-Perry controls in the benchmark fixtures.

use core::fmt;
use core::ops::{Add, Div, Mul, Neg, Sub};

const N: usize = 10;

/// Radial values through second derivative for the five metric functions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadialMetricJet5D {
    pub radius: f64,
    pub b: f64,
    pub f: f64,
    pub g: f64,
    pub h: f64,
    pub w: f64,
    pub b_prime: f64,
    pub f_prime: f64,
    pub g_prime: f64,
    pub h_prime: f64,
    pub w_prime: f64,
    pub b_second: f64,
    pub f_second: f64,
    pub g_second: f64,
    pub h_second: f64,
    pub w_second: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReducedActionError {
    InvalidRadius(f64),
    NonFiniteInput(&'static str),
    OutsideStaticExterior { b: f64, f: f64, g: f64, h: f64 },
    InvalidRadialGauge(&'static str),
    NonFiniteResidual(&'static str),
    InvalidCoupling(f64),
}

impl fmt::Display for ReducedActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRadius(radius) => write!(f, "radius must be positive and finite, got {radius}"),
            Self::NonFiniteInput(name) => write!(f, "non-finite radial input: {name}"),
            Self::OutsideStaticExterior { b, f: radial, g, h } => write!(
                f,
                "action evaluation requires b,f,g,h>0, got b={b}, f={radial}, g={g}, h={h}"
            ),
            Self::InvalidRadialGauge(name) => write!(f, "invalid radial gauge: {name}"),
            Self::NonFiniteResidual(name) => write!(f, "non-finite Euler-Lagrange residual: {name}"),
            Self::InvalidCoupling(alpha) => write!(f, "non-finite Gauss-Bonnet coupling: {alpha}"),
        }
    }
}

impl std::error::Error for ReducedActionError {}

impl RadialMetricJet5D {
    /// Build a sample in the areal-base gauge g=r^2 after the action has
    /// conceptually been varied with g unfixed.
    #[allow(clippy::too_many_arguments)]
    pub fn areal_base(
        radius: f64,
        b: f64,
        f: f64,
        h: f64,
        w: f64,
        b_prime: f64,
        f_prime: f64,
        h_prime: f64,
        w_prime: f64,
        b_second: f64,
        f_second: f64,
        h_second: f64,
        w_second: f64,
    ) -> Result<Self, ReducedActionError> {
        let sample = Self {
            radius,
            b,
            f,
            g: radius * radius,
            h,
            w,
            b_prime,
            f_prime,
            g_prime: 2.0 * radius,
            h_prime,
            w_prime,
            b_second,
            f_second,
            g_second: 2.0,
            h_second,
            w_second,
        };
        sample.validate()?;
        Ok(sample)
    }

    /// General radial sample, useful before applying a metric gauge.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        radius: f64,
        b: f64,
        f: f64,
        g: f64,
        h: f64,
        w: f64,
        b_prime: f64,
        f_prime: f64,
        g_prime: f64,
        h_prime: f64,
        w_prime: f64,
        b_second: f64,
        f_second: f64,
        g_second: f64,
        h_second: f64,
        w_second: f64,
    ) -> Result<Self, ReducedActionError> {
        let sample = Self {
            radius, b, f, g, h, w,
            b_prime, f_prime, g_prime, h_prime, w_prime,
            b_second, f_second, g_second, h_second, w_second,
        };
        sample.validate()?;
        Ok(sample)
    }

    pub fn validate(self) -> Result<(), ReducedActionError> {
        if !self.radius.is_finite() || self.radius <= 0.0 {
            return Err(ReducedActionError::InvalidRadius(self.radius));
        }
        let finite = [
            ("b", self.b), ("f", self.f), ("g", self.g), ("h", self.h), ("w", self.w),
            ("b_prime", self.b_prime), ("f_prime", self.f_prime),
            ("g_prime", self.g_prime), ("h_prime", self.h_prime), ("w_prime", self.w_prime),
            ("b_second", self.b_second), ("f_second", self.f_second),
            ("g_second", self.g_second), ("h_second", self.h_second),
            ("w_second", self.w_second),
        ];
        for (name, value) in finite {
            if !value.is_finite() {
                return Err(ReducedActionError::NonFiniteInput(name));
            }
        }
        if self.b <= 0.0 || self.f <= 0.0 || self.g <= 0.0 || self.h <= 0.0 {
            return Err(ReducedActionError::OutsideStaticExterior {
                b: self.b, f: self.f, g: self.g, h: self.h,
            });
        }
        Ok(())
    }
}

/// All five reduced Euler-Lagrange residuals and the canonical angular
/// momentum derived from the same action evaluation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReducedActionResiduals5D {
    pub e_b: f64,
    pub e_f: f64,
    pub e_g: f64,
    pub e_h: f64,
    pub e_w: f64,
    pub p_w_canonical: f64,
    pub p_w_closed_form: f64,
    pub f_prime_momentum: f64,
}

impl ReducedActionResiduals5D {
    /// Evaluate E_b,E_f,E_g,E_h,E_w from the unfixed radial action.
    ///
    /// This is a pointwise floating-point ODE residual. It is not an interval
    /// enclosure and is not by itself an existence certificate.
    pub fn evaluate(
        sample: RadialMetricJet5D,
        alpha: f64,
    ) -> Result<Self, ReducedActionError> {
        sample.validate()?;
        if !alpha.is_finite() {
            return Err(ReducedActionError::InvalidCoupling(alpha));
        }

        // State order: b,f,g,h,w,b',f',g',h',w'.
        let x = [
            Jet2::variable(sample.b, 0),
            Jet2::variable(sample.f, 1),
            Jet2::variable(sample.g, 2),
            Jet2::variable(sample.h, 3),
            Jet2::variable(sample.w, 4),
            Jet2::variable(sample.b_prime, 5),
            Jet2::variable(sample.f_prime, 6),
            Jet2::variable(sample.g_prime, 7),
            Jet2::variable(sample.h_prime, 8),
            Jet2::variable(sample.w_prime, 9),
        ];
        let b = x[0];
        let f = x[1];
        let g = x[2];
        let h = x[3];
        let _w = x[4];
        let bp = x[5];
        let _fp = x[6];
        let gp = x[7];
        let hp = x[8];
        let wp = x[9];

        let two = Jet2::constant(2.0);
        let four = Jet2::constant(4.0);
        let sixteen = Jet2::constant(16.0);
        let twelve = Jet2::constant(12.0);
        let alpha_j = Jet2::constant(alpha);
        let measure = (f * h / b).sqrt();

        // Brihaye et al. (2010), Eq. (2.11). Keep this term b*g'*h'/h.
        let l_einstein = measure * (
            bp * gp
            + g * bp * hp / (two * h)
            + b * gp * gp / (two * g)
            + b * gp * hp / h
            + g * h * wp * wp / two
            + two * b / f * (four - h / g)
        );

        let l_gauss_bonnet = measure / g * (
            four * h * bp * gp / g
            + two * (four * g - Jet2::constant(3.0) * h)
                * (bp * hp / h + h * wp * wp)
            - f * bp * hp * gp * gp / (two * h)
            - f * h * gp * gp * wp * wp / two
        );
        let l_eff = l_einstein + alpha_j * l_gauss_bonnet / four;

        let state_derivative = [
            sample.b_prime,
            sample.f_prime,
            sample.g_prime,
            sample.h_prime,
            sample.w_prime,
            sample.b_second,
            sample.f_second,
            sample.g_second,
            sample.h_second,
            sample.w_second,
        ];

        // E_q = partial L/partial q - d/dr(partial L/partial q').
        // Hessian row (q') dotted with x' is the exact chain-rule derivative
        // of the canonical momentum, with no finite differencing.
        let mut residual = [0.0; 5];
        for q in 0..5 {
            let momentum_index = q + 5;
            let radial_momentum_derivative = (0..N)
                .map(|j| l_eff.hessian[momentum_index][j] * state_derivative[j])
                .sum::<f64>();
            residual[q] = l_eff.gradient[q] - radial_momentum_derivative;
        }

        let radius_sq = sample.radius * sample.radius;
        let p_w_closed_form = sample.w_prime
            * sample.h
            * (sample.f * sample.h / sample.b).sqrt()
            * (sample.g
                - alpha / 4.0
                    * (sample.f * sample.g_prime * sample.g_prime / sample.g
                        - 16.0
                        + 12.0 * sample.h / sample.g));
        let p_w_canonical = l_eff.gradient[9];
        let f_prime_momentum = l_eff.gradient[6];

        let values = [
            residual[0], residual[1], residual[2], residual[3], residual[4],
            p_w_canonical, p_w_closed_form, f_prime_momentum,
        ];
        for (name, value) in [
            ("E_b", values[0]), ("E_f", values[1]), ("E_g", values[2]),
            ("E_h", values[3]), ("E_w", values[4]),
            ("P_w canonical", values[5]), ("P_w closed form", values[6]),
            ("f' momentum", values[7]),
        ] {
            if !value.is_finite() {
                return Err(ReducedActionError::NonFiniteResidual(name));
            }
        }

        // radius_sq is intentionally computed above for the gauge invariant
        // diagnostic; keep the areal-gauge identity explicit in the result.
        let _areal_base_consistency = radius_sq - sample.g;
        Ok(Self {
            e_b: residual[0],
            e_f: residual[1],
            e_g: residual[2],
            e_h: residual[3],
            e_w: residual[4],
            p_w_canonical,
            p_w_closed_form,
            f_prime_momentum,
        })
    }

    pub fn as_array(self) -> [f64; 5] {
        [self.e_b, self.e_f, self.e_g, self.e_h, self.e_w]
    }

    pub fn max_abs_equation_residual(self) -> f64 {
        self.as_array()
            .iter()
            .fold(0.0_f64, |maximum, value| maximum.max(value.abs()))
    }
}

/// Second-order multivariate forward-mode jet used only by the reduced action.
/// Hessian entries are exact algebraic derivatives evaluated in f64.
#[derive(Debug, Clone, Copy)]
struct Jet2 {
    value: f64,
    gradient: [f64; N],
    hessian: [[f64; N]; N],
}

impl Jet2 {
    fn constant(value: f64) -> Self {
        Self {
            value,
            gradient: [0.0; N],
            hessian: [[0.0; N]; N],
        }
    }

    fn variable(value: f64, index: usize) -> Self {
        let mut out = Self::constant(value);
        out.gradient[index] = 1.0;
        out
    }

    fn unary(self, value: f64, first: f64, second: f64) -> Self {
        let mut out = Self::constant(value);
        for i in 0..N {
            out.gradient[i] = first * self.gradient[i];
            for j in 0..N {
                out.hessian[i][j] = first * self.hessian[i][j]
                    + second * self.gradient[i] * self.gradient[j];
            }
        }
        out
    }

    fn reciprocal(self) -> Self {
        let inverse = 1.0 / self.value;
        self.unary(inverse, -inverse * inverse, 2.0 * inverse.powi(3))
    }

    fn sqrt(self) -> Self {
        let root = self.value.sqrt();
        self.unary(root, 0.5 / root, -0.25 / (self.value * root))
    }
}

impl Add for Jet2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        let mut out = Self::constant(self.value + rhs.value);
        for i in 0..N {
            out.gradient[i] = self.gradient[i] + rhs.gradient[i];
            for j in 0..N {
                out.hessian[i][j] = self.hessian[i][j] + rhs.hessian[i][j];
            }
        }
        out
    }
}

impl Sub for Jet2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        self + (-rhs)
    }
}

impl Neg for Jet2 {
    type Output = Self;
    fn neg(self) -> Self::Output {
        self * Self::constant(-1.0)
    }
}

impl Mul for Jet2 {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        let mut out = Self::constant(self.value * rhs.value);
        for i in 0..N {
            out.gradient[i] =
                self.gradient[i] * rhs.value + self.value * rhs.gradient[i];
            for j in 0..N {
                out.hessian[i][j] =
                    self.hessian[i][j] * rhs.value
                    + self.gradient[i] * rhs.gradient[j]
                    + self.gradient[j] * rhs.gradient[i]
                    + self.value * rhs.hessian[i][j];
            }
        }
        out
    }
}

impl Div for Jet2 {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.reciprocal()
    }
}
