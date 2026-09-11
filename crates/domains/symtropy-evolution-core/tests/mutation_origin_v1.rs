use symtropy_evolution_core::{
    append_descendant_ancestry_to_graph, assemble_diploid_linked_offspring_from_evidence,
    declare_at_birth_mutation_origin, derive_descendant_ancestry,
    derive_modeled_gamete_ancestry, derive_zero_crossover_linked_gamete, AlleleId,
    AncestryCopyId, AncestryGeneration, AncestryGraphNode, ChromosomeAncestryState,
    ChromosomeDefinition, ChromosomeHaplotype, ChromosomeId, ChromosomeLocus,
    ChromosomeMap, ChromosomeMapId, ChromosomeRecombinationDomain,
    ChromosomeRecombinationModel, ChromosomeRecombinationProfile,
    ChromosomeRecombinationProfileId, DescendantAncestryDerivation,
    EvolutionOperatorProfile, GeneticMapIntervalMicromorgans,
    GeneticMapPositionMicromorgans, HaplotypeAncestryClass, HereditarySchema,
    HereditarySchemaId, LinkedGameteDerivationEvidence, LocusDefinition, LocusId,
    ModeledAncestryGraph, MutationOrigin, MutationOriginError, MutationProfile,
    OperatorProfileId, ParentRole, PhasedAncestryState, PhasedChromosomeState,
    PhasedHereditaryState, RecombinationMode, RecombinationProfile,
    ReproductionEventId,
};

fn allele(id: &str) -> AlleleId {
    AlleleId::new(id).unwrap()
}

fn locus(id: &str) -> LocusId {
    LocusId::new(id).unwrap()
}

fn chromosome() -> ChromosomeId {
    ChromosomeId::new("chr-a").unwrap()
}

fn ancestry(id: &str) -> AncestryCopyId {
    AncestryCopyId::new(id).unwrap()
}

fn pos(value: u64) -> GeneticMapPositionMicromorgans {
    GeneticMapPositionMicromorgans::new(value)
}

fn schema() -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new("mut-05a-schema").unwrap(),
        2,
        [
            LocusDefinition::new(locus("a"), [allele("a0"), allele("a1")]).unwrap(),
            LocusDefinition::new(locus("b"), [allele("b0"), allele("b1")]).unwrap(),
        ],
    )
    .unwrap()
}

fn map(schema: &HereditarySchema) -> ChromosomeMap {
    ChromosomeMap::new(
        ChromosomeMapId::new("mut-05a-map").unwrap(),
        schema,
        [ChromosomeDefinition::new(
            chromosome(),
            vec![
                ChromosomeLocus::new(locus("a"), pos(1)),
                ChromosomeLocus::new(locus("b"), pos(7_000_001)),
            ],
        )
        .unwrap()],
    )
    .unwrap()
}

fn source(
    schema: &HereditarySchema,
    map: &ChromosomeMap,
    first: &[&str],
    second: &[&str],
) -> PhasedHereditaryState {
    PhasedHereditaryState::new(
        schema,
        map,
        [PhasedChromosomeState::new(
            chromosome(),
            vec![
                ChromosomeHaplotype::new(first.iter().map(|id| allele(id)).collect()),
                ChromosomeHaplotype::new(second.iter().map(|id| allele(id)).collect()),
            ],
        )],
    )
    .unwrap()
}

fn ancestry_state(
    schema: &HereditarySchema,
    map: &ChromosomeMap,
    source: &PhasedHereditaryState,
    first: &str,
    second: &str,
) -> PhasedAncestryState {
    PhasedAncestryState::new(
        schema,
        map,
        source,
        [ChromosomeAncestryState::new(
            chromosome(),
            vec![
                HaplotypeAncestryClass::new(0, vec![ancestry(first)]).unwrap(),
                HaplotypeAncestryClass::new(1, vec![ancestry(second)]).unwrap(),
            ],
        )
        .unwrap()],
    )
    .unwrap()
}

fn recombination_profile(
    schema: &HereditarySchema,
    map: &ChromosomeMap,
    id: &str,
) -> ChromosomeRecombinationProfile {
    ChromosomeRecombinationProfile::new(
        ChromosomeRecombinationProfileId::new(id).unwrap(),
        schema,
        map,
        ChromosomeRecombinationModel::NoCrossoversIndependentAssortmentV1,
        [ChromosomeRecombinationDomain::new(
            chromosome(),
            GeneticMapIntervalMicromorgans::new(pos(0), pos(7_000_002)).unwrap(),
        )
        .unwrap()],
    )
    .unwrap()
}

fn operators(rate_ppm: u32) -> EvolutionOperatorProfile {
    EvolutionOperatorProfile {
        profile_id: OperatorProfileId::new("mut-05a-operators").unwrap(),
        version: "1".to_owned(),
        mutation: MutationProfile {
            model_id: "modeled-locus-substitution".to_owned(),
            version: "1".to_owned(),
            per_copy_rate_ppm: rate_ppm,
        },
        recombination: RecombinationProfile {
            model_id: "linked-chromosome-context".to_owned(),
            version: "1".to_owned(),
            mode: RecombinationMode::IndependentLoci,
        },
    }
}

struct Fixture {
    schema: HereditarySchema,
    map: ChromosomeMap,
    source_a: PhasedHereditaryState,
    source_b: PhasedHereditaryState,
    ancestry_a: PhasedAncestryState,
    ancestry_b: PhasedAncestryState,
    profile_a: ChromosomeRecombinationProfile,
    profile_b: ChromosomeRecombinationProfile,
    gamete_a: LinkedGameteDerivationEvidence,
    gamete_b: LinkedGameteDerivationEvidence,
    gamete_ancestry_a: symtropy_evolution_core::GameteAncestryDerivation,
    gamete_ancestry_b: symtropy_evolution_core::GameteAncestryDerivation,
    descendant: DescendantAncestryDerivation,
}

fn make_fixture(event_name: &str) -> Fixture {
    let schema = schema();
    let map = map(&schema);
    let event = ReproductionEventId::new(event_name).unwrap();
    let source_a = source(&schema, &map, &["a0", "b0"], &["a1", "b1"]);
    let source_b = source(&schema, &map, &["a1", "b0"], &["a0", "b1"]);
    let ancestry_a = ancestry_state(&schema, &map, &source_a, "a-root-0", "a-root-1");
    let ancestry_b = ancestry_state(&schema, &map, &source_b, "b-root-0", "b-root-1");
    let profile_a = recombination_profile(&schema, &map, "mut-05a-parent-a");
    let profile_b = recombination_profile(&schema, &map, "mut-05a-parent-b");

    let gamete_a = LinkedGameteDerivationEvidence::ZeroCrossover(
        derive_zero_crossover_linked_gamete(
            &schema,
            &map,
            &source_a,
            &profile_a,
            &event,
            ParentRole::ParentA,
        )
        .unwrap(),
    );
    let gamete_b = LinkedGameteDerivationEvidence::ZeroCrossover(
        derive_zero_crossover_linked_gamete(
            &schema,
            &map,
            &source_b,
            &profile_b,
            &event,
            ParentRole::ParentB,
        )
        .unwrap(),
    );
    let gamete_ancestry_a = derive_modeled_gamete_ancestry(
        &schema,
        &map,
        &source_a,
        &ancestry_a,
        &profile_a,
        &gamete_a,
        &event,
        ParentRole::ParentA,
    )
    .unwrap();
    let gamete_ancestry_b = derive_modeled_gamete_ancestry(
        &schema,
        &map,
        &source_b,
        &ancestry_b,
        &profile_b,
        &gamete_b,
        &event,
        ParentRole::ParentB,
    )
    .unwrap();
    let offspring = assemble_diploid_linked_offspring_from_evidence(
        &schema,
        &map,
        &source_a,
        &profile_a,
        &gamete_a,
        &source_b,
        &profile_b,
        &gamete_b,
        &event,
    )
    .unwrap();
    let descendant = derive_descendant_ancestry(
        &schema,
        &map,
        &source_a,
        &ancestry_a,
        &profile_a,
        &gamete_a,
        &gamete_ancestry_a,
        &source_b,
        &ancestry_b,
        &profile_b,
        &gamete_b,
        &gamete_ancestry_b,
        &offspring,
        &event,
    )
    .unwrap();

    Fixture {
        schema,
        map,
        source_a,
        source_b,
        ancestry_a,
        ancestry_b,
        profile_a,
        profile_b,
        gamete_a,
        gamete_b,
        gamete_ancestry_a,
        gamete_ancestry_b,
        descendant,
    }
}

fn root_graph(fixture: &Fixture) -> ModeledAncestryGraph {
    ModeledAncestryGraph::new_roots(
        &fixture.schema,
        &fixture.map,
        [
            AncestryGraphNode::root(ancestry("b-root-1"), chromosome(), AncestryGeneration::new(0)),
            AncestryGraphNode::root(ancestry("a-root-0"), chromosome(), AncestryGeneration::new(0)),
            AncestryGraphNode::root(ancestry("b-root-0"), chromosome(), AncestryGeneration::new(0)),
            AncestryGraphNode::root(ancestry("a-root-1"), chromosome(), AncestryGeneration::new(0)),
        ],
    )
    .unwrap()
}

fn append(graph: &ModeledAncestryGraph, fixture: &Fixture, generation: u64) -> ModeledAncestryGraph {
    append_descendant_ancestry_to_graph(
        graph,
        &fixture.schema,
        &fixture.map,
        &fixture.source_a,
        &fixture.ancestry_a,
        &fixture.profile_a,
        &fixture.gamete_a,
        &fixture.gamete_ancestry_a,
        &fixture.source_b,
        &fixture.ancestry_b,
        &fixture.profile_b,
        &fixture.gamete_b,
        &fixture.gamete_ancestry_b,
        &fixture.descendant,
        AncestryGeneration::new(generation),
    )
    .unwrap()
    .graph
}

fn parent_a_copy(fixture: &Fixture) -> &AncestryCopyId {
    &fixture
        .descendant
        .materialization
        .descendant_copies
        .iter()
        .find(|copy| copy.parent_role == ParentRole::ParentA)
        .unwrap()
        .child_copy_id
}

fn parent_a_ancestral_a(fixture: &Fixture) -> AlleleId {
    fixture.gamete_a.gamete().chromosomes[&chromosome()].alleles[0].clone()
}

fn different_a(ancestral: &AlleleId) -> AlleleId {
    if ancestral == &allele("a0") {
        allele("a1")
    } else {
        allele("a0")
    }
}

fn declare(
    fixture: &Fixture,
    graph: &ModeledAncestryGraph,
    operators: &EvolutionOperatorProfile,
    copy_id: &AncestryCopyId,
    locus_id: &LocusId,
    derived: &AlleleId,
) -> Result<MutationOrigin, MutationOriginError> {
    declare_at_birth_mutation_origin(
        &fixture.schema,
        &fixture.map,
        operators,
        &fixture.source_a,
        &fixture.ancestry_a,
        &fixture.profile_a,
        &fixture.gamete_a,
        &fixture.gamete_ancestry_a,
        &fixture.source_b,
        &fixture.ancestry_b,
        &fixture.profile_b,
        &fixture.gamete_b,
        &fixture.gamete_ancestry_b,
        &fixture.descendant,
        graph,
        copy_id,
        locus_id,
        derived,
    )
}

fn validate(
    origin: &MutationOrigin,
    fixture: &Fixture,
    graph: &ModeledAncestryGraph,
    operators: &EvolutionOperatorProfile,
) -> Result<(), MutationOriginError> {
    origin.validate_current(
        &fixture.schema,
        &fixture.map,
        operators,
        &fixture.source_a,
        &fixture.ancestry_a,
        &fixture.profile_a,
        &fixture.gamete_a,
        &fixture.gamete_ancestry_a,
        &fixture.source_b,
        &fixture.ancestry_b,
        &fixture.profile_b,
        &fixture.gamete_b,
        &fixture.gamete_ancestry_b,
        &fixture.descendant,
        graph,
    )
}

#[test]
fn mutation_origin_replays_by_parent_contribution_and_survives_unrelated_graph_growth() {
    let first = make_fixture("mut-05a-birth-1");
    let graph = append(&root_graph(&first), &first, 1);
    let operator_authority = operators(125);
    let ancestral = parent_a_ancestral_a(&first);
    let derived = different_a(&ancestral);
    let origin = declare(
        &first,
        &graph,
        &operator_authority,
        parent_a_copy(&first),
        &locus("a"),
        &derived,
    )
    .unwrap();

    let selected_copy = first
        .descendant
        .materialization
        .descendant_copies
        .iter()
        .find(|copy| &copy.child_copy_id == origin.ancestry_copy_id())
        .unwrap();
    assert_eq!(selected_copy.parent_role, ParentRole::ParentA);
    assert_eq!(origin.parent_role(), ParentRole::ParentA);
    assert_eq!(origin.ancestral_allele(), &ancestral);
    assert_eq!(origin.derived_allele(), &derived);
    assert_eq!(origin.chromosome_id(), &chromosome());
    assert_eq!(origin.locus_id(), &locus("a"));
    assert_eq!(origin.ancestry_generation(), AncestryGeneration::new(1));

    let digest = origin.canonical_digest();
    let encoded = serde_json::to_vec(&origin).unwrap();
    let restored: MutationOrigin = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(restored, origin);
    assert_eq!(restored.canonical_digest(), digest);
    validate(&restored, &first, &graph, &operator_authority).unwrap();

    // Historical origin identity is local. A later, distinct descendant can
    // extend the graph without rewriting an already valid origin receipt.
    let later = make_fixture("mut-05a-unrelated-future-birth");
    let extended = append(&graph, &later, 2);
    assert_ne!(
        graph.canonical_digest(&first.schema, &first.map).unwrap(),
        extended.canonical_digest(&first.schema, &first.map).unwrap()
    );
    validate(&restored, &first, &extended, &operator_authority).unwrap();
    assert_eq!(restored.canonical_digest(), digest);

    // Exact operator context is authority. Changing it cannot reinterpret the
    // old origin under a new mutation rate/model context.
    assert!(matches!(
        validate(&restored, &first, &extended, &operators(126)),
        Err(MutationOriginError::ReplayMismatch)
    ));
}

#[test]
fn mutation_origin_fails_closed_for_invalid_local_claims() {
    let fixture = make_fixture("mut-05a-negative-birth");
    let graph = append(&root_graph(&fixture), &fixture, 1);
    let operator_authority = operators(125);
    let ancestral = parent_a_ancestral_a(&fixture);

    assert!(matches!(
        declare(
            &fixture,
            &graph,
            &operator_authority,
            parent_a_copy(&fixture),
            &locus("a"),
            &ancestral,
        ),
        Err(MutationOriginError::NoAllelicChange)
    ));

    assert!(matches!(
        declare(
            &fixture,
            &graph,
            &operator_authority,
            parent_a_copy(&fixture),
            &locus("not-modeled"),
            &allele("a1"),
        ),
        Err(MutationOriginError::LocusNotOnChromosome(_))
    ));

    assert!(matches!(
        declare(
            &fixture,
            &graph,
            &operator_authority,
            parent_a_copy(&fixture),
            &locus("a"),
            &allele("a-unknown"),
        ),
        Err(MutationOriginError::DerivedAlleleNotAllowed(_))
    ));

    assert!(matches!(
        declare(
            &fixture,
            &graph,
            &operator_authority,
            &ancestry("a-root-0"),
            &locus("a"),
            &different_a(&ancestral),
        ),
        Err(MutationOriginError::DescendantCopyMissing(_))
    ));
}
