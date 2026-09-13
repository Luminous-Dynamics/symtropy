include!("species_concept_relations_v1.rs");

#[test]
fn refines_input_normalizes_to_reversed_generalizes() {
    let refines_left_to_right = SpeciesConceptRelationAssertion::new(
        SpeciesConceptRelationKind::Refines,
        SpeciesConceptRelationDirection::LeftToRight,
    )
    .unwrap();
    let canonical = SpeciesConceptRelationAssertion::new(
        SpeciesConceptRelationKind::Generalizes,
        SpeciesConceptRelationDirection::RightToLeft,
    )
    .unwrap();
    assert_eq!(refines_left_to_right, canonical);

    let refines_right_to_left = SpeciesConceptRelationAssertion::new(
        SpeciesConceptRelationKind::Refines,
        SpeciesConceptRelationDirection::RightToLeft,
    )
    .unwrap();
    let canonical = SpeciesConceptRelationAssertion::new(
        SpeciesConceptRelationKind::Generalizes,
        SpeciesConceptRelationDirection::LeftToRight,
    )
    .unwrap();
    assert_eq!(refines_right_to_left, canonical);
}

#[test]
fn refines_without_direction_still_fails() {
    assert!(matches!(
        SpeciesConceptRelationAssertion::new(
            SpeciesConceptRelationKind::Refines,
            SpeciesConceptRelationDirection::Symmetric,
        ),
        Err(RelationError::DirectionalRelationNeedsDirection(
            SpeciesConceptRelationKind::Refines
        ))
    ));
}

#[test]
fn raw_restored_refines_alias_is_noncanonical() {
    let a = identity("concept-a", 101);
    let b = identity("concept-b", 102);
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
    let canonical_assertion = SpeciesConceptRelationAssertion::new(
        SpeciesConceptRelationKind::Generalizes,
        SpeciesConceptRelationDirection::LeftToRight,
    )
    .unwrap();
    let evidence = SpeciesConceptRelationEvidence::evaluate(
        &current,
        qualified_assessment(&design, canonical_assertion, 103),
    )
    .unwrap();

    let mut value = serde_json::to_value(evidence).unwrap();
    value["assessment"]["Qualified"]["assertion"]["kind"] =
        serde_json::json!("Refines");
    let restored: SpeciesConceptRelationEvidence = serde_json::from_value(value).unwrap();
    assert!(matches!(
        restored.canonical_digest(),
        Err(RelationError::NonCanonicalRefinesAlias)
    ));
}

#[test]
fn equivalent_inverse_spelling_cannot_create_two_evidence_identities() {
    let a = identity("concept-a", 104);
    let b = identity("concept-b", 105);
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

    let via_refines = SpeciesConceptRelationAssertion::new(
        SpeciesConceptRelationKind::Refines,
        SpeciesConceptRelationDirection::LeftToRight,
    )
    .unwrap();
    let via_generalizes = SpeciesConceptRelationAssertion::new(
        SpeciesConceptRelationKind::Generalizes,
        SpeciesConceptRelationDirection::RightToLeft,
    )
    .unwrap();
    assert_eq!(via_refines, via_generalizes);

    let first = SpeciesConceptRelationEvidence::evaluate(
        &current,
        qualified_assessment(&design, via_refines, 106),
    )
    .unwrap();
    let second = SpeciesConceptRelationEvidence::evaluate(
        &current,
        qualified_assessment(&design, via_generalizes, 106),
    )
    .unwrap();
    assert_eq!(first, second);
    assert_eq!(
        first.canonical_digest().unwrap(),
        second.canonical_digest().unwrap()
    );
}
