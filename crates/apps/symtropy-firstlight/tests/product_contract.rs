// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use symtropy_crawler_core::{CrawlerProfile, RouteClass};
use symtropy_firstlight::{OpeningPhase, canonical_service_span, run_reference_sequence};
use symtropy_firstlight_bent_feeder::OperationOutcome;
use symtropy_firstlight_catastrophe::CatastrophePhase;

#[test]
fn reference_sequence_crosses_product_boundaries() {
    let session = run_reference_sequence(42).expect("reference sequence");
    assert_eq!(session.world.phase, OpeningPhase::ContinuanceDeparture);
    assert_eq!(
        session.world.bent_feeder.outcome,
        Some(OperationOutcome::DurableRepair)
    );
    assert_eq!(
        session.world.catastrophe.phase,
        CatastrophePhase::AcuteBreaking
    );
    assert_eq!(
        session
            .world
            .crawler
            .qualify_route(&canonical_service_span(42))
            .class,
        RouteClass::Red
    );
    session.events.verify().expect("cross-system event chain");
}

#[test]
fn clv7_reference_values_close_and_remain_human_scale() {
    let profile = CrawlerProfile::clv7_wayhouse();
    profile.validate().expect("reference profile closes");
    assert_eq!(profile.length_mm, 34_800);
    assert_eq!(profile.reference_gross_mass_kg, 320_000);
    assert_eq!(profile.nominal_residents, 24);
    assert_eq!(profile.evacuation_positions, 48);
}

#[test]
fn identical_seed_produces_identical_serialized_world() {
    let first = run_reference_sequence(77).expect("first sequence");
    let second = run_reference_sequence(77).expect("second sequence");
    let first_json = serde_json::to_vec(&first.world).expect("serialize first world");
    let second_json = serde_json::to_vec(&second.world).expect("serialize second world");
    assert_eq!(first_json, second_json);
}
