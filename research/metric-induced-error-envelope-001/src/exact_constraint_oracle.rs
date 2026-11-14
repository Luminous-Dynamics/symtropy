// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Exact-rational oracle for combined generalized-coordinate and
//! constraint-equation rebasing. This is test-only and intentionally small.

#![cfg(test)]

use std::ops::{Add, Div, Mul, Neg, Sub, SubAssign};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Rational {
    numerator: i128,
    denominator: i128,
}

impl Rational {
    fn new(numerator: i128, denominator: i128) -> Self {
        assert_ne!(denominator, 0, "zero rational denominator");
        let sign = if denominator < 0 { -1 } else { 1 };
        let numerator = numerator * sign;
        let denominator = denominator.abs();
        let divisor = gcd(numerator.abs(), denominator);
        Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        }
    }

    fn integer(value: i128) -> Self {
        Self::new(value, 1)
    }

    fn zero() -> Self {
        Self::integer(0)
    }

    fn one() -> Self {
        Self::integer(1)
    }
}

fn gcd(mut left: i128, mut right: i128) -> i128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

impl Add for Rational {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(
            self.numerator * rhs.denominator + rhs.numerator * self.denominator,
            self.denominator * rhs.denominator,
        )
    }
}

impl Sub for Rational {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self + (-rhs)
    }
}

impl Mul for Rational {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(
            self.numerator * rhs.numerator,
            self.denominator * rhs.denominator,
        )
    }
}

impl SubAssign for Rational {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Div for Rational {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        assert_ne!(rhs.numerator, 0, "division by zero rational");
        Self::new(
            self.numerator * rhs.denominator,
            self.denominator * rhs.numerator,
        )
    }
}

impl Neg for Rational {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.numerator, self.denominator)
    }
}

type Mat<const R: usize, const C: usize> = [[Rational; C]; R];
type VecN<const N: usize> = [Rational; N];

fn transpose<const R: usize, const C: usize>(matrix: Mat<R, C>) -> Mat<C, R> {
    std::array::from_fn(|row| std::array::from_fn(|column| matrix[column][row]))
}

fn multiply<const R: usize, const K: usize, const C: usize>(
    left: Mat<R, K>,
    right: Mat<K, C>,
) -> Mat<R, C> {
    std::array::from_fn(|row| {
        std::array::from_fn(|column| {
            (0..K)
                .map(|inner| left[row][inner] * right[inner][column])
                .fold(Rational::zero(), |sum, value| sum + value)
        })
    })
}

fn inverse<const N: usize>(matrix: Mat<N, N>) -> Mat<N, N> {
    let mut augmented: Vec<Vec<Rational>> = (0..N)
        .map(|row| {
            (0..(2 * N))
                .map(|column| {
                    if column < N {
                        matrix[row][column]
                    } else if column - N == row {
                        Rational::one()
                    } else {
                        Rational::zero()
                    }
                })
                .collect()
        })
        .collect();

    for pivot in 0..N {
        let pivot_row = (pivot..N)
            .find(|row| augmented[*row][pivot] != Rational::zero())
            .expect("exact fixture matrix must be nonsingular");
        augmented.swap(pivot, pivot_row);

        let diagonal = augmented[pivot][pivot];
        for value in &mut augmented[pivot] {
            *value = *value / diagonal;
        }

        let pivot_values = augmented[pivot].clone();
        for (row_index, row_values) in augmented.iter_mut().enumerate() {
            if row_index == pivot {
                continue;
            }
            let factor = row_values[pivot];
            for (value, pivot_value) in row_values.iter_mut().zip(pivot_values.iter()) {
                *value -= factor * *pivot_value;
            }
        }
    }

    std::array::from_fn(|row| std::array::from_fn(|column| augmented[row][N + column]))
}

fn multiply_vector<const R: usize, const C: usize>(matrix: Mat<R, C>, vector: VecN<C>) -> VecN<R> {
    std::array::from_fn(|row| {
        (0..C)
            .map(|column| matrix[row][column] * vector[column])
            .fold(Rational::zero(), |sum, value| sum + value)
    })
}

fn dot<const N: usize>(left: VecN<N>, right: VecN<N>) -> Rational {
    (0..N)
        .map(|index| left[index] * right[index])
        .fold(Rational::zero(), |sum, value| sum + value)
}

#[test]
fn exact_rational_oracle_checks_two_independent_rebasing_actions() {
    let m_q: Mat<3, 3> = [
        [Rational::integer(2), Rational::zero(), Rational::zero()],
        [Rational::zero(), Rational::integer(3), Rational::zero()],
        [Rational::zero(), Rational::zero(), Rational::integer(5)],
    ];
    let j_q: Mat<2, 3> = [
        [Rational::one(), Rational::zero(), Rational::one()],
        [Rational::zero(), Rational::integer(2), -Rational::one()],
    ];

    // q = S y: a dense, integer, unit-upper-triangular coordinate change.
    let s: Mat<3, 3> = [
        [Rational::one(), Rational::integer(2), Rational::one()],
        [Rational::zero(), Rational::one(), Rational::integer(3)],
        [Rational::zero(), Rational::zero(), Rational::one()],
    ];
    // r_new = T r_old: an independent constraint-equation basis change.
    let t: Mat<2, 2> = [
        [Rational::one(), Rational::integer(3)],
        [Rational::zero(), Rational::one()],
    ];

    let m_y = multiply(multiply(transpose(s), m_q), s);
    let j_y = multiply(multiply(t, j_q), s);

    let w_q = multiply(multiply(j_q, inverse(m_q)), transpose(j_q));
    let w_y = multiply(multiply(j_y, inverse(m_y)), transpose(j_y));
    let expected_w_y = multiply(multiply(t, w_q), transpose(t));
    assert_eq!(w_y, expected_w_y, "exact Delassus congruence");

    let lambda_q = [Rational::new(3, 4), Rational::new(-5, 4)];
    let residual_q = [Rational::new(1, 4), Rational::new(-2, 5)];
    let lambda_y = multiply_vector(transpose(inverse(t)), lambda_q);
    let residual_y = multiply_vector(t, residual_q);

    assert_eq!(
        dot(lambda_q, residual_q),
        dot(lambda_y, residual_y),
        "exact multiplier/residual dual pairing"
    );

    let multiplier_norm_squared_q = dot(lambda_q, multiply_vector(w_q, lambda_q));
    let multiplier_norm_squared_y = dot(lambda_y, multiply_vector(w_y, lambda_y));
    assert_eq!(
        multiplier_norm_squared_q, multiplier_norm_squared_y,
        "exact Delassus multiplier norm"
    );

    let residual_norm_squared_q = dot(residual_q, multiply_vector(inverse(w_q), residual_q));
    let residual_norm_squared_y = dot(residual_y, multiply_vector(inverse(w_y), residual_y));
    assert_eq!(
        residual_norm_squared_q, residual_norm_squared_y,
        "exact Delassus residual norm"
    );

    // J_y^T lambda_y is the transformed generalized force covector S^T J_q^T lambda_q.
    let force_q = multiply_vector(transpose(j_q), lambda_q);
    let force_y = multiply_vector(transpose(j_y), lambda_y);
    assert_eq!(
        force_y,
        multiply_vector(transpose(s), force_q),
        "exact generalized constraint-force covariance"
    );

    // The physical velocity correction is coordinate-covariant:
    // M_q^-1 J_q^T lambda_q = S M_y^-1 J_y^T lambda_y.
    let acceleration_q = multiply_vector(inverse(m_q), force_q);
    let force_y_from_constraint = multiply_vector(transpose(j_y), lambda_y);
    let acceleration_y = multiply_vector(inverse(m_y), force_y_from_constraint);
    assert_eq!(
        acceleration_q,
        multiply_vector(s, acceleration_y),
        "exact induced velocity correction covariance"
    );

    // Minimum-kinetic correction for J delta_v = r also commutes with both rebases.
    let minimum_multiplier_q = multiply_vector(inverse(w_q), residual_q);
    let minimum_force_q = multiply_vector(transpose(j_q), minimum_multiplier_q);
    let minimum_correction_q = multiply_vector(inverse(m_q), minimum_force_q);

    let minimum_multiplier_y = multiply_vector(inverse(w_y), residual_y);
    let minimum_force_y = multiply_vector(transpose(j_y), minimum_multiplier_y);
    let minimum_correction_y = multiply_vector(inverse(m_y), minimum_force_y);
    assert_eq!(
        minimum_correction_q,
        multiply_vector(s, minimum_correction_y),
        "exact minimum-kinetic correction covariance"
    );

    let minimum_residual_norm_squared_q =
        dot(residual_q, multiply_vector(inverse(w_q), residual_q));
    let minimum_kinetic_norm_squared_q = dot(
        minimum_correction_q,
        multiply_vector(m_q, minimum_correction_q),
    );
    assert_eq!(
        minimum_residual_norm_squared_q, minimum_kinetic_norm_squared_q,
        "exact minimum-kinetic norm identity"
    );
}
