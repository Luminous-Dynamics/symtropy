// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Bounded Level-P population projection.
//!
//! A prospective projection is presentation/analysis state only. It may show
//! individual-like candidates without transferring count, biomass, identity,
//! or mutation authority out of canonical [`PopulationState`]. Canonical
//! interaction must cross a later explicit realization/refinement boundary.
//!
//! The projection cost is bounded by the requested candidate budget rather
//! than by source population size. Candidate attributes are selected through
//! independent deterministic sparse permutation prefixes of each canonical
//! marginal, so a population near `u64::MAX` can still project a small visible
//! cohort without expanding one record per organism.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use crate::population::{
    CountDistribution, PopulationAgeBand, PopulationCell, PopulationConditionBand, PopulationError,
    PopulationState,
};

const AGE_SALT: u64 = 0x8f8f_31a7_a8b1_5c21;
const CONDITION_SALT: u64 = 0x2d2f_7e19_c4d0_1710;
const OCCUPANCY_SALT: u64 = 0x713b_d90f_5a71_a100;
const STEP_SALT: u64 = 0xd134_2543_de82_ef95;

/// Version of the deterministic prospective-projection grammar.
///
/// A future realization boundary must reject a handle whose scheme version is
/// not understood rather than silently reinterpreting the same candidate index
/// under a different projection algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProjectionSchemeVersion(pub u16);

pub const SPARSE_FISHER_YATES_PROJECTION_V1: ProjectionSchemeVersion = ProjectionSchemeVersion(1);

/// Higher-layer population/region routing scope for Level-P projection.
///
/// This is deliberately not an organism identifier. It prevents candidate
/// handles from different populations from colliding merely because revision,
/// seed, and local candidate index happen to match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProjectionScope(pub u128);

/// Canonical higher-layer revision associated with a Level-P projection.
///
/// This value does not itself grant ecological authority. A later realization
/// boundary must compare it with current authoritative population/region state
/// before allowing a projected candidate to become Level A.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProjectionRevision(pub u64);

/// Deterministic presentation seed for one prospective projection family.
///
/// The seed affects only how unresolved marginal degrees of freedom are shown.
/// It is not a stable organism identifier or a canonical event identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProjectionSeed(pub u64);

/// Complete routing/version context for one projection family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProjectionContext {
    scope: ProjectionScope,
    revision: ProjectionRevision,
    scheme: ProjectionSchemeVersion,
    seed: ProjectionSeed,
}

impl ProjectionContext {
    pub const fn sparse_fisher_yates_v1(
        scope: ProjectionScope,
        revision: ProjectionRevision,
        seed: ProjectionSeed,
    ) -> Self {
        Self {
            scope,
            revision,
            scheme: SPARSE_FISHER_YATES_PROJECTION_V1,
            seed,
        }
    }

    pub const fn scope(self) -> ProjectionScope {
        self.scope
    }

    pub const fn revision(self) -> ProjectionRevision {
        self.revision
    }

    pub const fn scheme(self) -> ProjectionSchemeVersion {
        self.scheme
    }

    pub const fn seed(self) -> ProjectionSeed {
        self.seed
    }
}

/// Handle for one candidate inside a scoped, revisioned projection family.
///
/// The handle is suitable for a future `projection -> realize -> interaction`
/// API. It is never a persistent organism ID and cannot authorize mutation by
/// itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProspectiveCandidateHandle {
    context: ProjectionContext,
    candidate_index: u64,
}

impl ProspectiveCandidateHandle {
    pub const fn context(self) -> ProjectionContext {
        self.context
    }

    pub const fn candidate_index(self) -> u64 {
        self.candidate_index
    }
}

/// One read-only Level-P candidate.
///
/// The age/condition/cell association is a deterministic synthetic tuple when
/// the source population stores only independent marginals. No canonical
/// biomass is attached here: exact extensive quantity belongs to the future
/// Level-A reservation/realization boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProspectivePopulationCandidate {
    handle: ProspectiveCandidateHandle,
    age: PopulationAgeBand,
    condition: PopulationConditionBand,
    cell: PopulationCell,
}

impl ProspectivePopulationCandidate {
    pub const fn handle(self) -> ProspectiveCandidateHandle {
        self.handle
    }

    pub const fn projected_age(self) -> PopulationAgeBand {
        self.age
    }

    pub const fn projected_condition(self) -> PopulationConditionBand {
        self.condition
    }

    pub const fn projected_cell(self) -> PopulationCell {
        self.cell
    }
}

/// Bounded projection of a canonical coarse population.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProspectivePopulationProjection {
    context: ProjectionContext,
    source_count: u64,
    candidates: Vec<ProspectivePopulationCandidate>,
}

impl ProspectivePopulationProjection {
    pub const fn context(&self) -> ProjectionContext {
        self.context
    }

    pub const fn source_count(&self) -> u64 {
        self.source_count
    }

    pub fn candidates(&self) -> &[ProspectivePopulationCandidate] {
        &self.candidates
    }

    pub fn len(&self) -> usize {
        self.candidates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }
}

/// Project at most `requested` read-only candidates from `population`.
///
/// `max_candidates` is a hard caller-declared allocation ceiling. If
/// `requested` exceeds it, projection fails before allocation. When the source
/// contains fewer members than requested, every conceptual member may be
/// represented, but projection never allocates more than `max_candidates`.
///
/// Increasing `requested` with the same population and [`ProjectionContext`]
/// is prefix stable: previously returned candidates keep the same handle and
/// tuple. Source population state is borrowed immutably and never reserved or
/// mutated.
pub fn project_population_bounded(
    population: &PopulationState,
    context: ProjectionContext,
    requested: usize,
    max_candidates: usize,
) -> Result<ProspectivePopulationProjection, ProjectionError> {
    if requested > max_candidates {
        return Err(ProjectionError::ProjectionLimitExceeded {
            requested,
            limit: max_candidates,
        });
    }

    if context.scheme != SPARSE_FISHER_YATES_PROJECTION_V1 {
        return Err(ProjectionError::UnsupportedProjectionScheme(context.scheme));
    }

    population.verify().map_err(ProjectionError::InvalidPopulation)?;
    let source_count = population.count();
    let candidate_count = match usize::try_from(source_count) {
        Ok(count) => requested.min(count),
        Err(_) => requested,
    };

    if candidate_count == 0 {
        return Ok(ProspectivePopulationProjection {
            context,
            source_count,
            candidates: Vec::new(),
        });
    }

    let seed = context.seed;
    let mut age_permutation = SparsePermutationPrefix::new(source_count, seed, AGE_SALT);
    let mut condition_permutation =
        SparsePermutationPrefix::new(source_count, seed, CONDITION_SALT);
    let mut occupancy_permutation =
        SparsePermutationPrefix::new(source_count, seed, OCCUPANCY_SALT);

    let mut candidates = Vec::with_capacity(candidate_count);
    for index in 0..candidate_count {
        let candidate_index =
            u64::try_from(index).map_err(|_| ProjectionError::CandidateIndexOverflow)?;
        let age_ordinal = age_permutation.next()?;
        let condition_ordinal = condition_permutation.next()?;
        let occupancy_ordinal = occupancy_permutation.next()?;

        candidates.push(ProspectivePopulationCandidate {
            handle: ProspectiveCandidateHandle {
                context,
                candidate_index,
            },
            age: value_at_ordinal(population.age_distribution(), age_ordinal, "age")?,
            condition: value_at_ordinal(
                population.condition_distribution(),
                condition_ordinal,
                "condition",
            )?,
            cell: value_at_ordinal(
                population.occupancy_distribution(),
                occupancy_ordinal,
                "occupancy",
            )?,
        });
    }

    Ok(ProspectivePopulationProjection {
        context,
        source_count,
        candidates,
    })
}

/// First `k` values of a deterministic Fisher-Yates permutation without an
/// `N`-element array.
///
/// The sparse map stores only positions displaced by the prefix shuffle, so
/// memory is O(k) even when `domain_size` is close to `u64::MAX`. Calling
/// `next()` repeatedly is prefix stable by construction.
#[derive(Debug, Clone)]
struct SparsePermutationPrefix {
    domain_size: u64,
    seed: ProjectionSeed,
    salt: u64,
    next_index: u64,
    displaced: BTreeMap<u64, u64>,
}

impl SparsePermutationPrefix {
    fn new(domain_size: u64, seed: ProjectionSeed, salt: u64) -> Self {
        debug_assert!(domain_size != 0);
        Self {
            domain_size,
            seed,
            salt,
            next_index: 0,
            displaced: BTreeMap::new(),
        }
    }

    fn next(&mut self) -> Result<u64, ProjectionError> {
        if self.next_index >= self.domain_size {
            return Err(ProjectionError::PermutationExhausted {
                source_count: self.domain_size,
            });
        }

        let index = self.next_index;
        let remaining = self.domain_size - index;
        let offset = bounded_random(self.seed, self.salt, index, remaining);
        let swap_index = index + offset;

        let selected = self.displaced.get(&swap_index).copied().unwrap_or(swap_index);
        let displaced = self.displaced.get(&index).copied().unwrap_or(index);

        self.displaced.remove(&index);
        if swap_index != index {
            self.displaced.insert(swap_index, displaced);
        }

        self.next_index += 1;
        Ok(selected)
    }
}

/// Deterministically map a keyed 64-bit stream into `[0, bound)` without
/// modulo bias using Lemire's multiply-high rejection method.
fn bounded_random(seed: ProjectionSeed, salt: u64, step: u64, bound: u64) -> u64 {
    debug_assert!(bound != 0);
    let threshold = bound.wrapping_neg() % bound;
    let mut attempt = 0u64;

    loop {
        let counter = step
            .wrapping_mul(STEP_SALT)
            .wrapping_add(attempt.rotate_left(17));
        let random = mix64(seed.0 ^ salt ^ mix64(counter));
        let product = random as u128 * bound as u128;
        let low = product as u64;
        if low >= threshold {
            return (product >> 64) as u64;
        }
        attempt = attempt.wrapping_add(1);
    }
}

fn value_at_ordinal<K: Ord + Copy>(
    distribution: &CountDistribution<K>,
    ordinal: u64,
    dimension: &'static str,
) -> Result<K, ProjectionError> {
    let mut remaining = ordinal;
    for (key, count) in distribution.bins() {
        if remaining < count {
            return Ok(*key);
        }
        remaining -= count;
    }

    Err(ProjectionError::DistributionOrdinalOutOfRange {
        dimension,
        ordinal,
        total: distribution.total(),
    })
}

fn mix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectionError {
    ProjectionLimitExceeded {
        requested: usize,
        limit: usize,
    },
    UnsupportedProjectionScheme(ProjectionSchemeVersion),
    CandidateIndexOverflow,
    PermutationExhausted {
        source_count: u64,
    },
    DistributionOrdinalOutOfRange {
        dimension: &'static str,
        ordinal: u64,
        total: u64,
    },
    InvalidPopulation(PopulationError),
}

impl fmt::Display for ProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProjectionLimitExceeded { requested, limit } => write!(
                formatter,
                "requested {requested} prospective candidates exceeds projection limit {limit}"
            ),
            Self::UnsupportedProjectionScheme(version) => write!(
                formatter,
                "unsupported prospective projection scheme version {}",
                version.0
            ),
            Self::CandidateIndexOverflow => {
                write!(formatter, "prospective candidate index cannot fit in u64")
            }
            Self::PermutationExhausted { source_count } => write!(
                formatter,
                "prospective permutation exhausted source population of {source_count}"
            ),
            Self::DistributionOrdinalOutOfRange {
                dimension,
                ordinal,
                total,
            } => write!(
                formatter,
                "prospective {dimension} ordinal {ordinal} is outside distribution total {total}"
            ),
            Self::InvalidPopulation(error) => {
                write!(formatter, "cannot project invalid population: {error}")
            }
        }
    }
}

impl Error for ProjectionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidPopulation(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    const SCOPE_A: ProjectionScope = ProjectionScope(0xA11CE);
    const SCOPE_B: ProjectionScope = ProjectionScope(0xB0B);

    fn context(scope: ProjectionScope, revision: u64, seed: u64) -> ProjectionContext {
        ProjectionContext::sparse_fisher_yates_v1(
            scope,
            ProjectionRevision(revision),
            ProjectionSeed(seed),
        )
    }

    fn distribution<K: Ord>(pairs: impl IntoIterator<Item = (K, u64)>) -> CountDistribution<K> {
        CountDistribution::new(pairs.into_iter().collect()).unwrap()
    }

    fn population(count: u64) -> PopulationState {
        PopulationState::new(
            count,
            count.saturating_mul(10_000),
            if count == 0 {
                distribution([])
            } else {
                let juvenile = count / 3;
                let mature = count - juvenile;
                let mut bins = BTreeMap::new();
                if juvenile != 0 {
                    bins.insert(PopulationAgeBand::Juvenile, juvenile);
                }
                if mature != 0 {
                    bins.insert(PopulationAgeBand::Mature, mature);
                }
                CountDistribution::new(bins).unwrap()
            },
            if count == 0 {
                distribution([])
            } else {
                let stressed = count / 4;
                let stable = count - stressed;
                let mut bins = BTreeMap::new();
                if stable != 0 {
                    bins.insert(PopulationConditionBand::Stable, stable);
                }
                if stressed != 0 {
                    bins.insert(PopulationConditionBand::Stressed, stressed);
                }
                CountDistribution::new(bins).unwrap()
            },
            if count == 0 {
                distribution([])
            } else {
                let first = count / 2;
                let second = count - first;
                let mut bins = BTreeMap::new();
                if first != 0 {
                    bins.insert(PopulationCell::new(0, 0, 0), first);
                }
                if second != 0 {
                    bins.insert(PopulationCell::new(1, 0, 0), second);
                }
                CountDistribution::new(bins).unwrap()
            },
        )
        .unwrap()
    }

    #[test]
    fn projection_is_exactly_deterministic() {
        let population = population(100);
        let before = population.clone();
        let context = context(SCOPE_A, 7, 91);
        let a = project_population_bounded(&population, context, 16, 32).unwrap();
        let b = project_population_bounded(&population, context, 16, 32).unwrap();

        assert_eq!(a, b);
        assert_eq!(population, before);
        assert_eq!(a.context(), context);
        assert_eq!(a.context().scheme(), SPARSE_FISHER_YATES_PROJECTION_V1);
    }

    #[test]
    fn larger_projection_preserves_existing_prefix() {
        let population = population(100);
        let context = context(SCOPE_A, 3, 17);
        let small = project_population_bounded(&population, context, 5, 16).unwrap();
        let large = project_population_bounded(&population, context, 12, 16).unwrap();

        assert_eq!(small.candidates(), &large.candidates()[..small.len()]);
    }

    #[test]
    fn candidate_handles_are_population_and_revision_scoped() {
        let population = population(20);
        let a = project_population_bounded(&population, context(SCOPE_A, 4, 10), 3, 3).unwrap();
        let different_revision =
            project_population_bounded(&population, context(SCOPE_A, 5, 10), 3, 3).unwrap();
        let different_scope =
            project_population_bounded(&population, context(SCOPE_B, 4, 10), 3, 3).unwrap();

        let handle = a.candidates()[0].handle();
        assert_ne!(handle, different_revision.candidates()[0].handle());
        assert_ne!(handle, different_scope.candidates()[0].handle());
        assert_eq!(handle.candidate_index(), 0);
        assert_eq!(handle.context().scope(), SCOPE_A);
        assert_eq!(handle.context().revision(), ProjectionRevision(4));
        assert_eq!(handle.context().scheme(), SPARSE_FISHER_YATES_PROJECTION_V1);
    }

    #[test]
    fn projection_limit_fails_before_population_sized_allocation() {
        let population = population(10_000);
        assert_eq!(
            project_population_bounded(&population, context(SCOPE_A, 1, 2), 65, 64),
            Err(ProjectionError::ProjectionLimitExceeded {
                requested: 65,
                limit: 64,
            })
        );
    }

    #[test]
    fn request_is_capped_by_actual_population_count() {
        let population = population(3);
        let projection =
            project_population_bounded(&population, context(SCOPE_A, 1, 2), 10, 10).unwrap();

        assert_eq!(projection.len(), 3);
        assert_eq!(projection.source_count(), 3);
    }

    #[test]
    fn empty_population_projects_no_candidates() {
        let population = population(0);
        let projection =
            project_population_bounded(&population, context(SCOPE_A, 1, 2), 10, 10).unwrap();

        assert!(projection.is_empty());
        assert_eq!(projection.source_count(), 0);
    }

    #[test]
    fn arbitrary_domain_sizes_yield_complete_permutations() {
        for domain_size in 1..=257u64 {
            let mut permutation =
                SparsePermutationPrefix::new(domain_size, ProjectionSeed(1234), AGE_SALT);
            let values = (0..domain_size)
                .map(|_| permutation.next().unwrap())
                .collect::<BTreeSet<_>>();
            assert_eq!(values.len() as u64, domain_size, "domain={domain_size}");
            assert!(values.iter().all(|value| *value < domain_size));
            assert_eq!(
                permutation.next(),
                Err(ProjectionError::PermutationExhausted {
                    source_count: domain_size,
                })
            );
        }
    }

    #[test]
    fn bounded_random_never_leaves_requested_range() {
        for bound in [1, 2, 3, 7, 16, 257, u32::MAX as u64, u64::MAX] {
            for step in 0..512 {
                assert!(bounded_random(ProjectionSeed(7), AGE_SALT, step, bound) < bound);
            }
        }
    }

    #[test]
    fn huge_population_projection_remains_bounded() {
        let population = PopulationState::new(
            u64::MAX,
            u64::MAX,
            distribution([(PopulationAgeBand::Mature, u64::MAX)]),
            distribution([(PopulationConditionBand::Stable, u64::MAX)]),
            distribution([(PopulationCell::new(9, -2, 4), u64::MAX)]),
        )
        .unwrap();

        let projection = project_population_bounded(
            &population,
            context(SCOPE_A, u64::MAX - 1, u64::MAX),
            32,
            32,
        )
        .unwrap();

        assert_eq!(projection.len(), 32);
        assert_eq!(projection.source_count(), u64::MAX);
        assert!(projection.candidates().iter().all(|candidate| {
            candidate.projected_age() == PopulationAgeBand::Mature
                && candidate.projected_condition() == PopulationConditionBand::Stable
                && candidate.projected_cell() == PopulationCell::new(9, -2, 4)
        }));
    }

    #[test]
    fn independent_marginal_permutations_do_not_claim_joint_history() {
        let population = population(96);
        let a =
            project_population_bounded(&population, context(SCOPE_A, 8, 100), 24, 24).unwrap();
        let b =
            project_population_bounded(&population, context(SCOPE_A, 8, 101), 24, 24).unwrap();

        assert_ne!(a.candidates(), b.candidates());
        assert_eq!(a.source_count(), b.source_count());
        assert_eq!(population.verify(), Ok(()));
    }
}
