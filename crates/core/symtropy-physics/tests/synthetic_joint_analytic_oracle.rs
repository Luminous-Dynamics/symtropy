//! Independent analytic known-answer tests for synthetic musculoskeletal joint fixtures.
//!
//! These tests intentionally do not call Symtropy's hinge solver or controller.
//! They are equation-level oracles for the synthetic fixture corpus only.
//! They do not validate a physical mechanism, actuator, material, or safety case.

fn assert_close(actual: f64, expected: f64, label: &str) {
    let tolerance = 1.0e-12_f64.max(expected.abs() * 1.0e-12);
    assert!(
        (actual - expected).abs() <= tolerance,
        "{label}: expected {expected:.15e}, got {actual:.15e}, tolerance {tolerance:.3e}"
    );
}

#[test]
fn sj_analytic_001_linear_torsion_spring() {
    let stiffness = 12.0_f64; // N m / rad
    let angle = 0.25_f64; // rad
    let angular_velocity = 2.0_f64; // rad / s

    let restoring_torque = -stiffness * angle;
    let stored_energy = 0.5 * stiffness * angle.powi(2);
    let actuator_input_power = stiffness * angle * angular_velocity;

    assert_close(restoring_torque, -3.0, "restoring torque [N m]");
    assert_close(stored_energy, 0.375, "stored energy [J]");
    assert_close(actuator_input_power, 6.0, "actuator input power [W]");
}

#[test]
fn sj_analytic_002_antagonistic_tendon_pair() {
    let moment_arm = 0.04_f64; // m
    let flexor_tension = 110.0_f64; // N
    let extensor_tension = 50.0_f64; // N
    let each_tendon_stiffness = 8_000.0_f64; // N / m

    let net_torque = moment_arm * (flexor_tension - extensor_tension);
    let local_joint_stiffness = 2.0 * each_tendon_stiffness * moment_arm.powi(2);

    assert_close(net_torque, 2.4, "net joint torque [N m]");
    assert_close(local_joint_stiffness, 25.6, "local stiffness [N m/rad]");
}

#[test]
fn sj_analytic_003_sensorized_tendon_extension() {
    let tension = 100.0_f64; // N
    let axial_stiffness = 20_000.0_f64; // N / m

    let extension = tension / axial_stiffness;
    let stored_energy = 0.5 * tension * extension;

    assert_close(extension, 0.005, "tendon extension [m]");
    assert_close(stored_energy, 0.25, "tendon stored energy [J]");
}

#[test]
fn sj_analytic_004_gear_reflected_rotor_inertia() {
    let ratio = 10.0_f64; // motor speed / output speed
    let motor_inertia = 0.0002_f64; // kg m^2
    let motor_torque = 1.0_f64; // N m
    let motor_speed = 100.0_f64; // rad / s

    let output_reflected_inertia = ratio.powi(2) * motor_inertia;
    let ideal_output_torque = ratio * motor_torque;
    let ideal_output_speed = motor_speed / ratio;
    let input_power = motor_torque * motor_speed;
    let ideal_output_power = ideal_output_torque * ideal_output_speed;

    assert_close(output_reflected_inertia, 0.02, "reflected inertia [kg m^2]");
    assert_close(ideal_output_torque, 10.0, "ideal output torque [N m]");
    assert_close(ideal_output_speed, 10.0, "ideal output speed [rad/s]");
    assert_close(input_power, 100.0, "input power [W]");
    assert_close(ideal_output_power, 100.0, "ideal output power [W]");
}

#[test]
fn sj_analytic_005_signed_joint_work_and_regeneration_boundary() {
    let duration = 0.5_f64; // s
    let angular_velocity = 3.0_f64; // rad / s

    let positive_torque = 2.0_f64; // N m
    let positive_power = positive_torque * angular_velocity;
    let positive_work = positive_power * duration;

    let negative_torque = -2.0_f64; // N m
    let negative_power = negative_torque * angular_velocity;
    let negative_work = negative_power * duration;

    assert_close(positive_power, 6.0, "positive mechanical power [W]");
    assert_close(positive_work, 3.0, "positive mechanical work [J]");
    assert_close(negative_power, -6.0, "negative mechanical power [W]");
    assert_close(negative_work, -3.0, "negative mechanical work [J]");

    // Negative mechanical work is not an electrical-energy measurement.
    // The electrical regeneration boundary requires its own voltage/current
    // measurement and is intentionally not inferred by this oracle.
}

#[test]
fn sj_analytic_006_coupled_two_axis_stiffness_matrix() {
    let k11 = 4.0_f64;
    let k12 = 1.0_f64;
    let k21 = 1.0_f64;
    let k22 = 9.0_f64;
    let q1 = 0.1_f64; // rad
    let q2 = -0.2_f64; // rad

    let tau1 = k11 * q1 + k12 * q2;
    let tau2 = k21 * q1 + k22 * q2;
    let determinant = k11 * k22 - k12 * k21;
    let stored_energy = 0.5 * (q1 * tau1 + q2 * tau2);
    let positive_definite = k11 > 0.0 && determinant > 0.0;

    assert_close(tau1, 0.2, "axis-1 torque [N m]");
    assert_close(tau2, -1.7, "axis-2 torque [N m]");
    assert_close(determinant, 35.0, "stiffness determinant");
    assert_close(stored_energy, 0.18, "coupled stored energy [J]");
    assert!(positive_definite, "declared symmetric stiffness must be positive definite");
}


#[test]
fn sj_analytic_007_unilateral_tendon_slack_and_engagement() {
    let axial_stiffness = 50_000.0_f64; // N / m
    let total_extension = 0.005_f64; // m
    let slack_length = 0.002_f64; // m

    let engaged_extension = (total_extension - slack_length).max(0.0);
    let tension = axial_stiffness * engaged_extension;
    let stored_energy = 0.5 * axial_stiffness * engaged_extension.powi(2);

    assert_close(engaged_extension, 0.003, "engaged extension [m]");
    assert_close(tension, 150.0, "tendon tension [N]");
    assert_close(stored_energy, 0.225, "tendon stored energy [J]");

    let slack_case_extension = 0.0015_f64;
    let slack_case_engaged = (slack_case_extension - slack_length).max(0.0);
    assert_close(axial_stiffness * slack_case_engaged, 0.0, "slack tendon tension [N]");
    assert_close(
        0.5 * axial_stiffness * slack_case_engaged.powi(2),
        0.0,
        "slack tendon stored energy [J]",
    );
}

#[test]
fn sj_analytic_008_cubic_strain_stiffening_spring() {
    let linear_stiffness = 8.0_f64; // N m / rad
    let cubic_stiffness = 40.0_f64; // N m / rad^3
    let angle = 0.4_f64; // rad

    let restoring_torque =
        -(linear_stiffness * angle + cubic_stiffness * angle.powi(3));
    let stored_energy = 0.5 * linear_stiffness * angle.powi(2)
        + 0.25 * cubic_stiffness * angle.powi(4);
    let incremental_stiffness =
        linear_stiffness + 3.0 * cubic_stiffness * angle.powi(2);

    assert_close(restoring_torque, -5.76, "nonlinear restoring torque [N m]");
    assert_close(stored_energy, 0.896, "nonlinear spring energy [J]");
    assert_close(incremental_stiffness, 27.2, "incremental stiffness [N m/rad]");
}

#[test]
fn sj_analytic_009_transmission_backlash_deadband() {
    let torsional_stiffness = 100.0_f64; // N m / rad
    let total_backlash = 0.02_f64; // rad
    let relative_twist = 0.05_f64; // rad
    let half_deadband = 0.5 * total_backlash;

    let engaged_twist = relative_twist.signum()
        * (relative_twist.abs() - half_deadband).max(0.0);
    let transmitted_torque = torsional_stiffness * engaged_twist;
    let stored_energy = 0.5 * torsional_stiffness * engaged_twist.powi(2);

    assert_close(engaged_twist, 0.04, "engaged transmission twist [rad]");
    assert_close(transmitted_torque, 4.0, "transmitted torque [N m]");
    assert_close(stored_energy, 0.08, "transmission elastic energy [J]");

    let negative_twist = -0.05_f64;
    let negative_engaged_twist = negative_twist.signum()
        * (negative_twist.abs() - half_deadband).max(0.0);
    assert_close(
        torsional_stiffness * negative_engaged_twist,
        -4.0,
        "negative-direction transmitted torque [N m]",
    );

    let within_deadband = 0.005_f64;
    let unloaded_twist =
        within_deadband.signum() * (within_deadband.abs() - half_deadband).max(0.0);
    assert_close(
        torsional_stiffness * unloaded_twist,
        0.0,
        "within-deadband transmitted torque [N m]",
    );
}

#[test]
fn sj_analytic_010_coulomb_viscous_friction_is_dissipative() {
    let coulomb_friction = 0.8_f64; // N m
    let viscous_coefficient = 0.1_f64; // N m s / rad
    let angular_velocity = -2.0_f64; // rad / s

    let friction_torque = -(coulomb_friction
        + viscous_coefficient * angular_velocity.abs())
        * angular_velocity.signum();
    let mechanical_port_power = friction_torque * angular_velocity;
    let dissipation_rate = -mechanical_port_power;

    assert_close(friction_torque, 1.0, "friction torque [N m]");
    assert_close(mechanical_port_power, -2.0, "friction port power [W]");
    assert_close(dissipation_rate, 2.0, "friction dissipation [W]");
    assert!(dissipation_rate >= 0.0, "friction must not inject mechanical energy");

    // At zero speed, static friction is a set-valued reaction, not sign(0)=0.
    let static_capacity = 1.2_f64; // N m
    let applied_torque = 0.7_f64; // N m
    assert!(applied_torque.abs() <= static_capacity);
    let static_reaction = -applied_torque;
    let angular_velocity_at_rest = 0.0_f64;
    assert_close(static_reaction, -0.7, "static friction reaction [N m]");
    assert_close(angular_velocity_at_rest, 0.0, "held angular velocity [rad/s]");
}

#[test]
fn sj_analytic_011_corrected_compliance_deflection_uncertainty() {
    let effective_stiffness = 800.0_f64; // N m / rad
    let corrected_deflection = 0.003_f64; // rad
    let sigma_deflection = 0.0001_f64; // rad
    let sigma_stiffness = 10.0_f64; // N m / rad

    let estimated_torque = effective_stiffness * corrected_deflection;
    let variance = (corrected_deflection * sigma_stiffness).powi(2)
        + (effective_stiffness * sigma_deflection).powi(2);
    let sigma_torque = variance.sqrt();
    let relative_sigma_percent = 100.0 * sigma_torque / estimated_torque.abs();

    assert_close(estimated_torque, 2.4, "embedded torque estimate [N m]");
    assert_close(
        sigma_torque,
        0.08544003745317531,
        "first-order torque standard uncertainty [N m]",
    );
    assert_close(
        relative_sigma_percent,
        3.5600015605489714,
        "relative torque standard uncertainty [%]",
    );
}


#[test]
fn sj_analytic_012_dual_encoder_deflection_and_torque_uncertainty() {
    let gear_ratio = 10.0_f64; // motor angle / output angle
    let motor_encoder_angle = 12.6_f64; // rad
    let output_encoder_angle = 1.2_f64; // rad
    let calibrated_zero_offset = 0.01_f64; // rad

    let motor_angle_at_output = motor_encoder_angle / gear_ratio;
    let raw_deflection = motor_angle_at_output - output_encoder_angle;
    let corrected_deflection = raw_deflection - calibrated_zero_offset;

    let stiffness = 100.0_f64; // N m / rad
    let estimated_torque = stiffness * corrected_deflection;

    // Independent standard uncertainties: motor encoder, output encoder,
    // zero-offset calibration and effective transmission stiffness.
    let sigma_motor_encoder = 0.002_f64; // rad
    let sigma_output_encoder = 0.0003_f64; // rad
    let sigma_zero_offset = 0.0004_f64; // rad
    let sigma_stiffness = 2.0_f64; // N m / rad

    let deflection_variance = (sigma_motor_encoder / gear_ratio).powi(2)
        + sigma_output_encoder.powi(2)
        + sigma_zero_offset.powi(2);
    let torque_variance = (corrected_deflection * sigma_stiffness).powi(2)
        + stiffness.powi(2) * deflection_variance;
    let sigma_torque = torque_variance.sqrt();
    let relative_sigma_percent = 100.0 * sigma_torque / estimated_torque.abs();

    assert_close(motor_angle_at_output, 1.26, "motor angle mapped to output [rad]");
    assert_close(raw_deflection, 0.06, "raw differential encoder deflection [rad]");
    assert_close(corrected_deflection, 0.05, "zero-corrected encoder deflection [rad]");
    assert_close(estimated_torque, 5.0, "dual-encoder torque estimate [N m]");
    assert_close(
        sigma_torque,
        0.11357816691600547,
        "dual-encoder torque standard uncertainty [N m]",
    );
    assert_close(
        relative_sigma_percent,
        2.2715633383201093,
        "dual-encoder relative torque standard uncertainty [%]",
    );
}
