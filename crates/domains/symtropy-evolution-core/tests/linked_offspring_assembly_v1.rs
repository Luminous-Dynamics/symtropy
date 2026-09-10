use std::collections::BTreeMap;
use symtropy_evolution_core::{
    assemble_diploid_linked_offspring, derive_zero_crossover_linked_gamete, AlleleId,
    ChromosomeDefinition, ChromosomeHaplotype, ChromosomeId, ChromosomeLocus, ChromosomeMap,
    ChromosomeMapId, ChromosomeRecombinationDomain, ChromosomeRecombinationModel,
    ChromosomeRecombinationProfile, ChromosomeRecombinationProfileId,
    DiploidLinkedOffspringDerivation, EvolutionError, GeneticMapIntervalMicromorgans,
    GeneticMapPositionMicromorgans, HereditarySchema, HereditarySchemaId, HereditaryState,
    LinkedGameteDerivation, LocusDefinition, LocusId, ParentRole, PhasedChromosomeState,
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

fn schema() -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new("linked-offspring-schema-v1").unwrap(),
        2,
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
        ChromosomeMapId::new("linked-offspring-map-v1").unwrap(),
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

fn profile(schema: &HereditarySchema, map: &ChromosomeMap) -> ChromosomeRecombinationProfile {
    ChromosomeRecombinationProfile::new(
        ChromosomeRecombinationProfileId::new("linked-offspring-profile-v1").unwrap(),
        schema,
        map,
        ChromosomeRecombinationModel::NoCrossoversIndependentAssortmentV1,
        [
            ChromosomeRecombinationDomain::new(chromosome("chr-a"), interval(0, 250_000))
                .unwrap(),
            ChromosomeRecombinationDomain::new(chromosome("chr-b"), interval(0, 400_000))
                .unwrap(),
        ],
    )
    .unwrap()
}

fn hap(ids: &[&str]) -> ChromosomeHaplotype {
    ChromosomeHaplotype::new(ids.iter().map(|id| allele(id)).collect())
}

fn source(
    schema: &HereditarySchema,
    map: &ChromosomeMap,
    a: [&[&str]; 2],
    b: [&[&str]; 2],
) -> PhasedHereditaryState {
    PhasedHereditaryState::new(
        schema,
        map,
        [
            PhasedChromosomeState::new(
                chromosome("chr-a"),
                vec![hap(a[0]), hap(a[1])],
            ),
            PhasedChromosomeState::new(
                chromosome("chr-b"),
                vec![hap(b[0]), hap(b[1])],
            ),
        ],
    )
    .unwrap()
}

fn parent_a_source(schema: &HereditarySchema, map: &ChromosomeMap) -> PhasedHereditaryState {
    source(
        schema,
        map,
        [&["a0", "b0"], &["a1", "b1"]],
        [&["c0", "d0"], &["c1", "d1"]],
    )
}

fn parent_b_source(schema: &HereditarySchema, map: &ChromosomeMap) -> PhasedHereditaryState {
    source(
        schema,
        map,
        [&["a0", "b1"], &["a1", "b0"]],
        [&["c0", "d1"], &["c1", "d0"]],
    )
}

fn gamete(
    schema: &HereditarySchema,
    map: &ChromosomeMap,
    profile: &ChromosomeRecombinationProfile,
    source: &PhasedHereditaryState,
    event: &ReproductionEventId,
    role: ParentRole,
) -> LinkedGameteDerivation {
    derive_zero_crossover_linked_gamete(schema, map, source, profile, event, role).unwrap()
}

#[test]
fn linked_gametes_assemble_into_exact_diploid_phased_child() {
    let schema = schema();
    let map = map(&schema);
    let profile = profile(&schema, &map);
    let source_a = parent_a_source(&schema, &map);
    let source_b = parent_b_source(&schema, &map);
    let event = ReproductionEventId::new("offspring-0001").unwrap();
    let gamete_a = gamete(&schema, &map, &profile, &source_a, &event, ParentRole::ParentA);
    let gamete_b = gamete(&schema, &map, &profile, &source_b, &event, ParentRole::ParentB);

    let derived = assemble_diploid_linked_offspring(
        &schema,
        &map,
        &profile,
        &source_a,
        &gamete_a,
        &source_b,
        &gamete_b,
        &event,
    )
    .unwrap();

    for chromosome_id in map.chromosomes.keys() {
        let expected = PhasedChromosomeState::new(
            chromosome_id.clone(),
            vec![
                gamete_a.gamete.chromosomes[chromosome_id].clone(),
                gamete_b.gamete.chromosomes[chromosome_id].clone(),
            ],
        );
        assert_eq!(derived.child.chromosomes[chromosome_id], expected);
    }

    assert_eq!(derived.provenance.parent_a().role, ParentRole::ParentA);
    assert_eq!(derived.provenance.parent_b().role, ParentRole::ParentB);
    derived
        .provenance
        .validate_current(
            &schema,
            &map,
            &profile,
            &source_a,
            &gamete_a,
            &source_b,
            &gamete_b,
            &event,
            &derived.child,
        )
        .unwrap();
}

#[test]
fn child_unphased_projection_is_exact_union_of_parental_gamete_alleles() {
    let schema = schema();
    let map = map(&schema);
    let profile = profile(&schema, &map);
    let source_a = parent_a_source(&schema, &map);
    let source_b = parent_b_source(&schema, &map);
    let event = ReproductionEventId::new("offspring-union").unwrap();
    let gamete_a = gamete(&schema, &map, &profile, &source_a, &event, ParentRole::ParentA);
    let gamete_b = gamete(&schema, &map, &profile, &source_b, &event, ParentRole::ParentB);
    let derived = assemble_diploid_linked_offspring(
        &schema,
        &map,
        &profile,
        &source_a,
        &gamete_a,
        &source_b,
        &gamete_b,
        &event,
    )
    .unwrap();

    let mut expected_copies = BTreeMap::new();
    for (chromosome_id, definition) in &map.chromosomes {
        let from_a = &gamete_a.gamete.chromosomes[chromosome_id];
        let from_b = &gamete_b.gamete.chromosomes[chromosome_id];
        for (index, mapped_locus) in definition.loci.iter().enumerate() {
            expected_copies.insert(
                mapped_locus.locus_id.clone(),
                vec![from_a.alleles[index].clone(), from_b.alleles[index].clone()],
            );
        }
    }
    let expected = HereditaryState::new(&schema, expected_copies).unwrap();
    assert_eq!(derived.child.to_unphased(&schema, &map).unwrap(), expected);
}

#[test]
fn parent_role_order_is_not_encoded_into_child_homolog_row_order() {
    let schema = schema();
    let map = map(&schema);
    let profile = profile(&schema, &map);
    let source_a = source(
        &schema,
        &map,
        [&["a1", "b1"], &["a1", "b1"]],
        [&["c1", "d1"], &["c1", "d1"]],
    );
    let source_b = source(
        &schema,
        &map,
        [&["a0", "b0"], &["a0", "b0"]],
        [&["c0", "d0"], &["c0", "d0"]],
    );
    let event = ReproductionEventId::new("offspring-order").unwrap();
    let gamete_a = gamete(&schema, &map, &profile, &source_a, &event, ParentRole::ParentA);
    let gamete_b = gamete(&schema, &map, &profile, &source_b, &event, ParentRole::ParentB);
    let derived = assemble_diploid_linked_offspring(
        &schema,
        &map,
        &profile,
        &source_a,
        &gamete_a,
        &source_b,
        &gamete_b,
        &event,
    )
    .unwrap();

    assert_eq!(
        derived.child.chromosomes[&chromosome("chr-a")].haplotypes,
        vec![hap(&["a0", "b0"]), hap(&["a1", "b1"])]
    );
    assert_eq!(derived.provenance.parent_a().gamete_digest, gamete_a.gamete.canonical_digest(&schema, &map).unwrap());
    assert_eq!(derived.provenance.parent_b().gamete_digest, gamete_b.gamete.canonical_digest(&schema, &map).unwrap());
}

#[test]
fn identical_gamete_content_can_share_child_state_while_parentage_evidence_remains_distinct() {
    let schema = schema();
    let map = map(&schema);
    let profile = profile(&schema, &map);
    let common = source(
        &schema,
        &map,
        [&["a0", "b0"], &["a0", "b0"]],
        [&["c0", "d0"], &["c0", "d0"]],
    );
    let event = ReproductionEventId::new("offspring-identical").unwrap();
    let gamete_a = gamete(&schema, &map, &profile, &common, &event, ParentRole::ParentA);
    let gamete_b = gamete(&schema, &map, &profile, &common, &event, ParentRole::ParentB);
    assert_eq!(gamete_a.gamete, gamete_b.gamete);
    assert_ne!(
        gamete_a.provenance.canonical_digest(),
        gamete_b.provenance.canonical_digest()
    );

    let derived = assemble_diploid_linked_offspring(
        &schema,
        &map,
        &profile,
        &common,
        &gamete_a,
        &common,
        &gamete_b,
        &event,
    )
    .unwrap();
    assert_eq!(derived.provenance.parent_a().role, ParentRole::ParentA);
    assert_eq!(derived.provenance.parent_b().role, ParentRole::ParentB);
    assert_ne!(
        derived.provenance.parent_a().gamete_derivation_digest,
        derived.provenance.parent_b().gamete_derivation_digest
    );
}

#[test]
fn mismatched_event_or_wrong_role_derivation_is_rejected() {
    let schema = schema();
    let map = map(&schema);
    let profile = profile(&schema, &map);
    let source_a = parent_a_source(&schema, &map);
    let source_b = parent_b_source(&schema, &map);
    let event = ReproductionEventId::new("offspring-context").unwrap();
    let gamete_a = gamete(&schema, &map, &profile, &source_a, &event, ParentRole::ParentA);
    let gamete_b = gamete(&schema, &map, &profile, &source_b, &event, ParentRole::ParentB);

    assert_eq!(
        assemble_diploid_linked_offspring(
            &schema,
            &map,
            &profile,
            &source_a,
            &gamete_a,
            &source_b,
            &gamete_b,
            &ReproductionEventId::new("other-event").unwrap(),
        )
        .unwrap_err(),
        EvolutionError::LinkedGameteEventContextMismatch
    );

    assert_eq!(
        assemble_diploid_linked_offspring(
            &schema,
            &map,
            &profile,
            &source_b,
            &gamete_b,
            &source_a,
            &gamete_a,
            &event,
        )
        .unwrap_err(),
        EvolutionError::LinkedGameteEventContextMismatch
    );
}

#[test]
fn restored_child_receipt_requires_full_current_revalidation() {
    let schema = schema();
    let map = map(&schema);
    let profile = profile(&schema, &map);
    let source_a = parent_a_source(&schema, &map);
    let source_b = parent_b_source(&schema, &map);
    let event = ReproductionEventId::new("offspring-restore").unwrap();
    let gamete_a = gamete(&schema, &map, &profile, &source_a, &event, ParentRole::ParentA);
    let gamete_b = gamete(&schema, &map, &profile, &source_b, &event, ParentRole::ParentB);
    let derived = assemble_diploid_linked_offspring(
        &schema,
        &map,
        &profile,
        &source_a,
        &gamete_a,
        &source_b,
        &gamete_b,
        &event,
    )
    .unwrap();

    let encoded = serde_json::to_string(&derived).unwrap();
    let restored: DiploidLinkedOffspringDerivation = serde_json::from_str(&encoded).unwrap();
    restored
        .provenance
        .validate_current(
            &schema,
            &map,
            &profile,
            &source_a,
            &gamete_a,
            &source_b,
            &gamete_b,
            &event,
            &restored.child,
        )
        .unwrap();

    let mut tampered_child = restored.child.clone();
    tampered_child
        .chromosomes
        .get_mut(&chromosome("chr-a"))
        .unwrap()
        .haplotypes[0]
        .alleles[0] = allele("a1");
    assert!(restored
        .provenance
        .validate_current(
            &schema,
            &map,
            &profile,
            &source_a,
            &gamete_a,
            &source_b,
            &gamete_b,
            &event,
            &tampered_child,
        )
        .is_err());
}
