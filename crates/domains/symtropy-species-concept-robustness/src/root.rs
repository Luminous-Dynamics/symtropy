// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! SEL-10E2 cross-model robustness preregistration, current authority, and report surface.

#[path = "lib.rs"]
mod representation;
pub use representation::*;

mod current;
pub use current::*;

mod matrix;
pub use matrix::*;
