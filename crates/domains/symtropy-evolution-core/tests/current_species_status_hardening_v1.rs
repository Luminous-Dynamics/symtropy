include!("current_species_status_v1.rs");

#[test]
fn classification_design_wire_is_outcome_free() {
    with_status(
        HistoryCase::Clean,
        IsolationCase::Supported,
        inside(120),
        |evidence, _| {
            let value = serde_json::to_value(evidence.design()).unwrap();
            let mut keys = Vec::new();
            collect_keys(&value, &mut keys);
            for forbidden in [
                "status",
                "lineage_history",
                "reproductive_isolation",
                "applicability",
                "species_status",
                "speciation_event",
            ] {
                assert!(!keys.iter().any(|key| key == forbidden));
            }
        },
    );
}

#[test]
fn serialized_applicability_subject_transplant_fails_local_validation() {
    with_status(
        HistoryCase::Clean,
        IsolationCase::Supported,
        inside(121),
        |evidence, _| {
            let mut value = serde_json::to_value(evidence).unwrap();
            value["applicability"]["lineage_a"]["content_digest"][0] =
                serde_json::json!(254);
            let changed: CurrentSpeciesStatusEvidence = serde_json::from_value(value).unwrap();
            assert!(matches!(
                changed.canonical_digest(),
                Err(CurrentSpeciesStatusError::ApplicabilityLineageMismatch)
            ));
        },
    );
}

#[test]
fn serialized_species_model_digest_tampering_fails_local_validation() {
    with_status(
        HistoryCase::Clean,
        IsolationCase::Supported,
        inside(122),
        |evidence, _| {
            let mut value = serde_json::to_value(evidence).unwrap();
            value["species_model_digest"][0] = serde_json::json!(253);
            let changed: CurrentSpeciesStatusEvidence = serde_json::from_value(value).unwrap();
            assert!(matches!(
                changed.canonical_digest(),
                Err(CurrentSpeciesStatusError::SpeciesModelBindingMismatch)
            ));
        },
    );
}

#[test]
fn outside_validity_domain_is_not_a_negative_species_classification() {
    with_status(
        HistoryCase::Clean,
        IsolationCase::NotSupported,
        SpeciesModelApplicabilityInput::OutsideValidityDomain {
            evidence: authority("outside-model-domain", 123),
        },
        |evidence, _| {
            assert_eq!(
                evidence.status,
                CurrentSpeciesStatus::OutsideModelValidityDomain
            );
            assert_ne!(evidence.status, CurrentSpeciesStatus::NotSupportedUnderModel);
        },
    );
}
