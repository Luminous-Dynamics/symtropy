// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Dependency-light Mk0 scenario model and independent synthetic event verifier.
//!
//! This crate is kept separate from hardware and private monorepo dependencies so
//! the event contract can be tested in the public Symtropy workspace.

#![forbid(unsafe_code)]

pub mod mk0;
pub mod mk0_verification;

pub use mk0::*;
pub use mk0_verification::*;
