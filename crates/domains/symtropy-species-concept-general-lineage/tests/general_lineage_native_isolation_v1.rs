include!("general_lineage_v1.rs");

mod isolation_fixture {
    include!("../../symtropy-evolution-core/tests/reproductive_isolation_evidence_v1.rs");

    fn with_case(
        first: ContactKind,
        second: ContactKind,
        expected: ReproductiveIsolationStatus,
        f: impl FnOnce(&ValidatedReproductiveIsolationEvidence<'_>),
    ) {
        let ctx = context(150);
        // Seed 20 intentionally matches SEL-10A fixture lineage-a/lineage-b authorities.
        let auth = authorities(20);
        let a = iso_contact(&auth, ctx, "general-lineage-a", 10);
        let b = iso_contact(&auth, ctx, "general-lineage-b", 20);
        let a_design = iso_current_design(&a, &auth, ctx);
        let b_design = iso_current_design(&b, &auth, ctx);
        let design = iso_design(&a_design, &b_design, 30);
        let current_design = iso_current_isolation_design(&design, &a_design, &b_design, 30);

        let a_study = iso_study(&a, &auth, ctx, first, 40);
        let b_study = iso_study(&b, &auth, ctx, second, 60);
        let current_a = iso_current_study(&a, &a_study, &auth, ctx, first, 40);
        let current_b = iso_current_study(&b, &b_study, &auth, ctx, second, 60);
        let evidence = two_study_evidence(&current_design, &current_a, &current_b);
        let validated = ValidatedReproductiveIsolationEvidence::validate_current(
            &evidence,
            &current_design,
            vec![
                IsolationStudyEvidenceInput {
                    unit_id: IsolationStudyUnitId::new("study-b").unwrap(),
                    study: &current_b,
                },
                IsolationStudyEvidenceInput {
                    unit_id: IsolationStudyUnitId::new("study-a").unwrap(),
                    study: &current_a,
                },
            ],
        )
        .unwrap();
        assert_eq!(validated.evidence().status, expected);
        f(&validated);
    }

    pub(super) fn with_supported(
        f: impl FnOnce(&ValidatedReproductiveIsolationEvidence<'_>),
    ) {
        with_case(
            ContactKind::Barrier,
            ContactKind::Barrier,
            ReproductiveIsolationStatus::Supported,
            f,
        );
    }

    pub(super) fn with_contradicted(
        f: impl FnOnce(&ValidatedReproductiveIsolationEvidence<'_>),
    ) {
        with_case(
            ContactKind::Fertile,
            ContactKind::Barrier,
            ReproductiveIsolationStatus::Contradicted,
            f,
        );
    }
}

#[test]
fn preregistered_reproductive_isolation_channel_consumes_native_sel09b_authority() {
    lineage_fixture::with_clean(|history_design, history| {
        isolation_fixture::with_supported(|isolation| {
            let raw_model = model(GeneralLineageReproductiveModePolicy::SexualOnly, 110);
            let model = current_model(
                &raw_model,
                GeneralLineageReproductiveModePolicy::SexualOnly,
                110,
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
                GeneralLineageClassificationId::new("typed-isolation").unwrap(),
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
                [GeneralLineageChannelEvidenceInput::ReproductiveIsolation {
                    channel_id: GeneralLineageEvidenceChannelId::new("reproductive-isolation")
                        .unwrap(),
                    evidence: isolation,
                }],
            )
            .unwrap();
            assert_eq!(
                evidence.status,
                GeneralLineageSpeciesStatus::SupportedUnderGeneralLineageModel
            );
            assert_eq!(evidence.supporting_dependency_groups.len(), 2);
        });
    });
}

#[test]
fn complete_isolation_contradiction_is_not_general_lineage_contradiction() {
    lineage_fixture::with_clean(|history_design, history| {
        isolation_fixture::with_contradicted(|isolation| {
            let raw_model = model(GeneralLineageReproductiveModePolicy::SexualOnly, 111);
            let model = current_model(
                &raw_model,
                GeneralLineageReproductiveModePolicy::SexualOnly,
                111,
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
                GeneralLineageClassificationId::new("isolation-not-veto").unwrap(),
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
                [GeneralLineageChannelEvidenceInput::ReproductiveIsolation {
                    channel_id: GeneralLineageEvidenceChannelId::new("reproductive-isolation")
                        .unwrap(),
                    evidence: isolation,
                }],
            )
            .unwrap();

            assert_eq!(
                evidence.channels[1].disposition,
                GeneralLineageChannelDisposition::DoesNotSupportSeparation
            );
            assert_eq!(
                evidence.status,
                GeneralLineageSpeciesStatus::NotSupportedUnderGeneralLineageModel
            );
        });
    });
}

#[test]
fn independent_corroboration_can_support_despite_complete_isolation_failure() {
    lineage_fixture::with_clean(|history_design, history| {
        isolation_fixture::with_contradicted(|isolation| {
            let raw_model = model(GeneralLineageReproductiveModePolicy::SexualOnly, 112);
            let model = current_model(
                &raw_model,
                GeneralLineageReproductiveModePolicy::SexualOnly,
                112,
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
                    "ecology",
                    GeneralLineageEvidenceChannelKind::EcologicalDifferentiation,
                    GeneralLineageEvidenceChannelRole::Corroborating,
                    "ecology",
                    2,
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
                GeneralLineageClassificationId::new("isolation-failure-with-corroboration")
                    .unwrap(),
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
                [
                    external(
                        "ecology",
                        GeneralLineageChannelDisposition::SupportsSeparation,
                        113,
                    ),
                    GeneralLineageChannelEvidenceInput::ReproductiveIsolation {
                        channel_id: GeneralLineageEvidenceChannelId::new(
                            "reproductive-isolation",
                        )
                        .unwrap(),
                        evidence: isolation,
                    },
                ],
            )
            .unwrap();

            assert_eq!(
                evidence.status,
                GeneralLineageSpeciesStatus::SupportedUnderGeneralLineageModel
            );
            assert_eq!(evidence.supporting_dependency_groups.len(), 2);
        });
    });
}
