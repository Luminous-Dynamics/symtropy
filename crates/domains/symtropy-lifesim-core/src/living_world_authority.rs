// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Authority/evidence resolvers for Living World canonical transitions.
//!
//! This module is intentionally a stable crate-root surface. Individual
//! authority producers live underneath it so adding another resolver does not
//! repeatedly churn the large `lib.rs` module list.

pub mod candidate_domain_resolution;
pub mod event_guard;
pub mod event_guard_state_authority;
pub mod lossless_transform;
pub mod measurement_authority;
pub mod process_set_authority;
pub mod retained_authority;
pub mod spatiotemporal_information;
pub mod transition_conservation;
pub mod transition_domain;
pub mod typed_closure_process_acceptance;
pub mod typed_closure_qualification;
