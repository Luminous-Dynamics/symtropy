use crate::{
    canonical::{fmt_hex, put_u32}, neutral_wright_fisher_step, ChromosomeMap,
    HereditarySchema, LinkedCensusProjection, LinkedCensusProjectionDigest, MutationFateSubject,
    PopulationGeneticStateDigest, PopulationProcessModel, PopulationProcessProfile,
    PopulationTrajectoryPoint, PopulationTransitionId, PopulationTransitionProvenanceDigest,
    PopulationTransitionResult,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

pub const AGGREGATE_RESOLUTION_LOSS_VERSION: u32 = 1;
const LOSS_DIGEST_DOMAIN: &[u8] = b"symtropy:evolution:aggregate-resolution-loss:v1\0";
const CONTINUATION_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:allele-only-aggregate-continuation:v1\0";

/// Semantic information-loss profile for one aggregate fidelity transition.
///
/// V1 retains aggregate allele counts and trajectory/provenance authority while
/// explicitly ceasing to represent individual/genotype/phase/ancestry/mutation-origin history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AggregateResolutionLossProfile {
    AlleleOnlyNeutralIndependentLocusV1,
}

impl AggregateResolutionLossProfile {
    fn tag(self) -> u8 {
        match self {
            Self::AlleleOnlyNeutralIndependentLocusV1 => 0,
        }
    }

    /// Exact destination allele-copy counts remain represented.
    pub fn retains_allele_copy_counts(self) -> bool { true }

    /// Exact linked individuals/genotypes are no longer represented.
    pub fn retains_individual_genotypes(self) -> bool { false }

    /// Haplotype phase/linkage is no longer represented.
    pub fn retains_haplotype_phase(self) -> bool { false }

    /// Persistent ancestry-copy identity is no longer represented.
    pub fn retains_ancestry_copy_identity(self) -> bool { false }

    /// Active mutation-origin partition/counts are no longer represented.
    pub fn retains_mutation_origin_partition(self) -> bool { false }

    /// Mutation predecessor/history chains are no longer represented.
    pub fn retains_mutation_history(self) -> bool { false }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AggregateResolutionLossCertificate {
    version: u32,
    profile: AggregateResolutionLossProfile,
    source_projection_digest: LinkedCensusProjectionDigest,
    source_mutation_fate_digest: crate::MutationFateObservationDigest,
    source_population_digest: PopulationGeneticStateDigest,
    transition_digest: PopulationTransitionProvenanceDigest,
    destination_population_digest: PopulationGeneticStateDigest,
}

impl AggregateResolutionLossCertificate {
    pub fn profile(&self) -> AggregateResolutionLossProfile { self.profile }
    pub fn source_projection_digest(&self) -> LinkedCensusProjectionDigest {
        self.source_projection_digest
    }
    pub fn transition_digest(&self) -> PopulationTransitionProvenanceDigest {
        self.transition_digest
    }

    pub fn canonical_digest(&self) -> AggregateResolutionLossCertificateDigest {
        let mut digest = Sha256::new();
        digest.update(LOSS_DIGEST_DOMAIN);
        put_u32(&mut digest, self.version);
        digest.update([self.profile.tag()]);
        digest.update(self.source_projection_digest.as_bytes());
        digest.update(self.source_mutation_fate_digest.as_bytes());
        digest.update(self.source_population_digest.as_bytes());
        digest.update(self.transition_digest.as_bytes());
        digest.update(self.destination_population_digest.as_bytes());
        AggregateResolutionLossCertificateDigest(digest.finalize().into())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AggregateResolutionLossCertificateDigest([u8; 32]);

impl AggregateResolutionLossCertificateDigest {
    pub fn as_bytes(&self) -> &[u8; 32] { &self.0 }
}

impl fmt::Debug for AggregateResolutionLossCertificateDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AggregateResolutionLossCertificateDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for AggregateResolutionLossCertificateDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { fmt_hex(&self.0, f) }
}

/// One allele-only aggregate continuation from an exact linked-census projection.
///
/// Deliberately contains no destination `MutationFateObservation`; origin history
/// is not represented after this allele-only transition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlleleOnlyAggregateContinuation {
    pub transition: PopulationTransitionResult,
    pub loss: AggregateResolutionLossCertificate,
}

impl AlleleOnlyAggregateContinuation {
    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        &self,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        source_projection: &LinkedCensusProjection,
        subjects: &[MutationFateSubject<'_>],
        source_point: &PopulationTrajectoryPoint,
        transition_id: &PopulationTransitionId,
        profile: &PopulationProcessProfile,
    ) -> Result<(), AggregateResolutionLossError> {
        let recomputed = continue_projected_census_alleles_only(
            schema,
            chromosome_map,
            source_projection,
            subjects,
            source_point,
            transition_id,
            profile,
        )?;
        if recomputed != *self {
            return Err(AggregateResolutionLossError::ReplayMismatch);
        }
        Ok(())
    }

    pub fn canonical_digest(&self) -> AlleleOnlyAggregateContinuationDigest {
        let mut digest = Sha256::new();
        digest.update(CONTINUATION_DIGEST_DOMAIN);
        digest.update(self.transition.provenance.canonical_digest().as_bytes());
        digest.update(self.loss.canonical_digest().as_bytes());
        AlleleOnlyAggregateContinuationDigest(digest.finalize().into())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AlleleOnlyAggregateContinuationDigest([u8; 32]);

impl AlleleOnlyAggregateContinuationDigest {
    pub fn as_bytes(&self) -> &[u8; 32] { &self.0 }
}

impl fmt::Debug for AlleleOnlyAggregateContinuationDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AlleleOnlyAggregateContinuationDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for AlleleOnlyAggregateContinuationDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { fmt_hex(&self.0, f) }
}

#[allow(clippy::too_many_arguments)]
pub fn continue_projected_census_alleles_only(
    schema: &HereditarySchema,
    chromosome_map: &ChromosomeMap,
    source_projection: &LinkedCensusProjection,
    subjects: &[MutationFateSubject<'_>],
    source_point: &PopulationTrajectoryPoint,
    transition_id: &PopulationTransitionId,
    profile: &PopulationProcessProfile,
) -> Result<AlleleOnlyAggregateContinuation, AggregateResolutionLossError> {
    source_projection.validate_current(
        source_projection.population_id(),
        schema,
        chromosome_map,
        subjects,
    )?;
    if profile.model != PopulationProcessModel::NeutralIndependentLocusWrightFisher {
        return Err(AggregateResolutionLossError::UnsupportedProcessModel);
    }

    let transition = neutral_wright_fisher_step(
        schema,
        &source_projection.aggregate_population,
        source_point,
        transition_id,
        profile,
    )?;
    let loss = AggregateResolutionLossCertificate {
        version: AGGREGATE_RESOLUTION_LOSS_VERSION,
        profile: AggregateResolutionLossProfile::AlleleOnlyNeutralIndependentLocusV1,
        source_projection_digest: source_projection.canonical_digest(schema)?,
        source_mutation_fate_digest: source_projection.mutation_fate_digest(),
        source_population_digest: source_projection.aggregate_population_digest(),
        transition_digest: transition.provenance.canonical_digest(),
        destination_population_digest: transition.destination.canonical_digest(schema)?,
    };
    Ok(AlleleOnlyAggregateContinuation { transition, loss })
}

#[derive(Debug)]
pub enum AggregateResolutionLossError {
    Evolution(crate::EvolutionError),
    Projection(crate::LinkedCensusProjectionError),
    UnsupportedProcessModel,
    ReplayMismatch,
}

impl From<crate::EvolutionError> for AggregateResolutionLossError {
    fn from(value: crate::EvolutionError) -> Self { Self::Evolution(value) }
}

impl From<crate::LinkedCensusProjectionError> for AggregateResolutionLossError {
    fn from(value: crate::LinkedCensusProjectionError) -> Self { Self::Projection(value) }
}

impl fmt::Display for AggregateResolutionLossError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evolution(error) => write!(f, "evolution authority error: {error}"),
            Self::Projection(error) => write!(f, "linked-census projection error: {error}"),
            Self::UnsupportedProcessModel => write!(f, "aggregate continuation requires the neutral independent-locus Wright-Fisher model"),
            Self::ReplayMismatch => write!(f, "restored allele-only aggregate continuation does not replay"),
        }
    }
}

impl Error for AggregateResolutionLossError {}
