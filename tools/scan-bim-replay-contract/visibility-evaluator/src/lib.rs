// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Evaluator-only geometry visibility oracle.
//!
//! This crate is a separate process/package from candidate reconstruction.
//! Reference geometry, probe sets, occluder inventories and outputs belong to
//! the evaluator side of the replay boundary.

pub mod visibility;
