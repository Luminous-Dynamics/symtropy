// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Family-specific Kerr optical coordinates and QNM convergence domains.
//!
//! This module implements the oblate/prolate optical bounce construction of
//! Arnaudo & Withers, arXiv:2611.03791 (2026). It is a convergence-domain
//! reference, not a Teukolsky solver or a finite-overtone remainder estimate.
//!
//! Public lengths are SI metres and elapsed time is seconds. The formulas are
//! evaluated in units of the gravitational radius r_g = GM/c^2. Kerr spin is
//! signed by the existing KerrBlackHole API, but the optical bounce geometry
//! depends only on |a|.

use super::{KerrBlackHole, SPEED_OF_LIGHT_M_S};
use core::fmt;

const SCHWARZSCHILD_BOUNCE_OVER_RG: f64 = 2.556_929_085_522_148;
const SQRT_3_OVER_2: f64 = 0.866_025_403_784_438_6;
const REAL_QUADRATURE_TOLERANCE: f64 = 2.0e-14;
const COMPLEX_QUADRATURE_TOLERANCE: f64 = 2.0e-13;
const MAX_QUADRATURE_SEGMENTS: usize = 8192;

/// Fail-closed errors for the Kerr optical convergence reference.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KerrOpticalError {
    NonFiniteInput(&'static str),
    RadiusAtOrInsideHorizon {
        radius_m: f64,
        horizon_m: f64,
    },
    ExtremalSpinUnsupported,
    SourceEventAfterObservation {
        source_time_s: f64,
        observation_time_s: f64,
    },
    SourceEventsNotStrictlyChronological {
        index: usize,
        previous_time_s: f64,
        source_time_s: f64,
    },
    RootNotBracketed,
    RootNotConverged,
    QuadratureNotConverged,
    NonFiniteResult(&'static str),
}

impl fmt::Display for KerrOpticalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteInput(name) => write!(f, "{name} must be finite"),
            Self::RadiusAtOrInsideHorizon {
                radius_m,
                horizon_m,
            } => write!(
                f,
                "Boyer-Lindquist radius {radius_m} m must exceed outer horizon {horizon_m} m"
            ),
            Self::ExtremalSpinUnsupported => write!(
                f,
                "exact extremality is outside the non-extremal optical-coordinate formula"
            ),
            Self::SourceEventAfterObservation {
                source_time_s,
                observation_time_s,
            } => write!(
                f,
                "source event at t={source_time_s} s is later than observation at t={observation_time_s} s"
            ),
            Self::SourceEventsNotStrictlyChronological {
                index,
                previous_time_s,
                source_time_s,
            } => write!(
                f,
                "source event {index} at t={source_time_s} s is not later than preceding event at t={previous_time_s} s"
            ),
            Self::RootNotBracketed => write!(f, "Kerr optical bounce root was not bracketed"),
            Self::RootNotConverged => write!(f, "Kerr optical bounce root did not converge"),
            Self::QuadratureNotConverged => {
                write!(
                    f,
                    "Kerr prolate optical-coordinate quadrature did not converge"
                )
            }
            Self::NonFiniteResult(name) => {
                write!(f, "computed {name} is non-finite or outside the f64 range")
            }
        }
    }
}

impl std::error::Error for KerrOpticalError {}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ComplexCoordinate {
    re: f64,
    im: f64,
}

impl ComplexCoordinate {
    const fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    fn add(self, other: Self) -> Self {
        Self::new(self.re + other.re, self.im + other.im)
    }

    fn sub(self, other: Self) -> Self {
        Self::new(self.re - other.re, self.im - other.im)
    }

    fn neg(self) -> Self {
        Self::new(-self.re, -self.im)
    }

    fn scale(self, factor: f64) -> Self {
        Self::new(self.re * factor, self.im * factor)
    }

    fn mul(self, other: Self) -> Self {
        Self::new(
            self.re * other.re - self.im * other.im,
            self.re * other.im + self.im * other.re,
        )
    }

    fn div(self, other: Self) -> Self {
        let denominator = other.re * other.re + other.im * other.im;
        Self::new(
            (self.re * other.re + self.im * other.im) / denominator,
            (self.im * other.re - self.re * other.im) / denominator,
        )
    }

    fn norm(self) -> f64 {
        self.re.hypot(self.im)
    }

    fn sqrt(self) -> Self {
        let radius = self.norm();
        let re = ((radius + self.re) * 0.5).max(0.0).sqrt();
        let im_magnitude = ((radius - self.re) * 0.5).max(0.0).sqrt();
        let im = if self.im.is_sign_negative() {
            -im_magnitude
        } else {
            im_magnitude
        };
        Self::new(re, im)
    }

    fn ln(self) -> Self {
        Self::new(self.norm().ln(), self.im.atan2(self.re))
    }

    /// Principal log(1+z), using a series near z=0 to avoid cancellation.
    fn ln_1p(self) -> Self {
        if self.norm() >= 0.25 {
            return Self::new(1.0, 0.0).add(self).ln();
        }

        let mut power = self;
        let mut sum = self;
        for n in 2..=96 {
            power = power.mul(self);
            let sign = if n % 2 == 0 { -1.0 } else { 1.0 };
            let term = power.scale(sign / n as f64);
            sum = sum.add(term);
            if term.norm() <= 2.0 * f64::EPSILON * sum.norm().max(1.0) {
                break;
            }
        }
        sum
    }

    fn finite(self) -> bool {
        self.re.is_finite() && self.im.is_finite()
    }
}

/// Numerical reference data for the two Kerr QNM-family optical geometries.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrOpticalReference {
    pub mass_kg: f64,
    pub dimensionless_spin_abs: f64,
    pub gravitational_radius_m: f64,
    pub outer_horizon_radius_m: f64,
    pub inner_horizon_radius_m: f64,
    pub turning_point_re_m: f64,
    pub turning_point_im_m: f64,
    /// Re r_*(i a), the oblate complex-time bounce coordinate.
    pub oblate_bounce_coordinate_m: f64,
    /// Re x(r_c), the prolate complex-time bounce coordinate.
    pub prolate_bounce_coordinate_m: f64,
    /// Im x(r_c), retained for the prolate complex-time period check.
    pub prolate_turning_point_im_coordinate_m: f64,
    /// Oblate large-overtone damping spacing, equal to the outer surface-gravity rate.
    pub oblate_asymptotic_frequency_spacing_s_inv: f64,
    /// Prolate large-overtone damping spacing from pi/(2 Im x(r_c)).
    pub prolate_asymptotic_frequency_spacing_s_inv: f64,
    pub oblate_bounce_radius_m: f64,
    pub prolate_bounce_radius_m: f64,
    /// Composite-Simpson intervals used for x(r_c).
    pub prolate_complex_quadrature_segments: usize,
    /// Difference between the last two nested Simpson estimates, in metres.
    pub prolate_complex_quadrature_error_m: f64,
}

impl KerrOpticalReference {
    /// Construct the family-specific optical coordinates and bounce radii.
    ///
    /// The exact extremal case is rejected because r_* and the horizon
    /// logarithms must be treated by a separate extremal limiting derivation.
    pub fn new(black_hole: KerrBlackHole) -> Result<Self, KerrOpticalError> {
        let mass_kg = black_hole.mass().get();
        let rg = black_hole.gravitational_radius_m();
        let spin = black_hole.spin().get().abs();

        if !mass_kg.is_finite() || !rg.is_finite() || rg <= 0.0 {
            return Err(KerrOpticalError::NonFiniteResult("gravitational radius"));
        }
        if spin == 1.0 {
            return Err(KerrOpticalError::ExtremalSpinUnsupported);
        }

        let discriminant_root = ((1.0 - spin) * (1.0 + spin)).sqrt();
        let x_plus = 1.0 + discriminant_root;
        // Rationalized form avoids cancellation when the spin is small.
        let x_minus = spin * spin / x_plus;
        let horizon_gap = x_plus - x_minus;
        if !horizon_gap.is_finite() || horizon_gap <= 0.0 {
            return Err(KerrOpticalError::NonFiniteResult("horizon separation"));
        }

        let outer_horizon_radius_m = rg * x_plus;
        let inner_horizon_radius_m = rg * x_minus;

        if spin == 0.0 {
            let bounce = rg * SCHWARZSCHILD_BOUNCE_OVER_RG;
            return Ok(Self {
                mass_kg,
                dimensionless_spin_abs: 0.0,
                gravitational_radius_m: rg,
                outer_horizon_radius_m,
                inner_horizon_radius_m,
                turning_point_re_m: 0.0,
                turning_point_im_m: 0.0,
                oblate_bounce_coordinate_m: 0.0,
                prolate_bounce_coordinate_m: 0.0,
                prolate_turning_point_im_coordinate_m: 2.0 * core::f64::consts::PI * rg,
                oblate_asymptotic_frequency_spacing_s_inv: black_hole.surface_gravity_rate_s_inv(),
                prolate_asymptotic_frequency_spacing_s_inv: SPEED_OF_LIGHT_M_S / (4.0 * rg),
                oblate_bounce_radius_m: bounce,
                prolate_bounce_radius_m: bounce,
                prolate_complex_quadrature_segments: 0,
                prolate_complex_quadrature_error_m: 0.0,
            });
        }

        let turning_point = prolate_turning_point(spin);
        let oblate_coordinate =
            oblate_complex_bounce_coordinate(spin, x_plus, x_minus, horizon_gap);
        let (prolate_coordinate, quadrature_segments, quadrature_error) =
            prolate_complex_bounce_coordinate(spin, turning_point, x_plus, x_minus, horizon_gap)?;

        let oblate_bounce = solve_oblate_bounce(x_plus, x_minus, horizon_gap, oblate_coordinate)?;
        let prolate_bounce =
            solve_prolate_bounce(spin, x_plus, x_minus, horizon_gap, prolate_coordinate.re)?;

        let result = Self {
            mass_kg,
            dimensionless_spin_abs: spin,
            gravitational_radius_m: rg,
            outer_horizon_radius_m,
            inner_horizon_radius_m,
            turning_point_re_m: turning_point.re * rg,
            turning_point_im_m: turning_point.im * rg,
            oblate_bounce_coordinate_m: oblate_coordinate * rg,
            prolate_bounce_coordinate_m: prolate_coordinate.re * rg,
            prolate_turning_point_im_coordinate_m: prolate_coordinate.im * rg,
            oblate_asymptotic_frequency_spacing_s_inv: black_hole.surface_gravity_rate_s_inv(),
            prolate_asymptotic_frequency_spacing_s_inv: core::f64::consts::PI * SPEED_OF_LIGHT_M_S
                / (2.0 * prolate_coordinate.im * rg),
            oblate_bounce_radius_m: oblate_bounce * rg,
            prolate_bounce_radius_m: prolate_bounce * rg,
            prolate_complex_quadrature_segments: quadrature_segments,
            prolate_complex_quadrature_error_m: quadrature_error * rg,
        };

        let values = [
            result.turning_point_re_m,
            result.turning_point_im_m,
            result.oblate_bounce_coordinate_m,
            result.prolate_bounce_coordinate_m,
            result.prolate_turning_point_im_coordinate_m,
            result.oblate_asymptotic_frequency_spacing_s_inv,
            result.prolate_asymptotic_frequency_spacing_s_inv,
            result.oblate_bounce_radius_m,
            result.prolate_bounce_radius_m,
            result.prolate_complex_quadrature_error_m,
        ];
        if values.iter().all(|value| value.is_finite()) {
            Ok(result)
        } else {
            Err(KerrOpticalError::NonFiniteResult(
                "optical coordinates or bounce radii",
            ))
        }
    }

    /// Standard Kerr tortoise coordinate r_*(r), in metres.
    ///
    /// The grouped log1p form is algebraically equivalent to the two-horizon
    /// expression and is more stable near extremality.
    pub fn oblate_coordinate_m(self, radius_m: f64) -> Result<f64, KerrOpticalError> {
        let r = self.validate_exterior_radius(radius_m)?;
        let (x_plus, x_minus, gap) = self.dimensionless_horizons();
        Ok(self.gravitational_radius_m
            * oblate_tortoise_dimensionless(r / self.gravitational_radius_m, x_plus, x_minus, gap))
    }

    /// Prolate optical tortoise coordinate x(r), in metres, normalized by
    /// x-r_* -> 0 at spatial infinity.
    pub fn prolate_coordinate_m(self, radius_m: f64) -> Result<f64, KerrOpticalError> {
        let r = self.validate_exterior_radius(radius_m)?;
        let (x_plus, x_minus, gap) = self.dimensionless_horizons();
        let dimensionless_radius = r / self.gravitational_radius_m;
        let r_star = oblate_tortoise_dimensionless(dimensionless_radius, x_plus, x_minus, gap);
        let correction =
            prolate_real_correction(self.dimensionless_spin_abs, 1.0 / dimensionless_radius)?;
        let value = self.gravitational_radius_m * (r_star + correction);
        if value.is_finite() {
            Ok(value)
        } else {
            Err(KerrOpticalError::NonFiniteResult(
                "prolate optical coordinate",
            ))
        }
    }

    /// Evaluate the independent oblate and prolate QNM convergence margins.
    ///
    /// Positive margin means the corresponding QNM family is in its
    /// asymptotic absolute-convergence domain. The complete QNM contribution
    /// is qualified only when both margins are positive. This is not a
    /// finite-overtone truncation or rounding-error certificate.
    pub fn convergence_margins(
        self,
        delta_time_s: f64,
        observer_radius_m: f64,
        source_radius_m: f64,
        boundary_tolerance_m: f64,
    ) -> Result<KerrQnmConvergenceEvaluation, KerrOpticalError> {
        if !delta_time_s.is_finite() {
            return Err(KerrOpticalError::NonFiniteInput("delta_time_s"));
        }
        if !boundary_tolerance_m.is_finite() || boundary_tolerance_m <= 0.0 {
            return Err(KerrOpticalError::NonFiniteInput(
                "positive boundary_tolerance_m",
            ));
        }

        let observer_oblate = self.oblate_coordinate_m(observer_radius_m)?;
        let source_oblate = self.oblate_coordinate_m(source_radius_m)?;
        let observer_prolate = self.prolate_coordinate_m(observer_radius_m)?;
        let source_prolate = self.prolate_coordinate_m(source_radius_m)?;
        let elapsed_length = SPEED_OF_LIGHT_M_S * delta_time_s;

        if !elapsed_length.is_finite() {
            return Err(KerrOpticalError::NonFiniteResult(
                "elapsed coordinate-time length",
            ));
        }

        let oblate_distance = (observer_oblate - self.oblate_bounce_coordinate_m).abs()
            + (source_oblate - self.oblate_bounce_coordinate_m).abs();
        let prolate_distance = (observer_prolate - self.prolate_bounce_coordinate_m).abs()
            + (source_prolate - self.prolate_bounce_coordinate_m).abs();
        let oblate_margin = elapsed_length - oblate_distance;
        let prolate_margin = elapsed_length - prolate_distance;

        if !oblate_margin.is_finite() || !prolate_margin.is_finite() {
            return Err(KerrOpticalError::NonFiniteResult("QNM convergence margin"));
        }

        let near_oblate = oblate_margin.abs() <= boundary_tolerance_m;
        let near_prolate = prolate_margin.abs() <= boundary_tolerance_m;
        let region = if near_oblate || near_prolate {
            KerrQnmConvergenceRegion::TransitionUnresolved
        } else if oblate_margin > 0.0 && prolate_margin > 0.0 {
            KerrQnmConvergenceRegion::BothFamiliesConvergent
        } else if oblate_margin > 0.0 {
            KerrQnmConvergenceRegion::OblateFamilyOnlyConvergent
        } else if prolate_margin > 0.0 {
            KerrQnmConvergenceRegion::ProlateFamilyOnlyConvergent
        } else {
            KerrQnmConvergenceRegion::NeitherFamilyConvergent
        };

        Ok(KerrQnmConvergenceEvaluation {
            region,
            elapsed_coordinate_time_length_m: elapsed_length,
            oblate_margin_m: oblate_margin,
            prolate_margin_m: prolate_margin,
            boundary_tolerance_m,
        })
    }

    /// Classify an extended source history event-by-event against the
    /// prolate and oblate QNM convergence domains.
    ///
    /// The input order is preserved. This method evaluates geometry and
    /// coordinate-time margins only: it does not weight, evolve or sum source
    /// amplitudes. Dynamic activation and the physical source convolution
    /// remain owned by the source-history / Green-function layers.
    pub fn classify_source_events(
        self,
        observation_time_s: f64,
        observer_radius_m: f64,
        source_events: &[KerrQnmSourceEvent],
        boundary_tolerance_m: f64,
    ) -> Result<Vec<KerrQnmSourceEventQualification>, KerrOpticalError> {
        if !observation_time_s.is_finite() {
            return Err(KerrOpticalError::NonFiniteInput("observation_time_s"));
        }

        let mut qualifications = Vec::with_capacity(source_events.len());
        let mut previous_source_time_s: Option<f64> = None;
        for (index, event) in source_events.iter().enumerate() {
            if !event.coordinate_time_s.is_finite() {
                return Err(KerrOpticalError::NonFiniteInput(
                    "source_event.coordinate_time_s",
                ));
            }
            if event.coordinate_time_s > observation_time_s {
                return Err(KerrOpticalError::SourceEventAfterObservation {
                    source_time_s: event.coordinate_time_s,
                    observation_time_s,
                });
            }
            if let Some(previous_time_s) = previous_source_time_s
                && event.coordinate_time_s <= previous_time_s
            {
                return Err(KerrOpticalError::SourceEventsNotStrictlyChronological {
                    index,
                    previous_time_s,
                    source_time_s: event.coordinate_time_s,
                });
            }
            previous_source_time_s = Some(event.coordinate_time_s);

            let convergence = self.convergence_margins(
                observation_time_s - event.coordinate_time_s,
                observer_radius_m,
                event.radius_m,
                boundary_tolerance_m,
            )?;
            qualifications.push(KerrQnmSourceEventQualification {
                source_time_s: event.coordinate_time_s,
                source_radius_m: event.radius_m,
                convergence,
            });
        }

        Ok(qualifications)
    }

    fn dimensionless_horizons(self) -> (f64, f64, f64) {
        let d = ((1.0 - self.dimensionless_spin_abs) * (1.0 + self.dimensionless_spin_abs)).sqrt();
        let plus = 1.0 + d;
        let minus = self.dimensionless_spin_abs.powi(2) / plus;
        (plus, minus, plus - minus)
    }

    fn validate_exterior_radius(self, radius_m: f64) -> Result<f64, KerrOpticalError> {
        if !radius_m.is_finite() {
            return Err(KerrOpticalError::NonFiniteInput("radius_m"));
        }
        if radius_m <= self.outer_horizon_radius_m {
            return Err(KerrOpticalError::RadiusAtOrInsideHorizon {
                radius_m,
                horizon_m: self.outer_horizon_radius_m,
            });
        }
        Ok(radius_m)
    }
}

/// A point on an already-defined source trajectory, in Boyer-Lindquist
/// coordinate time (seconds) and areal radius (metres).
///
/// This is a kinematic query input only; source amplitudes and field content
/// belong to the source-history and Green-function owners.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrQnmSourceEvent {
    pub coordinate_time_s: f64,
    pub radius_m: f64,
}

/// The family-domain qualification for one source-trajectory event and a
/// fixed observation event.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrQnmSourceEventQualification {
    pub source_time_s: f64,
    pub source_radius_m: f64,
    pub convergence: KerrQnmConvergenceEvaluation,
}

/// Family-specific convergence region for the full QNM contribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KerrQnmConvergenceRegion {
    BothFamiliesConvergent,
    OblateFamilyOnlyConvergent,
    ProlateFamilyOnlyConvergent,
    NeitherFamilyConvergent,
    TransitionUnresolved,
}

/// The two family margins must remain visible; a single combined boolean
/// would hide which asymptotic sector limits the complete QNM sum.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrQnmConvergenceEvaluation {
    pub region: KerrQnmConvergenceRegion,
    pub elapsed_coordinate_time_length_m: f64,
    pub oblate_margin_m: f64,
    pub prolate_margin_m: f64,
    pub boundary_tolerance_m: f64,
}

impl KerrQnmConvergenceEvaluation {
    /// True only in the intersection of the prolate and oblate convergence
    /// half-planes, away from the caller-declared transition tolerance band.
    pub const fn complete_qnm_contribution_convergent(self) -> bool {
        matches!(
            self.region,
            KerrQnmConvergenceRegion::BothFamiliesConvergent
        )
    }
}

fn oblate_tortoise_dimensionless(radius: f64, x_plus: f64, x_minus: f64, gap: f64) -> f64 {
    radius
        + 2.0
            * (((radius - x_minus) * 0.5).ln()
                + (x_plus / gap) * (-gap / (radius - x_minus)).ln_1p())
}

fn oblate_complex_bounce_coordinate(spin: f64, x_plus: f64, x_minus: f64, gap: f64) -> f64 {
    // The exact Schwarzschild limit is r_*(0)=0 in the standard convention.
    if spin <= 1.0e-8 {
        return 0.0;
    }

    let minus_norm = x_minus.hypot(spin);
    let plus_norm = x_plus.hypot(spin);
    // (plus_norm-minus_norm)/minus_norm is rationalized to avoid
    // subtractive cancellation near extremality.
    let log_ratio = (2.0 * gap / (minus_norm * (plus_norm + minus_norm))).ln_1p();
    2.0 * ((minus_norm * 0.5).ln() + (x_plus / gap) * log_ratio)
}

fn prolate_turning_point(spin: f64) -> ComplexCoordinate {
    if spin == 0.0 {
        return ComplexCoordinate::new(0.0, 0.0);
    }
    // Cardano for z^3 + chi^2 z + 2 chi^2 = 0. The rationalized
    // positive cube-root term avoids sqrt(1+eps)-1 cancellation at low spin.
    let q = (1.0 + spin * spin / 27.0).sqrt();
    let u = spin.powf(4.0 / 3.0) / (27.0 * (q + 1.0)).powf(1.0 / 3.0);
    let v = -(spin * spin * (1.0 + q)).powf(1.0 / 3.0);
    ComplexCoordinate::new(-0.5 * (u + v), SQRT_3_OVER_2 * (u - v))
}

fn oblate_complex_tortoise(
    radius: ComplexCoordinate,
    x_plus: f64,
    x_minus: f64,
    gap: f64,
) -> ComplexCoordinate {
    let z = radius.sub(ComplexCoordinate::new(x_minus, 0.0));
    let logarithmic_terms = z.div(ComplexCoordinate::new(2.0, 0.0)).ln().add(
        ComplexCoordinate::new(-gap, 0.0)
            .div(z)
            .ln_1p()
            .scale(x_plus / gap),
    );
    radius.add(logarithmic_terms.scale(2.0))
}

fn integrate_real_simpson(spin: f64, endpoint: f64, segments: usize) -> f64 {
    let h = endpoint / segments as f64;
    let integrand = |y: f64| {
        let alpha2 = spin * spin;
        alpha2 / ((1.0 + alpha2 * y * y + 2.0 * alpha2 * y * y * y).sqrt() + 1.0 + alpha2 * y * y)
    };
    let mut sum = integrand(0.0) + integrand(endpoint);
    for index in 1..segments {
        let value = integrand(index as f64 * h);
        sum += if index % 2 == 0 {
            2.0 * value
        } else {
            4.0 * value
        };
    }
    sum * h / 3.0
}

fn prolate_real_correction(spin: f64, endpoint: f64) -> Result<f64, KerrOpticalError> {
    if spin == 0.0 {
        return Ok(0.0);
    }
    let mut previous = integrate_real_simpson(spin, endpoint, 64);
    for segments in [128_usize, 256, 512, 1024, 2048, 4096] {
        let current = integrate_real_simpson(spin, endpoint, segments);
        if (current - previous).abs() <= REAL_QUADRATURE_TOLERANCE * (1.0 + current.abs()) {
            return Ok(current);
        }
        previous = current;
    }
    Err(KerrOpticalError::QuadratureNotConverged)
}

fn integrate_complex_simpson(
    spin: f64,
    endpoint: ComplexCoordinate,
    segments: usize,
) -> ComplexCoordinate {
    // The direct t parameter has a square-root endpoint at t=1. Use
    // t=1-s^2 so the Jacobian 2s regularizes the integrand at the turning
    // point. Track the square-root sheet from s=1 (infinity, root=+1)
    // backwards to s=0 (the complex turning point).
    let h = 1.0 / segments as f64;
    let alpha2 = spin * spin;
    let mut values = vec![ComplexCoordinate::new(0.0, 0.0); segments + 1];
    let mut previous_sqrt: Option<ComplexCoordinate> = None;

    for index in (0..=segments).rev() {
        let s = index as f64 * h;
        let t = 1.0 - s * s;
        let y = endpoint.scale(t);
        let y2 = y.mul(y);
        let y3 = y2.mul(y);
        let polynomial = ComplexCoordinate::new(1.0, 0.0)
            .add(y2.scale(alpha2))
            .add(y3.scale(2.0 * alpha2));
        let mut root = polynomial.sqrt();
        if let Some(previous) = previous_sqrt
            && root.sub(previous).norm() > root.neg().sub(previous).norm()
        {
            root = root.neg();
        }
        previous_sqrt = Some(root);

        let denominator = root
            .add(ComplexCoordinate::new(1.0, 0.0))
            .add(y2.scale(alpha2));
        values[index] = ComplexCoordinate::new(2.0 * s * alpha2, 0.0).div(denominator);
    }

    let mut sum = values[0].add(values[segments]);
    for (index, value) in values.iter().enumerate().take(segments).skip(1) {
        let weight = if index % 2 == 0 { 2.0 } else { 4.0 };
        sum = sum.add(value.scale(weight));
    }

    endpoint.mul(sum).scale(h / 3.0)
}

fn prolate_complex_correction(
    spin: f64,
    turning_point: ComplexCoordinate,
) -> Result<(ComplexCoordinate, usize, f64), KerrOpticalError> {
    if spin == 0.0 {
        return Ok((ComplexCoordinate::new(0.0, 0.0), 0, 0.0));
    }
    if !turning_point.finite() || turning_point.norm() == 0.0 {
        return Err(KerrOpticalError::NonFiniteResult(
            "prolate complex turning point",
        ));
    }

    let endpoint = ComplexCoordinate::new(1.0, 0.0).div(turning_point);
    let mut previous = integrate_complex_simpson(spin, endpoint, 64);
    let mut segments = 128_usize;
    while segments <= MAX_QUADRATURE_SEGMENTS {
        let current = integrate_complex_simpson(spin, endpoint, segments);
        let error = current.sub(previous).norm();
        if error <= COMPLEX_QUADRATURE_TOLERANCE * (1.0 + current.norm()) {
            return Ok((current, segments, error));
        }
        previous = current;
        segments *= 2;
    }
    Err(KerrOpticalError::QuadratureNotConverged)
}

fn prolate_complex_bounce_coordinate(
    spin: f64,
    turning_point: ComplexCoordinate,
    x_plus: f64,
    x_minus: f64,
    gap: f64,
) -> Result<(ComplexCoordinate, usize, f64), KerrOpticalError> {
    if spin <= 1.0e-8 {
        return Ok((
            ComplexCoordinate::new(0.0, 2.0 * core::f64::consts::PI),
            0,
            0.0,
        ));
    }
    let r_star = oblate_complex_tortoise(turning_point, x_plus, x_minus, gap);
    let (correction, segments, error) = prolate_complex_correction(spin, turning_point)?;
    let value = r_star.add(correction);
    if !value.finite() {
        return Err(KerrOpticalError::NonFiniteResult(
            "complex prolate bounce coordinate",
        ));
    }
    Ok((value, segments, error))
}

fn solve_oblate_bounce(
    x_plus: f64,
    x_minus: f64,
    gap: f64,
    target: f64,
) -> Result<f64, KerrOpticalError> {
    let low_offset = (gap * 1.0e-10).max(64.0 * f64::EPSILON * x_plus);
    let mut low = x_plus + low_offset;
    let mut high = 4.0;
    let function =
        |radius: f64| oblate_tortoise_dimensionless(radius, x_plus, x_minus, gap) - target;
    let f_low = function(low);
    let mut f_high = function(high);
    if !f_low.is_finite() || !f_high.is_finite() {
        return Err(KerrOpticalError::NonFiniteResult(
            "oblate bounce root bracket",
        ));
    }
    for _ in 0..16 {
        if f_low <= 0.0 && f_high >= 0.0 {
            break;
        }
        high *= 2.0;
        f_high = function(high);
    }
    if f_low > 0.0 || f_high < 0.0 {
        return Err(KerrOpticalError::RootNotBracketed);
    }

    for _ in 0..160 {
        let middle = 0.5 * (low + high);
        let f_middle = function(middle);
        if !f_middle.is_finite() {
            return Err(KerrOpticalError::NonFiniteResult("oblate bounce residual"));
        }
        if f_middle > 0.0 {
            high = middle;
        } else {
            low = middle;
        }
        if high - low <= 4.0e-14 * (1.0 + middle.abs()) {
            return Ok(0.5 * (low + high));
        }
    }
    Err(KerrOpticalError::RootNotConverged)
}

fn solve_prolate_bounce(
    spin: f64,
    x_plus: f64,
    x_minus: f64,
    gap: f64,
    target: f64,
) -> Result<f64, KerrOpticalError> {
    let low_offset = (gap * 1.0e-10).max(64.0 * f64::EPSILON * x_plus);
    let mut low = x_plus + low_offset;
    let mut high = 4.0;
    let function = |radius: f64| -> Result<f64, KerrOpticalError> {
        let r_star = oblate_tortoise_dimensionless(radius, x_plus, x_minus, gap);
        let correction = prolate_real_correction(spin, 1.0 / radius)?;
        Ok(r_star + correction - target)
    };

    let f_low = function(low)?;
    let mut f_high = function(high)?;
    if !f_low.is_finite() || !f_high.is_finite() {
        return Err(KerrOpticalError::NonFiniteResult(
            "prolate bounce root bracket",
        ));
    }
    for _ in 0..16 {
        if f_low <= 0.0 && f_high >= 0.0 {
            break;
        }
        high *= 2.0;
        f_high = function(high)?;
    }
    if f_low > 0.0 || f_high < 0.0 {
        return Err(KerrOpticalError::RootNotBracketed);
    }

    for _ in 0..96 {
        let middle = 0.5 * (low + high);
        let f_middle = function(middle)?;
        if !f_middle.is_finite() {
            return Err(KerrOpticalError::NonFiniteResult("prolate bounce residual"));
        }
        if f_middle > 0.0 {
            high = middle;
        } else {
            low = middle;
        }
        if high - low <= 1.0e-12 * (1.0 + middle.abs()) {
            return Ok(0.5 * (low + high));
        }
    }
    Err(KerrOpticalError::RootNotConverged)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DimensionlessSpin, MassKg};

    fn hole(spin: f64) -> KerrBlackHole {
        KerrBlackHole::new(
            MassKg::new(1.988_47e30).unwrap(),
            DimensionlessSpin::new(spin).unwrap(),
        )
    }

    fn relative_close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance * expected.abs().max(1.0),
            "actual={actual:.16e}, expected={expected:.16e}, tolerance={tolerance:e}"
        );
    }

    fn legendre_value_and_derivative(order: usize, z: f64) -> (f64, f64) {
        let mut p_previous = 1.0;
        let mut p_current = z;
        for degree in 2..=order {
            let degree_f64 = degree as f64;
            let next = ((2.0 * degree_f64 - 1.0) * z * p_current
                - (degree_f64 - 1.0) * p_previous)
                / degree_f64;
            p_previous = p_current;
            p_current = next;
        }
        let derivative =
            order as f64 * (z * p_current - p_previous) / (z * z - 1.0);
        (p_current, derivative)
    }

    fn integrate_complex_gauss_legendre(
        spin: f64,
        endpoint: ComplexCoordinate,
        order: usize,
    ) -> ComplexCoordinate {
        assert!(order >= 16 && order.is_multiple_of(2));
        let alpha2 = spin * spin;
        let mut sum = ComplexCoordinate::new(0.0, 0.0);
        let mut previous_sqrt: Option<ComplexCoordinate> = None;

        // Roots are generated from the largest to smallest Legendre abscissa,
        // which traverses the regularized path from spatial infinity toward
        // the complex turning point and keeps square-root sheet tracking
        // directional and deterministic.
        for index in 0..order {
            let mut z = (core::f64::consts::PI * (index as f64 + 0.75)
                / (order as f64 + 0.5))
                .cos();
            for _ in 0..16 {
                let (polynomial, derivative) = legendre_value_and_derivative(order, z);
                let delta = polynomial / derivative;
                z -= delta;
                if delta.abs() <= 2.0 * f64::EPSILON {
                    break;
                }
            }
            let (_, derivative) = legendre_value_and_derivative(order, z);
            let weight = 1.0 / ((1.0 - z * z) * derivative * derivative);
            let s = 0.5 * (z + 1.0);
            let y = endpoint.scale(1.0 - s * s);
            let y2 = y.mul(y);
            let y3 = y2.mul(y);
            let polynomial = ComplexCoordinate::new(1.0, 0.0)
                .add(y2.scale(alpha2))
                .add(y3.scale(2.0 * alpha2));
            let mut root = polynomial.sqrt();
            if let Some(previous) = previous_sqrt
                && root.sub(previous).norm() > root.neg().sub(previous).norm()
            {
                root = root.neg();
            }
            previous_sqrt = Some(root);

            let denominator = root
                .add(ComplexCoordinate::new(1.0, 0.0))
                .add(y2.scale(alpha2));
            let integrand = ComplexCoordinate::new(2.0 * s * alpha2, 0.0).div(denominator);
            // The map from [-1,1] to [0,1] contributes a factor of 1/2.
            sum = sum.add(integrand.scale(0.5 * weight));
        }

        endpoint.mul(sum)
    }

    #[test]
    fn kerr_optical_bounces_reproduce_reference_spin_zero_point_seven() {
        let reference = KerrOpticalReference::new(hole(0.7)).unwrap();
        let rg = reference.gravitational_radius_m;
        relative_close(
            reference.oblate_bounce_radius_m / rg,
            2.496_684_247_950_247_5,
            2.0e-9,
        );
        relative_close(
            reference.prolate_bounce_radius_m / rg,
            2.506_982_739_799_383_2,
            2.0e-9,
        );
        relative_close(
            reference.prolate_bounce_coordinate_m / rg,
            0.339_627_686_616_607_2,
            2.0e-9,
        );
        relative_close(
            reference.prolate_turning_point_im_coordinate_m / rg,
            6.151_216_215_915_6,
            2.0e-9,
        );
    }

    #[test]
    fn kerr_optical_bounces_reproduce_schwarzschild_and_near_extremal_limits() {
        let schwarzschild = KerrOpticalReference::new(hole(0.0)).unwrap();
        relative_close(
            schwarzschild.oblate_bounce_radius_m / schwarzschild.gravitational_radius_m,
            SCHWARZSCHILD_BOUNCE_OVER_RG,
            2.0e-14,
        );
        relative_close(
            schwarzschild.prolate_bounce_radius_m / schwarzschild.gravitational_radius_m,
            SCHWARZSCHILD_BOUNCE_OVER_RG,
            2.0e-14,
        );

        let near_extremal = KerrOpticalReference::new(hole(1.0 - 1.0e-6)).unwrap();
        relative_close(
            near_extremal.oblate_bounce_radius_m / near_extremal.gravitational_radius_m,
            2.414_213_895_706_236,
            2.0e-8,
        );
        relative_close(
            near_extremal.prolate_bounce_radius_m / near_extremal.gravitational_radius_m,
            2.414_213_954_631_704_4,
            2.0e-8,
        );
    }

    #[test]
    fn kerr_optical_geometry_reproduces_published_family_convergence_boundaries() {
        let reference = KerrOpticalReference::new(hole(0.7)).unwrap();
        let rg = reference.gravitational_radius_m;
        let outer = reference
            .convergence_margins(0.0, 4.0 * rg, 3.0 * rg, 1.0e-9 * rg)
            .unwrap();
        // Set dt to the published boundary plus the tolerance, so both
        // margins should be positive and numerically match the reference.
        let expected_oblate = 5.481_912_223_034_183;
        let expected_prolate = 5.352_306_440_780_024_5;
        let actual = reference
            .convergence_margins(
                expected_oblate * rg / SPEED_OF_LIGHT_M_S,
                4.0 * rg,
                3.0 * rg,
                1.0e-8 * rg,
            )
            .unwrap();
        relative_close(actual.oblate_margin_m / rg, 0.0, 1.0e-6);
        relative_close(
            actual.prolate_margin_m / rg,
            expected_oblate - expected_prolate,
            2.0e-8,
        );
        assert_eq!(
            actual.region,
            KerrQnmConvergenceRegion::TransitionUnresolved
        );
        assert!(outer.oblate_margin_m < 0.0 && outer.prolate_margin_m < 0.0);

        let inner = reference
            .convergence_margins(
                1.249_527_693_832_751 * rg / SPEED_OF_LIGHT_M_S,
                2.4 * rg,
                2.3 * rg,
                1.0e-8 * rg,
            )
            .unwrap();
        relative_close(inner.oblate_margin_m / rg, 0.0, 1.0e-6);
        relative_close(
            inner.prolate_margin_m / rg,
            1.249_527_693_832_751 - 1.316_980_558_590_982_6,
            2.0e-8,
        );
        assert_eq!(inner.region, KerrQnmConvergenceRegion::TransitionUnresolved);
    }

    #[test]
    fn kerr_optical_region_requires_both_families_for_full_qnm_convergence() {
        let reference = KerrOpticalReference::new(hole(0.7)).unwrap();
        let rg = reference.gravitational_radius_m;

        let both = reference
            .convergence_margins(
                8.0 * rg / SPEED_OF_LIGHT_M_S,
                4.0 * rg,
                3.0 * rg,
                1.0e-8 * rg,
            )
            .unwrap();
        assert_eq!(
            both.region,
            KerrQnmConvergenceRegion::BothFamiliesConvergent
        );
        assert!(both.complete_qnm_contribution_convergent());

        // At the outer source/observer pair, prolate converges before oblate.
        let prolate_only = reference
            .convergence_margins(
                5.4 * rg / SPEED_OF_LIGHT_M_S,
                4.0 * rg,
                3.0 * rg,
                1.0e-8 * rg,
            )
            .unwrap();
        assert_eq!(
            prolate_only.region,
            KerrQnmConvergenceRegion::ProlateFamilyOnlyConvergent
        );
        assert!(!prolate_only.complete_qnm_contribution_convergent());

        // At the inner pair, oblate converges before prolate.
        let oblate_only = reference
            .convergence_margins(
                1.28 * rg / SPEED_OF_LIGHT_M_S,
                2.4 * rg,
                2.3 * rg,
                1.0e-8 * rg,
            )
            .unwrap();
        assert_eq!(
            oblate_only.region,
            KerrQnmConvergenceRegion::OblateFamilyOnlyConvergent
        );
        assert!(!oblate_only.complete_qnm_contribution_convergent());

        let neither = reference
            .convergence_margins(0.0, 4.0 * rg, 3.0 * rg, 1.0e-8 * rg)
            .unwrap();
        assert_eq!(
            neither.region,
            KerrQnmConvergenceRegion::NeitherFamilyConvergent
        );
    }

    #[test]
    fn kerr_optical_asymptotic_mode_spacings_match_reference_values() {
        let at_point_three = KerrOpticalReference::new(hole(0.3)).unwrap();
        let rg = at_point_three.gravitational_radius_m;
        relative_close(
            at_point_three.prolate_asymptotic_frequency_spacing_s_inv * rg / SPEED_OF_LIGHT_M_S,
            0.252_920_435_361_936_1,
            2.0e-9,
        );
        relative_close(
            at_point_three.oblate_asymptotic_frequency_spacing_s_inv * rg / SPEED_OF_LIGHT_M_S,
            0.244_106_674_538_586_94,
            2.0e-9,
        );

        let at_point_seven = KerrOpticalReference::new(hole(0.7)).unwrap();
        let rg = at_point_seven.gravitational_radius_m;
        relative_close(
            at_point_seven.prolate_asymptotic_frequency_spacing_s_inv * rg / SPEED_OF_LIGHT_M_S,
            0.255_363_536_519_921_5,
            2.0e-9,
        );
        relative_close(
            at_point_seven.oblate_asymptotic_frequency_spacing_s_inv * rg / SPEED_OF_LIGHT_M_S,
            0.208_309_023_320_699,
            2.0e-9,
        );
    }

    #[test]
    fn complex_prolate_turning_point_solves_the_expected_cubic() {
        for spin in [0.01_f64, 0.3, 0.7, 0.99, 0.999_999] {
            let root = prolate_turning_point(spin);
            let cube = root.mul(root).mul(root);
            let residual = cube
                .add(root.scale(spin * spin))
                .add(ComplexCoordinate::new(2.0 * spin * spin, 0.0));
            let scale = 1.0 + cube.norm() + 2.0 * spin * spin;
            assert!(
                residual.norm() / scale < 2.0e-14,
                "spin={spin}, residual={residual:?}"
            );
            assert!(root.re > 0.0 && root.im > 0.0);
        }
    }

    #[test]
    fn optical_coordinate_derivatives_match_their_defining_metrics() {
        let reference = KerrOpticalReference::new(hole(0.7)).unwrap();
        let rg = reference.gravitational_radius_m;
        let radius = 3.2 * rg;
        let step = 1.0e-5 * rg;
        let oblate_derivative = (reference.oblate_coordinate_m(radius + step).unwrap()
            - reference.oblate_coordinate_m(radius - step).unwrap())
            / (2.0 * step);
        let prolate_derivative = (reference.prolate_coordinate_m(radius + step).unwrap()
            - reference.prolate_coordinate_m(radius - step).unwrap())
            / (2.0 * step);

        let x = radius / rg;
        let a = reference.dimensionless_spin_abs;
        let delta = x * x - 2.0 * x + a * a;
        let expected_oblate = (x * x + a * a) / delta;
        let radial_polynomial = x.powi(4) + a * a * x * x + 2.0 * a * a * x;
        let expected_prolate = radial_polynomial.sqrt() / delta;

        relative_close(oblate_derivative, expected_oblate, 2.0e-8);
        relative_close(prolate_derivative, expected_prolate, 2.0e-8);
    }

    #[test]
    fn prolate_complex_turning_point_integral_agrees_with_independent_gauss_legendre() {
        for spin in [0.3_f64, 0.7, 0.99] {
            let turning_point = prolate_turning_point(spin);
            let endpoint = ComplexCoordinate::new(1.0, 0.0).div(turning_point);
            let simpson = prolate_complex_correction(spin, turning_point)
                .unwrap()
                .0;
            let gauss = integrate_complex_gauss_legendre(spin, endpoint, 192);
            let discrepancy = simpson.sub(gauss).norm();
            assert!(
                discrepancy < 5.0e-10 * (1.0 + simpson.norm()),
                "spin={spin}, Simpson={simpson:?}, Gauss-Legendre={gauss:?}, discrepancy={discrepancy:e}"
            );
        }
    }

    #[test]
    fn bounce_radii_satisfy_their_own_optical_coordinate_equations_across_spin() {
        for spin in [0.01_f64, 0.3, 0.7, 0.99, 0.999_999] {
            let reference = KerrOpticalReference::new(hole(spin)).unwrap();
            let rg = reference.gravitational_radius_m;
            let oblate_value = reference
                .oblate_coordinate_m(reference.oblate_bounce_radius_m)
                .unwrap();
            let prolate_value = reference
                .prolate_coordinate_m(reference.prolate_bounce_radius_m)
                .unwrap();
            let oblate_residual = (oblate_value - reference.oblate_bounce_coordinate_m).abs() / rg;
            let prolate_residual =
                (prolate_value - reference.prolate_bounce_coordinate_m).abs() / rg;

            assert!(
                oblate_residual < 2.0e-9,
                "spin={spin}, oblate root residual={oblate_residual:e}"
            );
            assert!(
                prolate_residual < 2.0e-8,
                "spin={spin}, prolate root residual={prolate_residual:e}"
            );
            assert!(
                reference.prolate_bounce_radius_m > reference.oblate_bounce_radius_m,
                "nonzero spin={spin} should split the prolate bounce outside the oblate bounce"
            );
        }
    }

    #[test]
    fn kerr_optical_bounces_are_even_under_spin_reversal() {
        let positive = KerrOpticalReference::new(hole(0.7)).unwrap();
        let negative = KerrOpticalReference::new(hole(-0.7)).unwrap();
        relative_close(
            positive.oblate_bounce_radius_m,
            negative.oblate_bounce_radius_m,
            1.0e-14,
        );
        relative_close(
            positive.prolate_bounce_radius_m,
            negative.prolate_bounce_radius_m,
            1.0e-14,
        );
    }

    #[test]
    fn kerr_source_history_is_partitioned_by_family_domain_per_event() {
        let reference = KerrOpticalReference::new(hole(0.7)).unwrap();
        let rg = reference.gravitational_radius_m;
        let observer_time_s = 10.0;
        let events = [
            KerrQnmSourceEvent {
                coordinate_time_s: observer_time_s - 5.2 * rg / SPEED_OF_LIGHT_M_S,
                radius_m: 3.0 * rg,
            },
            KerrQnmSourceEvent {
                coordinate_time_s: observer_time_s - 5.4 * rg / SPEED_OF_LIGHT_M_S,
                radius_m: 3.0 * rg,
            },
            KerrQnmSourceEvent {
                coordinate_time_s: observer_time_s - 5.5 * rg / SPEED_OF_LIGHT_M_S,
                radius_m: 3.0 * rg,
            },
        ];
        let classified = reference
            .classify_source_events(observer_time_s, 4.0 * rg, &events, 1.0e-8 * rg)
            .unwrap();

        assert_eq!(classified.len(), events.len());
        assert_eq!(
            classified[0].convergence.region,
            KerrQnmConvergenceRegion::NeitherFamilyConvergent
        );
        assert_eq!(
            classified[1].convergence.region,
            KerrQnmConvergenceRegion::ProlateFamilyOnlyConvergent
        );
        assert_eq!(
            classified[2].convergence.region,
            KerrQnmConvergenceRegion::BothFamiliesConvergent
        );
        assert_eq!(
            classified
                .iter()
                .map(|event| event.source_time_s)
                .collect::<Vec<_>>(),
            events
                .iter()
                .map(|event| event.coordinate_time_s)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn kerr_source_history_rejects_future_and_nonfinite_events() {
        let reference = KerrOpticalReference::new(hole(0.7)).unwrap();
        let rg = reference.gravitational_radius_m;
        let future = [KerrQnmSourceEvent {
            coordinate_time_s: 1.1,
            radius_m: 3.0 * rg,
        }];
        assert!(matches!(
            reference.classify_source_events(1.0, 4.0 * rg, &future, 1.0e-8 * rg),
            Err(KerrOpticalError::SourceEventAfterObservation { .. })
        ));

        let nonfinite = [KerrQnmSourceEvent {
            coordinate_time_s: f64::NAN,
            radius_m: 3.0 * rg,
        }];
        assert_eq!(
            reference.classify_source_events(1.0, 4.0 * rg, &nonfinite, 1.0e-8 * rg),
            Err(KerrOpticalError::NonFiniteInput(
                "source_event.coordinate_time_s"
            ))
        );
    }

    #[test]
    fn kerr_source_history_rejects_reversed_and_duplicate_event_times() {
        let reference = KerrOpticalReference::new(hole(0.7)).unwrap();
        let rg = reference.gravitational_radius_m;
        let reversed = [
            KerrQnmSourceEvent {
                coordinate_time_s: 1.0,
                radius_m: 3.0 * rg,
            },
            KerrQnmSourceEvent {
                coordinate_time_s: 0.5,
                radius_m: 3.1 * rg,
            },
        ];
        assert!(matches!(
            reference.classify_source_events(2.0, 4.0 * rg, &reversed, 1.0e-8 * rg),
            Err(KerrOpticalError::SourceEventsNotStrictlyChronological {
                index: 1,
                previous_time_s: 1.0,
                source_time_s: 0.5,
            })
        ));

        let duplicate = [
            KerrQnmSourceEvent {
                coordinate_time_s: 1.0,
                radius_m: 3.0 * rg,
            },
            KerrQnmSourceEvent {
                coordinate_time_s: 1.0,
                radius_m: 3.1 * rg,
            },
        ];
        assert!(matches!(
            reference.classify_source_events(2.0, 4.0 * rg, &duplicate, 1.0e-8 * rg),
            Err(KerrOpticalError::SourceEventsNotStrictlyChronological {
                index: 1,
                previous_time_s: 1.0,
                source_time_s: 1.0,
            })
        ));
    }

    #[test]
    fn kerr_source_history_classification_is_invariant_under_time_translation() {
        let reference = KerrOpticalReference::new(hole(0.7)).unwrap();
        let rg = reference.gravitational_radius_m;
        let events = [
            KerrQnmSourceEvent {
                coordinate_time_s: 10.0,
                radius_m: 3.0 * rg,
            },
            KerrQnmSourceEvent {
                coordinate_time_s: 10.0 + 0.25 * rg / SPEED_OF_LIGHT_M_S,
                radius_m: 3.1 * rg,
            },
        ];
        let observation_time_s = 10.0 + 6.0 * rg / SPEED_OF_LIGHT_M_S;
        let original = reference
            .classify_source_events(
                observation_time_s,
                4.0 * rg,
                &events,
                1.0e-8 * rg,
            )
            .unwrap();

        let shift_s = 1.0;
        let shifted_events = events.map(|event| KerrQnmSourceEvent {
            coordinate_time_s: event.coordinate_time_s + shift_s,
            radius_m: event.radius_m,
        });
        let shifted = reference
            .classify_source_events(
                observation_time_s + shift_s,
                4.0 * rg,
                &shifted_events,
                1.0e-8 * rg,
            )
            .unwrap();

        assert_eq!(original.len(), shifted.len());
        for (before, after) in original.iter().zip(&shifted) {
            assert_eq!(before.convergence.region, after.convergence.region);
            assert!(
                (before.convergence.oblate_margin_m - after.convergence.oblate_margin_m).abs()
                    < 1.0e-7 * rg
            );
            assert!(
                (before.convergence.prolate_margin_m - after.convergence.prolate_margin_m).abs()
                    < 1.0e-7 * rg
            );
        }
    }

    #[test]
    fn kerr_optical_coordinates_reject_horizon_and_nonfinite_radii() {
        let reference = KerrOpticalReference::new(hole(0.7)).unwrap();
        assert!(matches!(
            reference.oblate_coordinate_m(reference.outer_horizon_radius_m),
            Err(KerrOpticalError::RadiusAtOrInsideHorizon { .. })
        ));
        assert!(matches!(
            reference.prolate_coordinate_m(f64::NAN),
            Err(KerrOpticalError::NonFiniteInput("radius_m"))
        ));
    }

    #[test]
    fn exact_extremal_optical_reference_abstains() {
        assert_eq!(
            KerrOpticalReference::new(hole(1.0)),
            Err(KerrOpticalError::ExtremalSpinUnsupported)
        );
    }
}
