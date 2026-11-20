// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Independent full-component Bach reference for a static spherical metric.
//!
//! ds^2 = -B(r) dt^2 + dr^2/B(r) + r^2 dOmega^2, signature (-+++).
//! The component expressions come from the full covariant Bach tensor, not
//! from the symmetry-reduced B1 +/- B2 equations. Coordinate order is
//! (t, r, theta, phi); the radial Taylor variable is coordinate slot 1.
//! Radius and coefficients must use one consistent geometric length unit;
//! B(r) is dimensionless and component units follow the coordinate basis.
//!
//! The Bach components need fourth derivatives of B(r). In this static
//! specialization B_rr depends only through B'''(r), so differentiating it
//! for the radial divergence also needs only B''''(r). A general arbitrary-
//! metric contracted-divergence evaluator still needs fifth-order metric data.
//! This scalar jet does not itself carry chart identity; this module is a
//! chart-specific reference, not a generic tensor compatibility layer.

use core::fmt;
use super::taylor_jet::{TaylorJet4Order5, TaylorJetError};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StaticSphericalBachComponents {
    pub radius: f64,
    pub lapse: f64,
    pub b_tt: f64,
    pub b_rr: f64,
    pub b_theta_theta: f64,
    pub b_phi_phi_over_sin_squared: f64,
    pub b1_plus_b2: f64,
    pub b1_minus_b2: f64,
    pub trace: f64,
    pub radial_covariant_divergence: f64,
}

impl StaticSphericalBachComponents {
    pub fn max_abs_component(self) -> f64 {
        [
            self.b_tt,
            self.b_rr,
            self.b_theta_theta,
            self.b_phi_phi_over_sin_squared,
        ]
        .into_iter()
        .fold(0.0_f64, |maximum, value| maximum.max(value.abs()))
    }

    pub fn max_abs_reduced_residual(self) -> f64 {
        self.b1_plus_b2.abs().max(self.b1_minus_b2.abs())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StaticSphericalBachError {
    InvalidRadius(f64),
    OutsideStaticRegion { radius: f64, lapse: f64 },
    InsufficientDerivativeOrder { required: u8, actual: u8 },
    NonFiniteResult(&'static str),
    TaylorJet(TaylorJetError),
}

impl From<TaylorJetError> for StaticSphericalBachError {
    fn from(value: TaylorJetError) -> Self {
        Self::TaylorJet(value)
    }
}

impl fmt::Display for StaticSphericalBachError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRadius(radius) => write!(
                f,
                "static spherical Bach radius must be finite and positive, got {radius}"
            ),
            Self::OutsideStaticRegion { radius, lapse } => write!(
                f,
                "static spherical Bach evaluation requires B(r)>0, got B({radius})={lapse}"
            ),
            Self::InsufficientDerivativeOrder { required, actual } => write!(
                f,
                "static Bach requires Taylor-jet order {required}, got {actual}"
            ),
            Self::NonFiniteResult(name) => write!(f, "static spherical Bach {name} is non-finite"),
            Self::TaylorJet(error) => write!(f, "{error}"),
        }
    }
}
impl std::error::Error for StaticSphericalBachError {}

/// Evaluate the full covariant Bach components from a local Taylor jet B(r).
///
/// Bach component values require fourth-order metric data. In this static
/// specialization B_rr contains derivatives only through third order, so the
/// radial covariant-divergence diagnostic also requires only a fourth-order
/// lapse jet. The azimuthal component is returned divided by sin(theta)^2.
pub fn static_spherical_bach(
    radius: f64,
    lapse: TaylorJet4Order5,
) -> Result<StaticSphericalBachComponents, StaticSphericalBachError> {
    if !radius.is_finite() || radius <= 0.0 {
        return Err(StaticSphericalBachError::InvalidRadius(radius));
    }
    if lapse.order() < 4 {
        return Err(StaticSphericalBachError::InsufficientDerivativeOrder {
            required: 4,
            actual: lapse.order(),
        });
    }
    lapse.require_coordinate_center(1, radius)?;
    let b0 = lapse.value();
    if !b0.is_finite() || b0 <= 0.0 {
        return Err(StaticSphericalBachError::OutsideStaticRegion {
            radius,
            lapse: b0,
        });
    }

    let expansion_point = lapse
        .expansion_point()
        .ok_or(TaylorJetError::MissingExpansionPoint)?;
    let r = TaylorJet4Order5::variable_at(expansion_point, 1)?;
    let r2 = r.multiply(r)?;
    let r3 = r2.multiply(r)?;
    let r4 = r3.multiply(r)?;

    let b1 = lapse.partial(1)?;
    let b2 = b1.partial(1)?;
    let b3 = b2.partial(1)?;
    let b4 = b3.partial(1)?;

    let b1_sq = b1.multiply(b1)?;
    let b2_sq = b2.multiply(b2)?;
    let b1_b2 = b1.multiply(b2)?;
    let b1_b3 = b1.multiply(b3)?;
    let b_b1 = lapse.multiply(b1)?;
    let b_b2 = lapse.multiply(b2)?;
    let b_b3 = lapse.multiply(b3)?;
    let b_b4 = lapse.multiply(b4)?;

    // Shared lower-derivative polynomial in the four diagonal components.
    let common = r2
        .multiply(b_b2)?.scale(4.0)?
        .add(r2.multiply(b1_sq)?.scale(4.0)?)?
        .subtract(r.multiply(b_b1)?.scale(8.0)?)?
        .add(lapse.multiply(lapse)?.scale(4.0)?)?
        .subtract(TaylorJet4Order5::constant(4.0)?)?;

    let theta_numerator = r4
        .multiply(b_b4)?.scale(-2.0)?
        .add(r4.multiply(b1_b3)?.scale(-2.0)?)?
        .add(r4.multiply(b2_sq)?)?
        .add(r3.multiply(b_b3)?.scale(-4.0)?)?
        .add(r3.multiply(b1_b2)?.scale(-4.0)?)?
        .add(common)?;

    let tt_numerator = r4
        .multiply(b_b4)?.scale(-4.0)?
        .add(r4.multiply(b1_b3)?.scale(-2.0)?)?
        .add(r4.multiply(b2_sq)?)?
        .add(r3.multiply(b_b3)?.scale(-12.0)?)?
        .add(r3.multiply(b1_b2)?.scale(-4.0)?)?
        .add(common)?;

    let rr_numerator = r4
        .multiply(b1_b3)?.scale(-2.0)?
        .add(r4.multiply(b2_sq)?)?
        .add(r3.multiply(b_b3)?.scale(4.0)?)?
        .add(r3.multiply(b1_b2)?.scale(-4.0)?)?
        .add(common)?;

    let twelve = TaylorJet4Order5::constant(12.0)?;
    let r2_inverse = r2.reciprocal()?;
    let r4_inverse = r4.reciprocal()?;
    let b_inverse = lapse.reciprocal()?;

    let b_theta_jet = theta_numerator.multiply(r2_inverse)?.divide(twelve)?;
    let b_tt_jet = tt_numerator
        .multiply(lapse)?
        .multiply(r4_inverse)?
        .divide(twelve)?;
    let b_rr_jet = rr_numerator
        .multiply(b_inverse)?
        .multiply(r4_inverse)?
        .scale(-1.0)?
        .divide(twelve)?;

    let b_tt = b_tt_jet.value();
    let b_rr = b_rr_jet.value();
    let b_theta_theta = b_theta_jet.value();
    let b_phi_phi_over_sin_squared = b_theta_theta;

    let b1_plus_b2 = b_tt / b0 + b0 * b_rr;
    let b1_minus_b2 = 4.0 * b_theta_theta / radius.powi(2);
    let trace = -b_tt / b0 + b0 * b_rr + 2.0 * b_theta_theta / radius.powi(2);

    // For diagonal T_ab with T_phi_phi=sin(theta)^2 T_theta_theta:
    // nabla^a T_ar = B*T_rr' + (3B'/2+2B/r)T_rr + B'T_tt/(2B^2)
    //                - 2T_theta_theta/r^3.
    let b_rr_prime = b_rr_jet.partial(1)?.value();
    let radial_covariant_divergence = b0 * b_rr_prime
        + (1.5 * b1.value() + 2.0 * b0 / radius) * b_rr
        + b1.value() * b_tt / (2.0 * b0 * b0)
        - 2.0 * b_theta_theta / radius.powi(3);

    let outputs = [
        ("B_tt", b_tt),
        ("B_rr", b_rr),
        ("B_theta_theta", b_theta_theta),
        ("B_phi_phi_over_sin_squared", b_phi_phi_over_sin_squared),
        ("B1_plus_B2", b1_plus_b2),
        ("B1_minus_B2", b1_minus_b2),
        ("trace", trace),
        ("radial_covariant_divergence", radial_covariant_divergence),
    ];
    for (name, value) in outputs {
        if !value.is_finite() {
            return Err(StaticSphericalBachError::NonFiniteResult(name));
        }
    }

    Ok(StaticSphericalBachComponents {
        radius,
        lapse: b0,
        b_tt,
        b_rr,
        b_theta_theta,
        b_phi_phi_over_sin_squared,
        b1_plus_b2,
        b1_minus_b2,
        trace,
        radial_covariant_divergence,
    })
}

/// Evaluate the published factorization of the static B1+B2 Bach projection.
///
/// On the static domain B(r)>0, r>0:
/// B1+B2 = -B(r) * (r*B''''(r) + 4*B'''(r)) / (3r).
///
/// This is a reduced-equation oracle, independent of the full component
/// assembly in `static_spherical_bach`. The order-four jet is sufficient.
pub fn static_spherical_bach_sum_factorized(
    radius: f64,
    lapse: TaylorJet4Order5,
) -> Result<f64, StaticSphericalBachError> {
    if !radius.is_finite() || radius <= 0.0 {
        return Err(StaticSphericalBachError::InvalidRadius(radius));
    }
    if lapse.order() < 4 {
        return Err(StaticSphericalBachError::InsufficientDerivativeOrder {
            required: 4,
            actual: lapse.order(),
        });
    }
    lapse.require_coordinate_center(1, radius)?;
    let b0 = lapse.value();
    if !b0.is_finite() || b0 <= 0.0 {
        return Err(StaticSphericalBachError::OutsideStaticRegion {
            radius,
            lapse: b0,
        });
    }

    let b3 = lapse.partial(1)?.partial(1)?.partial(1)?;
    let b4 = b3.partial(1)?;
    let residual = -b0 * (radius * b4.value() + 4.0 * b3.value()) / (3.0 * radius);
    if !residual.is_finite() {
        return Err(StaticSphericalBachError::NonFiniteResult(
            "factorized B1_plus_B2",
        ));
    }
    Ok(residual)
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

    fn polynomial_jet(radius: f64, coefficients: &[f64]) -> TaylorJet4Order5 {
        let r = TaylorJet4Order5::variable(radius, 1).unwrap();
        let mut power = TaylorJet4Order5::constant(1.0).unwrap();
        let mut result = TaylorJet4Order5::constant(0.0).unwrap();
        for coefficient in coefficients {
            result = result.add(power.scale(*coefficient).unwrap()).unwrap();
            power = power.multiply(r).unwrap();
        }
        result
    }

    fn mk_metric_jet(
        radius: f64,
        c_inverse: f64,
        c_constant: f64,
        c_linear: f64,
        c_quadratic: f64,
    ) -> TaylorJet4Order5 {
        let r = TaylorJet4Order5::variable(radius, 1).unwrap();
        let r_inverse = r.reciprocal().unwrap();
        let r2 = r.multiply(r).unwrap();
        TaylorJet4Order5::constant(c_inverse)
            .unwrap()
            .multiply(r_inverse)
            .unwrap()
            .add(TaylorJet4Order5::constant(c_constant).unwrap())
            .unwrap()
            .add(r.scale(c_linear).unwrap())
            .unwrap()
            .add(r2.scale(c_quadratic).unwrap())
            .unwrap()
    }

    #[test]
    fn schwarzschild_bach_tensor_and_identities_vanish() {
        let jet = mk_metric_jet(8.0, -2.0, 1.0, 0.0, 0.0);
        let result = static_spherical_bach(8.0, jet).unwrap();
        assert!(result.max_abs_component() < 2e-12);
        assert!(result.max_abs_reduced_residual() < 2e-12);
        assert!(result.trace.abs() < 2e-12);
        assert!(result.radial_covariant_divergence.abs() < 2e-12);
    }

    #[test]
    fn mannheim_kazanas_bach_tensor_vanishes_with_nonzero_gamma_and_kappa() {
        let radius = 3.0;
        let jet = mk_metric_jet(radius, -1.7, 0.7, 0.1, -0.01);
        let result = static_spherical_bach(radius, jet).unwrap();
        assert!(result.max_abs_component() < 2e-12);
        assert!(result.trace.abs() < 2e-12);
        assert!(result.radial_covariant_divergence.abs() < 2e-12);
    }

    #[test]
    fn off_shell_components_match_constraint_and_inverse_fourth_power() {
        let q = 18.0 / 25.0;
        let at3 = static_spherical_bach(
            3.0,
            mk_metric_jet(3.0, -1.7, 1.1, 0.1, -0.01),
        ).unwrap();
        let at6 = static_spherical_bach(
            6.0,
            mk_metric_jet(6.0, -1.7, 1.1, 0.1, -0.01),
        ).unwrap();
        close(at3.b1_plus_b2, 0.0, 2e-12);
        close(at6.b1_plus_b2, 0.0, 2e-12);
        close(at3.b1_minus_b2, 4.0 * q / (3.0 * 3.0_f64.powi(4)), 2e-12);
        close(at6.b1_minus_b2, 4.0 * q / (3.0 * 6.0_f64.powi(4)), 2e-12);
        close(at3.b_theta_theta, q / (3.0 * 3.0_f64.powi(2)), 2e-12);
        close(at6.b_theta_theta, q / (3.0 * 6.0_f64.powi(2)), 2e-12);
        close(at3.trace, 0.0, 2e-12);
        close(at6.trace, 0.0, 2e-12);
        close(at3.radial_covariant_divergence, 0.0, 2e-12);
        close(at6.radial_covariant_divergence, 0.0, 2e-12);
        close(at3.b1_minus_b2 / at6.b1_minus_b2, 16.0, 2e-12);
    }

    #[test]
    fn factorized_sum_matches_full_tensor_projection_for_generic_lapses() {
        for radius in [1.75, 2.0, 2.25, 2.5] {
            let jet = polynomial_jet(
                radius,
                &[1.4, 0.03, -0.002, 0.0001, -0.00001, 0.000001],
            );
            let full = static_spherical_bach(radius, jet).unwrap();
            let factorized = static_spherical_bach_sum_factorized(radius, jet).unwrap();
            close(factorized, full.b1_plus_b2, 3e-10);
        }
    }

    #[test]
    fn factorized_sum_vanishes_for_on_and_off_shell_four_term_families() {
        for c_constant in [0.7, 1.1] {
            for radius in [3.0, 6.0] {
                let jet = mk_metric_jet(radius, -1.7, c_constant, 0.1, -0.01);
                close(
                    static_spherical_bach_sum_factorized(radius, jet).unwrap(),
                    0.0,
                    2e-12,
                );
            }
        }
    }

    #[test]
    fn lapse_jet_expansion_point_must_match_requested_radius() {
        let jet = mk_metric_jet(3.0, -2.0, 1.0, 0.0, 0.0);
        assert_eq!(
            static_spherical_bach(4.0, jet),
            Err(StaticSphericalBachError::TaylorJet(
                TaylorJetError::ExpansionPointCoordinateMismatch {
                    coordinate: 1,
                    expected: 4.0,
                    actual: 3.0
                }
            ))
        );
        let unbound = TaylorJet4Order5::constant(1.0).unwrap();
        assert_eq!(
            static_spherical_bach(3.0, unbound),
            Err(StaticSphericalBachError::TaylorJet(
                TaylorJetError::MissingExpansionPoint
            ))
        );
    }

    #[test]
    fn generic_degree_five_lapse_satisfies_trace_and_divergence_identities() {
        let radius = 2.0;
        let jet = polynomial_jet(radius, &[1.4, 0.03, -0.002, 0.0001, -0.00001, 0.000001]);
        let result = static_spherical_bach(radius, jet).unwrap();
        assert!(result.trace.abs() < 2e-10, "trace={:e}", result.trace);
        assert!(
            result.radial_covariant_divergence.abs() < 2e-9,
            "divergence={:e}",
            result.radial_covariant_divergence
        );
    }

    #[test]
    fn fourth_order_lapse_is_sufficient_for_full_static_result() {
        let radius = 8.0;
        let jet = mk_metric_jet(radius, -2.0, 1.0, 0.0, 0.0)
            .truncate(4)
            .unwrap();
        let result = static_spherical_bach(radius, jet).unwrap();
        assert!(result.max_abs_component() < 2e-12);
        assert!(result.trace.abs() < 2e-12);
        assert!(result.radial_covariant_divergence.abs() < 2e-12);
    }

    #[test]
    fn horizon_and_invalid_derivative_domains_fail_closed() {
        let horizon = mk_metric_jet(2.0, -2.0, 1.0, 0.0, 0.0);
        assert!(matches!(
            static_spherical_bach(2.0, horizon),
            Err(StaticSphericalBachError::OutsideStaticRegion { .. })
        ));
        let low_order = TaylorJet4Order5::variable(3.0, 1)
            .unwrap()
            .truncate(3)
            .unwrap();
        assert_eq!(
            static_spherical_bach(3.0, low_order),
            Err(StaticSphericalBachError::InsufficientDerivativeOrder {
                required: 4,
                actual: 3
            })
        );
        assert!(matches!(
            static_spherical_bach(0.0, polynomial_jet(1.0, &[1.0])),
            Err(StaticSphericalBachError::InvalidRadius(_))
        ));
    }
}
