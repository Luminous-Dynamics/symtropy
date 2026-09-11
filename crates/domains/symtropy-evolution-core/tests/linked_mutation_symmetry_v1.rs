use std::collections::BTreeSet;

use symtropy_evolution_core::{
    append_descendant_ancestry_to_graph, assemble_diploid_linked_offspring_from_evidence,
    derive_descendant_ancestry, derive_modeled_gamete_ancestry,
    derive_zero_crossover_linked_gamete, execute_linked_mutations, AlleleId, AncestryCopyId,
    AncestryGeneration, AncestryGraphNode, ChromosomeAncestryState, ChromosomeDefinition,
    ChromosomeHaplotype, ChromosomeId, ChromosomeLocus, ChromosomeMap, ChromosomeMapId,
    ChromosomeRecombinationDomain, ChromosomeRecombinationModel,
    ChromosomeRecombinationProfile, ChromosomeRecombinationProfileId,
    DescendantAncestryDerivation, EvolutionOperatorProfile, GeneticMapIntervalMicromorgans,
    GeneticMapPositionMicromorgans, HaplotypeAncestryClass, HereditarySchema,
    HereditarySchemaId, LinkedGameteDerivationEvidence, LinkedMutationExecution,
    LinkedMutationOutcome, LocusDefinition, LocusId, ModeledAncestryGraph, MutationProfile,
    OperatorProfileId, ParentRole, PhasedAncestryState, PhasedChromosomeState,
    PhasedHereditaryState, RecombinationMode, RecombinationProfile, ReproductionEventId,
    PROBABILITY_SCALE_PPM,
};

fn allele(id: &str) -> AlleleId {
    AlleleId::new(id).unwrap()
}

fn locus(id: &str) -> LocusId {
    LocusId::new(id).unwrap()
}

fn chromosome() -> ChromosomeId {
    ChromosomeId::new("chr-symmetry").unwrap()
}

fn ancestry(id: &str) -> AncestryCopyId {
    AncestryCopyId::new(id).unwrap()
}

fn pos(value: u64) -> GeneticMapPositionMicromorgans {
    GeneticMapPositionMicromorgans::new(value)
}

fn schema() -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new("mut-05b-symmetry-schema").unwrap(),
        2,
        [
            LocusDefinition::new(
                locus("a"),
                (0..17).map(|index| allele(&format!("a{index:02}"))),
            )
            .unwrap(),
            LocusDefinition::new(locus("b"), [allele("b0"), allele("b1")]).unwrap(),
            LocusDefinition::new(locus("c"), [allele("c0")]).unwrap(),
        ],
    )
    .unwrap()
}

fn map(schema: &HereditarySchema) -> ChromosomeMap {
    ChromosomeMap::new(
        ChromosomeMapId::new("mut-05b-symmetry-map").unwrap(),
        schema,
        [ChromosomeDefinition::new(
            chromosome(),
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

fn homozygous_source(schema: &HereditarySchema, map: &ChromosomeMap) -> PhasedHereditaryState {
    let haplotype = ChromosomeHaplotype::new(vec![allele("a00"), allele("b0"), allele("c0")]);
    PhasedHereditaryState::new(
        schema,
        map,
        [PhasedChromosomeState::new(
            chromosome(),
            vec![haplotype.clone(), haplotype],
        )],
    )
    .unwrap()
}

fn ambiguous_ancestry(
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
            GeneticMapIntervalMicromorgans::new(pos(0), pos(14_000_002)).unwrap(),
        )
        .unwrap()],
    )
    .unwrap()
}

fn operators() -> EvolutionOperatorProfile {
    EvolutionOperatorProfile {
        profile_id: OperatorProfileId::new("mut-05b-symmetry-operators").unwrap(),
        version: "1".into(),
        mutation: MutationProfile {
            model_id: "modeled-locus-substitution".into(),
            version: "1".into(),
            per_copy_rate_ppm: PROBABILITY_SCALE_PPM,
        },
        recombination: RecombinationProfile {
            model_id: "linked-chromosome-context".into(),
            version: "1".into(),
            mode: RecombinationMode::IndependentLoci,
        },
    }
}

struct SymmetryFixture {
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
    graph: ModeledAncestryGraph,
}

fn fixture(event_name: &str) -> SymmetryFixture {
    let schema = schema();
    let map = map(&schema);
    let event = ReproductionEventId::new(event_name).unwrap();
    let source_a = homozygous_source(&schema, &map);
    let source_b = homozygous_source(&schema, &map);
    let ancestry_a = ambiguous_ancestry(&schema, &map, &source_a, "a-root-0", "a-root-1");
    let ancestry_b = ambiguous_ancestry(&schema, &map, &source_b, "b-root-0", "b-root-1");
    let profile_a = recombination_profile(&schema, &map, "mut-05b-symmetry-parent-a");
    let profile_b = recombination_profile(&schema, &map, "mut-05b-symmetry-parent-b");

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
    assert_eq!(gamete_a.gamete(), gamete_b.gamete());

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

    let child_ancestry = descendant
        .child_ancestry
        .chromosomes
        .get(&chromosome())
        .unwrap();
    assert_eq!(child_ancestry.classes.len(), 1);
    assert_eq!(child_ancestry.classes[0].copy_ids.len(), 2);

    let roots = ModeledAncestryGraph::new_roots(
        &schema,
        &map,
        [
            AncestryGraphNode::root(ancestry("a-root-0"), chromosome(), AncestryGeneration::new(0)),
            AncestryGraphNode::root(ancestry("a-root-1"), chromosome(), AncestryGeneration::new(0)),
            AncestryGraphNode::root(ancestry("b-root-0"), chromosome(), AncestryGeneration::new(0)),
            AncestryGraphNode::root(ancestry("b-root-1"), chromosome(), AncestryGeneration::new(0)),
        ],
    )
    .unwrap();
    let graph = append_descendant_ancestry_to_graph(
        &roots,
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
        &descendant,
        AncestryGeneration::new(1),
    )
    .unwrap()
    .graph;

    SymmetryFixture {
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
        graph,
    }
}

fn execute(f: &SymmetryFixture) -> LinkedMutationExecution {
    execute_linked_mutations(
        &f.schema,
        &f.map,
        &operators(),
        &f.source_a,
        &f.ancestry_a,
        &f.profile_a,
        &f.gamete_a,
        &f.gamete_ancestry_a,
        &f.source_b,
        &f.ancestry_b,
        &f.profile_b,
        &f.gamete_b,
        &f.gamete_ancestry_b,
        &f.descendant,
        &f.graph,
    )
    .unwrap()
}

#[test]
fn copy_specific_mutation_can_break_equal_haplotype_symmetry_without_inventing_prior_row_parentage() {
    for index in 0..128_u32 {
        let event_name = format!("mut-05b-symmetry-{index:03}");
        let f = fixture(&event_name);
        let execution = execute(&f);
        let chromosome_ancestry = execution
            .mutated_child_ancestry
            .chromosomes
            .get(&chromosome())
            .unwrap();

        if chromosome_ancestry.classes.len() != 2 {
            continue;
        }

        execution
            .mutated_child_ancestry
            .validate_current(&f.schema, &f.map, &execution.mutated_child)
            .unwrap();

        let original_ids: BTreeSet<_> = f
            .descendant
            .materialization
            .descendant_copies
            .iter()
            .map(|copy| copy.child_copy_id.clone())
            .collect();
        let mutated_ids: BTreeSet<_> = chromosome_ancestry
            .classes
            .iter()
            .flat_map(|class| class.copy_ids.iter().cloned())
            .collect();
        assert_eq!(mutated_ids, original_ids);
        assert!(chromosome_ancestry
            .classes
            .iter()
            .all(|class| class.copy_ids.len() == 1));

        let parent_a = execution
            .opportunities
            .iter()
            .find(|opportunity| {
                opportunity.parent_role == ParentRole::ParentA
                    && opportunity.locus_id == locus("a")
            })
            .unwrap();
        let parent_b = execution
            .opportunities
            .iter()
            .find(|opportunity| {
                opportunity.parent_role == ParentRole::ParentB
                    && opportunity.locus_id == locus("a")
            })
            .unwrap();
        assert_ne!(parent_a.ancestry_copy_id, parent_b.ancestry_copy_id);
        assert_ne!(parent_a.canonical_digest(), parent_b.canonical_digest());
        assert!(matches!(
            &parent_a.outcome,
            LinkedMutationOutcome::Substitution { origin, .. }
                if origin.ancestry_copy_id() == &parent_a.ancestry_copy_id
        ));
        assert!(matches!(
            &parent_b.outcome,
            LinkedMutationOutcome::Substitution { origin, .. }
                if origin.ancestry_copy_id() == &parent_b.ancestry_copy_id
        ));

        let next_event = ReproductionEventId::new("mut-05b-symmetry-next-generation").unwrap();
        let next_gamete = LinkedGameteDerivationEvidence::ZeroCrossover(
            derive_zero_crossover_linked_gamete(
                &f.schema,
                &f.map,
                &execution.mutated_child,
                &f.profile_a,
                &next_event,
                ParentRole::ParentA,
            )
            .unwrap(),
        );
        let next_ancestry = derive_modeled_gamete_ancestry(
            &f.schema,
            &f.map,
            &execution.mutated_child,
            &execution.mutated_child_ancestry,
            &f.profile_a,
            &next_gamete,
            &next_event,
            ParentRole::ParentA,
        )
        .unwrap();
        next_ancestry
            .provenance
            .validate_current(
                &f.schema,
                &f.map,
                &execution.mutated_child,
                &execution.mutated_child_ancestry,
                &f.profile_a,
                &next_gamete,
                &next_event,
                ParentRole::ParentA,
                &next_ancestry.ancestry,
            )
            .unwrap();

        execution
            .validate_current(
                &f.schema,
                &f.map,
                &operators(),
                &f.source_a,
                &f.ancestry_a,
                &f.profile_a,
                &f.gamete_a,
                &f.gamete_ancestry_a,
                &f.source_b,
                &f.ancestry_b,
                &f.profile_b,
                &f.gamete_b,
                &f.gamete_ancestry_b,
                &f.descendant,
                &f.graph,
            )
            .unwrap();
        return;
    }

    panic!("no deterministic symmetry-breaking fixture found in frozen event corpus");
}
