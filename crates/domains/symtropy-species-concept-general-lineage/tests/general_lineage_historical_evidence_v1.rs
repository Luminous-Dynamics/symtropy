include!("general_lineage_v1.rs");

mod five_generation_history {
    include!("../../symtropy-evolution-core/tests/lineage_divergence_history_v1.rs");

    fn fixture_five() -> HistoryFixture {
        let schema = schema();
        let experiment = EvolutionExperimentId::new("lineage-history-five-generation-fixture").unwrap();
        let mut a_populations = Vec::new();
        let mut b_populations = Vec::new();
        let mut a_points = Vec::new();
        let mut b_points = Vec::new();
        for generation in 1..=5 {
            let a = population(&schema, "lineage-a-pop", 3, 1);
            let b = population(&schema, "lineage-b-pop", 1, 3);
            a_points.push(
                PopulationTrajectoryPoint::declare_reference_start(
                    &schema,
                    &a,
                    experiment.clone(),
                    PopulationGeneration(generation),
                )
                .unwrap(),
            );
            b_points.push(
                PopulationTrajectoryPoint::declare_reference_start(
                    &schema,
                    &b,
                    experiment.clone(),
                    PopulationGeneration(generation),
                )
                .unwrap(),
            );
            a_populations.push(a);
            b_populations.push(b);
        }
        HistoryFixture {
            schema,
            a_populations,
            b_populations,
            a_points,
            b_points,
        }
    }

    fn design_five() -> LineageDivergenceHistoryDesign {
        LineageDivergenceHistoryDesign::declare(
            LineageDivergenceHistoryId::new("history-a-b-five-generation").unwrap(),
            authority("lineage-a", 20),
            authority("lineage-b", 21),
            PopulationGeneration(1),
            PopulationGeneration(5),
            protocols(),
            LineageHistoryContextPolicy::ExactAcrossInterval,
            LineageHistoryMissingPolicy::ReportInsufficientEvidence,
            authority("history-completeness", 22),
        )
        .unwrap()
    }

    fn current_design_five<'a>(
        design: &'a LineageDivergenceHistoryDesign,
    ) -> ValidatedLineageDivergenceHistoryDesign<'a> {
        ValidatedLineageDivergenceHistoryDesign::validate_current(
            design,
            authority("lineage-a", 20),
            authority("lineage-b", 21),
            PopulationGeneration(1),
            PopulationGeneration(5),
            protocols(),
            LineageHistoryContextPolicy::ExactAcrossInterval,
            LineageHistoryMissingPolicy::ReportInsufficientEvidence,
            authority("history-completeness", 22),
        )
        .unwrap()
    }

    fn inputs<'a>(
        fixture: &'a HistoryFixture,
        fifth: GenerationKind,
    ) -> Vec<LineageHistoryGenerationInput<'a>> {
        vec![
            observed_input(fixture, 1, GenerationKind::Clean),
            observed_input(fixture, 2, GenerationKind::Clean),
            observed_input(fixture, 3, GenerationKind::Clean),
            observed_input(fixture, 4, GenerationKind::Clean),
            observed_input(fixture, 5, fifth),
        ]
    }

    fn with_case(
        fifth: GenerationKind,
        f: impl FnOnce(
            &ValidatedLineageDivergenceHistoryDesign<'_>,
            &ValidatedLineageDivergenceHistory<'_>,
        ),
    ) {
        let fixture = fixture_five();
        let design = design_five();
        let current_design = current_design_five(&design);
        let history = LineageDivergenceHistory::capture(&current_design, inputs(&fixture, fifth)).unwrap();
        let current_history = ValidatedLineageDivergenceHistory::validate_current(
            &history,
            &current_design,
            inputs(&fixture, fifth),
        )
        .unwrap();
        f(&current_design, &current_history);
    }

    pub(super) fn with_clean(
        f: impl FnOnce(
            &ValidatedLineageDivergenceHistoryDesign<'_>,
            &ValidatedLineageDivergenceHistory<'_>,
        ),
    ) {
        with_case(GenerationKind::Clean, f)
    }

    pub(super) fn with_fusion_after(
        f: impl FnOnce(
            &ValidatedLineageDivergenceHistoryDesign<'_>,
            &ValidatedLineageDivergenceHistory<'_>,
        ),
    ) {
        with_case(GenerationKind::Fusion, f)
    }
}

mod historical_isolation_fixture {
    include!("../../symtropy-evolution-core/tests/reproductive_isolation_evidence_v1.rs");

    pub(super) fn with_contradicted(
        f: impl FnOnce(&ValidatedReproductiveIsolationEvidence<'_>),
    ) {
        let ctx = context(150);
        let auth = authorities(20);
        let a = iso_contact(&auth, ctx, "f1a-a", 10);
        let b = iso_contact(&auth, ctx, "f1a-b", 20);
        let a_design = iso_current_design(&a, &auth, ctx);
        let b_design = iso_current_design(&b, &auth, ctx);
        let design = iso_design(&a_design, &b_design, 30);
        let current_design = iso_current_isolation_design(&design, &a_design, &b_design, 30);

        let a_study = iso_study(&a, &auth, ctx, ContactKind::Fertile, 40);
        let b_study = iso_study(&b, &auth, ctx, ContactKind::Barrier, 60);
        let current_a = iso_current_study(&a, &a_study, &auth, ctx, ContactKind::Fertile, 40);
        let current_b = iso_current_study(&b, &b_study, &auth, ctx, ContactKind::Barrier, 60);
        let evidence = two_study_evidence(&current_design, &current_a, &current_b);
        let current = ValidatedReproductiveIsolationEvidence::validate_current(
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
        assert_eq!(current.evidence().status, ReproductiveIsolationStatus::Contradicted);
        f(&current);
    }
}

fn historical_design<'a>(
    raw: &'a GeneralLineageHistoricalEvidenceDesign,
    current_classification: &ValidatedGeneralLineageClassificationDesign<'_>,
    projections: Vec<HistoricalChannelProjectionInput>,
) -> ValidatedGeneralLineageHistoricalEvidenceDesign<'a> {
    ValidatedGeneralLineageHistoricalEvidenceDesign::validate_current(
        raw,
        current_classification,
        symtropy_evolution_core::PopulationGeneration(2),
        symtropy_evolution_core::PopulationGeneration(3),
        projections,
        2,
        auth("f1a-common-source-protocol", 130),
        auth("f1a-historical-context-protocol", 131),
        auth("f1a-counter-history-protocol", 132),
        GeneralLineageHistoricalMissingPolicy::ReportIncompleteEvidence,
    )
    .unwrap()
}

fn projections(
    channels: &[GeneralLineageEvidenceChannelDeclaration],
) -> Vec<HistoricalChannelProjectionInput> {
    channels
        .iter()
        .rev()
        .enumerate()
        .map(|(index, channel)| HistoricalChannelProjectionInput {
            channel_id: channel.channel_id.clone(),
            temporal_projection_protocol: auth(
                &format!("{}-historical-projection", channel.channel_id.as_str()),
                140_u8.wrapping_add(index as u8),
            ),
        })
        .collect()
}

fn external_windows(
    channel_id: &str,
    disposition: HistoricalChannelDisposition,
) -> HistoricalExternalChannelEvidenceInput {
    HistoricalExternalChannelEvidenceInput {
        channel_id: GeneralLineageEvidenceChannelId::new(channel_id).unwrap(),
        windows: vec![
            HistoricalExternalWindowEvidenceInput {
                window: HistoricalChannelWindow::AfterCandidateInterval,
                disposition,
                observation_authority: auth("f1a-external-after-observation", 150),
                qualification_authority: auth("f1a-external-after-qualification", 151),
            },
            HistoricalExternalWindowEvidenceInput {
                window: HistoricalChannelWindow::BeforeCandidateInterval,
                disposition,
                observation_authority: auth("f1a-external-before-observation", 152),
                qualification_authority: auth("f1a-external-before-qualification", 153),
            },
            HistoricalExternalWindowEvidenceInput {
                window: HistoricalChannelWindow::CandidateInterval,
                disposition,
                observation_authority: auth("f1a-external-interval-observation", 154),
                qualification_authority: auth("f1a-external-interval-qualification", 155),
            },
        ],
    }
}

fn with_classification<R>(
    history_design: &symtropy_evolution_core::ValidatedLineageDivergenceHistoryDesign<'_>,
    channels: Vec<GeneralLineageEvidenceChannelDeclaration>,
    f: impl FnOnce(
        &ValidatedGeneralLineageClassificationDesign<'_>,
        Vec<GeneralLineageEvidenceChannelDeclaration>,
    ) -> R,
) -> R {
    let raw_model = model(GeneralLineageReproductiveModePolicy::SexualOrAsexual, 120);
    let current_model = current_model(
        &raw_model,
        GeneralLineageReproductiveModePolicy::SexualOrAsexual,
        120,
    );
    let raw_design = GeneralLineageClassificationDesign::declare(
        GeneralLineageClassificationId::new("f1a-current-classification").unwrap(),
        history_design,
        &current_model,
        auth("general-lineage-target-applicability", 80),
        channels.clone(),
        2,
        GeneralLineageMissingEvidencePolicy::ReportInsufficientIndependentEvidence,
    )
    .unwrap();
    let current_design = current_design(
        &raw_design,
        history_design,
        &current_model,
        channels.clone(),
        2,
    );
    f(&current_design, channels)
}

#[test]
fn historical_design_inherits_current_channel_ontology_and_forbids_exact_event_interval() {
    five_generation_history::with_clean(|history_design, _history| {
        with_classification(history_design, base_channels(), |classification, channels| {
            let projection_inputs = projections(&channels);
            let raw = GeneralLineageHistoricalEvidenceDesign::declare(
                GeneralLineageHistoricalEvidenceDesignId::new("f1a-history").unwrap(),
                classification,
                symtropy_evolution_core::PopulationGeneration(2),
                symtropy_evolution_core::PopulationGeneration(3),
                projection_inputs.clone(),
                2,
                auth("f1a-common-source-protocol", 130),
                auth("f1a-historical-context-protocol", 131),
                auth("f1a-counter-history-protocol", 132),
                GeneralLineageHistoricalMissingPolicy::ReportIncompleteEvidence,
            )
            .unwrap();
            let current = historical_design(&raw, classification, projection_inputs);
            assert_eq!(current.design().candidate_interval_generation_count, 2);
            assert_eq!(current.design().history_start_generation.0, 1);
            assert_eq!(current.design().history_end_generation.0, 5);
            assert_eq!(current.design().channels.len(), channels.len());
            assert!(current
                .design()
                .channels
                .iter()
                .zip(&classification.design().channels)
                .all(|(historical, current)| historical.current_channel == *current));

            let one_generation = GeneralLineageHistoricalEvidenceDesign::declare(
                GeneralLineageHistoricalEvidenceDesignId::new("f1a-exact-event-forbidden").unwrap(),
                classification,
                symtropy_evolution_core::PopulationGeneration(2),
                symtropy_evolution_core::PopulationGeneration(2),
                projections(&channels),
                2,
                auth("f1a-common-source-protocol", 130),
                auth("f1a-historical-context-protocol", 131),
                auth("f1a-counter-history-protocol", 132),
                GeneralLineageHistoricalMissingPolicy::ReportIncompleteEvidence,
            );
            assert!(matches!(
                one_generation,
                Err(GeneralLineageHistoricalDesignError::ExactEventIntervalForbidden)
            ));
        })
    });
}

#[test]
fn native_lineage_history_is_preserved_as_complete_one_two_two_windows_and_replays() {
    five_generation_history::with_clean(|history_design, history| {
        with_classification(history_design, base_channels(), |classification, channels| {
            let raw = GeneralLineageHistoricalEvidenceDesign::declare(
                GeneralLineageHistoricalEvidenceDesignId::new("f1a-complete-ledger").unwrap(),
                classification,
                symtropy_evolution_core::PopulationGeneration(2),
                symtropy_evolution_core::PopulationGeneration(3),
                projections(&channels),
                2,
                auth("f1a-common-source-protocol", 130),
                auth("f1a-historical-context-protocol", 131),
                auth("f1a-counter-history-protocol", 132),
                GeneralLineageHistoricalMissingPolicy::ReportIncompleteEvidence,
            )
            .unwrap();
            let current = historical_design(&raw, classification, projections(&channels));
            let external = external_windows(
                "ecology",
                HistoricalChannelDisposition::SupportsLineageSeparation,
            );
            let ledger = GeneralLineageHistoricalEvidenceLedger::capture(
                &current,
                history,
                None,
                [external.clone()],
                auth("f1a-common-source-evidence", 160),
                auth("f1a-historical-context-evidence", 161),
                auth("f1a-counter-history-evidence", 162),
            )
            .unwrap();

            let core = &ledger.channels[0].source;
            let HistoricalChannelEvidenceSource::NativeLineageHistory(core) = core else {
                panic!("core historical channel must retain native SEL-10A history")
            };
            assert_eq!(
                core.windows
                    .iter()
                    .map(|window| window.generations.len())
                    .collect::<Vec<_>>(),
                vec![1, 2, 2]
            );

            let restored: GeneralLineageHistoricalEvidenceLedger =
                serde_json::from_slice(&serde_json::to_vec(&ledger).unwrap()).unwrap();
            let validated = ValidatedGeneralLineageHistoricalEvidence::validate_current(
                &restored,
                &current,
                history,
                None,
                [external],
                auth("f1a-common-source-evidence", 160),
                auth("f1a-historical-context-evidence", 161),
                auth("f1a-counter-history-evidence", 162),
            )
            .unwrap();
            assert_eq!(validated.ledger_digest(), ledger.canonical_digest().unwrap());
        })
    });
}

#[test]
fn native_reproductive_isolation_is_temporally_preserved_without_becoming_a_transition_verdict() {
    five_generation_history::with_clean(|history_design, history| {
        historical_isolation_fixture::with_contradicted(|isolation| {
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
            with_classification(history_design, channels, |classification, channels| {
                let raw = GeneralLineageHistoricalEvidenceDesign::declare(
                    GeneralLineageHistoricalEvidenceDesignId::new("f1a-native-isolation").unwrap(),
                    classification,
                    symtropy_evolution_core::PopulationGeneration(2),
                    symtropy_evolution_core::PopulationGeneration(3),
                    projections(&channels),
                    2,
                    auth("f1a-common-source-protocol", 130),
                    auth("f1a-historical-context-protocol", 131),
                    auth("f1a-counter-history-protocol", 132),
                    GeneralLineageHistoricalMissingPolicy::ReportIncompleteEvidence,
                )
                .unwrap();
                let current = historical_design(&raw, classification, projections(&channels));
                let ledger = GeneralLineageHistoricalEvidenceLedger::capture(
                    &current,
                    history,
                    Some(isolation),
                    [],
                    auth("f1a-common-source-evidence", 160),
                    auth("f1a-historical-context-evidence", 161),
                    auth("f1a-counter-history-evidence", 162),
                )
                .unwrap();
                let isolation_source = ledger
                    .channels
                    .iter()
                    .find(|record| {
                        record.declaration.current_channel.kind
                            == GeneralLineageEvidenceChannelKind::ReproductiveIsolation
                    })
                    .unwrap();
                let HistoricalChannelEvidenceSource::NativeReproductiveIsolation(source) =
                    &isolation_source.source
                else {
                    panic!("reproductive-isolation historical channel must use SEL-09B")
                };
                assert_eq!(source.evidence_digest, isolation.evidence_digest());
                assert_eq!(source.outside_history_opportunity_count, 0);
                assert_eq!(
                    source
                        .windows
                        .iter()
                        .map(|window| window.opportunities.len())
                        .collect::<Vec<_>>(),
                    vec![0, 2, 2]
                );
            })
        })
    });
}

#[test]
fn post_interval_fusion_remains_explicit_counterhistory_in_the_native_ledger() {
    five_generation_history::with_fusion_after(|history_design, history| {
        with_classification(history_design, base_channels(), |classification, channels| {
            let raw = GeneralLineageHistoricalEvidenceDesign::declare(
                GeneralLineageHistoricalEvidenceDesignId::new("f1a-fusion-counterhistory").unwrap(),
                classification,
                symtropy_evolution_core::PopulationGeneration(2),
                symtropy_evolution_core::PopulationGeneration(3),
                projections(&channels),
                2,
                auth("f1a-common-source-protocol", 130),
                auth("f1a-historical-context-protocol", 131),
                auth("f1a-counter-history-protocol", 132),
                GeneralLineageHistoricalMissingPolicy::ReportIncompleteEvidence,
            )
            .unwrap();
            let current = historical_design(&raw, classification, projections(&channels));
            let ledger = GeneralLineageHistoricalEvidenceLedger::capture(
                &current,
                history,
                None,
                [external_windows(
                    "ecology",
                    HistoricalChannelDisposition::SupportsLineageSeparation,
                )],
                auth("f1a-common-source-evidence", 160),
                auth("f1a-historical-context-evidence", 161),
                auth("f1a-counter-history-evidence", 162),
            )
            .unwrap();
            let HistoricalChannelEvidenceSource::NativeLineageHistory(core) =
                &ledger.channels[0].source
            else {
                panic!("core source must be native lineage history")
            };
            let after = &core.windows[2];
            assert_eq!(after.bounds.window, HistoricalChannelWindow::AfterCandidateInterval);
            let generation_five = after
                .generations
                .iter()
                .find_map(|record| match record {
                    symtropy_evolution_core::LineageHistoryGenerationRecord::Observed(observed)
                        if observed.generation.0 == 5 => Some(observed),
                    _ => None,
                })
                .unwrap();
            assert!(matches!(
                &generation_five.fusion.state,
                symtropy_evolution_core::LineageObservationState::Observed { .. }
            ));
        })
    });
}
