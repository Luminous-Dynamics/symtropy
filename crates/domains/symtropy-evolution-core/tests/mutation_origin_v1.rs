use symtropy_evolution_core::{
    append_descendant_ancestry_to_graph, assemble_diploid_linked_offspring_from_evidence,
    declare_at_birth_mutation_origin, derive_descendant_ancestry,
    derive_marker_marginal_poisson_linked_gamete, derive_modeled_gamete_ancestry,
    derive_zero_crossover_linked_gamete, AlleleId, AncestryCopyId, AncestryGeneration,
    AncestryGraphNode, ChromosomeAncestryState, ChromosomeDefinition, ChromosomeHaplotype,
    ChromosomeId, ChromosomeLocus, ChromosomeMap, ChromosomeMapId,
    ChromosomeRecombinationDomain, ChromosomeRecombinationModel,
    ChromosomeRecombinationProfile, ChromosomeRecombinationProfileId,
    DescendantAncestryDerivation, EvolutionOperatorProfile,
    GeneticMapIntervalMicromorgans, GeneticMapPositionMicromorgans,
    HaplotypeAncestryClass, HereditarySchema, HereditarySchemaId,
    LinkedGameteDerivationEvidence, LocusDefinition, LocusId, ModeledAncestryGraph,
    MutationOrigin, MutationOriginError, MutationProfile, OperatorProfileId, ParentRole,
    PhasedAncestryState, PhasedChromosomeState, PhasedHereditaryState,
    RecombinationMode, RecombinationProfile, ReproductionEventId,
};

fn allele(id: &str) -> AlleleId {
    AlleleId::new(id).unwrap()
}

fn locus(id: &str) -> LocusId {
    LocusId::new(id).unwrap()
}

fn chromosome(id: &str) -> ChromosomeId {
    ChromosomeId::new(id).unwrap()
}

fn ancestry(id: &str) -> AncestryCopyId {
    AncestryCopyId::new(id).unwrap()
}

fn pos(value: u64) -> GeneticMapPositionMicromorgans {
    GeneticMapPositionMicromorgans::new(value)
}

fn interval(start: u64, end: u64) -> GeneticMapIntervalMicromorgans {
    GeneticMapIntervalMicromorgans::new(pos(start), pos(end)).unwrap()
}

fn hap(ids: &[&str]) -> ChromosomeHaplotype {
    ChromosomeHaplotype::new(ids.iter().map(|id| allele(id)).collect())
}

fn schema() -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new("mut-05a-schema").unwrap(),
        2,
        [
            LocusDefinition::new(locus("a"), [allele("a0"), allele("a1")]).unwrap(),
            LocusDefinition::new(locus("b"), [allele("b0"), allele("b1")]).unwrap(),
            LocusDefinition::new(locus("c"), [allele("c0"), allele("c1")]).unwrap(),
        ],
    )
    .unwrap()
}

fn map(schema: &HereditarySchema) -> ChromosomeMap {
    ChromosomeMap::new(
        ChromosomeMapId::new("mut-05a-map").unwrap(),
        schema,
        [ChromosomeDefinition::new(
            chromosome("chr-a"),
            vec![
                ChromosomeLocus::new(locus("a"), pos(1)),
                ChromosomeLocus::new(locus("b"), pos(7_000_001)),
                ChromosomeLocus::new(locus("c"), pos(14_000_001)),
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
    model: ChromosomeRecombinationModel,
) -> ChromosomeRecombinationProfile {
    ChromosomeRecombinationProfile::new(
        ChromosomeRecombinationProfileId::new(id).unwrap(),
        schema,
        map,
        model,
        [ChromosomeRecombinationDomain::new(
            chromosome("chr-a"),
            interval(0, 14_000_002),
        )
        .unwrap()],
    )
    .unwrap()
}

fn operator_profile(rate_ppm: u32) -> EvolutionOperatorProfile {
    EvolutionOperatorProfile {
        profile_id: OperatorProfileId::new("mut-05a-operators").unwrap(),
        version: "1".to_owned(),
        mutation: MutationProfile {
            model_id: "modeled-locus-substitution".to_owned(),
            version: "1".to_owned(),
            per_copy_rate_ppm: rate_ppm,
        },
        recombination: RecombinationProfile {
            model_id: "linked-chromosome-authority".to_owned(),
            version: "1".to_owned(),
            mode: RecombinationMode::IndependentLoci,
        },
    }
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
            chromosome("chr-a"),
            vec![hap(first), hap(second)],
        )],
    )
    .unwrap()
}

fn ancestry_class(slot: u8, ids: &[&str]) -> HaplotypeAncestryClass {
    HaplotypeAncestryClass::new(slot, ids.iter().map(|id| ancestry(id)).collect()).unwrap()
}

fn distinct_ancestry(
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
            chromosome("chr-a"),
            vec![
                ancestry_class(0, &[first]),
                ancestry_class(1, &[second]),
            ],
        )
        .unwrap()],
    )
    .unwrap()
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

fn fixture(event_name: &str, recombinant_parent_a: bool) -> Fixture {
    let schema = schema();
    let map = map(&schema);
    let event = ReproductionEventId::new(event_name).unwrap();
    let source_a = source(
        &schema,
        &map,
        &["a0", "b0", "c0"],
        &["a1", "b1", "c1"],
    );
    let source_b = source(
        &schema,
        &map,
        &["a0", "b1", "c0"],
        &["a1", "b0", "c1"],
    );
    let ancestry_a = distinct_ancestry(&schema, &map, &source_a, "a-root-0", "a-root-1");
    let ancestry_b = distinct_ancestry(&schema, &map, &source_b, "b-root-0", "b-root-1");
    let profile_a = recombination_profile(
        &schema,
        &map,
        if recombinant_parent_a { "a-poisson" } else { "a-zero" },
        if recombinant_parent_a {
            ChromosomeRecombinationModel::PoissonCrossoversNoInterferenceV1
        } else {
            ChromosomeRecombinationModel::NoCrossoversIndependentAssortmentV1
        },
    );
    let profile_b = recombination_profile(
        &schema,
        &map,
        "b-zero",
        ChromosomeRecombinationModel::NoCrossoversIndependentAssortmentV1,
    );
    let gamete_a = if recombinant_parent_a {
        LinkedGameteDerivationEvidence::MarkerMarginalPoisson(
            derive_marker_marginal_poisson_linked_gamete(
                &schema,
                &map,
                &source_a,
                &profile_a,
                &event,
                ParentRole::ParentA,
            )
            .unwrap(),
        )
    } else {
        LinkedGameteDerivationEvidence::ZeroCrossover(
            derive_zero_crossover_linked_gamete(
                &schema,
                &map,
                &source_a,
                &profile_a,
                &event,
                ParentRole::ParentA,
            )
            .unwrap(),
        )
    };
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
            AncestryGraphNode::root(
                ancestry("b-root-1"),
                chromosome("chr-a"),
                AncestryGeneration::new(0),
            ),
            AncestryGraphNode::root(
                ancestry("a-root-0"),
                chromosome("chr-a"),
                AncestryGeneration::new(0),
            ),
            AncestryGraphNode::root(
                ancestry("b-root-0"),
                chromosome("chr-a"),
                AncestryGeneration::new(0),
            ),
            AncestryGraphNode::root(
                ancestry("a-root-1"),
                chromosome("chr-a"),
                AncestryGeneration::new(0),
            ),
        ],
    )
    .unwrap()
}

fn append(
    graph: &ModeledAncestryGraph,
    fixture: &Fixture,
    generation: u64,
) -> ModeledAncestryGraph {
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

fn declare_for_parent_a(
    fixture: &Fixture,
    graph: &ModeledAncestryGraph,
    operators: &EvolutionOperatorProfile,
) -> MutationOrigin {
    let copy = fixture
        .descendant
        .materialization
        .descendant_copies
        .iter()
        .find(|copy| copy.parent_role == ParentRole::ParentA)
        .unwrap();
    let ancestral = &fixture.gamete_a.gamete().chromosomes[&chromosome("chr-a")].alleles[0];
    let derived = if ancestral == &allele("a0") {
        allele("a1")
    } else {
        allele("a0")
    };

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
        &copy.child_copy_id,
        &locus("a"),
        &derived,
    )
    .unwrap()
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
fn mutation_origin_replays_from_parent_contribution_and_survives_unrelated_graph_growth() {
    let fixture = fixture("mut-05a-birth-1", true);
    let graph = append(&root_graph(&fixture), &fixture, 1);
    let operators = operator_profile(125);
    let origin = declare_for_parent_a(&fixture, &graph, &operators);

    let selected_copy = fixture
        .descendant
        .materialization
        .descendant_copies
        .iter()
        .find(|copy| copy.child_copy_id == *origin.ancestry_copy_id())
        .unwrap();
    assert_eq!(selected_copy.parent_role, ParentRole::ParentA);
    assert_eq!(origin.parent_role(), ParentRole::ParentA);
    assert_eq!(origin.chromosome_id(), &chromosome("chr-a"));
    assert_eq!(origin.locus_id(), &locus("a"));
    assert_eq!(origin.ancestry_generation(), AncestryGeneration::new(1));
    assert_eq!(
        origin.ancestral_allele(),
        &fixture.gamete_a.gamete().chromosomes[&chromosome("chr-a")].alleles[0]
    );
    assert_ne!(origin.ancestral_allele(), origin.derived_allele());

    let digest = origin.canonical_digest();
    let encoded = serde_json::to_vec(&origin).unwrap();
    let restored: MutationOrigin = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(restored, origin);
    assert_eq!(restored.canonical_digest(), digest);
    validate(&restored, &fixture, &graph, &operators).unwrap();

    // Future graph growth is not part of historical mutation identity. Append a
    // distinct descendant from the same root authorities at a later generation.
    let later = fixture("mut-05a-unrelated-future-birth", false);
    let extended = append(&graph, &later, 2);
    assert_ne!(
        graph.canonical_digest(&fixture.schema, &fixture.map).unwrap(),
        extended
            .canonical_digest(&fixture.schema, &fixture.map)
            .unwrap()
    );
    validate(&restored, &fixture, &extended, &operators).unwrap();
    assert_eq!(restored.canonical_digest(), digest);

    // The operator profile is exact context authority. Drift must not silently
    // reinterpret the old receipt under a different mutation model/rate.
    let drifted_operators = operator_profile(126);
    assert!(matches!(
        validate(&restored, &fixture, &extended, &drifted_operators),
        Err(MutationOriginError::ReplayMismatch)
    ));
}

#[test]
fn mutation_origin_rejects_noops_unmodeled_loci_unknown_alleles_and_non_descendant_copies() {
    let fixture = fixture("mut-05a-birth-negative", true);
    let graph = append(&root_graph(&fixture), &fixture, 1);
    let operators = operator_profile(125);
    let copy = fixture
        .descendant
        .materialization
        .descendant_copies
        .iter()
        .find(|copy| copy.parent_role == ParentRole::ParentA)
        .unwrap();
    let ancestral = fixture.gamete_a.gamete().chromosomes[&chromosome("chr-a")].alleles[0]
        .clone();

    let no_op = declare_at_birth_mutation_origin(
        &fixture.schema,
        &fixture.map,
        &operators,
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
        &graph,
        &copy.child_copy_id,
        &locus("a"),
        &ancestral,
    );
    assert!(matches!(no_op, Err(MutationOriginError::NoAllelicChange)));

    let unmodeled = declare_at_birth_mutation_origin(
        &fixture.schema,
        &fixture.map,
        &operators,
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
        &graph,
        &copy.child_copy_id,
        &locus("not-modeled"),
        &allele("a1"),
    );
    assert!(matches!(
        unmodeled,
        Err(MutationOriginError::LocusNotOnChromosome(_))
    ));

    let unknown_allele = declare_at_birth_mutation_origin(
        &fixture.schema,
        &fixture.map,
        &operators,
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
        &graph,
        &copy.child_copy_id,
        &locus("a"),
        &allele("a-unknown"),
    );
    assert!(matches!(
        unknown_allele,
        Err(MutationOriginError::DerivedAlleleNotAllowed(_))
    ));

    let root_copy = declare_at_birth_mutation_origin(
        &fixture.schema,
        &fixture.map,
        &operators,
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
        &graph,
        &ancestry("a-root-0"),
        &locus("a"),
        &allele("a1"),
    );
    assert!(matches!(
        root_copy,
        Err(MutationOriginError::DescendantCopyMissing(_))
    ));
}
