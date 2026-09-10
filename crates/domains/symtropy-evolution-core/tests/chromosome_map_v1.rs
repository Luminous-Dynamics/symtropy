use std::collections::BTreeMap;
use symtropy_evolution_core::{
    AlleleId, ChromosomeDefinition, ChromosomeId, ChromosomeLocus, ChromosomeMap,
    ChromosomeMapId, EvolutionError, GeneticMapPositionMicromorgans, HereditarySchema,
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

fn position(value: u64) -> GeneticMapPositionMicromorgans {
    GeneticMapPositionMicromorgans::new(value)
}

fn mapped_locus(id: &str, value: u64) -> ChromosomeLocus {
    ChromosomeLocus::new(locus(id), position(value))
}

fn schema() -> HereditarySchema {
    HereditarySchema::new(
        HereditarySchemaId::new("chrom-map-schema-v1").unwrap(),
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

fn chr(id: &str, entries: &[(&str, u64)]) -> ChromosomeDefinition {
    ChromosomeDefinition::new(
        chromosome(id),
        entries
            .iter()
            .map(|(locus_id, map_position)| mapped_locus(locus_id, *map_position))
            .collect(),
    )
    .unwrap()
}

fn reference_map(schema: &HereditarySchema) -> ChromosomeMap {
    ChromosomeMap::new(
        ChromosomeMapId::new("chrom-map-v1").unwrap(),
        schema,
        [
            chr("chr-a", &[("a", 0), ("b", 125_000)]),
            chr("chr-b", &[("c", 10_000), ("d", 410_000)]),
        ],
    )
    .unwrap()
}

#[test]
fn genetic_map_position_is_explicit_and_round_trips() {
    let map_position = position(125_000);
    assert_eq!(map_position.get(), 125_000);
    let encoded = serde_json::to_string(&map_position).unwrap();
    let restored: GeneticMapPositionMicromorgans = serde_json::from_str(&encoded).unwrap();
    assert_eq!(restored, map_position);
}

#[test]
fn complete_two_chromosome_map_validates_and_round_trips() {
    let schema = schema();
    let map = reference_map(&schema);
    map.validate(&schema).unwrap();
    let expected = map.canonical_digest(&schema).unwrap();

    let encoded = serde_json::to_string(&map).unwrap();
    let restored: ChromosomeMap = serde_json::from_str(&encoded).unwrap();
    restored.validate(&schema).unwrap();
    assert_eq!(restored.canonical_digest(&schema).unwrap(), expected);
}

#[test]
fn chromosome_declaration_insertion_order_is_not_identity() {
    let schema = schema();
    let forward = ChromosomeMap::new(
        ChromosomeMapId::new("ordered-map").unwrap(),
        &schema,
        [
            chr("chr-a", &[("a", 0), ("b", 125_000)]),
            chr("chr-b", &[("c", 10_000), ("d", 410_000)]),
        ],
    )
    .unwrap();
    let reverse = ChromosomeMap::new(
        ChromosomeMapId::new("ordered-map").unwrap(),
        &schema,
        [
            chr("chr-b", &[("c", 10_000), ("d", 410_000)]),
            chr("chr-a", &[("a", 0), ("b", 125_000)]),
        ],
    )
    .unwrap();

    assert_eq!(forward, reverse);
    assert_eq!(
        forward.canonical_digest(&schema).unwrap(),
        reverse.canonical_digest(&schema).unwrap()
    );
}

#[test]
fn locus_order_and_position_are_biological_authority() {
    let schema = schema();
    let reference = reference_map(&schema);
    let moved = ChromosomeMap::new(
        ChromosomeMapId::new("chrom-map-v1").unwrap(),
        &schema,
        [
            chr("chr-a", &[("a", 0), ("b", 126_000)]),
            chr("chr-b", &[("c", 10_000), ("d", 410_000)]),
        ],
    )
    .unwrap();
    let reordered = ChromosomeMap::new(
        ChromosomeMapId::new("chrom-map-v1").unwrap(),
        &schema,
        [
            chr("chr-a", &[("b", 0), ("a", 125_000)]),
            chr("chr-b", &[("c", 10_000), ("d", 410_000)]),
        ],
    )
    .unwrap();

    assert_ne!(
        reference.canonical_digest(&schema).unwrap(),
        moved.canonical_digest(&schema).unwrap()
    );
    assert_ne!(
        reference.canonical_digest(&schema).unwrap(),
        reordered.canonical_digest(&schema).unwrap()
    );
}

#[test]
fn exact_schema_change_stales_map_even_when_locus_ids_are_unchanged() {
    let original_schema = schema();
    let map = reference_map(&original_schema);
    let changed_schema = HereditarySchema::new(
        original_schema.id.clone(),
        2,
        ["a", "b", "c", "d"]
            .into_iter()
            .map(|id| {
                let mut alleles = vec![allele(&format!("{id}0")), allele(&format!("{id}1"))];
                if id == "b" {
                    alleles.push(allele("b2"));
                }
                LocusDefinition::new(locus(id), alleles).unwrap()
            })
            .collect::<Vec<_>>(),
    )
    .unwrap();

    assert_eq!(
        map.validate(&changed_schema).unwrap_err(),
        EvolutionError::ChromosomeMapSchemaAuthorityMismatch
    );
}

#[test]
fn unknown_duplicate_and_missing_loci_fail_closed() {
    let schema = schema();

    assert!(ChromosomeMap::new(
        ChromosomeMapId::new("unknown").unwrap(),
        &schema,
        [
            chr("chr-a", &[("a", 0), ("b", 100)]),
            chr("chr-b", &[("c", 0), ("d", 100), ("x", 200)]),
        ],
    )
    .is_err());

    assert_eq!(
        ChromosomeMap::new(
            ChromosomeMapId::new("duplicate").unwrap(),
            &schema,
            [
                chr("chr-a", &[("a", 0), ("b", 100)]),
                chr("chr-b", &[("b", 0), ("c", 100), ("d", 200)]),
            ],
        )
        .unwrap_err(),
        EvolutionError::DuplicateChromosomeLocus { locus: locus("b") }
    );

    assert_eq!(
        ChromosomeMap::new(
            ChromosomeMapId::new("missing").unwrap(),
            &schema,
            [
                chr("chr-a", &[("a", 0), ("b", 100)]),
                chr("chr-b", &[("c", 0)]),
            ],
        )
        .unwrap_err(),
        EvolutionError::MissingChromosomeLocus { locus: locus("d") }
    );
}

#[test]
fn chromosome_positions_must_be_strictly_increasing() {
    assert!(matches!(
        ChromosomeDefinition::new(
            chromosome("chr-a"),
            vec![mapped_locus("a", 10), mapped_locus("b", 10)],
        ),
        Err(EvolutionError::NonIncreasingChromosomeMapPosition { .. })
    ));
    assert!(matches!(
        ChromosomeDefinition::new(
            chromosome("chr-a"),
            vec![mapped_locus("a", 20), mapped_locus("b", 10)],
        ),
        Err(EvolutionError::NonIncreasingChromosomeMapPosition { .. })
    ));
}

#[test]
fn raw_restored_key_mismatch_cannot_regain_authority() {
    let schema = schema();
    let valid = reference_map(&schema);
    let mut raw_chromosomes = valid.chromosomes.clone();
    let original = raw_chromosomes.remove(&chromosome("chr-a")).unwrap();
    raw_chromosomes.insert(chromosome("forged-key"), original);

    let raw = ChromosomeMap {
        map_version: valid.map_version,
        id: valid.id.clone(),
        schema_id: valid.schema_id.clone(),
        schema_digest: valid.schema_digest,
        chromosomes: raw_chromosomes,
    };
    let encoded = serde_json::to_string(&raw).unwrap();
    let restored: ChromosomeMap = serde_json::from_str(&encoded).unwrap();

    assert!(matches!(
        restored.validate(&schema),
        Err(EvolutionError::ChromosomeKeyMismatch { .. })
    ));
    assert!(restored.canonical_digest(&schema).is_err());
}

#[test]
fn raw_restored_noncanonical_locus_order_cannot_regain_authority() {
    let schema = schema();
    let valid = reference_map(&schema);
    let mut chromosomes = valid.chromosomes.clone();
    chromosomes.insert(
        chromosome("chr-a"),
        ChromosomeDefinition {
            id: chromosome("chr-a"),
            loci: vec![mapped_locus("a", 125_000), mapped_locus("b", 0)],
        },
    );
    let raw = ChromosomeMap {
        map_version: valid.map_version,
        id: valid.id.clone(),
        schema_id: valid.schema_id.clone(),
        schema_digest: valid.schema_digest,
        chromosomes,
    };
    let encoded = serde_json::to_string(&raw).unwrap();
    let restored: ChromosomeMap = serde_json::from_str(&encoded).unwrap();

    assert!(matches!(
        restored.validate(&schema),
        Err(EvolutionError::NonIncreasingChromosomeMapPosition { .. })
    ));
}

#[test]
fn raw_duplicate_chromosome_map_keys_are_detected_before_canonicalization() {
    let schema = schema();
    let duplicate = ChromosomeMap::new(
        ChromosomeMapId::new("duplicate-chromosome").unwrap(),
        &schema,
        [
            chr("same", &[("a", 0), ("b", 100)]),
            chr("same", &[("c", 0), ("d", 100)]),
        ],
    );
    assert_eq!(
        duplicate.unwrap_err(),
        EvolutionError::DuplicateChromosomeIdentity {
            chromosome: chromosome("same")
        }
    );
}

#[test]
fn raw_map_with_empty_chromosome_is_rejected_after_restore() {
    let schema = schema();
    let valid = reference_map(&schema);
    let mut chromosomes = BTreeMap::new();
    chromosomes.insert(
        chromosome("empty"),
        ChromosomeDefinition {
            id: chromosome("empty"),
            loci: vec![],
        },
    );
    let raw = ChromosomeMap {
        map_version: valid.map_version,
        id: valid.id,
        schema_id: valid.schema_id,
        schema_digest: valid.schema_digest,
        chromosomes,
    };
    let restored: ChromosomeMap =
        serde_json::from_str(&serde_json::to_string(&raw).unwrap()).unwrap();
    assert!(matches!(
        restored.validate(&schema),
        Err(EvolutionError::EmptyChromosome { .. })
    ));
}
