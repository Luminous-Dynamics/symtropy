// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Compatibility re-export of the renderer-independent motion contract.
//!
//! The actual API and its validation live in the lightweight `symtropy-motion-contract`
//! crate so cognition/planning clients do not need to import the Bevy integration.

pub use symtropy_motion_contract::*;
