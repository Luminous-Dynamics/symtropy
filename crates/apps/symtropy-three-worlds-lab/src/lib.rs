// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! LAB-14: one deterministic headless history across Aster, Vesper, and Helion.
//!
//! The point of this lab is not content volume. It is to prove that the
//! civilization primitives can disagree, propagate with delay, conserve state,
//! and compose without one subsystem silently claiming another subsystem's
//! authority.

use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use symtropy_assets_core::{AssetLedger, AssetRelation, AssetRelationKind};
use symtropy_civilization_core::{
    AuthorityCapability, AuthorityGrant, AuthorityPrincipal, AuthorityScope, Institution,
    OfficeDefinition, OfficeHolderRecord,
};
use symtropy_civilization_scale_core::{
    CivilizationProjection, CohortDescriptor, PopulationCohort, ProjectionOperation,
    ProjectionRequest, ProjectionRevision, ScaleAuthorityRef,
};
use symtropy_comms_core::{
    CommunicationChannel, CommunicationPayloadRef, CommunicationWorld, MessageDelivery,
    MessageEnvelope, MessageTransmission, SignalPropagationPlan,
};
use symtropy_conflict_core::{
    CeasefireAcceptance, CeasefireProposal, ConflictAim, ConflictAimKind, ConflictEvidenceRef,
    ConflictParticipant, ConflictSpec, ConflictState, ConflictWorld, SettlementAcceptance,
    SettlementProposal, SettlementTerm, SettlementTermKind,
};
use symtropy_diplomacy_core::{
    ClauseAssessment, ClausePosition, DiplomacyWorld, DiplomaticEvidenceRef, DiplomaticScope,
    TreatyClause, TreatyClauseKind, TreatyRatification, TreatySpec, TreatyState,
};
use symtropy_economy_core::{
    EconomyWorld, GenesisBalance, InventoryNode, ResourceKey, ShipmentArrival, ShipmentDeparture,
};
use symtropy_game_state::StableId;
use symtropy_player_org_core::{
    AdmissionEffectKind, AdmissionPolicy, AdmissionRequest, AdmissionReviewer, AdmissionRule,
    AdmissionTarget, CollaborationProvider, ExternalOrganizationRecord, ExternalOrganizationRef,
    PlayerOrganizationAdmission,
};
use symtropy_projects_core::{
    Contribution, ContributionReview, DeliverableDisposition, DeliverableSpec, ProjectEvidenceRef,
    ProjectLedger, ProjectSpec,
};
use symtropy_recognition_core::{
    RecognitionLedger, RecognitionPosition, RecognitionRecord, RecognitionSubject,
};
use symtropy_succession_core::{
    ClaimGrounds, OfficeClaim, SelectionMechanism, SuccessionLedger, SuccessionPolicy,
};
use symtropy_transfer_core::{
    AuthorityEndpoint, DestinationAcceptance, SubjectAuthorityRecord, SubjectAuthorityRevision,
    TransferAuthority, TransferRequest, TransferStateRef, TransferSubject, TransferSubjectKind,
};
use symtropy_transit_core::{
    TopologyPathRef, TransitBinding, TransitLeg, TransitMode, TransitPlan,
};

pub type LabResult<T> = Result<T, String>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ThreeWorldsReport {
    pub institutional_holder_survives_competing_claims: bool,
    pub simultaneous_succession_claims: bool,
    pub recognition_is_source_relative: bool,
    pub signal_unavailable_before_delivery: bool,
    pub signal_delivery_tick: u64,
    pub freight_unavailable_before_arrival: bool,
    pub freight_arrival_tick: u64,
    pub signal_arrives_before_freight: bool,
    pub treaty_active: bool,
    pub treaty_assessments_disagree: bool,
    pub external_org_admission_does_not_publish_project: bool,
    pub project_complete_after_explicit_review: bool,
    pub conflict_active_before_ceasefire: bool,
    pub conflict_in_ceasefire_window: bool,
    pub conflict_settled_by_acceptance: bool,
    pub settlement_term_count: usize,
    pub anonymous_population_before: u64,
    pub represented_population_after_materialization: u64,
    pub materialized_identity_begins_tick: u64,
    pub authority_before_transfer_commit: String,
    pub authority_after_transfer_commit: String,
    pub authority_transfer_retry_idempotent: bool,
    pub asset_owner_remains_aster_after_shard_transfer: bool,
    pub external_admission_does_not_grant_institutional_authority: bool,
}

fn id(value: &str) -> StableId {
    StableId::parse(value).expect("LAB-14 uses only portable authored ids")
}

fn transit_leg(
    leg_id: &str,
    path_id: &str,
    origin: &str,
    destination: &str,
    depart_not_before: u64,
    depart_not_after: u64,
    earliest_arrival: u64,
    freighter: &StableId,
) -> TransitLeg {
    TransitLeg {
        id: id(leg_id),
        path: TopologyPathRef {
            provider_id: id("provider:orbital-lab"),
            path_id: id(path_id),
            origin_location_id: id(origin),
            destination_location_id: id(destination),
        },
        mode: TransitMode {
            namespace: "orbital".into(),
            method: "provider-attested-lab-transfer".into(),
        },
        depart_not_before_tick: depart_not_before,
        depart_not_after_tick: depart_not_after,
        earliest_arrival_tick: earliest_arrival,
        latest_arrival_tick: Some(earliest_arrival + 20),
        carrier_asset_id: Some(freighter.clone()),
        provider_record_id: id(&format!("provider-record:{leg_id}")),
        source_event_id: id(&format!("event:{leg_id}")),
    }
}

pub fn run_three_worlds() -> LabResult<ThreeWorldsReport> {
    let aster = id("institution:aster-council");
    let vesper = id("institution:vesper-cooperative");
    let helion = id("institution:helion-assembly");
    let aster_loc = id("loc:aster");
    let hub_loc = id("loc:hub");
    let vesper_loc = id("loc:vesper");
    let helion_loc = id("loc:helion");
    let first_speaker = id("office:aster-first-speaker");
    let mara = id("resident:mara");
    let mina = id("resident:mina");
    let cael = id("resident:cael");
    let lio = id("resident:lio");
    let freighter = id("asset:relief-freighter-7");

    // ---------------------------------------------------------------------
    // 1. Institution -> succession -> source-relative recognition.
    // ---------------------------------------------------------------------
    let mut aster_institution = Institution::new(aster.clone(), "Aster Council");
    aster_institution.tags.insert("chartered-council".into());
    aster_institution
        .define_office(OfficeDefinition {
            id: first_speaker.clone(),
            name: "First Speaker".into(),
            tags: BTreeSet::from(["executive".into(), "treaty-signatory".into()]),
        })
        .map_err(|e| e.to_string())?;
    aster_institution
        .record_office_holder(OfficeHolderRecord {
            office_id: first_speaker.clone(),
            holder_id: mara.clone(),
            recorded_tick: 10,
            source_event_id: id("event:aster-records-mara"),
        })
        .map_err(|e| e.to_string())?;
    aster_institution
        .grant_authority(AuthorityGrant {
            id: id("grant:first-speaker-treaty-sign"),
            principal: AuthorityPrincipal::Office(first_speaker.clone()),
            capability: AuthorityCapability {
                namespace: "treaty".into(),
                operation: "sign".into(),
            },
            scope: AuthorityScope {
                jurisdiction_id: Some(aster.clone()),
                target_id: None,
            },
            valid_from_tick: 10,
            valid_until_tick: None,
            source_event_id: id("event:grant-first-speaker-treaty-sign"),
        })
        .map_err(|e| e.to_string())?;

    let authority_grants_before_external_admission = aster_institution.authority_grants().count();

    let succession_policy_id = id("policy:aster-speaker-succession");
    let mina_claim_id = id("claim:mina-first-speaker");
    let cael_claim_id = id("claim:cael-first-speaker");
    let mut succession = SuccessionLedger::new();
    succession
        .register_policy(
            &aster_institution,
            SuccessionPolicy {
                id: succession_policy_id.clone(),
                institution_id: aster.clone(),
                office_id: first_speaker.clone(),
                mechanism: SelectionMechanism {
                    namespace: "succession".into(),
                    method: "council-continuity-v1".into(),
                },
                valid_from_tick: 1,
                valid_until_tick: None,
                source_event_id: id("event:adopt-aster-succession"),
            },
        )
        .map_err(|e| e.to_string())?;
    succession
        .submit_claim(
            &aster_institution,
            OfficeClaim {
                id: mina_claim_id.clone(),
                institution_id: aster.clone(),
                office_id: first_speaker.clone(),
                claimant_id: mina.clone(),
                grounds: ClaimGrounds::Policy(succession_policy_id),
                asserted_tick: 50,
                valid_until_tick: None,
                epistemic_basis: BTreeSet::new(),
                source_event_id: id("event:mina-claim"),
            },
        )
        .map_err(|e| e.to_string())?;
    succession
        .submit_claim(
            &aster_institution,
            OfficeClaim {
                id: cael_claim_id.clone(),
                institution_id: aster.clone(),
                office_id: first_speaker.clone(),
                claimant_id: cael,
                grounds: ClaimGrounds::ExtraInstitutional {
                    namespace: "customary".into(),
                    basis: "frontier-assembly-acclamation".into(),
                },
                asserted_tick: 51,
                valid_until_tick: None,
                epistemic_basis: BTreeSet::new(),
                source_event_id: id("event:cael-claim"),
            },
        )
        .map_err(|e| e.to_string())?;

    let simultaneous_succession_claims = succession.claim_is_active(&mina_claim_id, 60)
        && succession.claim_is_active(&cael_claim_id, 60);
    let institutional_holder_survives_competing_claims = aster_institution
        .office_holders()
        .any(|record| record.office_id == first_speaker && record.holder_id == mara);

    let mina_subject = RecognitionSubject::OfficeClaim(mina_claim_id.clone());
    let mut recognition = RecognitionLedger::new();
    recognition
        .record(
            &succession,
            RecognitionRecord {
                id: id("recognition:vesper-mina"),
                recognizer_id: vesper.clone(),
                subject: mina_subject.clone(),
                position: RecognitionPosition::Recognizes,
                recorded_tick: 60,
                valid_until_tick: None,
                epistemic_basis: BTreeSet::new(),
                source_event_id: id("event:vesper-recognizes-mina"),
            },
        )
        .map_err(|e| e.to_string())?;
    recognition
        .record(
            &succession,
            RecognitionRecord {
                id: id("recognition:helion-mina"),
                recognizer_id: helion.clone(),
                subject: mina_subject.clone(),
                position: RecognitionPosition::Disputes,
                recorded_tick: 61,
                valid_until_tick: None,
                epistemic_basis: BTreeSet::new(),
                source_event_id: id("event:helion-disputes-mina"),
            },
        )
        .map_err(|e| e.to_string())?;
    let recognition_positions = recognition
        .effective_positions_for_subject(&mina_subject, 70)
        .map(|record| record.position)
        .collect::<BTreeSet<_>>();
    let recognition_is_source_relative = recognition_positions
        == BTreeSet::from([
            RecognitionPosition::Recognizes,
            RecognitionPosition::Disputes,
        ]);

    // ---------------------------------------------------------------------
    // 2. Ownership != operation != simulation authority.
    // ---------------------------------------------------------------------
    let mut assets = AssetLedger::new();
    assets
        .record_relation(AssetRelation {
            id: id("relation:freighter-owner-aster"),
            asset_id: freighter.clone(),
            party_id: aster.clone(),
            kind: AssetRelationKind::Owner,
            location_id: Some(aster_loc.clone()),
            interest_ppm: None,
            valid_from_tick: 1,
            valid_until_tick: None,
            epistemic_basis: BTreeSet::new(),
            source_event_id: id("event:aster-owns-freighter"),
        })
        .map_err(|e| e.to_string())?;
    assets
        .record_relation(AssetRelation {
            id: id("relation:freighter-operator-vesper"),
            asset_id: freighter.clone(),
            party_id: vesper.clone(),
            kind: AssetRelationKind::Operator,
            location_id: Some(aster_loc.clone()),
            interest_ppm: None,
            valid_from_tick: 1,
            valid_until_tick: None,
            epistemic_basis: BTreeSet::new(),
            source_event_id: id("event:vesper-operates-freighter"),
        })
        .map_err(|e| e.to_string())?;

    // ---------------------------------------------------------------------
    // 3. Shared topology; messages move much faster than physical cargo.
    // ---------------------------------------------------------------------
    let food = ResourceKey {
        resource_id: id("resource:relief-food"),
        unit_id: id("unit:crate"),
    };
    let aster_node = id("inventory:aster-relief-depot");
    let vesper_node = id("inventory:vesper-relief-depot");
    let mut economy = EconomyWorld::from_genesis(
        [
            InventoryNode {
                id: aster_node.clone(),
                location_id: aster_loc.clone(),
                operator_id: Some(aster.clone()),
            },
            InventoryNode {
                id: vesper_node.clone(),
                location_id: vesper_loc.clone(),
                operator_id: Some(vesper.clone()),
            },
        ],
        [GenesisBalance {
            node_id: aster_node.clone(),
            resource: food.clone(),
            amount: 1_000,
        }],
    )
    .map_err(|e| e.to_string())?;

    let transit_plan = TransitPlan {
        id: id("plan:aster-hub-vesper-relief"),
        provider_id: id("provider:orbital-lab"),
        legs: vec![
            transit_leg(
                "leg:aster-hub",
                "path:aster-hub",
                "loc:aster",
                "loc:hub",
                100,
                110,
                180,
                &freighter,
            ),
            transit_leg(
                "leg:hub-vesper",
                "path:hub-vesper",
                "loc:hub",
                "loc:vesper",
                180,
                190,
                260,
                &freighter,
            ),
        ],
        source_event_id: id("event:provider-admits-relief-plan"),
    };
    transit_plan.validate().map_err(|e| e.to_string())?;
    let transit_binding = TransitBinding::from_plan(
        &transit_plan,
        &aster_loc,
        &vesper_loc,
        105,
        Some(&freighter),
    )
    .map_err(|e| e.to_string())?;

    let shipment_id = id("shipment:aster-vesper-relief-1");
    let arrival_id = id("arrival:aster-vesper-relief-1");
    economy
        .depart_shipment(ShipmentDeparture {
            id: shipment_id.clone(),
            source_node_id: aster_node.clone(),
            destination_node_id: vesper_node.clone(),
            resource: food.clone(),
            amount: 100,
            departed_tick: transit_binding.departed_tick,
            earliest_arrival_tick: transit_binding.earliest_arrival_tick,
            carrier_asset_id: transit_binding.carrier_asset_id.clone(),
            source_event_id: id("event:relief-departs"),
        })
        .map_err(|e| e.to_string())?;

    let topology_paths = transit_plan.topology_paths().cloned().collect::<Vec<_>>();
    let mut comms = CommunicationWorld::default();
    let signal_plan_id = id("signal-plan:aster-vesper");
    comms
        .register_plan(SignalPropagationPlan {
            id: signal_plan_id.clone(),
            provider_id: id("provider:relay-lab"),
            topology_paths,
            channel: CommunicationChannel {
                namespace: "laser-relay".into(),
                method: "provider-attested".into(),
            },
            send_not_before_tick: 105,
            send_not_after_tick: 115,
            earliest_delivery_tick: 130,
            latest_delivery_tick: Some(135),
            provider_record_id: id("provider-record:signal-aster-vesper"),
            source_event_id: id("event:provider-admits-signal-plan"),
        })
        .map_err(|e| e.to_string())?;

    let treaty_id = id("treaty:aster-vesper-relief");
    let message_id = id("message:aster-relief-proposal");
    let transmission_id = id("transmission:aster-relief-proposal");
    comms
        .author_message(MessageEnvelope {
            id: message_id.clone(),
            sender_id: aster.clone(),
            origin_location_id: aster_loc.clone(),
            destination_location_id: vesper_loc.clone(),
            authored_tick: 106,
            payload_refs: vec![CommunicationPayloadRef {
                namespace: "treaty-proposal".into(),
                record_id: treaty_id.clone(),
            }],
            source_event_id: id("event:author-relief-proposal"),
        })
        .map_err(|e| e.to_string())?;
    comms
        .transmit(MessageTransmission {
            id: transmission_id.clone(),
            message_id: message_id.clone(),
            propagation_plan_id: signal_plan_id,
            sent_tick: 107,
            source_event_id: id("event:send-relief-proposal"),
        })
        .map_err(|e| e.to_string())?;
    let signal_unavailable_before_delivery = comms.delivered_message(&message_id).is_none()
        && comms
            .deliver(MessageDelivery {
                id: id("delivery:too-early"),
                transmission_id: transmission_id.clone(),
                delivered_tick: 129,
                receiver_id: Some(vesper.clone()),
                source_event_id: id("event:impossible-early-delivery"),
            })
            .is_err()
        && comms.delivered_message(&message_id).is_none();
    comms
        .deliver(MessageDelivery {
            id: id("delivery:aster-relief-proposal"),
            transmission_id,
            delivered_tick: 130,
            receiver_id: Some(vesper.clone()),
            source_event_id: id("event:deliver-relief-proposal"),
        })
        .map_err(|e| e.to_string())?;

    // ---------------------------------------------------------------------
    // 4. Treaty agreement is clause-level and later evidence can disagree.
    // ---------------------------------------------------------------------
    let relief_clause_id = id("clause:deliver-relief-food"),
    mut diplomacy = DiplomacyWorld::default();
    diplomacy
        .propose_treaty(TreatySpec {
            id: treaty_id.clone(),
            title: "Aster-Vesper Relief Compact".into(),
            parties: BTreeSet::from([aster.clone(), vesper.clone()]),
            required_ratifiers: BTreeSet::from([aster.clone(), vesper.clone()]),
            clauses: vec![TreatyClause {
                id: relief_clause_id.clone(),
                kind: TreatyClauseKind::PerformanceObligation {
                    obligor_id: aster.clone(),
                    beneficiary_id: vesper.clone(),
                    performance_namespace: "economy".into(),
                    performance_operation: "deliver".into(),
                    subject_id: Some(food.resource_id.clone()),
                    quantity: Some(100),
                    unit_id: Some(food.unit_id.clone()),
                    due_tick: Some(270),
                },
                valid_from_tick: 140,
                valid_until_tick: None,
                source_event_id: id("event:relief-clause"),
            }],
            proposed_tick: 140,
            source_event_id: id("event:propose-relief-treaty"),
        })
        .map_err(|e| e.to_string())?;
    for (ratification_id, ratifier, tick) in [
        ("ratification:aster-relief", aster.clone(), 145),
        ("ratification:vesper-relief", vesper.clone(), 150),
    ] {
        diplomacy
            .ratify(TreatyRatification {
                id: id(ratification_id),
                treaty_id: treaty_id.clone(),
                ratifier_id: ratifier,
                ratified_tick: tick,
                source_event_id: id(&format!("event:{ratification_id}")),
            })
            .map_err(|e| e.to_string())?;
    }
    let treaty_active = diplomacy.treaty_state(&treaty_id) == Some(TreatyState::Active);

    // ---------------------------------------------------------------------
    // 5. Distant population remains anonymous until one identity is materialized.
    // ---------------------------------------------------------------------
    let initial_scale_authority = ScaleAuthorityRef {
        authority_namespace: "living-world-transition".into(),
        authority_record_id: id("scale-authority:helion:190"),
        source_snapshot_id: id("snapshot:helion:180"),
        destination_snapshot_id: id("snapshot:helion:190"),
    };
    let cohort_id = id("cohort:helion-background-population"),
    let mut helion_projection = CivilizationProjection::new(
        id("projection:helion"),
        helion_loc.clone(),
        190,
        [PopulationCohort {
            id: cohort_id.clone(),
            descriptor: CohortDescriptor {
                namespace: "helion-residents".into(),
                dimensions: BTreeMap::from([("settlement".into(), "capital-basin".into())]),
            },
            anonymous_count: 1_000_000,
            authority_ref: initial_scale_authority,
        }],
        [],
    )
    .map_err(|e| e.to_string())?;
    let anonymous_population_before = helion_projection.represented_population().map_err(|e| e.to_string())?;
    helion_projection
        .apply(
            ProjectionRevision(0),
            ProjectionRequest {
                id: id("projection-request:materialize-lio"),
                canonical_tick: 200,
                authority_ref: ScaleAuthorityRef {
                    authority_namespace: "living-world-transition".into(),
                    authority_record_id: id("scale-authority:helion:200"),
                    source_snapshot_id: id("snapshot:helion:190"),
                    destination_snapshot_id: id("snapshot:helion:200"),
                },
                operation: ProjectionOperation::MaterializePersistentIdentity {
                    cohort_id,
                    actor_id: lio.clone(),
                    source_event_id: id("event:materialize-lio"),
                },
            },
        )
        .map_err(|e| e.to_string())?;
    let represented_population_after_materialization = helion_projection
        .represented_population()
        .map_err(|e| e.to_string())?;
    let materialized_identity_begins_tick = helion_projection
        .persistent_actor(&lio)
        .ok_or_else(|| "Lio was not materialized".to_string())?
        .canonical_since_tick;

    // ---------------------------------------------------------------------
    // 6. Cargo cannot appear early; arrival then becomes evidence, not truth for every layer.
    // ---------------------------------------------------------------------
    let freight_unavailable_before_arrival = economy.balance(&vesper_node, &food) == 0
        && economy
            .arrive_shipment(ShipmentArrival {
                id: id("arrival:too-early"),
                shipment_id: shipment_id.clone(),
                arrived_tick: 259,
                source_event_id: id("event:impossible-early-arrival"),
            })
            .is_err()
        && economy.balance(&vesper_node, &food) == 0;
    economy
        .arrive_shipment(ShipmentArrival {
            id: arrival_id.clone(),
            shipment_id: shipment_id.clone(),
            arrived_tick: 260,
            source_event_id: id("event:relief-arrives"),
        })
        .map_err(|e| e.to_string())?;

    diplomacy
        .assess_clause(ClauseAssessment {
            id: id("assessment:aster-relief-satisfied"),
            treaty_id: treaty_id.clone(),
            clause_id: relief_clause_id.clone(),
            assessor_id: aster.clone(),
            position: ClausePosition::Satisfied,
            assessed_tick: 265,
            evidence: vec![DiplomaticEvidenceRef {
                namespace: "economy.shipment-arrival".into(),
                record_id: arrival_id.clone(),
            }],
            source_event_id: id("event:aster-assesses-relief"),
        })
        .map_err(|e| e.to_string())?;
    diplomacy
        .assess_clause(ClauseAssessment {
            id: id("assessment:vesper-relief-disputed"),
            treaty_id: treaty_id.clone(),
            clause_id: relief_clause_id.clone(),
            assessor_id: vesper.clone(),
            position: ClausePosition::Disputed,
            assessed_tick: 266,
            evidence: vec![DiplomaticEvidenceRef {
                namespace: "inspection.spoilage".into(),
                record_id: id("inspection:vesper-relief-spoilage"),
            }],
            source_event_id: id("event:vesper-disputes-relief"),
        })
        .map_err(|e| e.to_string())?;
    let treaty_positions = diplomacy
        .assessments_for_clause(&treaty_id, &relief_clause_id)
        .map(|assessment| assessment.position)
        .collect::<BTreeSet<_>>();
    let treaty_assessments_disagree = treaty_positions
        == BTreeSet::from([ClausePosition::Satisfied, ClausePosition::Disputed]);

    // ---------------------------------------------------------------------
    // 7. Simulation/shard authority moves exactly once and does not rewrite ownership.
    // ---------------------------------------------------------------------
    let aster_shard = AuthorityEndpoint {
        id: id("authority:aster-shard"),
    };
    let vesper_shard = AuthorityEndpoint {
        id: id("authority:vesper-shard"),
    };
    let transfer_subject = TransferSubject {
        kind: TransferSubjectKind::Asset,
        id: freighter.clone(),
    };
    let transfer_state = TransferStateRef {
        namespace: "world-snapshot".into(),
        record_id: id("state:freighter:arrival-260"),
    };
    let mut transfer = TransferAuthority::default();
    transfer
        .register_subject(SubjectAuthorityRecord {
            subject: transfer_subject.clone(),
            authority: aster_shard.clone(),
            revision: SubjectAuthorityRevision(0),
            state_ref: transfer_state.clone(),
            canonical_tick: 260,
            source_event_id: id("event:aster-shard-owns-freighter-sim"),
        })
        .map_err(|e| e.to_string())?;
    let transfer_request_id = id("transfer:freighter-aster-vesper"),
    transfer
        .prepare(TransferRequest {
            id: transfer_request_id.clone(),
            subject: transfer_subject.clone(),
            source_authority: aster_shard.clone(),
            destination_authority: vesper_shard.clone(),
            expected_subject_revision: SubjectAuthorityRevision(0),
            source_state_ref: transfer_state.clone(),
            prepared_tick: 260,
            source_event_id: id("event:prepare-freighter-transfer"),
        })
        .map_err(|e| e.to_string())?;
    transfer
        .accept(DestinationAcceptance {
            id: id("acceptance:freighter-vesper"),
            transfer_request_id: transfer_request_id.clone(),
            destination_authority: vesper_shard.clone(),
            accepted_state_ref: transfer_state,
            accepted_tick: 261,
            source_event_id: id("event:vesper-accepts-freighter-state"),
        })
        .map_err(|e| e.to_string())?;
    let authority_before_transfer_commit = transfer
        .authority_for(&transfer_subject)
        .ok_or_else(|| "freighter authority disappeared before commit".to_string())?
        .authority
        .id
        .as_str()
        .to_string();
    let first_transfer_receipt = transfer
        .commit(
            &transfer_request_id,
            262,
            id("event:commit-freighter-transfer"),
        )
        .map_err(|e| e.to_string())?;
    let retry_transfer_receipt = transfer
        .commit(
            &transfer_request_id,
            999,
            id("event:retry-after-ack-loss"),
        )
        .map_err(|e| e.to_string())?;
    let authority_after_transfer_commit = transfer
        .authority_for(&transfer_subject)
        .ok_or_else(|| "freighter authority disappeared after commit".to_string())?
        .authority
        .id
        .as_str()
        .to_string();
    let authority_transfer_retry_idempotent = first_transfer_receipt == retry_transfer_receipt
        && first_transfer_receipt.resulting_revision == SubjectAuthorityRevision(1);

    // ---------------------------------------------------------------------
    // 8. External real-player governance may request a project; admission does not publish it.
    // ---------------------------------------------------------------------
    let project_id = id("project:vesper-relief-audit"),
    provider_id = id("provider:mycelix"),
    admission_rule_id = id("rule:mycelix-project-publication");
    let policy = AdmissionPolicy::new(
        id("policy:three-worlds-player-org"),
        1,
        [AdmissionRule {
            id: admission_rule_id.clone(),
            provider_id: provider_id.clone(),
            external_organization_id: "org:helix-players".into(),
            record_namespace: "governance.proposal-submitted".into(),
            allowed_effects: BTreeSet::from([AdmissionEffectKind::ProjectPublication]),
            allowed_target_id: Some(project_id.clone()),
        }],
    )
    .map_err(|e| e.to_string())?;
    let reviewer = AdmissionReviewer {
        id: id("reviewer:three-worlds-player-org"),
    };
    let mut player_org = PlayerOrganizationAdmission::new(
        [CollaborationProvider {
            id: provider_id.clone(),
            namespace: "mycelix-holochain".into(),
        }],
        policy,
        [reviewer.clone()],
    )
    .map_err(|e| e.to_string())?;
    let external_record_id = id("external:mycelix-project-proposal"),
    player_org
        .observe_external_record(ExternalOrganizationRecord {
            id: external_record_id.clone(),
            organization: ExternalOrganizationRef {
                provider_id,
                organization_id: "org:helix-players".into(),
            },
            record_namespace: "governance.proposal-submitted".into(),
            provider_record_id: "uhCkk-three-worlds-project-action-hash".into(),
            external_actor_id: Some("did:mycelix:player-alice".into()),
            external_subject_id: Some("proposal:relief-audit".into()),
            observed_tick: 270,
            source_event_id: id("event:observe-mycelix-project-proposal"),
        })
        .map_err(|e| e.to_string())?;
    let admitted = player_org
        .admit(
            AdmissionRequest {
                id: id("admission:mycelix-project-proposal"),
                policy_id: id("policy:three-worlds-player-org"),
                policy_generation: 1,
                rule_id: admission_rule_id,
                external_record_id,
                target: AdmissionTarget {
                    target_id: project_id.clone(),
                    effect: AdmissionEffectKind::ProjectPublication,
                    internal_actor_id: None,
                    internal_subject_id: None,
                },
                requested_tick: 271,
                source_event_id: id("event:request-project-admission"),
            },
            reviewer,
            271,
            id("event:admit-project-proposal"),
        )
        .map_err(|e| e.to_string())?;

    let mut projects = ProjectLedger::new();
    let external_org_admission_does_not_publish_project = projects.project(&project_id).is_none()
        && admitted.target.effect == AdmissionEffectKind::ProjectPublication;
    let deliverable_id = id("deliverable:relief-arrival-audit"),
    projects
        .publish_project(ProjectSpec {
            id: project_id.clone(),
            issuer_id: vesper.clone(),
            title: "Vesper Relief Arrival Audit".into(),
            opens_tick: 272,
            closes_tick: Some(400),
            deliverables: BTreeMap::from([(
                deliverable_id.clone(),
                DeliverableSpec {
                    id: deliverable_id.clone(),
                    kind: "shipment-arrival-audit".into(),
                    description: "Provide inspectable evidence that the relief shipment reached Vesper.".into(),
                    required: true,
                },
            )]),
            reviewer_ids: BTreeSet::from([vesper.clone()]),
            source_event_id: id("event:vesper-publishes-relief-audit"),
        })
        .map_err(|e| e.to_string())?;
    let contribution_id = id("contribution:helion-relief-audit"),
    projects
        .submit(Contribution {
            id: contribution_id.clone(),
            project_id: project_id.clone(),
            contributor_id: helion.clone(),
            deliverable_ids: BTreeSet::from([deliverable_id.clone()]),
            evidence: BTreeSet::from([ProjectEvidenceRef::EconomyShipmentArrival(
                arrival_id.clone(),
            )]),
            submitted_tick: 275,
            source_event_id: id("event:helion-submits-relief-audit"),
        })
        .map_err(|e| e.to_string())?;
    projects
        .review(ContributionReview {
            id: id("review:vesper-accepts-relief-audit"),
            contribution_id,
            reviewer_id: vesper.clone(),
            dispositions: BTreeMap::from([(
                deliverable_id,
                DeliverableDisposition::Accepted,
            )]),
            review_evidence: BTreeSet::from([ProjectEvidenceRef::EconomyShipmentArrival(
                arrival_id,
            )]),
            reviewed_tick: 280,
            source_event_id: id("event:vesper-reviews-relief-audit"),
        })
        .map_err(|e| e.to_string())?;
    let project_complete_after_explicit_review = projects
        .completion(&project_id)
        .map_err(|e| e.to_string())?
        .is_complete();

    // ---------------------------------------------------------------------
    // 9. Political conflict evolves by explicit proposals/acceptance, not a war score.
    // ---------------------------------------------------------------------
    let conflict_id = id("conflict:aster-helion-hub-crisis"),
    mut conflict = ConflictWorld::default();
    conflict
        .declare_conflict(ConflictSpec {
            id: conflict_id.clone(),
            title: "Hub Recognition and Access Crisis".into(),
            participants: vec![
                ConflictParticipant {
                    party_id: aster.clone(),
                    side_id: id("side:aster"),
                    joined_tick: 300,
                    source_event_id: id("event:aster-enters-hub-crisis"),
                },
                ConflictParticipant {
                    party_id: helion.clone(),
                    side_id: id("side:helion"),
                    joined_tick: 300,
                    source_event_id: id("event:helion-enters-hub-crisis"),
                },
            ],
            aims: vec![
                ConflictAim {
                    id: id("aim:aster-helion-recognize-mina"),
                    asserted_by_party_id: aster.clone(),
                    kind: ConflictAimKind::Recognition {
                        recognizer_id: helion.clone(),
                        subject_id: mina_claim_id.clone(),
                    },
                    asserted_tick: 300,
                    evidence: vec![ConflictEvidenceRef {
                        namespace: "recognition".into(),
                        record_id: id("recognition:helion-mina"),
                    }],
                    source_event_id: id("event:aster-conflict-aim"),
                },
                ConflictAim {
                    id: id("aim:helion-hub-access"),
                    asserted_by_party_id: helion.clone(),
                    kind: ConflictAimKind::AccessRight {
                        grantor_id: aster.clone(),
                        grantee_id: helion.clone(),
                        scope_id: hub_loc.clone(),
                    },
                    asserted_tick: 300,
                    evidence: Vec::new(),
                    source_event_id: id("event:helion-conflict-aim"),
                },
            ],
            declared_tick: 300,
            trigger_refs: vec![ConflictEvidenceRef {
                namespace: "recognition".into(),
                record_id: id("recognition:helion-mina"),
            }],
            source_event_id: id("event:declare-hub-crisis"),
        })
        .map_err(|e| e.to_string())?;
    let ceasefire_id = id("ceasefire:hub-crisis"),
    conflict
        .propose_ceasefire(CeasefireProposal {
            id: ceasefire_id.clone(),
            conflict_id: conflict_id.clone(),
            proposed_by_party_id: helion.clone(),
            required_acceptors: BTreeSet::from([aster.clone(), helion.clone()]),
            effective_not_before_tick: 330,
            expires_tick: Some(360),
            terms: Vec::new(),
            proposed_tick: 320,
            source_event_id: id("event:propose-hub-ceasefire"),
        })
        .map_err(|e| e.to_string())?;
    for (acceptance_id, party, tick) in [
        ("ceasefire-acceptance:aster", aster.clone(), 321),
        ("ceasefire-acceptance:helion", helion.clone(), 322),
    ] {
        conflict
            .accept_ceasefire(CeasefireAcceptance {
                id: id(acceptance_id),
                proposal_id: ceasefire_id.clone(),
                party_id: party,
                accepted_tick: tick,
                source_event_id: id(&format!("event:{acceptance_id}")),
            })
            .map_err(|e| e.to_string())?;
    }
    let conflict_active_before_ceasefire =
        conflict.conflict_state(&conflict_id, 329) == Some(ConflictState::Active);
    let conflict_in_ceasefire_window =
        conflict.conflict_state(&conflict_id, 330) == Some(ConflictState::Ceasefire);

    let settlement_id = id("settlement:hub-crisis"),
    conflict
        .propose_settlement(SettlementProposal {
            id: settlement_id.clone(),
            conflict_id: conflict_id.clone(),
            proposed_by_party_id: aster.clone(),
            required_acceptors: BTreeSet::from([aster.clone(), helion.clone()]),
            terms: vec![
                SettlementTerm {
                    id: id("settlement-term:helion-hub-access"),
                    kind: SettlementTermKind::AccessRightRequested {
                        grantor_id: aster.clone(),
                        grantee_id: helion.clone(),
                        scope_id: hub_loc.clone(),
                    },
                    evidence: Vec::new(),
                },
                SettlementTerm {
                    id: id("settlement-term:hub-reconstruction"),
                    kind: SettlementTermKind::ReconstructionObligation {
                        obligor_id: aster.clone(),
                        beneficiary_id: helion.clone(),
                        project_namespace: "open-project".into(),
                        subject_id: hub_loc,
                        quantity: None,
                        unit_id: None,
                        due_tick: Some(450),
                    },
                    evidence: Vec::new(),
                },
            ],
            proposed_tick: 350,
            source_event_id: id("event:propose-hub-settlement"),
        })
        .map_err(|e| e.to_string())?;
    for (acceptance_id, party, tick) in [
        ("settlement-acceptance:aster", aster.clone(), 351),
        ("settlement-acceptance:helion", helion.clone(), 352),
    ] {
        conflict
            .accept_settlement(SettlementAcceptance {
                id: id(acceptance_id),
                proposal_id: settlement_id.clone(),
                party_id: party,
                accepted_tick: tick,
                source_event_id: id(&format!("event:{acceptance_id}")),
            })
            .map_err(|e| e.to_string())?;
    }
    let conflict_settled_by_acceptance =
        conflict.conflict_state(&conflict_id, 352) == Some(ConflictState::Settled);
    let settlement_term_count = conflict
        .agreed_settlement_terms(&conflict_id)
        .ok_or_else(|| "accepted settlement terms unavailable".to_string())?
        .count();

    let asset_owner_remains_aster_after_shard_transfer = assets
        .active_relations_by_kind(&freighter, AssetRelationKind::Owner, 352)
        .any(|relation| relation.party_id == aster);
    let external_admission_does_not_grant_institutional_authority =
        aster_institution.authority_grants().count() == authority_grants_before_external_admission;

    Ok(ThreeWorldsReport {
        institutional_holder_survives_competing_claims,
        simultaneous_succession_claims,
        recognition_is_source_relative,
        signal_unavailable_before_delivery,
        signal_delivery_tick: 130,
        freight_unavailable_before_arrival,
        freight_arrival_tick: transit_binding.earliest_arrival_tick,
        signal_arrives_before_freight: 130 < transit_binding.earliest_arrival_tick,
        treaty_active,
        treaty_assessments_disagree,
        external_org_admission_does_not_publish_project,
        project_complete_after_explicit_review,
        conflict_active_before_ceasefire,
        conflict_in_ceasefire_window,
        conflict_settled_by_acceptance,
        settlement_term_count,
        anonymous_population_before,
        represented_population_after_materialization,
        materialized_identity_begins_tick,
        authority_before_transfer_commit,
        authority_after_transfer_commit,
        authority_transfer_retry_idempotent,
        asset_owner_remains_aster_after_shard_transfer,
        external_admission_does_not_grant_institutional_authority,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_worlds_history_preserves_every_authority_boundary() {
        let report = run_three_worlds().expect("Three Worlds lab should construct");
        assert!(report.institutional_holder_survives_competing_claims);
        assert!(report.simultaneous_succession_claims);
        assert!(report.recognition_is_source_relative);
        assert!(report.signal_unavailable_before_delivery);
        assert!(report.freight_unavailable_before_arrival);
        assert!(report.signal_arrives_before_freight);
        assert!(report.treaty_active);
        assert!(report.treaty_assessments_disagree);
        assert!(report.external_org_admission_does_not_publish_project);
        assert!(report.project_complete_after_explicit_review);
        assert!(report.conflict_active_before_ceasefire);
        assert!(report.conflict_in_ceasefire_window);
        assert!(report.conflict_settled_by_acceptance);
        assert_eq!(report.settlement_term_count, 2);
        assert_eq!(report.anonymous_population_before, 1_000_000);
        assert_eq!(report.represented_population_after_materialization, 1_000_000);
        assert_eq!(report.materialized_identity_begins_tick, 200);
        assert_eq!(report.authority_before_transfer_commit, "authority:aster-shard");
        assert_eq!(report.authority_after_transfer_commit, "authority:vesper-shard");
        assert!(report.authority_transfer_retry_idempotent);
        assert!(report.asset_owner_remains_aster_after_shard_transfer);
        assert!(report.external_admission_does_not_grant_institutional_authority);
    }
}
