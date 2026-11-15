//! Independent consumer-side state-space oracle for the synthetic transient.
//!
//! This integration target is separate from the implementation. It propagates
//! the relative coordinate with a closed-form matrix exponential and handles
//! the intermediate constant-force interval when one restraint releases first.

use ors_dynamic_support_reference::{Scenario, evaluate, parse_corpus};

fn propagate_relative(
    initial_separation_m: f64,
    initial_velocity_mps: f64,
    elapsed_s: f64,
    equilibrium_separation_m: f64,
    reduced_mass_kg: f64,
    stiffness_n_m: f64,
    damping_n_s_m: f64,
) -> (f64, f64) {
    let alpha = damping_n_s_m / (2.0 * reduced_mass_kg);
    let omega_n_squared = stiffness_n_m / reduced_mass_kg;
    let omega_d_squared = omega_n_squared - alpha * alpha;
    assert!(
        omega_d_squared > 0.0,
        "frozen oracle cases must be underdamped"
    );

    let omega_d = omega_d_squared.sqrt();
    let initial_offset = initial_separation_m - equilibrium_separation_m;
    let sine_coefficient = (initial_velocity_mps + alpha * initial_offset) / omega_d;
    let angle = omega_d * elapsed_s;
    let cosine = angle.cos();
    let sine = angle.sin();
    let decay = (-alpha * elapsed_s).exp();

    let offset = decay * (initial_offset * cosine + sine_coefficient * sine);
    let velocity = decay
        * (-alpha * (initial_offset * cosine + sine_coefficient * sine)
            - initial_offset * omega_d * sine
            + sine_coefficient * omega_d * cosine);
    (equilibrium_separation_m + offset, velocity)
}

fn exact_relative_state(scenario: &Scenario) -> (f64, f64, f64) {
    let reduced_mass_kg = (scenario.mass_stream_kg * scenario.mass_support_kg)
        / (scenario.mass_stream_kg + scenario.mass_support_kg);
    let initial_separation_m = scenario.preload_n / scenario.stiffness_n_m;
    let first_release_s = scenario
        .release_stream_time_s
        .min(scenario.release_support_time_s);
    let second_release_s = scenario
        .release_stream_time_s
        .max(scenario.release_support_time_s);

    // Until the first restraint releases, equal-and-opposite restraints hold
    // the preloaded spring stationary.
    let mut separation_m = initial_separation_m;
    let mut velocity_mps = 0.0;
    let skew_s = second_release_s - first_release_s;

    // Between release events, exactly one external restraint remains.
    if skew_s > 0.0 && scenario.preload_n > 0.0 {
        let equilibrium_separation_m =
            if scenario.release_stream_time_s < scenario.release_support_time_s {
                // Stream restraint releases first; the support-side restraint is -F0.
                reduced_mass_kg * scenario.preload_n
                    / (scenario.mass_support_kg * scenario.stiffness_n_m)
            } else {
                // Support restraint releases first; the stream-side restraint is +F0.
                reduced_mass_kg * scenario.preload_n
                    / (scenario.mass_stream_kg * scenario.stiffness_n_m)
            };
        (separation_m, velocity_mps) = propagate_relative(
            separation_m,
            velocity_mps,
            skew_s,
            equilibrium_separation_m,
            reduced_mass_kg,
            scenario.stiffness_n_m,
            scenario.damping_n_s_m,
        );
    }

    // After both restraints release, no external relative force remains.
    let free_flight_s = scenario.duration_s - second_release_s;
    (separation_m, velocity_mps) = propagate_relative(
        separation_m,
        velocity_mps,
        free_flight_s,
        0.0,
        reduced_mass_kg,
        scenario.stiffness_n_m,
        scenario.damping_n_s_m,
    );

    let expected_external_impulse_n_s = if scenario.preload_n == 0.0 {
        0.0
    } else if scenario.release_stream_time_s < scenario.release_support_time_s {
        -scenario.preload_n * skew_s
    } else if scenario.release_support_time_s < scenario.release_stream_time_s {
        scenario.preload_n * skew_s
    } else {
        0.0
    };

    (separation_m, velocity_mps, expected_external_impulse_n_s)
}

#[test]
fn consumer_side_state_space_oracle_matches_all_frozen_cases() {
    let cases = parse_corpus(include_str!("../cases-v0.1.csv")).expect("frozen corpus");
    assert_eq!(cases.len(), 9);

    for case in cases {
        let report = evaluate(&case.scenario).expect("synthetic evaluation succeeds");
        let (expected_separation, expected_velocity, expected_impulse) =
            exact_relative_state(&case.scenario);

        assert!(
            (report.final_separation_m - expected_separation).abs() <= case.absolute_tolerance,
            "{} separation differs from independent piecewise oracle",
            case.scenario.case_id
        );
        assert!(
            (report.final_relative_velocity_mps - expected_velocity).abs()
                <= case.absolute_tolerance,
            "{} relative velocity differs from independent piecewise oracle",
            case.scenario.case_id
        );
        assert!(
            (report.final_external_impulse_n_s - expected_impulse).abs()
                <= case.absolute_tolerance * case.scenario.preload_n.max(1.0),
            "{} external impulse differs from independent oracle",
            case.scenario.case_id
        );
        assert!(
            report.max_abs_linear_momentum_residual_kg_mps < 1.0e-7,
            "{} total momentum does not close against external impulse",
            case.scenario.case_id
        );
    }
}

#[test]
fn mass_and_force_scale_together_without_changing_kinematic_response() {
    let cases = parse_corpus(include_str!("../cases-v0.1.csv")).expect("frozen corpus");
    let baseline = cases
        .iter()
        .find(|case| case.scenario.case_id == "RELEASE_LIGHT_DAMPING")
        .expect("light damping fixture");
    let original = evaluate(&baseline.scenario).expect("baseline response");

    let mut scaled = baseline.scenario.clone();
    scaled.mass_stream_kg *= 17.0;
    scaled.mass_support_kg *= 17.0;
    scaled.stiffness_n_m *= 17.0;
    scaled.damping_n_s_m *= 17.0;
    scaled.preload_n *= 17.0;
    let scaled_result = evaluate(&scaled).expect("scaled response");

    assert!((original.final_separation_m - scaled_result.final_separation_m).abs() < 1.0e-10);
    assert!(
        (original.final_relative_velocity_mps - scaled_result.final_relative_velocity_mps).abs()
            < 1.0e-9
    );
    assert!(
        (17.0 * original.final_dissipated_energy_j - scaled_result.final_dissipated_energy_j).abs()
            < 1.0e-5
    );
}

#[test]
fn identical_reduced_mass_gives_identical_relative_response() {
    let cases = parse_corpus(include_str!("../cases-v0.1.csv")).expect("frozen corpus");
    let baseline = cases
        .iter()
        .find(|case| case.scenario.case_id == "RELEASE_LIGHT_DAMPING")
        .expect("light damping fixture");
    let original = evaluate(&baseline.scenario).expect("baseline response");

    let mut alternative = baseline.scenario.clone();
    // The relative-coordinate equation depends on the reduced mass; exchange
    // the two lumped inertias while preserving that reduced mass.
    std::mem::swap(
        &mut alternative.mass_stream_kg,
        &mut alternative.mass_support_kg,
    );
    let alternative_result = evaluate(&alternative).expect("swapped inertia response");

    assert!((original.final_separation_m - alternative_result.final_separation_m).abs() < 1.0e-10);
    assert!(
        (original.final_relative_velocity_mps - alternative_result.final_relative_velocity_mps)
            .abs()
            < 1.0e-9
    );
}
