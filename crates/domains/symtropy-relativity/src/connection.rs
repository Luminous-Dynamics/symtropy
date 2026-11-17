#![allow(clippy::needless_range_loop)]
// Fixed 4D tensor contractions use explicit Einstein-index loops.

// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Levi-Civita connection for a four-dimensional metric and its first
//! coordinate derivatives.
//!
//! Derivative storage is d[alpha][mu][nu] = partial_alpha g_{mu nu}.
//! Connection storage is Gamma[rho][mu][nu] = Gamma^rho_{mu nu}.
//! The last two metric indices and the lower two connection indices are
//! symmetric by construction/check. Coordinate convention is supplied by
//! the caller and must match the metric and derivative tensors.
//!
//! This module does not numerically differentiate a metric. It accepts an
//! explicit derivative tensor, avoiding hidden finite-difference step sizes.
//! Schwarzschild exterior derivatives are supplied as one analytic fixture;
//! Kerr derivatives and curvature/Riemann tensors are later work.

use core::fmt;

use super::{
    PhysicsDomainError, SchwarzschildBlackHole,
    metric::{MetricComponents4, MetricError, schwarzschild_exterior_metric},
};

/// First coordinate derivatives of a covariant 4x4 metric.
///
/// Index order: components[alpha][mu][nu] = partial_alpha g_(mu nu).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MetricDerivatives4 {
    components: [[[f64; 4]; 4]; 4],
}

impl MetricDerivatives4 {
    /// Validate a derivative tensor for finite values and metric-index symmetry.
    pub fn new(mut components: [[[f64; 4]; 4]; 4]) -> Result<Self, ConnectionError> {
        for alpha in 0..4 {
            for mu in 0..4 {
                for nu in 0..4 {
                    let value = components[alpha][mu][nu];
                    if !value.is_finite() {
                        return Err(ConnectionError::NonFiniteMetricDerivative {
                            derivative_index: alpha,
                            row: mu,
                            column: nu,
                        });
                    }
                }
            }
            for mu in 0..4 {
                for nu in (mu + 1)..4 {
                    let a = components[alpha][mu][nu];
                    let b = components[alpha][nu][mu];
                    let scale = a.abs().max(b.abs()).max(f64::MIN_POSITIVE);
                    let tolerance = 64.0 * f64::EPSILON * scale;
                    if (a - b).abs() > tolerance {
                        return Err(ConnectionError::AsymmetricMetricDerivative {
                            derivative_index: alpha,
                            row: mu,
                            column: nu,
                        });
                    }
                    let mean = 0.5 * a + 0.5 * b;
                    components[alpha][mu][nu] = mean;
                    components[alpha][nu][mu] = mean;
                }
            }
        }
        Ok(Self { components })
    }

    /// Constant metric derivative tensor (e.g. Cartesian Minkowski coordinates).
    pub const fn zero() -> Self {
        Self {
            components: [[[0.0; 4]; 4]; 4],
        }
    }

    pub const fn components(&self) -> &[[[f64; 4]; 4]; 4] {
        &self.components
    }

    /// Exact first derivatives of the Schwarzschild metric in coordinates
    /// (ct, r, theta, phi), where ct and r are in metres.
    ///
    /// Domain is the Schwarzschild exterior r > 2GM/c^2 and 0 < theta < pi.
    pub fn schwarzschild_exterior(
        hole: SchwarzschildBlackHole,
        radius_m: f64,
        theta_rad: f64,
    ) -> Result<Self, ConnectionError> {
        // Reuse the metric constructor as the single chart/domain authority.
        let _metric = schwarzschild_exterior_metric(hole, radius_m, theta_rad)?;
        let rg = hole.gravitational_radius_m();
        let f = 1.0 - hole.horizon_radius_m() / radius_m;
        let sin_theta = theta_rad.sin();
        let cos_theta = theta_rad.cos();

        let mut d = [[[0.0; 4]; 4]; 4];

        // Radial derivatives.
        d[1][0][0] = -2.0 * rg / radius_m.powi(2);
        d[1][1][1] = -2.0 * rg / (radius_m.powi(2) * f.powi(2));
        d[1][2][2] = 2.0 * radius_m;
        d[1][3][3] = 2.0 * radius_m * sin_theta.powi(2);

        // Polar-angle derivative.
        d[2][3][3] = 2.0 * radius_m.powi(2) * sin_theta * cos_theta;

        Self::new(d)
    }
}

/// Levi-Civita connection coefficients in the coordinate basis of the metric.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LeviCivitaConnection4 {
    symbols: [[[f64; 4]; 4]; 4],
}

impl LeviCivitaConnection4 {
    /// Calculate Gamma^rho_(mu nu) = 1/2 g^(rho sigma)
    /// (partial_mu g_(sigma nu) + partial_nu g_(sigma mu)
    ///  - partial_sigma g_(mu nu)).
    ///
    /// The metric and derivative tensor must use the same coordinate order,
    /// chart, units, and signature convention.
    pub fn from_metric_and_derivatives(
        metric: &MetricComponents4,
        derivatives: &MetricDerivatives4,
    ) -> Result<Self, ConnectionError> {
        let inverse = metric.inverse()?;
        let d = derivatives.components();
        let mut symbols = [[[0.0; 4]; 4]; 4];

        for rho in 0..4 {
            for mu in 0..4 {
                for nu in 0..4 {
                    let mut value = 0.0;
                    for sigma in 0..4 {
                        value += inverse[rho][sigma]
                            * (d[mu][sigma][nu] + d[nu][sigma][mu] - d[sigma][mu][nu]);
                    }
                    value *= 0.5;
                    if !value.is_finite() {
                        return Err(ConnectionError::NonFiniteConnection {
                            upper: rho,
                            lower_first: mu,
                            lower_second: nu,
                        });
                    }
                    symbols[rho][mu][nu] = value;
                }
            }
        }

        // Numerical accumulation should preserve lower-index symmetry; make
        // that contract exact for downstream equality / deterministic replay.
        for rho in 0..4 {
            for mu in 0..4 {
                for nu in (mu + 1)..4 {
                    let mean = 0.5 * symbols[rho][mu][nu] + 0.5 * symbols[rho][nu][mu];
                    symbols[rho][mu][nu] = mean;
                    symbols[rho][nu][mu] = mean;
                }
            }
        }

        Ok(Self { symbols })
    }

    /// Gamma[rho][mu][nu] = Gamma^rho_(mu nu).
    pub const fn symbols(&self) -> &[[[f64; 4]; 4]; 4] {
        &self.symbols
    }

    pub fn coefficient(&self, upper: usize, lower_first: usize, lower_second: usize) -> f64 {
        self.symbols[upper][lower_first][lower_second]
    }

    /// Maximum scale-normalized residual of metric compatibility:
    /// nabla_alpha g_(mu nu) = partial_alpha g_(mu nu)
    ///   - Gamma^sigma_(alpha mu) g_(sigma nu)
    ///   - Gamma^sigma_(alpha nu) g_(mu sigma) = 0.
    ///
    /// This is a diagnostic, not a rigorous floating-point error bound.
    pub fn metric_compatibility_relative_residual(
        &self,
        metric: &MetricComponents4,
        derivatives: &MetricDerivatives4,
    ) -> Result<f64, ConnectionError> {
        let g = metric.covariant();
        let d = derivatives.components();
        let mut maximum: f64 = 0.0;

        for alpha in 0..4 {
            for mu in 0..4 {
                for nu in 0..4 {
                    let mut first = 0.0;
                    let mut second = 0.0;
                    for sigma in 0..4 {
                        first += self.symbols[sigma][alpha][mu] * g[sigma][nu];
                        second += self.symbols[sigma][alpha][nu] * g[mu][sigma];
                    }
                    let derivative = d[alpha][mu][nu];
                    let residual = derivative - first - second;
                    let scale = derivative.abs().max(first.abs()).max(second.abs());
                    let normalized = if scale == 0.0 {
                        residual.abs()
                    } else {
                        residual.abs() / scale
                    };
                    if !normalized.is_finite() {
                        return Err(ConnectionError::NonFiniteMetricCompatibility);
                    }
                    maximum = maximum.max(normalized);
                }
            }
        }
        Ok(maximum)
    }

    /// Maximum absolute asymmetry between the two lower connection indices.
    pub fn lower_index_symmetry_residual(&self) -> f64 {
        let mut maximum: f64 = 0.0;
        for rho in 0..4 {
            for mu in 0..4 {
                for nu in 0..4 {
                    maximum =
                        maximum.max((self.symbols[rho][mu][nu] - self.symbols[rho][nu][mu]).abs());
                }
            }
        }
        maximum
    }
}

/// Errors while constructing metric derivatives or a Levi-Civita connection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionError {
    Metric(MetricError),
    NonFiniteMetricDerivative {
        derivative_index: usize,
        row: usize,
        column: usize,
    },
    AsymmetricMetricDerivative {
        derivative_index: usize,
        row: usize,
        column: usize,
    },
    NonFiniteConnection {
        upper: usize,
        lower_first: usize,
        lower_second: usize,
    },
    NonFiniteMetricCompatibility,
}

impl From<MetricError> for ConnectionError {
    fn from(value: MetricError) -> Self {
        Self::Metric(value)
    }
}

impl From<PhysicsDomainError> for ConnectionError {
    fn from(value: PhysicsDomainError) -> Self {
        Self::Metric(MetricError::PhysicsDomain(value))
    }
}

impl fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Metric(error) => write!(f, "{error}"),
            Self::NonFiniteMetricDerivative {
                derivative_index,
                row,
                column,
            } => write!(
                f,
                "metric derivative d[{derivative_index}][{row}][{column}] is non-finite"
            ),
            Self::AsymmetricMetricDerivative {
                derivative_index,
                row,
                column,
            } => write!(
                f,
                "metric derivative d[{derivative_index}] is asymmetric at ({row}, {column})"
            ),
            Self::NonFiniteConnection {
                upper,
                lower_first,
                lower_second,
            } => write!(
                f,
                "connection coefficient Gamma^{upper}_{{{lower_first}{lower_second}}} is non-finite"
            ),
            Self::NonFiniteMetricCompatibility => {
                write!(f, "metric compatibility residual is non-finite")
            }
        }
    }
}

impl std::error::Error for ConnectionError {}

#[cfg(test)]
mod tests {
    use super::super::{MassKg, SchwarzschildBlackHole};
    use super::*;

    const SOLAR_MASS_KG: f64 = 1.988_47e30;

    fn solar_mass() -> MassKg {
        MassKg::new(SOLAR_MASS_KG).unwrap()
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

    #[test]
    fn cartesian_minkowski_metric_has_zero_connection() {
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

        for rho in 0..4 {
            for mu in 0..4 {
                for nu in 0..4 {
                    assert_eq!(connection.coefficient(rho, mu, nu), 0.0);
                }
            }
        }
    }

    #[test]
    fn schwarzschild_connection_matches_analytic_coefficients() {
        let hole = SchwarzschildBlackHole::new(solar_mass());
        let radius = 5.0 * hole.gravitational_radius_m();
        let theta = 0.8;
        let metric = schwarzschild_exterior_metric(hole, radius, theta).unwrap();
        let derivatives = MetricDerivatives4::schwarzschild_exterior(hole, radius, theta).unwrap();
        let connection =
            LeviCivitaConnection4::from_metric_and_derivatives(&metric, &derivatives).unwrap();

        let rg = hole.gravitational_radius_m();
        let f = 1.0 - hole.horizon_radius_m() / radius;
        close(
            connection.coefficient(1, 0, 0),
            f * rg / radius.powi(2),
            2e-13,
        );
        close(
            connection.coefficient(0, 0, 1),
            rg / (f * radius.powi(2)),
            2e-13,
        );
        close(
            connection.coefficient(1, 1, 1),
            -rg / (f * radius.powi(2)),
            2e-13,
        );
        close(connection.coefficient(1, 2, 2), -f * radius, 2e-13);
        close(connection.coefficient(2, 1, 2), 1.0 / radius, 2e-13);
        close(
            connection.coefficient(2, 3, 3),
            -theta.sin() * theta.cos(),
            2e-13,
        );
        close(
            connection.coefficient(3, 2, 3),
            theta.cos() / theta.sin(),
            2e-13,
        );
    }

    #[test]
    fn schwarzschild_connection_is_metric_compatible() {
        let hole = SchwarzschildBlackHole::new(solar_mass());
        let radius = 6.0 * hole.gravitational_radius_m();
        let theta = 0.9;
        let metric = schwarzschild_exterior_metric(hole, radius, theta).unwrap();
        let derivatives = MetricDerivatives4::schwarzschild_exterior(hole, radius, theta).unwrap();
        let connection =
            LeviCivitaConnection4::from_metric_and_derivatives(&metric, &derivatives).unwrap();

        assert!(
            connection
                .metric_compatibility_relative_residual(&metric, &derivatives)
                .unwrap()
                < 2e-15
        );
    }

    #[test]
    fn connection_is_symmetric_in_its_lower_indices() {
        let hole = SchwarzschildBlackHole::new(solar_mass());
        let radius = 4.0 * hole.gravitational_radius_m();
        let theta = 1.0;
        let metric = schwarzschild_exterior_metric(hole, radius, theta).unwrap();
        let derivatives = MetricDerivatives4::schwarzschild_exterior(hole, radius, theta).unwrap();
        let connection =
            LeviCivitaConnection4::from_metric_and_derivatives(&metric, &derivatives).unwrap();

        assert_eq!(connection.lower_index_symmetry_residual(), 0.0);
        for rho in 0..4 {
            for mu in 0..4 {
                for nu in 0..4 {
                    assert_eq!(
                        connection.coefficient(rho, mu, nu),
                        connection.coefficient(rho, nu, mu)
                    );
                }
            }
        }
    }

    #[test]
    fn metric_derivative_constructor_rejects_asymmetric_and_nonfinite_data() {
        let mut derivatives = [[[0.0; 4]; 4]; 4];
        derivatives[1][0][1] = 1.0;
        assert!(matches!(
            MetricDerivatives4::new(derivatives),
            Err(ConnectionError::AsymmetricMetricDerivative { .. })
        ));

        derivatives[1][0][1] = 0.0;
        derivatives[1][1][0] = 0.0;
        derivatives[2][3][3] = f64::NAN;
        assert!(matches!(
            MetricDerivatives4::new(derivatives),
            Err(ConnectionError::NonFiniteMetricDerivative { .. })
        ));
    }

    #[test]
    fn schwarzschild_derivative_fixture_rejects_horizon_and_polar_axis() {
        let hole = SchwarzschildBlackHole::new(solar_mass());
        assert!(matches!(
            MetricDerivatives4::schwarzschild_exterior(hole, hole.horizon_radius_m(), 1.0),
            Err(ConnectionError::Metric(MetricError::PhysicsDomain(
                PhysicsDomainError::RadiusAtOrInsideHorizon { .. }
            )))
        ));
        assert!(matches!(
            MetricDerivatives4::schwarzschild_exterior(
                hole,
                4.0 * hole.gravitational_radius_m(),
                0.0
            ),
            Err(ConnectionError::Metric(
                MetricError::PolarAxisCoordinateSingularity
            ))
        ));
    }
}
