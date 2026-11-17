//! Step-refinement checks for both orders of asynchronous restraint release.
//!
//! The exact reference is the piecewise damped-oscillator solution, not a
//! second invocation of the RK4 integrator. Release events stay aligned to
//! every tested grid so no RK stage crosses a force discontinuity.

use ors_dynamic_support_reference::{ModelError, Scenario, evaluate, validate_scenario};

const STEP_SIZES_S: [f64; 4] = [0.001, 0.0005, 0.00025, 0.000125];

fn scenario(dt_s: f64, release_stream_time_s: f64, release_support_time_s: f64) -> Scenario {
    Scenario {
        case_id: "RELEASE_ORDER_REFINEMENT".to_owned(),
        force_law_id: "synthetic-linear-spring-damper-v1".to_owned(),
        claim_class: "SYNTHETIC_NOT_PHYSICAL".to_owned(),
        mass_stream_kg: 1_000.0,
        mass_support_kg: 2_000.0,
        stiffness_n_m: 1_000_000.0,
        damping_n_s_m: 2_000.0,
        preload_n: 100_000.0,
        release_stream_time_s,
        release_support_time_s,
        duration_s: 0.4,
        dt_s,
    }
}

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
        "the convergence fixture must remain underdamped"
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

fn exact_state(scenario: &Scenario) -> (f64, f64, f64) {
    let reduced_mass_kg = (scenario.mass_stream_kg * scenario.mass_support_kg)
        / (scenario.mass_stream_kg + scenario.mass_support_kg);
    let initial_separation_m = scenario.preload_n / scenario.stiffness_n_m;
    let first_release_s = scenario
        .release_stream_time_s
        .min(scenario.release_support_time_s);
    let second_release_s = scenario
        .release_stream_time_s
        .max(scenario.release_support_time_s);

    let mut separation_m = initial_separation_m;
    let mut velocity_mps = 0.0;
    let skew_s = second_release_s - first_release_s;

    // During the interval with one restraint still engaged, solve the
    // constant-force shifted oscillator exactly.
    if skew_s > 0.0 && scenario.preload_n > 0.0 {
        let equilibrium_separation_m =
            if scenario.release_stream_time_s < scenario.release_support_time_s {
                reduced_mass_kg * scenario.preload_n
                    / (scenario.mass_support_kg * scenario.stiffness_n_m)
            } else {
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

    // Once both restraints are released, the relative oscillator is free.
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

    let expected_impulse_n_s = if scenario.release_stream_time_s < scenario.release_support_time_s {
        -scenario.preload_n * skew_s
    } else if scenario.release_support_time_s < scenario.release_stream_time_s {
        scenario.preload_n * skew_s
    } else {
        0.0
    };

    (separation_m, velocity_mps, expected_impulse_n_s)
}

fn assert_grid_aligned(event_time_s: f64, dt_s: f64) {
    let steps = event_time_s / dt_s;
    assert!(
        (steps - steps.round()).abs() <= 1.0e-10 * steps.abs().max(1.0),
        "event at {event_time_s} s is not aligned to dt={dt_s} s"
    );
}

fn assert_fourth_order_convergence(release_stream_time_s: f64, release_support_time_s: f64) {
    let mut position_errors = Vec::new();
    let mut velocity_errors = Vec::new();
    let mut energy_residuals = Vec::new();

    for dt_s in STEP_SIZES_S {
        let scenario = scenario(dt_s, release_stream_time_s, release_support_time_s);
        assert_grid_aligned(scenario.duration_s, dt_s);
        assert_grid_aligned(scenario.release_stream_time_s, dt_s);
        assert_grid_aligned(scenario.release_support_time_s, dt_s);

        let report = evaluate(&scenario).expect("aligned synthetic case evaluates");
        let (expected_separation_m, expected_velocity_mps, expected_impulse_n_s) =
            exact_state(&scenario);

        position_errors.push((report.final_separation_m - expected_separation_m).abs());
        velocity_errors.push((report.final_relative_velocity_mps - expected_velocity_mps).abs());
        energy_residuals.push(report.max_abs_energy_balance_residual_j);

        assert!(
            (report.final_external_impulse_n_s - expected_impulse_n_s).abs() < 1.0e-8,
            "external impulse changed under refinement: {:?}",
            scenario.case_id
        );
        assert!(
            report.max_abs_linear_momentum_residual_kg_mps < 1.0e-7,
            "momentum does not close at dt={dt_s}"
        );
    }

    // RK4 global error should scale approximately as dt^4 for these smooth,
    // event-aligned segments. The deliberately broad interval avoids making
    // this regression test sensitive to platform-level floating-point noise.
    for errors in [&position_errors, &velocity_errors] {
        for pair in errors.windows(2) {
            assert!(pair[1] > 0.0, "error unexpectedly reached roundoff floor");
            let ratio = pair[0] / pair[1];
            assert!(
                (12.0..20.0).contains(&ratio),
                "step-halving ratio {ratio} is inconsistent with fourth-order convergence"
            );
        }
    }

    for pair in energy_residuals.windows(2) {
        assert!(pair[1] < pair[0], "energy residual did not decrease");
        let ratio = pair[0] / pair[1];
        assert!(
            ratio > 8.0,
            "energy residual refinement ratio {ratio} is too small"
        );
    }
}

#[test]
fn stream_restraint_releasing_first_converges_at_fourth_order() {
    assert_fourth_order_convergence(0.0, 0.05);
}

#[test]
fn support_restraint_releasing_first_converges_at_fourth_order() {
    assert_fourth_order_convergence(0.05, 0.0);
}

#[test]
fn off_grid_release_event_is_rejected_before_integration() {
    let mut invalid = scenario(0.001, 0.0, 0.05);
    invalid.release_support_time_s = 0.0501;

    assert_eq!(
        validate_scenario(&invalid),
        Err(ModelError::NonIntegralTimeGrid)
    );
}
