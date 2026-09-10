use crate::{AlleleId, LocusId, PROBABILITY_SCALE_PPM};
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
