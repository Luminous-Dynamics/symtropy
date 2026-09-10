use crate::{
    canonical::{fmt_hex, put_text, put_u64},
    EvolutionError, EvolutionExperimentId, HereditarySchema, HereditarySchemaDigest,
    PopulationGeneration, PopulationGeneticState, PopulationGeneticStateDigest, PopulationId,
    PopulationStructureProfile, PopulationStructureProfileDigest, PopulationTrajectoryPoint,
    PopulationTrajectoryPointDigest,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::{BTreeMap, BTreeSet}, fmt};

const METAPOPULATION_SNAPSHOT_DOMAIN: &[u8] =
    b"symtropy:evolution:metapopulation-source-snapshot:v1\0";

/// Compact manifest entry for one exact population at one simultaneous source cut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetapopulationSnapshotEntry {
    population_digest: PopulationGeneticStateDigest,
    trajectory_point_digest: PopulationTrajectoryPointDigest,
}

impl MetapopulationSnapshotEntry {
    pub fn population_digest(&self) -> PopulationGeneticStateDigest {
        self.population_digest
    }

    pub fn trajectory_point_digest(&self) -> PopulationTrajectoryPointDigest {
        self.trajectory_point_digest
    }
}

/// One exact simultaneous aggregate source state for a structured population.
///
/// This is a compact authority/provenance manifest. It does not own or clone the
/// canonical population state itself. Raw/restored snapshot data must be
/// revalidated against the current structure, populations, and trajectory points
/// before a structured population process may consume it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetapopulationSnapshot {
    schema_digest: HereditarySchemaDigest,
    structure_digest: PopulationStructureProfileDigest,
    experiment_id: EvolutionExperimentId,
    generation: PopulationGeneration,
    entries: BTreeMap<PopulationId, MetapopulationSnapshotEntry>,
}

impl MetapopulationSnapshot {
    /// Capture a free-standing/reference simultaneous source cut.
    ///
    /// This proves only that the supplied population states and trajectory
    /// points form one coherent generation/experiment under the supplied
    /// structure profile. It does not prove canonical world-history time.
    pub fn capture_reference(
        schema: &HereditarySchema,
        structure: &PopulationStructureProfile,
        populations: &BTreeMap<PopulationId, PopulationGeneticState>,
        points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    ) -> Result<Self, EvolutionError> {
        schema.validate()?;
        structure.validate()?;
        let (experiment_id, generation, entries) =
            validate_source_cut(schema, structure, populations, points)?;

        Ok(Self {
            schema_digest: schema.canonical_digest()?,
            structure_digest: structure.canonical_digest()?,
            experiment_id,
            generation,
            entries,
        })
    }

    pub fn schema_digest(&self) -> HereditarySchemaDigest {
        self.schema_digest
    }

    pub fn structure_digest(&self) -> PopulationStructureProfileDigest {
        self.structure_digest
    }

    pub fn experiment_id(&self) -> &EvolutionExperimentId {
        &self.experiment_id
    }

    pub fn generation(&self) -> PopulationGeneration {
        self.generation
    }

    pub fn entries(&self) -> &BTreeMap<PopulationId, MetapopulationSnapshotEntry> {
        &self.entries
    }

    pub fn validate_current(
        &self,
        schema: &HereditarySchema,
        structure: &PopulationStructureProfile,
        populations: &BTreeMap<PopulationId, PopulationGeneticState>,
        points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    ) -> Result<(), EvolutionError> {
        schema.validate()?;
        structure.validate()?;
        if schema.canonical_digest()? != self.schema_digest {
            return Err(EvolutionError::HereditarySchemaAuthorityMismatch);
        }
        if structure.canonical_digest()? != self.structure_digest {
            return Err(EvolutionError::MetapopulationStructureAuthorityMismatch);
        }

        let (experiment_id, generation, entries) =
            validate_source_cut(schema, structure, populations, points)?;
        if experiment_id != self.experiment_id {
            return Err(EvolutionError::PopulationExperimentMismatch);
        }
        if generation != self.generation {
            return Err(EvolutionError::PopulationGenerationMismatch);
        }
        if entries != self.entries {
            return Err(EvolutionError::MetapopulationSnapshotMismatch);
        }
        Ok(())
    }

    pub fn canonical_digest(&self) -> MetapopulationSnapshotDigest {
        let mut digest = Sha256::new();
        digest.update(METAPOPULATION_SNAPSHOT_DOMAIN);
        digest.update(self.schema_digest.as_bytes());
        digest.update(self.structure_digest.as_bytes());
        put_text(&mut digest, self.experiment_id.as_str());
        put_u64(&mut digest, self.generation.0);
        put_u64(&mut digest, self.entries.len() as u64);
        for (population, entry) in &self.entries {
            put_text(&mut digest, population.as_str());
            digest.update(entry.population_digest.as_bytes());
            digest.update(entry.trajectory_point_digest.as_bytes());
        }
        MetapopulationSnapshotDigest(digest.finalize().into())
    }
}

fn validate_source_cut(
    schema: &HereditarySchema,
    structure: &PopulationStructureProfile,
    populations: &BTreeMap<PopulationId, PopulationGeneticState>,
    points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
) -> Result<
    (
        EvolutionExperimentId,
        PopulationGeneration,
        BTreeMap<PopulationId, MetapopulationSnapshotEntry>,
    ),
    EvolutionError,
> {
    let population_keys: BTreeSet<PopulationId> = populations.keys().cloned().collect();
    let point_keys: BTreeSet<PopulationId> = points.keys().cloned().collect();
    if &population_keys != structure.populations() || &point_keys != structure.populations() {
        return Err(EvolutionError::MetapopulationSetMismatch);
    }

    let mut expected_experiment: Option<EvolutionExperimentId> = None;
    let mut expected_generation: Option<PopulationGeneration> = None;
    let mut entries = BTreeMap::new();

    for population_id in structure.populations() {
        let population = populations
            .get(population_id)
            .ok_or(EvolutionError::MetapopulationSetMismatch)?;
        if &population.population_id != population_id {
            return Err(EvolutionError::MetapopulationStateKeyMismatch {
                key: population_id.clone(),
                observed: population.population_id.clone(),
            });
        }
        population.validate(schema)?;

        let point = points
            .get(population_id)
            .ok_or(EvolutionError::MetapopulationSetMismatch)?;
        if point.population_id() != population_id {
            return Err(EvolutionError::MetapopulationPointKeyMismatch {
                key: population_id.clone(),
                observed: point.population_id().clone(),
            });
        }
        point.validate_current(schema, population)?;

        match &expected_experiment {
            Some(expected) if expected != point.experiment_id() => {
                return Err(EvolutionError::PopulationExperimentMismatch)
            }
            None => expected_experiment = Some(point.experiment_id().clone()),
            _ => {}
        }
        match expected_generation {
            Some(expected) if expected != point.generation() => {
                return Err(EvolutionError::PopulationGenerationMismatch)
            }
            None => expected_generation = Some(point.generation()),
            _ => {}
        }

        entries.insert(
            population_id.clone(),
            MetapopulationSnapshotEntry {
                population_digest: population.canonical_digest(schema)?,
                trajectory_point_digest: point.canonical_digest(),
            },
        );
    }

    Ok((
        expected_experiment.ok_or(EvolutionError::NoStructuredPopulations)?,
        expected_generation.ok_or(EvolutionError::NoStructuredPopulations)?,
        entries,
    ))
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MetapopulationSnapshotDigest([u8; 32]);

impl MetapopulationSnapshotDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for MetapopulationSnapshotDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MetapopulationSnapshotDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for MetapopulationSnapshotDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AlleleId, HereditarySchemaId, LocusDefinition, LocusId, ParentalSourceEdge,
        PopulationStructureModel, PopulationStructureProfileId,
    };

    fn pop(id: &str) -> PopulationId {
        PopulationId::new(id).unwrap()
    }

    fn allele(id: &str) -> AlleleId {
        AlleleId::new(id).unwrap()
    }

    fn schema() -> HereditarySchema {
        HereditarySchema::new(
            HereditarySchemaId::new("metapop-v0").unwrap(),
            2,
            vec![LocusDefinition::new(
                LocusId::new("focal").unwrap(),
                [allele("a"), allele("b")],
            )
            .unwrap()],
        )
        .unwrap()
    }

    fn structure(a: &PopulationId, b: &PopulationId) -> PopulationStructureProfile {
        PopulationStructureProfile::new(
            PopulationStructureProfileId::new("two-islands").unwrap(),
            "v1",
            PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
            vec![a.clone(), b.clone()],
            vec![ParentalSourceEdge::new(a.clone(), b.clone(), 100_000).unwrap()],
        )
        .unwrap()
    }

    fn state(
        schema: &HereditarySchema,
        id: PopulationId,
        a_copies: u64,
    ) -> PopulationGeneticState {
        let total = 8_u64;
        let mut counts = BTreeMap::new();
        if a_copies != 0 {
            counts.insert(allele("a"), a_copies);
        }
        if a_copies != total {
            counts.insert(allele("b"), total - a_copies);
        }
        PopulationGeneticState::from_counts(
            id,
            schema,
            4,
            BTreeMap::from([(LocusId::new("focal").unwrap(), counts)]),
        )
        .unwrap()
    }

    fn coherent_cut() -> (
        HereditarySchema,
        PopulationStructureProfile,
        BTreeMap<PopulationId, PopulationGeneticState>,
        BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    ) {
        let schema = schema();
        let a = pop("a");
        let b = pop("b");
        let structure = structure(&a, &b);
        let populations = BTreeMap::from([
            (a.clone(), state(&schema, a.clone(), 6)),
            (b.clone(), state(&schema, b.clone(), 2)),
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
    fn coherent_source_cut_captures_and_revalidates() {
        let (schema, structure, populations, points) = coherent_cut();
        let snapshot =
            MetapopulationSnapshot::capture_reference(&schema, &structure, &populations, &points)
                .unwrap();

        assert_eq!(snapshot.generation(), PopulationGeneration(12));
        assert_eq!(snapshot.entries().len(), 2);
        snapshot
            .validate_current(&schema, &structure, &populations, &points)
            .unwrap();
    }

    #[test]
    fn missing_or_extra_population_fails_closed() {
        let (schema, structure, mut populations, points) = coherent_cut();
        populations.remove(&pop("b"));
        assert_eq!(
            MetapopulationSnapshot::capture_reference(&schema, &structure, &populations, &points),
            Err(EvolutionError::MetapopulationSetMismatch)
        );

        let (schema, structure, mut populations, points) = coherent_cut();
        populations.insert(pop("c"), state(&schema, pop("c"), 4));
        assert_eq!(
            MetapopulationSnapshot::capture_reference(&schema, &structure, &populations, &points),
            Err(EvolutionError::MetapopulationSetMismatch)
        );
    }

    #[test]
    fn mixed_experiments_or_generations_fail_closed() {
        let (schema, structure, populations, mut points) = coherent_cut();
        let b = pop("b");
        points.insert(
            b.clone(),
            PopulationTrajectoryPoint::declare_reference_start(
                &schema,
                populations.get(&b).unwrap(),
                EvolutionExperimentId::new("other-experiment").unwrap(),
                PopulationGeneration(12),
            )
            .unwrap(),
        );
        assert_eq!(
            MetapopulationSnapshot::capture_reference(&schema, &structure, &populations, &points),
            Err(EvolutionError::PopulationExperimentMismatch)
        );

        let (schema, structure, populations, mut points) = coherent_cut();
        let b = pop("b");
        points.insert(
            b.clone(),
            PopulationTrajectoryPoint::declare_reference_start(
                &schema,
                populations.get(&b).unwrap(),
                EvolutionExperimentId::new("metapop-exp-01").unwrap(),
                PopulationGeneration(13),
            )
            .unwrap(),
        );
        assert_eq!(
            MetapopulationSnapshot::capture_reference(&schema, &structure, &populations, &points),
            Err(EvolutionError::PopulationGenerationMismatch)
        );
    }

    #[test]
    fn altered_population_state_stales_snapshot() {
        let (schema, structure, mut populations, points) = coherent_cut();
        let snapshot =
            MetapopulationSnapshot::capture_reference(&schema, &structure, &populations, &points)
                .unwrap();
        let a = pop("a");
        populations.insert(a.clone(), state(&schema, a, 4));

        assert!(snapshot
            .validate_current(&schema, &structure, &populations, &points)
            .is_err());
    }

    #[test]
    fn changed_structure_authority_stales_snapshot() {
        let (schema, structure, populations, points) = coherent_cut();
        let snapshot =
            MetapopulationSnapshot::capture_reference(&schema, &structure, &populations, &points)
                .unwrap();
        let a = pop("a");
        let b = pop("b");
        let changed = PopulationStructureProfile::new(
            PopulationStructureProfileId::new("two-islands").unwrap(),
            "v1",
            PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
            vec![a.clone(), b.clone()],
            vec![ParentalSourceEdge::new(a, b, 100_001).unwrap()],
        )
        .unwrap();

        assert_eq!(
            snapshot.validate_current(&schema, &changed, &populations, &points),
            Err(EvolutionError::MetapopulationStructureAuthorityMismatch)
        );
    }

    #[test]
    fn source_map_insertion_order_cannot_change_snapshot_identity() {
        let (schema, structure, populations, points) = coherent_cut();
        let first =
            MetapopulationSnapshot::capture_reference(&schema, &structure, &populations, &points)
                .unwrap();
        let populations_reversed: BTreeMap<_, _> = populations.into_iter().rev().collect();
        let points_reversed: BTreeMap<_, _> = points.into_iter().rev().collect();
        let second = MetapopulationSnapshot::capture_reference(
            &schema,
            &structure,
            &populations_reversed,
            &points_reversed,
        )
        .unwrap();

        assert_eq!(first, second);
        assert_eq!(first.canonical_digest(), second.canonical_digest());
    }

    #[test]
    fn serde_roundtrip_requires_current_revalidation_but_preserves_identity() {
        let (schema, structure, populations, points) = coherent_cut();
        let snapshot =
            MetapopulationSnapshot::capture_reference(&schema, &structure, &populations, &points)
                .unwrap();
        let encoded = serde_json::to_string(&snapshot).unwrap();
        let restored: MetapopulationSnapshot = serde_json::from_str(&encoded).unwrap();

        assert_eq!(restored.canonical_digest(), snapshot.canonical_digest());
        restored
            .validate_current(&schema, &structure, &populations, &points)
            .unwrap();
    }
}
