//! Independent Jacobian and bifurcation known-answer fixtures for CIVDYN.
//!
//! These tests establish analytic reference behavior for small maps. They are
//! not a general Jacobian estimator, bifurcation detector, or production
//! attractor classifier. A candidate implementation must be evaluated
//! against these fixtures by a separate path.

fn coupled_map(state: [f64; 2]) -> [f64; 2] {
    let [x, y] = state;
    [
        0.4 * x + 0.1 * y + 0.05 * x * y,
        0.2 * x + 0.6 * y - 0.03 * x * y,
    ]
}

fn coupled_map_jacobian(state: [f64; 2]) -> [[f64; 2]; 2] {
    let [x, y] = state;
    [
        [0.4 + 0.05 * y, 0.1 + 0.05 * x],
        [0.2 - 0.03 * y, 0.6 - 0.03 * x],
    ]
}

fn central_difference_jacobian(
    state: [f64; 2],
    step: f64,
) -> [[f64; 2]; 2] {
    let mut jacobian = [[0.0; 2]; 2];

    for column in 0..2 {
        let mut plus = state;
        let mut minus = state;
        plus[column] += step;
        minus[column] -= step;

        let plus_value = coupled_map(plus);
        let minus_value = coupled_map(minus);

        for row in 0..2 {
            jacobian[row][column] =
                (plus_value[row] - minus_value[row]) / (2.0 * step);
        }
    }

    jacobian
}

fn discrete_two_dimensional_stability(trace: f64, determinant: f64) -> bool {
    // Second-order Jury conditions for both eigenvalues to lie strictly
    // inside the unit circle for a real 2x2 discrete-time Jacobian.
    1.0 - trace + determinant > 0.0
        && 1.0 + trace + determinant > 0.0
        && 1.0 - determinant > 0.0
}

fn characteristic_polynomial(lambda: f64, trace: f64, determinant: f64) -> f64 {
    lambda * lambda - trace * lambda + determinant
}

fn saddle_node_map(x: f64, mu: f64) -> f64 {
    x + mu - x * x
}

fn saddle_node_multiplier(x: f64) -> f64 {
    1.0 - 2.0 * x
}

fn logistic_map(x: f64, r: f64) -> f64 {
    r * x * (1.0 - x)
}

fn logistic_nonzero_fixed_point(r: f64) -> f64 {
    1.0 - 1.0 / r
}

fn logistic_multiplier_at_nonzero_fixed_point(r: f64) -> f64 {
    let fixed_point = logistic_nonzero_fixed_point(r);
    r * (1.0 - 2.0 * fixed_point)
}

fn transcritical_map(x: f64, mu: f64) -> f64 {
    (1.0 + mu) * x - x * x
}

fn transcritical_multiplier(x: f64, mu: f64) -> f64 {
    1.0 + mu - 2.0 * x
}

#[test]
fn coupled_map_jacobian_matches_independent_central_difference() {
    let state = [0.3, -0.4];
    let analytic = coupled_map_jacobian(state);
    let numeric = central_difference_jacobian(state, 1.0e-6);

    for row in 0..2 {
        for column in 0..2 {
            assert!(
                (analytic[row][column] - numeric[row][column]).abs() < 1.0e-9,
                "Jacobian mismatch at ({row}, {column}): analytic={}, numeric={}",
                analytic[row][column],
                numeric[row][column]
            );
        }
    }
}

#[test]
fn two_dimensional_jacobian_has_known_discrete_time_stability_class() {
    // Upper-triangular Jacobians have their diagonal entries as exact
    // eigenvalues, so these references do not require a numerical eigensolver.
    let stable_eigenvalues = [0.5_f64, 0.75_f64];
    let stable_trace = stable_eigenvalues[0] + stable_eigenvalues[1];
    let stable_determinant = stable_eigenvalues[0] * stable_eigenvalues[1];

    assert!(discrete_two_dimensional_stability(
        stable_trace,
        stable_determinant
    ));
    for eigenvalue in stable_eigenvalues {
        assert!(
            characteristic_polynomial(
                eigenvalue,
                stable_trace,
                stable_determinant
            )
            .abs()
                < 1.0e-12
        );
    }

    let unstable_eigenvalues = [1.1_f64, 0.5_f64];
    let unstable_trace = unstable_eigenvalues[0] + unstable_eigenvalues[1];
    let unstable_determinant = unstable_eigenvalues[0] * unstable_eigenvalues[1];

    assert!(!discrete_two_dimensional_stability(
        unstable_trace,
        unstable_determinant
    ));
    for eigenvalue in unstable_eigenvalues {
        assert!(
            characteristic_polynomial(
                eigenvalue,
                unstable_trace,
                unstable_determinant
            )
            .abs()
                < 1.0e-12
        );
    }
}

#[test]
fn saddle_node_fixture_pins_fixed_points_and_bifurcation_parameter() {
    // x[t+1] = x[t] + mu - x[t]^2 has fixed points x*=+/-sqrt(mu)
    // when mu >= 0. They coalesce at mu=0, where the multiplier is +1.
    let mu = 0.04_f64;
    let root = mu.sqrt();
    let stable = root;
    let unstable = -root;

    assert!((saddle_node_map(stable, mu) - stable).abs() < 1.0e-12);
    assert!((saddle_node_map(unstable, mu) - unstable).abs() < 1.0e-12);
    assert!((saddle_node_multiplier(stable) - 0.6).abs() < 1.0e-12);
    assert!((saddle_node_multiplier(unstable) - 1.4).abs() < 1.0e-12);
    assert!(saddle_node_multiplier(stable).abs() < 1.0);
    assert!(saddle_node_multiplier(unstable).abs() > 1.0);

    assert_eq!(saddle_node_map(0.0, 0.0), 0.0);
    assert!((saddle_node_multiplier(0.0) - 1.0).abs() < 1.0e-12);
    assert!(4.0 * -0.01 < 0.0, "negative mu has no real fixed-point pair");
    assert!(4.0 * mu > 0.0, "positive mu has two distinct fixed points");
}

#[test]
fn logistic_map_period_doubling_threshold_is_r_three() {
    // The nonzero fixed point is x*=1-1/r and its multiplier is 2-r.
    // It crosses -1 at r=3, the exact period-doubling threshold.
    let below = logistic_multiplier_at_nonzero_fixed_point(2.9);
    let critical = logistic_multiplier_at_nonzero_fixed_point(3.0);
    let above = logistic_multiplier_at_nonzero_fixed_point(3.1);

    assert!((logistic_map(logistic_nonzero_fixed_point(3.0), 3.0)
        - logistic_nonzero_fixed_point(3.0))
        .abs()
        < 1.0e-12);
    assert!((below + 0.9).abs() < 1.0e-12);
    assert!((critical + 1.0).abs() < 1.0e-12);
    assert!((above + 1.1).abs() < 1.0e-12);
    assert!(below.abs() < 1.0);
    assert!((critical.abs() - 1.0).abs() < 1.0e-12);
    assert!(above.abs() > 1.0);
}

#[test]
fn transcritical_fixture_exchanges_fixed_point_stability_at_zero() {
    // x[t+1] = (1+mu)x[t] - x[t]^2 has fixed points x*=0 and x*=mu.
    // Their multipliers exchange across the critical parameter mu=0.
    for mu in [-0.2_f64, 0.2_f64] {
        assert!((transcritical_map(0.0, mu)).abs() < 1.0e-12);
        assert!((transcritical_map(mu, mu) - mu).abs() < 1.0e-12);
    }

    let negative_mu = -0.2_f64;
    let positive_mu = 0.2_f64;

    assert!(transcritical_multiplier(0.0, negative_mu).abs() < 1.0);
    assert!(transcritical_multiplier(negative_mu, negative_mu).abs() > 1.0);
    assert!(transcritical_multiplier(0.0, positive_mu).abs() > 1.0);
    assert!(transcritical_multiplier(positive_mu, positive_mu).abs() < 1.0);

    assert_eq!(transcritical_map(0.0, 0.0), 0.0);
    assert!((transcritical_multiplier(0.0, 0.0) - 1.0).abs() < 1.0e-12);
}
