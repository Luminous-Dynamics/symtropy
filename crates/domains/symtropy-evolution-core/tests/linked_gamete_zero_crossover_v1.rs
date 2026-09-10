use symtropy_evolution_core::{
    derive_zero_crossover_linked_gamete, AlleleId, ChromosomeDefinition, ChromosomeHaplotype,
    ChromosomeId, ChromosomeLocus, ChromosomeMap, ChromosomeMapId,
    ChromosomeRecombinationDomain, ChromosomeRecombinationModel, ChromosomeRecombinationProfile,
    ChromosomeRecombinationProfileId, EvolutionError, GeneticMapIntervalMicromorgans,
    GeneticMapPositionMicromorgans, HereditarySchema, HereditarySchemaId, LinkedGameteDerivation,
    LinkedGameteDerivationProvenance, LocusDefinition, LocusId, ParentRole, PhasedChromosomeState,
    PhasedHereditaryState, ReproductionEventId,
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

fn mapped_locus(id: &str, position: u64) -> ChromosomeLocus {
    ChromosomeLocus::new(
        locus(id),
        GeneticMapPositionMicromorgans::new(position),
    )
}

fn schema_with_loci(ploidy: u8, ids: &[&str]) -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new(format!("linked-gamete-schema-{ploidy}-{}", ids.len())).unwrap(),
        ploidy,
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

fn schema() -> HereditarySchema {
    schema_with_loci(2, &["a", "b", "c", "d"])
}

fn chromosome_map(schema: &HereditarySchema) -> ChromosomeMap {
    ChromosomeMap::new(
        ChromosomeMapId::new("linked-gamete-map-v1").unwrap(),
        schema,
        [
            ChromosomeDefinition::new(
                chromosome("chr-a"),
                vec![mapped_locus("a", 50_000), mapped_locus("b", 150_000)],
            )
            .unwrap(),
            ChromosomeDefinition::new(
                chromosome("chr-b"),
                vec![mapped_locus("c", 100_000), mapped_locus("d", 300_000)],
            )
            .unwrap(),
        ],
    )
    .unwrap()
}

fn interval(start: u64, end: u64) -> GeneticMapIntervalMicromorgans {
    GeneticMapIntervalMicromorgans::new(
        GeneticMapPositionMicromorgans::new(start),
        GeneticMapPositionMicromorgans::new(end),
    )
    .unwrap()
}

fn profile_with_model(
    schema: &HereditarySchema,
    map: &ChromosomeMap,
    model: ChromosomeRecombinationModel,
) -> ChromosomeRecombinationProfile {
    ChromosomeRecombinationProfile::new(
        ChromosomeRecombinationProfileId::new("linked-gamete-recombination-v1").unwrap(),
        schema,
        map,
        model,
        [
            ChromosomeRecombinationDomain::new(chromosome("chr-a"), interval(0, 250_000))
                .unwrap(),
            ChromosomeRecombinationDomain::new(chromosome("chr-b"), interval(0, 400_000))
                .unwrap(),
        ],
    )
    .unwrap()
}

fn zero_profile(
    schema: &HereditarySchema,
    map: &ChromosomeMap,
) -> ChromosomeRecombinationProfile {
    profile_with_model(
        schema,
        map,
        ChromosomeRecombinationModel::NoCrossoversIndependentAssortmentV1,
    )
}

fn hap(ids: &[&str]) -> ChromosomeHaplotype {
    ChromosomeHaplotype::new(ids.iter().map(|id| allele(id)).collect())
}

fn source(schema: &HereditarySchema, map: &ChromosomeMap) -> PhasedHereditaryState {
    PhasedHereditaryState::new(
        schema,
        map,
        [
            PhasedChromosomeState::new(
                chromosome("chr-a"),
                vec![hap(&["a0", "b0"]), hap(&["a1", "b1"])],
            ),
            PhasedChromosomeState::new(
                chromosome("chr-b"),
                vec![hap(&["c0", "d0"]), hap(&["c1", "d1"])],
            ),
        ],
    )
    .unwrap()
}

fn derive(
    schema: &HereditarySchema,
    map: &ChromosomeMap,
    source: &PhasedHereditaryState,
    profile: &ChromosomeRecombinationProfile,
    event: &str,
    role: ParentRole,
) -> LinkedGameteDerivation {
    derive_zero_crossover_linked_gamete(
        schema,
        map,
        source,
        profile,
        &ReproductionEventId::new(event).unwrap(),
        role,
    )
    .unwrap()
}

#[test]
fn zero_crossover_gamete_uses_one_whole_source_haplotype_per_chromosome() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let profile = zero_profile(&schema, &map);
    let source = source(&schema, &map);
    let event = ReproductionEventId::new("gamete-0001").unwrap();
    let derived = derive_zero_crossover_linked_gamete(
        &schema,
        &map,
        &source,
        &profile,
        &event,
        ParentRole::ParentA,
    )
    .unwrap();

    assert_eq!(derived.gamete.chromosomes.len(), map.chromosomes.len());
    assert_eq!(derived.provenance.segments().len(), map.chromosomes.len());

    for chromosome_id in map.chromosomes.keys() {
        let segment = derived.provenance.segments().get(chromosome_id).unwrap();
        let selected = &source.chromosomes[chromosome_id].haplotypes
            [usize::from(segment.source_haplotype_slot)];
        assert_eq!(&derived.gamete.chromosomes[chromosome_id], selected);
        assert_eq!(&segment.chromosome_id, chromosome_id);
        assert_eq!(segment.interval, profile.domains[chromosome_id].interval);
    }

    derived
        .provenance
        .validate_current(
            &schema,
            &map,
            &source,
            &profile,
            &event,
            ParentRole::ParentA,
            &derived.gamete,
        )
        .unwrap();
}

#[test]
fn identical_inputs_replay_to_identical_gamete_and_provenance() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let profile = zero_profile(&schema, &map);
    let source = source(&schema, &map);

    let first = derive(&schema, &map, &source, &profile, "gamete-0001", ParentRole::ParentA);
    let second = derive(&schema, &map, &source, &profile, "gamete-0001", ParentRole::ParentA);

    assert_eq!(first, second);
    assert_eq!(
        first.provenance.canonical_digest(),
        second.provenance.canonical_digest()
    );
}

#[test]
fn parent_role_is_part_of_the_semantic_draw_context() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let profile = zero_profile(&schema, &map);
    let source = source(&schema, &map);

    let parent_a = derive(&schema, &map, &source, &profile, "gamete-0001", ParentRole::ParentA);
    let parent_b = derive(&schema, &map, &source, &profile, "gamete-0001", ParentRole::ParentB);

    for chromosome_id in [chromosome("chr-a"), chromosome("chr-b")] {
        assert_eq!(
            parent_a.provenance.segments()[&chromosome_id].source_haplotype_slot,
            0
        );
        assert_eq!(
            parent_b.provenance.segments()[&chromosome_id].source_haplotype_slot,
            1
        );
    }
    assert_ne!(parent_a.gamete, parent_b.gamete);
}

#[test]
fn identical_source_haplotypes_do_not_create_fake_homolog_identity() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let profile = zero_profile(&schema, &map);
    let mut identical_source = source(&schema, &map);
    identical_source.chromosomes.insert(
        chromosome("chr-a"),
        PhasedChromosomeState::new(
            chromosome("chr-a"),
            vec![hap(&["a0", "b0"]), hap(&["a0", "b0"])],
        ),
    );
    identical_source.validate(&schema, &map).unwrap();

    let parent_a = derive(
        &schema,
        &map,
        &identical_source,
        &profile,
        "gamete-0001",
        ParentRole::ParentA,
    );
    let parent_b = derive(
        &schema,
        &map,
        &identical_source,
        &profile,
        "gamete-0001",
        ParentRole::ParentB,
    );

    assert_eq!(
        parent_a.provenance.segments()[&chromosome("chr-a")].source_haplotype_slot,
        0
    );
    assert_eq!(
        parent_b.provenance.segments()[&chromosome("chr-a")].source_haplotype_slot,
        0
    );
    assert_eq!(
        parent_a.gamete.chromosomes[&chromosome("chr-a")],
        parent_b.gamete.chromosomes[&chromosome("chr-a")]
    );
}

#[test]
fn unrelated_chromosome_does_not_shift_existing_chromosome_choices() {
    let base_schema = schema();
    let base_map = chromosome_map(&base_schema);
    let base_profile = zero_profile(&base_schema, &base_map);
    let base_source = source(&base_schema, &base_map);
    let base = derive(
        &base_schema,
        &base_map,
        &base_source,
        &base_profile,
        "gamete-0002",
        ParentRole::ParentA,
    );

    let extended_schema = schema_with_loci(2, &["a", "b", "c", "d", "e", "f"]);
    let extended_map = ChromosomeMap::new(
        ChromosomeMapId::new("linked-gamete-map-v1-extended").unwrap(),
        &extended_schema,
        [
            ChromosomeDefinition::new(
                chromosome("chr-c"),
                vec![mapped_locus("e", 10_000), mapped_locus("f", 90_000)],
            )
            .unwrap(),
            ChromosomeDefinition::new(
                chromosome("chr-b"),
                vec![mapped_locus("c", 100_000), mapped_locus("d", 300_000)],
            )
            .unwrap(),
            ChromosomeDefinition::new(
                chromosome("chr-a"),
                vec![mapped_locus("a", 50_000), mapped_locus("b", 150_000)],
            )
            .unwrap(),
        ],
    )
    .unwrap();
    let extended_profile = ChromosomeRecombinationProfile::new(
        ChromosomeRecombinationProfileId::new("linked-gamete-recombination-v1-extended").unwrap(),
        &extended_schema,
        &extended_map,
        ChromosomeRecombinationModel::NoCrossoversIndependentAssortmentV1,
        [
            ChromosomeRecombinationDomain::new(chromosome("chr-c"), interval(0, 100_000))
                .unwrap(),
            ChromosomeRecombinationDomain::new(chromosome("chr-a"), interval(0, 250_000))
                .unwrap(),
            ChromosomeRecombinationDomain::new(chromosome("chr-b"), interval(0, 400_000))
                .unwrap(),
        ],
    )
    .unwrap();
    let extended_source = PhasedHereditaryState::new(
        &extended_schema,
        &extended_map,
        [
            PhasedChromosomeState::new(
                chromosome("chr-c"),
                vec![hap(&["e0", "f0"]), hap(&["e1", "f1"])],
            ),
            PhasedChromosomeState::new(
                chromosome("chr-a"),
                vec![hap(&["a0", "b0"]), hap(&["a1", "b1"])],
            ),
            PhasedChromosomeState::new(
                chromosome("chr-b"),
                vec![hap(&["c0", "d0"]), hap(&["c1", "d1"])],
            ),
        ],
    )
    .unwrap();
    let extended = derive(
        &extended_schema,
        &extended_map,
        &extended_source,
        &extended_profile,
        "gamete-0002",
        ParentRole::ParentA,
    );

    for chromosome_id in [chromosome("chr-a"), chromosome("chr-b")] {
        assert_eq!(
            base.provenance.segments()[&chromosome_id].source_haplotype_slot,
            extended.provenance.segments()[&chromosome_id].source_haplotype_slot
        );
        assert_eq!(
            base.gamete.chromosomes[&chromosome_id],
            extended.gamete.chromosomes[&chromosome_id]
        );
    }
}

#[test]
fn chromosome_constructor_order_does_not_change_derivation() {
    let schema = schema();
    let forward_map = chromosome_map(&schema);
    let reverse_map = ChromosomeMap::new(
        forward_map.id.clone(),
        &schema,
        [
            forward_map.chromosomes[&chromosome("chr-b")].clone(),
            forward_map.chromosomes[&chromosome("chr-a")].clone(),
        ],
    )
    .unwrap();
    assert_eq!(forward_map, reverse_map);

    let forward_profile = zero_profile(&schema, &forward_map);
    let reverse_profile = ChromosomeRecombinationProfile::new(
        forward_profile.id.clone(),
        &schema,
        &reverse_map,
        ChromosomeRecombinationModel::NoCrossoversIndependentAssortmentV1,
        [
            forward_profile.domains[&chromosome("chr-b")].clone(),
            forward_profile.domains[&chromosome("chr-a")].clone(),
        ],
    )
    .unwrap();
    let source = source(&schema, &forward_map);

    let forward = derive(
        &schema,
        &forward_map,
        &source,
        &forward_profile,
        "gamete-order",
        ParentRole::ParentA,
    );
    let reverse = derive(
        &schema,
        &reverse_map,
        &source,
        &reverse_profile,
        "gamete-order",
        ParentRole::ParentA,
    );
    assert_eq!(forward, reverse);
}

#[test]
fn poisson_profile_and_clonal_parent_role_are_rejected() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let source = source(&schema, &map);
    let poisson = profile_with_model(
        &schema,
        &map,
        ChromosomeRecombinationModel::PoissonCrossoversNoInterferenceV1,
    );
    let event = ReproductionEventId::new("gamete-reject").unwrap();

    assert_eq!(
        derive_zero_crossover_linked_gamete(
            &schema,
            &map,
            &source,
            &poisson,
            &event,
            ParentRole::ParentA,
        )
        .unwrap_err(),
        EvolutionError::LinkedGameteRecombinationModelMismatch
    );

    let zero = zero_profile(&schema, &map);
    assert_eq!(
        derive_zero_crossover_linked_gamete(
            &schema,
            &map,
            &source,
            &zero,
            &event,
            ParentRole::ClonalParent,
        )
        .unwrap_err(),
        EvolutionError::LinkedGameteRequiresSexualParentRole
    );
}

#[test]
fn non_diploid_schema_is_rejected_before_gamete_derivation() {
    let diploid_schema = schema();
    let diploid_map = chromosome_map(&diploid_schema);
    let diploid_profile = zero_profile(&diploid_schema, &diploid_map);

    let triploid_schema = schema_with_loci(3, &["a", "b", "c", "d"]);
    let triploid_map = chromosome_map(&triploid_schema);
    let triploid_source = PhasedHereditaryState::new(
        &triploid_schema,
        &triploid_map,
        [
            PhasedChromosomeState::new(
                chromosome("chr-a"),
                vec![
                    hap(&["a0", "b0"]),
                    hap(&["a0", "b1"]),
                    hap(&["a1", "b1"]),
                ],
            ),
            PhasedChromosomeState::new(
                chromosome("chr-b"),
                vec![
                    hap(&["c0", "d0"]),
                    hap(&["c0", "d1"]),
                    hap(&["c1", "d1"]),
                ],
            ),
        ],
    )
    .unwrap();

    assert!(matches!(
        derive_zero_crossover_linked_gamete(
            &triploid_schema,
            &triploid_map,
            &triploid_source,
            &diploid_profile,
            &ReproductionEventId::new("triploid-reject").unwrap(),
            ParentRole::ParentA,
        ),
        Err(EvolutionError::ModeRequiresDiploid(3))
    ));
}

#[test]
fn restored_derivation_requires_exact_external_event_context() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let profile = zero_profile(&schema, &map);
    let source = source(&schema, &map);
    let event = ReproductionEventId::new("gamete-0001").unwrap();
    let derived = derive_zero_crossover_linked_gamete(
        &schema,
        &map,
        &source,
        &profile,
        &event,
        ParentRole::ParentA,
    )
    .unwrap();

    let encoded = serde_json::to_string(&derived).unwrap();
    let restored: LinkedGameteDerivation = serde_json::from_str(&encoded).unwrap();
    restored
        .provenance
        .validate_current(
            &schema,
            &map,
            &source,
            &profile,
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
                &profile,
                &ReproductionEventId::new("different-event").unwrap(),
                ParentRole::ParentA,
                &restored.gamete,
            )
            .unwrap_err(),
        EvolutionError::LinkedGameteEventContextMismatch
    );
    assert_eq!(
        restored
            .provenance
            .validate_current(
                &schema,
                &map,
                &source,
                &profile,
                &event,
                ParentRole::ParentB,
                &restored.gamete,
            )
            .unwrap_err(),
        EvolutionError::LinkedGameteEventContextMismatch
    );
}

#[test]
fn phase_change_stales_source_even_when_unphased_genotype_is_identical() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let profile = zero_profile(&schema, &map);
    let source = source(&schema, &map);
    let event = ReproductionEventId::new("phase-stale").unwrap();
    let derived = derive_zero_crossover_linked_gamete(
        &schema,
        &map,
        &source,
        &profile,
        &event,
        ParentRole::ParentA,
    )
    .unwrap();

    let repulsion = PhasedHereditaryState::new(
        &schema,
        &map,
        [
            PhasedChromosomeState::new(
                chromosome("chr-a"),
                vec![hap(&["a0", "b1"]), hap(&["a1", "b0"])],
            ),
            source.chromosomes[&chromosome("chr-b")].clone(),
        ],
    )
    .unwrap();
    assert_eq!(
        source.to_unphased(&schema, &map).unwrap(),
        repulsion.to_unphased(&schema, &map).unwrap()
    );
    assert_eq!(
        derived
            .provenance
            .validate_current(
                &schema,
                &map,
                &repulsion,
                &profile,
                &event,
                ParentRole::ParentA,
                &derived.gamete,
            )
            .unwrap_err(),
        EvolutionError::LinkedGameteSourceAuthorityMismatch
    );
}

#[test]
fn changed_profile_stales_provenance_even_when_zero_crossover_choices_need_no_span() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let profile = zero_profile(&schema, &map);
    let source = source(&schema, &map);
    let event = ReproductionEventId::new("profile-stale").unwrap();
    let derived = derive_zero_crossover_linked_gamete(
        &schema,
        &map,
        &source,
        &profile,
        &event,
        ParentRole::ParentA,
    )
    .unwrap();

    let changed_profile = ChromosomeRecombinationProfile::new(
        profile.id.clone(),
        &schema,
        &map,
        ChromosomeRecombinationModel::NoCrossoversIndependentAssortmentV1,
        [
            ChromosomeRecombinationDomain::new(chromosome("chr-a"), interval(0, 260_000))
                .unwrap(),
            profile.domains[&chromosome("chr-b")].clone(),
        ],
    )
    .unwrap();

    assert_eq!(
        derived
            .provenance
            .validate_current(
                &schema,
                &map,
                &source,
                &changed_profile,
                &event,
                ParentRole::ParentA,
                &derived.gamete,
            )
            .unwrap_err(),
        EvolutionError::LinkedGameteSourceAuthorityMismatch
    );
}

#[test]
fn hostile_segment_and_gamete_tampering_fail_closed() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let profile = zero_profile(&schema, &map);
    let source = source(&schema, &map);
    let event = ReproductionEventId::new("gamete-0001").unwrap();
    let derived = derive_zero_crossover_linked_gamete(
        &schema,
        &map,
        &source,
        &profile,
        &event,
        ParentRole::ParentA,
    )
    .unwrap();

    let mut provenance_value = serde_json::to_value(&derived.provenance).unwrap();
    provenance_value["segments"]["chr-a"]["source_haplotype_slot"] = serde_json::json!(1);
    let tampered_provenance: LinkedGameteDerivationProvenance =
        serde_json::from_value(provenance_value).unwrap();
    assert_eq!(
        tampered_provenance
            .validate_current(
                &schema,
                &map,
                &source,
                &profile,
                &event,
                ParentRole::ParentA,
                &derived.gamete,
            )
            .unwrap_err(),
        EvolutionError::LinkedGameteDerivationMismatch
    );

    let mut tampered_gamete = derived.gamete.clone();
    tampered_gamete
        .chromosomes
        .get_mut(&chromosome("chr-a"))
        .unwrap()
        .alleles[0] = allele("a1");
    assert_eq!(
        derived
            .provenance
            .validate_current(
                &schema,
                &map,
                &source,
                &profile,
                &event,
                ParentRole::ParentA,
                &tampered_gamete,
            )
            .unwrap_err(),
        EvolutionError::LinkedGameteResultMismatch
    );
}
