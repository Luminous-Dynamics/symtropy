// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! NEVER MERGE. Intentionally vacuous candidate-owned oracle for the v0.11r2 attack.
//! If candidate tests were authoritative this would hide the injected clock defect.

#[test]
fn candidate_controlled_shadow_is_vacuous() {
    assert!(true);
}
