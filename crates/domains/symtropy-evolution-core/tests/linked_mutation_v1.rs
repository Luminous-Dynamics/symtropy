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
    NoMutationReason, OperatorProfileId, ParentRole, PhasedAncestryState,
    PhasedChromosomeState, PhasedHereditaryState, RecombinationMode, RecombinationProfile,
    ReproductionEventId, PROBABILITY_SCALE_PPM,
};

fn allele(id: &str) -> AlleleId { AlleleId::new(id).unwrap() }
fn locus(id: &str) -> LocusId { LocusId::new(id).unwrap() }
fn chromosome() -> ChromosomeId { ChromosomeId::new("chr-a").unwrap() }
fn ancestry(id: &str) -> AncestryCopyId { AncestryCopyId::new(id).unwrap() }
fn pos(value: u64) -> GeneticMapPositionMicromorgans { GeneticMapPositionMicromorgans::new(value) }

fn schema() -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new("mut-05b-schema").unwrap(),
        2,
        [
            LocusDefinition::new(locus("a"), [allele("a0"), allele("a1"), allele("a2")]).unwrap(),
            LocusDefinition::new(locus("b"), [allele("b0"), allele("b1")]).unwrap(),
            LocusDefinition::new(locus("c"), [allele("c0")]).unwrap(),
        ],
    ).unwrap()
}

fn map(schema: &HereditarySchema) -> ChromosomeMap {
    ChromosomeMap::new(
        ChromosomeMapId::new("mut-05b-map").unwrap(),
        schema,
        [ChromosomeDefinition::new(
            chromosome(),
            vec![
                ChromosomeLocus::new(locus("a"), pos(1)),
                ChromosomeLocus::new(locus("b"), pos(7_000_001)),
                ChromosomeLocus::new(locus("c"), pos(14_000_001)),
            ],
        ).unwrap()],
    ).unwrap()
}

fn source(schema: &HereditarySchema, map: &ChromosomeMap, first: &[&str], second: &[&str]) -> PhasedHereditaryState {
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

fn recombination_profile(schema: &HereditarySchema, map: &ChromosomeMap, id: &str) -> ChromosomeRecombinationProfile {
    ChromosomeRecombinationProfile::new(
        ChromosomeRecombinationProfileId::new(id).unwrap(),
        schema,
        map,
        ChromosomeRecombinationModel::NoCrossoversIndependentAssortmentV1,
        [ChromosomeRecombinationDomain::new(
            chromosome(),
            GeneticMapIntervalMicromorgans::new(pos(0), pos(14_000_002)).unwrap(),
        ).unwrap()],
    ).unwrap()
}

fn operators(rate_ppm: u32) -> EvolutionOperatorProfile {
    EvolutionOperatorProfile {
        profile_id: OperatorProfileId::new("mut-05b-operators").unwrap(),
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

fn fixture(event_name: &str) -> Fixture {
    let schema = schema();
    let map = map(&schema);
    let event = ReproductionEventId::new(event_name).unwrap();
    let source_a = source(&schema, &map, &["a0", "b0", "c0"], &["a1", "b1", "c0"]);
    let source_b = source(&schema, &map, &["a1", "b0", "c0"], &["a0", "b1", "c0"]);
    let ancestry_a = ancestry_state(&schema, &map, &source_a, "a-root-0", "a-root-1");
    let ancestry_b = ancestry_state(&schema, &map, &source_b, "b-root-0", "b-root-1");
    let profile_a = recombination_profile(&schema, &map, "mut-05b-parent-a");
    let profile_b = recombination_profile(&schema, &map, "mut-05b-parent-b");
    let gamete_a = LinkedGameteDerivationEvidence::ZeroCrossover(
        derive_zero_crossover_linked_gamete(&schema, &map, &source_a, &profile_a, &event, ParentRole::ParentA).unwrap(),
    );
    let gamete_b = LinkedGameteDerivationEvidence::ZeroCrossover(
        derive_zero_crossover_linked_gamete(&schema, &map, &source_b, &profile_b, &event, ParentRole::ParentB).unwrap(),
    );
    let gamete_ancestry_a = derive_modeled_gamete_ancestry(
        &schema, &map, &source_a, &ancestry_a, &profile_a, &gamete_a, &event, ParentRole::ParentA,
    ).unwrap();
    let gamete_ancestry_b = derive_modeled_gamete_ancestry(
        &schema, &map, &source_b, &ancestry_b, &profile_b, &gamete_b, &event, ParentRole::ParentB,
    ).unwrap();
    let offspring = assemble_diploid_linked_offspring_from_evidence(
        &schema, &map, &source_a, &profile_a, &gamete_a, &source_b, &profile_b, &gamete_b, &event,
    ).unwrap();
    let descendant = derive_descendant_ancestry(
        &schema, &map,
        &source_a, &ancestry_a, &profile_a, &gamete_a, &gamete_ancestry_a,
        &source_b, &ancestry_b, &profile_b, &gamete_b, &gamete_ancestry_b,
        &offspring, &event,
    ).unwrap();
    let roots = ModeledAncestryGraph::new_roots(
        &schema,
        &map,
        [
            AncestryGraphNode::root(ancestry("a-root-0"), chromosome(), AncestryGeneration::new(0)),
            AncestryGraphNode::root(ancestry("a-root-1"), chromosome(), AncestryGeneration::new(0)),
            AncestryGraphNode::root(ancestry("b-root-0"), chromosome(), AncestryGeneration::new(0)),
            AncestryGraphNode::root(ancestry("b-root-1"), chromosome(), AncestryGeneration::new(0)),
        ],
    ).unwrap();
    let graph = append_descendant_ancestry_to_graph(
        &roots, &schema, &map,
        &source_a, &ancestry_a, &profile_a, &gamete_a, &gamete_ancestry_a,
        &source_b, &ancestry_b, &profile_b, &gamete_b, &gamete_ancestry_b,
        &descendant, AncestryGeneration::new(1),
    ).unwrap().graph;

    Fixture {
        schema, map, source_a, source_b, ancestry_a, ancestry_b, profile_a, profile_b,
        gamete_a, gamete_b, gamete_ancestry_a, gamete_ancestry_b, descendant, graph,
    }
}

fn execute(f: &Fixture, operators: &EvolutionOperatorProfile) -> LinkedMutationExecution {
    execute_linked_mutations(
        &f.schema, &f.map, operators,
        &f.source_a, &f.ancestry_a, &f.profile_a, &f.gamete_a, &f.gamete_ancestry_a,
        &f.source_b, &f.ancestry_b, &f.profile_b, &f.gamete_b, &f.gamete_ancestry_b,
        &f.descendant, &f.graph,
    ).unwrap()
}

fn validate(
    execution: &LinkedMutationExecution,
    f: &Fixture,
    operators: &EvolutionOperatorProfile,
) -> Result<(), symtropy_evolution_core::LinkedMutationError> {
    execution.validate_current(
        &f.schema, &f.map, operators,
        &f.source_a, &f.ancestry_a, &f.profile_a, &f.gamete_a, &f.gamete_ancestry_a,
        &f.source_b, &f.ancestry_b, &f.profile_b, &f.gamete_b, &f.gamete_ancestry_b,
        &f.descendant, &f.graph,
    )
}

#[test]
fn zero_rate_records_complete_rate_miss_census_and_preserves_child() {
    let f = fixture("mut-05b-zero-rate");
    let authority = operators(0);
    let execution = execute(&f, &authority);
    assert_eq!(execution.opportunities.len(), 6);
    assert!(execution.opportunities.iter().all(|opportunity| matches!(
        &opportunity.outcome,
        LinkedMutationOutcome::NoMutation { reason: NoMutationReason::RateMiss }
    )));

    let unmutated = assemble_diploid_linked_offspring_from_evidence(
        &f.schema, &f.map, &f.source_a, &f.profile_a, &f.gamete_a,
        &f.source_b, &f.profile_b, &f.gamete_b,
        &f.descendant.materialization.reproduction_event_id,
    ).unwrap();
    assert_eq!(&execution.mutated_child, &unmutated.child);
    assert_eq!(&execution.mutated_child_ancestry, &f.descendant.child_ancestry);
    assert_eq!(execution.mutated_child_digest(), execution.unmutated_child_digest());
    execution.mutated_child_ancestry.validate_current(
        &f.schema, &f.map, &execution.mutated_child,
    ).unwrap();
    validate(&execution, &f, &authority).unwrap();
}

#[test]
fn maximum_rate_substitutes_polymorphic_loci_and_preserves_copy_identity() {
    let f = fixture("mut-05b-max-rate");
    let authority = operators(PROBABILITY_SCALE_PPM);
    let execution = execute(&f, &authority);
    assert_eq!(execution.opportunities.len(), 6);

    let substitutions: Vec<_> = execution.opportunities.iter().filter_map(|opportunity| {
        match &opportunity.outcome {
            LinkedMutationOutcome::Substitution { origin, .. } => Some((opportunity, origin)),
            _ => None,
        }
    }).collect();
    assert_eq!(substitutions.len(), 4);
    assert_eq!(execution.opportunities.iter().filter(|opportunity| matches!(
        &opportunity.outcome,
        LinkedMutationOutcome::NoMutation { reason: NoMutationReason::NoAlternativeAllele }
    )).count(), 2);

    for (opportunity, origin) in substitutions {
        assert_eq!(origin.ancestry_copy_id(), &opportunity.ancestry_copy_id);
        assert_eq!(origin.chromosome_id(), &opportunity.chromosome_id);
        assert_eq!(origin.locus_id(), &opportunity.locus_id);
        assert_eq!(origin.parent_role(), opportunity.parent_role);
        assert_eq!(origin.ancestral_allele(), &opportunity.ancestral_allele);
        assert_ne!(origin.derived_allele(), origin.ancestral_allele());
        assert!(f.schema.loci[&opportunity.locus_id].allowed_alleles.contains(origin.derived_allele()));
    }
    assert_ne!(execution.mutated_child_digest(), execution.unmutated_child_digest());
    execution.mutated_child_ancestry.validate_current(
        &f.schema, &f.map, &execution.mutated_child,
    ).unwrap();

    let expected_ids: BTreeSet<_> = f.descendant.materialization.descendant_copies
        .iter().map(|copy| copy.child_copy_id.clone()).collect();
    let mutated_ids: BTreeSet<_> = execution.mutated_child_ancestry.chromosomes.values()
        .flat_map(|chromosome| chromosome.classes.iter())
        .flat_map(|class| class.copy_ids.iter().cloned())
        .collect();
    assert_eq!(mutated_ids, expected_ids);
    validate(&execution, &f, &authority).unwrap();
}

#[test]
fn persistent_copy_identity_not_canonical_row_position_addresses_mutation() {
    let f = fixture("mut-05b-copy-identity");
    let execution = execute(&f, &operators(PROBABILITY_SCALE_PPM));
    let parent_a = execution.opportunities.iter().find(|opportunity| {
        opportunity.parent_role == ParentRole::ParentA && opportunity.locus_id == locus("a")
    }).unwrap();
    let parent_b = execution.opportunities.iter().find(|opportunity| {
        opportunity.parent_role == ParentRole::ParentB && opportunity.locus_id == locus("a")
    }).unwrap();

    assert_ne!(&parent_a.ancestry_copy_id, &parent_b.ancestry_copy_id);
    assert_ne!(parent_a.canonical_digest(), parent_b.canonical_digest());
    assert_eq!(parent_a.parent_role, ParentRole::ParentA);
    assert_eq!(parent_b.parent_role, ParentRole::ParentB);
}

#[test]
fn restored_execution_fails_closed_on_operator_opportunity_or_ancestry_drift() {
    let f = fixture("mut-05b-replay");
    let authority = operators(450_000);
    let execution = execute(&f, &authority);
    let digest = execution.canonical_digest();
    let encoded = serde_json::to_vec(&execution).unwrap();
    let restored: LinkedMutationExecution = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(restored, execution);
    assert_eq!(restored.canonical_digest(), digest);
    validate(&restored, &f, &authority).unwrap();

    assert!(validate(&restored, &f, &operators(450_001)).is_err());

    let mut opportunity_tamper = restored.clone();
    opportunity_tamper.opportunities[0].occurrence_draw_ppm =
        opportunity_tamper.opportunities[0].occurrence_draw_ppm.wrapping_add(1);
    assert!(validate(&opportunity_tamper, &f, &authority).is_err());

    let mut ancestry_tamper = restored;
    ancestry_tamper.mutated_child_ancestry.chromosomes
        .get_mut(&chromosome()).unwrap().classes[0].copy_ids.clear();
    assert!(validate(&ancestry_tamper, &f, &authority).is_err());
}
