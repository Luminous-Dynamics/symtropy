include!("reproductive_contact_evidence_v1.rs");

use symtropy_evolution_core::{
    IsolationContextCompatibility, IsolationStudyDesignInput, IsolationStudyEvidenceInput,
    IsolationStudyUnitId, ReproductiveIsolationDesign, ReproductiveIsolationDesignError,
    ReproductiveIsolationDesignId, ReproductiveIsolationEvidence,
    ReproductiveIsolationEvidenceError, ReproductiveIsolationStatus,
    ValidatedReproductiveIsolationDesign, ValidatedReproductiveIsolationEvidence,
};

#[derive(Clone)]
struct IsolationContactCase {
    design: ReproductiveContactStudyDesign,
    opportunities: Vec<ReproductiveOpportunityDeclaration>,
}

#[derive(Clone, Copy)]
enum IsolationContactKind {
    Barrier,
    NoContact,
    FertileHybrid,
    GeneFlow,
    FertilityUnknown,
}

fn contact_case(
    auth: &ContactAuthorities,
    ctx: symtropy_evolution_core::EvolutionaryContextRefDigest,
    prefix: &str,
    zone_seed: u8,
) -> IsolationContactCase {
    let opportunities = vec![
        opportunity(
            auth,
            &format!("{prefix}-a"),
            3,
            ctx,
            zone_seed,
        ),
        opportunity(
            auth,
            &format!("{prefix}-b"),
            4,
            ctx,
            zone_seed.wrapping_add(1),
        ),
    ];
    let design = design_with(auth, ctx, opportunities.clone());
    IsolationContactCase {
        design,
        opportunities,
    }
}

fn current_contact_design<'a>(
    case: &'a IsolationContactCase,
    auth: &ContactAuthorities,
    ctx: symtropy_evolution_core::EvolutionaryContextRefDigest,
) -> ValidatedReproductiveContactStudyDesign<'a> {
    current_design(&case.design, auth, ctx, case.opportunities.clone())
}

fn contact_inputs(
    case: &IsolationContactCase,
    auth: &ContactAuthorities,
    kind: IsolationContactKind,
    byte: u8,
) -> Vec<ReproductiveOpportunityEvidenceInput> {
    let first = case.opportunities[0].opportunity_id.as_str();
    let second = case.opportunities[1].opportunity_id.as_str();
    match kind {
        IsolationContactKind::Barrier => vec![
            input(
                first,
                no_pairing_outcome(auth, byte),
                none_gene_flow(auth, byte.wrapping_add(20)),
                auth,
            ),
            input(
                second,
                no_conception_outcome(auth, byte.wrapping_add(4)),
                none_gene_flow(auth, byte.wrapping_add(21)),
                auth,
            ),
        ],
        IsolationContactKind::NoContact => vec![
            input(
                first,
                no_contact_outcome(auth, byte),
                none_gene_flow(auth, byte.wrapping_add(20)),
                auth,
            ),
            input(
                second,
                no_contact_outcome(auth, byte.wrapping_add(1)),
                none_gene_flow(auth, byte.wrapping_add(21)),
                auth,
            ),
        ],
        IsolationContactKind::FertileHybrid => vec![
            input(
                first,
                viable_fertile_outcome(
                    auth,
                    offspring_evidence(auth, &format!("{first}-event"), first),
                    byte,
                ),
                none_gene_flow(auth, byte.wrapping_add(20)),
                auth,
            ),
            input(
                second,
                no_pairing_outcome(auth, byte.wrapping_add(8)),
                none_gene_flow(auth, byte.wrapping_add(21)),
                auth,
            ),
        ],
        IsolationContactKind::GeneFlow => vec![
            input(
                first,
                no_pairing_outcome(auth, byte),
                realized_gene_flow(auth, byte.wrapping_add(20)),
                auth,
            ),
            input(
                second,
                no_pairing_outcome(auth, byte.wrapping_add(8)),
                none_gene_flow(auth, byte.wrapping_add(21)),
                auth,
            ),
        ],
        IsolationContactKind::FertilityUnknown => vec![
            input(
                first,
                ReproductiveOpportunityOutcome::ViableOffspringFertilityUnknown {
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
                    viability: stage(
                        auth,
                        ReproductiveObservationStage::OffspringViability,
                        byte.wrapping_add(4),
                    ),
                    offspring: offspring_evidence(
                        auth,
                        &format!("{first}-unknown-event"),
                        first,
                    ),
                },
                none_gene_flow(auth, byte.wrapping_add(20)),
                auth,
            ),
            input(
                second,
                no_pairing_outcome(auth, byte.wrapping_add(8)),
                none_gene_flow(auth, byte.wrapping_add(21)),
                auth,
            ),
        ],
    }
}

fn capture_contact_study(
    case: &IsolationContactCase,
    auth: &ContactAuthorities,
    ctx: symtropy_evolution_core::EvolutionaryContextRefDigest,
    kind: IsolationContactKind,
    byte: u8,
) -> ReproductiveContactStudy {
    let current = current_contact_design(case, auth, ctx);
    ReproductiveContactStudy::capture(&current, contact_inputs(case, auth, kind, byte)).unwrap()
}

fn current_contact_study<'a>(
    case: &IsolationContactCase,
    study: &'a ReproductiveContactStudy,
    auth: &ContactAuthorities,
    ctx: symtropy_evolution_core::EvolutionaryContextRefDigest,
    kind: IsolationContactKind,
    byte: u8,
) -> ValidatedReproductiveContactStudy<'a> {
    let current = current_contact_design(case, auth, ctx);
    ValidatedReproductiveContactStudy::validate_current(
        study,
        &current,
        contact_inputs(case, auth, kind, byte),
    )
    .unwrap()
}

fn isolation_rule_authority(label: &str, byte: u8) -> AnalysisAuthorityRef {
    authority(label, byte)
}

fn declare_isolation_design(
    first: &ValidatedReproductiveContactStudyDesign<'_>,
    second: &ValidatedReproductiveContactStudyDesign<'_>,
    independence_offset: u8,
) -> ReproductiveIsolationDesign {
    ReproductiveIsolationDesign::declare(
        ReproductiveIsolationDesignId::new("complete-barrier-v1").unwrap(),
        vec![
            IsolationStudyDesignInput {
                unit_id: IsolationStudyUnitId::new("study-b").unwrap(),
                contact_design: second,
                independence_evidence: isolation_rule_authority(
                    "isolation-study-b-independence",
                    independence_offset.wrapping_add(1),
                ),
            },
            IsolationStudyDesignInput {
                unit_id: IsolationStudyUnitId::new("study-a").unwrap(),
                contact_design: first,
                independence_evidence: isolation_rule_authority(
                    "isolation-study-a-independence",
                    independence_offset,
                ),
            },
        ],
        2,
        2,
        3,
        IsolationContextCompatibility::RequireSharedContextPolicy,
        isolation_rule_authority("isolation-cohort-independence-rule", 200),
        isolation_rule_authority("complete-barrier-decision-rule", 201),
    )
    .unwrap()
}

fn current_isolation_design<'a>(
    design: &'a ReproductiveIsolationDesign,
    first: &ValidatedReproductiveContactStudyDesign<'_>,
    second: &ValidatedReproductiveContactStudyDesign<'_>,
    independence_offset: u8,
) -> ValidatedReproductiveIsolationDesign<'a> {
    ValidatedReproductiveIsolationDesign::validate_current(
        design,
        vec![
            IsolationStudyDesignInput {
                unit_id: IsolationStudyUnitId::new("study-b").unwrap(),
                contact_design: second,
                independence_evidence: isolation_rule_authority(
                    "isolation-study-b-independence",
                    independence_offset.wrapping_add(1),
                ),
            },
            IsolationStudyDesignInput {
                unit_id: IsolationStudyUnitId::new("study-a").unwrap(),
                contact_design: first,
                independence_evidence: isolation_rule_authority(
                    "isolation-study-a-independence",
                    independence_offset,
                ),
            },
        ],
        2,
        2,
        3,
        IsolationContextCompatibility::RequireSharedContextPolicy,
        isolation_rule_authority("isolation-cohort-independence-rule", 200),
        isolation_rule_authority("complete-barrier-decision-rule", 201),
    )
    .unwrap()
}

#[test]
fn two_independent_complete_barrier_studies_support_isolation_and_replay() {
    let ctx = context(150);
    let auth = authorities(170);
    let a = contact_case(&auth, ctx, "iso-a", 10);
    let b = contact_case(&auth, ctx, "iso-b", 20);
    let current_a_design = current_contact_design(&a, &auth, ctx);
    let current_b_design = current_contact_design(&b, &auth, ctx);
    let design = declare_isolation_design(&current_a_design, &current_b_design, 30);
    assert_eq!(
        design
            .studies
            .iter()
            .map(|study| study.unit_id.as_str())
            .collect::<Vec<_>>(),
        vec!["study-a", "study-b"]
    );
    let current_design = current_isolation_design(
        &design,
        &current_a_design,
        &current_b_design,
        30,
    );

    let study_a = capture_contact_study(&a, &auth, ctx, IsolationContactKind::Barrier, 40);
    let study_b = capture_contact_study(&b, &auth, ctx, IsolationContactKind::Barrier, 60);
    let current_a = current_contact_study(
        &a,
        &study_a,
        &auth,
        ctx,
        IsolationContactKind::Barrier,
        40,
    );
    let current_b = current_contact_study(
        &b,
        &study_b,
        &auth,
        ctx,
        IsolationContactKind::Barrier,
        60,
    );

    let evidence = ReproductiveIsolationEvidence::capture(
        &current_design,
        vec![
            IsolationStudyEvidenceInput {
                unit_id: IsolationStudyUnitId::new("study-b").unwrap(),
                study: &current_b,
            },
            IsolationStudyEvidenceInput {
                unit_id: IsolationStudyUnitId::new("study-a").unwrap(),
                study: &current_a,
            },
        ],
    )
    .unwrap();
    assert_eq!(evidence.status, ReproductiveIsolationStatus::Supported);
    assert_eq!(evidence.barrier_profile.barrier_supporting_studies, 2);
    assert_eq!(evidence.barrier_profile.observed_contact_opportunities, 4);
    assert_eq!(evidence.barrier_profile.pairing_barriers, 2);
    assert_eq!(evidence.barrier_profile.conception_barriers, 2);

    let restored_design: ReproductiveIsolationDesign =
        serde_json::from_slice(&serde_json::to_vec(&design).unwrap()).unwrap();
    let restored_current_design = current_isolation_design(
        &restored_design,
        &current_a_design,
        &current_b_design,
        30,
    );
    let restored_evidence: ReproductiveIsolationEvidence =
        serde_json::from_slice(&serde_json::to_vec(&evidence).unwrap()).unwrap();
    let validated = ValidatedReproductiveIsolationEvidence::validate_current(
        &restored_evidence,
        &restored_current_design,
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
fn fertile_hybrid_or_realized_gene_flow_is_explicit_contradiction() {
    let ctx = context(151);
    let auth = authorities(171);
    let a = contact_case(&auth, ctx, "contr-a", 30);
    let b = contact_case(&auth, ctx, "contr-b", 40);
    let current_a_design = current_contact_design(&a, &auth, ctx);
    let current_b_design = current_contact_design(&b, &auth, ctx);
    let design = declare_isolation_design(&current_a_design, &current_b_design, 31);
    let current_design = current_isolation_design(
        &design,
        &current_a_design,
        &current_b_design,
        31,
    );

    let barrier = capture_contact_study(&a, &auth, ctx, IsolationContactKind::Barrier, 70);
    let fertile = capture_contact_study(
        &b,
        &auth,
        ctx,
        IsolationContactKind::FertileHybrid,
        90,
    );
    let current_barrier = current_contact_study(
        &a,
        &barrier,
        &auth,
        ctx,
        IsolationContactKind::Barrier,
        70,
    );
    let current_fertile = current_contact_study(
        &b,
        &fertile,
        &auth,
        ctx,
        IsolationContactKind::FertileHybrid,
        90,
    );
    let contradicted = ReproductiveIsolationEvidence::capture(
        &current_design,
        vec![
            IsolationStudyEvidenceInput {
                unit_id: IsolationStudyUnitId::new("study-a").unwrap(),
                study: &current_barrier,
            },
            IsolationStudyEvidenceInput {
                unit_id: IsolationStudyUnitId::new("study-b").unwrap(),
                study: &current_fertile,
            },
        ],
    )
    .unwrap();
    assert_eq!(contradicted.status, ReproductiveIsolationStatus::Contradicted);
    assert_eq!(contradicted.barrier_profile.viable_fertile_hybrids, 1);

    let gene_flow = capture_contact_study(&b, &auth, ctx, IsolationContactKind::GeneFlow, 110);
    let current_gene_flow = current_contact_study(
        &b,
        &gene_flow,
        &auth,
        ctx,
        IsolationContactKind::GeneFlow,
        110,
    );
    let contradicted_by_gene_flow = ReproductiveIsolationEvidence::capture(
        &current_design,
        vec![
            IsolationStudyEvidenceInput {
                unit_id: IsolationStudyUnitId::new("study-a").unwrap(),
                study: &current_barrier,
            },
            IsolationStudyEvidenceInput {
                unit_id: IsolationStudyUnitId::new("study-b").unwrap(),
                study: &current_gene_flow,
            },
        ],
    )
    .unwrap();
    assert_eq!(
        contradicted_by_gene_flow.status,
        ReproductiveIsolationStatus::Contradicted
    );
    assert_eq!(
        contradicted_by_gene_flow
            .barrier_profile
            .realized_gene_flow_observations,
        1
    );
}

#[test]
fn repeated_no_contact_and_unknown_fertility_are_insufficient_not_isolation() {
    let ctx = context(152);
    let auth = authorities(172);
    let a = contact_case(&auth, ctx, "ins-a", 50);
    let b = contact_case(&auth, ctx, "ins-b", 60);
    let current_a_design = current_contact_design(&a, &auth, ctx);
    let current_b_design = current_contact_design(&b, &auth, ctx);
    let design = declare_isolation_design(&current_a_design, &current_b_design, 32);
    let current_design = current_isolation_design(
        &design,
        &current_a_design,
        &current_b_design,
        32,
    );

    let no_contact_a = capture_contact_study(&a, &auth, ctx, IsolationContactKind::NoContact, 130);
    let no_contact_b = capture_contact_study(&b, &auth, ctx, IsolationContactKind::NoContact, 140);
    let current_a = current_contact_study(
        &a,
        &no_contact_a,
        &auth,
        ctx,
        IsolationContactKind::NoContact,
        130,
    );
    let current_b = current_contact_study(
        &b,
        &no_contact_b,
        &auth,
        ctx,
        IsolationContactKind::NoContact,
        140,
    );
    let no_contact_evidence = ReproductiveIsolationEvidence::capture(
        &current_design,
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
    assert_eq!(
        no_contact_evidence.status,
        ReproductiveIsolationStatus::InsufficientEvidence
    );
    assert_eq!(no_contact_evidence.barrier_profile.observed_contact_opportunities, 0);

    let barrier = capture_contact_study(&a, &auth, ctx, IsolationContactKind::Barrier, 150);
    let unknown = capture_contact_study(
        &b,
        &auth,
        ctx,
        IsolationContactKind::FertilityUnknown,
        170,
    );
    let current_barrier = current_contact_study(
        &a,
        &barrier,
        &auth,
        ctx,
        IsolationContactKind::Barrier,
        150,
    );
    let current_unknown = current_contact_study(
        &b,
        &unknown,
        &auth,
        ctx,
        IsolationContactKind::FertilityUnknown,
        170,
    );
    let unknown_evidence = ReproductiveIsolationEvidence::capture(
        &current_design,
        vec![
            IsolationStudyEvidenceInput {
                unit_id: IsolationStudyUnitId::new("study-a").unwrap(),
                study: &current_barrier,
            },
            IsolationStudyEvidenceInput {
                unit_id: IsolationStudyUnitId::new("study-b").unwrap(),
                study: &current_unknown,
            },
        ],
    )
    .unwrap();
    assert_eq!(
        unknown_evidence.status,
        ReproductiveIsolationStatus::InsufficientEvidence
    );
    assert_eq!(
        unknown_evidence
            .barrier_profile
            .viable_hybrids_fertility_unknown,
        1
    );
}

#[test]
fn adequate_contact_concentrated_in_one_study_is_not_supported() {
    let ctx = context(153);
    let auth = authorities(173);
    let a = contact_case(&auth, ctx, "null-a", 70);
    let b = contact_case(&auth, ctx, "null-b", 80);
    let current_a_design = current_contact_design(&a, &auth, ctx);
    let current_b_design = current_contact_design(&b, &auth, ctx);
    let design = declare_isolation_design(&current_a_design, &current_b_design, 33);
    let current_design = current_isolation_design(
        &design,
        &current_a_design,
        &current_b_design,
        33,
    );

    let barrier = capture_contact_study(&a, &auth, ctx, IsolationContactKind::Barrier, 190);
    let no_contact = capture_contact_study(&b, &auth, ctx, IsolationContactKind::NoContact, 210);
    let current_barrier = current_contact_study(
        &a,
        &barrier,
        &auth,
        ctx,
        IsolationContactKind::Barrier,
        190,
    );
    let current_no_contact = current_contact_study(
        &b,
        &no_contact,
        &auth,
        ctx,
        IsolationContactKind::NoContact,
        210,
    );
    let evidence = ReproductiveIsolationEvidence::capture(
        &current_design,
        vec![
            IsolationStudyEvidenceInput {
                unit_id: IsolationStudyUnitId::new("study-a").unwrap(),
                study: &current_barrier,
            },
            IsolationStudyEvidenceInput {
                unit_id: IsolationStudyUnitId::new("study-b").unwrap(),
                study: &current_no_contact,
            },
        ],
    )
    .unwrap();
    assert_eq!(evidence.barrier_profile.observed_contact_opportunities, 2);
    assert_eq!(evidence.barrier_profile.barrier_supporting_studies, 1);
    assert_eq!(evidence.status, ReproductiveIsolationStatus::NotSupported);
}

#[test]
fn duplicated_contact_design_and_one_study_cannot_preregister_isolation() {
    let ctx = context(154);
    let auth = authorities(174);
    let a = contact_case(&auth, ctx, "dup-a", 90);
    let current_a = current_contact_design(&a, &auth, ctx);

    assert!(matches!(
        ReproductiveIsolationDesign::declare(
            ReproductiveIsolationDesignId::new("one-study").unwrap(),
            vec![IsolationStudyDesignInput {
                unit_id: IsolationStudyUnitId::new("study-a").unwrap(),
                contact_design: &current_a,
                independence_evidence: isolation_rule_authority("one", 220),
            }],
            2,
            2,
            3,
            IsolationContextCompatibility::RequireSharedContextPolicy,
            isolation_rule_authority("cohort", 221),
            isolation_rule_authority("barrier", 222),
        ),
        Err(ReproductiveIsolationDesignError::InsufficientDeclaredStudies)
            | Err(ReproductiveIsolationDesignError::InvalidBarrierStudyThreshold)
    ));

    assert!(matches!(
        ReproductiveIsolationDesign::declare(
            ReproductiveIsolationDesignId::new("duplicate-study").unwrap(),
            vec![
                IsolationStudyDesignInput {
                    unit_id: IsolationStudyUnitId::new("study-a").unwrap(),
                    contact_design: &current_a,
                    independence_evidence: isolation_rule_authority("a", 223),
                },
                IsolationStudyDesignInput {
                    unit_id: IsolationStudyUnitId::new("study-b").unwrap(),
                    contact_design: &current_a,
                    independence_evidence: isolation_rule_authority("b", 224),
                },
            ],
            2,
            2,
            3,
            IsolationContextCompatibility::RequireSharedContextPolicy,
            isolation_rule_authority("cohort", 225),
            isolation_rule_authority("barrier", 226),
        ),
        Err(ReproductiveIsolationDesignError::DuplicateContactDesignDigest)
    ));
}

#[test]
fn omitted_study_and_serialized_profile_or_status_tampering_fail_closed() {
    let ctx = context(155);
    let auth = authorities(175);
    let a = contact_case(&auth, ctx, "tamper-a", 100);
    let b = contact_case(&auth, ctx, "tamper-b", 110);
    let current_a_design = current_contact_design(&a, &auth, ctx);
    let current_b_design = current_contact_design(&b, &auth, ctx);
    let design = declare_isolation_design(&current_a_design, &current_b_design, 34);
    let current_design = current_isolation_design(
        &design,
        &current_a_design,
        &current_b_design,
        34,
    );
    let study_a = capture_contact_study(&a, &auth, ctx, IsolationContactKind::Barrier, 230);
    let study_b = capture_contact_study(&b, &auth, ctx, IsolationContactKind::Barrier, 240);
    let current_a = current_contact_study(
        &a,
        &study_a,
        &auth,
        ctx,
        IsolationContactKind::Barrier,
        230,
    );
    let current_b = current_contact_study(
        &b,
        &study_b,
        &auth,
        ctx,
        IsolationContactKind::Barrier,
        240,
    );

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

    let evidence = ReproductiveIsolationEvidence::capture(
        &current_design,
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

    let mut profile_value = serde_json::to_value(&evidence).unwrap();
    profile_value["barrier_profile"]["pairing_barriers"] = serde_json::json!(99);
    let changed_profile: ReproductiveIsolationEvidence = serde_json::from_value(profile_value).unwrap();
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

fn collect_isolation_keys(value: &serde_json::Value, keys: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(object) => {
            for (key, nested) in object {
                keys.push(key.clone());
                collect_isolation_keys(nested, keys);
            }
        }
        serde_json::Value::Array(values) => {
            for nested in values {
                collect_isolation_keys(nested, keys);
            }
        }
        _ => {}
    }
}

#[test]
fn reproductive_isolation_wire_shape_does_not_claim_species_or_speciation() {
    let ctx = context(156);
    let auth = authorities(176);
    let a = contact_case(&auth, ctx, "wire-a", 120);
    let b = contact_case(&auth, ctx, "wire-b", 130);
    let current_a_design = current_contact_design(&a, &auth, ctx);
    let current_b_design = current_contact_design(&b, &auth, ctx);
    let design = declare_isolation_design(&current_a_design, &current_b_design, 35);
    let current_design = current_isolation_design(
        &design,
        &current_a_design,
        &current_b_design,
        35,
    );
    let study_a = capture_contact_study(&a, &auth, ctx, IsolationContactKind::Barrier, 1);
    let study_b = capture_contact_study(&b, &auth, ctx, IsolationContactKind::Barrier, 21);
    let current_a = current_contact_study(
        &a,
        &study_a,
        &auth,
        ctx,
        IsolationContactKind::Barrier,
        1,
    );
    let current_b = current_contact_study(
        &b,
        &study_b,
        &auth,
        ctx,
        IsolationContactKind::Barrier,
        21,
    );
    let evidence = ReproductiveIsolationEvidence::capture(
        &current_design,
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
    let value = serde_json::to_value(evidence).unwrap();
    let mut keys = Vec::new();
    collect_isolation_keys(&value, &mut keys);
    for forbidden in ["species", "species_status", "speciation", "speciation_event"] {
        assert!(!keys.iter().any(|key| key == forbidden));
    }
}
