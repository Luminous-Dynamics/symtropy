//! Independent matrix-reference checks for the SO(3) exponential Jacobians.
//!
//! The reference path below intentionally does not use Rotor::from_bivector,
//! so3_log, or the implementation's exponential-map helpers. It constructs
//! Rodrigues matrices directly and measures the left/right-trivialized
//! differential from matrix products.

use nalgebra::{Matrix3, Vector3};
use std::f64::consts::{PI, TAU};
use symtropy_math::{Bivector, So3Trivialization, so3_exp_jacobian};

fn skew(v: &Vector3<f64>) -> Matrix3<f64> {
    Matrix3::new(
        0.0, -v[2], v[1],
        v[2], 0.0, -v[0],
        -v[1], v[0], 0.0,
    )
}

/// Independent Rodrigues exponential for a rotation vector in conventional
/// [x, y, z] coordinates. A local series avoids cancellation at zero.
fn reference_exp(phi: &Vector3<f64>) -> Matrix3<f64> {
    let theta2 = phi.norm_squared();
    let k = skew(phi);
    let k2 = k * k;
    let (a, b) = if theta2 < 1.0e-8 {
        let theta4 = theta2 * theta2;
        (
            1.0 - theta2 / 6.0 + theta4 / 120.0,
            0.5 - theta2 / 24.0 + theta4 / 720.0,
        )
    } else {
        let theta = theta2.sqrt();
        (
            theta.sin() / theta,
            (1.0 - theta.cos()) / theta2,
        )
    };
    Matrix3::identity() + k * a + k2 * b
}

/// Symtropy uses [e01, e02, e12]. The corresponding conventional
/// rotation vector is [b12, -b02, b01].
fn as_bivector(phi: &Vector3<f64>) -> Bivector<3> {
    let mut value = Bivector::<3>::zero();
    value.set(0, 1, phi[2]);
    value.set(0, 2, -phi[1]);
    value.set(1, 2, phi[0]);
    value
}

fn as_rotation_vector(value: &Bivector<3>) -> Vector3<f64> {
    Vector3::new(value.get(1, 2), -value.get(0, 2), value.get(0, 1))
}

/// Measure D Exp(phi)[direction] after left or right trivialization.
///
/// A centered difference is used and the skew part is extracted directly.
fn reference_differential(
    phi: &Vector3<f64>,
    direction: &Vector3<f64>,
    trivialization: So3Trivialization,
    step: f64,
) -> Vector3<f64> {
    let base = reference_exp(phi);
    let plus = reference_exp(&(*phi + *direction * step));
    let minus = reference_exp(&(*phi - *direction * step));

    let (relative_plus, relative_minus) = match trivialization {
        So3Trivialization::LeftSpatial => {
            (plus * base.transpose(), minus * base.transpose())
        }
        So3Trivialization::RightBody => {
            (base.transpose() * plus, base.transpose() * minus)
        }
    };

    let derivative = (relative_plus - relative_minus) * (0.5 / step);
    let skew_derivative = (derivative - derivative.transpose()) * 0.5;
    Vector3::new(
        skew_derivative[(2, 1)],
        skew_derivative[(0, 2)],
        skew_derivative[(1, 0)],
    )
}

fn deterministic_unit(index: usize) -> Vector3<f64> {
    let i = index as f64 + 1.0;
    let value = Vector3::new(
        (0.73 * i).sin() + 0.17,
        (1.19 * i).cos() - 0.23,
        (0.47 * i + 0.31).sin() + 0.41,
    );
    value / value.norm()
}

#[test]
fn left_and_right_jacobians_match_independent_rodrigues_matrix_differences() {
    let angles: [f64; 12] = [
        0.0,
        1.0e-8,
        1.0e-5,
        1.0e-3,
        0.17,
        0.73,
        1.41,
        PI - 1.0e-5,
        PI,
        PI + 1.0e-5,
        4.9,
        TAU - 2.0e-2,
    ];
    let directions = [
        Vector3::new(0.31, -0.47, 0.83),
        Vector3::new(-0.71, 0.22, 0.39),
        Vector3::new(0.13, 0.91, -0.28),
    ];
    let trivializations = [
        So3Trivialization::LeftSpatial,
        So3Trivialization::RightBody,
    ];
    let axes = [
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
        deterministic_unit(0),
        deterministic_unit(1),
        deterministic_unit(2),
    ];

    for (axis_index, axis) in axes.into_iter().enumerate() {
        for angle in angles {
            let phi = axis * angle;
            let generator = as_bivector(&phi);

            for (direction_index, direction) in directions.iter().enumerate() {
                for trivialization in trivializations {
                    let jacobian = so3_exp_jacobian(&generator, trivialization)
                        .expect("finite SO(3) exponential Jacobian");
                    let predicted_bivector = jacobian
                        .apply(&as_bivector(direction))
                        .expect("finite tangent map result");
                    let predicted = as_rotation_vector(&predicted_bivector);
                    let measured =
                        reference_differential(&phi, direction, trivialization, 1.0e-5);
                    let error = (predicted - measured).norm();
                    let tolerance = 3.0e-8 * (1.0 + measured.norm());

                    assert!(
                        error <= tolerance,
                        "independent SO(3) matrix oracle mismatch: axis={axis_index}, angle={angle:e}, direction={direction_index}, trivialization={trivialization:?}, predicted={predicted:?}, measured={measured:?}, error={error:e}, tolerance={tolerance:e}"
                    );
                }
            }
        }
    }
}

#[test]
fn left_and_right_jacobians_are_distinct_away_from_identity() {
    let phi = Vector3::new(0.61, -0.38, 0.92);
    let direction = Vector3::new(-0.27, 0.53, 0.71);
    let generator = as_bivector(&phi);
    let tangent = as_bivector(&direction);

    let left = so3_exp_jacobian(&generator, So3Trivialization::LeftSpatial)
        .unwrap()
        .apply(&tangent)
        .unwrap();
    let right = so3_exp_jacobian(&generator, So3Trivialization::RightBody)
        .unwrap()
        .apply(&tangent)
        .unwrap();
    let difference = as_rotation_vector(&left) - as_rotation_vector(&right);

    assert!(
        difference.norm() > 1.0e-2,
        "left and right differentials must not silently collapse to one convention"
    );
}
