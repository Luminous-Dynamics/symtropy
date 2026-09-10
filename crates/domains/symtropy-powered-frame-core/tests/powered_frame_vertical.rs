// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use symtropy_game_state::{EventChain, StableId};
use symtropy_powered_frame_core::{
    PoweredSubsystem, ResourceArbiter, ResourceMode, ResourceRequest,
    experience::{ExperienceProjector, ExperienceSample},
};
use symtropy_protection_core::{
    Effect, EffectKind, LayerResponse, ProtectionLayer, ProtectionStack,
    active_field::{ActiveField, ActiveFieldConfig},
};
use symtropy_residents::condition::{
    CapabilityKind, CapabilityProfile, Condition, ConditionSet,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct ProofEvent {
    summary: String,
}

#[derive(Debug, Clone, PartialEq)]
struct ProofReport {
    field_absorbed: f64,
    armor_absorbed: f64,
    residual_harm: f64,
    locomotion_before_treatment: u16,
    mobility_grant: f64,
    protection_grant: f64,
    field_charge_after_service: f64,
    condition_severity_after_service: u16,
    locomotion_after_treatment: u16,
    muse_arousal: f64,
    muse_valence: f64,
    event_count: usize,
}

fn sid(value: &str) -> StableId {
    StableId::parse(value).expect("proof stable id must be valid")
}

fn record(
    chain: &mut EventChain<ProofEvent>,
    tick: u64,
    kind: &str,
    parents: Vec<StableId>,
    summary: &str,
) -> StableId {
    chain
        .append(
            tick,
            kind,
            Some(sid("resident:arin")),
            Some(sid("system:powered-frame-proof")),
            parents,
            ProofEvent {
                summary: summary.into(),
            },
        )
        .expect("proof event must append")
}

fn run_proof() -> ProofReport {
    let mut history = EventChain::new("powered-frame-proof", 0xE011);

    let incoming = Effect {
        magnitude: 80.0,
        kind: EffectKind::Impact,
    };
    let impact_event = record(
        &mut history,
        10,
        "impact.received",
        vec![],
        "an external impact reaches the powered frame",
    );

    // Fictional active field: deliberately too little charge to absorb the whole hit.
    let mut field = ActiveField::new(ActiveFieldConfig {
        max_charge: 20.0,
        ..ActiveFieldConfig::default()
    })
    .expect("field config is valid");
    let field_resolution = field.intercept(incoming).expect("field accepts valid effect");
    assert_eq!(field_resolution.absorbed, 20.0);
    assert_eq!(field_resolution.residual.magnitude, 60.0);
    let field_event = record(
        &mut history,
        11,
        "protection.field_intercepted",
        vec![impact_event.clone()],
        "field charge is exhausted and residual impact passes inward",
    );

    // Passive armor consumes its own finite capacity after the field.
    let armor_response = BTreeMap::from([(
        EffectKind::Impact,
        LayerResponse {
            absorption_fraction: 0.5,
            capacity: 20.0,
        },
    )]);
    let mut armor = ProtectionStack {
        layers: vec![
            ProtectionLayer::new("torso-frame-armor", 1.0, armor_response)
                .expect("armor layer is valid"),
        ],
    };
    let armor_resolution = armor
        .resolve(field_resolution.residual)
        .expect("armor resolves valid residual");
    assert_eq!(armor_resolution.layers[0].absorbed, 20.0);
    assert_eq!(armor_resolution.residual.magnitude, 40.0);
    let armor_event = record(
        &mut history,
        12,
        "protection.armor_transmitted",
        vec![field_event],
        "armor absorbs part of the residual and transmits the remainder",
    );

    // The world/health adapter turns residual harm into a persistent condition.
    // This proof deliberately uses a simple declared severity mapping; the generic
    // protection core itself never creates medical conditions.
    let severity = ((armor_resolution.residual.magnitude / incoming.magnitude) * 10_000.0)
        .round()
        .clamp(0.0, 10_000.0) as u16;
    assert_eq!(severity, 5_000);

    let condition_id = sid("condition:arin-left-leg-impact");
    let mut conditions = ConditionSet::default();
    conditions.insert(
        Condition::new(
            condition_id.clone(),
            "left-leg-impact",
            severity,
            13,
            Some(armor_event.clone()),
            BTreeMap::from([
                (CapabilityKind::Locomotion, 8_000),
                (CapabilityKind::Stability, 5_000),
            ]),
        )
        .expect("condition is valid"),
    );
    let condition_event = record(
        &mut history,
        13,
        "health.condition_recorded",
        vec![armor_event],
        "residual effect is recorded as a persistent wearer condition",
    );

    let mut profile = CapabilityProfile::default();
    profile
        .set(CapabilityKind::Locomotion, 10_000)
        .expect("baseline is valid");
    profile
        .set(CapabilityKind::Stability, 9_000)
        .expect("baseline is valid");
    let impaired = profile.assess(CapabilityKind::Locomotion, &conditions);
    assert_eq!(impaired.effective, 6_000);
    assert_eq!(impaired.deficit, 4_000);
    let capability_event = record(
        &mut history,
        14,
        "capability.locomotion_reassessed",
        vec![condition_event],
        "leg condition reduces natural locomotion capability",
    );

    // A finite shared resource budget must now cover critical loads, cooling,
    // mobility compensation, sensing, and field recovery at the same time.
    let mobility_requested = 3.0 + f64::from(impaired.deficit) / 1_000.0;
    assert_eq!(mobility_requested, 7.0);
    let receipt = ResourceArbiter::new(ResourceMode::MobilityPriority)
        .allocate(
            12.0,
            &[
                ResourceRequest {
                    subsystem: PoweredSubsystem::LifeSupport,
                    requested: 1.0,
                    minimum: 1.0,
                },
                ResourceRequest {
                    subsystem: PoweredSubsystem::Cooling,
                    requested: 3.0,
                    minimum: 1.0,
                },
                ResourceRequest {
                    subsystem: PoweredSubsystem::Mobility,
                    requested: mobility_requested,
                    minimum: 2.0,
                },
                ResourceRequest {
                    subsystem: PoweredSubsystem::Sensors,
                    requested: 2.0,
                    minimum: 0.0,
                },
                ResourceRequest {
                    subsystem: PoweredSubsystem::Protection,
                    requested: 8.0,
                    minimum: 0.0,
                },
            ],
        )
        .expect("valid resource request set must arbitrate");
    assert!(receipt.allocated <= 12.0 + 1e-12);
    let mobility_grant = receipt
        .grant(PoweredSubsystem::Mobility)
        .expect("mobility grant exists")
        .granted;
    let protection_grant = receipt
        .grant(PoweredSubsystem::Protection)
        .expect("protection grant exists")
        .granted;
    assert!(mobility_grant > protection_grant);
    let allocation_event = record(
        &mut history,
        15,
        "exoframe.resources_allocated",
        vec![capability_event],
        "finite budget prioritizes mobility assistance while preserving critical minima",
    );

    let severity_before_service = conditions.get(&condition_id).unwrap().severity;
    let field_service = field
        .service(protection_grant, 1.0)
        .expect("granted protection resource can service field");
    assert!(field_service.accepted_resource <= protection_grant + 1e-12);
    assert!(field_service.charge_after > 0.0);
    // Recharging equipment must not silently heal the wearer.
    assert_eq!(
        conditions.get(&condition_id).unwrap().severity,
        severity_before_service
    );
    let service_event = record(
        &mut history,
        16,
        "protection.field_serviced",
        vec![allocation_event],
        "only the explicitly granted protection budget reaches field recharge",
    );

    // Presentation observes the resulting state. It does not own or mutate any of it.
    let total_requested = 1.0 + 3.0 + mobility_requested + 2.0 + 8.0;
    let resource_pressure = (1.0 - receipt.allocated / total_requested).clamp(0.0, 1.0);
    let mobility_fraction = receipt
        .grant(PoweredSubsystem::Mobility)
        .unwrap()
        .request_fraction();
    let mut experience = ExperienceProjector::default();
    let narrative = experience
        .update(ExperienceSample {
            threat: 0.85,
            uncertainty: 0.35,
            agency: mobility_fraction,
            physiological_strain: f64::from(severity) / 10_000.0,
            resource_pressure,
            protection_instability: 1.0 - field.charge_fraction(),
            memory_resonance: 0.20,
            social_connection: 0.65,
            loss: 0.05,
            causal_significance: 0.80,
        })
        .expect("bounded experience sample is valid");
    let muse = narrative.muse_projection();
    assert!(muse.arousal > 0.0);
    assert!((-1.0..=1.0).contains(&muse.valence));
    let experience_event = record(
        &mut history,
        17,
        "experience.projected",
        vec![service_event],
        "presentation receives a read-only semantic projection of the consequences",
    );

    // Recovery is not granted by the field, arbiter, or experience layer. A separate
    // causal treatment/recovery event must revise the persistent condition.
    let treatment_event = record(
        &mut history,
        30,
        "health.treatment_recorded",
        vec![experience_event],
        "a separate treatment event establishes authority to revise condition severity",
    );
    conditions
        .revise_severity(&condition_id, 2_000, 30, treatment_event.clone())
        .expect("causally-provenanced treatment may revise condition");
    let recovered = profile.assess(CapabilityKind::Locomotion, &conditions);
    assert!(recovered.effective > impaired.effective);
    assert_eq!(conditions.get(&condition_id).unwrap().severity, 2_000);
    record(
        &mut history,
        31,
        "capability.locomotion_reassessed",
        vec![treatment_event],
        "reduced condition severity yields improved natural locomotion capability",
    );

    // On the current main lineage EventChain verifies deterministic IDs, hashes,
    // prior-hash continuity, and monotonic ticks. Causal-parent graph validation is a
    // separate civilization tranche and is deliberately not duplicated here.
    history
        .verify()
        .expect("chronological hash chain must verify");

    ProofReport {
        field_absorbed: field_resolution.absorbed,
        armor_absorbed: armor_resolution.layers[0].absorbed,
        residual_harm: armor_resolution.residual.magnitude,
        locomotion_before_treatment: impaired.effective,
        mobility_grant,
        protection_grant,
        field_charge_after_service: field_service.charge_after,
        condition_severity_after_service: severity_before_service,
        locomotion_after_treatment: recovered.effective,
        muse_arousal: muse.arousal,
        muse_valence: muse.valence,
        event_count: history.events().len(),
    }
}

#[test]
fn powered_frame_vertical_story_preserves_authority_boundaries() {
    let report = run_proof();
    assert_eq!(report.field_absorbed, 20.0);
    assert_eq!(report.armor_absorbed, 20.0);
    assert_eq!(report.residual_harm, 40.0);
    assert_eq!(report.locomotion_before_treatment, 6_000);
    assert_eq!(report.condition_severity_after_service, 5_000);
    assert!(report.field_charge_after_service > 0.0);
    assert!(report.mobility_grant > report.protection_grant);
    assert!(report.locomotion_after_treatment > report.locomotion_before_treatment);
    assert!(report.muse_arousal > 0.0);
    assert!((-1.0..=1.0).contains(&report.muse_valence));
    assert_eq!(report.event_count, 10);
}

#[test]
fn powered_frame_vertical_story_replays_deterministically() {
    assert_eq!(run_proof(), run_proof());
}
