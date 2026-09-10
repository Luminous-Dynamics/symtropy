use std::collections::BTreeMap;

use symtropy_evolution_core::{
    neutral_wright_fisher_step, AlleleId, EvolutionExperimentId, HereditarySchema,
    HereditarySchemaId, LocusDefinition, LocusId, PopulationGeneration,
    PopulationGeneticState, PopulationId, PopulationProcessModel,
    PopulationProcessProfile, PopulationProcessProfileId, PopulationTransitionId,
};

fn allele(id: &str) -> AlleleId {
    AlleleId::new(id).unwrap()
}

fn reference_schema() -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new("neutral-ensemble-v0").unwrap(),
        2,
        vec![LocusDefinition::new(
            LocusId::new("focal").unwrap(),
            [allele("a"), allele("b")],
        )
        .unwrap()],
    )
    .unwrap()
}

fn reference_population(schema: &HereditarySchema) -> PopulationGeneticState {
    PopulationGeneticState::from_counts(
        PopulationId::new("reference-population").unwrap(),
        schema,
        50,
        BTreeMap::from([(
            LocusId::new("focal").unwrap(),
            BTreeMap::from([(allele("a"), 50), (allele("b"), 50)]),
        )]),
    )
    .unwrap()
}

fn reference_profile() -> PopulationProcessProfile {
    PopulationProcessProfile {
        profile_id: PopulationProcessProfileId::new("neutral-independent-locus-wf").unwrap(),
        version: "v1".into(),
        model: PopulationProcessModel::NeutralIndependentLocusWrightFisher,
    }
}

fn one_generation_frequency(replicate: u64) -> f64 {
    let schema = reference_schema();
    let source = reference_population(&schema);
    let result = neutral_wright_fisher_step(
        &schema,
        &source,
        &EvolutionExperimentId::new(format!("neutral-wf-replicate-{replicate:05}")).unwrap(),
        &PopulationTransitionId::new("generation-step").unwrap(),
        PopulationGeneration(0),
        &reference_profile(),
    )
    .unwrap();

    f64::from(
        result
            .destination
            .allele_frequency_ppm(
                &schema,
                &LocusId::new("focal").unwrap(),
                &allele("a"),
            )
            .unwrap(),
    ) / 1_000_000.0
}

#[test]
fn neutral_one_generation_mean_and_variance_match_wright_fisher_expectation() {
    // Diploid N=50 => 2N=100 independently sampled allele copies.
    // With p=0.5, E[p'] = p and Var[p'] = p(1-p)/(2N) = 0.0025.
    const REPLICATES: u64 = 4_096;
    const EXPECTED_MEAN: f64 = 0.5;
    const EXPECTED_VARIANCE: f64 = 0.0025;

    let values: Vec<f64> = (0..REPLICATES).map(one_generation_frequency).collect();
    let mean = values.iter().sum::<f64>() / REPLICATES as f64;
    let variance = values
        .iter()
        .map(|value| {
            let delta = value - mean;
            delta * delta
        })
        .sum::<f64>()
        / REPLICATES as f64;

    // These are qualification tolerances for a deterministic seed ensemble,
    // not exact identities. They are intentionally much wider than the
    // expected Monte Carlo standard errors for 4096 replicates.
    assert!(
        (mean - EXPECTED_MEAN).abs() < 0.005,
        "neutral mean drifted: observed {mean}, expected {EXPECTED_MEAN}"
    );
    assert!(
        (variance - EXPECTED_VARIANCE).abs() < 0.00025,
        "neutral variance drifted: observed {variance}, expected {EXPECTED_VARIANCE}"
    );
}

#[test]
fn ensemble_result_is_invariant_to_replicate_execution_order() {
    let ascending: BTreeMap<u64, f64> = (0..128)
        .map(|replicate| (replicate, one_generation_frequency(replicate)))
        .collect();
    let descending: BTreeMap<u64, f64> = (0..128)
        .rev()
        .map(|replicate| (replicate, one_generation_frequency(replicate)))
        .collect();

    assert_eq!(ascending, descending);
}
