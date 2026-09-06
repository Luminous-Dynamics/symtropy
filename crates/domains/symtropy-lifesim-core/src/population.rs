// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Canonical coarse population state for multiscale living-world simulation.
//!
//! A distant population must remain authoritative without requiring every
//! organism to stay individually materialized. This module therefore stores
//! exact headcount and biomass plus deterministic integer distributions whose
//! totals must independently agree with population count.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

/// Canonical age/development bands for coarse population accounting.
///
/// Species-specific development may refine these bands later. The coarse state
/// deliberately avoids assuming mammalian sex/age semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PopulationAgeBand {
    Propagule,
    Juvenile,
    Reproductive,
    Mature,
    Elder,
    Dormant,
}

/// Canonical coarse physiological condition bands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PopulationConditionBand {
    Thriving,
    Stable,
    Stressed,
    Critical,
}

/// Integer spatial cell used by coarse population occupancy.
///
/// Cell size and world-to-cell mapping belong to the region/domain adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PopulationCell {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl PopulationCell {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

/// Canonical sparse integer distribution.
///
/// Zero-count bins are rejected so the same semantic distribution has one
/// representation. `BTreeMap` gives deterministic key ordering for telemetry,
/// persistence adapters, and future canonical encodings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CountDistribution<K> {
    counts: BTreeMap<K, u64>,
    total: u64,
}

impl<K: Ord> CountDistribution<K> {
    pub fn new(counts: BTreeMap<K, u64>) -> Result<Self, PopulationError> {
        let mut total = 0u64;
        for count in counts.values().copied() {
            if count == 0 {
                return Err(PopulationError::ZeroCountBin);
            }
            total = total
                .checked_add(count)
                .ok_or(PopulationError::CountOverflow)?;
        }
        Ok(Self { counts, total })
    }

    pub const fn total(&self) -> u64 {
        self.total
    }

    pub fn count(&self, key: &K) -> u64 {
        self.counts.get(key).copied().unwrap_or(0)
    }

    pub fn bins(&self) -> impl Iterator<Item = (&K, u64)> {
        self.counts.iter().map(|(key, count)| (key, *count))
    }

    pub fn is_empty(&self) -> bool {
        self.counts.is_empty()
    }
}

impl<K: Ord + Clone> CountDistribution<K> {
    /// Deterministically split the first `selected` members of this marginal
    /// distribution in canonical key order.
    ///
    /// This operation preserves only the marginal represented by this
    /// distribution. It does not invent correlations with other dimensions.
    pub fn split_prefix(&self, selected: u64) -> Result<(Self, Self), PopulationError> {
        if selected > self.total {
            return Err(PopulationError::RequestedSplitExceedsPopulation {
                requested: selected,
                available: self.total,
            });
        }

        let mut selected_remaining = selected;
        let mut selected_counts = BTreeMap::new();
        let mut remainder_counts = BTreeMap::new();

        for (key, count) in &self.counts {
            let take = selected_remaining.min(*count);
            let keep = *count - take;
            if take != 0 {
                selected_counts.insert(key.clone(), take);
                selected_remaining -= take;
            }
            if keep != 0 {
                remainder_counts.insert(key.clone(), keep);
            }
        }

        debug_assert_eq!(selected_remaining, 0);
        Ok((Self::new(selected_counts)?, Self::new(remainder_counts)?))
    }

    /// Merge two marginals of the same dimension, preserving exact counts.
    pub fn merge(&self, other: &Self) -> Result<Self, PopulationError> {
        let mut counts = self.counts.clone();
        for (key, count) in &other.counts {
            let existing = counts.get(key).copied().unwrap_or(0);
            let combined = existing
                .checked_add(*count)
                .ok_or(PopulationError::CountOverflow)?;
            counts.insert(key.clone(), combined);
        }
        Self::new(counts)
    }
}

/// Canonical aggregate population truth.
///
/// `biomass_milligrams` is fixed-point integer state rather than floating point
/// so population materialization/reduction can later conserve coarse biomass
/// exactly. One regional population can represent up to ~18 trillion kg while
/// retaining milligram resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopulationState {
    count: u64,
    biomass_milligrams: u64,
    age: CountDistribution<PopulationAgeBand>,
    condition: CountDistribution<PopulationConditionBand>,
    occupancy: CountDistribution<PopulationCell>,
}

impl PopulationState {
    pub fn new(
        count: u64,
        biomass_milligrams: u64,
        age: CountDistribution<PopulationAgeBand>,
        condition: CountDistribution<PopulationConditionBand>,
        occupancy: CountDistribution<PopulationCell>,
    ) -> Result<Self, PopulationError> {
        validate_distribution_total("age", count, age.total())?;
        validate_distribution_total("condition", count, condition.total())?;
        validate_distribution_total("occupancy", count, occupancy.total())?;

        if count == 0 && biomass_milligrams != 0 {
            return Err(PopulationError::NonZeroBiomassForEmptyPopulation { biomass_milligrams });
        }

        Ok(Self {
            count,
            biomass_milligrams,
            age,
            condition,
            occupancy,
        })
    }

    pub const fn count(&self) -> u64 {
        self.count
    }

    pub const fn biomass_milligrams(&self) -> u64 {
        self.biomass_milligrams
    }

    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn age_distribution(&self) -> &CountDistribution<PopulationAgeBand> {
        &self.age
    }

    pub fn condition_distribution(&self) -> &CountDistribution<PopulationConditionBand> {
        &self.condition
    }

    pub fn occupancy_distribution(&self) -> &CountDistribution<PopulationCell> {
        &self.occupancy
    }

    /// Derived convenience metric only; canonical biomass remains integer.
    pub fn mean_biomass_milligrams(&self) -> Option<f64> {
        (self.count != 0).then(|| self.biomass_milligrams as f64 / self.count as f64)
    }

    /// Verify every redundant count representation still agrees.
    pub fn verify(&self) -> Result<(), PopulationError> {
        validate_distribution_total("age", self.count, self.age.total())?;
        validate_distribution_total("condition", self.count, self.condition.total())?;
        validate_distribution_total("occupancy", self.count, self.occupancy.total())?;
        if self.count == 0 && self.biomass_milligrams != 0 {
            return Err(PopulationError::NonZeroBiomassForEmptyPopulation {
                biomass_milligrams: self.biomass_milligrams,
            });
        }
        Ok(())
    }

    /// Split this population into a deterministic selected prefix and a coarse
    /// remainder without creating or destroying headcount or biomass.
    ///
    /// Each marginal distribution is split independently in canonical key
    /// order. Because the coarse state stores marginals rather than a joint
    /// distribution, this operation intentionally does not claim that selected
    /// age/condition/occupancy bins belonged to the same historical organisms.
    pub fn split_prefix(&self, selected: u64) -> Result<(Self, Self), PopulationError> {
        if selected > self.count {
            return Err(PopulationError::RequestedSplitExceedsPopulation {
                requested: selected,
                available: self.count,
            });
        }

        let (selected_age, remainder_age) = self.age.split_prefix(selected)?;
        let (selected_condition, remainder_condition) = self.condition.split_prefix(selected)?;
        let (selected_occupancy, remainder_occupancy) = self.occupancy.split_prefix(selected)?;
        let selected_biomass = prefix_biomass(self.biomass_milligrams, self.count, selected)?;
        let remainder_biomass = self
            .biomass_milligrams
            .checked_sub(selected_biomass)
            .ok_or(PopulationError::BiomassArithmeticOverflow)?;
        let remainder_count = self.count - selected;

        Ok((
            Self::new(
                selected,
                selected_biomass,
                selected_age,
                selected_condition,
                selected_occupancy,
            )?,
            Self::new(
                remainder_count,
                remainder_biomass,
                remainder_age,
                remainder_condition,
                remainder_occupancy,
            )?,
        ))
    }

    /// Recombine two coarse population partitions exactly.
    ///
    /// This merges marginals and biomass only. It does not reconstruct joint
    /// correlations that were not present in the source representation.
    pub fn merge(&self, other: &Self) -> Result<Self, PopulationError> {
        let count = self
            .count
            .checked_add(other.count)
            .ok_or(PopulationError::CountOverflow)?;
        let biomass_milligrams = self
            .biomass_milligrams
            .checked_add(other.biomass_milligrams)
            .ok_or(PopulationError::BiomassArithmeticOverflow)?;

        Self::new(
            count,
            biomass_milligrams,
            self.age.merge(&other.age)?,
            self.condition.merge(&other.condition)?,
            self.occupancy.merge(&other.occupancy)?,
        )
    }

    /// Materialize this already-reserved population partition into an ephemeral
    /// local working set.
    ///
    /// The resulting member tuples are *derived representation*, not stable
    /// biological identity. Marginals are interleaved deterministically using
    /// `seed` to avoid sorted-bin alignment, but any cross-dimension
    /// correlations are synthetic because the coarse source never stored a
    /// joint distribution.
    pub fn materialize_derived(
        &self,
        seed: MaterializationSeed,
        max_individuals: usize,
    ) -> Result<DerivedPopulation, PopulationError> {
        self.verify()?;
        let count = usize::try_from(self.count)
            .map_err(|_| PopulationError::PopulationTooLargeToMaterialize { count: self.count })?;
        if count > max_individuals {
            return Err(PopulationError::MaterializationLimitExceeded {
                count: self.count,
                limit: max_individuals,
            });
        }

        let ages = interleaved_values(&self.age, seed, 0xA6E1_5D31, count)?;
        let conditions = interleaved_values(&self.condition, seed, 0xC04D_1710, count)?;
        let cells = interleaved_values(&self.occupancy, seed, 0x5A71_A100, count)?;

        let mut members = Vec::with_capacity(count);
        let base_biomass = self.biomass_milligrams.checked_div(self.count).unwrap_or(0);
        let biomass_remainder = self.biomass_milligrams.checked_rem(self.count).unwrap_or(0);

        for index in 0..count {
            let ordinal = u64::try_from(index).map_err(|_| {
                PopulationError::PopulationTooLargeToMaterialize { count: self.count }
            })?;
            let biomass_milligrams = base_biomass + u64::from(ordinal < biomass_remainder);
            members.push(DerivedPopulationMember {
                ordinal: DerivedOrdinal(ordinal),
                age: ages[index],
                condition: conditions[index],
                cell: cells[index],
                biomass_milligrams,
            });
        }

        Ok(DerivedPopulation { seed, members })
    }
}

/// Materialization context used only to choose a deterministic derived tuple
/// arrangement. It is not an organism identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterializationSeed(pub u64);

/// Ephemeral ordinal inside one materialization result.
///
/// This value must not be persisted or interpreted as stable organism identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DerivedOrdinal(pub u64);

/// One derived local working member.
///
/// The tuple is a deterministic projection of population marginals. Unless a
/// future authoritative organism record says otherwise, the association among
/// age, condition, and cell is synthetic representation rather than historical
/// identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DerivedPopulationMember {
    pub ordinal: DerivedOrdinal,
    pub age: PopulationAgeBand,
    pub condition: PopulationConditionBand,
    pub cell: PopulationCell,
    pub biomass_milligrams: u64,
}

/// Bounded ephemeral working set derived from one reserved population slice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedPopulation {
    seed: MaterializationSeed,
    members: Vec<DerivedPopulationMember>,
}

impl DerivedPopulation {
    pub const fn seed(&self) -> MaterializationSeed {
        self.seed
    }

    pub fn members(&self) -> &[DerivedPopulationMember] {
        &self.members
    }

    pub fn len(&self) -> usize {
        self.members.len()
    }

    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Reduce an unchanged or locally updated working set back into canonical
    /// coarse marginals and exact biomass.
    ///
    /// This intentionally preserves only the aggregate information represented
    /// by `PopulationState`; synthetic cross-dimension tuple correlations are
    /// discarded again during reduction.
    pub fn reduce_to_population(&self) -> Result<PopulationState, PopulationError> {
        let count =
            u64::try_from(self.members.len()).map_err(|_| PopulationError::CountOverflow)?;
        let mut biomass_milligrams = 0u64;
        let mut ages = BTreeMap::new();
        let mut conditions = BTreeMap::new();
        let mut cells = BTreeMap::new();

        for (index, member) in self.members.iter().enumerate() {
            let expected = u64::try_from(index).map_err(|_| PopulationError::CountOverflow)?;
            if member.ordinal != DerivedOrdinal(expected) {
                return Err(PopulationError::NonCanonicalDerivedOrdinal {
                    expected,
                    actual: member.ordinal.0,
                });
            }
            biomass_milligrams = biomass_milligrams
                .checked_add(member.biomass_milligrams)
                .ok_or(PopulationError::BiomassArithmeticOverflow)?;
            increment_bin(&mut ages, member.age)?;
            increment_bin(&mut conditions, member.condition)?;
            increment_bin(&mut cells, member.cell)?;
        }

        PopulationState::new(
            count,
            biomass_milligrams,
            CountDistribution::new(ages)?,
            CountDistribution::new(conditions)?,
            CountDistribution::new(cells)?,
        )
    }
}

fn interleaved_values<K: Ord + Copy>(
    distribution: &CountDistribution<K>,
    seed: MaterializationSeed,
    salt: u64,
    expected_len: usize,
) -> Result<Vec<K>, PopulationError> {
    let mut bins = distribution
        .bins()
        .map(|(key, count)| (*key, count))
        .collect::<Vec<_>>();
    if bins.is_empty() {
        return Ok(Vec::new());
    }

    let rotation = (mix64(seed.0 ^ salt) % bins.len() as u64) as usize;
    bins.rotate_left(rotation);

    let mut values = Vec::with_capacity(expected_len);
    while values.len() < expected_len {
        let before = values.len();
        for (key, remaining) in &mut bins {
            if *remaining != 0 {
                values.push(*key);
                *remaining -= 1;
            }
        }
        if values.len() == before {
            return Err(PopulationError::DistributionTotalMismatch {
                dimension: "materialization",
                expected: expected_len as u64,
                actual: values.len() as u64,
            });
        }
    }

    if values.len() != expected_len {
        return Err(PopulationError::DistributionTotalMismatch {
            dimension: "materialization",
            expected: expected_len as u64,
            actual: values.len() as u64,
        });
    }
    Ok(values)
}

fn increment_bin<K: Ord + Copy>(
    counts: &mut BTreeMap<K, u64>,
    key: K,
) -> Result<(), PopulationError> {
    let next = counts
        .get(&key)
        .copied()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or(PopulationError::CountOverflow)?;
    counts.insert(key, next);
    Ok(())
}

fn mix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn prefix_biomass(
    total_biomass: u64,
    total_count: u64,
    selected: u64,
) -> Result<u64, PopulationError> {
    if selected > total_count {
        return Err(PopulationError::RequestedSplitExceedsPopulation {
            requested: selected,
            available: total_count,
        });
    }
    if total_count == 0 {
        return Ok(0);
    }

    let base = total_biomass / total_count;
    let remainder = total_biomass % total_count;
    let base_selected = base
        .checked_mul(selected)
        .ok_or(PopulationError::BiomassArithmeticOverflow)?;
    base_selected
        .checked_add(remainder.min(selected))
        .ok_or(PopulationError::BiomassArithmeticOverflow)
}

fn validate_distribution_total(
    dimension: &'static str,
    expected: u64,
    actual: u64,
) -> Result<(), PopulationError> {
    if expected == actual {
        Ok(())
    } else {
        Err(PopulationError::DistributionTotalMismatch {
            dimension,
            expected,
            actual,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PopulationError {
    ZeroCountBin,
    CountOverflow,
    BiomassArithmeticOverflow,
    RequestedSplitExceedsPopulation {
        requested: u64,
        available: u64,
    },
    MaterializationLimitExceeded {
        count: u64,
        limit: usize,
    },
    PopulationTooLargeToMaterialize {
        count: u64,
    },
    NonCanonicalDerivedOrdinal {
        expected: u64,
        actual: u64,
    },
    DistributionTotalMismatch {
        dimension: &'static str,
        expected: u64,
        actual: u64,
    },
    NonZeroBiomassForEmptyPopulation {
        biomass_milligrams: u64,
    },
}

impl fmt::Display for PopulationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroCountBin => write!(
                formatter,
                "population distributions cannot store zero-count bins"
            ),
            Self::CountOverflow => write!(formatter, "population distribution count overflow"),
            Self::BiomassArithmeticOverflow => {
                write!(formatter, "population biomass arithmetic overflow")
            }
            Self::RequestedSplitExceedsPopulation {
                requested,
                available,
            } => write!(
                formatter,
                "cannot split {requested} organisms from population of {available}"
            ),
            Self::MaterializationLimitExceeded { count, limit } => write!(
                formatter,
                "population of {count} exceeds materialization limit {limit}"
            ),
            Self::PopulationTooLargeToMaterialize { count } => write!(
                formatter,
                "population of {count} cannot be represented by this platform's address space"
            ),
            Self::NonCanonicalDerivedOrdinal { expected, actual } => write!(
                formatter,
                "derived population ordinal {actual} is non-canonical; expected {expected}"
            ),
            Self::DistributionTotalMismatch {
                dimension,
                expected,
                actual,
            } => write!(
                formatter,
                "population {dimension} distribution totals {actual}, expected {expected}"
            ),
            Self::NonZeroBiomassForEmptyPopulation { biomass_milligrams } => write!(
                formatter,
                "empty population cannot retain {biomass_milligrams} mg living biomass"
            ),
        }
    }
}

impl Error for PopulationError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn distribution<K: Ord>(pairs: impl IntoIterator<Item = (K, u64)>) -> CountDistribution<K> {
        CountDistribution::new(pairs.into_iter().collect()).unwrap()
    }

    fn valid_population() -> PopulationState {
        PopulationState::new(
            10,
            2_500_003,
            distribution([
                (PopulationAgeBand::Juvenile, 3),
                (PopulationAgeBand::Mature, 7),
            ]),
            distribution([
                (PopulationConditionBand::Thriving, 6),
                (PopulationConditionBand::Stable, 4),
            ]),
            distribution([
                (PopulationCell::new(0, 0, 0), 4),
                (PopulationCell::new(1, 0, 0), 6),
            ]),
        )
        .unwrap()
    }

    #[test]
    fn valid_population_has_one_exact_headcount_across_dimensions() {
        let population = valid_population();
        assert_eq!(population.count(), 10);
        assert_eq!(population.age_distribution().total(), 10);
        assert_eq!(population.condition_distribution().total(), 10);
        assert_eq!(population.occupancy_distribution().total(), 10);
        assert_eq!(population.biomass_milligrams(), 2_500_003);
        assert_eq!(population.verify(), Ok(()));
    }

    #[test]
    fn mismatched_distribution_fails_closed() {
        let result = PopulationState::new(
            10,
            100,
            distribution([(PopulationAgeBand::Mature, 9)]),
            distribution([(PopulationConditionBand::Stable, 10)]),
            distribution([(PopulationCell::new(0, 0, 0), 10)]),
        );
        assert_eq!(
            result,
            Err(PopulationError::DistributionTotalMismatch {
                dimension: "age",
                expected: 10,
                actual: 9,
            })
        );
    }

    #[test]
    fn zero_count_bins_are_noncanonical() {
        let mut counts = BTreeMap::new();
        counts.insert(PopulationAgeBand::Juvenile, 0);
        assert_eq!(
            CountDistribution::new(counts),
            Err(PopulationError::ZeroCountBin)
        );
    }

    #[test]
    fn distribution_overflow_is_rejected() {
        let mut counts = BTreeMap::new();
        counts.insert(PopulationCell::new(0, 0, 0), u64::MAX);
        counts.insert(PopulationCell::new(1, 0, 0), 1);
        assert_eq!(
            CountDistribution::new(counts),
            Err(PopulationError::CountOverflow)
        );
    }

    #[test]
    fn empty_population_cannot_retain_living_biomass() {
        let empty_age = CountDistribution::new(BTreeMap::new()).unwrap();
        let empty_condition = CountDistribution::new(BTreeMap::new()).unwrap();
        let empty_occupancy = CountDistribution::new(BTreeMap::new()).unwrap();
        let result = PopulationState::new(0, 1, empty_age, empty_condition, empty_occupancy);
        assert_eq!(
            result,
            Err(PopulationError::NonZeroBiomassForEmptyPopulation {
                biomass_milligrams: 1,
            })
        );
    }

    #[test]
    fn exact_biomass_remains_canonical_while_mean_is_derived() {
        let population = valid_population();
        assert_eq!(population.biomass_milligrams(), 2_500_003);
        assert_eq!(population.mean_biomass_milligrams(), Some(250_000.3));
    }

    #[test]
    fn split_conserves_every_marginal_count_and_biomass() {
        let original = valid_population();
        let (selected, remainder) = original.split_prefix(4).unwrap();

        assert_eq!(selected.count(), 4);
        assert_eq!(remainder.count(), 6);
        assert_eq!(selected.count() + remainder.count(), original.count());
        assert_eq!(
            selected.biomass_milligrams() + remainder.biomass_milligrams(),
            original.biomass_milligrams()
        );
        assert_eq!(selected.age_distribution().total(), 4);
        assert_eq!(selected.condition_distribution().total(), 4);
        assert_eq!(selected.occupancy_distribution().total(), 4);
        assert_eq!(remainder.age_distribution().total(), 6);
        assert_eq!(remainder.condition_distribution().total(), 6);
        assert_eq!(remainder.occupancy_distribution().total(), 6);
    }

    #[test]
    fn split_then_merge_recovers_exact_source_marginals() {
        let original = valid_population();
        let (selected, remainder) = original.split_prefix(4).unwrap();
        assert_eq!(selected.merge(&remainder).unwrap(), original);
    }

    #[test]
    fn non_divisible_biomass_partition_is_exact_and_deterministic() {
        let population = PopulationState::new(
            3,
            11,
            distribution([(PopulationAgeBand::Mature, 3)]),
            distribution([(PopulationConditionBand::Stable, 3)]),
            distribution([(PopulationCell::new(0, 0, 0), 3)]),
        )
        .unwrap();

        let (first, remainder) = population.split_prefix(1).unwrap();
        assert_eq!(first.biomass_milligrams(), 4);
        assert_eq!(remainder.biomass_milligrams(), 7);
        assert_eq!(first.merge(&remainder).unwrap(), population);
    }

    #[test]
    fn split_bounds_fail_closed() {
        let population = valid_population();
        assert_eq!(
            population.split_prefix(11),
            Err(PopulationError::RequestedSplitExceedsPopulation {
                requested: 11,
                available: 10,
            })
        );
    }

    #[test]
    fn zero_and_full_splits_remain_valid() {
        let original = valid_population();
        let (none, all) = original.split_prefix(0).unwrap();
        assert!(none.is_empty());
        assert_eq!(all, original);

        let (all_again, none_again) = original.split_prefix(original.count()).unwrap();
        assert_eq!(all_again, original);
        assert!(none_again.is_empty());
    }

    #[test]
    fn materialization_is_deterministic_for_same_seed() {
        let population = valid_population();
        let a = population
            .materialize_derived(MaterializationSeed(41), 32)
            .unwrap();
        let b = population
            .materialize_derived(MaterializationSeed(41), 32)
            .unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn materialization_seed_changes_only_derived_tuple_arrangement() {
        let population = valid_population();
        let a = population
            .materialize_derived(MaterializationSeed(1), 32)
            .unwrap();
        let b = population
            .materialize_derived(MaterializationSeed(2), 32)
            .unwrap();

        assert_ne!(a.members(), b.members());
        assert_eq!(a.reduce_to_population().unwrap(), population);
        assert_eq!(b.reduce_to_population().unwrap(), population);
    }

    #[test]
    fn materialize_reduce_round_trip_preserves_exact_coarse_truth() {
        let population = valid_population();
        let materialized = population
            .materialize_derived(MaterializationSeed(77), 32)
            .unwrap();

        assert_eq!(materialized.len(), 10);
        assert_eq!(materialized.reduce_to_population().unwrap(), population);
        assert_eq!(
            materialized
                .members()
                .iter()
                .map(|member| member.biomass_milligrams)
                .sum::<u64>(),
            population.biomass_milligrams()
        );
    }

    #[test]
    fn materialization_limit_fails_closed_before_allocation() {
        let population = valid_population();
        assert_eq!(
            population.materialize_derived(MaterializationSeed(0), 5),
            Err(PopulationError::MaterializationLimitExceeded {
                count: 10,
                limit: 5,
            })
        );
    }

    #[test]
    fn empty_population_materializes_and_reduces_cleanly() {
        let empty = PopulationState::new(
            0,
            0,
            CountDistribution::new(BTreeMap::new()).unwrap(),
            CountDistribution::new(BTreeMap::new()).unwrap(),
            CountDistribution::new(BTreeMap::new()).unwrap(),
        )
        .unwrap();
        let materialized = empty
            .materialize_derived(MaterializationSeed(123), 0)
            .unwrap();
        assert!(materialized.is_empty());
        assert_eq!(materialized.reduce_to_population().unwrap(), empty);
    }
}
