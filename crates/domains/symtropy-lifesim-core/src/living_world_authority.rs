// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Authority/evidence resolvers for Living World canonical transitions.
//!
//! This module is intentionally a stable crate-root surface. Individual
//! authority producers live underneath it so adding another resolver does not
//! repeatedly churn the large `lib.rs` module list.

pub mod candidate_domain_resolution;
pub mod closure_usage_authority;
pub mod event_guard;
pub mod event_guard_state_authority;
pub mod flux_reconciliation;
pub mod lossless_transform;
pub mod measurement_authority;
pub mod process_set_authority;
pub mod retained_authority;
pub mod shadow_common_start;
pub mod shadow_execution_continuity;
pub mod shadow_execution_lineage;
pub mod shadow_observable_authority;
pub mod shadow_paired_execution;
pub mod shadow_runner_qualification;
pub mod shadow_runner_semantic_provenance;
pub mod shadow_validation;
pub mod spatiotemporal_information;
pub mod transition_conservation;
pub mod transition_domain;
pub mod typed_closure_process_acceptance;
pub mod typed_closure_qualification;
