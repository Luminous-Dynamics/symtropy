// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Intentionally weakened candidate-owned oracle for the v0.11 expected-red attack.
//!
//! NEVER MERGE. If candidate-controlled tests were authoritative this branch
//! would appear green. The trusted reusable pilot must ignore these bytes and
//! execute the exact target-base oracle instead.

#[test]
fn candidate_controlled_shadow_is_vacuous() {
    assert!(true);
}
