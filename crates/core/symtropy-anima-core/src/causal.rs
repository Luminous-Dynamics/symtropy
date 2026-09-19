//! Canonical causal coordinates for ANIMA state transitions.
//!
//! Causal ordering is determined only by `(tick, ordinal)`. `CausalPhase` is
//! descriptive metadata for observability and qualification; enum/source order
//! never grants causal authority.

use crate::Tick;

/// Descriptive stage of an ANIMA causal transition.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CausalPhase {
    Sense,
    Interpret,
    Experience,
    Memory,
    Belief,
    Decision,
    Action,
    Outcome,
}

/// Canonical coordinate for one causal occurrence inside the fixed simulation timeline.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CausalStamp {
    tick: Tick,
    ordinal: u32,
    phase: CausalPhase,
}

impl CausalStamp {
    #[must_use]
    pub const fn new(tick: Tick, ordinal: u32, phase: CausalPhase) -> Self {
        Self {
            tick,
            ordinal,
            phase,
        }
    }

    #[must_use]
    pub const fn tick(self) -> Tick {
        self.tick
    }

    #[must_use]
    pub const fn ordinal(self) -> u32 {
        self.ordinal
    }

    #[must_use]
    pub const fn phase(self) -> CausalPhase {
        self.phase
    }

    /// Returns true only when this causal coordinate strictly precedes `other`.
    ///
    /// Phase metadata is deliberately ignored. Two events at the same
    /// `(tick, ordinal)` never precede one another merely because their phase tags
    /// differ.
    #[must_use]
    pub const fn strictly_precedes(self, other: Self) -> bool {
        self.tick.raw() < other.tick.raw()
            || (self.tick.raw() == other.tick.raw() && self.ordinal < other.ordinal)
    }

    /// Whether two stamps occupy the same canonical causal coordinate.
    #[must_use]
    pub const fn same_coordinate(self, other: Self) -> bool {
        self.tick.raw() == other.tick.raw() && self.ordinal == other.ordinal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn earlier_tick_precedes_later_tick() {
        let earlier = CausalStamp::new(Tick::new(7), 99, CausalPhase::Outcome);
        let later = CausalStamp::new(Tick::new(8), 0, CausalPhase::Sense);

        assert!(earlier.strictly_precedes(later));
        assert!(!later.strictly_precedes(earlier));
    }

    #[test]
    fn lower_ordinal_precedes_within_same_tick() {
        let earlier = CausalStamp::new(Tick::new(7), 3, CausalPhase::Outcome);
        let later = CausalStamp::new(Tick::new(7), 4, CausalPhase::Sense);

        assert!(earlier.strictly_precedes(later));
        assert!(!later.strictly_precedes(earlier));
    }

    #[test]
    fn phase_alone_never_establishes_precedence() {
        let sense = CausalStamp::new(Tick::new(7), 3, CausalPhase::Sense);
        let outcome = CausalStamp::new(Tick::new(7), 3, CausalPhase::Outcome);

        assert!(sense.same_coordinate(outcome));
        assert!(!sense.strictly_precedes(outcome));
        assert!(!outcome.strictly_precedes(sense));
    }
}
