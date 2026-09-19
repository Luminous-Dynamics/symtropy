use symtropy_evolution_core::{
    append_descendant_ancestry_to_graph, assemble_diploid_linked_offspring_from_evidence,
    derive_descendant_ancestry, derive_descendant_mutation_lineage,
    derive_modeled_gamete_ancestry, derive_zero_crossover_linked_gamete,
    execute_linked_mutations, initialize_root_mutation_lineage, project_declared_linked_census,
    AlleleId, AncestryCopyId, AncestryGeneration, AncestryGraphNode,
    ChromosomeAncestryState, ChromosomeDefinition, ChromosomeHaplotype, ChromosomeId,
    ChromosomeLocus, ChromosomeMap, ChromosomeMapId, ChromosomeRecombinationDomain,
    ChromosomeRecombinationModel, ChromosomeRecombinationProfile,
    ChromosomeRecombinationProfileId, EvolutionOperatorProfile, GeneticMapIntervalMicromorgans,
    GeneticMapPositionMicromorgans, HaplotypeAncestryClass, HereditarySchema,
    HereditarySchemaId, LinkedCensusProjection, LinkedGameteDerivationEvidence,
    LocusDefinition, LocusId, ModeledAncestryGraph, MutationFateSubject,
    MutationLineageState, MutationProfile, OperatorProfileId, ParentRole,
    PhasedAncestryState, PhasedChromosomeState, PhasedHereditaryState, PopulationId,
    RecombinationMode, RecombinationProfile, ReproductionEventId, PROBABILITY_SCALE_PPM,
};

fn allele(id: &str) -> AlleleId { AlleleId::new(id).unwrap() }
fn locus(id: &str) -> LocusId { LocusId::new(id).unwrap() }
fn chromosome() -> ChromosomeId { ChromosomeId::new("chr-projection").unwrap() }
fn ancestry(id: &str) -> AncestryCopyId { AncestryCopyId::new(id).unwrap() }
fn pos(value: u64) -> GeneticMapPositionMicromorgans { GeneticMapPositionMicromorgans::new(value) }

fn schema() -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new("popgen-05a-schema").unwrap(),
        2,
        [
            LocusDefinition::new(locus("a"), [allele("a0"), allele("a1"), allele("a2")]).unwrap(),
            LocusDefinition::new(locus("b"), [allele("b0"), allele("b1")]).unwrap(),
        ],
    ).unwrap()
}

fn map(schema: &HereditarySchema) -> ChromosomeMap {
    ChromosomeMap::new(
        ChromosomeMapId::new("popgen-05a-map").unwrap(),
        schema,
        [ChromosomeDefinition::new(
            chromosome(),
            vec![
                ChromosomeLocus::new(locus("a"), pos(1)),
                ChromosomeLocus::new(locus("b"), pos(8_000_001)),
            ],
        ).unwrap()],
    ).unwrap()
}

fn state(
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
    ).unwrap()
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
            vec![
                HaplotypeAncestryClass::new(0, vec![ancestry(first)]).unwrap(),
                HaplotypeAncestryClass::new(1, vec![ancestry(second)]).unwrap(),
            ],
        ).unwrap()],
    ).unwrap()
}

fn profile(
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
            GeneticMapIntervalMicromorgans::new(pos(0), pos(8_000_002)).unwrap(),
        ).unwrap()],
    ).unwrap()
}

fn operators(rate_ppm: u32) -> EvolutionOperatorProfile {
    EvolutionOperatorProfile {
        profile_id: OperatorProfileId::new("popgen-05a-operators").unwrap(),
        version: "1".into(),
        mutation: MutationProfile {
            model_id: "modeled-locus-substitution".into(),
            version: "1".into(),
            per_copy_rate_ppm: rate_ppm,
        },
        recombination: RecombinationProfile {
            model_id: "linked-chromosome-context".into(),
            version: "1".into(),
            mode: RecombinationMode::IndependentLoci,
        },
    }
}

struct Fixture {
    schema: HereditarySchema,
    map: ChromosomeMap,
    state_a: PhasedHereditaryState,
    state_b: PhasedHereditaryState,
    ancestry_a: PhasedAncestryState,
    ancestry_b: PhasedAncestryState,
    lineage_a: MutationLineageState,
    lineage_b: MutationLineageState,
    profile_a: ChromosomeRecombinationProfile,
    profile_b: ChromosomeRecombinationProfile,
    roots: ModeledAncestryGraph,
}

fn fixture() -> Fixture {
    let schema = schema();
    let map = map(&schema);
    let state_a = state(&schema, &map, &["a0", "b0"], &["a1", "b1"]);
    let state_b = state(&schema, &map, &["a1", "b0"], &["a0", "b1"]);
    let ancestry_a = ancestry_state(&schema, &map, &state_a, "projection-a-0", "projection-a-1");
    let ancestry_b = ancestry_state(&schema, &map, &state_b, "projection-b-0", "projection-b-1");
    let lineage_a = initialize_root_mutation_lineage(&schema, &map, &state_a, &ancestry_a).unwrap();
    let lineage_b = initialize_root_mutation_lineage(&schema, &map, &state_b, &ancestry_b).unwrap();
    let profile_a = profile(&schema, &map, "projection-profile-a");
    let profile_b = profile(&schema, &map, "projection-profile-b");
    let roots = ModeledAncestryGraph::new_roots(
        &schema,
        &map,
        [
            AncestryGraphNode::root(ancestry("projection-a-0"), chromosome(), AncestryGeneration::new(0)),
            AncestryGraphNode::root(ancestry("projection-a-1"), chromosome(), AncestryGeneration::new(0)),
            AncestryGraphNode::root(ancestry("projection-b-0"), chromosome(), AncestryGeneration::new(0)),
            AncestryGraphNode::root(ancestry("projection-b-1"), chromosome(), AncestryGeneration::new(0)),
        ],
    ).unwrap();
    Fixture {
        schema, map, state_a, state_b, ancestry_a, ancestry_b,
        lineage_a, lineage_b, profile_a, profile_b, roots,
    }
}

struct Child {
    state: PhasedHereditaryState,
    ancestry: PhasedAncestryState,
    lineage: MutationLineageState,
}

fn child_for_event(f: &Fixture, event_name: &str) -> Child {
    let event = ReproductionEventId::new(event_name).unwrap();
    let gamete_a = LinkedGameteDerivationEvidence::ZeroCrossover(
        derive_zero_crossover_linked_gamete(
            &f.schema, &f.map, &f.state_a, &f.profile_a, &event, ParentRole::ParentA,
        ).unwrap(),
    );
    let gamete_b = LinkedGameteDerivationEvidence::ZeroCrossover(
        derive_zero_crossover_linked_gamete(
            &f.schema, &f.map, &f.state_b, &f.profile_b, &event, ParentRole::ParentB,
        ).unwrap(),
    );
    let gamete_ancestry_a = derive_modeled_gamete_ancestry(
        &f.schema, &f.map, &f.state_a, &f.ancestry_a, &f.profile_a,
        &gamete_a, &event, ParentRole::ParentA,
    ).unwrap();
    let gamete_ancestry_b = derive_modeled_gamete_ancestry(
        &f.schema, &f.map, &f.state_b, &f.ancestry_b, &f.profile_b,
        &gamete_b, &event, ParentRole::ParentB,
    ).unwrap();
    let offspring = assemble_diploid_linked_offspring_from_evidence(
        &f.schema, &f.map,
        &f.state_a, &f.profile_a, &gamete_a,
        &f.state_b, &f.profile_b, &gamete_b,
        &event,
    ).unwrap();
    let descendant = derive_descendant_ancestry(
        &f.schema, &f.map,
        &f.state_a, &f.ancestry_a, &f.profile_a, &gamete_a, &gamete_ancestry_a,
        &f.state_b, &f.ancestry_b, &f.profile_b, &gamete_b, &gamete_ancestry_b,
        &offspring, &event,
    ).unwrap();
    let graph = append_descendant_ancestry_to_graph(
        &f.roots, &f.schema, &f.map,
        &f.state_a, &f.ancestry_a, &f.profile_a, &gamete_a, &gamete_ancestry_a,
        &f.state_b, &f.ancestry_b, &f.profile_b, &gamete_b, &gamete_ancestry_b,
        &descendant, AncestryGeneration::new(1),
    ).unwrap().graph;
    let authority = operators(PROBABILITY_SCALE_PPM);
    let execution = execute_linked_mutations(
        &f.schema, &f.map, &authority,
        &f.state_a, &f.ancestry_a, &f.profile_a, &gamete_a, &gamete_ancestry_a,
        &f.state_b, &f.ancestry_b, &f.profile_b, &gamete_b, &gamete_ancestry_b,
        &descendant, &graph,
    ).unwrap();
    let lineage = derive_descendant_mutation_lineage(
        &f.schema, &f.map, &authority,
        &f.state_a, &f.ancestry_a, &f.lineage_a, &f.profile_a, &gamete_a, &gamete_ancestry_a,
        &f.state_b, &f.ancestry_b, &f.lineage_b, &f.profile_b, &gamete_b, &gamete_ancestry_b,
        &descendant, &graph, &execution,
    ).unwrap().state;
    Child {
        state: execution.mutated_child,
        ancestry: execution.mutated_child_ancestry,
        lineage,
    }
}

#[test]
fn root_declared_census_projects_exact_aggregate_counts_and_is_order_invariant() {
    let f = fixture();
    let a = MutationFateSubject::new(&f.state_a, &f.ancestry_a, &f.lineage_a, 2).unwrap();
    let b = MutationFateSubject::new(&f.state_b, &f.ancestry_b, &f.lineage_b, 1).unwrap();
    let id = PopulationId::new("projection-population").unwrap();
    let first = project_declared_linked_census(id.clone(), &f.schema, &f.map, &[a, b]).unwrap();
    let second = project_declared_linked_census(id.clone(), &f.schema, &f.map, &[b, a]).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.census_individuals(), 3);
    assert_eq!(first.aggregate_population.census_individuals, 3);

    for observed in &first.mutation_fate.loci {
        let aggregate = &first.aggregate_population.allele_copy_counts[&observed.locus_id];
        assert_eq!(aggregate.len(), observed.allele_counts.len());
        for count in &observed.allele_counts {
            assert_eq!(aggregate.get(&count.allele_id), Some(&count.count));
        }
    }

    let encoded = serde_json::to_vec(&first).unwrap();
    let restored: LinkedCensusProjection = serde_json::from_slice(&encoded).unwrap();
    restored.validate_current(&id, &f.schema, &f.map, &[a, b]).unwrap();
    assert!(restored.validate_current(
        &PopulationId::new("different-population").unwrap(),
        &f.schema, &f.map, &[a, b],
    ).is_err());
}

#[test]
fn recurrent_origins_collapse_only_in_aggregate_allele_counts() {
    let f = fixture();
    let child = (0..128).find_map(|index| {
        let child = child_for_event(&f, &format!("projection-recurrent-{index}"));
        let b_entries: Vec<_> = child.lineage.entries.iter()
            .filter(|entry| entry.locus_id == locus("b"))
            .collect();
        if b_entries.len() == 2
            && b_entries[0].allele == b_entries[1].allele
            && b_entries[0].active_origin.is_some()
            && b_entries[1].active_origin.is_some()
            && b_entries[0].active_origin != b_entries[1].active_origin
        {
            Some(child)
        } else {
            None
        }
    }).expect("frozen 128-event corpus must contain recurrent equal-allele origins");

    let subject = MutationFateSubject::new(&child.state, &child.ancestry, &child.lineage, 1).unwrap();
    let projection = project_declared_linked_census(
        PopulationId::new("recurrent-projection").unwrap(),
        &f.schema, &f.map, &[subject],
    ).unwrap();
    let b = projection.mutation_fate.loci.iter().find(|entry| entry.locus_id == locus("b")).unwrap();
    assert_eq!(b.allele_counts.len(), 1);
    assert_eq!(b.allele_counts[0].count, 2);
    assert_eq!(b.origin_counts.len(), 2);
    assert!(b.origin_counts.iter().all(|count| count.count == 1));
    assert_eq!(projection.aggregate_population.allele_copy_counts[&locus("b")].len(), 1);
    assert_eq!(projection.aggregate_population.allele_copy_counts[&locus("b")]
        [&b.allele_counts[0].allele_id], 2);
}

#[test]
fn duplicate_multiplicity_scales_projection_and_overflow_fails_closed() {
    let f = fixture();
    let one = MutationFateSubject::new(&f.state_a, &f.ancestry_a, &f.lineage_a, 1).unwrap();
    let two = MutationFateSubject::new(&f.state_a, &f.ancestry_a, &f.lineage_a, 2).unwrap();
    let split = project_declared_linked_census(
        PopulationId::new("split-multiplicity").unwrap(),
        &f.schema, &f.map, &[one, two],
    ).unwrap();
    assert_eq!(split.census_individuals(), 3);
    for locus in &split.mutation_fate.loci {
        assert_eq!(locus.total_copy_count, 6);
    }

    let huge = MutationFateSubject {
        phased_state: &f.state_a,
        ancestry_state: &f.ancestry_a,
        lineage_state: &f.lineage_a,
        multiplicity: u64::MAX,
    };
    assert!(project_declared_linked_census(
        PopulationId::new("overflow").unwrap(),
        &f.schema, &f.map, &[huge, one],
    ).is_err());
}
