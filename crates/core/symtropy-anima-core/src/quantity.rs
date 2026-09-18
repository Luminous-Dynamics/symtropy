//! Deterministic bounded dimensionless quantities.
//!
//! Semantic score domains remain type-separated even when they share the same
//! normalized representation:
//!
//! ```compile_fail
//! use symtropy_anima_core::{ConfidenceQ, RiskQ};
//!
//! fn require_confidence(_: ConfidenceQ) {}
//!
//! let risk = RiskQ::new(500_000).unwrap();
//! require_confidence(risk);
//! ```

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

macro_rules! semantic_unit_q {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(UnitQ);

        impl $name {
            pub const ZERO: Self = Self(UnitQ::ZERO);
            pub const ONE: Self = Self(UnitQ::ONE);

            /// Constructs this semantic normalized value from a validated raw Q value.
            pub const fn new(raw: u32) -> Result<Self, UnitQError> {
                match UnitQ::new(raw) {
                    Ok(value) => Ok(Self(value)),
                    Err(error) => Err(error),
                }
            }

            /// Wraps an already validated generic normalized value explicitly.
            #[must_use]
            pub const fn from_unit(value: UnitQ) -> Self {
                Self(value)
            }

            /// Returns the underlying generic normalized value explicitly.
            #[must_use]
            pub const fn into_unit(self) -> UnitQ {
                self.0
            }

            /// Returns the exact canonical raw Q representation.
            #[must_use]
            pub const fn raw(self) -> u32 {
                self.0.raw()
            }
        }
    };
}

semantic_unit_q!(
    /// Detected or represented normalized signal strength.
    StrengthQ
);
semantic_unit_q!(
    /// Confidence/precision proxy. This is not automatically a calibrated probability.
    ConfidenceQ
);
semantic_unit_q!(
    /// Bounded risk estimate or policy score. This is not automatically a probability.
    RiskQ
);
semantic_unit_q!(
    /// Bounded affordance/action feasibility score.
    FeasibilityQ
);
semantic_unit_q!(
    /// Bounded memory/attention salience score.
    SalienceQ
);
semantic_unit_q!(
    /// Bounded expected information-gain score.
    InformationGainQ
);

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

    #[test]
    fn semantic_wrappers_preserve_exact_q_values() {
        let raw = 654_321;
        let unit = UnitQ::new(raw).unwrap();

        assert_eq!(StrengthQ::from_unit(unit).raw(), raw);
        assert_eq!(ConfidenceQ::from_unit(unit).raw(), raw);
        assert_eq!(RiskQ::from_unit(unit).raw(), raw);
        assert_eq!(FeasibilityQ::from_unit(unit).raw(), raw);
        assert_eq!(SalienceQ::from_unit(unit).raw(), raw);
        assert_eq!(InformationGainQ::from_unit(unit).raw(), raw);
    }

    #[test]
    fn semantic_wrappers_fail_closed_on_invalid_raw_values() {
        let invalid = UnitQ::SCALE + 1;

        assert_eq!(StrengthQ::new(invalid), Err(UnitQError { raw: invalid }));
        assert_eq!(ConfidenceQ::new(invalid), Err(UnitQError { raw: invalid }));
        assert_eq!(RiskQ::new(invalid), Err(UnitQError { raw: invalid }));
        assert_eq!(FeasibilityQ::new(invalid), Err(UnitQError { raw: invalid }));
        assert_eq!(SalienceQ::new(invalid), Err(UnitQError { raw: invalid }));
        assert_eq!(
            InformationGainQ::new(invalid),
            Err(UnitQError { raw: invalid })
        );
    }
}
