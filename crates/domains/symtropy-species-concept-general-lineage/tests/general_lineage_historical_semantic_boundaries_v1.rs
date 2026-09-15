include!("general_lineage_historical_evidence_v1.rs");

mod five_generation_recontact {
    include!("../../symtropy-evolution-core/tests/lineage_divergence_history_v1.rs");

    fn fixture_five() -> HistoryFixture {
        let schema = schema();
        let experiment = EvolutionExperimentId::new("f1a-recontact-five-generation-fixture").unwrap();
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
            LineageDivergenceHistoryId::new("f1a-recontact-history-five-generation").unwrap(),
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

    fn inputs<'a>(fixture: &'a HistoryFixture) -> Vec<LineageHistoryGenerationInput<'a>> {
        vec![
            observed_input(fixture, 1, GenerationKind::Clean),
            observed_input(fixture, 2, GenerationKind::Clean),
            observed_input(fixture, 3, GenerationKind::Clean),
            observed_input(fixture, 4, GenerationKind::Clean),
            observed_input(fixture, 5, GenerationKind::Recontact),
        ]
    }

    pub(super) fn with_recontact_after(
        f: impl FnOnce(
            &ValidatedLineageDivergenceHistoryDesign<'_>,
            &ValidatedLineageDivergenceHistory<'_>,
        ),
    ) {
        let fixture = fixture_five();
        let design = design_five();
        let current_design = current_design_five(&design);
        let history = LineageDivergenceHistory::capture(&current_design, inputs(&fixture)).unwrap();
        let current_history = ValidatedLineageDivergenceHistory::validate_current(
            &history,
            &current_design,
            inputs(&fixture),
        )
        .unwrap();
        f(&current_design, &current_history);
    }
}

#[test]
fn post_interval_recontact_remains_recontact_and_does_not_become_fusion() {
    five_generation_recontact::with_recontact_after(|history_design, history| {
        with_classification(history_design, base_channels(), |classification, channels| {
            let raw = GeneralLineageHistoricalEvidenceDesign::declare(
                GeneralLineageHistoricalEvidenceDesignId::new("f1a-recontact-counterhistory").unwrap(),
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
                panic!("core historical source must remain native SEL-10A history")
            };
            let generation_five = core.windows[2]
                .generations
                .iter()
                .find_map(|record| match record {
                    symtropy_evolution_core::LineageHistoryGenerationRecord::Observed(observed)
                        if observed.generation.0 == 5 => Some(observed),
                    _ => None,
                })
                .unwrap();

            assert!(matches!(
                &generation_five.recontact.state,
                symtropy_evolution_core::LineageObservationState::Observed { .. }
            ));
            assert!(!matches!(
                &generation_five.fusion.state,
                symtropy_evolution_core::LineageObservationState::Observed { .. }
            ));
        })
    });
}

#[test]
fn sel09b_contradiction_remains_native_evidence_and_cannot_be_recast_as_external_disposition() {
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
                    GeneralLineageHistoricalEvidenceDesignId::new("f1a-native-isolation-boundary").unwrap(),
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

                let recast = GeneralLineageHistoricalEvidenceLedger::capture(
                    &current,
                    history,
                    Some(isolation),
                    [external_windows(
                        "reproductive-isolation",
                        HistoricalChannelDisposition::ContradictsLineageSeparation,
                    )],
                    auth("f1a-common-source-evidence", 160),
                    auth("f1a-historical-context-evidence", 161),
                    auth("f1a-counter-history-evidence", 162),
                );
                assert!(matches!(
                    recast,
                    Err(GeneralLineageHistoricalEvidenceError::NativeChannelExternalOverride)
                ));

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
                assert_eq!(
                    isolation.evidence().status,
                    symtropy_evolution_core::ReproductiveIsolationStatus::Contradicted
                );
                let source = ledger
                    .channels
                    .iter()
                    .find(|record| {
                        record.declaration.current_channel.kind
                            == GeneralLineageEvidenceChannelKind::ReproductiveIsolation
                    })
                    .unwrap();
                assert!(matches!(
                    &source.source,
                    HistoricalChannelEvidenceSource::NativeReproductiveIsolation(_)
                ));
            })
        })
    });
}
