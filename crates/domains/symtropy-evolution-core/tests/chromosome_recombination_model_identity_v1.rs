use symtropy_evolution_core::{
    AlleleId, ChromosomeDefinition, ChromosomeId, ChromosomeLocus, ChromosomeMap,
    ChromosomeMapId, ChromosomeRecombinationDomain, ChromosomeRecombinationModel,
    ChromosomeRecombinationProfile, ChromosomeRecombinationProfileId,
    GeneticMapIntervalMicromorgans, GeneticMapPositionMicromorgans, HereditarySchema,
    HereditarySchemaId, LocusDefinition, LocusId,
};

#[test]
fn zero_crossover_and_poisson_models_have_distinct_exact_authority() {
    let locus_id = LocusId::new("a").unwrap();
    let schema = HereditarySchema::new(
        HereditarySchemaId::new("model-identity-schema").unwrap(),
        2,
        vec![LocusDefinition::new(
            locus_id.clone(),
            [AlleleId::new("a0").unwrap(), AlleleId::new("a1").unwrap()],
        )
        .unwrap()],
    )
    .unwrap();
    let chromosome_id = ChromosomeId::new("chr-a").unwrap();
    let map = ChromosomeMap::new(
        ChromosomeMapId::new("model-identity-map").unwrap(),
        &schema,
        vec![ChromosomeDefinition::new(
            chromosome_id.clone(),
            vec![ChromosomeLocus::new(
                locus_id,
                GeneticMapPositionMicromorgans::new(100_000),
            )],
        )
        .unwrap()],
    )
    .unwrap();
    let domain = ChromosomeRecombinationDomain::new(
        chromosome_id,
        GeneticMapIntervalMicromorgans::new(
            GeneticMapPositionMicromorgans::new(0),
            GeneticMapPositionMicromorgans::new(500_000),
        )
        .unwrap(),
    )
    .unwrap();
    let id = ChromosomeRecombinationProfileId::new("same-profile-id").unwrap();

    let zero = ChromosomeRecombinationProfile::new(
        id.clone(),
        &schema,
        &map,
        ChromosomeRecombinationModel::NoCrossoversIndependentAssortmentV1,
        vec![domain.clone()],
    )
    .unwrap();
    let poisson = ChromosomeRecombinationProfile::new(
        id,
        &schema,
        &map,
        ChromosomeRecombinationModel::PoissonCrossoversNoInterferenceV1,
        vec![domain],
    )
    .unwrap();

    assert_ne!(
        zero.canonical_digest(&schema, &map).unwrap(),
        poisson.canonical_digest(&schema, &map).unwrap()
    );
}