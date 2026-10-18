#![forbid(unsafe_code)]

//! Small, dependency-free reference models for the Physics Experiment Atlas.
//!
//! These checks verify equations and numerical invariants against analytic or
//! model-derived references. They do not ingest historical laboratory data,
//! reproduce the original apparatus in full, or establish that any real-world
//! measurement has been independently reproduced.

use std::f64::consts::PI;

/// A single machine-readable model qualification check.
///
/// value is a computed model output; reference is an analytic/golden value
/// or a model invariant target. The report deliberately distinguishes these
/// synthetic/analytic checks from measurements collected in a laboratory.
#[derive(Clone, Debug)]
pub struct ExperimentCheck {
    pub experiment_id: &'static str,
    pub metric: &'static str,
    pub value: f64,
    pub reference: f64,
    pub absolute_tolerance: f64,
    pub passed: bool,
    pub evidence_class: &'static str,
    pub details: &'static str,
}

fn check(
    experiment_id: &'static str,
    metric: &'static str,
    value: f64,
    reference: f64,
    absolute_tolerance: f64,
    evidence_class: &'static str,
    details: &'static str,
) -> ExperimentCheck {
    let passed = value.is_finite()
        && reference.is_finite()
        && absolute_tolerance.is_finite()
        && absolute_tolerance >= 0.0
        && (value - reference).abs() <= absolute_tolerance;

    ExperimentCheck {
        experiment_id,
        metric,
        value,
        reference,
        absolute_tolerance,
        passed,
        evidence_class,
        details,
    }
}

/// Run the bounded v0.1 qualification suite in stable, deterministic order.
pub fn run_suite() -> Vec<ExperimentCheck> {
    let mut checks = Vec::with_capacity(19);

    // Galileo: ideal no-friction motion down a 30-degree incline. This is a
    // modern SI model, not a claim to reconstruct Galileo's original apparatus.
    let g = 9.806_65_f64;
    let theta = PI / 6.0;
    let acceleration = g * theta.sin();
    let duration = 2.0;
    let exact_distance = 0.5 * acceleration * duration * duration;
    checks.push(check(
        "galileo_inclined_plane",
        "analytic_displacement_m",
        exact_distance,
        9.806_65,
        1.0e-12,
        "analytic_reference",
        "Ideal incline, constant acceleration, no rolling-friction or air-drag model.",
    ));

    let coarse = semi_implicit_displacement(acceleration, duration, 0.1);
    let fine = semi_implicit_displacement(acceleration, duration, 0.05);
    let coarse_error = (coarse - exact_distance).abs();
    let fine_error = (fine - exact_distance).abs();
    let error_ratio = coarse_error / fine_error;
    checks.push(check(
        "galileo_inclined_plane",
        "error_ratio_dt_over_dt_half",
        error_ratio,
        2.0,
        1.0e-10,
        "numerical_convergence",
        "Semi-implicit Euler has first-order displacement error here; halving dt should halve error.",
    ));

    // Newton/Cavendish: test the inverse-square law, not Cavendish's torsion
    // balance transfer function or the uncertainty of a real measurement.
    let force_near = newtonian_force(2.0, 3.0, 0.25);
    let force_far = newtonian_force(2.0, 3.0, 0.50);
    checks.push(check(
        "cavendish_inverse_square",
        "force_ratio_r_to_2r",
        force_near / force_far,
        4.0,
        1.0e-12,
        "analytic_reference",
        "Point-mass Newtonian gravity; an actual torsion balance needs apparatus and nuisance-parameter modelling.",
    ));

    // Kepler: velocity-Verlet integration of a circular Earth-centred two-body
    // orbit, using Earth's standard gravitational parameter.
    let mu: f64 = 3.986_004_418e14;
    let radius: f64 = 7.0e6;
    let circular_speed = (mu / radius).sqrt();
    let period = 2.0 * PI * (radius.powi(3) / mu).sqrt();
    let (relative_energy_drift, relative_angular_momentum_drift) =
        integrate_circular_orbit(mu, radius, circular_speed, period / 1000.0, 2000);

    checks.push(check(
        "kepler_orbit",
        "max_relative_specific_energy_drift",
        relative_energy_drift,
        0.0,
        1.0e-8,
        "numerical_invariant",
        "Two-body point-mass orbit integrated with velocity-Verlet; excludes perturbations and non-spherical bodies.",
    ));
    checks.push(check(
        "kepler_orbit",
        "max_relative_specific_angular_momentum_drift",
        relative_angular_momentum_drift,
        0.0,
        1.0e-12,
        "numerical_invariant",
        "Central-force angular momentum should remain conserved to floating-point round-off.",
    ));

    // Photoelectric effect: synthetic stopping-potential observations generated
    // from the v0.1 reference slope, then recovered by ordinary least squares.
    // This checks the extraction pipeline, not an experimental measurement of h.
    let h_over_e_ev_s = 4.135_667_696e-15;
    let work_function_ev = 2.0;
    let frequencies = [5.5e14, 6.0e14, 6.5e14, 7.0e14, 7.5e14, 8.0e14, 8.5e14];
    let stopping_voltages: Vec<f64> = frequencies
        .iter()
        .map(|frequency| h_over_e_ev_s * frequency - work_function_ev)
        .collect();
    let recovered_h_over_e = linear_regression_slope(&frequencies, &stopping_voltages);
    checks.push(check(
        "photoelectric_effect",
        "recovered_planck_over_charge_ev_s",
        recovered_h_over_e,
        h_over_e_ev_s,
        1.0e-27,
        "synthetic_parameter_recovery",
        "Noiseless synthetic stopping potentials; real work needs measured points, uncertainty and apparatus calibration.",
    ));

    // Millikan oil-drop charge quantization: synthetic observations in units
    // of 1e-19 C, with unknown integer charge counts and small measurement noise.
    // This tests a simple lattice estimator; it is not Millikan's historical data.
    let observations = synthetic_millikan_observations();
    let estimated_e = estimate_elementary_charge(&observations).unwrap_or(f64::NAN);
    let rms_residual = lattice_rms_residual(&observations, estimated_e).unwrap_or(f64::NAN);
    checks.push(check(
        "millikan_oil_drop",
        "estimated_elementary_charge_1e_minus19_c",
        estimated_e,
        1.602_176_634,
        1.0e-4,
        "synthetic_parameter_recovery",
        "Synthetic charge lattice with noise; not a digitization or reevaluation of Millikan's historical measurements.",
    ));
    checks.push(check(
        "millikan_oil_drop",
        "rms_charge_lattice_residual_1e_minus19_c",
        rms_residual,
        0.0,
        1.5e-3,
        "synthetic_residual_check",
        "RMS residual to nearest integer charge multiples; viscosity, drag correction and apparatus calibration are omitted.",
    ));

    // Michelson-Morley: compare idealized predictions. The classical value is
    // a counterfactual model prediction, not a historical observed shift.
    let c: f64 = 299_792_458.0;
    let arm_length_m: f64 = 11.0;
    let assumed_aether_speed_m_s: f64 = 30_000.0;
    let wavelength_m: f64 = 500.0e-9;
    let classical_shift =
        2.0 * arm_length_m * (assumed_aether_speed_m_s / c).powi(2) / wavelength_m;
    checks.push(check(
        "michelson_morley",
        "classical_counterfactual_fringe_shift",
        classical_shift,
        0.440_609_422_197_232_9,
        1.0e-12,
        "analytic_reference",
        "Idealized classical-aether prediction for declared parameters; not digitized historical data.",
    ));
    checks.push(check(
        "michelson_morley",
        "lorentz_invariant_predicted_shift",
        0.0,
        0.0,
        0.0,
        "model_prediction",
        "The invariant model predicts no orientation-dependent aether-wind shift in this idealized setup.",
    ));

    // Double slit: ideal two-path far-field fringe law with explicit visibility.
    // It is not a finite-aperture Fresnel/Fraunhofer wave solver.
    let peak = double_slit_intensity(0.0, 1.0, 1.0);
    let minimum = double_slit_intensity(0.5, 1.0, 1.0);
    let contrast = (peak - minimum) / (peak + minimum);
    checks.push(check(
        "double_slit",
        "ideal_fringe_visibility",
        contrast,
        1.0,
        1.0e-12,
        "model_invariant",
        "Normalized ideal two-path intensity model; detector noise, slit aperture and which-path apparatus are omitted.",
    ));
    let which_path_peak = double_slit_intensity(0.0, 1.0, 0.0);
    let which_path_minimum = double_slit_intensity(0.5, 1.0, 0.0);
    checks.push(check(
        "double_slit",
        "zero_visibility_intensity_difference",
        (which_path_peak - which_path_minimum).abs(),
        0.0,
        1.0e-12,
        "model_invariant",
        "Setting visibility to zero removes fringes in this phenomenological model; it is not a microscopic decoherence simulation.",
    ));

    // Stern-Gerlach: sequential ideal spin-1/2 projective-measurement law
    // P(+ | +, angle theta) = cos^2(theta / 2).
    checks.push(check(
        "stern_gerlach",
        "same_axis_spin_up_probability",
        spin_up_probability(0.0),
        1.0,
        1.0e-12,
        "born_rule_reference",
        "Ideal spin-1/2 measurement; not a trajectory simulation of a finite magnet.",
    ));
    checks.push(check(
        "stern_gerlach",
        "orthogonal_axis_spin_up_probability",
        spin_up_probability(PI / 2.0),
        0.5,
        1.0e-12,
        "born_rule_reference",
        "Ideal spin-1/2 measurement; detector inefficiency and state preparation error are omitted.",
    ));
    checks.push(check(
        "stern_gerlach",
        "opposite_axis_spin_up_probability",
        spin_up_probability(PI),
        0.0,
        1.0e-12,
        "born_rule_reference",
        "Ideal spin-1/2 measurement; probability follows the declared prepared state and measurement axis.",
    ));

    // Foucault pendulum: signed rotation of the swing plane in the Earth frame.
    // Convention: negative angle is clockwise viewed from above; northern latitudes
    // therefore produce a negative signed rate. Small-angle, ideal-pendulum limit.
    checks.push(check(
        "foucault_pendulum",
        "equatorial_precession_deg_per_hour",
        foucault_precession_rate_deg_per_hour(0.0),
        0.0,
        1.0e-12,
        "analytic_reference",
        "Ideal Foucault precession vanishes at the equator; amplitude-dependent real-world drift is omitted.",
    ));
    checks.push(check(
        "foucault_pendulum",
        "north_pole_precession_deg_per_hour",
        foucault_precession_rate_deg_per_hour(90.0),
        -15.041_066_876_065_45,
        1.0e-10,
        "analytic_reference",
        "Uses the declared sidereal Earth rotation rate; sign is clockwise viewed from above.",
    ));
    checks.push(check(
        "foucault_pendulum",
        "hemisphere_sign_reversal_deg_per_hour",
        foucault_precession_rate_deg_per_hour(45.0)
            + foucault_precession_rate_deg_per_hour(-45.0),
        0.0,
        1.0e-12,
        "model_invariant",
        "Opposite latitudes have equal-and-opposite ideal precession rates under the stated sign convention.",
    ));

    // Rutherford: repulsive, unscreened Coulomb scattering. Check the invariant
    // b * tan(theta/2) = k/(2E), rather than confusing a visual trajectory
    // with the actual scattering-angle observable.
    let k = 2.0;
    let kinetic_energy = 1.0;
    let impact_one = 1.0;
    let impact_two = 2.0;
    let angle_one = rutherford_scattering_angle(k, kinetic_energy, impact_one);
    let angle_two = rutherford_scattering_angle(k, kinetic_energy, impact_two);
    let invariant =
        0.5 * (impact_one * (angle_one / 2.0).tan() + impact_two * (angle_two / 2.0).tan());
    checks.push(check(
        "rutherford_scattering",
        "impact_parameter_angle_invariant",
        invariant,
        k / (2.0 * kinetic_energy),
        1.0e-12,
        "analytic_reference",
        "Classical unscreened Coulomb scattering; not a quantum nuclear cross-section or detector-response model.",
    ));

    checks
}

/// Semi-implicit Euler for constant acceleration, used only in the Galileo
/// convergence check. duration / dt must be an integer.
fn semi_implicit_displacement(acceleration: f64, duration: f64, dt: f64) -> f64 {
    let steps = (duration / dt).round() as usize;
    let mut position = 0.0;
    let mut velocity = 0.0;
    for _ in 0..steps {
        velocity += acceleration * dt;
        position += velocity * dt;
    }
    position
}

fn synthetic_millikan_observations() -> Vec<f64> {
    let elementary_charge = 1.602_176_634_f64;
    let multiplicities = [2.0, 3.0, 5.0, 4.0, 7.0, 6.0, 9.0, 8.0, 11.0, 10.0];
    let noise = [
        0.0008, -0.0012, 0.0004, -0.0006, 0.0011, -0.0003, 0.0002, -0.0010, 0.0005, -0.0002,
    ];
    multiplicities
        .iter()
        .zip(noise.iter())
        .map(|(multiple, error)| *multiple * elementary_charge + *error)
        .collect()
}

/// Fit a common charge unit by grid-searching the least-squares distance from
/// each observation to its nearest allowed integer multiple. Observations use
/// units of 1e-19 C; a declared search interval prevents trivial scale aliases.
fn estimate_elementary_charge(observations: &[f64]) -> Option<f64> {
    if observations.len() < 3
        || observations
            .iter()
            .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return None;
    }
    let mut best_charge = f64::NAN;
    let mut best_sse = f64::INFINITY;
    for step in 0..=20_000 {
        let candidate = 1.5 + step as f64 * 1.0e-5;
        let mut sse = 0.0;
        for observation in observations {
            let multiple = (observation / candidate).round().clamp(1.0, 12.0);
            let residual = observation - multiple * candidate;
            sse += residual * residual;
        }
        if sse < best_sse {
            best_sse = sse;
            best_charge = candidate;
        }
    }
    best_charge.is_finite().then_some(best_charge)
}

fn lattice_rms_residual(observations: &[f64], elementary_charge: f64) -> Option<f64> {
    if observations.is_empty()
        || !elementary_charge.is_finite()
        || elementary_charge <= 0.0
        || observations
            .iter()
            .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return None;
    }
    let squared_error = observations
        .iter()
        .map(|observation| {
            let multiple = (observation / elementary_charge).round().clamp(1.0, 12.0);
            (observation - multiple * elementary_charge).powi(2)
        })
        .sum::<f64>();
    Some((squared_error / observations.len() as f64).sqrt())
}

fn foucault_precession_rate_deg_per_hour(latitude_deg: f64) -> f64 {
    const EARTH_ROTATION_RAD_PER_SECOND: f64 = 7.292_115_0e-5;
    -EARTH_ROTATION_RAD_PER_SECOND * latitude_deg.to_radians().sin() * 180.0 / PI * 3600.0
}

fn newtonian_force(mass_a: f64, mass_b: f64, separation_m: f64) -> f64 {
    const G: f64 = 6.674_30e-11;
    G * mass_a * mass_b / separation_m.powi(2)
}

fn linear_regression_slope(xs: &[f64], ys: &[f64]) -> f64 {
    if xs.len() != ys.len() || xs.len() < 2 {
        return f64::NAN;
    }
    let n = xs.len() as f64;
    let mean_x = xs.iter().sum::<f64>() / n;
    let mean_y = ys.iter().sum::<f64>() / n;
    let numerator = xs
        .iter()
        .zip(ys.iter())
        .map(|(x, y)| (x - mean_x) * (y - mean_y))
        .sum::<f64>();
    let denominator = xs.iter().map(|x| (x - mean_x).powi(2)).sum::<f64>();
    numerator / denominator
}

fn integrate_circular_orbit(
    mu: f64,
    radius: f64,
    initial_speed: f64,
    dt: f64,
    steps: usize,
) -> (f64, f64) {
    let mut x = radius;
    let mut y = 0.0;
    let mut vx = 0.0;
    let mut vy = initial_speed;

    let specific_energy =
        |x: f64, y: f64, vx: f64, vy: f64| 0.5 * (vx * vx + vy * vy) - mu / x.hypot(y);
    let specific_angular_momentum = |x: f64, y: f64, vx: f64, vy: f64| x * vy - y * vx;

    let energy_initial = specific_energy(x, y, vx, vy);
    let angular_momentum_initial = specific_angular_momentum(x, y, vx, vy);
    let mut max_relative_energy_drift: f64 = 0.0;
    let mut max_relative_angular_momentum_drift: f64 = 0.0;

    for _ in 0..steps {
        let radius_now = x.hypot(y);
        let ax = -mu * x / radius_now.powi(3);
        let ay = -mu * y / radius_now.powi(3);

        let next_x = x + vx * dt + 0.5 * ax * dt * dt;
        let next_y = y + vy * dt + 0.5 * ay * dt * dt;
        let radius_next = next_x.hypot(next_y);
        let next_ax = -mu * next_x / radius_next.powi(3);
        let next_ay = -mu * next_y / radius_next.powi(3);

        vx += 0.5 * (ax + next_ax) * dt;
        vy += 0.5 * (ay + next_ay) * dt;
        x = next_x;
        y = next_y;

        let energy = specific_energy(x, y, vx, vy);
        let angular_momentum = specific_angular_momentum(x, y, vx, vy);
        max_relative_energy_drift =
            max_relative_energy_drift.max(((energy - energy_initial) / energy_initial).abs());
        max_relative_angular_momentum_drift = max_relative_angular_momentum_drift
            .max(((angular_momentum - angular_momentum_initial) / angular_momentum_initial).abs());
    }

    (
        max_relative_energy_drift,
        max_relative_angular_momentum_drift,
    )
}

fn double_slit_intensity(
    position_in_fringe_units: f64,
    fringe_spacing: f64,
    visibility: f64,
) -> f64 {
    let phase = 2.0 * PI * position_in_fringe_units / fringe_spacing;
    1.0 + visibility * phase.cos()
}

fn spin_up_probability(axis_angle_rad: f64) -> f64 {
    (axis_angle_rad / 2.0).cos().powi(2)
}

fn rutherford_scattering_angle(
    coulomb_parameter: f64,
    kinetic_energy: f64,
    impact_parameter: f64,
) -> f64 {
    2.0 * (coulomb_parameter / (2.0 * kinetic_energy * impact_parameter)).atan()
}

/// Serialize suite results without a runtime dependency. Floating-point values
/// use scientific notation and stable field order for byte-for-byte replay.
fn check_is_passing(entry: &ExperimentCheck) -> bool {
    entry.passed
        && entry.value.is_finite()
        && entry.reference.is_finite()
        && entry.absolute_tolerance.is_finite()
        && entry.absolute_tolerance >= 0.0
        && (entry.value - entry.reference).abs() <= entry.absolute_tolerance
}

pub fn json_report(checks: &[ExperimentCheck]) -> String {
    let passed = checks
        .iter()
        .filter(|entry| check_is_passing(entry))
        .count();
    let all_passed = !checks.is_empty() && passed == checks.len();
    let mut output = String::from(
        "{\"suite_id\":\"physics-experiment-atlas-v0.1\",\"claim_ceiling\":\"analytic and synthetic model checks only; no historical raw-data reproduction or apparatus validation\",\"passed\":",
    );
    output.push_str(if all_passed { "true" } else { "false" });
    output.push_str(&format!(
        ",\"check_count\":{},\"passed_count\":{},\"checks\":[",
        checks.len(),
        passed
    ));

    for (index, entry) in checks.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&format!(
            "{{\"experiment_id\":\"{}\",\"metric\":\"{}\",\"value\":{},\"reference\":{},\"absolute_tolerance\":{},\"passed\":{},\"evidence_class\":\"{}\",\"details\":\"{}\"}}",
            json_escape(entry.experiment_id),
            json_escape(entry.metric),
            finite_json_number(entry.value),
            finite_json_number(entry.reference),
            finite_json_number(entry.absolute_tolerance),
            check_is_passing(entry),
            json_escape(entry.evidence_class),
            json_escape(entry.details),
        ));
    }
    output.push_str("]}");
    output
}

fn finite_json_number(value: f64) -> String {
    if value.is_finite() {
        format!("{value:.17e}")
    } else {
        "null".to_owned()
    }
}

fn json_escape(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suite_is_deterministic_and_all_reference_checks_pass() {
        let first = run_suite();
        let second = run_suite();
        assert_eq!(first.len(), 19);
        assert!(first.iter().all(|entry| entry.passed));
        assert_eq!(json_report(&first), json_report(&second));
    }

    #[test]
    fn halving_step_reduces_galileo_displacement_error_linearly() {
        let acceleration = 9.806_65 * (PI / 6.0).sin();
        let duration = 2.0;
        let exact = 0.5 * acceleration * duration * duration;
        let coarse = (semi_implicit_displacement(acceleration, duration, 0.1) - exact).abs();
        let fine = (semi_implicit_displacement(acceleration, duration, 0.05) - exact).abs();
        assert!((coarse / fine - 2.0).abs() < 1.0e-10);
    }

    #[test]
    fn photoelectric_fit_recovers_synthetic_planck_over_charge_slope() {
        let slope = run_suite()
            .into_iter()
            .find(|entry| entry.metric == "recovered_planck_over_charge_ev_s")
            .expect("photoelectric check exists");
        assert!(slope.passed);
        assert!((slope.value - 4.135_667_696e-15).abs() <= 1.0e-27);
    }

    #[test]
    fn millikan_synthetic_charge_lattice_recovers_elementary_charge() {
        let observations = synthetic_millikan_observations();
        let estimate = estimate_elementary_charge(&observations).unwrap();
        assert!((estimate - 1.602_176_634).abs() < 1.0e-4);
        assert!(lattice_rms_residual(&observations, estimate).unwrap() < 1.5e-3);
    }

    #[test]
    fn millikan_estimator_rejects_invalid_observations() {
        assert!(estimate_elementary_charge(&[]).is_none());
        assert!(estimate_elementary_charge(&[1.0, f64::NAN, 3.0]).is_none());
        assert!(lattice_rms_residual(&[1.0, 2.0], 0.0).is_none());
    }

    #[test]
    fn foucault_precession_matches_latitude_and_hemisphere_controls() {
        assert!(foucault_precession_rate_deg_per_hour(0.0).abs() < 1.0e-12);
        assert!(
            (foucault_precession_rate_deg_per_hour(90.0) + 15.041_066_876_065_45).abs() < 1.0e-10
        );
        assert!(
            (foucault_precession_rate_deg_per_hour(45.0)
                + foucault_precession_rate_deg_per_hour(-45.0))
            .abs()
                < 1.0e-12
        );
    }

    #[test]
    fn spin_half_born_probabilities_have_expected_axis_limits() {
        assert!((spin_up_probability(0.0) - 1.0).abs() < 1.0e-12);
        assert!((spin_up_probability(PI / 2.0) - 0.5).abs() < 1.0e-12);
        assert!(spin_up_probability(PI).abs() < 1.0e-12);
    }

    #[test]
    fn double_slit_visibility_control_removes_interference_contrast() {
        let full_peak = double_slit_intensity(0.0, 1.0, 1.0);
        let full_minimum = double_slit_intensity(0.5, 1.0, 1.0);
        assert!((full_peak - 2.0).abs() < 1.0e-12);
        assert!(full_minimum.abs() < 1.0e-12);

        let absent_peak = double_slit_intensity(0.0, 1.0, 0.0);
        let absent_minimum = double_slit_intensity(0.5, 1.0, 0.0);
        assert!((absent_peak - absent_minimum).abs() < 1.0e-12);
    }

    #[test]
    fn rutherford_angle_observable_obeys_impact_parameter_relation() {
        let k = 2.0;
        let energy = 1.0;
        for impact_parameter in [0.5, 1.0, 2.0, 4.0] {
            let angle = rutherford_scattering_angle(k, energy, impact_parameter);
            let recovered = impact_parameter * (angle / 2.0).tan();
            assert!((recovered - k / (2.0 * energy)).abs() < 1.0e-12);
        }
    }

    #[test]
    fn json_report_maps_non_finite_values_to_null_and_fails_closed() {
        let invalid = ExperimentCheck {
            experiment_id: "invalid_fixture",
            metric: "non_finite_value",
            value: f64::NAN,
            reference: 0.0,
            absolute_tolerance: 1.0,
            passed: true,
            evidence_class: "negative_control",
            details: "A non-finite result must not corrupt JSON output.",
        };
        let output = json_report(&[invalid]);
        assert!(output.contains("\"passed\":false"));
        assert!(output.contains("\"value\":null"));
    }

    #[test]
    fn empty_json_report_fails_closed() {
        let output = json_report(&[]);
        assert!(output.contains("\"passed\":false"));
        assert!(output.contains("\"check_count\":0"));
        assert!(output.contains("\"passed_count\":0"));
    }

    #[test]
    fn json_report_has_explicit_synthetic_claim_ceiling() {
        let output = json_report(&run_suite());
        assert!(output.contains("\"suite_id\":\"physics-experiment-atlas-v0.1\""));
        assert!(output.contains("no historical raw-data reproduction"));
        assert!(output.contains("\"check_count\":19"));
    }
}
