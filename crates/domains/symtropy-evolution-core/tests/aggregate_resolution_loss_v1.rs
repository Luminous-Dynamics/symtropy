use symtropy_evolution_core::{
    continue_projected_census_alleles_only, initialize_root_mutation_lineage,
    neutral_wright_fisher_step, project_declared_linked_census, AlleleId, AncestryCopyId,
    AggregateResolutionLossProfile, ChromosomeAncestryState, ChromosomeDefinition,
    ChromosomeHaplotype, ChromosomeId, ChromosomeLocus, ChromosomeMap, ChromosomeMapId,
    EvolutionExperimentId, GeneticMapPositionMicromorgans, HaplotypeAncestryClass,
    HereditarySchema, HereditarySchemaId, LocusDefinition, LocusId, MutationFateSubject,
    PopulationGeneration, PopulationId, PopulationProcessModel, PopulationProcessProfile,
    PopulationProcessProfileId, PopulationTrajectoryPoint, PopulationTransitionId,
    PhasedAncestryState, PhasedChromosomeState, PhasedHereditaryState,
};

fn allele(id: &str) -> AlleleId { AlleleId::new(id).unwrap() }
fn locus() -> LocusId { LocusId::new("focal").unwrap() }
fn chromosome() -> ChromosomeId { ChromosomeId::new("chr-loss").unwrap() }
fn ancestry(id: &str) -> AncestryCopyId { AncestryCopyId::new(id).unwrap() }

fn fixture(
    ancestry_prefix: &str,
) -> (
    HereditarySchema,
    ChromosomeMap,
    PhasedHereditaryState,
    PhasedAncestryState,
    symtropy_evolution_core::MutationLineageState,
) {
    let schema = HereditarySchema::new(
        HereditarySchemaId::new("popgen-05b-schema").unwrap(),
        2,
        [LocusDefinition::new(locus(), [allele("a"), allele("b")]).unwrap()],
    ).unwrap();
    let map = ChromosomeMap::new(
        ChromosomeMapId::new("popgen-05b-map").unwrap(),
        &schema,
        [ChromosomeDefinition::new(
            chromosome(),
            vec![ChromosomeLocus::new(
                locus(),
                GeneticMapPositionMicromorgans::new(1),
            )],
        ).unwrap()],
    ).unwrap();
    let phased = PhasedHereditaryState::new(
        &schema,
        &map,
        [PhasedChromosomeState::new(
            chromosome(),
            vec![
                ChromosomeHaplotype::new(vec![allele("a")]),
                ChromosomeHaplotype::new(vec![allele("b")]),
            ],
        )],
    ).unwrap();
    let ancestry_state = PhasedAncestryState::new(
        &schema,
        &map,
        &phased,
        [ChromosomeAncestryState::new(
            chromosome(),
            vec![
                HaplotypeAncestryClass::new(
                    0,
                    vec![ancestry(&format!("{ancestry_prefix}-a"))],
                ).unwrap(),
                HaplotypeAncestryClass::new(
                    1,
                    vec![ancestry(&format!("{ancestry_prefix}-b"))],
                ).unwrap(),
            ],
        ).unwrap()],
    ).unwrap();
    let lineage = initialize_root_mutation_lineage(&schema, &map, &phased, &ancestry_state).unwrap();
    (schema, map, phased, ancestry_state, lineage)
}

fn profile() -> PopulationProcessProfile {
    PopulationProcessProfile {
        profile_id: PopulationProcessProfileId::new("neutral-wf-loss-v1").unwrap(),
        version: "1".into(),
        model: PopulationProcessModel::NeutralIndependentLocusWrightFisher,
    }
}

#[test]
fn wrapped_transition_is_exact_direct_wright_fisher_and_loss_is_explicit() {
    let (schema, map, phased, ancestry_state, lineage) = fixture("history-a");
    let subject = MutationFateSubject::new(&phased, &ancestry_state, &lineage, 4).unwrap();
    let population_id = PopulationId::new("island-loss").unwrap();
    let projection = project_declared_linked_census(
        population_id,
        &schema,
        &map,
        &[subject],
    ).unwrap();
    let point = PopulationTrajectoryPoint::declare_reference_start(
        &schema,
        &projection.aggregate_population,
        EvolutionExperimentId::new("loss-exp").unwrap(),
        PopulationGeneration(0),
    ).unwrap();
    let transition_id = PopulationTransitionId::new("loss-g0-g1").unwrap();
    let process = profile();

    let direct = neutral_wright_fisher_step(
        &schema,
        &projection.aggregate_population,
        &point,
        &transition_id,
        &process,
    ).unwrap();
    let wrapped = continue_projected_census_alleles_only(
        &schema,
        &map,
        &projection,
        &[subject],
        &point,
        &transition_id,
        &process,
    ).unwrap();

    assert_eq!(wrapped.transition, direct);
    assert_eq!(wrapped.transition.destination.population_id, projection.aggregate_population.population_id);
    assert_eq!(wrapped.transition.destination.census_individuals, projection.aggregate_population.census_individuals);

    let loss = wrapped.loss.profile();
    assert_eq!(loss, AggregateResolutionLossProfile::AlleleOnlyNeutralIndependentLocusV1);
    assert!(loss.retains_allele_copy_counts());
    assert!(!loss.retains_individual_genotypes());
    assert!(!loss.retains_haplotype_phase());
    assert!(!loss.retains_ancestry_copy_identity());
    assert!(!loss.retains_mutation_origin_partition());
    assert!(!loss.retains_mutation_history());

    let encoded = serde_json::to_vec(&wrapped).unwrap();
    let restored: symtropy_evolution_core::AlleleOnlyAggregateContinuation =
        serde_json::from_slice(&encoded).unwrap();
    restored.validate_current(
        &schema,
        &map,
        &projection,
        &[subject],
        &point,
        &transition_id,
        &process,
    ).unwrap();
}

#[test]
fn equal_alleles_with_different_microhistory_share_transition_but_not_loss_authority() {
    let (schema_a, map_a, phased_a, ancestry_a, lineage_a) = fixture("history-a");
    let (schema_b, map_b, phased_b, ancestry_b, lineage_b) = fixture("history-b");
    assert_eq!(schema_a.canonical_digest().unwrap(), schema_b.canonical_digest().unwrap());
    assert_eq!(map_a.canonical_digest(&schema_a).unwrap(), map_b.canonical_digest(&schema_b).unwrap());

    let subject_a = MutationFateSubject::new(&phased_a, &ancestry_a, &lineage_a, 4).unwrap();
    let subject_b = MutationFateSubject::new(&phased_b, &ancestry_b, &lineage_b, 4).unwrap();
    let population_id = PopulationId::new("same-alleles").unwrap();
    let projection_a = project_declared_linked_census(
        population_id.clone(), &schema_a, &map_a, &[subject_a],
    ).unwrap();
    let projection_b = project_declared_linked_census(
        population_id, &schema_b, &map_b, &[subject_b],
    ).unwrap();
    assert_eq!(projection_a.aggregate_population, projection_b.aggregate_population);
    assert_ne!(projection_a.mutation_fate_digest(), projection_b.mutation_fate_digest());

    let point = PopulationTrajectoryPoint::declare_reference_start(
        &schema_a,
        &projection_a.aggregate_population,
        EvolutionExperimentId::new("same-allele-exp").unwrap(),
        PopulationGeneration(0),
    ).unwrap();
    let transition_id = PopulationTransitionId::new("same-alleles-g0-g1").unwrap();
    let process = profile();
    let a = continue_projected_census_alleles_only(
        &schema_a, &map_a, &projection_a, &[subject_a], &point, &transition_id, &process,
    ).unwrap();
    let b = continue_projected_census_alleles_only(
        &schema_b, &map_b, &projection_b, &[subject_b], &point, &transition_id, &process,
    ).unwrap();

    assert_eq!(a.transition, b.transition);
    assert_ne!(a.loss.canonical_digest(), b.loss.canonical_digest());
    assert_ne!(a.canonical_digest(), b.canonical_digest());
}
