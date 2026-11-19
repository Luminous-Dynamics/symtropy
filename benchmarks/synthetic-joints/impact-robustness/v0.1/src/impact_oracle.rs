//! Independent analytic oracles for two distinct impact-robustness models.
//!
//! R1: series-elastic actuator gearbox-interface peak torque and maximum
//! stiffness bounds (Shu et al., IEEE RA-L 2026, DOI
//! 10.1109/LRA.2026.3710324).
//! R2: QDD normalized impact-robustness and use-case gear-ratio transition
//! (Shu et al., accepted Humanoids 2026 contribution).
//!
//! All test inputs are synthetic. These tests do not reproduce either paper's
//! complete dynamics simulation or any physical actuator.

const PI: f64 = std::f64::consts::PI;

fn positive_finite(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

fn nonnegative_finite(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn natural_frequency_rad_s(
    stiffness_nm_per_rad: f64,
    gear_ratio: f64,
    rotor_inertia_kg_m2: f64,
    link_inertia_kg_m2: f64,
) -> Option<f64> {
    if !positive_finite(stiffness_nm_per_rad)
        || !positive_finite(gear_ratio)
        || !positive_finite(rotor_inertia_kg_m2)
        || !positive_finite(link_inertia_kg_m2)
    {
        return None;
    }

    let inverse_reflected_rotor = 1.0 / (gear_ratio.powi(2) * rotor_inertia_kg_m2);
    let inverse_link = 1.0 / link_inertia_kg_m2;
    let omega = (stiffness_nm_per_rad * (inverse_reflected_rotor + inverse_link)).sqrt();
    positive_finite(omega).then_some(omega)
}

fn finite_contact_peak_torque_nm(
    stiffness_nm_per_rad: f64,
    gear_ratio: f64,
    rotor_inertia_kg_m2: f64,
    link_inertia_kg_m2: f64,
    impact_impulse_nm_s: f64,
    contact_duration_s: f64,
    operational_torque_nm: f64,
) -> Option<f64> {
    if !nonnegative_finite(impact_impulse_nm_s)
        || !nonnegative_finite(contact_duration_s)
        || !operational_torque_nm.is_finite()
    {
        return None;
    }

    let omega = natural_frequency_rad_s(
        stiffness_nm_per_rad,
        gear_ratio,
        rotor_inertia_kg_m2,
        link_inertia_kg_m2,
    )?;
    let scaled_duration = contact_duration_s * omega;

    // The finite-contact closed form is only admitted for Delta * omega < pi.
    if !scaled_duration.is_finite() || scaled_duration >= PI {
        return None;
    }

    let denominator = PI.powi(2) - scaled_duration.powi(2);
    if !positive_finite(denominator) {
        return None;
    }

    let finite_duration_factor = PI.powi(2) * (scaled_duration / 2.0).cos() / denominator;
    let peak = stiffness_nm_per_rad * impact_impulse_nm_s / (link_inertia_kg_m2 * omega)
        * finite_duration_factor
        + operational_torque_nm;

    peak.is_finite().then_some(peak)
}

fn short_contact_peak_torque_nm(
    stiffness_nm_per_rad: f64,
    gear_ratio: f64,
    rotor_inertia_kg_m2: f64,
    link_inertia_kg_m2: f64,
    impact_impulse_nm_s: f64,
    operational_torque_nm: f64,
) -> Option<f64> {
    if !positive_finite(stiffness_nm_per_rad)
        || !positive_finite(gear_ratio)
        || !positive_finite(rotor_inertia_kg_m2)
        || !positive_finite(link_inertia_kg_m2)
        || !nonnegative_finite(impact_impulse_nm_s)
        || !operational_torque_nm.is_finite()
    {
        return None;
    }

    let gear_squared = gear_ratio.powi(2);
    let numerator = rotor_inertia_kg_m2 * stiffness_nm_per_rad * gear_squared;
    let denominator =
        gear_squared * rotor_inertia_kg_m2 * link_inertia_kg_m2 + link_inertia_kg_m2.powi(2);
    if !positive_finite(numerator) || !positive_finite(denominator) {
        return None;
    }

    let peak = (numerator / denominator).sqrt() * impact_impulse_nm_s + operational_torque_nm;
    peak.is_finite().then_some(peak)
}

fn maximum_stiffness_from_impulse_nm_per_rad(
    link_inertia_kg_m2: f64,
    rotor_inertia_kg_m2: f64,
    gear_ratio: f64,
    maximum_output_torque_nm: f64,
    maximum_operational_torque_nm: f64,
    maximum_impact_impulse_nm_s: f64,
) -> Option<f64> {
    if !positive_finite(link_inertia_kg_m2)
        || !positive_finite(rotor_inertia_kg_m2)
        || !positive_finite(gear_ratio)
        || !positive_finite(maximum_output_torque_nm)
        || !maximum_operational_torque_nm.is_finite()
        || !positive_finite(maximum_impact_impulse_nm_s)
    {
        return None;
    }

    let torque_margin = maximum_output_torque_nm - maximum_operational_torque_nm;
    if !positive_finite(torque_margin) {
        return None;
    }

    let inertia_term = 1.0 / link_inertia_kg_m2 + 1.0 / (gear_ratio.powi(2) * rotor_inertia_kg_m2);
    let stiffness = link_inertia_kg_m2.powi(2)
        * inertia_term
        * (torque_margin / maximum_impact_impulse_nm_s).powi(2);

    positive_finite(stiffness).then_some(stiffness)
}

fn maximum_stiffness_from_speed_nm_per_rad(
    link_inertia_kg_m2: f64,
    rotor_inertia_kg_m2: f64,
    gear_ratio: f64,
    maximum_output_torque_nm: f64,
    maximum_operational_torque_nm: f64,
    maximum_link_speed_rad_s: f64,
) -> Option<f64> {
    if !positive_finite(link_inertia_kg_m2)
        || !positive_finite(rotor_inertia_kg_m2)
        || !positive_finite(gear_ratio)
        || !positive_finite(maximum_output_torque_nm)
        || !maximum_operational_torque_nm.is_finite()
        || !positive_finite(maximum_link_speed_rad_s)
    {
        return None;
    }

    let torque_margin = maximum_output_torque_nm - maximum_operational_torque_nm;
    if !positive_finite(torque_margin) {
        return None;
    }

    let inertia_term = 1.0 / link_inertia_kg_m2 + 1.0 / (gear_ratio.powi(2) * rotor_inertia_kg_m2);
    let stiffness = inertia_term * (torque_margin / maximum_link_speed_rad_s).powi(2);

    positive_finite(stiffness).then_some(stiffness)
}

fn minimum_spring_energy_j(
    link_inertia_kg_m2: f64,
    rotor_inertia_kg_m2: f64,
    gear_ratio: f64,
    impact_impulse_nm_s: f64,
) -> Option<f64> {
    if !positive_finite(link_inertia_kg_m2)
        || !positive_finite(rotor_inertia_kg_m2)
        || !positive_finite(gear_ratio)
        || !nonnegative_finite(impact_impulse_nm_s)
    {
        return None;
    }

    let inertia_term = 1.0 / link_inertia_kg_m2 + 1.0 / (gear_ratio.powi(2) * rotor_inertia_kg_m2);
    let energy = impact_impulse_nm_s.powi(2) / (2.0 * link_inertia_kg_m2.powi(2) * inertia_term);

    energy.is_finite().then_some(energy)
}

// R2 uses motor inertia and gear ratio directly. Do not substitute the R1
// gearbox-interface torque bound for this normalized energy-based metric.
fn reflected_motor_inertia_kg_m2(motor_inertia_kg_m2: f64, gear_ratio: f64) -> Option<f64> {
    if !positive_finite(motor_inertia_kg_m2) || !positive_finite(gear_ratio) {
        return None;
    }
    let reflected = motor_inertia_kg_m2 * gear_ratio.powi(2);
    positive_finite(reflected).then_some(reflected)
}

fn motor_to_link_inertia_ratio(
    motor_inertia_kg_m2: f64,
    gear_ratio: f64,
    link_inertia_kg_m2: f64,
) -> Option<f64> {
    if !positive_finite(link_inertia_kg_m2) {
        return None;
    }
    Some(reflected_motor_inertia_kg_m2(motor_inertia_kg_m2, gear_ratio)? / link_inertia_kg_m2)
}

fn qdd_gear_ratio_transition(
    motor_inertia_kg_m2: f64,
    link_inertia_kg_m2: f64,
    transition_gamma: f64,
) -> Option<f64> {
    if !positive_finite(motor_inertia_kg_m2)
        || !positive_finite(link_inertia_kg_m2)
        || !positive_finite(transition_gamma)
    {
        return None;
    }
    let ratio = (transition_gamma * link_inertia_kg_m2 / motor_inertia_kg_m2).sqrt();
    positive_finite(ratio).then_some(ratio)
}

fn qdd_normalized_impact_robustness(
    motor_inertia_kg_m2: f64,
    link_inertia_kg_m2: f64,
    gear_ratio: f64,
    maximum_torque_nm: f64,
    stiffness_nm_per_rad: f64,
    reference_impulse_nm_s: f64,
) -> Option<f64> {
    if !positive_finite(link_inertia_kg_m2)
        || !positive_finite(maximum_torque_nm)
        || !positive_finite(stiffness_nm_per_rad)
        || !positive_finite(reference_impulse_nm_s)
    {
        return None;
    }

    let gamma = motor_to_link_inertia_ratio(motor_inertia_kg_m2, gear_ratio, link_inertia_kg_m2)?;
    let potential_energy = maximum_torque_nm.powi(2) / (2.0 * stiffness_nm_per_rad);
    let impact_kinetic_energy = reference_impulse_nm_s.powi(2) / (2.0 * link_inertia_kg_m2);
    let robustness =
        (potential_energy / impact_kinetic_energy).sqrt() * ((gamma + 1.0) / gamma).sqrt();

    positive_finite(robustness).then_some(robustness)
}

fn assert_close(actual: f64, expected: f64, label: &str) {
    let tolerance = 1.0e-10_f64.max(expected.abs() * 1.0e-10);
    assert!(
        (actual - expected).abs() <= tolerance,
        "{label}: expected {expected:.15e}, got {actual:.15e}, tolerance {tolerance:.3e}"
    );
}

#[test]
fn impact_001_natural_frequency_and_contact_parameter() {
    let omega = natural_frequency_rad_s(250.0, 5.0, 0.0004, 0.02)
        .expect("positive synthetic inertias and stiffness must be admitted");
    assert_close(omega, 193.64916731037084, "natural frequency [rad/s]");
    let contact_parameter = 0.001 * omega / PI;
    assert_close(
        contact_parameter,
        0.061640444406149984,
        "dimensionless finite-contact parameter",
    );
}

#[test]
fn impact_002_finite_contact_peak_torque_known_answer() {
    let peak = finite_contact_peak_torque_nm(250.0, 5.0, 0.0004, 0.02, 0.2, 0.001, 5.0)
        .expect("synthetic contact duration is within the declared domain");
    assert_close(peak, 17.89848475076076, "finite-contact peak torque [N m]");
}

#[test]
fn impact_003_finite_contact_converges_to_short_contact_limit() {
    let finite = finite_contact_peak_torque_nm(250.0, 5.0, 0.0004, 0.02, 0.2, 1.0e-9, 5.0)
        .expect("small positive contact duration must be admitted");
    let short = short_contact_peak_torque_nm(250.0, 5.0, 0.0004, 0.02, 0.2, 5.0)
        .expect("positive synthetic parameters must be admitted");
    assert_close(finite, short, "finite-duration zero-contact limit [N m]");
}

#[test]
fn impact_004_finite_contact_rejects_out_of_domain_duration() {
    let omega = natural_frequency_rad_s(250.0, 5.0, 0.0004, 0.02)
        .expect("positive synthetic parameters must be admitted");
    let boundary_duration = PI / omega;
    assert!(
        finite_contact_peak_torque_nm(250.0, 5.0, 0.0004, 0.02, 0.2, boundary_duration, 5.0,)
            .is_none(),
        "the finite-contact expression must reject Delta*omega >= pi"
    );
}

#[test]
fn impact_005_impulse_and_speed_stiffness_bounds_are_equivalent() {
    let from_impulse = maximum_stiffness_from_impulse_nm_per_rad(0.02, 0.0004, 5.0, 80.0, 5.0, 0.2)
        .expect("positive torque margin and impulse must be admitted");
    let from_speed = maximum_stiffness_from_speed_nm_per_rad(0.02, 0.0004, 5.0, 80.0, 5.0, 10.0)
        .expect("positive torque margin and speed must be admitted");
    assert_close(from_impulse, 8437.5, "impulse maximum stiffness [N m/rad]");
    assert_close(from_speed, 8437.5, "speed maximum stiffness [N m/rad]");
    assert_close(
        from_impulse,
        from_speed,
        "equivalent stiffness formulations",
    );
}

#[test]
fn impact_006_minimum_spring_energy_known_answer() {
    let energy = minimum_spring_energy_j(0.02, 0.0004, 5.0, 0.2)
        .expect("positive synthetic inertias and impulse must be admitted");
    assert_close(energy, 1.0 / 3.0, "minimum spring energy [J]");
}

#[test]
fn impact_007_qdd_reflected_inertia_and_use_case_gear_boundary() {
    let reflected = reflected_motor_inertia_kg_m2(0.0004, 10.0)
        .expect("positive motor inertia and ratio must be admitted");
    let gamma = motor_to_link_inertia_ratio(0.0004, 10.0, 0.02)
        .expect("positive inertias and ratio must be admitted");
    let boundary = qdd_gear_ratio_transition(0.0004, 0.02, 1.18)
        .expect("positive synthetic use-case parameters must be admitted");

    assert_close(reflected, 0.04, "reflected motor inertia [kg m^2]");
    assert_close(gamma, 2.0, "motor-to-link inertia ratio");
    assert_close(
        boundary,
        7.681145747868608,
        "use-case gear-ratio transition",
    );
}

#[test]
fn impact_008_qdd_energy_normalized_robustness_known_answer() {
    let robustness = qdd_normalized_impact_robustness(0.0004, 0.02, 10.0, 20.0, 1000.0, 0.03)
        .expect("positive synthetic QDD parameters must be admitted");
    assert_close(
        robustness,
        3.651483716701107,
        "normalized QDD impact-robustness index",
    );
}

#[test]
fn impact_009_invalid_and_non_finite_inputs_fail_closed() {
    assert!(natural_frequency_rad_s(0.0, 5.0, 0.0004, 0.02).is_none());
    assert!(natural_frequency_rad_s(250.0, f64::INFINITY, 0.0004, 0.02).is_none());
    assert!(finite_contact_peak_torque_nm(250.0, 5.0, 0.0004, 0.02, -0.1, 0.001, 5.0,).is_none());
    assert!(short_contact_peak_torque_nm(250.0, 0.0, 0.0004, 0.02, 0.2, 5.0,).is_none());
    assert!(minimum_spring_energy_j(0.02, 0.0004, 5.0, f64::NAN).is_none());
    assert!(reflected_motor_inertia_kg_m2(0.0004, -5.0).is_none());
    assert!(qdd_gear_ratio_transition(0.0004, 0.02, 0.0).is_none());
    assert!(qdd_normalized_impact_robustness(0.0004, 0.02, 10.0, 20.0, 1000.0, 0.0,).is_none());
}

#[test]
fn impact_010_stiffness_bounds_reject_zero_torque_margin() {
    assert!(
        maximum_stiffness_from_impulse_nm_per_rad(0.02, 0.0004, 5.0, 80.0, 80.0, 0.2,).is_none()
    );
    assert!(
        maximum_stiffness_from_speed_nm_per_rad(0.02, 0.0004, 5.0, 80.0, 81.0, 10.0,).is_none()
    );
}
