use crate::{AlleleId, LocusId, PopulationId, PROBABILITY_SCALE_PPM};
use std::{error::Error, fmt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvolutionError {
    UnsupportedSchema(u32),
    EmptyText { field: &'static str },
    UnsupportedPloidy(u8),
    ModeRequiresDiploid(u8),
    NoLoci,
    DuplicateLocus,
    NoAlleles { locus: LocusId },
    LocusKeyMismatch { key: LocusId, value: LocusId },
    HereditarySchemaMismatch,
    HereditarySchemaAuthorityMismatch,
    OperatorAuthorityMismatch,
    LocusSetMismatch,
    MissingLocus(LocusId),
    CopyCountMismatch {
        locus: LocusId,
        expected: u8,
        observed: usize,
    },
    NonCanonicalAlleleCopyOrder { locus: LocusId },
    NonCanonicalZeroAlleleCount { locus: LocusId, allele: AlleleId },
    UnknownAllele { locus: LocusId, allele: AlleleId },
    ParentCountMismatch { expected: usize, observed: usize },
    ParentageMismatch,
    ChildDigestMismatch,
    ChildDerivationMismatch,
    ProbabilityOutOfRange { observed_ppm: u32 },
    EmptyPopulation,
    PopulationCopyTotalMismatch {
        locus: LocusId,
        expected: u64,
        observed: u64,
    },
    PopulationIdentityMismatch,
    PopulationExperimentMismatch,
    PopulationTrajectoryStateMismatch,
    PopulationTrajectoryPointMismatch,
    PopulationProcessAuthorityMismatch,
    PopulationProcessModelMismatch,
    PopulationSourceMismatch,
    PopulationDestinationMismatch,
    PopulationGenerationMismatch,
    PopulationTransitionMismatch,
    NoStructuredPopulations,
    DuplicatePopulationIdentity { population: PopulationId },
    UnknownStructurePopulation { population: PopulationId },
    DuplicateMigrationEdge {
        destination: PopulationId,
        source: PopulationId,
    },
    SelfMigrationEntry { population: PopulationId },
    NonCanonicalEmptyMigrationRow { destination: PopulationId },
    NonCanonicalZeroMigration {
        destination: PopulationId,
        source: PopulationId,
    },
    MigrationRowExceedsProbabilityScale {
        destination: PopulationId,
        observed_ppm: u64,
    },
    MetapopulationSetMismatch,
    MetapopulationStateKeyMismatch {
        key: PopulationId,
        observed: PopulationId,
    },
    MetapopulationPointKeyMismatch {
        key: PopulationId,
        observed: PopulationId,
    },
    MetapopulationStructureAuthorityMismatch,
    MetapopulationSnapshotMismatch,
    StructuredPopulationStructureAuthorityMismatch,
    StructuredPopulationSourceSnapshotMismatch,
    StructuredPopulationDestinationMismatch,
    StructuredPopulationTransitionMismatch,
    DemographicCensusMustBePositive,
    UnknownDemographicPopulation { population: PopulationId },
    DemographicPopulationAlreadyExists { population: PopulationId },
    DuplicateDemographicDaughter { population: PopulationId },
    DemographicSplitRequiresTwoDaughters,
    NonCanonicalDemographicDaughterOrder,
    DemographicSelfAdmixture { population: PopulationId },
    DemographicAdmixtureFractionOutOfRange { observed_ppm: u32 },
    DemographicEventTimingMismatch,
    DemographicStructureAuthorityMismatch,
    DemographicSourceSnapshotMismatch,
    DemographicStructureTransitionEventMismatch,
    DemographicSuccessorStructureMismatch,
    DemographicMembershipPreservingStructureChanged,
    DemographicCannotProduceEmptyMetapopulation,
    DemographicSuccessorSetMismatch,
    DemographicHistoryCursorShapeMismatch,
    DemographicHistoryCursorNotRoot,
    DemographicHistoryCursorMismatch,
    DemographicEventKindUnsupportedForExecutor,
    DemographicExpansionUnsupported {
        current_census: u64,
        target_census: u64,
    },
    DemographicExecutionAuthorityMismatch,
    DemographicExecutionSourceMismatch,
    DemographicExecutionResultMismatch,
    DemographicExecutionHistoryMismatch,
    SamplingInvariantViolation,
    CountOverflow,
    InvalidDrawUpperBound,
}

impl fmt::Display for EvolutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchema(version) => write!(f, "unsupported evolution schema {version}"),
            Self::EmptyText { field } => write!(f, "{field} must not be empty"),
            Self::UnsupportedPloidy(ploidy) => write!(f, "unsupported ploidy {ploidy}"),
            Self::ModeRequiresDiploid(ploidy) => {
                write!(f, "biparental V0 mode requires diploid schema, observed {ploidy}")
            }
            Self::NoLoci => write!(f, "hereditary schema must contain at least one locus"),
            Self::DuplicateLocus => write!(f, "duplicate locus identity"),
            Self::NoAlleles { locus } => write!(f, "locus {} has no allowed alleles", locus.as_str()),
            Self::LocusKeyMismatch { key, value } => write!(
                f,
                "locus map key {} does not match value {}",
                key.as_str(),
                value.as_str()
            ),
            Self::HereditarySchemaMismatch => write!(f, "hereditary schema id mismatch"),
            Self::HereditarySchemaAuthorityMismatch => {
                write!(f, "exact hereditary schema authority mismatch")
            }
            Self::OperatorAuthorityMismatch => write!(f, "exact evolution operator authority mismatch"),
            Self::LocusSetMismatch => write!(f, "hereditary locus set mismatch"),
            Self::MissingLocus(locus) => write!(f, "missing locus {}", locus.as_str()),
            Self::CopyCountMismatch { locus, expected, observed } => write!(
                f,
                "locus {} expected {expected} copies, observed {observed}",
                locus.as_str()
            ),
            Self::NonCanonicalAlleleCopyOrder { locus } => write!(
                f,
                "locus {} allele copies are not in canonical unphased order",
                locus.as_str()
            ),
            Self::NonCanonicalZeroAlleleCount { locus, allele } => write!(
                f,
                "locus {} contains noncanonical zero count for allele {}",
                locus.as_str(),
                allele.as_str()
            ),
            Self::UnknownAllele { locus, allele } => write!(
                f,
                "allele {} is not allowed at locus {}",
                allele.as_str(),
                locus.as_str()
            ),
            Self::ParentCountMismatch { expected, observed } => {
                write!(f, "expected {expected} parent(s), observed {observed}")
            }
            Self::ParentageMismatch => write!(f, "reproduction parentage/provenance mismatch"),
            Self::ChildDigestMismatch => write!(f, "reproduction child digest mismatch"),
            Self::ChildDerivationMismatch => write!(f, "reproduction child derivation mismatch"),
            Self::ProbabilityOutOfRange { observed_ppm } => write!(
                f,
                "probability {observed_ppm} ppm exceeds {PROBABILITY_SCALE_PPM} ppm"
            ),
            Self::EmptyPopulation => write!(f, "population must contain at least one individual"),
            Self::PopulationCopyTotalMismatch { locus, expected, observed } => write!(
                f,
                "locus {} expected {expected} allele copies, observed {observed}",
                locus.as_str()
            ),
            Self::PopulationIdentityMismatch => write!(f, "population identity mismatch"),
            Self::PopulationExperimentMismatch => {
                write!(f, "population stochastic experiment identity mismatch")
            }
            Self::PopulationTrajectoryStateMismatch => {
                write!(f, "population trajectory point does not match current population state")
            }
            Self::PopulationTrajectoryPointMismatch => {
                write!(f, "population transition trajectory-point mismatch")
            }
            Self::PopulationProcessAuthorityMismatch => {
                write!(f, "population process authority mismatch")
            }
            Self::PopulationProcessModelMismatch => {
                write!(f, "population process model mismatch")
            }
            Self::PopulationSourceMismatch => write!(f, "population transition source mismatch"),
            Self::PopulationDestinationMismatch => {
                write!(f, "population transition destination mismatch")
            }
            Self::PopulationGenerationMismatch => {
                write!(f, "population transition generation mismatch")
            }
            Self::PopulationTransitionMismatch => {
                write!(f, "population transition derivation mismatch")
            }
            Self::NoStructuredPopulations => {
                write!(f, "population structure must declare at least one population")
            }
            Self::DuplicatePopulationIdentity { population } => write!(
                f,
                "population structure declares duplicate population {}",
                population.as_str()
            ),
            Self::UnknownStructurePopulation { population } => write!(
                f,
                "population structure references undeclared population {}",
                population.as_str()
            ),
            Self::DuplicateMigrationEdge { destination, source } => write!(
                f,
                "population structure repeats migration edge {} <- {}",
                destination.as_str(),
                source.as_str()
            ),
            Self::SelfMigrationEntry { population } => write!(
                f,
                "population structure must not encode explicit self migration for {}",
                population.as_str()
            ),
            Self::NonCanonicalEmptyMigrationRow { destination } => write!(
                f,
                "population structure contains noncanonical empty migration row for {}",
                destination.as_str()
            ),
            Self::NonCanonicalZeroMigration { destination, source } => write!(
                f,
                "population structure contains noncanonical zero migration {} <- {}",
                destination.as_str(),
                source.as_str()
            ),
            Self::MigrationRowExceedsProbabilityScale {
                destination,
                observed_ppm,
            } => write!(
                f,
                "population {} migration row totals {observed_ppm} ppm, exceeding {PROBABILITY_SCALE_PPM} ppm",
                destination.as_str()
            ),
            Self::MetapopulationSetMismatch => {
                write!(f, "metapopulation source set does not match structure authority")
            }
            Self::MetapopulationStateKeyMismatch { key, observed } => write!(
                f,
                "metapopulation state key {} does not match embedded population {}",
                key.as_str(),
                observed.as_str()
            ),
            Self::MetapopulationPointKeyMismatch { key, observed } => write!(
                f,
                "metapopulation trajectory key {} does not match point population {}",
                key.as_str(),
                observed.as_str()
            ),
            Self::MetapopulationStructureAuthorityMismatch => {
                write!(f, "metapopulation snapshot structure authority mismatch")
            }
            Self::MetapopulationSnapshotMismatch => {
                write!(f, "metapopulation snapshot no longer matches current source state")
            }
            Self::StructuredPopulationStructureAuthorityMismatch => {
                write!(f, "structured population transition structure authority mismatch")
            }
            Self::StructuredPopulationSourceSnapshotMismatch => {
                write!(f, "structured population transition source snapshot mismatch")
            }
            Self::StructuredPopulationDestinationMismatch => {
                write!(f, "structured population transition destination evidence mismatch")
            }
            Self::StructuredPopulationTransitionMismatch => {
                write!(f, "structured population transition derivation mismatch")
            }
            Self::DemographicCensusMustBePositive => {
                write!(f, "demographic census must be positive at aggregate population fidelity")
            }
            Self::UnknownDemographicPopulation { population } => write!(
                f,
                "demographic event references undeclared population {}",
                population.as_str()
            ),
            Self::DemographicPopulationAlreadyExists { population } => write!(
                f,
                "demographic event requires new population identity, but {} already exists",
                population.as_str()
            ),
            Self::DuplicateDemographicDaughter { population } => write!(
                f,
                "population split repeats daughter population {}",
                population.as_str()
            ),
            Self::DemographicSplitRequiresTwoDaughters => {
                write!(f, "population split requires at least two daughter populations")
            }
            Self::NonCanonicalDemographicDaughterOrder => {
                write!(f, "population split daughters are not in canonical population-id order")
            }
            Self::DemographicSelfAdmixture { population } => write!(
                f,
                "pulse admixture cannot use the same source and destination population {}",
                population.as_str()
            ),
            Self::DemographicAdmixtureFractionOutOfRange { observed_ppm } => write!(
                f,
                "pulse-admixture source fraction must be in 1..={PROBABILITY_SCALE_PPM} ppm, observed {observed_ppm}"
            ),
            Self::DemographicEventTimingMismatch => {
                write!(f, "demographic event timing convention mismatch")
            }
            Self::DemographicStructureAuthorityMismatch => {
                write!(f, "demographic event structure authority mismatch")
            }
            Self::DemographicSourceSnapshotMismatch => {
                write!(f, "demographic event source snapshot mismatch")
            }
            Self::DemographicStructureTransitionEventMismatch => {
                write!(f, "demographic structure transition event authority mismatch")
            }
            Self::DemographicSuccessorStructureMismatch => {
                write!(f, "demographic structure transition successor authority mismatch")
            }
            Self::DemographicMembershipPreservingStructureChanged => write!(
                f,
                "membership-preserving demographic event cannot change population structure authority"
            ),
            Self::DemographicCannotProduceEmptyMetapopulation => write!(
                f,
                "demographic event cannot produce an empty aggregate metapopulation in V0"
            ),
            Self::DemographicSuccessorSetMismatch => write!(
                f,
                "successor population set does not match demographic event semantics"
            ),
            Self::DemographicHistoryCursorShapeMismatch => {
                write!(f, "demographic history cursor has a noncanonical link shape")
            }
            Self::DemographicHistoryCursorNotRoot => {
                write!(f, "non-root demographic history cursor requires execution revalidation")
            }
            Self::DemographicHistoryCursorMismatch => {
                write!(f, "demographic history cursor does not match the exact current snapshot trajectory")
            }
            Self::DemographicEventKindUnsupportedForExecutor => {
                write!(f, "demographic event kind is unsupported by this reference executor")
            }
            Self::DemographicExpansionUnsupported {
                current_census,
                target_census,
            } => write!(
                f,
                "instantaneous census expansion is not a random-survivor bottleneck: current {current_census}, target {target_census}"
            ),
            Self::DemographicExecutionAuthorityMismatch => {
                write!(f, "demographic execution authority mismatch")
            }
            Self::DemographicExecutionSourceMismatch => {
                write!(f, "demographic execution source evidence mismatch")
            }
            Self::DemographicExecutionResultMismatch => {
                write!(f, "demographic execution result evidence mismatch")
            }
            Self::DemographicExecutionHistoryMismatch => {
                write!(f, "demographic execution history cursor mismatch")
            }
            Self::SamplingInvariantViolation => write!(f, "population sampling invariant violated"),
            Self::CountOverflow => write!(f, "population/genetic count overflow"),
            Self::InvalidDrawUpperBound => write!(f, "semantic draw upper bound must be positive"),
        }
    }
}

impl Error for EvolutionError {}

pub(crate) fn validate_text(field: &'static str, value: &str) -> Result<(), EvolutionError> {
    if value.trim().is_empty() {
        Err(EvolutionError::EmptyText { field })
    } else {
        Ok(())
    }
}
