use symtropy_evolution_core::{AnalysisAuthorityRef, AnalysisContentDigest, AnalysisMethodId};
use symtropy_species_concept::{
    OpenSpeciesConceptContentDigest, OpenSpeciesConceptFamilyId, OpenSpeciesConceptIdentity,
};
use symtropy_species_concept_relations::*;

fn identity(name: &str, byte: u8) -> OpenSpeciesConceptIdentity {
    OpenSpeciesConceptIdentity::new(
        OpenSpeciesConceptFamilyId::new(name).unwrap(),
        1,
        OpenSpeciesConceptContentDigest::new([byte; 32]),
    )
    .unwrap()
}

fn authority(name: &str, byte: u8) -> AnalysisAuthorityRef {
    AnalysisAuthorityRef::new(
        AnalysisMethodId::new(name).unwrap(),
        1,
        AnalysisContentDigest::new([byte; 32]),
    )
}

fn scope(byte: u8) -> SpeciesConceptRelationScopeRef {
    SpeciesConceptRelationScopeRef::new(
        SpeciesConceptRelationScopeId::new("comparative-taxonomy-scope").unwrap(),
        1,
        SpeciesConceptRelationScopeDigest::new([byte; 32]),
    )
    .unwrap()
}

fn design(
    first: OpenSpeciesConceptIdentity,
    second: OpenSpeciesConceptIdentity,
    policy: RelationMissingPolicy,
) -> SpeciesConceptRelationDesign {
    SpeciesConceptRelationDesign::declare(
        SpeciesConceptRelationDesignId::new("bsc-vs-lineage-relation").unwrap(),
        first,
        second,
        scope(10),
        authority("semantic-relation-protocol", 11),
        policy,
    )
    .unwrap()
}

fn current_design<'a>(
    design: &'a SpeciesConceptRelationDesign,
    first: OpenSpeciesConceptIdentity,
    second: OpenSpeciesConceptIdentity,
    policy: RelationMissingPolicy,
) -> ValidatedSpeciesConceptRelationDesign<'a> {
    ValidatedSpeciesConceptRelationDesign::validate_current(
        design,
        first,
        second,
        scope(10),
        authority("semantic-relation-protocol", 11),
        policy,
    )
    .unwrap()
}

fn qualified_assessment(
    design: &SpeciesConceptRelationDesign,
    assertion: SpeciesConceptRelationAssertion,
    qualification_byte: u8,
) -> SpeciesConceptRelationAssessment {
    let mapping = semantic_mapping_digest_v1(
        design,
        &assertion,
        b"explicit semantic mapping between the two concept families",
    )
    .unwrap();
    SpeciesConceptRelationAssessment::Qualified {
        assertion,
        semantic_mapping_digest: mapping,
        scientific_reference_authority: authority("semantic-reference", 20),
        qualification_authority: authority("semantic-relation-qualification", qualification_byte),
    }
}

#[test]
fn endpoint_order_is_canonical_and_nonsemantic() {
    let a = identity("strict-biological-species", 1);
    let b = identity("general-lineage-species", 2);
    let first = design(
        a.clone(),
        b.clone(),
        RelationMissingPolicy::ReportUnknownOrUnavailable,
    );
    let second = design(
        b.clone(),
        a.clone(),
        RelationMissingPolicy::ReportUnknownOrUnavailable,
    );

    assert_eq!(first, second);
    assert_eq!(
        first.canonical_digest().unwrap(),
        second.canonical_digest().unwrap()
    );
    assert!(first.left < first.right);

    let _current = current_design(
        &first,
        b,
        a,
        RelationMissingPolicy::ReportUnknownOrUnavailable,
    );
}

#[test]
fn directional_reversal_is_semantically_distinct() {
    let a = identity("strict-biological-species", 3);
    let b = identity("general-lineage-species", 4);
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

    let forward = SpeciesConceptRelationAssertion::new(
        SpeciesConceptRelationKind::Generalizes,
        SpeciesConceptRelationDirection::LeftToRight,
    )
    .unwrap();
    let reverse = SpeciesConceptRelationAssertion::new(
        SpeciesConceptRelationKind::Generalizes,
        SpeciesConceptRelationDirection::RightToLeft,
    )
    .unwrap();

    let forward_evidence = SpeciesConceptRelationEvidence::evaluate(
        &current,
        qualified_assessment(&design, forward, 21),
    )
    .unwrap();
    let reverse_evidence = SpeciesConceptRelationEvidence::evaluate(
        &current,
        qualified_assessment(&design, reverse, 21),
    )
    .unwrap();

    assert_ne!(
        forward_evidence.canonical_digest().unwrap(),
        reverse_evidence.canonical_digest().unwrap()
    );
}

#[test]
fn nested_and_operational_relations_are_dependency_visible() {
    for kind in [
        SpeciesConceptRelationKind::EquivalentSemanticTarget,
        SpeciesConceptRelationKind::Generalizes,
        SpeciesConceptRelationKind::Refines,
        SpeciesConceptRelationKind::OperationalCriterionWithin,
    ] {
        assert_eq!(
            kind.dependency_class(),
            SemanticDependencyClass::NestedOrCriterionDependent
        );
    }
    assert_eq!(
        SpeciesConceptRelationKind::PartiallyOverlaps.dependency_class(),
        SemanticDependencyClass::Overlapping
    );
}

#[test]
fn orthogonal_or_competing_only_means_potentially_non_nested() {
    for kind in [
        SpeciesConceptRelationKind::OrthogonalEvidenceFramework,
        SpeciesConceptRelationKind::CompetingOntology,
    ] {
        assert_eq!(
            kind.dependency_class(),
            SemanticDependencyClass::PotentiallyNonNested
        );
    }
}

#[test]
fn disputed_and_unknown_relations_never_expose_a_dependency_class() {
    let a = identity("concept-a", 5);
    let b = identity("concept-b", 6);
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
    let candidate = SpeciesConceptRelationAssertion::new(
        SpeciesConceptRelationKind::PartiallyOverlaps,
        SpeciesConceptRelationDirection::Symmetric,
    )
    .unwrap();
    let mapping = semantic_mapping_digest_v1(&design, &candidate, b"disputed mapping").unwrap();

    let disputed = SpeciesConceptRelationEvidence::evaluate(
        &current,
        SpeciesConceptRelationAssessment::Disputed {
            candidate,
            semantic_mapping_digest: mapping,
            scientific_reference_authority: authority("semantic-reference", 30),
            qualification_authority: authority("semantic-relation-qualification", 31),
            dispute_authority: authority("semantic-relation-dispute", 32),
        },
    )
    .unwrap();
    assert_eq!(disputed.assessment.dependency_class_if_qualified(), None);

    let unknown = SpeciesConceptRelationEvidence::evaluate(
        &current,
        SpeciesConceptRelationAssessment::UnknownOrUnqualified {
            evidence_authority: authority("unknown-relation-evidence", 33),
        },
    )
    .unwrap();
    assert_eq!(unknown.assessment.dependency_class_if_qualified(), None);
}

#[test]
fn fail_closed_policy_rejects_unknown_and_unavailable_relation_evidence() {
    let a = identity("concept-a", 7);
    let b = identity("concept-b", 8);
    let design = design(a.clone(), b.clone(), RelationMissingPolicy::FailClosed);
    let current = current_design(&design, a, b, RelationMissingPolicy::FailClosed);

    for assessment in [
        SpeciesConceptRelationAssessment::UnknownOrUnqualified {
            evidence_authority: authority("unknown-relation-evidence", 40),
        },
        SpeciesConceptRelationAssessment::Unavailable {
            evidence_authority: authority("unavailable-relation-evidence", 41),
        },
    ] {
        assert!(matches!(
            SpeciesConceptRelationEvidence::evaluate(&current, assessment),
            Err(RelationError::MissingRelationEvidenceFailClosed)
        ));
    }
}

#[test]
fn qualification_drift_changes_evidence_identity_and_stales_replay() {
    let a = identity("strict-biological-species", 9);
    let b = identity("general-lineage-species", 10);
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
        SpeciesConceptRelationKind::OperationalCriterionWithin,
        SpeciesConceptRelationDirection::LeftToRight,
    )
    .unwrap();

    let first_input = qualified_assessment(&design, assertion.clone(), 50);
    let second_input = qualified_assessment(&design, assertion, 51);
    let first = SpeciesConceptRelationEvidence::evaluate(&current, first_input.clone()).unwrap();
    let second = SpeciesConceptRelationEvidence::evaluate(&current, second_input.clone()).unwrap();

    assert_ne!(
        first.canonical_digest().unwrap(),
        second.canonical_digest().unwrap()
    );
    ValidatedSpeciesConceptRelationEvidence::validate_current(&first, &current, first_input)
        .unwrap();
    assert!(matches!(
        ValidatedSpeciesConceptRelationEvidence::validate_current(&first, &current, second_input),
        Err(RelationError::EvidenceReplayMismatch)
    ));
}

#[test]
fn same_concept_identity_cannot_be_related_to_itself() {
    let a = identity("same-concept", 60);
    assert!(matches!(
        SpeciesConceptRelationDesign::declare(
            SpeciesConceptRelationDesignId::new("self-relation").unwrap(),
            a.clone(),
            a,
            scope(61),
            authority("semantic-relation-protocol", 62),
            RelationMissingPolicy::ReportUnknownOrUnavailable,
        ),
        Err(RelationError::SameConceptEndpoint)
    ));
}

#[test]
fn wire_shape_contains_no_species_result_or_independence_claim() {
    let a = identity("concept-a", 70);
    let b = identity("concept-b", 71);
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
        qualified_assessment(&design, assertion, 72),
    )
    .unwrap();
    let json = serde_json::to_string(&evidence).unwrap();

    for forbidden in [
        "species_status",
        "speciation_status",
        "transition_generation",
        "robustness_status",
        "independent_confirmation",
        "universal_taxonomy",
    ] {
        assert!(!json.contains(forbidden));
    }
}
