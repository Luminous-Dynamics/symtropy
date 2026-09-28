// Copyright (c) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

pub mod config;
pub mod economic_events;
pub mod events;
pub mod plugin;
pub mod recognition;
pub mod resource;
pub mod scenarios;
pub mod systems;

pub use config::MycelixConfig;
pub use economic_events::{
    EconomicEventEnvelopeV1, EconomicEventKind, EconomicEventValidationError,
    ReplayFingerprint, SimulationQuantity, SimulationValidity,
};
pub use events::{MycelixRequest, MycelixResponse};
pub use plugin::BevyMycelixPlugin;
pub use recognition::{RecognitionError, RecognitionRecord, RecognitionTarget};
pub use resource::{MycelixClient, MycelixSendError};
pub use scenarios::{ScenarioConfig, ScenarioReport, proposal_vote_invariant};

#[cfg(test)]
mod tests;
