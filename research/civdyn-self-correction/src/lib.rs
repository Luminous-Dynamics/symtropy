// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Small, ledger-backed reference for civilizational self-correction.
//!
//! This is a controlled benchmark kernel, not a general-purpose civilization
//! simulator. It composes the shared resource ledger and deliberately keeps
//! household service, industrial service, reserve capital, maintenance energy,
//! sensor error and correction evidence as separate quantities.

pub mod causal_drift;
pub mod causal_identifiability;
pub mod interference_identifiability;
pub mod joint_adequacy;
pub mod model_adequacy;
pub mod network_adequacy;
pub mod network_interference;
pub mod noisy_causal_drift;
pub mod sequential_methods;

