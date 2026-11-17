// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Fixed-dimension second-order forward-mode automatic differentiation.
//!
//! Jet2 carries a scalar value, its four first coordinate derivatives, and
//! its 4x4 Hessian. It is used to differentiate analytic metric expressions
//! without finite-difference step sizes. This is a reference implementation,
//! not yet a SIMD / production-performance AD backend.

use core::fmt;

use super::{
    KerrBlackHole, PhysicsDomainError, SchwarzschildBlackHole,
    connection::{ConnectionError, MetricDerivatives4},
    curvature::{CurvatureError, MetricSecondDerivatives4, RiemannTensor4},
    metric::{
        MetricComponents4, MetricError, kerr_exterior_metric, kerr_ingoing_metric,
        schwarzschild_exterior_metric, schwarzschild_ingoing_ef_metric,
    },
};

/// Scalar second-order jet over four spacetime coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Jet2 {
    value: f64,
    first: [f64; 4],
    second: [[f64; 4]; 4],
}

impl Jet2 {
    pub const fn constant(value: f64) -> Self {
        Self {
            value,
            first: [0.0; 4],
            second: [[0.0; 4]; 4],
        }
    }

    pub fn variable(value: f64, coordinate_index: usize) -> Result<Self, JetError> {
        if !value.is_finite() {
            return Err(JetError::NonFiniteInput);
        }
        if coordinate_index >= 4 {
            return Err(JetError::CoordinateIndexOutOfRange(coordinate_index));
        }
        let mut jet = Self::constant(value);
        jet.first[coordinate_index] = 1.0;
        Ok(jet)
    }

    pub const fn value(self) -> f64 {
        self.value
    }

    pub const fn first_derivatives(self) -> [f64; 4] {
        self.first
    }

    pub const fn second_derivatives(self) -> [[f64; 4]; 4] {
        self.second
    }

    pub fn plus(self, other: Self) -> Self {
        let mut result = Self::constant(self.value + other.value);
        for i in 0..4 {
            result.first[i] = self.first[i] + other.first[i];
            for j in 0..4 {
                result.second[i][j] = self.second[i][j] + other.second[i][j];
            }
        }
        result
    }

    pub fn negative(self) -> Self {
        let mut result = Self::constant(-self.value);
        for i in 0..4 {
            result.first[i] = -self.first[i];
            for j in 0..4 {
                result.second[i][j] = -self.second[i][j];
            }
        }
        result
    }

    pub fn minus(self, other: Self) -> Self {
        self.plus(other.negative())
    }

    pub fn times(self, other: Self) -> Self {
        let mut result = Self::constant(self.value * other.value);
        for i in 0..4 {
            result.first[i] = self.first[i] * other.value + self.value * other.first[i];
            for j in 0..4 {
                result.second[i][j] = self.second[i][j] * other.value
                    + self.first[i] * other.first[j]
                    + self.first[j] * other.first[i]
                    + self.value * other.second[i][j];
            }
        }
        result
    }

    pub fn reciprocal(self) -> Result<Self, JetError> {
        if !self.value.is_finite() {
            return Err(JetError::NonFiniteIntermediate);
        }
        if self.value == 0.0 {
            return Err(JetError::DivisionByZero);
        }
        let inverse = 1.0 / self.value;
        let inverse_squared = inverse * inverse;
        let inverse_cubed = inverse_squared * inverse;
        let mut result = Self::constant(inverse);
        for i in 0..4 {
            result.first[i] = -self.first[i] * inverse_squared;
            for j in 0..4 {
                result.second[i][j] = 2.0 * self.first[i] * self.first[j] * inverse_cubed
                    - self.second[i][j] * inverse_squared;
            }
        }
        result.ensure_finite("reciprocal")?;
        Ok(result)
    }

    pub fn over(self, other: Self) -> Result<Self, JetError> {
        self.times(other.reciprocal()?).ensure_finite("division")
    }

    pub fn sin(self) -> Self {
        let sin_value = self.value.sin();
        let cos_value = self.value.cos();
        let mut result = Self::constant(sin_value);
        for i in 0..4 {
            result.first[i] = cos_value * self.first[i];
            for j in 0..4 {
                result.second[i][j] =
                    cos_value * self.second[i][j] - sin_value * self.first[i] * self.first[j];
            }
        }
        result
    }

    pub fn cos(self) -> Self {
        let sin_value = self.value.sin();
        let cos_value = self.value.cos();
        let mut result = Self::constant(cos_value);
        for i in 0..4 {
            result.first[i] = -sin_value * self.first[i];
            for j in 0..4 {
                result.second[i][j] =
                    -sin_value * self.second[i][j] - cos_value * self.first[i] * self.first[j];
            }
        }
        result
    }

    pub fn sqrt(self) -> Result<Self, JetError> {
        if !self.value.is_finite() {
            return Err(JetError::NonFiniteIntermediate);
        }
        if self.value <= 0.0 {
            return Err(JetError::NonPositiveSquareRoot(self.value));
        }
        let root = self.value.sqrt();
        let first_factor = 0.5 / root;
        let second_factor = -0.25 / (self.value * root);
        let mut result = Self::constant(root);
        for i in 0..4 {
            result.first[i] = first_factor * self.first[i];
            for j in 0..4 {
                result.second[i][j] = first_factor * self.second[i][j]
                    + second_factor * self.first[i] * self.first[j];
            }
        }
        result.ensure_finite("square root")?;
        Ok(result)
    }

    pub fn ensure_finite(self, operation: &'static str) -> Result<Self, JetError> {
        if !self.value.is_finite()
            || self.first.iter().any(|value| !value.is_finite())
            || self.second.iter().flatten().any(|value| !value.is_finite())
        {
            Err(JetError::NonFiniteOperation(operation))
        } else {
            Ok(self)
        }
    }
}

/// A metric together with exact / AD-generated first and second derivatives.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DifferentiatedMetric4 {
    pub metric: MetricComponents4,
    pub first: MetricDerivatives4,
    pub second: MetricSecondDerivatives4,
}

impl DifferentiatedMetric4 {
    /// Build a Schwarzschild exterior metric and its first/second derivatives
    /// using second-order automatic differentiation.
    pub fn schwarzschild_exterior(
        hole: SchwarzschildBlackHole,
        radius_m: f64,
        theta_rad: f64,
    ) -> Result<Self, JetError> {
        let _ = schwarzschild_exterior_metric(hole, radius_m, theta_rad)?;
        let r = Jet2::variable(radius_m, 1)?;
        let theta = Jet2::variable(theta_rad, 2)?;
        let rg = Jet2::constant(hole.gravitational_radius_m());
        let two = Jet2::constant(2.0);
        let one = Jet2::constant(1.0);

        let f = one.minus(two.times(rg).times(r.reciprocal()?));
        let r_squared = r.times(r);
        let sin_theta = theta.sin();
        let sin_squared = sin_theta.times(sin_theta);

        let g_tt = f.negative();
        let g_rr = f.reciprocal()?;
        let g_theta_theta = r_squared;
        let g_phi_phi = r_squared.times(sin_squared);

        Self::from_jets([
            [
                g_tt,
                Jet2::constant(0.0),
                Jet2::constant(0.0),
                Jet2::constant(0.0),
            ],
            [
                Jet2::constant(0.0),
                g_rr,
                Jet2::constant(0.0),
                Jet2::constant(0.0),
            ],
            [
                Jet2::constant(0.0),
                Jet2::constant(0.0),
                g_theta_theta,
                Jet2::constant(0.0),
            ],
            [
                Jet2::constant(0.0),
                Jet2::constant(0.0),
                Jet2::constant(0.0),
                g_phi_phi,
            ],
        ])
    }

    /// Build the horizon-regular Schwarzschild ingoing Eddington-Finkelstein
    /// metric and its first/second derivatives using automatic differentiation.
    ///
    /// This chart remains regular at r=2GM/c^2 and is suitable for testing
    /// curvature invariants at and across the future event horizon.
    pub fn schwarzschild_ingoing_ef(
        hole: SchwarzschildBlackHole,
        radius_m: f64,
        theta_rad: f64,
    ) -> Result<Self, JetError> {
        let _ = schwarzschild_ingoing_ef_metric(hole, radius_m, theta_rad)?;
        let r = Jet2::variable(radius_m, 1)?;
        let theta = Jet2::variable(theta_rad, 2)?;
        let rg = Jet2::constant(hole.gravitational_radius_m());
        let two = Jet2::constant(2.0);
        let one = Jet2::constant(1.0);
        let f = one.minus(two.times(rg).times(r.reciprocal()?));
        let r_squared = r.times(r);
        let sin_theta = theta.sin();
        let sin_squared = sin_theta.times(sin_theta);

        let zero = Jet2::constant(0.0);
        let one_cross = Jet2::constant(1.0);
        let g_vv = f.negative();
        let g_vr = one_cross;
        let g_theta_theta = r_squared;
        let g_phi_phi = r_squared.times(sin_squared);

        Self::from_jets([
            [g_vv, g_vr, zero, zero],
            [g_vr, zero, zero, zero],
            [zero, zero, g_theta_theta, zero],
            [zero, zero, zero, g_phi_phi],
        ])
    }

    /// Build a Kerr exterior metric and its first/second derivatives using
    /// second-order automatic differentiation in r and theta.
    pub fn kerr_exterior(
        hole: KerrBlackHole,
        radius_m: f64,
        theta_rad: f64,
    ) -> Result<Self, JetError> {
        let _ = kerr_exterior_metric(hole, radius_m, theta_rad)?;
        let r = Jet2::variable(radius_m, 1)?;
        let theta = Jet2::variable(theta_rad, 2)?;
        let rg = Jet2::constant(hole.gravitational_radius_m());
        let a = Jet2::constant(hole.kerr_parameter_m());
        let a_squared = a.times(a);
        let two = Jet2::constant(2.0);
        let one = Jet2::constant(1.0);

        let sin_theta = theta.sin();
        let cos_theta = theta.cos();
        let sin_squared = sin_theta.times(sin_theta);
        let cos_squared = cos_theta.times(cos_theta);
        let r_squared = r.times(r);

        let sigma = r_squared.plus(a_squared.times(cos_squared));
        let delta = r_squared.minus(two.times(rg).times(r)).plus(a_squared);

        let two_m_r_over_sigma = two.times(rg).times(r).over(sigma)?;
        let g_tt = one.minus(two_m_r_over_sigma).negative();

        let g_t_phi = two
            .times(rg)
            .times(a)
            .times(r)
            .times(sin_squared)
            .over(sigma)?
            .negative();

        let g_rr = sigma.over(delta)?;
        let g_theta_theta = sigma;

        let azimuthal_bracket = r_squared.plus(a_squared).plus(
            two.times(rg)
                .times(a_squared)
                .times(r)
                .times(sin_squared)
                .over(sigma)?,
        );
        let g_phi_phi = sin_squared.times(azimuthal_bracket);

        Self::from_jets([
            [g_tt, Jet2::constant(0.0), Jet2::constant(0.0), g_t_phi],
            [
                Jet2::constant(0.0),
                g_rr,
                Jet2::constant(0.0),
                Jet2::constant(0.0),
            ],
            [
                Jet2::constant(0.0),
                Jet2::constant(0.0),
                g_theta_theta,
                Jet2::constant(0.0),
            ],
            [g_t_phi, Jet2::constant(0.0), Jet2::constant(0.0), g_phi_phi],
        ])
    }

    /// Build the horizon-regular ingoing Kerr metric and its first/second
    /// derivatives using second-order automatic differentiation.
    pub fn kerr_ingoing(
        hole: KerrBlackHole,
        radius_m: f64,
        theta_rad: f64,
    ) -> Result<Self, JetError> {
        let _ = kerr_ingoing_metric(hole, radius_m, theta_rad)?;
        let r = Jet2::variable(radius_m, 1)?;
        let theta = Jet2::variable(theta_rad, 2)?;
        let rg = Jet2::constant(hole.gravitational_radius_m());
        let a = Jet2::constant(hole.kerr_parameter_m());
        let a_squared = a.times(a);
        let two = Jet2::constant(2.0);

        let sin_theta = theta.sin();
        let cos_theta = theta.cos();
        let sin_squared = sin_theta.times(sin_theta);
        let cos_squared = cos_theta.times(cos_theta);
        let r_squared = r.times(r);
        let sigma = r_squared.plus(a_squared.times(cos_squared));

        let g_vv = Jet2::constant(1.0)
            .minus(two.times(rg).times(r).over(sigma)?)
            .negative();
        let g_vr = Jet2::constant(1.0);
        let g_v_phi = two
            .times(rg)
            .times(a)
            .times(r)
            .times(sin_squared)
            .over(sigma)?
            .negative();
        let g_r_phi = a.times(sin_squared).negative();
        let g_theta_theta = sigma;
        let g_phi_phi = sin_squared.times(
            r_squared.plus(a_squared).plus(
                two.times(rg)
                    .times(a_squared)
                    .times(r)
                    .times(sin_squared)
                    .over(sigma)?,
            ),
        );

        let zero = Jet2::constant(0.0);
        Self::from_jets([
            [g_vv, g_vr, zero, g_v_phi],
            [g_vr, zero, zero, g_r_phi],
            [zero, zero, g_theta_theta, zero],
            [g_v_phi, g_r_phi, zero, g_phi_phi],
        ])
    }

    fn from_jets(jets: [[Jet2; 4]; 4]) -> Result<Self, JetError> {
        let mut metric = [[0.0; 4]; 4];
        let mut first = [[[0.0; 4]; 4]; 4];
        let mut second = [[[[0.0; 4]; 4]; 4]; 4];

        for mu in 0..4 {
            for nu in 0..4 {
                let jet = jets[mu][nu].ensure_finite("metric jet")?;
                metric[mu][nu] = jet.value();
                let gradient = jet.first_derivatives();
                let hessian = jet.second_derivatives();
                for alpha in 0..4 {
                    first[alpha][mu][nu] = gradient[alpha];
                    for beta in 0..4 {
                        second[alpha][beta][mu][nu] = hessian[alpha][beta];
                    }
                }
            }
        }

        Ok(Self {
            metric: MetricComponents4::new(metric)?,
            first: MetricDerivatives4::new(first)?,
            second: MetricSecondDerivatives4::new(second)?,
        })
    }

    pub fn curvature(self) -> Result<RiemannTensor4, JetError> {
        Ok(RiemannTensor4::from_metric_derivatives(
            &self.metric,
            &self.first,
            &self.second,
        )?)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JetError {
    NonFiniteInput,
    CoordinateIndexOutOfRange(usize),
    DivisionByZero,
    NonPositiveSquareRoot(f64),
    NonFiniteIntermediate,
    NonFiniteOperation(&'static str),
    Metric(MetricError),
    Connection(ConnectionError),
    Curvature(CurvatureError),
    PhysicsDomain(PhysicsDomainError),
}

impl From<MetricError> for JetError {
    fn from(value: MetricError) -> Self {
        Self::Metric(value)
    }
}
impl From<ConnectionError> for JetError {
    fn from(value: ConnectionError) -> Self {
        Self::Connection(value)
    }
}
impl From<CurvatureError> for JetError {
    fn from(value: CurvatureError) -> Self {
        Self::Curvature(value)
    }
}
impl From<PhysicsDomainError> for JetError {
    fn from(value: PhysicsDomainError) -> Self {
        Self::PhysicsDomain(value)
    }
}

impl fmt::Display for JetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteInput => write!(f, "automatic-differentiation input is non-finite"),
            Self::CoordinateIndexOutOfRange(index) => {
                write!(f, "coordinate index {index} is outside [0, 3]")
            }
            Self::DivisionByZero => write!(f, "automatic-differentiation division by zero"),
            Self::NonPositiveSquareRoot(value) => write!(
                f,
                "automatic-differentiation square root requires positive input, got {value}"
            ),
            Self::NonFiniteIntermediate => write!(
                f,
                "automatic-differentiation encountered a non-finite intermediate"
            ),
            Self::NonFiniteOperation(operation) => write!(
                f,
                "automatic-differentiation {operation} produced non-finite derivatives"
            ),
            Self::Metric(error) => write!(f, "{error}"),
            Self::Connection(error) => write!(f, "{error}"),
            Self::Curvature(error) => write!(f, "{error}"),
            Self::PhysicsDomain(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for JetError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DimensionlessSpin;

    const SOLAR_MASS_KG: f64 = 1.988_47e30;
    fn mass() -> super::super::MassKg {
        super::super::MassKg::new(SOLAR_MASS_KG).unwrap()
    }
    fn close(a: f64, b: f64, tol: f64) {
        let allowed = if b == 0.0 { tol } else { tol * b.abs() };
        assert!(
            (a - b).abs() <= allowed,
            "actual={a:e}, expected={b:e}, allowed={allowed:e}"
        );
    }

    #[test]
    fn jet2_product_and_chain_rules_match_polynomial() {
        let x = Jet2::variable(3.0, 1).unwrap();
        let y = Jet2::variable(2.0, 2).unwrap();
        let f = x.times(x).times(y);
        close(f.value(), 18.0, 1e-14);
        close(f.first_derivatives()[1], 12.0, 1e-14);
        close(f.first_derivatives()[2], 9.0, 1e-14);
        close(f.second_derivatives()[1][1], 4.0, 1e-14);
        close(f.second_derivatives()[1][2], 6.0, 1e-14);
        close(f.second_derivatives()[2][1], 6.0, 1e-14);
    }

    #[test]
    fn jet2_trigonometric_chain_rule_matches_analytic_derivatives() {
        let theta = Jet2::variable(0.7, 2).unwrap();
        let sine = theta.sin();
        let cosine = theta.cos();
        close(sine.value(), 0.7_f64.sin(), 1e-14);
        close(sine.first_derivatives()[2], 0.7_f64.cos(), 1e-14);
        close(sine.second_derivatives()[2][2], -0.7_f64.sin(), 1e-14);
        close(cosine.first_derivatives()[2], -0.7_f64.sin(), 1e-14);
        close(cosine.second_derivatives()[2][2], -0.7_f64.cos(), 1e-14);
    }

    #[test]
    fn autodiff_schwarzschild_derivatives_match_closed_form_fixture() {
        let h = SchwarzschildBlackHole::new(mass());
        let r = 7.0 * h.gravitational_radius_m();
        let theta = 1.1;
        let ad = DifferentiatedMetric4::schwarzschild_exterior(h, r, theta).unwrap();
        let analytic_first = MetricDerivatives4::schwarzschild_exterior(h, r, theta).unwrap();
        let analytic_second =
            MetricSecondDerivatives4::schwarzschild_exterior(h, r, theta).unwrap();
        for a in 0..4 {
            for mu in 0..4 {
                for nu in 0..4 {
                    close(
                        ad.first.components()[a][mu][nu],
                        analytic_first.components()[a][mu][nu],
                        2e-13,
                    );
                    for b in 0..4 {
                        close(
                            ad.second.components()[a][b][mu][nu],
                            analytic_second.components()[a][b][mu][nu],
                            2e-13,
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn zero_spin_kerr_automatic_derivatives_match_schwarzschild() {
        let m = mass();
        let schwarzschild = SchwarzschildBlackHole::new(m);
        let kerr = KerrBlackHole::new(m, DimensionlessSpin::new(0.0).unwrap());
        let r = 8.0 * schwarzschild.gravitational_radius_m();
        let theta = 0.9;
        let s = DifferentiatedMetric4::schwarzschild_exterior(schwarzschild, r, theta).unwrap();
        let k = DifferentiatedMetric4::kerr_exterior(kerr, r, theta).unwrap();
        for mu in 0..4 {
            for nu in 0..4 {
                close(
                    s.metric.covariant()[mu][nu],
                    k.metric.covariant()[mu][nu],
                    2e-13,
                );
                for a in 0..4 {
                    close(
                        s.first.components()[a][mu][nu],
                        k.first.components()[a][mu][nu],
                        3e-12,
                    );
                    for b in 0..4 {
                        close(
                            s.second.components()[a][b][mu][nu],
                            k.second.components()[a][b][mu][nu],
                            3e-11,
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn kerr_autodiff_curvature_is_vacuum_and_matches_analytic_kretschmann() {
        let h = KerrBlackHole::new(mass(), DimensionlessSpin::new(0.8).unwrap());
        let r = 7.0 * h.gravitational_radius_m();
        let theta: f64 = 0.8;
        let differentiated = DifferentiatedMetric4::kerr_exterior(h, r, theta).unwrap();
        let curvature = differentiated.curvature().unwrap();
        let inverse = differentiated.metric.inverse().unwrap();
        let ricci = curvature.ricci_tensor();
        let mut mixed = [[0.0; 4]; 4];
        for mu in 0..4 {
            for nu in 0..4 {
                for alpha in 0..4 {
                    mixed[mu][nu] += inverse[mu][alpha] * ricci[alpha][nu];
                }
            }
        }
        let rg = h.gravitational_radius_m();
        let max_ricci = mixed.iter().flatten().fold(0.0_f64, |m, x| m.max(x.abs()));
        assert!(
            max_ricci * rg * rg < 2e-9,
            "normalized Ricci residual={:e}",
            max_ricci * rg * rg
        );

        let a = h.kerr_parameter_m();
        let cos = theta.cos();
        let sigma = r * r + a * a * cos * cos;
        let numerator = r.powi(6) - 15.0 * a * a * r.powi(4) * cos.powi(2)
            + 15.0 * a.powi(4) * r * r * cos.powi(4)
            - a.powi(6) * cos.powi(6);
        let expected = 48.0 * rg * rg * numerator / sigma.powi(6);
        let actual = curvature
            .kretschmann_scalar(&differentiated.metric)
            .unwrap();
        close(actual, expected, 2e-8);
    }

    #[test]
    fn ingoing_ef_chart_is_regular_at_schwarzschild_horizon() {
        let h = SchwarzschildBlackHole::new(mass());
        let r = h.horizon_radius_m();
        let theta = 0.9;
        let differentiated = DifferentiatedMetric4::schwarzschild_ingoing_ef(h, r, theta).unwrap();
        assert_eq!(differentiated.metric.covariant()[0][0], 0.0);
        assert_eq!(differentiated.metric.covariant()[0][1], 1.0);
        assert!(differentiated.metric.determinant().unwrap().is_finite());
        assert!(differentiated.metric.inverse_identity_residual().unwrap() < 1e-13);

        let curvature = differentiated.curvature().unwrap();
        let expected = 48.0 * h.gravitational_radius_m().powi(2) / r.powi(6);
        let actual = curvature
            .kretschmann_scalar(&differentiated.metric)
            .unwrap();
        close(actual, expected, 2e-11);
    }

    #[test]
    fn schwarzschild_curvature_scalar_agrees_across_exterior_charts() {
        let h = SchwarzschildBlackHole::new(mass());
        let r = 8.0 * h.gravitational_radius_m();
        let theta = 0.9;
        let schwarzschild = DifferentiatedMetric4::schwarzschild_exterior(h, r, theta).unwrap();
        let ingoing = DifferentiatedMetric4::schwarzschild_ingoing_ef(h, r, theta).unwrap();
        let k_s = schwarzschild
            .curvature()
            .unwrap()
            .kretschmann_scalar(&schwarzschild.metric)
            .unwrap();
        let k_ef = ingoing
            .curvature()
            .unwrap()
            .kretschmann_scalar(&ingoing.metric)
            .unwrap();
        close(k_s, k_ef, 2e-11);
    }

    #[test]
    fn ingoing_ef_chart_curvature_is_regular_inside_horizon() {
        let h = SchwarzschildBlackHole::new(mass());
        let r = 1.5 * h.gravitational_radius_m();
        let theta = 1.0;
        let differentiated = DifferentiatedMetric4::schwarzschild_ingoing_ef(h, r, theta).unwrap();
        let curvature = differentiated.curvature().unwrap();
        let expected = 48.0 * h.gravitational_radius_m().powi(2) / r.powi(6);
        let actual = curvature
            .kretschmann_scalar(&differentiated.metric)
            .unwrap();
        close(actual, expected, 2e-10);
    }

    #[test]
    fn ingoing_kerr_chart_is_regular_at_outer_horizon() {
        let h = KerrBlackHole::new(mass(), DimensionlessSpin::new(0.8).unwrap());
        let r = h.outer_horizon_radius_m();
        let theta = 0.8;
        let differentiated = DifferentiatedMetric4::kerr_ingoing(h, r, theta).unwrap();
        let determinant = differentiated.metric.determinant().unwrap();
        let sigma = r * r + h.kerr_parameter_m().powi(2) * theta.cos().powi(2);
        close(determinant, -sigma.powi(2) * theta.sin().powi(2), 2e-12);
        assert!(differentiated.metric.inverse_identity_residual().unwrap() < 2e-12);
        let curvature = differentiated.curvature().unwrap();
        let numerator = r.powi(6)
            - 15.0 * h.kerr_parameter_m().powi(2) * r.powi(4) * theta.cos().powi(2)
            + 15.0 * h.kerr_parameter_m().powi(4) * r.powi(2) * theta.cos().powi(4)
            - h.kerr_parameter_m().powi(6) * theta.cos().powi(6);
        let expected = 48.0 * h.gravitational_radius_m().powi(2) * numerator / sigma.powi(6);
        close(
            curvature
                .kretschmann_scalar(&differentiated.metric)
                .unwrap(),
            expected,
            3e-8,
        );
    }

    #[test]
    fn kerr_curvature_scalar_agrees_across_boyer_lindquist_and_ingoing_charts() {
        let h = KerrBlackHole::new(mass(), DimensionlessSpin::new(0.7).unwrap());
        let r = 7.0 * h.gravitational_radius_m();
        let theta = 1.0;
        let bl = DifferentiatedMetric4::kerr_exterior(h, r, theta).unwrap();
        let ingoing = DifferentiatedMetric4::kerr_ingoing(h, r, theta).unwrap();
        let k_bl = bl
            .curvature()
            .unwrap()
            .kretschmann_scalar(&bl.metric)
            .unwrap();
        let k_in = ingoing
            .curvature()
            .unwrap()
            .kretschmann_scalar(&ingoing.metric)
            .unwrap();
        close(k_bl, k_in, 3e-8);
    }

    #[test]
    fn ingoing_kerr_chart_curvature_is_finite_inside_outer_horizon() {
        let h = KerrBlackHole::new(mass(), DimensionlessSpin::new(0.8).unwrap());
        let r = 0.8 * h.outer_horizon_radius_m();
        let theta = 0.8;
        let differentiated = DifferentiatedMetric4::kerr_ingoing(h, r, theta).unwrap();
        let curvature = differentiated.curvature().unwrap();
        let sigma = r * r + h.kerr_parameter_m().powi(2) * theta.cos().powi(2);
        let a = h.kerr_parameter_m();
        let rg = h.gravitational_radius_m();
        let cos = theta.cos();
        let numerator = r.powi(6) - 15.0 * a * a * r.powi(4) * cos.powi(2)
            + 15.0 * a.powi(4) * r * r * cos.powi(4)
            - a.powi(6) * cos.powi(6);
        let expected = 48.0 * rg * rg * numerator / sigma.powi(6);
        close(
            curvature
                .kretschmann_scalar(&differentiated.metric)
                .unwrap(),
            expected,
            4e-8,
        );
    }

    #[test]
    fn differentiated_metric_rejects_exterior_chart_boundaries() {
        let h = KerrBlackHole::new(mass(), DimensionlessSpin::new(0.5).unwrap());
        assert!(matches!(
            DifferentiatedMetric4::kerr_exterior(h, h.outer_horizon_radius_m(), 1.0),
            Err(JetError::Metric(MetricError::PhysicsDomain(
                PhysicsDomainError::RadiusAtOrInsideHorizon { .. }
            )))
        ));
    }
}
