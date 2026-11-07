// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Analytic stationary black-hole metrics in the exterior coordinate charts.
//! Coordinates are x^mu=(c*t, r, theta, phi), with c*t and r in metres,
//! angles in radians, ds^2 in m^2, and signature (-,+,+,+).
//! Schwarzschild and Boyer-Lindquist charts exclude the horizon and polar axis.
//! This is not an evolved spacetime, a connection, or a numerical-relativity solver.

use core::fmt;
use super::{KerrBlackHole, PhysicsDomainError, SchwarzschildBlackHole};

/// Finite symmetric 4x4 covariant metric components in one coordinate basis.
/// Generic construction checks finiteness/symmetry, not Lorentzian signature.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MetricComponents4 {
    covariant: [[f64; 4]; 4],
}

impl MetricComponents4 {
    pub fn new(mut covariant: [[f64; 4]; 4]) -> Result<Self, MetricError> {
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
                let tol = 64.0 * f64::EPSILON * a.abs().max(b.abs()).max(1.0);
                if (a - b).abs() > tol {
                    return Err(MetricError::AsymmetricComponents { row: i, column: j });
                }
                let mean = 0.5 * a + 0.5 * b;
                covariant[i][j] = mean;
                covariant[j][i] = mean;
            }
        }
        Ok(Self { covariant })
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
        if result.is_finite() { Ok(result) } else {
            Err(MetricError::ComputationNonFinite("metric determinant"))
        }
    }

    /// Contravariant matrix via pivoted Gauss-Jordan elimination.
    pub fn inverse(&self) -> Result<[[f64; 4]; 4], MetricError> {
        let mut a = self.covariant;
        let mut inv = [[0.0; 4]; 4];
        for i in 0..4 { inv[i][i] = 1.0; }
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
                if row == col { continue; }
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

    /// max absolute residual of g_{mu a} g^{a nu} - delta_mu^nu.
    pub fn inverse_identity_residual(&self) -> Result<f64, MetricError> {
        let inv = self.inverse()?;
        let mut max: f64 = 0.0;
        for i in 0..4 {
            for j in 0..4 {
                let mut sum = 0.0;
                for k in 0..4 { sum += self.covariant[i][k] * inv[k][j]; }
                let expected = if i == j { 1.0 } else { 0.0 };
                max = max.max((sum - expected).abs());
            }
        }
        if max.is_finite() { Ok(max) } else {
            Err(MetricError::ComputationNonFinite("inverse identity residual"))
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
    ComputationNonFinite(&'static str),
}

impl From<PhysicsDomainError> for MetricError {
    fn from(value: PhysicsDomainError) -> Self { Self::PhysicsDomain(value) }
}

impl fmt::Display for MetricError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PhysicsDomain(e) => write!(f, "{e}"),
            Self::PolarAxisCoordinateSingularity => write!(f, "chart excludes theta = 0 and pi"),
            Self::NonFiniteComponent { row, column } =>
                write!(f, "metric component g[{row}][{column}] is not finite"),
            Self::AsymmetricComponents { row, column } =>
                write!(f, "metric is not symmetric at ({row}, {column})"),
            Self::SingularMatrix => write!(f, "metric component matrix is singular"),
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
            radius_m, horizon_m: rs,
        }.into());
    }
    let f = 1.0 - rs / radius_m;
    if !f.is_finite() || f <= 0.0 {
        return Err(MetricError::ComputationNonFinite("Schwarzschild metric coefficient"));
    }
    let r2 = radius_m * radius_m;
    let s2 = theta_rad.sin().powi(2);
    MetricComponents4::new([
        [-f, 0.0, 0.0, 0.0],
        [0.0, 1.0 / f, 0.0, 0.0],
        [0.0, 0.0, r2, 0.0],
        [0.0, 0.0, 0.0, r2 * s2],
    ])
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
            radius_m, horizon_m: rp,
        }.into());
    }
    let s = theta_rad.sin();
    let c = theta_rad.cos();
    let s2 = s * s;
    let sigma = radius_m.powi(2) + a.powi(2) * c * c;
    // Factor the horizon polynomial to avoid cancellation near r = r_+:
    // Delta = (r - r_+)(r - r_-).
    let delta = (radius_m - hole.outer_horizon_radius_m())
        * (radius_m - hole.inner_horizon_radius_m());
    if !sigma.is_finite() || !delta.is_finite() || sigma <= 0.0 || delta <= 0.0 {
        return Err(MetricError::ComputationNonFinite("Kerr metric coefficient"));
    }
    let g_tt = -(1.0 - 2.0 * rg * radius_m / sigma);
    let g_t_phi = -2.0 * rg * a * radius_m * s2 / sigma;
    let g_rr = sigma / delta;
    let g_theta_theta = sigma;
    let g_phi_phi = s2 * (
        radius_m.powi(2) + a.powi(2)
        + 2.0 * rg * a.powi(2) * radius_m * s2 / sigma
    );
    MetricComponents4::new([
        [g_tt, 0.0, 0.0, g_t_phi],
        [0.0, g_rr, 0.0, 0.0],
        [0.0, 0.0, g_theta_theta, 0.0],
        [g_t_phi, 0.0, 0.0, g_phi_phi],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::{DimensionlessSpin, MassKg};

    const SOLAR_MASS: f64 = 1.988_47e30;
    fn mass() -> MassKg { MassKg::new(SOLAR_MASS).unwrap() }
    fn close(a: f64, b: f64, tol: f64) {
        let allowed = if b == 0.0 { tol } else { tol * b.abs() };
        assert!((a-b).abs() <= allowed, "actual={a:e}, expected={b:e}");
    }
    fn residual(g: [[f64; 4]; 4], gi: [[f64; 4]; 4]) -> f64 {
        let mut max: f64 = 0.0;
        for i in 0..4 { for j in 0..4 {
            let mut v = 0.0;
            for k in 0..4 { v += g[i][k]*gi[k][j]; }
            max = max.max((v-if i==j {1.0} else {0.0}).abs());
        }}
        max
    }

    #[test]
    fn schwarzschild_components_match_analytic_values() {
        let h = SchwarzschildBlackHole::new(mass());
        let r = 4.0*h.gravitational_radius_m();
        let t = core::f64::consts::FRAC_PI_3;
        let g = schwarzschild_exterior_metric(h,r,t).unwrap();
        close(g.covariant()[0][0],-0.5,1e-14);
        close(g.covariant()[1][1],2.0,1e-14);
        close(g.covariant()[2][2],r*r,1e-14);
        close(g.covariant()[3][3],r*r*0.75,1e-14);
    }

    #[test]
    fn schwarzschild_determinant_matches_chart_formula() {
        let h = SchwarzschildBlackHole::new(mass());
        let r = 5.0*h.gravitational_radius_m();
        let t: f64 = 0.7;
        let g = schwarzschild_exterior_metric(h,r,t).unwrap();
        close(g.determinant().unwrap(),-r.powi(4)*t.sin().powi(2),2e-13);
    }

    #[test]
    fn schwarzschild_inverse_reconstructs_identity() {
        let h = SchwarzschildBlackHole::new(mass());
        let g = schwarzschild_exterior_metric(h,7.0*h.gravitational_radius_m(),1.1).unwrap();
        assert!(residual(*g.covariant(),g.inverse().unwrap()) < 1e-13);
        assert!(g.inverse_identity_residual().unwrap() < 1e-13);
    }

    #[test]
    fn zero_spin_kerr_reduces_to_schwarzschild() {
        let m = mass();
        let s = SchwarzschildBlackHole::new(m);
        let k = KerrBlackHole::new(m,DimensionlessSpin::new(0.0).unwrap());
        let r = 8.0*s.gravitational_radius_m();
        let t = 0.9;
        let a = schwarzschild_exterior_metric(s,r,t).unwrap();
        let b = kerr_exterior_metric(k,r,t).unwrap();
        for i in 0..4 { for j in 0..4 { close(a.covariant()[i][j],b.covariant()[i][j],2e-14); }}
    }

    #[test]
    fn kerr_determinant_matches_minus_sigma_squared_sine_squared() {
        let h = KerrBlackHole::new(mass(),DimensionlessSpin::new(0.8).unwrap());
        let r = 6.0*h.gravitational_radius_m();
        let t: f64 = 1.0;
        let a = h.kerr_parameter_m();
        let sigma = r.powi(2)+a.powi(2)*t.cos().powi(2);
        let g = kerr_exterior_metric(h,r,t).unwrap();
        close(g.determinant().unwrap(),-sigma.powi(2)*t.sin().powi(2),3e-13);
    }

    #[test]
    fn near_horizon_kerr_metric_uses_positive_factorized_delta() {
        let h = KerrBlackHole::new(mass(), DimensionlessSpin::new(0.9).unwrap());
        let rp = h.outer_horizon_radius_m();
        let offset = 1.0e-8 * h.gravitational_radius_m();
        let g = kerr_exterior_metric(h, rp + offset, 1.0).unwrap();
        assert!(g.covariant()[1][1].is_finite());
        assert!(g.covariant()[1][1] > 0.0);
        assert!(g.inverse_identity_residual().unwrap() < 1.0e-7);
    }

    #[test]
    fn kerr_inverse_handles_time_azimuth_coupling() {
        let h = KerrBlackHole::new(mass(),DimensionlessSpin::new(0.9).unwrap());
        let g = kerr_exterior_metric(h,5.0*h.gravitational_radius_m(),0.8).unwrap();
        assert_ne!(g.covariant()[0][3],0.0);
        assert!(residual(*g.covariant(),g.inverse().unwrap()) < 2e-13);
    }

    #[test]
    fn kerr_metric_is_invertible_at_equatorial_ergosurface() {
        let h = KerrBlackHole::new(mass(),DimensionlessSpin::new(0.9).unwrap());
        let t = core::f64::consts::FRAC_PI_2;
        let r = h.ergosurface_radius_m(t).unwrap();
        let g = kerr_exterior_metric(h,r,t).unwrap();
        close(g.covariant()[0][0],0.0,1e-12);
        assert!(g.determinant().unwrap().is_finite());
        assert!(g.inverse_identity_residual().unwrap() < 1e-12);
    }

    #[test]
    fn metric_constructors_reject_horizon_and_axis_domains() {
        let s = SchwarzschildBlackHole::new(mass());
        assert!(matches!(
            schwarzschild_exterior_metric(s,s.horizon_radius_m(),1.0),
            Err(MetricError::PhysicsDomain(PhysicsDomainError::RadiusAtOrInsideHorizon{..}))
        ));
        assert!(matches!(
            schwarzschild_exterior_metric(s,4.0*s.gravitational_radius_m(),0.0),
            Err(MetricError::PolarAxisCoordinateSingularity)
        ));
        let k = KerrBlackHole::new(mass(),DimensionlessSpin::new(0.5).unwrap());
        assert!(matches!(
            kerr_exterior_metric(k,k.outer_horizon_radius_m(),1.0),
            Err(MetricError::PhysicsDomain(PhysicsDomainError::RadiusAtOrInsideHorizon{..}))
        ));
    }

    #[test]
    fn generic_constructor_rejects_asymmetric_and_nonfinite_matrices() {
        let mut a = [[0.0;4];4];
        a[0][0] = -1.0; a[1][1] = 1.0; a[2][2] = 1.0; a[3][3] = 1.0;
        a[0][1] = 0.1;
        assert!(matches!(MetricComponents4::new(a),Err(MetricError::AsymmetricComponents{..})));
        a[0][1] = 0.0; a[0][0] = f64::NAN;
        assert!(matches!(MetricComponents4::new(a),Err(MetricError::NonFiniteComponent{..})));
    }

    #[test]
    fn matrix_operations_detect_singularity() {
        let m = MetricComponents4::new([[0.0;4];4]).unwrap();
        assert!(matches!(m.inverse(),Err(MetricError::SingularMatrix)));
        assert!(matches!(m.determinant(),Err(MetricError::SingularMatrix)));
    }
}
