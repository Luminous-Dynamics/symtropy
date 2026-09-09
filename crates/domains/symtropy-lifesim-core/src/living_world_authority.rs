// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Authority/evidence resolvers for Living World canonical transitions.
//!
//! This module is intentionally a stable crate-root surface. Individual
//! authority producers live underneath it so adding another resolver does not
//! repeatedly churn the large `lib.rs` module list.

pub mod lossless_transform;
pub mod transition_domain;
