use symtropy_evolution_core::{
    AlleleId, ChromosomeDefinition, ChromosomeHaplotype, ChromosomeId, ChromosomeLocus,
    ChromosomeMap, ChromosomeMapId, GeneticMapPositionMicromorgans, HereditarySchema,
    HereditarySchemaId, LocusDefinition, LocusId, PhasedChromosomeState,
    PhasedHereditaryState,
};

fn allele(id: &str) -> AlleleId {
    AlleleId::new(id).unwrap()
}

fn locus(id: &str) -> LocusId {
    LocusId::new(id).unwrap()
}

#[test]
fn phased_storage_supports_euploid_triploid_state_without_implying_meiosis() {
    let schema = HereditarySchema::new(
        HereditarySchemaId::new("triploid-schema-v1").unwrap(),
        3,
        vec![
            LocusDefinition::new(locus("a"), [allele("a0"), allele("a1"), allele("a2")])
                .unwrap(),
            LocusDefinition::new(locus("b"), [allele("b0"), allele("b1"), allele("b2")])
                .unwrap(),
        ],
    )
    .unwrap();

    let chromosome_id = ChromosomeId::new("chr-a").unwrap();
    let map = ChromosomeMap::new(
        ChromosomeMapId::new("triploid-map-v1").unwrap(),
        &schema,
        vec![
            ChromosomeDefinition::new(
                chromosome_id.clone(),
                vec![
                    ChromosomeLocus::new(
                        locus("a"),
                        GeneticMapPositionMicromorgans::new(0),
                    ),
                    ChromosomeLocus::new(
                        locus("b"),
                        GeneticMapPositionMicromorgans::new(100_000),
                    ),
                ],
            )
            .unwrap(),
        ],
    )
    .unwrap();

    let phased = PhasedHereditaryState::new(
        &schema,
        &map,
        vec![PhasedChromosomeState::new(
            chromosome_id,
            vec![
                ChromosomeHaplotype::new(vec![allele("a2"), allele("b0")]),
                ChromosomeHaplotype::new(vec![allele("a0"), allele("b2")]),
                ChromosomeHaplotype::new(vec![allele("a1"), allele("b1")]),
            ],
        )],
    )
    .unwrap();

    phased.validate(&schema, &map).unwrap();
    let unphased = phased.to_unphased(&schema, &map).unwrap();

    assert_eq!(unphased.copies[&locus("a")], vec![allele("a0"), allele("a1"), allele("a2")]);
    assert_eq!(unphased.copies[&locus("b")], vec![allele("b0"), allele("b1"), allele("b2")]);
}