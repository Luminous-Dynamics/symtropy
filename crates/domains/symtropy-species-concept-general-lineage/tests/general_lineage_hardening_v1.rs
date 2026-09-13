include!("general_lineage_v1.rs");

#[test]
fn support_threshold_below_two_is_rejected() {
    lineage_fixture::with_clean(|history_design, _| {
        let raw_model = model(GeneralLineageReproductiveModePolicy::SexualOrAsexual, 120);
        let model = current_model(
            &raw_model,
            GeneralLineageReproductiveModePolicy::SexualOrAsexual,
            120,
        );
        assert!(matches!(
            GeneralLineageClassificationDesign::declare(
                GeneralLineageClassificationId::new("too-low").unwrap(),
                history_design,
                &model,
                auth("general-lineage-target-applicability", 80),
                base_channels(),
                1,
                GeneralLineageMissingEvidencePolicy::ReportInsufficientIndependentEvidence,
            ),
            Err(GeneralLineageDesignError::SupportThresholdTooLow)
        ));
    });
}

#[test]
fn exactly_one_reserved_core_channel_is_required() {
    lineage_fixture::with_clean(|history_design, _| {
        let raw_model = model(GeneralLineageReproductiveModePolicy::SexualOrAsexual, 121);
        let model = current_model(
            &raw_model,
            GeneralLineageReproductiveModePolicy::SexualOrAsexual,
            121,
        );
        let no_core = vec![
            channel(
                "ecology",
                GeneralLineageEvidenceChannelKind::EcologicalDifferentiation,
                GeneralLineageEvidenceChannelRole::Corroborating,
                "ecology",
                1,
            ),
            channel(
                "genetics",
                GeneralLineageEvidenceChannelKind::GeneticDiagnosability,
                GeneralLineageEvidenceChannelRole::Corroborating,
                "genetics",
                3,
            ),
        ];
        assert!(matches!(
            GeneralLineageClassificationDesign::declare(
                GeneralLineageClassificationId::new("no-core").unwrap(),
                history_design,
                &model,
                auth("general-lineage-target-applicability", 80),
                no_core,
                2,
                GeneralLineageMissingEvidencePolicy::ReportInsufficientIndependentEvidence,
            ),
            Err(GeneralLineageDesignError::CoreChannelCount(0))
        ));

        let mut two_core = base_channels();
        two_core.push(channel(
            "second-core",
            GeneralLineageEvidenceChannelKind::LongitudinalLineageSeparation,
            GeneralLineageEvidenceChannelRole::CoreRequired,
            "second-history",
            5,
        ));
        assert!(matches!(
            GeneralLineageClassificationDesign::declare(
                GeneralLineageClassificationId::new("two-core").unwrap(),
                history_design,
                &model,
                auth("general-lineage-target-applicability", 80),
                two_core,
                2,
                GeneralLineageMissingEvidencePolicy::ReportInsufficientIndependentEvidence,
            ),
            Err(GeneralLineageDesignError::CoreChannelCount(2))
        ));
    });
}

#[test]
fn model_requalification_changes_authority_not_conceptual_identity() {
    let raw_a = model(GeneralLineageReproductiveModePolicy::SexualOrAsexual, 122);
    let raw_b = model(GeneralLineageReproductiveModePolicy::SexualOrAsexual, 123);
    let current_a = current_model(
        &raw_a,
        GeneralLineageReproductiveModePolicy::SexualOrAsexual,
        122,
    );
    let current_b = current_model(
        &raw_b,
        GeneralLineageReproductiveModePolicy::SexualOrAsexual,
        123,
    );
    let a = general_lineage_family_descriptor(&current_a).unwrap();
    let b = general_lineage_family_descriptor(&current_b).unwrap();

    assert_eq!(a.conceptual_identity, b.conceptual_identity);
    assert_ne!(a.source_authority.authority_digest, b.source_authority.authority_digest);
    assert_ne!(a.canonical_digest().unwrap(), b.canonical_digest().unwrap());
    assert!(ValidatedGeneralLineageFamilyDescriptor::validate_current(&a, &current_b).is_err());
}

#[test]
fn validity_mode_changes_authority_not_general_lineage_concept_identity() {
    let sexual = model(GeneralLineageReproductiveModePolicy::SexualOnly, 124);
    let asexual = model(GeneralLineageReproductiveModePolicy::AsexualOnly, 124);
    let current_sexual = current_model(
        &sexual,
        GeneralLineageReproductiveModePolicy::SexualOnly,
        124,
    );
    let current_asexual = current_model(
        &asexual,
        GeneralLineageReproductiveModePolicy::AsexualOnly,
        124,
    );
    let sexual_descriptor = general_lineage_family_descriptor(&current_sexual).unwrap();
    let asexual_descriptor = general_lineage_family_descriptor(&current_asexual).unwrap();
    assert_eq!(
        sexual_descriptor.conceptual_identity,
        asexual_descriptor.conceptual_identity
    );
    assert_ne!(
        sexual_descriptor.source_authority.validity_domain_digest,
        asexual_descriptor.source_authority.validity_domain_digest
    );
}

#[test]
fn restored_noncanonical_channel_order_is_rejected() {
    lineage_fixture::with_clean(|history_design, _| {
        let raw_model = model(GeneralLineageReproductiveModePolicy::SexualOrAsexual, 125);
        let model = current_model(
            &raw_model,
            GeneralLineageReproductiveModePolicy::SexualOrAsexual,
            125,
        );
        let design = GeneralLineageClassificationDesign::declare(
            GeneralLineageClassificationId::new("channel-order").unwrap(),
            history_design,
            &model,
            auth("general-lineage-target-applicability", 80),
            base_channels(),
            2,
            GeneralLineageMissingEvidencePolicy::ReportInsufficientIndependentEvidence,
        )
        .unwrap();
        let mut value = serde_json::to_value(design).unwrap();
        value["channels"].as_array_mut().unwrap().reverse();
        let restored: GeneralLineageClassificationDesign = serde_json::from_value(value).unwrap();
        assert!(matches!(
            restored.canonical_digest(),
            Err(GeneralLineageDesignError::NonCanonicalChannelOrder)
        ));
    });
}

#[test]
fn restored_zero_channel_protocol_revision_is_rejected() {
    lineage_fixture::with_clean(|history_design, _| {
        let raw_model = model(GeneralLineageReproductiveModePolicy::SexualOrAsexual, 126);
        let model = current_model(
            &raw_model,
            GeneralLineageReproductiveModePolicy::SexualOrAsexual,
            126,
        );
        let design = GeneralLineageClassificationDesign::declare(
            GeneralLineageClassificationId::new("zero-protocol").unwrap(),
            history_design,
            &model,
            auth("general-lineage-target-applicability", 80),
            base_channels(),
            2,
            GeneralLineageMissingEvidencePolicy::ReportInsufficientIndependentEvidence,
        )
        .unwrap();
        let mut value = serde_json::to_value(design).unwrap();
        value["channels"][0]["protocol_authority"]["revision"] = serde_json::json!(0);
        let restored: GeneralLineageClassificationDesign = serde_json::from_value(value).unwrap();
        assert!(matches!(
            restored.canonical_digest(),
            Err(GeneralLineageDesignError::ZeroRevision("channel_protocol_revision"))
        ));
    });
}

fn supported_evidence_fixture(
    f: impl FnOnce(
        &GeneralLineageSpeciesEvidence,
        &ValidatedGeneralLineageClassificationDesign<'_>,
        &symtropy_evolution_core::ValidatedLineageDivergenceHistory<'_>,
        &ValidatedGeneralLineageSpeciesModel<'_>,
    ),
) {
    lineage_fixture::with_clean(|history_design, history| {
        let raw_model = model(GeneralLineageReproductiveModePolicy::SexualOrAsexual, 127);
        let model = current_model(
            &raw_model,
            GeneralLineageReproductiveModePolicy::SexualOrAsexual,
            127,
        );
        let channels = base_channels();
        let raw_design = GeneralLineageClassificationDesign::declare(
            GeneralLineageClassificationId::new("hardening-supported").unwrap(),
            history_design,
            &model,
            auth("general-lineage-target-applicability", 80),
            channels.clone(),
            2,
            GeneralLineageMissingEvidencePolicy::ReportInsufficientIndependentEvidence,
        )
        .unwrap();
        let design = current_design(&raw_design, history_design, &model, channels, 2);
        let evidence = GeneralLineageSpeciesEvidence::evaluate(
            &design,
            history,
            &model,
            applicability(GeneralLineageModelApplicabilityDisposition::InDomain),
            [external(
                "ecology",
                GeneralLineageChannelDisposition::SupportsSeparation,
                128,
            )],
        )
        .unwrap();
        f(&evidence, &design, history, &model);
    });
}

#[test]
fn forged_aggregate_status_cannot_canonicalize() {
    supported_evidence_fixture(|evidence, _, _, _| {
        let mut value = serde_json::to_value(evidence).unwrap();
        value["status"] = serde_json::json!("NotSupportedUnderGeneralLineageModel");
        let restored: GeneralLineageSpeciesEvidence = serde_json::from_value(value).unwrap();
        assert!(matches!(
            restored.canonical_digest(),
            Err(GeneralLineageEvidenceError::StatusInvariant)
        ));
    });
}

#[test]
fn forged_support_group_summary_cannot_canonicalize() {
    supported_evidence_fixture(|evidence, _, _, _| {
        let mut value = serde_json::to_value(evidence).unwrap();
        value["supporting_dependency_groups"] = serde_json::json!([]);
        let restored: GeneralLineageSpeciesEvidence = serde_json::from_value(value).unwrap();
        assert!(matches!(
            restored.canonical_digest(),
            Err(GeneralLineageEvidenceError::DependencyGroupInvariant)
        ));
    });
}

#[test]
fn exact_current_replay_rejects_external_qualification_drift() {
    supported_evidence_fixture(|evidence, design, history, model| {
        ValidatedGeneralLineageSpeciesEvidence::validate_current(
            evidence,
            design,
            history,
            model,
            applicability(GeneralLineageModelApplicabilityDisposition::InDomain),
            [external(
                "ecology",
                GeneralLineageChannelDisposition::SupportsSeparation,
                128,
            )],
        )
        .unwrap();

        assert!(matches!(
            ValidatedGeneralLineageSpeciesEvidence::validate_current(
                evidence,
                design,
                history,
                model,
                applicability(GeneralLineageModelApplicabilityDisposition::InDomain),
                [external(
                    "ecology",
                    GeneralLineageChannelDisposition::SupportsSeparation,
                    130,
                )],
            ),
            Err(GeneralLineageEvidenceError::ReplayMismatch)
        ));
    });
}
