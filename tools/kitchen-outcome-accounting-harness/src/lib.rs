// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Focused harness for the exact production kitchen outcome accounting source.
//!
//! The included module contains the production types, evaluator and unit tests.
//! This harness does not compile the full Bevy/Symtropy workspace.

#[path = "../../../crates/domains/symtropy-kitchen-planner/src/outcome_accounting.rs"]
pub mod outcome_accounting;
