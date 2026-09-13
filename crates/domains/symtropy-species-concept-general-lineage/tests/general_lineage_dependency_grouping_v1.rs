include!("general_lineage_v1.rs");

fn channel_with_group_qualification(
    id: &str,
    kind: GeneralLineageEvidenceChannelKind,
    role: GeneralLineageEvidenceChannelRole,
    group: &str,
    group_qualification: AnalysisAuthorityRef,
    byte: u8,
) -> GeneralLineageEvidenceChannelDeclaration {
    GeneralLineageEvidenceChannelDeclaration::new(
        GeneralLineageEvidenceChannelId::new(id).unwrap(),
        kind,
        role,
        GeneralLineageEvidenceDependencyGroupId::new(group).unwrap(),
        group_qualification,
        auth(&format!("{id}-protocol"), byte),
        auth(&format!("{id}-applicability"), byte.wrapping_add(1)),
    )
    .unwrap()
}

#[test]
fn one_group_cannot_carry_conflicting_grouping_qualifications() {
    lineage_fixture::with_clean(|history_design, _| {
        let raw_model = model(GeneralLineageReproductiveModePolicy::SexualOrAsexual, 140);
        let model = current_model(
            &raw_model,
            GeneralLineageReproductiveModePolicy::SexualOrAsexual,
            140,
        );
        let channels = vec![
            channel(
                "lineage-history",
                GeneralLineageEvidenceChannelKind::LongitudinalLineageSeparation,
                GeneralLineageEvidenceChannelRole::CoreRequired,
                "history",
                1,
            ),
            channel_with_group_qualification(
                "genetic-a",
                GeneralLineageEvidenceChannelKind::GeneticDiagnosability,
                GeneralLineageEvidenceChannelRole::Corroborating,
                "same-sequence-dataset",
                auth("grouping-a", 10),
                10,
            ),
            channel_with_group_qualification(
                "genetic-b",
                GeneralLineageEvidenceChannelKind::GeneticDiagnosability,
                GeneralLineageEvidenceChannelRole::Corroborating,
                "same-sequence-dataset",
                auth("grouping-b", 11),
                12,
            ),
        ];
        assert!(matches!(
            GeneralLineageClassificationDesign::declare(
                GeneralLineageClassificationId::new("conflicting-group-qualification").unwrap(),
                history_design,
                &model,
                auth("general-lineage-target-applicability", 80),
                channels,
                2,
                GeneralLineageMissingEvidencePolicy::ReportInsufficientIndependentEvidence,
            ),
            Err(GeneralLineageDesignError::DependencyGroupQualificationMismatch(_))
        ));
    });
}

#[test]
fn one_grouping_qualification_cannot_be_relabelled_as_multiple_groups() {
    lineage_fixture::with_clean(|history_design, _| {
        let raw_model = model(GeneralLineageReproductiveModePolicy::SexualOrAsexual, 141);
        let model = current_model(
            &raw_model,
            GeneralLineageReproductiveModePolicy::SexualOrAsexual,
            141,
        );
        let shared = auth("same-dependency-profile", 20);
        let channels = vec![
            channel(
                "lineage-history",
                GeneralLineageEvidenceChannelKind::LongitudinalLineageSeparation,
                GeneralLineageEvidenceChannelRole::CoreRequired,
                "history",
                1,
            ),
            channel_with_group_qualification(
                "genetics",
                GeneralLineageEvidenceChannelKind::GeneticDiagnosability,
                GeneralLineageEvidenceChannelRole::Corroborating,
                "genetics",
                shared.clone(),
                20,
            ),
            channel_with_group_qualification(
                "ecology",
                GeneralLineageEvidenceChannelKind::EcologicalDifferentiation,
                GeneralLineageEvidenceChannelRole::Corroborating,
                "ecology",
                shared,
                22,
            ),
        ];
        assert!(matches!(
            GeneralLineageClassificationDesign::declare(
                GeneralLineageClassificationId::new("renamed-dependency-profile").unwrap(),
                history_design,
                &model,
                auth("general-lineage-target-applicability", 80),
                channels,
                2,
                GeneralLineageMissingEvidencePolicy::ReportInsufficientIndependentEvidence,
            ),
            Err(GeneralLineageDesignError::DependencyQualificationReusedAcrossGroups)
        ));
    });
}

#[test]
fn grouping_qualification_drift_changes_design_identity_and_stales_replay() {
    lineage_fixture::with_clean(|history_design, _| {
        let raw_model = model(GeneralLineageReproductiveModePolicy::SexualOrAsexual, 142);
        let model = current_model(
            &raw_model,
            GeneralLineageReproductiveModePolicy::SexualOrAsexual,
            142,
        );
        let channels = base_channels();
        let design = GeneralLineageClassificationDesign::declare(
            GeneralLineageClassificationId::new("group-qualification-drift").unwrap(),
            history_design,
            &model,
            auth("general-lineage-target-applicability", 80),
            channels.clone(),
            2,
            GeneralLineageMissingEvidencePolicy::ReportInsufficientIndependentEvidence,
        )
        .unwrap();
        let original_digest = design.canonical_digest().unwrap();

        let mut value = serde_json::to_value(&design).unwrap();
        value["channels"][0]["dependency_group_qualification_authority"]["content_digest"] =
            serde_json::json!(vec![201u8; 32]);
        let drifted: GeneralLineageClassificationDesign = serde_json::from_value(value).unwrap();
        let drifted_digest = drifted.canonical_digest().unwrap();
        assert_ne!(original_digest, drifted_digest);

        assert!(matches!(
            ValidatedGeneralLineageClassificationDesign::validate_current(
                &drifted,
                history_design,
                &model,
                auth("general-lineage-target-applicability", 80),
                channels,
                2,
                GeneralLineageMissingEvidencePolicy::ReportInsufficientIndependentEvidence,
            ),
            Err(GeneralLineageDesignError::DesignReplayMismatch)
        ));
    });
}

#[test]
fn dependency_grouping_rule_tamper_is_rejected_locally() {
    lineage_fixture::with_clean(|history_design, _| {
        let raw_model = model(GeneralLineageReproductiveModePolicy::SexualOrAsexual, 143);
        let model = current_model(
            &raw_model,
            GeneralLineageReproductiveModePolicy::SexualOrAsexual,
            143,
        );
        let design = GeneralLineageClassificationDesign::declare(
            GeneralLineageClassificationId::new("group-rule-tamper").unwrap(),
            history_design,
            &model,
            auth("general-lineage-target-applicability", 80),
            base_channels(),
            2,
            GeneralLineageMissingEvidencePolicy::ReportInsufficientIndependentEvidence,
        )
        .unwrap();
        let mut value = serde_json::to_value(design).unwrap();
        value["dependency_grouping_rule_authority"]["revision"] = serde_json::json!(2);
        let restored: GeneralLineageClassificationDesign = serde_json::from_value(value).unwrap();
        assert!(matches!(
            restored.canonical_digest(),
            Err(GeneralLineageDesignError::DependencyGroupingRuleMismatch)
        ));
    });
}
