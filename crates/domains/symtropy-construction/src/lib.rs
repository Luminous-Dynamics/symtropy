// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Evidence-backed construction orchestration for Symtropy.
//!
//! This crate owns site/work orchestration truth. It does not own conserved
//! matter, structural physics, fabrication process truth, technical
//! commissioning, Device Bus registration, or civic authorization.

mod site;
mod work_order;

pub use site::*;
pub use work_order::*;
