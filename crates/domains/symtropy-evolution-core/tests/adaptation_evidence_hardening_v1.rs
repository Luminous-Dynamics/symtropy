include!("adaptation_evidence_v1.rs");

#[test]
fn omitted_declared_replication_fails_before_status_is_computed() {
    let a = owned_b2_unit(104, 134);
    let b = owned_b2_unit(105, 135);
    let design_a = current_b2_design(&a);
    let design_b = current_b2_design(&b);
    let authorities = b2_authorities(167);
    let design = declare_two_unit_design(&design_a, &design_b, &authorities);
    let current_design = current_two_unit_design(
        &design,
        &design_a,
        &design_b,
        &authorities,
    );
    let study_a = capture_b2_study(&a, B2StudyKind::Directional);
    let current_a = current_b2_study(&a, &study_a, B2StudyKind::Directional);

    assert!(matches!(
        AdaptationEvidence::capture(
            &current_design,
            vec![ReplicationStudyInput {
                unit_id: ReplicationUnitId::new("replicate-a").unwrap(),
                study: &current_a,
            }],
        ),
        Err(symtropy_evolution_core::AdaptationEvidenceError::IncompleteReplicationCoverage)
    ));
}

#[test]
fn one_current_b1_study_cannot_substitute_for_another_preregistered_unit() {
    let a = owned_b2_unit(106, 136);
    let b = owned_b2_unit(107, 137);
    let design_a = current_b2_design(&a);
    let design_b = current_b2_design(&b);
    let authorities = b2_authorities(168);
    let design = declare_two_unit_design(&design_a, &design_b, &authorities);
    let current_design = current_two_unit_design(
        &design,
        &design_a,
        &design_b,
        &authorities,
    );
    let study_a = capture_b2_study(&a, B2StudyKind::Directional);
    let current_a = current_b2_study(&a, &study_a, B2StudyKind::Directional);

    assert!(matches!(
        AdaptationEvidence::capture(
            &current_design,
            vec![
                ReplicationStudyInput {
                    unit_id: ReplicationUnitId::new("replicate-a").unwrap(),
                    study: &current_a,
                },
                ReplicationStudyInput {
                    unit_id: ReplicationUnitId::new("replicate-b").unwrap(),
                    study: &current_a,
                },
            ],
        ),
        Err(symtropy_evolution_core::AdaptationEvidenceError::StudyDesignMismatch)
    ));
}

#[test]
fn altered_serialized_adaptation_status_is_locally_invalid() {
    let a = owned_b2_unit(108, 138);
    let b = owned_b2_unit(109, 139);
    let design_a = current_b2_design(&a);
    let design_b = current_b2_design(&b);
    let authorities = b2_authorities(169);
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
    assert_eq!(evidence.status, AdaptationEvidenceStatus::Supported);

    let mut value = serde_json::to_value(evidence).unwrap();
    value["status"] = serde_json::json!("NotSupported");
    let altered: AdaptationEvidence = serde_json::from_value(value).unwrap();
    assert!(matches!(
        altered.canonical_digest(),
        Err(symtropy_evolution_core::AdaptationEvidenceError::StatusInvariant)
    ));
}

#[test]
fn independence_evidence_only_drift_changes_replication_design_identity() {
    let a = owned_b2_unit(110, 140);
    let b = owned_b2_unit(111, 141);
    let design_a = current_b2_design(&a);
    let design_b = current_b2_design(&b);
    let authorities = b2_authorities(170);

    let declare = |second_independence: AnalysisAuthorityRef| {
        AdaptationReplicationDesign::declare(
            AdaptationReplicationDesignId::new("independence-sensitive-design").unwrap(),
            vec![
                ReplicationDesignUnitInput {
                    unit_id: ReplicationUnitId::new("replicate-a").unwrap(),
                    study_design: &design_a,
                    independence_evidence: independence("a", 191),
                },
                ReplicationDesignUnitInput {
                    unit_id: ReplicationUnitId::new("replicate-b").unwrap(),
                    study_design: &design_b,
                    independence_evidence: second_independence,
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
    };

    let first = declare(independence("b", 192));
    let second = declare(independence("b", 193));
    assert_ne!(
        first.canonical_digest().unwrap(),
        second.canonical_digest().unwrap()
    );
}
