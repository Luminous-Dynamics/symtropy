#![forbid(unsafe_code)]

//! Three-dimensional rotational task-space capability primitives.
//!
//! This module is a source-independent analytic reference. It evaluates a
//! declared local 3x3 rotational Jacobian and stiffness map; it does not
//! implement a specific wrist/ankle geometry, a 6D mixed-unit spatial Jacobian,
//! redundant constraints, or physical actuator limits.

type Matrix3 = [[f64; 3]; 3];
type Vector3 = [f64; 3];

const RELATIVE_EPSILON: f64 = 1.0e-12;
const INVERSE_CONDITION_LIMIT: f64 = 1.0e8;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TaskSpaceMetrics {
    /// Singular values in descending order.
    pub singular_values: Vector3,
    pub condition_number: f64,
    /// Absolute determinant of the declared local rotational Jacobian.
    pub manipulability_volume: f64,
}

fn finite_matrix(matrix: Matrix3) -> bool {
    matrix.iter().flatten().all(|value| value.is_finite())
}

fn finite_vector(vector: Vector3) -> bool {
    vector.iter().all(|value| value.is_finite())
}

fn identity() -> Matrix3 {
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
}

fn transpose(matrix: Matrix3) -> Matrix3 {
    [
        [matrix[0][0], matrix[1][0], matrix[2][0]],
        [matrix[0][1], matrix[1][1], matrix[2][1]],
        [matrix[0][2], matrix[1][2], matrix[2][2]],
    ]
}

fn multiply(left: Matrix3, right: Matrix3) -> Matrix3 {
    let mut result = [[0.0; 3]; 3];
    for row in 0..3 {
        for col in 0..3 {
            for inner in 0..3 {
                result[row][col] += left[row][inner] * right[inner][col];
            }
        }
    }
    result
}

fn determinant(matrix: Matrix3) -> f64 {
    matrix[0][0] * (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1])
        - matrix[0][1] * (matrix[1][0] * matrix[2][2] - matrix[1][2] * matrix[2][0])
        + matrix[0][2] * (matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0])
}

fn matrix_max_abs(matrix: Matrix3) -> f64 {
    matrix
        .iter()
        .flatten()
        .fold(0.0_f64, |maximum, value| maximum.max(value.abs()))
}

fn inverse(matrix: Matrix3) -> Option<Matrix3> {
    if !finite_matrix(matrix) {
        return None;
    }

    let det = determinant(matrix);
    let scale = matrix_max_abs(matrix);
    if !det.is_finite()
        || !scale.is_finite()
        || scale == 0.0
        || det.abs() <= RELATIVE_EPSILON * scale.powi(3)
    {
        return None;
    }

    let adjugate = [
        [
            matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1],
            matrix[0][2] * matrix[2][1] - matrix[0][1] * matrix[2][2],
            matrix[0][1] * matrix[1][2] - matrix[0][2] * matrix[1][1],
        ],
        [
            matrix[1][2] * matrix[2][0] - matrix[1][0] * matrix[2][2],
            matrix[0][0] * matrix[2][2] - matrix[0][2] * matrix[2][0],
            matrix[0][2] * matrix[1][0] - matrix[0][0] * matrix[1][2],
        ],
        [
            matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0],
            matrix[0][1] * matrix[2][0] - matrix[0][0] * matrix[2][1],
            matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0],
        ],
    ];
    let result = adjugate.map(|row| row.map(|value| value / det));
    finite_matrix(result).then_some(result)
}

fn symmetric_eigenvalues(mut matrix: Matrix3) -> Option<Vector3> {
    if !finite_matrix(matrix) {
        return None;
    }

    let symmetry_scale = matrix_max_abs(matrix).max(1.0);
    for row in 0..3 {
        for col in (row + 1)..3 {
            if (matrix[row][col] - matrix[col][row]).abs() > RELATIVE_EPSILON * symmetry_scale {
                return None;
            }
            let average = 0.5 * (matrix[row][col] + matrix[col][row]);
            matrix[row][col] = average;
            matrix[col][row] = average;
        }
    }

    let scale = matrix_max_abs(matrix).max(1.0);
    for _ in 0..64 {
        let pairs = [(0, 1), (0, 2), (1, 2)];
        let (p, q) = pairs
            .iter()
            .copied()
            .max_by(|(a1, b1), (a2, b2)| {
                matrix[*a1][*b1].abs().total_cmp(&matrix[*a2][*b2].abs())
            })?;

        let apq = matrix[p][q];
        if apq.abs() <= RELATIVE_EPSILON * scale {
            break;
        }

        let tau = (matrix[q][q] - matrix[p][p]) / (2.0 * apq);
        let t = if tau >= 0.0 {
            1.0 / (tau + (1.0 + tau * tau).sqrt())
        } else {
            -1.0 / (-tau + (1.0 + tau * tau).sqrt())
        };
        let cosine = 1.0 / (1.0 + t * t).sqrt();
        let sine = t * cosine;

        let app = matrix[p][p];
        let aqq = matrix[q][q];
        matrix[p][p] = app - t * apq;
        matrix[q][q] = aqq + t * apq;
        matrix[p][q] = 0.0;
        matrix[q][p] = 0.0;

        for k in 0..3 {
            if k == p || k == q {
                continue;
            }
            let akp = matrix[k][p];
            let akq = matrix[k][q];
            let new_kp = cosine * akp - sine * akq;
            let new_kq = sine * akp + cosine * akq;
            matrix[k][p] = new_kp;
            matrix[p][k] = new_kp;
            matrix[k][q] = new_kq;
            matrix[q][k] = new_kq;
        }
    }

    let mut values = [matrix[0][0], matrix[1][1], matrix[2][2]];
    if !finite_vector(values) {
        return None;
    }
    values.sort_by(|left, right| right.total_cmp(left));
    Some(values)
}

/// Calculates singular values, condition number and local manipulability volume
/// for a declared 3x3 rotational Jacobian.
pub fn task_space_metrics(jacobian: Matrix3) -> Option<TaskSpaceMetrics> {
    if !finite_matrix(jacobian) {
        return None;
    }

    let gram = multiply(transpose(jacobian), jacobian);
    let eigenvalues = symmetric_eigenvalues(gram)?;
    let scale = eigenvalues[0].abs().max(1.0);
    if eigenvalues.iter().any(|value| *value < -RELATIVE_EPSILON * scale) {
        return None;
    }

    let singular_values = [
        eigenvalues[0].max(0.0).sqrt(),
        eigenvalues[1].max(0.0).sqrt(),
        eigenvalues[2].max(0.0).sqrt(),
    ];
    let sigma_max = singular_values[0];
    let sigma_min = singular_values[2];
    if !sigma_max.is_finite() || sigma_max <= 0.0 || !sigma_min.is_finite() || sigma_min <= 0.0 {
        return None;
    }

    let condition_number = sigma_max / sigma_min;
    let manipulability_volume = determinant(jacobian).abs();
    if !condition_number.is_finite() || !manipulability_volume.is_finite() {
        return None;
    }

    Some(TaskSpaceMetrics {
        singular_values,
        condition_number,
        manipulability_volume,
    })
}

/// Matrix inverse is permitted only when its Jacobian condition number is
/// at or below the declared software cutoff. The cutoff is not a physical
/// singularity/safety threshold.
pub fn well_conditioned_inverse(jacobian: Matrix3) -> Option<Matrix3> {
    let metrics = task_space_metrics(jacobian)?;
    if metrics.condition_number > INVERSE_CONDITION_LIMIT {
        return None;
    }
    inverse(jacobian)
}

fn skew(vector: Vector3) -> Matrix3 {
    [
        [0.0, -vector[2], vector[1]],
        [vector[2], 0.0, -vector[0]],
        [-vector[1], vector[0], 0.0],
    ]
}

fn trace(matrix: Matrix3) -> f64 {
    matrix[0][0] + matrix[1][1] + matrix[2][2]
}

/// Exponential map from a rotation vector in radians to SO(3).
pub fn so3_exp(rotation_vector_rad: Vector3) -> Option<Matrix3> {
    if !finite_vector(rotation_vector_rad) {
        return None;
    }

    let theta_squared = rotation_vector_rad.iter().map(|value| value * value).sum::<f64>();
    if !theta_squared.is_finite() {
        return None;
    }
    let theta = theta_squared.sqrt();
    let (a, b) = if theta < 1.0e-8 {
        let theta_fourth = theta_squared * theta_squared;
        (
            1.0 - theta_squared / 6.0 + theta_fourth / 120.0,
            0.5 - theta_squared / 24.0 + theta_fourth / 720.0,
        )
    } else {
        (theta.sin() / theta, (1.0 - theta.cos()) / theta_squared)
    };

    let k = skew(rotation_vector_rad);
    let k_squared = multiply(k, k);
    let mut result = identity();
    for row in 0..3 {
        for col in 0..3 {
            result[row][col] += a * k[row][col] + b * k_squared[row][col];
        }
    }
    is_rotation_matrix(result).then_some(result)
}

/// Checks orthonormality and right-handed determinant for a candidate SO(3) matrix.
pub fn is_rotation_matrix(matrix: Matrix3) -> bool {
    if !finite_matrix(matrix) {
        return false;
    }
    let orthogonality = multiply(transpose(matrix), matrix);
    let det = determinant(matrix);
    let tolerance = 1.0e-8;
    for row in 0..3 {
        for col in 0..3 {
            let expected = if row == col { 1.0 } else { 0.0 };
            if (orthogonality[row][col] - expected).abs() > tolerance {
                return false;
            }
        }
    }
    (det - 1.0).abs() <= tolerance
}

/// Principal logarithm map from SO(3) to a rotation vector in radians.
pub fn so3_log(rotation: Matrix3) -> Option<Vector3> {
    if !is_rotation_matrix(rotation) {
        return None;
    }

    let cosine = ((trace(rotation) - 1.0) * 0.5).clamp(-1.0, 1.0);
    let theta = cosine.acos();
    if !theta.is_finite() {
        return None;
    }

    let vee = [
        rotation[2][1] - rotation[1][2],
        rotation[0][2] - rotation[2][0],
        rotation[1][0] - rotation[0][1],
    ];

    if theta < 1.0e-7 {
        return Some([0.5 * vee[0], 0.5 * vee[1], 0.5 * vee[2]]);
    }

    if std::f64::consts::PI - theta < 1.0e-6 {
        let diagonal_axis = [
            ((rotation[0][0] + 1.0) * 0.5).max(0.0).sqrt(),
            ((rotation[1][1] + 1.0) * 0.5).max(0.0).sqrt(),
            ((rotation[2][2] + 1.0) * 0.5).max(0.0).sqrt(),
        ];
        let largest = if diagonal_axis[0] >= diagonal_axis[1]
            && diagonal_axis[0] >= diagonal_axis[2]
        {
            0
        } else if diagonal_axis[1] >= diagonal_axis[2] {
            1
        } else {
            2
        };

        let mut axis = [0.0; 3];
        match largest {
            0 if diagonal_axis[0] > 1.0e-8 => {
                axis[0] = diagonal_axis[0];
                axis[1] = (rotation[0][1] + rotation[1][0]) / (4.0 * axis[0]);
                axis[2] = (rotation[0][2] + rotation[2][0]) / (4.0 * axis[0]);
            }
            1 if diagonal_axis[1] > 1.0e-8 => {
                axis[1] = diagonal_axis[1];
                axis[0] = (rotation[0][1] + rotation[1][0]) / (4.0 * axis[1]);
                axis[2] = (rotation[1][2] + rotation[2][1]) / (4.0 * axis[1]);
            }
            2 if diagonal_axis[2] > 1.0e-8 => {
                axis[2] = diagonal_axis[2];
                axis[0] = (rotation[0][2] + rotation[2][0]) / (4.0 * axis[2]);
                axis[1] = (rotation[1][2] + rotation[2][1]) / (4.0 * axis[2]);
            }
            _ => return None,
        }

        let norm = axis.iter().map(|value| value * value).sum::<f64>().sqrt();
        if !norm.is_finite() || norm <= 0.0 {
            return None;
        }
        axis = axis.map(|value| value / norm);
        return Some(axis.map(|value| value * theta));
    }

    let scale = theta / (2.0 * theta.sin());
    let result = vee.map(|value| value * scale);
    finite_vector(result).then_some(result)
}

fn is_symmetric_positive_definite(matrix: Matrix3) -> bool {
    if !finite_matrix(matrix) {
        return false;
    }
    let scale = matrix_max_abs(matrix).max(1.0);
    for row in 0..3 {
        for col in (row + 1)..3 {
            if (matrix[row][col] - matrix[col][row]).abs() > RELATIVE_EPSILON * scale {
                return false;
            }
        }
    }

    let minor2 = matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0];
    matrix[0][0] > 0.0 && minor2 > 0.0 && determinant(matrix) > 0.0
}

/// For a locally square, nonsingular rotational map and small perturbations,
/// computes K_task = J^(-T) K_joint J^(-1).
pub fn task_space_stiffness(
    jacobian: Matrix3,
    joint_stiffness: Matrix3,
) -> Option<Matrix3> {
    if !is_symmetric_positive_definite(joint_stiffness) {
        return None;
    }
    let inverse_j = well_conditioned_inverse(jacobian)?;
    let result = multiply(multiply(transpose(inverse_j), joint_stiffness), inverse_j);
    (finite_matrix(result) && is_symmetric_positive_definite(result)).then_some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64, tolerance: f64, label: &str) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "{label}: expected {expected:.12e}, got {actual:.12e}, tolerance {tolerance:.3e}"
        );
    }

    fn close_matrix(actual: Matrix3, expected: Matrix3, tolerance: f64, label: &str) {
        for row in 0..3 {
            for col in 0..3 {
                close(actual[row][col], expected[row][col], tolerance, label);
            }
        }
    }

    #[test]
    fn field_001_identity_jacobian_is_isotropic() {
        let metrics = task_space_metrics(identity()).unwrap();
        close(metrics.singular_values[0], 1.0, 1.0e-12, "sigma max");
        close(metrics.singular_values[1], 1.0, 1.0e-12, "sigma middle");
        close(metrics.singular_values[2], 1.0, 1.0e-12, "sigma min");
        close(metrics.condition_number, 1.0, 1.0e-12, "condition");
        close(metrics.manipulability_volume, 1.0, 1.0e-12, "volume");
    }

    #[test]
    fn field_002_anisotropic_diagonal_jacobian_has_known_metrics() {
        let jacobian = [[1.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 3.0]];
        let metrics = task_space_metrics(jacobian).unwrap();
        assert_eq!(metrics.singular_values, [3.0, 2.0, 1.0]);
        close(metrics.condition_number, 3.0, 1.0e-12, "condition");
        close(metrics.manipulability_volume, 6.0, 1.0e-12, "volume");
    }

    #[test]
    fn field_003_left_rotation_preserves_singular_values_and_volume() {
        let rotation = so3_exp([0.3, -0.2, 0.4]).unwrap();
        let jacobian = [[1.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 3.0]];
        let rotated = multiply(rotation, jacobian);
        let metrics = task_space_metrics(rotated).unwrap();
        close(metrics.singular_values[0], 3.0, 1.0e-10, "rotated sigma max");
        close(metrics.singular_values[1], 2.0, 1.0e-10, "rotated sigma middle");
        close(metrics.singular_values[2], 1.0, 1.0e-10, "rotated sigma min");
        close(metrics.manipulability_volume, 6.0, 1.0e-10, "rotated volume");
    }

    #[test]
    fn field_004_singular_and_ill_conditioned_jacobians_fail_inverse_closed() {
        let singular = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.0]];
        assert!(task_space_metrics(singular).is_none());

        let ill = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0e-9]];
        let metrics = task_space_metrics(ill).unwrap();
        close(metrics.condition_number, 1.0e9, 1.0e-6, "ill condition");
        assert!(well_conditioned_inverse(ill).is_none());
    }

    #[test]
    fn field_005_so3_exp_log_round_trip_general_rotation() {
        let vector = [0.3, -0.2, 0.4];
        let rotation = so3_exp(vector).unwrap();
        let recovered = so3_log(rotation).unwrap();
        for axis in 0..3 {
            close(recovered[axis], vector[axis], 1.0e-10, "SO3 round trip");
        }
    }

    #[test]
    fn field_006_so3_small_angle_uses_stable_limit() {
        let vector = [1.0e-10, -2.0e-10, 3.0e-10];
        let rotation = so3_exp(vector).unwrap();
        let recovered = so3_log(rotation).unwrap();
        for axis in 0..3 {
            close(recovered[axis], vector[axis], 1.0e-12, "small SO3 round trip");
        }
    }

    #[test]
    fn field_007_so3_near_pi_rotation_has_finite_principal_log() {
        let theta = std::f64::consts::PI - 1.0e-7;
        let vector = [theta, 0.0, 0.0];
        let rotation = so3_exp(vector).unwrap();
        let recovered = so3_log(rotation).unwrap();
        close(recovered[0].abs(), theta, 1.0e-6, "near-pi angle");
        close(recovered[1], 0.0, 1.0e-6, "near-pi y");
        close(recovered[2], 0.0, 1.0e-6, "near-pi z");
    }

    #[test]
    fn field_008_so3_rejects_non_rotation_matrices() {
        let scaling = [[2.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        assert!(so3_log(scaling).is_none());
        assert!(!is_rotation_matrix(scaling));
    }

    #[test]
    fn field_009_identity_map_preserves_joint_stiffness() {
        let stiffness = [[100.0, 5.0, 0.0], [5.0, 80.0, 2.0], [0.0, 2.0, 60.0]];
        let result = task_space_stiffness(identity(), stiffness).unwrap();
        close_matrix(result, stiffness, 1.0e-10, "identity stiffness map");
    }

    #[test]
    fn field_010_rotated_map_produces_symmetric_positive_definite_stiffness() {
        let rotation = so3_exp([0.2, 0.3, -0.1]).unwrap();
        let stiffness = [[100.0, 0.0, 0.0], [0.0, 80.0, 0.0], [0.0, 0.0, 60.0]];
        let result = task_space_stiffness(rotation, stiffness).unwrap();
        assert!(is_symmetric_positive_definite(result));
        close(trace(multiply(transpose(rotation), result)), 240.0, 1.0e-8, "stiffness trace invariant");
    }

    #[test]
    fn field_011_non_spd_stiffness_is_rejected() {
        let indefinite = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0]];
        assert!(task_space_stiffness(identity(), indefinite).is_none());
    }

    #[test]
    fn field_012_non_finite_inputs_fail_closed() {
        let invalid_j = [[1.0, 0.0, 0.0], [0.0, f64::NAN, 0.0], [0.0, 0.0, 1.0]];
        assert!(task_space_metrics(invalid_j).is_none());
        assert!(so3_exp([0.0, f64::INFINITY, 0.0]).is_none());
    }
}
