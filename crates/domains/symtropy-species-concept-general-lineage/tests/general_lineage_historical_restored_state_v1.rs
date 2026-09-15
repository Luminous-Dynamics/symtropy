include!("general_lineage_historical_evidence_v1.rs");

fn round_trip(
    ledger: &GeneralLineageHistoricalEvidenceLedger,
) -> GeneralLineageHistoricalEvidenceLedger {
    serde_json::from_slice(&serde_json::to_vec(ledger).unwrap()).unwrap()
}

#[test]
fn restored_external_disposition_can_be_locally_valid_but_cannot_inherit_current_trust() {
    five_generation_history::with_clean(|history_design, history| {
        with_classification(history_design, base_channels(), |classification, channels| {
            let raw = GeneralLineageHistoricalEvidenceDesign::declare(
                GeneralLineageHistoricalEvidenceDesignId::new("f1a-restored-external").unwrap(),
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

            let original_digest = ledger.canonical_digest().unwrap();
            let mut restored = round_trip(&ledger);
            let ecology = restored
                .channels
                .iter_mut()
                .find(|record| record.declaration.current_channel.channel_id.as_str() == "ecology")
                .unwrap();
            let HistoricalChannelEvidenceSource::External(source) = &mut ecology.source else {
                panic!("ecology must remain an externally qualified historical channel")
            };
            source.windows[1].disposition =
                HistoricalChannelDisposition::ContradictsLineageSeparation;

            let restored_digest = restored.canonical_digest().unwrap();
            assert_ne!(restored_digest, original_digest);
            let replay = ValidatedGeneralLineageHistoricalEvidence::validate_current(
                &restored,
                &current,
                history,
                None,
                [external],
                auth("f1a-common-source-evidence", 160),
                auth("f1a-historical-context-evidence", 161),
                auth("f1a-counter-history-evidence", 162),
            );
            assert!(matches!(
                replay,
                Err(GeneralLineageHistoricalEvidenceError::LedgerReplayMismatch)
            ));
        })
    });
}

#[test]
fn restored_evidence_authority_can_be_locally_valid_but_cannot_inherit_current_trust() {
    five_generation_history::with_clean(|history_design, history| {
        with_classification(history_design, base_channels(), |classification, channels| {
            let raw = GeneralLineageHistoricalEvidenceDesign::declare(
                GeneralLineageHistoricalEvidenceDesignId::new("f1a-restored-authority").unwrap(),
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

            let mut restored = round_trip(&ledger);
            restored.common_source_evidence.evidence =
                auth("f1a-stale-common-source-evidence", 200);
            assert!(restored.canonical_digest().is_ok());

            let replay = ValidatedGeneralLineageHistoricalEvidence::validate_current(
                &restored,
                &current,
                history,
                None,
                [external],
                auth("f1a-common-source-evidence", 160),
                auth("f1a-historical-context-evidence", 161),
                auth("f1a-counter-history-evidence", 162),
            );
            assert!(matches!(
                replay,
                Err(GeneralLineageHistoricalEvidenceError::LedgerReplayMismatch)
            ));
        })
    });
}

#[test]
fn restored_native_history_identity_can_be_locally_valid_but_cannot_inherit_current_trust() {
    five_generation_history::with_clean(|history_design, history| {
        with_classification(history_design, base_channels(), |classification, channels| {
            let raw = GeneralLineageHistoricalEvidenceDesign::declare(
                GeneralLineageHistoricalEvidenceDesignId::new("f1a-restored-native-history").unwrap(),
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

            let mut alternate_history_digest = None;
            five_generation_history::with_fusion_after(|_, alternate_history| {
                alternate_history_digest = Some(alternate_history.history_digest());
            });
            let alternate_history_digest = alternate_history_digest.unwrap();
            assert_ne!(alternate_history_digest, history.history_digest());

            let mut restored = round_trip(&ledger);
            let core = restored
                .channels
                .iter_mut()
                .find(|record| {
                    record.declaration.current_channel.kind
                        == GeneralLineageEvidenceChannelKind::LongitudinalLineageSeparation
                })
                .unwrap();
            let HistoricalChannelEvidenceSource::NativeLineageHistory(source) = &mut core.source else {
                panic!("core channel must remain native SEL-10A history")
            };
            source.history_digest = alternate_history_digest;
            assert!(restored.canonical_digest().is_ok());

            let replay = ValidatedGeneralLineageHistoricalEvidence::validate_current(
                &restored,
                &current,
                history,
                None,
                [external],
                auth("f1a-common-source-evidence", 160),
                auth("f1a-historical-context-evidence", 161),
                auth("f1a-counter-history-evidence", 162),
            );
            assert!(matches!(
                replay,
                Err(GeneralLineageHistoricalEvidenceError::LedgerReplayMismatch)
            ));
        })
    });
}
