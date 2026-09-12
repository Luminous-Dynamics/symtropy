use std::collections::BTreeMap;

use symtropy_evolution_core::{
    derive_origin_fate_delta, neutral_origin_aware_population_step, neutral_wright_fisher_step,
    AggregateActiveOriginCount, AggregateAlleleProvenance, AggregateLocusProvenance, AlleleId,
    EvolutionExperimentId, HereditarySchema, HereditarySchemaId, LocusDefinition, LocusId,
    MutationOriginDigest, OriginAwarePopulationState, PopulationGeneration, PopulationGeneticState,
    PopulationId, PopulationProcessModel, PopulationProcessProfile, PopulationProcessProfileId,
    PopulationTrajectoryPoint, PopulationTransitionId, ORIGIN_AWARE_POPULATION_STATE_VERSION,
};

const BINARY_REPLICATES: usize = 8_192;
const ASYMMETRIC_REPLICATES: usize = 12_288;
const MULTINOMIAL_REPLICATES: usize = 8_192;
const FREQUENCY_TOLERANCE: f64 = 0.03;
const MOMENT_TOLERANCE: f64 = 0.08;
const CROSS_LOCUS_COVARIANCE_TOLERANCE: f64 = 0.08;

fn allele() -> AlleleId {
    AlleleId::new("derived").unwrap()
}

fn spare_allele() -> AlleleId {
    AlleleId::new("spare").unwrap()
}

fn locus(id: &str) -> LocusId {
    LocusId::new(id).unwrap()
}

fn schema(id: &str, ploidy: u8, locus_ids: &[&str]) -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new(id).unwrap(),
        ploidy,
        locus_ids.iter().map(|id| {
            LocusDefinition::new(locus(id), [allele(), spare_allele()]).unwrap()
        }),
    )
    .unwrap()
}

fn profile() -> PopulationProcessProfile {
    PopulationProcessProfile {
        profile_id: PopulationProcessProfileId::new("popgen-05e1-analytical-null").unwrap(),
        version: "1".into(),
        model: PopulationProcessModel::NeutralIndependentLocusWrightFisher,
    }
}

fn synthetic_origin(byte: u8) -> MutationOriginDigest {
    let wire = serde_json::to_value([byte; 32]).unwrap();
    serde_json::from_value(wire).unwrap()
}

fn state_from_partitions(
    schema: &HereditarySchema,
    population_name: &str,
    partitions: &[(&str, u64, &[(u8, u64)])],
) -> OriginAwarePopulationState {
    assert_eq!(partitions.len(), schema.loci.len());
    let expected_per_locus = u64::from(schema.ploidy);

    let mut population_counts = BTreeMap::new();
    let mut loci = Vec::new();
    for (locus_id, baseline, origins) in partitions {
        let origin_total: u64 = origins.iter().map(|(_, count)| *count).sum();
        assert_eq!(*baseline + origin_total, expected_per_locus);
        population_counts.insert(
            locus(locus_id),
            BTreeMap::from([(allele(), expected_per_locus)]),
        );

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
        loci.push(AggregateLocusProvenance {
            locus_id: locus(locus_id),
            alleles: vec![AggregateAlleleProvenance {
                allele_id: allele(),
                modeled_baseline_count: *baseline,
                active_origin_counts,
            }],
        });
    }
    loci.sort_by(|left, right| left.locus_id.cmp(&right.locus_id));

    let population = PopulationGeneticState::from_counts(
        PopulationId::new(population_name).unwrap(),
        schema,
        1,
        population_counts,
    )
    .unwrap();

    // Qualification-only restored-state construction. This deliberately uses the
    // public wire format rather than adding a production constructor for arbitrary
    // mutation-origin partitions.
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

fn start_point(schema: &HereditarySchema, state: &OriginAwarePopulationState, name: &str) -> PopulationTrajectoryPoint {
    PopulationTrajectoryPoint::declare_reference_start(
        schema,
        &state.population,
        EvolutionExperimentId::new(name).unwrap(),
        PopulationGeneration(0),
    )
    .unwrap()
}

fn origin_count(state: &OriginAwarePopulationState, locus_id: &str, origin: MutationOriginDigest) -> u64 {
    state
        .allele_provenance(&locus(locus_id), &allele())
        .unwrap()
        .active_origin_counts
        .iter()
        .find(|entry| entry.origin_digest == origin)
        .map(|entry| entry.count)
        .unwrap_or(0)
}

fn baseline_count(state: &OriginAwarePopulationState, locus_id: &str) -> u64 {
    state
        .allele_provenance(&locus(locus_id), &allele())
        .unwrap()
        .modeled_baseline_count
}

fn execute(
    schema: &HereditarySchema,
    source: &OriginAwarePopulationState,
    point: &PopulationTrajectoryPoint,
    transition_name: &str,
) -> symtropy_evolution_core::OriginAwarePopulationTransitionResult {
    let transition_id = PopulationTransitionId::new(transition_name).unwrap();
    let process = profile();
    let direct = neutral_wright_fisher_step(
        schema,
        &source.population,
        point,
        &transition_id,
        &process,
    )
    .unwrap();
    let origin_aware = neutral_origin_aware_population_step(
        schema,
        source,
        point,
        &transition_id,
        &process,
    )
    .unwrap();

    // The provenance partition is subordinate to the already-established allele
    // marginal. It must never perturb the ordinary Wright-Fisher destination.
    assert_eq!(origin_aware.ordinary_transition, direct);
    assert_eq!(origin_aware.destination.population, direct.destination);
    origin_aware
}

fn mean_and_variance(values: &[u64]) -> (f64, f64) {
    let n = values.len() as f64;
    let mean = values.iter().map(|value| *value as f64).sum::<f64>() / n;
    let second = values
        .iter()
        .map(|value| {
            let value = *value as f64;
            value * value
        })
        .sum::<f64>()
        / n;
    (mean, second - mean * mean)
}

fn covariance(left: &[u64], right: &[u64]) -> f64 {
    assert_eq!(left.len(), right.len());
    let n = left.len() as f64;
    let left_mean = left.iter().map(|value| *value as f64).sum::<f64>() / n;
    let right_mean = right.iter().map(|value| *value as f64).sum::<f64>() / n;
    left.iter()
        .zip(right)
        .map(|(left, right)| (*left as f64 - left_mean) * (*right as f64 - right_mean))
        .sum::<f64>()
        / n
}

fn assert_close(observed: f64, expected: f64, tolerance: f64, label: &str) {
    assert!(
        (observed - expected).abs() <= tolerance,
        "{label}: observed={observed}, expected={expected}, tolerance={tolerance}"
    );
}

#[test]
fn degenerate_baseline_and_single_origin_classes_are_absorbing_conditionally() {
    let schema = schema("popgen-05e1-degenerate", 2, &["focal"]);
    let baseline = state_from_partitions(&schema, "degenerate-baseline", &[("focal", 2, &[])]);
    let one_origin = state_from_partitions(
        &schema,
        "degenerate-origin",
        &[("focal", 0, &[(11, 2)])],
    );
    let baseline_point = start_point(&schema, &baseline, "degenerate-baseline-experiment");
    let origin_point = start_point(&schema, &one_origin, "degenerate-origin-experiment");
    let origin = synthetic_origin(11);

    for replicate in 0..128 {
        let baseline_result = execute(
            &schema,
            &baseline,
            &baseline_point,
            &format!("degenerate-baseline-{replicate}"),
        );
        assert_eq!(baseline_count(&baseline_result.destination, "focal"), 2);
        assert!(baseline_result
            .destination
            .allele_provenance(&locus("focal"), &allele())
            .unwrap()
            .active_origin_counts
            .is_empty());

        let origin_result = execute(
            &schema,
            &one_origin,
            &origin_point,
            &format!("degenerate-origin-{replicate}"),
        );
        assert_eq!(baseline_count(&origin_result.destination, "focal"), 0);
        assert_eq!(origin_count(&origin_result.destination, "focal", origin), 2);
    }
}

#[test]
fn fifty_fifty_two_origin_case_matches_binomial_null_and_fate_predicates() {
    let schema = schema("popgen-05e1-binary", 2, &["focal"]);
    let first = synthetic_origin(21);
    let second = synthetic_origin(22);
    let source = state_from_partitions(
        &schema,
        "binary-pop",
        &[("focal", 0, &[(21, 1), (22, 1)])],
    );
    let point = start_point(&schema, &source, "binary-experiment");

    let mut histogram = [0_u64; 3];
    let mut first_counts = Vec::with_capacity(BINARY_REPLICATES);
    let mut second_counts = Vec::with_capacity(BINARY_REPLICATES);

    for replicate in 0..BINARY_REPLICATES {
        let result = execute(
            &schema,
            &source,
            &point,
            &format!("binary-null-{replicate}"),
        );
        let first_count = origin_count(&result.destination, "focal", first);
        let second_count = origin_count(&result.destination, "focal", second);
        assert_eq!(first_count + second_count, 2);
        histogram[first_count as usize] += 1;
        first_counts.push(first_count);
        second_counts.push(second_count);

        let fate = derive_origin_fate_delta(&schema, &source, &result.destination).unwrap();
        for (origin, destination_count) in [(first, first_count), (second, second_count)] {
            let entry = fate
                .origins
                .iter()
                .find(|entry| entry.origin_digest == origin)
                .unwrap();
            assert_eq!(entry.source_origin_count, 1);
            assert_eq!(entry.destination_origin_count, destination_count);
            assert_eq!(entry.source_allele_count, 2);
            assert_eq!(entry.destination_allele_count, 2);
            assert_eq!(entry.destination_locus_total, 2);
            assert_eq!(entry.lost(), destination_count == 0);
            assert_eq!(entry.persisting(), destination_count > 0);
            assert_eq!(entry.fixed_within_allele(), destination_count == 2);
            assert_eq!(entry.fixed_at_locus(), destination_count == 2);
        }
    }

    let n = BINARY_REPLICATES as f64;
    assert_close(histogram[0] as f64 / n, 0.25, FREQUENCY_TOLERANCE, "P(N=0)");
    assert_close(histogram[1] as f64 / n, 0.50, FREQUENCY_TOLERANCE, "P(N=1)");
    assert_close(histogram[2] as f64 / n, 0.25, FREQUENCY_TOLERANCE, "P(N=2)");

    let (mean, variance) = mean_and_variance(&first_counts);
    assert_close(mean, 1.0, MOMENT_TOLERANCE, "binary mean");
    assert_close(variance, 0.5, MOMENT_TOLERANCE, "binary variance");
    assert_close(
        covariance(&first_counts, &second_counts),
        -0.5,
        MOMENT_TOLERANCE,
        "binary covariance",
    );
}

#[test]
fn asymmetric_one_third_two_thirds_case_matches_binomial_null() {
    let schema = schema("popgen-05e1-asymmetric", 3, &["focal"]);
    let first = synthetic_origin(31);
    let second = synthetic_origin(32);
    let source = state_from_partitions(
        &schema,
        "asymmetric-pop",
        &[("focal", 0, &[(31, 1), (32, 2)])],
    );
    let point = start_point(&schema, &source, "asymmetric-experiment");
    let mut histogram = [0_u64; 4];
    let mut first_counts = Vec::with_capacity(ASYMMETRIC_REPLICATES);
    let mut second_counts = Vec::with_capacity(ASYMMETRIC_REPLICATES);

    for replicate in 0..ASYMMETRIC_REPLICATES {
        let result = execute(
            &schema,
            &source,
            &point,
            &format!("asymmetric-null-{replicate}"),
        );
        let first_count = origin_count(&result.destination, "focal", first);
        let second_count = origin_count(&result.destination, "focal", second);
        assert_eq!(first_count + second_count, 3);
        histogram[first_count as usize] += 1;
        first_counts.push(first_count);
        second_counts.push(second_count);
    }

    let n = ASYMMETRIC_REPLICATES as f64;
    for (observed, expected, label) in [
        (histogram[0] as f64 / n, 8.0 / 27.0, "P(N=0)"),
        (histogram[1] as f64 / n, 12.0 / 27.0, "P(N=1)"),
        (histogram[2] as f64 / n, 6.0 / 27.0, "P(N=2)"),
        (histogram[3] as f64 / n, 1.0 / 27.0, "P(N=3)"),
    ] {
        assert_close(observed, expected, FREQUENCY_TOLERANCE, label);
    }

    let (mean, variance) = mean_and_variance(&first_counts);
    assert_close(mean, 1.0, MOMENT_TOLERANCE, "asymmetric mean");
    assert_close(variance, 2.0 / 3.0, MOMENT_TOLERANCE, "asymmetric variance");
    assert_close(
        covariance(&first_counts, &second_counts),
        -2.0 / 3.0,
        MOMENT_TOLERANCE,
        "asymmetric covariance",
    );
}

#[test]
fn baseline_plus_origin_follows_the_same_conditional_binomial_law() {
    let schema = schema("popgen-05e1-baseline-origin", 2, &["focal"]);
    let origin = synthetic_origin(41);
    let source = state_from_partitions(
        &schema,
        "baseline-origin-pop",
        &[("focal", 1, &[(41, 1)])],
    );
    let point = start_point(&schema, &source, "baseline-origin-experiment");
    let mut origin_counts = Vec::with_capacity(BINARY_REPLICATES);
    let mut baseline_counts = Vec::with_capacity(BINARY_REPLICATES);

    for replicate in 0..BINARY_REPLICATES {
        let result = execute(
            &schema,
            &source,
            &point,
            &format!("baseline-origin-null-{replicate}"),
        );
        let observed_origin = origin_count(&result.destination, "focal", origin);
        let observed_baseline = baseline_count(&result.destination, "focal");
        assert_eq!(observed_origin + observed_baseline, 2);
        origin_counts.push(observed_origin);
        baseline_counts.push(observed_baseline);
    }

    let (mean, variance) = mean_and_variance(&origin_counts);
    assert_close(mean, 1.0, MOMENT_TOLERANCE, "baseline+origin mean");
    assert_close(variance, 0.5, MOMENT_TOLERANCE, "baseline+origin variance");
    assert_close(
        covariance(&origin_counts, &baseline_counts),
        -0.5,
        MOMENT_TOLERANCE,
        "baseline+origin covariance",
    );
}

#[test]
fn three_equal_origins_match_multinomial_moments_and_category_symmetry() {
    let schema = schema("popgen-05e1-multinomial", 3, &["focal"]);
    let origins = [synthetic_origin(51), synthetic_origin(52), synthetic_origin(53)];
    let source = state_from_partitions(
        &schema,
        "multinomial-pop",
        &[("focal", 0, &[(51, 1), (52, 1), (53, 1)])],
    );
    let point = start_point(&schema, &source, "multinomial-experiment");
    let mut counts = [Vec::with_capacity(MULTINOMIAL_REPLICATES), Vec::with_capacity(MULTINOMIAL_REPLICATES), Vec::with_capacity(MULTINOMIAL_REPLICATES)];

    for replicate in 0..MULTINOMIAL_REPLICATES {
        let result = execute(
            &schema,
            &source,
            &point,
            &format!("multinomial-null-{replicate}"),
        );
        let observed = [
            origin_count(&result.destination, "focal", origins[0]),
            origin_count(&result.destination, "focal", origins[1]),
            origin_count(&result.destination, "focal", origins[2]),
        ];
        assert_eq!(observed.iter().sum::<u64>(), 3);
        for index in 0..3 {
            counts[index].push(observed[index]);
        }
    }

    let mut means = [0.0; 3];
    for index in 0..3 {
        let (mean, variance) = mean_and_variance(&counts[index]);
        means[index] = mean;
        assert_close(mean, 1.0, MOMENT_TOLERANCE, "multinomial category mean");
        assert_close(
            variance,
            2.0 / 3.0,
            MOMENT_TOLERANCE,
            "multinomial category variance",
        );
    }
    assert_close(
        covariance(&counts[0], &counts[1]),
        -1.0 / 3.0,
        MOMENT_TOLERANCE,
        "multinomial covariance 0/1",
    );
    assert_close(
        covariance(&counts[0], &counts[2]),
        -1.0 / 3.0,
        MOMENT_TOLERANCE,
        "multinomial covariance 0/2",
    );
    assert_close(
        covariance(&counts[1], &counts[2]),
        -1.0 / 3.0,
        MOMENT_TOLERANCE,
        "multinomial covariance 1/2",
    );
    assert!((means[0] - means[1]).abs() <= MOMENT_TOLERANCE);
    assert!((means[0] - means[2]).abs() <= MOMENT_TOLERANCE);
    assert!((means[1] - means[2]).abs() <= MOMENT_TOLERANCE);
}

#[test]
fn separate_locus_addresses_do_not_create_cross_locus_provenance_correlation() {
    let schema = schema("popgen-05e1-two-locus", 2, &["left", "right"]);
    let left_origin = synthetic_origin(61);
    let right_origin = synthetic_origin(63);
    let source = state_from_partitions(
        &schema,
        "two-locus-pop",
        &[("left", 0, &[(61, 1), (62, 1)]), ("right", 0, &[(63, 1), (64, 1)])],
    );
    let point = start_point(&schema, &source, "two-locus-experiment");
    let mut left_counts = Vec::with_capacity(BINARY_REPLICATES);
    let mut right_counts = Vec::with_capacity(BINARY_REPLICATES);

    for replicate in 0..BINARY_REPLICATES {
        let result = execute(
            &schema,
            &source,
            &point,
            &format!("two-locus-null-{replicate}"),
        );
        left_counts.push(origin_count(&result.destination, "left", left_origin));
        right_counts.push(origin_count(&result.destination, "right", right_origin));
    }

    let (left_mean, left_variance) = mean_and_variance(&left_counts);
    let (right_mean, right_variance) = mean_and_variance(&right_counts);
    assert_close(left_mean, 1.0, MOMENT_TOLERANCE, "left-locus mean");
    assert_close(right_mean, 1.0, MOMENT_TOLERANCE, "right-locus mean");
    assert_close(left_variance, 0.5, MOMENT_TOLERANCE, "left-locus variance");
    assert_close(right_variance, 0.5, MOMENT_TOLERANCE, "right-locus variance");
    assert!(
        covariance(&left_counts, &right_counts).abs() <= CROSS_LOCUS_COVARIANCE_TOLERANCE,
        "cross-locus covariance exceeded frozen null tolerance"
    );
}
