// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Dimension-correct compact-support SPH reference kernels.
//!
//! The Poly6 kernel here is normalized over an unbounded D-dimensional
//! Euclidean domain. It does not correct kernel-support deficiency at walls or
//! free surfaces, and it is not by itself a validated fluid discretization.

use std::error::Error;
use std::fmt;

use nalgebra::SVector;

/// Failure reason for constructing or evaluating a reference SPH kernel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SphKernelError {
    InvalidDimension,
    UnsupportedDimension,
    InvalidSmoothingLength,
    NonFiniteDisplacement { axis: usize },
    NumericRange,
}

impl fmt::Display for SphKernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDimension => write!(f, "SPH kernel requires D >= 1"),
            Self::UnsupportedDimension => {
                write!(f, "SPH kernel dimension exceeds the reference envelope")
            }
            Self::InvalidSmoothingLength => {
                write!(
                    f,
                    "smoothing length must be finite, positive, and numerically representable"
                )
            }
            Self::NonFiniteDisplacement { axis } => {
                write!(f, "non-finite kernel displacement on axis {axis}")
            }
            Self::NumericRange => {
                write!(f, "SPH kernel arithmetic exceeded the finite numeric range")
            }
        }
    }
}

impl Error for SphKernelError {}

/// Selectable radial kernel profile for the WCSPH reference solver.
///
/// Both profiles use the same constructor convention: the supplied value is
/// the physical support radius. Their shapes and normalization differ, so
/// benchmark comparisons must state whether they match support radius or
/// neighbour count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SphKernelProfile {
    #[default]
    Poly6,
    WendlandC2,
}

/// Runtime-selected kernel evaluator over compile-time spatial dimension.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KernelEvaluator<const D: usize> {
    Poly6(Poly6Kernel<D>),
    WendlandC2(WendlandC2Kernel<D>),
}

impl<const D: usize> KernelEvaluator<D> {
    pub fn new(
        profile: SphKernelProfile,
        support_radius: f64,
    ) -> Result<Self, SphKernelError> {
        match profile {
            SphKernelProfile::Poly6 => Ok(Self::Poly6(Poly6Kernel::new(support_radius)?)),
            SphKernelProfile::WendlandC2 => {
                Ok(Self::WendlandC2(WendlandC2Kernel::new(support_radius)?))
            }
        }
    }

    pub fn profile(&self) -> SphKernelProfile {
        match self {
            Self::Poly6(_) => SphKernelProfile::Poly6,
            Self::WendlandC2(_) => SphKernelProfile::WendlandC2,
        }
    }

    pub fn support_radius(&self) -> f64 {
        match self {
            Self::Poly6(kernel) => kernel.smoothing_length(),
            Self::WendlandC2(kernel) => kernel.support_radius(),
        }
    }

    pub fn value(
        &self,
        displacement: &SVector<f64, D>,
    ) -> Result<f64, SphKernelError> {
        match self {
            Self::Poly6(kernel) => kernel.value(displacement),
            Self::WendlandC2(kernel) => kernel.value(displacement),
        }
    }

    pub fn gradient(
        &self,
        displacement: &SVector<f64, D>,
    ) -> Result<SVector<f64, D>, SphKernelError> {
        match self {
            Self::Poly6(kernel) => kernel.gradient(displacement),
            Self::WendlandC2(kernel) => kernel.gradient(displacement),
        }
    }
}

/// A Poly6 kernel with cached dimension-dependent normalization.
///
/// The initial implementation deliberately limits D to 128 so normalization
/// remains bounded to a small fixed-cost recurrence. Physical reference cases
/// should start with D=2 and D=3; other dimensions are mathematical kernel
/// support, not an automatic claim of physically validated fluid dynamics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Poly6Kernel<const D: usize> {
    smoothing_length: f64,
    h_squared: f64,
    normalization: f64,
}

impl<const D: usize> Poly6Kernel<D> {
    pub fn new(smoothing_length: f64) -> Result<Self, SphKernelError> {
        if D == 0 {
            return Err(SphKernelError::InvalidDimension);
        }
        if D > 128 {
            return Err(SphKernelError::UnsupportedDimension);
        }
        if !smoothing_length.is_finite() || smoothing_length <= 0.0 {
            return Err(SphKernelError::InvalidSmoothingLength);
        }

        let h_squared = smoothing_length * smoothing_length;
        let h_power = smoothing_length.powf((D + 6) as f64);
        if !h_squared.is_finite() || h_squared <= 0.0 || !h_power.is_finite() || h_power <= 0.0 {
            return Err(SphKernelError::InvalidSmoothingLength);
        }

        let gamma = gamma_dimension_half_plus_four::<D>()?;
        let denominator = 6.0 * std::f64::consts::PI.powf(D as f64 * 0.5) * h_power;
        let normalization = gamma / denominator;
        if !denominator.is_finite()
            || denominator <= 0.0
            || !normalization.is_finite()
            || normalization <= 0.0
        {
            return Err(SphKernelError::NumericRange);
        }

        Ok(Self {
            smoothing_length,
            h_squared,
            normalization,
        })
    }

    pub fn smoothing_length(&self) -> f64 {
        self.smoothing_length
    }

    pub fn normalization(&self) -> f64 {
        self.normalization
    }

    /// Evaluate W_D(r,h) = C_D(h) (h² - |r|²)^3 for |r| < h, else zero.
    pub fn value(&self, displacement: &SVector<f64, D>) -> Result<f64, SphKernelError> {
        self.validate_displacement(displacement)?;
        let r_squared = displacement.norm_squared();
        if !r_squared.is_finite() {
            return Err(SphKernelError::NumericRange);
        }
        if r_squared >= self.h_squared {
            return Ok(0.0);
        }

        let gap = self.h_squared - r_squared;
        let value = self.normalization * gap * gap * gap;
        if !value.is_finite() || value < 0.0 {
            return Err(SphKernelError::NumericRange);
        }
        Ok(value)
    }

    /// Gradient with respect to the first point, using r = x_i - x_j.
    ///
    /// grad_i W = -6 C_D (h² - |r|²)² r for |r| < h, else zero.
    pub fn gradient(
        &self,
        displacement: &SVector<f64, D>,
    ) -> Result<SVector<f64, D>, SphKernelError> {
        self.validate_displacement(displacement)?;
        let r_squared = displacement.norm_squared();
        if !r_squared.is_finite() {
            return Err(SphKernelError::NumericRange);
        }
        if r_squared >= self.h_squared {
            return Ok(SVector::<f64, D>::zeros());
        }

        let gap = self.h_squared - r_squared;
        let factor = -6.0 * self.normalization * gap * gap;
        if !factor.is_finite() {
            return Err(SphKernelError::NumericRange);
        }
        let gradient = displacement * factor;
        if !gradient.iter().all(|component| component.is_finite()) {
            return Err(SphKernelError::NumericRange);
        }
        Ok(gradient)
    }

    fn validate_displacement(&self, displacement: &SVector<f64, D>) -> Result<(), SphKernelError> {
        for axis in 0..D {
            if !displacement[axis].is_finite() {
                return Err(SphKernelError::NonFiniteDisplacement { axis });
            }
        }
        Ok(())
    }
}

fn gamma_dimension_half_plus_four<const D: usize>() -> Result<f64, SphKernelError> {
    let mut value;
    if D.is_multiple_of(2) {
        // D/2 + 4 is an integer; Gamma(n) = (n - 1)!.
        let argument = D / 2 + 4;
        value = 1.0;
        for factor in 2..argument {
            value *= factor as f64;
            if !value.is_finite() {
                return Err(SphKernelError::NumericRange);
            }
        }
    } else {
        // Gamma(n + 1/2) = sqrt(pi) * product(k + 1/2), k = 0..n-1.
        let terms = (D + 7) / 2;
        value = std::f64::consts::PI.sqrt();
        for factor in 0..terms {
            value *= factor as f64 + 0.5;
            if !value.is_finite() {
                return Err(SphKernelError::NumericRange);
            }
        }
    }
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(SphKernelError::NumericRange)
    }
}

/// Compact-support Wendland C2 kernel for D=1,2,3.
///
/// The constructor argument is the **full support radius** R, so the kernel
/// vanishes for |r| >= R. The radial shape is
/// W(r,R) = C_D(R) (1-q)^4 (1+4q), q = |r|/R, 0 <= q < 1.
///
/// Normalizations:
/// - D=1: C = 3/(2 R)
/// - D=2: C = 7/(pi R^2)
/// - D=3: C = 21/(2 pi R^3)
///
/// This is a standalone kernel primitive. The WCSPH reference solver still
/// selects Poly6 until a separate matched-kernel comparison is completed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WendlandC2Kernel<const D: usize> {
    support_radius: f64,
    support_radius_squared: f64,
    normalization: f64,
}

impl<const D: usize> WendlandC2Kernel<D> {
    pub fn new(support_radius: f64) -> Result<Self, SphKernelError> {
        if D == 0 {
            return Err(SphKernelError::InvalidDimension);
        }
        if D > 3 {
            return Err(SphKernelError::UnsupportedDimension);
        }
        if !support_radius.is_finite() || support_radius <= 0.0 {
            return Err(SphKernelError::InvalidSmoothingLength);
        }

        let support_radius_squared = support_radius * support_radius;
        if !support_radius_squared.is_finite() || support_radius_squared <= 0.0 {
            return Err(SphKernelError::InvalidSmoothingLength);
        }

        let radius_power = support_radius.powi(D as i32);
        if !radius_power.is_finite() || radius_power <= 0.0 {
            return Err(SphKernelError::InvalidSmoothingLength);
        }
        let dimension_constant = match D {
            1 => 1.5,
            2 => 7.0 / std::f64::consts::PI,
            3 => 21.0 / (2.0 * std::f64::consts::PI),
            _ => return Err(SphKernelError::UnsupportedDimension),
        };
        let normalization = dimension_constant / radius_power;
        if !normalization.is_finite() || normalization <= 0.0 {
            return Err(SphKernelError::NumericRange);
        }

        Ok(Self {
            support_radius,
            support_radius_squared,
            normalization,
        })
    }

    pub fn support_radius(&self) -> f64 {
        self.support_radius
    }

    pub fn normalization(&self) -> f64 {
        self.normalization
    }

    /// Evaluate the normalized Wendland C2 radial weight.
    pub fn value(&self, displacement: &SVector<f64, D>) -> Result<f64, SphKernelError> {
        self.validate_displacement(displacement)?;
        let radius_squared = displacement.norm_squared();
        if !radius_squared.is_finite() {
            return Err(SphKernelError::NumericRange);
        }
        if radius_squared >= self.support_radius_squared {
            return Ok(0.0);
        }

        let q = radius_squared.sqrt() / self.support_radius;
        let gap = 1.0 - q;
        let value = self.normalization * gap.powi(4) * (1.0 + 4.0 * q);
        if !value.is_finite() || value < 0.0 {
            return Err(SphKernelError::NumericRange);
        }
        Ok(value)
    }

    /// Gradient with respect to the first point, using r = x_i - x_j.
    ///
    /// grad_i W = -20 C_D / R² * (1 - |r|/R)^3 * r inside support.
    pub fn gradient(
        &self,
        displacement: &SVector<f64, D>,
    ) -> Result<SVector<f64, D>, SphKernelError> {
        self.validate_displacement(displacement)?;
        let radius_squared = displacement.norm_squared();
        if !radius_squared.is_finite() {
            return Err(SphKernelError::NumericRange);
        }
        if radius_squared >= self.support_radius_squared {
            return Ok(SVector::<f64, D>::zeros());
        }

        let q = radius_squared.sqrt() / self.support_radius;
        let gap = 1.0 - q;
        let factor =
            -20.0 * self.normalization / self.support_radius_squared * gap.powi(3);
        if !factor.is_finite() {
            return Err(SphKernelError::NumericRange);
        }
        let gradient = displacement * factor;
        if !gradient.iter().all(|component| component.is_finite()) {
            return Err(SphKernelError::NumericRange);
        }
        Ok(gradient)
    }

    fn validate_displacement(
        &self,
        displacement: &SVector<f64, D>,
    ) -> Result<(), SphKernelError> {
        for axis in 0..D {
            if !displacement[axis].is_finite() {
                return Err(SphKernelError::NonFiniteDisplacement { axis });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Poly6Kernel, SphKernelError, WendlandC2Kernel};
    use nalgebra::SVector;

    fn radial_integral<const D: usize>(unit_sphere_surface_area: f64) -> f64 {
        let kernel = Poly6Kernel::<D>::new(1.0).unwrap();
        let bins = 50_000;
        let dr = 1.0 / bins as f64;
        let mut integral = 0.0;
        for bin in 0..bins {
            let radius = (bin as f64 + 0.5) * dr;
            let mut displacement = SVector::<f64, D>::zeros();
            displacement[0] = radius;
            integral += unit_sphere_surface_area
                * radius.powf(D as f64 - 1.0)
                * kernel.value(&displacement).unwrap()
                * dr;
        }
        integral
    }

    fn close(a: f64, b: f64, tolerance: f64) {
        assert!(
            (a - b).abs() <= tolerance,
            "expected {a:.16e} ~= {b:.16e}; tolerance {tolerance:.3e}",
        );
    }

    #[test]
    fn poly6_normalizes_in_one_two_and_three_dimensions() {
        close(radial_integral::<1>(2.0), 1.0, 2e-8);
        close(radial_integral::<2>(2.0 * std::f64::consts::PI), 1.0, 2e-8);
        close(radial_integral::<3>(4.0 * std::f64::consts::PI), 1.0, 2e-8);
    }

    #[test]
    fn gradient_matches_central_finite_difference() {
        let kernel = Poly6Kernel::<2>::new(1.3).unwrap();
        let displacement = SVector::from([0.2, -0.3]);
        let analytic = kernel.gradient(&displacement).unwrap();
        let epsilon = 1e-6;

        for axis in 0..2 {
            let mut plus = displacement;
            let mut minus = displacement;
            plus[axis] += epsilon;
            minus[axis] -= epsilon;
            let numeric =
                (kernel.value(&plus).unwrap() - kernel.value(&minus).unwrap()) / (2.0 * epsilon);
            close(analytic[axis], numeric, 1e-8);
        }
    }

    #[test]
    fn kernel_is_zero_at_and_outside_support() {
        let kernel = Poly6Kernel::<3>::new(1.0).unwrap();
        assert_eq!(kernel.value(&SVector::from([1.0, 0.0, 0.0])).unwrap(), 0.0);
        assert_eq!(kernel.value(&SVector::from([2.0, 0.0, 0.0])).unwrap(), 0.0);
        assert_eq!(
            kernel.gradient(&SVector::from([1.0, 0.0, 0.0])).unwrap(),
            SVector::<f64, 3>::zeros()
        );
    }

    #[test]
    fn rejects_invalid_smoothing_length_and_non_finite_displacement() {
        assert!(matches!(
            Poly6Kernel::<2>::new(0.0),
            Err(SphKernelError::InvalidSmoothingLength)
        ));
        assert!(matches!(
            Poly6Kernel::<2>::new(f64::INFINITY),
            Err(SphKernelError::InvalidSmoothingLength)
        ));

        let kernel = Poly6Kernel::<2>::new(1.0).unwrap();
        assert!(matches!(
            kernel.value(&SVector::from([f64::NAN, 0.0])),
            Err(SphKernelError::NonFiniteDisplacement { axis: 0 })
        ));
    }

    fn scaling_law<const D: usize>() {
        let base_h = 1.3;
        let mut displacement = SVector::<f64, D>::zeros();
        for axis in 0..D {
            displacement[axis] = 0.08 * (axis as f64 + 1.0);
        }
        let base = Poly6Kernel::<D>::new(base_h).unwrap();
        let base_value = base.value(&displacement).unwrap();
        let base_gradient = base.gradient(&displacement).unwrap();

        for scale in [0.5_f64, 2.0] {
            let scaled = Poly6Kernel::<D>::new(base_h * scale).unwrap();
            let scaled_value = scaled.value(&(displacement * scale)).unwrap();
            let scaled_gradient = scaled.gradient(&(displacement * scale)).unwrap();
            close(
                scaled_value * scale.powi(D as i32),
                base_value,
                2e-12,
            );
            for axis in 0..D {
                close(
                    scaled_gradient[axis] * scale.powi(D as i32 + 1),
                    base_gradient[axis],
                    2e-12,
                );
            }
        }
    }

    #[test]
    fn poly6_obeys_dimensionally_correct_smoothing_length_scaling() {
        scaling_law::<1>();
        scaling_law::<2>();
        scaling_law::<3>();
    }

    fn wendland_radial_integral<const D: usize>(surface_area: f64) -> f64 {
        let kernel = WendlandC2Kernel::<D>::new(1.7).unwrap();
        let bins = 50_000;
        let dr = kernel.support_radius() / bins as f64;
        let mut integral = 0.0;
        for bin in 0..bins {
            let radius = (bin as f64 + 0.5) * dr;
            let mut displacement = SVector::<f64, D>::zeros();
            displacement[0] = radius;
            integral += surface_area
                * radius.powf(D as f64 - 1.0)
                * kernel.value(&displacement).unwrap()
                * dr;
        }
        integral
    }

    #[test]
    fn wendland_c2_normalizes_in_one_two_and_three_dimensions() {
        close(wendland_radial_integral::<1>(2.0), 1.0, 2e-8);
        close(
            wendland_radial_integral::<2>(2.0 * std::f64::consts::PI),
            1.0,
            2e-8,
        );
        close(
            wendland_radial_integral::<3>(4.0 * std::f64::consts::PI),
            1.0,
            2e-8,
        );
    }

    #[test]
    fn wendland_c2_gradient_matches_central_finite_difference() {
        let kernel = WendlandC2Kernel::<3>::new(1.4).unwrap();
        let displacement = SVector::from([0.2, -0.3, 0.1]);
        let analytic = kernel.gradient(&displacement).unwrap();
        let epsilon = 1e-6;

        for axis in 0..3 {
            let mut plus = displacement;
            let mut minus = displacement;
            plus[axis] += epsilon;
            minus[axis] -= epsilon;
            let numeric =
                (kernel.value(&plus).unwrap() - kernel.value(&minus).unwrap())
                    / (2.0 * epsilon);
            close(analytic[axis], numeric, 1e-8);
        }
    }

    #[test]
    fn wendland_c2_is_positive_monotone_and_flat_at_support_boundary() {
        let kernel = WendlandC2Kernel::<2>::new(1.0).unwrap();
        let mut previous = kernel.value(&SVector::zeros()).unwrap();
        assert!(previous > 0.0);
        for step in 1..100 {
            let radius = step as f64 / 100.0;
            let value = kernel
                .value(&SVector::from([radius, 0.0]))
                .unwrap();
            assert!(value >= 0.0);
            assert!(value < previous, "kernel must be strictly decreasing away from zero");
            previous = value;
        }
        assert_eq!(kernel.value(&SVector::from([1.0, 0.0])).unwrap(), 0.0);
        assert_eq!(
            kernel.gradient(&SVector::from([1.0, 0.0])).unwrap(),
            SVector::<f64, 2>::zeros()
        );
        assert_eq!(
            kernel.gradient(&SVector::zeros()).unwrap(),
            SVector::<f64, 2>::zeros()
        );
    }

    fn wendland_scaling_law<const D: usize>() {
        let base_radius = 1.7;
        let mut displacement = SVector::<f64, D>::zeros();
        for axis in 0..D {
            displacement[axis] = 0.08 * (axis as f64 + 1.0);
        }
        let base = WendlandC2Kernel::<D>::new(base_radius).unwrap();
        let base_value = base.value(&displacement).unwrap();
        let base_gradient = base.gradient(&displacement).unwrap();

        for scale in [0.5_f64, 2.0] {
            let scaled = WendlandC2Kernel::<D>::new(base_radius * scale).unwrap();
            let scaled_value = scaled.value(&(displacement * scale)).unwrap();
            let scaled_gradient = scaled.gradient(&(displacement * scale)).unwrap();
            close(scaled_value * scale.powi(D as i32), base_value, 2e-12);
            for axis in 0..D {
                close(
                    scaled_gradient[axis] * scale.powi(D as i32 + 1),
                    base_gradient[axis],
                    2e-12,
                );
            }
        }
    }

    #[test]
    fn wendland_c2_obeys_dimensionally_correct_scaling() {
        wendland_scaling_law::<1>();
        wendland_scaling_law::<2>();
        wendland_scaling_law::<3>();
    }

    #[test]
    fn wendland_c2_is_isotropic_under_orthogonal_rotation() {
        let kernel = WendlandC2Kernel::<2>::new(1.2).unwrap();
        let angle = 0.73_f64;
        let rotation = nalgebra::SMatrix::<f64, 2, 2>::new(
            angle.cos(),
            -angle.sin(),
            angle.sin(),
            angle.cos(),
        );
        let displacement = SVector::from([0.3, -0.4]);
        let rotated = rotation * displacement;
        close(
            kernel.value(&displacement).unwrap(),
            kernel.value(&rotated).unwrap(),
            1e-14,
        );
        assert!(
            (kernel.gradient(&rotated).unwrap() - rotation * kernel.gradient(&displacement).unwrap())
                .norm() < 1e-12
        );
    }

    #[test]
    fn wendland_c2_uses_known_support_radius_normalizations() {
        close(
            WendlandC2Kernel::<1>::new(1.0).unwrap().normalization(),
            1.5,
            1e-14,
        );
        close(
            WendlandC2Kernel::<2>::new(1.0).unwrap().normalization(),
            7.0 / std::f64::consts::PI,
            1e-14,
        );
        close(
            WendlandC2Kernel::<3>::new(1.0).unwrap().normalization(),
            21.0 / (2.0 * std::f64::consts::PI),
            1e-14,
        );
    }

    #[test]
    fn wendland_c2_rejects_unsupported_dimensions_and_invalid_radius() {
        assert!(matches!(
            WendlandC2Kernel::<4>::new(1.0),
            Err(SphKernelError::UnsupportedDimension)
        ));
        assert!(matches!(
            WendlandC2Kernel::<2>::new(f64::NAN),
            Err(SphKernelError::InvalidSmoothingLength)
        ));
        assert!(matches!(
            WendlandC2Kernel::<2>::new(0.0),
            Err(SphKernelError::InvalidSmoothingLength)
        ));
    }

    #[test]
    fn kernel_normalization_matches_known_three_dimensional_poly6() {
        let kernel = Poly6Kernel::<3>::new(1.0).unwrap();
        let expected = 315.0 / (64.0 * std::f64::consts::PI);
        close(kernel.normalization(), expected, 1e-14);
    }
}
