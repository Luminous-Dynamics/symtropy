//! Analytic temporal quadrature oracle for EHD elastic drag impulse and heat.
//!
//! For a non-negative drag-rate coefficient k(t) and relative drift du(t):
//!
//!     I_n = integral_0^1 k(t) du(t) dt
//!     Q_f = integral_0^1 k(t) |du(t)|^2 dt >= 0.
//!
//! This module uses quadratic manufactured polynomials for k and each velocity
//! component. Exact reference integrals are obtained by polynomial convolution
//! and analytic antiderivatives. Gauss-Legendre 4-point integrates through
//! degree 7; the heating integrand can reach degree 6, so GL4 is an exact
//! reference for this manufactured class (up to floating-point arithmetic).
//!
//! This is a temporal quadrature oracle for spatially integrated coefficients.
//! It does not solve the PDE, construct spatial quadrature, or certify arbitrary
//! solver fields. The calling solver must still demonstrate quadrature
//! refinement on non-polynomial and sharp-gradient source histories.

use std::error::Error;
use std::fmt;

/// Polynomial coefficients in ascending powers: c[0] + c[1] t + c[2] t^2.
pub type Quadratic = [f64; 3];

/// Manufactured time history for the integrated elastic drag coefficient
/// and the three components of charged-minus-neutral relative velocity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragQuadratureCase {
    /// Non-negative collision/drag rate coefficient after spatial integration.
    pub drag_rate_kg_per_s: Quadratic,
    pub relative_velocity_m_s: [Quadratic; 3],
}

/// Supported reference temporal quadrature rules on normalized time [0, 1].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuadratureRule {
    GaussLegendre1,
    GaussLegendre2,
    GaussLegendre3,
    GaussLegendre4,
    Simpson3,
}

impl QuadratureRule {
    pub fn name(self) -> &'static str {
        match self {
            Self::GaussLegendre1 => "gauss_legendre_1",
            Self::GaussLegendre2 => "gauss_legendre_2",
            Self::GaussLegendre3 => "gauss_legendre_3",
            Self::GaussLegendre4 => "gauss_legendre_4",
            Self::Simpson3 => "simpson_3",
        }
    }

    /// Algebraic exactness degree on [0, 1] for the canonical rule.
    pub fn exactness_degree(self) -> u32 {
        match self {
            Self::GaussLegendre1 => 1,
            Self::GaussLegendre2 => 3,
            Self::GaussLegendre3 => 5,
            Self::GaussLegendre4 => 7,
            Self::Simpson3 => 3,
        }
    }

    fn nodes_and_weights(self) -> (&'static [f64], &'static [f64]) {
        const X1: [f64; 1] = [0.5];
        const W1: [f64; 1] = [1.0];

        const X2: [f64; 2] = [
            0.211_324_865_405_187_13,
            0.788_675_134_594_812_9,
        ];
        const W2: [f64; 2] = [0.5, 0.5];

        const X3: [f64; 3] = [
            0.112_701_665_379_258_3,
            0.5,
            0.887_298_334_620_741_7,
        ];
        const W3: [f64; 3] = [
            0.277_777_777_777_777_8,
            0.444_444_444_444_444_4,
            0.277_777_777_777_777_8,
        ];

        const X4: [f64; 4] = [
            0.069_431_844_202_973_71,
            0.330_009_478_207_571_87,
            0.669_990_521_792_428_1,
            0.930_568_155_797_026_2,
        ];
        const W4: [f64; 4] = [
            0.173_927_422_568_726_9,
            0.326_072_577_431_273_1,
            0.326_072_577_431_273_1,
            0.173_927_422_568_726_9,
        ];

        const XS: [f64; 3] = [0.0, 0.5, 1.0];
        const WS: [f64; 3] = [
            0.166_666_666_666_666_66,
            0.666_666_666_666_666_6,
            0.166_666_666_666_666_66,
        ];

        match self {
            Self::GaussLegendre1 => (&X1, &W1),
            Self::GaussLegendre2 => (&X2, &W2),
            Self::GaussLegendre3 => (&X3, &W3),
            Self::GaussLegendre4 => (&X4, &W4),
            Self::Simpson3 => (&XS, &WS),
        }
    }
}

/// Case-specific absolute-plus-relative tolerances for impulse and heating.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuadratureTolerance {
    pub impulse_absolute_ns: f64,
    pub impulse_relative: f64,
    pub heating_absolute_j: f64,
    pub heating_relative: f64,
}

/// Numerical-versus-analytic quadrature result.
#[derive(Clone, Debug, PartialEq)]
pub struct QuadratureReport {
    pub rule: &'static str,
    pub algebraic_exactness_degree: u32,
    pub impulse_degree_max: u32,
    pub heating_degree_max: u32,
    pub quadrature_impulse_ns: [f64; 3],
    pub exact_impulse_ns: [f64; 3],
    pub impulse_residual_ns: [f64; 3],
    pub quadrature_heating_j: f64,
    pub exact_heating_j: f64,
    pub heating_residual_j: f64,
    pub allowed_impulse_ns: [f64; 3],
    pub allowed_heating_j: f64,
    pub impulse_within_tolerance: [bool; 3],
    pub heating_within_tolerance: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum QuadratureError {
    NonFiniteCoefficient,
    NegativeDragRate { time: f64, value: f64 },
    InvalidTolerance,
    NonFiniteIntegral,
    RuleWeightsInvalid,
}

impl fmt::Display for QuadratureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteCoefficient => write!(f, "manufactured drag/velocity polynomial has a non-finite coefficient"),
            Self::NegativeDragRate { time, value } => write!(
                f, "drag-rate coefficient is negative at t={time}: {value} kg/s"
            ),
            Self::InvalidTolerance => write!(f, "quadrature tolerances must be finite and non-negative"),
            Self::NonFiniteIntegral => write!(f, "analytic or quadrature integral became non-finite"),
            Self::RuleWeightsInvalid => write!(f, "quadrature rule has invalid nodes or weights"),
        }
    }
}

impl Error for QuadratureError {}

fn evaluate(coefficients: &[f64], time: f64) -> f64 {
    coefficients.iter().rev().fold(0.0, |value, coefficient| {
        value.mul_add(time, *coefficient)
    })
}

fn multiply(left: &[f64], right: &[f64]) -> Vec<f64> {
    let mut result = vec![0.0; left.len() + right.len() - 1];
    for (i, a) in left.iter().enumerate() {
        for (j, b) in right.iter().enumerate() {
            result[i + j] += a * b;
        }
    }
    result
}

fn add_polynomials(left: &[f64], right: &[f64]) -> Vec<f64> {
    let mut result = vec![0.0; left.len().max(right.len())];
    for (index, value) in left.iter().enumerate() {
        result[index] += value;
    }
    for (index, value) in right.iter().enumerate() {
        result[index] += value;
    }
    result
}

fn exact_integral(coefficients: &[f64]) -> f64 {
    coefficients
        .iter()
        .enumerate()
        .map(|(degree, coefficient)| coefficient / (degree as f64 + 1.0))
        .sum()
}

fn validate_case(case: &DragQuadratureCase) -> Result<(), QuadratureError> {
    if !case.drag_rate_kg_per_s.iter().all(|value| value.is_finite())
        || !case.relative_velocity_m_s
            .iter()
            .all(|component| component.iter().all(|value| value.is_finite()))
    {
        return Err(QuadratureError::NonFiniteCoefficient);
    }

    let [c0, c1, c2] = case.drag_rate_kg_per_s;
    // A quadratic's minimum on [0,1] is at an endpoint or its interior vertex.
    let mut candidates = vec![
        (0.0, c0),
        (1.0, c0 + c1 + c2),
    ];
    if c2 > 0.0 {
        let vertex = -c1 / (2.0 * c2);
        if (0.0..=1.0).contains(&vertex) {
            candidates.push((vertex, evaluate(&case.drag_rate_kg_per_s, vertex)));
        }
    }
    for (time, value) in candidates {
        if !value.is_finite() {
            return Err(QuadratureError::NonFiniteIntegral);
        }
        if value < 0.0 {
            return Err(QuadratureError::NegativeDragRate { time, value });
        }
    }
    Ok(())
}

fn analytic_integrals(
    case: &DragQuadratureCase,
) -> Result<([f64; 3], f64, u32, u32), QuadratureError> {
    let drag = case.drag_rate_kg_per_s;
    let mut impulse = [0.0; 3];
    let mut impulse_degree = 0_u32;
    let mut speed_squared = vec![0.0];
    let mut drift_degree = 0_u32;

    for component in 0..3 {
        let velocity = case.relative_velocity_m_s[component];
        let product = multiply(&drag, &velocity);
        impulse[component] = exact_integral(&product);
        impulse_degree = impulse_degree.max((product.len() - 1) as u32);

        let square = multiply(&velocity, &velocity);
        speed_squared = add_polynomials(&speed_squared, &square);
        drift_degree = drift_degree.max((velocity.iter().rposition(|value| *value != 0.0).unwrap_or(0)) as u32);
    }

    let heating_polynomial = multiply(&drag, &speed_squared);
    let heating = exact_integral(&heating_polynomial);
    let heating_degree = (heating_polynomial.len() - 1) as u32;

    if !impulse.iter().all(|value| value.is_finite()) || !heating.is_finite() {
        return Err(QuadratureError::NonFiniteIntegral);
    }
    let _ = drift_degree;
    Ok((impulse, heating, impulse_degree, heating_degree))
}

/// Integrate the manufactured case with the selected rule and compare
/// impulse and frictional heating independently against analytic moments.
pub fn audit_temporal_quadrature(
    case: &DragQuadratureCase,
    rule: QuadratureRule,
    tolerance: QuadratureTolerance,
) -> Result<QuadratureReport, QuadratureError> {
    for value in [
        tolerance.impulse_absolute_ns,
        tolerance.impulse_relative,
        tolerance.heating_absolute_j,
        tolerance.heating_relative,
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(QuadratureError::InvalidTolerance);
        }
    }
    validate_case(case)?;

    let (nodes, weights) = rule.nodes_and_weights();
    if nodes.is_empty()
        || nodes.len() != weights.len()
        || !nodes.iter().all(|node| node.is_finite() && (0.0..=1.0).contains(node))
        || !weights.iter().all(|weight| weight.is_finite() && *weight > 0.0)
        || (weights.iter().sum::<f64>() - 1.0).abs() > 1.0e-13
    {
        return Err(QuadratureError::RuleWeightsInvalid);
    }

    let mut quadrature_impulse = [0.0_f64; 3];
    let mut quadrature_heating = 0.0_f64;
    for (time, weight) in nodes.iter().zip(weights.iter()) {
        let drag_rate = evaluate(&case.drag_rate_kg_per_s, *time);
        if !drag_rate.is_finite() || drag_rate < 0.0 {
            return Err(QuadratureError::NegativeDragRate {
                time: *time,
                value: drag_rate,
            });
        }

        let mut speed_squared = 0.0_f64;
        for component in 0..3 {
            let velocity = evaluate(&case.relative_velocity_m_s[component], *time);
            quadrature_impulse[component] += weight * drag_rate * velocity;
            speed_squared += velocity * velocity;
        }
        quadrature_heating += weight * drag_rate * speed_squared;
    }

    let (exact_impulse, exact_heating, impulse_degree_max, heating_degree_max) =
        analytic_integrals(case)?;
    if !quadrature_impulse.iter().all(|value| value.is_finite())
        || !quadrature_heating.is_finite()
    {
        return Err(QuadratureError::NonFiniteIntegral);
    }

    let mut impulse_residual = [0.0; 3];
    let mut allowed_impulse = [0.0; 3];
    let mut impulse_within_tolerance = [false; 3];
    for component in 0..3 {
        impulse_residual[component] = quadrature_impulse[component] - exact_impulse[component];
        allowed_impulse[component] = tolerance.impulse_absolute_ns
            + tolerance.impulse_relative * exact_impulse[component].abs();
        impulse_within_tolerance[component] =
            impulse_residual[component].abs() <= allowed_impulse[component];
    }

    let heating_residual = quadrature_heating - exact_heating;
    let allowed_heating = tolerance.heating_absolute_j
        + tolerance.heating_relative * exact_heating.abs();

    Ok(QuadratureReport {
        rule: rule.name(),
        algebraic_exactness_degree: rule.exactness_degree(),
        impulse_degree_max,
        heating_degree_max,
        quadrature_impulse_ns: quadrature_impulse,
        exact_impulse_ns: exact_impulse,
        impulse_residual_ns: impulse_residual,
        quadrature_heating_j: quadrature_heating,
        exact_heating_j: exact_heating,
        heating_residual_j: heating_residual,
        allowed_impulse_ns: allowed_impulse,
        allowed_heating_j: allowed_heating,
        impulse_within_tolerance,
        heating_within_tolerance: heating_residual.abs() <= allowed_heating,
    })
}

/// Manufactured case with a degree-six heating integrand. Four-point
/// Gauss-Legendre is exact for the whole class (degree <= 7).
pub fn reference_case() -> DragQuadratureCase {
    DragQuadratureCase {
        drag_rate_kg_per_s: [1.0, 0.5, 0.75],
        relative_velocity_m_s: [
            [0.5, 1.0, 0.75],
            [1.0, -0.25, 0.5],
            [0.25, 0.75, -0.5],
        ],
    }
}

pub fn reference_self_check() -> Result<(), QuadratureError> {
    let report = audit_temporal_quadrature(
        &reference_case(),
        QuadratureRule::GaussLegendre4,
        QuadratureTolerance {
            impulse_absolute_ns: 1.0e-12,
            impulse_relative: 1.0e-12,
            heating_absolute_j: 1.0e-12,
            heating_relative: 1.0e-12,
        },
    )?;
    if report.impulse_within_tolerance.iter().any(|passed| !passed)
        || !report.heating_within_tolerance
        || report.heating_degree_max != 6
        || report.impulse_degree_max != 4
    {
        return Err(QuadratureError::NonFiniteIntegral);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tolerance() -> QuadratureTolerance {
        QuadratureTolerance {
            impulse_absolute_ns: 1.0e-12,
            impulse_relative: 1.0e-12,
            heating_absolute_j: 1.0e-12,
            heating_relative: 1.0e-12,
        }
    }

    #[test]
    fn four_point_gauss_legendre_exactly_integrates_quadratic_drag_and_drift() {
        let report = audit_temporal_quadrature(
            &reference_case(),
            QuadratureRule::GaussLegendre4,
            tolerance(),
        ).unwrap();
        assert_eq!(report.impulse_degree_max, 4);
        assert_eq!(report.heating_degree_max, 6);
        assert!(report.impulse_within_tolerance.iter().all(|passed| *passed));
        assert!(report.heating_within_tolerance);
    }

    #[test]
    fn impulse_can_pass_while_quadratic_heating_fails_lower_order_rule() {
        let case = DragQuadratureCase {
            drag_rate_kg_per_s: [1.0, 0.0, 0.0],
            relative_velocity_m_s: [
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0],
            ],
        };
        let report = audit_temporal_quadrature(
            &case,
            QuadratureRule::GaussLegendre2,
            tolerance(),
        ).unwrap();
        // Integral(t^2) is exact for GL2; integral(t^4) is not.
        assert!(report.impulse_within_tolerance[0]);
        assert!(!report.heating_within_tolerance);
    }

    #[test]
    fn degree_six_heating_requires_four_point_gauss_reference() {
        let case = DragQuadratureCase {
            drag_rate_kg_per_s: [0.0, 0.0, 1.0],
            relative_velocity_m_s: [
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0],
            ],
        };
        let three = audit_temporal_quadrature(
            &case,
            QuadratureRule::GaussLegendre3,
            tolerance(),
        ).unwrap();
        let four = audit_temporal_quadrature(
            &case,
            QuadratureRule::GaussLegendre4,
            tolerance(),
        ).unwrap();
        assert!(three.impulse_within_tolerance[0]);
        assert!(!three.heating_within_tolerance);
        assert!(four.heating_within_tolerance);
    }

    #[test]
    fn Simpson_rule_exactly_integrates_degree_three_or_lower() {
        let case = DragQuadratureCase {
            drag_rate_kg_per_s: [1.0, 0.0, 0.0],
            relative_velocity_m_s: [
                [1.0, 1.0, 0.0],
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0],
            ],
        };
        let report = audit_temporal_quadrature(
            &case,
            QuadratureRule::Simpson3,
            tolerance(),
        ).unwrap();
        assert!(report.impulse_within_tolerance.iter().all(|passed| *passed));
        assert!(report.heating_within_tolerance);
    }

    #[test]
    fn rejects_drag_rate_that_becomes_negative_inside_interval() {
        let case = DragQuadratureCase {
            drag_rate_kg_per_s: [1.0, -6.0, 6.0],
            relative_velocity_m_s: [[1.0, 0.0, 0.0]; 3],
        };
        assert!(matches!(
            audit_temporal_quadrature(&case, QuadratureRule::GaussLegendre4, tolerance()).unwrap_err(),
            QuadratureError::NegativeDragRate { .. }
        ));
    }

    #[test]
    fn rejects_non_finite_polynomial_coefficients() {
        let mut case = reference_case();
        case.relative_velocity_m_s[1][2] = f64::NAN;
        assert_eq!(
            audit_temporal_quadrature(&case, QuadratureRule::GaussLegendre4, tolerance()).unwrap_err(),
            QuadratureError::NonFiniteCoefficient
        );
    }

    #[test]
    fn rejects_invalid_tolerances() {
        let invalid = QuadratureTolerance {
            heating_relative: f64::NAN,
            ..tolerance()
        };
        assert_eq!(
            audit_temporal_quadrature(&reference_case(), QuadratureRule::GaussLegendre4, invalid).unwrap_err(),
            QuadratureError::InvalidTolerance
        );
    }

    #[test]
    fn reference_self_check_passes() {
        reference_self_check().unwrap();
    }
}
