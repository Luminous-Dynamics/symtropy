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
        let dual_bound = convex_combination_l1_upper_bound(
            &point_gradients,
            &[rational_fraction(1, 6), rational_fraction(5, 6)],
        )
        .expect("the dual weights form an exact simplex point");
        assert_eq!(dual_bound, candidate.margin);
        assert!(verify_robust_descent(
            &branches,
            &candidate.direction,
            &candidate.margin
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
