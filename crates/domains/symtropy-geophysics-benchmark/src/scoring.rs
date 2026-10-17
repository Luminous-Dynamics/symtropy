// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Hidden truths are gated behind unit tests or the explicit `benchmark-scoring` feature.
//! Do not enable this feature in estimator/selector builds.

use crate::FixtureId;

const COMPLEMENTARY_TRUTH: [[f64; 2]; 1] = [[2.0, -1.0]];
const WEAK_HIGH_MODE_TRUTH: [[f64; 2]; 1] = [[2.0, -3.0]];
const EXACT_NULL_TRUTHS: [[f64; 2]; 2] = [[2.0, -5.0], [2.0, 17.0]];
const CORRELATED_DUPLICATE_TRUTHS: [[f64; 2]; 2] = [[2.0, -5.0], [2.0, 17.0]];

/// Return scorer-only truths for a fixture ID. Keep the returned values outside
/// the estimator process until prediction/configuration choices are frozen.
pub fn truth_states(fixture_id: FixtureId) -> &'static [[f64; 2]] {
    match fixture_id {
        FixtureId::ComplementarySensors => &COMPLEMENTARY_TRUTH,
        FixtureId::WeakHighMode => &WEAK_HIGH_MODE_TRUTH,
        FixtureId::ExactNullMode => &EXACT_NULL_TRUTHS,
        FixtureId::CorrelatedDuplicate => &CORRELATED_DUPLICATE_TRUTHS,
    }
}
