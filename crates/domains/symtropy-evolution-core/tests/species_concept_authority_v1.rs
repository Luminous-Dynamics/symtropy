include!("species_model_v1.rs");

use symtropy_evolution_core::{
    SpeciesConceptAuthority, SpeciesConceptCapability, SpeciesConceptDomainConstraint,
    SpeciesConceptEvidenceRequirement, ValidatedSpeciesConceptAuthority,
};

fn current_model<'a>(
    model: &'a BiologicalSpeciesModel,
    validity: SpeciesModelValidityDomainRef,
    qualification_byte: u8,
) -> ValidatedBiologicalSpeciesModel<'a> {
    ValidatedBiologicalSpeciesModel::validate_current(
        model,
        validity,
        authority("species-model-qualification", qualification_byte),
    )
    .unwrap()
}

#[test]
fn strict_biological_model_adapts_and_replays_without_semantic_dilution() {
    let validity = domain("diploid-sexual-validity-domain", 21);
    let source = model(validity.clone(), 22);
    let current = current_model(&source, validity, 22);
    let concept = SpeciesConceptAuthority::from_strict_biological(&current).unwrap();

    assert!(concept.supports(SpeciesConceptCapability::CurrentSpeciesStatus));
    assert!(concept.supports(SpeciesConceptCapability::HistoricalTransitionInterval));
    assert_eq!(
        concept.evidence_requirements,
        vec![
            SpeciesConceptEvidenceRequirement::ExplicitQualifiedModelApplicability,
            SpeciesConceptEvidenceRequirement::CurrentCompleteReproductiveBarrier,
            SpeciesConceptEvidenceRequirement::PersistentOrRecontactLineageHistory,
            SpeciesConceptEvidenceRequirement::HistoricalTransitionTemporalEvidence,
        ]
    );
    assert_eq!(
        concept.domain_constraints,
        vec![SpeciesConceptDomainConstraint::ReproductiveIsolationMustBeBiologicallyMeaningful]
    );

    let restored: SpeciesConceptAuthority =
        serde_json::from_slice(&serde_json::to_vec(&concept).unwrap()).unwrap();
    let validated =
        ValidatedSpeciesConceptAuthority::validate_current_strict_biological(&restored, &current)
            .unwrap();
    assert_eq!(
        validated.authority_digest(),
        concept.canonical_digest().unwrap()
    );
    validated
        .require_capability(SpeciesConceptCapability::CurrentSpeciesStatus)
        .unwrap();
    validated
        .require_capability(SpeciesConceptCapability::HistoricalTransitionInterval)
        .unwrap();
}

#[test]
fn qualification_drift_changes_authority_not_conceptual_identity() {
    let validity = domain("diploid-sexual-validity-domain", 23);
    let first_source = model(validity.clone(), 24);
    let second_source = model(validity.clone(), 25);
    let first_current = current_model(&first_source, validity.clone(), 24);
    let second_current = current_model(&second_source, validity, 25);

    let first = SpeciesConceptAuthority::from_strict_biological(&first_current).unwrap();
    let second = SpeciesConceptAuthority::from_strict_biological(&second_current).unwrap();

    assert_eq!(
        first.conceptual_identity().unwrap(),
        second.conceptual_identity().unwrap()
    );
    assert_eq!(first.validity_domain_digest, second.validity_domain_digest);
    assert_ne!(
        first.canonical_digest().unwrap(),
        second.canonical_digest().unwrap()
    );
}

#[test]
fn validity_domain_drift_changes_authority_not_conceptual_identity() {
    let first_validity = domain("diploid-sexual-domain-a", 26);
    let second_validity = domain("diploid-sexual-domain-b", 27);
    let first_source = model(first_validity.clone(), 28);
    let second_source = model(second_validity.clone(), 28);
    let first_current = current_model(&first_source, first_validity, 28);
    let second_current = current_model(&second_source, second_validity, 28);

    let first = SpeciesConceptAuthority::from_strict_biological(&first_current).unwrap();
    let second = SpeciesConceptAuthority::from_strict_biological(&second_current).unwrap();

    assert_eq!(
        first.conceptual_identity().unwrap(),
        second.conceptual_identity().unwrap()
    );
    assert_ne!(first.validity_domain_digest, second.validity_domain_digest);
    assert_ne!(
        first.canonical_digest().unwrap(),
        second.canonical_digest().unwrap()
    );
}

#[test]
fn concept_identity_is_family_and_semantic_content_not_qualification_or_domain() {
    let first_validity = domain("concept-identity-a", 29);
    let second_validity = domain("concept-identity-b", 30);
    let first_source = model(first_validity.clone(), 31);
    let second_source = model(second_validity.clone(), 32);
    let first = SpeciesConceptAuthority::from_strict_biological(&current_model(
        &first_source,
        first_validity,
        31,
    ))
    .unwrap();
    let second = SpeciesConceptAuthority::from_strict_biological(&current_model(
        &second_source,
        second_validity,
        32,
    ))
    .unwrap();

    let first_identity = first.conceptual_identity().unwrap();
    let second_identity = second.conceptual_identity().unwrap();
    assert_eq!(first_identity, second_identity);
    assert_eq!(first_identity.family_id.as_str(), "strict-biological-species");
}

#[test]
fn wire_shape_has_no_target_status_or_historical_outcome() {
    let validity = domain("wire-domain", 33);
    let source = model(validity.clone(), 34);
    let concept = SpeciesConceptAuthority::from_strict_biological(&current_model(
        &source,
        validity,
        34,
    ))
    .unwrap();
    let value = serde_json::to_value(concept).unwrap();
    let mut keys = Vec::new();
    collect_keys(&value, &mut keys);

    for forbidden in [
        "lineage_a",
        "lineage_b",
        "status",
        "current_species_status",
        "transition_status",
        "speciation_transition_status",
        "candidate_start_generation",
        "candidate_end_generation",
        "transition_generation",
        "speciation_time",
    ] {
        assert!(!keys.iter().any(|key| key == forbidden));
    }
}
