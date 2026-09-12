use symtropy_evolution_core::{
    initialize_root_mutation_lineage, AlleleId, AnalysisAuthorityRef, AnalysisContentDigest,
    AnalysisMethodId, AncestryCopyId, CalibrationAuthorityId, CalibrationAuthorityRef,
    ChannelSupportPolicy, ChromosomeAncestryState, ChromosomeDefinition, ChromosomeHaplotype,
    ChromosomeId, ChromosomeLocus, ChromosomeMap, ChromosomeMapId, ComparisonAuthorityId,
    ComparisonAuthorityRef, CompetingRiskPolicy, ConfoundingPolicy, ConsequenceObservationId,
    ConsequenceWindowId, DeathBeforeEndpointPolicy, EvidenceContentDigest,
    EvidenceProtocolContentDigest, EvidenceProtocolId, EvolutionIndividualId,
    EvolutionaryContextContentDigest, EvolutionaryContextId, EvolutionaryContextRef,
    ExclusionDeclaration, ExclusionReasonId, ExplicitConsequenceLedger,
    ExplicitLinkedPopulationCensus, ExplicitSelectionEvidenceLedger, ExposureEvidenceRef,
    ExposureEvidenceSourceId, ExposureEvidenceStatus, GeneticMapPositionMicromorgans,
    HaplotypeAncestryClass, HereditarySchema, HereditarySchemaId,
    IndividualConsequenceObservation, IndividualConsequences, IndividualSelectionEvidenceInput,
    InsufficientSupportPolicy, LinkedIndividualManifest, LinkedIndividualSubject, LocusDefinition,
    LocusId, MutationLineageState, PhasedAncestryState, PhasedChromosomeState,
    PhasedHereditaryState, PhenotypeEvidenceRef, PhenotypeEvidenceSourceId,
    PhenotypeEvidenceStatus, PopulationId, PredictorDefinitionId, PredictorDefinitionRef,
    PredictorSourceKind, ProtocolComparabilityPolicy, SelectionComparisonDesign,
    SelectionComparisonDesignId, SelectionDesignClass, SelectionDesignError, SelectionEstimand,
    UncertaintyPlan, ValidatedConsequenceLedger, ValidatedSelectionComparisonDesign,
    ValidatedSelectionEvidenceLedger, ViabilityConsequence,
};

fn allele(id: &str) -> AlleleId {
    AlleleId::new(id).unwrap()
}

fn locus() -> LocusId {
    LocusId::new("focal").unwrap()
}

fn chromosome() -> ChromosomeId {
    ChromosomeId::new("chr-selection-design").unwrap()
}

fn ancestry(id: &str) -> AncestryCopyId {
    AncestryCopyId::new(id).unwrap()
}

fn individual_id(id: &str) -> EvolutionIndividualId {
    EvolutionIndividualId::new(id).unwrap()
}

fn population() -> PopulationId {
    PopulationId::new("selection-design-pop").unwrap()
}

fn schema() -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new("selection-design-schema").unwrap(),
        2,
        [LocusDefinition::new(locus(), [allele("a0"), allele("a1")]).unwrap()],
    )
    .unwrap()
}

fn chromosome_map(schema: &HereditarySchema) -> ChromosomeMap {
    ChromosomeMap::new(
        ChromosomeMapId::new("selection-design-map").unwrap(),
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
        EvolutionaryContextId::new("selection-design-context").unwrap(),
        1,
        EvolutionaryContextContentDigest::new([7; 32]),
        ConsequenceWindowId::new("selection-design-window").unwrap(),
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
            ConsequenceObservationId::new(format!("design-obs-{index}")).unwrap(),
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
    protocol_byte: u8,
) -> PhenotypeEvidenceRef {
    PhenotypeEvidenceRef::new(
        individual_id(id),
        PhenotypeEvidenceSourceId::new("design-phenotype-source").unwrap(),
        1,
        EvidenceContentDigest::new([21; 32]),
        EvidenceProtocolId::new("design-phenotype-protocol").unwrap(),
        EvidenceProtocolContentDigest::new([protocol_byte; 32]),
        context.canonical_digest().unwrap(),
    )
}

fn exposure(
    id: &str,
    context: &EvolutionaryContextRef,
    protocol_byte: u8,
) -> ExposureEvidenceRef {
    ExposureEvidenceRef::new(
        individual_id(id),
        ExposureEvidenceSourceId::new("design-exposure-source").unwrap(),
        1,
        EvidenceContentDigest::new([31; 32]),
        EvidenceProtocolId::new("design-exposure-protocol").unwrap(),
        EvidenceProtocolContentDigest::new([protocol_byte; 32]),
        context.canonical_digest().unwrap(),
    )
}

fn method(id: &str, byte: u8) -> AnalysisAuthorityRef {
    AnalysisAuthorityRef::new(
        AnalysisMethodId::new(id).unwrap(),
        1,
        AnalysisContentDigest::new([byte; 32]),
    )
}

fn comparison(byte: u8) -> ComparisonAuthorityRef {
    ComparisonAuthorityRef::new(
        ComparisonAuthorityId::new("design-comparison").unwrap(),
        1,
        AnalysisContentDigest::new([byte; 32]),
    )
}

fn calibration(byte: u8) -> CalibrationAuthorityRef {
    CalibrationAuthorityRef::new(
        CalibrationAuthorityId::new("design-calibration").unwrap(),
        1,
        AnalysisContentDigest::new([byte; 32]),
    )
}

fn uncertainty(byte: u8) -> UncertaintyPlan {
    UncertaintyPlan {
        method: method("uncertainty-method", byte),
        replicate_count: Some(100),
        seed_lineage_digest: Some(AnalysisContentDigest::new([byte.wrapping_add(1); 32])),
        multiplicity: None,
        minimum_information: method("minimum-information", byte.wrapping_add(2)),
        insufficient_support: InsufficientSupportPolicy::FailClosed,
    }
}

fn predictor() -> PredictorDefinitionRef {
    PredictorDefinitionRef::new(
        PredictorDefinitionId::new("phenotype-predictor").unwrap(),
        PredictorSourceKind::Phenotype,
        1,
        AnalysisContentDigest::new([61; 32]),
    )
}

fn selection_evidence(
    validated_consequences: &ValidatedConsequenceLedger<'_>,
    schema: &HereditarySchema,
    map: &ChromosomeMap,
    census: &ExplicitLinkedPopulationCensus,
    subjects: &[LinkedIndividualSubject<'_>],
    context: &EvolutionaryContextRef,
    phenotype_protocols: [u8; 2],
    second_exposure_available: bool,
) -> ExplicitSelectionEvidenceLedger {
    let second_exposure = if second_exposure_available {
        ExposureEvidenceStatus::CompleteWindow(exposure("individual-b", context, 81))
    } else {
        ExposureEvidenceStatus::Unavailable
    };
    ExplicitSelectionEvidenceLedger::capture(
        validated_consequences,
        schema,
        map,
        census,
        subjects,
        [
            IndividualSelectionEvidenceInput::new(
                individual_id("individual-a"),
                PhenotypeEvidenceStatus::CompleteWindow(phenotype(
                    "individual-a",
                    context,
                    phenotype_protocols[0],
                )),
                ExposureEvidenceStatus::CompleteWindow(exposure("individual-a", context, 81)),
            ),
            IndividualSelectionEvidenceInput::new(
                individual_id("individual-b"),
                PhenotypeEvidenceStatus::CompleteWindow(phenotype(
                    "individual-b",
                    context,
                    phenotype_protocols[1],
                )),
                second_exposure,
            ),
        ],
    )
    .unwrap()
}

#[allow(clippy::too_many_arguments)]
fn descriptive_design(
    evidence: &ValidatedSelectionEvidenceLedger<'_>,
    included: Vec<EvolutionIndividualId>,
    exclusions: Vec<ExclusionDeclaration>,
    phenotype_support: ChannelSupportPolicy,
    exposure_support: ChannelSupportPolicy,
    phenotype_protocols: ProtocolComparabilityPolicy,
    uncertainty: UncertaintyPlan,
) -> Result<SelectionComparisonDesign, SelectionDesignError> {
    SelectionComparisonDesign::declare(
        SelectionComparisonDesignId::new("design-v1").unwrap(),
        evidence,
        predictor(),
        SelectionEstimand::ViabilityWindowRiskContrast,
        SelectionDesignClass::DescriptiveAssociation,
        None,
        included,
        exclusions,
        method("eligibility-rule", 71),
        phenotype_support,
        exposure_support,
        phenotype_protocols,
        ProtocolComparabilityPolicy::ExactIdentityOnly,
        ConfoundingPolicy::DescriptiveNoCausalClaim,
        DeathBeforeEndpointPolicy::NotApplicable,
        CompetingRiskPolicy::NoneDeclared,
        uncertainty,
    )
}

#[test]
fn denominator_transformation_is_explicit_complete_and_order_invariant() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let first = individual(&schema, &map, "individual-a", "a-copy-0", "a-copy-1");
    let second = individual(&schema, &map, "individual-b", "b-copy-0", "b-copy-1");
    let subjects = [subject(&schema, &map, &first), subject(&schema, &map, &second)];
    let population = population();
    let census = ExplicitLinkedPopulationCensus::capture(population.clone(), &schema, &map, &subjects).unwrap();
    let context = context();
    let consequences = consequence_ledger(&schema, &map, &subjects, &census, &context);
    let validated_consequences = ValidatedConsequenceLedger::validate_current(
        &consequences, &population, &schema, &map, &census, &subjects, &context,
    ).unwrap();
    let evidence = selection_evidence(
        &validated_consequences, &schema, &map, &census, &subjects, &context, [91, 91], true,
    );
    let validated_evidence = ValidatedSelectionEvidenceLedger::validate_current(
        &evidence, &validated_consequences, &schema, &map, &census, &subjects,
    ).unwrap();

    assert!(matches!(
        descriptive_design(
            &validated_evidence,
            vec![individual_id("individual-a")],
            vec![],
            ChannelSupportPolicy::CompleteOnly,
            ChannelSupportPolicy::NotRequired,
            ProtocolComparabilityPolicy::ExactIdentityOnly,
            uncertainty(1),
        ),
        Err(SelectionDesignError::IncompleteDenominatorAccounting)
    ));

    let design = descriptive_design(
        &validated_evidence,
        vec![individual_id("individual-a")],
        vec![ExclusionDeclaration::new(
            individual_id("individual-b"),
            ExclusionReasonId::new("predeclared-exclusion").unwrap(),
        )],
        ChannelSupportPolicy::CompleteOnly,
        ChannelSupportPolicy::NotRequired,
        ProtocolComparabilityPolicy::ExactIdentityOnly,
        uncertainty(1),
    ).unwrap();
    assert_eq!(design.original_denominator(), 2);
    assert_eq!(design.estimand_denominator(), 1);

    let forward = descriptive_design(
        &validated_evidence,
        vec![individual_id("individual-a"), individual_id("individual-b")],
        vec![],
        ChannelSupportPolicy::CompleteOnly,
        ChannelSupportPolicy::NotRequired,
        ProtocolComparabilityPolicy::ExactIdentityOnly,
        uncertainty(1),
    ).unwrap();
    let reverse = descriptive_design(
        &validated_evidence,
        vec![individual_id("individual-b"), individual_id("individual-a")],
        vec![],
        ChannelSupportPolicy::CompleteOnly,
        ChannelSupportPolicy::NotRequired,
        ProtocolComparabilityPolicy::ExactIdentityOnly,
        uncertainty(1),
    ).unwrap();
    assert_eq!(forward, reverse);
    assert_eq!(forward.canonical_digest().unwrap(), reverse.canonical_digest().unwrap());
}

#[test]
fn unavailable_required_exposure_cannot_silently_become_zero_or_disappear() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let first = individual(&schema, &map, "individual-a", "a-copy-0", "a-copy-1");
    let second = individual(&schema, &map, "individual-b", "b-copy-0", "b-copy-1");
    let subjects = [subject(&schema, &map, &first), subject(&schema, &map, &second)];
    let population = population();
    let census = ExplicitLinkedPopulationCensus::capture(population.clone(), &schema, &map, &subjects).unwrap();
    let context = context();
    let consequences = consequence_ledger(&schema, &map, &subjects, &census, &context);
    let validated_consequences = ValidatedConsequenceLedger::validate_current(
        &consequences, &population, &schema, &map, &census, &subjects, &context,
    ).unwrap();
    let evidence = selection_evidence(
        &validated_consequences, &schema, &map, &census, &subjects, &context, [91, 91], false,
    );
    let validated_evidence = ValidatedSelectionEvidenceLedger::validate_current(
        &evidence, &validated_consequences, &schema, &map, &census, &subjects,
    ).unwrap();

    assert!(matches!(
        descriptive_design(
            &validated_evidence,
            vec![individual_id("individual-a"), individual_id("individual-b")],
            vec![],
            ChannelSupportPolicy::CompleteOnly,
            ChannelSupportPolicy::CompleteOnly,
            ProtocolComparabilityPolicy::ExactIdentityOnly,
            uncertainty(2),
        ),
        Err(SelectionDesignError::ExposureSupportInsufficient(_))
    ));

    let design = descriptive_design(
        &validated_evidence,
        vec![individual_id("individual-a"), individual_id("individual-b")],
        vec![],
        ChannelSupportPolicy::CompleteOnly,
        ChannelSupportPolicy::UnavailableAllowed {
            method: method("missing-exposure-policy", 72),
        },
        ProtocolComparabilityPolicy::ExactIdentityOnly,
        uncertainty(2),
    ).unwrap();
    assert_eq!(design.original_denominator(), 2);
}

#[test]
fn protocol_drift_fails_strict_comparability_but_requires_explicit_calibration_to_continue() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let first = individual(&schema, &map, "individual-a", "a-copy-0", "a-copy-1");
    let second = individual(&schema, &map, "individual-b", "b-copy-0", "b-copy-1");
    let subjects = [subject(&schema, &map, &first), subject(&schema, &map, &second)];
    let population = population();
    let census = ExplicitLinkedPopulationCensus::capture(population.clone(), &schema, &map, &subjects).unwrap();
    let context = context();
    let consequences = consequence_ledger(&schema, &map, &subjects, &census, &context);
    let validated_consequences = ValidatedConsequenceLedger::validate_current(
        &consequences, &population, &schema, &map, &census, &subjects, &context,
    ).unwrap();
    let evidence = selection_evidence(
        &validated_consequences, &schema, &map, &census, &subjects, &context, [91, 92], true,
    );
    let validated_evidence = ValidatedSelectionEvidenceLedger::validate_current(
        &evidence, &validated_consequences, &schema, &map, &census, &subjects,
    ).unwrap();

    assert!(matches!(
        descriptive_design(
            &validated_evidence,
            vec![individual_id("individual-a"), individual_id("individual-b")],
            vec![],
            ChannelSupportPolicy::CompleteOnly,
            ChannelSupportPolicy::NotRequired,
            ProtocolComparabilityPolicy::ExactIdentityOnly,
            uncertainty(3),
        ),
        Err(SelectionDesignError::PhenotypeProtocolMismatch)
    ));

    let calibrated = descriptive_design(
        &validated_evidence,
        vec![individual_id("individual-a"), individual_id("individual-b")],
        vec![],
        ChannelSupportPolicy::CompleteOnly,
        ChannelSupportPolicy::NotRequired,
        ProtocolComparabilityPolicy::Calibrated {
            authority: calibration(93),
        },
        uncertainty(3),
    ).unwrap();
    assert_eq!(calibrated.estimand_denominator(), 2);
}

#[test]
fn non_descriptive_design_requires_comparison_authority_and_endpoint_policy() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let first = individual(&schema, &map, "individual-a", "a-copy-0", "a-copy-1");
    let second = individual(&schema, &map, "individual-b", "b-copy-0", "b-copy-1");
    let subjects = [subject(&schema, &map, &first), subject(&schema, &map, &second)];
    let population = population();
    let census = ExplicitLinkedPopulationCensus::capture(population.clone(), &schema, &map, &subjects).unwrap();
    let context = context();
    let consequences = consequence_ledger(&schema, &map, &subjects, &census, &context);
    let validated_consequences = ValidatedConsequenceLedger::validate_current(
        &consequences, &population, &schema, &map, &census, &subjects, &context,
    ).unwrap();
    let evidence = selection_evidence(
        &validated_consequences, &schema, &map, &census, &subjects, &context, [91, 91], true,
    );
    let validated_evidence = ValidatedSelectionEvidenceLedger::validate_current(
        &evidence, &validated_consequences, &schema, &map, &census, &subjects,
    ).unwrap();

    let declare = |comparison_authority, death_before_endpoint| {
        SelectionComparisonDesign::declare(
            SelectionComparisonDesignId::new("controlled-design").unwrap(),
            &validated_evidence,
            predictor(),
            SelectionEstimand::DescendantRecruitmentContrast,
            SelectionDesignClass::ControlledSimulationIntervention,
            comparison_authority,
            vec![individual_id("individual-a"), individual_id("individual-b")],
            vec![],
            method("eligibility-rule", 71),
            ChannelSupportPolicy::CompleteOnly,
            ChannelSupportPolicy::CompleteOnly,
            ProtocolComparabilityPolicy::ExactIdentityOnly,
            ProtocolComparabilityPolicy::ExactIdentityOnly,
            ConfoundingPolicy::DesignBasedNoAdjustmentDeclared,
            death_before_endpoint,
            CompetingRiskPolicy::NoneDeclared,
            uncertainty(4),
        )
    };

    assert!(matches!(
        declare(None, DeathBeforeEndpointPolicy::EndpointFailure),
        Err(SelectionDesignError::MissingComparisonAuthority)
    ));
    assert!(matches!(
        declare(Some(comparison(94)), DeathBeforeEndpointPolicy::NotApplicable),
        Err(SelectionDesignError::MissingDeathBeforeEndpointPolicy)
    ));
    let valid = declare(
        Some(comparison(94)),
        DeathBeforeEndpointPolicy::EndpointFailure,
    ).unwrap();
    assert_eq!(valid.design_class, SelectionDesignClass::ControlledSimulationIntervention);
}

#[test]
fn estimand_uncertainty_and_design_class_are_part_of_evidence_identity() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let first = individual(&schema, &map, "individual-a", "a-copy-0", "a-copy-1");
    let second = individual(&schema, &map, "individual-b", "b-copy-0", "b-copy-1");
    let subjects = [subject(&schema, &map, &first), subject(&schema, &map, &second)];
    let population = population();
    let census = ExplicitLinkedPopulationCensus::capture(population.clone(), &schema, &map, &subjects).unwrap();
    let context = context();
    let consequences = consequence_ledger(&schema, &map, &subjects, &census, &context);
    let validated_consequences = ValidatedConsequenceLedger::validate_current(
        &consequences, &population, &schema, &map, &census, &subjects, &context,
    ).unwrap();
    let evidence = selection_evidence(
        &validated_consequences, &schema, &map, &census, &subjects, &context, [91, 91], true,
    );
    let validated_evidence = ValidatedSelectionEvidenceLedger::validate_current(
        &evidence, &validated_consequences, &schema, &map, &census, &subjects,
    ).unwrap();

    let first_design = descriptive_design(
        &validated_evidence,
        vec![individual_id("individual-a"), individual_id("individual-b")],
        vec![],
        ChannelSupportPolicy::CompleteOnly,
        ChannelSupportPolicy::NotRequired,
        ProtocolComparabilityPolicy::ExactIdentityOnly,
        uncertainty(5),
    ).unwrap();
    let second_design = descriptive_design(
        &validated_evidence,
        vec![individual_id("individual-a"), individual_id("individual-b")],
        vec![],
        ChannelSupportPolicy::CompleteOnly,
        ChannelSupportPolicy::NotRequired,
        ProtocolComparabilityPolicy::ExactIdentityOnly,
        uncertainty(6),
    ).unwrap();
    assert_ne!(
        first_design.canonical_digest().unwrap(),
        second_design.canonical_digest().unwrap()
    );

    let descendant = SelectionComparisonDesign::declare(
        SelectionComparisonDesignId::new("descendant-design").unwrap(),
        &validated_evidence,
        predictor(),
        SelectionEstimand::DescendantProductionContrast,
        SelectionDesignClass::DescriptiveAssociation,
        None,
        vec![individual_id("individual-a"), individual_id("individual-b")],
        vec![],
        method("eligibility-rule", 71),
        ChannelSupportPolicy::CompleteOnly,
        ChannelSupportPolicy::NotRequired,
        ProtocolComparabilityPolicy::ExactIdentityOnly,
        ProtocolComparabilityPolicy::ExactIdentityOnly,
        ConfoundingPolicy::DescriptiveNoCausalClaim,
        DeathBeforeEndpointPolicy::EndpointFailure,
        CompetingRiskPolicy::NoneDeclared,
        uncertainty(5),
    ).unwrap();
    assert_ne!(
        first_design.canonical_digest().unwrap(),
        descendant.canonical_digest().unwrap()
    );
}

#[test]
fn serde_restored_design_must_replay_current_evidence_before_analysis() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let first = individual(&schema, &map, "individual-a", "a-copy-0", "a-copy-1");
    let second = individual(&schema, &map, "individual-b", "b-copy-0", "b-copy-1");
    let subjects = [subject(&schema, &map, &first), subject(&schema, &map, &second)];
    let population = population();
    let census = ExplicitLinkedPopulationCensus::capture(population.clone(), &schema, &map, &subjects).unwrap();
    let context = context();
    let consequences = consequence_ledger(&schema, &map, &subjects, &census, &context);
    let validated_consequences = ValidatedConsequenceLedger::validate_current(
        &consequences, &population, &schema, &map, &census, &subjects, &context,
    ).unwrap();
    let evidence = selection_evidence(
        &validated_consequences, &schema, &map, &census, &subjects, &context, [91, 91], true,
    );
    let validated_evidence = ValidatedSelectionEvidenceLedger::validate_current(
        &evidence, &validated_consequences, &schema, &map, &census, &subjects,
    ).unwrap();
    let design = descriptive_design(
        &validated_evidence,
        vec![individual_id("individual-a"), individual_id("individual-b")],
        vec![],
        ChannelSupportPolicy::CompleteOnly,
        ChannelSupportPolicy::NotRequired,
        ProtocolComparabilityPolicy::ExactIdentityOnly,
        uncertainty(7),
    ).unwrap();

    let encoded = serde_json::to_vec(&design).unwrap();
    let restored: SelectionComparisonDesign = serde_json::from_slice(&encoded).unwrap();
    let validated_design = ValidatedSelectionComparisonDesign::validate_current(
        &restored,
        &validated_evidence,
    ).unwrap();
    assert_eq!(validated_design.design_digest(), design.canonical_digest().unwrap());

    let changed_evidence = selection_evidence(
        &validated_consequences, &schema, &map, &census, &subjects, &context, [99, 99], true,
    );
    let changed_validated = ValidatedSelectionEvidenceLedger::validate_current(
        &changed_evidence, &validated_consequences, &schema, &map, &census, &subjects,
    ).unwrap();
    assert!(restored.validate_current(&changed_validated).is_err());
}

#[test]
fn comparison_design_wire_shape_contains_no_result_or_adaptation_claims() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let first = individual(&schema, &map, "individual-a", "a-copy-0", "a-copy-1");
    let second = individual(&schema, &map, "individual-b", "b-copy-0", "b-copy-1");
    let subjects = [subject(&schema, &map, &first), subject(&schema, &map, &second)];
    let population = population();
    let census = ExplicitLinkedPopulationCensus::capture(population.clone(), &schema, &map, &subjects).unwrap();
    let context = context();
    let consequences = consequence_ledger(&schema, &map, &subjects, &census, &context);
    let validated_consequences = ValidatedConsequenceLedger::validate_current(
        &consequences, &population, &schema, &map, &census, &subjects, &context,
    ).unwrap();
    let evidence = selection_evidence(
        &validated_consequences, &schema, &map, &census, &subjects, &context, [91, 91], true,
    );
    let validated_evidence = ValidatedSelectionEvidenceLedger::validate_current(
        &evidence, &validated_consequences, &schema, &map, &census, &subjects,
    ).unwrap();
    let design = descriptive_design(
        &validated_evidence,
        vec![individual_id("individual-a"), individual_id("individual-b")],
        vec![],
        ChannelSupportPolicy::CompleteOnly,
        ChannelSupportPolicy::NotRequired,
        ProtocolComparabilityPolicy::ExactIdentityOnly,
        uncertainty(8),
    ).unwrap();

    let value = serde_json::to_value(design).unwrap();
    let object = value.as_object().unwrap();
    for forbidden in [
        "fitness",
        "selection_coefficient",
        "beneficial",
        "deleterious",
        "adaptation",
        "causal_effect",
        "estimate",
        "p_value",
    ] {
        assert!(!object.contains_key(forbidden));
    }
}
