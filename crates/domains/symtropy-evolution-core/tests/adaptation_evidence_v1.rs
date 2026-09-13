use symtropy_evolution_core::{
    AdaptationEvidence, AdaptationEvidenceStatus, AdaptationReplicationDesign,
    AdaptationReplicationDesignError, AdaptationReplicationDesignId, EvolutionExperimentId,
    ReplicationContextCompatibility, ReplicationDesignUnitInput, ReplicationStudyInput,
    ReplicationUnitId, ValidatedAdaptationEvidence, ValidatedAdaptationReplicationDesign,
};

include!("heritable_response_study_v1_body.inc");

struct OwnedB2Unit {
    selection: OwnedB1Selection,
    authorities: B1Authorities,
    design: HeritableResponseStudyDesign,
}

#[derive(Clone, Copy)]
enum B2StudyKind {
    Directional,
    Null,
    Reversed,
    HereditaryOnly,
    TraitOnly,
    Insufficient,
}

fn owned_b2_unit(offset: u8, authority_seed: u8) -> OwnedB2Unit {
    let selection = owned_b1_selection(false, offset);
    let authorities = b1_authorities(authority_seed);
    let design = {
        let current_selection = current_b1_selection(&selection);
        b1_design(&current_selection, &authorities)
    };
    OwnedB2Unit {
        selection,
        authorities,
        design,
    }
}

fn current_b2_design<'a>(unit: &'a OwnedB2Unit) -> ValidatedHeritableResponseStudyDesign<'a> {
    let selection = current_b1_selection(&unit.selection);
    current_b1_design(&unit.design, &selection, &unit.authorities)
}

fn b2_inputs(
    unit: &OwnedB2Unit,
    selection: &ValidatedModelSpecificSelectionEstimate<'_>,
    kind: B2StudyKind,
) -> Vec<GenerationResponseEvidenceInput> {
    match kind {
        B2StudyKind::Directional => directional_inputs(
            &unit.selection,
            selection,
            &unit.design,
        ),
        B2StudyKind::Null => vec![
            b1_input(
                &unit.selection,
                selection,
                &unit.design,
                0,
                2,
                GenerationTraitDirection::Neutral,
            ),
            b1_input(
                &unit.selection,
                selection,
                &unit.design,
                1,
                2,
                GenerationTraitDirection::Neutral,
            ),
            b1_input(
                &unit.selection,
                selection,
                &unit.design,
                2,
                2,
                GenerationTraitDirection::Neutral,
            ),
        ],
        B2StudyKind::Reversed => vec![
            b1_input(
                &unit.selection,
                selection,
                &unit.design,
                0,
                1,
                GenerationTraitDirection::ComparisonFavored,
            ),
            b1_input(
                &unit.selection,
                selection,
                &unit.design,
                1,
                2,
                GenerationTraitDirection::ComparisonFavored,
            ),
            b1_input(
                &unit.selection,
                selection,
                &unit.design,
                2,
                3,
                GenerationTraitDirection::ComparisonFavored,
            ),
        ],
        B2StudyKind::HereditaryOnly => vec![
            b1_input(
                &unit.selection,
                selection,
                &unit.design,
                0,
                3,
                GenerationTraitDirection::Neutral,
            ),
            b1_input(
                &unit.selection,
                selection,
                &unit.design,
                1,
                2,
                GenerationTraitDirection::Neutral,
            ),
            b1_input(
                &unit.selection,
                selection,
                &unit.design,
                2,
                1,
                GenerationTraitDirection::Neutral,
            ),
        ],
        B2StudyKind::TraitOnly => vec![
            b1_input(
                &unit.selection,
                selection,
                &unit.design,
                0,
                2,
                GenerationTraitDirection::ReferenceFavored,
            ),
            b1_input(
                &unit.selection,
                selection,
                &unit.design,
                1,
                2,
                GenerationTraitDirection::ReferenceFavored,
            ),
            b1_input(
                &unit.selection,
                selection,
                &unit.design,
                2,
                2,
                GenerationTraitDirection::ReferenceFavored,
            ),
        ],
        B2StudyKind::Insufficient => {
            let mut inputs = directional_inputs(
                &unit.selection,
                selection,
                &unit.design,
            );
            inputs[1].transmission = TransmissionEvidenceStatus::Unavailable {
                reason: b1_authority("b2-missing-transmission", 211),
            };
            inputs
        }
    }
}

fn capture_b2_study(unit: &OwnedB2Unit, kind: B2StudyKind) -> HeritableResponseStudy {
    let selection = current_b1_selection(&unit.selection);
    let design = current_b1_design(&unit.design, &selection, &unit.authorities);
    HeritableResponseStudy::capture(
        &design,
        &selection,
        b2_inputs(unit, &selection, kind),
    )
    .unwrap()
}

fn current_b2_study<'a>(
    unit: &OwnedB2Unit,
    study: &'a HeritableResponseStudy,
    kind: B2StudyKind,
) -> ValidatedHeritableResponseStudy<'a> {
    let selection = current_b1_selection(&unit.selection);
    let design = current_b1_design(&unit.design, &selection, &unit.authorities);
    ValidatedHeritableResponseStudy::validate_current(
        study,
        &design,
        &selection,
        b2_inputs(unit, &selection, kind),
    )
    .unwrap()
}

fn b2_authority(label: &str, byte: u8) -> AnalysisAuthorityRef {
    AnalysisAuthorityRef::new(
        AnalysisMethodId::new(label).unwrap(),
        1,
        AnalysisContentDigest::new([byte; 32]),
    )
}

#[derive(Clone)]
struct B2Authorities {
    context: ReplicationContextCompatibility,
    model_compatibility: AnalysisAuthorityRef,
    independence_rule: AnalysisAuthorityRef,
    decision_rule: AnalysisAuthorityRef,
}

fn b2_authorities(seed: u8) -> B2Authorities {
    B2Authorities {
        context: ReplicationContextCompatibility::AllowDeclaredVariation {
            authority: b2_authority("b2-context-compatibility", seed),
        },
        model_compatibility: b2_authority(
            "b2-model-compatibility",
            seed.wrapping_add(1),
        ),
        independence_rule: b2_authority(
            "b2-independence-rule",
            seed.wrapping_add(2),
        ),
        decision_rule: b2_authority(
            "b2-decision-rule",
            seed.wrapping_add(3),
        ),
    }
}

fn independence(unit: &str, byte: u8) -> AnalysisAuthorityRef {
    b2_authority(&format!("b2-independence-{unit}"), byte)
}

fn declare_two_unit_design(
    first: &ValidatedHeritableResponseStudyDesign<'_>,
    second: &ValidatedHeritableResponseStudyDesign<'_>,
    authorities: &B2Authorities,
) -> AdaptationReplicationDesign {
    AdaptationReplicationDesign::declare(
        AdaptationReplicationDesignId::new("two-unit-adaptation-v1").unwrap(),
        vec![
            ReplicationDesignUnitInput {
                unit_id: ReplicationUnitId::new("replicate-b").unwrap(),
                study_design: second,
                independence_evidence: independence("b", 182),
            },
            ReplicationDesignUnitInput {
                unit_id: ReplicationUnitId::new("replicate-a").unwrap(),
                study_design: first,
                independence_evidence: independence("a", 181),
            },
        ],
        2,
        3,
        authorities.context.clone(),
        authorities.model_compatibility.clone(),
        authorities.independence_rule.clone(),
        authorities.decision_rule.clone(),
    )
    .unwrap()
}

fn current_two_unit_design<'a>(
    design: &'a AdaptationReplicationDesign,
    first: &ValidatedHeritableResponseStudyDesign<'_>,
    second: &ValidatedHeritableResponseStudyDesign<'_>,
    authorities: &B2Authorities,
) -> ValidatedAdaptationReplicationDesign<'a> {
    ValidatedAdaptationReplicationDesign::validate_current(
        design,
        vec![
            ReplicationDesignUnitInput {
                unit_id: ReplicationUnitId::new("replicate-b").unwrap(),
                study_design: second,
                independence_evidence: independence("b", 182),
            },
            ReplicationDesignUnitInput {
                unit_id: ReplicationUnitId::new("replicate-a").unwrap(),
                study_design: first,
                independence_evidence: independence("a", 181),
            },
        ],
        2,
        3,
        authorities.context.clone(),
        authorities.model_compatibility.clone(),
        authorities.independence_rule.clone(),
        authorities.decision_rule.clone(),
    )
    .unwrap()
}

#[test]
fn two_current_directional_replications_support_adaptation_and_replay() {
    let a = owned_b2_unit(90, 120);
    let b = owned_b2_unit(91, 121);
    let design_a = current_b2_design(&a);
    let design_b = current_b2_design(&b);
    let authorities = b2_authorities(160);
    let design = declare_two_unit_design(&design_a, &design_b, &authorities);
    assert_eq!(
        design.units.iter().map(|unit| unit.unit_id.as_str()).collect::<Vec<_>>(),
        vec!["replicate-a", "replicate-b"]
    );
    assert_ne!(
        design.units[0].selection_model_digest,
        design.units[1].selection_model_digest
    );

    let current_design = current_two_unit_design(
        &design,
        &design_a,
        &design_b,
        &authorities,
    );
    let study_a = capture_b2_study(&a, B2StudyKind::Directional);
    let study_b = capture_b2_study(&b, B2StudyKind::Directional);
    let current_a = current_b2_study(&a, &study_a, B2StudyKind::Directional);
    let current_b = current_b2_study(&b, &study_b, B2StudyKind::Directional);

    let evidence = AdaptationEvidence::capture(
        &current_design,
        vec![
            ReplicationStudyInput {
                unit_id: ReplicationUnitId::new("replicate-b").unwrap(),
                study: &current_b,
            },
            ReplicationStudyInput {
                unit_id: ReplicationUnitId::new("replicate-a").unwrap(),
                study: &current_a,
            },
        ],
    )
    .unwrap();
    assert_eq!(evidence.status, AdaptationEvidenceStatus::Supported);
    assert_eq!(
        evidence.records.iter().map(|record| record.unit_id.as_str()).collect::<Vec<_>>(),
        vec!["replicate-a", "replicate-b"]
    );

    let restored_design: AdaptationReplicationDesign =
        serde_json::from_slice(&serde_json::to_vec(&design).unwrap()).unwrap();
    let restored_current_design = current_two_unit_design(
        &restored_design,
        &design_a,
        &design_b,
        &authorities,
    );
    let restored_evidence: AdaptationEvidence =
        serde_json::from_slice(&serde_json::to_vec(&evidence).unwrap()).unwrap();
    let validated = ValidatedAdaptationEvidence::validate_current(
        &restored_evidence,
        &restored_current_design,
        vec![
            ReplicationStudyInput {
                unit_id: ReplicationUnitId::new("replicate-a").unwrap(),
                study: &current_a,
            },
            ReplicationStudyInput {
                unit_id: ReplicationUnitId::new("replicate-b").unwrap(),
                study: &current_b,
            },
        ],
    )
    .unwrap();
    assert_eq!(validated.evidence_digest(), evidence.canonical_digest().unwrap());
}

#[test]
fn one_study_or_two_labels_for_one_b1_design_cannot_preregister_adaptation() {
    let a = owned_b2_unit(92, 122);
    let design_a = current_b2_design(&a);
    let authorities = b2_authorities(161);

    assert!(matches!(
        AdaptationReplicationDesign::declare(
            AdaptationReplicationDesignId::new("one-unit").unwrap(),
            vec![ReplicationDesignUnitInput {
                unit_id: ReplicationUnitId::new("replicate-a").unwrap(),
                study_design: &design_a,
                independence_evidence: independence("a", 183),
            }],
            2,
            3,
            authorities.context.clone(),
            authorities.model_compatibility.clone(),
            authorities.independence_rule.clone(),
            authorities.decision_rule.clone(),
        ),
        Err(AdaptationReplicationDesignError::InsufficientDeclaredReplications)
    ));

    assert!(matches!(
        AdaptationReplicationDesign::declare(
            AdaptationReplicationDesignId::new("duplicate-design").unwrap(),
            vec![
                ReplicationDesignUnitInput {
                    unit_id: ReplicationUnitId::new("replicate-a").unwrap(),
                    study_design: &design_a,
                    independence_evidence: independence("a", 184),
                },
                ReplicationDesignUnitInput {
                    unit_id: ReplicationUnitId::new("replicate-b").unwrap(),
                    study_design: &design_a,
                    independence_evidence: independence("b", 185),
                },
            ],
            2,
            3,
            authorities.context.clone(),
            authorities.model_compatibility.clone(),
            authorities.independence_rule.clone(),
            authorities.decision_rule.clone(),
        ),
        Err(AdaptationReplicationDesignError::DuplicateStudyDesignDigest)
    ));
}

#[test]
fn reversed_replication_contradicts_and_unavailable_replication_is_insufficient() {
    let a = owned_b2_unit(93, 123);
    let b = owned_b2_unit(94, 124);
    let design_a = current_b2_design(&a);
    let design_b = current_b2_design(&b);
    let authorities = b2_authorities(162);
    let design = declare_two_unit_design(&design_a, &design_b, &authorities);
    let current_design = current_two_unit_design(
        &design,
        &design_a,
        &design_b,
        &authorities,
    );

    let directional = capture_b2_study(&a, B2StudyKind::Directional);
    let reversed = capture_b2_study(&b, B2StudyKind::Reversed);
    let current_directional = current_b2_study(&a, &directional, B2StudyKind::Directional);
    let current_reversed = current_b2_study(&b, &reversed, B2StudyKind::Reversed);
    let contradicted = AdaptationEvidence::capture(
        &current_design,
        vec![
            ReplicationStudyInput {
                unit_id: ReplicationUnitId::new("replicate-a").unwrap(),
                study: &current_directional,
            },
            ReplicationStudyInput {
                unit_id: ReplicationUnitId::new("replicate-b").unwrap(),
                study: &current_reversed,
            },
        ],
    )
    .unwrap();
    assert_eq!(contradicted.status, AdaptationEvidenceStatus::Contradicted);

    let insufficient = capture_b2_study(&b, B2StudyKind::Insufficient);
    let current_insufficient = current_b2_study(&b, &insufficient, B2StudyKind::Insufficient);
    let insufficient_evidence = AdaptationEvidence::capture(
        &current_design,
        vec![
            ReplicationStudyInput {
                unit_id: ReplicationUnitId::new("replicate-a").unwrap(),
                study: &current_directional,
            },
            ReplicationStudyInput {
                unit_id: ReplicationUnitId::new("replicate-b").unwrap(),
                study: &current_insufficient,
            },
        ],
    )
    .unwrap();
    assert_eq!(
        insufficient_evidence.status,
        AdaptationEvidenceStatus::InsufficientEvidence
    );
}

#[test]
fn preregistered_threshold_can_support_with_visible_null_history_but_history_still_changes_identity() {
    let a = owned_b2_unit(95, 125);
    let b = owned_b2_unit(96, 126);
    let c = owned_b2_unit(97, 127);
    let design_a = current_b2_design(&a);
    let design_b = current_b2_design(&b);
    let design_c = current_b2_design(&c);
    let authorities = b2_authorities(163);

    let design = AdaptationReplicationDesign::declare(
        AdaptationReplicationDesignId::new("three-unit-threshold-two").unwrap(),
        vec![
            ReplicationDesignUnitInput {
                unit_id: ReplicationUnitId::new("replicate-c").unwrap(),
                study_design: &design_c,
                independence_evidence: independence("c", 188),
            },
            ReplicationDesignUnitInput {
                unit_id: ReplicationUnitId::new("replicate-a").unwrap(),
                study_design: &design_a,
                independence_evidence: independence("a", 186),
            },
            ReplicationDesignUnitInput {
                unit_id: ReplicationUnitId::new("replicate-b").unwrap(),
                study_design: &design_b,
                independence_evidence: independence("b", 187),
            },
        ],
        2,
        3,
        authorities.context.clone(),
        authorities.model_compatibility.clone(),
        authorities.independence_rule.clone(),
        authorities.decision_rule.clone(),
    )
    .unwrap();
    let current_design = ValidatedAdaptationReplicationDesign::validate_current(
        &design,
        vec![
            ReplicationDesignUnitInput {
                unit_id: ReplicationUnitId::new("replicate-a").unwrap(),
                study_design: &design_a,
                independence_evidence: independence("a", 186),
            },
            ReplicationDesignUnitInput {
                unit_id: ReplicationUnitId::new("replicate-b").unwrap(),
                study_design: &design_b,
                independence_evidence: independence("b", 187),
            },
            ReplicationDesignUnitInput {
                unit_id: ReplicationUnitId::new("replicate-c").unwrap(),
                study_design: &design_c,
                independence_evidence: independence("c", 188),
            },
        ],
        2,
        3,
        authorities.context.clone(),
        authorities.model_compatibility.clone(),
        authorities.independence_rule.clone(),
        authorities.decision_rule.clone(),
    )
    .unwrap();

    let study_a = capture_b2_study(&a, B2StudyKind::Directional);
    let study_b = capture_b2_study(&b, B2StudyKind::Directional);
    let null_c = capture_b2_study(&c, B2StudyKind::Null);
    let mismatch_c = capture_b2_study(&c, B2StudyKind::HereditaryOnly);
    let current_a = current_b2_study(&a, &study_a, B2StudyKind::Directional);
    let current_b = current_b2_study(&b, &study_b, B2StudyKind::Directional);
    let current_null_c = current_b2_study(&c, &null_c, B2StudyKind::Null);
    let current_mismatch_c = current_b2_study(&c, &mismatch_c, B2StudyKind::HereditaryOnly);

    let with_null = AdaptationEvidence::capture(
        &current_design,
        vec![
            ReplicationStudyInput {
                unit_id: ReplicationUnitId::new("replicate-a").unwrap(),
                study: &current_a,
            },
            ReplicationStudyInput {
                unit_id: ReplicationUnitId::new("replicate-b").unwrap(),
                study: &current_b,
            },
            ReplicationStudyInput {
                unit_id: ReplicationUnitId::new("replicate-c").unwrap(),
                study: &current_null_c,
            },
        ],
    )
    .unwrap();
    let with_mismatch = AdaptationEvidence::capture(
        &current_design,
        vec![
            ReplicationStudyInput {
                unit_id: ReplicationUnitId::new("replicate-a").unwrap(),
                study: &current_a,
            },
            ReplicationStudyInput {
                unit_id: ReplicationUnitId::new("replicate-b").unwrap(),
                study: &current_b,
            },
            ReplicationStudyInput {
                unit_id: ReplicationUnitId::new("replicate-c").unwrap(),
                study: &current_mismatch_c,
            },
        ],
    )
    .unwrap();

    assert_eq!(with_null.status, AdaptationEvidenceStatus::Supported);
    assert_eq!(with_mismatch.status, AdaptationEvidenceStatus::Supported);
    assert_ne!(
        with_null.canonical_digest().unwrap(),
        with_mismatch.canonical_digest().unwrap()
    );
}

#[test]
fn two_non_directional_complete_replications_are_not_supported() {
    let a = owned_b2_unit(98, 128);
    let b = owned_b2_unit(99, 129);
    let design_a = current_b2_design(&a);
    let design_b = current_b2_design(&b);
    let authorities = b2_authorities(164);
    let design = declare_two_unit_design(&design_a, &design_b, &authorities);
    let current_design = current_two_unit_design(
        &design,
        &design_a,
        &design_b,
        &authorities,
    );
    let null_a = capture_b2_study(&a, B2StudyKind::Null);
    let trait_only_b = capture_b2_study(&b, B2StudyKind::TraitOnly);
    let current_null_a = current_b2_study(&a, &null_a, B2StudyKind::Null);
    let current_trait_only_b = current_b2_study(&b, &trait_only_b, B2StudyKind::TraitOnly);
    let evidence = AdaptationEvidence::capture(
        &current_design,
        vec![
            ReplicationStudyInput {
                unit_id: ReplicationUnitId::new("replicate-a").unwrap(),
                study: &current_null_a,
            },
            ReplicationStudyInput {
                unit_id: ReplicationUnitId::new("replicate-b").unwrap(),
                study: &current_trait_only_b,
            },
        ],
    )
    .unwrap();
    assert_eq!(evidence.status, AdaptationEvidenceStatus::NotSupported);
}

#[test]
fn stale_model_compatibility_authority_cannot_regain_replication_design_authority() {
    let a = owned_b2_unit(100, 130);
    let b = owned_b2_unit(101, 131);
    let design_a = current_b2_design(&a);
    let design_b = current_b2_design(&b);
    let authorities = b2_authorities(165);
    let design = declare_two_unit_design(&design_a, &design_b, &authorities);
    let restored: AdaptationReplicationDesign =
        serde_json::from_slice(&serde_json::to_vec(&design).unwrap()).unwrap();

    assert!(matches!(
        ValidatedAdaptationReplicationDesign::validate_current(
            &restored,
            vec![
                ReplicationDesignUnitInput {
                    unit_id: ReplicationUnitId::new("replicate-a").unwrap(),
                    study_design: &design_a,
                    independence_evidence: independence("a", 181),
                },
                ReplicationDesignUnitInput {
                    unit_id: ReplicationUnitId::new("replicate-b").unwrap(),
                    study_design: &design_b,
                    independence_evidence: independence("b", 182),
                },
            ],
            2,
            3,
            authorities.context.clone(),
            b2_authority("changed-model-compatibility", 253),
            authorities.independence_rule.clone(),
            authorities.decision_rule.clone(),
        ),
        Err(AdaptationReplicationDesignError::ReplayMismatch)
    ));
}

fn collect_b2_keys(value: &serde_json::Value, keys: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(object) => {
            for (key, nested) in object {
                keys.push(key.clone());
                collect_b2_keys(nested, keys);
            }
        }
        serde_json::Value::Array(values) => {
            for nested in values {
                collect_b2_keys(nested, keys);
            }
        }
        _ => {}
    }
}

#[test]
fn adaptation_wire_shape_does_not_claim_reproductive_isolation_or_speciation() {
    let a = owned_b2_unit(102, 132);
    let b = owned_b2_unit(103, 133);
    let design_a = current_b2_design(&a);
    let design_b = current_b2_design(&b);
    let authorities = b2_authorities(166);
    let design = declare_two_unit_design(&design_a, &design_b, &authorities);
    let current_design = current_two_unit_design(
        &design,
        &design_a,
        &design_b,
        &authorities,
    );
    let study_a = capture_b2_study(&a, B2StudyKind::Directional);
    let study_b = capture_b2_study(&b, B2StudyKind::Directional);
    let current_a = current_b2_study(&a, &study_a, B2StudyKind::Directional);
    let current_b = current_b2_study(&b, &study_b, B2StudyKind::Directional);
    let evidence = AdaptationEvidence::capture(
        &current_design,
        vec![
            ReplicationStudyInput {
                unit_id: ReplicationUnitId::new("replicate-a").unwrap(),
                study: &current_a,
            },
            ReplicationStudyInput {
                unit_id: ReplicationUnitId::new("replicate-b").unwrap(),
                study: &current_b,
            },
        ],
    )
    .unwrap();

    let value = serde_json::to_value(evidence).unwrap();
    let mut keys = Vec::new();
    collect_b2_keys(&value, &mut keys);
    for forbidden in [
        "reproductive_isolation",
        "species",
        "species_status",
        "speciation",
        "ecological_superiority",
        "universal_beneficial",
    ] {
        assert!(!keys.iter().any(|key| key == forbidden));
    }
}
