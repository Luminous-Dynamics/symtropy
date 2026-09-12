use std::collections::{BTreeMap, BTreeSet};

use symtropy_evolution_core::{
    AggregateActiveOriginCount, AggregateAlleleProvenance, AggregateLocusProvenance, AlleleId,
    EvolutionExperimentId, HereditarySchema, HereditarySchemaId, LocusDefinition, LocusId,
    MutationOriginDigest, OriginAwarePopulationState, PopulationGeneration, PopulationGeneticState,
    PopulationId, PopulationProcessModel, PopulationProcessProfile, PopulationProcessProfileId,
    PopulationTrajectoryPoint, PopulationTransitionId, ORIGIN_AWARE_POPULATION_STATE_VERSION,
    neutral_origin_aware_population_step,
};

const REPLICATES: usize = 16_384;
const TV_TOLERANCE: f64 = 0.05;
const EVENT_TOLERANCE: f64 = 0.04;
const REFERENCE_ANALYTICAL_TOLERANCE: f64 = 0.025;

fn allele() -> AlleleId {
    AlleleId::new("derived").unwrap()
}

fn spare_allele() -> AlleleId {
    AlleleId::new("spare").unwrap()
}

fn locus() -> LocusId {
    LocusId::new("focal").unwrap()
}

fn schema(id: &str, ploidy: u8) -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new(id).unwrap(),
        ploidy,
        [LocusDefinition::new(locus(), [allele(), spare_allele()]).unwrap()],
    )
    .unwrap()
}

fn process_profile() -> PopulationProcessProfile {
    PopulationProcessProfile {
        profile_id: PopulationProcessProfileId::new("popgen-05e2-reference-sampler").unwrap(),
        version: "1".into(),
        model: PopulationProcessModel::NeutralIndependentLocusWrightFisher,
    }
}

fn synthetic_origin(byte: u8) -> MutationOriginDigest {
    serde_json::from_value(serde_json::to_value([byte; 32]).unwrap()).unwrap()
}

fn source_state(
    schema: &HereditarySchema,
    population_name: &str,
    baseline: u64,
    origins: &[(u8, u64)],
) -> OriginAwarePopulationState {
    let total = u64::from(schema.ploidy);
    assert_eq!(baseline + origins.iter().map(|(_, count)| *count).sum::<u64>(), total);

    let population = PopulationGeneticState::from_counts(
        PopulationId::new(population_name).unwrap(),
        schema,
        1,
        BTreeMap::from([(locus(), BTreeMap::from([(allele(), total)]))]),
    )
    .unwrap();

    let mut active_origin_counts: Vec<_> = origins
        .iter()
        .map(|(byte, count)| AggregateActiveOriginCount {
            origin_digest: synthetic_origin(*byte),
            count: *count,
        })
        .collect();
    active_origin_counts.sort_by(|left, right| {
        left.origin_digest
            .as_bytes()
            .cmp(right.origin_digest.as_bytes())
    });

    let loci = vec![AggregateLocusProvenance {
        locus_id: locus(),
        alleles: vec![AggregateAlleleProvenance {
            allele_id: allele(),
            modeled_baseline_count: baseline,
            active_origin_counts,
        }],
    }];

    let restored: OriginAwarePopulationState = serde_json::from_value(serde_json::json!({
        "state_version": ORIGIN_AWARE_POPULATION_STATE_VERSION,
        "schema_digest": serde_json::to_value(schema.canonical_digest().unwrap()).unwrap(),
        "population_digest": serde_json::to_value(population.canonical_digest(schema).unwrap()).unwrap(),
        "population": serde_json::to_value(&population).unwrap(),
        "loci": serde_json::to_value(&loci).unwrap(),
    }))
    .unwrap();
    restored.validate_current(schema).unwrap();
    restored
}

fn start_point(
    schema: &HereditarySchema,
    state: &OriginAwarePopulationState,
    experiment: &str,
) -> PopulationTrajectoryPoint {
    PopulationTrajectoryPoint::declare_reference_start(
        schema,
        &state.population,
        EvolutionExperimentId::new(experiment).unwrap(),
        PopulationGeneration(0),
    )
    .unwrap()
}

fn production_counts(
    schema: &HereditarySchema,
    source: &OriginAwarePopulationState,
    point: &PopulationTrajectoryPoint,
    origins: &[MutationOriginDigest],
    replicate: usize,
    case: &str,
) -> Vec<u64> {
    let result = neutral_origin_aware_population_step(
        schema,
        source,
        point,
        &PopulationTransitionId::new(format!("{case}-{replicate}")).unwrap(),
        &process_profile(),
    )
    .unwrap();
    let provenance = result
        .destination
        .allele_provenance(&locus(), &allele())
        .unwrap();

    let mut counts = Vec::with_capacity(origins.len() + 1);
    counts.push(provenance.modeled_baseline_count);
    for origin in origins {
        counts.push(
            provenance
                .active_origin_counts
                .iter()
                .find(|entry| &entry.origin_digest == origin)
                .map(|entry| entry.count)
                .unwrap_or(0),
        );
    }
    counts
}

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e3779b97f4a7c15);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
    value ^ (value >> 31)
}

fn reference_draw_below(state: &mut u64, upper: u64) -> u64 {
    assert!(upper > 0);
    if upper == 1 {
        return 0;
    }
    let zone = u64::MAX - (u64::MAX % upper);
    loop {
        let value = splitmix64(state);
        if value < zone {
            return value % upper;
        }
    }
}

fn reference_sample(weights: &[u64], destination_count: u64, seed: u64) -> Vec<u64> {
    let total: u64 = weights.iter().sum();
    assert!(total > 0);
    let mut state = seed;
    let mut counts = vec![0_u64; weights.len()];

    for _ in 0..destination_count {
        let ordinal = reference_draw_below(&mut state, total);
        let mut upper = 0_u64;
        for (index, weight) in weights.iter().enumerate() {
            upper += *weight;
            if ordinal < upper {
                counts[index] += 1;
                break;
            }
        }
    }
    counts
}

fn histogram(samples: impl IntoIterator<Item = Vec<u64>>) -> BTreeMap<Vec<u64>, u64> {
    let mut histogram = BTreeMap::new();
    for sample in samples {
        *histogram.entry(sample).or_insert(0) += 1;
    }
    histogram
}

fn total_variation(
    left: &BTreeMap<Vec<u64>, u64>,
    right: &BTreeMap<Vec<u64>, u64>,
    sample_count: usize,
) -> f64 {
    let mut keys = BTreeSet::new();
    keys.extend(left.keys().cloned());
    keys.extend(right.keys().cloned());
    let n = sample_count as f64;
    0.5 * keys
        .iter()
        .map(|key| {
            let left = left.get(key).copied().unwrap_or(0) as f64 / n;
            let right = right.get(key).copied().unwrap_or(0) as f64 / n;
            (left - right).abs()
        })
        .sum::<f64>()
}

fn event_probability(
    histogram: &BTreeMap<Vec<u64>, u64>,
    sample_count: usize,
    predicate: impl Fn(&[u64]) -> bool,
) -> f64 {
    let matches: u64 = histogram
        .iter()
        .filter(|(outcome, _)| predicate(outcome))
        .map(|(_, count)| *count)
        .sum();
    matches as f64 / sample_count as f64
}

fn compare_case(
    schema: &HereditarySchema,
    source: &OriginAwarePopulationState,
    weights: &[u64],
    origin_bytes: &[u8],
    destination_count: u64,
    case_tag: u64,
    case_name: &str,
) -> (BTreeMap<Vec<u64>, u64>, BTreeMap<Vec<u64>, u64>) {
    let origins: Vec<_> = origin_bytes.iter().map(|byte| synthetic_origin(*byte)).collect();
    let point = start_point(schema, source, &format!("{case_name}-experiment"));

    let production = histogram((0..REPLICATES).map(|replicate| {
        let counts = production_counts(schema, source, &point, &origins, replicate, case_name);
        assert_eq!(counts.iter().sum::<u64>(), destination_count);
        counts
    }));
    let reference = histogram((0..REPLICATES).map(|replicate| {
        reference_sample(
            weights,
            destination_count,
            0xd1b54a32d192ed03_u64 ^ case_tag ^ replicate as u64,
        )
    }));

    let distance = total_variation(&production, &reference, REPLICATES);
    assert!(
        distance <= TV_TOLERANCE,
        "{case_name}: total variation {distance} exceeded frozen tolerance {TV_TOLERANCE}"
    );
    (production, reference)
}

#[test]
fn independent_reference_sampler_matches_fifty_fifty_two_origin_distribution() {
    let schema = schema("popgen-05e2-binary", 2);
    let source = source_state(&schema, "e2-binary-pop", 0, &[(71, 1), (72, 1)]);
    let (production, reference) = compare_case(
        &schema,
        &source,
        &[0, 1, 1],
        &[71, 72],
        2,
        0x01,
        "e2-binary",
    );

    for (outcome, expected) in [
        (vec![0, 2, 0], 0.25),
        (vec![0, 1, 1], 0.50),
        (vec![0, 0, 2], 0.25),
    ] {
        let observed = reference.get(&outcome).copied().unwrap_or(0) as f64 / REPLICATES as f64;
        assert!((observed - expected).abs() <= REFERENCE_ANALYTICAL_TOLERANCE);
    }

    let production_extinction = event_probability(&production, REPLICATES, |outcome| outcome[1] == 0);
    let reference_extinction = event_probability(&reference, REPLICATES, |outcome| outcome[1] == 0);
    let production_fixation = event_probability(&production, REPLICATES, |outcome| outcome[1] == 2);
    let reference_fixation = event_probability(&reference, REPLICATES, |outcome| outcome[1] == 2);
    assert!((production_extinction - reference_extinction).abs() <= EVENT_TOLERANCE);
    assert!((production_fixation - reference_fixation).abs() <= EVENT_TOLERANCE);
}

#[test]
fn independent_reference_sampler_matches_asymmetric_two_origin_distribution() {
    let schema = schema("popgen-05e2-asymmetric", 3);
    let source = source_state(&schema, "e2-asymmetric-pop", 0, &[(81, 1), (82, 2)]);
    let (production, reference) = compare_case(
        &schema,
        &source,
        &[0, 1, 2],
        &[81, 82],
        3,
        0x02,
        "e2-asymmetric",
    );

    let production_extinction = event_probability(&production, REPLICATES, |outcome| outcome[1] == 0);
    let reference_extinction = event_probability(&reference, REPLICATES, |outcome| outcome[1] == 0);
    let production_fixation = event_probability(&production, REPLICATES, |outcome| outcome[1] == 3);
    let reference_fixation = event_probability(&reference, REPLICATES, |outcome| outcome[1] == 3);
    assert!((production_extinction - reference_extinction).abs() <= EVENT_TOLERANCE);
    assert!((production_fixation - reference_fixation).abs() <= EVENT_TOLERANCE);
}

#[test]
fn independent_reference_sampler_matches_baseline_plus_origin_distribution() {
    let schema = schema("popgen-05e2-baseline-origin", 2);
    let source = source_state(&schema, "e2-baseline-origin-pop", 1, &[(91, 1)]);
    let (production, reference) = compare_case(
        &schema,
        &source,
        &[1, 1],
        &[91],
        2,
        0x03,
        "e2-baseline-origin",
    );

    let production_origin_loss = event_probability(&production, REPLICATES, |outcome| outcome[1] == 0);
    let reference_origin_loss = event_probability(&reference, REPLICATES, |outcome| outcome[1] == 0);
    assert!((production_origin_loss - reference_origin_loss).abs() <= EVENT_TOLERANCE);
}

#[test]
fn independent_reference_sampler_matches_three_origin_multinomial_distribution() {
    let schema = schema("popgen-05e2-three-origin", 3);
    let source = source_state(
        &schema,
        "e2-three-origin-pop",
        0,
        &[(101, 1), (102, 1), (103, 1)],
    );
    let (production, reference) = compare_case(
        &schema,
        &source,
        &[0, 1, 1, 1],
        &[101, 102, 103],
        3,
        0x04,
        "e2-three-origin",
    );

    for class_index in 1..=3 {
        let production_extinction =
            event_probability(&production, REPLICATES, |outcome| outcome[class_index] == 0);
        let reference_extinction =
            event_probability(&reference, REPLICATES, |outcome| outcome[class_index] == 0);
        assert!((production_extinction - reference_extinction).abs() <= EVENT_TOLERANCE);
    }
}
