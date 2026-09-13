use std::collections::BTreeMap;
use symtropy_evolution_core::{
    derive_offspring, AlleleId, AnalysisAuthorityRef, AnalysisContentDigest, AnalysisMethodId,
    ConsequenceWindowId, EvolutionIndividualId, EvolutionOperatorProfile,
    EvolutionaryContextContentDigest, EvolutionaryContextId, EvolutionaryContextRef,
    HereditarySchema, HereditarySchemaId, HereditaryState, LocusDefinition, LocusId,
    MutationProfile, ObservedOffspringEvidence, OperatorProfileId, PopulationGeneration,
    RealizedGeneFlowObservation, RecombinationMode, RecombinationProfile, ReproductionEventId,
    ReproductionMode, ReproductiveContactContextPolicy, ReproductiveContactDesignError,
    ReproductiveContactEvidenceError, ReproductiveContactStudy, ReproductiveContactStudyDesign,
    ReproductiveContactStudyId, ReproductiveContactStudyStatus, ReproductiveObservationStage,
    ReproductiveOpportunityDeclaration, ReproductiveOpportunityEvidenceInput,
    ReproductiveOpportunityId, ReproductiveOpportunityOutcome, ReproductiveStageEvidence,
    ValidatedReproductiveContactStudy, ValidatedReproductiveContactStudyDesign,
};

fn authority(label: &str, byte: u8) -> AnalysisAuthorityRef {
    AnalysisAuthorityRef::new(
        AnalysisMethodId::new(label).unwrap(),
        1,
        AnalysisContentDigest::new([byte; 32]),
    )
}

fn context(byte: u8) -> symtropy_evolution_core::EvolutionaryContextRefDigest {
    EvolutionaryContextRef::new(
        EvolutionaryContextId::new(format!("contact-context-{byte}")).unwrap(),
        1,
        EvolutionaryContextContentDigest::new([byte; 32]),
        ConsequenceWindowId::new(format!("contact-window-{byte}")).unwrap(),
    )
    .canonical_digest()
    .unwrap()
}

#[derive(Clone)]
struct ContactAuthorities {
    lineage_a: AnalysisAuthorityRef,
    lineage_b: AnalysisAuthorityRef,
    opportunity_definition: AnalysisAuthorityRef,
    contact: AnalysisAuthorityRef,
    pairing: AnalysisAuthorityRef,
    mating: AnalysisAuthorityRef,
    conception: AnalysisAuthorityRef,
    viability: AnalysisAuthorityRef,
    fertility: AnalysisAuthorityRef,
    parentage: AnalysisAuthorityRef,
    gene_flow: AnalysisAuthorityRef,
    demography: AnalysisAuthorityRef,
    missing: AnalysisAuthorityRef,
}

fn authorities(seed: u8) -> ContactAuthorities {
    ContactAuthorities {
        lineage_a: authority("lineage-a", seed),
        lineage_b: authority("lineage-b", seed.wrapping_add(1)),
        opportunity_definition: authority("opportunity-definition", seed.wrapping_add(2)),
        contact: authority("contact-protocol", seed.wrapping_add(3)),
        pairing: authority("pairing-protocol", seed.wrapping_add(4)),
        mating: authority("mating-protocol", seed.wrapping_add(5)),
        conception: authority("conception-protocol", seed.wrapping_add(6)),
        viability: authority("offspring-viability-protocol", seed.wrapping_add(7)),
        fertility: authority("offspring-fertility-protocol", seed.wrapping_add(8)),
        parentage: authority("parentage-authority", seed.wrapping_add(9)),
        gene_flow: authority("gene-flow-materialization", seed.wrapping_add(10)),
        demography: authority("contact-demography", seed.wrapping_add(11)),
        missing: authority("contact-missing-data", seed.wrapping_add(12)),
    }
}

fn opportunity(
    auth: &ContactAuthorities,
    id: &str,
    generation: u64,
    context_digest: symtropy_evolution_core::EvolutionaryContextRefDigest,
    zone_byte: u8,
) -> ReproductiveOpportunityDeclaration {
    ReproductiveOpportunityDeclaration::new(
        ReproductiveOpportunityId::new(id).unwrap(),
        PopulationGeneration(generation),
        EvolutionIndividualId::new(format!("{id}-parent-a")).unwrap(),
        auth.lineage_a.clone(),
        authority("lineage-a-membership", zone_byte.wrapping_add(20)),
        EvolutionIndividualId::new(format!("{id}-parent-b")).unwrap(),
        auth.lineage_b.clone(),
        authority("lineage-b-membership", zone_byte.wrapping_add(21)),
        context_digest,
        authority("contact-zone", zone_byte),
    )
    .unwrap()
}

fn design_with(
    auth: &ContactAuthorities,
    ctx: symtropy_evolution_core::EvolutionaryContextRefDigest,
    opportunities: Vec<ReproductiveOpportunityDeclaration>,
) -> ReproductiveContactStudyDesign {
    ReproductiveContactStudyDesign::declare(
        ReproductiveContactStudyId::new("contact-study-v1").unwrap(),
        auth.lineage_a.clone(),
        auth.lineage_b.clone(),
        PopulationGeneration(3),
        PopulationGeneration(5),
        ReproductiveContactContextPolicy::ExactContext { context_digest: ctx },
        opportunities,
        auth.opportunity_definition.clone(),
        auth.contact.clone(),
        auth.pairing.clone(),
        auth.mating.clone(),
        auth.conception.clone(),
        auth.viability.clone(),
        auth.fertility.clone(),
        auth.parentage.clone(),
        auth.gene_flow.clone(),
        auth.demography.clone(),
        auth.missing.clone(),
    )
    .unwrap()
}

fn current_design<'a>(
    design: &'a ReproductiveContactStudyDesign,
    auth: &ContactAuthorities,
    ctx: symtropy_evolution_core::EvolutionaryContextRefDigest,
    opportunities: Vec<ReproductiveOpportunityDeclaration>,
) -> ValidatedReproductiveContactStudyDesign<'a> {
    ValidatedReproductiveContactStudyDesign::validate_current(
        design,
        auth.lineage_a.clone(),
        auth.lineage_b.clone(),
        ReproductiveContactContextPolicy::ExactContext { context_digest: ctx },
        opportunities,
        auth.opportunity_definition.clone(),
        auth.contact.clone(),
        auth.pairing.clone(),
        auth.mating.clone(),
        auth.conception.clone(),
        auth.viability.clone(),
        auth.fertility.clone(),
        auth.parentage.clone(),
        auth.gene_flow.clone(),
        auth.demography.clone(),
        auth.missing.clone(),
    )
    .unwrap()
}

fn stage(
    auth: &ContactAuthorities,
    stage: ReproductiveObservationStage,
    byte: u8,
) -> ReproductiveStageEvidence {
    let protocol = match stage {
        ReproductiveObservationStage::Contact => auth.contact.clone(),
        ReproductiveObservationStage::Pairing => auth.pairing.clone(),
        ReproductiveObservationStage::Mating => auth.mating.clone(),
        ReproductiveObservationStage::Conception => auth.conception.clone(),
        ReproductiveObservationStage::OffspringViability => auth.viability.clone(),
        ReproductiveObservationStage::OffspringFertility => auth.fertility.clone(),
    };
    ReproductiveStageEvidence::new(
        protocol,
        authority(&format!("{:?}-observation", stage), byte),
    )
}

fn no_contact_outcome(auth: &ContactAuthorities, byte: u8) -> ReproductiveOpportunityOutcome {
    ReproductiveOpportunityOutcome::NoContact {
        contact: stage(auth, ReproductiveObservationStage::Contact, byte),
    }
}

fn no_pairing_outcome(auth: &ContactAuthorities, byte: u8) -> ReproductiveOpportunityOutcome {
    ReproductiveOpportunityOutcome::ContactNoPairing {
        contact: stage(auth, ReproductiveObservationStage::Contact, byte),
        pairing: stage(
            auth,
            ReproductiveObservationStage::Pairing,
            byte.wrapping_add(1),
        ),
    }
}

fn no_conception_outcome(auth: &ContactAuthorities, byte: u8) -> ReproductiveOpportunityOutcome {
    ReproductiveOpportunityOutcome::MatingNoConception {
        contact: stage(auth, ReproductiveObservationStage::Contact, byte),
        pairing: stage(
            auth,
            ReproductiveObservationStage::Pairing,
            byte.wrapping_add(1),
        ),
        mating: stage(
            auth,
            ReproductiveObservationStage::Mating,
            byte.wrapping_add(2),
        ),
        conception: stage(
            auth,
            ReproductiveObservationStage::Conception,
            byte.wrapping_add(3),
        ),
    }
}

fn none_gene_flow(auth: &ContactAuthorities, byte: u8) -> RealizedGeneFlowObservation {
    RealizedGeneFlowObservation::NoneObserved {
        materialization_authority: auth.gene_flow.clone(),
        evidence: authority("no-realized-gene-flow", byte),
    }
}

fn realized_gene_flow(auth: &ContactAuthorities, byte: u8) -> RealizedGeneFlowObservation {
    RealizedGeneFlowObservation::Realized {
        materialization_authority: auth.gene_flow.clone(),
        ancestry_evidence: authority("realized-gene-flow", byte),
    }
}

fn input(
    id: &str,
    outcome: ReproductiveOpportunityOutcome,
    gene_flow: RealizedGeneFlowObservation,
    auth: &ContactAuthorities,
) -> ReproductiveOpportunityEvidenceInput {
    ReproductiveOpportunityEvidenceInput {
        opportunity_id: ReproductiveOpportunityId::new(id).unwrap(),
        outcome,
        realized_gene_flow: gene_flow,
        demography_accounting_authority: auth.demography.clone(),
    }
}

fn offspring_evidence(auth: &ContactAuthorities, label: &str) -> ObservedOffspringEvidence {
    let locus = LocusId::new("hybrid-locus").unwrap();
    let a = AlleleId::new("a").unwrap();
    let b = AlleleId::new("b").unwrap();
    let schema = HereditarySchema::new(
        HereditarySchemaId::new("contact-hybrid-schema").unwrap(),
        2,
        vec![LocusDefinition::new(locus.clone(), [a.clone(), b.clone()]).unwrap()],
    )
    .unwrap();
    let parent_a = HereditaryState::new(
        &schema,
        BTreeMap::from([(locus.clone(), vec![a.clone(), a])]),
    )
    .unwrap();
    let parent_b = HereditaryState::new(
        &schema,
        BTreeMap::from([(locus, vec![b.clone(), b])]),
    )
    .unwrap();
    let operators = EvolutionOperatorProfile {
        profile_id: OperatorProfileId::new("contact-operators").unwrap(),
        version: "1".to_string(),
        mutation: MutationProfile {
            model_id: "none".to_string(),
            version: "1".to_string(),
            per_copy_rate_ppm: 0,
        },
        recombination: RecombinationProfile {
            model_id: "independent-loci".to_string(),
            version: "1".to_string(),
            mode: RecombinationMode::IndependentLoci,
        },
    };
    let event_id = ReproductionEventId::new(label).unwrap();
    let offspring = derive_offspring(
        &schema,
        &[&parent_a, &parent_b],
        &event_id,
        &operators,
        ReproductionMode::BiparentalDiploidIndependentLoci,
    )
    .unwrap();
    ObservedOffspringEvidence {
        event_id,
        reproduction_provenance_digest: offspring.provenance.canonical_digest(),
        parentage_authority: auth.parentage.clone(),
        parentage_evidence: authority("parentage-observation", 220),
    }
}

fn viable_infertile_outcome(
    auth: &ContactAuthorities,
    offspring: ObservedOffspringEvidence,
    byte: u8,
) -> ReproductiveOpportunityOutcome {
    ReproductiveOpportunityOutcome::ViableInfertileOffspring {
        contact: stage(auth, ReproductiveObservationStage::Contact, byte),
        pairing: stage(auth, ReproductiveObservationStage::Pairing, byte.wrapping_add(1)),
        mating: stage(auth, ReproductiveObservationStage::Mating, byte.wrapping_add(2)),
        conception: stage(
            auth,
            ReproductiveObservationStage::Conception,
            byte.wrapping_add(3),
        ),
        viability: stage(
            auth,
            ReproductiveObservationStage::OffspringViability,
            byte.wrapping_add(4),
        ),
        offspring,
        fertility: stage(
            auth,
            ReproductiveObservationStage::OffspringFertility,
            byte.wrapping_add(5),
        ),
    }
}

fn viable_fertile_outcome(
    auth: &ContactAuthorities,
    offspring: ObservedOffspringEvidence,
    byte: u8,
) -> ReproductiveOpportunityOutcome {
    ReproductiveOpportunityOutcome::ViableFertileOffspring {
        contact: stage(auth, ReproductiveObservationStage::Contact, byte),
        pairing: stage(auth, ReproductiveObservationStage::Pairing, byte.wrapping_add(1)),
        mating: stage(auth, ReproductiveObservationStage::Mating, byte.wrapping_add(2)),
        conception: stage(
            auth,
            ReproductiveObservationStage::Conception,
            byte.wrapping_add(3),
        ),
        viability: stage(
            auth,
            ReproductiveObservationStage::OffspringViability,
            byte.wrapping_add(4),
        ),
        offspring,
        fertility: stage(
            auth,
            ReproductiveObservationStage::OffspringFertility,
            byte.wrapping_add(5),
        ),
    }
}

#[test]
fn no_contact_is_distinct_from_observed_reproductive_failure() {
    let ctx = context(40);
    let auth = authorities(60);
    let opportunities = vec![
        opportunity(&auth, "op-b", 4, ctx, 71),
        opportunity(&auth, "op-a", 3, ctx, 70),
    ];
    let design = design_with(&auth, ctx, opportunities.clone());
    assert_eq!(
        design
            .opportunities
            .iter()
            .map(|opportunity| opportunity.opportunity_id.as_str())
            .collect::<Vec<_>>(),
        vec!["op-a", "op-b"]
    );
    let current = current_design(&design, &auth, ctx, opportunities);

    let no_contact = ReproductiveContactStudy::capture(
        &current,
        vec![
            input("op-b", no_contact_outcome(&auth, 81), none_gene_flow(&auth, 91), &auth),
            input("op-a", no_contact_outcome(&auth, 80), none_gene_flow(&auth, 90), &auth),
        ],
    )
    .unwrap();
    assert_eq!(no_contact.status, ReproductiveContactStudyStatus::NoContactObserved);

    let observed_failure = ReproductiveContactStudy::capture(
        &current,
        vec![
            input("op-a", no_pairing_outcome(&auth, 82), none_gene_flow(&auth, 92), &auth),
            input(
                "op-b",
                no_conception_outcome(&auth, 83),
                none_gene_flow(&auth, 93),
                &auth,
            ),
        ],
    )
    .unwrap();
    assert_eq!(
        observed_failure.status,
        ReproductiveContactStudyStatus::NoRealizedGeneFlowWithObservedOpportunity
    );
    assert_ne!(
        no_contact.canonical_digest().unwrap(),
        observed_failure.canonical_digest().unwrap()
    );
}

#[test]
fn viable_infertile_and_fertile_hybrids_are_distinct_and_bind_real_parentage() {
    let ctx = context(41);
    let auth = authorities(61);
    let opportunities = vec![opportunity(&auth, "hybrid", 3, ctx, 72)];
    let design = design_with(&auth, ctx, opportunities.clone());
    let current = current_design(&design, &auth, ctx, opportunities);
    let offspring = offspring_evidence(&auth, "hybrid-event");

    let infertile = ReproductiveContactStudy::capture(
        &current,
        vec![input(
            "hybrid",
            viable_infertile_outcome(&auth, offspring.clone(), 100),
            none_gene_flow(&auth, 106),
            &auth,
        )],
    )
    .unwrap();
    assert_eq!(
        infertile.status,
        ReproductiveContactStudyStatus::ViableInfertileHybridObserved
    );

    let fertile = ReproductiveContactStudy::capture(
        &current,
        vec![input(
            "hybrid",
            viable_fertile_outcome(&auth, offspring, 110),
            none_gene_flow(&auth, 116),
            &auth,
        )],
    )
    .unwrap();
    assert_eq!(fertile.status, ReproductiveContactStudyStatus::ViableFertileHybridObserved);
    assert_ne!(infertile.canonical_digest().unwrap(), fertile.canonical_digest().unwrap());
}

#[test]
fn realized_ancestry_gene_flow_overrides_direct_no_contact_observation() {
    let ctx = context(42);
    let auth = authorities(62);
    let opportunities = vec![opportunity(&auth, "ancestry", 3, ctx, 73)];
    let design = design_with(&auth, ctx, opportunities.clone());
    let current = current_design(&design, &auth, ctx, opportunities);
    let study = ReproductiveContactStudy::capture(
        &current,
        vec![input(
            "ancestry",
            no_contact_outcome(&auth, 104),
            realized_gene_flow(&auth, 105),
            &auth,
        )],
    )
    .unwrap();
    assert_eq!(study.status, ReproductiveContactStudyStatus::CrossLineageGeneFlowObserved);
}

#[test]
fn omitted_opportunity_and_authority_or_protocol_drift_fail_closed() {
    let ctx = context(43);
    let auth = authorities(63);
    let opportunities = vec![
        opportunity(&auth, "a", 3, ctx, 74),
        opportunity(&auth, "b", 4, ctx, 75),
    ];
    let design = design_with(&auth, ctx, opportunities.clone());
    let current = current_design(&design, &auth, ctx, opportunities);
    assert!(matches!(
        ReproductiveContactStudy::capture(
            &current,
            vec![input("a", no_contact_outcome(&auth, 106), none_gene_flow(&auth, 107), &auth)],
        ),
        Err(ReproductiveContactEvidenceError::IncompleteOpportunityCoverage)
    ));

    let wrong_protocol = ReproductiveOpportunityOutcome::NoContact {
        contact: ReproductiveStageEvidence::new(
            authority("changed-contact-protocol", 108),
            authority("no-contact", 109),
        ),
    };
    assert!(matches!(
        ReproductiveContactStudy::capture(
            &current,
            vec![
                input("a", wrong_protocol, none_gene_flow(&auth, 110), &auth),
                input("b", no_contact_outcome(&auth, 111), none_gene_flow(&auth, 112), &auth),
            ],
        ),
        Err(ReproductiveContactEvidenceError::StageProtocolMismatch {
            stage: ReproductiveObservationStage::Contact,
            ..
        })
    ));

    let wrong_flow = ReproductiveOpportunityEvidenceInput {
        opportunity_id: ReproductiveOpportunityId::new("a").unwrap(),
        outcome: no_contact_outcome(&auth, 113),
        realized_gene_flow: RealizedGeneFlowObservation::NoneObserved {
            materialization_authority: authority("changed-flow-authority", 114),
            evidence: authority("none", 115),
        },
        demography_accounting_authority: auth.demography.clone(),
    };
    let valid_b = input(
        "b",
        no_contact_outcome(&auth, 116),
        none_gene_flow(&auth, 117),
        &auth,
    );
    assert!(matches!(
        ReproductiveContactStudy::capture(&current, vec![wrong_flow, valid_b]),
        Err(ReproductiveContactEvidenceError::GeneFlowAuthorityMismatch(_))
    ));
}

#[test]
fn lineage_membership_is_preregistered_and_changes_design_identity() {
    let ctx = context(44);
    let auth = authorities(64);
    let good = opportunity(&auth, "a", 3, ctx, 76);
    let design = design_with(&auth, ctx, vec![good]);

    let changed_membership = ReproductiveOpportunityDeclaration::new(
        ReproductiveOpportunityId::new("a").unwrap(),
        PopulationGeneration(3),
        EvolutionIndividualId::new("a-parent-a").unwrap(),
        auth.lineage_a.clone(),
        authority("lineage-a-membership-changed", 210),
        EvolutionIndividualId::new("a-parent-b").unwrap(),
        auth.lineage_b.clone(),
        authority("lineage-b-membership", 97),
        ctx,
        authority("contact-zone", 76),
    )
    .unwrap();
    let changed = design_with(&auth, ctx, vec![changed_membership]);
    assert_ne!(design.canonical_digest().unwrap(), changed.canonical_digest().unwrap());

    let wrong_lineage = ReproductiveOpportunityDeclaration::new(
        ReproductiveOpportunityId::new("bad").unwrap(),
        PopulationGeneration(3),
        EvolutionIndividualId::new("bad-parent-a").unwrap(),
        auth.lineage_b.clone(),
        authority("wrong-membership", 211),
        EvolutionIndividualId::new("bad-parent-b").unwrap(),
        auth.lineage_b.clone(),
        authority("lineage-b-membership", 212),
        ctx,
        authority("contact-zone", 77),
    )
    .unwrap();
    assert!(matches!(
        ReproductiveContactStudyDesign::declare(
            ReproductiveContactStudyId::new("wrong-lineage").unwrap(),
            auth.lineage_a.clone(),
            auth.lineage_b.clone(),
            PopulationGeneration(3),
            PopulationGeneration(5),
            ReproductiveContactContextPolicy::ExactContext { context_digest: ctx },
            vec![wrong_lineage],
            auth.opportunity_definition.clone(),
            auth.contact.clone(),
            auth.pairing.clone(),
            auth.mating.clone(),
            auth.conception.clone(),
            auth.viability.clone(),
            auth.fertility.clone(),
            auth.parentage.clone(),
            auth.gene_flow.clone(),
            auth.demography.clone(),
            auth.missing.clone(),
        ),
        Err(ReproductiveContactDesignError::LineageMembershipMismatch(_))
    ));
}

#[test]
fn restored_design_rechecks_context_membership_and_current_replay() {
    let ctx = context(45);
    let auth = authorities(65);
    let opportunities = vec![opportunity(&auth, "a", 3, ctx, 77)];
    let design = design_with(&auth, ctx, opportunities.clone());
    let restored: ReproductiveContactStudyDesign =
        serde_json::from_slice(&serde_json::to_vec(&design).unwrap()).unwrap();
    let current = current_design(&restored, &auth, ctx, opportunities.clone());
    let study = ReproductiveContactStudy::capture(
        &current,
        vec![input("a", no_contact_outcome(&auth, 118), none_gene_flow(&auth, 119), &auth)],
    )
    .unwrap();
    let restored_study: ReproductiveContactStudy =
        serde_json::from_slice(&serde_json::to_vec(&study).unwrap()).unwrap();
    let validated = ValidatedReproductiveContactStudy::validate_current(
        &restored_study,
        &current,
        vec![input("a", no_contact_outcome(&auth, 118), none_gene_flow(&auth, 119), &auth)],
    )
    .unwrap();
    assert_eq!(validated.study_digest(), study.canonical_digest().unwrap());

    let drifted = vec![opportunity(&auth, "a", 3, context(99), 77)];
    assert!(matches!(
        ReproductiveContactStudyDesign::declare(
            ReproductiveContactStudyId::new("drift").unwrap(),
            auth.lineage_a.clone(),
            auth.lineage_b.clone(),
            PopulationGeneration(3),
            PopulationGeneration(5),
            ReproductiveContactContextPolicy::ExactContext { context_digest: ctx },
            drifted,
            auth.opportunity_definition.clone(),
            auth.contact.clone(),
            auth.pairing.clone(),
            auth.mating.clone(),
            auth.conception.clone(),
            auth.viability.clone(),
            auth.fertility.clone(),
            auth.parentage.clone(),
            auth.gene_flow.clone(),
            auth.demography.clone(),
            auth.missing.clone(),
        ),
        Err(ReproductiveContactDesignError::UndeclaredContextDrift(_))
    ));
}

#[test]
fn unavailable_evidence_is_typed_and_serialized_status_cannot_be_forged() {
    let ctx = context(46);
    let auth = authorities(66);
    let opportunities = vec![opportunity(&auth, "a", 3, ctx, 78)];
    let design = design_with(&auth, ctx, opportunities.clone());
    let current = current_design(&design, &auth, ctx, opportunities);
    let study = ReproductiveContactStudy::capture(
        &current,
        vec![input(
            "a",
            ReproductiveOpportunityOutcome::Unavailable {
                stage: ReproductiveObservationStage::Pairing,
                missing_data_authority: auth.missing.clone(),
                reason: authority("camera-failure", 120),
            },
            RealizedGeneFlowObservation::Unavailable {
                missing_data_authority: auth.missing.clone(),
                reason: authority("ancestry-not-yet-observed", 121),
            },
            &auth,
        )],
    )
    .unwrap();
    assert_eq!(study.status, ReproductiveContactStudyStatus::InsufficientEvidence);

    let mut value = serde_json::to_value(study).unwrap();
    value["status"] = serde_json::json!("NoContactObserved");
    let altered: ReproductiveContactStudy = serde_json::from_value(value).unwrap();
    assert!(matches!(
        altered.canonical_digest(),
        Err(ReproductiveContactEvidenceError::StatusInvariant)
    ));
}

fn collect_keys(value: &serde_json::Value, keys: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(object) => {
            for (key, nested) in object {
                keys.push(key.clone());
                collect_keys(nested, keys);
            }
        }
        serde_json::Value::Array(values) => {
            for nested in values {
                collect_keys(nested, keys);
            }
        }
        _ => {}
    }
}

#[test]
fn contact_evidence_wire_shape_does_not_claim_isolation_or_species_status() {
    let ctx = context(47);
    let auth = authorities(67);
    let opportunities = vec![opportunity(&auth, "a", 3, ctx, 79)];
    let design = design_with(&auth, ctx, opportunities.clone());
    let current = current_design(&design, &auth, ctx, opportunities);
    let study = ReproductiveContactStudy::capture(
        &current,
        vec![input("a", no_pairing_outcome(&auth, 122), none_gene_flow(&auth, 123), &auth)],
    )
    .unwrap();
    let value = serde_json::to_value(study).unwrap();
    let mut keys = Vec::new();
    collect_keys(&value, &mut keys);
    for forbidden in [
        "reproductive_isolation",
        "isolated",
        "species",
        "species_status",
        "speciation",
    ] {
        assert!(!keys.iter().any(|key| key == forbidden));
    }
}
