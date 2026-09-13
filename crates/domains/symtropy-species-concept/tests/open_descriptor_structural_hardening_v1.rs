include!("open_descriptor_v1.rs");

#[test]
fn restored_zero_capability_revision_fails_closed() {
    with_strict_descriptor("zero-capability-revision", 90, 91, |_concept, _current, descriptor, _| {
        let mut value = serde_json::to_value(descriptor).unwrap();
        value["capabilities"][0]["revision"] = serde_json::json!(0);
        let changed: SpeciesConceptFamilyDescriptor = serde_json::from_value(value).unwrap();
        assert!(matches!(
            changed.canonical_digest(),
            Err(DescriptorError::ZeroVersion("capability_revision"))
        ));
    });
}

#[test]
fn restored_zero_schema_term_revision_fails_before_digest_reinterpretation() {
    with_strict_descriptor("zero-term-revision", 92, 93, |_concept, _current, descriptor, _| {
        let mut value = serde_json::to_value(descriptor).unwrap();
        value["evidence_schema"]["terms"][0]["revision"] = serde_json::json!(0);
        let changed: SpeciesConceptFamilyDescriptor = serde_json::from_value(value).unwrap();
        assert!(matches!(
            changed.canonical_digest(),
            Err(DescriptorError::ZeroVersion("schema_term_revision"))
        ));
    });
}

#[test]
fn generic_descriptor_requires_at_least_one_common_capability() {
    let evidence = SpeciesConceptSchema::declare(
        SpeciesConceptSchemaId::new("nonempty-evidence").unwrap(),
        1,
        vec![term("evidence", 94)],
    )
    .unwrap();
    let domain = SpeciesConceptSchema::declare(
        SpeciesConceptSchemaId::new("domain").unwrap(),
        1,
        Vec::<SpeciesConceptSchemaTerm>::new(),
    )
    .unwrap();
    let result = SpeciesConceptFamilyDescriptor::declare(
        OpenSpeciesConceptIdentity::new(
            OpenSpeciesConceptFamilyId::new("empty-capability-family").unwrap(),
            1,
            OpenSpeciesConceptContentDigest::new([95; 32]),
        )
        .unwrap(),
        OpenSpeciesConceptModelId::new("empty-capability-model").unwrap(),
        SpeciesConceptSourceAuthorityRef::new(
            SpeciesConceptSourceKindId::new("empty-capability-source").unwrap(),
            1,
            SpeciesConceptSourceAuthorityDigest::new([96; 32]),
            authority("empty-capability-qualification", 97),
            SpeciesConceptSourceValidityDomainDigest::new([98; 32]),
            authority("empty-capability-source-adapter", 99),
        )
        .unwrap(),
        Vec::<SpeciesConceptCapabilityRef>::new(),
        evidence,
        domain,
        open_species_concept_descriptor_rule_v1(),
    );
    assert!(matches!(result, Err(DescriptorError::EmptyCapabilitySurface)));
}

#[test]
fn generic_descriptor_requires_a_nonempty_evidence_contract() {
    let evidence = SpeciesConceptSchema::declare(
        SpeciesConceptSchemaId::new("empty-evidence").unwrap(),
        1,
        Vec::<SpeciesConceptSchemaTerm>::new(),
    )
    .unwrap();
    let domain = SpeciesConceptSchema::declare(
        SpeciesConceptSchemaId::new("nonempty-domain").unwrap(),
        1,
        vec![term("domain", 100)],
    )
    .unwrap();
    let result = SpeciesConceptFamilyDescriptor::declare(
        OpenSpeciesConceptIdentity::new(
            OpenSpeciesConceptFamilyId::new("empty-evidence-family").unwrap(),
            1,
            OpenSpeciesConceptContentDigest::new([101; 32]),
        )
        .unwrap(),
        OpenSpeciesConceptModelId::new("empty-evidence-model").unwrap(),
        SpeciesConceptSourceAuthorityRef::new(
            SpeciesConceptSourceKindId::new("empty-evidence-source").unwrap(),
            1,
            SpeciesConceptSourceAuthorityDigest::new([102; 32]),
            authority("empty-evidence-qualification", 103),
            SpeciesConceptSourceValidityDomainDigest::new([104; 32]),
            authority("empty-evidence-source-adapter", 105),
        )
        .unwrap(),
        vec![capability("current-species-status", 106)],
        evidence,
        domain,
        open_species_concept_descriptor_rule_v1(),
    );
    assert!(matches!(result, Err(DescriptorError::EmptyEvidenceSchema)));
}
