use symtropy_evolution_core::{
    AnalysisAuthorityRef, AnalysisContentDigest, AnalysisMethodId, BiologicalSpeciesModel,
    SpeciesConceptAuthority, SpeciesModelId, SpeciesModelValidityDomainId,
    SpeciesModelValidityDomainRef, ValidatedBiologicalSpeciesModel,
    ValidatedSpeciesConceptAuthority,
};
use symtropy_species_concept::{
    open_species_concept_descriptor_rule_v1, DescriptorError, OpenSpeciesConceptContentDigest,
    OpenSpeciesConceptFamilyId, OpenSpeciesConceptIdentity, OpenSpeciesConceptModelId,
    SpeciesConceptCapabilityContentDigest, SpeciesConceptCapabilityId,
    SpeciesConceptCapabilityRef, SpeciesConceptFamilyDescriptor, SpeciesConceptSchema,
    SpeciesConceptSchemaId, SpeciesConceptSchemaTerm, SpeciesConceptSchemaTermDigest,
    SpeciesConceptSchemaTermId, SpeciesConceptSourceAuthorityDigest,
    SpeciesConceptSourceAuthorityRef, SpeciesConceptSourceKindId,
    SpeciesConceptSourceValidityDomainDigest, ValidatedSpeciesConceptFamilyDescriptor,
};

fn authority(label: &str, byte: u8) -> AnalysisAuthorityRef {
    AnalysisAuthorityRef::new(
        AnalysisMethodId::new(label).unwrap(),
        1,
        AnalysisContentDigest::new([byte; 32]),
    )
}

fn model(domain_id: &str, domain_byte: u8, qualification_byte: u8) -> BiologicalSpeciesModel {
    BiologicalSpeciesModel::qualify(
        SpeciesModelId::new("strict-biological-species-v1").unwrap(),
        SpeciesModelValidityDomainRef::new(
            SpeciesModelValidityDomainId::new(domain_id).unwrap(),
            authority("species-validity-domain", domain_byte),
        ),
        authority("species-model-qualification", qualification_byte),
    )
    .unwrap()
}

fn with_strict_descriptor<R>(
    domain_id: &str,
    domain_byte: u8,
    qualification_byte: u8,
    f: impl FnOnce(
        &SpeciesConceptAuthority,
        &ValidatedSpeciesConceptAuthority<'_>,
        &SpeciesConceptFamilyDescriptor,
        &ValidatedSpeciesConceptFamilyDescriptor<'_>,
    ) -> R,
) -> R {
    let model = model(domain_id, domain_byte, qualification_byte);
    let current_model = ValidatedBiologicalSpeciesModel::validate_current(
        &model,
        model.validity_domain.clone(),
        authority("species-model-qualification", qualification_byte),
    )
    .unwrap();
    let concept = SpeciesConceptAuthority::from_strict_biological(&current_model).unwrap();
    let current_concept =
        ValidatedSpeciesConceptAuthority::validate_current_strict_biological(
            &concept,
            &current_model,
        )
        .unwrap();
    let descriptor = SpeciesConceptFamilyDescriptor::from_strict_biological(&current_concept)
        .unwrap();
    let current_descriptor =
        ValidatedSpeciesConceptFamilyDescriptor::validate_current_strict_biological(
            &descriptor,
            &current_concept,
        )
        .unwrap();
    f(&concept, &current_concept, &descriptor, &current_descriptor)
}

#[test]
fn strict_bsc_projection_preserves_exact_concept_and_source_authority_identity() {
    with_strict_descriptor("diploid-sexual-domain", 10, 11, |concept, current, descriptor, validated| {
        let source_identity = current.conceptual_identity();
        assert_eq!(
            descriptor.conceptual_identity.family_id.as_str(),
            source_identity.family_id.as_str()
        );
        assert_eq!(
            descriptor.conceptual_identity.family_version,
            source_identity.family_version
        );
        assert_eq!(
            descriptor.conceptual_identity.content_digest.as_bytes(),
            source_identity.content_digest.as_bytes()
        );
        assert_eq!(
            descriptor.source_authority.authority_digest.as_bytes(),
            current.authority_digest().as_bytes()
        );
        assert_eq!(
            descriptor.source_authority.qualification_authority,
            concept.qualification.authority
        );
        assert_eq!(
            descriptor.source_authority.validity_domain_digest.as_bytes(),
            concept.validity_domain_digest.as_bytes()
        );
        assert_eq!(validated.descriptor(), descriptor);
        assert_eq!(
            validated.descriptor_digest(),
            descriptor.canonical_digest().unwrap()
        );
    });
}

#[test]
fn descriptor_round_trip_requires_fresh_source_specific_replay_for_current_authority() {
    with_strict_descriptor("round-trip-domain", 12, 13, |_concept, current, descriptor, _| {
        let encoded = serde_json::to_vec(descriptor).unwrap();
        let restored: SpeciesConceptFamilyDescriptor = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(
            restored.canonical_digest().unwrap(),
            descriptor.canonical_digest().unwrap()
        );
        let replayed =
            ValidatedSpeciesConceptFamilyDescriptor::validate_current_strict_biological(
                &restored,
                current,
            )
            .unwrap();
        assert_eq!(replayed.descriptor(), &restored);
    });
}

#[test]
fn qualification_drift_changes_descriptor_authority_but_not_concept_identity_or_schemas() {
    let mut first_identity = None;
    let mut first_digest = None;
    let mut first_evidence_schema = None;
    let mut first_domain_schema = None;

    with_strict_descriptor("qualification-domain", 14, 15, |_concept, _current, descriptor, _| {
        first_identity = Some(descriptor.conceptual_identity.clone());
        first_digest = Some(descriptor.canonical_digest().unwrap());
        first_evidence_schema = Some(descriptor.evidence_schema.content_digest);
        first_domain_schema = Some(descriptor.domain_schema.content_digest);
    });

    with_strict_descriptor("qualification-domain", 14, 16, |_concept, _current, descriptor, _| {
        assert_eq!(Some(descriptor.conceptual_identity.clone()), first_identity);
        assert_eq!(Some(descriptor.evidence_schema.content_digest), first_evidence_schema);
        assert_eq!(Some(descriptor.domain_schema.content_digest), first_domain_schema);
        assert_ne!(Some(descriptor.canonical_digest().unwrap()), first_digest);
    });
}

#[test]
fn validity_domain_drift_changes_source_domain_binding_not_conceptual_identity() {
    let mut first_identity = None;
    let mut first_digest = None;
    let mut first_source_domain = None;
    let mut first_evidence_schema = None;
    let mut first_domain_schema = None;

    with_strict_descriptor("domain-a", 17, 18, |_concept, _current, descriptor, _| {
        first_identity = Some(descriptor.conceptual_identity.clone());
        first_digest = Some(descriptor.canonical_digest().unwrap());
        first_source_domain = Some(descriptor.source_authority.validity_domain_digest);
        first_evidence_schema = Some(descriptor.evidence_schema.content_digest);
        first_domain_schema = Some(descriptor.domain_schema.content_digest);
    });

    with_strict_descriptor("domain-b", 19, 18, |_concept, _current, descriptor, _| {
        assert_eq!(Some(descriptor.conceptual_identity.clone()), first_identity);
        assert_ne!(Some(descriptor.source_authority.validity_domain_digest), first_source_domain);
        assert_eq!(Some(descriptor.evidence_schema.content_digest), first_evidence_schema);
        assert_eq!(Some(descriptor.domain_schema.content_digest), first_domain_schema);
        assert_ne!(Some(descriptor.canonical_digest().unwrap()), first_digest);
    });
}

fn term(id: &str, byte: u8) -> SpeciesConceptSchemaTerm {
    SpeciesConceptSchemaTerm::new(
        SpeciesConceptSchemaTermId::new(id).unwrap(),
        1,
        SpeciesConceptSchemaTermDigest::new([byte; 32]),
    )
    .unwrap()
}

fn capability(id: &str, byte: u8) -> SpeciesConceptCapabilityRef {
    SpeciesConceptCapabilityRef::new(
        SpeciesConceptCapabilityId::new(id).unwrap(),
        1,
        SpeciesConceptCapabilityContentDigest::new([byte; 32]),
    )
    .unwrap()
}

#[test]
fn generic_declaration_canonicalizes_caller_order_without_claiming_source_currentness() {
    let evidence_schema = SpeciesConceptSchema::declare(
        SpeciesConceptSchemaId::new("generic-evidence").unwrap(),
        1,
        vec![term("zeta", 30), term("alpha", 31)],
    )
    .unwrap();
    let domain_schema = SpeciesConceptSchema::declare(
        SpeciesConceptSchemaId::new("generic-domain").unwrap(),
        1,
        vec![term("beta", 32)],
    )
    .unwrap();
    let descriptor = SpeciesConceptFamilyDescriptor::declare(
        OpenSpeciesConceptIdentity::new(
            OpenSpeciesConceptFamilyId::new("generic-family").unwrap(),
            1,
            OpenSpeciesConceptContentDigest::new([33; 32]),
        )
        .unwrap(),
        OpenSpeciesConceptModelId::new("generic-model").unwrap(),
        SpeciesConceptSourceAuthorityRef::new(
            SpeciesConceptSourceKindId::new("generic-source").unwrap(),
            1,
            SpeciesConceptSourceAuthorityDigest::new([34; 32]),
            authority("generic-qualification", 35),
            SpeciesConceptSourceValidityDomainDigest::new([36; 32]),
            authority("generic-source-adapter", 37),
        )
        .unwrap(),
        vec![capability("zeta-capability", 38), capability("alpha-capability", 39)],
        evidence_schema,
        domain_schema,
        open_species_concept_descriptor_rule_v1(),
    )
    .unwrap();

    assert_eq!(descriptor.capabilities[0].capability_id.as_str(), "alpha-capability");
    assert_eq!(descriptor.evidence_schema.terms[0].term_id.as_str(), "alpha");
    assert!(descriptor.canonical_digest().is_ok());
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
fn descriptor_wire_has_no_concrete_species_or_transition_outcome() {
    with_strict_descriptor("wire-domain", 40, 41, |_concept, _current, descriptor, _| {
        let mut keys = Vec::new();
        collect_keys(&serde_json::to_value(descriptor).unwrap(), &mut keys);
        for forbidden in [
            "lineage_a",
            "lineage_b",
            "status",
            "current_species_status",
            "transition_status",
            "candidate_start_generation",
            "candidate_end_generation",
            "speciation_time",
            "robustness_status",
            "independent",
        ] {
            assert!(!keys.iter().any(|key| key == forbidden));
        }
    });
}

#[test]
fn schema_role_collision_is_rejected() {
    let schema = SpeciesConceptSchema::declare(
        SpeciesConceptSchemaId::new("same-schema").unwrap(),
        1,
        vec![term("one", 50)],
    )
    .unwrap();
    let result = SpeciesConceptFamilyDescriptor::declare(
        OpenSpeciesConceptIdentity::new(
            OpenSpeciesConceptFamilyId::new("collision-family").unwrap(),
            1,
            OpenSpeciesConceptContentDigest::new([51; 32]),
        )
        .unwrap(),
        OpenSpeciesConceptModelId::new("collision-model").unwrap(),
        SpeciesConceptSourceAuthorityRef::new(
            SpeciesConceptSourceKindId::new("collision-source").unwrap(),
            1,
            SpeciesConceptSourceAuthorityDigest::new([52; 32]),
            authority("collision-qualification", 53),
            SpeciesConceptSourceValidityDomainDigest::new([54; 32]),
            authority("collision-source-adapter", 55),
        )
        .unwrap(),
        vec![capability("current", 56)],
        schema.clone(),
        schema,
        open_species_concept_descriptor_rule_v1(),
    );
    assert!(matches!(result, Err(DescriptorError::SchemaRoleCollision)));
}
