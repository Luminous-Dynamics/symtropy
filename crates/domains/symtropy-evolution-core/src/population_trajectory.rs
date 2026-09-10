use crate::{
    canonical::{fmt_hex, put_text, put_u64},
    EvolutionError, EvolutionExperimentId, HereditarySchema, HereditarySchemaDigest,
    PopulationGeneticState, PopulationGeneticStateDigest, PopulationId,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

const TRAJECTORY_POINT_DOMAIN: &[u8] = b"symtropy:evolution:population-trajectory-point:v1\0";

/// Canonical generation coordinate for generation-based reference processes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PopulationGeneration(pub u64);

/// One exact aggregate population state located on one stochastic trajectory.
///
/// This is deliberately distinct from `PopulationGeneticState`: equal allele
/// counts may recur at different generations or in different experiments and
/// therefore represent different causal positions.
///
/// A trajectory point is still a simulation/reference construct. Declaring one
/// does not prove canonical world time; a later Living World/continuation adapter
/// must bind that higher-level authority explicitly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PopulationTrajectoryPoint {
    schema_digest: HereditarySchemaDigest,
    population_id: PopulationId,
    population_digest: PopulationGeneticStateDigest,
    experiment_id: EvolutionExperimentId,
    generation: PopulationGeneration,
}

impl PopulationTrajectoryPoint {
    /// Declare an explicit starting point for a free-standing/reference
    /// population experiment.
    ///
    /// Callers should not use this constructor as a substitute for canonical
    /// world-history time. Ordinary continuation should consume the destination
    /// point returned by the previous population transition.
    pub fn declare_reference_start(
        schema: &HereditarySchema,
        population: &PopulationGeneticState,
        experiment_id: EvolutionExperimentId,
        generation: PopulationGeneration,
    ) -> Result<Self, EvolutionError> {
        population.validate(schema)?;
        Ok(Self::from_validated_state(
            schema,
            population,
            experiment_id,
            generation,
        )?)
    }

    pub(crate) fn from_validated_state(
        schema: &HereditarySchema,
        population: &PopulationGeneticState,
        experiment_id: EvolutionExperimentId,
        generation: PopulationGeneration,
    ) -> Result<Self, EvolutionError> {
        population.validate(schema)?;
        Ok(Self {
            schema_digest: schema.canonical_digest()?,
            population_id: population.population_id.clone(),
            population_digest: population.canonical_digest(schema)?,
            experiment_id,
            generation,
        })
    }

    pub fn schema_digest(&self) -> HereditarySchemaDigest {
        self.schema_digest
    }

    pub fn population_id(&self) -> &PopulationId {
        &self.population_id
    }

    pub fn population_digest(&self) -> PopulationGeneticStateDigest {
        self.population_digest
    }

    pub fn experiment_id(&self) -> &EvolutionExperimentId {
        &self.experiment_id
    }

    pub fn generation(&self) -> PopulationGeneration {
        self.generation
    }

    /// Revalidate raw/restored trajectory data against the exact current
    /// population/schema. This checks state identity; callers that expect a
    /// particular experiment should additionally call `validate_experiment`.
    pub fn validate_current(
        &self,
        schema: &HereditarySchema,
        population: &PopulationGeneticState,
    ) -> Result<(), EvolutionError> {
        population.validate(schema)?;
        if schema.canonical_digest()? != self.schema_digest {
            return Err(EvolutionError::HereditarySchemaAuthorityMismatch);
        }
        if population.population_id != self.population_id {
            return Err(EvolutionError::PopulationIdentityMismatch);
        }
        if population.canonical_digest(schema)? != self.population_digest {
            return Err(EvolutionError::PopulationTrajectoryStateMismatch);
        }
        Ok(())
    }

    pub fn validate_experiment(
        &self,
        expected: &EvolutionExperimentId,
    ) -> Result<(), EvolutionError> {
        if &self.experiment_id == expected {
            Ok(())
        } else {
            Err(EvolutionError::PopulationExperimentMismatch)
        }
    }

    pub fn canonical_digest(&self) -> PopulationTrajectoryPointDigest {
        let mut digest = Sha256::new();
        digest.update(TRAJECTORY_POINT_DOMAIN);
        digest.update(self.schema_digest.as_bytes());
        put_text(&mut digest, self.population_id.as_str());
        digest.update(self.population_digest.as_bytes());
        put_text(&mut digest, self.experiment_id.as_str());
        put_u64(&mut digest, self.generation.0);
        PopulationTrajectoryPointDigest(digest.finalize().into())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PopulationTrajectoryPointDigest([u8; 32]);

impl PopulationTrajectoryPointDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for PopulationTrajectoryPointDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PopulationTrajectoryPointDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for PopulationTrajectoryPointDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AlleleId, HereditarySchemaId, LocusDefinition, LocusId};
    use std::collections::BTreeMap;

    fn allele(id: &str) -> AlleleId {
        AlleleId::new(id).unwrap()
    }

    fn fixture() -> (HereditarySchema, PopulationGeneticState) {
        let schema = HereditarySchema::new(
            HereditarySchemaId::new("trajectory-v0").unwrap(),
            2,
            vec![LocusDefinition::new(
                LocusId::new("focal").unwrap(),
                [allele("a"), allele("b")],
            )
            .unwrap()],
        )
        .unwrap();
        let population = PopulationGeneticState::from_counts(
            PopulationId::new("population-a").unwrap(),
            &schema,
            2,
            BTreeMap::from([(
                LocusId::new("focal").unwrap(),
                BTreeMap::from([(allele("a"), 2), (allele("b"), 2)]),
            )]),
        )
        .unwrap();
        (schema, population)
    }

    #[test]
    fn equal_population_state_at_different_generations_is_not_same_trajectory_point() {
        let (schema, population) = fixture();
        let experiment = EvolutionExperimentId::new("replicate-a").unwrap();
        let g4 = PopulationTrajectoryPoint::declare_reference_start(
            &schema,
            &population,
            experiment.clone(),
            PopulationGeneration(4),
        )
        .unwrap();
        let g9 = PopulationTrajectoryPoint::declare_reference_start(
            &schema,
            &population,
            experiment,
            PopulationGeneration(9),
        )
        .unwrap();

        assert_ne!(g4.canonical_digest(), g9.canonical_digest());
    }

    #[test]
    fn changed_population_state_stales_point() {
        let (schema, population) = fixture();
        let point = PopulationTrajectoryPoint::declare_reference_start(
            &schema,
            &population,
            EvolutionExperimentId::new("replicate-a").unwrap(),
            PopulationGeneration(0),
        )
        .unwrap();
        let changed = PopulationGeneticState::from_counts(
            PopulationId::new("population-a").unwrap(),
            &schema,
            2,
            BTreeMap::from([(
                LocusId::new("focal").unwrap(),
                BTreeMap::from([(allele("a"), 3), (allele("b"), 1)]),
            )]),
        )
        .unwrap();

        assert_eq!(
            point.validate_current(&schema, &changed),
            Err(EvolutionError::PopulationTrajectoryStateMismatch)
        );
    }

    #[test]
    fn expected_experiment_is_explicit() {
        let (schema, population) = fixture();
        let point = PopulationTrajectoryPoint::declare_reference_start(
            &schema,
            &population,
            EvolutionExperimentId::new("replicate-a").unwrap(),
            PopulationGeneration(0),
        )
        .unwrap();

        assert_eq!(
            point.validate_experiment(&EvolutionExperimentId::new("replicate-b").unwrap()),
            Err(EvolutionError::PopulationExperimentMismatch)
        );
    }
}
