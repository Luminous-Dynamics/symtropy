#![allow(clippy::needless_range_loop)]
// Fixed-size multi-index tensor algebra intentionally uses explicit index loops.

// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Fixed-size four-coordinate Taylor jets through total degree five.
//!
//! Coefficients are normalized Taylor coefficients:
//! c_alpha = partial^alpha f / (alpha_0! alpha_1! alpha_2! alpha_3!).
//! The explicit order field prevents a differentiated, truncated jet from
//! being silently treated as though it still contained higher derivatives.
//! This is a correctness-oriented reference implementation, not an optimized AD backend.

use core::fmt;

const MAX_ORDER: u8 = 5;
const COEFFICIENT_COUNT: usize = 126;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MultiIndex([u8; 4]);

const fn build_multi_indices() -> [MultiIndex; COEFFICIENT_COUNT] {
    let mut indices = [MultiIndex([0; 4]); COEFFICIENT_COUNT];
    let mut count = 0;
    let mut degree = 0;
    while degree <= MAX_ORDER {
        let mut a0 = 0;
        while a0 <= degree {
            let mut a1 = 0;
            while a1 <= degree - a0 {
                let mut a2 = 0;
                while a2 <= degree - a0 - a1 {
                    let a3 = degree - a0 - a1 - a2;
                    indices[count] = MultiIndex([a0, a1, a2, a3]);
                    count += 1;
                    a2 += 1;
                }
                a1 += 1;
            }
            a0 += 1;
        }
        degree += 1;
    }
    indices
}

const MULTI_INDICES: [MultiIndex; COEFFICIENT_COUNT] = build_multi_indices();

fn total_degree(alpha: [u8; 4]) -> u16 {
    u16::from(alpha[0]) + u16::from(alpha[1]) + u16::from(alpha[2]) + u16::from(alpha[3])
}

fn index_of(alpha: [u8; 4]) -> Option<usize> {
    MULTI_INDICES.iter().position(|candidate| candidate.0 == alpha)
}

fn factorial(n: u8) -> f64 {
    let mut result = 1.0;
    let mut value = 2;
    while value <= n {
        result *= f64::from(value);
        value += 1;
    }
    result
}

fn multi_factorial(alpha: [u8; 4]) -> f64 {
    factorial(alpha[0]) * factorial(alpha[1]) * factorial(alpha[2]) * factorial(alpha[3])
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TaylorJetError {
    InvalidOrder(u8),
    CannotIncreaseOrder { current: u8, requested: u8 },
    CoordinateIndexOutOfRange(usize),
    DerivativeOrderExhausted,
    InvalidMultiIndex([u8; 4]),
    ExpansionPointMismatch,
    MissingExpansionPoint,
    ExpansionPointCoordinateMismatch {
        coordinate: usize,
        expected: f64,
        actual: f64,
    },
    DivisionByZero,
    NonFiniteInput,
    NonFiniteIntermediate(&'static str),
}

impl fmt::Display for TaylorJetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOrder(order) => {
                write!(f, "Taylor-jet order {order} exceeds maximum {MAX_ORDER}")
            }
            Self::CannotIncreaseOrder { current, requested } => {
                write!(f, "cannot increase Taylor-jet order from {current} to {requested}")
            }
            Self::CoordinateIndexOutOfRange(index) => {
                write!(f, "coordinate index {index} is outside [0, 3]")
            }
            Self::DerivativeOrderExhausted => write!(f, "Taylor-jet derivative order is exhausted"),
            Self::InvalidMultiIndex(alpha) => write!(f, "unsupported multi-index {alpha:?}"),
            Self::ExpansionPointMismatch => {
                write!(f, "Taylor jets expanded at different coordinate points cannot be combined")
            }
            Self::MissingExpansionPoint => {
                write!(f, "Taylor jet has no declared expansion point")
            }
            Self::ExpansionPointCoordinateMismatch {
                coordinate,
                expected,
                actual,
            } => write!(
                f,
                "Taylor-jet coordinate {coordinate} is centered at {actual}, expected {expected}"
            ),
            Self::DivisionByZero => write!(f, "Taylor-jet reciprocal has zero base value"),
            Self::NonFiniteInput => write!(f, "Taylor-jet input contains a non-finite value"),
            Self::NonFiniteIntermediate(operation) => {
                write!(f, "Taylor-jet {operation} produced a non-finite coefficient")
            }
        }
    }
}

impl std::error::Error for TaylorJetError {}

/// A scalar Taylor expansion in four coordinates, truncated by total degree.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TaylorJet4Order5 {
    coefficients: [f64; COEFFICIENT_COUNT],
    order: u8,
    expansion_point: Option<[f64; 4]>,
}

impl TaylorJet4Order5 {
    /// Construct from normalized Taylor coefficients.
    /// Coefficients above the declared total order must be exactly zero.
    pub fn new(
        coefficients: [f64; COEFFICIENT_COUNT],
        order: u8,
    ) -> Result<Self, TaylorJetError> {
        if order > MAX_ORDER {
            return Err(TaylorJetError::InvalidOrder(order));
        }
        if coefficients.iter().any(|value| !value.is_finite()) {
            return Err(TaylorJetError::NonFiniteInput);
        }
        for (index, alpha) in MULTI_INDICES.iter().enumerate() {
            let degree = total_degree(alpha.0);
            if degree > u16::from(order) && coefficients[index] != 0.0 {
                return Err(TaylorJetError::InvalidMultiIndex(alpha.0));
            }
            if degree > 0 && coefficients[index] != 0.0 {
                return Err(TaylorJetError::MissingExpansionPoint);
            }
        }
        Ok(Self {
            coefficients,
            order,
            expansion_point: None,
        })
    }

    /// Construct normalized Taylor coefficients with an explicit coordinate point.
    pub fn new_at(
        coefficients: [f64; COEFFICIENT_COUNT],
        order: u8,
        expansion_point: [f64; 4],
    ) -> Result<Self, TaylorJetError> {
        if expansion_point.iter().any(|value| !value.is_finite()) {
            return Err(TaylorJetError::NonFiniteInput);
        }
        if order > MAX_ORDER {
            return Err(TaylorJetError::InvalidOrder(order));
        }
        if coefficients.iter().any(|value| !value.is_finite()) {
            return Err(TaylorJetError::NonFiniteInput);
        }
        for (index, alpha) in MULTI_INDICES.iter().enumerate() {
            if total_degree(alpha.0) > u16::from(order) && coefficients[index] != 0.0 {
                return Err(TaylorJetError::InvalidMultiIndex(alpha.0));
            }
        }
        Ok(Self {
            coefficients,
            order,
            expansion_point: Some(expansion_point),
        })
    }

    pub fn zero(order: u8) -> Result<Self, TaylorJetError> {
        Self::new([0.0; COEFFICIENT_COUNT], order)
    }

    /// Exact constant function; all five derivative orders are known.
    pub fn constant(value: f64) -> Result<Self, TaylorJetError> {
        if !value.is_finite() {
            return Err(TaylorJetError::NonFiniteInput);
        }
        let mut coefficients = [0.0; COEFFICIENT_COUNT];
        coefficients[0] = value;
        Self::new(coefficients, MAX_ORDER)
    }

    /// Coordinate variable centered at value for one of the four axes.
    ///
    /// Unspecified coordinates are centered at zero. Use variable_at when
    /// combining variables at a shared nonzero spacetime point.
    pub fn variable(value: f64, coordinate: usize) -> Result<Self, TaylorJetError> {
        if !value.is_finite() {
            return Err(TaylorJetError::NonFiniteInput);
        }
        if coordinate >= 4 {
            return Err(TaylorJetError::CoordinateIndexOutOfRange(coordinate));
        }
        let mut point = [0.0; 4];
        point[coordinate] = value;
        Self::variable_at(point, coordinate)
    }

    /// Coordinate variable centered at a complete four-coordinate point.
    pub fn variable_at(
        expansion_point: [f64; 4],
        coordinate: usize,
    ) -> Result<Self, TaylorJetError> {
        if expansion_point.iter().any(|value| !value.is_finite()) {
            return Err(TaylorJetError::NonFiniteInput);
        }
        if coordinate >= 4 {
            return Err(TaylorJetError::CoordinateIndexOutOfRange(coordinate));
        }
        let mut coefficients = [0.0; COEFFICIENT_COUNT];
        coefficients[0] = expansion_point[coordinate];
        let mut alpha = [0; 4];
        alpha[coordinate] = 1;
        let index = index_of(alpha).ok_or(TaylorJetError::InvalidMultiIndex(alpha))?;
        coefficients[index] = 1.0;
        Self::new_at(coefficients, MAX_ORDER, expansion_point)
    }

    pub const fn expansion_point(&self) -> Option<[f64; 4]> {
        self.expansion_point
    }

    /// Require this jet to be expanded at the given coordinate on one axis.
    pub fn require_coordinate_center(
        &self,
        coordinate: usize,
        expected: f64,
    ) -> Result<(), TaylorJetError> {
        if coordinate >= 4 {
            return Err(TaylorJetError::CoordinateIndexOutOfRange(coordinate));
        }
        if !expected.is_finite() {
            return Err(TaylorJetError::NonFiniteInput);
        }
        let point = self.expansion_point.ok_or(TaylorJetError::MissingExpansionPoint)?;
        let actual = point[coordinate];
        if actual != expected {
            return Err(TaylorJetError::ExpansionPointCoordinateMismatch {
                coordinate,
                expected,
                actual,
            });
        }
        Ok(())
    }

    pub const fn order(&self) -> u8 {
        self.order
    }

    pub const fn value(&self) -> f64 {
        self.coefficients[0]
    }

    pub const fn coefficients(&self) -> &[f64; COEFFICIENT_COUNT] {
        &self.coefficients
    }

    pub fn coefficient(&self, alpha: [u8; 4]) -> Result<f64, TaylorJetError> {
        if total_degree(alpha) > u16::from(self.order) {
            return Err(TaylorJetError::InvalidMultiIndex(alpha));
        }
        let index = index_of(alpha).ok_or(TaylorJetError::InvalidMultiIndex(alpha))?;
        Ok(self.coefficients[index])
    }

    /// Extract an ordinary mixed partial at the expansion point.
    pub fn derivative_at(&self, alpha: [u8; 4]) -> Result<f64, TaylorJetError> {
        let value = self.coefficient(alpha)? * multi_factorial(alpha);
        if value.is_finite() {
            Ok(value)
        } else {
            Err(TaylorJetError::NonFiniteIntermediate("derivative extraction"))
        }
    }

    /// Lower the available order. Increasing it is rejected.
    pub fn truncate(self, requested: u8) -> Result<Self, TaylorJetError> {
        if requested > self.order {
            return Err(TaylorJetError::CannotIncreaseOrder {
                current: self.order,
                requested,
            });
        }
        let mut coefficients = self.coefficients;
        for (index, alpha) in MULTI_INDICES.iter().enumerate() {
            if total_degree(alpha.0) > u16::from(requested) {
                coefficients[index] = 0.0;
            }
        }
        Self::new_with_point(coefficients, requested, self.expansion_point)
    }

    /// Differentiate with respect to one coordinate; available order decreases by one.
    pub fn partial(self, coordinate: usize) -> Result<Self, TaylorJetError> {
        if coordinate >= 4 {
            return Err(TaylorJetError::CoordinateIndexOutOfRange(coordinate));
        }
        if self.order == 0 {
            return Err(TaylorJetError::DerivativeOrderExhausted);
        }
        let output_order = self.order - 1;
        let mut coefficients = [0.0; COEFFICIENT_COUNT];
        for (index, alpha) in MULTI_INDICES.iter().enumerate() {
            if total_degree(alpha.0) > u16::from(output_order) {
                continue;
            }
            let mut source = alpha.0;
            source[coordinate] += 1;
            let source_index = index_of(source).ok_or(TaylorJetError::InvalidMultiIndex(source))?;
            coefficients[index] =
                f64::from(source[coordinate]) * self.coefficients[source_index];
        }
        Self::new_with_point(coefficients, output_order, self.expansion_point)
    }

    fn merged_expansion_point(
        self,
        other: Self,
    ) -> Result<Option<[f64; 4]>, TaylorJetError> {
        match (self.expansion_point, other.expansion_point) {
            (Some(left), Some(right)) if left != right => {
                Err(TaylorJetError::ExpansionPointMismatch)
            }
            (Some(point), _) | (_, Some(point)) => Ok(Some(point)),
            (None, None) => Ok(None),
        }
    }

    fn new_with_point(
        coefficients: [f64; COEFFICIENT_COUNT],
        order: u8,
        expansion_point: Option<[f64; 4]>,
    ) -> Result<Self, TaylorJetError> {
        match expansion_point {
            Some(point) => Self::new_at(coefficients, order, point),
            None => Self::new(coefficients, order),
        }
    }

    pub fn add(self, other: Self) -> Result<Self, TaylorJetError> {
        let expansion_point = self.merged_expansion_point(other)?;
        let order = self.order.min(other.order);
        let mut coefficients = [0.0; COEFFICIENT_COUNT];
        for (index, alpha) in MULTI_INDICES.iter().enumerate() {
            if total_degree(alpha.0) <= u16::from(order) {
                coefficients[index] = self.coefficients[index] + other.coefficients[index];
            }
        }
        Self::new_with_point(coefficients, order, expansion_point)?.ensure_finite("addition")
    }

    pub fn subtract(self, other: Self) -> Result<Self, TaylorJetError> {
        self.add(other.scale(-1.0)?)
    }

    pub fn scale(self, factor: f64) -> Result<Self, TaylorJetError> {
        if !factor.is_finite() {
            return Err(TaylorJetError::NonFiniteInput);
        }
        let mut coefficients = [0.0; COEFFICIENT_COUNT];
        for (index, alpha) in MULTI_INDICES.iter().enumerate() {
            if total_degree(alpha.0) <= u16::from(self.order) {
                coefficients[index] = self.coefficients[index] * factor;
            }
        }
        Self::new_with_point(coefficients, self.order, self.expansion_point)?.ensure_finite("scaling")
    }

    /// Truncated product by normalized-coefficient convolution.
    pub fn multiply(self, other: Self) -> Result<Self, TaylorJetError> {
        let expansion_point = self.merged_expansion_point(other)?;
        let order = self.order.min(other.order);
        let mut coefficients = [0.0; COEFFICIENT_COUNT];
        for (left_index, left_alpha) in MULTI_INDICES.iter().enumerate() {
            if total_degree(left_alpha.0) > u16::from(order) {
                continue;
            }
            let left_value = self.coefficients[left_index];
            if left_value == 0.0 {
                continue;
            }
            for (right_index, right_alpha) in MULTI_INDICES.iter().enumerate() {
                if total_degree(right_alpha.0) > u16::from(order) {
                    continue;
                }
                let right_value = other.coefficients[right_index];
                if right_value == 0.0 {
                    continue;
                }
                let alpha = [
                    left_alpha.0[0] + right_alpha.0[0],
                    left_alpha.0[1] + right_alpha.0[1],
                    left_alpha.0[2] + right_alpha.0[2],
                    left_alpha.0[3] + right_alpha.0[3],
                ];
                if total_degree(alpha) > u16::from(order) {
                    continue;
                }
                let output_index =
                    index_of(alpha).ok_or(TaylorJetError::InvalidMultiIndex(alpha))?;
                coefficients[output_index] += left_value * right_value;
            }
        }
        Self::new_with_point(coefficients, order, expansion_point)?.ensure_finite("multiplication")
    }

    /// Reciprocal by a total-degree-ordered power-series recurrence.
    pub fn reciprocal(self) -> Result<Self, TaylorJetError> {
        let base = self.value();
        if !base.is_finite() {
            return Err(TaylorJetError::NonFiniteInput);
        }
        if base == 0.0 {
            return Err(TaylorJetError::DivisionByZero);
        }

        let mut output = [0.0; COEFFICIENT_COUNT];
        output[0] = 1.0 / base;
        for degree in 1..=self.order {
            for (alpha_index, alpha_entry) in MULTI_INDICES.iter().enumerate() {
                let alpha = alpha_entry.0;
                if total_degree(alpha) != u16::from(degree) {
                    continue;
                }
                let mut sum = 0.0;
                for (beta_index, beta_entry) in MULTI_INDICES.iter().enumerate().skip(1) {
                    let beta = beta_entry.0;
                    if total_degree(beta) > u16::from(degree) {
                        break;
                    }
                    let mut bounded = true;
                    let mut delta = [0; 4];
                    for axis in 0..4 {
                        if beta[axis] > alpha[axis] {
                            bounded = false;
                            break;
                        }
                        delta[axis] = alpha[axis] - beta[axis];
                    }
                    if bounded {
                        let delta_index =
                            index_of(delta).ok_or(TaylorJetError::InvalidMultiIndex(delta))?;
                        sum += self.coefficients[beta_index] * output[delta_index];
                    }
                }
                output[alpha_index] = -sum / base;
            }
        }
        Self::new_with_point(output, self.order, self.expansion_point)?.ensure_finite("reciprocal")
    }

    pub fn divide(self, denominator: Self) -> Result<Self, TaylorJetError> {
        self.multiply(denominator.reciprocal()?)
    }

    pub fn sin(self) -> Result<Self, TaylorJetError> {
        self.compose_trigonometric(true)
    }

    pub fn cos(self) -> Result<Self, TaylorJetError> {
        self.compose_trigonometric(false)
    }

    fn compose_trigonometric(self, sine: bool) -> Result<Self, TaylorJetError> {
        let x0 = self.value();
        let sin_x0 = x0.sin();
        let cos_x0 = x0.cos();
        let initial = if sine { sin_x0 } else { cos_x0 };
        let mut result = Self::constant(initial)?.truncate(self.order)?;
        let h = self.subtract(Self::constant(x0)?)?;
        let mut power = Self::constant(1.0)?.truncate(self.order)?;
        let mut factorial_value = 1.0;

        for degree in 1..=self.order {
            power = power.multiply(h)?;
            factorial_value *= f64::from(degree);
            let derivative = if sine {
                match degree % 4 {
                    0 => sin_x0,
                    1 => cos_x0,
                    2 => -sin_x0,
                    _ => -cos_x0,
                }
            } else {
                match degree % 4 {
                    0 => cos_x0,
                    1 => -sin_x0,
                    2 => -cos_x0,
                    _ => sin_x0,
                }
            };
            result = result.add(power.scale(derivative / factorial_value)?)?;
        }
        result.ensure_finite(if sine { "sine" } else { "cosine" })
    }

    fn ensure_finite(self, operation: &'static str) -> Result<Self, TaylorJetError> {
        if self.coefficients.iter().any(|value| !value.is_finite()) {
            Err(TaylorJetError::NonFiniteIntermediate(operation))
        } else {
            Ok(self)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual={actual:.16e}, expected={expected:.16e}, tolerance={tolerance:.3e}"
        );
    }

    #[test]
    fn coefficient_count_and_order_partition_are_explicit() {
        assert_eq!(MULTI_INDICES.len(), 126);
        let expected = [1, 4, 10, 20, 35, 56];
        for degree in 0..=MAX_ORDER {
            let actual = MULTI_INDICES
                .iter()
                .filter(|alpha| total_degree(alpha.0) == u16::from(degree))
                .count();
            assert_eq!(actual, expected[usize::from(degree)]);
        }
    }

    #[test]
    fn mixed_polynomial_derivative_matches_hand_calculation() {
        let point = [2.0, 3.0, 0.0, 0.0];
        let x = TaylorJet4Order5::variable_at(point, 0).unwrap();
        let y = TaylorJet4Order5::variable_at(point, 1).unwrap();
        let x3 = x.multiply(x).unwrap().multiply(x).unwrap();
        let y2 = y.multiply(y).unwrap();
        let polynomial = x3.multiply(y2).unwrap();
        close(polynomial.derivative_at([3, 2, 0, 0]).unwrap(), 12.0, 1e-13);
    }

    #[test]
    fn reciprocal_coefficients_match_geometric_series() {
        let x = TaylorJet4Order5::variable(2.0, 0).unwrap();
        let inverse = x.reciprocal().unwrap();
        for degree in 0..=5 {
            let expected = (if degree % 2 == 0 { 1.0 } else { -1.0 })
                / 2.0_f64.powi(i32::from(degree + 1));
            close(inverse.coefficient([degree, 0, 0, 0]).unwrap(), expected, 1e-14);
        }
        close(inverse.derivative_at([5, 0, 0, 0]).unwrap(), -120.0 / 64.0, 1e-13);
    }

    #[test]
    fn fifth_order_sine_and_cosine_derivatives_match_analytic_values() {
        let theta = TaylorJet4Order5::variable(0.7, 2).unwrap();
        let sine = theta.sin().unwrap();
        let cosine = theta.cos().unwrap();
        let sin_value = 0.7_f64.sin();
        let cos_value = 0.7_f64.cos();
        for degree in 0..=5 {
            let expected_sine = match degree % 4 {
                0 => sin_value,
                1 => cos_value,
                2 => -sin_value,
                _ => -cos_value,
            };
            let expected_cosine = match degree % 4 {
                0 => cos_value,
                1 => -sin_value,
                2 => -cos_value,
                _ => sin_value,
            };
            let mut alpha = [0; 4];
            alpha[2] = degree;
            close(sine.derivative_at(alpha).unwrap(), expected_sine, 2e-13);
            close(cosine.derivative_at(alpha).unwrap(), expected_cosine, 2e-13);
        }
    }

    #[test]
    fn repeated_partials_exhaust_the_declared_derivative_order() {
        let x = TaylorJet4Order5::variable(1.0, 0).unwrap();
        let x5 = x
            .multiply(x)
            .unwrap()
            .multiply(x)
            .unwrap()
            .multiply(x)
            .unwrap()
            .multiply(x)
            .unwrap();
        let mut derivative = x5;
        for _ in 0..5 {
            derivative = derivative.partial(0).unwrap();
        }
        assert_eq!(derivative.order(), 0);
        close(derivative.value(), 120.0, 1e-12);
        assert_eq!(
            derivative.partial(0),
            Err(TaylorJetError::DerivativeOrderExhausted)
        );
    }

    #[test]
    fn truncation_and_order_mismatch_fail_closed() {
        let x = TaylorJet4Order5::variable(1.0, 0).unwrap();
        let lower = x.truncate(3).unwrap();
        assert_eq!(lower.order(), 3);
        assert_eq!(
            lower.truncate(4),
            Err(TaylorJetError::CannotIncreaseOrder {
                current: 3,
                requested: 4
            })
        );
        assert_eq!(
            lower.derivative_at([4, 0, 0, 0]),
            Err(TaylorJetError::InvalidMultiIndex([4, 0, 0, 0]))
        );
    }

    #[test]
    fn nonconstant_coefficients_require_an_expansion_point() {
        let mut coefficients = [0.0; COEFFICIENT_COUNT];
        coefficients[0] = 1.0;
        coefficients[index_of([1, 0, 0, 0]).unwrap()] = 2.0;
        assert_eq!(
            TaylorJet4Order5::new(coefficients, 5),
            Err(TaylorJetError::MissingExpansionPoint)
        );
        assert!(
            TaylorJet4Order5::new_at(coefficients, 5, [0.0; 4]).is_ok()
        );
    }

    #[test]
    fn jets_at_different_expansion_points_cannot_be_combined() {
        let x = TaylorJet4Order5::variable(2.0, 0).unwrap();
        let y = TaylorJet4Order5::variable(3.0, 1).unwrap();
        assert_eq!(x.add(y), Err(TaylorJetError::ExpansionPointMismatch));
        assert_eq!(x.multiply(y), Err(TaylorJetError::ExpansionPointMismatch));
    }

    #[test]
    fn invalid_multi_indices_cannot_overflow_degree_validation() {
        let x = TaylorJet4Order5::constant(1.0).unwrap();
        assert_eq!(
            x.coefficient([u8::MAX; 4]),
            Err(TaylorJetError::InvalidMultiIndex([u8::MAX; 4]))
        );
    }

    #[test]
    fn reciprocal_rejects_zero_and_non_finite_inputs() {
        assert_eq!(
            TaylorJet4Order5::constant(0.0).unwrap().reciprocal(),
            Err(TaylorJetError::DivisionByZero)
        );
        assert_eq!(
            TaylorJet4Order5::constant(f64::NAN),
            Err(TaylorJetError::NonFiniteInput)
        );
    }
}
