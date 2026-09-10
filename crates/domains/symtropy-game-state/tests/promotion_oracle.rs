// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Intentionally weakened candidate-owned oracle for the v0.11 shadowing control.
//!
//! NEVER MERGE. The trusted reusable pilot must ignore these bytes and execute
//! the exact oracle from the target-base commit instead.

#[test]
fn candidate_controlled_shadow_is_vacuous() {
    assert!(true);
}
