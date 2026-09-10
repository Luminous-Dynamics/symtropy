use crate::{
    canonical::{fmt_hex, put_text, put_u64},
    neutral_wright_fisher_step, AlleleId, EvolutionError, EvolutionExperimentId,
    HereditarySchema, HereditarySchemaDigest, LocusId, MetapopulationSnapshot,
    MetapopulationSnapshotDigest, PopulationGeneration, PopulationGeneticState,
    PopulationGeneticStateDigest, PopulationId, PopulationProcessModel, PopulationProcessProfile,
    PopulationProcessProfileDigest, PopulationStructureProfile, PopulationStructureProfileDigest,
    PopulationTrajectoryPoint, PopulationTrajectoryPointDigest, PopulationTransitionId,
    PROBABILITY_SCALE_PPM,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fmt};

const STRUCTURED_TRANSITION_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:structured-population-transition:v1\0";
const SOURCE_CHOICE_RNG_DOMAIN: &[u8] =
    b"symtropy:evolution:structured-parental-source-choice:v1\0";
const SOURCE_ALLELE_RNG_DOMAIN: &[u8] =
    b"symtropy:evolution:structured-source-allele-choice:v1\0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuredPopulationTransitionProvenance {
    schema_digest: HereditarySchemaDigest,
    structure_digest: PopulationStructureProfileDigest,
    process_profile_digest: PopulationProcessProfileDigest,
    source_snapshot_digest: MetapopulationSnapshotDigest,
    experiment_id: EvolutionExperimentId,
    transition_id: PopulationTransitionId,
    generation_from: PopulationGeneration,
    generation_to: PopulationGeneration,
    destination_digests: BTreeMap<PopulationId, PopulationGeneticStateDigest>,
    destination_point_digests: BTreeMap<PopulationId, PopulationTrajectoryPointDigest>,
}

impl StructuredPopulationTransitionProvenance {
    pub fn structure_digest(&self) -> PopulationStructureProfileDigest {
        self.structure_digest
    }

    pub fn source_snapshot_digest(&self) -> MetapopulationSnapshotDigest {
        self.source_snapshot_digest
    }

    pub fn experiment_id(&self) -> &EvolutionExperimentId {
        &self.experiment_id
    }

    pub fn transition_id(&self) -> &PopulationTransitionId {
        &self.transition_id
    }

    pub fn generation_from(&self) -> PopulationGeneration {
        self.generation_from
    }

    pub fn generation_to(&self) -> PopulationGeneration {
        self.generation_to
    }

    pub fn canonical_digest(&self) -> StructuredPopulationTransitionProvenanceDigest {
        let mut digest = Sha256::new();
        digest.update(STRUCTURED_TRANSITION_DIGEST_DOMAIN);
        digest.update(self.schema_digest.as_bytes());
        digest.update(self.structure_digest.as_bytes());
        digest.update(self.process_profile_digest.as_bytes());
        digest.update(self.source_snapshot_digest.as_bytes());
        put_text(&mut digest, self.experiment_id.as_str());
        put_text(&mut digest, self.transition_id.as_str());
        put_u64(&mut digest, self.generation_from.0);
        put_u64(&mut digest, self.generation_to.0);

        put_u64(&mut digest, self.destination_digests.len() as u64);
        for (population, state_digest) in &self.destination_digests {
            put_text(&mut digest, population.as_str());
            digest.update(state_digest.as_bytes());
            let point_digest = self
                .destination_point_digests
                .get(population)
                .expect("validated structured provenance uses identical destination key sets");
            digest.update(point_digest.as_bytes());
        }
        StructuredPopulationTransitionProvenanceDigest(digest.finalize().into())
    }

    pub fn validate_current(
        &self,
        schema: &HereditarySchema,
        structure: &PopulationStructureProfile,
        source_populations: &BTreeMap<PopulationId, PopulationGeneticState>,
        source_points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
        source_snapshot: &MetapopulationSnapshot,
        transition_id: &PopulationTransitionId,
        process_profile: &PopulationProcessProfile,
        destinations: &BTreeMap<PopulationId, PopulationGeneticState>,
        destination_points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    ) -> Result<(), EvolutionError> {
        schema.validate()?;
        structure.validate()?;
        process_profile.validate()?;
        if process_profile.model != PopulationProcessModel::NeutralIndependentLocusWrightFisher {
            return Err(EvolutionError::PopulationProcessModelMismatch);
        }
        source_snapshot.validate_current(
            schema,
            structure,
            source_populations,
            source_points,
        )?;

        if schema.canonical_digest()? != self.schema_digest {
            return Err(EvolutionError::HereditarySchemaAuthorityMismatch);
        }
        if structure.canonical_digest()? != self.structure_digest {
            return Err(EvolutionError::StructuredPopulationStructureAuthorityMismatch);
        }
        if process_profile.canonical_digest()? != self.process_profile_digest {
            return Err(EvolutionError::PopulationProcessAuthorityMismatch);
        }
        if source_snapshot.canonical_digest() != self.source_snapshot_digest {
            return Err(EvolutionError::StructuredPopulationSourceSnapshotMismatch);
        }
        if source_snapshot.experiment_id() != &self.experiment_id {
            return Err(EvolutionError::PopulationExperimentMismatch);
        }
        if source_snapshot.generation() != self.generation_from
            || self.generation_to.0
                != self
                    .generation_from
                    .0
                    .checked_add(1)
                    .ok_or(EvolutionError::CountOverflow)?
        {
            return Err(EvolutionError::PopulationGenerationMismatch);
        }
        if transition_id != &self.transition_id {
            return Err(EvolutionError::StructuredPopulationTransitionMismatch);
        }

        validate_destination_cut(
            schema,
            structure,
            &self.experiment_id,
            self.generation_to,
            destinations,
            destination_points,
        )?;
        let current_destination_digests = population_digest_map(schema, destinations)?;
        let current_point_digests = trajectory_digest_map(destination_points);
        if current_destination_digests != self.destination_digests
            || current_point_digests != self.destination_point_digests
        {
            return Err(EvolutionError::StructuredPopulationDestinationMismatch);
        }

        let (expected_destinations, expected_points) = derive_destination_cut(
            schema,
            structure,
            source_populations,
            source_points,
            source_snapshot,
            transition_id,
            process_profile,
        )?;
        if &expected_destinations != destinations || &expected_points != destination_points {
            return Err(EvolutionError::StructuredPopulationTransitionMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StructuredPopulationTransitionProvenanceDigest([u8; 32]);

impl StructuredPopulationTransitionProvenanceDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for StructuredPopulationTransitionProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StructuredPopulationTransitionProvenanceDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for StructuredPopulationTransitionProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuredPopulationTransitionResult {
    pub destinations: BTreeMap<PopulationId, PopulationGeneticState>,
    pub destination_points: BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    pub provenance: StructuredPopulationTransitionProvenance,
}

/// Advance one simultaneous fixed-census structured Wright-Fisher generation.
///
/// The supplied metapopulation snapshot proves which exact generation-G states
/// form the immutable parental source cut. Every generation-G+1 destination is
/// derived from that cut before any destination state can become a source.
pub fn structured_wright_fisher_step(
    schema: &HereditarySchema,
    structure: &PopulationStructureProfile,
    source_populations: &BTreeMap<PopulationId, PopulationGeneticState>,
    source_points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    source_snapshot: &MetapopulationSnapshot,
    transition_id: &PopulationTransitionId,
    process_profile: &PopulationProcessProfile,
) -> Result<StructuredPopulationTransitionResult, EvolutionError> {
    schema.validate()?;
    structure.validate()?;
    process_profile.validate()?;
    if process_profile.model != PopulationProcessModel::NeutralIndependentLocusWrightFisher {
        return Err(EvolutionError::PopulationProcessModelMismatch);
    }
    source_snapshot.validate_current(
        schema,
        structure,
        source_populations,
        source_points,
    )?;

    let generation_from = source_snapshot.generation();
    let generation_to = PopulationGeneration(
        generation_from
            .0
            .checked_add(1)
            .ok_or(EvolutionError::CountOverflow)?,
    );
    let (destinations, destination_points) = derive_destination_cut(
        schema,
        structure,
        source_populations,
        source_points,
        source_snapshot,
        transition_id,
        process_profile,
    )?;

    let provenance = StructuredPopulationTransitionProvenance {
        schema_digest: schema.canonical_digest()?,
        structure_digest: structure.canonical_digest()?,
        process_profile_digest: process_profile.canonical_digest()?,
        source_snapshot_digest: source_snapshot.canonical_digest(),
        experiment_id: source_snapshot.experiment_id().clone(),
        transition_id: transition_id.clone(),
        generation_from,
        generation_to,
        destination_digests: population_digest_map(schema, &destinations)?,
        destination_point_digests: trajectory_digest_map(&destination_points),
    };
    provenance.validate_current(
        schema,
        structure,
        source_populations,
        source_points,
        source_snapshot,
        transition_id,
        process_profile,
        &destinations,
        &destination_points,
    )?;

    Ok(StructuredPopulationTransitionResult {
        destinations,
        destination_points,
        provenance,
    })
}

fn derive_destination_cut(
    schema: &HereditarySchema,
    structure: &PopulationStructureProfile,
    source_populations: &BTreeMap<PopulationId, PopulationGeneticState>,
    source_points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    source_snapshot: &MetapopulationSnapshot,
    transition_id: &PopulationTransitionId,
    process_profile: &PopulationProcessProfile,
) -> Result<
    (
        BTreeMap<PopulationId, PopulationGeneticState>,
        BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    ),
    EvolutionError,
> {
    source_snapshot.validate_current(
        schema,
        structure,
        source_populations,
        source_points,
    )?;

    // Exact zero-migration theorem: use the pre-existing neutral implementation
    // rather than reimplementing an equivalent distribution here.
    if structure.is_zero_migration() {
        let mut destinations = BTreeMap::new();
        let mut destination_points = BTreeMap::new();
        for population_id in structure.populations() {
            let source = source_populations
                .get(population_id)
                .ok_or(EvolutionError::MetapopulationSetMismatch)?;
            let source_point = source_points
                .get(population_id)
                .ok_or(EvolutionError::MetapopulationSetMismatch)?;
            let result = neutral_wright_fisher_step(
                schema,
                source,
                source_point,
                transition_id,
                process_profile,
            )?;
            destinations.insert(population_id.clone(), result.destination);
            destination_points.insert(population_id.clone(), result.destination_point);
        }
        return Ok((destinations, destination_points));
    }

    let process_digest = process_profile.canonical_digest()?;
    let generation_from = source_snapshot.generation();
    let generation_to = PopulationGeneration(
        generation_from
            .0
            .checked_add(1)
            .ok_or(EvolutionError::CountOverflow)?,
    );
    let source_distributions = parental_source_distributions(structure)?;
    let mut destinations = BTreeMap::new();

    for destination_id in structure.populations() {
        let destination_source = source_populations
            .get(destination_id)
            .ok_or(EvolutionError::MetapopulationSetMismatch)?;
        let destination_total_copies = destination_source
            .census_individuals
            .checked_mul(u64::from(schema.ploidy))
            .ok_or(EvolutionError::CountOverflow)?;
        let distribution = source_distributions
            .get(destination_id)
            .ok_or(EvolutionError::SamplingInvariantViolation)?;
        let mut destination_counts = BTreeMap::new();

        for locus_id in schema.loci.keys() {
            let mut counts: BTreeMap<AlleleId, u64> = BTreeMap::new();
            for destination_copy in 0..destination_total_copies {
                let source_choice = structured_draw_below(
                    SOURCE_CHOICE_RNG_DOMAIN,
                    destination_id,
                    None,
                    source_snapshot.experiment_id(),
                    transition_id,
                    generation_from,
                    &process_digest,
                    locus_id,
                    destination_copy,
                    u64::from(PROBABILITY_SCALE_PPM),
                )?;
                let source_id = source_at_ppm(distribution, source_choice as u32)?;
                let source = source_populations
                    .get(source_id)
                    .ok_or(EvolutionError::MetapopulationSetMismatch)?;
                let source_counts = source
                    .allele_copy_counts
                    .get(locus_id)
                    .ok_or_else(|| EvolutionError::MissingLocus(locus_id.clone()))?;
                let source_total_copies = source
                    .census_individuals
                    .checked_mul(u64::from(schema.ploidy))
                    .ok_or(EvolutionError::CountOverflow)?;
                let ordinal = structured_draw_below(
                    SOURCE_ALLELE_RNG_DOMAIN,
                    destination_id,
                    Some(source_id),
                    source_snapshot.experiment_id(),
                    transition_id,
                    generation_from,
                    &process_digest,
                    locus_id,
                    destination_copy,
                    source_total_copies,
                )?;
                let allele = allele_at_ordinal(source_counts, ordinal)?;
                let count = counts.entry(allele).or_insert(0);
                *count = count.checked_add(1).ok_or(EvolutionError::CountOverflow)?;
            }
            destination_counts.insert(locus_id.clone(), counts);
        }

        let destination = PopulationGeneticState::from_counts(
            destination_id.clone(),
            schema,
            destination_source.census_individuals,
            destination_counts,
        )?;
        destinations.insert(destination_id.clone(), destination);
    }

    let mut destination_points = BTreeMap::new();
    for (population_id, destination) in &destinations {
        let point = PopulationTrajectoryPoint::from_validated_state(
            schema,
            destination,
            source_snapshot.experiment_id().clone(),
            generation_to,
        )?;
        destination_points.insert(population_id.clone(), point);
    }
    Ok((destinations, destination_points))
}

fn parental_source_distributions(
    structure: &PopulationStructureProfile,
) -> Result<BTreeMap<PopulationId, Vec<(PopulationId, u32)>>, EvolutionError> {
    structure.validate()?;
    let mut result = BTreeMap::new();
    for destination in structure.populations() {
        let mut row = Vec::new();
        let mut total = 0_u64;
        for source in structure.populations() {
            let rate = structure.parental_source_probability_ppm(destination, source)?;
            if rate == 0 {
                continue;
            }
            total = total
                .checked_add(u64::from(rate))
                .ok_or(EvolutionError::CountOverflow)?;
            row.push((source.clone(), rate));
        }
        if total != u64::from(PROBABILITY_SCALE_PPM) {
            return Err(EvolutionError::SamplingInvariantViolation);
        }
        result.insert(destination.clone(), row);
    }
    Ok(result)
}

fn source_at_ppm(
    distribution: &[(PopulationId, u32)],
    variate_ppm: u32,
) -> Result<&PopulationId, EvolutionError> {
    if variate_ppm >= PROBABILITY_SCALE_PPM {
        return Err(EvolutionError::SamplingInvariantViolation);
    }
    let mut upper = 0_u64;
    for (source, probability) in distribution {
        upper = upper
            .checked_add(u64::from(*probability))
            .ok_or(EvolutionError::CountOverflow)?;
        if u64::from(variate_ppm) < upper {
            return Ok(source);
        }
    }
    Err(EvolutionError::SamplingInvariantViolation)
}

fn allele_at_ordinal(
    counts: &BTreeMap<AlleleId, u64>,
    ordinal: u64,
) -> Result<AlleleId, EvolutionError> {
    let mut lower = 0_u64;
    for (allele, count) in counts {
        let upper = lower
            .checked_add(*count)
            .ok_or(EvolutionError::CountOverflow)?;
        if ordinal < upper {
            return Ok(allele.clone());
        }
        lower = upper;
    }
    Err(EvolutionError::SamplingInvariantViolation)
}

#[allow(clippy::too_many_arguments)]
fn structured_draw_below(
    domain: &[u8],
    destination: &PopulationId,
    source: Option<&PopulationId>,
    experiment_id: &EvolutionExperimentId,
    transition_id: &PopulationTransitionId,
    generation_from: PopulationGeneration,
    process_digest: &PopulationProcessProfileDigest,
    locus: &LocusId,
    destination_copy: u64,
    upper: u64,
) -> Result<u64, EvolutionError> {
    if upper == 0 {
        return Err(EvolutionError::InvalidDrawUpperBound);
    }
    if upper == 1 {
        return Ok(0);
    }

    let zone = u64::MAX - (u64::MAX % upper);
    let mut attempt = 0_u64;
    loop {
        let mut digest = Sha256::new();
        digest.update(domain);
        put_text(&mut digest, destination.as_str());
        match source {
            Some(source) => {
                digest.update([1]);
                put_text(&mut digest, source.as_str());
            }
            None => digest.update([0]),
        }
        put_text(&mut digest, experiment_id.as_str());
        put_text(&mut digest, transition_id.as_str());
        put_u64(&mut digest, generation_from.0);
        digest.update(process_digest.as_bytes());
        put_text(&mut digest, locus.as_str());
        put_u64(&mut digest, destination_copy);
        put_u64(&mut digest, attempt);
        let bytes: [u8; 32] = digest.finalize().into();
        let value = u64::from_le_bytes(
            bytes[..8]
                .try_into()
                .expect("SHA-256 output has an 8-byte prefix"),
        );
        if value < zone {
            return Ok(value % upper);
        }
        attempt = attempt
            .checked_add(1)
            .ok_or(EvolutionError::CountOverflow)?;
    }
}

fn validate_destination_cut(
    schema: &HereditarySchema,
    structure: &PopulationStructureProfile,
    experiment_id: &EvolutionExperimentId,
    generation: PopulationGeneration,
    destinations: &BTreeMap<PopulationId, PopulationGeneticState>,
    destination_points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
) -> Result<(), EvolutionError> {
    let destination_keys: std::collections::BTreeSet<_> = destinations.keys().cloned().collect();
    let point_keys: std::collections::BTreeSet<_> = destination_points.keys().cloned().collect();
    if &destination_keys != structure.populations() || &point_keys != structure.populations() {
        return Err(EvolutionError::MetapopulationSetMismatch);
    }

    for population_id in structure.populations() {
        let destination = destinations
            .get(population_id)
            .ok_or(EvolutionError::MetapopulationSetMismatch)?;
        if &destination.population_id != population_id {
            return Err(EvolutionError::MetapopulationStateKeyMismatch {
                key: population_id.clone(),
                observed: destination.population_id.clone(),
            });
        }
        destination.validate(schema)?;
        let point = destination_points
            .get(population_id)
            .ok_or(EvolutionError::MetapopulationSetMismatch)?;
        if point.population_id() != population_id {
            return Err(EvolutionError::MetapopulationPointKeyMismatch {
                key: population_id.clone(),
                observed: point.population_id().clone(),
            });
        }
        point.validate_current(schema, destination)?;
        point.validate_experiment(experiment_id)?;
        if point.generation() != generation {
            return Err(EvolutionError::PopulationGenerationMismatch);
        }
    }
    Ok(())
}

fn population_digest_map(
    schema: &HereditarySchema,
    populations: &BTreeMap<PopulationId, PopulationGeneticState>,
) -> Result<BTreeMap<PopulationId, PopulationGeneticStateDigest>, EvolutionError> {
    populations
        .iter()
        .map(|(id, population)| Ok((id.clone(), population.canonical_digest(schema)?)))
        .collect()
}

fn trajectory_digest_map(
    points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
) -> BTreeMap<PopulationId, PopulationTrajectoryPointDigest> {
    points
        .iter()
        .map(|(id, point)| (id.clone(), point.canonical_digest()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        HereditarySchemaId, LocusDefinition, ParentalSourceEdge, PopulationProcessProfileId,
        PopulationStructureModel, PopulationStructureProfileId,
    };

    fn allele(id: &str) -> AlleleId {
        AlleleId::new(id).unwrap()
    }

    fn pop(id: &str) -> PopulationId {
        PopulationId::new(id).unwrap()
    }

    fn schema() -> HereditarySchema {
        HereditarySchema::new(
            HereditarySchemaId::new("structured-wf-v0").unwrap(),
            2,
            vec![LocusDefinition::new(
                LocusId::new("focal").unwrap(),
                [allele("a"), allele("b")],
            )
            .unwrap()],
        )
        .unwrap()
    }

    fn process_profile() -> PopulationProcessProfile {
        PopulationProcessProfile {
            profile_id: PopulationProcessProfileId::new("neutral-wf").unwrap(),
            version: "v1".into(),
            model: PopulationProcessModel::NeutralIndependentLocusWrightFisher,
        }
    }

    fn state(
        schema: &HereditarySchema,
        id: PopulationId,
        census: u64,
        a_copies: u64,
    ) -> PopulationGeneticState {
        let total = census * u64::from(schema.ploidy);
        let mut allele_counts = BTreeMap::new();
        if a_copies > 0 {
            allele_counts.insert(allele("a"), a_copies);
        }
        if a_copies < total {
            allele_counts.insert(allele("b"), total - a_copies);
        }
        PopulationGeneticState::from_counts(
            id,
            schema,
            census,
            BTreeMap::from([(LocusId::new("focal").unwrap(), allele_counts)]),
        )
        .unwrap()
    }

    fn source_cut(
        schema: &HereditarySchema,
        populations: BTreeMap<PopulationId, PopulationGeneticState>,
    ) -> (
        BTreeMap<PopulationId, PopulationGeneticState>,
        BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    ) {
        let experiment = EvolutionExperimentId::new("structured-exp").unwrap();
        let points = populations
            .iter()
            .map(|(id, population)| {
                (
                    id.clone(),
                    PopulationTrajectoryPoint::declare_reference_start(
                        schema,
                        population,
                        experiment.clone(),
                        PopulationGeneration(4),
                    )
                    .unwrap(),
                )
            })
            .collect();
        (populations, points)
    }

    fn structure(
        populations: Vec<PopulationId>,
        edges: Vec<ParentalSourceEdge>,
    ) -> PopulationStructureProfile {
        PopulationStructureProfile::new(
            PopulationStructureProfileId::new("islands").unwrap(),
            "v1",
            PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
            populations,
            edges,
        )
        .unwrap()
    }

    #[test]
    fn zero_migration_is_pathwise_identical_to_independent_neutral_steps() {
        let schema = schema();
        let a = pop("a");
        let b = pop("b");
        let structure = structure(vec![a.clone(), b.clone()], vec![]);
        let (populations, points) = source_cut(
            &schema,
            BTreeMap::from([
                (a.clone(), state(&schema, a.clone(), 5, 6)),
                (b.clone(), state(&schema, b.clone(), 7, 5)),
            ]),
        );
        let snapshot =
            MetapopulationSnapshot::capture_reference(&schema, &structure, &populations, &points)
                .unwrap();
        let transition = PopulationTransitionId::new("generation-step").unwrap();
        let profile = process_profile();
        let structured = structured_wright_fisher_step(
            &schema,
            &structure,
            &populations,
            &points,
            &snapshot,
            &transition,
            &profile,
        )
        .unwrap();

        for id in [&a, &b] {
            let neutral = neutral_wright_fisher_step(
                &schema,
                populations.get(id).unwrap(),
                points.get(id).unwrap(),
                &transition,
                &profile,
            )
            .unwrap();
            assert_eq!(structured.destinations.get(id), Some(&neutral.destination));
            assert_eq!(
                structured.destination_points.get(id),
                Some(&neutral.destination_point)
            );
        }
    }

    #[test]
    fn full_cyclic_migration_reads_one_immutable_source_generation() {
        let schema = schema();
        let a = pop("a");
        let b = pop("b");
        let structure = structure(
            vec![a.clone(), b.clone()],
            vec![
                ParentalSourceEdge::new(a.clone(), b.clone(), PROBABILITY_SCALE_PPM).unwrap(),
                ParentalSourceEdge::new(b.clone(), a.clone(), PROBABILITY_SCALE_PPM).unwrap(),
            ],
        );
        let (populations, points) = source_cut(
            &schema,
            BTreeMap::from([
                (a.clone(), state(&schema, a.clone(), 4, 8)),
                (b.clone(), state(&schema, b.clone(), 6, 0)),
            ]),
        );
        let snapshot =
            MetapopulationSnapshot::capture_reference(&schema, &structure, &populations, &points)
                .unwrap();
        let result = structured_wright_fisher_step(
            &schema,
            &structure,
            &populations,
            &points,
            &snapshot,
            &PopulationTransitionId::new("generation-step").unwrap(),
            &process_profile(),
        )
        .unwrap();
        let focal = LocusId::new("focal").unwrap();

        assert_eq!(result.destinations[&a].census_individuals, 4);
        assert_eq!(result.destinations[&b].census_individuals, 6);
        assert_eq!(
            result.destinations[&a].allele_copy_counts[&focal],
            BTreeMap::from([(allele("b"), 8)])
        );
        assert_eq!(
            result.destinations[&b].allele_copy_counts[&focal],
            BTreeMap::from([(allele("a"), 12)])
        );
    }

    #[test]
    fn all_destination_points_advance_one_generation_in_same_experiment() {
        let schema = schema();
        let a = pop("a");
        let b = pop("b");
        let structure = structure(
            vec![a.clone(), b.clone()],
            vec![ParentalSourceEdge::new(a.clone(), b.clone(), 250_000).unwrap()],
        );
        let (populations, points) = source_cut(
            &schema,
            BTreeMap::from([
                (a.clone(), state(&schema, a.clone(), 5, 5)),
                (b.clone(), state(&schema, b.clone(), 5, 5)),
            ]),
        );
        let snapshot =
            MetapopulationSnapshot::capture_reference(&schema, &structure, &populations, &points)
                .unwrap();
        let result = structured_wright_fisher_step(
            &schema,
            &structure,
            &populations,
            &points,
            &snapshot,
            &PopulationTransitionId::new("generation-step").unwrap(),
            &process_profile(),
        )
        .unwrap();

        for point in result.destination_points.values() {
            assert_eq!(point.generation(), PopulationGeneration(5));
            assert_eq!(point.experiment_id(), snapshot.experiment_id());
        }
    }

    #[test]
    fn changed_structure_stales_structured_provenance() {
        let schema = schema();
        let a = pop("a");
        let b = pop("b");
        let structure = structure(
            vec![a.clone(), b.clone()],
            vec![ParentalSourceEdge::new(a.clone(), b.clone(), 100_000).unwrap()],
        );
        let (populations, points) = source_cut(
            &schema,
            BTreeMap::from([
                (a.clone(), state(&schema, a.clone(), 5, 6)),
                (b.clone(), state(&schema, b.clone(), 5, 4)),
            ]),
        );
        let snapshot =
            MetapopulationSnapshot::capture_reference(&schema, &structure, &populations, &points)
                .unwrap();
        let transition = PopulationTransitionId::new("generation-step").unwrap();
        let profile = process_profile();
        let result = structured_wright_fisher_step(
            &schema,
            &structure,
            &populations,
            &points,
            &snapshot,
            &transition,
            &profile,
        )
        .unwrap();
        let changed = structure(
            vec![a.clone(), b.clone()],
            vec![ParentalSourceEdge::new(a, b, 100_001).unwrap()],
        );

        assert!(result
            .provenance
            .validate_current(
                &schema,
                &changed,
                &populations,
                &points,
                &snapshot,
                &transition,
                &profile,
                &result.destinations,
                &result.destination_points,
            )
            .is_err());
    }
}
