//! Independent consumer-side closed-form oracle for the synthetic baseline.
//!
//! This integration test is a separate crate target from the implementation.
//! The oracle uses the reduced-mass state-space matrix exponential, not the
//! implementation's RK4 stages or energy accumulator.

use ors_dynamic_support_reference::{evaluate, parse_corpus, Scenario};

fn exact_state_space(
    scenario: &Scenario,
    elapsed_s: f64,
) -> (f64, f64) {
    let reduced_mass = (scenario.mass_stream_kg * scenario.mass_support_kg)
        / (scenario.mass_stream_kg + scenario.mass_support_kg);
    let alpha = scenario.damping_n_s_m / (2.0 * reduced_mass);
    let omega_n_squared = scenario.stiffness_n_m / reduced_mass;
    let omega_d_squared = omega_n_squared - alpha * alpha;
    assert!(omega_d_squared >= 0.0, "oracle supports underdamped fixtures only");

    let omega_d = omega_d_squared.sqrt();
    let initial_separation = scenario.preload_n / scenario.stiffness_n_m;
    let decay = (-alpha * elapsed_s).exp();
    let angle = omega_d * elapsed_s;
    let cosine = angle.cos();
    let sine_over_frequency = if omega_d == 0.0 {
        elapsed_s
    } else {
        angle.sin() / omega_d
    };

    // Matrix exponential:
    // exp([0,1;-wn²,-2alpha]t) = exp(-alpha*t) * (I*cos(wd*t) + B*sin(wd*t)/wd)
    // for B = [alpha,1;-wn²,-alpha], with B² = -wd² I.
    let separation = initial_separation
        * decay
        * (cosine + alpha * sine_over_frequency);
    let velocity = -initial_separation
        * decay
        * omega_n_squared
        * sine_over_frequency;
    (separation, velocity)
}

#[test]
fn consumer_side_state_space_oracle_matches_all_frozen_cases() {
    let corpus = include_str!("../cases-v0.1.csv");
    let cases = parse_corpus(corpus).expect("frozen corpus must parse");
    assert_eq!(cases.len(), 7);

    for case in cases {
        let result = evaluate(&case.scenario).expect("synthetic evaluation succeeds");
        let elapsed = case.scenario.duration_s - case.scenario.release_time_s;
        let (expected_separation, expected_velocity) =
            exact_state_space(&case.scenario, elapsed);

        assert!(
            (result.final_separation_m - expected_separation).abs()
                <= case.absolute_tolerance,
            "{} separation differs from independent state-space oracle",
            case.scenario.case_id
        );
        assert!(
            (result.final_relative_velocity_mps - expected_velocity).abs()
                <= case.absolute_tolerance,
            "{} relative velocity differs from independent state-space oracle",
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

    assert!(
        (original.final_separation_m - scaled_result.final_separation_m).abs() < 1.0e-10
    );
    assert!(
        (original.final_relative_velocity_mps - scaled_result.final_relative_velocity_mps).abs()
            < 1.0e-9
    );
    assert!(
        (17.0 * original.final_dissipated_energy_j - scaled_result.final_dissipated_energy_j)
            .abs()
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
    // (1000 * 2000)/(1000 + 2000) = (2000 * 1000)/(2000 + 1000);
    // exchange the two inertias. The relative coordinate equation is unchanged.
    std::mem::swap(
        &mut alternative.mass_stream_kg,
        &mut alternative.mass_support_kg,
    );
    let alternative_result = evaluate(&alternative).expect("swapped inertia response");

    assert!(
        (original.final_separation_m - alternative_result.final_separation_m).abs() < 1.0e-10
    );
    assert!(
        (original.final_relative_velocity_mps
            - alternative_result.final_relative_velocity_mps)
            .abs()
            < 1.0e-9
    );
}
