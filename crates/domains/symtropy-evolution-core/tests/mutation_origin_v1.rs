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
            LocusDefinition::new(
                locus("a"),
                [allele("a0"), allele("a1"), allele("a2")],
            )
            .unwrap(),
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

fn copy_for_role(fixture: &Fixture, role: ParentRole) -> &AncestryCopyId {
    &fixture
        .descendant
        .materialization
        .descendant_copies
        .iter()
        .find(|copy| copy.parent_role == role)
        .unwrap()
        .child_copy_id
}

fn ancestral_a(fixture: &Fixture, role: ParentRole) -> AlleleId {
    let gamete = match role {
        ParentRole::ParentA => &fixture.gamete_a,
        ParentRole::ParentB => &fixture.gamete_b,
        ParentRole::ClonalParent => panic!("linked fixture has no clonal contribution"),
    };
    gamete.gamete().chromosomes[&chromosome()].alleles[0].clone()
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
    let ancestral = ancestral_a(&first, ParentRole::ParentA);
    let derived = different_a(&ancestral);
    let origin = declare(
        &first,
        &graph,
        &operator_authority,
        copy_for_role(&first, ParentRole::ParentA),
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
fn parent_contribution_and_copy_identity_are_part_of_mutation_identity() {
    let fixture = make_fixture("mut-05a-two-copy-identity");
    let graph = append(&root_graph(&fixture), &fixture, 1);
    let operator_authority = operators(125);
    let shared_derived = allele("a2");

    let origin_a = declare(
        &fixture,
        &graph,
        &operator_authority,
        copy_for_role(&fixture, ParentRole::ParentA),
        &locus("a"),
        &shared_derived,
    )
    .unwrap();
    let origin_b = declare(
        &fixture,
        &graph,
        &operator_authority,
        copy_for_role(&fixture, ParentRole::ParentB),
        &locus("a"),
        &shared_derived,
    )
    .unwrap();

    assert_eq!(origin_a.parent_role(), ParentRole::ParentA);
    assert_eq!(origin_b.parent_role(), ParentRole::ParentB);
    assert_eq!(origin_a.derived_allele(), origin_b.derived_allele());
    assert_ne!(origin_a.ancestry_copy_id(), origin_b.ancestry_copy_id());
    assert_ne!(origin_a.canonical_digest(), origin_b.canonical_digest());
    validate(&origin_a, &fixture, &graph, &operator_authority).unwrap();
    validate(&origin_b, &fixture, &graph, &operator_authority).unwrap();

    // A restored receipt cannot relabel its parent contribution.
    let mut tampered = serde_json::to_value(&origin_a).unwrap();
    tampered["parent_role"] = serde_json::Value::String("ParentB".to_owned());
    let relabeled: MutationOrigin = serde_json::from_value(tampered).unwrap();
    assert!(matches!(
        validate(&relabeled, &fixture, &graph, &operator_authority),
        Err(MutationOriginError::ReplayMismatch)
    ));
}

#[test]
fn restore_rejects_generation_birth_event_and_gamete_authority_drift() {
    let fixture = make_fixture("mut-05a-replay-drift");
    let graph = append(&root_graph(&fixture), &fixture, 1);
    let operator_authority = operators(125);
    let ancestral = ancestral_a(&fixture, ParentRole::ParentA);
    let origin = declare(
        &fixture,
        &graph,
        &operator_authority,
        copy_for_role(&fixture, ParentRole::ParentA),
        &locus("a"),
        &different_a(&ancestral),
    )
    .unwrap();

    let mut generation_drift = graph.clone();
    generation_drift
        .nodes
        .get_mut(origin.ancestry_copy_id())
        .unwrap()
        .generation = AncestryGeneration::new(2);
    assert!(matches!(
        validate(&origin, &fixture, &generation_drift, &operator_authority),
        Err(MutationOriginError::ReplayMismatch)
    ));

    let mut event_drift = graph.clone();
    event_drift
        .nodes
        .get_mut(origin.ancestry_copy_id())
        .unwrap()
        .birth_event = Some(ReproductionEventId::new("mut-05a-wrong-birth").unwrap());
    assert!(matches!(
        validate(&origin, &fixture, &event_drift, &operator_authority),
        Err(MutationOriginError::BirthEventMismatch)
    ));

    // Substituting ParentB's derivation evidence where ParentA's exact linked
    // gamete is required must fail before an old origin can be re-earned.
    assert!(origin
        .validate_current(
            &fixture.schema,
            &fixture.map,
            &operator_authority,
            &fixture.source_a,
            &fixture.ancestry_a,
            &fixture.profile_a,
            &fixture.gamete_b,
            &fixture.gamete_ancestry_a,
            &fixture.source_b,
            &fixture.ancestry_b,
            &fixture.profile_b,
            &fixture.gamete_b,
            &fixture.gamete_ancestry_b,
            &fixture.descendant,
            &graph,
        )
        .is_err());
}

#[test]
fn mutation_origin_fails_closed_for_invalid_local_claims() {
    let fixture = make_fixture("mut-05a-negative-birth");
    let graph = append(&root_graph(&fixture), &fixture, 1);
    let operator_authority = operators(125);
    let ancestral = ancestral_a(&fixture, ParentRole::ParentA);

    assert!(matches!(
        declare(
            &fixture,
            &graph,
            &operator_authority,
            copy_for_role(&fixture, ParentRole::ParentA),
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
            copy_for_role(&fixture, ParentRole::ParentA),
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
            copy_for_role(&fixture, ParentRole::ParentA),
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
