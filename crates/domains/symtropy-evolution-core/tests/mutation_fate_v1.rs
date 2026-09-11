use symtropy_evolution_core::{
    append_descendant_ancestry_to_graph, assemble_diploid_linked_offspring_from_evidence,
    derive_descendant_ancestry, derive_descendant_mutation_lineage,
    derive_modeled_gamete_ancestry, derive_zero_crossover_linked_gamete,
    execute_linked_mutations, initialize_root_mutation_lineage, observe_mutation_fates,
    AlleleId, AncestryCopyId, AncestryGeneration, AncestryGraphNode,
    ChromosomeAncestryState, ChromosomeDefinition, ChromosomeHaplotype, ChromosomeId,
    ChromosomeLocus, ChromosomeMap, ChromosomeMapId, ChromosomeRecombinationDomain,
    ChromosomeRecombinationModel, ChromosomeRecombinationProfile,
    ChromosomeRecombinationProfileId, EvolutionOperatorProfile, GeneticMapIntervalMicromorgans,
    GeneticMapPositionMicromorgans, HaplotypeAncestryClass, HereditarySchema,
    HereditarySchemaId, LinkedGameteDerivationEvidence, LinkedMutationOutcome,
    LocusDefinition, LocusId, ModeledAncestryGraph, MutationFateObservation,
    MutationFateSubject, MutationLineageState, MutationProfile, OperatorProfileId, ParentRole,
    PhasedAncestryState, PhasedChromosomeState, PhasedHereditaryState, RecombinationMode,
    RecombinationProfile, ReproductionEventId, PROBABILITY_SCALE_PPM,
};

fn allele(id: &str) -> AlleleId { AlleleId::new(id).unwrap() }
fn locus(id: &str) -> LocusId { LocusId::new(id).unwrap() }
fn chromosome() -> ChromosomeId { ChromosomeId::new("chr-fate").unwrap() }
fn ancestry(id: &str) -> AncestryCopyId { AncestryCopyId::new(id).unwrap() }
fn pos(value: u64) -> GeneticMapPositionMicromorgans { GeneticMapPositionMicromorgans::new(value) }

fn schema() -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new("mut-05d-schema").unwrap(),
        2,
        [
            LocusDefinition::new(locus("a"), [allele("a0"), allele("a1"), allele("a2")]).unwrap(),
            LocusDefinition::new(locus("b"), [allele("b0"), allele("b1")]).unwrap(),
        ],
    ).unwrap()
}

fn map(schema: &HereditarySchema) -> ChromosomeMap {
    ChromosomeMap::new(
        ChromosomeMapId::new("mut-05d-map").unwrap(),
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
        profile_id: OperatorProfileId::new("mut-05d-operators").unwrap(),
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

struct RootFixture {
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

fn fixture() -> RootFixture {
    let schema = schema();
    let map = map(&schema);
    let state_a = state(&schema, &map, &["a0", "b0"], &["a1", "b1"]);
    let state_b = state(&schema, &map, &["a1", "b0"], &["a0", "b1"]);
    let ancestry_a = ancestry_state(&schema, &map, &state_a, "fate-a-0", "fate-a-1");
    let ancestry_b = ancestry_state(&schema, &map, &state_b, "fate-b-0", "fate-b-1");
    let lineage_a = initialize_root_mutation_lineage(&schema, &map, &state_a, &ancestry_a).unwrap();
    let lineage_b = initialize_root_mutation_lineage(&schema, &map, &state_b, &ancestry_b).unwrap();
    let profile_a = profile(&schema, &map, "mut-05d-profile-a");
    let profile_b = profile(&schema, &map, "mut-05d-profile-b");
    let roots = ModeledAncestryGraph::new_roots(
        &schema,
        &map,
        [
            AncestryGraphNode::root(ancestry("fate-a-0"), chromosome(), AncestryGeneration::new(0)),
            AncestryGraphNode::root(ancestry("fate-a-1"), chromosome(), AncestryGeneration::new(0)),
            AncestryGraphNode::root(ancestry("fate-b-0"), chromosome(), AncestryGeneration::new(0)),
            AncestryGraphNode::root(ancestry("fate-b-1"), chromosome(), AncestryGeneration::new(0)),
        ],
    ).unwrap();
    RootFixture {
        schema, map, state_a, state_b, ancestry_a, ancestry_b,
        lineage_a, lineage_b, profile_a, profile_b, roots,
    }
}

struct Child {
    state: PhasedHereditaryState,
    ancestry: PhasedAncestryState,
    lineage: MutationLineageState,
}

fn child_for_event(f: &RootFixture, event_name: &str) -> Child {
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

fn locus_observation<'a>(
    observation: &'a MutationFateObservation,
    id: &str,
) -> &'a symtropy_evolution_core::MutationFateLocusObservation {
    observation.loci.iter().find(|entry| entry.locus_id == locus(id)).unwrap()
}

#[test]
fn root_counts_multiplicity_and_input_order_are_exact() {
    let f = fixture();
    let a = MutationFateSubject::new(&f.state_a, &f.ancestry_a, &f.lineage_a, 3).unwrap();
    let b = MutationFateSubject::new(&f.state_b, &f.ancestry_b, &f.lineage_b, 2).unwrap();
    let first = observe_mutation_fates(&f.schema, &f.map, &[a, b]).unwrap();
    let second = observe_mutation_fates(&f.schema, &f.map, &[b, a]).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.canonical_digest().unwrap(), second.canonical_digest().unwrap());

    for observed in &first.loci {
        assert_eq!(observed.total_copy_count, 10);
        assert_eq!(observed.modeled_baseline_count, 10);
        assert!(observed.origin_counts.is_empty());
        assert!(observed.allele_origin_counts.is_empty());
    }

    let encoded = serde_json::to_vec(&first).unwrap();
    let restored: MutationFateObservation = serde_json::from_slice(&encoded).unwrap();
    restored.validate_current(&f.schema, &f.map, &[a, b]).unwrap();
    assert!(restored.validate_current(&f.schema, &f.map, &[a]).is_err());
    assert!(MutationFateSubject::new(&f.state_a, &f.ancestry_a, &f.lineage_a, 0).is_err());
}

#[test]
fn recurrent_origins_share_one_allele_bucket_but_remain_distinct_origins() {
    let f = fixture();
    let child = (0..128).find_map(|index| {
        let child = child_for_event(&f, &format!("mut-05d-recurrent-{index}"));
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
    }).expect("frozen 128-event corpus must contain convergent binary mutation origins");

    let subject = MutationFateSubject::new(&child.state, &child.ancestry, &child.lineage, 1).unwrap();
    let observed = observe_mutation_fates(&f.schema, &f.map, &[subject]).unwrap();
    let b = locus_observation(&observed, "b");
    assert_eq!(b.total_copy_count, 2);
    assert_eq!(b.modeled_baseline_count, 0);
    assert_eq!(b.allele_counts.len(), 1);
    assert_eq!(b.allele_counts[0].count, 2);
    assert_eq!(b.origin_counts.len(), 2);
    assert!(b.origin_counts.iter().all(|count| count.count == 1));
    assert_eq!(b.allele_origin_counts.len(), 2);
    assert!(b.allele_origin_counts.iter().all(|count| count.count == 1));
}

#[test]
fn duplicate_subjects_collapse_into_one_source_digest_with_scaled_counts() {
    let f = fixture();
    let child = child_for_event(&f, "mut-05d-duplicate-subject");
    let one = MutationFateSubject::new(&child.state, &child.ancestry, &child.lineage, 1).unwrap();
    let two = MutationFateSubject::new(&child.state, &child.ancestry, &child.lineage, 2).unwrap();
    let split = observe_mutation_fates(&f.schema, &f.map, &[one, two]).unwrap();
    let direct = observe_mutation_fates(
        &f.schema,
        &f.map,
        &[MutationFateSubject::new(&child.state, &child.ancestry, &child.lineage, 3).unwrap()],
    ).unwrap();
    assert_eq!(split, direct);
    assert_eq!(split.sources.len(), 1);
    assert_eq!(split.sources[0].multiplicity, 3);
    for locus in &split.loci {
        assert_eq!(locus.total_copy_count, 6);
    }
}
