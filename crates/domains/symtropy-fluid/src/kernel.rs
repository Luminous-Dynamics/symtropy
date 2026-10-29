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
            },
            Self::InvalidSmoothingLength => {
                write!(
                    f,
                    "smoothing length must be finite, positive, and numerically representable"
                )
            }
            Self::NonFiniteDisplacement { axis } => {
                write!(f, "non-finite kernel displacement on axis {axis}")
            }
            Self::NumericRange => write!(f, "SPH kernel arithmetic exceeded the finite numeric range"),
        }
    }
}

impl Error for SphKernelError {}

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
    if D % 2 == 0 {
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

#[cfg(test)]
mod tests {
    use super::{Poly6Kernel, SphKernelError};
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

    #[test]
    fn kernel_normalization_matches_known_three_dimensional_poly6() {
        let kernel = Poly6Kernel::<3>::new(1.0).unwrap();
        let expected = 315.0 / (64.0 * std::f64::consts::PI);
        close(kernel.normalization(), expected, 1e-14);
    }
}
