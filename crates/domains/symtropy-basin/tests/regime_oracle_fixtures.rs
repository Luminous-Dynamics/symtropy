//! Independent known-answer fixtures for CIVDYN regime qualification.
//!
//! These tests define small reference dynamics with analytically known
//! properties. They do not implement a general attractor detector and do not
//! establish that the production Living Basin model exhibits these regimes.
//! A future candidate classifier must consume these fixtures through a
//! separate evaluator rather than reuse the expected-answer functions as its
//! own oracle.

const BISTABLE_MU: f64 = 0.25;

fn stable_linear_step(x: f64) -> f64 {
    0.5 * x
}

fn unstable_linear_step(x: f64) -> f64 {
    1.1 * x
}

fn bistable_step(x: f64) -> f64 {
    x + BISTABLE_MU * x * (1.0 - x * x)
}

fn bistable_derivative(x: f64) -> f64 {
    1.0 + BISTABLE_MU - 3.0 * BISTABLE_MU * x * x
}

fn iterate<F>(mut state: f64, steps: usize, mut step: F) -> f64
where
    F: FnMut(f64) -> f64,
{
    for _ in 0..steps {
        state = step(state);
    }
    state
}

fn two_state_distribution_step(distribution: [f64; 2]) -> [f64; 2] {
    // Row-stochastic transition matrix:
    //   state 0 -> (0.8, 0.2)
    //   state 1 -> (0.4, 0.6)
    let [p0, p1] = distribution;
    [0.8 * p0 + 0.4 * p1, 0.2 * p0 + 0.6 * p1]
}

fn forced_state(phase: usize) -> f64 {
    const FORCING: [f64; 4] = [0.0, 1.0, 0.0, -1.0];
    FORCING[phase % FORCING.len()]
}

#[test]
fn stable_fixed_point_has_local_contraction() {
    let x0 = 0.25;
    let x40 = iterate(x0, 40, stable_linear_step);

    // x* = 0, derivative = 0.5, so perturbations contract geometrically.
    assert!(x40.abs() < 1.0e-12);
    assert!((stable_linear_step(0.0)).abs() < f64::EPSILON);
    assert!(stable_linear_step(x0).abs() < x0.abs());
}

#[test]
fn unstable_fixed_point_has_local_expansion() {
    let x0 = 1.0e-6;
    let x20 = iterate(x0, 20, unstable_linear_step);

    // x* = 0, derivative = 1.1 > 1.
    assert!(x20 > x0);
    assert!(x20 > x0 * 6.0);
    assert!((unstable_linear_step(0.0)).abs() < f64::EPSILON);
}

#[test]
fn bistable_map_has_two_stable_fixed_points_and_one_unstable_boundary() {
    // On [-1, 1], f(x) = x + mu*x*(1-x^2) has fixed points -1, 0, +1.
    // Derivatives are 0.5, 1.25, 0.5 respectively.
    assert!((bistable_derivative(-1.0) - 0.5).abs() < 1.0e-12);
    assert!((bistable_derivative(0.0) - 1.25).abs() < 1.0e-12);
    assert!((bistable_derivative(1.0) - 0.5).abs() < 1.0e-12);

    assert!((bistable_step(-1.0) + 1.0).abs() < 1.0e-12);
    assert!(bistable_step(0.0).abs() < 1.0e-12);
    assert!((bistable_step(1.0) - 1.0).abs() < 1.0e-12);

    let negative = iterate(-0.2, 500, bistable_step);
    let positive = iterate(0.2, 500, bistable_step);
    assert!((negative + 1.0).abs() < 1.0e-10);
    assert!((positive - 1.0).abs() < 1.0e-10);
}

#[test]
fn bistable_finite_grid_recovers_exact_symmetric_basin_fraction() {
    // The independent analytical oracle is sign(x0): the separatrix is x=0.
    // The grid is symmetric and excludes the boundary itself.
    let mut negative = 0usize;
    let mut positive = 0usize;

    for numerator in -10_000i32..=10_000i32 {
        if numerator == 0 {
            continue;
        }
        let initial = numerator as f64 / 10_000.0;
        let terminal = iterate(initial, 500, bistable_step);

        if (terminal + 1.0).abs() < 1.0e-8 {
            negative += 1;
        } else if (terminal - 1.0).abs() < 1.0e-8 {
            positive += 1;
        } else {
            panic!("initial state {initial} did not reach a known basin");
        }
    }

    assert_eq!(negative, 10_000);
    assert_eq!(positive, 10_000);
    assert_eq!(negative + positive, 20_000);
}

#[test]
fn finite_state_cycle_has_exact_period_four() {
    let next = |state: u8| (state + 1) % 4;
    let orbit: Vec<u8> = (0..=4).scan(0u8, |state, _| {
        let current = *state;
        *state = next(*state);
        Some(current)
    }).collect();

    assert_eq!(orbit, vec![0, 1, 2, 3, 0]);
    for state in 0..4u8 {
        assert_ne!(next(state), state);
    }
}

#[test]
fn stochastic_chain_has_known_stationary_distribution() {
    // Solve pi = pi P for P = [[0.8, 0.2], [0.4, 0.6]].
    // The unique stationary distribution is (2/3, 1/3).
    let stationary = [2.0 / 3.0, 1.0 / 3.0];
    let advanced = two_state_distribution_step(stationary);

    assert!((advanced[0] - stationary[0]).abs() < 1.0e-12);
    assert!((advanced[1] - stationary[1]).abs() < 1.0e-12);

    let mut distribution = [1.0, 0.0];
    for _ in 0..80 {
        distribution = two_state_distribution_step(distribution);
    }
    assert!((distribution[0] - stationary[0]).abs() < 1.0e-12);
    assert!((distribution[1] - stationary[1]).abs() < 1.0e-12);
}

#[test]
fn metastable_residence_times_are_not_absorbing_state_proofs() {
    // A two-state chain with transition probabilities a=0.01 and b=0.02.
    // Geometric residence times have means 1/a=100 and 1/b=50 steps.
    let leave_zero = 0.01_f64;
    let leave_one = 0.02_f64;
    let mean_residence_zero = 1.0 / leave_zero;
    let mean_residence_one = 1.0 / leave_one;

    assert!((mean_residence_zero - 100.0).abs() < 1.0e-12);
    assert!((mean_residence_one - 50.0).abs() < 1.0e-12);
    assert!(leave_zero > 0.0 && leave_one > 0.0);
    assert!(leave_zero < 1.0 && leave_one < 1.0);
}

#[test]
fn absorbing_state_is_invariant_after_entry() {
    let next = |state: u8| state.saturating_add(1).min(2);
    let mut state = 0u8;

    for _ in 0..10 {
        state = next(state);
    }
    assert_eq!(state, 2);
    assert_eq!(next(state), state);
    assert_eq!(next(next(state)), state);
}

#[test]
fn forced_periodicity_is_not_an_autonomous_one_dimensional_map() {
    // x_t is the externally imposed phase signal. The same observed state
    // x=0 occurs at phases 0 and 2, but its next state differs. Therefore no
    // single-valued autonomous map x[t+1] = g(x[t]) explains this trace.
    let phase_zero_next = forced_state(1);
    let phase_two_next = forced_state(3);

    assert_eq!(forced_state(0), 0.0);
    assert_eq!(forced_state(2), 0.0);
    assert_eq!(phase_zero_next, 1.0);
    assert_eq!(phase_two_next, -1.0);
    assert_ne!(phase_zero_next, phase_two_next);
}

#[test]
fn long_transient_plateau_does_not_establish_a_nonzero_attractor() {
    // x[t+1] = 0.999*x[t] has only the fixed point x=0.
    // Over 100 steps it still appears large; over 10,000 it decays near zero.
    let short = iterate(1.0, 100, |x| 0.999 * x);
    let long = iterate(1.0, 10_000, |x| 0.999 * x);

    assert!(short > 0.9);
    assert!(long < 1.0e-4);
    assert!(long >= 0.0);
}

#[test]
fn reference_fixture_inputs_are_finite_and_bounded() {
    for x in [-1.0, -0.5, 0.0, 0.5, 1.0] {
        let y = bistable_step(x);
        assert!(x.is_finite() && y.is_finite());
        assert!((-1.0..=1.0).contains(&y));
    }
}
