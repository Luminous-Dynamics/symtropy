use symtropy_evolution_core::{
    initialize_root_mutation_lineage, AlleleId, AncestryCopyId, ChromosomeAncestryState,
    ChromosomeDefinition, ChromosomeHaplotype, ChromosomeId, ChromosomeLocus, ChromosomeMap,
    ChromosomeMapId, ConsequenceObservationId, ConsequenceWindowId, EvidenceContentDigest,
    EvidenceProtocolContentDigest, EvidenceProtocolId, EvolutionIndividualId,
    EvolutionaryContextContentDigest, EvolutionaryContextId, EvolutionaryContextRef,
    ExplicitConsequenceLedger, ExplicitLinkedPopulationCensus, ExplicitSelectionEvidenceLedger,
    ExposureEvidenceRef, ExposureEvidenceSourceId, ExposureEvidenceStatus,
    GeneticMapPositionMicromorgans, HaplotypeAncestryClass, HereditarySchema,
    HereditarySchemaId, IndividualConsequenceObservation, IndividualConsequences,
    IndividualSelectionEvidenceInput, LinkedIndividualManifest, LinkedIndividualSubject,
    LocusDefinition, LocusId, MutationLineageState, ObservationSupportDigest,
    PhasedAncestryState, PhasedChromosomeState, PhasedHereditaryState, PhenotypeEvidenceRef,
    PhenotypeEvidenceSourceId, PhenotypeEvidenceStatus, PopulationId, SelectionEvidenceError,
    ValidatedConsequenceLedger, ValidatedSelectionEvidenceLedger, ViabilityConsequence,
};

fn allele(id: &str) -> AlleleId {
    AlleleId::new(id).unwrap()
}

fn locus() -> LocusId {
    LocusId::new("focal").unwrap()
}

fn chromosome() -> ChromosomeId {
    ChromosomeId::new("chr-selection-evidence").unwrap()
}

fn ancestry(id: &str) -> AncestryCopyId {
    AncestryCopyId::new(id).unwrap()
}

fn individual_id(id: &str) -> EvolutionIndividualId {
    EvolutionIndividualId::new(id).unwrap()
}

fn population() -> PopulationId {
    PopulationId::new("selection-evidence-pop").unwrap()
}

fn schema() -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new("selection-evidence-schema").unwrap(),
        2,
        [LocusDefinition::new(locus(), [allele("a0"), allele("a1")]).unwrap()],
    )
    .unwrap()
}

fn chromosome_map(schema: &HereditarySchema) -> ChromosomeMap {
    ChromosomeMap::new(
        ChromosomeMapId::new("selection-evidence-map").unwrap(),
        schema,
        [ChromosomeDefinition::new(
            chromosome(),
            vec![ChromosomeLocus::new(
                locus(),
                GeneticMapPositionMicromorgans::new(1),
            )],
        )
        .unwrap()],
    )
    .unwrap()
}

fn state(schema: &HereditarySchema, map: &ChromosomeMap) -> PhasedHereditaryState {
    PhasedHereditaryState::new(
        schema,
        map,
        [PhasedChromosomeState::new(
            chromosome(),
            vec![
                ChromosomeHaplotype::new(vec![allele("a0")]),
                ChromosomeHaplotype::new(vec![allele("a0")]),
            ],
        )],
    )
    .unwrap()
}

fn ancestry_state(
    schema: &HereditarySchema,
    map: &ChromosomeMap,
    state: &PhasedHereditaryState,
    first: &str,
    second: &str,
) -> PhasedAncestryState {
    PhasedAncestryState::new(
        schema,
        map,
        state,
        [ChromosomeAncestryState::new(
            chromosome(),
            vec![HaplotypeAncestryClass::new(
                0,
                vec![ancestry(first), ancestry(second)],
            )
            .unwrap()],
        )
        .unwrap()],
    )
    .unwrap()
}

struct Individual {
    state: PhasedHereditaryState,
    ancestry: PhasedAncestryState,
    lineage: MutationLineageState,
    manifest: LinkedIndividualManifest,
}

fn individual(
    schema: &HereditarySchema,
    map: &ChromosomeMap,
    id: &str,
    first_copy: &str,
    second_copy: &str,
) -> Individual {
    let state = state(schema, map);
    let ancestry = ancestry_state(schema, map, &state, first_copy, second_copy);
    let lineage = initialize_root_mutation_lineage(schema, map, &state, &ancestry).unwrap();
    let manifest = LinkedIndividualManifest::new(
        individual_id(id),
        schema,
        map,
        &state,
        &ancestry,
        &lineage,
    )
    .unwrap();
    Individual {
        state,
        ancestry,
        lineage,
        manifest,
    }
}

fn subject<'a>(
    schema: &HereditarySchema,
    map: &ChromosomeMap,
    individual: &'a Individual,
) -> LinkedIndividualSubject<'a> {
    LinkedIndividualSubject::new(
        &individual.manifest,
        schema,
        map,
        &individual.state,
        &individual.ancestry,
        &individual.lineage,
    )
    .unwrap()
}

fn context(byte: u8, window: &str) -> EvolutionaryContextRef {
    EvolutionaryContextRef::new(
        EvolutionaryContextId::new("selection-context").unwrap(),
        1,
        EvolutionaryContextContentDigest::new([byte; 32]),
        ConsequenceWindowId::new(window).unwrap(),
    )
}

fn viability(value: ViabilityConsequence) -> IndividualConsequences {
    IndividualConsequences {
        viability: Some(value),
        reproductive_events: None,
        descendant_production: None,
        descendant_recruitment: None,
    }
}

fn consequence_ledger(
    schema: &HereditarySchema,
    map: &ChromosomeMap,
    subjects: &[LinkedIndividualSubject<'_>],
    census: &ExplicitLinkedPopulationCensus,
    context: &EvolutionaryContextRef,
) -> ExplicitConsequenceLedger {
    let population = population();
    let observations = subjects.iter().enumerate().map(|(index, subject)| {
        IndividualConsequenceObservation::declare(
            ConsequenceObservationId::new(format!("obs-{index}")).unwrap(),
            subject.manifest.individual_id.clone(),
            &population,
            schema,
            map,
            census,
            subjects,
            context,
            viability(if index == 0 {
                ViabilityConsequence::SurvivedWindow
            } else {
                ViabilityConsequence::DiedDuringWindow
            }),
        )
        .unwrap()
    });
    ExplicitConsequenceLedger::capture(
        &population,
        schema,
        map,
        census,
        subjects,
        context,
        observations,
    )
    .unwrap()
}

fn phenotype(
    id: &str,
    context: &EvolutionaryContextRef,
    protocol_digest_byte: u8,
) -> PhenotypeEvidenceRef {
    PhenotypeEvidenceRef::new(
        individual_id(id),
        PhenotypeEvidenceSourceId::new("phenotype-source").unwrap(),
        1,
        EvidenceContentDigest::new([11; 32]),
        EvidenceProtocolId::new("phenotype-protocol").unwrap(),
        EvidenceProtocolContentDigest::new([protocol_digest_byte; 32]),
        context.canonical_digest().unwrap(),
    )
}

fn exposure(
    id: &str,
    context: &EvolutionaryContextRef,
    content_byte: u8,
    protocol_digest_byte: u8,
) -> ExposureEvidenceRef {
    ExposureEvidenceRef::new(
        individual_id(id),
        ExposureEvidenceSourceId::new("exposure-source").unwrap(),
        1,
        EvidenceContentDigest::new([content_byte; 32]),
        EvidenceProtocolId::new("exposure-protocol").unwrap(),
        EvidenceProtocolContentDigest::new([protocol_digest_byte; 32]),
        context.canonical_digest().unwrap(),
    )
}

#[test]
fn complete_bridge_preserves_every_consequence_individual_and_order_is_nonsemantic() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let first = individual(&schema, &map, "individual-a", "a-copy-0", "a-copy-1");
    let second = individual(&schema, &map, "individual-b", "b-copy-0", "b-copy-1");
    let subjects = [
        subject(&schema, &map, &first),
        subject(&schema, &map, &second),
    ];
    let population = population();
    let census = ExplicitLinkedPopulationCensus::capture(
        population.clone(),
        &schema,
        &map,
        &subjects,
    )
    .unwrap();
    let context = context(7, "window-a");
    let consequences = consequence_ledger(&schema, &map, &subjects, &census, &context);
    let validated_consequences = ValidatedConsequenceLedger::validate_current(
        &consequences,
        &population,
        &schema,
        &map,
        &census,
        &subjects,
        &context,
    )
    .unwrap();

    let inputs = [
        IndividualSelectionEvidenceInput::new(
            individual_id("individual-a"),
            PhenotypeEvidenceStatus::Unavailable,
            ExposureEvidenceStatus::CompleteWindow(exposure(
                "individual-a",
                &context,
                21,
                31,
            )),
        ),
        IndividualSelectionEvidenceInput::new(
            individual_id("individual-b"),
            PhenotypeEvidenceStatus::PartialWindow {
                evidence: phenotype("individual-b", &context, 41),
                support_digest: ObservationSupportDigest::new([51; 32]),
            },
            ExposureEvidenceStatus::Unavailable,
        ),
    ];

    let forward = ExplicitSelectionEvidenceLedger::capture(
        &validated_consequences,
        &schema,
        &map,
        &census,
        &subjects,
        inputs.clone(),
    )
    .unwrap();
    let reverse = ExplicitSelectionEvidenceLedger::capture(
        &validated_consequences,
        &schema,
        &map,
        &census,
        &subjects,
        inputs.into_iter().rev(),
    )
    .unwrap();

    assert_eq!(forward, reverse);
    assert_eq!(forward.records.len(), 2);
    assert_eq!(forward.records[0].individual_id, individual_id("individual-a"));
    assert_eq!(forward.records[1].individual_id, individual_id("individual-b"));
    assert_eq!(
        forward.canonical_digest().unwrap(),
        reverse.canonical_digest().unwrap()
    );

    let validated = ValidatedSelectionEvidenceLedger::validate_current(
        &forward,
        &validated_consequences,
        &schema,
        &map,
        &census,
        &subjects,
    )
    .unwrap();
    assert_eq!(validated.ledger_digest(), forward.canonical_digest().unwrap());
}

#[test]
fn omission_extra_and_duplicate_inputs_cannot_change_the_denominator() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let first = individual(&schema, &map, "individual-a", "a-copy-0", "a-copy-1");
    let second = individual(&schema, &map, "individual-b", "b-copy-0", "b-copy-1");
    let subjects = [
        subject(&schema, &map, &first),
        subject(&schema, &map, &second),
    ];
    let population = population();
    let census = ExplicitLinkedPopulationCensus::capture(
        population.clone(),
        &schema,
        &map,
        &subjects,
    )
    .unwrap();
    let context = context(7, "window-a");
    let consequences = consequence_ledger(&schema, &map, &subjects, &census, &context);
    let validated = ValidatedConsequenceLedger::validate_current(
        &consequences,
        &population,
        &schema,
        &map,
        &census,
        &subjects,
        &context,
    )
    .unwrap();
    let unavailable = |id: &str| {
        IndividualSelectionEvidenceInput::new(
            individual_id(id),
            PhenotypeEvidenceStatus::Unavailable,
            ExposureEvidenceStatus::Unavailable,
        )
    };

    assert!(matches!(
        ExplicitSelectionEvidenceLedger::capture(
            &validated,
            &schema,
            &map,
            &census,
            &subjects,
            [unavailable("individual-a")],
        ),
        Err(SelectionEvidenceError::IncompleteConsequenceCoverage)
    ));

    assert!(ExplicitSelectionEvidenceLedger::capture(
        &validated,
        &schema,
        &map,
        &census,
        &subjects,
        [
            unavailable("individual-a"),
            unavailable("individual-b"),
            unavailable("individual-extra"),
        ],
    )
    .is_err());

    assert!(matches!(
        ExplicitSelectionEvidenceLedger::capture(
            &validated,
            &schema,
            &map,
            &census,
            &subjects,
            [unavailable("individual-a"), unavailable("individual-a")],
        ),
        Err(SelectionEvidenceError::DuplicateIndividualEvidence)
    ));
}

#[test]
fn unavailable_partial_and_complete_have_distinct_evidence_identity() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let first = individual(&schema, &map, "individual-a", "a-copy-0", "a-copy-1");
    let subjects = [subject(&schema, &map, &first)];
    let population = population();
    let census = ExplicitLinkedPopulationCensus::capture(
        population.clone(),
        &schema,
        &map,
        &subjects,
    )
    .unwrap();
    let context = context(7, "window-a");
    let consequences = consequence_ledger(&schema, &map, &subjects, &census, &context);
    let validated = ValidatedConsequenceLedger::validate_current(
        &consequences,
        &population,
        &schema,
        &map,
        &census,
        &subjects,
        &context,
    )
    .unwrap();
    let evidence = phenotype("individual-a", &context, 41);

    let capture = |status| {
        ExplicitSelectionEvidenceLedger::capture(
            &validated,
            &schema,
            &map,
            &census,
            &subjects,
            [IndividualSelectionEvidenceInput::new(
                individual_id("individual-a"),
                status,
                ExposureEvidenceStatus::Unavailable,
            )],
        )
        .unwrap()
    };

    let unavailable = capture(PhenotypeEvidenceStatus::Unavailable);
    let partial = capture(PhenotypeEvidenceStatus::PartialWindow {
        evidence: evidence.clone(),
        support_digest: ObservationSupportDigest::new([51; 32]),
    });
    let complete = capture(PhenotypeEvidenceStatus::CompleteWindow(evidence));

    assert_ne!(
        unavailable.canonical_digest().unwrap(),
        partial.canonical_digest().unwrap()
    );
    assert_ne!(
        partial.canonical_digest().unwrap(),
        complete.canonical_digest().unwrap()
    );
    assert_ne!(
        unavailable.canonical_digest().unwrap(),
        complete.canonical_digest().unwrap()
    );
}

#[test]
fn protocol_or_context_drift_changes_or_rejects_evidence() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let first = individual(&schema, &map, "individual-a", "a-copy-0", "a-copy-1");
    let subjects = [subject(&schema, &map, &first)];
    let population = population();
    let census = ExplicitLinkedPopulationCensus::capture(
        population.clone(),
        &schema,
        &map,
        &subjects,
    )
    .unwrap();
    let context = context(7, "window-a");
    let consequences = consequence_ledger(&schema, &map, &subjects, &census, &context);
    let validated = ValidatedConsequenceLedger::validate_current(
        &consequences,
        &population,
        &schema,
        &map,
        &census,
        &subjects,
        &context,
    )
    .unwrap();

    let capture = |protocol_byte| {
        ExplicitSelectionEvidenceLedger::capture(
            &validated,
            &schema,
            &map,
            &census,
            &subjects,
            [IndividualSelectionEvidenceInput::new(
                individual_id("individual-a"),
                PhenotypeEvidenceStatus::CompleteWindow(phenotype(
                    "individual-a",
                    &context,
                    protocol_byte,
                )),
                ExposureEvidenceStatus::Unavailable,
            )],
        )
        .unwrap()
    };
    let protocol_a = capture(41);
    let protocol_b = capture(42);
    assert_ne!(
        protocol_a.canonical_digest().unwrap(),
        protocol_b.canonical_digest().unwrap()
    );

    let wrong_context = context(8, "window-a");
    assert!(matches!(
        ExplicitSelectionEvidenceLedger::capture(
            &validated,
            &schema,
            &map,
            &census,
            &subjects,
            [IndividualSelectionEvidenceInput::new(
                individual_id("individual-a"),
                PhenotypeEvidenceStatus::Unavailable,
                ExposureEvidenceStatus::CompleteWindow(exposure(
                    "individual-a",
                    &wrong_context,
                    21,
                    31,
                )),
            )],
        ),
        Err(SelectionEvidenceError::EvidenceContextMismatch)
    ));
}

#[test]
fn restored_evidence_requires_exact_current_replay_before_regaining_authority() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let first = individual(&schema, &map, "individual-a", "a-copy-0", "a-copy-1");
    let subjects = [subject(&schema, &map, &first)];
    let population = population();
    let census = ExplicitLinkedPopulationCensus::capture(
        population.clone(),
        &schema,
        &map,
        &subjects,
    )
    .unwrap();
    let context = context(7, "window-a");
    let consequences = consequence_ledger(&schema, &map, &subjects, &census, &context);
    let validated_consequences = ValidatedConsequenceLedger::validate_current(
        &consequences,
        &population,
        &schema,
        &map,
        &census,
        &subjects,
        &context,
    )
    .unwrap();
    let ledger = ExplicitSelectionEvidenceLedger::capture(
        &validated_consequences,
        &schema,
        &map,
        &census,
        &subjects,
        [IndividualSelectionEvidenceInput::new(
            individual_id("individual-a"),
            PhenotypeEvidenceStatus::CompleteWindow(phenotype("individual-a", &context, 41)),
            ExposureEvidenceStatus::CompleteWindow(exposure(
                "individual-a",
                &context,
                21,
                31,
            )),
        )],
    )
    .unwrap();

    let encoded = serde_json::to_vec(&ledger).unwrap();
    let restored: ExplicitSelectionEvidenceLedger = serde_json::from_slice(&encoded).unwrap();
    ValidatedSelectionEvidenceLedger::validate_current(
        &restored,
        &validated_consequences,
        &schema,
        &map,
        &census,
        &subjects,
    )
    .unwrap();

    let mut raw = serde_json::to_value(&ledger).unwrap();
    raw["records"][0]["consequence_observation_digest"] =
        serde_json::to_value(vec![0_u8; 32]).unwrap();
    let stale: ExplicitSelectionEvidenceLedger = serde_json::from_value(raw).unwrap();
    assert!(stale.canonical_digest().is_ok());
    assert!(matches!(
        ValidatedSelectionEvidenceLedger::validate_current(
            &stale,
            &validated_consequences,
            &schema,
            &map,
            &census,
            &subjects,
        ),
        Err(SelectionEvidenceError::LedgerReplayMismatch)
    ));
}

#[test]
fn selection_evidence_wire_shape_contains_no_fitness_or_causal_selection_claims() {
    let input = IndividualSelectionEvidenceInput::new(
        individual_id("individual-a"),
        PhenotypeEvidenceStatus::Unavailable,
        ExposureEvidenceStatus::Unavailable,
    );
    let encoded = serde_json::to_string(&input).unwrap();
    for forbidden in [
        "fitness",
        "selection_coefficient",
        "beneficial",
        "deleterious",
        "adaptation",
        "causal_genotype",
    ] {
        assert!(!encoded.contains(forbidden));
    }
}
