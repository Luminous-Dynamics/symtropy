use std::collections::BTreeMap;

use symtropy_evolution_core::{
    AlleleId, EvolutionError, EvolutionExperimentId, HereditarySchema, HereditarySchemaId,
    LocusDefinition, LocusId, MetapopulationSnapshot, ParentalSourceEdge, PopulationGeneration,
    PopulationGeneticState, PopulationId, PopulationStructureModel, PopulationStructureProfile,
    PopulationStructureProfileId, PopulationTrajectoryPoint,
};

fn pop(id: &str) -> PopulationId {
    PopulationId::new(id).unwrap()
}

fn allele(id: &str) -> AlleleId {
    AlleleId::new(id).unwrap()
}

fn fixture() -> (
    HereditarySchema,
    PopulationStructureProfile,
    BTreeMap<PopulationId, PopulationGeneticState>,
    BTreeMap<PopulationId, PopulationTrajectoryPoint>,
) {
    let schema = HereditarySchema::new(
        HereditarySchemaId::new("metapop-adversarial-v0").unwrap(),
        2,
        vec![LocusDefinition::new(
            LocusId::new("focal").unwrap(),
            [allele("a"), allele("b")],
        )
        .unwrap()],
    )
    .unwrap();

    let a = pop("a");
    let b = pop("b");
    let structure = PopulationStructureProfile::new(
        PopulationStructureProfileId::new("two-islands").unwrap(),
        "v1",
        PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
        vec![a.clone(), b.clone()],
        vec![ParentalSourceEdge::new(a.clone(), b.clone(), 100_000).unwrap()],
    )
    .unwrap();

    let state = |id: PopulationId, a_copies: u64| {
        PopulationGeneticState::from_counts(
            id,
            &schema,
            4,
            BTreeMap::from([(
                LocusId::new("focal").unwrap(),
                BTreeMap::from([(allele("a"), a_copies), (allele("b"), 8 - a_copies)]),
            )]),
        )
        .unwrap()
    };

    let populations = BTreeMap::from([
        (a.clone(), state(a.clone(), 6)),
        (b.clone(), state(b.clone(), 2)),
    ]);
    let experiment = EvolutionExperimentId::new("metapop-exp-01").unwrap();
    let generation = PopulationGeneration(12);
    let points = populations
        .iter()
        .map(|(id, population)| {
            (
                id.clone(),
                PopulationTrajectoryPoint::declare_reference_start(
                    &schema,
                    population,
                    experiment.clone(),
                    generation,
                )
                .unwrap(),
            )
        })
        .collect();

    (schema, structure, populations, points)
}

#[test]
fn swapped_state_map_values_fail_before_snapshot_authority_is_minted() {
    let (schema, structure, mut populations, points) = fixture();
    let a = pop("a");
    let b = pop("b");
    let state_a = populations.remove(&a).unwrap();
    let state_b = populations.remove(&b).unwrap();
    populations.insert(a.clone(), state_b);
    populations.insert(b, state_a);

    assert!(matches!(
        MetapopulationSnapshot::capture_reference(&schema, &structure, &populations, &points),
        Err(EvolutionError::MetapopulationStateKeyMismatch { key, .. }) if key == a
    ));
}

#[test]
fn swapped_trajectory_map_values_fail_before_snapshot_authority_is_minted() {
    let (schema, structure, populations, mut points) = fixture();
    let a = pop("a");
    let b = pop("b");
    let point_a = points.remove(&a).unwrap();
    let point_b = points.remove(&b).unwrap();
    points.insert(a.clone(), point_b);
    points.insert(b, point_a);

    assert!(matches!(
        MetapopulationSnapshot::capture_reference(&schema, &structure, &populations, &points),
        Err(EvolutionError::MetapopulationPointKeyMismatch { key, .. }) if key == a
    ));
}

#[test]
fn restored_snapshot_with_tampered_manifest_digest_fails_revalidation() {
    let (schema, structure, populations, points) = fixture();
    let snapshot =
        MetapopulationSnapshot::capture_reference(&schema, &structure, &populations, &points)
            .unwrap();
    let mut raw = serde_json::to_value(&snapshot).unwrap();
    let first = raw["entries"]["a"]["population_digest"][0]
        .as_u64()
        .unwrap() as u8;
    raw["entries"]["a"]["population_digest"][0] =
        serde_json::Value::from(u64::from(first ^ 0xff));
    let restored: MetapopulationSnapshot = serde_json::from_value(raw).unwrap();

    assert_eq!(
        restored.validate_current(&schema, &structure, &populations, &points),
        Err(EvolutionError::MetapopulationSnapshotMismatch)
    );
}
