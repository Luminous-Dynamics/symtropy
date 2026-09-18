// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Dependency-neutral canonical value types for ANIMA embodied-agent systems.
//!
//! This crate intentionally contains no Bevy, renderer, physics-world, networking,
//! persistence, HDC, FEP, robotics, or random-number dependencies. Product layers
//! adapt their authoritative identities, clocks, and observations into these
//! bounded value types.

#![forbid(unsafe_code)]

pub mod evidence;
pub mod id;
pub mod quantity;
pub mod reason;
pub mod sample;
pub mod tick;

pub use evidence::EvidenceRef;
pub use id::{
    ActionId, AgentId, AggregationMethodId, EmissionId, ExperienceId, IntentId, PerceptId,
    PolicyId, ProfileId,
};
pub use quantity::{
    ConfidenceQ, FeasibilityQ, InformationGainQ, RiskQ, SalienceQ, StrengthQ, UnitQ, UnitQError,
};
pub use reason::{ReasonCode, UnknownReasonCode};
pub use sample::{
    EvidenceValue, MissingEvidenceReason, SampleProvenance, SampleWindowError,
    UnknownMissingEvidenceReasonCode,
};
pub use tick::{Tick, TickArithmeticError};
