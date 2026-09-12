use symtropy_evolution_core::{
    binary_viability_exact_reference_method_v1,
    binary_viability_exact_reference_minimum_information_v1, execute_binary_viability_association,
    initialize_root_mutation_lineage, AlleleId, AnalysisAuthorityRef, AnalysisContentDigest,
    AnalysisMethodId, AncestryCopyId, AssociationExecutionError, BinaryComparisonGroup,
    ChannelSupportPolicy, ChromosomeAncestryState, ChromosomeDefinition, ChromosomeHaplotype,
    ChromosomeId, ChromosomeLocus, ChromosomeMap, ChromosomeMapId, CompetingRiskPolicy,
    ConfoundingPolicy, ConsequenceAssociationStatus, ConsequenceObservationId,
    ConsequenceWindowId, DeathBeforeEndpointPolicy, EvolutionIndividualId,
    EvolutionaryContextContentDigest, EvolutionaryContextId, EvolutionaryContextRef,
    ExactRiskRatio, ExplicitConsequenceLedger, ExplicitLinkedPopulationCensus,
    ExplicitSelectionAnalysisFrame, ExplicitSelectionEvidenceLedger,
    GeneticMapPositionMicromorgans, HaplotypeAncestryClass, HereditarySchema,
    HereditarySchemaId, IndividualConsequenceObservation, IndividualConsequences,
    IndividualSelectionEvidenceInput, InsufficientSupportPolicy, LinkedIndividualManifest,
    LinkedIndividualSubject, LocusDefinition, LocusId, MaterializedPredictorValue,
    MutationLineageState, PhasedAncestryState, PhasedChromosomeState, PhasedHereditaryState,
    PhenotypeEvidenceStatus, PopulationId, PredictorDefinitionId, PredictorDefinitionRef,
    PredictorMaterialization, PredictorMaterializationInput, PredictorMaterializationStatus,
    PredictorRepresentation, PredictorSourceBinding, PredictorSourceKind,
    ProtocolComparabilityPolicy, SelectionComparisonDesign, SelectionComparisonDesignId,
    SelectionDesignClass, SelectionEstimand, UncertaintyPlan, ValidatedConsequenceAssociation,
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
    ChromosomeId::new("chr-association").unwrap()
}

fn ancestry(id: &str) -> AncestryCopyId {
    AncestryCopyId::new(id).unwrap()
}

fn individual_id(id: &str) -> EvolutionIndividualId {
    EvolutionIndividualId::new(id).unwrap()
}

fn schema() -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new("association-schema").unwrap(),
        2,
        [LocusDefinition::new(locus(), [allele("a0"), allele("a1")]).unwrap()],
    )
    .unwrap()
}

fn chromosome_map(schema: &HereditarySchema) -> ChromosomeMap {
    ChromosomeMap::new(
        ChromosomeMapId::new("association-map").unwrap(),
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
    prefix: &str,
) -> PhasedAncestryState {
    PhasedAncestryState::new(
        schema,
        map,
        state,
        [ChromosomeAncestryState::new(
            chromosome(),
            vec![HaplotypeAncestryClass::new(
                0,
                vec![
                    ancestry(&format!("{prefix}-copy-0")),
                    ancestry(&format!("{prefix}-copy-1")),
                ],
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

fn individual(schema: &HereditarySchema, map: &ChromosomeMap, id: &str) -> Individual {
    let state = state(schema, map);
    let ancestry = ancestry_state(schema, map, &state, id);
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
        EvolutionaryContextId::new("association-context").unwrap(),
        1,
        EvolutionaryContextContentDigest::new([7; 32]),
        ConsequenceWindowId::new("association-window").unwrap(),
    )
}

fn method(id: &str, byte: u8) -> AnalysisAuthorityRef {
    AnalysisAuthorityRef::new(
        AnalysisMethodId::new(id).unwrap(),
        1,
        AnalysisContentDigest::new([byte; 32]),
    )
}

struct Fixture {
    schema: HereditarySchema,
    map: ChromosomeMap,
    individuals: Vec<Individual>,
    population: PopulationId,
    census: ExplicitLinkedPopulationCensus,
    context: EvolutionaryContextRef,
    consequences: ExplicitConsequenceLedger,
    evidence: ExplicitSelectionEvidenceLedger,
    design: SelectionComparisonDesign,
}

impl Fixture {
    fn subjects(&self) -> Vec<LinkedIndividualSubject<'_>> {
        self.individuals
            .iter()
            .map(|individual| subject(&self.schema, &self.map, individual))
            .collect()
    }
}

fn fixture(
    support_policy: InsufficientSupportPolicy,
    method_override: Option<AnalysisAuthorityRef>,
) -> Fixture {
    let schema = schema();
    let map = chromosome_map(&schema);
    let individuals = ["individual-a", "individual-b", "individual-c", "individual-d"]
        .into_iter()
        .map(|id| individual(&schema, &map, id))
        .collect::<Vec<_>>();
    let population = PopulationId::new("association-pop").unwrap();
    let context = context();
    let subjects = individuals
        .iter()
        .map(|individual| subject(&schema, &map, individual))
        .collect::<Vec<_>>();
    let census = ExplicitLinkedPopulationCensus::capture(
        population.clone(),
        &schema,
        &map,
        &subjects,
    )
    .unwrap();
    let observations = subjects.iter().enumerate().map(|(index, linked)| {
        IndividualConsequenceObservation::declare(
            ConsequenceObservationId::new(format!("association-observation-{index}")).unwrap(),
            linked.manifest.individual_id.clone(),
            &population,
            &schema,
            &map,
            &census,
            &subjects,
            &context,
            IndividualConsequences {
                viability: Some(if index < 2 {
                    ViabilityConsequence::SurvivedWindow
                } else {
                    ViabilityConsequence::DiedDuringWindow
                }),
                reproductive_events: None,
                descendant_production: None,
                descendant_recruitment: None,
            },
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
    let evidence = ExplicitSelectionEvidenceLedger::capture(
        &validated_consequences,
        &schema,
        &map,
        &census,
        &subjects,
        subjects.iter().map(|linked| {
            IndividualSelectionEvidenceInput::new(
                linked.manifest.individual_id.clone(),
                PhenotypeEvidenceStatus::Unavailable,
                symtropy_evolution_core::ExposureEvidenceStatus::Unavailable,
            )
        }),
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
    let design = SelectionComparisonDesign::declare(
        SelectionComparisonDesignId::new("association-design").unwrap(),
        &validated_evidence,
        PredictorDefinitionRef::new(
            PredictorDefinitionId::new("association-genotype-group").unwrap(),
            PredictorSourceKind::GenotypeOrLineage,
            1,
            AnalysisContentDigest::new([41; 32]),
        ),
        SelectionEstimand::ViabilityWindowRiskContrast,
        SelectionDesignClass::DescriptiveAssociation,
        None,
        subjects
            .iter()
            .map(|linked| linked.manifest.individual_id.clone()),
        [],
        method("association-eligibility", 42),
        ChannelSupportPolicy::NotRequired,
        ChannelSupportPolicy::NotRequired,
        ProtocolComparabilityPolicy::ExactIdentityOnly,
        ProtocolComparabilityPolicy::ExactIdentityOnly,
        ConfoundingPolicy::DescriptiveNoCausalClaim,
        DeathBeforeEndpointPolicy::NotApplicable,
        CompetingRiskPolicy::NoneDeclared,
        UncertaintyPlan {
            method: method_override.unwrap_or_else(binary_viability_exact_reference_method_v1),
            replicate_count: None,
            seed_lineage_digest: None,
            multiplicity: None,
            minimum_information: binary_viability_exact_reference_minimum_information_v1(),
            insufficient_support: support_policy,
        },
    )
    .unwrap();

    Fixture {
        schema,
        map,
        individuals,
        population,
        census,
        context,
        consequences,
        evidence,
        design,
    }
}

fn predictor_inputs(
    fixture: &Fixture,
    reverse_groups: bool,
    missing_index: Option<usize>,
) -> Vec<PredictorMaterializationInput> {
    fixture
        .evidence
        .records
        .iter()
        .enumerate()
        .map(|(index, record)| {
            let base_group = if index < 2 {
                BinaryComparisonGroup::Reference
            } else {
                BinaryComparisonGroup::Comparison
            };
            let group = if reverse_groups {
                match base_group {
                    BinaryComparisonGroup::Reference => BinaryComparisonGroup::Comparison,
                    BinaryComparisonGroup::Comparison => BinaryComparisonGroup::Reference,
                }
            } else {
                base_group
            };
            let status = if missing_index == Some(index) {
                PredictorMaterializationStatus::Missing {
                    reason: method("association-missing-predictor", 43),
                }
            } else {
                PredictorMaterializationStatus::Observed(MaterializedPredictorValue::Binary(group))
            };
            PredictorMaterializationInput::new(
                record.individual_id.clone(),
                PredictorMaterialization {
                    source: PredictorSourceBinding::HereditaryManifest {
                        manifest_digest: record.individual_manifest_digest(),
                    },
                    status,
                    calibration: None,
                },
            )
        })
        .collect()
}

fn validated_frame<'a>(
    fixture: &'a Fixture,
    subjects: &[LinkedIndividualSubject<'_>],
    inputs: Vec<PredictorMaterializationInput>,
) -> (
    ExplicitSelectionAnalysisFrame,
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
    let frame = ExplicitSelectionAnalysisFrame::capture(
        &design,
        &evidence,
        &consequences,
        method("association-materialization", 44),
        PredictorRepresentation::BinaryComparisonGroup,
        inputs,
    )
    .unwrap();
    (frame, consequences, evidence, design)
}

#[test]
fn exact_binary_viability_lane_matches_hand_checkable_reference_distribution() {
    let fixture = fixture(InsufficientSupportPolicy::FailClosed, None);
    let subjects = fixture.subjects();
    let inputs = predictor_inputs(&fixture, false, None);
    let (frame, consequences, evidence, design) =
        validated_frame(&fixture, &subjects, inputs.clone());
    let validated_frame = ValidatedSelectionAnalysisFrame::validate_current(
        &frame,
        &design,
        &evidence,
        &consequences,
        method("association-materialization", 44),
        PredictorRepresentation::BinaryComparisonGroup,
        inputs,
    )
    .unwrap();
    let estimate = execute_binary_viability_association(&validated_frame).unwrap();

    let ConsequenceAssociationStatus::Estimated(result) = &estimate.status else {
        panic!("expected estimated association");
    };
    assert_eq!(result.reference.total, 2);
    assert_eq!(result.reference.died_during_window, 0);
    assert_eq!(result.comparison.total, 2);
    assert_eq!(result.comparison.died_during_window, 2);
    assert!(!result.comparison_minus_reference_risk_difference.negative);
    assert_eq!(result.comparison_minus_reference_risk_difference.numerator, 1);
    assert_eq!(result.comparison_minus_reference_risk_difference.denominator, 1);
    assert_eq!(
        result.comparison_over_reference_risk_ratio,
        ExactRiskRatio::PositiveInfinity
    );
    assert_eq!(result.exact_reference.total_weight, 6);
    assert_eq!(result.exact_reference.observed_weight, 1);
    assert_eq!(
        result
            .exact_reference
            .support
            .iter()
            .map(|point| (point.comparison_deaths, point.weight))
            .collect::<Vec<_>>(),
        vec![(0, 1), (1, 4), (2, 1)]
    );
    assert_eq!(
        result
            .exact_reference
            .two_sided_probability_ordered_p
            .numerator,
        1
    );
    assert_eq!(
        result
            .exact_reference
            .two_sided_probability_ordered_p
            .denominator,
        3
    );
}

#[test]
fn reversing_group_labels_reverses_signed_contrast_and_changes_evidence_identity() {
    let fixture = fixture(InsufficientSupportPolicy::FailClosed, None);
    let subjects = fixture.subjects();

    let forward_inputs = predictor_inputs(&fixture, false, None);
    let (forward_frame, consequences, evidence, design) =
        validated_frame(&fixture, &subjects, forward_inputs.clone());
    let forward_validated = ValidatedSelectionAnalysisFrame::validate_current(
        &forward_frame,
        &design,
        &evidence,
        &consequences,
        method("association-materialization", 44),
        PredictorRepresentation::BinaryComparisonGroup,
        forward_inputs,
    )
    .unwrap();
    let forward = execute_binary_viability_association(&forward_validated).unwrap();

    let reverse_inputs = predictor_inputs(&fixture, true, None);
    let reverse_frame = ExplicitSelectionAnalysisFrame::capture(
        &design,
        &evidence,
        &consequences,
        method("association-materialization", 44),
        PredictorRepresentation::BinaryComparisonGroup,
        reverse_inputs.clone(),
    )
    .unwrap();
    let reverse_validated = ValidatedSelectionAnalysisFrame::validate_current(
        &reverse_frame,
        &design,
        &evidence,
        &consequences,
        method("association-materialization", 44),
        PredictorRepresentation::BinaryComparisonGroup,
        reverse_inputs,
    )
    .unwrap();
    let reverse = execute_binary_viability_association(&reverse_validated).unwrap();

    let ConsequenceAssociationStatus::Estimated(forward_result) = &forward.status else {
        panic!("expected forward estimate");
    };
    let ConsequenceAssociationStatus::Estimated(reverse_result) = &reverse.status else {
        panic!("expected reverse estimate");
    };
    assert!(!forward_result.comparison_minus_reference_risk_difference.negative);
    assert!(reverse_result.comparison_minus_reference_risk_difference.negative);
    assert_ne!(forward.frame_digest(), reverse.frame_digest());
    assert_ne!(
        forward.canonical_digest().unwrap(),
        reverse.canonical_digest().unwrap()
    );
}

#[test]
fn method_authority_drift_is_rejected_before_statistics_execute() {
    let fixture = fixture(
        InsufficientSupportPolicy::FailClosed,
        Some(method("post-hoc-method", 99)),
    );
    let subjects = fixture.subjects();
    let inputs = predictor_inputs(&fixture, false, None);
    let (frame, consequences, evidence, design) =
        validated_frame(&fixture, &subjects, inputs.clone());
    let validated_frame = ValidatedSelectionAnalysisFrame::validate_current(
        &frame,
        &design,
        &evidence,
        &consequences,
        method("association-materialization", 44),
        PredictorRepresentation::BinaryComparisonGroup,
        inputs,
    )
    .unwrap();
    assert!(matches!(
        execute_binary_viability_association(&validated_frame),
        Err(AssociationExecutionError::MethodAuthorityMismatch)
    ));
}

#[test]
fn missing_predictor_follows_frozen_insufficient_support_policy() {
    let reporting_fixture = fixture(InsufficientSupportPolicy::ReportInsufficientSupport, None);
    let reporting_subjects = reporting_fixture.subjects();
    let reporting_inputs = predictor_inputs(&reporting_fixture, false, Some(1));
    let (reporting_frame, consequences, evidence, design) = validated_frame(
        &reporting_fixture,
        &reporting_subjects,
        reporting_inputs.clone(),
    );
    let reporting_validated = ValidatedSelectionAnalysisFrame::validate_current(
        &reporting_frame,
        &design,
        &evidence,
        &consequences,
        method("association-materialization", 44),
        PredictorRepresentation::BinaryComparisonGroup,
        reporting_inputs,
    )
    .unwrap();
    let reported = execute_binary_viability_association(&reporting_validated).unwrap();
    assert!(matches!(
        reported.status,
        ConsequenceAssociationStatus::InsufficientSupport(_)
    ));

    let failing_fixture = fixture(InsufficientSupportPolicy::FailClosed, None);
    let failing_subjects = failing_fixture.subjects();
    let failing_inputs = predictor_inputs(&failing_fixture, false, Some(1));
    let (failing_frame, consequences, evidence, design) =
        validated_frame(&failing_fixture, &failing_subjects, failing_inputs.clone());
    let failing_validated = ValidatedSelectionAnalysisFrame::validate_current(
        &failing_frame,
        &design,
        &evidence,
        &consequences,
        method("association-materialization", 44),
        PredictorRepresentation::BinaryComparisonGroup,
        failing_inputs,
    )
    .unwrap();
    assert!(matches!(
        execute_binary_viability_association(&failing_validated),
        Err(AssociationExecutionError::InsufficientSupport(_))
    ));
}

#[test]
fn serialized_association_must_replay_against_current_validated_frame() {
    let fixture = fixture(InsufficientSupportPolicy::FailClosed, None);
    let subjects = fixture.subjects();
    let inputs = predictor_inputs(&fixture, false, None);
    let (frame, consequences, evidence, design) =
        validated_frame(&fixture, &subjects, inputs.clone());
    let validated_frame = ValidatedSelectionAnalysisFrame::validate_current(
        &frame,
        &design,
        &evidence,
        &consequences,
        method("association-materialization", 44),
        PredictorRepresentation::BinaryComparisonGroup,
        inputs,
    )
    .unwrap();
    let estimate = execute_binary_viability_association(&validated_frame).unwrap();
    let encoded = serde_json::to_vec(&estimate).unwrap();
    let restored: symtropy_evolution_core::ConsequenceAssociationEstimate =
        serde_json::from_slice(&encoded).unwrap();
    let validated =
        ValidatedConsequenceAssociation::validate_current(&restored, &validated_frame).unwrap();
    assert_eq!(
        validated.estimate_digest(),
        estimate.canonical_digest().unwrap()
    );
}

#[test]
fn association_wire_shape_remains_descriptive_not_causal_or_adaptive() {
    let fixture = fixture(InsufficientSupportPolicy::FailClosed, None);
    let subjects = fixture.subjects();
    let inputs = predictor_inputs(&fixture, false, None);
    let (frame, consequences, evidence, design) =
        validated_frame(&fixture, &subjects, inputs.clone());
    let validated_frame = ValidatedSelectionAnalysisFrame::validate_current(
        &frame,
        &design,
        &evidence,
        &consequences,
        method("association-materialization", 44),
        PredictorRepresentation::BinaryComparisonGroup,
        inputs,
    )
    .unwrap();
    let estimate = execute_binary_viability_association(&validated_frame).unwrap();
    let encoded = serde_json::to_string(&estimate).unwrap();
    for forbidden in [
        "selection_coefficient",
        "beneficial",
        "deleterious",
        "adaptation",
        "causal_selection",
        "fitness",
        "speciation",
    ] {
        assert!(!encoded.contains(forbidden));
    }
}
