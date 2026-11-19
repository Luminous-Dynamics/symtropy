//! Independent known-answer oracles for the source-faithful J1 vertical-guide task contract.
 //!
//! These tests cover only equations transcribed from arXiv:2609.22795v1 and
 //! explicitly synthetic values. They do not implement the source dynamics,
 //! contact integrator, actuator models, optimizer, or a physical robot.

fn assert_close(actual: f64, expected: f64, label: &str) {
    let tolerance = 1.0e-12_f64.max(expected.abs() * 1.0e-12);
    assert!(
        (actual - expected).abs() <= tolerance,
        "{label}: expected {expected:.15e}, got {actual:.15e}, tolerance {tolerance:.3e}"
    );
}

fn guide_coordinates(l_shank: f64, l_thigh: f64, q1: f64, q2: f64) -> (f64, f64) {
    let included = q1 - q2;
    (
        l_shank * q1.cos() - l_thigh * included.cos(),
        l_shank * q1.sin() - l_thigh * included.sin(),
    )
}

fn guide_jacobians(
    l_shank: f64,
    l_thigh: f64,
    q1: f64,
    q2: f64,
) -> ([f64; 2], [f64; 2]) {
    let included = q1 - q2;
    (
        [
            -l_shank * q1.sin() + l_thigh * included.sin(),
            -l_thigh * included.sin(),
        ],
        [
            l_shank * q1.cos() - l_thigh * included.cos(),
            l_thigh * included.cos(),
        ],
    )
}

fn generalized_torque(tau_hip: f64, tau_knee: f64) -> [f64; 2] {
    [-tau_hip, tau_hip + tau_knee]
}

fn bezier_fifth_order(s: f64, coefficients: [f64; 6]) -> (f64, f64) {
    let choose = [1.0, 5.0, 10.0, 10.0, 5.0, 1.0];
    let mut sum_basis = 0.0;
    let mut alpha = 0.0;
    for (index, coefficient) in coefficients.iter().enumerate() {
        let basis = choose[index]
            * (1.0 - s).powi(5 - index as i32)
            * s.powi(index as i32);
        sum_basis += basis;
        alpha += *coefficient * basis;
    }
    (sum_basis, alpha)
}

fn stopping_distance(
    omega_plus: f64,
    delay_s: f64,
    effective_inertia: f64,
    braking_torque: f64,
    position_margin: f64,
) -> Option<f64> {
    if !omega_plus.is_finite()
        || !delay_s.is_finite()
        || !effective_inertia.is_finite()
        || !braking_torque.is_finite()
        || !position_margin.is_finite()
        || omega_plus < 0.0
        || delay_s < 0.0
        || effective_inertia < 0.0
        || braking_torque <= 0.0
        || position_margin < 0.0
    {
        return None;
    }
    Some(
        omega_plus * delay_s
            + effective_inertia * omega_plus.powi(2) / (2.0 * braking_torque)
            + position_margin,
    )
}

fn contact_admissible(normal_force: f64, friction: f64, tangential_force: f64) -> bool {
    normal_force.is_finite()
        && friction.is_finite()
        && tangential_force.is_finite()
        && normal_force >= 0.0
        && friction >= 0.0
        && tangential_force.abs() <= friction * normal_force
}

#[test]
fn j1_task_001_equal_segment_vertical_guide_pose_and_jacobians() {
    let (x_h, y_h) = guide_coordinates(0.5, 0.5, 0.25, 0.5);
    let (a, j_y) = guide_jacobians(0.5, 0.5, 0.25, 0.5);

    assert_close(x_h, 0.0, "guide constraint x_h [m]");
    assert_close(y_h, 0.24740395925452294, "slider height y_h [m]");
    assert_close(a[0], -0.24740395925452294, "guide Jacobian A_1");
    assert_close(a[1], 0.12370197962726147, "guide Jacobian A_2");
    assert_close(j_y[0], 0.0, "slider Jacobian J_y1");
    assert_close(j_y[1], 0.48445621085532237, "slider Jacobian J_y2");
}

#[test]
fn j1_task_002_guide_tangent_velocity_and_slider_velocity() {
    let (a, j_y) = guide_jacobians(0.5, 0.5, 0.25, 0.5);
    let q_dot = [0.1_f64, 0.2_f64];

    let constraint_rate = a[0] * q_dot[0] + a[1] * q_dot[1];
    let slider_velocity = j_y[0] * q_dot[0] + j_y[1] * q_dot[1];

    assert_close(constraint_rate, 0.0, "guide tangent condition A*q_dot");
    assert_close(slider_velocity, 0.09689124217106448, "slider velocity [m/s]");
}

#[test]
fn j1_task_003_generalized_torque_mapping_preserves_virtual_work() {
    let tau_hip = 10.0_f64;
    let tau_knee = 5.0_f64;
    let q_dot = [0.1_f64, 0.2_f64];
    let generalized = generalized_torque(tau_hip, tau_knee);

    let mapped_power = q_dot[0] * generalized[0] + q_dot[1] * generalized[1];
    let joint_power = tau_hip * (-q_dot[0] + q_dot[1]) + tau_knee * q_dot[1];

    assert_close(generalized[0], -10.0, "generalized torque axis 1 [N m]");
    assert_close(generalized[1], 15.0, "generalized torque axis 2 [N m]");
    assert_close(mapped_power, 2.0, "generalized power [W]");
    assert_close(joint_power, mapped_power, "virtual-work equality [W]");
}

#[test]
fn j1_task_004_bezier_partition_of_unity_and_linear_controls() {
    let coefficients = [0.0_f64, 0.2, 0.4, 0.6, 0.8, 1.0];
    let (basis_sum, alpha) = bezier_fifth_order(0.5, coefficients);

    assert_close(basis_sum, 1.0, "fifth-order Bezier basis sum");
    assert_close(alpha, 0.5, "linear control-point interpolation at s=0.5");
}

#[test]
fn j1_task_005_predictive_stopping_distance_known_answer() {
    let distance = stopping_distance(3.0, 0.01, 0.02, 5.0, 0.02)
        .expect("declared nonnegative stopping inputs must be admitted");
    assert_close(distance, 0.068, "predictive stopping distance [rad]");
}

#[test]
fn j1_task_006_contact_cone_and_invalid_state_controls() {
    assert!(contact_admissible(100.0, 0.6, 40.0));
    assert!(!contact_admissible(100.0, 0.6, 61.0));
    assert!(!contact_admissible(-1.0, 0.6, 0.0));
    assert!(!contact_admissible(100.0, -0.1, 0.0));
    assert!(!contact_admissible(100.0, 0.6, f64::NAN));
}

#[test]
fn j1_task_007_stopping_distance_rejects_invalid_parameters() {
    assert!(stopping_distance(3.0, 0.01, 0.02, 0.0, 0.02).is_none());
    assert!(stopping_distance(-0.1, 0.01, 0.02, 5.0, 0.02).is_none());
    assert!(stopping_distance(3.0, f64::INFINITY, 0.02, 5.0, 0.02).is_none());
}
