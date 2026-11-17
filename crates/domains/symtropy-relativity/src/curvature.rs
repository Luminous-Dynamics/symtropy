#![allow(clippy::needless_range_loop)]
// Fixed 4D tensor contractions use explicit Einstein-index loops.

// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Convention-explicit Riemann, Ricci, and Einstein curvature tensors.
//!
//! Riemann sign convention:
//! R^rho_(sigma mu nu) = d_mu Gamma^rho_(nu sigma)
//!                     - d_nu Gamma^rho_(mu sigma)
//!                     + Gamma^rho_(mu lambda) Gamma^lambda_(nu sigma)
//!                     - Gamma^rho_(nu lambda) Gamma^lambda_(mu sigma).
//!
//! Ricci convention: R_(sigma nu) = R^rho_(sigma rho nu).
//!
//! Derivative tensor order:
//! d2[alpha][beta][mu][nu] = partial_alpha partial_beta g_(mu nu).
//! Riemann storage order: mixed[rho][sigma][mu][nu] = R^rho_(sigma mu nu).
//!
//! This implementation is analytic / derivative-fed: it never silently
//! finite-differences a metric. The first exact second-derivative fixture is
//! Schwarzschild in the exterior Schwarzschild chart.

use core::fmt;

use super::{
    PhysicsDomainError, SchwarzschildBlackHole,
    connection::{ConnectionError, LeviCivitaConnection4, MetricDerivatives4},
    metric::{MetricComponents4, MetricError, schwarzschild_exterior_metric},
};

/// Second coordinate derivatives of the covariant metric.
///
/// Index order: components[alpha][beta][mu][nu] =
/// partial_alpha partial_beta g_(mu nu).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MetricSecondDerivatives4 {
    components: [[[[f64; 4]; 4]; 4]; 4],
}

impl MetricSecondDerivatives4 {
    pub fn new(mut components: [[[[f64; 4]; 4]; 4]; 4]) -> Result<Self, CurvatureError> {
        for alpha in 0..4 {
            for beta in 0..4 {
                for mu in 0..4 {
                    for nu in 0..4 {
                        if !components[alpha][beta][mu][nu].is_finite() {
                            return Err(CurvatureError::NonFiniteSecondMetricDerivative {
                                first_derivative: alpha,
                                second_derivative: beta,
                                row: mu,
                                column: nu,
                            });
                        }
                    }
                }
            }
        }

        // Mixed partial derivatives commute for the smooth metrics in scope.
        for alpha in 0..4 {
            for beta in (alpha + 1)..4 {
                for mu in 0..4 {
                    for nu in 0..4 {
                        let a = components[alpha][beta][mu][nu];
                        let b = components[beta][alpha][mu][nu];
                        if !approximately_symmetric(a, b) {
                            return Err(CurvatureError::AsymmetricSecondDerivativeIndices {
                                first_derivative: alpha,
                                second_derivative: beta,
                            });
                        }
                        let mean = 0.5 * a + 0.5 * b;
                        components[alpha][beta][mu][nu] = mean;
                        components[beta][alpha][mu][nu] = mean;
                    }
                }
            }
        }

        // The metric's covariant indices are symmetric.
        for alpha in 0..4 {
            for beta in 0..4 {
                for mu in 0..4 {
                    for nu in (mu + 1)..4 {
                        let a = components[alpha][beta][mu][nu];
                        let b = components[alpha][beta][nu][mu];
                        if !approximately_symmetric(a, b) {
                            return Err(CurvatureError::AsymmetricSecondMetricIndices {
                                first_derivative: alpha,
                                second_derivative: beta,
                                row: mu,
                                column: nu,
                            });
                        }
                        let mean = 0.5 * a + 0.5 * b;
                        components[alpha][beta][mu][nu] = mean;
                        components[alpha][beta][nu][mu] = mean;
                    }
                }
            }
        }

        Ok(Self { components })
    }

    pub const fn zero() -> Self {
        Self {
            components: [[[[0.0; 4]; 4]; 4]; 4],
        }
    }

    pub const fn components(&self) -> &[[[[f64; 4]; 4]; 4]; 4] {
        &self.components
    }

    /// Exact second coordinate derivatives of the Schwarzschild exterior
    /// metric in coordinates (ct, r, theta, phi).
    pub fn schwarzschild_exterior(
        hole: SchwarzschildBlackHole,
        radius_m: f64,
        theta_rad: f64,
    ) -> Result<Self, CurvatureError> {
        // Reuse the exact metric and first-derivative constructors as domain
        // authorities. All derivatives below are analytic closed forms.
        let _metric = schwarzschild_exterior_metric(hole, radius_m, theta_rad)?;
        let _first = MetricDerivatives4::schwarzschild_exterior(hole, radius_m, theta_rad)?;

        let rg = hole.gravitational_radius_m();
        let f = 1.0 - hole.horizon_radius_m() / radius_m;
        let sin_theta = theta_rad.sin();
        let cos_theta = theta_rad.cos();
        let sin_squared = sin_theta * sin_theta;

        let mut d2 = [[[[0.0; 4]; 4]; 4]; 4];

        // d_r d_r g_tt and g_rr.
        d2[1][1][0][0] = 4.0 * rg / radius_m.powi(3);
        d2[1][1][1][1] = 4.0 * rg / (radius_m.powi(3) * f.powi(3));

        // d_r d_r g_theta_theta and g_phi_phi.
        d2[1][1][2][2] = 2.0;
        d2[1][1][3][3] = 2.0 * sin_squared;

        // Mixed r/theta derivatives of g_phi_phi.
        let mixed_r_theta = 4.0 * radius_m * sin_theta * cos_theta;
        d2[1][2][3][3] = mixed_r_theta;
        d2[2][1][3][3] = mixed_r_theta;

        // d_theta d_theta g_phi_phi.
        d2[2][2][3][3] = 2.0 * radius_m.powi(2) * (cos_theta * cos_theta - sin_squared);

        Self::new(d2)
    }
}

/// Mixed-index Riemann curvature tensor R^rho_(sigma mu nu).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RiemannTensor4 {
    mixed: [[[[f64; 4]; 4]; 4]; 4],
}

impl RiemannTensor4 {
    /// Construct Riemann curvature from metric, first metric derivatives,
    /// and symmetric second metric derivatives.
    pub fn from_metric_derivatives(
        metric: &MetricComponents4,
        first: &MetricDerivatives4,
        second: &MetricSecondDerivatives4,
    ) -> Result<Self, CurvatureError> {
        let inverse = metric.inverse()?;
        let d1 = first.components();
        let d2 = second.components();
        let connection = LeviCivitaConnection4::from_metric_and_derivatives(metric, first)?;
        let gamma = connection.symbols();

        // dgamma[alpha][rho][mu][nu] = partial_alpha Gamma^rho_(mu nu).
        let mut dgamma = [[[[0.0; 4]; 4]; 4]; 4];

        for alpha in 0..4 {
            for rho in 0..4 {
                for mu in 0..4 {
                    for nu in 0..4 {
                        let mut value = 0.0;
                        for sigma in 0..4 {
                            // partial_alpha g^rho_sigma =
                            // -g^rho_a g^b_sigma partial_alpha g_ab.
                            let mut d_inverse = 0.0;
                            for a in 0..4 {
                                for b in 0..4 {
                                    d_inverse -=
                                        inverse[rho][a] * d1[alpha][a][b] * inverse[b][sigma];
                                }
                            }

                            let connection_kernel =
                                d1[mu][sigma][nu] + d1[nu][sigma][mu] - d1[sigma][mu][nu];

                            let second_kernel = d2[alpha][mu][sigma][nu] + d2[alpha][nu][sigma][mu]
                                - d2[alpha][sigma][mu][nu];

                            value += 0.5
                                * (d_inverse * connection_kernel
                                    + inverse[rho][sigma] * second_kernel);
                        }

                        if !value.is_finite() {
                            return Err(CurvatureError::NonFiniteConnectionDerivative {
                                derivative_index: alpha,
                                upper: rho,
                                lower_first: mu,
                                lower_second: nu,
                            });
                        }
                        dgamma[alpha][rho][mu][nu] = value;
                    }
                }
            }
        }

        let mut riemann = [[[[0.0; 4]; 4]; 4]; 4];

        for rho in 0..4 {
            for sigma in 0..4 {
                for mu in 0..4 {
                    for nu in 0..4 {
                        // R^rho_(sigma mu nu) follows the convention in this
                        // module's documentation, including the overall sign.
                        let mut value = dgamma[mu][rho][nu][sigma] - dgamma[nu][rho][mu][sigma];

                        for lambda in 0..4 {
                            value += gamma[rho][mu][lambda] * gamma[lambda][nu][sigma]
                                - gamma[rho][nu][lambda] * gamma[lambda][mu][sigma];
                        }

                        if !value.is_finite() {
                            return Err(CurvatureError::NonFiniteRiemannComponent {
                                upper: rho,
                                lower: sigma,
                                first_derivative: mu,
                                second_derivative: nu,
                            });
                        }
                        riemann[rho][sigma][mu][nu] = value;
                    }
                }
            }
        }

        Ok(Self { mixed: riemann })
    }

    pub const fn mixed(&self) -> &[[[[f64; 4]; 4]; 4]; 4] {
        &self.mixed
    }

    /// Lower the leading Riemann index to obtain R_(alpha beta mu nu).
    pub fn fully_covariant(
        &self,
        metric: &MetricComponents4,
    ) -> Result<[[[[f64; 4]; 4]; 4]; 4], CurvatureError> {
        let g = metric.covariant();
        let mut lowered = [[[[0.0; 4]; 4]; 4]; 4];

        for alpha in 0..4 {
            for beta in 0..4 {
                for mu in 0..4 {
                    for nu in 0..4 {
                        let mut value = 0.0;
                        for rho in 0..4 {
                            value += g[alpha][rho] * self.mixed[rho][beta][mu][nu];
                        }
                        if !value.is_finite() {
                            return Err(CurvatureError::NonFiniteLoweredRiemann {
                                alpha,
                                beta,
                                mu,
                                nu,
                            });
                        }
                        lowered[alpha][beta][mu][nu] = value;
                    }
                }
            }
        }
        Ok(lowered)
    }

    /// Ricci tensor R_(sigma nu) = R^rho_(sigma rho nu).
    pub fn ricci_tensor(&self) -> [[f64; 4]; 4] {
        let mut ricci = [[0.0; 4]; 4];
        for sigma in 0..4 {
            for nu in 0..4 {
                for rho in 0..4 {
                    ricci[sigma][nu] += self.mixed[rho][sigma][rho][nu];
                }
            }
        }
        ricci
    }

    /// Maximum componentwise absolute residual of Ricci symmetry.
    pub fn ricci_symmetry_residual(&self) -> f64 {
        let ricci = self.ricci_tensor();
        let mut maximum: f64 = 0.0;
        for mu in 0..4 {
            for nu in 0..4 {
                maximum = maximum.max((ricci[mu][nu] - ricci[nu][mu]).abs());
            }
        }
        maximum
    }

    /// Scalar curvature R = g^mu_nu R_mu_nu.
    pub fn ricci_scalar(&self, metric: &MetricComponents4) -> Result<f64, CurvatureError> {
        let inverse = metric.inverse()?;
        let ricci = self.ricci_tensor();
        let mut scalar = 0.0;
        for mu in 0..4 {
            for nu in 0..4 {
                scalar += inverse[mu][nu] * ricci[mu][nu];
            }
        }
        if scalar.is_finite() {
            Ok(scalar)
        } else {
            Err(CurvatureError::ComputationNonFinite("Ricci scalar"))
        }
    }

    /// Einstein tensor G_mu_nu = R_mu_nu - 1/2 g_mu_nu R.
    pub fn einstein_tensor(
        &self,
        metric: &MetricComponents4,
    ) -> Result<[[f64; 4]; 4], CurvatureError> {
        let ricci = self.ricci_tensor();
        let scalar = self.ricci_scalar(metric)?;
        let g = metric.covariant();
        let mut einstein = [[0.0; 4]; 4];
        for mu in 0..4 {
            for nu in 0..4 {
                einstein[mu][nu] = ricci[mu][nu] - 0.5 * g[mu][nu] * scalar;
                if !einstein[mu][nu].is_finite() {
                    return Err(CurvatureError::ComputationNonFinite("Einstein tensor"));
                }
            }
        }
        Ok(einstein)
    }

    /// Kretschmann scalar R_abcd R^abcd.
    ///
    /// The contraction is performed with four inverse metrics in the declared
    /// coordinate basis; this is a scalar, unlike the metric determinant.
    pub fn kretschmann_scalar(&self, metric: &MetricComponents4) -> Result<f64, CurvatureError> {
        let covariant = self.fully_covariant(metric)?;
        let inverse = metric.inverse()?;
        let mut scalar = 0.0;

        for a in 0..4 {
            for b in 0..4 {
                for c in 0..4 {
                    for d in 0..4 {
                        for e in 0..4 {
                            for f in 0..4 {
                                for g in 0..4 {
                                    for h in 0..4 {
                                        scalar += covariant[a][b][c][d]
                                            * covariant[e][f][g][h]
                                            * inverse[a][e]
                                            * inverse[b][f]
                                            * inverse[c][g]
                                            * inverse[d][h];
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if scalar.is_finite() {
            Ok(scalar)
        } else {
            Err(CurvatureError::ComputationNonFinite("Kretschmann scalar"))
        }
    }

    /// Riemann algebraic symmetry residuals for the fully lowered tensor.
    ///
    /// Returns maxima for:
    /// - antisymmetry in first pair;
    /// - antisymmetry in second pair;
    /// - pair-exchange symmetry;
    /// - first Bianchi identity.
    pub fn algebraic_symmetry_residuals(
        &self,
        metric: &MetricComponents4,
    ) -> Result<RiemannSymmetryResiduals, CurvatureError> {
        let r = self.fully_covariant(metric)?;
        let mut residuals = RiemannSymmetryResiduals::default();

        for a in 0..4 {
            for b in 0..4 {
                for c in 0..4 {
                    for d in 0..4 {
                        residuals.first_pair_antisymmetry = residuals
                            .first_pair_antisymmetry
                            .max((r[a][b][c][d] + r[b][a][c][d]).abs());
                        residuals.second_pair_antisymmetry = residuals
                            .second_pair_antisymmetry
                            .max((r[a][b][c][d] + r[a][b][d][c]).abs());
                        residuals.pair_exchange = residuals
                            .pair_exchange
                            .max((r[a][b][c][d] - r[c][d][a][b]).abs());
                        residuals.first_bianchi = residuals
                            .first_bianchi
                            .max((r[a][b][c][d] + r[a][c][d][b] + r[a][d][b][c]).abs());
                    }
                }
            }
        }

        Ok(residuals)
    }
}

/// Absolute residuals for the algebraic Riemann tensor identities.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RiemannSymmetryResiduals {
    pub first_pair_antisymmetry: f64,
    pub second_pair_antisymmetry: f64,
    pub pair_exchange: f64,
    pub first_bianchi: f64,
}

/// Errors while constructing and contracting curvature tensors.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CurvatureError {
    Metric(MetricError),
    Connection(ConnectionError),
    PhysicsDomain(PhysicsDomainError),
    NonFiniteSecondMetricDerivative {
        first_derivative: usize,
        second_derivative: usize,
        row: usize,
        column: usize,
    },
    AsymmetricSecondDerivativeIndices {
        first_derivative: usize,
        second_derivative: usize,
    },
    AsymmetricSecondMetricIndices {
        first_derivative: usize,
        second_derivative: usize,
        row: usize,
        column: usize,
    },
    NonFiniteConnectionDerivative {
        derivative_index: usize,
        upper: usize,
        lower_first: usize,
        lower_second: usize,
    },
    NonFiniteRiemannComponent {
        upper: usize,
        lower: usize,
        first_derivative: usize,
        second_derivative: usize,
    },
    NonFiniteLoweredRiemann {
        alpha: usize,
        beta: usize,
        mu: usize,
        nu: usize,
    },
    ComputationNonFinite(&'static str),
}

impl From<MetricError> for CurvatureError {
    fn from(value: MetricError) -> Self {
        Self::Metric(value)
    }
}
impl From<ConnectionError> for CurvatureError {
    fn from(value: ConnectionError) -> Self {
        Self::Connection(value)
    }
}
impl From<PhysicsDomainError> for CurvatureError {
    fn from(value: PhysicsDomainError) -> Self {
        Self::PhysicsDomain(value)
    }
}

impl fmt::Display for CurvatureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Metric(e) => write!(f, "{e}"),
            Self::Connection(e) => write!(f, "{e}"),
            Self::PhysicsDomain(e) => write!(f, "{e}"),
            Self::NonFiniteSecondMetricDerivative {
                first_derivative,
                second_derivative,
                row,
                column,
            } => write!(
                f,
                "second metric derivative d2[{first_derivative}][{second_derivative}][{row}][{column}] is non-finite"
            ),
            Self::AsymmetricSecondDerivativeIndices {
                first_derivative,
                second_derivative,
            } => write!(
                f,
                "mixed second derivatives are asymmetric at ({first_derivative}, {second_derivative})"
            ),
            Self::AsymmetricSecondMetricIndices {
                first_derivative,
                second_derivative,
                row,
                column,
            } => write!(
                f,
                "second metric derivative has asymmetric metric indices at ({first_derivative}, {second_derivative}, {row}, {column})"
            ),
            Self::NonFiniteConnectionDerivative {
                derivative_index,
                upper,
                lower_first,
                lower_second,
            } => write!(
                f,
                "connection derivative d[{derivative_index}] Gamma^{upper}_{{{lower_first}{lower_second}}} is non-finite"
            ),
            Self::NonFiniteRiemannComponent {
                upper,
                lower,
                first_derivative,
                second_derivative,
            } => write!(
                f,
                "Riemann component R^{upper}_{{{lower}{first_derivative}{second_derivative}}} is non-finite"
            ),
            Self::NonFiniteLoweredRiemann {
                alpha,
                beta,
                mu,
                nu,
            } => write!(
                f,
                "lowered Riemann component R_{{{alpha}{beta}{mu}{nu}}} is non-finite"
            ),
            Self::ComputationNonFinite(name) => write!(f, "{name} produced a non-finite result"),
        }
    }
}
impl std::error::Error for CurvatureError {}

fn approximately_symmetric(a: f64, b: f64) -> bool {
    let scale = a.abs().max(b.abs()).max(f64::MIN_POSITIVE);
    (a - b).abs() <= 64.0 * f64::EPSILON * scale
}

#[cfg(test)]
mod tests {
    use super::super::{MassKg, SchwarzschildBlackHole};
    use super::*;

    const SOLAR_MASS_KG: f64 = 1.988_47e30;

    fn hole() -> SchwarzschildBlackHole {
        SchwarzschildBlackHole::new(MassKg::new(SOLAR_MASS_KG).unwrap())
    }

    fn max_abs(matrix: [[f64; 4]; 4]) -> f64 {
        matrix.iter().flatten().fold(0.0_f64, |m, x| m.max(x.abs()))
    }

    fn close(actual: f64, expected: f64, tolerance: f64) {
        let allowed = if expected == 0.0 {
            tolerance
        } else {
            tolerance * expected.abs()
        };
        assert!(
            (actual - expected).abs() <= allowed,
            "actual={actual:.16e}, expected={expected:.16e}, allowed={allowed:.3e}"
        );
    }

    fn schwarzschild_curvature() -> (MetricComponents4, RiemannTensor4) {
        let h = hole();
        let r = 8.0 * h.gravitational_radius_m();
        let theta = 0.9;
        let metric = schwarzschild_exterior_metric(h, r, theta).unwrap();
        let first = MetricDerivatives4::schwarzschild_exterior(h, r, theta).unwrap();
        let second = MetricSecondDerivatives4::schwarzschild_exterior(h, r, theta).unwrap();
        let curvature = RiemannTensor4::from_metric_derivatives(&metric, &first, &second).unwrap();
        (metric, curvature)
    }

    #[test]
    fn minkowski_cartesian_curvature_vanishes() {
        let metric = MetricComponents4::new([
            [-1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
        .unwrap();
        let curvature = RiemannTensor4::from_metric_derivatives(
            &metric,
            &MetricDerivatives4::zero(),
            &MetricSecondDerivatives4::zero(),
        )
        .unwrap();
        assert_eq!(curvature.ricci_scalar(&metric).unwrap(), 0.0);
        assert_eq!(curvature.kretschmann_scalar(&metric).unwrap(), 0.0);
        assert_eq!(max_abs(curvature.einstein_tensor(&metric).unwrap()), 0.0);
    }

    #[test]
    fn schwarzschild_vacuum_ricci_and_einstein_tensors_vanish() {
        let (metric, curvature) = schwarzschild_curvature();
        let ricci = curvature.ricci_tensor();
        let einstein = curvature.einstein_tensor(&metric).unwrap();
        let inverse = metric.inverse().unwrap();
        let mut ricci_mixed = [[0.0; 4]; 4];
        let mut einstein_mixed = [[0.0; 4]; 4];
        for mu in 0..4 {
            for nu in 0..4 {
                for alpha in 0..4 {
                    ricci_mixed[mu][nu] += inverse[mu][alpha] * ricci[alpha][nu];
                    einstein_mixed[mu][nu] += inverse[mu][alpha] * einstein[alpha][nu];
                }
            }
        }
        let rg = hole().gravitational_radius_m();
        let normalized_ricci = max_abs(ricci_mixed) * rg * rg;
        let normalized_einstein = max_abs(einstein_mixed) * rg * rg;
        assert!(
            normalized_ricci < 1e-10,
            "normalized Ricci={normalized_ricci:e}"
        );
        assert!(
            normalized_einstein < 1e-10,
            "normalized Einstein={normalized_einstein:e}"
        );

        let lowered = curvature.fully_covariant(&metric).unwrap();
        let riemann_scale = lowered
            .iter()
            .flatten()
            .flatten()
            .flatten()
            .fold(0.0_f64, |m, x| m.max(x.abs()));
        assert!(riemann_scale > 0.0);
        assert!(curvature.ricci_symmetry_residual() / riemann_scale < 1e-12);

        let scalar = curvature.ricci_scalar(&metric).unwrap();
        assert!(
            scalar.abs() * rg * rg < 1e-10,
            "normalized scalar curvature={:e}",
            scalar * rg * rg
        );
    }

    #[test]
    fn schwarzschild_kretschmann_matches_closed_form() {
        let (metric, curvature) = schwarzschild_curvature();
        let h = hole();
        let r = 8.0 * h.gravitational_radius_m();
        let rg = h.gravitational_radius_m();
        let expected = 48.0 * rg * rg / r.powi(6);
        let actual = curvature.kretschmann_scalar(&metric).unwrap();
        close(actual, expected, 2e-11);
    }

    #[test]
    fn schwarzschild_riemann_has_algebraic_symmetries() {
        let (metric, curvature) = schwarzschild_curvature();
        let residuals = curvature.algebraic_symmetry_residuals(&metric).unwrap();
        let lowered = curvature.fully_covariant(&metric).unwrap();
        let scale = lowered
            .iter()
            .flatten()
            .flatten()
            .flatten()
            .fold(0.0_f64, |m, x| m.max(x.abs()));
        let normalized = [
            residuals.first_pair_antisymmetry,
            residuals.second_pair_antisymmetry,
            residuals.pair_exchange,
            residuals.first_bianchi,
        ]
        .into_iter()
        .fold(0.0_f64, f64::max)
            / scale;
        assert!(
            normalized < 1e-11,
            "normalized symmetry residual={normalized:e}"
        );
    }

    #[test]
    fn schwarzschild_second_derivative_rejects_invalid_chart_domains() {
        let h = hole();
        assert!(matches!(
            MetricSecondDerivatives4::schwarzschild_exterior(h, h.horizon_radius_m(), 1.0),
            Err(CurvatureError::Metric(MetricError::PhysicsDomain(
                PhysicsDomainError::RadiusAtOrInsideHorizon { .. }
            )))
        ));
        assert!(matches!(
            MetricSecondDerivatives4::schwarzschild_exterior(
                h,
                4.0 * h.gravitational_radius_m(),
                0.0
            ),
            Err(CurvatureError::Metric(
                MetricError::PolarAxisCoordinateSingularity
            ))
        ));
    }

    #[test]
    fn second_derivative_constructor_rejects_asymmetric_data() {
        let mut d2 = [[[[0.0; 4]; 4]; 4]; 4];
        d2[0][1][2][2] = 1.0;
        assert!(matches!(
            MetricSecondDerivatives4::new(d2),
            Err(CurvatureError::AsymmetricSecondDerivativeIndices { .. })
        ));
        d2 = [[[[0.0; 4]; 4]; 4]; 4];
        d2[1][1][0][2] = 1.0;
        assert!(matches!(
            MetricSecondDerivatives4::new(d2),
            Err(CurvatureError::AsymmetricSecondMetricIndices { .. })
        ));
    }
}
