use symtropy_evolution_core::{
    AlleleId, ChromosomeDefinition, ChromosomeId, ChromosomeLocus, ChromosomeMap,
    ChromosomeMapId, ChromosomeRecombinationDomain, ChromosomeRecombinationModel,
    ChromosomeRecombinationProfile, ChromosomeRecombinationProfileId, EvolutionError,
    GeneticMapIntervalMicromorgans, GeneticMapPositionMicromorgans, HereditarySchema,
    HereditarySchemaId, LocusDefinition, LocusId,
};

fn allele(id: &str) -> AlleleId {
    AlleleId::new(id).unwrap()
}
fn locus(id: &str) -> LocusId {
    LocusId::new(id).unwrap()
}
fn chromosome(id: &str) -> ChromosomeId {
    ChromosomeId::new(id).unwrap()
}
fn pos(value: u64) -> GeneticMapPositionMicromorgans {
    GeneticMapPositionMicromorgans::new(value)
}
fn interval(start: u64, end: u64) -> GeneticMapIntervalMicromorgans {
    GeneticMapIntervalMicromorgans::new(pos(start), pos(end)).unwrap()
}
fn domain(id: &str, start: u64, end: u64) -> ChromosomeRecombinationDomain {
    ChromosomeRecombinationDomain::new(chromosome(id), interval(start, end)).unwrap()
}

fn schema(ploidy: u8) -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new("recombination-schema-v1").unwrap(),
        ploidy,
        ["a", "b", "c", "d"]
            .into_iter()
            .map(|id| {
                LocusDefinition::new(
                    locus(id),
                    [allele(&format!("{id}0")), allele(&format!("{id}1"))],
                )
                .unwrap()
            })
            .collect::<Vec<_>>(),
    )
    .unwrap()
}

fn map(schema: &HereditarySchema) -> ChromosomeMap {
    ChromosomeMap::new(
        ChromosomeMapId::new("recombination-map-v1").unwrap(),
        schema,
        [
            ChromosomeDefinition::new(
                chromosome("chr-a"),
                vec![
                    ChromosomeLocus::new(locus("a"), pos(100_000)),
                    ChromosomeLocus::new(locus("b"), pos(250_000)),
                ],
            )
            .unwrap(),
            ChromosomeDefinition::new(
                chromosome("chr-b"),
                vec![
                    ChromosomeLocus::new(locus("c"), pos(50_000)),
                    ChromosomeLocus::new(locus("d"), pos(400_000)),
                ],
            )
            .unwrap(),
        ],
    )
    .unwrap()
}

fn profile(schema: &HereditarySchema, map: &ChromosomeMap) -> ChromosomeRecombinationProfile {
    ChromosomeRecombinationProfile::new(
        ChromosomeRecombinationProfileId::new("poisson-no-interference-v1").unwrap(),
        schema,
        map,
        ChromosomeRecombinationModel::PoissonCrossoversNoInterferenceV1,
        [domain("chr-a", 0, 500_000), domain("chr-b", 0, 600_000)],
    )
    .unwrap()
}

#[test]
fn valid_profile_round_trips_and_process_span_may_exceed_marker_span() {
    let schema = schema(2);
    let map = map(&schema);
    let profile = profile(&schema, &map);
    profile.validate(&schema, &map).unwrap();
    assert_eq!(
        profile.domains[&chromosome("chr-a")]
            .interval
            .length_micromorgans()
            .unwrap(),
        500_000
    );
    assert!(profile.domains[&chromosome("chr-a")].interval.start < map.chromosomes[&chromosome("chr-a")].loci[0].position);
    assert!(profile.domains[&chromosome("chr-a")].interval.end > map.chromosomes[&chromosome("chr-a")].loci[1].position);

    let digest = profile.canonical_digest(&schema, &map).unwrap();
    let restored: ChromosomeRecombinationProfile =
        serde_json::from_str(&serde_json::to_string(&profile).unwrap()).unwrap();
    restored.validate(&schema, &map).unwrap();
    assert_eq!(restored.canonical_digest(&schema, &map).unwrap(), digest);
}

#[test]
fn domain_declaration_order_is_not_semantic_identity() {
    let schema = schema(2);
    let map = map(&schema);
    let forward = profile(&schema, &map);
    let reverse = ChromosomeRecombinationProfile::new(
        forward.id.clone(),
        &schema,
        &map,
        ChromosomeRecombinationModel::PoissonCrossoversNoInterferenceV1,
        [domain("chr-b", 0, 600_000), domain("chr-a", 0, 500_000)],
    )
    .unwrap();
    assert_eq!(forward, reverse);
    assert_eq!(
        forward.canonical_digest(&schema, &map).unwrap(),
        reverse.canonical_digest(&schema, &map).unwrap()
    );
}

#[test]
fn changing_process_interval_changes_exact_authority() {
    let schema = schema(2);
    let map = map(&schema);
    let a = profile(&schema, &map);
    let b = ChromosomeRecombinationProfile::new(
        a.id.clone(),
        &schema,
        &map,
        ChromosomeRecombinationModel::PoissonCrossoversNoInterferenceV1,
        [domain("chr-a", 0, 510_000), domain("chr-b", 0, 600_000)],
    )
    .unwrap();
    assert_ne!(
        a.canonical_digest(&schema, &map).unwrap(),
        b.canonical_digest(&schema, &map).unwrap()
    );
}

#[test]
fn reference_process_rejects_non_diploid_schema() {
    let triploid = schema(3);
    let map = map(&triploid);
    let result = ChromosomeRecombinationProfile::new(
        ChromosomeRecombinationProfileId::new("triploid-rejected").unwrap(),
        &triploid,
        &map,
        ChromosomeRecombinationModel::PoissonCrossoversNoInterferenceV1,
        [domain("chr-a", 0, 500_000), domain("chr-b", 0, 600_000)],
    );
    assert_eq!(result.unwrap_err(), EvolutionError::ModeRequiresDiploid(3));
}

#[test]
fn missing_unknown_and_duplicate_domains_fail_closed() {
    let schema = schema(2);
    let map = map(&schema);

    assert!(ChromosomeRecombinationProfile::new(
        ChromosomeRecombinationProfileId::new("missing-domain").unwrap(),
        &schema,
        &map,
        ChromosomeRecombinationModel::PoissonCrossoversNoInterferenceV1,
        [domain("chr-a", 0, 500_000)],
    )
    .is_err());

    assert!(ChromosomeRecombinationProfile::new(
        ChromosomeRecombinationProfileId::new("unknown-domain").unwrap(),
        &schema,
        &map,
        ChromosomeRecombinationModel::PoissonCrossoversNoInterferenceV1,
        [
            domain("chr-a", 0, 500_000),
            domain("chr-b", 0, 600_000),
            domain("chr-x", 0, 100_000),
        ],
    )
    .is_err());

    assert!(matches!(
        ChromosomeRecombinationProfile::new(
            ChromosomeRecombinationProfileId::new("duplicate-domain").unwrap(),
            &schema,
            &map,
            ChromosomeRecombinationModel::PoissonCrossoversNoInterferenceV1,
            [
                domain("chr-a", 0, 500_000),
                domain("chr-a", 0, 550_000),
                domain("chr-b", 0, 600_000),
            ],
        ),
        Err(EvolutionError::DuplicateChromosomeIdentity { .. })
    ));
}

#[test]
fn mapped_locus_must_lie_inside_process_interval() {
    let schema = schema(2);
    let map = map(&schema);
    assert!(ChromosomeRecombinationProfile::new(
        ChromosomeRecombinationProfileId::new("too-short").unwrap(),
        &schema,
        &map,
        ChromosomeRecombinationModel::PoissonCrossoversNoInterferenceV1,
        [domain("chr-a", 150_000, 500_000), domain("chr-b", 0, 600_000)],
    )
    .is_err());
}

#[test]
fn changed_exact_map_or_schema_stales_profile() {
    let schema = schema(2);
    let map = map(&schema);
    let profile = profile(&schema, &map);

    let changed_map = ChromosomeMap::new(
        map.id.clone(),
        &schema,
        [
            ChromosomeDefinition::new(
                chromosome("chr-a"),
                vec![
                    ChromosomeLocus::new(locus("a"), pos(100_000)),
                    ChromosomeLocus::new(locus("b"), pos(251_000)),
                ],
            )
            .unwrap(),
            map.chromosomes[&chromosome("chr-b")].clone(),
        ],
    )
    .unwrap();
    assert!(profile.validate(&schema, &changed_map).is_err());

    let changed_schema = HereditarySchema::new(
        schema.id.clone(),
        2,
        ["a", "b", "c", "d"]
            .into_iter()
            .map(|id| {
                let mut alleles = vec![allele(&format!("{id}0")), allele(&format!("{id}1"))];
                if id == "d" {
                    alleles.push(allele("d2"));
                }
                LocusDefinition::new(locus(id), alleles).unwrap()
            })
            .collect::<Vec<_>>(),
    )
    .unwrap();
    assert!(profile.validate(&changed_schema, &map).is_err());
}

#[test]
fn hostile_restored_domain_key_mismatch_cannot_regain_authority() {
    let schema = schema(2);
    let map = map(&schema);
    let mut raw = profile(&schema, &map);
    let chr_a = raw.domains.remove(&chromosome("chr-a")).unwrap();
    raw.domains.insert(chromosome("forged-key"), chr_a);
    let restored: ChromosomeRecombinationProfile =
        serde_json::from_str(&serde_json::to_string(&raw).unwrap()).unwrap();
    assert!(matches!(
        restored.validate(&schema, &map),
        Err(EvolutionError::ChromosomeKeyMismatch { .. }) | Err(EvolutionError::OperatorAuthorityMismatch)
    ));
}

#[test]
fn hostile_restored_invalid_interval_is_panic_free_and_non_authoritative() {
    let schema = schema(2);
    let map = map(&schema);
    let mut raw = profile(&schema, &map);
    raw.domains.get_mut(&chromosome("chr-a")).unwrap().interval =
        GeneticMapIntervalMicromorgans {
            start: pos(600_000),
            end: pos(100_000),
        };

    let restored: ChromosomeRecombinationProfile =
        serde_json::from_str(&serde_json::to_string(&raw).unwrap()).unwrap();
    assert!(restored.domains[&chromosome("chr-a")]
        .interval
        .length_micromorgans()
        .is_err());
    assert!(restored.validate(&schema, &map).is_err());
    assert!(restored.canonical_digest(&schema, &map).is_err());
}