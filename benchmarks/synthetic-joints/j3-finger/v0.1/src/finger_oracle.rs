//! Exact directional-force feasibility oracle for a four-DoF tendon-driven finger.
//!
//! The model follows the equilibrium structure in PACT (arXiv:2611.05302):
//! J(q)^T f = R T + e_AA * tau_AA, with pull-only tendon bounds and a
//! separate bidirectional abduction/adduction actuator.
//!
//! The implementation solves one force direction exactly for a full-rank
//! 4x4 routing matrix. It does not compute PACT's complete 3D force-polytope
//! volume-coverage score and does not model tendon friction or bandwidth.

const RELATIVE_PIVOT_EPSILON: f64 = 1.0e-12;
const CONSTRAINT_TOLERANCE: f64 = 1.0e-9;

type Matrix4 = [[f64; 4]; 4];
type Matrix3x4 = [[f64; 4]; 3];
type Vector4 = [f64; 4];
type Vector3 = [f64; 3];

#[derive(Debug, Clone, Copy)]
struct Inequality2 {
    alpha: f64,
    auxiliary_torque: f64,
    bound: f64,
}

#[derive(Debug, Clone, Copy)]
struct DirectionalCapacity {
    force_n: f64,
    tendon_tensions_n: Vector4,
    abduction_torque_nm: f64,
}

fn finite_matrix4(matrix: Matrix4) -> bool {
    matrix.iter().flatten().all(|value| value.is_finite())
}

fn finite_matrix3x4(matrix: Matrix3x4) -> bool {
    matrix.iter().flatten().all(|value| value.is_finite())
}

fn inverse4(matrix: Matrix4) -> Option<Matrix4> {
    if !finite_matrix4(matrix) {
        return None;
    }

    let scale = matrix.iter().flatten().fold(0.0_f64, |maximum, value| {
        maximum.max(value.abs())
    });
    if !scale.is_finite() || scale == 0.0 {
        return None;
    }

    let mut augmented = [[0.0_f64; 8]; 4];
    for row in 0..4 {
        for col in 0..4 {
            augmented[row][col] = matrix[row][col];
        }
        augmented[row][row + 4] = 1.0;
    }

    for pivot_col in 0..4 {
        let mut pivot_row = pivot_col;
        for row in (pivot_col + 1)..4 {
            if augmented[row][pivot_col].abs() > augmented[pivot_row][pivot_col].abs() {
                pivot_row = row;
            }
        }

        let pivot = augmented[pivot_row][pivot_col];
        if !pivot.is_finite() || pivot.abs() <= RELATIVE_PIVOT_EPSILON * scale {
            return None;
        }

        if pivot_row != pivot_col {
            augmented.swap(pivot_row, pivot_col);
        }

        let pivot_value = augmented[pivot_col][pivot_col];
        for col in 0..8 {
            augmented[pivot_col][col] /= pivot_value;
        }

        for row in 0..4 {
            if row == pivot_col {
                continue;
            }
            let factor = augmented[row][pivot_col];
            for col in 0..8 {
                augmented[row][col] -= factor * augmented[pivot_col][col];
            }
        }
    }

    let mut result = [[0.0_f64; 4]; 4];
    for row in 0..4 {
        for col in 0..4 {
            result[row][col] = augmented[row][col + 4];
        }
    }
    finite_matrix4(result).then_some(result)
}

fn multiply4_vector(matrix: Matrix4, vector: Vector4) -> Option<Vector4> {
    if !finite_matrix4(matrix) || vector.iter().any(|value| !value.is_finite()) {
        return None;
    }
    let mut result = [0.0_f64; 4];
    for row in 0..4 {
        for col in 0..4 {
            result[row] += matrix[row][col] * vector[col];
        }
    }
    result.iter().all(|value| value.is_finite()).then_some(result)
}

fn dot4(left: Vector4, right: Vector4) -> f64 {
    (0..4).map(|index| left[index] * right[index]).sum()
}

fn constraints_satisfied(
    constraints: &[Inequality2],
    alpha: f64,
    auxiliary_torque: f64,
) -> bool {
    if !alpha.is_finite() || !auxiliary_torque.is_finite() {
        return false;
    }
    constraints.iter().all(|constraint| {
        let lhs = constraint.alpha * alpha
            + constraint.auxiliary_torque * auxiliary_torque;
        let tolerance = CONSTRAINT_TOLERANCE
            * 1.0_f64
                .max(lhs.abs())
                .max(constraint.bound.abs())
                .max((constraint.alpha * alpha).abs())
                .max((constraint.auxiliary_torque * auxiliary_torque).abs());
        lhs <= constraint.bound + tolerance
    })
}

fn directional_force_capacity_n(
    fingertip_jacobian_m: Matrix3x4,
    routing_matrix_m: Matrix4,
    unit_force_direction: Vector3,
    tendon_max_tensions_n: Vector4,
    maximum_abduction_torque_nm: f64,
) -> Option<DirectionalCapacity> {
    if !finite_matrix3x4(fingertip_jacobian_m)
        || unit_force_direction.iter().any(|value| !value.is_finite())
        || tendon_max_tensions_n
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
        || !maximum_abduction_torque_nm.is_finite()
        || maximum_abduction_torque_nm < 0.0
    {
        return None;
    }

    let direction_norm = unit_force_direction.iter().map(|value| value.powi(2)).sum::<f64>().sqrt();
    if !direction_norm.is_finite() || (direction_norm - 1.0).abs() > 1.0e-9 {
        return None;
    }

    let routing_inverse = inverse4(routing_matrix_m)?;

    // b = J(q)^T d. Desired joint torque is alpha * b.
    let mut task_torque_direction = [0.0_f64; 4];
    for joint in 0..4 {
        for axis in 0..3 {
            task_torque_direction[joint] +=
                fingertip_jacobian_m[axis][joint] * unit_force_direction[axis];
        }
    }
    if task_torque_direction.iter().all(|value| *value == 0.0) {
        return None;
    }

    // From alpha*b = R*T + e_AA*tau_AA:
    // T = alpha*(R^-1*b) - tau_AA*(R^-1*e_AA).
    // e_AA is the first joint-coordinate basis vector.
    let alpha_coefficients = multiply4_vector(routing_inverse, task_torque_direction)?;
    let auxiliary_coefficients = [
        -routing_inverse[0][0],
        -routing_inverse[1][0],
        -routing_inverse[2][0],
        -routing_inverse[3][0],
    ];

    let mut constraints = Vec::with_capacity(11);
    for index in 0..4 {
        // T_i >= 0  <=> -c_i*alpha - d_i*tau <= 0.
        constraints.push(Inequality2 {
            alpha: -alpha_coefficients[index],
            auxiliary_torque: -auxiliary_coefficients[index],
            bound: 0.0,
        });
        // T_i <= T_i,max.
        constraints.push(Inequality2 {
            alpha: alpha_coefficients[index],
            auxiliary_torque: auxiliary_coefficients[index],
            bound: tendon_max_tensions_n[index],
        });
    }

    // Bidirectional A/A actuator and nonnegative force magnitude.
    constraints.push(Inequality2 {
        alpha: 0.0,
        auxiliary_torque: 1.0,
        bound: maximum_abduction_torque_nm,
    });
    constraints.push(Inequality2 {
        alpha: 0.0,
        auxiliary_torque: -1.0,
        bound: maximum_abduction_torque_nm,
    });
    constraints.push(Inequality2 {
        alpha: -1.0,
        auxiliary_torque: 0.0,
        bound: 0.0,
    });

    // Enumerate vertices of the bounded two-variable feasible polygon.
    // The variables are force scale alpha and auxiliary A/A torque.
    let mut best: Option<(f64, f64)> = None;
    if constraints_satisfied(&constraints, 0.0, 0.0) {
        best = Some((0.0, 0.0));
    }

    for first in 0..constraints.len() {
        for second in (first + 1)..constraints.len() {
            let a = constraints[first];
            let b = constraints[second];
            let determinant =
                a.alpha * b.auxiliary_torque - b.alpha * a.auxiliary_torque;
            let determinant_scale = (a.alpha * b.auxiliary_torque).abs()
                + (b.alpha * a.auxiliary_torque).abs();
            if !determinant.is_finite()
                || determinant.abs()
                    <= RELATIVE_PIVOT_EPSILON * determinant_scale.max(f64::MIN_POSITIVE)
            {
                continue;
            }

            let alpha =
                (a.bound * b.auxiliary_torque - b.bound * a.auxiliary_torque) / determinant;
            let auxiliary_torque =
                (a.alpha * b.bound - b.alpha * a.bound) / determinant;

            if constraints_satisfied(&constraints, alpha, auxiliary_torque)
                && best.is_none_or(|(best_alpha, _)| alpha > best_alpha)
            {
                best = Some((alpha, auxiliary_torque));
            }
        }
    }

    let (force_n, abduction_torque_nm) = best?;
    let tendon_tensions_n = [
        alpha_coefficients[0] * force_n + auxiliary_coefficients[0] * abduction_torque_nm,
        alpha_coefficients[1] * force_n + auxiliary_coefficients[1] * abduction_torque_nm,
        alpha_coefficients[2] * force_n + auxiliary_coefficients[2] * abduction_torque_nm,
        alpha_coefficients[3] * force_n + auxiliary_coefficients[3] * abduction_torque_nm,
    ];

    let reconstructed_torque = [
        routing_matrix_m[0][0] * tendon_tensions_n[0]
            + routing_matrix_m[0][1] * tendon_tensions_n[1]
            + routing_matrix_m[0][2] * tendon_tensions_n[2]
            + routing_matrix_m[0][3] * tendon_tensions_n[3]
            + abduction_torque_nm,
        routing_matrix_m[1][0] * tendon_tensions_n[0]
            + routing_matrix_m[1][1] * tendon_tensions_n[1]
            + routing_matrix_m[1][2] * tendon_tensions_n[2]
            + routing_matrix_m[1][3] * tendon_tensions_n[3],
        routing_matrix_m[2][0] * tendon_tensions_n[0]
            + routing_matrix_m[2][1] * tendon_tensions_n[1]
            + routing_matrix_m[2][2] * tendon_tensions_n[2]
            + routing_matrix_m[2][3] * tendon_tensions_n[3],
        routing_matrix_m[3][0] * tendon_tensions_n[0]
            + routing_matrix_m[3][1] * tendon_tensions_n[1]
            + routing_matrix_m[3][2] * tendon_tensions_n[2]
            + routing_matrix_m[3][3] * tendon_tensions_n[3],
    ];
    let expected_torque = task_torque_direction.map(|value| value * force_n);
    for joint in 0..4 {
        let tolerance =
            CONSTRAINT_TOLERANCE * 1.0_f64.max(expected_torque[joint].abs());
        if (reconstructed_torque[joint] - expected_torque[joint]).abs() > tolerance {
            return None;
        }
    }

    for index in 0..4 {
        let tension = tendon_tensions_n[index];
        let tolerance = CONSTRAINT_TOLERANCE
            * 1.0_f64.max(tendon_max_tensions_n[index]);
        if tension < -tolerance || tension > tendon_max_tensions_n[index] + tolerance {
            return None;
        }
    }

    Some(DirectionalCapacity {
        force_n,
        tendon_tensions_n,
        abduction_torque_nm,
    })
}

fn tendon_length_rates_m_s(routing_matrix_m: Matrix4, joint_velocity_rad_s: Vector4) -> Option<Vector4> {
    if !finite_matrix4(routing_matrix_m)
        || joint_velocity_rad_s.iter().any(|value| !value.is_finite())
    {
        return None;
    }

    // l_dot = -R^T * q_dot.
    let mut rates = [0.0_f64; 4];
    for tendon in 0..4 {
        for joint in 0..4 {
            rates[tendon] -= routing_matrix_m[joint][tendon] * joint_velocity_rad_s[joint];
        }
    }
    rates.iter().all(|value| value.is_finite()).then_some(rates)
}

fn assert_close(actual: f64, expected: f64, label: &str) {
    let tolerance = 1.0e-10_f64.max(expected.abs() * 1.0e-10);
    assert!(
        (actual - expected).abs() <= tolerance,
        "{label}: expected {expected:.15e}, got {actual:.15e}, tolerance {tolerance:.3e}"
    );
}

fn identity4() -> Matrix4 {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

fn identity_fingertip_jacobian() -> Matrix3x4 {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
    ]
}

#[test]
fn finger_001_source_pact_routing_matrix_is_full_rank() {
    // Source values from PACT Table IV converted from mm to m.
    let routing = [
        [0.0030, -0.0030, 0.0, 0.0],
        [0.0069, -0.0030, -0.0080, 0.0056],
        [-0.0034, -0.0030, 0.0, 0.0046],
        [-0.0069, 0.0, 0.0, 0.0031],
    ];
    let inverse = inverse4(routing).expect("source routing map must have full rank");
    for row in 0..4 {
        for col in 0..4 {
            let reconstructed = (0..4)
                .map(|index| routing[row][index] * inverse[index][col])
                .sum::<f64>();
            assert_close(
                reconstructed,
                if row == col { 1.0 } else { 0.0 },
                "PACT routing matrix inverse identity",
            );
        }
    }
}

#[test]
fn finger_002_directional_force_capacity_respects_pull_only_bounds() {
    let result = directional_force_capacity_n(
        identity_fingertip_jacobian(),
        identity4(),
        [1.0, 0.0, 0.0],
        [10.0, 10.0, 10.0, 10.0],
        2.0,
    )
    .expect("synthetic bounded force direction should be feasible");

    assert_close(result.force_n, 12.0, "maximum directional fingertip force [N]");
    assert_close(result.tendon_tensions_n[0], 10.0, "tendon 1 tension [N]");
    assert_close(result.tendon_tensions_n[1], 0.0, "tendon 2 tension [N]");
    assert_close(result.tendon_tensions_n[2], 0.0, "tendon 3 tension [N]");
    assert_close(result.tendon_tensions_n[3], 0.0, "tendon 4 tension [N]");
    assert_close(result.abduction_torque_nm, 2.0, "bidirectional A/A actuator torque [N m]");
}

#[test]
fn finger_003_exact_equilibrium_is_reconstructed_at_optimum() {
    let jacobian = identity_fingertip_jacobian();
    let routing = identity4();
    let direction = [1.0, 0.0, 0.0];
    let result = directional_force_capacity_n(
        jacobian,
        routing,
        direction,
        [10.0, 10.0, 10.0, 10.0],
        2.0,
    )
    .expect("synthetic bounded force direction should be feasible");

    let joint_torque = [
        result.tendon_tensions_n[0] + result.abduction_torque_nm,
        result.tendon_tensions_n[1],
        result.tendon_tensions_n[2],
        result.tendon_tensions_n[3],
    ];
    let expected = [
        result.force_n * direction[0],
        result.force_n * direction[1],
        result.force_n * direction[2],
        0.0,
    ];
    for joint in 0..4 {
        assert_close(joint_torque[joint], expected[joint], "force/joint equilibrium");
    }
}

#[test]
fn finger_004_tendon_velocity_and_power_conserve_virtual_work() {
    let routing = identity4();
    let tensions = [10.0, 20.0, 30.0, 40.0];
    let joint_velocity = [0.1, 0.2, -0.1, 0.3];
    let aa_torque = 2.0;
    let tendon_rates = tendon_length_rates_m_s(routing, joint_velocity)
        .expect("finite synthetic joint velocity must map to tendon rates");

    let joint_torque = [
        tensions[0] + aa_torque,
        tensions[1],
        tensions[2],
        tensions[3],
    ];
    let joint_power = dot4(joint_torque, joint_velocity);
    let tendon_power = -(0..4)
        .map(|index| tensions[index] * tendon_rates[index])
        .sum::<f64>();
    let total_actuator_power = tendon_power + aa_torque * joint_velocity[0];

    assert_close(joint_power, 14.2, "joint power [W]");
    assert_close(total_actuator_power, joint_power, "actuator/joint virtual-work equality");
}

#[test]
fn finger_005_rank_deficient_and_unreachable_directions_fail_closed() {
    let singular_routing = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 2.0, 0.0],
    ];
    assert!(inverse4(singular_routing).is_none());

    let unreachable_jacobian = [[0.0; 4]; 3];
    assert!(directional_force_capacity_n(
        unreachable_jacobian,
        identity4(),
        [1.0, 0.0, 0.0],
        [10.0; 4],
        2.0,
    )
    .is_none());
}

#[test]
fn finger_006_invalid_bounds_and_non_unit_direction_are_rejected() {
    assert!(directional_force_capacity_n(
        identity_fingertip_jacobian(),
        identity4(),
        [2.0, 0.0, 0.0],
        [10.0; 4],
        2.0,
    )
    .is_none());
    assert!(directional_force_capacity_n(
        identity_fingertip_jacobian(),
        identity4(),
        [1.0, 0.0, 0.0],
        [10.0, -1.0, 10.0, 10.0],
        2.0,
    )
    .is_none());
    assert!(directional_force_capacity_n(
        identity_fingertip_jacobian(),
        identity4(),
        [1.0, 0.0, 0.0],
        [10.0; 4],
        f64::NAN,
    )
    .is_none());
}
