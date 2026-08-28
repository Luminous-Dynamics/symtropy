// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic artistic timeline and frame-pacing receipts for Bevy studios.
//!
//! Studio time is an artistic/simulation coordinate, not wall-clock identity.
//! Expensive perception and critique can therefore run asynchronously without
//! changing which frame a shot, proposal, or render observation refers to.

use bevy::prelude::*;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Reflect)]
pub struct StudioFrame(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub struct StudioFrameRate {
    pub numerator: u32,
    pub denominator: u32,
}

impl StudioFrameRate {
    pub fn new(numerator: u32, denominator: u32) -> Result<Self, StudioTimelineError> {
        if numerator == 0 || denominator == 0 {
            return Err(StudioTimelineError::InvalidFrameRate);
        }
        Ok(Self {
            numerator,
            denominator,
        })
    }

    pub fn frames_per_second(self) -> f64 {
        f64::from(self.numerator) / f64::from(self.denominator)
    }
}

#[derive(Resource, Debug, Clone)]
pub struct StudioClock {
    frame: StudioFrame,
    frame_rate: StudioFrameRate,
    paused: bool,
}

impl StudioClock {
    pub fn new(frame_rate: StudioFrameRate) -> Self {
        Self {
            frame: StudioFrame(0),
            frame_rate,
            paused: false,
        }
    }

    pub fn frame(&self) -> StudioFrame {
        self.frame
    }

    pub fn frame_rate(&self) -> StudioFrameRate {
        self.frame_rate
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    /// Advance by one artistic frame. Wall time never determines frame identity.
    pub fn advance(&mut self) -> StudioFrame {
        if !self.paused {
            self.frame = StudioFrame(self.frame.0.saturating_add(1));
        }
        self.frame
    }

    pub fn seek(&mut self, frame: StudioFrame) {
        self.frame = frame;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FramePacingSample {
    pub frame: StudioFrame,
    pub wall_duration_micros: u64,
    pub simulation_duration_micros: u64,
    pub capture_duration_micros: u64,
}

/// Bounded performance evidence. It is diagnostic timing data, never an
/// aesthetic objective or reward signal.
#[derive(Resource, Debug, Clone)]
pub struct FramePacingLedger {
    capacity: usize,
    samples: VecDeque<FramePacingSample>,
    dropped_samples: u64,
}

impl FramePacingLedger {
    pub fn new(capacity: usize) -> Result<Self, StudioTimelineError> {
        if capacity == 0 {
            return Err(StudioTimelineError::ZeroCapacity);
        }
        Ok(Self {
            capacity,
            samples: VecDeque::with_capacity(capacity),
            dropped_samples: 0,
        })
    }

    /// Append a sample and return the frame evicted by the bounded ledger.
    pub fn push(&mut self, sample: FramePacingSample) -> Option<StudioFrame> {
        let evicted = if self.samples.len() == self.capacity {
            self.samples.pop_front().map(|sample| sample.frame)
        } else {
            None
        };
        if evicted.is_some() {
            self.dropped_samples = self.dropped_samples.saturating_add(1);
        }
        self.samples.push_back(sample);
        evicted
    }

    pub fn samples(&self) -> impl Iterator<Item = &FramePacingSample> {
        self.samples.iter()
    }

    pub fn dropped_samples(&self) -> u64 {
        self.dropped_samples
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StudioTimelineError {
    InvalidFrameRate,
    ZeroCapacity,
}

impl std::fmt::Display for StudioTimelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFrameRate => {
                write!(f, "studio frame-rate numerator/denominator must be non-zero")
            }
            Self::ZeroCapacity => write!(f, "bounded studio ledger capacity must be non-zero"),
        }
    }
}

impl std::error::Error for StudioTimelineError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artistic_frame_identity_is_deterministic() {
        let rate = StudioFrameRate::new(24, 1).unwrap();
        let mut a = StudioClock::new(rate);
        let mut b = StudioClock::new(rate);
        for _ in 0..100 {
            assert_eq!(a.advance(), b.advance());
        }
        assert_eq!(a.frame(), StudioFrame(100));
    }

    #[test]
    fn pause_preserves_frame_identity() {
        let mut clock = StudioClock::new(StudioFrameRate::new(30, 1).unwrap());
        clock.advance();
        clock.set_paused(true);
        assert_eq!(clock.advance(), StudioFrame(1));
    }

    #[test]
    fn pacing_ledger_is_bounded_and_reports_eviction() {
        let mut ledger = FramePacingLedger::new(2).unwrap();
        let sample = |frame| FramePacingSample {
            frame: StudioFrame(frame),
            wall_duration_micros: 100,
            simulation_duration_micros: 80,
            capture_duration_micros: 5,
        };
        assert_eq!(ledger.push(sample(1)), None);
        assert_eq!(ledger.push(sample(2)), None);
        assert_eq!(ledger.push(sample(3)), Some(StudioFrame(1)));
        assert_eq!(ledger.samples().count(), 2);
        assert_eq!(ledger.dropped_samples(), 1);
    }
}
