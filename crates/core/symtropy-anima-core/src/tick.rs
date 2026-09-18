//! Canonical simulation tick semantics.

/// Authoritative fixed-step simulation tick.
///
/// This is deliberately not wall-clock time, elapsed floating-point seconds, or
/// a render-frame counter. Product adapters bind it to their authoritative clock.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Tick(u64);

/// Failure from canonical tick arithmetic.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TickArithmeticError {
    /// Adding the requested delta would exceed `u64::MAX`.
    Overflow,
    /// The referenced sample/event tick lies in the future relative to the query tick.
    FutureReference,
}

impl Tick {
    pub const ZERO: Self = Self(0);

    #[must_use]
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }

    pub fn checked_add(self, delta: u64) -> Result<Self, TickArithmeticError> {
        self.0
            .checked_add(delta)
            .map(Self)
            .ok_or(TickArithmeticError::Overflow)
    }

    /// Returns the number of canonical ticks since `earlier`.
    pub fn elapsed_since(self, earlier: Self) -> Result<u64, TickArithmeticError> {
        self.0
            .checked_sub(earlier.0)
            .ok_or(TickArithmeticError::FutureReference)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_add_never_wraps() {
        assert_eq!(
            Tick::new(u64::MAX).checked_add(1),
            Err(TickArithmeticError::Overflow)
        );
    }

    #[test]
    fn elapsed_since_rejects_future_reference() {
        assert_eq!(
            Tick::new(4).elapsed_since(Tick::new(5)),
            Err(TickArithmeticError::FutureReference)
        );
    }

    #[test]
    fn elapsed_since_is_exact() {
        assert_eq!(Tick::new(11).elapsed_since(Tick::new(7)), Ok(4));
    }
}
