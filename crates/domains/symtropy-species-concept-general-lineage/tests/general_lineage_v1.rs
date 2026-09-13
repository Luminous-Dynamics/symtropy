use symtropy_evolution_core::{AnalysisAuthorityRef, AnalysisContentDigest, AnalysisMethodId};
use symtropy_species_concept_general_lineage::*;

mod lineage_fixture {
    include!("../../symtropy-evolution-core/tests/lineage_divergence_history_v1.rs");

    fn case_inputs<'a>(
        fixture: &'a HistoryFixture,
        middle: GenerationKind,
    ) -> Vec<LineageHistoryGenerationInput<'a>> {
        vec![
            observed_input(fixture, 1, GenerationKind::Clean),
            observed_input(fixture, 2, middle),
            observed_input(fixture, 3, GenerationKind::Clean),
        ]
    }

    pub(super) fn with_clean(
        f: impl FnOnce(
            &ValidatedLineageDivergenceHistoryDesign<'_>,
            &ValidatedLineageDivergenceHistory<'_>,
        ),
    ) {
        with_middle(GenerationKind::Clean, f)
    }

    pub(super) fn with_recontact(
        f: impl FnOnce(
            &ValidatedLineageDivergenceHistoryDesign<'_>,
            &ValidatedLineageDivergenceHistory<'_>,
        ),
    ) {
        with_middle(GenerationKind::Recontact, f)
    }

    pub(super) fn with_fusion(
        f: impl FnOnce(
            &ValidatedLineageDivergenceHistoryDesign<'_>,
            &ValidatedLineageDivergenceHistory<'_>,
        ),
    ) {
        with_middle(GenerationKind::Fusion, f)
    }

    fn with_middle(
        middle: GenerationKind,
        f: impl FnOnce(
            &ValidatedLineageDivergenceHistoryDesign<'_>,
            &ValidatedLineageDivergenceHistory<'_>,
        ),
    ) {
        let fixture = fixture();
        let design = design_with(
            LineageHistoryMissingPolicy::ReportInsufficientEvidence,
            LineageHistoryContextPolicy::ExactAcrossInterval,
        );
        let current_design = current_design(&design);
        let history = LineageDivergenceHistory::capture(
            &current_design,
            case_inputs(&fixture, middle),
        )
        .unwrap();
        let current_history = ValidatedLineageDivergenceHistory::validate_current(
            &history,
            &current_design,
            case_inputs(&fixture, middle),
        )
        .unwrap();
        f(&current_design, &current_history);
    }
}

fn auth(name: &str, byte: u8) -> AnalysisAuthorityRef {
    AnalysisAuthorityRef::new(
        AnalysisMethodId::new(name).unwrap(),
        1,
        AnalysisContentDigest::new([byte; 32]),
    )
}

fn model(
    mode: GeneralLineageReproductiveModePolicy,
    qualification_byte: u8,
) -> GeneralLineageSpeciesModel {
    GeneralLineageSpeciesModel::declare(
        GeneralLineageSpeciesModelId::new("general-lineage-v1").unwrap(),
        GeneralLineageValidityDomainRef::new(
            auth("general-lineage-validity", 70),
            mode,
        )
        .unwrap(),
        auth("general-lineage-model-qualification", qualification_byte),
    )
    .unwrap()
}

fn current_model<'a>(
    model: &'a GeneralLineageSpeciesModel,
    mode: GeneralLineageReproductiveModePolicy,
    qualification_byte: u8,
) -> ValidatedGeneralLineageSpeciesModel<'a> {
    ValidatedGeneralLineageSpeciesModel::validate_current(
        model,
        GeneralLineageValidityDomainRef::new(
            auth("general-lineage-validity", 70),
            mode,
        )
        .unwrap(),
        auth("general-lineage-model-qualification", qualification_byte),
    )
    .unwrap()
}

fn channel(
    id: &str,
    kind: GeneralLineageEvidenceChannelKind,
    role: GeneralLineageEvidenceChannelRole,
    group: &str,
    byte: u8,
) -> GeneralLineageEvidenceChannelDeclaration {
    GeneralLineageEvidenceChannelDeclaration::new(
        GeneralLineageEvidenceChannelId::new(id).unwrap(),
        kind,
        role,
        GeneralLineageEvidenceDependencyGroupId::new(group).unwrap(),
        auth(&format!("{id}-protocol"), byte),
        auth(&format!("{id}-applicability"), byte.wrapping_add(1)),
    )
    .unwrap()
}

fn base_channels() -> Vec<GeneralLineageEvidenceChannelDeclaration> {
    vec![
        channel(
            "lineage-history",
            GeneralLineageEvidenceChannelKind::LongitudinalLineageSeparation,
            GeneralLineageEvidenceChannelRole::CoreRequired,
            "history",
            1,
        ),
        channel(
            "ecology",
            GeneralLineageEvidenceChannelKind::EcologicalDifferentiation,
            GeneralLineageEvidenceChannelRole::Corroborating,
            "ecology",
            3,
        ),
    ]
}

fn current_design<'a>(
    raw: &'a GeneralLineageClassificationDesign,
    history: &symtropy_evolution_core::ValidatedLineageDivergenceHistoryDesign<'_>,
    model: &ValidatedGeneralLineageSpeciesModel<'_>,
    channels: Vec<GeneralLineageEvidenceChannelDeclaration>,
    threshold: u32,
) -> ValidatedGeneralLineageClassificationDesign<'a> {
    ValidatedGeneralLineageClassificationDesign::validate_current(
        raw,
        history,
        model,
        auth("general-lineage-target-applicability", 80),
        channels,
        threshold,
        GeneralLineageMissingEvidencePolicy::ReportInsufficientIndependentEvidence,
    )
    .unwrap()
}

fn applicability(
    disposition: GeneralLineageModelApplicabilityDisposition,
) -> GeneralLineageModelApplicabilityInput {
    GeneralLineageModelApplicabilityInput {
        disposition,
        evidence_authority: auth("target-applicability-evidence", 81),
        qualification_authority: auth("target-applicability-qualification", 82),
    }
}

fn external(
    id: &str,
    disposition: GeneralLineageChannelDisposition,
    byte: u8,
) -> GeneralLineageChannelEvidenceInput<'static, 'static> {
    GeneralLineageChannelEvidenceInput::External {
        channel_id: GeneralLineageEvidenceChannelId::new(id).unwrap(),
        disposition,
        evidence_authority: auth(&format!("{id}-evidence"), byte),
        qualification_authority: auth(&format!("{id}-qualification"), byte.wrapping_add(1)),
    }
}

#[test]
fn general_lineage_model_projects_a_distinct_open_family_and_replays() {
    let raw = model(GeneralLineageReproductiveModePolicy::SexualOrAsexual, 71);
    let current = current_model(
        &raw,
        GeneralLineageReproductiveModePolicy::SexualOrAsexual,
        71,
    );
    let descriptor = general_lineage_family_descriptor(&current).unwrap();
    assert_eq!(
        descriptor.conceptual_identity.family_id.as_str(),
        "general-lineage-species"
    );
    assert_eq!(descriptor.capabilities.len(), 1);
    let restored = serde_json::from_slice(&serde_json::to_vec(&descriptor).unwrap()).unwrap();
    let validated =
        ValidatedGeneralLineageFamilyDescriptor::validate_current(&restored, &current).unwrap();
    assert_eq!(
        validated.conceptual_identity().family_id.as_str(),
        "general-lineage-species"
    );
}

#[test]
fn asexual_in_domain_target_can_be_supported_without_reproductive_isolation() {
    lineage_fixture::with_clean(|history_design, history| {
        let raw_model = model(GeneralLineageReproductiveModePolicy::AsexualOnly, 72);
        let model = current_model(
            &raw_model,
            GeneralLineageReproductiveModePolicy::AsexualOnly,
            72,
        );
        let channels = base_channels();
        let raw_design = GeneralLineageClassificationDesign::declare(
            GeneralLineageClassificationId::new("asexual-target").unwrap(),
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
                83,
            )],
        )
        .unwrap();
        assert_eq!(
            evidence.status,
            GeneralLineageSpeciesStatus::SupportedUnderGeneralLineageModel
        );
        assert_eq!(evidence.supporting_dependency_groups.len(), 2);
    });
}

#[test]
fn repeated_metrics_from_one_dependency_group_do_not_inflate_corroboration() {
    lineage_fixture::with_clean(|history_design, history| {
        let raw_model = model(GeneralLineageReproductiveModePolicy::SexualOrAsexual, 73);
        let model = current_model(
            &raw_model,
            GeneralLineageReproductiveModePolicy::SexualOrAsexual,
            73,
        );
        let channels = vec![
            channel(
                "lineage-history",
                GeneralLineageEvidenceChannelKind::LongitudinalLineageSeparation,
                GeneralLineageEvidenceChannelRole::CoreRequired,
                "history",
                1,
            ),
            channel(
                "genetic-a",
                GeneralLineageEvidenceChannelKind::GeneticDiagnosability,
                GeneralLineageEvidenceChannelRole::Corroborating,
                "same-sequence-dataset",
                10,
            ),
            channel(
                "genetic-b",
                GeneralLineageEvidenceChannelKind::GeneticDiagnosability,
                GeneralLineageEvidenceChannelRole::Corroborating,
                "same-sequence-dataset",
                12,
            ),
            channel(
                "ecology",
                GeneralLineageEvidenceChannelKind::EcologicalDifferentiation,
                GeneralLineageEvidenceChannelRole::Corroborating,
                "ecology",
                14,
            ),
        ];
        let raw_design = GeneralLineageClassificationDesign::declare(
            GeneralLineageClassificationId::new("dependency-groups").unwrap(),
            history_design,
            &model,
            auth("general-lineage-target-applicability", 80),
            channels.clone(),
            3,
            GeneralLineageMissingEvidencePolicy::ReportInsufficientIndependentEvidence,
        )
        .unwrap();
        let design = current_design(&raw_design, history_design, &model, channels, 3);
        let evidence = GeneralLineageSpeciesEvidence::evaluate(
            &design,
            history,
            &model,
            applicability(GeneralLineageModelApplicabilityDisposition::InDomain),
            [
                external(
                    "genetic-a",
                    GeneralLineageChannelDisposition::SupportsSeparation,
                    84,
                ),
                external(
                    "genetic-b",
                    GeneralLineageChannelDisposition::SupportsSeparation,
                    86,
                ),
                external(
                    "ecology",
                    GeneralLineageChannelDisposition::DoesNotSupportSeparation,
                    88,
                ),
            ],
        )
        .unwrap();
        assert_eq!(
            evidence.status,
            GeneralLineageSpeciesStatus::NotSupportedUnderGeneralLineageModel
        );
        assert_eq!(evidence.supporting_dependency_groups.len(), 2);
    });
}

#[test]
fn unavailable_independent_group_is_insufficient_only_when_it_could_change_the_threshold() {
    lineage_fixture::with_clean(|history_design, history| {
        let raw_model = model(GeneralLineageReproductiveModePolicy::SexualOrAsexual, 74);
        let model = current_model(
            &raw_model,
            GeneralLineageReproductiveModePolicy::SexualOrAsexual,
            74,
        );
        let channels = vec![
            channel(
                "lineage-history",
                GeneralLineageEvidenceChannelKind::LongitudinalLineageSeparation,
                GeneralLineageEvidenceChannelRole::CoreRequired,
                "history",
                1,
            ),
            channel(
                "genetics",
                GeneralLineageEvidenceChannelKind::GeneticDiagnosability,
                GeneralLineageEvidenceChannelRole::Corroborating,
                "genetics",
                20,
            ),
            channel(
                "ecology",
                GeneralLineageEvidenceChannelKind::EcologicalDifferentiation,
                GeneralLineageEvidenceChannelRole::Corroborating,
                "ecology",
                22,
            ),
        ];
        let raw_design = GeneralLineageClassificationDesign::declare(
            GeneralLineageClassificationId::new("counterfactual-missingness").unwrap(),
            history_design,
            &model,
            auth("general-lineage-target-applicability", 80),
            channels.clone(),
            3,
            GeneralLineageMissingEvidencePolicy::ReportInsufficientIndependentEvidence,
        )
        .unwrap();
        let design = current_design(&raw_design, history_design, &model, channels, 3);
        let evidence = GeneralLineageSpeciesEvidence::evaluate(
            &design,
            history,
            &model,
            applicability(GeneralLineageModelApplicabilityDisposition::InDomain),
            [
                external(
                    "genetics",
                    GeneralLineageChannelDisposition::SupportsSeparation,
                    90,
                ),
                external(
                    "ecology",
                    GeneralLineageChannelDisposition::Unavailable,
                    92,
                ),
            ],
        )
        .unwrap();
        assert_eq!(
            evidence.status,
            GeneralLineageSpeciesStatus::InsufficientIndependentEvidence
        );
        assert_eq!(evidence.supporting_dependency_groups.len(), 2);
        assert_eq!(evidence.unavailable_potential_dependency_groups.len(), 1);
    });
}

#[test]
fn recontact_can_remain_supported_when_lineages_persist() {
    lineage_fixture::with_recontact(|history_design, history| {
        assert_eq!(
            history.history().status,
            symtropy_evolution_core::LineageDivergenceHistoryStatus::DivergenceWithRecontact
        );
        let raw_model = model(GeneralLineageReproductiveModePolicy::SexualOrAsexual, 75);
        let model = current_model(
            &raw_model,
            GeneralLineageReproductiveModePolicy::SexualOrAsexual,
            75,
        );
        let channels = base_channels();
        let raw_design = GeneralLineageClassificationDesign::declare(
            GeneralLineageClassificationId::new("recontact-target").unwrap(),
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
                94,
            )],
        )
        .unwrap();
        assert_eq!(
            evidence.status,
            GeneralLineageSpeciesStatus::SupportedUnderGeneralLineageModel
        );
    });
}

#[test]
fn lineage_fusion_contradicts_even_with_positive_external_channels() {
    lineage_fixture::with_fusion(|history_design, history| {
        let raw_model = model(GeneralLineageReproductiveModePolicy::SexualOrAsexual, 76);
        let model = current_model(
            &raw_model,
            GeneralLineageReproductiveModePolicy::SexualOrAsexual,
            76,
        );
        let channels = base_channels();
        let raw_design = GeneralLineageClassificationDesign::declare(
            GeneralLineageClassificationId::new("fusion-target").unwrap(),
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
                96,
            )],
        )
        .unwrap();
        assert_eq!(
            evidence.status,
            GeneralLineageSpeciesStatus::ContradictedUnderGeneralLineageModel
        );
    });
}

#[test]
fn outside_model_domain_is_not_a_negative_species_classification() {
    lineage_fixture::with_clean(|history_design, history| {
        let raw_model = model(GeneralLineageReproductiveModePolicy::SexualOnly, 77);
        let model = current_model(
            &raw_model,
            GeneralLineageReproductiveModePolicy::SexualOnly,
            77,
        );
        let channels = base_channels();
        let raw_design = GeneralLineageClassificationDesign::declare(
            GeneralLineageClassificationId::new("outside-domain").unwrap(),
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
            applicability(
                GeneralLineageModelApplicabilityDisposition::OutsideModelValidityDomain,
            ),
            [external(
                "ecology",
                GeneralLineageChannelDisposition::SupportsSeparation,
                98,
            )],
        )
        .unwrap();
        assert_eq!(
            evidence.status,
            GeneralLineageSpeciesStatus::OutsideModelValidityDomain
        );
    });
}

#[test]
fn opaque_external_reproductive_isolation_cannot_bypass_native_sel09b_authority() {
    lineage_fixture::with_clean(|history_design, history| {
        let raw_model = model(GeneralLineageReproductiveModePolicy::SexualOnly, 78);
        let model = current_model(
            &raw_model,
            GeneralLineageReproductiveModePolicy::SexualOnly,
            78,
        );
        let channels = vec![
            channel(
                "lineage-history",
                GeneralLineageEvidenceChannelKind::LongitudinalLineageSeparation,
                GeneralLineageEvidenceChannelRole::CoreRequired,
                "history",
                1,
            ),
            channel(
                "reproductive-isolation",
                GeneralLineageEvidenceChannelKind::ReproductiveIsolation,
                GeneralLineageEvidenceChannelRole::Corroborating,
                "reproductive-isolation",
                30,
            ),
        ];
        let raw_design = GeneralLineageClassificationDesign::declare(
            GeneralLineageClassificationId::new("native-isolation-required").unwrap(),
            history_design,
            &model,
            auth("general-lineage-target-applicability", 80),
            channels.clone(),
            2,
            GeneralLineageMissingEvidencePolicy::ReportInsufficientIndependentEvidence,
        )
        .unwrap();
        let design = current_design(&raw_design, history_design, &model, channels, 2);
        assert!(matches!(
            GeneralLineageSpeciesEvidence::evaluate(
                &design,
                history,
                &model,
                applicability(GeneralLineageModelApplicabilityDisposition::InDomain),
                [external(
                    "reproductive-isolation",
                    GeneralLineageChannelDisposition::SupportsSeparation,
                    100,
                )],
            ),
            Err(GeneralLineageEvidenceError::NativeIsolationAuthorityRequired)
        ));
    });
}

#[test]
fn wire_shape_contains_no_scalar_score_or_historical_speciation_claim() {
    lineage_fixture::with_clean(|history_design, history| {
        let raw_model = model(GeneralLineageReproductiveModePolicy::SexualOrAsexual, 79);
        let model = current_model(
            &raw_model,
            GeneralLineageReproductiveModePolicy::SexualOrAsexual,
            79,
        );
        let channels = base_channels();
        let raw_design = GeneralLineageClassificationDesign::declare(
            GeneralLineageClassificationId::new("wire-shape").unwrap(),
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
                102,
            )],
        )
        .unwrap();
        let json = serde_json::to_string(&evidence).unwrap();
        for forbidden in [
            "species_score",
            "confidence_score",
            "transition_generation",
            "speciation_time",
            "historical_speciation_status",
            "universal_taxonomy",
        ] {
            assert!(!json.contains(forbidden));
        }
    });
}
