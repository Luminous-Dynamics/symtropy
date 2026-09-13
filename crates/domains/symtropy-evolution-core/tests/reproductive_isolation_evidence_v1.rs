include!("reproductive_contact_evidence_v1.rs");

use symtropy_evolution_core::{
    IsolationContextCompatibility, IsolationStudyDesignInput, IsolationStudyEvidenceInput,
    IsolationStudyUnitId, ReproductiveIsolationDesign, ReproductiveIsolationDesignError,
    ReproductiveIsolationDesignId, ReproductiveIsolationEvidence,
    ReproductiveIsolationEvidenceError, ReproductiveIsolationStatus,
    ValidatedReproductiveIsolationDesign, ValidatedReproductiveIsolationEvidence,
};

#[derive(Clone)]
struct IsoContact {
    design: ReproductiveContactStudyDesign,
    opportunities: Vec<ReproductiveOpportunityDeclaration>,
}

#[derive(Clone, Copy)]
enum ContactKind {
    Barrier,
    NoContact,
    Fertile,
    GeneFlow,
    UnknownFertility,
}

fn iso_contact(
    auth: &ContactAuthorities,
    ctx: symtropy_evolution_core::EvolutionaryContextRefDigest,
    prefix: &str,
    zone: u8,
) -> IsoContact {
    let opportunities = vec![
        opportunity(auth, &format!("{prefix}-a"), 3, ctx, zone),
        opportunity(auth, &format!("{prefix}-b"), 4, ctx, zone.wrapping_add(1)),
    ];
    IsoContact {
        design: design_with(auth, ctx, opportunities.clone()),
        opportunities,
    }
}

fn iso_current_design<'a>(
    case: &'a IsoContact,
    auth: &ContactAuthorities,
    ctx: symtropy_evolution_core::EvolutionaryContextRefDigest,
) -> ValidatedReproductiveContactStudyDesign<'a> {
    current_design(&case.design, auth, ctx, case.opportunities.clone())
}

fn iso_inputs(
    case: &IsoContact,
    auth: &ContactAuthorities,
    kind: ContactKind,
    byte: u8,
) -> Vec<ReproductiveOpportunityEvidenceInput> {
    let a = case.opportunities[0].opportunity_id.as_str();
    let b = case.opportunities[1].opportunity_id.as_str();
    let barrier = || {
        vec![
            input(
                a,
                no_pairing_outcome(auth, byte),
                none_gene_flow(auth, byte.wrapping_add(20)),
                auth,
            ),
            input(
                b,
                no_conception_outcome(auth, byte.wrapping_add(4)),
                none_gene_flow(auth, byte.wrapping_add(21)),
                auth,
            ),
        ]
    };
    match kind {
        ContactKind::Barrier => barrier(),
        ContactKind::NoContact => vec![
            input(
                a,
                no_contact_outcome(auth, byte),
                none_gene_flow(auth, byte.wrapping_add(20)),
                auth,
            ),
            input(
                b,
                no_contact_outcome(auth, byte.wrapping_add(1)),
                none_gene_flow(auth, byte.wrapping_add(21)),
                auth,
            ),
        ],
        ContactKind::Fertile => vec![
            input(
                a,
                viable_fertile_outcome(
                    auth,
                    offspring_evidence(auth, &format!("{a}-event"), a),
                    byte,
                ),
                none_gene_flow(auth, byte.wrapping_add(20)),
                auth,
            ),
            input(
                b,
                no_pairing_outcome(auth, byte.wrapping_add(8)),
                none_gene_flow(auth, byte.wrapping_add(21)),
                auth,
            ),
        ],
        ContactKind::GeneFlow => vec![
            input(
                a,
                no_pairing_outcome(auth, byte),
                realized_gene_flow(auth, byte.wrapping_add(20)),
                auth,
            ),
            input(
                b,
                no_pairing_outcome(auth, byte.wrapping_add(8)),
                none_gene_flow(auth, byte.wrapping_add(21)),
                auth,
            ),
        ],
        ContactKind::UnknownFertility => vec![
            input(
                a,
                ReproductiveOpportunityOutcome::ViableOffspringFertilityUnknown {
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
                    offspring: offspring_evidence(auth, &format!("{a}-unknown"), a),
                },
                none_gene_flow(auth, byte.wrapping_add(20)),
                auth,
            ),
            input(
                b,
                no_pairing_outcome(auth, byte.wrapping_add(8)),
                none_gene_flow(auth, byte.wrapping_add(21)),
                auth,
            ),
        ],
    }
}

fn iso_study(
    case: &IsoContact,
    auth: &ContactAuthorities,
    ctx: symtropy_evolution_core::EvolutionaryContextRefDigest,
    kind: ContactKind,
    byte: u8,
) -> ReproductiveContactStudy {
    let design = iso_current_design(case, auth, ctx);
    ReproductiveContactStudy::capture(&design, iso_inputs(case, auth, kind, byte)).unwrap()
}

fn iso_current_study<'a>(
    case: &IsoContact,
    study: &'a ReproductiveContactStudy,
    auth: &ContactAuthorities,
    ctx: symtropy_evolution_core::EvolutionaryContextRefDigest,
    kind: ContactKind,
    byte: u8,
) -> ValidatedReproductiveContactStudy<'a> {
    let design = iso_current_design(case, auth, ctx);
    ValidatedReproductiveContactStudy::validate_current(
        study,
        &design,
        iso_inputs(case, auth, kind, byte),
    )
    .unwrap()
}

fn iso_authority(label: &str, byte: u8) -> AnalysisAuthorityRef {
    authority(label, byte)
}

fn iso_design(
    a: &ValidatedReproductiveContactStudyDesign<'_>,
    b: &ValidatedReproductiveContactStudyDesign<'_>,
    seed: u8,
) -> ReproductiveIsolationDesign {
    ReproductiveIsolationDesign::declare(
        ReproductiveIsolationDesignId::new("complete-barrier-v1").unwrap(),
        vec![
            IsolationStudyDesignInput {
                unit_id: IsolationStudyUnitId::new("study-b").unwrap(),
                contact_design: b,
                independence_evidence: iso_authority(
                    "study-b-independence",
                    seed.wrapping_add(1),
                ),
            },
            IsolationStudyDesignInput {
                unit_id: IsolationStudyUnitId::new("study-a").unwrap(),
                contact_design: a,
                independence_evidence: iso_authority("study-a-independence", seed),
            },
        ],
        2,
        2,
        3,
        IsolationContextCompatibility::RequireSharedContextPolicy,
        iso_authority("isolation-cohort-independence", 200),
    )
    .unwrap()
}

fn iso_current_isolation_design<'a>(
    design: &'a ReproductiveIsolationDesign,
    a: &ValidatedReproductiveContactStudyDesign<'_>,
    b: &ValidatedReproductiveContactStudyDesign<'_>,
    seed: u8,
) -> ValidatedReproductiveIsolationDesign<'a> {
    ValidatedReproductiveIsolationDesign::validate_current(
        design,
        vec![
            IsolationStudyDesignInput {
                unit_id: IsolationStudyUnitId::new("study-b").unwrap(),
                contact_design: b,
                independence_evidence: iso_authority(
                    "study-b-independence",
                    seed.wrapping_add(1),
                ),
            },
            IsolationStudyDesignInput {
                unit_id: IsolationStudyUnitId::new("study-a").unwrap(),
                contact_design: a,
                independence_evidence: iso_authority("study-a-independence", seed),
            },
        ],
        2,
        2,
        3,
        IsolationContextCompatibility::RequireSharedContextPolicy,
        iso_authority("isolation-cohort-independence", 200),
    )
    .unwrap()
}

fn two_study_evidence(
    current_design: &ValidatedReproductiveIsolationDesign<'_>,
    a: &ValidatedReproductiveContactStudy<'_>,
    b: &ValidatedReproductiveContactStudy<'_>,
) -> ReproductiveIsolationEvidence {
    ReproductiveIsolationEvidence::capture(
        current_design,
        vec![
            IsolationStudyEvidenceInput {
                unit_id: IsolationStudyUnitId::new("study-b").unwrap(),
                study: b,
            },
            IsolationStudyEvidenceInput {
                unit_id: IsolationStudyUnitId::new("study-a").unwrap(),
                study: a,
            },
        ],
    )
    .unwrap()
}

#[test]
fn complete_barriers_across_two_independent_studies_support_and_replay() {
    let ctx = context(150);
    let auth = authorities(170);
    let a = iso_contact(&auth, ctx, "support-a", 10);
    let b = iso_contact(&auth, ctx, "support-b", 20);
    let a_design = iso_current_design(&a, &auth, ctx);
    let b_design = iso_current_design(&b, &auth, ctx);
    let design = iso_design(&a_design, &b_design, 30);
    let current_design = iso_current_isolation_design(&design, &a_design, &b_design, 30);

    let a_study = iso_study(&a, &auth, ctx, ContactKind::Barrier, 40);
    let b_study = iso_study(&b, &auth, ctx, ContactKind::Barrier, 60);
    let current_a = iso_current_study(&a, &a_study, &auth, ctx, ContactKind::Barrier, 40);
    let current_b = iso_current_study(&b, &b_study, &auth, ctx, ContactKind::Barrier, 60);
    let evidence = two_study_evidence(&current_design, &current_a, &current_b);

    assert_eq!(evidence.status, ReproductiveIsolationStatus::Supported);
    assert_eq!(evidence.barrier_profile.barrier_supporting_studies, 2);
    assert_eq!(evidence.barrier_profile.observed_contact_opportunities, 4);
    assert_eq!(evidence.barrier_profile.pairing_barriers, 2);
    assert_eq!(evidence.barrier_profile.conception_barriers, 2);

    let restored_design: ReproductiveIsolationDesign =
        serde_json::from_slice(&serde_json::to_vec(&design).unwrap()).unwrap();
    let restored_current =
        iso_current_isolation_design(&restored_design, &a_design, &b_design, 30);
    let restored: ReproductiveIsolationEvidence =
        serde_json::from_slice(&serde_json::to_vec(&evidence).unwrap()).unwrap();
    let validated = ValidatedReproductiveIsolationEvidence::validate_current(
        &restored,
        &restored_current,
        vec![
            IsolationStudyEvidenceInput {
                unit_id: IsolationStudyUnitId::new("study-a").unwrap(),
                study: &current_a,
            },
            IsolationStudyEvidenceInput {
                unit_id: IsolationStudyUnitId::new("study-b").unwrap(),
                study: &current_b,
            },
        ],
    )
    .unwrap();
    assert_eq!(validated.evidence_digest(), evidence.canonical_digest().unwrap());
}

#[test]
fn fertile_hybrid_and_realized_gene_flow_each_contradict_complete_isolation() {
    let ctx = context(151);
    let auth = authorities(171);
    let a = iso_contact(&auth, ctx, "contr-a", 30);
    let b = iso_contact(&auth, ctx, "contr-b", 40);
    let a_design = iso_current_design(&a, &auth, ctx);
    let b_design = iso_current_design(&b, &auth, ctx);
    let design = iso_design(&a_design, &b_design, 31);
    let current_design = iso_current_isolation_design(&design, &a_design, &b_design, 31);
    let barrier = iso_study(&a, &auth, ctx, ContactKind::Barrier, 70);
    let current_barrier =
        iso_current_study(&a, &barrier, &auth, ctx, ContactKind::Barrier, 70);

    let fertile = iso_study(&b, &auth, ctx, ContactKind::Fertile, 90);
    let current_fertile =
        iso_current_study(&b, &fertile, &auth, ctx, ContactKind::Fertile, 90);
    let fertile_evidence = two_study_evidence(&current_design, &current_barrier, &current_fertile);
    assert_eq!(fertile_evidence.status, ReproductiveIsolationStatus::Contradicted);
    assert_eq!(fertile_evidence.barrier_profile.viable_fertile_hybrids, 1);

    let flow = iso_study(&b, &auth, ctx, ContactKind::GeneFlow, 110);
    let current_flow =
        iso_current_study(&b, &flow, &auth, ctx, ContactKind::GeneFlow, 110);
    let flow_evidence = two_study_evidence(&current_design, &current_barrier, &current_flow);
    assert_eq!(flow_evidence.status, ReproductiveIsolationStatus::Contradicted);
    assert_eq!(flow_evidence.barrier_profile.realized_gene_flow_observations, 1);
}

#[test]
fn no_contact_or_unknown_fertility_is_insufficient_and_one_study_is_not_supported() {
    let ctx = context(152);
    let auth = authorities(172);
    let a = iso_contact(&auth, ctx, "ins-a", 50);
    let b = iso_contact(&auth, ctx, "ins-b", 60);
    let a_design = iso_current_design(&a, &auth, ctx);
    let b_design = iso_current_design(&b, &auth, ctx);
    let design = iso_design(&a_design, &b_design, 32);
    let current_design = iso_current_isolation_design(&design, &a_design, &b_design, 32);

    let no_a = iso_study(&a, &auth, ctx, ContactKind::NoContact, 130);
    let no_b = iso_study(&b, &auth, ctx, ContactKind::NoContact, 140);
    let current_no_a =
        iso_current_study(&a, &no_a, &auth, ctx, ContactKind::NoContact, 130);
    let current_no_b =
        iso_current_study(&b, &no_b, &auth, ctx, ContactKind::NoContact, 140);
    let no_contact = two_study_evidence(&current_design, &current_no_a, &current_no_b);
    assert_eq!(no_contact.status, ReproductiveIsolationStatus::InsufficientEvidence);
    assert_eq!(no_contact.barrier_profile.observed_contact_opportunities, 0);

    let barrier = iso_study(&a, &auth, ctx, ContactKind::Barrier, 150);
    let unknown = iso_study(&b, &auth, ctx, ContactKind::UnknownFertility, 170);
    let current_barrier =
        iso_current_study(&a, &barrier, &auth, ctx, ContactKind::Barrier, 150);
    let current_unknown = iso_current_study(
        &b,
        &unknown,
        &auth,
        ctx,
        ContactKind::UnknownFertility,
        170,
    );
    let unknown_evidence = two_study_evidence(&current_design, &current_barrier, &current_unknown);
    assert_eq!(
        unknown_evidence.status,
        ReproductiveIsolationStatus::InsufficientEvidence
    );

    let one_barrier = two_study_evidence(&current_design, &current_barrier, &current_no_b);
    assert_eq!(one_barrier.barrier_profile.observed_contact_opportunities, 2);
    assert_eq!(one_barrier.barrier_profile.barrier_supporting_studies, 1);
    assert_eq!(one_barrier.status, ReproductiveIsolationStatus::NotSupported);
}

#[test]
fn duplicated_contact_design_omission_and_serialized_tampering_fail_closed() {
    let ctx = context(153);
    let auth = authorities(173);
    let a = iso_contact(&auth, ctx, "hard-a", 70);
    let b = iso_contact(&auth, ctx, "hard-b", 80);
    let a_design = iso_current_design(&a, &auth, ctx);
    let b_design = iso_current_design(&b, &auth, ctx);

    assert!(matches!(
        ReproductiveIsolationDesign::declare(
            ReproductiveIsolationDesignId::new("duplicate").unwrap(),
            vec![
                IsolationStudyDesignInput {
                    unit_id: IsolationStudyUnitId::new("study-a").unwrap(),
                    contact_design: &a_design,
                    independence_evidence: iso_authority("a", 210),
                },
                IsolationStudyDesignInput {
                    unit_id: IsolationStudyUnitId::new("study-b").unwrap(),
                    contact_design: &a_design,
                    independence_evidence: iso_authority("b", 211),
                },
            ],
            2,
            2,
            3,
            IsolationContextCompatibility::RequireSharedContextPolicy,
            iso_authority("cohort", 212),
        ),
        Err(ReproductiveIsolationDesignError::DuplicateContactDesignDigest)
    ));

    let design = iso_design(&a_design, &b_design, 33);
    let current_design = iso_current_isolation_design(&design, &a_design, &b_design, 33);
    let a_study = iso_study(&a, &auth, ctx, ContactKind::Barrier, 190);
    let b_study = iso_study(&b, &auth, ctx, ContactKind::Barrier, 210);
    let current_a =
        iso_current_study(&a, &a_study, &auth, ctx, ContactKind::Barrier, 190);
    let current_b =
        iso_current_study(&b, &b_study, &auth, ctx, ContactKind::Barrier, 210);

    assert!(matches!(
        ReproductiveIsolationEvidence::capture(
            &current_design,
            vec![IsolationStudyEvidenceInput {
                unit_id: IsolationStudyUnitId::new("study-a").unwrap(),
                study: &current_a,
            }],
        ),
        Err(ReproductiveIsolationEvidenceError::IncompleteStudyCoverage)
    ));

    let evidence = two_study_evidence(&current_design, &current_a, &current_b);
    let mut profile_value = serde_json::to_value(&evidence).unwrap();
    profile_value["barrier_profile"]["pairing_barriers"] = serde_json::json!(99);
    let changed_profile: ReproductiveIsolationEvidence =
        serde_json::from_value(profile_value).unwrap();
    assert!(matches!(
        changed_profile.canonical_digest(),
        Err(ReproductiveIsolationEvidenceError::BarrierProfileInvariant)
    ));

    let mut status_value = serde_json::to_value(evidence).unwrap();
    status_value["status"] = serde_json::json!("Contradicted");
    let changed_status: ReproductiveIsolationEvidence = serde_json::from_value(status_value).unwrap();
    assert!(matches!(
        changed_status.canonical_digest(),
        Err(ReproductiveIsolationEvidenceError::StatusInvariant)
    ));
}

#[test]
fn isolation_wire_shape_contains_no_species_or_speciation_claim() {
    let ctx = context(154);
    let auth = authorities(174);
    let a = iso_contact(&auth, ctx, "wire-a", 90);
    let b = iso_contact(&auth, ctx, "wire-b", 100);
    let a_design = iso_current_design(&a, &auth, ctx);
    let b_design = iso_current_design(&b, &auth, ctx);
    let design = iso_design(&a_design, &b_design, 34);
    let current_design = iso_current_isolation_design(&design, &a_design, &b_design, 34);
    let a_study = iso_study(&a, &auth, ctx, ContactKind::Barrier, 230);
    let b_study = iso_study(&b, &auth, ctx, ContactKind::Barrier, 250);
    let current_a =
        iso_current_study(&a, &a_study, &auth, ctx, ContactKind::Barrier, 230);
    let current_b =
        iso_current_study(&b, &b_study, &auth, ctx, ContactKind::Barrier, 250);
    let evidence = two_study_evidence(&current_design, &current_a, &current_b);
    let value = serde_json::to_value(evidence).unwrap();
    let mut keys = Vec::new();
    collect_keys(&value, &mut keys);
    for forbidden in ["species", "species_status", "speciation", "speciation_event"] {
        assert!(!keys.iter().any(|key| key == forbidden));
    }
}
