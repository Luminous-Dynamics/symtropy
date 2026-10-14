// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic fabrication identity and evidence for Symtropy.
//!
//! Physical matter remains owned by an external authority. This crate records
//! intentional fabrication semantics against explicit revisioned evidence.

mod assembly;
mod capability;
mod commissioning;
mod constraints;
mod diagnostic;
mod identity;
mod interface;
mod joint;
mod plan;
mod process;
mod substitution;
mod workmanship;

pub use assembly::*;
pub use capability::*;
pub use commissioning::*;
pub use constraints::*;
pub use diagnostic::*;
pub use identity::*;
pub use interface::*;
pub use joint::*;
pub use plan::*;
pub use process::*;
pub use substitution::*;
pub use workmanship::*;
