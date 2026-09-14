include!("current_robustness_design_v1.rs");

#[test]
fn mismatched_history_design_subject_fails_before_pair_reasoning() {
    let models = independent_models();
    let mut forged_value = serde_json::to_value(&models[1]).unwrap();
    forged_value["lineage_history_design_digest"] = serde_json::json!(vec![231u8; 32]);
    let forged_general: SpeciesModelDesignRecord = serde_json::from_value(forged_value).unwrap();
    let forged_models = vec![models[0].clone(), forged_general];

    assert!(matches!(
        CurrentCrossModelRobustnessDesign::declare(
            CurrentRobustnessDesignId::new("history-substitution").unwrap(),
            subject(&models, 220),
            forged_models,
            [],
            policy(
                [
                    FaultDomainDimension::EvidenceSourceLineage,
                    FaultDomainDimension::UpstreamEvidenceAuthorityLineage,
                ],
                221,
            ),
            [],
            2,
            2,
            MissingModelPolicy::ReportInsufficientCoverage,
        ),
        Err(RobustnessDesignError::EvidenceSubjectMismatch)
    ));
}

#[test]
fn restored_noncanonical_model_order_cannot_canonicalize() {
    let raw = design_with(
        independent_models(),
        SpeciesConceptRelationKind::OperationalCriterionWithin,
        policy(
            [
                FaultDomainDimension::EvidenceSourceLineage,
                FaultDomainDimension::UpstreamEvidenceAuthorityLineage,
            ],
            222,
        ),
        PairwiseFaultDomainDisposition::KnownDependent,
    )
    .unwrap();
    let mut value = serde_json::to_value(raw).unwrap();
    value["models"].as_array_mut().unwrap().reverse();
    let restored: CurrentCrossModelRobustnessDesign = serde_json::from_value(value).unwrap();
    assert!(matches!(
        restored.canonical_digest(),
        Err(RobustnessDesignError::NonCanonicalModelOrder)
    ));
}

#[test]
fn restored_semantic_dependency_summary_cannot_self_authorize() {
    let raw = design_with(
        independent_models(),
        SpeciesConceptRelationKind::OperationalCriterionWithin,
        policy(
            [
                FaultDomainDimension::EvidenceSourceLineage,
                FaultDomainDimension::UpstreamEvidenceAuthorityLineage,
            ],
            223,
        ),
        PairwiseFaultDomainDisposition::KnownDependent,
    )
    .unwrap();
    let mut value = serde_json::to_value(raw).unwrap();
    value["semantic_relations"][0]["dependency_class"] =
        serde_json::json!("PotentiallyNonNested");
    let restored: CurrentCrossModelRobustnessDesign = serde_json::from_value(value).unwrap();
    assert!(matches!(
        restored.canonical_digest(),
        Err(RobustnessDesignError::RelationDependencyInvariant)
    ));
}

#[test]
fn restored_noncanonical_fault_policy_order_is_rejected() {
    let p = policy(
        [
            FaultDomainDimension::QualificationProcess,
            FaultDomainDimension::EvidenceSourceLineage,
            FaultDomainDimension::UpstreamEvidenceAuthorityLineage,
        ],
        224,
    );
    let mut value = serde_json::to_value(p).unwrap();
    value["required_distinct_dimensions"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let restored: FaultDomainIndependencePolicy = serde_json::from_value(value).unwrap();
    assert!(matches!(
        restored.canonical_digest(),
        Err(RobustnessDesignError::NonCanonicalFaultDomainDimensions)
    ));
}

#[test]
fn restored_fault_policy_rule_tamper_is_rejected() {
    let p = policy(
        [
            FaultDomainDimension::EvidenceSourceLineage,
            FaultDomainDimension::UpstreamEvidenceAuthorityLineage,
        ],
        225,
    );
    let mut value = serde_json::to_value(p).unwrap();
    value["rule_authority"]["revision"] = serde_json::json!(2);
    let restored: FaultDomainIndependencePolicy = serde_json::from_value(value).unwrap();
    assert!(matches!(
        restored.canonical_digest(),
        Err(RobustnessDesignError::FaultPolicyRuleMismatch)
    ));
}

#[test]
fn duplicate_fault_dimensions_are_rejected_at_declaration() {
    assert!(matches!(
        FaultDomainIndependencePolicy::declare(
            [
                FaultDomainDimension::EvidenceSourceLineage,
                FaultDomainDimension::EvidenceSourceLineage,
                FaultDomainDimension::UpstreamEvidenceAuthorityLineage,
            ],
            auth("duplicate-policy", 226),
        ),
        Err(RobustnessDesignError::DuplicateFaultDomainDimension)
    ));
}

#[test]
fn evidence_universe_qualification_drift_changes_design_identity() {
    let models = independent_models();
    let p = policy(
        [
            FaultDomainDimension::EvidenceSourceLineage,
            FaultDomainDimension::UpstreamEvidenceAuthorityLineage,
        ],
        227,
    );
    let relation = relation_record(
        &models[0],
        &models[1],
        SpeciesConceptRelationKind::OperationalCriterionWithin,
        228,
    );
    let assessment = PairwiseFaultDomainAssessment::new(
        &p,
        &models[0].fault_profile,
        &models[1].fault_profile,
        PairwiseFaultDomainDisposition::KnownDependent,
        auth("pair", 229),
    )
    .unwrap();

    let first = CurrentCrossModelRobustnessDesign::declare(
        CurrentRobustnessDesignId::new("universe-drift").unwrap(),
        subject(&models, 230),
        models.clone(),
        [relation.clone()],
        p.clone(),
        [assessment.clone()],
        2,
        2,
        MissingModelPolicy::ReportInsufficientCoverage,
    )
    .unwrap();
    let second = CurrentCrossModelRobustnessDesign::declare(
        CurrentRobustnessDesignId::new("universe-drift").unwrap(),
        subject(&models, 231),
        models,
        [relation],
        p,
        [assessment],
        2,
        2,
        MissingModelPolicy::ReportInsufficientCoverage,
    )
    .unwrap();
    assert_ne!(first.canonical_digest().unwrap(), second.canonical_digest().unwrap());
}

fn unknown_relation_record(
    first: &SpeciesModelDesignRecord,
    second: &SpeciesModelDesignRecord,
    byte: u8,
) -> SemanticRelationRecord {
    let first_identity = first.conceptual_identity().clone();
    let second_identity = second.conceptual_identity().clone();
    let scope = SpeciesConceptRelationScopeRef::new(
        SpeciesConceptRelationScopeId::new("e2a-unknown-relation-scope").unwrap(),
        1,
        SpeciesConceptRelationScopeDigest::new([byte; 32]),
    )
    .unwrap();
    let raw_design = SpeciesConceptRelationDesign::declare(
        SpeciesConceptRelationDesignId::new("e2a-unknown-relation").unwrap(),
        first_identity.clone(),
        second_identity.clone(),
        scope.clone(),
        auth("unknown-relation-protocol", byte.wrapping_add(1)),
        RelationMissingPolicy::ReportUnknownOrUnavailable,
    )
    .unwrap();
    let current_design = ValidatedSpeciesConceptRelationDesign::validate_current(
        &raw_design,
        first_identity,
        second_identity,
        scope,
        auth("unknown-relation-protocol", byte.wrapping_add(1)),
        RelationMissingPolicy::ReportUnknownOrUnavailable,
    )
    .unwrap();
    let assessment = SpeciesConceptRelationAssessment::UnknownOrUnqualified {
        evidence_authority: auth("unknown-relation-evidence", byte.wrapping_add(2)),
    };
    let evidence = SpeciesConceptRelationEvidence::evaluate(&current_design, assessment.clone())
        .unwrap();
    let current = ValidatedSpeciesConceptRelationEvidence::validate_current(
        &evidence,
        &current_design,
        assessment,
    )
    .unwrap();
    SemanticRelationRecord::from_current(&current).unwrap()
}

#[test]
fn unknown_relation_never_upgrades_pair_eligibility() {
    let models = independent_models();
    let first = models[0].conceptual_identity().clone();
    let second = models[1].conceptual_identity().clone();
    let p = policy(
        [
            FaultDomainDimension::EvidenceSourceLineage,
            FaultDomainDimension::UpstreamEvidenceAuthorityLineage,
        ],
        232,
    );
    let assessment = PairwiseFaultDomainAssessment::new(
        &p,
        &models[0].fault_profile,
        &models[1].fault_profile,
        PairwiseFaultDomainDisposition::QualifiedSufficientlyDistinct,
        auth("pair", 233),
    )
    .unwrap();
    let design = CurrentCrossModelRobustnessDesign::declare(
        CurrentRobustnessDesignId::new("unknown-relation-design").unwrap(),
        subject(&models, 234),
        models.clone(),
        [unknown_relation_record(&models[0], &models[1], 235)],
        p,
        [assessment],
        2,
        2,
        MissingModelPolicy::ReportInsufficientCoverage,
    )
    .unwrap();
    assert!(!design.pair_is_eligible_for_independent_coverage(&first, &second));
}
