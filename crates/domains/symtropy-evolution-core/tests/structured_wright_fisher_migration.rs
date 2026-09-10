use std::collections::BTreeMap;

use symtropy_evolution_core::{
    structured_wright_fisher_step, AlleleId, EvolutionExperimentId, HereditarySchema,
    HereditarySchemaId, LocusDefinition, LocusId, MetapopulationSnapshot, ParentalSourceEdge,
    PopulationGeneration, PopulationGeneticState, PopulationId, PopulationProcessModel,
    PopulationProcessProfile, PopulationProcessProfileId, PopulationStructureModel,
    PopulationStructureProfile, PopulationStructureProfileId, PopulationTrajectoryPoint,
    PopulationTransitionId,
};

fn allele(id: &str) -> AlleleId {
    AlleleId::new(id).unwrap()
}

fn pop(id: &str) -> PopulationId {
    PopulationId::new(id).unwrap()
}

fn schema() -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new("structured-ensemble-v0").unwrap(),
        2,
        vec![LocusDefinition::new(
            LocusId::new("focal").unwrap(),
            [allele("a"), allele("b")],
        )
        .unwrap()],
    )
    .unwrap()
}

fn process_profile() -> PopulationProcessProfile {
    PopulationProcessProfile {
        profile_id: PopulationProcessProfileId::new("neutral-wf").unwrap(),
        version: "v1".into(),
        model: PopulationProcessModel::NeutralIndependentLocusWrightFisher,
    }
}

fn fixed_state(
    schema: &HereditarySchema,
    id: PopulationId,
    census: u64,
    fixed: &str,
) -> PopulationGeneticState {
    PopulationGeneticState::from_counts(
        id,
        schema,
        census,
        BTreeMap::from([(
            LocusId::new("focal").unwrap(),
            BTreeMap::from([(allele(fixed), census * u64::from(schema.ploidy))]),
        )]),
    )
    .unwrap()
}

fn mixed_state(
    schema: &HereditarySchema,
    id: PopulationId,
    census: u64,
    a_copies: u64,
) -> PopulationGeneticState {
    let total = census * u64::from(schema.ploidy);
    PopulationGeneticState::from_counts(
        id,
        schema,
        census,
        BTreeMap::from([(
            LocusId::new("focal").unwrap(),
            BTreeMap::from([(allele("a"), a_copies), (allele("b"), total - a_copies)]),
        )]),
    )
    .unwrap()
}

fn structure(
    a: &PopulationId,
    b: &PopulationId,
    a_from_b_ppm: u32,
    b_from_a_ppm: u32,
) -> PopulationStructureProfile {
    let mut edges = Vec::new();
    if a_from_b_ppm != 0 {
        edges.push(ParentalSourceEdge::new(a.clone(), b.clone(), a_from_b_ppm).unwrap());
    }
    if b_from_a_ppm != 0 {
        edges.push(ParentalSourceEdge::new(b.clone(), a.clone(), b_from_a_ppm).unwrap());
    }
    PopulationStructureProfile::new(
        PopulationStructureProfileId::new("two-island-migration").unwrap(),
        "v1",
        PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
        vec![a.clone(), b.clone()],
        edges,
    )
    .unwrap()
}

fn points(
    schema: &HereditarySchema,
    populations: &BTreeMap<PopulationId, PopulationGeneticState>,
    experiment: EvolutionExperimentId,
    generation: PopulationGeneration,
) -> BTreeMap<PopulationId, PopulationTrajectoryPoint> {
    populations
        .iter()
        .map(|(id, population)| {
            (
                id.clone(),
                PopulationTrajectoryPoint::declare_reference_start(
                    schema,
                    population,
                    experiment.clone(),
                    generation,
                )
                .unwrap(),
            )
        })
        .collect()
}

fn one_generation_fixed_pools(
    replicate: u64,
    a_from_b_ppm: u32,
    b_from_a_ppm: u32,
) -> (f64, f64) {
    let schema = schema();
    let a = pop("a");
    let b = pop("b");
    let structure = structure(&a, &b, a_from_b_ppm, b_from_a_ppm);
    let populations = BTreeMap::from([
        (a.clone(), fixed_state(&schema, a.clone(), 20, "a")),
        (b.clone(), fixed_state(&schema, b.clone(), 20, "b")),
    ]);
    let source_points = points(
        &schema,
        &populations,
        EvolutionExperimentId::new(format!("structured-replicate-{replicate:05}")).unwrap(),
        PopulationGeneration(0),
    );
    let snapshot =
        MetapopulationSnapshot::capture_reference(&schema, &structure, &populations, &source_points)
            .unwrap();
    let result = structured_wright_fisher_step(
        &schema,
        &structure,
        &populations,
        &source_points,
        &snapshot,
        &PopulationTransitionId::new("generation-step").unwrap(),
        &process_profile(),
    )
    .unwrap();
    let focal = LocusId::new("focal").unwrap();
    let freq = |id: &PopulationId| {
        f64::from(
            result.destinations[id]
                .allele_frequency_ppm(&schema, &focal, &allele("a"))
                .unwrap(),
        ) / 1_000_000.0
    };
    (freq(&a), freq(&b))
}

#[test]
fn symmetric_migration_matches_one_generation_mixture_expectation() {
    const REPLICATES: u64 = 1_024;
    let values: Vec<(f64, f64)> = (0..REPLICATES)
        .map(|replicate| one_generation_fixed_pools(replicate, 200_000, 200_000))
        .collect();
    let mean_a = values.iter().map(|value| value.0).sum::<f64>() / REPLICATES as f64;
    let mean_b = values.iter().map(|value| value.1).sum::<f64>() / REPLICATES as f64;

    // A starts fixed for allele a, B fixed for b. With symmetric m=0.2,
    // E[p_A'] = 0.8 and E[p_B'] = 0.2. For 40 destination copies per
    // replicate, these tolerances are >5 standard errors for the ensemble.
    assert!((mean_a - 0.8).abs() < 0.01, "observed A mean {mean_a}");
    assert!((mean_b - 0.2).abs() < 0.01, "observed B mean {mean_b}");
}

#[test]
fn asymmetric_migration_matches_declared_destination_rows() {
    const REPLICATES: u64 = 1_024;
    let values: Vec<(f64, f64)> = (0..REPLICATES)
        .map(|replicate| one_generation_fixed_pools(replicate, 100_000, 350_000))
        .collect();
    let mean_a = values.iter().map(|value| value.0).sum::<f64>() / REPLICATES as f64;
    let mean_b = values.iter().map(|value| value.1).sum::<f64>() / REPLICATES as f64;

    assert!((mean_a - 0.9).abs() < 0.01, "observed A mean {mean_a}");
    assert!((mean_b - 0.35).abs() < 0.012, "observed B mean {mean_b}");
}

#[test]
fn migration_rate_sweep_reuses_source_choice_opportunity_field() {
    // With two fixed opposing source pools, increasing A<-B from 10% to 20%
    // should only move the deterministic threshold over the same source-choice
    // variates. It must not reroll the opportunities.
    for replicate in 0..256 {
        let low = one_generation_fixed_pools(replicate, 100_000, 0).0;
        let high = one_generation_fixed_pools(replicate, 200_000, 0).0;
        assert!(
            high <= low,
            "higher import from fixed-b pool increased allele-a frequency: replicate {replicate}, low={low}, high={high}"
        );
    }
}

#[test]
fn structured_checkpoint_restore_is_pathwise_identical_to_continuous_run() {
    let schema = schema();
    let a = pop("a");
    let b = pop("b");
    let structure = structure(&a, &b, 125_000, 75_000);
    let profile = process_profile();
    let transition = PopulationTransitionId::new("generation-step").unwrap();
    let initial = BTreeMap::from([
        (a.clone(), mixed_state(&schema, a.clone(), 12, 18)),
        (b.clone(), mixed_state(&schema, b.clone(), 12, 6)),
    ]);
    let initial_points = points(
        &schema,
        &initial,
        EvolutionExperimentId::new("checkpoint-exp").unwrap(),
        PopulationGeneration(0),
    );

    fn advance(
        schema: &HereditarySchema,
        structure: &PopulationStructureProfile,
        profile: &PopulationProcessProfile,
        transition: &PopulationTransitionId,
        mut populations: BTreeMap<PopulationId, PopulationGeneticState>,
        mut points: BTreeMap<PopulationId, PopulationTrajectoryPoint>,
        steps: usize,
    ) -> (
        BTreeMap<PopulationId, PopulationGeneticState>,
        BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    ) {
        for _ in 0..steps {
            let snapshot = MetapopulationSnapshot::capture_reference(
                schema,
                structure,
                &populations,
                &points,
            )
            .unwrap();
            let result = structured_wright_fisher_step(
                schema,
                structure,
                &populations,
                &points,
                &snapshot,
                transition,
                profile,
            )
            .unwrap();
            populations = result.destinations;
            points = result.destination_points;
        }
        (populations, points)
    }

    let continuous = advance(
        &schema,
        &structure,
        &profile,
        &transition,
        initial.clone(),
        initial_points.clone(),
        12,
    );
    let checkpoint = advance(
        &schema,
        &structure,
        &profile,
        &transition,
        initial,
        initial_points,
        5,
    );
    let encoded = serde_json::to_string(&checkpoint).unwrap();
    let restored: (
        BTreeMap<PopulationId, PopulationGeneticState>,
        BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    ) = serde_json::from_str(&encoded).unwrap();
    let resumed = advance(
        &schema,
        &structure,
        &profile,
        &transition,
        restored.0,
        restored.1,
        7,
    );

    assert_eq!(continuous, resumed);
    assert!(continuous
        .1
        .values()
        .all(|point| point.generation() == PopulationGeneration(12)));
}
