include!("species_concept_authority_v1.rs");

use std::collections::BTreeSet;
use symtropy_evolution_core::SpeciesConceptError;

#[test]
fn qualifier_and_domain_variants_count_as_one_conceptual_identity() {
    let a_domain = domain("concept-count-domain-a", 60);
    let b_domain = domain("concept-count-domain-b", 61);

    let a_model = model(a_domain.clone(), 62);
    let a_requalified_model = model(a_domain.clone(), 63);
    let b_model = model(b_domain.clone(), 64);

    let a = SpeciesConceptAuthority::from_strict_biological(&current_model(
        &a_model,
        a_domain.clone(),
        62,
    ))
    .unwrap();
    let a_requalified = SpeciesConceptAuthority::from_strict_biological(&current_model(
        &a_requalified_model,
        a_domain,
        63,
    ))
    .unwrap();
    let b = SpeciesConceptAuthority::from_strict_biological(&current_model(
        &b_model,
        b_domain,
        64,
    ))
    .unwrap();

    let identities = BTreeSet::from([
        a.conceptual_identity().unwrap(),
        a_requalified.conceptual_identity().unwrap(),
        b.conceptual_identity().unwrap(),
    ]);
    assert_eq!(identities.len(), 1);

    assert_ne!(a.canonical_digest().unwrap(), a_requalified.canonical_digest().unwrap());
    assert_ne!(a.canonical_digest().unwrap(), b.canonical_digest().unwrap());
}

#[test]
fn reordered_raw_capabilities_do_not_fool_inspection_but_fail_authority_validation() {
    let validity = domain("capability-order-domain", 65);
    let source = model(validity.clone(), 66);
    let concept = SpeciesConceptAuthority::from_strict_biological(&current_model(
        &source,
        validity,
        66,
    ))
    .unwrap();

    let mut value = serde_json::to_value(&concept).unwrap();
    value["capabilities"].as_array_mut().unwrap().reverse();
    let changed: SpeciesConceptAuthority = serde_json::from_value(value).unwrap();

    assert!(changed.supports(SpeciesConceptCapability::CurrentSpeciesStatus));
    assert!(changed.supports(SpeciesConceptCapability::HistoricalTransitionInterval));
    assert!(matches!(
        changed.canonical_digest(),
        Err(SpeciesConceptError::CapabilitySurfaceMismatch)
    ));
}

#[test]
fn removing_a_capability_cannot_turn_the_adapter_into_a_different_model_family() {
    let validity = domain("capability-removal-domain", 67);
    let source = model(validity.clone(), 68);
    let concept = SpeciesConceptAuthority::from_strict_biological(&current_model(
        &source,
        validity,
        68,
    ))
    .unwrap();

    let mut value = serde_json::to_value(&concept).unwrap();
    value["capabilities"].as_array_mut().unwrap().pop();
    let changed: SpeciesConceptAuthority = serde_json::from_value(value).unwrap();
    assert!(matches!(
        changed.canonical_digest(),
        Err(SpeciesConceptError::CapabilitySurfaceMismatch)
    ));
}

#[test]
fn adapter_rule_identity_is_separate_from_source_qualification_and_tamper_evident() {
    let validity = domain("adapter-rule-domain", 69);
    let source = model(validity.clone(), 70);
    let concept = SpeciesConceptAuthority::from_strict_biological(&current_model(
        &source,
        validity,
        70,
    ))
    .unwrap();

    assert_ne!(
        concept.adapter_rule_authority.method_id,
        concept.qualification.authority.method_id
    );

    let mut value = serde_json::to_value(&concept).unwrap();
    value["adapter_rule_authority"]["revision"] = serde_json::json!(2);
    let changed: SpeciesConceptAuthority = serde_json::from_value(value).unwrap();
    assert!(matches!(
        changed.canonical_digest(),
        Err(SpeciesConceptError::AdapterRuleMismatch)
    ));
}

#[test]
fn archived_authority_cannot_replay_against_requalified_source_model() {
    let validity = domain("replay-domain", 71);
    let original_model = model(validity.clone(), 72);
    let requalified_model = model(validity.clone(), 73);
    let original_current = current_model(&original_model, validity.clone(), 72);
    let requalified_current = current_model(&requalified_model, validity, 73);

    let concept = SpeciesConceptAuthority::from_strict_biological(&original_current).unwrap();
    assert!(matches!(
        ValidatedSpeciesConceptAuthority::validate_current_strict_biological(
            &concept,
            &requalified_current,
        ),
        Err(SpeciesConceptError::ReplayMismatch)
    ));
}

#[test]
fn tampered_evidence_requirement_or_domain_constraint_fails_closed() {
    let validity = domain("semantic-surface-domain", 74);
    let source = model(validity.clone(), 75);
    let concept = SpeciesConceptAuthority::from_strict_biological(&current_model(
        &source,
        validity,
        75,
    ))
    .unwrap();

    let mut requirements = serde_json::to_value(&concept).unwrap();
    requirements["evidence_requirements"].as_array_mut().unwrap().pop();
    let changed: SpeciesConceptAuthority = serde_json::from_value(requirements).unwrap();
    assert!(matches!(
        changed.canonical_digest(),
        Err(SpeciesConceptError::EvidenceRequirementMismatch)
    ));

    let mut constraints = serde_json::to_value(&concept).unwrap();
    constraints["domain_constraints"] = serde_json::json!([]);
    let changed: SpeciesConceptAuthority = serde_json::from_value(constraints).unwrap();
    assert!(matches!(
        changed.canonical_digest(),
        Err(SpeciesConceptError::DomainConstraintMismatch)
    ));
}
