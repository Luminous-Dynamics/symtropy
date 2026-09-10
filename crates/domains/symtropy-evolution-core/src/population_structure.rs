use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64},
    error::validate_text,
    EvolutionError, PopulationId, PopulationStructureProfileId, PROBABILITY_SCALE_PPM,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::{BTreeMap, BTreeSet}, fmt};

const STRUCTURE_PROFILE_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:population-structure-profile:v1\0";

/// Exact interpretation of a migration matrix in the aggregate reference lane.
///
/// Rows are destination populations. Off-diagonal entries are the per-generation
/// probability, in integer ppm, that one destination allele copy chooses its
/// parental gene pool from the named source population. The self/stay
/// probability is implicit as one minus the off-diagonal row sum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PopulationStructureModel {
    DestinationParentalSourceFractionsPpmV1,
}

impl PopulationStructureModel {
    fn tag(self) -> u8 {
        match self {
            Self::DestinationParentalSourceFractionsPpmV1 => 0,
        }
    }
}

/// Canonical structured-population authority for aggregate gene-pool migration.
///
/// This profile says nothing about literal migrant organisms, genotypes,
/// haplotypes, kinship, or ancestry. Those facts do not exist at the current
/// marginal allele-count fidelity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PopulationStructureProfile {
    profile_id: PopulationStructureProfileId,
    version: String,
    model: PopulationStructureModel,
    populations: BTreeSet<PopulationId>,
    /// destination -> source -> off-diagonal parental-source probability (ppm)
    parental_source_ppm: BTreeMap<PopulationId, BTreeMap<PopulationId, u32>>,
}

impl PopulationStructureProfile {
    /// Construct a canonical migration profile from declared populations and
    /// explicit `(destination, source, rate_ppm)` off-diagonal edges.
    ///
    /// Duplicate populations and duplicate edges are rejected before canonical
    /// collection. Zero-rate edges are accepted as input but canonicalized away;
    /// raw/deserialized zero edges are rejected by `validate()`.
    pub fn new(
        profile_id: PopulationStructureProfileId,
        version: impl Into<String>,
        model: PopulationStructureModel,
        populations: impl IntoIterator<Item = PopulationId>,
        migration_edges: impl IntoIterator<Item = (PopulationId, PopulationId, u32)>,
    ) -> Result<Self, EvolutionError> {
        let version = version.into();
        validate_text("population_structure.version", &version)?;

        let mut population_set = BTreeSet::new();
        for population in populations {
            if !population_set.insert(population.clone()) {
                return Err(EvolutionError::DuplicatePopulationIdentity { population });
            }
        }
        if population_set.is_empty() {
            return Err(EvolutionError::NoStructuredPopulations);
        }

        let mut seen_edges = BTreeSet::new();
        let mut parental_source_ppm: BTreeMap<
            PopulationId,
            BTreeMap<PopulationId, u32>,
        > = BTreeMap::new();

        for (destination, source, rate_ppm) in migration_edges {
            if !seen_edges.insert((destination.clone(), source.clone())) {
                return Err(EvolutionError::DuplicateMigrationEdge {
                    destination,
                    source,
                });
            }
            if !population_set.contains(&destination) {
                return Err(EvolutionError::UnknownStructurePopulation {
                    population: destination,
                });
            }
            if !population_set.contains(&source) {
                return Err(EvolutionError::UnknownStructurePopulation { population: source });
            }
            if destination == source {
                return Err(EvolutionError::SelfMigrationEntry {
                    population: destination,
                });
            }
            if rate_ppm > PROBABILITY_SCALE_PPM {
                return Err(EvolutionError::ProbabilityOutOfRange {
                    observed_ppm: rate_ppm,
                });
            }
            if rate_ppm == 0 {
                continue;
            }

            parental_source_ppm
                .entry(destination)
                .or_default()
                .insert(source, rate_ppm);
        }

        let profile = Self {
            profile_id,
            version,
            model,
            populations: population_set,
            parental_source_ppm,
        };
        profile.validate()?;
        Ok(profile)
    }

    pub fn profile_id(&self) -> &PopulationStructureProfileId {
        &self.profile_id
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn model(&self) -> PopulationStructureModel {
        self.model
    }

    pub fn populations(&self) -> &BTreeSet<PopulationId> {
        &self.populations
    }

    pub fn migration_rows(
        &self,
    ) -> &BTreeMap<PopulationId, BTreeMap<PopulationId, u32>> {
        &self.parental_source_ppm
    }

    pub fn is_zero_migration(&self) -> bool {
        self.parental_source_ppm.is_empty()
    }

    pub fn contains_population(&self, population: &PopulationId) -> bool {
        self.populations.contains(population)
    }

    /// Return the implicit probability that a destination draws from its own
    /// parental gene pool in one generation.
    pub fn stay_probability_ppm(
        &self,
        destination: &PopulationId,
    ) -> Result<u32, EvolutionError> {
        self.validate()?;
        if !self.populations.contains(destination) {
            return Err(EvolutionError::UnknownStructurePopulation {
                population: destination.clone(),
            });
        }
        let outgoing_to_other_sources = self
            .parental_source_ppm
            .get(destination)
            .map(|row| row.values().map(|rate| u64::from(*rate)).sum::<u64>())
            .unwrap_or(0);
        Ok(u32::try_from(u64::from(PROBABILITY_SCALE_PPM) - outgoing_to_other_sources)
            .expect("validated migration row cannot exceed probability scale"))
    }

    /// Return the destination's complete parental-source probability for one
    /// declared source. Asking for `destination == source` returns the implicit
    /// stay probability; absent off-diagonal edges return zero.
    pub fn parental_source_probability_ppm(
        &self,
        destination: &PopulationId,
        source: &PopulationId,
    ) -> Result<u32, EvolutionError> {
        self.validate()?;
        if !self.populations.contains(destination) {
            return Err(EvolutionError::UnknownStructurePopulation {
                population: destination.clone(),
            });
        }
        if !self.populations.contains(source) {
            return Err(EvolutionError::UnknownStructurePopulation {
                population: source.clone(),
            });
        }
        if destination == source {
            return self.stay_probability_ppm(destination);
        }
        Ok(self
            .parental_source_ppm
            .get(destination)
            .and_then(|row| row.get(source))
            .copied()
            .unwrap_or(0))
    }

    pub fn validate(&self) -> Result<(), EvolutionError> {
        validate_text("population_structure.version", &self.version)?;
        if self.populations.is_empty() {
            return Err(EvolutionError::NoStructuredPopulations);
        }

        for (destination, row) in &self.parental_source_ppm {
            if !self.populations.contains(destination) {
                return Err(EvolutionError::UnknownStructurePopulation {
                    population: destination.clone(),
                });
            }
            if row.is_empty() {
                return Err(EvolutionError::NonCanonicalEmptyMigrationRow {
                    destination: destination.clone(),
                });
            }

            let mut row_sum = 0_u64;
            for (source, rate_ppm) in row {
                if !self.populations.contains(source) {
                    return Err(EvolutionError::UnknownStructurePopulation {
                        population: source.clone(),
                    });
                }
                if destination == source {
                    return Err(EvolutionError::SelfMigrationEntry {
                        population: destination.clone(),
                    });
                }
                if *rate_ppm == 0 {
                    return Err(EvolutionError::NonCanonicalZeroMigration {
                        destination: destination.clone(),
                        source: source.clone(),
                    });
                }
                if *rate_ppm > PROBABILITY_SCALE_PPM {
                    return Err(EvolutionError::ProbabilityOutOfRange {
                        observed_ppm: *rate_ppm,
                    });
                }
                row_sum = row_sum
                    .checked_add(u64::from(*rate_ppm))
                    .ok_or(EvolutionError::CountOverflow)?;
            }
            if row_sum > u64::from(PROBABILITY_SCALE_PPM) {
                return Err(EvolutionError::MigrationRowExceedsProbabilityScale {
                    destination: destination.clone(),
                    observed_ppm: row_sum,
                });
            }
        }
        Ok(())
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<PopulationStructureProfileDigest, EvolutionError> {
        self.validate()?;
        let mut digest = Sha256::new();
        digest.update(STRUCTURE_PROFILE_DIGEST_DOMAIN);
        put_text(&mut digest, self.profile_id.as_str());
        put_text(&mut digest, &self.version);
        digest.update([self.model.tag()]);

        put_u64(&mut digest, self.populations.len() as u64);
        for population in &self.populations {
            put_text(&mut digest, population.as_str());
        }

        put_u64(&mut digest, self.parental_source_ppm.len() as u64);
        for (destination, row) in &self.parental_source_ppm {
            put_text(&mut digest, destination.as_str());
            put_u64(&mut digest, row.len() as u64);
            for (source, rate_ppm) in row {
                put_text(&mut digest, source.as_str());
                put_u32(&mut digest, *rate_ppm);
            }
        }

        Ok(PopulationStructureProfileDigest(digest.finalize().into()))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PopulationStructureProfileDigest([u8; 32]);

impl PopulationStructureProfileDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for PopulationStructureProfileDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PopulationStructureProfileDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for PopulationStructureProfileDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pop(id: &str) -> PopulationId {
        PopulationId::new(id).unwrap()
    }

    fn profile(
        populations: Vec<PopulationId>,
        edges: Vec<(PopulationId, PopulationId, u32)>,
    ) -> Result<PopulationStructureProfile, EvolutionError> {
        PopulationStructureProfile::new(
            PopulationStructureProfileId::new("archipelago-v1").unwrap(),
            "v1",
            PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
            populations,
            edges,
        )
    }

    #[test]
    fn insertion_order_does_not_change_structure_authority() {
        let a = pop("a");
        let b = pop("b");
        let c = pop("c");
        let first = profile(
            vec![a.clone(), b.clone(), c.clone()],
            vec![
                (a.clone(), b.clone(), 100_000),
                (a.clone(), c.clone(), 50_000),
                (b.clone(), a.clone(), 25_000),
            ],
        )
        .unwrap();
        let second = profile(
            vec![c.clone(), a.clone(), b.clone()],
            vec![
                (b, a.clone(), 25_000),
                (a.clone(), c, 50_000),
                (a, pop("b"), 100_000),
            ],
        )
        .unwrap();

        assert_eq!(first.canonical_digest().unwrap(), second.canonical_digest().unwrap());
        assert_eq!(first, second);
    }

    #[test]
    fn changing_one_rate_changes_exact_authority() {
        let a = pop("a");
        let b = pop("b");
        let first = profile(
            vec![a.clone(), b.clone()],
            vec![(a.clone(), b.clone(), 100_000)],
        )
        .unwrap();
        let second = profile(vec![a.clone(), b.clone()], vec![(a, b, 100_001)]).unwrap();

        assert_ne!(first.canonical_digest().unwrap(), second.canonical_digest().unwrap());
    }

    #[test]
    fn zero_edges_are_canonicalized_away_by_constructor() {
        let a = pop("a");
        let b = pop("b");
        let with_zero = profile(
            vec![a.clone(), b.clone()],
            vec![(a.clone(), b.clone(), 0)],
        )
        .unwrap();
        let without_edge = profile(vec![a, b], vec![]).unwrap();

        assert!(with_zero.is_zero_migration());
        assert_eq!(with_zero, without_edge);
        assert_eq!(
            with_zero.canonical_digest().unwrap(),
            without_edge.canonical_digest().unwrap()
        );
    }

    #[test]
    fn raw_zero_and_empty_rows_fail_as_noncanonical() {
        let a = pop("a");
        let b = pop("b");
        let mut raw_zero = profile(vec![a.clone(), b.clone()], vec![]).unwrap();
        raw_zero
            .parental_source_ppm
            .insert(a.clone(), BTreeMap::from([(b.clone(), 0)]));
        assert!(matches!(
            raw_zero.validate(),
            Err(EvolutionError::NonCanonicalZeroMigration { .. })
        ));

        let mut raw_empty = profile(vec![a.clone(), b], vec![]).unwrap();
        raw_empty.parental_source_ppm.insert(a, BTreeMap::new());
        assert!(matches!(
            raw_empty.validate(),
            Err(EvolutionError::NonCanonicalEmptyMigrationRow { .. })
        ));
    }

    #[test]
    fn duplicate_populations_and_edges_fail_before_canonicalization() {
        let a = pop("a");
        let b = pop("b");
        assert!(matches!(
            profile(vec![a.clone(), a.clone()], vec![]),
            Err(EvolutionError::DuplicatePopulationIdentity { .. })
        ));
        assert!(matches!(
            profile(
                vec![a.clone(), b.clone()],
                vec![
                    (a.clone(), b.clone(), 0),
                    (a.clone(), b.clone(), 10_000),
                ],
            ),
            Err(EvolutionError::DuplicateMigrationEdge { .. })
        ));
    }

    #[test]
    fn self_and_unknown_population_edges_fail_closed() {
        let a = pop("a");
        let b = pop("b");
        assert!(matches!(
            profile(vec![a.clone(), b.clone()], vec![(a.clone(), a, 1)]),
            Err(EvolutionError::SelfMigrationEntry { .. })
        ));
        assert!(matches!(
            profile(
                vec![b.clone()],
                vec![(b, PopulationId::new("missing").unwrap(), 1)],
            ),
            Err(EvolutionError::UnknownStructurePopulation { .. })
        ));
    }

    #[test]
    fn off_diagonal_row_cannot_exceed_one() {
        let a = pop("a");
        let b = pop("b");
        let c = pop("c");
        assert!(matches!(
            profile(
                vec![a.clone(), b.clone(), c.clone()],
                vec![(a.clone(), b, 600_000), (a, c, 500_001)],
            ),
            Err(EvolutionError::MigrationRowExceedsProbabilityScale { .. })
        ));
    }

    #[test]
    fn complete_parental_source_probabilities_include_implicit_stay() {
        let a = pop("a");
        let b = pop("b");
        let c = pop("c");
        let structure = profile(
            vec![a.clone(), b.clone(), c.clone()],
            vec![
                (a.clone(), b.clone(), 125_000),
                (a.clone(), c.clone(), 75_000),
            ],
        )
        .unwrap();

        assert_eq!(structure.stay_probability_ppm(&a).unwrap(), 800_000);
        assert_eq!(
            structure.parental_source_probability_ppm(&a, &a).unwrap(),
            800_000
        );
        assert_eq!(
            structure.parental_source_probability_ppm(&a, &b).unwrap(),
            125_000
        );
        assert_eq!(
            structure.parental_source_probability_ppm(&a, &c).unwrap(),
            75_000
        );
        assert_eq!(
            structure.parental_source_probability_ppm(&b, &c).unwrap(),
            0
        );
    }
}
