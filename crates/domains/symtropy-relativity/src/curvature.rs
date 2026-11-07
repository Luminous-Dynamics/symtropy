// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Riemann, Ricci and Einstein curvature tensors from an explicit derivative
//! of the Levi-Civita connection.
//!
//! Conventions:
//! - R^rho_(sigma mu nu) = partial_mu Gamma^rho_(nu sigma)
//!   - partial_nu Gamma^rho_(mu sigma)
//!   + Gamma^rho_(mu lambda) Gamma^lambda_(nu sigma)
//!   - Gamma^rho_(nu lambda) Gamma^lambda_(mu sigma).
//! - Ricci: R_(sigma nu) = R^rho_(sigma rho nu).
//! - Metric signature and coordinate chart are supplied by the metric.
//! - Connection derivatives are d[mu][rho][nu][sigma] =
//!   partial_mu Gamma^rho_(nu sigma).
//!
//! No hidden finite differences are used. This first implementation includes
//! an analytic Schwarzschild exterior derivative fixture. Kerr connection
//! derivatives, chart-regular horizon calculations, and automatic
//! differentiation are later work.

use core::fmt;

use super::{
    connection::{ConnectionError, LeviCivitaConnection4, MetricDerivatives4},
    metric::{MetricComponents4, MetricError, schwarzschild_exterior_metric},
    PhysicsDomainError, SchwarzschildBlackHole,
};

/// First coordinate derivatives of connection coefficients.
///
/// Index order: components[mu][rho][nu][sigma] =
/// partial_mu Gamma^rho_(nu sigma). The final two indices are symmetric.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConnectionDerivatives4 {
    components: [[[[f64; 4]; 4]; 4]; 4],
}

impl ConnectionDerivatives4 {
    pub fn new(
        mut components: [[[[f64; 4]; 4]; 4]; 4],
    ) -> Result<Self, CurvatureError> {
        for mu in 0..4 {
            for rho in 0..4 {
                for nu in 0..4 {
                    for sigma in 0..4 {
                        if !components[mu][rho][nu][sigma].is_finite() {
                            return Err(CurvatureError::NonFiniteConnectionDerivative {
                                derivative_index: mu,
                                upper: rho,
                                lower_first: nu,
                                lower_second: sigma,
                            });
                        }
                    }
                }
                for nu in 0..4 {
                    for sigma in (nu + 1)..4 {
                        let left = components[mu][rho][nu][sigma];
                        let right = components[mu][rho][sigma][nu];
                        let tolerance =
                            64.0 * f64::EPSILON * left.abs().max(right.abs()).max(1.0);
                        if (left - right).abs() > tolerance {
                            return Err(CurvatureError::AsymmetricConnectionDerivative {
                                derivative_index: mu,
                                upper: rho,
                                lower_first: nu,
                                lower_second: sigma,
                            });
                        }
                        let mean = 0.5 * left + 0.5 * right;
                        components[mu][rho][nu][sigma] = mean;
                        components[mu][rho][sigma][nu] = mean;
                    }
                }
            }
        }
        Ok(Self { components })
    }

    /// Zero derivative tensor, suitable for a constant connection / flat
    /// Cartesian metric fixture.
    pub const fn zero() -> Self {
        Self {
            components: [[[[0.0; 4]; 4]; 4]; 4],
        }
    }

    pub const fn components(&self) -> &[[[[f64; 4]; 4]; 4]; 4] {
        &self.components
    }

    /// Exact derivatives of the Schwarzschild-coordinate Levi-Civita
    /// connection in coordinates (ct, r, theta, phi).
    ///
    /// Domain: r > 2GM/c^2 and 0 < theta < pi.
    pub fn schwarzschild_exterior(
        hole: SchwarzschildBlackHole,
        radius_m: f64,
        theta_rad: f64,
    ) -> Result<Self, CurvatureError> {
        // Reuse the analytic metric and derivative fixtures as the chart-domain
        // authorities before constructing second-order data.
        let _metric = schwarzschild_exterior_metric(hole, radius_m, theta_rad)?;
        let _first_derivatives =
            MetricDerivatives4::schwarzschild_exterior(hole, radius_m, theta_rad)?;

        let rg = hole.gravitational_radius_m();
        let f = 1.0 - hole.horizon_radius_m() / radius_m;
        let sin_theta = theta_rad.sin();
        let cos_theta = theta_rad.cos();

        // Nonzero independent Christoffel functions:
        // A = Gamma^t_tr, B = Gamma^r_tt, C = Gamma^r_rr,
        // D = Gamma^r_thetatheta, E = Gamma^theta_rtheta,
        // F = Gamma^theta_phiphi, H = Gamma^phi_thetaphi.
        let a_prime =
            -2.0 * rg / (radius_m.powi(3) * f)
            - 2.0 * rg.powi(2) / (radius_m.powi(4) * f.powi(2));
        let b_prime =
            2.0 * rg.powi(2) / radius_m.powi(4)
            - 2.0 * f * rg / radius_m.powi(3);
        let c_prime = -a_prime;
        let d_prime = -1.0;
        let e_prime = -1.0 / radius_m.powi(2);
        let f_prime = sin_theta.powi(2) - cos_theta.powi(2);
        let h_prime = -1.0 / sin_theta.powi(2);

        let mut d = [[[[0.0; 4]; 4]; 4]; 4];

        // partial_r Gamma^t_(tr)
        d[1][0][0][1] = a_prime;
        d[1][0][1][0] = a_prime;

        // partial_r Gamma^r_(tt), Gamma^r_(rr), Gamma^r_(theta theta)
        d[1][1][0][0] = b_prime;
        d[1][1][1][1] = c_prime;
        d[1][1][2][2] = d_prime;

        // partial_r Gamma^r_(phi phi) and partial_theta Gamma^r_(phi phi)
        d[1][1][3][3] = d_prime * sin_theta.powi(2);
        d[2][1][3][3] = -2.0 * f * radius_m * sin_theta * cos_theta;

        // partial_r Gamma^theta_(r theta), Gamma^phi_(r phi)
        d[1][2][1][2] = e_prime;
        d[1][2][2][1] = e_prime;
        d[1][3][1][3] = e_prime;
        d[1][3][3][1] = e_prime;

        // partial_theta Gamma^theta_(phi phi), Gamma^phi_(theta phi)
        d[2][2][3][3] = f_prime;
        d[2][3][2][3] = h_prime;
        d[2][3][3][2] = h_prime;

        Self::new(d)
    }
}

/// Mixed-index Riemann tensor R^rho_(sigma mu nu).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RiemannTensor4 {
    components: [[[[f64; 4]; 4]; 4]; 4],
}

impl RiemannTensor4 {
    /// Compute the Riemann tensor from one connection and its explicit
    /// coordinate derivatives.
    pub fn from_connection_and_derivatives(
        connection: &LeviCivitaConnection4,
        derivatives: &ConnectionDerivatives4,
    ) -> Result<Self, CurvatureError> {
        let gamma = connection.symbols();
        let d = derivatives.components();
        let mut components = [[[[0.0; 4]; 4]; 4]; 4];

        for rho in 0..4 {
            for sigma in 0..4 {
                for mu in 0..4 {
                    for nu in (mu + 1)..4 {
                        let mut value =
                            d[mu][rho][nu][sigma] - d[nu][rho][mu][sigma];

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

                        components[rho][sigma][mu][nu] = value;
                        components[rho][sigma][nu][mu] = -value;
                    }
                }
            }
        }

        Ok(Self { components })
    }

    /// R^rho_(sigma mu nu).
    pub const fn mixed(&self) -> &[[[[f64; 4]; 4]; 4]; 4] {
        &self.components
    }

    /// Lower the first Riemann index: R_(alpha sigma mu nu).
    pub fn lower_first_index(
        &self,
        metric: &MetricComponents4,
    ) -> Result<RiemannTensorCovariant4, CurvatureError> {
        let g = metric.covariant();
        let mut lowered = [[[[0.0; 4]; 4]; 4]; 4];

        for alpha in 0..4 {
            for sigma in 0..4 {
                for mu in 0..4 {
                    for nu in 0..4 {
                        let mut value = 0.0;
                        for rho in 0..4 {
                            value += g[alpha][rho] * self.components[rho][sigma][mu][nu];
                        }
                        if !value.is_finite() {
                            return Err(CurvatureError::NonFiniteLoweredRiemannComponent {
                                alpha,
                                sigma,
                                mu,
                                nu,
                            });
                        }
                        lowered[alpha][sigma][mu][nu] = value;
                    }
                }
            }
        }

        Ok(RiemannTensorCovariant4 {
            components: lowered,
        })
    }

    /// Ricci tensor R_(sigma nu) = R^rho_(sigma rho nu).
    pub fn ricci(&self) -> [[f64; 4]; 4] {
        let mut ricci = [[0.0; 4]; 4];
        for sigma in 0..4 {
            for nu in 0..4 {
                for rho in 0..4 {
                    ricci[sigma][nu] += self.components[rho][sigma][rho][nu];
                }
            }
        }
        ricci
    }

    /// Scalar curvature R = g^mu nu R_mu nu.
    pub fn scalar_curvature(
        &self,
        metric: &MetricComponents4,
    ) -> Result<f64, CurvatureError> {
        let inverse = metric.inverse()?;
        let ricci = self.ricci();
        let mut scalar = 0.0;

        for mu in 0..4 {
            for nu in 0..4 {
                scalar += inverse[mu][nu] * ricci[mu][nu];
            }
        }

        if scalar.is_finite() {
            Ok(scalar)
        } else {
            Err(CurvatureError::NonFiniteScalarCurvature)
        }
    }

    /// Einstein tensor G_mu nu = R_mu nu - 1/2 g_mu nu R.
    pub fn einstein_tensor(
        &self,
        metric: &MetricComponents4,
    ) -> Result<[[f64; 4]; 4], CurvatureError> {
        let scalar = self.scalar_curvature(metric)?;
        let ricci = self.ricci();
        let g = metric.covariant();
        let mut einstein = [[0.0; 4]; 4];

        for mu in 0..4 {
            for nu in 0..4 {
                einstein[mu][nu] = ricci[mu][nu] - 0.5 * g[mu][nu] * scalar;
                if !einstein[mu][nu].is_finite() {
                    return Err(CurvatureError::NonFiniteEinsteinComponent { mu, nu });
                }
            }
        }

        Ok(einstein)
    }

    /// Kretschmann scalar R_abcd R^abcd.
    ///
    /// The result is coordinate invariant (unlike the individual coordinate
    /// components and metric determinant).
    pub fn kretschmann_scalar(
        &self,
        metric: &MetricComponents4,
    ) -> Result<f64, CurvatureError> {
        let lowered = self.lower_first_index(metric)?;
        lowered.kretschmann_scalar(metric)
    }

    /// Maximum absolute Ricci component. Coordinate-dependent and intended
    /// only as a diagnostic within one declared chart and unit convention.
    pub fn max_abs_ricci_component(&self) -> f64 {
        self.ricci()
            .iter()
            .flatten()
            .fold(0.0_f64, |maximum, value| maximum.max(value.abs()))
    }
}

/// Fully covariant Riemann tensor R_(alpha beta gamma delta).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RiemannTensorCovariant4 {
    components: [[[[f64; 4]; 4]; 4]; 4],
}

impl RiemannTensorCovariant4 {
    pub const fn components(&self) -> &[[[[f64; 4]; 4]; 4]; 4] {
        &self.components
    }

    /// Maximum absolute residual across pair antisymmetry, pair exchange and
    /// the algebraic first Bianchi identity.
    pub fn algebraic_symmetry_residual(&self) -> f64 {
        let r = &self.components;
        let mut maximum: f64 = 0.0;

        for a in 0..4 {
            for b in 0..4 {
                for c in 0..4 {
                    for d in 0..4 {
                        maximum = maximum.max((r[a][b][c][d] + r[b][a][c][d]).abs());
                        maximum = maximum.max((r[a][b][c][d] + r[a][b][d][c]).abs());
                        maximum = maximum.max((r[a][b][c][d] - r[c][d][a][b]).abs());

                        let bianchi =
                            r[a][b][c][d] + r[a][c][d][b] + r[a][d][b][c];
                        maximum = maximum.max(bianchi.abs());
                    }
                }
            }
        }

        maximum
    }

    /// Contract all four Riemann indices with the inverse metric:
    /// K = g^ae g^bf g^cg g^dh R_abcd R_efgh.
    pub fn kretschmann_scalar(
        &self,
        metric: &MetricComponents4,
    ) -> Result<f64, CurvatureError> {
        let inverse = metric.inverse()?;
        let mut result = 0.0;

        for a in 0..4 {
            for b in 0..4 {
                for c in 0..4 {
                    for d in 0..4 {
                        let rabcd = self.components[a][b][c][d];
                        if rabcd == 0.0 {
                            continue;
                        }

                        for e in 0..4 {
                            let gae = inverse[a][e];
                            if gae == 0.0 {
                                continue;
                            }
                            for f in 0..4 {
                                let gbf = inverse[b][f];
                                if gbf == 0.0 {
                                    continue;
                                }
                                for g in 0..4 {
                                    let gcg = inverse[c][g];
                                    if gcg == 0.0 {
                                        continue;
                                    }
                                    for h in 0..4 {
                                        let gdh = inverse[d][h];
                                        if gdh == 0.0 {
                                            continue;
                                        }
                                        result += rabcd
                                            * self.components[e][f][g][h]
                                            * gae
                                            * gbf
                                            * gcg
                                            * gdh;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if result.is_finite() {
            Ok(result)
        } else {
            Err(CurvatureError::NonFiniteKretschmannScalar)
        }
    }
}

/// Errors produced by connection-derivative and curvature calculations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CurvatureError {
    PhysicsDomain(PhysicsDomainError),
    Metric(MetricError),
    Connection(ConnectionError),
    NonFiniteConnectionDerivative {
        derivative_index: usize,
        upper: usize,
        lower_first: usize,
        lower_second: usize,
    },
    AsymmetricConnectionDerivative {
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
    NonFiniteLoweredRiemannComponent {
        alpha: usize,
        sigma: usize,
        mu: usize,
        nu: usize,
    },
    NonFiniteScalarCurvature,
    NonFiniteEinsteinComponent {
        mu: usize,
        nu: usize,
    },
    NonFiniteKretschmannScalar,
}

impl From<PhysicsDomainError> for CurvatureError {
    fn from(value: PhysicsDomainError) -> Self {
        Self::PhysicsDomain(value)
    }
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

impl fmt::Display for CurvatureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PhysicsDomain(error) => write!(f, "{error}"),
            Self::Metric(error) => write!(f, "{error}"),
            Self::Connection(error) => write!(f, "{error}"),
            Self::NonFiniteConnectionDerivative {
                derivative_index,
                upper,
                lower_first,
                lower_second,
            } => write!(
                f,
                "connection derivative d[{derivative_index}] Gamma^{upper}_{{{lower_first}{lower_second}}} is non-finite"
            ),
            Self::AsymmetricConnectionDerivative {
                derivative_index,
                upper,
                lower_first,
                lower_second,
            } => write!(
                f,
                "connection derivative d[{derivative_index}] Gamma^{upper}_{{{lower_first}{lower_second}}} is asymmetric"
            ),
            Self::NonFiniteRiemannComponent {
                upper,
                lower,
                first_derivative,
                second_derivative,
            } => write!(
                f,
                "Riemann component R^{upper}_{{{lower},{first_derivative},{second_derivative}}} is non-finite"
            ),
            Self::NonFiniteLoweredRiemannComponent { alpha, sigma, mu, nu } => write!(
                f,
                "lowered Riemann component R_{{{alpha},{sigma},{mu},{nu}}} is non-finite"
            ),
            Self::NonFiniteScalarCurvature => write!(f, "scalar curvature is non-finite"),
            Self::NonFiniteEinsteinComponent { mu, nu } => {
                write!(f, "Einstein component G_{{{mu},{nu}}} is non-finite")
            }
            Self::NonFiniteKretschmannScalar => {
                write!(f, "Kretschmann scalar is non-finite")
            }
        }
    }
}

impl std::error::Error for CurvatureError {}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::{
        connection::MetricDerivatives4,
        metric::MetricComponents4,
        MassKg, SchwarzschildBlackHole,
    };

    const SOLAR_MASS_KG: f64 = 1.988_47e30;

    fn solar_mass() -> MassKg {
        MassKg::new(SOLAR_MASS_KG).unwrap()
    }

    fn max_abs_matrix(matrix: [[f64; 4]; 4]) -> f64 {
        matrix
            .iter()
            .flatten()
            .fold(0.0_f64, |maximum, value| maximum.max(value.abs()))
    }

    #[test]
    fn cartesian_minkowski_curvature_is_zero() {
        let metric = MetricComponents4::new([
            [-1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
        .unwrap();
        let connection = LeviCivitaConnection4::from_metric_and_derivatives(
            &metric,
            &MetricDerivatives4::zero(),
        )
        .unwrap();
        let riemann = RiemannTensor4::from_connection_and_derivatives(
            &connection,
            &ConnectionDerivatives4::zero(),
        )
        .unwrap();

        assert_eq!(riemann.max_abs_ricci_component(), 0.0);
        assert_eq!(riemann.scalar_curvature(&metric).unwrap(), 0.0);
        assert_eq!(riemann.kretschmann_scalar(&metric).unwrap(), 0.0);
    }

    #[test]
    fn schwarzschild_vacuum_ricci_and_einstein_tensors_vanish() {
        let hole = SchwarzschildBlackHole::new(solar_mass());
        let radius = 5.0 * hole.gravitational_radius_m();
        let theta = 0.8;
        let metric = schwarzschild_exterior_metric(hole, radius, theta).unwrap();
        let first_derivatives =
            MetricDerivatives4::schwarzschild_exterior(hole, radius, theta).unwrap();
        let connection = LeviCivitaConnection4::from_metric_and_derivatives(
            &metric,
            &first_derivatives,
        )
        .unwrap();
        let second_derivatives =
            ConnectionDerivatives4::schwarzschild_exterior(hole, radius, theta).unwrap();
        let riemann = RiemannTensor4::from_connection_and_derivatives(
            &connection,
            &second_derivatives,
        )
        .unwrap();

        assert!(riemann.max_abs_ricci_component() < 2e-13);
        assert!(riemann.scalar_curvature(&metric).unwrap().abs() < 2e-13);
        assert!(max_abs_matrix(riemann.einstein_tensor(&metric).unwrap()) < 3e-13);
    }

    #[test]
    fn schwarzschild_kretschmann_matches_closed_form() {
        let hole = SchwarzschildBlackHole::new(solar_mass());
        let radius = 5.0 * hole.gravitational_radius_m();
        let theta = 1.1;
        let metric = schwarzschild_exterior_metric(hole, radius, theta).unwrap();
        let connection = LeviCivitaConnection4::from_metric_and_derivatives(
            &metric,
            &MetricDerivatives4::schwarzschild_exterior(hole, radius, theta).unwrap(),
        )
        .unwrap();
        let riemann = RiemannTensor4::from_connection_and_derivatives(
            &connection,
            &ConnectionDerivatives4::schwarzschild_exterior(hole, radius, theta).unwrap(),
        )
        .unwrap();

        let actual = riemann.kretschmann_scalar(&metric).unwrap();
        let expected = 48.0 * hole.gravitational_radius_m().powi(2) / radius.powi(6);
        let relative_error = (actual - expected).abs() / expected.abs();

        assert!(relative_error < 2e-12, "relative error={relative_error:e}");
    }

    #[test]
    fn schwarzschild_riemann_obeys_algebraic_symmetries_and_bianchi_identity() {
        let hole = SchwarzschildBlackHole::new(solar_mass());
        let radius = 6.0 * hole.gravitational_radius_m();
        let theta = 0.9;
        let metric = schwarzschild_exterior_metric(hole, radius, theta).unwrap();
        let connection = LeviCivitaConnection4::from_metric_and_derivatives(
            &metric,
            &MetricDerivatives4::schwarzschild_exterior(hole, radius, theta).unwrap(),
        )
        .unwrap();
        let riemann = RiemannTensor4::from_connection_and_derivatives(
            &connection,
            &ConnectionDerivatives4::schwarzschild_exterior(hole, radius, theta).unwrap(),
        )
        .unwrap();
        let lowered = riemann.lower_first_index(&metric).unwrap();

        assert!(lowered.algebraic_symmetry_residual() < 3e-13);
    }

    #[test]
    fn schwarzschild_connection_derivative_fixture_rejects_invalid_chart() {
        let hole = SchwarzschildBlackHole::new(solar_mass());
        assert!(matches!(
            ConnectionDerivatives4::schwarzschild_exterior(
                hole,
                hole.horizon_radius_m(),
                1.0
            ),
            Err(CurvatureError::Metric(MetricError::PhysicsDomain(
                PhysicsDomainError::RadiusAtOrInsideHorizon { .. }
            )))
        ));
        assert!(matches!(
            ConnectionDerivatives4::schwarzschild_exterior(
                hole,
                4.0 * hole.gravitational_radius_m(),
                0.0
            ),
            Err(CurvatureError::Metric(
                MetricError::PolarAxisCoordinateSingularity
            ))
        ));
    }

    #[test]
    fn connection_derivative_constructor_rejects_nonfinite_and_asymmetric_data() {
        let mut derivatives = [[[[0.0; 4]; 4]; 4]; 4];
        derivatives[1][0][0][1] = 1.0;
        assert!(matches!(
            ConnectionDerivatives4::new(derivatives),
            Err(CurvatureError::AsymmetricConnectionDerivative { .. })
        ));

        derivatives[1][0][0][1] = 0.0;
        derivatives[1][0][1][0] = 0.0;
        derivatives[1][2][3][3] = f64::NAN;
        assert!(matches!(
            ConnectionDerivatives4::new(derivatives),
            Err(CurvatureError::NonFiniteConnectionDerivative { .. })
        ));
    }
}
