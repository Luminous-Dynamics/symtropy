// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Exact rational reference profile for robust descent over branch-gradient sets.
//!
//! This is a test-only research witness. It deliberately separates:
//! - a grid proposer, which searches only a declared finite rational direction grid;
//! - an exact verifier, which checks every branch support bound;
//! - an exact convex-combination witness for no strict common descent on a
//!   complete finite gradient set.
//!
//! A failed grid search is INCONCLUSIVE. It is not an infeasibility proof.

use num_bigint::BigInt;
use num_rational::BigRational;

const INPUT_DIMENSION: usize = 6;

type RationalVector = [BigRational; INPUT_DIMENSION];
type GradientBox = [RationalInterval; INPUT_DIMENSION];

#[derive(Clone, Debug, PartialEq, Eq)]
struct RationalInterval {
    lower: BigRational,
    upper: BigRational,
}

impl RationalInterval {
    fn new(lower: BigRational, upper: BigRational) -> Self {
        assert!(lower <= upper);
        Self { lower, upper }
    }

    fn point(value: BigRational) -> Self {
        Self {
            lower: value.clone(),
            upper: value,
        }
    }

    /// Exact upper support of this interval in the supplied scalar direction.
    fn support_upper(&self, direction: &BigRational) -> BigRational {
        let lower_product = &self.lower * direction;
        let upper_product = &self.upper * direction;
        lower_product.max(upper_product)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DescentCandidate {
    direction: RationalVector,
    margin: BigRational,
    branch_upper_bounds: Vec<BigRational>,
}

fn rational(value: i64) -> BigRational {
    BigRational::from_integer(BigInt::from(value))
}

fn rational_fraction(numerator: i64, denominator: i64) -> BigRational {
    assert!(denominator > 0);
    BigRational::new(BigInt::from(numerator), BigInt::from(denominator))
}

fn point_gradient(values: [i64; INPUT_DIMENSION]) -> GradientBox {
    std::array::from_fn(|index| RationalInterval::point(rational(values[index])))
}

fn interval_gradient(values: [(i64, i64); INPUT_DIMENSION]) -> GradientBox {
    std::array::from_fn(|index| {
        RationalInterval::new(rational(values[index].0), rational(values[index].1))
    })
}

fn gradient_box_support_upper(gradient: &GradientBox, direction: &RationalVector) -> BigRational {
    gradient
        .iter()
        .zip(direction.iter())
        .map(|(component, coefficient)| component.support_upper(coefficient))
        .fold(rational(0), |sum, value| sum + value)
}

/// Verify a strict common-descent certificate for all declared gradient boxes.
///
/// direction is an input-space direction and must lie in the l-infinity unit
/// box. Every interval is a conservative enclosure; success proves descent for
/// the complete declared boxes, while failure says only that this certificate
/// did not verify.
fn verify_robust_descent(
    branches: &[GradientBox],
    direction: &RationalVector,
    margin: &BigRational,
) -> bool {
    if branches.is_empty() || margin <= &rational(0) {
        return false;
    }

    let one = rational(1);
    if direction
        .iter()
        .any(|coefficient| coefficient < &-one.clone() || coefficient > &one)
    {
        return false;
    }

    let required_upper = -margin.clone();
    branches
        .iter()
        .all(|gradient| gradient_box_support_upper(gradient, direction) <= required_upper)
}

fn direction_l1_norm(direction: &RationalVector) -> BigRational {
    direction.iter().fold(rational(0), |sum, value| {
        let magnitude = if value < &rational(0) {
            -value.clone()
        } else {
            value.clone()
        };
        sum + magnitude
    })
}

const LP_VARIABLE_COUNT: usize = INPUT_DIMENSION + 1;
const MAX_ACTIVE_SETS: usize = 250_000;
const CORNERS_PER_GRADIENT_BOX: usize = 1 << INPUT_DIMENSION;

type LpVector = [BigRational; LP_VARIABLE_COUNT];

#[derive(Clone, Debug, PartialEq, Eq)]
struct LpConstraint {
    coefficients: LpVector,
    upper_bound: BigRational,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExactLpOptimum {
    direction: RationalVector,
    margin: BigRational,
    branch_upper_bounds: Vec<BigRational>,
    gradient_cuts: Vec<RationalVector>,
    gradient_cut_owners: Vec<usize>,
    dual_constraint_indices: Vec<usize>,
    dual_multipliers: Vec<BigRational>,
}

fn robust_descent_constraints(gradients: &[RationalVector]) -> Vec<LpConstraint> {
    let mut constraints = Vec::with_capacity(gradients.len() + 2 * INPUT_DIMENSION);

    for gradient in gradients {
        let coefficients = std::array::from_fn(|index| {
            if index == INPUT_DIMENSION {
                rational(1)
            } else {
                gradient[index].clone()
            }
        });
        constraints.push(LpConstraint {
            coefficients,
            upper_bound: rational(0),
        });
    }

    for coordinate in 0..INPUT_DIMENSION {
        let upper: LpVector = std::array::from_fn(|index| {
            if index == coordinate {
                rational(1)
            } else {
                rational(0)
            }
        });
        let lower: LpVector = std::array::from_fn(|index| {
            if index == coordinate {
                rational(-1)
            } else {
                rational(0)
            }
        });
        constraints.push(LpConstraint {
            coefficients: upper,
            upper_bound: rational(1),
        });
        constraints.push(LpConstraint {
            coefficients: lower,
            upper_bound: rational(1),
        });
    }

    constraints
}

/// Count n-choose-k, returning None when the result exceeds the supplied cap.
fn bounded_binomial(n: usize, k: usize, cap: usize) -> Option<usize> {
    if k > n {
        return Some(0);
    }
    let k = k.min(n - k);
    let mut value = 1_usize;
    for index in 1..=k {
        value = value.checked_mul(n - k + index)? / index;
        if value > cap {
            return None;
        }
    }
    Some(value)
}

#[allow(clippy::needless_range_loop)]
fn solve_exact_rational_system(
    mut matrix: Vec<Vec<BigRational>>,
    mut right_hand_side: Vec<BigRational>,
) -> Option<Vec<BigRational>> {
    let dimension = matrix.len();
    if right_hand_side.len() != dimension || matrix.iter().any(|row| row.len() != dimension) {
        return None;
    }
    let zero = rational(0);

    // Exact Gauss-Jordan elimination. Singular systems are rejected.
    for pivot_column in 0..dimension {
        let pivot_row = (pivot_column..dimension).find(|row| matrix[*row][pivot_column] != zero)?;
        if pivot_row != pivot_column {
            matrix.swap(pivot_row, pivot_column);
            right_hand_side.swap(pivot_row, pivot_column);
        }

        let pivot = matrix[pivot_column][pivot_column].clone();
        for column in pivot_column..dimension {
            matrix[pivot_column][column] /= pivot.clone();
        }
        right_hand_side[pivot_column] /= pivot;

        let pivot_values = matrix[pivot_column].clone();
        let pivot_rhs = right_hand_side[pivot_column].clone();
        for row in 0..dimension {
            if row == pivot_column {
                continue;
            }
            let factor = matrix[row][pivot_column].clone();
            if factor == zero {
                continue;
            }
            for column in pivot_column..dimension {
                matrix[row][column] -= factor.clone() * pivot_values[column].clone();
            }
            right_hand_side[row] -= factor * pivot_rhs.clone();
        }
    }

    Some(right_hand_side)
}

fn solve_active_lp_constraints(
    constraints: &[LpConstraint],
    active_indices: &[usize],
) -> Option<LpVector> {
    if active_indices.len() != LP_VARIABLE_COUNT {
        return None;
    }

    let matrix = active_indices
        .iter()
        .map(|index| constraints[*index].coefficients.to_vec())
        .collect();
    let right_hand_side = active_indices
        .iter()
        .map(|index| constraints[*index].upper_bound.clone())
        .collect();

    solve_exact_rational_system(matrix, right_hand_side)?
        .try_into()
        .ok()
}

/// Solve A_active^T lambda = e_margin. Nonnegative multipliers are an exact
/// dual certificate for the active-set primal vertex.
fn solve_active_lp_dual_constraints(
    constraints: &[LpConstraint],
    active_indices: &[usize],
) -> Option<LpVector> {
    if active_indices.len() != LP_VARIABLE_COUNT
        || active_indices
            .iter()
            .any(|index| *index >= constraints.len())
    {
        return None;
    }

    let matrix: Vec<Vec<BigRational>> = (0..LP_VARIABLE_COUNT)
        .map(|variable| {
            active_indices
                .iter()
                .map(|index| constraints[*index].coefficients[variable].clone())
                .collect()
        })
        .collect();
    let right_hand_side = (0..LP_VARIABLE_COUNT)
        .map(|variable| {
            if variable == INPUT_DIMENSION {
                rational(1)
            } else {
                rational(0)
            }
        })
        .collect();
    let multipliers: LpVector = solve_exact_rational_system(matrix, right_hand_side)?
        .try_into()
        .ok()?;
    multipliers
        .iter()
        .all(|multiplier| multiplier >= &rational(0))
        .then_some(multipliers)
}

#[allow(clippy::needless_range_loop)]
fn verify_exact_lp_certificate(
    constraints: &[LpConstraint],
    active_indices: &[usize],
    dual_multipliers: &[BigRational],
    candidate: &LpVector,
) -> bool {
    if !lp_constraints_satisfied(constraints, candidate)
        || active_indices.len() != LP_VARIABLE_COUNT
        || dual_multipliers.len() != LP_VARIABLE_COUNT
        || active_indices
            .iter()
            .any(|index| *index >= constraints.len())
        || dual_multipliers.iter().any(|value| value < &rational(0))
    {
        return false;
    }

    // Dual stationarity: A^T lambda = e_margin.
    for variable in 0..LP_VARIABLE_COUNT {
        let weighted_coefficient = active_indices.iter().zip(dual_multipliers.iter()).fold(
            rational(0),
            |sum, (index, multiplier)| {
                sum + &constraints[*index].coefficients[variable] * multiplier
            },
        );
        let required = if variable == INPUT_DIMENSION {
            rational(1)
        } else {
            rational(0)
        };
        if weighted_coefficient != required {
            return false;
        }
    }

    // Complementary slackness on positive multipliers.
    for (index, multiplier) in active_indices.iter().zip(dual_multipliers.iter()) {
        if multiplier > &rational(0) {
            let constraint = &constraints[*index];
            let value = constraint
                .coefficients
                .iter()
                .zip(candidate.iter())
                .fold(rational(0), |sum, (coefficient, variable)| {
                    sum + coefficient * variable
                });
            if value != constraint.upper_bound {
                return false;
            }
        }
    }

    // Exact primal/dual objective equality.
    let dual_objective = active_indices
        .iter()
        .zip(dual_multipliers.iter())
        .fold(rational(0), |sum, (index, multiplier)| {
            sum + &constraints[*index].upper_bound * multiplier
        });
    dual_objective == candidate[INPUT_DIMENSION]
}

fn lp_constraints_satisfied(constraints: &[LpConstraint], candidate: &LpVector) -> bool {
    constraints.iter().all(|constraint| {
        let value = constraint
            .coefficients
            .iter()
            .zip(candidate.iter())
            .fold(rational(0), |sum, (coefficient, variable)| {
                sum + coefficient * variable
            });
        value <= constraint.upper_bound
    })
}

fn enumerate_lp_active_sets(
    constraints: &[LpConstraint],
    gradients: &[RationalVector],
    start_index: usize,
    active_indices: &mut Vec<usize>,
    best: &mut Option<ExactLpOptimum>,
) {
    if active_indices.len() == LP_VARIABLE_COUNT {
        let Some(candidate) = solve_active_lp_constraints(constraints, active_indices) else {
            return;
        };
        let Some(dual_multipliers) = solve_active_lp_dual_constraints(constraints, active_indices)
        else {
            return;
        };
        if !verify_exact_lp_certificate(constraints, active_indices, &dual_multipliers, &candidate)
        {
            return;
        }

        let margin = candidate[INPUT_DIMENSION].clone();
        let direction: RationalVector = std::array::from_fn(|index| candidate[index].clone());
        let should_replace = best.as_ref().is_none_or(|previous| {
            margin > previous.margin
                || (margin == previous.margin
                    && direction_l1_norm(&direction) < direction_l1_norm(&previous.direction))
        });
        if should_replace {
            let branch_upper_bounds = gradients
                .iter()
                .map(|gradient| {
                    gradient
                        .iter()
                        .zip(direction.iter())
                        .fold(rational(0), |sum, (coefficient, value)| {
                            sum + coefficient * value
                        })
                })
                .collect::<Vec<_>>();
            *best = Some(ExactLpOptimum {
                direction,
                margin,
                branch_upper_bounds,
                gradient_cuts: gradients.to_vec(),
                gradient_cut_owners: (0..gradients.len()).collect(),
                dual_constraint_indices: active_indices.to_vec(),
                dual_multipliers: dual_multipliers.to_vec(),
            });
        }
        return;
    }

    let remaining = LP_VARIABLE_COUNT - active_indices.len();
    if constraints.len().saturating_sub(start_index) < remaining {
        return;
    }
    let last_start = constraints.len() - remaining;
    for (index, _) in constraints
        .iter()
        .enumerate()
        .take(last_start + 1)
        .skip(start_index)
    {
        active_indices.push(index);
        enumerate_lp_active_sets(constraints, gradients, index + 1, active_indices, best);
        active_indices.pop();
    }
}

fn verify_robust_descent_interval_lp_certificate(
    branches: &[GradientBox],
    optimum: &ExactLpOptimum,
) -> bool {
    if branches.is_empty()
        || optimum.gradient_cuts.len() != optimum.gradient_cut_owners.len()
        || optimum.branch_upper_bounds.len() != branches.len()
    {
        return false;
    }

    let required_upper = -optimum.margin.clone();
    let worst_support = optimum.branch_upper_bounds.iter().max();
    if worst_support != Some(&required_upper) {
        return false;
    }

    // Each generated point-gradient cut must be a member of its declared
    // interval box, so the cut constraint is implied by the full robust box.
    for (gradient_cut, owner) in optimum
        .gradient_cuts
        .iter()
        .zip(optimum.gradient_cut_owners.iter())
    {
        let Some(branch) = branches.get(*owner) else {
            return false;
        };
        if gradient_cut
            .iter()
            .zip(branch.iter())
            .any(|(value, interval)| value < &interval.lower || value > &interval.upper)
        {
            return false;
        }
    }

    let constraints = robust_descent_constraints(&optimum.gradient_cuts);
    let candidate: LpVector = std::array::from_fn(|index| {
        if index == INPUT_DIMENSION {
            optimum.margin.clone()
        } else {
            optimum.direction[index].clone()
        }
    });
    verify_exact_lp_certificate(
        &constraints,
        &optimum.dual_constraint_indices,
        &optimum.dual_multipliers,
        &candidate,
    ) && branches
        .iter()
        .all(|branch| gradient_box_support_upper(branch, &optimum.direction) <= required_upper)
}

fn adverse_box_corner(gradient: &GradientBox, direction: &RationalVector) -> RationalVector {
    std::array::from_fn(|coordinate| {
        if direction[coordinate] < rational(0) {
            gradient[coordinate].lower.clone()
        } else {
            gradient[coordinate].upper.clone()
        }
    })
}

/// Solve the robust LP over independent rational interval boxes by exact
/// constraint generation. Each point-gradient LP gives an upper bound on the
/// full robust optimum; the support oracle either verifies that candidate for
/// every box or returns an adverse box corner as a violated constraint.
/// Finite termination with no violated corner proves the robust optimum for
/// the complete declared boxes. A cap or repeated violated cut returns None.
fn solve_robust_descent_interval_lp(branches: &[GradientBox]) -> Option<ExactLpOptimum> {
    if branches.is_empty() {
        return None;
    }

    let mut cuts: Vec<RationalVector> = branches
        .iter()
        .map(|gradient| std::array::from_fn(|coordinate| gradient[coordinate].lower.clone()))
        .collect();
    let mut cut_owners: Vec<usize> = (0..branches.len()).collect();

    // Each box has at most 2^6 distinct rational vertices. Every
    // nonterminal iteration adds at least one previously unseen adverse
    // corner, so this bound is sufficient for finite termination.
    let max_cut_iterations = CORNERS_PER_GRADIENT_BOX
        .checked_mul(branches.len())?
        .saturating_add(1);
    for _ in 0..max_cut_iterations {
        let mut optimum = solve_robust_descent_lp(&cuts)?;
        let required_upper = -optimum.margin.clone();
        let mut new_cuts = Vec::new();

        for (branch_index, gradient) in branches.iter().enumerate() {
            let support_upper = gradient_box_support_upper(gradient, &optimum.direction);
            if support_upper > required_upper {
                let corner = adverse_box_corner(gradient, &optimum.direction);
                if cuts.contains(&corner)
                    || new_cuts.iter().any(|(candidate, _)| *candidate == corner)
                {
                    // A previously enforced exact corner cannot violate its
                    // own LP constraint; treat this as an internal inconsistency.
                    return None;
                }
                new_cuts.push((corner, branch_index));
            }
        }

        if new_cuts.is_empty() {
            optimum.branch_upper_bounds = branches
                .iter()
                .map(|gradient| gradient_box_support_upper(gradient, &optimum.direction))
                .collect();
            optimum.gradient_cut_owners = cut_owners;
            return verify_robust_descent_interval_lp_certificate(branches, &optimum)
                .then_some(optimum);
        }

        for (corner, owner) in new_cuts {
            cuts.push(corner);
            cut_owners.push(owner);
        }
    }

    None
}

/// Solve the finite point-gradient robust-descent LP exactly over the
/// six-dimensional L-infinity trust box:
///
///     maximize margin
///     subject to gradient_i dot direction <= -margin for every branch,
///                -1 <= direction_j <= 1.
///
/// The implementation enumerates all nonsingular seven-constraint active sets
/// and verifies every candidate against every LP constraint. It is a small
/// reference solver, not a large-scale simplex implementation. To avoid
/// accidental combinatorial blow-ups it returns None when the active-set count
/// exceeds MAX_ACTIVE_SETS. None is an abstention, never an infeasibility proof.
fn solve_robust_descent_lp(gradients: &[RationalVector]) -> Option<ExactLpOptimum> {
    if gradients.is_empty() {
        return None;
    }

    let constraints = robust_descent_constraints(gradients);
    bounded_binomial(constraints.len(), LP_VARIABLE_COUNT, MAX_ACTIVE_SETS)?;

    let mut best = None;
    enumerate_lp_active_sets(
        &constraints,
        gradients,
        0,
        &mut Vec::with_capacity(LP_VARIABLE_COUNT),
        &mut best,
    );
    let mut optimum = best?;
    // Canonicalize coordinates that no declared gradient observes. Their
    // values cannot affect any branch derivative or the optimum margin.
    for coordinate in 0..INPUT_DIMENSION {
        if gradients
            .iter()
            .all(|gradient| gradient[coordinate] == rational(0))
        {
            optimum.direction[coordinate] = rational(0);
        }
    }
    optimum.branch_upper_bounds = gradients
        .iter()
        .map(|gradient| {
            gradient
                .iter()
                .zip(optimum.direction.iter())
                .fold(rational(0), |sum, (coefficient, value)| {
                    sum + coefficient * value
                })
        })
        .collect();
    let candidate: LpVector = std::array::from_fn(|index| {
        if index == INPUT_DIMENSION {
            optimum.margin.clone()
        } else {
            optimum.direction[index].clone()
        }
    });
    let certificate_verified = verify_exact_lp_certificate(
        &constraints,
        &optimum.dual_constraint_indices,
        &optimum.dual_multipliers,
        &candidate,
    );
    certificate_verified.then_some(optimum)
}

fn candidate_for_direction(
    branches: &[GradientBox],
    direction: RationalVector,
) -> Option<DescentCandidate> {
    let branch_upper_bounds: Vec<BigRational> = branches
        .iter()
        .map(|gradient| gradient_box_support_upper(gradient, &direction))
        .collect();
    let worst_branch_upper = branch_upper_bounds.iter().max()?.clone();
    let margin = -worst_branch_upper;
    if margin <= rational(0) {
        return None;
    }
    Some(DescentCandidate {
        direction,
        margin,
        branch_upper_bounds,
    })
}

fn search_grid_recursive(
    branches: &[GradientBox],
    grid_values: &[BigRational],
    coordinate: usize,
    current: &mut Vec<BigRational>,
    best: &mut Option<DescentCandidate>,
) {
    if coordinate == INPUT_DIMENSION {
        let direction: RationalVector = current
            .clone()
            .try_into()
            .expect("the grid recursion emits exactly six coordinates");
        if let Some(candidate) = candidate_for_direction(branches, direction) {
            let improves = best.as_ref().is_none_or(|previous| {
                candidate.margin > previous.margin
                    || (candidate.margin == previous.margin
                        && direction_l1_norm(&candidate.direction)
                            < direction_l1_norm(&previous.direction))
            });
            if improves {
                *best = Some(candidate);
            }
        }
        return;
    }

    for value in grid_values {
        current.push(value.clone());
        search_grid_recursive(branches, grid_values, coordinate + 1, current, best);
        current.pop();
    }
}

/// Find the best verified direction on the rational grid -1..=1 divided by
/// grid_denominator. This is a deterministic finite-grid proposer, not a
/// continuous LP solver. None means no positive-margin direction was found on
/// this grid; it does not prove that no common descent direction exists.
fn search_rational_grid(
    branches: &[GradientBox],
    grid_denominator: i64,
) -> Option<DescentCandidate> {
    if branches.is_empty() || grid_denominator <= 0 {
        return None;
    }

    let denominator = BigInt::from(grid_denominator);
    let grid_values: Vec<BigRational> = (-grid_denominator..=grid_denominator)
        .map(|numerator| BigRational::new(BigInt::from(numerator), denominator.clone()))
        .collect();

    let mut best = None;
    search_grid_recursive(
        branches,
        &grid_values,
        0,
        &mut Vec::with_capacity(INPUT_DIMENSION),
        &mut best,
    );
    best
}

/// Validate simplex weights and construct their exact convex combination.
fn convex_combination(
    gradients: &[RationalVector],
    weights: &[BigRational],
) -> Option<RationalVector> {
    if gradients.is_empty() || gradients.len() != weights.len() {
        return None;
    }
    let zero = rational(0);
    let one = rational(1);
    if weights.iter().any(|weight| weight < &zero)
        || weights
            .iter()
            .fold(zero.clone(), |sum, weight| sum + weight)
            != one
    {
        return None;
    }

    Some(std::array::from_fn(|coordinate| {
        gradients
            .iter()
            .zip(weights.iter())
            .fold(zero.clone(), |sum, (gradient, weight)| {
                sum + &gradient[coordinate] * weight
            })
    }))
}

/// Exact dual upper bound on the max-min descent margin for a finite set
/// of point gradients: ||sum_i lambda_i g_i||_1, with lambda in the simplex.
fn convex_combination_l1_upper_bound(
    gradients: &[RationalVector],
    weights: &[BigRational],
) -> Option<BigRational> {
    let combination = convex_combination(gradients, weights)?;
    Some(combination.iter().fold(rational(0), |sum, value| {
        if value < &rational(0) {
            sum - value
        } else {
            sum + value
        }
    }))
}

/// Verify a Farkas/Gordan witness that zero is in the convex hull of a complete
/// finite gradient set. Such a witness rules out a direction with strictly
/// negative directional derivative on every gradient in that set.
fn verify_zero_convex_combination(gradients: &[RationalVector], weights: &[BigRational]) -> bool {
    convex_combination(gradients, weights)
        .is_some_and(|combination| combination.iter().all(|value| value == &rational(0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_synthesis_finds_common_descent_when_coordinate_axes_conflict() {
        // Each coordinate-axis direction helps one branch and hurts the other,
        // but the combined rational direction (-1/3, -1, 0, ...) improves both.
        let branches = vec![
            point_gradient([10, -1, 0, 0, 0, 0]),
            point_gradient([-2, 3, 0, 0, 0, 0]),
        ];

        // The mean gradient is (4,1). Its normalized negative direction
        // (-1,-1/4) has derivative +5/4 on branch 2, so it is not robust.
        let mean_direction = [
            rational(-1),
            rational_fraction(-1, 4),
            rational(0),
            rational(0),
            rational(0),
            rational(0),
        ];
        assert!(gradient_box_support_upper(&branches[1], &mean_direction) > rational(0));
        assert!(!verify_robust_descent(
            &branches,
            &mean_direction,
            &rational_fraction(1, 100)
        ));

        // Each sign of either coordinate-axis direction improves one
        // branch only; the other branch has a strictly positive derivative.
        for (axis, _) in [0, 1].iter().enumerate() {
            for sign in [-1, 1] {
                let axis_direction: RationalVector = std::array::from_fn(|coordinate| {
                    if coordinate == axis {
                        rational(sign)
                    } else {
                        rational(0)
                    }
                });
                assert!(branches.iter().any(|gradient| {
                    gradient_box_support_upper(gradient, &axis_direction) > rational(0)
                }));
            }
        }

        let candidate = search_rational_grid(&branches, 3)
            .expect("the rational grid contains a common descent direction");
        assert_eq!(
            candidate.direction,
            [
                rational_fraction(-1, 3),
                rational(-1),
                rational(0),
                rational(0),
                rational(0),
                rational(0)
            ]
        );
        assert_eq!(candidate.margin, rational_fraction(7, 3));
        assert_eq!(
            candidate.branch_upper_bounds,
            vec![rational_fraction(-7, 3), rational_fraction(-7, 3)]
        );

        // A matching exact dual convex combination proves the margin is
        // globally optimal for this complete two-gradient point set.
        let point_gradients = vec![
            [
                rational(10),
                rational(-1),
                rational(0),
                rational(0),
                rational(0),
                rational(0),
            ],
            [
                rational(-2),
                rational(3),
                rational(0),
                rational(0),
                rational(0),
                rational(0),
            ],
        ];
        let lp_optimum = solve_robust_descent_lp(&point_gradients)
            .expect("the exact active-set LP returns a continuous optimum");
        assert_eq!(lp_optimum.direction, candidate.direction);
        assert_eq!(lp_optimum.margin, rational_fraction(7, 3));
        assert_eq!(
            lp_optimum.branch_upper_bounds,
            vec![rational_fraction(-7, 3), rational_fraction(-7, 3)]
        );

        // Independently replay the returned primal/dual certificate, then
        // corrupt one multiplier and require the verifier to reject it.
        let point_constraints = robust_descent_constraints(&point_gradients);
        let point_candidate: LpVector = std::array::from_fn(|index| {
            if index == INPUT_DIMENSION {
                lp_optimum.margin.clone()
            } else {
                lp_optimum.direction[index].clone()
            }
        });
        assert!(verify_exact_lp_certificate(
            &point_constraints,
            &lp_optimum.dual_constraint_indices,
            &lp_optimum.dual_multipliers,
            &point_candidate
        ));
        let mut corrupted_dual = lp_optimum.dual_multipliers.clone();
        corrupted_dual[0] += rational(1);
        assert!(!verify_exact_lp_certificate(
            &point_constraints,
            &lp_optimum.dual_constraint_indices,
            &corrupted_dual,
            &point_candidate
        ));

        let dual_bound = convex_combination_l1_upper_bound(
            &point_gradients,
            &[rational_fraction(1, 6), rational_fraction(5, 6)],
        )
        .expect("the dual weights form an exact simplex point");
        assert_eq!(dual_bound, lp_optimum.margin);
        assert_eq!(lp_optimum.margin, candidate.margin);
        assert!(verify_robust_descent(
            &branches,
            &candidate.direction,
            &candidate.margin
        ));
    }

    #[test]
    fn exact_interval_lp_uses_cut_generation_and_certifies_the_complete_box() {
        // One gradient box has a positive robust descent direction. Its lower
        // corner alone is an incomplete relaxation; the exact support oracle
        // adds the adverse upper endpoint when a proposed direction needs it.
        let positive_box = interval_gradient([(1, 2), (-1, 0), (0, 0), (0, 0), (0, 0), (0, 0)]);
        let positive = solve_robust_descent_interval_lp(&[positive_box])
            .expect("the exact interval LP certifies a common descent direction");
        assert_eq!(positive.margin, rational(1));
        assert_eq!(positive.branch_upper_bounds, vec![rational(-1)]);
        assert!(verify_robust_descent_interval_lp_certificate(
            &[interval_gradient([
                (1, 2),
                (-1, 0),
                (0, 0),
                (0, 0),
                (0, 0),
                (0, 0)
            ])],
            &positive
        ));
        let mut corrupted_positive = positive.clone();
        corrupted_positive.margin += rational(1);
        assert!(!verify_robust_descent_interval_lp_certificate(
            &[interval_gradient([
                (1, 2),
                (-1, 0),
                (0, 0),
                (0, 0),
                (0, 0),
                (0, 0)
            ])],
            &corrupted_positive
        ));
        assert_eq!(
            gradient_box_support_upper(
                &interval_gradient([(1, 2), (-1, 0), (0, 0), (0, 0), (0, 0), (0, 0),]),
                &positive.direction
            ),
            rational(-1)
        );

        // If the complete interval contains opposite gradients, the cutting
        // plane loop must add both adverse endpoints and return margin zero.
        // Their exact half/half convex combination is a no-strict-descent
        // witness for this complete one-dimensional uncertainty box.
        let opposing_box = interval_gradient([(-1, 1), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]);
        let no_descent = solve_robust_descent_interval_lp(std::slice::from_ref(&opposing_box))
            .expect("the exact interval LP returns the optimum even when it is zero");
        assert_eq!(no_descent.margin, rational(0));
        assert_eq!(no_descent.branch_upper_bounds, vec![rational(0)]);
        assert!(verify_robust_descent_interval_lp_certificate(
            &[opposing_box],
            &no_descent
        ));
        assert_eq!(no_descent.gradient_cuts.len(), 2);
        assert!(verify_zero_convex_combination(
            &no_descent.gradient_cuts,
            &[rational_fraction(1, 2), rational_fraction(1, 2)]
        ));
    }

    #[test]
    fn interval_support_uses_the_adverse_endpoint_for_each_direction_sign() {
        let gradient = interval_gradient([(-2, -1), (3, 5), (0, 0), (0, 0), (0, 0), (0, 0)]);
        let direction = [
            rational(-1),
            rational(1),
            rational(0),
            rational(0),
            rational(0),
            rational(0),
        ];

        // (-2,-1)*(-1) has upper support 2; (3,5)*(+1) has upper support 5.
        assert_eq!(
            gradient_box_support_upper(&gradient, &direction),
            rational(7)
        );
    }

    #[test]
    fn exact_convex_combination_proves_no_strict_common_descent() {
        let gradients = vec![
            [
                rational(1),
                rational(0),
                rational(0),
                rational(0),
                rational(0),
                rational(0),
            ],
            [
                rational(-1),
                rational(0),
                rational(0),
                rational(0),
                rational(0),
                rational(0),
            ],
        ];
        let weights = vec![rational_fraction(1, 2), rational_fraction(1, 2)];

        assert!(verify_zero_convex_combination(&gradients, &weights));
        assert_eq!(
            convex_combination_l1_upper_bound(&gradients, &weights),
            Some(rational(0))
        );
        assert!(!verify_zero_convex_combination(
            &gradients,
            &[rational_fraction(1, 3), rational_fraction(2, 3)]
        ));
        assert!(
            search_rational_grid(
                &gradients
                    .iter()
                    .map(|gradient| std::array::from_fn(|index| {
                        RationalInterval::point(gradient[index].clone())
                    }))
                    .collect::<Vec<_>>(),
                1
            )
            .is_none()
        );
    }

    #[test]
    fn a_failed_complete_box_search_is_not_a_pointwise_infeasibility_certificate() {
        // The complete gradient box contains zero in every coordinate, so
        // no nonzero direction can be certified as strictly descending for
        // every gradient in the box.
        let branches = vec![interval_gradient([(-1, 1); INPUT_DIMENSION])];
        assert!(search_rational_grid(&branches, 1).is_none());

        // However, a particular gradient inside the same box has a descent
        // direction. The box-level verifier must not confuse failure to
        // certify the whole set with pointwise infeasibility.
        let candidate = [
            rational(-1),
            rational(0),
            rational(0),
            rational(0),
            rational(0),
            rational(0),
        ];
        assert!(!verify_robust_descent(
            &branches,
            &candidate,
            &rational_fraction(1, 100)
        ));

        let one_point = point_gradient([1, 0, 0, 0, 0, 0]);
        assert_eq!(
            gradient_box_support_upper(&one_point, &candidate),
            rational(-1)
        );
    }
}
