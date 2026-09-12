use symtropy_evolution_core::{
    initialize_root_mutation_lineage, AlleleId, AnalysisAuthorityRef, AnalysisContentDigest,
    AnalysisMethodId, AncestryCopyId, BinaryComparisonGroup, CalibrationAuthorityId,
    CalibrationAuthorityRef, ChannelSupportPolicy, ChromosomeAncestryState, ChromosomeDefinition,
    ChromosomeHaplotype, ChromosomeId, ChromosomeLocus, ChromosomeMap, ChromosomeMapId,
    CompetingRiskPolicy, ConfoundingPolicy, ConsequenceObservationId, ConsequenceWindowId,
    DeathBeforeEndpointPolicy, DescendantProductionConsequence, EvidenceContentDigest,
    EvidenceProtocolContentDigest, EvidenceProtocolId, EvolutionIndividualId,
    EvolutionaryContextContentDigest, EvolutionaryContextId, EvolutionaryContextRef,
    ExplicitConsequenceLedger, ExplicitLinkedPopulationCensus, ExplicitSelectionAnalysisFrame,
    ExplicitSelectionEvidenceLedger, ExposureEvidenceStatus, GeneticMapPositionMicromorgans,
    HaplotypeAncestryClass, HereditarySchema, HereditarySchemaId,
    IndividualConsequenceObservation, IndividualConsequences, IndividualSelectionEvidenceInput,
    InsufficientSupportPolicy, LinkedIndividualManifest, LinkedIndividualSubject, LocusDefinition,
    LocusId, MaterializedOutcome, MaterializedPredictorValue, MutationLineageState,
    PhasedAncestryState, PhasedChromosomeState, PhasedHereditaryState, PhenotypeEvidenceRef,
    PhenotypeEvidenceSourceId, PhenotypeEvidenceStatus, PopulationId,
    PredictorCalibrationProvenance, PredictorDefinitionId, PredictorDefinitionRef,
    PredictorMaterialization, PredictorMaterializationInput, PredictorMaterializationStatus,
    PredictorRepresentation, PredictorSourceBinding, PredictorSourceKind,
    ProtocolComparabilityPolicy, SelectionAnalysisFrameError, SelectionComparisonDesign,
    SelectionComparisonDesignId, SelectionDesignClass, SelectionEstimand, UncertaintyPlan,
    ValidatedConsequenceLedger, ValidatedSelectionAnalysisFrame,
    ValidatedSelectionComparisonDesign, ValidatedSelectionEvidenceLedger, ViabilityConsequence,
};

fn allele(id: &str) -> AlleleId {
    AlleleId::new(id).unwrap()
}

fn locus() -> LocusId {
    LocusId::new("focal").unwrap()
}

fn chromosome() -> ChromosomeId {
    ChromosomeId::new("chr-analysis-frame").unwrap()
}

fn ancestry(id: &str) -> AncestryCopyId {
    AncestryCopyId::new(id).unwrap()
}

fn individual_id(id: &str) -> EvolutionIndividualId {
    EvolutionIndividualId::new(id).unwrap()
}

fn population() -> PopulationId {
    PopulationId::new("analysis-frame-pop").unwrap()
}

fn schema() -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new("analysis-frame-schema").unwrap(),
        2,
        [LocusDefinition::new(locus(), [allele("a0"), allele("a1")]).unwrap()],
    )
    .unwrap()
}

fn chromosome_map(schema: &HereditarySchema) -> ChromosomeMap {
    ChromosomeMap::new(
        ChromosomeMapId::new("analysis-frame-map").unwrap(),
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

fn context() -> EvolutionaryContextRef {
    EvolutionaryContextRef::new(
        EvolutionaryContextId::new("analysis-frame-context").unwrap(),
        1,
        EvolutionaryContextContentDigest::new([7; 32]),
        ConsequenceWindowId::new("analysis-frame-window").unwrap(),
    )
}

fn method(id: &str, byte: u8) -> AnalysisAuthorityRef {
    AnalysisAuthorityRef::new(
        AnalysisMethodId::new(id).unwrap(),
        1,
        AnalysisContentDigest::new([byte; 32]),
    )
}

fn calibration(byte: u8) -> CalibrationAuthorityRef {
    CalibrationAuthorityRef::new(
        CalibrationAuthorityId::new("analysis-frame-calibration").unwrap(),
        1,
        AnalysisContentDigest::new([byte; 32]),
    )
}

fn predictor() -> PredictorDefinitionRef {
    PredictorDefinitionRef::new(
        PredictorDefinitionId::new("analysis-frame-phenotype").unwrap(),
        PredictorSourceKind::Phenotype,
        1,
        AnalysisContentDigest::new([41; 32]),
    )
}

fn uncertainty() -> UncertaintyPlan {
    UncertaintyPlan {
        method: method("analysis-frame-uncertainty", 51),
        replicate_count: Some(100),
        seed_lineage_digest: Some(AnalysisContentDigest::new([52; 32])),
        multiplicity: None,
        minimum_information: method("analysis-frame-minimum-information", 53),
        insufficient_support: InsufficientSupportPolicy::FailClosed,
    }
}

fn phenotype(
    id: &str,
    context: &EvolutionaryContextRef,
    protocol_byte: u8,
) -> PhenotypeEvidenceRef {
    PhenotypeEvidenceRef::new(
        individual_id(id),
        PhenotypeEvidenceSourceId::new("analysis-frame-phenotype-source").unwrap(),
        1,
        EvidenceContentDigest::new([61; 32]),
        EvidenceProtocolId::new("analysis-frame-phenotype-protocol").unwrap(),
        EvidenceProtocolContentDigest::new([protocol_byte; 32]),
        context.canonical_digest().unwrap(),
    )
}

#[derive(Clone, Copy)]
enum OutcomeFixture {
    Viability,
    DescendantProduction,
}

fn consequences_for(index: usize, fixture: OutcomeFixture) -> IndividualConsequences {
    match fixture {
        OutcomeFixture::Viability => IndividualConsequences {
            viability: Some(if index == 0 {
                ViabilityConsequence::SurvivedWindow
            } else {
                ViabilityConsequence::DiedDuringWindow
            }),
            reproductive_events: None,
            descendant_production: None,
            descendant_recruitment: None,
        },
        OutcomeFixture::DescendantProduction => IndividualConsequences {
            viability: Some(if index == 0 {
                ViabilityConsequence::SurvivedWindow
            } else {
                ViabilityConsequence::DiedDuringWindow
            }),
            reproductive_events: None,
            descendant_production: if index == 0 {
                Some(DescendantProductionConsequence { produced: 0 })
            } else {
                None
            },
            descendant_recruitment: None,
        },
    }
}

struct Fixture {
    schema: HereditarySchema,
    map: ChromosomeMap,
    first: Individual,
    second: Individual,
    population: PopulationId,
    census: ExplicitLinkedPopulationCensus,
    context: EvolutionaryContextRef,
    consequences: ExplicitConsequenceLedger,
    evidence: ExplicitSelectionEvidenceLedger,
    design: SelectionComparisonDesign,
}

impl Fixture {
    fn subjects(&self) -> [LinkedIndividualSubject<'_>; 2] {
        [
            subject(&self.schema, &self.map, &self.first),
            subject(&self.schema, &self.map, &self.second),
        ]
    }
}

fn fixture(
    outcome_fixture: OutcomeFixture,
    second_phenotype_available: bool,
    phenotype_protocols: [u8; 2],
    protocol_policy: ProtocolComparabilityPolicy,
) -> Fixture {
    let schema = schema();
    let map = chromosome_map(&schema);
    let first = individual(&schema, &map, "individual-a", "a-copy-0", "a-copy-1");
    let second = individual(&schema, &map, "individual-b", "b-copy-0", "b-copy-1");
    let population = population();
    let context = context();

    let subjects = [
        subject(&schema, &map, &first),
        subject(&schema, &map, &second),
    ];
    let census =
        ExplicitLinkedPopulationCensus::capture(population.clone(), &schema, &map, &subjects)
            .unwrap();
    let observations = subjects.iter().enumerate().map(|(index, linked)| {
        IndividualConsequenceObservation::declare(
            ConsequenceObservationId::new(format!("analysis-frame-observation-{index}")).unwrap(),
            linked.manifest.individual_id.clone(),
            &population,
            &schema,
            &map,
            &census,
            &subjects,
            &context,
            consequences_for(index, outcome_fixture),
        )
        .unwrap()
    });
    let consequences = ExplicitConsequenceLedger::capture(
        &population,
        &schema,
        &map,
        &census,
        &subjects,
        &context,
        observations,
    )
    .unwrap();
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

    let second_phenotype = if second_phenotype_available {
        PhenotypeEvidenceStatus::CompleteWindow(phenotype(
            "individual-b",
            &context,
            phenotype_protocols[1],
        ))
    } else {
        PhenotypeEvidenceStatus::Unavailable
    };
    let evidence = ExplicitSelectionEvidenceLedger::capture(
        &validated_consequences,
        &schema,
        &map,
        &census,
        &subjects,
        [
            IndividualSelectionEvidenceInput::new(
                individual_id("individual-a"),
                PhenotypeEvidenceStatus::CompleteWindow(phenotype(
                    "individual-a",
                    &context,
                    phenotype_protocols[0],
                )),
                ExposureEvidenceStatus::Unavailable,
            ),
            IndividualSelectionEvidenceInput::new(
                individual_id("individual-b"),
                second_phenotype,
                ExposureEvidenceStatus::Unavailable,
            ),
        ],
    )
    .unwrap();
    let validated_evidence = ValidatedSelectionEvidenceLedger::validate_current(
        &evidence,
        &validated_consequences,
        &schema,
        &map,
        &census,
        &subjects,
    )
    .unwrap();

    let estimand = match outcome_fixture {
        OutcomeFixture::Viability => SelectionEstimand::ViabilityWindowRiskContrast,
        OutcomeFixture::DescendantProduction => SelectionEstimand::DescendantProductionContrast,
    };
    let death_policy = match outcome_fixture {
        OutcomeFixture::Viability => DeathBeforeEndpointPolicy::NotApplicable,
        OutcomeFixture::DescendantProduction => DeathBeforeEndpointPolicy::Censored,
    };
    let support = if second_phenotype_available {
        ChannelSupportPolicy::CompleteOnly
    } else {
        ChannelSupportPolicy::UnavailableAllowed {
            method: method("analysis-frame-missing-phenotype", 71),
        }
    };
    let design = SelectionComparisonDesign::declare(
        SelectionComparisonDesignId::new("analysis-frame-design").unwrap(),
        &validated_evidence,
        predictor(),
        estimand,
        SelectionDesignClass::DescriptiveAssociation,
        None,
        [individual_id("individual-a"), individual_id("individual-b")],
        [],
        method("analysis-frame-eligibility", 72),
        support,
        ChannelSupportPolicy::NotRequired,
        protocol_policy,
        ProtocolComparabilityPolicy::ExactIdentityOnly,
        ConfoundingPolicy::DescriptiveNoCausalClaim,
        death_policy,
        CompetingRiskPolicy::NoneDeclared,
        uncertainty(),
    )
    .unwrap();

    Fixture {
        schema,
        map,
        first,
        second,
        population,
        census,
        context,
        consequences,
        evidence,
        design,
    }
}

fn source_binding(status: &PhenotypeEvidenceStatus) -> PredictorSourceBinding {
    match status {
        PhenotypeEvidenceStatus::Unavailable => PredictorSourceBinding::PhenotypeUnavailable,
        PhenotypeEvidenceStatus::PartialWindow {
            evidence,
            support_digest,
        } => PredictorSourceBinding::Phenotype {
            source_id: evidence.source_id.clone(),
            revision: evidence.revision,
            content_digest: evidence.content_digest,
            protocol_id: evidence.protocol_id.clone(),
            protocol_content_digest: evidence.protocol_content_digest,
            window: symtropy_evolution_core::EvidenceWindowBinding::Partial {
                support_digest: *support_digest,
            },
        },
        PhenotypeEvidenceStatus::CompleteWindow(evidence) => PredictorSourceBinding::Phenotype {
            source_id: evidence.source_id.clone(),
            revision: evidence.revision,
            content_digest: evidence.content_digest,
            protocol_id: evidence.protocol_id.clone(),
            protocol_content_digest: evidence.protocol_content_digest,
            window: symtropy_evolution_core::EvidenceWindowBinding::Complete,
        },
    }
}

fn predictor_inputs(
    fixture: &Fixture,
    calibration_authority: Option<CalibrationAuthorityRef>,
) -> Vec<PredictorMaterializationInput> {
    fixture
        .evidence
        .records
        .iter()
        .enumerate()
        .map(|(index, record)| {
            let source = source_binding(&record.phenotype);
            let status = if matches!(&record.phenotype, PhenotypeEvidenceStatus::Unavailable) {
                PredictorMaterializationStatus::Missing {
                    reason: method("analysis-frame-missing-value", 81),
                }
            } else {
                PredictorMaterializationStatus::Observed(MaterializedPredictorValue::Binary(
                    if index == 0 {
                        BinaryComparisonGroup::Reference
                    } else {
                        BinaryComparisonGroup::Comparison
                    },
                ))
            };
            let calibration = calibration_authority.clone().and_then(|authority| {
                if matches!(&status, PredictorMaterializationStatus::Observed(_)) {
                    Some(PredictorCalibrationProvenance {
                        authority,
                        transformed_value_digest: AnalysisContentDigest::new([
                            90u8.wrapping_add(index as u8);
                            32
                        ]),
                    })
                } else {
                    None
                }
            });
            PredictorMaterializationInput::new(
                record.individual_id.clone(),
                PredictorMaterialization {
                    source,
                    status,
                    calibration,
                },
            )
        })
        .collect()
}

fn with_validated<'a>(
    fixture: &'a Fixture,
    subjects: &[LinkedIndividualSubject<'_>],
) -> (
    ValidatedConsequenceLedger<'a>,
    ValidatedSelectionEvidenceLedger<'a>,
    ValidatedSelectionComparisonDesign<'a>,
) {
    let consequences = ValidatedConsequenceLedger::validate_current(
        &fixture.consequences,
        &fixture.population,
        &fixture.schema,
        &fixture.map,
        &fixture.census,
        subjects,
        &fixture.context,
    )
    .unwrap();
    let evidence = ValidatedSelectionEvidenceLedger::validate_current(
        &fixture.evidence,
        &consequences,
        &fixture.schema,
        &fixture.map,
        &fixture.census,
        subjects,
    )
    .unwrap();
    let design = ValidatedSelectionComparisonDesign::validate_current(&fixture.design, &evidence)
        .unwrap();
    (consequences, evidence, design)
}

#[test]
fn predictor_channel_cannot_be_reintroduced_after_design_declares_it_not_required() {
    let mut fixture = fixture(
        OutcomeFixture::Viability,
        true,
        [91, 91],
        ProtocolComparabilityPolicy::ExactIdentityOnly,
    );
    fixture.design.phenotype_support = ChannelSupportPolicy::NotRequired;
    let subjects = fixture.subjects();
    let (consequences, evidence, design) = with_validated(&fixture, &subjects);

    assert!(matches!(
        ExplicitSelectionAnalysisFrame::capture(
            &design,
            &evidence,
            &consequences,
            method("analysis-frame-materialization", 100),
            PredictorRepresentation::BinaryComparisonGroup,
            predictor_inputs(&fixture, None),
        ),
        Err(SelectionAnalysisFrameError::PredictorChannelNotRequired)
    ));
}

#[test]
fn frame_preserves_exact_denominator_and_input_order_is_nonsemantic() {
    let fixture = fixture(
        OutcomeFixture::Viability,
        true,
        [91, 91],
        ProtocolComparabilityPolicy::ExactIdentityOnly,
    );
    let subjects = fixture.subjects();
    let (consequences, evidence, design) = with_validated(&fixture, &subjects);
    let inputs = predictor_inputs(&fixture, None);
    let authority = method("analysis-frame-materialization", 101);

    let forward = ExplicitSelectionAnalysisFrame::capture(
        &design,
        &evidence,
        &consequences,
        authority.clone(),
        PredictorRepresentation::BinaryComparisonGroup,
        inputs.clone(),
    )
    .unwrap();
    let reverse = ExplicitSelectionAnalysisFrame::capture(
        &design,
        &evidence,
        &consequences,
        authority,
        PredictorRepresentation::BinaryComparisonGroup,
        inputs.into_iter().rev(),
    )
    .unwrap();

    assert_eq!(forward, reverse);
    assert_eq!(forward.original_denominator(), 2);
    assert_eq!(forward.estimand_denominator(), 2);
    assert_eq!(forward.rows.len(), 2);
    assert_eq!(
        &forward.rows[0].outcome,
        &MaterializedOutcome::ViabilityObserved(ViabilityConsequence::SurvivedWindow)
    );
    assert_eq!(
        &forward.rows[1].outcome,
        &MaterializedOutcome::ViabilityObserved(ViabilityConsequence::DiedDuringWindow)
    );
    assert_eq!(
        forward.canonical_digest().unwrap(),
        reverse.canonical_digest().unwrap()
    );
}

#[test]
fn omitted_duplicate_and_excluded_predictor_inputs_fail_closed() {
    let fixture = fixture(
        OutcomeFixture::Viability,
        true,
        [91, 91],
        ProtocolComparabilityPolicy::ExactIdentityOnly,
    );
    let subjects = fixture.subjects();
    let (consequences, evidence, design) = with_validated(&fixture, &subjects);
    let inputs = predictor_inputs(&fixture, None);

    assert!(matches!(
        ExplicitSelectionAnalysisFrame::capture(
            &design,
            &evidence,
            &consequences,
            method("analysis-frame-materialization", 101),
            PredictorRepresentation::BinaryComparisonGroup,
            [inputs[0].clone()],
        ),
        Err(SelectionAnalysisFrameError::IncompleteIncludedCoverage)
    ));

    assert!(matches!(
        ExplicitSelectionAnalysisFrame::capture(
            &design,
            &evidence,
            &consequences,
            method("analysis-frame-materialization", 101),
            PredictorRepresentation::BinaryComparisonGroup,
            [inputs[0].clone(), inputs[0].clone()],
        ),
        Err(SelectionAnalysisFrameError::DuplicatePredictorInput(_))
    ));

    let mut unknown = inputs[0].clone();
    unknown.individual_id = individual_id("individual-extra");
    assert!(matches!(
        ExplicitSelectionAnalysisFrame::capture(
            &design,
            &evidence,
            &consequences,
            method("analysis-frame-materialization", 101),
            PredictorRepresentation::BinaryComparisonGroup,
            [inputs[0].clone(), unknown],
        ),
        Err(SelectionAnalysisFrameError::UnknownOrExcludedInput(_))
    ));
}

#[test]
fn unavailable_predictor_source_is_missing_not_zero_and_cannot_be_observed() {
    let fixture = fixture(
        OutcomeFixture::Viability,
        false,
        [91, 91],
        ProtocolComparabilityPolicy::ExactIdentityOnly,
    );
    let subjects = fixture.subjects();
    let (consequences, evidence, design) = with_validated(&fixture, &subjects);
    let mut inputs = predictor_inputs(&fixture, None);

    let frame = ExplicitSelectionAnalysisFrame::capture(
        &design,
        &evidence,
        &consequences,
        method("analysis-frame-materialization", 102),
        PredictorRepresentation::BinaryComparisonGroup,
        inputs.clone(),
    )
    .unwrap();
    assert!(matches!(
        &frame.rows[1].predictor.source,
        PredictorSourceBinding::PhenotypeUnavailable
    ));
    assert!(matches!(
        &frame.rows[1].predictor.status,
        PredictorMaterializationStatus::Missing { .. }
    ));

    inputs[1].materialization.status = PredictorMaterializationStatus::Observed(
        MaterializedPredictorValue::Binary(BinaryComparisonGroup::Comparison),
    );
    assert!(matches!(
        ExplicitSelectionAnalysisFrame::capture(
            &design,
            &evidence,
            &consequences,
            method("analysis-frame-materialization", 102),
            PredictorRepresentation::BinaryComparisonGroup,
            inputs,
        ),
        Err(SelectionAnalysisFrameError::ObservedValueFromUnavailableSource(_))
    ));
}

#[test]
fn observed_zero_descendants_remains_distinct_from_censored_missing_outcome() {
    let fixture = fixture(
        OutcomeFixture::DescendantProduction,
        true,
        [91, 91],
        ProtocolComparabilityPolicy::ExactIdentityOnly,
    );
    let subjects = fixture.subjects();
    let (consequences, evidence, design) = with_validated(&fixture, &subjects);
    let frame = ExplicitSelectionAnalysisFrame::capture(
        &design,
        &evidence,
        &consequences,
        method("analysis-frame-materialization", 103),
        PredictorRepresentation::BinaryComparisonGroup,
        predictor_inputs(&fixture, None),
    )
    .unwrap();

    assert_eq!(
        &frame.rows[0].outcome,
        &MaterializedOutcome::DescendantProductionObserved { produced: 0 }
    );
    assert_eq!(
        &frame.rows[1].outcome,
        &MaterializedOutcome::CensoredByObservedDeath
    );
    assert_ne!(&frame.rows[0].outcome, &frame.rows[1].outcome);
}

#[test]
fn calibrated_predictor_materialization_requires_exact_calibration_provenance() {
    let authority = calibration(111);
    let fixture = fixture(
        OutcomeFixture::Viability,
        true,
        [91, 92],
        ProtocolComparabilityPolicy::Calibrated {
            authority: authority.clone(),
        },
    );
    let subjects = fixture.subjects();
    let (consequences, evidence, design) = with_validated(&fixture, &subjects);

    assert!(matches!(
        ExplicitSelectionAnalysisFrame::capture(
            &design,
            &evidence,
            &consequences,
            method("analysis-frame-materialization", 104),
            PredictorRepresentation::BinaryComparisonGroup,
            predictor_inputs(&fixture, None),
        ),
        Err(SelectionAnalysisFrameError::CalibrationProvenanceMismatch(_))
    ));

    let frame = ExplicitSelectionAnalysisFrame::capture(
        &design,
        &evidence,
        &consequences,
        method("analysis-frame-materialization", 104),
        PredictorRepresentation::BinaryComparisonGroup,
        predictor_inputs(&fixture, Some(authority)),
    )
    .unwrap();
    assert_eq!(frame.rows.len(), 2);
}

#[test]
fn serde_restored_frame_requires_fresh_materialization_replay() {
    let fixture = fixture(
        OutcomeFixture::Viability,
        true,
        [91, 91],
        ProtocolComparabilityPolicy::ExactIdentityOnly,
    );
    let subjects = fixture.subjects();
    let (consequences, evidence, design) = with_validated(&fixture, &subjects);
    let authority = method("analysis-frame-materialization", 105);
    let inputs = predictor_inputs(&fixture, None);
    let frame = ExplicitSelectionAnalysisFrame::capture(
        &design,
        &evidence,
        &consequences,
        authority.clone(),
        PredictorRepresentation::BinaryComparisonGroup,
        inputs.clone(),
    )
    .unwrap();

    let encoded = serde_json::to_vec(&frame).unwrap();
    let restored: ExplicitSelectionAnalysisFrame = serde_json::from_slice(&encoded).unwrap();
    let validated = ValidatedSelectionAnalysisFrame::validate_current(
        &restored,
        &design,
        &evidence,
        &consequences,
        authority.clone(),
        PredictorRepresentation::BinaryComparisonGroup,
        inputs.clone(),
    )
    .unwrap();
    assert_eq!(validated.frame_digest(), frame.canonical_digest().unwrap());

    assert!(matches!(
        ValidatedSelectionAnalysisFrame::validate_current(
            &restored,
            &design,
            &evidence,
            &consequences,
            method("analysis-frame-materialization-drift", 106),
            PredictorRepresentation::BinaryComparisonGroup,
            inputs.clone(),
        ),
        Err(SelectionAnalysisFrameError::FrameReplayMismatch)
    ));

    let mut tampered = restored.clone();
    tampered.rows[0].predictor.status = PredictorMaterializationStatus::Observed(
        MaterializedPredictorValue::Binary(BinaryComparisonGroup::Comparison),
    );
    assert_ne!(tampered.canonical_digest().unwrap(), frame.canonical_digest().unwrap());
    assert!(matches!(
        ValidatedSelectionAnalysisFrame::validate_current(
            &tampered,
            &design,
            &evidence,
            &consequences,
            authority,
            PredictorRepresentation::BinaryComparisonGroup,
            inputs,
        ),
        Err(SelectionAnalysisFrameError::FrameReplayMismatch)
    ));
}

#[test]
fn frame_wire_shape_contains_no_estimate_fitness_selection_or_adaptation_claims() {
    let fixture = fixture(
        OutcomeFixture::Viability,
        true,
        [91, 91],
        ProtocolComparabilityPolicy::ExactIdentityOnly,
    );
    let subjects = fixture.subjects();
    let (consequences, evidence, design) = with_validated(&fixture, &subjects);
    let frame = ExplicitSelectionAnalysisFrame::capture(
        &design,
        &evidence,
        &consequences,
        method("analysis-frame-materialization", 106),
        PredictorRepresentation::BinaryComparisonGroup,
        predictor_inputs(&fixture, None),
    )
    .unwrap();
    let encoded = serde_json::to_string(&frame).unwrap();
    for forbidden in [
        "\"fitness\"",
        "\"selection_coefficient\"",
        "\"beneficial\"",
        "\"deleterious\"",
        "\"adaptation\"",
        "\"causal_effect\"",
        "\"p_value\"",
        "\"point_estimate\"",
    ] {
        assert!(!encoded.contains(forbidden));
    }
}
