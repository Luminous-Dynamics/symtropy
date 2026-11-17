//! Research-only analytic reference kernel for the static
//! Mannheim–Kazanas line element used in pure Weyl conformal gravity.
//!
//! This module is deliberately dependency-free so it can be compiled directly:
//!   rustc --edition=2024 --test mk_static.rs -o /tmp/mk_static_tests
//!   /tmp/mk_static_tests
//!
//! It is NOT the production Lorentzian solver, a general four-dimensional
//! Bach-tensor implementation, or a complete scalar-conformal matter model.
//! It covers the static spherical metric, selected null observables, a
//! caller-bracketed simple-horizon root, and the published symmetry-reduced
//! Bach scalar residual for the four-term static ansatz.
//!
//! Convention:
//! ds^2 = -B(r) dt^2 + dr^2/B(r) + r^2 dOmega^2
//! B(r) = 1 - 3 beta gamma - beta(2 - 3 beta gamma)/r
//!        + gamma r - kappa r^2
//! Signature (-,+,+,+); geometrized units; beta has length units.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MkParameters {
    pub beta: f64,
    pub gamma: f64,
    pub kappa: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DomainError {
    NonFiniteParameter,
    NonPositiveRadius,
    SingularStaticChart,
    NoRealPhotonSphere,
    NoPositivePhotonSphere,
    InvalidObserverDomain,
    UnphysicalShadowAngle,
    OutsideStaticRegion,
    InvalidRootBracket,
    RootNotBracketed,
    RootDidNotConverge,
}

fn finite(x: f64) -> bool {
    x.is_finite()
}

pub fn lapse(p: MkParameters, r: f64) -> Result<f64, DomainError> {
    if !finite(p.beta) || !finite(p.gamma) || !finite(p.kappa) {
        return Err(DomainError::NonFiniteParameter);
    }
    if !finite(r) || r <= 0.0 {
        return Err(DomainError::NonPositiveRadius);
    }
    let b = 1.0 - 3.0 * p.beta * p.gamma
        - p.beta * (2.0 - 3.0 * p.beta * p.gamma) / r
        + p.gamma * r
        - p.kappa * r * r;
    if finite(b) { Ok(b) } else { Err(DomainError::NonFiniteParameter) }
}

/// Return all distinct positive circular-null-orbit radii.
///
/// For this metric, r B'(r) - 2 B(r) = 0. The kappa term cancels
/// identically, leaving
/// -gamma r^2 + (-2 + 6 beta gamma) r
///     + 3 beta (2 - 3 beta gamma) = 0.
///
/// The result is a coordinate radius, not by itself an observable.
pub fn photon_sphere_radii(p: MkParameters) -> Result<Vec<f64>, DomainError> {
    if !finite(p.beta) || !finite(p.gamma) || !finite(p.kappa) {
        return Err(DomainError::NonFiniteParameter);
    }
    let a = -p.gamma;
    let b = -2.0 + 6.0 * p.beta * p.gamma;
    let c = 3.0 * p.beta * (2.0 - 3.0 * p.beta * p.gamma);
    let scale = a.abs().max(b.abs()).max(c.abs()).max(1.0);
    let mut roots = Vec::new();

    if a.abs() <= 32.0 * f64::EPSILON * scale {
        if b.abs() <= 32.0 * f64::EPSILON * scale {
            return Err(DomainError::NoPositivePhotonSphere);
        }
        let x = -c / b;
        if finite(x) && x > 0.0 { roots.push(x); }
    } else {
        let disc = b * b - 4.0 * a * c;
        let disc_tol = 64.0 * f64::EPSILON * (b * b).max((4.0 * a * c).abs()).max(1.0);
        if disc < -disc_tol { return Err(DomainError::NoRealPhotonSphere); }
        let sqrt_disc = disc.max(0.0).sqrt();
        // Stable quadratic formula avoids cancellation in one of the roots.
        // copysign convention: choose +sqrt when b == 0, avoiding q == 0
        // for a valid pair of symmetric nonzero roots.
        let signed_sqrt = if b >= 0.0 { sqrt_disc } else { -sqrt_disc };
        let q = -0.5 * (b + signed_sqrt);
        if q == 0.0 {
            let x = -b / (2.0 * a);
            if finite(x) && x > 0.0 { roots.push(x); }
        } else {
            for x in [q / a, c / q] {
                if finite(x) && x > 0.0 { roots.push(x); }
            }
        }
    }

    roots.sort_by(f64::total_cmp);
    roots.dedup_by(|x, y| (*x - *y).abs() <= 64.0 * f64::EPSILON * x.abs().max(y.abs()).max(1.0));
    if roots.is_empty() { Err(DomainError::NoPositivePhotonSphere) } else { Ok(roots) }
}

/// Critical impact parameter b_c = r_ph / sqrt(B(r_ph)) for a
/// circular null orbit in a static region where B(r_ph) > 0.
pub fn critical_impact_parameter(p: MkParameters, r_ph: f64) -> Result<f64, DomainError> {
    let b = lapse(p, r_ph)?;
    if b <= 0.0 { return Err(DomainError::SingularStaticChart); }
    Ok(r_ph / b.sqrt())
}

/// Local angular radius for a static observer at finite areal radius r_obs:
/// sin(alpha) = b_c sqrt(B(r_obs)) / r_obs.
///
/// This is not an observer-at-infinity formula. It requires a static observer
/// in a static region and a real local angle.
pub fn finite_observer_shadow_angle(
    p: MkParameters,
    b_critical: f64,
    r_obs: f64,
) -> Result<f64, DomainError> {
    if !finite(b_critical) || b_critical < 0.0 || !finite(r_obs) || r_obs <= 0.0 {
        return Err(DomainError::InvalidObserverDomain);
    }
    let b_obs = lapse(p, r_obs)?;
    if b_obs <= 0.0 { return Err(DomainError::InvalidObserverDomain); }
    let sin_alpha = b_critical * b_obs.sqrt() / r_obs;
    if !finite(sin_alpha) || !(0.0..=1.0).contains(&sin_alpha) {
        return Err(DomainError::UnphysicalShadowAngle);
    }
    Ok(sin_alpha.asin())
}

/// Gravitational frequency ratio nu_obs / nu_emit for static emitter and
/// observer in the same static chart: sqrt(B_emit / B_obs).
pub fn static_redshift_ratio(
    p: MkParameters,
    r_emit: f64,
    r_obs: f64,
) -> Result<f64, DomainError> {
    let b_emit = lapse(p, r_emit)?;
    let b_obs = lapse(p, r_obs)?;
    if b_emit <= 0.0 || b_obs <= 0.0 {
        return Err(DomainError::InvalidObserverDomain);
    }
    Ok((b_emit / b_obs).sqrt())
}

/// Determinant of the diagonal metric in (t,r,theta,phi) coordinates:
/// det(g) = -r^4 sin(theta)^2. The static chart itself excludes B(r)=0.
pub fn metric_determinant(r: f64, theta: f64) -> Result<f64, DomainError> {
    if !finite(r) || r <= 0.0 || !finite(theta) {
        return Err(DomainError::InvalidObserverDomain);
    }
    Ok(-r.powi(4) * theta.sin().powi(2))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCHWARZSCHILD: MkParameters = MkParameters { beta: 1.0, gamma: 0.0, kappa: 0.0 };

    #[test]
    fn schwarzschild_limit_and_photon_sphere_are_recovered() {
        assert!((lapse(SCHWARZSCHILD, 10.0).unwrap() - 0.8).abs() < 1e-14);
        let roots = photon_sphere_radii(SCHWARZSCHILD).unwrap();
        assert_eq!(roots.len(), 1);
        assert!((roots[0] - 3.0).abs() < 1e-13);
        let b = critical_impact_parameter(SCHWARZSCHILD, roots[0]).unwrap();
        assert!((b - 3.0 * 3.0_f64.sqrt()).abs() < 1e-13);
    }

    #[test]
    fn bracketed_horizon_root_recovers_schwarzschild_horizon() {
        let root = bracketed_horizon_root(SCHWARZSCHILD, 1.0, 3.0, 1e-13, 128).unwrap();
        assert!((root - 2.0).abs() < 1e-12);
    }

    #[test]
    fn horizon_root_requires_a_valid_sign_changing_bracket() {
        assert_eq!(
            bracketed_horizon_root(SCHWARZSCHILD, 3.0, 4.0, 1e-12, 64),
            Err(DomainError::RootNotBracketed)
        );
        assert_eq!(
            bracketed_horizon_root(SCHWARZSCHILD, 0.0, 3.0, 1e-12, 64),
            Err(DomainError::InvalidRootBracket)
        );
    }

    #[test]
    fn simple_horizon_bracketer_does_not_misclassify_extremal_double_root() {
        // Schwarzschild-de Sitter Nariai limit: B(r)=1-2/r-r²/27
        // has a double root at r=3 and no sign change across it.
        let nariai = MkParameters { beta: 1.0, gamma: 0.0, kappa: 1.0 / 27.0 };
        assert_eq!(
            bracketed_horizon_root(nariai, 2.5, 3.5, 1e-12, 128),
            Err(DomainError::RootNotBracketed)
        );
    }

    #[test]
    fn quadratic_solver_handles_zero_linear_coefficient() {
        // beta*gamma = 1/3 makes the linear coefficient exactly zero;
        // the positive root is r=3 and the negative root is discarded.
        let p = MkParameters { beta: 1.0, gamma: 1.0 / 3.0, kappa: 0.0 };
        let roots = photon_sphere_radii(p).unwrap();
        assert_eq!(roots.len(), 1);
        assert!((roots[0] - 3.0).abs() < 1e-12);
    }

    #[test]
    fn bach_residuals_vanish_for_mannheim_kazanas_family() {
        let p = MkParameters { beta: 1.0, gamma: 0.01, kappa: 1e-5 };
        let coefficients = StaticSphericalCoefficients::from_mk(p);
        for r in [2.0, 3.0, 8.0, 20.0] {
            let residual = coefficients.bach_scalar_residuals(r).unwrap();
            assert!(residual.max_abs() < 2e-13, "r={r}, residual={residual:?}");
        }
    }

    #[test]
    fn bach_residual_negative_control_detects_off_shell_coefficients() {
        // Perturb c0 away from the vacuum constraint. The residual must not
        // be silently projected back onto the Mannheim–Kazanas family.
        let off_shell = StaticSphericalCoefficients {
            inverse_r: -2.0,
            constant: 1.1,
            linear: 0.0,
            quadratic: 0.0,
        };
        let residual = off_shell.bach_scalar_residuals(3.0).unwrap();
        let expected = 4.0 * (1.1_f64.powi(2) - 1.0) / (3.0 * 3.0_f64.powi(4));
        assert!(residual.b1_plus_b2.abs() < 1e-14);
        assert!((residual.b1_minus_b2 - expected).abs() < 1e-14);
        assert!(residual.max_abs() > 1e-4);
    }

    #[test]
    fn bach_off_shell_residual_has_expected_inverse_fourth_power_scaling() {
        let off_shell = StaticSphericalCoefficients {
            inverse_r: -2.0,
            constant: 1.1,
            linear: 0.0,
            quadratic: 0.0,
        };
        let r1 = off_shell.bach_scalar_residuals(3.0).unwrap().b1_minus_b2;
        let r2 = off_shell.bach_scalar_residuals(6.0).unwrap().b1_minus_b2;
        assert!((r1 / r2 - 16.0).abs() < 1e-12);
    }

    #[test]
    fn bach_residual_requires_static_chart_domain() {
        let coefficients = StaticSphericalCoefficients {
            inverse_r: -2.0, constant: 1.0, linear: 0.0, quadratic: 0.0,
        };
        assert_eq!(coefficients.bach_scalar_residuals(1.0), Err(DomainError::OutsideStaticRegion));
    }

    #[test]
    fn kappa_cancels_from_circular_null_orbit_condition() {
        let p0 = MkParameters { beta: 1.0, gamma: 0.01, kappa: 0.0 };
        let p1 = MkParameters { kappa: 1e-4, ..p0 };
        let r0 = photon_sphere_radii(p0).unwrap()[0];
        let r1 = photon_sphere_radii(p1).unwrap()[0];
        assert!((r0 - 3.0).abs() < 1e-12);
        assert!((r0 - r1).abs() < 1e-12);
    }

    #[test]
    fn finite_observer_angle_is_not_the_infinity_formula() {
        let p = SCHWARZSCHILD;
        let b = 3.0 * 3.0_f64.sqrt();
        let angle = finite_observer_shadow_angle(p, b, 10.0).unwrap();
        let expected = (b * 0.8_f64.sqrt() / 10.0).asin();
        assert!((angle - expected).abs() < 1e-14);
    }

    #[test]
    fn static_redshift_and_metric_determinant_match_analytic_controls() {
        let ratio = static_redshift_ratio(SCHWARZSCHILD, 4.0, 8.0).unwrap();
        assert!((ratio - (2.0_f64 / 3.0).sqrt()).abs() < 1e-14);
        let det = metric_determinant(2.0, std::f64::consts::FRAC_PI_2).unwrap();
        assert!((det + 16.0).abs() < 1e-13);
    }

    #[test]
    fn invalid_or_non_static_observer_domains_fail_closed() {
        assert_eq!(lapse(SCHWARZSCHILD, 0.0), Err(DomainError::NonPositiveRadius));
        let inside = finite_observer_shadow_angle(SCHWARZSCHILD, 5.0, 1.0);
        assert_eq!(inside, Err(DomainError::InvalidObserverDomain));
    }
}
