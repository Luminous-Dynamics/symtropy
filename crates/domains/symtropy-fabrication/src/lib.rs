// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic fabrication identity and evidence for Symtropy.
//!
//! Physical matter remains owned by an external authority. This crate records
//! intentional fabrication semantics against explicit revisioned evidence.

mod identity;
mod interface;

pub use identity::*;
pub use interface::*;
