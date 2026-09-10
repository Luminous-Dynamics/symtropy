use std::collections::BTreeMap;
use symtropy_evolution_core::{
    AlleleId, ChromosomeDefinition, ChromosomeHaplotype, ChromosomeId, ChromosomeLocus,
    ChromosomeMap, ChromosomeMapId, EvolutionError, GeneticMapPositionMicromorgans,
    HereditarySchema, HereditarySchemaId, HereditaryState, LocusDefinition, LocusId,
    PhasedChromosomeState, PhasedHereditaryState,
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
        HereditarySchemaId::new("phased-schema-v1").unwrap(),
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

fn chromosome_map(schema: &HereditarySchema) -> ChromosomeMap {
    ChromosomeMap::new(
        ChromosomeMapId::new("phased-map-v1").unwrap(),
        schema,
        [
            ChromosomeDefinition::new(
                chromosome("chr-a"),
                vec![mapped_locus("a", 0), mapped_locus("b", 100_000)],
            )
            .unwrap(),
            ChromosomeDefinition::new(
                chromosome("chr-b"),
                vec![mapped_locus("c", 0), mapped_locus("d", 250_000)],
            )
            .unwrap(),
        ],
    )
    .unwrap()
}

fn hap(alleles: &[&str]) -> ChromosomeHaplotype {
    ChromosomeHaplotype::new(alleles.iter().map(|id| allele(id)).collect())
}

fn phased_chromosome(id: &str, haplotypes: Vec<ChromosomeHaplotype>) -> PhasedChromosomeState {
    PhasedChromosomeState::new(chromosome(id), haplotypes)
}

fn reference_state(schema: &HereditarySchema, map: &ChromosomeMap) -> PhasedHereditaryState {
    PhasedHereditaryState::new(
        schema,
        map,
        [
            phased_chromosome(
                "chr-a",
                vec![hap(&["a0", "b0"]), hap(&["a1", "b1"])],
            ),
            phased_chromosome(
                "chr-b",
                vec![hap(&["c0", "d1"]), hap(&["c1", "d0"])],
            ),
        ],
    )
    .unwrap()
}

#[test]
fn valid_phased_state_round_trips_with_exact_digest() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let state = reference_state(&schema, &map);
    state.validate(&schema, &map).unwrap();
    let expected = state.canonical_digest(&schema, &map).unwrap();

    let encoded = serde_json::to_string(&state).unwrap();
    let restored: PhasedHereditaryState = serde_json::from_str(&encoded).unwrap();
    restored.validate(&schema, &map).unwrap();
    assert_eq!(restored.canonical_digest(&schema, &map).unwrap(), expected);
}

#[test]
fn swapping_whole_unlabeled_homolog_rows_is_not_biological_identity() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let forward = PhasedHereditaryState::new(
        &schema,
        &map,
        [
            phased_chromosome(
                "chr-a",
                vec![hap(&["a0", "b0"]), hap(&["a1", "b1"])],
            ),
            phased_chromosome(
                "chr-b",
                vec![hap(&["c0", "d0"]), hap(&["c1", "d1"])],
            ),
        ],
    )
    .unwrap();
    let reversed = PhasedHereditaryState::new(
        &schema,
        &map,
        [
            phased_chromosome(
                "chr-b",
                vec![hap(&["c1", "d1"]), hap(&["c0", "d0"])],
            ),
            phased_chromosome(
                "chr-a",
                vec![hap(&["a1", "b1"]), hap(&["a0", "b0"])],
            ),
        ],
    )
    .unwrap();

    assert_eq!(forward, reversed);
    assert_eq!(
        forward.canonical_digest(&schema, &map).unwrap(),
        reversed.canonical_digest(&schema, &map).unwrap()
    );
}

#[test]
fn coupling_and_repulsion_have_same_unphased_projection_but_distinct_phase() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let coupling = PhasedHereditaryState::new(
        &schema,
        &map,
        [
            phased_chromosome(
                "chr-a",
                vec![hap(&["a0", "b0"]), hap(&["a1", "b1"])],
            ),
            phased_chromosome(
                "chr-b",
                vec![hap(&["c0", "d0"]), hap(&["c1", "d1"])],
            ),
        ],
    )
    .unwrap();
    let repulsion = PhasedHereditaryState::new(
        &schema,
        &map,
        [
            phased_chromosome(
                "chr-a",
                vec![hap(&["a0", "b1"]), hap(&["a1", "b0"])],
            ),
            phased_chromosome(
                "chr-b",
                vec![hap(&["c0", "d0"]), hap(&["c1", "d1"])],
            ),
        ],
    )
    .unwrap();

    assert_eq!(
        coupling.to_unphased(&schema, &map).unwrap(),
        repulsion.to_unphased(&schema, &map).unwrap()
    );
    assert_ne!(
        coupling.canonical_digest(&schema, &map).unwrap(),
        repulsion.canonical_digest(&schema, &map).unwrap()
    );
}

#[test]
fn phase_forgetting_exactly_matches_existing_unphased_authority() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let phased = reference_state(&schema, &map);
    let expected = HereditaryState::new(
        &schema,
        BTreeMap::from([
            (locus("a"), vec![allele("a0"), allele("a1")]),
            (locus("b"), vec![allele("b0"), allele("b1")]),
            (locus("c"), vec![allele("c0"), allele("c1")]),
            (locus("d"), vec![allele("d0"), allele("d1")]),
        ]),
    )
    .unwrap();

    assert_eq!(phased.to_unphased(&schema, &map).unwrap(), expected);
}

#[test]
fn wrong_chromosome_set_and_embedded_key_fail_closed() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let valid = reference_state(&schema, &map);

    let mut missing = valid.clone();
    missing.chromosomes.remove(&chromosome("chr-b"));
    assert_eq!(
        missing.validate(&schema, &map).unwrap_err(),
        EvolutionError::PhasedChromosomeSetMismatch
    );

    let mut key_mismatch = valid.clone();
    let chr_a = key_mismatch.chromosomes.remove(&chromosome("chr-a")).unwrap();
    key_mismatch
        .chromosomes
        .insert(chromosome("forged-key"), chr_a);
    assert!(matches!(
        key_mismatch.validate(&schema, &map),
        Err(EvolutionError::PhasedChromosomeKeyMismatch { .. })
            | Err(EvolutionError::PhasedChromosomeSetMismatch)
    ));
}

#[test]
fn wrong_haplotype_and_locus_counts_fail_closed() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let valid = reference_state(&schema, &map);

    let mut wrong_haplotype_count = valid.clone();
    wrong_haplotype_count
        .chromosomes
        .get_mut(&chromosome("chr-a"))
        .unwrap()
        .haplotypes
        .pop();
    assert!(matches!(
        wrong_haplotype_count.validate(&schema, &map),
        Err(EvolutionError::PhasedHaplotypeCountMismatch { .. })
    ));

    let mut wrong_locus_count = valid.clone();
    wrong_locus_count
        .chromosomes
        .get_mut(&chromosome("chr-a"))
        .unwrap()
        .haplotypes[0]
        .alleles
        .pop();
    assert!(matches!(
        wrong_locus_count.validate(&schema, &map),
        Err(EvolutionError::PhasedHaplotypeLocusCountMismatch { .. })
    ));
}

#[test]
fn unknown_allele_fails_at_its_exact_mapped_locus() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let mut state = reference_state(&schema, &map);
    let chr_a = state.chromosomes.get_mut(&chromosome("chr-a")).unwrap();
    chr_a.haplotypes[1].alleles[1] = allele("not-allowed");

    assert_eq!(
        state.validate(&schema, &map).unwrap_err(),
        EvolutionError::UnknownAllele {
            locus: locus("b"),
            allele: allele("not-allowed"),
        }
    );
}

#[test]
fn changed_map_or_schema_stales_phased_state() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let state = reference_state(&schema, &map);

    let changed_map = ChromosomeMap::new(
        map.id.clone(),
        &schema,
        [
            ChromosomeDefinition::new(
                chromosome("chr-a"),
                vec![mapped_locus("a", 0), mapped_locus("b", 101_000)],
            )
            .unwrap(),
            ChromosomeDefinition::new(
                chromosome("chr-b"),
                vec![mapped_locus("c", 0), mapped_locus("d", 250_000)],
            )
            .unwrap(),
        ],
    )
    .unwrap();
    assert_eq!(
        state.validate(&schema, &changed_map).unwrap_err(),
        EvolutionError::PhasedChromosomeMapAuthorityMismatch
    );

    let changed_schema = HereditarySchema::new(
        schema.id.clone(),
        2,
        ["a", "b", "c", "d"]
            .into_iter()
            .map(|id| {
                let mut allowed = vec![allele(&format!("{id}0")), allele(&format!("{id}1"))];
                if id == "c" {
                    allowed.push(allele("c2"));
                }
                LocusDefinition::new(locus(id), allowed).unwrap()
            })
            .collect::<Vec<_>>(),
    )
    .unwrap();
    assert!(state.validate(&changed_schema, &map).is_err());
}

#[test]
fn hostile_restored_noncanonical_homolog_order_cannot_regain_authority() {
    let schema = schema();
    let map = chromosome_map(&schema);
    let mut raw = reference_state(&schema, &map);
    raw.chromosomes
        .get_mut(&chromosome("chr-a"))
        .unwrap()
        .haplotypes
        .reverse();

    let encoded = serde_json::to_string(&raw).unwrap();
    let restored: PhasedHereditaryState = serde_json::from_str(&encoded).unwrap();
    assert!(matches!(
        restored.validate(&schema, &map),
        Err(EvolutionError::NonCanonicalHaplotypeOrder { .. })
    ));
    assert!(restored.canonical_digest(&schema, &map).is_err());
}
