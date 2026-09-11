use symtropy_evolution_core::{
    derive_modeled_gamete_ancestry, derive_zero_crossover_linked_gamete, AlleleId,
    AncestryCopyId, ChromosomeAncestryState, ChromosomeDefinition, ChromosomeHaplotype,
    ChromosomeId, ChromosomeLocus, ChromosomeMap, ChromosomeMapId,
    ChromosomeRecombinationDomain, ChromosomeRecombinationModel, ChromosomeRecombinationProfile,
    ChromosomeRecombinationProfileId, GeneticMapIntervalMicromorgans,
    GeneticMapPositionMicromorgans, HaplotypeAncestryClass, HereditarySchema,
    HereditarySchemaId, LinkedGameteDerivationEvidence, LocusDefinition, LocusId, ParentRole,
    PhasedAncestryState, PhasedChromosomeState, PhasedHereditaryState, ReproductionEventId,
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
fn hap(id: &str) -> ChromosomeHaplotype {
    ChromosomeHaplotype::new(vec![allele(id)])
}

fn build(include_unrelated: bool) -> (
    HereditarySchema,
    ChromosomeMap,
    PhasedHereditaryState,
    PhasedAncestryState,
    ChromosomeRecombinationProfile,
) {
    let schema = HereditarySchema::new(
        HereditarySchemaId::new(if include_unrelated {
            "phylo-locality-two"
        } else {
            "phylo-locality-one"
        })
        .unwrap(),
        2,
        if include_unrelated {
            vec![
                LocusDefinition::new(locus("a"), [allele("a0"), allele("a1")]).unwrap(),
                LocusDefinition::new(locus("x"), [allele("x0"), allele("x1")]).unwrap(),
            ]
        } else {
            vec![LocusDefinition::new(
                locus("a"),
                [allele("a0"), allele("a1")],
            )
            .unwrap()]
        },
    )
    .unwrap();

    let mut definitions = vec![ChromosomeDefinition::new(
        chromosome("chr-a"),
        vec![ChromosomeLocus::new(locus("a"), pos(1))],
    )
    .unwrap()];
    if include_unrelated {
        definitions.push(
            ChromosomeDefinition::new(
                chromosome("chr-b"),
                vec![ChromosomeLocus::new(locus("x"), pos(1))],
            )
            .unwrap(),
        );
    }
    let map = ChromosomeMap::new(
        ChromosomeMapId::new(if include_unrelated {
            "phylo-locality-map-two"
        } else {
            "phylo-locality-map-one"
        })
        .unwrap(),
        &schema,
        definitions,
    )
    .unwrap();

    let mut phased = vec![PhasedChromosomeState::new(
        chromosome("chr-a"),
        vec![hap("a0"), hap("a0")],
    )];
    if include_unrelated {
        phased.push(PhasedChromosomeState::new(
            chromosome("chr-b"),
            vec![hap("x0"), hap("x0")],
        ));
    }
    let source = PhasedHereditaryState::new(&schema, &map, phased).unwrap();

    let mut ancestry_chromosomes = vec![ChromosomeAncestryState::new(
        chromosome("chr-a"),
        vec![HaplotypeAncestryClass::new(
            0,
            vec![ancestry("copy-a0"), ancestry("copy-a1")],
        )
        .unwrap()],
    )
    .unwrap()];
    if include_unrelated {
        ancestry_chromosomes.push(
            ChromosomeAncestryState::new(
                chromosome("chr-b"),
                vec![HaplotypeAncestryClass::new(
                    0,
                    vec![ancestry("copy-b0"), ancestry("copy-b1")],
                )
                .unwrap()],
            )
            .unwrap(),
        );
    }
    let source_ancestry =
        PhasedAncestryState::new(&schema, &map, &source, ancestry_chromosomes).unwrap();

    let mut domains = vec![ChromosomeRecombinationDomain::new(
        chromosome("chr-a"),
        interval(0, 10),
    )
    .unwrap()];
    if include_unrelated {
        domains.push(
            ChromosomeRecombinationDomain::new(chromosome("chr-b"), interval(0, 10)).unwrap(),
        );
    }
    let profile = ChromosomeRecombinationProfile::new(
        ChromosomeRecombinationProfileId::new(if include_unrelated {
            "phylo-locality-profile-two"
        } else {
            "phylo-locality-profile-one"
        })
        .unwrap(),
        &schema,
        &map,
        ChromosomeRecombinationModel::NoCrossoversIndependentAssortmentV1,
        domains,
    )
    .unwrap();

    (schema, map, source, source_ancestry, profile)
}

#[test]
fn unrelated_chromosome_does_not_reroll_existing_ambiguous_ancestry_choice() {
    let event = ReproductionEventId::new("ancestry-0").unwrap();
    let (schema_one, map_one, source_one, ancestry_one, profile_one) = build(false);
    let evidence_one = LinkedGameteDerivationEvidence::ZeroCrossover(
        derive_zero_crossover_linked_gamete(
            &schema_one,
            &map_one,
            &source_one,
            &profile_one,
            &event,
            ParentRole::ParentA,
        )
        .unwrap(),
    );
    let derived_one = derive_modeled_gamete_ancestry(
        &schema_one,
        &map_one,
        &source_one,
        &ancestry_one,
        &profile_one,
        &evidence_one,
        &event,
        ParentRole::ParentA,
    )
    .unwrap();

    let (schema_two, map_two, source_two, ancestry_two, profile_two) = build(true);
    let evidence_two = LinkedGameteDerivationEvidence::ZeroCrossover(
        derive_zero_crossover_linked_gamete(
            &schema_two,
            &map_two,
            &source_two,
            &profile_two,
            &event,
            ParentRole::ParentA,
        )
        .unwrap(),
    );
    let derived_two = derive_modeled_gamete_ancestry(
        &schema_two,
        &map_two,
        &source_two,
        &ancestry_two,
        &profile_two,
        &evidence_two,
        &event,
        ParentRole::ParentA,
    )
    .unwrap();

    assert_eq!(
        derived_one.ancestry.chromosomes[&chromosome("chr-a")].loci[0].source_copy_id,
        derived_two.ancestry.chromosomes[&chromosome("chr-a")].loci[0].source_copy_id
    );
}

#[test]
fn provenance_rejects_wrong_event_role_or_ancestry_authority() {
    let event = ReproductionEventId::new("ancestry-0").unwrap();
    let (schema, map, source, source_ancestry, profile) = build(false);
    let evidence = LinkedGameteDerivationEvidence::ZeroCrossover(
        derive_zero_crossover_linked_gamete(
            &schema,
            &map,
            &source,
            &profile,
            &event,
            ParentRole::ParentA,
        )
        .unwrap(),
    );
    let derived = derive_modeled_gamete_ancestry(
        &schema,
        &map,
        &source,
        &source_ancestry,
        &profile,
        &evidence,
        &event,
        ParentRole::ParentA,
    )
    .unwrap();

    let wrong_event = ReproductionEventId::new("ancestry-5").unwrap();
    assert!(derived
        .provenance
        .validate_current(
            &schema,
            &map,
            &source,
            &source_ancestry,
            &profile,
            &evidence,
            &wrong_event,
            ParentRole::ParentA,
            &derived.ancestry,
        )
        .is_err());
    assert!(derived
        .provenance
        .validate_current(
            &schema,
            &map,
            &source,
            &source_ancestry,
            &profile,
            &evidence,
            &event,
            ParentRole::ParentB,
            &derived.ancestry,
        )
        .is_err());

    let forged_ancestry = PhasedAncestryState::new(
        &schema,
        &map,
        &source,
        [ChromosomeAncestryState::new(
            chromosome("chr-a"),
            vec![HaplotypeAncestryClass::new(
                0,
                vec![ancestry("forged-0"), ancestry("forged-1")],
            )
            .unwrap()],
        )
        .unwrap()],
    )
    .unwrap();
    assert!(derived
        .provenance
        .validate_current(
            &schema,
            &map,
            &source,
            &forged_ancestry,
            &profile,
            &evidence,
            &event,
            ParentRole::ParentA,
            &derived.ancestry,
        )
        .is_err());
}
