// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic multi-rate ecological scheduling over one authoritative tick.
//!
//! Living processes operate at very different rates, but they must not invent
//! independent authoritative clocks. [`EcologicalCadence`] derives slow/fast
//! process schedules from the world simulation tick without consulting wall
//! time, frame time, or renderer state.

use std::error::Error;
use std::fmt;
use std::num::NonZeroU64;

/// Periodic schedule expressed entirely in authoritative simulation ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EcologicalCadence {
    interval_ticks: NonZeroU64,
    phase_tick: u64,
}

impl EcologicalCadence {
    /// Construct a cadence with a non-zero interval and optional phase offset.
    pub fn new(interval_ticks: u64, phase_tick: u64) -> Result<Self, CadenceError> {
        let interval_ticks = NonZeroU64::new(interval_ticks).ok_or(CadenceError::ZeroInterval)?;
        Ok(Self {
            interval_ticks,
            phase_tick,
        })
    }

    pub const fn interval_ticks(self) -> u64 {
        self.interval_ticks.get()
    }

    pub const fn phase_tick(self) -> u64 {
        self.phase_tick
    }

    /// Return whether this process is scheduled exactly at `simulation_tick`.
    pub fn fires_at(self, simulation_tick: u64) -> bool {
        simulation_tick >= self.phase_tick
            && (simulation_tick - self.phase_tick).is_multiple_of(self.interval_ticks.get())
    }

    /// Count scheduled process steps in `(previous_tick, current_tick]`.
    ///
    /// This supports deterministic catch-up when a coarse/off-screen system is
    /// advanced in larger chunks. A caller can execute the returned number of
    /// logical process steps without changing the process rate.
    pub fn due_steps(self, previous_tick: u64, current_tick: u64) -> Result<u64, CadenceError> {
        if current_tick < previous_tick {
            return Err(CadenceError::NonMonotonicTick {
                previous_tick,
                current_tick,
            });
        }

        let due = self.count_through(current_tick) - self.count_through(previous_tick);
        u64::try_from(due).map_err(|_| CadenceError::StepCountOverflow {
            previous_tick,
            current_tick,
        })
    }

    /// Number of scheduled firings in `[phase_tick, tick]`.
    ///
    /// This count is deliberately widened to `u128`: for an interval of one
    /// tick beginning at zero, the inclusive count through `u64::MAX` is
    /// `2^64`, even though any `(previous, current]` catch-up interval remains
    /// representable as `u64` steps.
    fn count_through(self, tick: u64) -> u128 {
        if tick < self.phase_tick {
            return 0;
        }
        u128::from(tick - self.phase_tick) / u128::from(self.interval_ticks.get()) + 1
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CadenceError {
    ZeroInterval,
    NonMonotonicTick {
        previous_tick: u64,
        current_tick: u64,
    },
    StepCountOverflow {
        previous_tick: u64,
        current_tick: u64,
    },
}

impl fmt::Display for CadenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroInterval => write!(formatter, "ecological cadence interval must be non-zero"),
            Self::NonMonotonicTick {
                previous_tick,
                current_tick,
            } => write!(
                formatter,
                "ecological cadence cannot run backward: previous tick {previous_tick}, current tick {current_tick}"
            ),
            Self::StepCountOverflow {
                previous_tick,
                current_tick,
            } => write!(
                formatter,
                "ecological cadence step count does not fit u64 for interval ({previous_tick}, {current_tick}]"
            ),
        }
    }
}

impl Error for CadenceError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_interval_is_rejected() {
        assert_eq!(
            EcologicalCadence::new(0, 0),
            Err(CadenceError::ZeroInterval)
        );
    }

    #[test]
    fn cadence_fires_only_on_authoritative_tick_boundaries() {
        let cadence = EcologicalCadence::new(5, 0).unwrap();
        assert!(cadence.fires_at(0));
        assert!(!cadence.fires_at(1));
        assert!(cadence.fires_at(5));
        assert!(cadence.fires_at(10));
    }

    #[test]
    fn phase_offset_is_respected() {
        let cadence = EcologicalCadence::new(4, 3).unwrap();
        assert!(!cadence.fires_at(0));
        assert!(!cadence.fires_at(2));
        assert!(cadence.fires_at(3));
        assert!(cadence.fires_at(7));
        assert!(!cadence.fires_at(8));
    }

    #[test]
    fn catch_up_counts_steps_without_changing_rate() {
        let cadence = EcologicalCadence::new(5, 0).unwrap();
        assert_eq!(cadence.due_steps(0, 4).unwrap(), 0);
        assert_eq!(cadence.due_steps(0, 5).unwrap(), 1);
        assert_eq!(cadence.due_steps(5, 20).unwrap(), 3);
        assert_eq!(cadence.due_steps(20, 20).unwrap(), 0);
    }

    #[test]
    fn phase_aware_catch_up_is_exact() {
        let cadence = EcologicalCadence::new(4, 3).unwrap();
        assert_eq!(cadence.due_steps(0, 2).unwrap(), 0);
        assert_eq!(cadence.due_steps(0, 3).unwrap(), 1);
        assert_eq!(cadence.due_steps(3, 11).unwrap(), 2);
    }

    #[test]
    fn maximum_tick_range_does_not_overflow_inclusive_counter() {
        let cadence = EcologicalCadence::new(1, 0).unwrap();
        assert_eq!(cadence.due_steps(0, u64::MAX).unwrap(), u64::MAX);
        assert!(cadence.fires_at(u64::MAX));
    }

    #[test]
    fn backward_time_fails_closed() {
        let cadence = EcologicalCadence::new(5, 0).unwrap();
        assert_eq!(
            cadence.due_steps(10, 9),
            Err(CadenceError::NonMonotonicTick {
                previous_tick: 10,
                current_tick: 9,
            })
        );
    }
}
