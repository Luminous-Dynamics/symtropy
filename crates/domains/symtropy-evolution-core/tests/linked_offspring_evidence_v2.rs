use symtropy_evolution_core::{
    assemble_diploid_linked_offspring, assemble_diploid_linked_offspring_from_evidence,
    derive_marker_marginal_poisson_linked_gamete, derive_zero_crossover_linked_gamete, AlleleId,
    ChromosomeDefinition, ChromosomeHaplotype, ChromosomeId, ChromosomeLocus, ChromosomeMap,
    ChromosomeMapId, ChromosomeRecombinationDomain, ChromosomeRecombinationModel,
    ChromosomeRecombinationProfile, ChromosomeRecombinationProfileId,
    DiploidLinkedOffspringDerivationV2, EvolutionError, GeneticMapIntervalMicromorgans,
    GeneticMapPositionMicromorgans, HereditarySchema, HereditarySchemaId,
    LinkedGameteDerivationEvidence, LocusDefinition, LocusId, ParentRole, PhasedChromosomeState,
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
        HereditarySchemaId::new("offspring-evidence-v2-schema").unwrap(),
        2,
        [
            LocusDefinition::new(locus("a"), [allele("a0"), allele("a1")]).unwrap(),
            LocusDefinition::new(locus("b"), [allele("b0"), allele("b1")]).unwrap(),
            LocusDefinition::new(locus("c"), [allele("c0"), allele("c1")]).unwrap(),
        ],
    )
    .unwrap()
}

fn map(schema: &HereditarySchema, middle: u64) -> ChromosomeMap {
    ChromosomeMap::new(
        ChromosomeMapId::new("offspring-evidence-v2-map").unwrap(),
        schema,
        [ChromosomeDefinition::new(
            chromosome("chr-a"),
            vec![
                ChromosomeLocus::new(locus("a"), pos(1)),
                ChromosomeLocus::new(locus("b"), pos(middle)),
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

#[test]
fn zero_zero_v2_child_matches_frozen_v1_child_state() {
    let schema = schema();
    let map = map(&schema, 7_000_001);
    let zero = profile(
        &schema,
        &map,
        "zero-shared",
        ChromosomeRecombinationModel::NoCrossoversIndependentAssortmentV1,
    );
    let source_a = source(&schema, &map, &["a0", "b0", "c0"], &["a1", "b1", "c1"]);
    let source_b = source(&schema, &map, &["a0", "b1", "c0"], &["a1", "b0", "c1"]);
    let event = ReproductionEventId::new("offspring-v2-zero-zero").unwrap();

    let gamete_a = derive_zero_crossover_linked_gamete(
        &schema, &map, &source_a, &zero, &event, ParentRole::ParentA,
    )
    .unwrap();
    let gamete_b = derive_zero_crossover_linked_gamete(
        &schema, &map, &source_b, &zero, &event, ParentRole::ParentB,
    )
    .unwrap();

    let v1 = assemble_diploid_linked_offspring(
        &schema, &map, &zero, &source_a, &gamete_a, &source_b, &gamete_b, &event,
    )
    .unwrap();
    let v2 = assemble_diploid_linked_offspring_from_evidence(
        &schema,
        &map,
        &source_a,
        &zero,
        &LinkedGameteDerivationEvidence::ZeroCrossover(gamete_a),
        &source_b,
        &zero,
        &LinkedGameteDerivationEvidence::ZeroCrossover(gamete_b),
        &event,
    )
    .unwrap();

    assert_eq!(v1.child, v2.child);
}

#[test]
fn marker_marker_and_mixed_process_parents_assemble_under_one_event() {
    let schema = schema();
    let map = map(&schema, 7_000_001);
    let zero = profile(
        &schema,
        &map,
        "parent-a-zero",
        ChromosomeRecombinationModel::NoCrossoversIndependentAssortmentV1,
    );
    let poisson_a = profile(
        &schema,
        &map,
        "parent-a-poisson",
        ChromosomeRecombinationModel::PoissonCrossoversNoInterferenceV1,
    );
    let poisson_b = profile(
        &schema,
        &map,
        "parent-b-poisson",
        ChromosomeRecombinationModel::PoissonCrossoversNoInterferenceV1,
    );
    let source_a = source(&schema, &map, &["a0", "b0", "c0"], &["a1", "b1", "c1"]);
    let source_b = source(&schema, &map, &["a0", "b1", "c0"], &["a1", "b0", "c1"]);
    let event = ReproductionEventId::new("offspring-v2-mixed").unwrap();

    let zero_a = LinkedGameteDerivationEvidence::ZeroCrossover(
        derive_zero_crossover_linked_gamete(
            &schema, &map, &source_a, &zero, &event, ParentRole::ParentA,
        )
        .unwrap(),
    );
    let marker_a = LinkedGameteDerivationEvidence::MarkerMarginalPoisson(
        derive_marker_marginal_poisson_linked_gamete(
            &schema, &map, &source_a, &poisson_a, &event, ParentRole::ParentA,
        )
        .unwrap(),
    );
    let marker_b = LinkedGameteDerivationEvidence::MarkerMarginalPoisson(
        derive_marker_marginal_poisson_linked_gamete(
            &schema, &map, &source_b, &poisson_b, &event, ParentRole::ParentB,
        )
        .unwrap(),
    );

    let marker_marker = assemble_diploid_linked_offspring_from_evidence(
        &schema, &map, &source_a, &poisson_a, &marker_a, &source_b, &poisson_b, &marker_b, &event,
    )
    .unwrap();
    marker_marker.child.validate(&schema, &map).unwrap();

    let mixed = assemble_diploid_linked_offspring_from_evidence(
        &schema, &map, &source_a, &zero, &zero_a, &source_b, &poisson_b, &marker_b, &event,
    )
    .unwrap();
    mixed.child.validate(&schema, &map).unwrap();

    assert_eq!(mixed.provenance.parent_a().role, ParentRole::ParentA);
    assert_eq!(mixed.provenance.parent_b().role, ParentRole::ParentB);
    assert_ne!(
        mixed.provenance.parent_a().recombination_profile_digest,
        mixed.provenance.parent_b().recombination_profile_digest
    );
}

#[test]
fn wrong_process_profile_cannot_validate_an_evidence_variant() {
    let schema = schema();
    let map = map(&schema, 7_000_001);
    let zero = profile(
        &schema,
        &map,
        "zero-profile",
        ChromosomeRecombinationModel::NoCrossoversIndependentAssortmentV1,
    );
    let poisson = profile(
        &schema,
        &map,
        "poisson-profile",
        ChromosomeRecombinationModel::PoissonCrossoversNoInterferenceV1,
    );
    let source = source(&schema, &map, &["a0", "b0", "c0"], &["a1", "b1", "c1"]);
    let event = ReproductionEventId::new("wrong-profile").unwrap();
    let evidence = LinkedGameteDerivationEvidence::ZeroCrossover(
        derive_zero_crossover_linked_gamete(
            &schema, &map, &source, &zero, &event, ParentRole::ParentA,
        )
        .unwrap(),
    );

    assert!(evidence
        .validate_current(
            &schema, &map, &source, &poisson, &event, ParentRole::ParentA,
        )
        .is_err());
}

#[test]
fn same_gamete_state_can_have_distinct_derivation_evidence() {
    let schema = schema();
    let tiny_map = ChromosomeMap::new(
        ChromosomeMapId::new("same-gamete-map").unwrap(),
        &schema,
        [ChromosomeDefinition::new(
            chromosome("chr-a"),
            vec![
                ChromosomeLocus::new(locus("a"), pos(1)),
                ChromosomeLocus::new(locus("b"), pos(2)),
                ChromosomeLocus::new(locus("c"), pos(3)),
            ],
        )
        .unwrap()],
    )
    .unwrap();
    let zero = profile(
        &schema,
        &tiny_map,
        "same-gamete-zero",
        ChromosomeRecombinationModel::NoCrossoversIndependentAssortmentV1,
    );
    let poisson = profile(
        &schema,
        &tiny_map,
        "same-gamete-poisson",
        ChromosomeRecombinationModel::PoissonCrossoversNoInterferenceV1,
    );
    let source = source(
        &schema,
        &tiny_map,
        &["a0", "b0", "c0"],
        &["a1", "b1", "c1"],
    );
    let event = ReproductionEventId::new("parity-3").unwrap();
    let zero_evidence = LinkedGameteDerivationEvidence::ZeroCrossover(
        derive_zero_crossover_linked_gamete(
            &schema, &tiny_map, &source, &zero, &event, ParentRole::ParentA,
        )
        .unwrap(),
    );
    let marker_evidence = LinkedGameteDerivationEvidence::MarkerMarginalPoisson(
        derive_marker_marginal_poisson_linked_gamete(
            &schema, &tiny_map, &source, &poisson, &event, ParentRole::ParentA,
        )
        .unwrap(),
    );

    assert_eq!(zero_evidence.gamete(), marker_evidence.gamete());
    assert_ne!(zero_evidence.canonical_digest(), marker_evidence.canonical_digest());
}

#[test]
fn restored_generalized_child_requires_full_current_revalidation() {
    let schema = schema();
    let map = map(&schema, 7_000_001);
    let profile_a = profile(
        &schema,
        &map,
        "restore-zero",
        ChromosomeRecombinationModel::NoCrossoversIndependentAssortmentV1,
    );
    let profile_b = profile(
        &schema,
        &map,
        "restore-poisson",
        ChromosomeRecombinationModel::PoissonCrossoversNoInterferenceV1,
    );
    let source_a = source(&schema, &map, &["a0", "b0", "c0"], &["a1", "b1", "c1"]);
    let source_b = source(&schema, &map, &["a0", "b1", "c0"], &["a1", "b0", "c1"]);
    let event = ReproductionEventId::new("restore-child-v2").unwrap();
    let evidence_a = LinkedGameteDerivationEvidence::ZeroCrossover(
        derive_zero_crossover_linked_gamete(
            &schema, &map, &source_a, &profile_a, &event, ParentRole::ParentA,
        )
        .unwrap(),
    );
    let evidence_b = LinkedGameteDerivationEvidence::MarkerMarginalPoisson(
        derive_marker_marginal_poisson_linked_gamete(
            &schema, &map, &source_b, &profile_b, &event, ParentRole::ParentB,
        )
        .unwrap(),
    );
    let derived = assemble_diploid_linked_offspring_from_evidence(
        &schema, &map, &source_a, &profile_a, &evidence_a, &source_b, &profile_b, &evidence_b, &event,
    )
    .unwrap();

    let encoded = serde_json::to_string(&derived).unwrap();
    let restored: DiploidLinkedOffspringDerivationV2 = serde_json::from_str(&encoded).unwrap();
    restored
        .provenance
        .validate_current(
            &schema,
            &map,
            &source_a,
            &profile_a,
            &evidence_a,
            &source_b,
            &profile_b,
            &evidence_b,
            &event,
            &restored.child,
        )
        .unwrap();

    let mut tampered = restored.child.clone();
    tampered
        .chromosomes
        .get_mut(&chromosome("chr-a"))
        .unwrap()
        .haplotypes[0]
        .alleles[0] = allele("a1");
    assert_eq!(
        restored
            .provenance
            .validate_current(
                &schema,
                &map,
                &source_a,
                &profile_a,
                &evidence_a,
                &source_b,
                &profile_b,
                &evidence_b,
                &event,
                &tampered,
            )
            .unwrap_err(),
        EvolutionError::LinkedOffspringResultMismatch
    );
}
