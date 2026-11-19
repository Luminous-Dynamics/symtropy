//! Small binary64 outward-rounded intervals for exploratory quadrature gates.
//!
//! This module is deliberately NOT the proof-grade ball backend. It provides
//! a conservative first filter when every input interval is itself a valid
//! enclosure. For accepted black-hole solution certificates, replay the same
//! predicates with the independent FLINT Arb verifier tracked in issue #6013
//! and cross-check interval behavior against the IEEE-1788 fixtures in #2348.
//!
//! No silent fallback is permitted: overflow, non-finite inputs, division by
//! an interval containing zero, or an unresolved angular-flux denominator
//! return an explicit error.

use core::fmt;

/// Closed finite interval [lower, upper].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    pub lower: f64,
    pub upper: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntervalError {
    InvalidBounds,
    NonFiniteInput,
    NonFiniteArithmetic,
    DivisionByIntervalContainingZero,
    SquareRootOfNegativeInterval,
    IntervalContainsZero,
    IntervalNotStrictlyPositive,
}

impl fmt::Display for IntervalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBounds => write!(f, "interval bounds are reversed"),
            Self::NonFiniteInput => write!(f, "interval input must be finite"),
            Self::NonFiniteArithmetic => write!(f, "outward-rounded operation overflowed"),
            Self::DivisionByIntervalContainingZero => {
                write!(f, "division by an interval containing zero is inconclusive")
            }
            Self::SquareRootOfNegativeInterval => {
                write!(f, "square root requires a nonnegative interval")
            }
            Self::IntervalContainsZero => {
                write!(f, "interval contains zero; nonvanishing is not established")
            }
            Self::IntervalNotStrictlyPositive => {
                write!(f, "interval is not strictly positive")
            }
        }
    }
}

impl std::error::Error for IntervalError {}

/// Return the adjacent representable binary64 value toward negative infinity.
#[inline]
pub fn next_down(value: f64) -> f64 {
    if value.is_nan() || value == f64::NEG_INFINITY {
        return value;
    }
    if value == 0.0 {
        return -f64::from_bits(1);
    }
    let bits = value.to_bits();
    f64::from_bits(if value > 0.0 { bits - 1 } else { bits + 1 })
}

/// Return the adjacent representable binary64 value toward positive infinity.
#[inline]
pub fn next_up(value: f64) -> f64 {
    if value.is_nan() || value == f64::INFINITY {
        return value;
    }
    if value == 0.0 {
        return f64::from_bits(1);
    }
    let bits = value.to_bits();
    f64::from_bits(if value > 0.0 { bits + 1 } else { bits - 1 })
}

impl Interval {
    pub fn new(lower: f64, upper: f64) -> Result<Self, IntervalError> {
        if !lower.is_finite() || !upper.is_finite() {
            return Err(IntervalError::NonFiniteInput);
        }
        if lower > upper {
            return Err(IntervalError::InvalidBounds);
        }
        Ok(Self { lower, upper })
    }

    /// The singleton containing the exact binary64 value supplied.
    pub fn point(value: f64) -> Result<Self, IntervalError> {
        Self::new(value, value)
    }

    pub fn zero() -> Self {
        Self { lower: 0.0, upper: 0.0 }
    }

    pub fn one() -> Self {
        Self { lower: 1.0, upper: 1.0 }
    }

    pub fn contains(self, value: f64) -> bool {
        value.is_finite() && self.lower <= value && value <= self.upper
    }

    pub fn contains_zero(self) -> bool {
        self.lower <= 0.0 && self.upper >= 0.0
    }

    pub fn excludes_zero(self) -> bool {
        self.upper < 0.0 || self.lower > 0.0
    }

    pub fn strictly_positive(self) -> bool {
        self.lower > 0.0
    }

    pub fn absolute_lower_bound(self) -> f64 {
        if self.lower > 0.0 {
            self.lower
        } else if self.upper < 0.0 {
            -self.upper
        } else {
            0.0
        }
    }

    fn checked(lower: f64, upper: f64) -> Result<Self, IntervalError> {
        if !lower.is_finite() || !upper.is_finite() {
            return Err(IntervalError::NonFiniteArithmetic);
        }
        Self::new(lower, upper)
    }

    pub fn add(self, rhs: Self) -> Result<Self, IntervalError> {
        Self::checked(
            next_down(self.lower + rhs.lower),
            next_up(self.upper + rhs.upper),
        )
    }

    pub fn sub(self, rhs: Self) -> Result<Self, IntervalError> {
        Self::checked(
            next_down(self.lower - rhs.upper),
            next_up(self.upper - rhs.lower),
        )
    }

    pub fn mul(self, rhs: Self) -> Result<Self, IntervalError> {
        let products = [
            self.lower * rhs.lower,
            self.lower * rhs.upper,
            self.upper * rhs.lower,
            self.upper * rhs.upper,
        ];
        if products.iter().any(|value| !value.is_finite()) {
            return Err(IntervalError::NonFiniteArithmetic);
        }
        let lower = products.iter().copied().fold(f64::INFINITY, f64::min);
        let upper = products.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        Self::checked(next_down(lower), next_up(upper))
    }

    pub fn div(self, rhs: Self) -> Result<Self, IntervalError> {
        if rhs.contains_zero() {
            return Err(IntervalError::DivisionByIntervalContainingZero);
        }
        let reciprocal = Self::checked(
            next_down(1.0 / rhs.upper),
            next_up(1.0 / rhs.lower),
        )?;
        self.mul(reciprocal)
    }

    pub fn sqrt(self) -> Result<Self, IntervalError> {
        if self.lower < 0.0 {
            return Err(IntervalError::SquareRootOfNegativeInterval);
        }
        let lower = if self.lower == 0.0 {
            0.0
        } else {
            next_down(self.lower.sqrt())
        };
        let upper = next_up(self.upper.sqrt());
        Self::checked(lower, upper)
    }

    /// Square a strictly positive interval without dependency overestimation.
    pub fn square_positive(self) -> Result<Self, IntervalError> {
        if !self.strictly_positive() {
            return Err(IntervalError::IntervalNotStrictlyPositive);
        }
        Self::checked(
            next_down(self.lower * self.lower),
            next_up(self.upper * self.upper),
        )
    }

    pub fn require_excludes_zero(self) -> Result<Self, IntervalError> {
        if self.excludes_zero() {
            Ok(self)
        } else {
            Err(IntervalError::IntervalContainsZero)
        }
    }

    pub fn require_strictly_positive(self) -> Result<Self, IntervalError> {
        if self.strictly_positive() {
            Ok(self)
        } else {
            Err(IntervalError::IntervalNotStrictlyPositive)
        }
    }
}

/// Interval enclosure of the angular-flux quadrature on one radial patch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AngularQuadratureEnclosure5D {
    /// K(r)=r^2-alpha*(f-4+3h/r^2).
    pub bracket_k: Interval,
    /// h*sqrt(f*h/b)*K, the denominator in w'=P_w/denominator.
    pub denominator: Interval,
    /// Enclosure of w'(r) over the supplied radial patch.
    pub reconstructed_w_prime: Interval,
}

/// Enclose K and the angular quadrature for a whole radial patch.
///
/// Every supplied interval must already enclose its corresponding function
/// over the same patch. b, f, h and radius must be strictly positive. If K
/// or the complete denominator contains zero, this returns inconclusive
/// rather than dividing. The caller must also retain the full EGB equations:
/// the Noether current only constrains the cyclic w equation.
pub fn certify_angular_quadrature(
    radius: Interval,
    b: Interval,
    f: Interval,
    h: Interval,
    alpha: Interval,
    conserved_flux: Interval,
) -> Result<AngularQuadratureEnclosure5D, IntervalError> {
    radius.require_strictly_positive()?;
    b.require_strictly_positive()?;
    f.require_strictly_positive()?;
    h.require_strictly_positive()?;

    let radius_sq = radius.square_positive()?;
    let three_h = Interval::point(3.0)?.mul(h)?;
    let angular_term = three_h.div(radius_sq)?;
    let bracket_inner = f
        .sub(Interval::point(4.0)?)?
        .add(angular_term)?;
    let correction = alpha.mul(bracket_inner)?;
    let bracket_k = radius_sq.sub(correction)?.require_excludes_zero()?;

    let fh_over_b = f.mul(h)?.div(b)?;
    let radial_factor = fh_over_b.sqrt()?;
    let denominator = h
        .mul(radial_factor)?
        .mul(bracket_k)?
        .require_excludes_zero()?;
    let reconstructed_w_prime = conserved_flux.div(denominator)?;

    Ok(AngularQuadratureEnclosure5D {
        bracket_k,
        denominator,
        reconstructed_w_prime,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outward_rounding_encloses_basic_non_dyadic_arithmetic() {
        let a = Interval::point(0.1).unwrap();
        let b = Interval::point(0.2).unwrap();
        let sum = a.add(b).unwrap();
        assert!(sum.contains(0.1_f64 + 0.2_f64));
        assert!(sum.lower <= 0.3 && sum.upper >= 0.3);

        let product = a.mul(b).unwrap();
        assert!(product.contains(0.02));
        let quotient = product.div(a).unwrap();
        assert!(quotient.contains(0.2));
    }

    #[test]
    fn division_by_zero_containing_interval_is_rejected() {
        let numerator = Interval::point(1.0).unwrap();
        let denominator = Interval::new(-1.0, 1.0).unwrap();
        assert_eq!(
            numerator.div(denominator),
            Err(IntervalError::DivisionByIntervalContainingZero)
        );
    }

    #[test]
    fn angular_quadrature_certifies_a_separated_positive_k_patch() {
        let enclosure = certify_angular_quadrature(
            Interval::new(2.0, 2.01).unwrap(),
            Interval::new(0.5, 0.55).unwrap(),
            Interval::new(0.7, 0.8).unwrap(),
            Interval::new(3.5, 3.7).unwrap(),
            Interval::new(0.1, 0.2).unwrap(),
            Interval::new(-0.5, -0.4).unwrap(),
        )
        .unwrap();
        assert!(enclosure.bracket_k.strictly_positive());
        assert!(enclosure.denominator.strictly_positive());
        assert!(enclosure.reconstructed_w_prime.upper < 0.0);
    }

    #[test]
    fn angular_quadrature_abstains_when_k_interval_crosses_zero() {
        let result = certify_angular_quadrature(
            Interval::point(2.0).unwrap(),
            Interval::point(0.5).unwrap(),
            Interval::point(0.9).unwrap(),
            Interval::point(8.0).unwrap(),
            Interval::new(1.3, 1.5).unwrap(),
            Interval::point(-0.1).unwrap(),
        );
        assert_eq!(result, Err(IntervalError::IntervalContainsZero));
    }

    #[test]
    fn angular_quadrature_rejects_nonpositive_metric_intervals() {
        let result = certify_angular_quadrature(
            Interval::new(2.0, 2.1).unwrap(),
            Interval::new(-0.1, 0.2).unwrap(),
            Interval::new(0.7, 0.8).unwrap(),
            Interval::new(3.5, 3.7).unwrap(),
            Interval::point(0.2).unwrap(),
            Interval::point(-0.5).unwrap(),
        );
        assert_eq!(result, Err(IntervalError::IntervalNotStrictlyPositive));
    }

    #[test]
    fn overflow_is_inconclusive_not_a_finite_certificate() {
        let huge = Interval::point(f64::MAX).unwrap();
        assert_eq!(
            huge.mul(Interval::point(2.0).unwrap()),
            Err(IntervalError::NonFiniteArithmetic)
        );
    }
}
