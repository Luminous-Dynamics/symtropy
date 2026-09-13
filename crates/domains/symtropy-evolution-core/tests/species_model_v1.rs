use symtropy_evolution_core::{
    strict_biological_species_model_content_digest_v1, AnalysisAuthorityRef,
    AnalysisContentDigest, AnalysisMethodId, BiologicalSpeciesModel, CurrentGeneFlowPolicy,
    EcologySpeciesEvidencePolicy, GeographicIsolationPolicy, HistoricalRecontactPolicy,
    IncompleteLineageSortingPolicy, LineageDivergenceModelRequirement, LineageFusionSpeciesPolicy,
    ReproductiveIsolationModelRequirement, SpeciesModelError, SpeciesModelId,
    SpeciesModelMissingConflictPolicy, SpeciesModelValidityDomainId,
    SpeciesModelValidityDomainRef, ValidatedBiologicalSpeciesModel,
};

fn authority(label: &str, byte: u8) -> AnalysisAuthorityRef {
    AnalysisAuthorityRef::new(
        AnalysisMethodId::new(label).unwrap(),
        1,
        AnalysisContentDigest::new([byte; 32]),
    )
}

fn domain(label: &str, byte: u8) -> SpeciesModelValidityDomainRef {
    SpeciesModelValidityDomainRef::new(
        SpeciesModelValidityDomainId::new(label).unwrap(),
        authority("species-validity-domain-authority", byte),
    )
}

fn model(
    domain: SpeciesModelValidityDomainRef,
    qualification_byte: u8,
) -> BiologicalSpeciesModel {
    BiologicalSpeciesModel::qualify(
        SpeciesModelId::new("strict-biological-species-v1").unwrap(),
        domain,
        authority("species-model-qualification", qualification_byte),
    )
    .unwrap()
}

#[test]
fn strict_v1_policy_surface_is_frozen_and_outcome_free() {
    let model = model(domain("diploid-sexual-validity-domain", 1), 2);
    assert_eq!(
        model.model_content_digest,
        strict_biological_species_model_content_digest_v1()
    );
    assert_eq!(
        model.reproductive_isolation,
        ReproductiveIsolationModelRequirement::CurrentCompleteBarrierSupported
    );
    assert_eq!(
        model.lineage_divergence,
        LineageDivergenceModelRequirement::PersistentOrRecontactWithoutFusion
    );
    assert_eq!(
        model.current_gene_flow,
        CurrentGeneFlowPolicy::ContradictsCompleteIsolation
    );
    assert_eq!(
        model.historical_recontact,
        HistoricalRecontactPolicy::CompatibleWhenCurrentCompleteBarrierSupported
    );
    assert_eq!(
        model.lineage_fusion,
        LineageFusionSpeciesPolicy::ContradictsDistinctCurrentSpecies
    );
    assert_eq!(
        model.geographic_isolation,
        GeographicIsolationPolicy::NeverSufficientAlone
    );
    assert_eq!(
        model.incomplete_lineage_sorting,
        IncompleteLineageSortingPolicy::DoesNotDecideStatusAlone
    );
    assert_eq!(
        model.ecology,
        EcologySpeciesEvidencePolicy::NotRequiredInStrictV1
    );
    assert_eq!(
        model.missing_or_conflicting_evidence,
        SpeciesModelMissingConflictPolicy::InsufficientEvidence
    );
}

#[test]
fn qualifier_drift_changes_authority_identity_without_changing_model_content() {
    let validity = domain("diploid-sexual-validity-domain", 3);
    let first = model(validity.clone(), 4);
    let second = model(validity, 5);

    assert_eq!(first.model_content_digest, second.model_content_digest);
    assert_ne!(
        first.canonical_digest().unwrap(),
        second.canonical_digest().unwrap()
    );
}

#[test]
fn validity_domain_drift_changes_authority_identity_without_changing_model_content() {
    let first = model(domain("domain-a", 6), 8);
    let second = model(domain("domain-b", 7), 8);

    assert_eq!(first.model_content_digest, second.model_content_digest);
    assert_ne!(
        first.validity_domain.canonical_digest(),
        second.validity_domain.canonical_digest()
    );
    assert_ne!(
        first.canonical_digest().unwrap(),
        second.canonical_digest().unwrap()
    );
}

#[test]
fn restored_model_requires_fresh_matching_domain_and_qualification() {
    let validity = domain("diploid-sexual-validity-domain", 9);
    let model = model(validity.clone(), 10);
    let restored: BiologicalSpeciesModel =
        serde_json::from_slice(&serde_json::to_vec(&model).unwrap()).unwrap();

    let validated = ValidatedBiologicalSpeciesModel::validate_current(
        &restored,
        validity.clone(),
        authority("species-model-qualification", 10),
    )
    .unwrap();
    assert_eq!(
        validated.model_digest(),
        model.canonical_digest().unwrap()
    );

    assert!(matches!(
        ValidatedBiologicalSpeciesModel::validate_current(
            &restored,
            validity,
            authority("species-model-qualification", 11),
        ),
        Err(SpeciesModelError::ReplayMismatch)
    ));
}

#[test]
fn serialized_qualification_binding_tampering_fails_local_validation() {
    let model = model(domain("diploid-sexual-validity-domain", 12), 13);
    let mut value = serde_json::to_value(model).unwrap();
    value["qualification"]["model_content_digest"][0] = serde_json::json!(255);
    let changed: BiologicalSpeciesModel = serde_json::from_value(value).unwrap();
    assert!(matches!(
        changed.canonical_digest(),
        Err(SpeciesModelError::QualificationBindingMismatch)
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
fn species_model_wire_shape_has_no_target_lineage_or_classification_result() {
    let model = model(domain("diploid-sexual-validity-domain", 14), 15);
    let value = serde_json::to_value(model).unwrap();
    let mut keys = Vec::new();
    collect_keys(&value, &mut keys);

    for forbidden in [
        "lineage_a",
        "lineage_b",
        "lineage_divergence_status",
        "reproductive_isolation_status",
        "current_species_status",
        "species_status",
        "speciation_event",
        "transition_interval",
    ] {
        assert!(!keys.iter().any(|key| key == forbidden));
    }
}
