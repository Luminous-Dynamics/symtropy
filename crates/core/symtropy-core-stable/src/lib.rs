// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Core Spacetime Crystallization Engine.
//! Self-contained HDC-based physics and perception substrate.

pub mod hdc;
pub mod spacetime;

/// Versioned deterministic HDC substrate for new research integrations.
/// The legacy `hdc` module remains unchanged for replay compatibility.
pub use symtropy_hdc_core as deterministic_hdc;

pub use hdc::*;
pub use spacetime::*;
