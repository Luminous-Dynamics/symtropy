use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64},
    DemographicEventId, EvolutionError, EvolutionExperimentId, HereditarySchema,
    HereditarySchemaDigest, MetapopulationSnapshot, MetapopulationSnapshotDigest,
    PopulationGeneration, PopulationGeneticState, PopulationId, PopulationStructureProfile,
    PopulationStructureProfileDigest, PopulationTrajectoryPoint, PROBABILITY_SCALE_PPM,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

const DEMOGRAPHIC_EVENT_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:demographic-event-declaration:v1\0";

/// V0 timing convention for discrete demographic interventions.
///
/// The event consumes one exact generation-G metapopulation snapshot and is
/// interpreted as an instantaneous intervention before reproduction from G.
/// Runtime successors should therefore produce a post-event source cut still at
/// generation G; the ordinary population process subsequently advances to G+1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DemographicEventTiming {
    BeforeReproductionFromSourceGenerationV1,
}

impl DemographicEventTiming {
    fn tag(self) -> u8 {
        match self {
            Self::BeforeReproductionFromSourceGenerationV1 => 0,
        }
    }
}

/// One declared daughter population for an aggregate population split.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaughterPopulation {
    population_id: PopulationId,
    census_individuals: u64,
}

impl DaughterPopulation {
    pub fn new(
        population_id: PopulationId,
        census_individuals: u64,
    ) -> Result<Self, EvolutionError> {
        if census_individuals == 0 {
            return Err(EvolutionError::DemographicCensusMustBePositive);
        }
        Ok(Self {
            population_id,
            census_individuals,
        })
    }

    pub fn population_id(&self) -> &PopulationId {
        &self.population_id
    }

    pub fn census_individuals(&self) -> u64 {
        self.census_individuals
    }
}

/// Exact declaration of one discrete demographic event at aggregate fidelity.
///
/// These variants describe population-level transformations only. They do not
/// create exact migrant/founder organisms, haplotypes, pedigrees, kinship,
/// chromosome ancestry, ecological causes, or sex/age structure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DemographicEventKind {
    CensusResize {
        population: PopulationId,
        target_census: u64,
    },
    FounderEvent {
        source: PopulationId,
        founded_population: PopulationId,
        founder_census: u64,
    },
    PopulationSplit {
        source: PopulationId,
        daughters: Vec<DaughterPopulation>,
    },
    PulseAdmixture {
        destination: PopulationId,
        source: PopulationId,
        source_fraction_ppm: u32,
    },
    Extinction {
        population: PopulationId,
    },
    Recolonization {
        source: PopulationId,
        recolonized_population: PopulationId,
        founder_census: u64,
    },
}

impl DemographicEventKind {
    /// Canonicalize semantically unordered collections before authority minting.
    fn canonicalized(mut self) -> Result<Self, EvolutionError> {
        if let Self::PopulationSplit { daughters, .. } = &mut self {
            let mut seen = BTreeSet::new();
            for daughter in daughters.iter() {
                if daughter.census_individuals == 0 {
                    return Err(EvolutionError::DemographicCensusMustBePositive);
                }
                if !seen.insert(daughter.population_id.clone()) {
                    return Err(EvolutionError::DuplicateDemographicDaughter {
                        population: daughter.population_id.clone(),
                    });
                }
            }
            daughters.sort_by(|a, b| a.population_id.cmp(&b.population_id));
        }
        self.validate_local_canonical()?;
        Ok(self)
    }

    /// Validate representation-local invariants without granting current-world
    /// authority. This is sufficient for safe canonical hashing of restored
    /// event-shaped data, but not for execution.
    fn validate_local_canonical(&self) -> Result<(), EvolutionError> {
        match self {
            Self::CensusResize { target_census, .. } => require_positive(*target_census)?,
            Self::FounderEvent { founder_census, .. }
            | Self::Recolonization { founder_census, .. } => require_positive(*founder_census)?,
            Self::PopulationSplit { daughters, .. } => {
                if daughters.len() < 2 {
                    return Err(EvolutionError::DemographicSplitRequiresTwoDaughters);
                }
                let mut previous: Option<&PopulationId> = None;
                for daughter in daughters {
                    require_positive(daughter.census_individuals)?;
                    if let Some(prev) = previous {
                        if prev == &daughter.population_id {
                            return Err(EvolutionError::DuplicateDemographicDaughter {
                                population: daughter.population_id.clone(),
                            });
                        }
                        if prev > &daughter.population_id {
                            return Err(EvolutionError::NonCanonicalDemographicDaughterOrder);
                        }
                    }
                    previous = Some(&daughter.population_id);
                }
            }
            Self::PulseAdmixture {
                destination,
                source,
                source_fraction_ppm,
            } => {
                if destination == source {
                    return Err(EvolutionError::DemographicSelfAdmixture {
                        population: destination.clone(),
                    });
                }
                if *source_fraction_ppm == 0 || *source_fraction_ppm > PROBABILITY_SCALE_PPM {
                    return Err(EvolutionError::DemographicAdmixtureFractionOutOfRange {
                        observed_ppm: *source_fraction_ppm,
                    });
                }
            }
            Self::Extinction { .. } => {}
        }
        Ok(())
    }

    fn validate_canonical(
        &self,
        structure: &PopulationStructureProfile,
    ) -> Result<(), EvolutionError> {
        self.validate_local_canonical()?;
        match self {
            Self::CensusResize { population, .. } => require_present(structure, population)?,
            Self::FounderEvent {
                source,
                founded_population,
                ..
            } => {
                require_present(structure, source)?;
                require_absent(structure, founded_population)?;
            }
            Self::PopulationSplit { source, daughters } => {
                require_present(structure, source)?;
                for daughter in daughters {
                    require_absent(structure, &daughter.population_id)?;
                }
            }
            Self::PulseAdmixture {
                destination, source, ..
            } => {
                require_present(structure, destination)?;
                require_present(structure, source)?;
            }
            Self::Extinction { population } => require_present(structure, population)?,
            Self::Recolonization {
                source,
                recolonized_population,
                ..
            } => {
                require_present(structure, source)?;
                require_absent(structure, recolonized_population)?;
            }
        }
        Ok(())
    }

    fn write_canonical(&self, digest: &mut Sha256) {
        match self {
            Self::CensusResize {
                population,
                target_census,
            } => {
                digest.update([0]);
                put_text(digest, population.as_str());
                put_u64(digest, *target_census);
            }
            Self::FounderEvent {
                source,
                founded_population,
                founder_census,
            } => {
                digest.update([1]);
                put_text(digest, source.as_str());
                put_text(digest, founded_population.as_str());
                put_u64(digest, *founder_census);
            }
            Self::PopulationSplit { source, daughters } => {
                digest.update([2]);
                put_text(digest, source.as_str());
                put_u64(digest, daughters.len() as u64);
                for daughter in daughters {
                    put_text(digest, daughter.population_id.as_str());
                    put_u64(digest, daughter.census_individuals);
                }
            }
            Self::PulseAdmixture {
                destination,
                source,
                source_fraction_ppm,
            } => {
                digest.update([3]);
                put_text(digest, destination.as_str());
                put_text(digest, source.as_str());
                put_u32(digest, *source_fraction_ppm);
            }
            Self::Extinction { population } => {
                digest.update([4]);
                put_text(digest, population.as_str());
            }
            Self::Recolonization {
                source,
                recolonized_population,
                founder_census,
            } => {
                digest.update([5]);
                put_text(digest, source.as_str());
                put_text(digest, recolonized_population.as_str());
                put_u64(digest, *founder_census);
            }
        }
    }
}

fn require_positive(value: u64) -> Result<(), EvolutionError> {
    if value == 0 {
        Err(EvolutionError::DemographicCensusMustBePositive)
    } else {
        Ok(())
    }
}

fn require_present(
    structure: &PopulationStructureProfile,
    population: &PopulationId,
) -> Result<(), EvolutionError> {
    if structure.contains_population(population) {
        Ok(())
    } else {
        Err(EvolutionError::UnknownDemographicPopulation {
            population: population.clone(),
        })
    }
}

fn require_absent(
    structure: &PopulationStructureProfile,
    population: &PopulationId,
) -> Result<(), EvolutionError> {
    if structure.contains_population(population) {
        Err(EvolutionError::DemographicPopulationAlreadyExists {
            population: population.clone(),
        })
    } else {
        Ok(())
    }
}

/// Revalidatable declaration of one discrete demographic intervention.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemographicEventDeclaration {
    event_id: DemographicEventId,
    version: String,
    timing: DemographicEventTiming,
    kind: DemographicEventKind,
    schema_digest: HereditarySchemaDigest,
    structure_digest: PopulationStructureProfileDigest,
    source_snapshot_digest: MetapopulationSnapshotDigest,
    experiment_id: EvolutionExperimentId,
    generation: PopulationGeneration,
}

impl DemographicEventDeclaration {
    #[allow(clippy::too_many_arguments)]
    pub fn declare_current(
        event_id: DemographicEventId,
        version: impl Into<String>,
        kind: DemographicEventKind,
        schema: &HereditarySchema,
        structure: &PopulationStructureProfile,
        populations: &BTreeMap<PopulationId, PopulationGeneticState>,
        points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
        source_snapshot: &MetapopulationSnapshot,
    ) -> Result<Self, EvolutionError> {
        let version = version.into();
        crate::error::validate_text("demographic_event.version", &version)?;
        schema.validate()?;
        structure.validate()?;
        source_snapshot.validate_current(schema, structure, populations, points)?;

        let kind = kind.canonicalized()?;
        kind.validate_canonical(structure)?;

        let declaration = Self {
            event_id,
            version,
            timing: DemographicEventTiming::BeforeReproductionFromSourceGenerationV1,
            kind,
            schema_digest: schema.canonical_digest()?,
            structure_digest: structure.canonical_digest()?,
            source_snapshot_digest: source_snapshot.canonical_digest(),
            experiment_id: source_snapshot.experiment_id().clone(),
            generation: source_snapshot.generation(),
        };
        declaration.validate_current(schema, structure, populations, points, source_snapshot)?;
        Ok(declaration)
    }

    pub fn event_id(&self) -> &DemographicEventId {
        &self.event_id
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn timing(&self) -> DemographicEventTiming {
        self.timing
    }

    pub fn kind(&self) -> &DemographicEventKind {
        &self.kind
    }

    pub fn experiment_id(&self) -> &EvolutionExperimentId {
        &self.experiment_id
    }

    pub fn generation(&self) -> PopulationGeneration {
        self.generation
    }

    pub fn source_snapshot_digest(&self) -> MetapopulationSnapshotDigest {
        self.source_snapshot_digest
    }

    pub fn validate_current(
        &self,
        schema: &HereditarySchema,
        structure: &PopulationStructureProfile,
        populations: &BTreeMap<PopulationId, PopulationGeneticState>,
        points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
        source_snapshot: &MetapopulationSnapshot,
    ) -> Result<(), EvolutionError> {
        crate::error::validate_text("demographic_event.version", &self.version)?;
        self.kind.validate_local_canonical()?;
        schema.validate()?;
        structure.validate()?;
        source_snapshot.validate_current(schema, structure, populations, points)?;

        if self.timing != DemographicEventTiming::BeforeReproductionFromSourceGenerationV1 {
            return Err(EvolutionError::DemographicEventTimingMismatch);
        }
        if schema.canonical_digest()? != self.schema_digest {
            return Err(EvolutionError::HereditarySchemaAuthorityMismatch);
        }
        if structure.canonical_digest()? != self.structure_digest {
            return Err(EvolutionError::DemographicStructureAuthorityMismatch);
        }
        if source_snapshot.canonical_digest() != self.source_snapshot_digest {
            return Err(EvolutionError::DemographicSourceSnapshotMismatch);
        }
        if source_snapshot.experiment_id() != &self.experiment_id {
            return Err(EvolutionError::PopulationExperimentMismatch);
        }
        if source_snapshot.generation() != self.generation {
            return Err(EvolutionError::PopulationGenerationMismatch);
        }
        self.kind.validate_canonical(structure)?;
        Ok(())
    }

    pub fn canonical_digest(&self) -> Result<DemographicEventDeclarationDigest, EvolutionError> {
        crate::error::validate_text("demographic_event.version", &self.version)?;
        self.kind.validate_local_canonical()?;
        let mut digest = Sha256::new();
        digest.update(DEMOGRAPHIC_EVENT_DIGEST_DOMAIN);
        put_text(&mut digest, self.event_id.as_str());
        put_text(&mut digest, &self.version);
        digest.update([self.timing.tag()]);
        self.kind.write_canonical(&mut digest);
        digest.update(self.schema_digest.as_bytes());
        digest.update(self.structure_digest.as_bytes());
        digest.update(self.source_snapshot_digest.as_bytes());
        put_text(&mut digest, self.experiment_id.as_str());
        put_u64(&mut digest, self.generation.0);
        Ok(DemographicEventDeclarationDigest(digest.finalize().into()))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DemographicEventDeclarationDigest([u8; 32]);

impl DemographicEventDeclarationDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for DemographicEventDeclarationDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DemographicEventDeclarationDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for DemographicEventDeclarationDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AlleleId, HereditarySchemaId, LocusDefinition, LocusId, PopulationStructureModel,
        PopulationStructureProfileId,
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
        MetapopulationSnapshot,
    ) {
        let schema = HereditarySchema::new(
            HereditarySchemaId::new("demog-event-v0").unwrap(),
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
            PopulationStructureProfileId::new("demog-structure").unwrap(),
            "v1",
            PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
            vec![a.clone(), b.clone()],
            vec![],
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
        let experiment = EvolutionExperimentId::new("demog-exp").unwrap();
        let points = populations
            .iter()
            .map(|(id, state)| {
                (
                    id.clone(),
                    PopulationTrajectoryPoint::declare_reference_start(
                        &schema,
                        state,
                        experiment.clone(),
                        PopulationGeneration(20),
                    )
                    .unwrap(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let snapshot =
            MetapopulationSnapshot::capture_reference(&schema, &structure, &populations, &points)
                .unwrap();
        (schema, structure, populations, points, snapshot)
    }

    fn declare(
        kind: DemographicEventKind,
    ) -> Result<DemographicEventDeclaration, EvolutionError> {
        let (schema, structure, populations, points, snapshot) = fixture();
        DemographicEventDeclaration::declare_current(
            DemographicEventId::new("event-01").unwrap(),
            "v1",
            kind,
            &schema,
            &structure,
            &populations,
            &points,
            &snapshot,
        )
    }

    #[test]
    fn split_daughter_order_is_canonicalized() {
        let first = declare(DemographicEventKind::PopulationSplit {
            source: pop("a"),
            daughters: vec![
                DaughterPopulation::new(pop("a-east"), 2).unwrap(),
                DaughterPopulation::new(pop("a-west"), 2).unwrap(),
            ],
        })
        .unwrap();
        let second = declare(DemographicEventKind::PopulationSplit {
            source: pop("a"),
            daughters: vec![
                DaughterPopulation::new(pop("a-west"), 2).unwrap(),
                DaughterPopulation::new(pop("a-east"), 2).unwrap(),
            ],
        })
        .unwrap();

        assert_eq!(first, second);
        assert_eq!(first.canonical_digest().unwrap(), second.canonical_digest().unwrap());
    }

    #[test]
    fn changing_one_event_parameter_changes_authority() {
        let first = declare(DemographicEventKind::CensusResize {
            population: pop("a"),
            target_census: 3,
        })
        .unwrap();
        let second = declare(DemographicEventKind::CensusResize {
            population: pop("a"),
            target_census: 2,
        })
        .unwrap();
        assert_ne!(first.canonical_digest().unwrap(), second.canonical_digest().unwrap());
    }

    #[test]
    fn invalid_event_preconditions_fail_closed() {
        assert!(matches!(
            declare(DemographicEventKind::CensusResize {
                population: pop("a"),
                target_census: 0,
            }),
            Err(EvolutionError::DemographicCensusMustBePositive)
        ));
        assert!(matches!(
            declare(DemographicEventKind::FounderEvent {
                source: pop("a"),
                founded_population: pop("b"),
                founder_census: 2,
            }),
            Err(EvolutionError::DemographicPopulationAlreadyExists { .. })
        ));
        assert!(matches!(
            declare(DemographicEventKind::PulseAdmixture {
                destination: pop("a"),
                source: pop("a"),
                source_fraction_ppm: 1,
            }),
            Err(EvolutionError::DemographicSelfAdmixture { .. })
        ));
        assert!(matches!(
            declare(DemographicEventKind::PulseAdmixture {
                destination: pop("a"),
                source: pop("b"),
                source_fraction_ppm: 0,
            }),
            Err(EvolutionError::DemographicAdmixtureFractionOutOfRange { .. })
        ));
    }

    #[test]
    fn serde_restore_preserves_identity_but_requires_current_source() {
        let (schema, structure, mut populations, points, snapshot) = fixture();
        let declaration = DemographicEventDeclaration::declare_current(
            DemographicEventId::new("event-restore").unwrap(),
            "v1",
            DemographicEventKind::Extinction { population: pop("b") },
            &schema,
            &structure,
            &populations,
            &points,
            &snapshot,
        )
        .unwrap();
        let encoded = serde_json::to_string(&declaration).unwrap();
        let restored: DemographicEventDeclaration = serde_json::from_str(&encoded).unwrap();
        assert_eq!(restored.canonical_digest().unwrap(), declaration.canonical_digest().unwrap());
        restored
            .validate_current(&schema, &structure, &populations, &points, &snapshot)
            .unwrap();

        let a = pop("a");
        let changed = PopulationGeneticState::from_counts(
            a.clone(),
            &schema,
            4,
            BTreeMap::from([(
                LocusId::new("focal").unwrap(),
                BTreeMap::from([(allele("a"), 4), (allele("b"), 4)]),
            )]),
        )
        .unwrap();
        populations.insert(a, changed);
        assert!(restored
            .validate_current(&schema, &structure, &populations, &points, &snapshot)
            .is_err());
    }

    #[test]
    fn restored_noncanonical_split_order_cannot_be_hashed_as_canonical() {
        let declaration = declare(DemographicEventKind::PopulationSplit {
            source: pop("a"),
            daughters: vec![
                DaughterPopulation::new(pop("a-east"), 2).unwrap(),
                DaughterPopulation::new(pop("a-west"), 2).unwrap(),
            ],
        })
        .unwrap();
        let mut value = serde_json::to_value(&declaration).unwrap();
        let daughters = value
            .get_mut("kind")
            .and_then(|kind| kind.get_mut("PopulationSplit"))
            .and_then(|split| split.get_mut("daughters"))
            .and_then(|daughters| daughters.as_array_mut())
            .unwrap();
        daughters.reverse();
        let restored: DemographicEventDeclaration = serde_json::from_value(value).unwrap();
        assert_eq!(
            restored.canonical_digest(),
            Err(EvolutionError::NonCanonicalDemographicDaughterOrder)
        );
    }
}
