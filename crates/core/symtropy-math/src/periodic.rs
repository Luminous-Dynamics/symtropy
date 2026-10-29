// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Validated orthogonal periodic-domain primitives.
//!
//! A periodic axis represents a quotient by integer multiples of its period.
//! This module provides point wrapping and displacement only; it does not claim
//! that rigid-body collision, CCD, or curved-manifold physics are periodic-aware.
//!
//! The canonical interval on each periodic axis is half-open:
//! [origin, origin + period). Image offsets satisfy
//! original_point = canonical_point + offset * period (axis by axis).

use std::error::Error;
use std::fmt;

use nalgebra::SVector;

/// Largest period quotient accepted before floating-point remainder semantics
/// become too difficult to trust. This is a conservative guard, not a proof of
/// exact coordinate representation.
const MAX_EXACT_PERIOD_INDEX: f64 = 4_503_599_627_370_496.0; // 2^52

/// Why a periodic-domain operation could not produce a trustworthy result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeriodicDomainError {
    InvalidDimension,
    NonFiniteOrigin { axis: usize },
    NonFinitePeriod { axis: usize },
    NonPositivePeriod { axis: usize },
    UnrepresentablePeriod { axis: usize },
    NonFiniteCoordinate { axis: usize },
    CoordinatePrecisionLimit { axis: usize },
    InvalidImageOffset { axis: usize },
    OffsetOnNonPeriodicAxis { axis: usize },
}

impl fmt::Display for PeriodicDomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDimension => write!(f, "periodic domain requires D >= 1"),
            Self::NonFiniteOrigin { axis } => write!(f, "non-finite origin on axis {axis}"),
            Self::NonFinitePeriod { axis } => write!(f, "non-finite period on axis {axis}"),
            Self::NonPositivePeriod { axis } => {
                write!(f, "period must be positive on periodic axis {axis}")
            }
            Self::UnrepresentablePeriod { axis } => {
                write!(
                    f,
                    "origin + period is not representably greater than origin on axis {axis}"
                )
            }
            Self::NonFiniteCoordinate { axis } => {
                write!(f, "non-finite point/displacement on axis {axis}")
            }
            Self::CoordinatePrecisionLimit { axis } => {
                write!(
                    f,
                    "periodic coordinate exceeds precision envelope on axis {axis}"
                )
            }
            Self::InvalidImageOffset { axis } => {
                write!(
                    f,
                    "image offset is outside the supported numeric envelope on axis {axis}"
                )
            }
            Self::OffsetOnNonPeriodicAxis { axis } => {
                write!(f, "nonzero image offset on non-periodic axis {axis}")
            }
        }
    }
}

impl Error for PeriodicDomainError {}

/// Integer lattice-image coordinates.
///
/// An offset component of +1 means one positive period translation on that
/// axis; -1 means one negative period translation. Non-periodic axes must
/// always have offset zero when applied to a domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LatticeOffset<const D: usize> {
    pub periods: [i64; D],
}

impl<const D: usize> LatticeOffset<D> {
    /// Zero image: the canonical copy in the original domain.
    pub const fn zero() -> Self {
        Self { periods: [0; D] }
    }

    /// Create an explicit integer image offset. Domain compatibility is checked
    /// when applied through PeriodicBox::apply_offset.
    pub const fn new(periods: [i64; D]) -> Self {
        Self { periods }
    }

    pub const fn as_array(&self) -> &[i64; D] {
        &self.periods
    }
}

/// Canonical position plus the image offset from which it was wrapped.
#[derive(Clone, Debug, PartialEq)]
pub struct WrappedPoint<const D: usize> {
    pub position: SVector<f64, D>,
    pub image_offset: LatticeOffset<D>,
}

/// An orthogonal domain with a selectable periodic mask.
///
/// periods[i] is used only when periodic_axes[i] is true; all period
/// components must nevertheless be finite so invalid configuration is never
/// hidden in inactive axes. The domain is immutable after construction.
#[derive(Clone, Debug, PartialEq)]
pub struct PeriodicBox<const D: usize> {
    origin: SVector<f64, D>,
    periods: SVector<f64, D>,
    periodic_axes: [bool; D],
}

impl<const D: usize> PeriodicBox<D> {
    /// Construct a domain or return a typed configuration error.
    pub fn new(
        origin: SVector<f64, D>,
        periods: SVector<f64, D>,
        periodic_axes: [bool; D],
    ) -> Result<Self, PeriodicDomainError> {
        if D == 0 {
            return Err(PeriodicDomainError::InvalidDimension);
        }

        for axis in 0..D {
            if !origin[axis].is_finite() {
                return Err(PeriodicDomainError::NonFiniteOrigin { axis });
            }
            if !periods[axis].is_finite() {
                return Err(PeriodicDomainError::NonFinitePeriod { axis });
            }

            if periodic_axes[axis] {
                if periods[axis] <= 0.0 {
                    return Err(PeriodicDomainError::NonPositivePeriod { axis });
                }
                let upper = origin[axis] + periods[axis];
                if !upper.is_finite() || upper <= origin[axis] {
                    return Err(PeriodicDomainError::UnrepresentablePeriod { axis });
                }
            }
        }

        Ok(Self {
            origin,
            periods,
            periodic_axes,
        })
    }

    pub fn origin(&self) -> &SVector<f64, D> {
        &self.origin
    }

    pub fn periods(&self) -> &SVector<f64, D> {
        &self.periods
    }

    pub fn periodic_axes(&self) -> &[bool; D] {
        &self.periodic_axes
    }

    pub fn is_periodic(&self, axis: usize) -> Option<bool> {
        self.periodic_axes.get(axis).copied()
    }

    /// Wrap a point into [origin, origin + period) on active axes.
    ///
    /// The returned offset records which lattice image the original point
    /// occupied. If floating-point arithmetic cannot represent the canonical
    /// point reliably, return an error rather than silently aliasing it.
    pub fn wrap_point(
        &self,
        point: &SVector<f64, D>,
    ) -> Result<WrappedPoint<D>, PeriodicDomainError> {
        let mut canonical = *point;
        let mut offsets = [0_i64; D];

        for axis in 0..D {
            if !point[axis].is_finite() {
                return Err(PeriodicDomainError::NonFiniteCoordinate { axis });
            }
            if !self.periodic_axes[axis] {
                continue;
            }

            let delta = point[axis] - self.origin[axis];
            if !delta.is_finite() {
                return Err(PeriodicDomainError::CoordinatePrecisionLimit { axis });
            }
            let quotient = delta / self.periods[axis];
            if !quotient.is_finite() || quotient.abs() >= MAX_EXACT_PERIOD_INDEX {
                return Err(PeriodicDomainError::CoordinatePrecisionLimit { axis });
            }

            let image = quotient.floor();
            offsets[axis] = image as i64;

            let remainder = delta.rem_euclid(self.periods[axis]);
            let wrapped = self.origin[axis] + remainder;
            let upper = self.origin[axis] + self.periods[axis];

            if !remainder.is_finite() || !wrapped.is_finite() {
                return Err(PeriodicDomainError::CoordinatePrecisionLimit { axis });
            }
            if remainder > 0.0 && (wrapped <= self.origin[axis] || wrapped >= upper) {
                return Err(PeriodicDomainError::CoordinatePrecisionLimit { axis });
            }

            canonical[axis] = if remainder == 0.0 {
                self.origin[axis]
            } else {
                wrapped
            };
        }

        Ok(WrappedPoint {
            position: canonical,
            image_offset: LatticeOffset::new(offsets),
        })
    }

    /// Convert an explicit lattice offset into a translation vector.
    pub fn offset_vector(
        &self,
        offset: LatticeOffset<D>,
    ) -> Result<SVector<f64, D>, PeriodicDomainError> {
        let mut result = SVector::<f64, D>::zeros();

        for axis in 0..D {
            let count = offset.periods[axis];
            if count == 0 {
                continue;
            }
            if !self.periodic_axes[axis] {
                return Err(PeriodicDomainError::OffsetOnNonPeriodicAxis { axis });
            }
            let count_f64 = count as f64;
            if count_f64.abs() >= MAX_EXACT_PERIOD_INDEX {
                return Err(PeriodicDomainError::InvalidImageOffset { axis });
            }
            let translation = count_f64 * self.periods[axis];
            if !translation.is_finite() {
                return Err(PeriodicDomainError::InvalidImageOffset { axis });
            }
            result[axis] = translation;
        }

        Ok(result)
    }

    /// Translate a point by an explicit periodic image offset.
    pub fn apply_offset(
        &self,
        point: &SVector<f64, D>,
        offset: LatticeOffset<D>,
    ) -> Result<SVector<f64, D>, PeriodicDomainError> {
        for axis in 0..D {
            if !point[axis].is_finite() {
                return Err(PeriodicDomainError::NonFiniteCoordinate { axis });
            }
        }

        let translated = point + self.offset_vector(offset)?;
        for axis in 0..D {
            if !translated[axis].is_finite() {
                return Err(PeriodicDomainError::InvalidImageOffset { axis });
            }
        }
        Ok(translated)
    }

    /// Find a nearest periodic representative of a displacement.
    ///
    /// The result lies in [-period/2, period/2] for periodic axes. At exact
    /// half-period ties, the sign of the input displacement chooses the sign
    /// of the result; this preserves oddness under displacement reversal.
    /// Because nearest images are non-unique at ties, translation invariance
    /// is only asserted away from that boundary.
    pub fn minimum_image_displacement(
        &self,
        displacement: &SVector<f64, D>,
    ) -> Result<SVector<f64, D>, PeriodicDomainError> {
        let mut result = *displacement;

        for axis in 0..D {
            let delta = displacement[axis];
            if !delta.is_finite() {
                return Err(PeriodicDomainError::NonFiniteCoordinate { axis });
            }
            if !self.periodic_axes[axis] {
                continue;
            }

            let quotient = delta / self.periods[axis];
            if !quotient.is_finite() || quotient.abs() >= MAX_EXACT_PERIOD_INDEX {
                return Err(PeriodicDomainError::CoordinatePrecisionLimit { axis });
            }

            let wrapped = delta.rem_euclid(self.periods[axis]);
            let half = self.periods[axis] * 0.5;
            result[axis] = if wrapped > half {
                wrapped - self.periods[axis]
            } else if wrapped == half && delta < 0.0 {
                -half
            } else {
                wrapped
            };

            if !result[axis].is_finite() {
                return Err(PeriodicDomainError::CoordinatePrecisionLimit { axis });
            }
        }

        Ok(result)
    }

    /// Compute the nearest periodic representative of to - from.
    pub fn displacement(
        &self,
        from: &SVector<f64, D>,
        to: &SVector<f64, D>,
    ) -> Result<SVector<f64, D>, PeriodicDomainError> {
        let delta = to - from;
        if !delta.iter().all(|x| x.is_finite()) {
            let axis = delta.iter().position(|x| !x.is_finite()).unwrap_or(0);
            return Err(PeriodicDomainError::NonFiniteCoordinate { axis });
        }
        self.minimum_image_displacement(&delta)
    }
}

#[cfg(test)]
mod tests {
    use super::{LatticeOffset, PeriodicBox, PeriodicDomainError};
    use nalgebra::SVector;

    fn domain() -> PeriodicBox<2> {
        PeriodicBox::new(
            SVector::from([10.0, -2.0]),
            SVector::from([4.0, 6.0]),
            [true, true],
        )
        .unwrap()
    }

    fn close(a: f64, b: f64) {
        assert!((a - b).abs() <= 1e-12, "expected {a} ~= {b}");
    }

    #[test]
    fn wraps_positive_and_negative_coordinates_with_explicit_offsets() {
        let d = domain();
        let w = d.wrap_point(&SVector::from([21.0, -3.0])).unwrap();
        close(w.position[0], 13.0);
        close(w.position[1], 3.0);
        assert_eq!(w.image_offset, LatticeOffset::new([2, -1]));
        let restored = d.apply_offset(&w.position, w.image_offset).unwrap();
        close(restored[0], 21.0);
        close(restored[1], -3.0);
    }

    #[test]
    fn wrapping_is_idempotent_for_canonical_points() {
        let d = domain();
        let first = d.wrap_point(&SVector::from([21.0, -3.0])).unwrap();
        let second = d.wrap_point(&first.position).unwrap();
        assert_eq!(second.image_offset, LatticeOffset::zero());
        for axis in 0..2 {
            close(second.position[axis], first.position[axis]);
        }
    }

    #[test]
    fn minimum_image_crosses_both_seams() {
        let d =
            PeriodicBox::new(SVector::zeros(), SVector::from([10.0, 10.0]), [true, true]).unwrap();
        let v = d
            .minimum_image_displacement(&SVector::from([9.0, -9.0]))
            .unwrap();
        close(v[0], -1.0);
        close(v[1], 1.0);
    }

    #[test]
    fn exact_half_period_ties_preserve_reversal_oddness() {
        let d = PeriodicBox::new(
            SVector::zeros(),
            SVector::from([10.0, 10.0]),
            [true, true],
        )
        .unwrap();
        let positive = d
            .minimum_image_displacement(&SVector::from([5.0, -5.0]))
            .unwrap();
        let negative = d
            .minimum_image_displacement(&SVector::from([-5.0, 5.0]))
            .unwrap();
        close(positive[0], 5.0);
        close(positive[1], -5.0);
        close(negative[0], -5.0);
        close(negative[1], 5.0);
    }

    #[test]
    fn non_periodic_axes_are_unchanged() {
        let d = PeriodicBox::new(
            SVector::from([0.0, -100.0]),
            SVector::from([10.0, 0.0]),
            [true, false],
        )
        .unwrap();
        let w = d.wrap_point(&SVector::from([12.0, 900.0])).unwrap();
        close(w.position[0], 2.0);
        close(w.position[1], 900.0);
        assert_eq!(w.image_offset, LatticeOffset::new([1, 0]));
        let delta = d
            .minimum_image_displacement(&SVector::from([9.0, 1500.0]))
            .unwrap();
        close(delta[0], -1.0);
        close(delta[1], 1500.0);
    }

    #[test]
    fn lattice_translation_preserves_canonical_point() {
        let d = domain();
        let p = SVector::from([11.25, -0.5]);
        let shifted = d.apply_offset(&p, LatticeOffset::new([17, -4])).unwrap();
        let a = d.wrap_point(&p).unwrap();
        let b = d.wrap_point(&shifted).unwrap();
        for axis in 0..2 {
            close(a.position[axis], b.position[axis]);
        }
    }

    #[test]
    fn rejects_invalid_period_and_non_finite_coordinates() {
        assert!(matches!(
            PeriodicBox::new(SVector::zeros(), SVector::from([0.0, 1.0]), [true, false]),
            Err(PeriodicDomainError::NonPositivePeriod { axis: 0 })
        ));

        let d = domain();
        assert!(matches!(
            d.wrap_point(&SVector::from([f64::NAN, 0.0])),
            Err(PeriodicDomainError::NonFiniteCoordinate { axis: 0 })
        ));
    }

    #[test]
    fn refuses_unrepresentably_large_period_quotients() {
        let d =
            PeriodicBox::new(SVector::zeros(), SVector::from([1.0, 1.0]), [true, true]).unwrap();
        assert!(matches!(
            d.wrap_point(&SVector::from([1e18, 0.0])),
            Err(PeriodicDomainError::CoordinatePrecisionLimit { axis: 0 })
        ));
    }

    #[test]
    fn offsets_are_rejected_on_non_periodic_axes() {
        let d =
            PeriodicBox::new(SVector::zeros(), SVector::from([10.0, 1.0]), [true, false]).unwrap();
        assert!(matches!(
            d.offset_vector(LatticeOffset::new([0, 1])),
            Err(PeriodicDomainError::OffsetOnNonPeriodicAxis { axis: 1 })
        ));
    }
}
