use symtropy_evolution_core::{
    assemble_diploid_linked_offspring_from_evidence,
    derive_descendant_ancestry, derive_marker_marginal_poisson_linked_gamete,
    derive_modeled_gamete_ancestry, derive_zero_crossover_linked_gamete, AlleleId,
    AncestryCopyId, ChromosomeAncestryState, ChromosomeDefinition, ChromosomeHaplotype,
    ChromosomeId, ChromosomeLocus, ChromosomeMap, ChromosomeMapId,
    ChromosomeRecombinationDomain, ChromosomeRecombinationModel,
    ChromosomeRecombinationProfile, ChromosomeRecombinationProfileId,
    GeneticMapIntervalMicromorgans, GeneticMapPositionMicromorgans,
    HaplotypeAncestryClass, HereditarySchema, HereditarySchemaId,
    LinkedGameteDerivationEvidence, LocusDefinition, LocusId, ParentRole,
    PhasedAncestryState, PhasedChromosomeState, PhasedHereditaryState,
    ReproductionEventId,
};
use std::collections::BTreeSet;

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
        HereditarySchemaId::new("phylo-04c-schema").unwrap(),
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
        ChromosomeMapId::new("phylo-04c-map").unwrap(),
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
    first: &str,
    second: &str,
) -> PhasedAncestryState {
    PhasedAncestryState::new(
        schema,
        map,
        source,
        [ChromosomeAncestryState::new(
            chromosome("chr-a"),
            vec![class(0, &[first]), class(1, &[second])],
        )
        .unwrap()],
    )
    .unwrap()
}

fn identical_ancestry(
    schema: &HereditarySchema,
    map: &ChromosomeMap,
    source: &PhasedHereditaryState,
    first: &str,
    second: &str,
) -> PhasedAncestryState {
    PhasedAncestryState::new(
        schema,
        map,
        source,
        [ChromosomeAncestryState::new(
            chromosome("chr-a"),
            vec![class(0, &[first, second])],
        )
        .unwrap()],
    )
    .unwrap()
}

#[test]
fn recombinant_parent_sources_flow_into_one_new_child_copy() {
    let schema = schema();
    let map = map(&schema);
    let event = ReproductionEventId::new("parity-8").unwrap();

    let source_a = source(
        &schema,
        &map,
        &["a0", "b0", "c0"],
        &["a1", "b1", "c1"],
    );
    let source_b = source(
        &schema,
        &map,
        &["a0", "b1", "c0"],
        &["a1", "b0", "c1"],
    );
    let ancestry_a = distinct_ancestry(&schema, &map, &source_a, "a-copy-0", "a-copy-1");
    let ancestry_b = distinct_ancestry(&schema, &map, &source_b, "b-copy-0", "b-copy-1");

    let profile_a = profile(
        &schema,
        &map,
        "parent-a-poisson",
        ChromosomeRecombinationModel::PoissonCrossoversNoInterferenceV1,
    );
    let profile_b = profile(
        &schema,
        &map,
        "parent-b-zero",
        ChromosomeRecombinationModel::NoCrossoversIndependentAssortmentV1,
    );

    let gamete_a = LinkedGameteDerivationEvidence::MarkerMarginalPoisson(
        derive_marker_marginal_poisson_linked_gamete(
            &schema,
            &map,
            &source_a,
            &profile_a,
            &event,
            ParentRole::ParentA,
        )
        .unwrap(),
    );
    let gamete_b = LinkedGameteDerivationEvidence::ZeroCrossover(
        derive_zero_crossover_linked_gamete(
            &schema,
            &map,
            &source_b,
            &profile_b,
            &event,
            ParentRole::ParentB,
        )
        .unwrap(),
    );

    let gamete_ancestry_a = derive_modeled_gamete_ancestry(
        &schema,
        &map,
        &source_a,
        &ancestry_a,
        &profile_a,
        &gamete_a,
        &event,
        ParentRole::ParentA,
    )
    .unwrap();
    let gamete_ancestry_b = derive_modeled_gamete_ancestry(
        &schema,
        &map,
        &source_b,
        &ancestry_b,
        &profile_b,
        &gamete_b,
        &event,
        ParentRole::ParentB,
    )
    .unwrap();

    let parent_a_origins = &gamete_ancestry_a.ancestry.chromosomes[&chromosome("chr-a")].loci;
    assert_eq!(parent_a_origins.len(), 3);
    assert_eq!(parent_a_origins[0].source_copy_id, parent_a_origins[2].source_copy_id);
    assert_ne!(parent_a_origins[0].source_copy_id, parent_a_origins[1].source_copy_id);

    let offspring = assemble_diploid_linked_offspring_from_evidence(
        &schema,
        &map,
        &source_a,
        &profile_a,
        &gamete_a,
        &source_b,
        &profile_b,
        &gamete_b,
        &event,
    )
    .unwrap();

    let derived = derive_descendant_ancestry(
        &schema,
        &map,
        &source_a,
        &ancestry_a,
        &profile_a,
        &gamete_a,
        &gamete_ancestry_a,
        &source_b,
        &ancestry_b,
        &profile_b,
        &gamete_b,
        &gamete_ancestry_b,
        &offspring,
        &event,
    )
    .unwrap();

    assert_eq!(derived.materialization.descendant_copies.len(), 2);
    assert_eq!(derived.materialization.edges.len(), 6);

    let child_a = &derived.materialization.descendant_copies[0];
    let child_b = &derived.materialization.descendant_copies[1];
    assert_eq!(child_a.parent_role, ParentRole::ParentA);
    assert_eq!(child_b.parent_role, ParentRole::ParentB);
    assert_ne!(child_a.child_copy_id, child_b.child_copy_id);

    let parent_ids: BTreeSet<_> = derived
        .materialization
        .edges
        .iter()
        .map(|edge| edge.source_copy_id.clone())
        .collect();
    assert!(!parent_ids.contains(&child_a.child_copy_id));
    assert!(!parent_ids.contains(&child_b.child_copy_id));

    let edges_a: Vec<_> = derived
        .materialization
        .edges
        .iter()
        .filter(|edge| edge.parent_role == ParentRole::ParentA)
        .collect();
    assert_eq!(edges_a.len(), 3);
    assert!(edges_a
        .iter()
        .all(|edge| edge.child_copy_id == child_a.child_copy_id));
    assert_eq!(edges_a[0].source_copy_id, edges_a[2].source_copy_id);
    assert_ne!(edges_a[0].source_copy_id, edges_a[1].source_copy_id);

    let edges_b: Vec<_> = derived
        .materialization
        .edges
        .iter()
        .filter(|edge| edge.parent_role == ParentRole::ParentB)
        .collect();
    assert_eq!(edges_b.len(), 3);
    assert!(edges_b
        .iter()
        .all(|edge| edge.child_copy_id == child_b.child_copy_id));

    derived
        .provenance
        .validate_current(
            &schema,
            &map,
            &source_a,
            &ancestry_a,
            &profile_a,
            &gamete_a,
            &gamete_ancestry_a,
            &source_b,
            &ancestry_b,
            &profile_b,
            &gamete_b,
            &gamete_ancestry_b,
            &offspring,
            &event,
            &derived.child_ancestry,
            &derived.materialization,
        )
        .unwrap();

    let encoded = serde_json::to_string(&derived).unwrap();
    let restored: symtropy_evolution_core::DescendantAncestryDerivation =
        serde_json::from_str(&encoded).unwrap();
    restored
        .provenance
        .validate_current(
            &schema,
            &map,
            &source_a,
            &ancestry_a,
            &profile_a,
            &gamete_a,
            &gamete_ancestry_a,
            &source_b,
            &ancestry_b,
            &profile_b,
            &gamete_b,
            &gamete_ancestry_b,
            &offspring,
            &event,
            &restored.child_ancestry,
            &restored.materialization,
        )
        .unwrap();
    assert_eq!(restored, derived);
}

#[test]
fn identical_child_haplotype_content_keeps_two_descendant_identities_in_one_class() {
    let schema = schema();
    let map = map(&schema);
    let event = ReproductionEventId::new("identical-child-event").unwrap();
    let zero_a = profile(
        &schema,
        &map,
        "identical-a-zero",
        ChromosomeRecombinationModel::NoCrossoversIndependentAssortmentV1,
    );
    let zero_b = profile(
        &schema,
        &map,
        "identical-b-zero",
        ChromosomeRecombinationModel::NoCrossoversIndependentAssortmentV1,
    );

    let source_a = source(
        &schema,
        &map,
        &["a0", "b0", "c0"],
        &["a0", "b0", "c0"],
    );
    let source_b = source(
        &schema,
        &map,
        &["a0", "b0", "c0"],
        &["a0", "b0", "c0"],
    );
    let ancestry_a = identical_ancestry(&schema, &map, &source_a, "a-old-0", "a-old-1");
    let ancestry_b = identical_ancestry(&schema, &map, &source_b, "b-old-0", "b-old-1");

    let gamete_a = LinkedGameteDerivationEvidence::ZeroCrossover(
        derive_zero_crossover_linked_gamete(
            &schema,
            &map,
            &source_a,
            &zero_a,
            &event,
            ParentRole::ParentA,
        )
        .unwrap(),
    );
    let gamete_b = LinkedGameteDerivationEvidence::ZeroCrossover(
        derive_zero_crossover_linked_gamete(
            &schema,
            &map,
            &source_b,
            &zero_b,
            &event,
            ParentRole::ParentB,
        )
        .unwrap(),
    );
    assert_eq!(gamete_a.gamete(), gamete_b.gamete());

    let gamete_ancestry_a = derive_modeled_gamete_ancestry(
        &schema,
        &map,
        &source_a,
        &ancestry_a,
        &zero_a,
        &gamete_a,
        &event,
        ParentRole::ParentA,
    )
    .unwrap();
    let gamete_ancestry_b = derive_modeled_gamete_ancestry(
        &schema,
        &map,
        &source_b,
        &ancestry_b,
        &zero_b,
        &gamete_b,
        &event,
        ParentRole::ParentB,
    )
    .unwrap();

    let offspring = assemble_diploid_linked_offspring_from_evidence(
        &schema,
        &map,
        &source_a,
        &zero_a,
        &gamete_a,
        &source_b,
        &zero_b,
        &gamete_b,
        &event,
    )
    .unwrap();
    let derived = derive_descendant_ancestry(
        &schema,
        &map,
        &source_a,
        &ancestry_a,
        &zero_a,
        &gamete_a,
        &gamete_ancestry_a,
        &source_b,
        &ancestry_b,
        &zero_b,
        &gamete_b,
        &gamete_ancestry_b,
        &offspring,
        &event,
    )
    .unwrap();

    let chromosome_state = &derived.child_ancestry.chromosomes[&chromosome("chr-a")];
    assert_eq!(chromosome_state.classes.len(), 1);
    assert_eq!(chromosome_state.classes[0].copy_ids.len(), 2);
    assert_ne!(
        chromosome_state.classes[0].copy_ids[0],
        chromosome_state.classes[0].copy_ids[1]
    );

    let role_ids: BTreeSet<_> = derived
        .materialization
        .descendant_copies
        .iter()
        .map(|copy| (copy.parent_role, copy.child_copy_id.clone()))
        .collect();
    assert_eq!(role_ids.len(), 2);

    let wrong_event = ReproductionEventId::new("other-child-event").unwrap();
    assert!(derived
        .provenance
        .validate_current(
            &schema,
            &map,
            &source_a,
            &ancestry_a,
            &zero_a,
            &gamete_a,
            &gamete_ancestry_a,
            &source_b,
            &ancestry_b,
            &zero_b,
            &gamete_b,
            &gamete_ancestry_b,
            &offspring,
            &wrong_event,
            &derived.child_ancestry,
            &derived.materialization,
        )
        .is_err());
}
