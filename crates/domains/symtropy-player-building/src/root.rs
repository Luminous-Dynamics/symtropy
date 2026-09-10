// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! PB-01 crate root while #439 hardening is integrated.

#[path = "lib.rs"]
mod proposal;
pub use proposal::*;

pub mod hardening_contract;
