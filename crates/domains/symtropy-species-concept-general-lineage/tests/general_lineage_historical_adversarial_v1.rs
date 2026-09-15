include!("general_lineage_historical_evidence_v1.rs");

fn evidence_authorities() -> (AnalysisAuthorityRef, AnalysisAuthorityRef, AnalysisAuthorityRef) {
    (
        auth("f1a-common-source-evidence", 160),
        auth("f1a-historical-context-evidence", 161),
        auth("f1a-counter-history-evidence", 162),
    )
}

#[test]
fn preregistration_rejects_boundary_abuse_and_posthoc_channel_projection_changes() {
    five_generation_history::with_clean(|history_design, _history| {
        with_classification(history_design, base_channels(), |classification, channels| {
            let missing_before = GeneralLineageHistoricalEvidenceDesign::declare(
                GeneralLineageHistoricalEvidenceDesignId::new("f1a-missing-before").unwrap(),
                classification,
                symtropy_evolution_core::PopulationGeneration(1),
                symtropy_evolution_core::PopulationGeneration(2),
                projections(&channels),
                2,
                auth("f1a-common-source-protocol", 130),
                auth("f1a-historical-context-protocol", 131),
                auth("f1a-counter-history-protocol", 132),
                GeneralLineageHistoricalMissingPolicy::ReportIncompleteEvidence,
            );
            assert!(matches!(
                missing_before,
                Err(GeneralLineageHistoricalDesignError::MissingBeforeOrAfterCoverage)
            ));

            let missing_after = GeneralLineageHistoricalEvidenceDesign::declare(
                GeneralLineageHistoricalEvidenceDesignId::new("f1a-missing-after").unwrap(),
                classification,
                symtropy_evolution_core::PopulationGeneration(4),
                symtropy_evolution_core::PopulationGeneration(5),
                projections(&channels),
                2,
                auth("f1a-common-source-protocol", 130),
                auth("f1a-historical-context-protocol", 131),
                auth("f1a-counter-history-protocol", 132),
                GeneralLineageHistoricalMissingPolicy::ReportIncompleteEvidence,
            );
            assert!(matches!(
                missing_after,
                Err(GeneralLineageHistoricalDesignError::MissingBeforeOrAfterCoverage)
            ));

            let mut missing_projection = projections(&channels);
            missing_projection.pop();
            let missing_projection = GeneralLineageHistoricalEvidenceDesign::declare(
                GeneralLineageHistoricalEvidenceDesignId::new("f1a-missing-projection").unwrap(),
                classification,
                symtropy_evolution_core::PopulationGeneration(2),
                symtropy_evolution_core::PopulationGeneration(3),
                missing_projection,
                2,
                auth("f1a-common-source-protocol", 130),
                auth("f1a-historical-context-protocol", 131),
                auth("f1a-counter-history-protocol", 132),
                GeneralLineageHistoricalMissingPolicy::ReportIncompleteEvidence,
            );
            assert!(matches!(
                missing_projection,
                Err(GeneralLineageHistoricalDesignError::MissingChannelProjection)
            ));

            let mut duplicate_projection = projections(&channels);
            duplicate_projection.push(duplicate_projection[0].clone());
            let duplicate_projection = GeneralLineageHistoricalEvidenceDesign::declare(
                GeneralLineageHistoricalEvidenceDesignId::new("f1a-duplicate-projection").unwrap(),
                classification,
                symtropy_evolution_core::PopulationGeneration(2),
                symtropy_evolution_core::PopulationGeneration(3),
                duplicate_projection,
                2,
                auth("f1a-common-source-protocol", 130),
                auth("f1a-historical-context-protocol", 131),
                auth("f1a-counter-history-protocol", 132),
                GeneralLineageHistoricalMissingPolicy::ReportIncompleteEvidence,
            );
            assert!(matches!(
                duplicate_projection,
                Err(GeneralLineageHistoricalDesignError::DuplicateChannelProjection)
            ));
        })
    });
}

#[test]
fn repeated_temporal_metrics_from_one_dependency_group_do_not_create_independence() {
    five_generation_history::with_clean(|history_design, _history| {
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
        ];
        with_classification(history_design, channels, |classification, channels| {
            let result = GeneralLineageHistoricalEvidenceDesign::declare(
                GeneralLineageHistoricalEvidenceDesignId::new("f1a-dependent-metrics").unwrap(),
                classification,
                symtropy_evolution_core::PopulationGeneration(2),
                symtropy_evolution_core::PopulationGeneration(3),
                projections(&channels),
                3,
                auth("f1a-common-source-protocol", 130),
                auth("f1a-historical-context-protocol", 131),
                auth("f1a-counter-history-protocol", 132),
                GeneralLineageHistoricalMissingPolicy::ReportIncompleteEvidence,
            );
            assert!(matches!(
                result,
                Err(GeneralLineageHistoricalDesignError::TemporalSupportThresholdExceedsGroups)
            ));
        })
    });
}

#[test]
fn materialization_rejects_missing_channels_duplicate_or_missing_windows_and_failclosed_unavailability() {
    five_generation_history::with_clean(|history_design, history| {
        with_classification(history_design, base_channels(), |classification, channels| {
            let raw = GeneralLineageHistoricalEvidenceDesign::declare(
                GeneralLineageHistoricalEvidenceDesignId::new("f1a-materialization-adversarial").unwrap(),
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
            let (common, context, counter) = evidence_authorities();

            let missing_channel = GeneralLineageHistoricalEvidenceLedger::capture(
                &current,
                history,
                None,
                [],
                common.clone(),
                context.clone(),
                counter.clone(),
            );
            assert!(matches!(
                missing_channel,
                Err(GeneralLineageHistoricalEvidenceError::MissingExternalChannel)
            ));

            let mut missing_window = external_windows(
                "ecology",
                HistoricalChannelDisposition::SupportsLineageSeparation,
            );
            missing_window.windows.pop();
            let missing_window = GeneralLineageHistoricalEvidenceLedger::capture(
                &current,
                history,
                None,
                [missing_window],
                common.clone(),
                context.clone(),
                counter.clone(),
            );
            assert!(matches!(
                missing_window,
                Err(GeneralLineageHistoricalEvidenceError::MissingExternalWindow)
            ));

            let mut duplicate_window = external_windows(
                "ecology",
                HistoricalChannelDisposition::SupportsLineageSeparation,
            );
            duplicate_window.windows.push(duplicate_window.windows[0].clone());
            let duplicate_window = GeneralLineageHistoricalEvidenceLedger::capture(
                &current,
                history,
                None,
                [duplicate_window],
                common.clone(),
                context.clone(),
                counter.clone(),
            );
            assert!(matches!(
                duplicate_window,
                Err(GeneralLineageHistoricalEvidenceError::DuplicateExternalWindow)
            ));

            let fail_closed_raw = GeneralLineageHistoricalEvidenceDesign::declare(
                GeneralLineageHistoricalEvidenceDesignId::new("f1a-failclosed-unavailable").unwrap(),
                classification,
                symtropy_evolution_core::PopulationGeneration(2),
                symtropy_evolution_core::PopulationGeneration(3),
                projections(&channels),
                2,
                auth("f1a-common-source-protocol", 130),
                auth("f1a-historical-context-protocol", 131),
                auth("f1a-counter-history-protocol", 132),
                GeneralLineageHistoricalMissingPolicy::FailClosed,
            )
            .unwrap();
            let fail_closed = ValidatedGeneralLineageHistoricalEvidenceDesign::validate_current(
                &fail_closed_raw,
                classification,
                symtropy_evolution_core::PopulationGeneration(2),
                symtropy_evolution_core::PopulationGeneration(3),
                projections(&channels),
                2,
                auth("f1a-common-source-protocol", 130),
                auth("f1a-historical-context-protocol", 131),
                auth("f1a-counter-history-protocol", 132),
                GeneralLineageHistoricalMissingPolicy::FailClosed,
            )
            .unwrap();
            let unavailable = GeneralLineageHistoricalEvidenceLedger::capture(
                &fail_closed,
                history,
                None,
                [external_windows(
                    "ecology",
                    HistoricalChannelDisposition::Unavailable,
                )],
                common,
                context,
                counter,
            );
            assert!(matches!(
                unavailable,
                Err(GeneralLineageHistoricalEvidenceError::UnavailableEvidenceForbidden)
            ));
        })
    });
}

#[test]
fn wrong_history_subject_and_stale_temporal_policy_cannot_replay_as_current() {
    five_generation_history::with_clean(|history_design, history| {
        with_classification(history_design, base_channels(), |classification, channels| {
            let raw = GeneralLineageHistoricalEvidenceDesign::declare(
                GeneralLineageHistoricalEvidenceDesignId::new("f1a-current-replay-adversarial").unwrap(),
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

            let mut stale_projections = projections(&channels);
            stale_projections[0].temporal_projection_protocol = auth("f1a-stale-projection", 199);
            let stale_design = ValidatedGeneralLineageHistoricalEvidenceDesign::validate_current(
                &raw,
                classification,
                symtropy_evolution_core::PopulationGeneration(2),
                symtropy_evolution_core::PopulationGeneration(3),
                stale_projections,
                2,
                auth("f1a-common-source-protocol", 130),
                auth("f1a-historical-context-protocol", 131),
                auth("f1a-counter-history-protocol", 132),
                GeneralLineageHistoricalMissingPolicy::ReportIncompleteEvidence,
            );
            assert!(matches!(
                stale_design,
                Err(GeneralLineageHistoricalDesignError::DesignReplayMismatch)
            ));

            lineage_fixture::with_clean(|_other_history_design, other_history| {
                let (common, context, counter) = evidence_authorities();
                let wrong_history = GeneralLineageHistoricalEvidenceLedger::capture(
                    &current,
                    other_history,
                    None,
                    [external_windows(
                        "ecology",
                        HistoricalChannelDisposition::SupportsLineageSeparation,
                    )],
                    common,
                    context,
                    counter,
                );
                assert!(matches!(
                    wrong_history,
                    Err(GeneralLineageHistoricalEvidenceError::LineageHistoryDesignMismatch)
                ));
            });

            let _ = history;
        })
    });
}

#[test]
fn restored_ledger_cannot_drop_preregistered_channels_or_windows_and_remain_valid() {
    five_generation_history::with_clean(|history_design, history| {
        with_classification(history_design, base_channels(), |classification, channels| {
            let raw = GeneralLineageHistoricalEvidenceDesign::declare(
                GeneralLineageHistoricalEvidenceDesignId::new("f1a-restored-coverage").unwrap(),
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
            let (common, context, counter) = evidence_authorities();
            let ledger = GeneralLineageHistoricalEvidenceLedger::capture(
                &current,
                history,
                None,
                [external_windows(
                    "ecology",
                    HistoricalChannelDisposition::SupportsLineageSeparation,
                )],
                common,
                context,
                counter,
            )
            .unwrap();

            let mut missing_channel: GeneralLineageHistoricalEvidenceLedger =
                serde_json::from_slice(&serde_json::to_vec(&ledger).unwrap()).unwrap();
            missing_channel.channels.pop();
            assert!(matches!(
                missing_channel.canonical_digest(),
                Err(GeneralLineageHistoricalEvidenceError::IncompleteChannelCoverage)
            ));

            let mut missing_window: GeneralLineageHistoricalEvidenceLedger =
                serde_json::from_slice(&serde_json::to_vec(&ledger).unwrap()).unwrap();
            let external = missing_window
                .channels
                .iter_mut()
                .find(|record| record.declaration.current_channel.channel_id.as_str() == "ecology")
                .unwrap();
            let HistoricalChannelEvidenceSource::External(source) = &mut external.source else {
                panic!("ecology must remain external historical evidence")
            };
            source.windows.pop();
            assert!(matches!(
                missing_window.canonical_digest(),
                Err(GeneralLineageHistoricalEvidenceError::IncompleteWindowCoverage)
            ));
        })
    });
}
