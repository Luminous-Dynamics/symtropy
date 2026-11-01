// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Small Rust reference for the Song–Fan–Ascher–Pai exact reduced-Coulomb
//! forward-backward-forward (FBF) splitting architecture.
//!
//! This is a test-only projected-gradient backend for the proximal product-cone
//! QP. It is an independent numerical candidate generator, not a production
//! solver and not a root-completeness proof. The acceptance check evaluates the
//! original unrelaxed De Saxce–Feng law.

type Matrix6 = [[f64; 6]; 6];
type Vector6 = [f64; 6];

const CONTACT_JACOBIAN: Matrix6 = [
    [1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
    [1.0, 0.0, 0.0, 0.0, 0.0, -1.0],
    [0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 1.0, 0.0, 0.0],
];

const INVERSE_MASS: Vector6 = [1.0, 1.0, 1.0, 1.0, 0.5, 1.0 / 3.0];
const FRICTION: f64 = 0.5;
const INNER_ITERATIONS: usize = 100;
const MAX_OUTER_ITERATIONS: usize = 400;
const MAX_STEP_REJECTIONS: usize = 16;
const LOCAL_VARIATION_LIMIT: f64 = 0.9;
const STEP_SHRINK: f64 = 0.7;
const OUTER_TOLERANCE: f64 = 1.0e-8;

#[derive(Clone, Debug)]
struct FbfOutcome {
    impulse: Vector6,
    residual: f64,
    outer_iterations: usize,
    step_rejections: usize,
    inner_iterations: usize,
    final_gamma: f64,
}

fn dot(lhs: &Vector6, rhs: &Vector6) -> f64 {
    (0..6).map(|index| lhs[index] * rhs[index]).sum()
}

fn norm(vector: &Vector6) -> f64 {
    dot(vector, vector).sqrt()
}

fn matrix_vector(matrix: &Matrix6, vector: &Vector6) -> Vector6 {
    std::array::from_fn(|row| {
        (0..6)
            .map(|column| matrix[row][column] * vector[column])
            .sum()
    })
}

fn add(lhs: &Vector6, rhs: &Vector6) -> Vector6 {
    std::array::from_fn(|index| lhs[index] + rhs[index])
}

fn subtract(lhs: &Vector6, rhs: &Vector6) -> Vector6 {
    std::array::from_fn(|index| lhs[index] - rhs[index])
}

fn scaled_add(lhs: &Vector6, scale: f64, rhs: &Vector6) -> Vector6 {
    std::array::from_fn(|index| lhs[index] + scale * rhs[index])
}

fn free_contact_velocity(generalized_velocity: &Vector6) -> Vector6 {
    std::array::from_fn(|row| {
        (0..6)
            .map(|column| CONTACT_JACOBIAN[row][column] * generalized_velocity[column])
            .sum()
    })
}

fn delassus_from_geometry() -> Matrix6 {
    let mut matrix = [[0.0; 6]; 6];
    for row in 0..6 {
        for column in 0..6 {
            for coordinate in 0..6 {
                matrix[row][column] += CONTACT_JACOBIAN[row][coordinate]
                    * INVERSE_MASS[coordinate]
                    * CONTACT_JACOBIAN[column][coordinate];
            }
        }
    }
    matrix
}

fn matrix_infinity_norm(matrix: &Matrix6) -> f64 {
    matrix
        .iter()
        .map(|row| row.iter().map(|value| value.abs()).sum::<f64>())
        .fold(0.0, f64::max)
}

fn project_contact_cone(point: [f64; 3], mu: f64) -> [f64; 3] {
    let [normal, tangent_one, tangent_two] = point;
    let tangent_norm = (tangent_one * tangent_one + tangent_two * tangent_two).sqrt();

    if mu == 0.0 {
        return [normal.max(0.0), 0.0, 0.0];
    }
    if normal >= 0.0 && tangent_norm <= mu * normal {
        return point;
    }
    if normal + mu * tangent_norm <= 0.0 {
        return [0.0; 3];
    }

    // Euclidean projection onto the circular Coulomb cone. The active-boundary
    // minimization is one-dimensional in the projected normal impulse.
    let projected_normal = (normal + mu * tangent_norm) / (1.0 + mu * mu);
    let tangent_scale = mu * projected_normal / tangent_norm;
    [
        projected_normal,
        tangent_scale * tangent_one,
        tangent_scale * tangent_two,
    ]
}

fn project_product_cone(vector: &Vector6, mu: f64) -> Vector6 {
    let first = project_contact_cone([vector[0], vector[1], vector[2]], mu);
    let second = project_contact_cone([vector[3], vector[4], vector[5]], mu);
    [
        first[0], first[1], first[2], second[0], second[1], second[2],
    ]
}

fn saxce_coupling(w: &Matrix6, q: &Vector6, impulse: &Vector6, mu: f64) -> Vector6 {
    let post_velocity = add(q, &matrix_vector(w, impulse));
    let first_tangent = post_velocity[1].hypot(post_velocity[2]);
    let second_tangent = post_velocity[4].hypot(post_velocity[5]);
    [
        mu * first_tangent,
        0.0,
        0.0,
        mu * second_tangent,
        0.0,
        0.0,
    ]
}

fn proximal_cone_qp(
    w: &Matrix6,
    q_linear: &Vector6,
    initial: &Vector6,
    gamma: f64,
    mu: f64,
) -> (Vector6, usize) {
    // min 0.5*x^T*(W + gamma^-1 I)*x + q_linear^T*x
    // subject to each contact impulse lying in its Coulomb cone.
    let w_gamma = std::array::from_fn(|row| {
        std::array::from_fn(|column| {
            w[row][column] + if row == column { 1.0 / gamma } else { 0.0 }
        })
    });
    let lipschitz = matrix_infinity_norm(&w_gamma);
    assert!(lipschitz.is_finite() && lipschitz > 0.0);
    let step = 1.0 / lipschitz;

    // FISTA is used only as the modular inner cone-QP backend. The proximal
    // matrix is SPD because W is PSD and gamma>0.
    let mut current = project_product_cone(initial, mu);
    let mut extrapolated = current;
    let mut acceleration = 1.0;
    for iteration in 0..INNER_ITERATIONS {
        let gradient = add(&matrix_vector(&w_gamma, &extrapolated), q_linear);
        let candidate = project_product_cone(
            &scaled_add(&extrapolated, -step, &gradient),
            mu,
        );
        if norm(&subtract(&candidate, &current))
            <= 1.0e-13 * (1.0 + norm(&current))
        {
            current = candidate;
            return (current, iteration + 1);
        }

        let next_acceleration =
            (1.0 + (1.0 + 4.0 * acceleration * acceleration).sqrt()) / 2.0;
        let momentum = (acceleration - 1.0) / next_acceleration;
        extrapolated = scaled_add(&candidate, momentum, &subtract(&candidate, &current));
        current = candidate;
        acceleration = next_acceleration;
    }
    (current, INNER_ITERATIONS)
}

fn original_reduced_law_residual(w: &Matrix6, q: &Vector6, impulse: &Vector6, mu: f64) -> f64 {
    let post_velocity = add(q, &matrix_vector(w, impulse));
    let augmented_velocity = [
        post_velocity[0] + mu * post_velocity[1].hypot(post_velocity[2]),
        post_velocity[1],
        post_velocity[2],
        post_velocity[3] + mu * post_velocity[4].hypot(post_velocity[5]),
        post_velocity[4],
        post_velocity[5],
    ];

    let mut primal_violation: f64 = 0.0;
    let mut dual_violation: f64 = 0.0;
    let mut orthogonality: f64 = 0.0;
    for contact in 0..2 {
        let offset = 3 * contact;
        let lambda = impulse[offset];
        let tangent_norm = impulse[offset + 1].hypot(impulse[offset + 2]);
        primal_violation = primal_violation
            .max((-lambda).max(0.0))
            .max((tangent_norm - mu * lambda).max(0.0));

        let augmented_normal = augmented_velocity[offset];
        let augmented_tangent =
            augmented_velocity[offset + 1].hypot(augmented_velocity[offset + 2]);
        dual_violation = dual_violation.max((mu * augmented_tangent - augmented_normal).max(0.0));

        let contact_gap = lambda * augmented_normal
            + impulse[offset + 1] * augmented_velocity[offset + 1]
            + impulse[offset + 2] * augmented_velocity[offset + 2];
        orthogonality = orthogonality.max(contact_gap.abs());
    }

    // Freeze the scales from q and the physical W; do not mix impulse and
    // velocity units in one unnormalized norm.
    let impulse_scale = (norm(q) / matrix_infinity_norm(w)).max(1.0e-12);
    let velocity_scale = norm(q).max(1.0e-12);
    let primal = primal_violation / impulse_scale;
    let dual = dual_violation / velocity_scale;
    let gap = orthogonality / (impulse_scale * velocity_scale);
    primal.max(dual).max(gap)
}

fn solve_fbf_reference(w: &Matrix6, q: &Vector6, mu: f64) -> Option<FbfOutcome> {
    if !mu.is_finite() || mu <= 0.0 {
        return None;
    }

    let lipschitz_bound = mu * matrix_infinity_norm(w);
    if !lipschitz_bound.is_finite() || lipschitz_bound <= 0.0 {
        return None;
    }

    // Conservative base step: gamma * L_B <= 0.8. The adaptive local-variation
    // guard below can shrink it, but never grows it within this solve.
    let mut gamma = 0.8 / lipschitz_bound;
    let mut impulse = [0.0; 6];
    let mut total_rejections = 0;
    let mut total_inner_iterations = 0;

    for outer_iteration in 1..=MAX_OUTER_ITERATIONS {
        let coupling = saxce_coupling(w, q, &impulse, mu);
        let mut accepted = None;

        for _ in 0..MAX_STEP_REJECTIONS {
            let q_linear = subtract(
                &add(q, &coupling),
                &std::array::from_fn(|index| impulse[index] / gamma),
            );
            let (intermediate, inner_iterations) =
                proximal_cone_qp(w, &q_linear, &impulse, gamma, mu);
            total_inner_iterations += inner_iterations;
            let next_coupling = saxce_coupling(w, q, &intermediate, mu);
            let state_change = norm(&subtract(&intermediate, &impulse));
            let coupling_change = norm(&subtract(&next_coupling, &coupling));
            let local_variation = if state_change <= 1.0e-14 {
                if coupling_change <= 1.0e-14 {
                    0.0
                } else {
                    f64::INFINITY
                }
            } else {
                gamma * coupling_change / state_change
            };

            if local_variation <= LOCAL_VARIATION_LIMIT {
                accepted = Some((intermediate, next_coupling));
                break;
            }
            gamma *= STEP_SHRINK;
            total_rejections += 1;
        }

        let (intermediate, next_coupling) = accepted?;
        let corrected = scaled_add(
            &intermediate,
            -gamma,
            &subtract(&next_coupling, &coupling),
        );
        let next_impulse = project_product_cone(&corrected, mu);
        let residual = original_reduced_law_residual(w, q, &next_impulse, mu);

        if residual <= OUTER_TOLERANCE {
            return Some(FbfOutcome {
                impulse: next_impulse,
                residual,
                outer_iterations: outer_iteration,
                step_rejections: total_rejections,
                inner_iterations: total_inner_iterations,
                final_gamma: gamma,
            });
        }
        impulse = next_impulse;
    }
    None
}

fn max_abs_difference(lhs: &Vector6, rhs: &Vector6) -> f64 {
    (0..6)
        .map(|index| (lhs[index] - rhs[index]).abs())
        .fold(0.0, f64::max)
}

#[test]
fn fbf_exact_reduced_coulomb_recovers_rank_deficient_sticking_response() {
    let w = delassus_from_geometry();
    let reference = [2.0, 0.0, 0.0, 2.0, 0.0, 0.0];
    let free_velocity = [-4.0, 0.0, 0.0, 0.0, 0.0, 2.0 / 3.0];
    let q = free_contact_velocity(&free_velocity);
    let expected_q = std::array::from_fn(|index| -matrix_vector(&w, &reference)[index]);
    assert!(max_abs_difference(&q, &expected_q) < 1.0e-14);

    let outcome = solve_fbf_reference(&w, &q, FRICTION)
        .expect("FBF projected-gradient reference should recover the sticking response");

    assert!(outcome.residual <= OUTER_TOLERANCE);
    assert!(max_abs_difference(&outcome.impulse, &reference) < 5.0e-5);
    assert!(outcome.outer_iterations < MAX_OUTER_ITERATIONS);
    assert!(outcome.final_gamma > 0.0);
    assert!(norm(&add(&q, &matrix_vector(&w, &outcome.impulse))) < 5.0e-5);
    assert!(outcome.step_rejections <= 8);
    assert!(outcome.inner_iterations >= outcome.outer_iterations);
}

#[test]
fn fbf_exact_reduced_coulomb_recovers_rank_deficient_sliding_response() {
    let w = delassus_from_geometry();
    let free_velocity = [-4.0, 3.0, 0.0, 0.0, 0.0, 2.0 / 3.0];
    let q = free_contact_velocity(&free_velocity);
    assert!(max_abs_difference(&q, &[-4.0, 3.0, 0.0, -14.0 / 3.0, 3.0, 0.0]) < 1.0e-14);
    let reference = [2.0, -1.0, 0.0, 2.0, -1.0, 0.0];

    let outcome = solve_fbf_reference(&w, &q, FRICTION)
        .expect("FBF projected-gradient reference should recover the sliding response");

    assert!(outcome.residual <= OUTER_TOLERANCE);
    assert!(max_abs_difference(&outcome.impulse, &reference) < 5.0e-5);
    assert!(outcome.outer_iterations < MAX_OUTER_ITERATIONS);
    assert!(outcome.final_gamma > 0.0);
    assert!(outcome.step_rejections <= 8);
    assert!(outcome.inner_iterations >= outcome.outer_iterations);

    let post_velocity = add(&q, &matrix_vector(&w, &outcome.impulse));
    for contact in 0..2 {
        let offset = 3 * contact;
        assert!(post_velocity[offset].abs() < 5.0e-5);
        assert!(post_velocity[offset + 1] > 0.99);
        assert!(post_velocity[offset + 2].abs() < 5.0e-5);
    }
}
#[test]
fn circular_coulomb_cone_projection_matches_closed_form_cases() {
    assert_eq!(
        project_contact_cone([1.0, 0.1, 0.1], FRICTION),
        [1.0, 0.1, 0.1],
        "an interior point is unchanged"
    );
    assert_eq!(
        project_contact_cone([-1.0, 0.0, 0.0], FRICTION),
        [0.0; 3],
        "a point in the polar cone projects to the apex"
    );
    let boundary = project_contact_cone([0.0, 1.0, 0.0], FRICTION);
    assert!((boundary[0] - 0.4).abs() < 1.0e-14);
    assert!((boundary[1] - 0.2).abs() < 1.0e-14);
    assert_eq!(boundary[2], 0.0);
    assert!((boundary[1].hypot(boundary[2]) - FRICTION * boundary[0]).abs() < 1.0e-14);
}

