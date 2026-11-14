// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Turning-point-regular equatorial prograde Kerr strong-deflection quadrature.
//!
//! The near-extremal coordinate is y=(r/M-1)/kappa, where
//! kappa=sqrt(1-a^2). The radial potential is evaluated in the scaled form
//! R/kappa^2 to avoid subtracting O(1) terms near the horizon. The outer
//! turning point is bracketed at the minimum of the associated depressed
//! cubic, then the square-root endpoint singularity is removed with
//! y=y_turn+S*x^2/(1-x^2), x in [0,1].
//!
//! The returned leading-uniform phase is a comparison reference, not an
//! assertion that the asymptotic expression equals the finite-spin integral.

use core::fmt;

use crate::kerr_geodesic::{
    KerrEquatorialProgradePhotonOrbit, KerrGeodesicError,
};

/// Numerical controls for the near-extremal strong-deflection quadrature.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrStrongDeflectionOptions {
    pub relative_tolerance: f64,
    pub absolute_tolerance: f64,
    pub maximum_root_iterations: usize,
    pub maximum_quadrature_depth: usize,
    pub maximum_quadrature_evaluations: usize,
}

impl Default for KerrStrongDeflectionOptions {
    fn default() -> Self {
        Self {
            relative_tolerance: 1.0e-10,
            absolute_tolerance: 1.0e-10,
            maximum_root_iterations: 128,
            maximum_quadrature_depth: 30,
            maximum_quadrature_evaluations: 200_000,
        }
    }
}

impl KerrStrongDeflectionOptions {
    pub fn validate(self) -> Result<Self, KerrStrongDeflectionError> {
        if !self.relative_tolerance.is_finite()
            || self.relative_tolerance <= 0.0
            || !self.absolute_tolerance.is_finite()
            || self.absolute_tolerance <= 0.0
            || self.maximum_root_iterations == 0
            || self.maximum_quadrature_depth == 0
            || self.maximum_quadrature_evaluations < 5
        {
            return Err(KerrStrongDeflectionError::InvalidOptions);
        }
        Ok(self)
    }
}

/// Failure while evaluating the finite-spin strong-deflection integral.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KerrStrongDeflectionError {
    Geodesic(KerrGeodesicError),
    InvalidOptions,
    NoScatteringTurningPoint,
    TurningPointNotBracketed,
    TurningPointDidNotConverge,
    QuadratureDidNotConverge {
        estimated_error: f64,
        requested_tolerance: f64,
        evaluations: usize,
    },
}

impl From<KerrGeodesicError> for KerrStrongDeflectionError {
    fn from(value: KerrGeodesicError) -> Self {
        Self::Geodesic(value)
    }
}

impl fmt::Display for KerrStrongDeflectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Geodesic(error) => write!(f, "{error}"),
            Self::InvalidOptions => write!(f, "invalid near-extremal strong-deflection quadrature options"),
            Self::NoScatteringTurningPoint => {
                write!(f, "no exterior scattering turning point was found above the radial-potential minimum")
            }
            Self::TurningPointNotBracketed => {
                write!(f, "outer Kerr scattering turning point could not be bracketed")
            }
            Self::TurningPointDidNotConverge => {
                write!(f, "outer Kerr scattering turning point did not converge within the iteration limit")
            }
            Self::QuadratureDidNotConverge {
                estimated_error,
                requested_tolerance,
                evaluations,
            } => write!(
                f,
                "strong-deflection quadrature did not converge: estimated error {estimated_error}, requested {requested_tolerance}, evaluations {evaluations}"
            ),
        }
    }
}

impl std::error::Error for KerrStrongDeflectionError {}

/// A finite-spin equatorial prograde scattering result in M=1 units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KerrStrongDeflectionResult {
    pub spin: f64,
    pub extremality_parameter: f64,
    pub scaled_offset_chi: f64,
    pub impact_parameter_over_m: f64,
    pub critical_impact_parameter_over_m: f64,
    pub photon_orbit_radius_over_m: f64,
    pub outer_turning_radius_over_m: f64,
    /// Absolute residual of R/kappa^2 at the computed outer turning point.
    pub scaled_radial_potential_residual: f64,
    /// d(R/kappa^2)/dy at the outer turning point; positive on the scattering branch.
    pub scaled_radial_potential_derivative_at_turn: f64,
    /// Total Boyer-Lindquist azimuth accumulated from infinity to the turn and back.
    pub azimuthal_change_rad: f64,
    /// Scattering deflection, azimuthal_change - pi.
    pub deflection_angle_rad: f64,
    /// Leading uniform asymptotic phase ln(1+2*sqrt(3)/chi)/kappa.
    pub leading_uniform_phase_rad: f64,
    /// kappa*deflection - ln(1+2*sqrt(3)/chi); should tend to zero as kappa decreases.
    pub scaled_leading_uniform_residual: f64,
    /// Conservative adaptive-Simpson error estimate for the full azimuthal change.
    pub quadrature_error_estimate: f64,
    pub quadrature_evaluations: usize,
    pub root_iterations: usize,
}

/// Integrate the exact equatorial prograde null-geodesic deflection at
/// b=b_c+kappa*chi for a subextremal spin in [0,1).
///
/// The radial potential and azimuthal Mino derivative are evaluated using a
/// near-horizon scaled coordinate. The exact extremal endpoint is rejected:
/// it requires the separately derived extremal normal form, not a division
/// by kappa=0.
pub fn integrate_equatorial_prograde_strong_deflection(
    spin: f64,
    scaled_offset_chi: f64,
    options: KerrStrongDeflectionOptions,
) -> Result<KerrStrongDeflectionResult, KerrStrongDeflectionError> {
    let options = options.validate()?;
    let orbit = KerrEquatorialProgradePhotonOrbit::from_spin(spin)?;
    let kappa = orbit.extremality_parameter;
    if !scaled_offset_chi.is_finite()
        || scaled_offset_chi <= 0.0
        || kappa <= 0.0
    {
        return Err(KerrGeodesicError::InvalidCriticalOffset(scaled_offset_chi).into());
    }

    let a = orbit.spin;
    let y_ph = (orbit.radius_over_m - 1.0) / kappa;
    let delta_ph_scaled = (y_ph - 1.0) * (y_ph + 1.0);
    if !y_ph.is_finite() || y_ph <= 1.0 || !delta_ph_scaled.is_finite() || delta_ph_scaled <= 0.0 {
        return Err(KerrGeodesicError::NonFinite("near_extremal_photon_orbit_scaled").into());
    }

    let impact_parameter = orbit.impact_parameter_for_scaled_offset(scaled_offset_chi)?;
    let b_minus_a = impact_parameter - a;
    let b_squared_minus_a_squared = impact_parameter * impact_parameter - a * a;
    if !b_minus_a.is_finite() || !b_squared_minus_a_squared.is_finite() || b_squared_minus_a_squared <= 0.0 {
        return Err(KerrGeodesicError::NonFinite("near_extremal_impact_parameter").into());
    }

    // The cubic factor of the equatorial radial potential is
    // R(r)=r*[r^3-(b^2-a^2)r+2(b-a)^2]. Its positive local minimum is at
    // r_min=sqrt((b^2-a^2)/3). The expression below avoids subtracting two
    // nearly equal radii when converting to y=(r-1)/kappa.
    let radial_cubic_minimum_radius =
        (b_squared_minus_a_squared / 3.0).sqrt();
    let y_minimum = (b_squared_minus_a_squared - 3.0)
        / (3.0 * kappa * (radial_cubic_minimum_radius + 1.0));
    if !radial_cubic_minimum_radius.is_finite()
        || radial_cubic_minimum_radius <= 1.0
        || !y_minimum.is_finite()
        || y_minimum <= 1.0
    {
        return Err(KerrStrongDeflectionError::NoScatteringTurningPoint);
    }

    // P(r,b)/kappa, evaluated relative to the exact critical orbit so that
    // near-horizon O(kappa) quantities are not formed by subtracting O(1)s.
    let p_scaled_base =
        delta_ph_scaled.sqrt() * (orbit.critical_impact_parameter_over_m - a)
            - a * scaled_offset_chi;
    let radial_potential_scaled = |y: f64| -> f64 {
        let p_scaled = p_scaled_base
            + (y - y_ph) * (2.0 + kappa * (y + y_ph));
        let horizon_factor = (y - 1.0) * (y + 1.0);
        let p_squared = p_scaled * p_scaled;
        let angular_squared = b_minus_a * b_minus_a;
        let angular_term = horizon_factor * angular_squared;
        let p_roundoff = p_scaled.mul_add(p_scaled, -p_squared);
        let angular_roundoff = horizon_factor.mul_add(angular_squared, -angular_term);
        let difference = p_squared - angular_term;
        let virtual_angular = p_squared - difference;
        let virtual_p = difference + virtual_angular;
        let subtraction_error = (p_squared - virtual_p) + (virtual_angular - angular_term);
        difference + (subtraction_error + p_roundoff - angular_roundoff)
    };
    let radial_potential_scaled_derivative = |y: f64| -> f64 {
        let r = 1.0 + kappa * y;
        let p_scaled =
            p_scaled_base + (y - y_ph) * (2.0 + kappa * (y + y_ph));
        4.0 * r * p_scaled - 2.0 * y * b_minus_a * b_minus_a
    };

    let minimum_value = radial_potential_scaled(y_minimum);
    if !minimum_value.is_finite() {
        return Err(KerrGeodesicError::NonFinite("near_extremal_scaled_radial_potential").into());
    }
    if minimum_value >= 0.0 {
        return Err(KerrStrongDeflectionError::NoScatteringTurningPoint);
    }

    let mut lower = y_minimum;
    let mut upper_increment = (0.1 * y_minimum.abs()).max(0.1);
    let mut upper = lower + upper_increment;
    let mut upper_value = radial_potential_scaled(upper);
    let mut bracketed = false;
    for _ in 0..options.maximum_root_iterations {
        if !upper_value.is_finite() {
            return Err(KerrGeodesicError::NonFinite("near_extremal_turning_point_bracket").into());
        }
        if upper_value > 0.0 {
            bracketed = true;
            break;
        }
        upper_increment *= 2.0;
        upper = lower + upper_increment;
        upper_value = radial_potential_scaled(upper);
    }
    if !bracketed {
        return Err(KerrStrongDeflectionError::TurningPointNotBracketed);
    }

    let root_tolerance = options.relative_tolerance.max(8.0 * f64::EPSILON);
    let mut root_iterations = 0;
    let mut converged = false;
    for iteration in 0..options.maximum_root_iterations {
        root_iterations = iteration + 1;
        let midpoint = lower + 0.5 * (upper - lower);
        let midpoint_value = radial_potential_scaled(midpoint);
        if !midpoint_value.is_finite() {
            return Err(KerrGeodesicError::NonFinite("near_extremal_turning_point_root").into());
        }
        if midpoint_value > 0.0 {
            upper = midpoint;
        } else {
            lower = midpoint;
        }
        let scale = midpoint.abs().max(1.0);
        if upper - lower <= root_tolerance * scale {
            converged = true;
            break;
        }
    }
    if !converged {
        return Err(KerrStrongDeflectionError::TurningPointDidNotConverge);
    }
    let y_turn = lower + 0.5 * (upper - lower);
    let scaled_radial_potential_residual = radial_potential_scaled(y_turn).abs();
    let scaled_radial_potential_derivative_at_turn =
        radial_potential_scaled_derivative(y_turn);
    if !scaled_radial_potential_residual.is_finite()
        || !scaled_radial_potential_derivative_at_turn.is_finite()
        || scaled_radial_potential_derivative_at_turn <= 0.0
    {
        return Err(KerrGeodesicError::NonFinite("near_extremal_turning_point_residual").into());
    }

    // Compactify y in [y_turn,+infinity). The x=0 and x=1 endpoint limits are
    // evaluated analytically, so the transformed integrand is continuous.
    let compactification_scale = y_turn.max(1.0);
    let phi_numerator = |y: f64| -> f64 {
        let p_scaled =
            p_scaled_base + (y - y_ph) * (2.0 + kappa * (y + y_ph));
        b_minus_a + a * p_scaled / (kappa * (y - 1.0) * (y + 1.0))
    };
    let phi_at_turn = phi_numerator(y_turn);
    let left_endpoint = 2.0
        * (compactification_scale / scaled_radial_potential_derivative_at_turn).sqrt()
        * phi_at_turn;
    let right_endpoint = 2.0 * impact_parameter / (kappa * compactification_scale);

    let mut evaluations = 0_usize;
    let mut integrand = |x: f64| -> Result<f64, KerrStrongDeflectionError> {
        evaluations = evaluations.saturating_add(1);
        let value = if x <= 0.0 {
            left_endpoint
        } else if x >= 1.0 {
            right_endpoint
        } else {
            let denominator = 1.0 - x * x;
            let y = y_turn + compactification_scale * x * x / denominator;
            let dy_dx = 2.0 * compactification_scale * x / (denominator * denominator);
            let radial = radial_potential_scaled(y);
            if !y.is_finite() || !dy_dx.is_finite() || !radial.is_finite() || radial <= 0.0 {
                return Err(KerrGeodesicError::NonFinite("near_extremal_deflection_integrand").into());
            }
            phi_numerator(y) / radial.sqrt() * dy_dx
        };
        if value.is_finite() {
            Ok(value)
        } else {
            Err(KerrGeodesicError::NonFinite("near_extremal_deflection_integrand").into())
        }
    };

    let fa = integrand(0.0)?;
    let fm = integrand(0.5)?;
    let fb = integrand(1.0)?;
    let whole = simpson(0.0, 1.0, fa, fm, fb);
    let requested_tolerance = options.absolute_tolerance.max(options.relative_tolerance * whole.abs());
    let (half_orbit_azimuth, estimated_half_orbit_error) = adaptive_simpson(
        &mut integrand,
        0.0,
        1.0,
        fa,
        fm,
        fb,
        whole,
        requested_tolerance,
        options.maximum_quadrature_depth,
        options.maximum_quadrature_evaluations,
        &mut evaluations,
    )?;
    let azimuthal_change_rad = 2.0 * half_orbit_azimuth;
    let deflection_angle_rad = azimuthal_change_rad - core::f64::consts::PI;
    let leading_uniform_phase_rad = orbit.leading_uniform_deflection_phase(scaled_offset_chi)?;
    let scaled_leading_uniform_residual =
        kappa * deflection_angle_rad
            - (1.0 + 2.0 * 3.0_f64.sqrt() / scaled_offset_chi).ln();
    let quadrature_error_estimate = 2.0 * estimated_half_orbit_error;
    let outer_turning_radius_over_m = 1.0 + kappa * y_turn;

    if !azimuthal_change_rad.is_finite()
        || !deflection_angle_rad.is_finite()
        || !leading_uniform_phase_rad.is_finite()
        || !scaled_leading_uniform_residual.is_finite()
        || !quadrature_error_estimate.is_finite()
        || !outer_turning_radius_over_m.is_finite()
    {
        return Err(KerrGeodesicError::NonFinite("near_extremal_deflection_result").into());
    }

    Ok(KerrStrongDeflectionResult {
        spin: a,
        extremality_parameter: kappa,
        scaled_offset_chi,
        impact_parameter_over_m: impact_parameter,
        critical_impact_parameter_over_m: orbit.critical_impact_parameter_over_m,
        photon_orbit_radius_over_m: orbit.radius_over_m,
        outer_turning_radius_over_m,
        scaled_radial_potential_residual,
        scaled_radial_potential_derivative_at_turn,
        azimuthal_change_rad,
        deflection_angle_rad,
        leading_uniform_phase_rad,
        scaled_leading_uniform_residual,
        quadrature_error_estimate,
        quadrature_evaluations: evaluations,
        root_iterations,
    })
}

fn simpson(a: f64, b: f64, fa: f64, fm: f64, fb: f64) -> f64 {
    (b - a) * (fa + 4.0 * fm + fb) / 6.0
}

#[allow(clippy::too_many_arguments)]
fn adaptive_simpson(
    function: &mut impl FnMut(f64) -> Result<f64, KerrStrongDeflectionError>,
    a: f64,
    b: f64,
    fa: f64,
    fm: f64,
    fb: f64,
    whole: f64,
    tolerance: f64,
    depth_remaining: usize,
    maximum_evaluations: usize,
    evaluations: &mut usize,
) -> Result<(f64, f64), KerrStrongDeflectionError> {
    let midpoint = a + 0.5 * (b - a);
    let left_midpoint = a + 0.5 * (midpoint - a);
    let right_midpoint = midpoint + 0.5 * (b - midpoint);
    if *evaluations + 2 > maximum_evaluations {
        return Err(KerrStrongDeflectionError::QuadratureDidNotConverge {
            estimated_error: f64::INFINITY,
            requested_tolerance: tolerance,
            evaluations: *evaluations,
        });
    }
    let flm = function(left_midpoint)?;
    let frm = function(right_midpoint)?;
    let left = simpson(a, midpoint, fa, flm, fm);
    let right = simpson(midpoint, b, fm, frm, fb);
    let combined = left + right;
    let correction = combined - whole;
    let estimated_error = correction.abs() / 15.0;
    if estimated_error <= tolerance {
        return Ok((combined + correction / 15.0, estimated_error));
    }
    if depth_remaining == 0 {
        return Err(KerrStrongDeflectionError::QuadratureDidNotConverge {
            estimated_error,
            requested_tolerance: tolerance,
            evaluations: *evaluations,
        });
    }
    let (left_value, left_error) = adaptive_simpson(
        function,
        a,
        midpoint,
        fa,
        flm,
        fm,
        left,
        0.5 * tolerance,
        depth_remaining - 1,
        maximum_evaluations,
        evaluations,
    )?;
    let (right_value, right_error) = adaptive_simpson(
        function,
        midpoint,
        b,
        fm,
        frm,
        fb,
        right,
        0.5 * tolerance,
        depth_remaining - 1,
        maximum_evaluations,
        evaluations,
    )?;
    Ok((left_value + right_value, left_error + right_error))
}

#[cfg(test)]
mod tests {
    use super::{
        KerrStrongDeflectionOptions, integrate_equatorial_prograde_strong_deflection,
    };

    fn close(actual: f64, expected: f64, relative_tolerance: f64) {
        let scale = expected.abs().max(1.0);
        assert!(
            (actual - expected).abs() <= relative_tolerance * scale,
            "actual={actual:.16e}, expected={expected:.16e}, tolerance={relative_tolerance:.3e}"
        );
    }

    #[test]
    fn near_extremal_deflection_matches_independent_rescaled_quadrature_grid() {
        // Independent SciPy reference values generated from the exact
        // equatorial Kerr radial potential with a turning-point-regular
        // coordinate. These are equation-level references, not Rust outputs.
        let references = [
            (0.1, 0.1, 37.658_611_510_5),
            (0.1, 0.5, 22.047_821_659_2),
            (0.1, 1.0, 16.055_152_261_2),
            (0.1, 2.0, 10.899_241_244_9),
            (0.03, 0.1, 121.865_648_309),
            (0.03, 0.5, 71.132_577_138_2),
            (0.03, 1.0, 51.700_664_805_7),
            (0.03, 2.0, 35.032_156_460_5),
            (0.01, 0.1, 360.902_017_816),
            (0.01, 0.5, 209.945_986_421),
            (0.01, 1.0, 152.208_191_910),
            (0.01, 2.0, 102.780_381_134),
            (0.003, 0.1, 1195.625_333_06),
            (0.003, 0.5, 693.945_276_525),
            (0.003, 1.0, 502.183_721_671),
            (0.003, 2.0, 338.174_027_408),
            (0.001, 0.1, 3578.795_511_17),
            (0.001, 0.5, 2075.065_864_17),
            (0.001, 1.0, 1500.396_343_95),
            (0.001, 2.0, 1009.038_899_70),
        ];
        let options = KerrStrongDeflectionOptions::default();
        for (kappa, chi, expected_deflection) in references {
            let spin = (1.0 - kappa * kappa).sqrt();
            let result =
                integrate_equatorial_prograde_strong_deflection(spin, chi, options).unwrap();
            close(result.deflection_angle_rad, expected_deflection, 2.0e-7);
            assert!(result.scaled_radial_potential_residual <= 1.0e-10);
            assert!(result.scaled_radial_potential_derivative_at_turn > 0.0);
            assert!(result.quadrature_error_estimate.is_finite());
            assert!(result.quadrature_error_estimate >= 0.0);
            assert!(result.quadrature_evaluations >= 5);
            assert!(result.root_iterations > 0);
            assert!(result.outer_turning_radius_over_m > result.photon_orbit_radius_over_m);
        }
    }

    #[test]
    fn near_extremal_scaled_deflection_phase_converges_to_uniform_crossover() {
        let chi = 0.5;
        let expected = (1.0 + 2.0 * 3.0_f64.sqrt() / chi).ln();
        let kappas = [0.1, 0.03, 0.01, 0.003, 0.001];
        let mut previous_residual = f64::INFINITY;
        for kappa in kappas {
            let spin = (1.0 - kappa * kappa).sqrt();
            let result = integrate_equatorial_prograde_strong_deflection(
                spin,
                chi,
                KerrStrongDeflectionOptions::default(),
            )
            .unwrap();
            let residual = result.scaled_leading_uniform_residual.abs();
            assert!(residual < previous_residual);
            assert!(residual < 5.0e-3);
            close(
                result.scaled_leading_uniform_residual,
                kappa * result.deflection_angle_rad - expected,
                1.0e-12,
            );
            previous_residual = residual;
        }
    }

    #[test]
    fn near_extremal_deflection_rejects_invalid_options_offsets_and_exact_extremality() {
        let invalid_options = KerrStrongDeflectionOptions {
            relative_tolerance: f64::NAN,
            ..KerrStrongDeflectionOptions::default()
        };
        assert!(integrate_equatorial_prograde_strong_deflection(0.99, 0.5, invalid_options).is_err());
        assert!(integrate_equatorial_prograde_strong_deflection(
            0.99,
            0.0,
            KerrStrongDeflectionOptions::default()
        )
        .is_err());
        assert!(integrate_equatorial_prograde_strong_deflection(
            1.0,
            0.5,
            KerrStrongDeflectionOptions::default()
        )
        .is_err());
        assert!(integrate_equatorial_prograde_strong_deflection(
            -0.5,
            0.5,
            KerrStrongDeflectionOptions::default()
        )
        .is_err());
    }

    #[test]
    fn near_extremal_deflection_fails_closed_when_quadrature_budget_is_exhausted() {
        let options = KerrStrongDeflectionOptions {
            maximum_quadrature_evaluations: 5,
            maximum_quadrature_depth: 1,
            ..KerrStrongDeflectionOptions::default()
        };
        let result = integrate_equatorial_prograde_strong_deflection(0.999_999_5, 0.1, options);
        assert!(matches!(
            result,
            Err(super::KerrStrongDeflectionError::QuadratureDidNotConverge { .. })
        ));
    }
}
