use crate::{
    canonical::{fmt_hex, put_text, put_u64},
    error::validate_text,
    AlleleId, EvolutionError, HereditarySchema, HereditarySchemaDigest, LocusId,
    PopulationGeneticState, PopulationGeneticStateDigest, PopulationId,
    PopulationProcessProfileId, PopulationTransitionId,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fmt};

const PROFILE_DIGEST_DOMAIN: &[u8] = b"symtropy:evolution:population-process-profile:v1\0";
const TRANSITION_DIGEST_DOMAIN: &[u8] = b"symtropy:evolution:population-transition:v1\0";
const DRIFT_RNG_DOMAIN: &[u8] = b"symtropy:evolution:neutral-wright-fisher-draw:v1\0";

/// Canonical generation coordinate for generation-based reference processes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PopulationGeneration(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PopulationProcessModel {
    /// Fixed-census neutral Wright-Fisher resampling of allele copies at each
    /// unlinked/unphased locus independently.
    ///
    /// This is a marginal population-genetics reference model. It does not
    /// preserve genotype, haplotype, linkage, ancestry, or individual identity.
    NeutralIndependentLocusWrightFisher,
}

impl PopulationProcessModel {
    fn tag(self) -> u8 {
        match self {
            Self::NeutralIndependentLocusWrightFisher => 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PopulationProcessProfile {
    pub profile_id: PopulationProcessProfileId,
    pub version: String,
    pub model: PopulationProcessModel,
}

impl PopulationProcessProfile {
    pub fn validate(&self) -> Result<(), EvolutionError> {
        validate_text("population_process.version", &self.version)
    }

    pub fn canonical_digest(&self) -> Result<PopulationProcessProfileDigest, EvolutionError> {
        self.validate()?;
        let mut digest = Sha256::new();
        digest.update(PROFILE_DIGEST_DOMAIN);
        put_text(&mut digest, self.profile_id.as_str());
        put_text(&mut digest, &self.version);
        digest.update([self.model.tag()]);
        Ok(PopulationProcessProfileDigest(digest.finalize().into()))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PopulationProcessProfileDigest([u8; 32]);

impl PopulationProcessProfileDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for PopulationProcessProfileDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PopulationProcessProfileDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for PopulationProcessProfileDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

/// Revalidatable evidence for one aggregate population transition.
///
/// A restored receipt is data until `validate_current` succeeds against the
/// exact current schema/source/profile/destination. It does not create
/// individual ancestry or claim that any exact organism existed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PopulationTransitionProvenance {
    schema_digest: HereditarySchemaDigest,
    source_population_id: PopulationId,
    transition_id: PopulationTransitionId,
    profile_digest: PopulationProcessProfileDigest,
    generation_from: PopulationGeneration,
    generation_to: PopulationGeneration,
    source_digest: PopulationGeneticStateDigest,
    destination_digest: PopulationGeneticStateDigest,
}

impl PopulationTransitionProvenance {
    pub fn schema_digest(&self) -> HereditarySchemaDigest {
        self.schema_digest
    }

    pub fn transition_id(&self) -> &PopulationTransitionId {
        &self.transition_id
    }

    pub fn profile_digest(&self) -> PopulationProcessProfileDigest {
        self.profile_digest
    }

    pub fn generation_from(&self) -> PopulationGeneration {
        self.generation_from
    }

    pub fn generation_to(&self) -> PopulationGeneration {
        self.generation_to
    }

    pub fn source_digest(&self) -> PopulationGeneticStateDigest {
        self.source_digest
    }

    pub fn destination_digest(&self) -> PopulationGeneticStateDigest {
        self.destination_digest
    }

    pub fn canonical_digest(&self) -> PopulationTransitionProvenanceDigest {
        let mut digest = Sha256::new();
        digest.update(TRANSITION_DIGEST_DOMAIN);
        digest.update(self.schema_digest.as_bytes());
        put_text(&mut digest, self.source_population_id.as_str());
        put_text(&mut digest, self.transition_id.as_str());
        digest.update(self.profile_digest.as_bytes());
        put_u64(&mut digest, self.generation_from.0);
        put_u64(&mut digest, self.generation_to.0);
        digest.update(self.source_digest.as_bytes());
        digest.update(self.destination_digest.as_bytes());
        PopulationTransitionProvenanceDigest(digest.finalize().into())
    }

    pub fn validate_current(
        &self,
        schema: &HereditarySchema,
        source: &PopulationGeneticState,
        profile: &PopulationProcessProfile,
        destination: &PopulationGeneticState,
    ) -> Result<(), EvolutionError> {
        source.validate(schema)?;
        destination.validate(schema)?;
        profile.validate()?;

        if schema.canonical_digest()? != self.schema_digest {
            return Err(EvolutionError::HereditarySchemaAuthorityMismatch);
        }
        if source.population_id != self.source_population_id
            || destination.population_id != self.source_population_id
        {
            return Err(EvolutionError::PopulationIdentityMismatch);
        }
        if profile.canonical_digest()? != self.profile_digest {
            return Err(EvolutionError::PopulationProcessAuthorityMismatch);
        }
        if source.canonical_digest(schema)? != self.source_digest {
            return Err(EvolutionError::PopulationSourceMismatch);
        }
        if destination.canonical_digest(schema)? != self.destination_digest {
            return Err(EvolutionError::PopulationDestinationMismatch);
        }
        if self.generation_to.0 != self.generation_from.0.checked_add(1).ok_or(EvolutionError::CountOverflow)? {
            return Err(EvolutionError::PopulationGenerationMismatch);
        }

        let recomputed = derive_destination(
            schema,
            source,
            &self.transition_id,
            self.generation_from,
            profile,
        )?;
        if recomputed != *destination {
            return Err(EvolutionError::PopulationTransitionMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PopulationTransitionProvenanceDigest([u8; 32]);

impl PopulationTransitionProvenanceDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for PopulationTransitionProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PopulationTransitionProvenanceDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for PopulationTransitionProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PopulationTransitionResult {
    pub destination: PopulationGeneticState,
    pub provenance: PopulationTransitionProvenance,
}

/// Advance one fixed-census neutral independent-locus Wright-Fisher generation.
///
/// Each destination allele copy samples one source allele copy uniformly at the
/// same locus. The keyed draw is independent of iteration order and of other
/// loci, so adding an unrelated locus cannot shift an existing locus's drift
/// stream. This is intentionally not an individual/genotype simulator.
pub fn neutral_wright_fisher_step(
    schema: &HereditarySchema,
    source: &PopulationGeneticState,
    transition_id: &PopulationTransitionId,
    generation_from: PopulationGeneration,
    profile: &PopulationProcessProfile,
) -> Result<PopulationTransitionResult, EvolutionError> {
    schema.validate()?;
    source.validate(schema)?;
    profile.validate()?;
    if profile.model != PopulationProcessModel::NeutralIndependentLocusWrightFisher {
        return Err(EvolutionError::PopulationProcessModelMismatch);
    }

    let destination = derive_destination(
        schema,
        source,
        transition_id,
        generation_from,
        profile,
    )?;
    let provenance = PopulationTransitionProvenance {
        schema_digest: schema.canonical_digest()?,
        source_population_id: source.population_id.clone(),
        transition_id: transition_id.clone(),
        profile_digest: profile.canonical_digest()?,
        generation_from,
        generation_to: PopulationGeneration(
            generation_from.0.checked_add(1).ok_or(EvolutionError::CountOverflow)?,
        ),
        source_digest: source.canonical_digest(schema)?,
        destination_digest: destination.canonical_digest(schema)?,
    };
    provenance.validate_current(schema, source, profile, &destination)?;

    Ok(PopulationTransitionResult {
        destination,
        provenance,
    })
}

fn derive_destination(
    schema: &HereditarySchema,
    source: &PopulationGeneticState,
    transition_id: &PopulationTransitionId,
    generation_from: PopulationGeneration,
    profile: &PopulationProcessProfile,
) -> Result<PopulationGeneticState, EvolutionError> {
    let total_copies = source
        .census_individuals
        .checked_mul(u64::from(schema.ploidy))
        .ok_or(EvolutionError::CountOverflow)?;

    let mut destination_counts = BTreeMap::new();
    for locus_id in schema.loci.keys() {
        let source_counts = source
            .allele_copy_counts
            .get(locus_id)
            .ok_or_else(|| EvolutionError::MissingLocus(locus_id.clone()))?;
        let mut counts: BTreeMap<AlleleId, u64> = BTreeMap::new();

        for destination_copy in 0..total_copies {
            let ordinal = drift_draw_below(
                source,
                transition_id,
                generation_from,
                profile,
                locus_id,
                destination_copy,
                total_copies,
            )?;
            let allele = allele_at_ordinal(source_counts, ordinal)?;
            let count = counts.entry(allele).or_insert(0);
            *count = count.checked_add(1).ok_or(EvolutionError::CountOverflow)?;
        }
        destination_counts.insert(locus_id.clone(), counts);
    }

    PopulationGeneticState::from_counts(
        source.population_id.clone(),
        schema,
        source.census_individuals,
        destination_counts,
    )
}

fn allele_at_ordinal(
    counts: &BTreeMap<AlleleId, u64>,
    ordinal: u64,
) -> Result<AlleleId, EvolutionError> {
    let mut lower = 0_u64;
    for (allele, count) in counts {
        let upper = lower.checked_add(*count).ok_or(EvolutionError::CountOverflow)?;
        if ordinal < upper {
            return Ok(allele.clone());
        }
        lower = upper;
    }
    Err(EvolutionError::SamplingInvariantViolation)
}

fn drift_draw_below(
    source: &PopulationGeneticState,
    transition_id: &PopulationTransitionId,
    generation_from: PopulationGeneration,
    profile: &PopulationProcessProfile,
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
        digest.update(DRIFT_RNG_DOMAIN);
        put_text(&mut digest, source.population_id.as_str());
        put_text(&mut digest, transition_id.as_str());
        put_u64(&mut digest, generation_from.0);
        put_text(&mut digest, profile.profile_id.as_str());
        put_text(&mut digest, &profile.version);
        digest.update([profile.model.tag()]);
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
        attempt = attempt.checked_add(1).ok_or(EvolutionError::CountOverflow)?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HereditarySchemaId, LocusDefinition};

    fn allele(id: &str) -> AlleleId {
        AlleleId::new(id).unwrap()
    }

    fn locus(id: &str, alleles: &[&str]) -> LocusDefinition {
        LocusDefinition::new(
            LocusId::new(id).unwrap(),
            alleles.iter().map(|id| allele(id)),
        )
        .unwrap()
    }

    fn schema(loci: Vec<LocusDefinition>) -> HereditarySchema {
        HereditarySchema::new(HereditarySchemaId::new("wf-v0").unwrap(), 2, loci).unwrap()
    }

    fn profile() -> PopulationProcessProfile {
        PopulationProcessProfile {
            profile_id: PopulationProcessProfileId::new("neutral-wf").unwrap(),
            version: "v1".into(),
            model: PopulationProcessModel::NeutralIndependentLocusWrightFisher,
        }
    }

    fn population(
        schema: &HereditarySchema,
        locus_counts: &[(&str, &[(&str, u64)])],
        census: u64,
    ) -> PopulationGeneticState {
        let counts = locus_counts
            .iter()
            .map(|(locus, alleles)| {
                (
                    LocusId::new(*locus).unwrap(),
                    alleles
                        .iter()
                        .map(|(allele_id, count)| (allele(allele_id), *count))
                        .collect(),
                )
            })
            .collect();
        PopulationGeneticState::from_counts(
            PopulationId::new("island-a").unwrap(),
            schema,
            census,
            counts,
        )
        .unwrap()
    }

    #[test]
    fn exact_replay_and_provenance_revalidation() {
        let schema = schema(vec![locus("pigment", &["dark", "light"])]);
        let source = population(
            &schema,
            &[("pigment", &[("dark", 6), ("light", 4)])],
            5,
        );
        let transition = PopulationTransitionId::new("generation-12-to-13").unwrap();
        let profile = profile();

        let first = neutral_wright_fisher_step(
            &schema,
            &source,
            &transition,
            PopulationGeneration(12),
            &profile,
        )
        .unwrap();
        let second = neutral_wright_fisher_step(
            &schema,
            &source,
            &transition,
            PopulationGeneration(12),
            &profile,
        )
        .unwrap();

        assert_eq!(first, second);
        first
            .provenance
            .validate_current(&schema, &source, &profile, &first.destination)
            .unwrap();
    }

    #[test]
    fn fixed_allele_is_absorbing_under_neutral_reference_process() {
        let schema = schema(vec![locus("pigment", &["dark", "light"])]);
        let source = population(&schema, &[("pigment", &[("dark", 10)])], 5);
        let result = neutral_wright_fisher_step(
            &schema,
            &source,
            &PopulationTransitionId::new("fixed").unwrap(),
            PopulationGeneration(0),
            &profile(),
        )
        .unwrap();

        assert_eq!(result.destination, source);
    }

    #[test]
    fn unrelated_locus_does_not_shift_existing_locus_drift() {
        let schema_a = schema(vec![locus("pigment", &["dark", "light"])]);
        let source_a = population(
            &schema_a,
            &[("pigment", &[("dark", 6), ("light", 4)])],
            5,
        );
        let schema_ab = schema(vec![
            locus("pigment", &["dark", "light"]),
            locus("enzyme", &["slow", "fast"]),
        ]);
        let source_ab = population(
            &schema_ab,
            &[
                ("pigment", &[("dark", 6), ("light", 4)]),
                ("enzyme", &[("slow", 5), ("fast", 5)]),
            ],
            5,
        );
        let transition = PopulationTransitionId::new("keyed-locus-drift").unwrap();
        let profile = profile();

        let result_a = neutral_wright_fisher_step(
            &schema_a,
            &source_a,
            &transition,
            PopulationGeneration(4),
            &profile,
        )
        .unwrap();
        let result_ab = neutral_wright_fisher_step(
            &schema_ab,
            &source_ab,
            &transition,
            PopulationGeneration(4),
            &profile,
        )
        .unwrap();
        let pigment = LocusId::new("pigment").unwrap();

        assert_eq!(
            result_a.destination.allele_copy_counts.get(&pigment),
            result_ab.destination.allele_copy_counts.get(&pigment)
        );
    }

    #[test]
    fn transition_preserves_exact_copy_count_at_every_locus() {
        let schema = schema(vec![
            locus("pigment", &["dark", "light"]),
            locus("enzyme", &["slow", "fast"]),
        ]);
        let source = population(
            &schema,
            &[
                ("pigment", &[("dark", 12), ("light", 8)]),
                ("enzyme", &[("slow", 7), ("fast", 13)]),
            ],
            10,
        );
        let result = neutral_wright_fisher_step(
            &schema,
            &source,
            &PopulationTransitionId::new("copy-conservation").unwrap(),
            PopulationGeneration(99),
            &profile(),
        )
        .unwrap();

        result.destination.validate(&schema).unwrap();
        for counts in result.destination.allele_copy_counts.values() {
            assert_eq!(counts.values().sum::<u64>(), 20);
        }
    }
}
