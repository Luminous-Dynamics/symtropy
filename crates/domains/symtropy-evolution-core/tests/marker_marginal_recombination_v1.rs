use symtropy_evolution_core::{
    derive_marker_marginal_poisson_linked_gamete, derive_zero_crossover_linked_gamete, AlleleId,
    ChromosomeDefinition, ChromosomeHaplotype, ChromosomeId, ChromosomeLocus, ChromosomeMap,
    ChromosomeMapId, ChromosomeRecombinationDomain, ChromosomeRecombinationModel,
    ChromosomeRecombinationProfile, ChromosomeRecombinationProfileId, CrossoverParity,
    EvolutionError, GeneticMapIntervalMicromorgans, GeneticMapPositionMicromorgans,
    HereditarySchema, HereditarySchemaId, LocusDefinition, LocusId, ParentRole,
    PhasedChromosomeState, PhasedHereditaryState, ReproductionEventId,
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

fn position(value: u64) -> GeneticMapPositionMicromorgans {
    GeneticMapPositionMicromorgans::new(value)
}

fn interval(start: u64, end: u64) -> GeneticMapIntervalMicromorgans {
    GeneticMapIntervalMicromorgans::new(position(start), position(end)).unwrap()
}

fn schema(ids: &[&str]) -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new(format!("marker-marginal-schema-{}", ids.len())).unwrap(),
        2,
        ids.iter()
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

fn one_chromosome_map(
    schema: &HereditarySchema,
    map_id: &str,
    loci: &[(&str, u64)],
) -> ChromosomeMap {
    ChromosomeMap::new(
        ChromosomeMapId::new(map_id).unwrap(),
        schema,
        [ChromosomeDefinition::new(
            chromosome("chr-a"),
            loci.iter()
                .map(|(id, pos)| ChromosomeLocus::new(locus(id), position(*pos)))
                .collect(),
        )
        .unwrap()],
    )
    .unwrap()
}

fn profile(
    schema: &HereditarySchema,
    map: &ChromosomeMap,
    model: ChromosomeRecombinationModel,
    end: u64,
) -> ChromosomeRecombinationProfile {
    ChromosomeRecombinationProfile::new(
        ChromosomeRecombinationProfileId::new("marker-marginal-profile-v1").unwrap(),
        schema,
        map,
        model,
        [ChromosomeRecombinationDomain::new(
            chromosome("chr-a"),
            interval(0, end),
        )
        .unwrap()],
    )
    .unwrap()
}

fn hap(ids: &[&str]) -> ChromosomeHaplotype {
    ChromosomeHaplotype::new(ids.iter().map(|id| allele(id)).collect())
}

fn source(
    schema: &HereditarySchema,
    map: &ChromosomeMap,
    hap0: &[&str],
    hap1: &[&str],
) -> PhasedHereditaryState {
    PhasedHereditaryState::new(
        schema,
        map,
        [PhasedChromosomeState::new(
            chromosome("chr-a"),
            vec![hap(hap0), hap(hap1)],
        )],
    )
    .unwrap()
}

#[test]
fn no_toggle_poisson_realization_matches_zero_crossover_at_modeled_loci() {
    let schema = schema(&["a", "b", "c"]);
    let map = one_chromosome_map(&schema, "tiny-distance-map", &[("a", 10), ("b", 11), ("c", 12)]);
    let source = source(&schema, &map, &["a0", "b0", "c0"], &["a1", "b1", "c1"]);
    let poisson = profile(
        &schema,
        &map,
        ChromosomeRecombinationModel::PoissonCrossoversNoInterferenceV1,
        100,
    );
    let zero = profile(
        &schema,
        &map,
        ChromosomeRecombinationModel::NoCrossoversIndependentAssortmentV1,
        100,
    );
    let event = ReproductionEventId::new("parity-3").unwrap();

    let recombinant = derive_marker_marginal_poisson_linked_gamete(
        &schema,
        &map,
        &source,
        &poisson,
        &event,
        ParentRole::ParentA,
    )
    .unwrap();
    let baseline = derive_zero_crossover_linked_gamete(
        &schema,
        &map,
        &source,
        &zero,
        &event,
        ParentRole::ParentA,
    )
    .unwrap();

    assert_eq!(recombinant.gamete, baseline.gamete);
    for record in &recombinant.provenance.chromosomes()[&chromosome("chr-a")].adjacent_intervals {
        assert_eq!(record.odd_probability_ppm, 1);
        assert_eq!(record.parity, CrossoverParity::Even);
    }
}

#[test]
fn known_large_distance_vectors_toggle_exactly_on_odd_parity() {
    let schema = schema(&["a", "b", "c"]);
    let map = one_chromosome_map(
        &schema,
        "large-distance-map",
        &[("a", 1), ("b", 7_000_001), ("c", 14_000_001)],
    );
    let source = source(&schema, &map, &["a0", "b0", "c0"], &["a1", "b1", "c1"]);
    let poisson = profile(
        &schema,
        &map,
        ChromosomeRecombinationModel::PoissonCrossoversNoInterferenceV1,
        14_000_002,
    );

    // Frozen V1 interval draws for ParentA/chr-a are below 500_000 for both
    // a-b and b-c under event "parity-8".
    let event = ReproductionEventId::new("parity-8").unwrap();
    let derived = derive_marker_marginal_poisson_linked_gamete(
        &schema,
        &map,
        &source,
        &poisson,
        &event,
        ParentRole::ParentA,
    )
    .unwrap();
    let evidence = &derived.provenance.chromosomes()[&chromosome("chr-a")];
    assert_eq!(evidence.adjacent_intervals.len(), 2);
    assert_eq!(evidence.adjacent_intervals[0].opportunity_draw_ppm, 15_227);
    assert_eq!(evidence.adjacent_intervals[1].opportunity_draw_ppm, 197_183);
    assert_eq!(evidence.adjacent_intervals[0].parity, CrossoverParity::Odd);
    assert_eq!(evidence.adjacent_intervals[1].parity, CrossoverParity::Odd);
    assert_ne!(
        evidence.adjacent_intervals[0].source_haplotype_slot_after,
        evidence.initial_source_haplotype_slot
    );
    assert_eq!(
        evidence.adjacent_intervals[1].source_haplotype_slot_after,
        evidence.initial_source_haplotype_slot
    );
}

#[test]
fn map_distance_sweep_reuses_interval_opportunity_draw() {
    let schema = schema(&["a", "b"]);
    let near_map = one_chromosome_map(&schema, "distance-map", &[("a", 10), ("b", 100_010)]);
    let far_map = one_chromosome_map(&schema, "distance-map", &[("a", 10), ("b", 500_010)]);
    let near_source = source(&schema, &near_map, &["a0", "b0"], &["a1", "b1"]);
    let far_source = source(&schema, &far_map, &["a0", "b0"], &["a1", "b1"]);
    let near_profile = profile(
        &schema,
        &near_map,
        ChromosomeRecombinationModel::PoissonCrossoversNoInterferenceV1,
        600_000,
    );
    let far_profile = profile(
        &schema,
        &far_map,
        ChromosomeRecombinationModel::PoissonCrossoversNoInterferenceV1,
        600_000,
    );
    let event = ReproductionEventId::new("distance-sweep").unwrap();

    let near = derive_marker_marginal_poisson_linked_gamete(
        &schema,
        &near_map,
        &near_source,
        &near_profile,
        &event,
        ParentRole::ParentA,
    )
    .unwrap();
    let far = derive_marker_marginal_poisson_linked_gamete(
        &schema,
        &far_map,
        &far_source,
        &far_profile,
        &event,
        ParentRole::ParentA,
    )
    .unwrap();

    let near_record = &near.provenance.chromosomes()[&chromosome("chr-a")].adjacent_intervals[0];
    let far_record = &far.provenance.chromosomes()[&chromosome("chr-a")].adjacent_intervals[0];
    assert_eq!(near_record.opportunity_draw_ppm, far_record.opportunity_draw_ppm);
    assert!(near_record.odd_probability_ppm < far_record.odd_probability_ppm);
}

#[test]
fn zero_crossover_profile_and_clonal_role_are_rejected() {
    let schema = schema(&["a", "b"]);
    let map = one_chromosome_map(&schema, "reject-map", &[("a", 10), ("b", 100_010)]);
    let source = source(&schema, &map, &["a0", "b0"], &["a1", "b1"]);
    let zero = profile(
        &schema,
        &map,
        ChromosomeRecombinationModel::NoCrossoversIndependentAssortmentV1,
        200_000,
    );
    let event = ReproductionEventId::new("reject-event").unwrap();

    assert_eq!(
        derive_marker_marginal_poisson_linked_gamete(
            &schema,
            &map,
            &source,
            &zero,
            &event,
            ParentRole::ParentA,
        )
        .unwrap_err(),
        EvolutionError::LinkedGameteRecombinationModelMismatch
    );

    let poisson = profile(
        &schema,
        &map,
        ChromosomeRecombinationModel::PoissonCrossoversNoInterferenceV1,
        200_000,
    );
    assert_eq!(
        derive_marker_marginal_poisson_linked_gamete(
            &schema,
            &map,
            &source,
            &poisson,
            &event,
            ParentRole::ClonalParent,
        )
        .unwrap_err(),
        EvolutionError::LinkedGameteRequiresSexualParentRole
    );
}

#[test]
fn restored_parity_evidence_requires_exact_replay_and_external_event_context() {
    let schema = schema(&["a", "b"]);
    let map = one_chromosome_map(&schema, "restore-map", &[("a", 1), ("b", 7_000_001)]);
    let source = source(&schema, &map, &["a0", "b0"], &["a1", "b1"]);
    let poisson = profile(
        &schema,
        &map,
        ChromosomeRecombinationModel::PoissonCrossoversNoInterferenceV1,
        7_000_002,
    );
    let event = ReproductionEventId::new("parity-8").unwrap();
    let derived = derive_marker_marginal_poisson_linked_gamete(
        &schema,
        &map,
        &source,
        &poisson,
        &event,
        ParentRole::ParentA,
    )
    .unwrap();

    let encoded = serde_json::to_string(&derived).unwrap();
    let restored = serde_json::from_str::<symtropy_evolution_core::MarkerMarginalGameteDerivation>(
        &encoded,
    )
    .unwrap();
    restored
        .provenance
        .validate_current(
            &schema,
            &map,
            &source,
            &poisson,
            &event,
            ParentRole::ParentA,
            &restored.gamete,
        )
        .unwrap();

    assert_eq!(
        restored
            .provenance
            .validate_current(
                &schema,
                &map,
                &source,
                &poisson,
                &ReproductionEventId::new("different-event").unwrap(),
                ParentRole::ParentA,
                &restored.gamete,
            )
            .unwrap_err(),
        EvolutionError::LinkedGameteEventContextMismatch
    );

    let mut value = serde_json::to_value(&restored.provenance).unwrap();
    value["chromosomes"]["chr-a"]["adjacent_intervals"][0]["parity"] =
        serde_json::json!("Even");
    let tampered = serde_json::from_value::<
        symtropy_evolution_core::MarkerMarginalGameteDerivationProvenance,
    >(value)
    .unwrap();
    assert_eq!(
        tampered
            .validate_current(
                &schema,
                &map,
                &source,
                &poisson,
                &event,
                ParentRole::ParentA,
                &restored.gamete,
            )
            .unwrap_err(),
        EvolutionError::LinkedGameteDerivationMismatch
    );
}
