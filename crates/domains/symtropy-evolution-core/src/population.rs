use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64},
    AlleleId, EvolutionError, HereditarySchema, HereditarySchemaDigest,
    HereditarySchemaId, HereditaryState, LocusId, PopulationId,
    EVOLUTION_SCHEMA_VERSION, PROBABILITY_SCALE_PPM,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fmt};

const POPULATION_DIGEST_DOMAIN: &[u8] = b"symtropy:evolution:population-genetics:v1\0";

/// Aggregate allele-copy state. It intentionally stores no historical
/// individual genomes and therefore cannot by itself establish micro-history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PopulationGeneticState {
    pub schema_version: u32,
    pub population_id: PopulationId,
    pub hereditary_schema_id: HereditarySchemaId,
    pub hereditary_schema_digest: HereditarySchemaDigest,
    pub census_individuals: u64,
    pub allele_copy_counts: BTreeMap<LocusId, BTreeMap<AlleleId, u64>>,
}

impl PopulationGeneticState {
    pub fn from_individuals(
        population_id: PopulationId,
        schema: &HereditarySchema,
        individuals: &[HereditaryState],
    ) -> Result<Self, EvolutionError> {
        schema.validate()?;
        if individuals.is_empty() {
            return Err(EvolutionError::EmptyPopulation);
        }

        let mut counts: BTreeMap<LocusId, BTreeMap<AlleleId, u64>> = schema
            .loci
            .keys()
            .cloned()
            .map(|locus| (locus, BTreeMap::new()))
            .collect();

        for individual in individuals {
            individual.validate(schema)?;
            for (locus, copies) in &individual.copies {
                let locus_counts = counts
                    .get_mut(locus)
                    .expect("validated state uses schema loci only");
                for allele in copies {
                    let count = locus_counts.entry(allele.clone()).or_insert(0);
                    *count = count
                        .checked_add(1)
                        .ok_or(EvolutionError::CountOverflow)?;
                }
            }
        }

        let census_individuals = u64::try_from(individuals.len())
            .map_err(|_| EvolutionError::CountOverflow)?;
        let state = Self {
            schema_version: EVOLUTION_SCHEMA_VERSION,
            population_id,
            hereditary_schema_id: schema.id.clone(),
            hereditary_schema_digest: schema.canonical_digest()?,
            census_individuals,
            allele_copy_counts: counts,
        };
        state.validate(schema)?;
        Ok(state)
    }

    /// Construct aggregate state directly from declared allele-copy counts.
    /// No exact individual genomes are reconstructed by this operation.
    ///
    /// Zero-count allele entries carry no population state in V0, so this
    /// constructor removes them before validation. Raw/deserialized state that
    /// contains zero-count entries fails validation as noncanonical.
    pub fn from_counts(
        population_id: PopulationId,
        schema: &HereditarySchema,
        census_individuals: u64,
        mut allele_copy_counts: BTreeMap<LocusId, BTreeMap<AlleleId, u64>>,
    ) -> Result<Self, EvolutionError> {
        for counts in allele_copy_counts.values_mut() {
            counts.retain(|_, count| *count != 0);
        }
        let state = Self {
            schema_version: EVOLUTION_SCHEMA_VERSION,
            population_id,
            hereditary_schema_id: schema.id.clone(),
            hereditary_schema_digest: schema.canonical_digest()?,
            census_individuals,
            allele_copy_counts,
        };
        state.validate(schema)?;
        Ok(state)
    }

    pub fn validate(&self, schema: &HereditarySchema) -> Result<(), EvolutionError> {
        schema.validate()?;
        if self.schema_version != EVOLUTION_SCHEMA_VERSION {
            return Err(EvolutionError::UnsupportedSchema(self.schema_version));
        }
        if self.hereditary_schema_id != schema.id {
            return Err(EvolutionError::HereditarySchemaMismatch);
        }
        if self.hereditary_schema_digest != schema.canonical_digest()? {
            return Err(EvolutionError::HereditarySchemaAuthorityMismatch);
        }
        if self.census_individuals == 0 {
            return Err(EvolutionError::EmptyPopulation);
        }
        if self.allele_copy_counts.len() != schema.loci.len() {
            return Err(EvolutionError::LocusSetMismatch);
        }

        let expected_per_locus = self
            .census_individuals
            .checked_mul(u64::from(schema.ploidy))
            .ok_or(EvolutionError::CountOverflow)?;

        for (locus_id, locus) in &schema.loci {
            let counts = self
                .allele_copy_counts
                .get(locus_id)
                .ok_or_else(|| EvolutionError::MissingLocus(locus_id.clone()))?;
            let mut total = 0_u64;
            for (allele, count) in counts {
                if !locus.allowed_alleles.contains(allele) {
                    return Err(EvolutionError::UnknownAllele {
                        locus: locus_id.clone(),
                        allele: allele.clone(),
                    });
                }
                if *count == 0 {
                    return Err(EvolutionError::NonCanonicalZeroAlleleCount {
                        locus: locus_id.clone(),
                        allele: allele.clone(),
                    });
                }
                total = total
                    .checked_add(*count)
                    .ok_or(EvolutionError::CountOverflow)?;
            }
            if total != expected_per_locus {
                return Err(EvolutionError::PopulationCopyTotalMismatch {
                    locus: locus_id.clone(),
                    expected: expected_per_locus,
                    observed: total,
                });
            }
        }
        Ok(())
    }

    pub fn allele_frequency_ppm(
        &self,
        schema: &HereditarySchema,
        locus: &LocusId,
        allele: &AlleleId,
    ) -> Result<u32, EvolutionError> {
        self.validate(schema)?;
        let locus_definition = schema
            .loci
            .get(locus)
            .ok_or_else(|| EvolutionError::MissingLocus(locus.clone()))?;
        if !locus_definition.allowed_alleles.contains(allele) {
            return Err(EvolutionError::UnknownAllele {
                locus: locus.clone(),
                allele: allele.clone(),
            });
        }

        let total = self
            .census_individuals
            .checked_mul(u64::from(schema.ploidy))
            .ok_or(EvolutionError::CountOverflow)?;
        let count = self
            .allele_copy_counts
            .get(locus)
            .and_then(|counts| counts.get(allele))
            .copied()
            .unwrap_or(0);
        let ppm = count
            .checked_mul(u64::from(PROBABILITY_SCALE_PPM))
            .ok_or(EvolutionError::CountOverflow)?
            / total;
        Ok(ppm as u32)
    }

    pub fn canonical_digest(
        &self,
        schema: &HereditarySchema,
    ) -> Result<PopulationGeneticStateDigest, EvolutionError> {
        self.validate(schema)?;
        let mut digest = Sha256::new();
        digest.update(POPULATION_DIGEST_DOMAIN);
        put_u32(&mut digest, self.schema_version);
        put_text(&mut digest, self.population_id.as_str());
        put_text(&mut digest, self.hereditary_schema_id.as_str());
        digest.update(self.hereditary_schema_digest.as_bytes());
        put_u64(&mut digest, self.census_individuals);
        put_u64(&mut digest, self.allele_copy_counts.len() as u64);
        for (locus, counts) in &self.allele_copy_counts {
            put_text(&mut digest, locus.as_str());
            put_u64(&mut digest, counts.len() as u64);
            for (allele, count) in counts {
                put_text(&mut digest, allele.as_str());
                put_u64(&mut digest, *count);
            }
        }
        Ok(PopulationGeneticStateDigest(digest.finalize().into()))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PopulationGeneticStateDigest([u8; 32]);

impl PopulationGeneticStateDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for PopulationGeneticStateDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PopulationGeneticStateDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for PopulationGeneticStateDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HereditarySchemaId, LocusDefinition};

    fn allele(id: &str) -> AlleleId {
        AlleleId::new(id).unwrap()
    }

    fn schema() -> HereditarySchema {
        HereditarySchema::new(
            HereditarySchemaId::new("population-test").unwrap(),
            2,
            vec![LocusDefinition::new(
                LocusId::new("pigment").unwrap(),
                [allele("dark"), allele("light")],
            )
            .unwrap()],
        )
        .unwrap()
    }

    fn individual(schema: &HereditarySchema, copies: [&str; 2]) -> HereditaryState {
        HereditaryState::new(
            schema,
            BTreeMap::from([(
                LocusId::new("pigment").unwrap(),
                copies.into_iter().map(allele).collect(),
            )]),
        )
        .unwrap()
    }

    #[test]
    fn exact_allele_copy_frequencies_are_derived_without_micro_history() {
        let schema = schema();
        let population = PopulationGeneticState::from_individuals(
            PopulationId::new("island-a").unwrap(),
            &schema,
            &[
                individual(&schema, ["dark", "light"]),
                individual(&schema, ["dark", "dark"]),
            ],
        )
        .unwrap();
        let locus = LocusId::new("pigment").unwrap();

        assert_eq!(
            population
                .allele_frequency_ppm(&schema, &locus, &allele("dark"))
                .unwrap(),
            750_000
        );
        assert_eq!(
            population
                .allele_frequency_ppm(&schema, &locus, &allele("light"))
                .unwrap(),
            250_000
        );
    }

    #[test]
    fn malformed_aggregate_counts_fail_closed() {
        let schema = schema();
        let counts = BTreeMap::from([(
            LocusId::new("pigment").unwrap(),
            BTreeMap::from([(allele("dark"), 2), (allele("light"), 1)]),
        )]);
        let result = PopulationGeneticState::from_counts(
            PopulationId::new("island-a").unwrap(),
            &schema,
            2,
            counts,
        );
        assert!(matches!(
            result,
            Err(EvolutionError::PopulationCopyTotalMismatch { .. })
        ));
    }

    #[test]
    fn constructor_canonicalizes_zero_count_alleles() {
        let schema = schema();
        let population = PopulationGeneticState::from_counts(
            PopulationId::new("island-a").unwrap(),
            &schema,
            2,
            BTreeMap::from([(
                LocusId::new("pigment").unwrap(),
                BTreeMap::from([(allele("dark"), 4), (allele("light"), 0)]),
            )]),
        )
        .unwrap();

        let counts = population
            .allele_copy_counts
            .get(&LocusId::new("pigment").unwrap())
            .unwrap();
        assert_eq!(counts, &BTreeMap::from([(allele("dark"), 4)]));
    }

    #[test]
    fn raw_zero_count_entry_is_noncanonical() {
        let schema = schema();
        let mut population = PopulationGeneticState::from_counts(
            PopulationId::new("island-a").unwrap(),
            &schema,
            2,
            BTreeMap::from([(
                LocusId::new("pigment").unwrap(),
                BTreeMap::from([(allele("dark"), 4)]),
            )]),
        )
        .unwrap();
        population
            .allele_copy_counts
            .get_mut(&LocusId::new("pigment").unwrap())
            .unwrap()
            .insert(allele("light"), 0);

        assert!(matches!(
            population.validate(&schema),
            Err(EvolutionError::NonCanonicalZeroAlleleCount { .. })
        ));
    }

    #[test]
    fn same_schema_name_with_changed_content_stales_population_authority() {
        let schema_a = schema();
        let schema_b = HereditarySchema::new(
            HereditarySchemaId::new("population-test").unwrap(),
            2,
            vec![LocusDefinition::new(
                LocusId::new("pigment").unwrap(),
                [allele("dark"), allele("light"), allele("red")],
            )
            .unwrap()],
        )
        .unwrap();
        let population = PopulationGeneticState::from_individuals(
            PopulationId::new("island-a").unwrap(),
            &schema_a,
            &[individual(&schema_a, ["dark", "light"])],
        )
        .unwrap();

        assert_eq!(
            population.validate(&schema_b),
            Err(EvolutionError::HereditarySchemaAuthorityMismatch)
        );
    }
}
