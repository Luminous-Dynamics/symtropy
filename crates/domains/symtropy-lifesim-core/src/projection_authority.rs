// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Source-authority binding for prospective Level-P population projections.
//!
//! [`crate::projection`] intentionally keeps presentation projection
//! non-authoritative. This module adds the separate provenance layer a future
//! Level-A realization boundary needs: a projected candidate can be considered
//! realizable only if the projection was created against an explicit source
//! authority stamp and that exact source remains current.

use std::error::Error;
use std::fmt;

use crate::population::PopulationState;
use crate::projection::{
    ProspectiveCandidateHandle, ProspectivePopulationProjection, ProjectionContext,
    ProjectionError, ProjectionSchemeVersion, ProjectionScope, SPARSE_FISHER_YATES_PROJECTION_V1,
    project_population_bounded,
};

/// Canonical authority representation from which a projection was derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProjectionAuthorityKind {
    /// Qualified coarse population authority represented by independent
    /// age/condition/occupancy marginals.
    MarginalPopulation,
    /// Correlation-preserving sparse joint population strata.
    SparseStrata,
}

/// Version of the source authority schema, independent from projection scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProjectionAuthoritySchemaVersion(pub u16);

/// Monotonic generation within one higher-layer authority scope.
///
/// Generation alone is deliberately insufficient for freshness. Rollback,
/// restore, or branch reuse can produce the same numeric generation while the
/// underlying source state differs; [`ProjectionSourceStateToken`] closes that
/// alias.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProjectionAuthorityGeneration(pub u64);

/// Opaque identity of the exact source state used to create a projection.
///
/// Higher layers may derive this from a snapshot digest, event head, authority
/// epoch, or another non-reused state token. `lifesim-core` treats it purely as
/// opaque provenance and owns no cryptographic semantics for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProjectionSourceStateToken(pub u128);

/// Complete source-authority stamp captured at projection creation time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProjectionSourceAuthorityStamp {
    scope: ProjectionScope,
    kind: ProjectionAuthorityKind,
    schema: ProjectionAuthoritySchemaVersion,
    generation: ProjectionAuthorityGeneration,
    state_token: ProjectionSourceStateToken,
}

impl ProjectionSourceAuthorityStamp {
    pub const MARGINAL_POPULATION_SCHEMA_V1: ProjectionAuthoritySchemaVersion =
        ProjectionAuthoritySchemaVersion(1);

    pub const fn marginal_population_v1(
        scope: ProjectionScope,
        generation: ProjectionAuthorityGeneration,
        state_token: ProjectionSourceStateToken,
    ) -> Self {
        Self {
            scope,
            kind: ProjectionAuthorityKind::MarginalPopulation,
            schema: Self::MARGINAL_POPULATION_SCHEMA_V1,
            generation,
            state_token,
        }
    }

    pub const fn new(
        scope: ProjectionScope,
        kind: ProjectionAuthorityKind,
        schema: ProjectionAuthoritySchemaVersion,
        generation: ProjectionAuthorityGeneration,
        state_token: ProjectionSourceStateToken,
    ) -> Self {
        Self {
            scope,
            kind,
            schema,
            generation,
            state_token,
        }
    }

    pub const fn scope(self) -> ProjectionScope {
        self.scope
    }

    pub const fn kind(self) -> ProjectionAuthorityKind {
        self.kind
    }

    pub const fn schema(self) -> ProjectionAuthoritySchemaVersion {
        self.schema
    }

    pub const fn generation(self) -> ProjectionAuthorityGeneration {
        self.generation
    }

    pub const fn state_token(self) -> ProjectionSourceStateToken {
        self.state_token
    }
}

/// A Level-P projection created against one exact source authority state.
///
/// This wrapper still grants no ecological mutation authority. It only freezes
/// enough provenance for a later realization transaction to prove that the
/// candidate it is about to consume is the same candidate that was shown from
/// the same source state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityStampedProjection {
    source: ProjectionSourceAuthorityStamp,
    projection: ProspectivePopulationProjection,
}

impl AuthorityStampedProjection {
    pub const fn source(&self) -> ProjectionSourceAuthorityStamp {
        self.source
    }

    pub const fn projection(&self) -> &ProspectivePopulationProjection {
        &self.projection
    }

    /// Bind one candidate from this projection to the exact source authority
    /// that produced it.
    pub fn bound_handle(
        &self,
        candidate: ProspectiveCandidateHandle,
    ) -> Result<SourceBoundCandidateHandle, ProjectionAuthorityError> {
        if candidate.context() != self.projection.context() {
            return Err(ProjectionAuthorityError::CandidateContextMismatch);
        }
        let index = usize::try_from(candidate.candidate_index())
            .map_err(|_| ProjectionAuthorityError::CandidateIndexOutOfRange)?;
        let expected = self
            .projection
            .candidates()
            .get(index)
            .ok_or(ProjectionAuthorityError::CandidateIndexOutOfRange)?
            .handle();
        if expected != candidate {
            return Err(ProjectionAuthorityError::CandidateContextMismatch);
        }
        Ok(SourceBoundCandidateHandle {
            source: self.source,
            candidate,
        })
    }
}

/// Candidate handle whose source authority was captured when projected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceBoundCandidateHandle {
    source: ProjectionSourceAuthorityStamp,
    candidate: ProspectiveCandidateHandle,
}

impl SourceBoundCandidateHandle {
    pub const fn source(self) -> ProjectionSourceAuthorityStamp {
        self.source
    }

    pub const fn candidate(self) -> ProspectiveCandidateHandle {
        self.candidate
    }

    /// Validate freshness against the exact current source authority stamp.
    ///
    /// No mutation or reservation occurs here. A mismatch must be resolved by
    /// re-projection or a separately qualified compatibility rule, never by
    /// silently rebinding this old candidate to new authority.
    pub fn validate_current_source(
        self,
        current: ProjectionSourceAuthorityStamp,
    ) -> Result<(), ProjectionAuthorityError> {
        if self.source == current {
            Ok(())
        } else {
            Err(ProjectionAuthorityError::SourceAuthorityChanged {
                projected: self.source,
                current,
            })
        }
    }
}

/// Create a bounded marginal-source Level-P projection with authority
/// provenance captured at creation time.
pub fn project_population_bounded_with_authority(
    population: &PopulationState,
    context: ProjectionContext,
    source: ProjectionSourceAuthorityStamp,
    requested: usize,
    max_candidates: usize,
) -> Result<AuthorityStampedProjection, ProjectionAuthorityError> {
    validate_source_for_projection(context, source)?;
    let projection = project_population_bounded(population, context, requested, max_candidates)
        .map_err(ProjectionAuthorityError::Projection)?;
    Ok(AuthorityStampedProjection { source, projection })
}

fn validate_source_for_projection(
    context: ProjectionContext,
    source: ProjectionSourceAuthorityStamp,
) -> Result<(), ProjectionAuthorityError> {
    if source.scope != context.scope() {
        return Err(ProjectionAuthorityError::ScopeMismatch {
            projection: context.scope(),
            source: source.scope,
        });
    }

    if source.generation.0 != context.revision().0 {
        return Err(ProjectionAuthorityError::GenerationMismatch {
            projection: context.revision().0,
            source: source.generation.0,
        });
    }

    validate_scheme_source(context.scheme(), source)
}

fn validate_scheme_source(
    scheme: ProjectionSchemeVersion,
    source: ProjectionSourceAuthorityStamp,
) -> Result<(), ProjectionAuthorityError> {
    if scheme != SPARSE_FISHER_YATES_PROJECTION_V1 {
        return Err(ProjectionAuthorityError::UnsupportedProjectionScheme(scheme));
    }

    if source.kind != ProjectionAuthorityKind::MarginalPopulation {
        return Err(ProjectionAuthorityError::IncompatibleAuthorityKind {
            scheme,
            kind: source.kind,
        });
    }

    if source.schema != ProjectionSourceAuthorityStamp::MARGINAL_POPULATION_SCHEMA_V1 {
        return Err(ProjectionAuthorityError::IncompatibleAuthoritySchema {
            scheme,
            schema: source.schema,
        });
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectionAuthorityError {
    ScopeMismatch {
        projection: ProjectionScope,
        source: ProjectionScope,
    },
    GenerationMismatch {
        projection: u64,
        source: u64,
    },
    UnsupportedProjectionScheme(ProjectionSchemeVersion),
    IncompatibleAuthorityKind {
        scheme: ProjectionSchemeVersion,
        kind: ProjectionAuthorityKind,
    },
    IncompatibleAuthoritySchema {
        scheme: ProjectionSchemeVersion,
        schema: ProjectionAuthoritySchemaVersion,
    },
    CandidateContextMismatch,
    CandidateIndexOutOfRange,
    SourceAuthorityChanged {
        projected: ProjectionSourceAuthorityStamp,
        current: ProjectionSourceAuthorityStamp,
    },
    Projection(ProjectionError),
}

impl fmt::Display for ProjectionAuthorityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ScopeMismatch { projection, source } => write!(
                formatter,
                "projection scope {:?} does not match source authority scope {:?}",
                projection, source
            ),
            Self::GenerationMismatch { projection, source } => write!(
                formatter,
                "projection revision {projection} does not match source authority generation {source}"
            ),
            Self::UnsupportedProjectionScheme(scheme) => {
                write!(formatter, "unsupported authority-bound projection scheme {}", scheme.0)
            }
            Self::IncompatibleAuthorityKind { scheme, kind } => write!(
                formatter,
                "projection scheme {} is incompatible with source authority kind {kind:?}",
                scheme.0
            ),
            Self::IncompatibleAuthoritySchema { scheme, schema } => write!(
                formatter,
                "projection scheme {} is incompatible with source authority schema {}",
                scheme.0, schema.0
            ),
            Self::CandidateContextMismatch => {
                write!(formatter, "candidate does not belong to the stamped projection")
            }
            Self::CandidateIndexOutOfRange => {
                write!(formatter, "candidate index is outside the stamped projection")
            }
            Self::SourceAuthorityChanged { .. } => {
                write!(formatter, "projection source authority is stale or incompatible")
            }
            Self::Projection(error) => write!(formatter, "projection failed: {error}"),
        }
    }
}

impl Error for ProjectionAuthorityError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Projection(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::population::{
        CountDistribution, PopulationAgeBand, PopulationCell, PopulationConditionBand,
    };
    use crate::projection::{ProjectionRevision, ProjectionSeed};

    const SCOPE: ProjectionScope = ProjectionScope(0xCA115A1);

    fn distribution<K: Ord>(pairs: impl IntoIterator<Item = (K, u64)>) -> CountDistribution<K> {
        CountDistribution::new(pairs.into_iter().collect()).unwrap()
    }

    fn population() -> PopulationState {
        PopulationState::new(
            4,
            400_000,
            distribution([
                (PopulationAgeBand::Juvenile, 2),
                (PopulationAgeBand::Mature, 2),
            ]),
            distribution([
                (PopulationConditionBand::Stable, 3),
                (PopulationConditionBand::Stressed, 1),
            ]),
            distribution([
                (PopulationCell::new(0, 0, 0), 2),
                (PopulationCell::new(1, 0, 0), 2),
            ]),
        )
        .unwrap()
    }

    fn context(seed: u64) -> ProjectionContext {
        ProjectionContext::sparse_fisher_yates_v1(
            SCOPE,
            ProjectionRevision(7),
            ProjectionSeed(seed),
        )
    }

    fn source(token: u128) -> ProjectionSourceAuthorityStamp {
        ProjectionSourceAuthorityStamp::marginal_population_v1(
            SCOPE,
            ProjectionAuthorityGeneration(7),
            ProjectionSourceStateToken(token),
        )
    }

    #[test]
    fn stamped_projection_binds_candidate_to_exact_source() {
        let projection =
            project_population_bounded_with_authority(&population(), context(11), source(99), 2, 4)
                .unwrap();
        let candidate = projection.projection().candidates()[0].handle();
        let bound = projection.bound_handle(candidate).unwrap();

        assert_eq!(bound.source(), source(99));
        assert_eq!(bound.candidate(), candidate);
        assert_eq!(bound.validate_current_source(source(99)), Ok(()));
    }

    #[test]
    fn same_generation_different_authority_kind_is_rejected() {
        let strata_source = ProjectionSourceAuthorityStamp::new(
            SCOPE,
            ProjectionAuthorityKind::SparseStrata,
            ProjectionAuthoritySchemaVersion(1),
            ProjectionAuthorityGeneration(7),
            ProjectionSourceStateToken(99),
        );

        assert!(matches!(
            project_population_bounded_with_authority(
                &population(),
                context(11),
                strata_source,
                2,
                4,
            ),
            Err(ProjectionAuthorityError::IncompatibleAuthorityKind { .. })
        ));
    }

    #[test]
    fn same_kind_generation_different_schema_is_rejected() {
        let wrong_schema = ProjectionSourceAuthorityStamp::new(
            SCOPE,
            ProjectionAuthorityKind::MarginalPopulation,
            ProjectionAuthoritySchemaVersion(2),
            ProjectionAuthorityGeneration(7),
            ProjectionSourceStateToken(99),
        );

        assert!(matches!(
            project_population_bounded_with_authority(
                &population(),
                context(11),
                wrong_schema,
                2,
                4,
            ),
            Err(ProjectionAuthorityError::IncompatibleAuthoritySchema { .. })
        ));
    }

    #[test]
    fn state_token_prevents_numeric_generation_reuse_from_reviving_candidate() {
        let projection =
            project_population_bounded_with_authority(&population(), context(11), source(99), 2, 4)
                .unwrap();
        let bound = projection
            .bound_handle(projection.projection().candidates()[0].handle())
            .unwrap();

        assert!(matches!(
            bound.validate_current_source(source(100)),
            Err(ProjectionAuthorityError::SourceAuthorityChanged { .. })
        ));
    }

    #[test]
    fn scope_and_generation_must_match_projection_context() {
        let wrong_scope = ProjectionSourceAuthorityStamp::marginal_population_v1(
            ProjectionScope(SCOPE.0 + 1),
            ProjectionAuthorityGeneration(7),
            ProjectionSourceStateToken(99),
        );
        let wrong_generation = ProjectionSourceAuthorityStamp::marginal_population_v1(
            SCOPE,
            ProjectionAuthorityGeneration(8),
            ProjectionSourceStateToken(99),
        );

        assert!(matches!(
            project_population_bounded_with_authority(
                &population(),
                context(11),
                wrong_scope,
                2,
                4,
            ),
            Err(ProjectionAuthorityError::ScopeMismatch { .. })
        ));
        assert!(matches!(
            project_population_bounded_with_authority(
                &population(),
                context(11),
                wrong_generation,
                2,
                4,
            ),
            Err(ProjectionAuthorityError::GenerationMismatch { .. })
        ));
    }

    #[test]
    fn presentation_seed_changes_projection_not_source_authority() {
        let a =
            project_population_bounded_with_authority(&population(), context(11), source(99), 4, 4)
                .unwrap();
        let b =
            project_population_bounded_with_authority(&population(), context(12), source(99), 4, 4)
                .unwrap();

        assert_eq!(a.source(), b.source());
        assert_ne!(a.projection().context(), b.projection().context());
    }

    #[test]
    fn candidate_from_another_projection_cannot_be_rebound() {
        let a =
            project_population_bounded_with_authority(&population(), context(11), source(99), 2, 4)
                .unwrap();
        let b =
            project_population_bounded_with_authority(&population(), context(12), source(99), 2, 4)
                .unwrap();

        assert_eq!(
            a.bound_handle(b.projection().candidates()[0].handle()),
            Err(ProjectionAuthorityError::CandidateContextMismatch)
        );
    }
}
