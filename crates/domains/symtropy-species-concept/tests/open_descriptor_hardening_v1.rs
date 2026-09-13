include!("open_descriptor_v1.rs");

#[test]
fn persisted_capability_reordering_fails_local_canonicalization() {
    with_strict_descriptor("capability-order-domain", 60, 61, |_concept, _current, descriptor, _| {
        let mut value = serde_json::to_value(descriptor).unwrap();
        value["capabilities"].as_array_mut().unwrap().reverse();
        let changed: SpeciesConceptFamilyDescriptor = serde_json::from_value(value).unwrap();
        assert!(matches!(
            changed.canonical_digest(),
            Err(DescriptorError::NonCanonicalCapabilityOrder)
        ));
    });
}

#[test]
fn persisted_schema_reordering_fails_local_canonicalization() {
    with_strict_descriptor("schema-order-domain", 62, 63, |_concept, _current, descriptor, _| {
        assert!(descriptor.evidence_schema.terms.len() > 1);
        let mut value = serde_json::to_value(descriptor).unwrap();
        value["evidence_schema"]["terms"].as_array_mut().unwrap().reverse();
        let changed: SpeciesConceptFamilyDescriptor = serde_json::from_value(value).unwrap();
        assert!(matches!(
            changed.canonical_digest(),
            Err(DescriptorError::NonCanonicalSchemaOrder(_))
        ));
    });
}

#[test]
fn duplicate_capability_id_is_rejected_even_with_different_content() {
    with_strict_descriptor("duplicate-cap-domain", 64, 65, |_concept, _current, descriptor, _| {
        let mut value = serde_json::to_value(descriptor).unwrap();
        let mut duplicate = value["capabilities"][0].clone();
        duplicate["content_digest"][0] = serde_json::json!(255);
        value["capabilities"].as_array_mut().unwrap().push(duplicate);
        let changed: SpeciesConceptFamilyDescriptor = serde_json::from_value(value).unwrap();
        assert!(matches!(
            changed.canonical_digest(),
            Err(DescriptorError::DuplicateCapability(_))
        ));
    });
}

#[test]
fn duplicate_schema_term_id_is_rejected_even_with_different_content() {
    with_strict_descriptor("duplicate-term-domain", 66, 67, |_concept, _current, descriptor, _| {
        let mut value = serde_json::to_value(descriptor).unwrap();
        let mut duplicate = value["evidence_schema"]["terms"][0].clone();
        duplicate["content_digest"][0] = serde_json::json!(254);
        value["evidence_schema"]["terms"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        let changed: SpeciesConceptFamilyDescriptor = serde_json::from_value(value).unwrap();
        assert!(matches!(
            changed.canonical_digest(),
            Err(DescriptorError::DuplicateSchemaTerm(_))
        ));
    });
}

#[test]
fn schema_digest_tampering_fails_closed() {
    with_strict_descriptor("schema-digest-domain", 68, 69, |_concept, _current, descriptor, _| {
        let mut value = serde_json::to_value(descriptor).unwrap();
        value["evidence_schema"]["content_digest"][0] = serde_json::json!(253);
        let changed: SpeciesConceptFamilyDescriptor = serde_json::from_value(value).unwrap();
        assert!(matches!(
            changed.canonical_digest(),
            Err(DescriptorError::SchemaDigestMismatch(_))
        ));
    });
}

#[test]
fn schema_term_content_tampering_fails_via_schema_commitment() {
    with_strict_descriptor("term-digest-domain", 70, 71, |_concept, _current, descriptor, _| {
        let mut value = serde_json::to_value(descriptor).unwrap();
        value["domain_schema"]["terms"][0]["content_digest"][0] = serde_json::json!(252);
        let changed: SpeciesConceptFamilyDescriptor = serde_json::from_value(value).unwrap();
        assert!(matches!(
            changed.canonical_digest(),
            Err(DescriptorError::SchemaDigestMismatch(_))
        ));
    });
}

#[test]
fn descriptor_rule_tampering_fails_local_validation() {
    with_strict_descriptor("rule-domain", 72, 73, |_concept, _current, descriptor, _| {
        let mut value = serde_json::to_value(descriptor).unwrap();
        value["descriptor_rule_authority"]["revision"] = serde_json::json!(2);
        let changed: SpeciesConceptFamilyDescriptor = serde_json::from_value(value).unwrap();
        assert!(matches!(
            changed.canonical_digest(),
            Err(DescriptorError::DescriptorRuleMismatch)
        ));
    });
}

#[test]
fn opaque_source_digest_tampering_changes_representation_but_cannot_regain_current_authority() {
    with_strict_descriptor("opaque-source-domain", 74, 75, |_concept, current, descriptor, _| {
        let original_digest = descriptor.canonical_digest().unwrap();
        let mut value = serde_json::to_value(descriptor).unwrap();
        value["source_authority"]["authority_digest"][0] = serde_json::json!(251);
        let changed: SpeciesConceptFamilyDescriptor = serde_json::from_value(value).unwrap();

        let changed_digest = changed.canonical_digest().unwrap();
        assert_ne!(changed_digest, original_digest);
        assert!(matches!(
            ValidatedSpeciesConceptFamilyDescriptor::validate_current_strict_biological(
                &changed,
                current,
            ),
            Err(DescriptorError::ReplayMismatch)
        ));
    });
}

#[test]
fn opaque_source_adapter_tampering_is_structurally_representable_but_stale_for_current_replay() {
    with_strict_descriptor("source-adapter-domain", 76, 77, |_concept, current, descriptor, _| {
        let mut value = serde_json::to_value(descriptor).unwrap();
        value["source_authority"]["source_adapter_rule_authority"]["revision"] =
            serde_json::json!(99);
        let changed: SpeciesConceptFamilyDescriptor = serde_json::from_value(value).unwrap();
        assert!(changed.canonical_digest().is_ok());
        assert!(matches!(
            ValidatedSpeciesConceptFamilyDescriptor::validate_current_strict_biological(
                &changed,
                current,
            ),
            Err(DescriptorError::ReplayMismatch)
        ));
    });
}

#[test]
fn source_requalification_stales_descriptor_without_changing_conceptual_identity() {
    let original_model = model("requalification-domain", 78, 79);
    let changed_model = model("requalification-domain", 78, 80);

    let original_current_model = ValidatedBiologicalSpeciesModel::validate_current(
        &original_model,
        original_model.validity_domain.clone(),
        authority("species-model-qualification", 79),
    )
    .unwrap();
    let changed_current_model = ValidatedBiologicalSpeciesModel::validate_current(
        &changed_model,
        changed_model.validity_domain.clone(),
        authority("species-model-qualification", 80),
    )
    .unwrap();

    let original_concept = SpeciesConceptAuthority::from_strict_biological(&original_current_model).unwrap();
    let changed_concept = SpeciesConceptAuthority::from_strict_biological(&changed_current_model).unwrap();
    let original_current =
        ValidatedSpeciesConceptAuthority::validate_current_strict_biological(
            &original_concept,
            &original_current_model,
        )
        .unwrap();
    let changed_current =
        ValidatedSpeciesConceptAuthority::validate_current_strict_biological(
            &changed_concept,
            &changed_current_model,
        )
        .unwrap();

    let descriptor = SpeciesConceptFamilyDescriptor::from_strict_biological(&original_current).unwrap();
    let changed_descriptor = SpeciesConceptFamilyDescriptor::from_strict_biological(&changed_current).unwrap();
    assert_eq!(descriptor.conceptual_identity, changed_descriptor.conceptual_identity);
    assert_ne!(descriptor.canonical_digest().unwrap(), changed_descriptor.canonical_digest().unwrap());
    assert!(matches!(
        ValidatedSpeciesConceptFamilyDescriptor::validate_current_strict_biological(
            &descriptor,
            &changed_current,
        ),
        Err(DescriptorError::ReplayMismatch)
    ));
}
