//! Deterministic bounded dimensionless quantities.

use core::fmt;

/// Canonical normalized value in the closed interval `[0, 1]` using a fixed
/// integer scale of one million units.
///
/// This matches the frozen fauna-perception oracle convention in ANIMA-07:
/// `Q = 1_000_000`, widened multiplication, and integer-division floor.
/// It is **not** a physical-unit type and must not represent metres, seconds,
/// joules, temperature, force, torque, or other dimensional quantities.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct UnitQ(u32);

/// Invalid canonical normalized input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnitQError {
    raw: u32,
}

impl UnitQ {
    pub const SCALE: u32 = 1_000_000;
    pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(Self::SCALE);

    /// Constructs a canonical value, rejecting values outside the closed domain.
    pub const fn new(raw: u32) -> Result<Self, UnitQError> {
        if raw <= Self::SCALE {
            Ok(Self(raw))
        } else {
            Err(UnitQError { raw })
        }
    }

    #[must_use]
    pub const fn raw(self) -> u32 {
        self.0
    }

    /// Multiplies two normalized values using a widened intermediate and floors
    /// the result by canonical integer division.
    #[must_use]
    pub const fn mul_floor(self, rhs: Self) -> Self {
        let product = (self.0 as u128) * (rhs.0 as u128);
        let raw = product / (Self::SCALE as u128);
        // Both operands are already in-domain, therefore raw <= SCALE.
        Self(raw as u32)
    }
}

impl UnitQError {
    #[must_use]
    pub const fn raw(self) -> u32 {
        self.raw
    }
}

impl fmt::Display for UnitQError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "normalized value {} exceeds canonical scale {}",
            self.raw,
            UnitQ::SCALE
        )
    }
}

impl std::error::Error for UnitQError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_is_closed_and_fail_closed() {
        assert_eq!(UnitQ::new(0), Ok(UnitQ::ZERO));
        assert_eq!(UnitQ::new(UnitQ::SCALE), Ok(UnitQ::ONE));
        assert_eq!(
            UnitQ::new(UnitQ::SCALE + 1),
            Err(UnitQError {
                raw: UnitQ::SCALE + 1
            })
        );
    }

    #[test]
    fn multiplication_matches_frozen_floor_semantics() {
        let half = UnitQ::new(500_000).unwrap();
        let thirdish = UnitQ::new(333_333).unwrap();
        let near_one = UnitQ::new(999_999).unwrap();

        assert_eq!(UnitQ::ZERO.mul_floor(UnitQ::ONE), UnitQ::ZERO);
        assert_eq!(UnitQ::ONE.mul_floor(half), half);
        assert_eq!(half.mul_floor(half).raw(), 250_000);
        assert_eq!(thirdish.mul_floor(thirdish).raw(), 111_110);
        assert_eq!(near_one.mul_floor(near_one).raw(), 999_998);
    }

    #[test]
    fn maximum_product_is_widened_before_division() {
        let raw_product = (UnitQ::SCALE as u128) * (UnitQ::SCALE as u128);
        assert_eq!(raw_product, 1_000_000_000_000);
        assert_eq!(UnitQ::ONE.mul_floor(UnitQ::ONE), UnitQ::ONE);
    }
}
