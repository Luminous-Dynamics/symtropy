//! Five-dimensional equal-angular-momentum Lovelock reference controls.
//!
//! This crate deliberately does not reuse the four-dimensional
//! symtropy-relativity metric/tensor types. It starts with the
//! cohomogeneity-one equal-spin ansatz in the areal-base gauge g(r)=r^2.
//!
//! The exact rotating reference is the five-dimensional Myers-Perry solution
//! at alpha=0. The EGB rotational Noether flux is implemented as an
//! independent diagnostic for candidate radial profiles.

#![forbid(unsafe_code)]

use core::fmt;

/// Parameters for the nonextremal, equal-angular-momentum 5D Myers-Perry
/// solution in the gauge g(r)=r^2.
///
/// horizon_radius is the horizon coordinate in this gauge. It is not the
/// Boyer-Lindquist radial coordinate of the standard Myers-Perry line element.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MyersPerry5D {
    horizon_radius: f64,
    horizon_angular_velocity: f64,
    q: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReferenceError {
    NonFiniteParameter(&'static str),
    NonPositiveHorizonRadius(f64),
    ExtremalOrSuperextremal { q: f64 },
    RadiusInsideHorizon { radius: f64, horizon_radius: f64 },
    InvalidRadius(f64),
    InvalidCoupling(f64),
    InvalidNewtonConstant(f64),
    OutsideStaticExterior { b: f64, f: f64, h: f64 },
    InvalidMetricSample(&'static str),
    FluxDenominatorNearZero { denominator: f64, scale: f64 },
    NonFiniteResult(&'static str),
}

impl fmt::Display for ReferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteParameter(name) => write!(f, "{name} must be finite"),
            Self::NonPositiveHorizonRadius(value) => {
                write!(f, "horizon radius must be positive, got {value}")
            }
            Self::ExtremalOrSuperextremal { q } => write!(
                f,
                "the nonextremal Myers-Perry control requires 0 <= q < 1/2, got q={q}"
            ),
            Self::RadiusInsideHorizon { radius, horizon_radius } => write!(
                f,
                "radius {radius} is inside the horizon {horizon_radius}"
            ),
            Self::InvalidRadius(value) => write!(f, "radius must be positive, got {value}"),
            Self::InvalidCoupling(value) => {
                write!(f, "the static EGB control requires alpha >= 0, got {value}")
            }
            Self::InvalidNewtonConstant(value) => {
                write!(f, "Newton's constant must be positive, got {value}")
            }
            Self::OutsideStaticExterior { b, f: radial, h } => write!(
                f,
                "flux evaluation requires b>0, f>0 and h>0, got b={b}, f={radial}, h={h}"
            ),
            Self::InvalidMetricSample(name) => write!(f, "invalid radial metric sample: {name}"),
            Self::FluxDenominatorNearZero { denominator, scale } => write!(
                f,
                "rotational flux denominator is unresolved: K={denominator}, scale={scale}"
            ),
            Self::NonFiniteResult(name) => write!(f, "{name} is non-finite"),
        }
    }
}

impl std::error::Error for ReferenceError {}

/// Five radial metric functions and their first radial derivatives in the
/// g(r)=r^2 gauge. All metric functions are dimensionally in geometric units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadialMetricSample5D {
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
}

impl RadialMetricSample5D {
    fn validate_exterior(self) -> Result<(), ReferenceError> {
        let finite = [
            self.radius,
            self.b,
            self.f,
            self.g,
            self.h,
            self.w,
            self.b_prime,
            self.f_prime,
            self.g_prime,
            self.h_prime,
            self.w_prime,
        ];
        if finite.iter().any(|value| !value.is_finite()) {
            return Err(ReferenceError::InvalidMetricSample("non-finite component"));
        }
        if self.radius <= 0.0 || self.g <= 0.0 {
            return Err(ReferenceError::InvalidMetricSample("radius and g must be positive"));
        }
        // The implemented first integral is specialized to the areal-base
        // gauge g(r)=r^2. Reject a sample from another radial gauge rather
        // than silently applying the gauge-fixed expression.
        let expected_g = self.radius.powi(2);
        let expected_g_prime = 2.0 * self.radius;
        let g_scale = expected_g.abs().max(1.0);
        let gp_scale = expected_g_prime.abs().max(1.0);
        if (self.g - expected_g).abs() > 32.0 * f64::EPSILON * g_scale
            || (self.g_prime - expected_g_prime).abs()
                > 32.0 * f64::EPSILON * gp_scale
        {
            return Err(ReferenceError::InvalidMetricSample(
                "angular flux requires the fixed gauge g(r)=r^2",
            ));
        }
        if self.b <= 0.0 || self.f <= 0.0 || self.h <= 0.0 {
            return Err(ReferenceError::OutsideStaticExterior {
                b: self.b,
                f: self.f,
                h: self.h,
            });
        }
        Ok(())
    }
}

/// The five-dimensional Myers-Perry profile in the g(r)=r^2 gauge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MyersPerryProfile5D {
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
    /// Exact ratio f/b, with its finite horizon limit evaluated analytically.
    pub f_over_b: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HorizonData5D {
    pub b1: f64,
    pub f1: f64,
    pub h_horizon: f64,
    pub omega_horizon: f64,
    pub w1: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AsymptoticParameters5D {
    /// Coefficient U in f=1+U/r^2+V/r^4 and the leading b expansion.
    pub u_mass: f64,
    /// Coefficient V in h=r^2+V/r^2+... .
    pub v_rotation: f64,
    /// Coefficient W in w=W/r^4+... .
    pub w_angular: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicalCharges5D {
    pub energy: f64,
    /// Equal angular momentum in each rotation plane.
    pub angular_momentum_per_plane: f64,
}

impl MyersPerry5D {
    /// Construct a nonextremal equal-spin Myers-Perry solution.
    ///
    /// The nonextremal condition is q=(r_H*Omega_H)^2 < 1/2. Signed
    /// angular velocity is supported; the metric is invariant under
    /// Omega_H -> -Omega_H with w -> -w.
    pub fn new(
        horizon_radius: f64,
        horizon_angular_velocity: f64,
    ) -> Result<Self, ReferenceError> {
        if !horizon_radius.is_finite() {
            return Err(ReferenceError::NonFiniteParameter("horizon_radius"));
        }
        if !horizon_angular_velocity.is_finite() {
            return Err(ReferenceError::NonFiniteParameter(
                "horizon_angular_velocity",
            ));
        }
        if horizon_radius <= 0.0 {
            return Err(ReferenceError::NonPositiveHorizonRadius(horizon_radius));
        }
        let spin = horizon_radius * horizon_angular_velocity;
        let q = spin * spin;
        if !q.is_finite() || q >= 0.5 {
            return Err(ReferenceError::ExtremalOrSuperextremal { q });
        }
        Ok(Self {
            horizon_radius,
            horizon_angular_velocity,
            q,
        })
    }

    pub fn horizon_radius(self) -> f64 {
        self.horizon_radius
    }

    pub fn horizon_angular_velocity(self) -> f64 {
        self.horizon_angular_velocity
    }

    pub fn q(self) -> f64 {
        self.q
    }

    /// Mass parameter mu and rotation parameter a in the conventional
    /// equal-spin Myers-Perry representation.
    pub fn conventional_parameters(self) -> (f64, f64) {
        let mu = self.horizon_radius.powi(2) / (1.0 - self.q);
        let a = self.horizon_radius.powi(2) * self.horizon_angular_velocity;
        (mu, a)
    }

    pub fn horizon_data(self) -> HorizonData5D {
        let r_h = self.horizon_radius;
        let q = self.q;
        HorizonData5D {
            b1: 2.0 * (1.0 - 2.0 * q) / r_h,
            f1: 2.0 * (1.0 - 2.0 * q) / (r_h * (1.0 - q)),
            h_horizon: r_h.powi(2) / (1.0 - q),
            omega_horizon: self.horizon_angular_velocity,
            w1: 4.0 * self.horizon_angular_velocity * (q - 1.0) / r_h,
        }
    }

    pub fn asymptotic_parameters(self) -> AsymptoticParameters5D {
        let r_h = self.horizon_radius;
        let q = self.q;
        let omega = self.horizon_angular_velocity;
        AsymptoticParameters5D {
            u_mass: -r_h.powi(2) / (1.0 - q),
            v_rotation: r_h.powi(6) * omega.powi(2) / (1.0 - q),
            w_angular: r_h.powi(4) * omega / (1.0 - q),
        }
    }

    pub fn physical_charges(self, newton_constant: f64) -> Result<PhysicalCharges5D, ReferenceError> {
        if !newton_constant.is_finite() {
            return Err(ReferenceError::NonFiniteParameter("newton_constant"));
        }
        if newton_constant <= 0.0 {
            return Err(ReferenceError::InvalidNewtonConstant(newton_constant));
        }
        let asymptotic = self.asymptotic_parameters();
        let sphere_volume = 2.0 * core::f64::consts::PI.powi(2);
        Ok(PhysicalCharges5D {
            energy: -3.0 * sphere_volume * asymptotic.u_mass
                / (16.0 * core::f64::consts::PI * newton_constant),
            angular_momentum_per_plane: sphere_volume * asymptotic.w_angular
                / (8.0 * core::f64::consts::PI * newton_constant),
        })
    }

    /// Evaluate the exact profile and first derivatives for r >= r_H.
    /// The formula is arranged to keep b and f factored at the horizon.
    pub fn evaluate(self, radius: f64) -> Result<MyersPerryProfile5D, ReferenceError> {
        if !radius.is_finite() || radius <= 0.0 {
            return Err(ReferenceError::InvalidRadius(radius));
        }
        if radius < self.horizon_radius {
            return Err(ReferenceError::RadiusInsideHorizon {
                radius,
                horizon_radius: self.horizon_radius,
            });
        }

        let r_h = self.horizon_radius;
        let omega = self.horizon_angular_velocity;
        let q = self.q;
        let u = (r_h / radius).powi(2);
        let denominator = 1.0 - q + q * u.powi(2);
        let common = (1.0 - u) * (1.0 - q * (1.0 + u));
        let b = common / denominator;
        let f = common / (1.0 - q);
        let g = radius.powi(2);
        let h = g + r_h.powi(2) * q * u / (1.0 - q);
        let w = omega * u.powi(2) / denominator;

        let b_prime =
            2.0 * u * (1.0 - q - q * u.powi(2)) / (radius * denominator.powi(2));
        let f_prime =
            2.0 * u * (1.0 - 2.0 * q * u) / (radius * (1.0 - q));
        let g_prime = 2.0 * radius;
        let h_prime =
            2.0 * radius - 2.0 * r_h.powi(2) * q * u / ((1.0 - q) * radius);
        let w_prime =
            -4.0 * omega * u.powi(2) * (1.0 - q) / (radius * denominator.powi(2));
        let f_over_b = denominator / (1.0 - q);

        let outputs = [
            ("b", b),
            ("f", f),
            ("h", h),
            ("w", w),
            ("b_prime", b_prime),
            ("f_prime", f_prime),
            ("h_prime", h_prime),
            ("w_prime", w_prime),
            ("f_over_b", f_over_b),
        ];
        for (name, value) in outputs {
            if !value.is_finite() {
                return Err(ReferenceError::NonFiniteResult(name));
            }
        }

        Ok(MyersPerryProfile5D {
            radius,
            b,
            f,
            g,
            h,
            w,
            b_prime,
            f_prime,
            g_prime,
            h_prime,
            w_prime,
            f_over_b,
        })
    }

    /// Evaluate the rotational first integral by its regular horizon limit
    /// for the exact alpha=0 Myers-Perry family.
    pub fn horizon_angular_flux(self) -> f64 {
        let horizon = self.horizon_data();
        let sqrt_fh_over_b =
            (horizon.f1 * horizon.h_horizon / horizon.b1).sqrt();
        horizon.w1 * horizon.h_horizon * sqrt_fh_over_b * self.horizon_radius.powi(2)
    }
}

/// Evaluate the generalized 5D EGB angular Noether flux for a sampled
/// cohomogeneity-one metric in g(r)=r^2 gauge.
///
/// The conserved first integral is
/// P_w = w' h sqrt(f h / b) [r^2 - alpha (f - 4 + 3 h/r^2)].
///
/// The function is a diagnostic only: it does not prove that the remaining
/// EGB field equations hold.
pub fn egb_angular_momentum_flux(
    sample: RadialMetricSample5D,
    alpha: f64,
) -> Result<f64, ReferenceError> {
    if !alpha.is_finite() {
        return Err(ReferenceError::NonFiniteParameter("alpha"));
    }
    sample.validate_exterior()?;
    let radius_sq = sample.radius.powi(2);
    let k = radius_sq - alpha * (sample.f - 4.0 + 3.0 * sample.h / radius_sq);
    let value = sample.w_prime
        * sample.h
        * (sample.f * sample.h / sample.b).sqrt()
        * k;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(ReferenceError::NonFiniteResult("angular_momentum_flux"))
    }
}

/// Evaluate the time-sector radial current appearing in Brihaye et al.
/// (2010), Eq. (2.14).
///
/// The full current includes the Gauss-Bonnet corrections:
/// P_t(r) = sqrt(f*h/b) * [
///   r^2*(h*w*w' - b')
///   + alpha*((f - 4 + h/r^2 + r*f*h'/h)*b'
///       + (4 - 3*h/r^2 - f)*h*w*w'
///       + r*f*h*w'^2)
/// ].
///
/// IMPORTANT: at finite alpha this is NOT a conserved constant. On a full
/// 5D EGB solution the printed identity gives a source-balance equation.
/// Its asymptotic limit is 2*U. At alpha=0 it reduces to the Einstein
/// constant current P_t=2*U.
///
/// The current is a diagnostic / sourced identity, not a substitute for
/// the remaining EGB field equations.
pub fn time_sector_radial_flux(
    sample: RadialMetricSample5D,
    alpha: f64,
) -> Result<f64, ReferenceError> {
    if !alpha.is_finite() {
        return Err(ReferenceError::NonFiniteParameter("alpha"));
    }
    sample.validate_exterior()?;
    let radius_sq = sample.radius.powi(2);
    let base = radius_sq * (sample.h * sample.w * sample.w_prime - sample.b_prime);
    let b_prime_coefficient = sample.f - 4.0
        + sample.h / radius_sq
        + sample.radius * sample.f * sample.h_prime / sample.h;
    let ww_prime_coefficient = 4.0 - 3.0 * sample.h / radius_sq - sample.f;
    let gauss_bonnet = b_prime_coefficient * sample.b_prime
        + ww_prime_coefficient * sample.h * sample.w * sample.w_prime
        + sample.radius * sample.f * sample.h * sample.w_prime.powi(2);
    let value = (sample.f * sample.h / sample.b).sqrt() * (base + alpha * gauss_bonnet);
    if value.is_finite() {
        Ok(value)
    } else {
        Err(ReferenceError::NonFiniteResult("time_sector_radial_flux"))
    }
}

/// The on-shell source term in the derivative of the time-sector current,
/// derived by combining Eq. (2.14) with the trace of the 5D EGB equations:
///
///   dP_t/dr = (alpha/6) * r^2 * sqrt(b*h/f) * L_GB.
///
/// The caller must provide the Gauss-Bonnet scalar from an independent
/// curvature evaluator. This helper only applies the source prefactor.
pub fn time_sector_current_source(
    sample: RadialMetricSample5D,
    alpha: f64,
    gauss_bonnet_scalar: f64,
) -> Result<f64, ReferenceError> {
    if !alpha.is_finite() {
        return Err(ReferenceError::NonFiniteParameter("alpha"));
    }
    if !gauss_bonnet_scalar.is_finite() {
        return Err(ReferenceError::NonFiniteParameter("gauss_bonnet_scalar"));
    }
    sample.validate_exterior()?;
    let value = alpha / 6.0
        * sample.radius.powi(2)
        * (sample.b * sample.h / sample.f).sqrt()
        * gauss_bonnet_scalar;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(ReferenceError::NonFiniteResult("time_sector_current_source"))
    }
}

/// Reconstruct b'(r) from the local value of the full time-sector current
/// P_t(r), including the alpha-dependent terms in Eq. (2.14).
///
/// This is an exploratory pointwise inversion. A certified implementation
/// must interval-enclose the coefficient of b' away from zero over each
/// subdomain and independently control the source balance.
pub fn reconstruct_b_prime_from_time_flux(
    sample: RadialMetricSample5D,
    alpha: f64,
    local_current: f64,
) -> Result<f64, ReferenceError> {
    if !alpha.is_finite() {
        return Err(ReferenceError::NonFiniteParameter("alpha"));
    }
    if !local_current.is_finite() {
        return Err(ReferenceError::NonFiniteParameter("local_current"));
    }
    sample.validate_exterior()?;
    let radius_sq = sample.radius.powi(2);
    let b_prime_coefficient = sample.f - 4.0
        + sample.h / radius_sq
        + sample.radius * sample.f * sample.h_prime / sample.h;
    let coefficient = -radius_sq + alpha * b_prime_coefficient;
    let scale = radius_sq.abs() + (alpha * b_prime_coefficient).abs() + 1.0;
    if coefficient.abs() <= 64.0 * f64::EPSILON * scale {
        return Err(ReferenceError::FluxDenominatorNearZero {
            denominator: coefficient,
            scale,
        });
    }
    let ww_prime_coefficient = 4.0 - 3.0 * sample.h / radius_sq - sample.f;
    let non_b_prime = (radius_sq + alpha * ww_prime_coefficient)
        * sample.h * sample.w * sample.w_prime
        + alpha * sample.radius * sample.f * sample.h * sample.w_prime.powi(2);
    let normalized_current = local_current / (sample.f * sample.h / sample.b).sqrt();
    let result = (normalized_current - non_b_prime) / coefficient;
    if result.is_finite() {
        Ok(result)
    } else {
        Err(ReferenceError::NonFiniteResult("reconstructed_b_prime"))
    }
}

/// Recover w'(r) from the conserved EGB rotational flux. This is permitted
/// only when K(r) is numerically separated from zero. A rigorous verifier
/// must replace this floating-point guard with an interval enclosure of K.
pub fn reconstruct_w_prime_from_flux(
    sample: RadialMetricSample5D,
    alpha: f64,
    conserved_flux: f64,
) -> Result<f64, ReferenceError> {
    if !alpha.is_finite() {
        return Err(ReferenceError::NonFiniteParameter("alpha"));
    }
    if !conserved_flux.is_finite() {
        return Err(ReferenceError::NonFiniteParameter("conserved_flux"));
    }
    sample.validate_exterior()?;
    let radius_sq = sample.radius.powi(2);
    let correction = alpha * (sample.f - 4.0 + 3.0 * sample.h / radius_sq);
    let k = radius_sq - correction;
    let scale = radius_sq.abs() + correction.abs() + 1.0;
    if k.abs() <= 64.0 * f64::EPSILON * scale {
        return Err(ReferenceError::FluxDenominatorNearZero {
            denominator: k,
            scale,
        });
    }
    let denominator = sample.h * (sample.f * sample.h / sample.b).sqrt() * k;
    let result = conserved_flux / denominator;
    if result.is_finite() {
        Ok(result)
    } else {
        Err(ReferenceError::NonFiniteResult("reconstructed_w_prime"))
    }
}

/// Exact static Boulware-Deser branch in five-dimensional EGB gravity.
/// The rationalized expression avoids cancellation as alpha -> 0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoulwareDeser5D {
    pub horizon_radius: f64,
    pub alpha: f64,
}

/// Independent static 5D curvature data for the Boulware-Deser control.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoulwareDeserCurvatureData5D {
    pub lapse: f64,
    pub lapse_prime: f64,
    pub lapse_second: f64,
    pub gauss_bonnet_scalar: f64,
}


impl BoulwareDeser5D {
    pub fn new(horizon_radius: f64, alpha: f64) -> Result<Self, ReferenceError> {
        if !horizon_radius.is_finite() {
            return Err(ReferenceError::NonFiniteParameter("horizon_radius"));
        }
        if !alpha.is_finite() {
            return Err(ReferenceError::NonFiniteParameter("alpha"));
        }
        if horizon_radius <= 0.0 {
            return Err(ReferenceError::NonPositiveHorizonRadius(horizon_radius));
        }
        if alpha < 0.0 {
            return Err(ReferenceError::InvalidCoupling(alpha));
        }
        Ok(Self {
            horizon_radius,
            alpha,
        })
    }

    pub fn lapse(self, radius: f64) -> Result<f64, ReferenceError> {
        if !radius.is_finite() || radius <= 0.0 {
            return Err(ReferenceError::InvalidRadius(radius));
        }
        if radius < self.horizon_radius {
            return Err(ReferenceError::RadiusInsideHorizon {
                radius,
                horizon_radius: self.horizon_radius,
            });
        }
        let r4 = radius.powi(4);
        let radical = (1.0
            + self.alpha * (self.alpha + 2.0 * self.horizon_radius.powi(2)) / r4)
            .sqrt();
        let value = 1.0
            - (self.alpha + 2.0 * self.horizon_radius.powi(2))
                / (radius.powi(2) * (1.0 + radical));
        if value.is_finite() {
            Ok(value)
        } else {
            Err(ReferenceError::NonFiniteResult("Boulware-Deser lapse"))
        }
    }

    /// Return the lapse, its first two radial derivatives, and the 5D
    /// Gauss-Bonnet scalar for the static spherical Einstein branch.
    ///
    /// For ds^2=-F dt^2+dr^2/F+r^2 dOmega_3^2,
    /// L_GB = 12/r^3 * ((F-1)*(r*F''+2*F') + r*(F')^2).
    ///
    /// Derivatives are evaluated in a cancellation-resistant form.
    pub fn curvature_data(
        self,
        radius: f64,
    ) -> Result<BoulwareDeserCurvatureData5D, ReferenceError> {
        if !radius.is_finite() || radius <= 0.0 {
            return Err(ReferenceError::InvalidRadius(radius));
        }
        if radius < self.horizon_radius {
            return Err(ReferenceError::RadiusInsideHorizon {
                radius,
                horizon_radius: self.horizon_radius,
            });
        }
        let r_h_sq = self.horizon_radius.powi(2);
        let alpha = self.alpha;
        let amplitude = alpha + 2.0 * r_h_sq;
        let radical = (1.0 + alpha * amplitude / radius.powi(4)).sqrt();
        let lapse = 1.0 - amplitude / (radius.powi(2) * (1.0 + radical));
        let lapse_prime =
            2.0 * amplitude / (radius.powi(3) * radical * (1.0 + radical));
        let radical_prime =
            -2.0 * alpha * amplitude / (radical * radius.powi(5));
        let lapse_second = lapse_prime
            * (-3.0 / radius
                - radical_prime * (1.0 + 2.0 * radical)
                    / (radical * (1.0 + radical)));
        let gauss_bonnet_scalar = 12.0 / radius.powi(3)
            * ((lapse - 1.0) * (radius * lapse_second + 2.0 * lapse_prime)
                + radius * lapse_prime.powi(2));
        let values = [lapse, lapse_prime, lapse_second, gauss_bonnet_scalar];
        if values.iter().all(|value| value.is_finite()) {
            Ok(BoulwareDeserCurvatureData5D {
                lapse,
                lapse_prime,
                lapse_second,
                gauss_bonnet_scalar,
            })
        } else {
            Err(ReferenceError::NonFiniteResult("Boulware-Deser curvature data"))
        }
    }

    /// Convert the static Boulware-Deser solution into the common 5D radial
    /// sample used by the time-sector source-balance verifier.
    pub fn radial_sample(
        self,
        radius: f64,
    ) -> Result<RadialMetricSample5D, ReferenceError> {
        let data = self.curvature_data(radius)?;
        Ok(RadialMetricSample5D {
            radius,
            b: data.lapse,
            f: data.lapse,
            g: radius.powi(2),
            h: radius.powi(2),
            w: 0.0,
            b_prime: data.lapse_prime,
            f_prime: data.lapse_prime,
            g_prime: 2.0 * radius,
            h_prime: 2.0 * radius,
            w_prime: 0.0,
        })
    }

    pub fn schwarzschild_tangherlini_limit(self, radius: f64) -> Result<f64, ReferenceError> {
        if !radius.is_finite() || radius <= 0.0 {
            return Err(ReferenceError::InvalidRadius(radius));
        }
        if radius < self.horizon_radius {
            return Err(ReferenceError::RadiusInsideHorizon {
                radius,
                horizon_radius: self.horizon_radius,
            });
        }
        Ok(1.0 - self.horizon_radius.powi(2) / radius.powi(2))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual={actual:.16e}, expected={expected:.16e}, tolerance={tolerance:.3e}"
        );
    }

    fn mp() -> MyersPerry5D {
        MyersPerry5D::new(1.0, 0.33).expect("valid Akribeia MP control")
    }

    fn sample(profile: MyersPerryProfile5D) -> RadialMetricSample5D {
        RadialMetricSample5D {
            radius: profile.radius,
            b: profile.b,
            f: profile.f,
            g: profile.g,
            h: profile.h,
            w: profile.w,
            b_prime: profile.b_prime,
            f_prime: profile.f_prime,
            g_prime: profile.g_prime,
            h_prime: profile.h_prime,
            w_prime: profile.w_prime,
        }
    }

    #[test]
    fn akribeia_myers_perry_horizon_values_match_exact_rationals() {
        let solution = mp();
        let horizon = solution.horizon_data();
        close(solution.q(), 0.1089, 1e-15);
        close(horizon.b1, 3911.0 / 2500.0, 2e-15);
        close(horizon.f1, 15644.0 / 8911.0, 2e-15);
        close(horizon.h_horizon, 10000.0 / 8911.0, 2e-15);
        close(horizon.w1, -294063.0 / 250000.0, 2e-15);
    }

    #[test]
    fn akribeia_myers_perry_asymptotic_parameters_match_exact_rationals() {
        let asymptotic = mp().asymptotic_parameters();
        close(asymptotic.u_mass, -10000.0 / 8911.0, 2e-15);
        close(asymptotic.v_rotation, 1089.0 / 8911.0, 2e-15);
        close(asymptotic.w_angular, 3300.0 / 8911.0, 2e-15);
        close(
            asymptotic.v_rotation,
            -asymptotic.w_angular.powi(2) / asymptotic.u_mass,
            2e-15,
        );
    }

    #[test]
    fn myers_perry_functions_have_correct_horizon_and_asymptotic_limits() {
        let solution = mp();
        let at_horizon = solution.evaluate(1.0).unwrap();
        close(at_horizon.b, 0.0, 1e-15);
        close(at_horizon.f, 0.0, 1e-15);
        close(at_horizon.h, 10000.0 / 8911.0, 2e-15);
        close(at_horizon.w, 0.33, 1e-15);

        let far = solution.evaluate(1e6).unwrap();
        close(far.b, 1.0, 2e-12);
        close(far.f, 1.0, 2e-12);
        close(far.h / 1e12, 1.0, 2e-12);
        close(far.w, 0.0, 2e-20);
    }

    #[test]
    fn myers_perry_radial_derivatives_match_horizon_expansion() {
        let solution = mp();
        let epsilon = 1e-7;
        let profile = solution.evaluate(1.0 + epsilon).unwrap();
        let horizon = solution.horizon_data();
        close(profile.b / epsilon, horizon.b1, 2e-7);
        close(profile.f / epsilon, horizon.f1, 2e-7);
        close((profile.w - 0.33) / epsilon, horizon.w1, 2e-7);
    }

    #[test]
    fn egb_angular_noether_flux_is_constant_on_exact_mp_solution() {
        let solution = mp();
        let expected = -4.0 * solution.asymptotic_parameters().w_angular;
        close(solution.horizon_angular_flux(), expected, 2e-14);
        for radius in [1.01, 1.1, 1.5, 2.0, 3.0, 10.0, 100.0] {
            let profile = solution.evaluate(radius).unwrap();
            let radial_sample = sample(profile);
            let flux = egb_angular_momentum_flux(radial_sample, 0.0).unwrap();
            close(flux, expected, 2e-12);
            close(
                reconstruct_w_prime_from_flux(radial_sample, 0.0, expected).unwrap(),
                profile.w_prime,
                2e-14,
            );
        }
    }

    #[test]
    fn time_sector_current_reduces_to_einstein_constant_on_exact_mp_solution() {
        let solution = mp();
        let expected = 2.0 * solution.asymptotic_parameters().u_mass;
        for radius in [1.01, 1.1, 1.5, 2.0, 3.0, 10.0, 100.0] {
            let profile = solution.evaluate(radius).unwrap();
            let flux = time_sector_radial_flux(sample(profile), 0.0).unwrap();
            close(flux, expected, 3e-12);
            close(
                reconstruct_b_prime_from_time_flux(sample(profile), 0.0, expected).unwrap(),
                profile.b_prime,
                3e-14,
            );
        }

        let horizon = solution.horizon_data();
        let horizon_flux = solution.horizon_radius().powi(2)
            * (horizon.f1 * horizon.h_horizon / horizon.b1).sqrt()
            * (horizon.h_horizon
                * horizon.omega_horizon
                * horizon.w1
                - horizon.b1);
        close(horizon_flux, expected, 3e-14);
    }

    #[test]
    fn time_sector_current_detects_a_mutated_lapse_derivative_in_einstein_limit() {
        let solution = mp();
        let profile = solution.evaluate(2.0).unwrap();
        let mut mutated = sample(profile);
        mutated.b_prime += 0.01;
        let correct = 2.0 * solution.asymptotic_parameters().u_mass;
        let residual = time_sector_radial_flux(mutated, 0.0).unwrap() - correct;
        assert!(residual.abs() > 1e-3);
    }

    #[test]
    fn time_sector_b_prime_reconstruction_matches_einstein_mp() {
        let solution = mp();
        let expected_current = 2.0 * solution.asymptotic_parameters().u_mass;
        for radius in [1.1, 1.5, 2.0, 3.0, 10.0] {
            let profile = solution.evaluate(radius).unwrap();
            let radial_sample = sample(profile);
            let reconstructed = reconstruct_b_prime_from_time_flux(
                radial_sample,
                0.0,
                expected_current,
            )
            .unwrap();
            close(reconstructed, profile.b_prime, 3e-14);
        }
    }

    #[test]
    fn omitted_gauss_bonnet_flux_factor_is_a_detectable_negative_control() {
        let solution = mp();
        let sample0 = sample(solution.evaluate(2.0).unwrap());
        let sample1 = sample(solution.evaluate(3.0).unwrap());
        let wrong0 = egb_angular_momentum_flux(sample0, 0.5).unwrap();
        let wrong1 = egb_angular_momentum_flux(sample1, 0.5).unwrap();
        assert!((wrong0 - wrong1).abs() > 1e-4);
    }

    #[test]
    fn rotational_flux_quadrature_requires_nonzero_denominator() {
        let radial_sample = sample(mp().evaluate(2.0).unwrap());
        let alpha = radial_sample.radius.powi(2)
            / (radial_sample.f - 4.0
                + 3.0 * radial_sample.h / radial_sample.radius.powi(2));
        let error =
            reconstruct_w_prime_from_flux(radial_sample, alpha, -1.0).unwrap_err();
        assert!(matches!(error, ReferenceError::FluxDenominatorNearZero { .. }));
        let mut invalid = radial_sample;
        invalid.b = f64::NAN;
        assert!(matches!(
            egb_angular_momentum_flux(invalid, 0.0),
            Err(ReferenceError::InvalidMetricSample(_))
        ));
    }

    #[test]
    fn nonextremal_domain_is_fail_closed() {
        assert!(matches!(
            MyersPerry5D::new(1.0, 0.8),
            Err(ReferenceError::ExtremalOrSuperextremal { .. })
        ));
        assert!(MyersPerry5D::new(1.0, -0.33).is_ok());
    }

    #[test]
    fn b_d_curvature_scalar_has_the_schwarzschild_tangherlini_limit() {
        let bd = BoulwareDeser5D::new(1.0, 0.0).unwrap();
        for radius in [1.1, 1.5, 2.0, 3.0, 5.0] {
            let data = bd.curvature_data(radius).unwrap();
            close(
                data.gauss_bonnet_scalar,
                72.0 / radius.powi(8),
                2e-12,
            );
        }
    }

    #[test]
    fn corrected_time_sector_current_satisfies_b_d_source_balance() {
        let alpha = 0.7;
        let bd = BoulwareDeser5D::new(1.0, alpha).unwrap();
        let step = 1e-5;
        for radius in [1.1, 1.5, 2.0, 3.0] {
            let sample = bd.radial_sample(radius).unwrap();
            let current = time_sector_radial_flux(sample, alpha).unwrap();
            let reconstructed_b_prime =
                reconstruct_b_prime_from_time_flux(sample, alpha, current).unwrap();
            close(reconstructed_b_prime, sample.b_prime, 2e-12);
            let current_plus =
                time_sector_radial_flux(bd.radial_sample(radius + step).unwrap(), alpha)
                    .unwrap();
            let current_minus =
                time_sector_radial_flux(bd.radial_sample(radius - step).unwrap(), alpha)
                    .unwrap();
            let derivative = (current_plus - current_minus) / (2.0 * step);
            let source = time_sector_current_source(
                sample,
                alpha,
                bd.curvature_data(radius).unwrap().gauss_bonnet_scalar,
            )
            .unwrap();
            close(derivative, source, 2e-8);
        }

        // The finite-alpha current is explicitly not constant. Its asymptotic
        // value is 2U, with U=-(r_H^2+alpha/2) in this convention.
        let near = time_sector_radial_flux(bd.radial_sample(1.5).unwrap(), alpha).unwrap();
        let far = time_sector_radial_flux(bd.radial_sample(1000.0).unwrap(), alpha).unwrap();
        let asymptotic = -2.0 * (bd.horizon_radius.powi(2) + alpha / 2.0);
        assert!((near - far).abs() > 1e-3);
        close(far, asymptotic, 2e-5);
    }

    #[test]
    fn static_boulware_deser_reduces_to_schwarzschild_tangherlini() {
        let solution = BoulwareDeser5D::new(1.0, 0.7).unwrap();
        close(solution.lapse(1.0).unwrap(), 0.0, 2e-15);
        for radius in [1.1, 1.5, 2.0, 4.0, 10.0] {
            let einstein = 1.0 - 1.0 / radius.powi(2);
            let exact = BoulwareDeser5D::new(1.0, 1e-8)
                .unwrap()
                .lapse(radius)
                .unwrap();
            close(exact, einstein, 2e-8);
        }
    }
}
