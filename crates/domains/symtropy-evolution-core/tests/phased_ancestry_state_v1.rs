use symtropy_evolution_core::{
    AlleleId, AncestryAuthorityError, AncestryCopyId, ChromosomeAncestryState,
    ChromosomeDefinition, ChromosomeHaplotype, ChromosomeId, ChromosomeLocus, ChromosomeMap,
    ChromosomeMapId, GeneticMapPositionMicromorgans, HaplotypeAncestryClass, HereditarySchema,
    HereditarySchemaId, LocusDefinition, LocusId, PhasedAncestryState, PhasedChromosomeState,
    PhasedHereditaryState,
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

fn hap(ids: &[&str]) -> ChromosomeHaplotype {
    ChromosomeHaplotype::new(ids.iter().map(|id| allele(id)).collect())
}

fn schema() -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new("phylo-04a-schema").unwrap(),
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
        ChromosomeMapId::new("phylo-04a-map").unwrap(),
        schema,
        [ChromosomeDefinition::new(
            chromosome("chr-a"),
            vec![
                ChromosomeLocus::new(locus("a"), pos(1)),
                ChromosomeLocus::new(locus("b"), pos(1_000_001)),
                ChromosomeLocus::new(locus("c"), pos(2_000_001)),
            ],
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

fn chromosome_ancestry(classes: Vec<HaplotypeAncestryClass>) -> ChromosomeAncestryState {
    ChromosomeAncestryState::new(chromosome("chr-a"), classes).unwrap()
}

#[test]
fn distinct_haplotype_classes_are_exact_and_order_stable() {
    let schema = schema();
    let map = map(&schema);
    let source_state = source(
        &schema,
        &map,
        &["a0", "b0", "c0"],
        &["a1", "b1", "c1"],
    );

    let a = PhasedAncestryState::new(
        &schema,
        &map,
        &source_state,
        [chromosome_ancestry(vec![
            class(1, &["copy-z"]),
            class(0, &["copy-a"]),
        ])],
    )
    .unwrap();
    let b = PhasedAncestryState::new(
        &schema,
        &map,
        &source_state,
        [chromosome_ancestry(vec![
            class(0, &["copy-a"]),
            class(1, &["copy-z"]),
        ])],
    )
    .unwrap();

    assert_eq!(a, b);
    assert_eq!(
        a.canonical_digest(&schema, &map, &source_state).unwrap(),
        b.canonical_digest(&schema, &map, &source_state).unwrap()
    );
    assert_eq!(
        a.copy_ids_for_haplotype_content_at_slot(
            &schema,
            &map,
            &source_state,
            &chromosome("chr-a"),
            0,
        )
        .unwrap(),
        &[ancestry("copy-a")]
    );
    assert_eq!(
        a.copy_ids_for_haplotype_content_at_slot(
            &schema,
            &map,
            &source_state,
            &chromosome("chr-a"),
            1,
        )
        .unwrap(),
        &[ancestry("copy-z")]
    );
}

#[test]
fn identical_genetic_rows_preserve_multiple_ancestry_copies_without_row_assignment() {
    let schema = schema();
    let map = map(&schema);
    let source_state = source(
        &schema,
        &map,
        &["a0", "b0", "c0"],
        &["a0", "b0", "c0"],
    );
    let ancestry_state = PhasedAncestryState::new(
        &schema,
        &map,
        &source_state,
        [chromosome_ancestry(vec![class(
            0,
            &["maternal-17", "paternal-42"],
        )])],
    )
    .unwrap();

    let at_zero = ancestry_state
        .copy_ids_for_haplotype_content_at_slot(
            &schema,
            &map,
            &source_state,
            &chromosome("chr-a"),
            0,
        )
        .unwrap();
    let at_one = ancestry_state
        .copy_ids_for_haplotype_content_at_slot(
            &schema,
            &map,
            &source_state,
            &chromosome("chr-a"),
            1,
        )
        .unwrap();

    assert_eq!(at_zero, at_one);
    assert_eq!(
        at_zero,
        &[ancestry("maternal-17"), ancestry("paternal-42")]
    );
}

#[test]
fn identical_genetic_rows_cannot_be_split_into_fake_row_ancestry_classes() {
    let schema = schema();
    let map = map(&schema);
    let source_state = source(
        &schema,
        &map,
        &["a0", "b0", "c0"],
        &["a0", "b0", "c0"],
    );

    let result = PhasedAncestryState::new(
        &schema,
        &map,
        &source_state,
        [chromosome_ancestry(vec![
            class(0, &["copy-a"]),
            class(1, &["copy-b"]),
        ])],
    );

    assert!(matches!(
        result,
        Err(AncestryAuthorityError::ContentClassMismatch { .. })
    ));
}

#[test]
fn content_class_multiplicity_and_global_copy_identity_fail_closed() {
    let schema = schema();
    let map = map(&schema);

    let identical = source(
        &schema,
        &map,
        &["a0", "b0", "c0"],
        &["a0", "b0", "c0"],
    );
    let too_few = PhasedAncestryState::new(
        &schema,
        &map,
        &identical,
        [chromosome_ancestry(vec![class(0, &["only-one"])])],
    );
    assert!(matches!(
        too_few,
        Err(AncestryAuthorityError::ClassMultiplicityMismatch {
            expected: 2,
            observed: 1,
            ..
        })
    ));

    let distinct = source(
        &schema,
        &map,
        &["a0", "b0", "c0"],
        &["a1", "b1", "c1"],
    );
    let duplicate = PhasedAncestryState::new(
        &schema,
        &map,
        &distinct,
        [chromosome_ancestry(vec![
            class(0, &["same-copy"]),
            class(1, &["same-copy"]),
        ])],
    );
    assert!(matches!(
        duplicate,
        Err(AncestryAuthorityError::DuplicateAncestryCopyIdentity(_))
    ));
}

#[test]
fn restored_noncanonical_evidence_does_not_regain_authority() {
    let schema = schema();
    let map = map(&schema);
    let source_state = source(
        &schema,
        &map,
        &["a0", "b0", "c0"],
        &["a0", "b0", "c0"],
    );
    let authoritative = PhasedAncestryState::new(
        &schema,
        &map,
        &source_state,
        [chromosome_ancestry(vec![class(0, &["copy-a", "copy-b"])])],
    )
    .unwrap();

    let encoded = serde_json::to_string(&authoritative).unwrap();
    let mut restored: PhasedAncestryState = serde_json::from_str(&encoded).unwrap();
    restored
        .validate_current(&schema, &map, &source_state)
        .unwrap();

    restored
        .chromosomes
        .get_mut(&chromosome("chr-a"))
        .unwrap()
        .classes[0]
        .copy_ids
        .reverse();
    assert!(matches!(
        restored.validate_current(&schema, &map, &source_state),
        Err(AncestryAuthorityError::NonCanonicalCopyIdOrder { .. })
    ));
}

#[test]
fn exact_genetic_state_binding_rejects_stale_sidecar() {
    let schema = schema();
    let map = map(&schema);
    let source_state = source(
        &schema,
        &map,
        &["a0", "b0", "c0"],
        &["a1", "b1", "c1"],
    );
    let ancestry_state = PhasedAncestryState::new(
        &schema,
        &map,
        &source_state,
        [chromosome_ancestry(vec![
            class(0, &["copy-a"]),
            class(1, &["copy-b"]),
        ])],
    )
    .unwrap();

    let changed = source(
        &schema,
        &map,
        &["a0", "b0", "c0"],
        &["a1", "b0", "c1"],
    );
    assert!(matches!(
        ancestry_state.validate_current(&schema, &map, &changed),
        Err(AncestryAuthorityError::CurrentAuthorityMismatch)
    ));
}

#[test]
fn class_lookup_rejects_out_of_range_slots() {
    let schema = schema();
    let map = map(&schema);
    let source_state = source(
        &schema,
        &map,
        &["a0", "b0", "c0"],
        &["a1", "b1", "c1"],
    );
    let ancestry_state = PhasedAncestryState::new(
        &schema,
        &map,
        &source_state,
        [chromosome_ancestry(vec![
            class(0, &["copy-a"]),
            class(1, &["copy-b"]),
        ])],
    )
    .unwrap();

    assert!(matches!(
        ancestry_state.copy_ids_for_haplotype_content_at_slot(
            &schema,
            &map,
            &source_state,
            &chromosome("chr-a"),
            2,
        ),
        Err(AncestryAuthorityError::HaplotypeSlotOutOfRange { slot: 2, .. })
    ));
}
