//! Topology-agnostic two-DoF task-space oracle for J2 wrist comparisons.
//!
//! The reference checks local Jacobian, virtual-work and stiffness
//! transformations. It does not implement any specific serial/parallel
//! wrist geometry. All test values are synthetic and are not extracted from
//! the cited wrist prototypes.

const CONDITION_LIMIT: f64 = 1.0e8;
const RELATIVE_EPSILON: f64 = 1.0e-12;
const MIN_EIGENVALUE_RATIO: f64 = 1.0e-20;

type Matrix2 = [[f64; 2]; 2];
type Vector2 = [f64; 2];

fn all_finite_matrix(matrix: Matrix2) -> bool {
    matrix.iter().flatten().all(|value| value.is_finite())
}

fn determinant(matrix: Matrix2) -> f64 {
    matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0]
}

fn transpose(matrix: Matrix2) -> Matrix2 {
    [
        [matrix[0][0], matrix[1][0]],
        [matrix[0][1], matrix[1][1]],
    ]
}

fn multiply(left: Matrix2, right: Matrix2) -> Matrix2 {
    [
        [
            left[0][0] * right[0][0] + left[0][1] * right[1][0],
            left[0][0] * right[0][1] + left[0][1] * right[1][1],
        ],
        [
            left[1][0] * right[0][0] + left[1][1] * right[1][0],
            left[1][0] * right[0][1] + left[1][1] * right[1][1],
        ],
    ]
}

fn multiply_vector(matrix: Matrix2, vector: Vector2) -> Option<Vector2> {
    if !all_finite_matrix(matrix) || vector.iter().any(|value| !value.is_finite()) {
        return None;
    }
    let result = [
        matrix[0][0] * vector[0] + matrix[0][1] * vector[1],
        matrix[1][0] * vector[0] + matrix[1][1] * vector[1],
    ];
    result.iter().all(|value| value.is_finite()).then_some(result)
}

fn jacobian_singular_values(matrix: Matrix2) -> Option<(f64, f64)> {
    if !all_finite_matrix(matrix) {
        return None;
    }

    // Eigenvalues of J^T J are obtained from trace and determinant.
    let trace = matrix.iter().flatten().map(|value| value.powi(2)).sum::<f64>();
    let det = determinant(matrix);
    let discriminant_squared = trace.powi(2) - 4.0 * det.powi(2);
    if !trace.is_finite() || !det.is_finite() || !discriminant_squared.is_finite() {
        return None;
    }

    let discriminant = discriminant_squared.max(0.0).sqrt();
    let lambda_max = (trace + discriminant) / 2.0;
    if !lambda_max.is_finite() || lambda_max <= 0.0 {
        return None;
    }
    // Avoid catastrophic cancellation in (trace - discriminant) / 2 for
    // ill-conditioned matrices: lambda_max * lambda_min = det(J)^2.
    let lambda_min = det.powi(2) / lambda_max;
    if !lambda_min.is_finite() || lambda_min <= MIN_EIGENVALUE_RATIO * lambda_max {
        return None;
    }

    let sigma_max = lambda_max.sqrt();
    let sigma_min = lambda_min.sqrt();
    Some((sigma_max, sigma_min))
}

fn jacobian_condition_number(matrix: Matrix2) -> Option<f64> {
    let (sigma_max, sigma_min) = jacobian_singular_values(matrix)?;
    let condition = sigma_max / sigma_min;
    condition.is_finite().then_some(condition)
}

fn jacobian_manipulability_area(matrix: Matrix2) -> Option<f64> {
    if !all_finite_matrix(matrix) {
        return None;
    }
    let area = determinant(matrix).abs();
    area.is_finite().then_some(area)
}

fn inverse(matrix: Matrix2) -> Option<Matrix2> {
    let condition = jacobian_condition_number(matrix)?;
    if condition > CONDITION_LIMIT {
        return None;
    }

    let det = determinant(matrix);
    if !det.is_finite() || det == 0.0 {
        return None;
    }

    let result = [
        [matrix[1][1] / det, -matrix[0][1] / det],
        [-matrix[1][0] / det, matrix[0][0] / det],
    ];
    all_finite_matrix(result).then_some(result)
}

fn is_symmetric_positive_definite(matrix: Matrix2) -> bool {
    if !all_finite_matrix(matrix) {
        return false;
    }
    let symmetry_scale = matrix[0][1].abs().max(matrix[1][0].abs()).max(1.0);
    if (matrix[0][1] - matrix[1][0]).abs() > RELATIVE_EPSILON * symmetry_scale {
        return false;
    }
    matrix[0][0] > 0.0
        && matrix[1][1] > 0.0
        && determinant(matrix) > 0.0
}

fn task_wrench_to_joint_effort(jacobian: Matrix2, task_wrench: Vector2) -> Option<Vector2> {
    multiply_vector(transpose(jacobian), task_wrench)
}

fn task_stiffness_from_joint_stiffness(
    jacobian: Matrix2,
    joint_stiffness: Matrix2,
) -> Option<Matrix2> {
    if !is_symmetric_positive_definite(joint_stiffness) {
        return None;
    }
    let jacobian_inverse = inverse(jacobian)?;
    let result = multiply(
        multiply(transpose(jacobian_inverse), joint_stiffness),
        jacobian_inverse,
    );
    (all_finite_matrix(result) && is_symmetric_positive_definite(result)).then_some(result)
}

fn quadratic_energy_j(x: Vector2, stiffness: Matrix2) -> Option<f64> {
    if x.iter().any(|value| !value.is_finite())
        || !is_symmetric_positive_definite(stiffness)
    {
        return None;
    }
    let kx = multiply_vector(stiffness, x)?;
    let energy = 0.5 * (x[0] * kx[0] + x[1] * kx[1]);
    (energy.is_finite() && energy >= 0.0).then_some(energy)
}

fn assert_close(actual: f64, expected: f64, label: &str) {
    let tolerance = 1.0e-10_f64.max(expected.abs() * 1.0e-10);
    assert!(
        (actual - expected).abs() <= tolerance,
        "{label}: expected {expected:.15e}, got {actual:.15e}, tolerance {tolerance:.3e}"
    );
}

#[test]
fn wrist_001_identity_jacobian_isotropic_reference() {
    let jacobian = [[1.0, 0.0], [0.0, 1.0]];
    let (sigma_max, sigma_min) =
        jacobian_singular_values(jacobian).expect("identity Jacobian must be nonsingular");
    let condition = jacobian_condition_number(jacobian)
        .expect("identity Jacobian must have a finite condition number");
    let area = jacobian_manipulability_area(jacobian)
        .expect("identity Jacobian must have finite manipulability area");

    assert_close(sigma_max, 1.0, "identity sigma max");
    assert_close(sigma_min, 1.0, "identity sigma min");
    assert_close(condition, 1.0, "identity condition number");
    assert_close(area, 1.0, "identity manipulability area");
}

#[test]
fn wrist_002_synthetic_coupled_jacobian_conditioning() {
    let jacobian = [[1.0, 0.0], [0.5, 0.8660254037844386]];
    let (sigma_max, sigma_min) =
        jacobian_singular_values(jacobian).expect("synthetic Jacobian must be nonsingular");
    let condition = jacobian_condition_number(jacobian)
        .expect("synthetic Jacobian must be well conditioned");
    let area = jacobian_manipulability_area(jacobian)
        .expect("synthetic Jacobian must have finite area");

    assert_close(sigma_max, 1.224744871391589, "synthetic sigma max");
    assert_close(sigma_min, 0.7071067811865475, "synthetic sigma min");
    assert_close(condition, 1.7320508075688772, "synthetic condition number");
    assert_close(area, 0.8660254037844386, "synthetic manipulability area");
}

#[test]
fn wrist_003_virtual_work_maps_task_wrench_to_joint_effort() {
    let jacobian = [[1.0, 0.0], [0.5, 0.8660254037844386]];
    let task_wrench = [12.0, -4.0];
    let joint_velocity = [0.2, -0.1];

    let task_velocity = multiply_vector(jacobian, joint_velocity)
        .expect("finite synthetic velocity must map");
    let joint_effort = task_wrench_to_joint_effort(jacobian, task_wrench)
        .expect("finite task wrench must map");

    let joint_power =
        joint_effort[0] * joint_velocity[0] + joint_effort[1] * joint_velocity[1];
    let task_power = task_wrench[0] * task_velocity[0] + task_wrench[1] * task_velocity[1];

    assert_close(task_velocity[0], 0.2, "task velocity x");
    assert_close(task_velocity[1], 0.01339745962155614, "task velocity y");
    assert_close(joint_effort[0], 10.0, "joint effort 1");
    assert_close(joint_effort[1], -3.4641016151377544, "joint effort 2");
    assert_close(joint_power, 2.3464101615137753, "joint power");
    assert_close(task_power, joint_power, "task/joint virtual-work equality");
}

#[test]
fn wrist_004_joint_stiffness_transforms_to_task_space() {
    let jacobian = [[1.0, 0.0], [0.5, 0.8660254037844386]];
    let joint_stiffness = [[100.0, 10.0], [10.0, 60.0]];
    let task_stiffness = task_stiffness_from_joint_stiffness(jacobian, joint_stiffness)
        .expect("positive-definite stiffness and nonsingular Jacobian must map");

    assert_close(task_stiffness[0][0], 108.45299461620749, "task stiffness xx");
    assert_close(task_stiffness[0][1], -28.45299461620749, "task stiffness xy");
    assert_close(task_stiffness[1][0], -28.45299461620749, "task stiffness yx");
    assert_close(task_stiffness[1][1], 80.0, "task stiffness yy");
    assert!(is_symmetric_positive_definite(task_stiffness));
}

#[test]
fn wrist_005_task_stiffness_energy_is_positive() {
    let jacobian = [[1.0, 0.0], [0.5, 0.8660254037844386]];
    let joint_stiffness = [[100.0, 10.0], [10.0, 60.0]];
    let task_stiffness = task_stiffness_from_joint_stiffness(jacobian, joint_stiffness)
        .expect("valid synthetic stiffness transform must succeed");
    let energy = quadratic_energy_j([0.1, -0.2], task_stiffness)
        .expect("valid displacement and SPD stiffness must yield energy");
    assert_close(energy, 2.711324865405188, "task-space stored energy [J]");
}

#[test]
fn wrist_006_singular_and_ill_conditioned_jacobians_are_rejected_for_inversion() {
    let singular = [[1.0, 2.0], [2.0, 4.0]];
    assert!(jacobian_singular_values(singular).is_none());
    assert!(inverse(singular).is_none());

    let ill_conditioned = [[1.0, 0.0], [0.0, 1.0e-9]];
    let condition = jacobian_condition_number(ill_conditioned)
        .expect("ill-conditioned but resolvable matrix should report its condition");
    assert_close(condition, 1.0e9, "ill-conditioned Jacobian number");
    assert!(inverse(ill_conditioned).is_none());
}

#[test]
fn wrist_007_non_spd_or_non_symmetric_stiffness_is_rejected() {
    let jacobian = [[1.0, 0.0], [0.0, 1.0]];
    assert!(task_stiffness_from_joint_stiffness(
        jacobian,
        [[1.0, 0.0], [0.0, -1.0]],
    )
    .is_none());
    assert!(task_stiffness_from_joint_stiffness(
        jacobian,
        [[1.0, 0.5], [0.0, 1.0]],
    )
    .is_none());
}

#[test]
fn wrist_008_invalid_and_non_finite_inputs_fail_closed() {
    let non_finite = [[1.0, f64::NAN], [0.0, 1.0]];
    assert!(jacobian_singular_values(non_finite).is_none());
    assert!(multiply_vector([[1.0, 0.0], [0.0, 1.0]], [0.0, f64::INFINITY]).is_none());
    assert!(task_wrench_to_joint_effort(
        [[1.0, 0.0], [0.0, 1.0]],
        [1.0, f64::NAN],
    )
    .is_none());
    assert!(quadratic_energy_j([1.0, f64::NAN], [[1.0, 0.0], [0.0, 1.0]]).is_none());
}
