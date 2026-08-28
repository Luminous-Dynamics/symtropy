// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Explicit Bevy plugin for the real-time artistic studio substrate.
//!
//! This plugin initializes deterministic studio time, bounded capture
//! backpressure, pacing evidence, and cinematic history. It does not attach
//! mutation authority, create cameras, or alter existing `CognitiveBrain`
//! behavior.

use bevy::prelude::*;

use crate::art_capture::{ArtCaptureOverflowPolicy, ArtCaptureQueue};
use crate::art_cinema::CinematicHistory;
use crate::art_timeline::{FramePacingLedger, StudioClock, StudioFrameRate};

pub struct RealtimeArtStudioPlugin {
    frame_rate: StudioFrameRate,
    capture_capacity: usize,
    capture_overflow: ArtCaptureOverflowPolicy,
    pacing_capacity: usize,
    auto_advance_clock: bool,
}

impl RealtimeArtStudioPlugin {
    pub fn new(frame_rate: StudioFrameRate) -> Self {
        Self {
            frame_rate,
            capture_capacity: 8,
            capture_overflow: ArtCaptureOverflowPolicy::RejectNewest,
            pacing_capacity: 240,
            auto_advance_clock: true,
        }
    }

    pub fn with_capture_capacity(mut self, capacity: usize) -> Result<Self, StudioPluginError> {
        if capacity == 0 {
            return Err(StudioPluginError::ZeroCaptureCapacity);
        }
        self.capture_capacity = capacity;
        Ok(self)
    }

    pub fn with_capture_overflow(mut self, policy: ArtCaptureOverflowPolicy) -> Self {
        self.capture_overflow = policy;
        self
    }

    pub fn with_pacing_capacity(mut self, capacity: usize) -> Result<Self, StudioPluginError> {
        if capacity == 0 {
            return Err(StudioPluginError::ZeroPacingCapacity);
        }
        self.pacing_capacity = capacity;
        Ok(self)
    }

    pub fn with_auto_advance_clock(mut self, enabled: bool) -> Self {
        self.auto_advance_clock = enabled;
        self
    }
}

impl Default for RealtimeArtStudioPlugin {
    fn default() -> Self {
        Self::new(StudioFrameRate {
            numerator: 24,
            denominator: 1,
        })
    }
}

impl Plugin for RealtimeArtStudioPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(StudioClock::new(self.frame_rate));
        app.insert_resource(
            ArtCaptureQueue::new(self.capture_capacity, self.capture_overflow)
                .expect("RealtimeArtStudioPlugin validates capture capacity"),
        );
        app.insert_resource(
            FramePacingLedger::new(self.pacing_capacity)
                .expect("RealtimeArtStudioPlugin validates pacing capacity"),
        );
        app.insert_resource(CinematicHistory::default());

        if self.auto_advance_clock {
            app.add_systems(FixedUpdate, advance_studio_clock_system);
        }
    }
}

fn advance_studio_clock_system(mut clock: ResMut<StudioClock>) {
    clock.advance();
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StudioPluginError {
    ZeroCaptureCapacity,
    ZeroPacingCapacity,
}

impl std::fmt::Display for StudioPluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroCaptureCapacity => write!(f, "studio capture capacity must be non-zero"),
            Self::ZeroPacingCapacity => write!(f, "studio pacing capacity must be non-zero"),
        }
    }
}

impl std::error::Error for StudioPluginError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::art_timeline::StudioFrame;

    #[test]
    fn plugin_is_opt_in_and_clock_is_deterministic() {
        let mut app = App::new();
        app.add_plugins(
            RealtimeArtStudioPlugin::new(StudioFrameRate::new(24, 1).unwrap())
                .with_auto_advance_clock(false),
        );
        assert_eq!(app.world().resource::<StudioClock>().frame(), StudioFrame(0));
    }

    #[test]
    fn invalid_zero_capacity_is_rejected_before_build() {
        assert!(matches!(
            RealtimeArtStudioPlugin::default().with_capture_capacity(0),
            Err(StudioPluginError::ZeroCaptureCapacity)
        ));
    }
}
