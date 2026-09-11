use symtropy_evolution_core::{
    derive_marker_marginal_poisson_linked_gamete, derive_modeled_gamete_ancestry,
    derive_zero_crossover_linked_gamete, AlleleId, AncestryCopyId, ChromosomeAncestryState,
    ChromosomeDefinition, ChromosomeHaplotype, ChromosomeId, ChromosomeLocus, ChromosomeMap,
    ChromosomeMapId, ChromosomeRecombinationDomain, ChromosomeRecombinationModel,
    ChromosomeRecombinationProfile, ChromosomeRecombinationProfileId, CrossoverParity,
    GeneticMapIntervalMicromorgans, GeneticMapPositionMicromorgans, HaplotypeAncestryClass,
    HereditarySchema, HereditarySchemaId, LinkedGameteDerivationEvidence, LocusDefinition,
    LocusId, ParentRole, PhasedAncestryState, PhasedChromosomeState, PhasedHereditaryState,
    ReproductionEventId,
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
fn ancestry(id: &str) -> AncestryCopyId {
    AncestryCopyId::new(id).unwrap()
}
fn pos(value: u64) -> GeneticMapPositionMicromorgans {
    GeneticMapPositionMicromorgans::new(value)
}
fn interval(start: u64, end: u64) -> GeneticMapIntervalMicromorgans {
    GeneticMapIntervalMicromorgans::new(pos(start), pos(end)).unwrap()
}
fn hap(ids: &[&str]) -> ChromosomeHaplotype {
    ChromosomeHaplotype::new(ids.iter().map(|id| allele(id)).collect())
}

fn schema() -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new("phylo-04b-schema").unwrap(),
        2,
        [
            LocusDefinition::new(locus("a"), [allele("a0"), allele("a1")]).unwrap(),
            LocusDefinition::new(locus("b"), [allele("b0"), allele("b1")]).unwrap(),
            LocusDefinition::new(locus("c"), [allele("c0"), allele("c1")]).unwrap(),
        ],
    )
    .unwrap()
}

fn map(schema: &HereditarySchema) -> ChromosomeMap {
    ChromosomeMap::new(
        ChromosomeMapId::new("phylo-04b-map").unwrap(),
        schema,
        [ChromosomeDefinition::new(
            chromosome("chr-a"),
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

fn profile(
    schema: &HereditarySchema,
    map: &ChromosomeMap,
    id: &str,
    model: ChromosomeRecombinationModel,
) -> ChromosomeRecombinationProfile {
    ChromosomeRecombinationProfile::new(
        ChromosomeRecombinationProfileId::new(id).unwrap(),
        schema,
        map,
        model,
        [ChromosomeRecombinationDomain::new(
            chromosome("chr-a"),
            interval(0, 14_000_002),
        )
        .unwrap()],
    )
    .unwrap()
}

fn source(
    schema: &HereditarySchema,
    map: &ChromosomeMap,
    first: &[&str],
    second: &[&str],
) -> PhasedHereditaryState {
    PhasedHereditaryState::new(
        schema,
        map,
        [PhasedChromosomeState::new(
            chromosome("chr-a"),
            vec![hap(first), hap(second)],
        )],
    )
    .unwrap()
}

fn class(slot: u8, ids: &[&str]) -> HaplotypeAncestryClass {
    HaplotypeAncestryClass::new(slot, ids.iter().map(|id| ancestry(id)).collect()).unwrap()
}

fn distinct_ancestry(
    schema: &HereditarySchema,
    map: &ChromosomeMap,
    source: &PhasedHereditaryState,
) -> PhasedAncestryState {
    PhasedAncestryState::new(
        schema,
        map,
        source,
        [ChromosomeAncestryState::new(
            chromosome("chr-a"),
            vec![class(0, &["copy-a"]), class(1, &["copy-b"])],
        )
        .unwrap()],
    )
    .unwrap()
}

fn identical_ancestry(
    schema: &HereditarySchema,
    map: &ChromosomeMap,
    source: &PhasedHereditaryState,
) -> PhasedAncestryState {
    PhasedAncestryState::new(
        schema,
        map,
        source,
        [ChromosomeAncestryState::new(
            chromosome("chr-a"),
            vec![class(0, &["copy-a", "copy-b"])],
        )
        .unwrap()],
    )
    .unwrap()
}

#[test]
fn distinct_zero_crossover_uses_genetically_selected_singleton_ancestry_class() {
    let schema = schema();
    let map = map(&schema);
    let source_state = source(
        &schema,
        &map,
        &["a0", "b0", "c0"],
        &["a1", "b1", "c1"],
    );
    let ancestry_state = distinct_ancestry(&schema, &map, &source_state);
    let zero = profile(
        &schema,
        &map,
        "zero-v1",
        ChromosomeRecombinationModel::NoCrossoversIndependentAssortmentV1,
    );
    let event = ReproductionEventId::new("distinct-zero").unwrap();
    let genetic = derive_zero_crossover_linked_gamete(
        &schema,
        &map,
        &source_state,
        &zero,
        &event,
        ParentRole::ParentA,
    )
    .unwrap();
    let slot = usize::from(
        genetic.provenance.segments()[&chromosome("chr-a")].source_haplotype_slot,
    );
    let expected = ancestry_state
        .copy_ids_for_haplotype_content_at_slot(
            &schema,
            &map,
            &source_state,
            &chromosome("chr-a"),
            slot,
        )
        .unwrap()[0]
        .clone();
    let evidence = LinkedGameteDerivationEvidence::ZeroCrossover(genetic);
    let derived = derive_modeled_gamete_ancestry(
        &schema,
        &map,
        &source_state,
        &ancestry_state,
        &zero,
        &evidence,
        &event,
        ParentRole::ParentA,
    )
    .unwrap();

    let origins = &derived.ancestry.chromosomes[&chromosome("chr-a")].loci;
    assert_eq!(origins.len(), 3);
    assert!(origins.iter().all(|origin| origin.source_copy_id == expected));
}

#[test]
fn identical_zero_crossover_can_change_genealogy_without_changing_genetics() {
    let schema = schema();
    let map = map(&schema);
    let source_state = source(
        &schema,
        &map,
        &["a0", "b0", "c0"],
        &["a0", "b0", "c0"],
    );
    let ancestry_state = identical_ancestry(&schema, &map, &source_state);
    let zero = profile(
        &schema,
        &map,
        "zero-identical",
        ChromosomeRecombinationModel::NoCrossoversIndependentAssortmentV1,
    );
    let event_one = ReproductionEventId::new("ancestry-0").unwrap();
    let event_zero = ReproductionEventId::new("ancestry-5").unwrap();

    let genetic_one = derive_zero_crossover_linked_gamete(
        &schema,
        &map,
        &source_state,
        &zero,
        &event_one,
        ParentRole::ParentA,
    )
    .unwrap();
    let genetic_zero = derive_zero_crossover_linked_gamete(
        &schema,
        &map,
        &source_state,
        &zero,
        &event_zero,
        ParentRole::ParentA,
    )
    .unwrap();
    assert_eq!(genetic_one.gamete, genetic_zero.gamete);

    let evidence_one = LinkedGameteDerivationEvidence::ZeroCrossover(genetic_one);
    let evidence_zero = LinkedGameteDerivationEvidence::ZeroCrossover(genetic_zero);
    let ancestry_one = derive_modeled_gamete_ancestry(
        &schema,
        &map,
        &source_state,
        &ancestry_state,
        &zero,
        &evidence_one,
        &event_one,
        ParentRole::ParentA,
    )
    .unwrap();
    let ancestry_zero = derive_modeled_gamete_ancestry(
        &schema,
        &map,
        &source_state,
        &ancestry_state,
        &zero,
        &evidence_zero,
        &event_zero,
        ParentRole::ParentA,
    )
    .unwrap();

    let one = &ancestry_one.ancestry.chromosomes[&chromosome("chr-a")].loci;
    let zero_origins = &ancestry_zero.ancestry.chromosomes[&chromosome("chr-a")].loci;
    assert!(one.iter().all(|origin| origin.source_copy_id == ancestry("copy-b")));
    assert!(zero_origins
        .iter()
        .all(|origin| origin.source_copy_id == ancestry("copy-a")));
    assert_ne!(ancestry_one.ancestry, ancestry_zero.ancestry);
    assert_ne!(
        ancestry_one.provenance.canonical_digest(),
        ancestry_zero.provenance.canonical_digest()
    );
}

#[test]
fn distinct_marker_parity_tracks_the_genetically_selected_content_class() {
    let schema = schema();
    let map = map(&schema);
    let source_state = source(
        &schema,
        &map,
        &["a0", "b0", "c0"],
        &["a1", "b1", "c1"],
    );
    let ancestry_state = distinct_ancestry(&schema, &map, &source_state);
    let poisson = profile(
        &schema,
        &map,
        "poisson-distinct",
        ChromosomeRecombinationModel::PoissonCrossoversNoInterferenceV1,
    );
    let event = ReproductionEventId::new("parity-8").unwrap();
    let genetic = derive_marker_marginal_poisson_linked_gamete(
        &schema,
        &map,
        &source_state,
        &poisson,
        &event,
        ParentRole::ParentA,
    )
    .unwrap();
    let marker = &genetic.provenance.chromosomes()[&chromosome("chr-a")];
    assert_eq!(marker.adjacent_intervals[0].parity, CrossoverParity::Odd);
    assert_eq!(marker.adjacent_intervals[1].parity, CrossoverParity::Odd);

    let mut expected_slots = vec![usize::from(marker.initial_source_haplotype_slot)];
    expected_slots.extend(
        marker
            .adjacent_intervals
            .iter()
            .map(|interval| usize::from(interval.source_haplotype_slot_after)),
    );
    let expected: Vec<AncestryCopyId> = expected_slots
        .into_iter()
        .map(|slot| {
            ancestry_state
                .copy_ids_for_haplotype_content_at_slot(
                    &schema,
                    &map,
                    &source_state,
                    &chromosome("chr-a"),
                    slot,
                )
                .unwrap()[0]
                .clone()
        })
        .collect();

    let evidence = LinkedGameteDerivationEvidence::MarkerMarginalPoisson(genetic);
    let derived = derive_modeled_gamete_ancestry(
        &schema,
        &map,
        &source_state,
        &ancestry_state,
        &poisson,
        &evidence,
        &event,
        ParentRole::ParentA,
    )
    .unwrap();
    let observed: Vec<AncestryCopyId> = derived.ancestry.chromosomes[&chromosome("chr-a")]
        .loci
        .iter()
        .map(|origin| origin.source_copy_id.clone())
        .collect();
    assert_eq!(observed, expected);
}

#[test]
fn identical_marker_odd_parity_toggles_genealogy_while_genetic_slot_stays_canonical() {
    let schema = schema();
    let map = map(&schema);
    let source_state = source(
        &schema,
        &map,
        &["a0", "b0", "c0"],
        &["a0", "b0", "c0"],
    );
    let ancestry_state = identical_ancestry(&schema, &map, &source_state);
    let poisson = profile(
        &schema,
        &map,
        "poisson-identical",
        ChromosomeRecombinationModel::PoissonCrossoversNoInterferenceV1,
    );
    let event = ReproductionEventId::new("parity-8").unwrap();
    let genetic = derive_marker_marginal_poisson_linked_gamete(
        &schema,
        &map,
        &source_state,
        &poisson,
        &event,
        ParentRole::ParentA,
    )
    .unwrap();
    let marker = &genetic.provenance.chromosomes()[&chromosome("chr-a")];
    assert_eq!(marker.initial_source_haplotype_slot, 0);
    assert!(marker
        .adjacent_intervals
        .iter()
        .all(|interval| interval.source_haplotype_slot_after == 0));
    assert!(marker
        .adjacent_intervals
        .iter()
        .all(|interval| interval.parity == CrossoverParity::Odd));

    let evidence = LinkedGameteDerivationEvidence::MarkerMarginalPoisson(genetic);
    let derived = derive_modeled_gamete_ancestry(
        &schema,
        &map,
        &source_state,
        &ancestry_state,
        &poisson,
        &evidence,
        &event,
        ParentRole::ParentA,
    )
    .unwrap();
    let origins = &derived.ancestry.chromosomes[&chromosome("chr-a")].loci;
    assert_eq!(origins.len(), 3);
    assert_ne!(origins[0].source_copy_id, origins[1].source_copy_id);
    assert_ne!(origins[1].source_copy_id, origins[2].source_copy_id);
    assert_eq!(origins[0].source_copy_id, origins[2].source_copy_id);
}

#[test]
fn restored_or_tampered_ancestry_requires_full_current_replay() {
    let schema = schema();
    let map = map(&schema);
    let source_state = source(
        &schema,
        &map,
        &["a0", "b0", "c0"],
        &["a0", "b0", "c0"],
    );
    let ancestry_state = identical_ancestry(&schema, &map, &source_state);
    let zero = profile(
        &schema,
        &map,
        "zero-restore",
        ChromosomeRecombinationModel::NoCrossoversIndependentAssortmentV1,
    );
    let event = ReproductionEventId::new("ancestry-0").unwrap();
    let evidence = LinkedGameteDerivationEvidence::ZeroCrossover(
        derive_zero_crossover_linked_gamete(
            &schema,
            &map,
            &source_state,
            &zero,
            &event,
            ParentRole::ParentA,
        )
        .unwrap(),
    );
    let derived = derive_modeled_gamete_ancestry(
        &schema,
        &map,
        &source_state,
        &ancestry_state,
        &zero,
        &evidence,
        &event,
        ParentRole::ParentA,
    )
    .unwrap();

    let encoded = serde_json::to_string(&derived).unwrap();
    let mut restored: symtropy_evolution_core::GameteAncestryDerivation =
        serde_json::from_str(&encoded).unwrap();
    restored
        .provenance
        .validate_current(
            &schema,
            &map,
            &source_state,
            &ancestry_state,
            &zero,
            &evidence,
            &event,
            ParentRole::ParentA,
            &restored.ancestry,
        )
        .unwrap();

    restored.ancestry.chromosomes.get_mut(&chromosome("chr-a")).unwrap().loci[0]
        .source_copy_id = ancestry("forged-copy");
    assert!(restored
        .provenance
        .validate_current(
            &schema,
            &map,
            &source_state,
            &ancestry_state,
            &zero,
            &evidence,
            &event,
            ParentRole::ParentA,
            &restored.ancestry,
        )
        .is_err());
}
