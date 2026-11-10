//! Independent homogeneous-matrix reference checks for SE(3) spatial algebra.
//!
//! The oracle constructs a homogeneous transform independently, applies the
//! adjoint by conjugating se(3) matrices, and obtains the wrench coadjoint as
//! the inverse transpose. It does not use Symtropy's production adjoint
//! formulas to construct its expected values.

use nalgebra::{Matrix3, Matrix4, SMatrix, SVector, Vector3};
use symtropy_math::{
    Bivector, BodyTwist3, BodyWrench3, Point, Rotor, Transform,
};

type Vector6 = SVector<f64, 6>;
type Matrix6 = SMatrix<f64, 6, 6>;

fn skew(v: &Vector3<f64>) -> Matrix3<f64> {
    Matrix3::new(
        0.0, -v[2], v[1],
        v[2], 0.0, -v[0],
        -v[1], v[0], 0.0,
    )
}

fn reference_rotation(phi: &Vector3<f64>) -> Matrix3<f64> {
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

fn as_bivector(vector: &Vector3<f64>) -> Bivector<3> {
    let mut value = Bivector::<3>::zero();
    value.set(0, 1, vector[2]);
    value.set(0, 2, -vector[1]);
    value.set(1, 2, vector[0]);
    value
}

fn as_vector(value: &Bivector<3>) -> Vector3<f64> {
    Vector3::new(value.get(1, 2), -value.get(0, 2), value.get(0, 1))
}

fn homogeneous(rotation: &Matrix3<f64>, translation: &Vector3<f64>) -> Matrix4<f64> {
    let mut transform = Matrix4::<f64>::identity();
    for row in 0..3 {
        for column in 0..3 {
            transform[(row, column)] = rotation[(row, column)];
        }
        transform[(row, 3)] = translation[row];
    }
    transform
}

/// The se(3) matrix for Symtropy's [linear, angular] coordinate order.
fn se3_hat(coordinates: &Vector6) -> Matrix4<f64> {
    let linear = Vector3::new(coordinates[0], coordinates[1], coordinates[2]);
    let angular = Vector3::new(coordinates[3], coordinates[4], coordinates[5]);
    let angular_hat = skew(&angular);
    let mut result = Matrix4::<f64>::zeros();
    for row in 0..3 {
        for column in 0..3 {
            result[(row, column)] = angular_hat[(row, column)];
        }
        result[(row, 3)] = linear[row];
    }
    result
}

/// Recover [linear, angular] from a matrix in se(3).
fn se3_vee(matrix: &Matrix4<f64>) -> Vector6 {
    Vector6::new(
        matrix[(0, 3)],
        matrix[(1, 3)],
        matrix[(2, 3)],
        matrix[(2, 1)],
        matrix[(0, 2)],
        matrix[(1, 0)],
    )
}

/// Build Ad_T column by column from T * hat(xi) * T^-1.
fn reference_adjoint(transform: &Matrix4<f64>) -> Matrix6 {
    let inverse = transform.try_inverse().expect("homogeneous transform is invertible");
    let mut adjoint = Matrix6::zeros();
    for column in 0..6 {
        let mut basis = Vector6::zeros();
        basis[column] = 1.0;
        let mapped = se3_vee(&(transform * se3_hat(&basis) * inverse));
        for row in 0..6 {
            adjoint[(row, column)] = mapped[row];
        }
    }
    adjoint
}

fn twist_coordinates(linear: &Vector3<f64>, angular: &Bivector<3>) -> Vector6 {
    let angular = as_vector(angular);
    Vector6::new(
        linear[0],
        linear[1],
        linear[2],
        angular[0],
        angular[1],
        angular[2],
    )
}

fn wrench_coordinates(force: &Vector3<f64>, torque: &Bivector<3>) -> Vector6 {
    let torque = as_vector(torque);
    Vector6::new(
        force[0],
        force[1],
        force[2],
        torque[0],
        torque[1],
        torque[2],
    )
}

fn assert_vector_close(actual: &Vector6, expected: &Vector6, label: &str) {
    let error = (actual - expected).norm();
    assert!(
        error < 3.0e-12,
        "{label} mismatch: actual={actual:?}, expected={expected:?}, error={error:e}"
    );
}

#[test]
fn se3_adjoint_and_coadjoint_match_independent_homogeneous_matrix_actions() {
    let rotation_vector = Vector3::new(0.31, -0.22, 0.47);
    let translation = Vector3::new(1.3, -0.7, 2.1);
    let rotation_matrix = reference_rotation(&rotation_vector);
    let homogeneous_transform = homogeneous(&rotation_matrix, &translation);
    let rotor = Rotor::<3>::from_matrix(rotation_matrix);
    let transform = Transform {
        translation: Point::new([translation[0], translation[1], translation[2]]),
        rotation: rotor,
    };

    let reference_adjoint = reference_adjoint(&homogeneous_transform);
    let reference_coadjoint = reference_adjoint
        .try_inverse()
        .expect("SE(3) adjoint is invertible")
        .transpose();

    let production_adjoint = transform
        .body_to_spatial_adjoint_matrix()
        .expect("valid pose has a finite adjoint");
    let production_coadjoint = transform
        .body_to_spatial_coadjoint_matrix()
        .expect("valid pose has a finite coadjoint");

    assert!(
        (production_adjoint - reference_adjoint).norm() < 3.0e-12,
        "production adjoint differs from homogeneous-matrix conjugation"
    );
    assert!(
        (production_coadjoint - reference_coadjoint).norm() < 3.0e-12,
        "production coadjoint differs from inverse-transpose dual action"
    );

    let body_linear = Vector3::new(0.3, -1.1, 0.7);
    let body_angular = as_bivector(&Vector3::new(0.41, -0.36, 0.27));
    let body_twist = BodyTwist3::new(body_linear, body_angular);
    let expected_spatial_twist =
        reference_adjoint * twist_coordinates(&body_linear, &body_angular);
    let actual_spatial_twist = transform
        .body_twist_to_spatial(&body_twist)
        .expect("valid body twist maps to a finite spatial twist");
    let actual_twist_coordinates = twist_coordinates(
        &actual_spatial_twist.linear,
        &actual_spatial_twist.angular,
    );
    assert_vector_close(
        &actual_twist_coordinates,
        &expected_spatial_twist,
        "SE(3) twist adjoint",
    );

    let body_force = Vector3::new(4.0, 1.5, -2.0);
    let body_torque = as_bivector(&Vector3::new(-0.5, 0.9, 1.2));
    let body_wrench = BodyWrench3::new(body_force, body_torque);
    let expected_spatial_wrench =
        reference_coadjoint * wrench_coordinates(&body_force, &body_torque);
    let actual_spatial_wrench = transform
        .body_wrench_to_spatial(&body_wrench)
        .expect("valid body wrench maps to a finite spatial wrench");
    let actual_wrench_coordinates = wrench_coordinates(
        &actual_spatial_wrench.force,
        &actual_spatial_wrench.torque,
    );
    assert_vector_close(
        &actual_wrench_coordinates,
        &expected_spatial_wrench,
        "SE(3) wrench coadjoint",
    );

    let body_power = body_twist.power_with(&body_wrench);
    let spatial_power = actual_spatial_twist.power_with(&actual_spatial_wrench);
    assert!(
        (body_power - spatial_power).abs() < 3.0e-12,
        "independent matrix transforms must preserve twist-wrench power"
    );
}

#[test]
fn se3_reference_exposes_translation_coupling_and_rejects_rotation_only_shortcut() {
    let rotation_vector = Vector3::new(-0.28, 0.61, 0.39);
    let translation = Vector3::new(1.7, -0.9, 0.8);
    let rotation = reference_rotation(&rotation_vector);
    let transform = homogeneous(&rotation, &translation);
    let adjoint = reference_adjoint(&transform);

    let mut rotation_only = Matrix6::zeros();
    for row in 0..3 {
        for column in 0..3 {
            rotation_only[(row, column)] = rotation[(row, column)];
            rotation_only[(row + 3, column + 3)] = rotation[(row, column)];
        }
    }

    assert!(
        (adjoint - rotation_only).norm() > 1.0,
        "nonzero translation and angular motion must produce adjoint coupling"
    );
}
