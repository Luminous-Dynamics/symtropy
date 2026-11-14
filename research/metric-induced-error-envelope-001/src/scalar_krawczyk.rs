// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Scalar Krawczyk inclusion using outward-rounded interval enclosures.
//!
//! K(x0, X) = x0 - C F(x0) + (1 - C F'(X)) (X - x0).
//!
//! Strict inclusion K(x0, X) subset int(X) conditionally establishes a unique
//! zero in X, provided the supplied function and derivative intervals are
//! valid enclosures and F is continuously differentiable throughout X.
//!
//! This module checks interval arithmetic and strict inclusion only. It does
//! not prove that caller-supplied function/derivative enclosures are sound.

use std::fmt;

use crate::outward_interval::{IntervalErrorV1, IntervalV1};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarKrawczykStatusV1 {
    ConditionalUniqueRoot,
    Inconclusive,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScalarKrawczykReceiptV1 {
    pub candidate_center: f64,
    pub preconditioner: f64,
    pub search_box: IntervalV1,
    pub krawczyk_image: IntervalV1,
    pub strict_inclusion: bool,
    pub status: ScalarKrawczykStatusV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarKrawczykErrorV1 {
    NonFiniteCenter,
    NonFinitePreconditioner,
    ZeroPreconditioner,
    CenterOutsideSearchBox,
    Interval(IntervalErrorV1),
}

impl fmt::Display for ScalarKrawczykErrorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ScalarKrawczykErrorV1 {}

impl From<IntervalErrorV1> for ScalarKrawczykErrorV1 {
    fn from(value: IntervalErrorV1) -> Self {
        Self::Interval(value)
    }
}

/// Evaluate a scalar Krawczyk operator from caller-supplied enclosures.
///
/// Preconditions not checked here:
/// - function_value_at_center encloses exact F(center);
/// - derivative_over_box encloses F'(x) for every x in search_box;
/// - F is continuously differentiable throughout search_box.
///
/// Strict inclusion yields a conditional unique-root result. Failure of
/// strict inclusion is INCONCLUSIVE, not proof that no root exists.
pub fn scalar_krawczyk_inclusion(
    center: f64,
    preconditioner: f64,
    function_value_at_center: IntervalV1,
    derivative_over_box: IntervalV1,
    search_box: IntervalV1,
) -> Result<ScalarKrawczykReceiptV1, ScalarKrawczykErrorV1> {
    if !center.is_finite() {
        return Err(ScalarKrawczykErrorV1::NonFiniteCenter);
    }
    if !preconditioner.is_finite() {
        return Err(ScalarKrawczykErrorV1::NonFinitePreconditioner);
    }
    if preconditioner == 0.0 {
        return Err(ScalarKrawczykErrorV1::ZeroPreconditioner);
    }
    if !search_box.contains(center) {
        return Err(ScalarKrawczykErrorV1::CenterOutsideSearchBox);
    }

    let center_interval = IntervalV1::point(center)?;
    let preconditioner_interval = IntervalV1::point(preconditioner)?;
    let one = IntervalV1::point(1.0)?;

    let preconditioned_residual =
        preconditioner_interval.mul(function_value_at_center)?;
    let newton_center = center_interval.sub(preconditioned_residual)?;

    let preconditioned_derivative =
        preconditioner_interval.mul(derivative_over_box)?;
    let defect = one.sub(preconditioned_derivative)?;

    let centered_box = search_box.sub(center_interval)?;
    let correction = defect.mul(centered_box)?;
    let krawczyk_image = newton_center.add(correction)?;

    let strict_inclusion = krawczyk_image.lower() > search_box.lower()
        && krawczyk_image.upper() < search_box.upper();
    let status = if strict_inclusion {
        ScalarKrawczykStatusV1::ConditionalUniqueRoot
    } else {
        ScalarKrawczykStatusV1::Inconclusive
    };

    Ok(ScalarKrawczykReceiptV1 {
        candidate_center: center,
        preconditioner,
        search_box,
        krawczyk_image,
        strict_inclusion,
        status,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quadratic_enclosures(
        eta: f64,
        gamma: f64,
        center: f64,
        search_box: IntervalV1,
    ) -> (IntervalV1, IntervalV1) {
        // F(x) = eta + x + gamma*x^2/2 and F'(x) = 1 + gamma*x.
        let eta_interval = IntervalV1::point(eta).unwrap();
        let x = IntervalV1::point(center).unwrap();
        let quadratic_scale = IntervalV1::point(0.5 * gamma).unwrap();
        let function_value = eta_interval
            .add(x)
            .unwrap()
            .add(quadratic_scale.mul(x.square().unwrap()).unwrap())
            .unwrap();
        let derivative = IntervalV1::point(1.0)
            .unwrap()
            .add(IntervalV1::point(gamma).unwrap().mul(search_box).unwrap())
            .unwrap();
        (function_value, derivative)
    }

    #[test]
    fn analytic_quadratic_has_unique_root_in_strict_krawczyk_box() {
        let eta = 0.125;
        let gamma = 2.0;
        let center = -0.125;
        let search_box = IntervalV1::new(-0.2, -0.05).unwrap();
        let (function_value, derivative) =
            quadratic_enclosures(eta, gamma, center, search_box);

        let receipt = scalar_krawczyk_inclusion(
            center,
            1.25,
            function_value,
            derivative,
            search_box,
        )
        .unwrap();

        assert_eq!(receipt.status, ScalarKrawczykStatusV1::ConditionalUniqueRoot);
        assert!(receipt.strict_inclusion);
        assert!(search_box.contains_interval(receipt.krawczyk_image));

        let exact_root = (-1.0 + (1.0 - 2.0 * eta * gamma).sqrt()) / gamma;
        assert!(search_box.contains(exact_root));
    }

    #[test]
    fn wide_box_without_strict_inclusion_is_inconclusive() {
        let search_box = IntervalV1::new(-10.0, 10.0).unwrap();
        let (function_value, derivative) =
            quadratic_enclosures(0.125, 2.0, -0.125, search_box);
        let receipt = scalar_krawczyk_inclusion(
            -0.125,
            1.25,
            function_value,
            derivative,
            search_box,
        )
        .unwrap();
        assert_eq!(receipt.status, ScalarKrawczykStatusV1::Inconclusive);
        assert!(!receipt.strict_inclusion);
    }

    #[test]
    fn zero_preconditioner_is_rejected() {
        let point = IntervalV1::point(0.0).unwrap();
        let search = IntervalV1::new(-1.0, 1.0).unwrap();
        assert_eq!(
            scalar_krawczyk_inclusion(0.0, 0.0, point, point, search),
            Err(ScalarKrawczykErrorV1::ZeroPreconditioner)
        );
    }

    #[test]
    fn center_outside_search_box_is_rejected() {
        let point = IntervalV1::point(0.0).unwrap();
        let search = IntervalV1::new(-1.0, 1.0).unwrap();
        assert_eq!(
            scalar_krawczyk_inclusion(2.0, 1.0, point, point, search),
            Err(ScalarKrawczykErrorV1::CenterOutsideSearchBox)
        );
    }

    #[test]
    fn nonfinite_preconditioner_and_center_are_rejected() {
        let point = IntervalV1::point(0.0).unwrap();
        let search = IntervalV1::new(-1.0, 1.0).unwrap();
        assert_eq!(
            scalar_krawczyk_inclusion(0.0, f64::NAN, point, point, search),
            Err(ScalarKrawczykErrorV1::NonFinitePreconditioner)
        );
        assert_eq!(
            scalar_krawczyk_inclusion(f64::INFINITY, 1.0, point, point, search),
            Err(ScalarKrawczykErrorV1::NonFiniteCenter)
        );
    }
}
