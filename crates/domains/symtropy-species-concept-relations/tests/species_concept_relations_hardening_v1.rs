include!("species_concept_relations_v1.rs");

#[test]
fn restored_zero_family_version_is_rejected() {
    let a = identity("concept-a", 80);
    let b = identity("concept-b", 81);
    let design = design(
        a,
        b,
        RelationMissingPolicy::ReportUnknownOrUnavailable,
    );
    let mut value = serde_json::to_value(design).unwrap();
    value["left"]["family_version"] = serde_json::json!(0);
    let restored: SpeciesConceptRelationDesign = serde_json::from_value(value).unwrap();
    assert!(matches!(
        restored.canonical_digest(),
        Err(RelationError::ZeroConceptFamilyVersion)
    ));
}

#[test]
fn restored_endpoint_reordering_is_not_accepted_as_canonical_bytes() {
    let a = identity("concept-a", 82);
    let b = identity("concept-b", 83);
    let design = design(
        a,
        b,
        RelationMissingPolicy::ReportUnknownOrUnavailable,
    );
    let mut value = serde_json::to_value(design).unwrap();
    let left = value["left"].clone();
    let right = value["right"].clone();
    value["left"] = right;
    value["right"] = left;
    let restored: SpeciesConceptRelationDesign = serde_json::from_value(value).unwrap();
    assert!(matches!(
        restored.canonical_digest(),
        Err(RelationError::NonCanonicalEndpoints)
    ));
}

#[test]
fn restored_zero_scope_or_protocol_revision_fails_closed() {
    let a = identity("concept-a", 84);
    let b = identity("concept-b", 85);
    let design = design(
        a.clone(),
        b.clone(),
        RelationMissingPolicy::ReportUnknownOrUnavailable,
    );

    let mut scope_value = serde_json::to_value(&design).unwrap();
    scope_value["scope"]["revision"] = serde_json::json!(0);
    let restored: SpeciesConceptRelationDesign = serde_json::from_value(scope_value).unwrap();
    assert!(matches!(
        restored.canonical_digest(),
        Err(RelationError::ZeroRevision("relation_scope_revision"))
    ));

    let mut protocol_value = serde_json::to_value(design).unwrap();
    protocol_value["protocol_authority"]["revision"] = serde_json::json!(0);
    let restored: SpeciesConceptRelationDesign = serde_json::from_value(protocol_value).unwrap();
    assert!(matches!(
        restored.canonical_digest(),
        Err(RelationError::ZeroRevision("relation_protocol_revision"))
    ));
}

#[test]
fn symmetric_and_directional_relation_shapes_are_enforced() {
    assert!(matches!(
        SpeciesConceptRelationAssertion::new(
            SpeciesConceptRelationKind::Generalizes,
            SpeciesConceptRelationDirection::Symmetric,
        ),
        Err(RelationError::DirectionalRelationNeedsDirection(
            SpeciesConceptRelationKind::Generalizes
        ))
    ));
    assert!(matches!(
        SpeciesConceptRelationAssertion::new(
            SpeciesConceptRelationKind::PartiallyOverlaps,
            SpeciesConceptRelationDirection::LeftToRight,
        ),
        Err(RelationError::SymmetricRelationCannotBeDirected(
            SpeciesConceptRelationKind::PartiallyOverlaps
        ))
    ));
}

#[test]
fn embedded_design_digest_tampering_is_detected_locally() {
    let a = identity("concept-a", 86);
    let b = identity("concept-b", 87);
    let design = design(
        a.clone(),
        b.clone(),
        RelationMissingPolicy::ReportUnknownOrUnavailable,
    );
    let current = current_design(
        &design,
        a,
        b,
        RelationMissingPolicy::ReportUnknownOrUnavailable,
    );
    let assertion = SpeciesConceptRelationAssertion::new(
        SpeciesConceptRelationKind::PartiallyOverlaps,
        SpeciesConceptRelationDirection::Symmetric,
    )
    .unwrap();
    let evidence = SpeciesConceptRelationEvidence::evaluate(
        &current,
        qualified_assessment(&design, assertion, 88),
    )
    .unwrap();

    let mut value = serde_json::to_value(evidence).unwrap();
    value["design_digest"][0] = serde_json::json!(255);
    let restored: SpeciesConceptRelationEvidence = serde_json::from_value(value).unwrap();
    assert!(matches!(
        restored.canonical_digest(),
        Err(RelationError::DesignDigestMismatch)
    ));
}

#[test]
fn restored_zero_relation_qualification_revision_is_rejected() {
    let a = identity("concept-a", 89);
    let b = identity("concept-b", 90);
    let design = design(
        a.clone(),
        b.clone(),
        RelationMissingPolicy::ReportUnknownOrUnavailable,
    );
    let current = current_design(
        &design,
        a,
        b,
        RelationMissingPolicy::ReportUnknownOrUnavailable,
    );
    let assertion = SpeciesConceptRelationAssertion::new(
        SpeciesConceptRelationKind::PartiallyOverlaps,
        SpeciesConceptRelationDirection::Symmetric,
    )
    .unwrap();
    let evidence = SpeciesConceptRelationEvidence::evaluate(
        &current,
        qualified_assessment(&design, assertion, 91),
    )
    .unwrap();

    let mut value = serde_json::to_value(evidence).unwrap();
    value["assessment"]["Qualified"]["qualification_authority"]["revision"] =
        serde_json::json!(0);
    let restored: SpeciesConceptRelationEvidence = serde_json::from_value(value).unwrap();
    assert!(matches!(
        restored.canonical_digest(),
        Err(RelationError::ZeroRevision("relation_qualification_revision"))
    ));
}

#[test]
fn relation_rule_tampering_fails_local_design_validation() {
    let a = identity("concept-a", 92);
    let b = identity("concept-b", 93);
    let design = design(
        a,
        b,
        RelationMissingPolicy::ReportUnknownOrUnavailable,
    );
    let mut value = serde_json::to_value(design).unwrap();
    value["relation_rule_authority"]["revision"] = serde_json::json!(2);
    let restored: SpeciesConceptRelationDesign = serde_json::from_value(value).unwrap();
    assert!(matches!(
        restored.canonical_digest(),
        Err(RelationError::RelationRuleMismatch)
    ));
}

#[test]
fn locally_canonical_semantic_relabeling_cannot_regain_current_authority() {
    let a = identity("concept-a", 94);
    let b = identity("concept-b", 95);
    let design = design(
        a.clone(),
        b.clone(),
        RelationMissingPolicy::ReportUnknownOrUnavailable,
    );
    let current = current_design(
        &design,
        a,
        b,
        RelationMissingPolicy::ReportUnknownOrUnavailable,
    );
    let original_assertion = SpeciesConceptRelationAssertion::new(
        SpeciesConceptRelationKind::PartiallyOverlaps,
        SpeciesConceptRelationDirection::Symmetric,
    )
    .unwrap();
    let original_assessment = qualified_assessment(&design, original_assertion, 96);
    let evidence = SpeciesConceptRelationEvidence::evaluate(&current, original_assessment.clone())
        .unwrap();

    let mut value = serde_json::to_value(&evidence).unwrap();
    value["assessment"]["Qualified"]["assertion"]["kind"] =
        serde_json::json!("EquivalentSemanticTarget");
    let relabeled: SpeciesConceptRelationEvidence = serde_json::from_value(value).unwrap();

    // The mapping digest is an opaque qualified semantic artifact, so this forms a different
    // locally canonical representation. Local hashing is intentionally not scientific replay.
    assert_ne!(
        relabeled.canonical_digest().unwrap(),
        evidence.canonical_digest().unwrap()
    );
    assert!(matches!(
        ValidatedSpeciesConceptRelationEvidence::validate_current(
            &relabeled,
            &current,
            original_assessment,
        ),
        Err(RelationError::EvidenceReplayMismatch)
    ));
}
