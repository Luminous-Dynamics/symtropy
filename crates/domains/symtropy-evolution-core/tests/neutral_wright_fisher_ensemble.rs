use std::collections::BTreeMap;

use symtropy_evolution_core::{
    neutral_wright_fisher_step, AlleleId, EvolutionExperimentId, HereditarySchema,
    HereditarySchemaId, LocusDefinition, LocusId, PopulationGeneration,
    PopulationGeneticState, PopulationId, PopulationProcessModel,
    PopulationProcessProfile, PopulationProcessProfileId, PopulationTrajectoryPoint,
    PopulationTransitionId,
};

fn allele(id: &str) -> AlleleId {
    AlleleId::new(id).unwrap()
}

fn focal_locus() -> LocusId {
    LocusId::new("focal").unwrap()
}

fn reference_schema() -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new("neutral-ensemble-v0").unwrap(),
        2,
        vec![LocusDefinition::new(focal_locus(), [allele("a"), allele("b")]).unwrap()],
    )
    .unwrap()
}

fn population_with_a_copies(
    schema: &HereditarySchema,
    census_individuals: u64,
    a_copies: u64,
) -> PopulationGeneticState {
    let total_copies = census_individuals.checked_mul(2).unwrap();
    assert!(a_copies <= total_copies);

    let mut alleles = BTreeMap::new();
    if a_copies != 0 {
        alleles.insert(allele("a"), a_copies);
    }
    if a_copies != total_copies {
        alleles.insert(allele("b"), total_copies - a_copies);
    }

    PopulationGeneticState::from_counts(
        PopulationId::new("reference-population").unwrap(),
        schema,
        census_individuals,
        BTreeMap::from([(focal_locus(), alleles)]),
    )
    .unwrap()
}

fn reference_population(schema: &HereditarySchema) -> PopulationGeneticState {
    population_with_a_copies(schema, 50, 50)
}

fn reference_profile() -> PopulationProcessProfile {
    PopulationProcessProfile {
        profile_id: PopulationProcessProfileId::new("neutral-independent-locus-wf").unwrap(),
        version: "v1".into(),
        model: PopulationProcessModel::NeutralIndependentLocusWrightFisher,
    }
}

fn a_copy_count(population: &PopulationGeneticState) -> u64 {
    population
        .allele_copy_counts
        .get(&focal_locus())
        .and_then(|counts| counts.get(&allele("a")))
        .copied()
        .unwrap_or(0)
}

fn a_frequency(population: &PopulationGeneticState, census_individuals: u64) -> f64 {
    a_copy_count(population) as f64 / (2 * census_individuals) as f64
}

fn advance_generations(
    schema: &HereditarySchema,
    initial: PopulationGeneticState,
    experiment: EvolutionExperimentId,
    generations: u64,
) -> (PopulationGeneticState, PopulationTrajectoryPoint) {
    let profile = reference_profile();
    let mut population = initial;
    let mut point = PopulationTrajectoryPoint::declare_reference_start(
        schema,
        &population,
        experiment,
        PopulationGeneration(0),
    )
    .unwrap();

    for _ in 0..generations {
        let generation = point.generation().0;
        let transition = PopulationTransitionId::new(format!(
            "generation-{generation:05}-to-{:05}",
            generation + 1
        ))
        .unwrap();
        let result = neutral_wright_fisher_step(
            schema,
            &population,
            &point,
            &transition,
            &profile,
        )
        .unwrap();
        population = result.destination;
        point = result.destination_point;
    }

    (population, point)
}

fn one_generation_frequency(replicate: u64) -> f64 {
    let schema = reference_schema();
    let source = reference_population(&schema);
    let experiment =
        EvolutionExperimentId::new(format!("neutral-wf-replicate-{replicate:05}")).unwrap();
    let point = PopulationTrajectoryPoint::declare_reference_start(
        &schema,
        &source,
        experiment,
        PopulationGeneration(0),
    )
    .unwrap();
    let result = neutral_wright_fisher_step(
        &schema,
        &source,
        &point,
        &PopulationTransitionId::new("generation-step").unwrap(),
        &reference_profile(),
    )
    .unwrap();

    a_frequency(&result.destination, 50)
}

fn longrun_frequency(replicate: u64, generations: u64) -> f64 {
    const CENSUS: u64 = 20;
    let schema = reference_schema();
    let source = population_with_a_copies(&schema, CENSUS, CENSUS);
    let experiment =
        EvolutionExperimentId::new(format!("neutral-longrun-replicate-{replicate:05}")).unwrap();
    let (population, _) = advance_generations(&schema, source, experiment, generations);
    a_frequency(&population, CENSUS)
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
fn neutral_multigeneration_martingale_and_heterozygosity_decay_match_expectation() {
    // Diploid N=20 => 2N=40 allele copies. For neutral Wright-Fisher drift:
    //
    // E[p_t] = p_0
    // E[H_t] = H_0 * (1 - 1/(2N))^t, H_t = 2 p_t (1-p_t).
    const REPLICATES: u64 = 2_048;
    const CENSUS: u64 = 20;
    const GENERATIONS: u64 = 20;
    const P0: f64 = 0.5;
    const H0: f64 = 0.5;

    let values: Vec<f64> = (0..REPLICATES)
        .map(|replicate| longrun_frequency(replicate, GENERATIONS))
        .collect();
    let mean_p = values.iter().sum::<f64>() / REPLICATES as f64;
    let mean_h = values
        .iter()
        .map(|p| 2.0 * p * (1.0 - p))
        .sum::<f64>()
        / REPLICATES as f64;

    let retention = 1.0 - 1.0 / (2 * CENSUS) as f64;
    let expected_h = H0 * retention.powi(GENERATIONS as i32);

    // From E[H_t], the exact Wright-Fisher ensemble variance of p_t is
    // p0(1-p0) - E[H_t]/2. Use four analytical standard errors for the mean
    // rather than a hand-tuned absolute threshold.
    let expected_var_p = P0 * (1.0 - P0) - expected_h / 2.0;
    let mean_standard_error = (expected_var_p / REPLICATES as f64).sqrt();
    assert!(
        (mean_p - P0).abs() < 4.0 * mean_standard_error,
        "neutral martingale drifted: observed E[p_t]={mean_p}, expected {P0}, four-SE bound {}",
        4.0 * mean_standard_error
    );

    // The H statistic has a non-Gaussian finite-population distribution by
    // this horizon. This fixed acceptance band is deliberately several times
    // wider than the expected Monte Carlo error for this declared corpus while
    // still rejecting materially incorrect heterozygosity decay.
    assert!(
        (mean_h - expected_h).abs() < 0.015,
        "heterozygosity decay drifted: observed E[H_t]={mean_h}, expected {expected_h}"
    );
}

#[test]
fn neutral_fixation_probability_matches_initial_allele_frequency() {
    // Small N makes exact absorption cheap enough for a deterministic reference
    // ensemble. Under neutral Wright-Fisher drift, ultimate fixation
    // probability equals the initial allele frequency.
    const REPLICATES: u64 = 4_096;
    const CENSUS: u64 = 4;
    const TOTAL_COPIES: u64 = 2 * CENSUS;
    const INITIAL_A_COPIES: u64 = 2;
    const MAX_GENERATIONS: u64 = 512;

    let schema = reference_schema();
    let profile = reference_profile();
    let expected_fixation = INITIAL_A_COPIES as f64 / TOTAL_COPIES as f64;
    let mut a_fixations = 0_u64;

    for replicate in 0..REPLICATES {
        let mut population = population_with_a_copies(&schema, CENSUS, INITIAL_A_COPIES);
        let experiment =
            EvolutionExperimentId::new(format!("neutral-fixation-replicate-{replicate:05}"))
                .unwrap();
        let mut point = PopulationTrajectoryPoint::declare_reference_start(
            &schema,
            &population,
            experiment,
            PopulationGeneration(0),
        )
        .unwrap();

        while point.generation().0 < MAX_GENERATIONS {
            let a_copies = a_copy_count(&population);
            if a_copies == 0 || a_copies == TOTAL_COPIES {
                break;
            }

            let generation = point.generation().0;
            let transition = PopulationTransitionId::new(format!(
                "generation-{generation:05}-to-{:05}",
                generation + 1
            ))
            .unwrap();
            let result = neutral_wright_fisher_step(
                &schema,
                &population,
                &point,
                &transition,
                &profile,
            )
            .unwrap();
            population = result.destination;
            point = result.destination_point;
        }

        let final_a_copies = a_copy_count(&population);
        assert!(
            final_a_copies == 0 || final_a_copies == TOTAL_COPIES,
            "replicate {replicate} did not absorb by generation {MAX_GENERATIONS}"
        );
        if final_a_copies == TOTAL_COPIES {
            a_fixations += 1;
        }
    }

    let observed_fixation = a_fixations as f64 / REPLICATES as f64;
    let standard_error =
        (expected_fixation * (1.0 - expected_fixation) / REPLICATES as f64).sqrt();
    assert!(
        (observed_fixation - expected_fixation).abs() < 4.0 * standard_error,
        "neutral fixation probability drifted: observed {observed_fixation}, expected {expected_fixation}, four-SE bound {}",
        4.0 * standard_error
    );
}

#[test]
fn fixed_population_remains_absorbed_while_trajectory_time_advances() {
    const CENSUS: u64 = 8;
    const GENERATIONS: u64 = 32;

    let schema = reference_schema();
    let source = population_with_a_copies(&schema, CENSUS, 2 * CENSUS);
    let (destination, point) = advance_generations(
        &schema,
        source.clone(),
        EvolutionExperimentId::new("absorbing-boundary-reference").unwrap(),
        GENERATIONS,
    );

    assert_eq!(destination, source);
    assert_eq!(point.generation(), PopulationGeneration(GENERATIONS));
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

#[test]
fn longrun_ensemble_is_invariant_to_replicate_execution_order() {
    const GENERATIONS: u64 = 20;
    let ascending: BTreeMap<u64, f64> = (0..64)
        .map(|replicate| (replicate, longrun_frequency(replicate, GENERATIONS)))
        .collect();
    let descending: BTreeMap<u64, f64> = (0..64)
        .rev()
        .map(|replicate| (replicate, longrun_frequency(replicate, GENERATIONS)))
        .collect();

    assert_eq!(ascending, descending);
}
