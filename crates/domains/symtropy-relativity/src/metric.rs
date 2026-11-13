#![allow(clippy::needless_range_loop)]
// Fixed 4D tensor contractions use explicit Einstein-index loops.

// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Analytic stationary black-hole metrics in the exterior coordinate charts.
//! Coordinates are x^mu=(c*t, r, theta, phi), with c*t and r in metres,
//! angles in radians, ds^2 in m^2, and signature (-,+,+,+).
//! Schwarzschild and Boyer-Lindquist charts exclude the horizon and polar axis.
//! This is not an evolved spacetime, a connection, or a numerical-relativity solver.

use super::{KerrBlackHole, PhysicsDomainError, SchwarzschildBlackHole};
use core::fmt;

/// Finite symmetric 4x4 covariant metric components in one coordinate basis.
/// Generic construction checks finiteness/symmetry, not Lorentzian signature.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MetricComponents4 {
    covariant: [[f64; 4]; 4],
    /// Positive diagonal congruence scales used for numerically conditioned
    /// tensor contractions and inversion. They do not change the represented
    /// metric; the public covariant components remain in the original basis.
    basis_scales: [f64; 4],
}

impl MetricComponents4 {
    /// Construct with the identity congruence scale. This is appropriate
    /// for generic matrices whose coordinate components already share a
    /// common numerical scale.
    pub fn new(covariant: [[f64; 4]; 4]) -> Result<Self, MetricError> {
        Self::new_with_basis_scales(covariant, [1.0; 4])
    }

    /// Construct a metric while retaining positive diagonal basis scales for
    /// stable inversion and residual evaluation.
    ///
    /// The normalized matrix is B = D g D, where D = diag(basis_scales).
    /// Its inverse is mapped back as g^{-1} = D B^{-1} D. The represented
    /// covariant tensor and its public coordinate components are unchanged.
    /// For SI Boyer-Lindquist / spherical charts (ct,r,theta,phi), a useful
    /// scale is [1,1,1/r_g,1/r_g], which removes the metre and square-metre
    /// disparity in the angular blocks.
    pub fn new_with_basis_scales(
        mut covariant: [[f64; 4]; 4],
        basis_scales: [f64; 4],
    ) -> Result<Self, MetricError> {
        for (i, scale) in basis_scales.iter().copied().enumerate() {
            if !scale.is_finite() {
                return Err(MetricError::ComputationNonFinite("metric basis scale"));
            }
            if scale <= 0.0 {
                return Err(MetricError::InvalidBasisScale {
                    coordinate: i,
                    value: scale,
                });
            }
        }
        for (i, row) in covariant.iter().enumerate() {
            for (j, value) in row.iter().enumerate() {
                if !value.is_finite() {
                    return Err(MetricError::NonFiniteComponent { row: i, column: j });
                }
            }
        }
        for i in 0..4 {
            for j in (i + 1)..4 {
                let a = covariant[i][j];
                let b = covariant[j][i];
                let component_scale = a.abs().max(b.abs()).max(f64::MIN_POSITIVE);
                let tol = 64.0 * f64::EPSILON * component_scale;
                if (a - b).abs() > tol {
                    return Err(MetricError::AsymmetricComponents { row: i, column: j });
                }
                let mean = 0.5 * a + 0.5 * b;
                covariant[i][j] = mean;
                covariant[j][i] = mean;
            }
        }
        Ok(Self {
            covariant,
            basis_scales,
        })
    }

    pub const fn basis_scales(&self) -> &[f64; 4] {
        &self.basis_scales
    }

    pub const fn covariant(&self) -> &[[f64; 4]; 4] {
        &self.covariant
    }

    /// Coordinate-dependent determinant of the covariant component matrix.
    pub fn determinant(&self) -> Result<f64, MetricError> {
        let mut a = self.covariant;
        let mut sign = 1.0;
        let mut det = 1.0;
        for col in 0..4 {
            let p = (col..4)
                .max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))
                .ok_or(MetricError::SingularMatrix)?;
            if a[p][col] == 0.0 {
                return Err(MetricError::SingularMatrix);
            }
            if p != col {
                a.swap(p, col);
                sign = -sign;
            }
            let diagonal = a[col][col];
            det *= diagonal;
            if !det.is_finite() {
                return Err(MetricError::ComputationNonFinite("metric determinant"));
            }
            for row in (col + 1)..4 {
                let factor = a[row][col] / diagonal;
                for k in (col + 1)..4 {
                    a[row][k] -= factor * a[col][k];
                    if !a[row][k].is_finite() {
                        return Err(MetricError::ComputationNonFinite("determinant elimination"));
                    }
                }
                a[row][col] = 0.0;
            }
        }
        let result = sign * det;
        if result.is_finite() {
            Ok(result)
        } else {
            Err(MetricError::ComputationNonFinite("metric determinant"))
        }
    }

    /// Inverse of B = D g D in the numerically scaled basis, where D is
    /// the retained positive diagonal basis scale.
    fn inverse_in_scaled_basis(&self) -> Result<[[f64; 4]; 4], MetricError> {
        let mut a = [[0.0; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                a[i][j] = self.basis_scales[i] * self.covariant[i][j] * self.basis_scales[j];
                if !a[i][j].is_finite() {
                    return Err(MetricError::ComputationNonFinite("scaled metric component"));
                }
            }
        }

        let mut inv = [[0.0; 4]; 4];
        for i in 0..4 {
            inv[i][i] = 1.0;
        }
        for col in 0..4 {
            let p = (col..4)
                .max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))
                .ok_or(MetricError::SingularMatrix)?;
            if a[p][col] == 0.0 || !a[p][col].is_finite() {
                return Err(MetricError::SingularMatrix);
            }
            if p != col {
                a.swap(p, col);
                inv.swap(p, col);
            }
            let d = a[col][col];
            for k in 0..4 {
                a[col][k] /= d;
                inv[col][k] /= d;
                if !a[col][k].is_finite() || !inv[col][k].is_finite() {
                    return Err(MetricError::ComputationNonFinite("metric inverse"));
                }
            }
            for row in 0..4 {
                if row == col {
                    continue;
                }
                let factor = a[row][col];
                for k in 0..4 {
                    a[row][k] -= factor * a[col][k];
                    inv[row][k] -= factor * inv[col][k];
                    if !a[row][k].is_finite() || !inv[row][k].is_finite() {
                        return Err(MetricError::ComputationNonFinite("metric inverse"));
                    }
                }
            }
        }
        Ok(inv)
    }

    /// Contravariant matrix in the original coordinate basis.
    ///
    /// The inversion is performed on B = D g D, then transformed back as
    /// g^{-1} = D B^{-1} D. This avoids mixing poorly scaled SI components
    /// in the elimination pivots.
    pub fn inverse(&self) -> Result<[[f64; 4]; 4], MetricError> {
        let mut inv = self.inverse_in_scaled_basis()?;
        for i in 0..4 {
            for j in 0..4 {
                inv[i][j] *= self.basis_scales[i] * self.basis_scales[j];
                if !inv[i][j].is_finite() {
                    return Err(MetricError::ComputationNonFinite("metric inverse"));
                }
            }
        }
        Ok(inv)
    }

    /// Maximum absolute identity residual in the declared normalized basis:
    /// max |(D g D)(D^{-1} g^{-1} D^{-1}) - I|.
    ///
    /// Unlike a raw componentwise residual in a dimensionful coordinate basis,
    /// this diagnostic is invariant to the retained positive diagonal
    /// congruence scaling used by this metric object.
    pub fn inverse_identity_residual(&self) -> Result<f64, MetricError> {
        let inv = self.inverse_in_scaled_basis()?;
        let mut max: f64 = 0.0;
        for i in 0..4 {
            for j in 0..4 {
                let mut sum = 0.0;
                for k in 0..4 {
                    let normalized =
                        self.basis_scales[i] * self.covariant[i][k] * self.basis_scales[k];
                    sum += normalized * inv[k][j];
                }
                let expected = if i == j { 1.0 } else { 0.0 };
                max = max.max((sum - expected).abs());
            }
        }
        if max.is_finite() {
            Ok(max)
        } else {
            Err(MetricError::ComputationNonFinite(
                "inverse identity residual",
            ))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MetricError {
    PhysicsDomain(PhysicsDomainError),
    PolarAxisCoordinateSingularity,
    NonFiniteComponent { row: usize, column: usize },
    AsymmetricComponents { row: usize, column: usize },
    SingularMatrix,
    InvalidBasisScale { coordinate: usize, value: f64 },
    ComputationNonFinite(&'static str),
}

impl From<PhysicsDomainError> for MetricError {
    fn from(value: PhysicsDomainError) -> Self {
        Self::PhysicsDomain(value)
    }
}

impl fmt::Display for MetricError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PhysicsDomain(e) => write!(f, "{e}"),
            Self::PolarAxisCoordinateSingularity => write!(f, "chart excludes theta = 0 and pi"),
            Self::NonFiniteComponent { row, column } => {
                write!(f, "metric component g[{row}][{column}] is not finite")
            }
            Self::AsymmetricComponents { row, column } => {
                write!(f, "metric is not symmetric at ({row}, {column})")
            }
            Self::SingularMatrix => write!(f, "metric component matrix is singular"),
            Self::InvalidBasisScale { coordinate, value } => {
                write!(
                    f,
                    "metric basis scale for coordinate {coordinate} must be positive, got {value}"
                )
            }
            Self::ComputationNonFinite(op) => write!(f, "{op} produced a non-finite result"),
        }
    }
}
impl std::error::Error for MetricError {}

fn validate_theta(theta: f64) -> Result<(), MetricError> {
    if !theta.is_finite() {
        return Err(PhysicsDomainError::NonFinite("theta_rad").into());
    }
    if theta <= 0.0 || theta >= core::f64::consts::PI {
        return Err(MetricError::PolarAxisCoordinateSingularity);
    }
    Ok(())
}

/// Schwarzschild metric in Schwarzschild coordinates (ct,r,theta,phi).
/// Domain: r > 2GM/c^2 and 0 < theta < pi.
pub fn schwarzschild_exterior_metric(
    hole: SchwarzschildBlackHole,
    radius_m: f64,
    theta_rad: f64,
) -> Result<MetricComponents4, MetricError> {
    super::validate_positive_finite(radius_m, "radius_m")?;
    validate_theta(theta_rad)?;
    let rs = hole.horizon_radius_m();
    if radius_m <= rs {
        return Err(PhysicsDomainError::RadiusAtOrInsideHorizon {
            radius_m,
            horizon_m: rs,
        }
        .into());
    }
    let f = 1.0 - rs / radius_m;
    if !f.is_finite() || f <= 0.0 {
        return Err(MetricError::ComputationNonFinite(
            "Schwarzschild metric coefficient",
        ));
    }
    let r2 = radius_m * radius_m;
    let s2 = theta_rad.sin().powi(2);
    let rg = 0.5 * rs;
    MetricComponents4::new_with_basis_scales(
        [
            [-f, 0.0, 0.0, 0.0],
            [0.0, 1.0 / f, 0.0, 0.0],
            [0.0, 0.0, r2, 0.0],
            [0.0, 0.0, 0.0, r2 * s2],
        ],
        [1.0, 1.0, 1.0 / rg, 1.0 / rg],
    )
}

/// Schwarzschild metric in ingoing Eddington-Finkelstein coordinates
/// (v, r, theta, phi), where v = c*t + r_* is measured in metres.
///
/// Unlike Schwarzschild coordinates, this chart is regular at the future
/// event horizon. Domain: r > 0 and 0 < theta < pi. It is not regular at
/// the physical curvature singularity r = 0 or on the spherical polar axis.
pub fn schwarzschild_ingoing_ef_metric(
    hole: SchwarzschildBlackHole,
    radius_m: f64,
    theta_rad: f64,
) -> Result<MetricComponents4, MetricError> {
    super::validate_positive_finite(radius_m, "radius_m")?;
    validate_theta(theta_rad)?;
    let f = 1.0 - hole.horizon_radius_m() / radius_m;
    if !f.is_finite() {
        return Err(MetricError::ComputationNonFinite(
            "ingoing Eddington-Finkelstein metric coefficient",
        ));
    }
    let r_squared = radius_m * radius_m;
    let sin_squared = theta_rad.sin().powi(2);
    let rg = 0.5 * hole.horizon_radius_m();
    MetricComponents4::new_with_basis_scales(
        [
            [-f, 1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, r_squared, 0.0],
            [0.0, 0.0, 0.0, r_squared * sin_squared],
        ],
        [1.0, 1.0, 1.0 / rg, 1.0 / rg],
    )
}

/// Kerr metric in ingoing Kerr coordinates (v, r, theta, phi).
///
/// This chart is regular at the future event horizon and uses
/// v = ct + integral((r^2+a^2)/Delta) dr and the corresponding ingoing
/// azimuthal coordinate. Domain: r > 0 and 0 < theta < pi. The polar axis
/// remains a coordinate singularity of this chart.
pub fn kerr_ingoing_metric(
    hole: KerrBlackHole,
    radius_m: f64,
    theta_rad: f64,
) -> Result<MetricComponents4, MetricError> {
    super::validate_positive_finite(radius_m, "radius_m")?;
    validate_theta(theta_rad)?;

    let rg = hole.gravitational_radius_m();
    let a = hole.kerr_parameter_m();
    let sin_theta = theta_rad.sin();
    let sin_squared = sin_theta * sin_theta;
    let cos_theta = theta_rad.cos();
    let sigma = radius_m * radius_m + a * a * cos_theta * cos_theta;

    if !sigma.is_finite() || sigma <= 0.0 {
        return Err(MetricError::ComputationNonFinite(
            "ingoing Kerr metric coefficient",
        ));
    }

    let g_vv = -(1.0 - 2.0 * rg * radius_m / sigma);
    let g_v_r = 1.0;
    let g_v_phi = -2.0 * rg * a * radius_m * sin_squared / sigma;
    let g_r_phi = -a * sin_squared;
    let g_theta_theta = sigma;
    let g_phi_phi = sin_squared
        * (radius_m * radius_m + a * a + 2.0 * rg * a * a * radius_m * sin_squared / sigma);

    MetricComponents4::new_with_basis_scales(
        [
            [g_vv, g_v_r, 0.0, g_v_phi],
            [g_v_r, 0.0, 0.0, g_r_phi],
            [0.0, 0.0, g_theta_theta, 0.0],
            [g_v_phi, g_r_phi, 0.0, g_phi_phi],
        ],
        [1.0, 1.0, 1.0 / rg, 1.0 / rg],
    )
}

/// Kerr metric in Boyer-Lindquist coordinates (ct,r,theta,phi).
/// Domain: r > r_+ and 0 < theta < pi. a = chi GM/c^2.
pub fn kerr_exterior_metric(
    hole: KerrBlackHole,
    radius_m: f64,
    theta_rad: f64,
) -> Result<MetricComponents4, MetricError> {
    super::validate_positive_finite(radius_m, "radius_m")?;
    validate_theta(theta_rad)?;
    let rg = hole.gravitational_radius_m();
    let a = hole.kerr_parameter_m();
    let rp = hole.outer_horizon_radius_m();
    if radius_m <= rp {
        return Err(PhysicsDomainError::RadiusAtOrInsideHorizon {
            radius_m,
            horizon_m: rp,
        }
        .into());
    }
    let s = theta_rad.sin();
    let c = theta_rad.cos();
    let s2 = s * s;
    let sigma = radius_m.powi(2) + a.powi(2) * c * c;
    // Factor the horizon polynomial to reduce cancellation near r = r_+:
    // Delta = (r - r_+)(r - r_-).
    let delta =
        (radius_m - hole.outer_horizon_radius_m()) * (radius_m - hole.inner_horizon_radius_m());
    if !sigma.is_finite() || !delta.is_finite() || sigma <= 0.0 || delta <= 0.0 {
        return Err(MetricError::ComputationNonFinite("Kerr metric coefficient"));
    }
    let g_tt = -(1.0 - 2.0 * rg * radius_m / sigma);
    let g_t_phi = -2.0 * rg * a * radius_m * s2 / sigma;
    let g_rr = sigma / delta;
    let g_theta_theta = sigma;
    let g_phi_phi =
        s2 * (radius_m.powi(2) + a.powi(2) + 2.0 * rg * a.powi(2) * radius_m * s2 / sigma);
    MetricComponents4::new_with_basis_scales(
        [
            [g_tt, 0.0, 0.0, g_t_phi],
            [0.0, g_rr, 0.0, 0.0],
            [0.0, 0.0, g_theta_theta, 0.0],
            [g_t_phi, 0.0, 0.0, g_phi_phi],
        ],
        [1.0, 1.0, 1.0 / rg, 1.0 / rg],
    )
}

#[cfg(test)]
mod tests {
    use super::super::{DimensionlessSpin, MassKg};
    use super::*;

    const SOLAR_MASS: f64 = 1.988_47e30;
    fn mass() -> MassKg {
        MassKg::new(SOLAR_MASS).unwrap()
    }
    fn close(a: f64, b: f64, tol: f64) {
        let allowed = if b == 0.0 { tol } else { tol * b.abs() };
        assert!((a - b).abs() <= allowed, "actual={a:e}, expected={b:e}");
    }
    fn residual(g: [[f64; 4]; 4], gi: [[f64; 4]; 4]) -> f64 {
        let mut max: f64 = 0.0;
        for i in 0..4 {
            for j in 0..4 {
                let mut v = 0.0;
                for k in 0..4 {
                    v += g[i][k] * gi[k][j];
                }
                max = max.max((v - if i == j { 1.0 } else { 0.0 }).abs());
            }
        }
        max
    }

    #[test]
    fn schwarzschild_components_match_analytic_values() {
        let h = SchwarzschildBlackHole::new(mass());
        let r = 4.0 * h.gravitational_radius_m();
        let t = core::f64::consts::FRAC_PI_3;
        let g = schwarzschild_exterior_metric(h, r, t).unwrap();
        close(g.covariant()[0][0], -0.5, 1e-14);
        close(g.covariant()[1][1], 2.0, 1e-14);
        close(g.covariant()[2][2], r * r, 1e-14);
        close(g.covariant()[3][3], r * r * 0.75, 1e-14);
    }

    #[test]
    fn schwarzschild_determinant_matches_chart_formula() {
        let h = SchwarzschildBlackHole::new(mass());
        let r = 5.0 * h.gravitational_radius_m();
        let t: f64 = 0.7;
        let g = schwarzschild_exterior_metric(h, r, t).unwrap();
        close(
            g.determinant().unwrap(),
            -r.powi(4) * t.sin().powi(2),
            2e-13,
        );
    }

    #[test]
    fn schwarzschild_inverse_reconstructs_identity() {
        let h = SchwarzschildBlackHole::new(mass());
        let g = schwarzschild_exterior_metric(h, 7.0 * h.gravitational_radius_m(), 1.1).unwrap();
        assert!(residual(*g.covariant(), g.inverse().unwrap()) < 1e-13);
        assert!(g.inverse_identity_residual().unwrap() < 1e-13);
    }

    #[test]
    fn zero_spin_kerr_reduces_to_schwarzschild() {
        let m = mass();
        let s = SchwarzschildBlackHole::new(m);
        let k = KerrBlackHole::new(m, DimensionlessSpin::new(0.0).unwrap());
        let r = 8.0 * s.gravitational_radius_m();
        let t = 0.9;
        let a = schwarzschild_exterior_metric(s, r, t).unwrap();
        let b = kerr_exterior_metric(k, r, t).unwrap();
        for i in 0..4 {
            for j in 0..4 {
                close(a.covariant()[i][j], b.covariant()[i][j], 2e-14);
            }
        }
    }

    #[test]
    fn kerr_determinant_matches_minus_sigma_squared_sine_squared() {
        let h = KerrBlackHole::new(mass(), DimensionlessSpin::new(0.8).unwrap());
        let r = 6.0 * h.gravitational_radius_m();
        let t: f64 = 1.0;
        let a = h.kerr_parameter_m();
        let sigma = r.powi(2) + a.powi(2) * t.cos().powi(2);
        let g = kerr_exterior_metric(h, r, t).unwrap();
        close(
            g.determinant().unwrap(),
            -sigma.powi(2) * t.sin().powi(2),
            3e-13,
        );
    }

    #[test]
    fn near_horizon_kerr_metric_uses_positive_factorized_delta() {
        let h = KerrBlackHole::new(mass(), DimensionlessSpin::new(0.9).unwrap());
        let rp = h.outer_horizon_radius_m();
        let offset = 1.0e-8 * h.gravitational_radius_m();
        let metric = kerr_exterior_metric(h, rp + offset, 1.0).unwrap();
        assert!(metric.covariant()[1][1].is_finite());
        assert!(metric.covariant()[1][1] > 0.0);
        assert!(metric.inverse_identity_residual().unwrap() < 1.0e-7);
    }

    #[test]
    fn kerr_inverse_handles_time_azimuth_coupling() {
        let h = KerrBlackHole::new(mass(), DimensionlessSpin::new(0.9).unwrap());
        let g = kerr_exterior_metric(h, 5.0 * h.gravitational_radius_m(), 0.8).unwrap();
        assert_ne!(g.covariant()[0][3], 0.0);
        assert!(residual(*g.covariant(), g.inverse().unwrap()) < 2e-13);
    }

    #[test]
    fn kerr_metric_is_invertible_at_equatorial_ergosurface() {
        let h = KerrBlackHole::new(mass(), DimensionlessSpin::new(0.9).unwrap());
        let t = core::f64::consts::FRAC_PI_2;
        let r = h.ergosurface_radius_m(t).unwrap();
        let g = kerr_exterior_metric(h, r, t).unwrap();
        close(g.covariant()[0][0], 0.0, 1e-12);
        assert!(g.determinant().unwrap().is_finite());
        assert!(g.inverse_identity_residual().unwrap() < 1e-12);
    }

    #[test]
    fn kerr_inverse_matches_independent_analytic_block_formula_after_basis_scaling() {
        let h = KerrBlackHole::new(mass(), DimensionlessSpin::new(0.9).unwrap());
        let rg = h.gravitational_radius_m();
        let a = h.kerr_parameter_m();
        let rp = h.outer_horizon_radius_m();
        let cases = [
            (
                h.ergosurface_radius_m(core::f64::consts::FRAC_PI_2)
                    .unwrap(),
                core::f64::consts::FRAC_PI_2,
            ),
            (rp + 1.0e-8 * rg, 1.0),
            (rp + 1.0e-6 * rg, 0.4),
        ];

        for (r, theta) in cases {
            let sin_theta = theta.sin();
            let sin_squared = sin_theta * sin_theta;
            let sigma = r * r + a * a * theta.cos().powi(2);
            let delta = (r - h.outer_horizon_radius_m()) * (r - h.inner_horizon_radius_m());

            // Independent inverse in the analytic Kerr t-phi block.
            let mut expected = [[0.0; 4]; 4];
            expected[0][0] =
                -((r * r + a * a).powi(2) - a * a * delta * sin_squared) / (sigma * delta);
            expected[0][3] = -2.0 * rg * a * r / (sigma * delta);
            expected[3][0] = expected[0][3];
            expected[1][1] = delta / sigma;
            expected[2][2] = 1.0 / sigma;
            expected[3][3] = (delta - a * a * sin_squared) / (sigma * delta * sin_squared);

            let metric = kerr_exterior_metric(h, r, theta).unwrap();
            let actual = metric.inverse().unwrap();
            let scales = metric.basis_scales();

            // Compare both inverses after applying the same dimensionless
            // congruence normalization. This avoids a unit-dependent raw
            // component norm while still checking the full inverse tensor.
            let near_horizon = (r - rp) / rg < 1.0e-5;
            let relative_tolerance = if near_horizon { 1.0e-7 } else { 5.0e-12 };
            for i in 0..4 {
                for j in 0..4 {
                    let scale = scales[i] * scales[j];
                    let actual_normalized = actual[i][j] / scale;
                    let expected_normalized = expected[i][j] / scale;
                    let allowed = relative_tolerance * (1.0 + expected_normalized.abs());
                    assert!(
                        (actual_normalized - expected_normalized).abs() <= allowed,
                        "r/r_g={}, theta={}, component=({}, {}), actual={}, expected={}, allowed={}",
                        r / rg,
                        theta,
                        i,
                        j,
                        actual_normalized,
                        expected_normalized,
                        allowed
                    );
                }
            }
            let residual_limit = if near_horizon { 1.0e-7 } else { 1.0e-12 };
            assert!(metric.inverse_identity_residual().unwrap() < residual_limit);
        }
    }

    #[test]
    fn metric_basis_scales_must_be_positive_and_finite() {
        let mut minkowski = [[0.0; 4]; 4];
        minkowski[0][0] = -1.0;
        for i in 1..4 {
            minkowski[i][i] = 1.0;
        }
        assert!(matches!(
            MetricComponents4::new_with_basis_scales(minkowski, [1.0, 1.0, 0.0, 1.0]),
            Err(MetricError::InvalidBasisScale { coordinate: 2, .. })
        ));
        assert!(matches!(
            MetricComponents4::new_with_basis_scales(minkowski, [1.0, f64::INFINITY, 1.0, 1.0]),
            Err(MetricError::ComputationNonFinite("metric basis scale"))
        ));
    }

    #[test]
    fn metric_constructors_reject_horizon_and_axis_domains() {
        let s = SchwarzschildBlackHole::new(mass());
        assert!(matches!(
            schwarzschild_exterior_metric(s, s.horizon_radius_m(), 1.0),
            Err(MetricError::PhysicsDomain(
                PhysicsDomainError::RadiusAtOrInsideHorizon { .. }
            ))
        ));
        assert!(matches!(
            schwarzschild_exterior_metric(s, 4.0 * s.gravitational_radius_m(), 0.0),
            Err(MetricError::PolarAxisCoordinateSingularity)
        ));
        let k = KerrBlackHole::new(mass(), DimensionlessSpin::new(0.5).unwrap());
        assert!(matches!(
            kerr_exterior_metric(k, k.outer_horizon_radius_m(), 1.0),
            Err(MetricError::PhysicsDomain(
                PhysicsDomainError::RadiusAtOrInsideHorizon { .. }
            ))
        ));
    }

    #[test]
    fn generic_constructor_rejects_asymmetric_and_nonfinite_matrices() {
        let mut a = [[0.0; 4]; 4];
        a[0][0] = -1.0;
        a[1][1] = 1.0;
        a[2][2] = 1.0;
        a[3][3] = 1.0;
        a[0][1] = 0.1;
        assert!(matches!(
            MetricComponents4::new(a),
            Err(MetricError::AsymmetricComponents { .. })
        ));
        a[0][1] = 0.0;
        a[0][0] = f64::NAN;
        assert!(matches!(
            MetricComponents4::new(a),
            Err(MetricError::NonFiniteComponent { .. })
        ));
    }

    #[test]
    fn matrix_operations_detect_singularity() {
        let m = MetricComponents4::new([[0.0; 4]; 4]).unwrap();
        assert!(matches!(m.inverse(), Err(MetricError::SingularMatrix)));
        assert!(matches!(m.determinant(), Err(MetricError::SingularMatrix)));
    }
}
