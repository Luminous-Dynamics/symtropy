// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Sparse canonical joint population strata.
//!
//! Independent population marginals are intentionally insufficient whenever
//! future dynamics depend on correlations such as age × condition × location.
//! This module provides an additive canonical representation that stores only
//! occupied joint strata and binds exact headcount plus exact living biomass to
//! each stratum.
//!
//! The conversion from [`StratifiedPopulationState`] to [`PopulationState`] is
//! deliberately one-way and lossy: marginals can be derived from known joint
//! state, but joint state cannot be reconstructed from marginals without
//! inventing correlations that the source never retained.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use crate::population::{
    CountDistribution, PopulationAgeBand, PopulationCell, PopulationConditionBand, PopulationError,
    PopulationState,
};

/// Canonical v0 joint stratum key.
///
/// These are exactly the axes already represented by today's coarse population
/// model. Disease, genotype, development, social group, spatial-structure, and
/// other axes can be added later only when their authority/versioning contract
/// is explicit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PopulationStratumKey {
    pub age: PopulationAgeBand,
    pub condition: PopulationConditionBand,
    pub cell: PopulationCell,
}

impl PopulationStratumKey {
    pub const fn new(
        age: PopulationAgeBand,
        condition: PopulationConditionBand,
        cell: PopulationCell,
    ) -> Self {
        Self {
            age,
            condition,
            cell,
        }
    }
}

/// Exact extensive state owned by one occupied population stratum.
///
/// A canonical stratum never stores zero members. Zero-count strata are absent
/// from the sparse map so one ecological state has one canonical encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PopulationStratum {
    count: u64,
    biomass_milligrams: u64,
}

impl PopulationStratum {
    pub fn new(count: u64, biomass_milligrams: u64) -> Result<Self, StrataError> {
        if count == 0 {
            return Err(StrataError::ZeroCountStratum);
        }
        Ok(Self {
            count,
            biomass_milligrams,
        })
    }

    pub const fn count(self) -> u64 {
        self.count
    }

    pub const fn biomass_milligrams(self) -> u64 {
        self.biomass_milligrams
    }
}

/// Sparse canonical joint population state.
///
/// This type has no public mutation/reservation API. It establishes the
/// information-preserving authority representation first; Level-A reservation,
/// mortality, migration, reproduction, and settlement belong to later layers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StratifiedPopulationState {
    strata: BTreeMap<PopulationStratumKey, PopulationStratum>,
    count: u64,
    biomass_milligrams: u64,
}

impl StratifiedPopulationState {
    /// Construct and validate an exact sparse joint population state.
    pub fn new(
        strata: BTreeMap<PopulationStratumKey, PopulationStratum>,
    ) -> Result<Self, StrataError> {
        let mut count = 0u64;
        let mut biomass_milligrams = 0u64;

        for stratum in strata.values().copied() {
            if stratum.count == 0 {
                return Err(StrataError::ZeroCountStratum);
            }
            count = count
                .checked_add(stratum.count)
                .ok_or(StrataError::CountOverflow)?;
            biomass_milligrams = biomass_milligrams
                .checked_add(stratum.biomass_milligrams)
                .ok_or(StrataError::BiomassArithmeticOverflow)?;
        }

        Ok(Self {
            strata,
            count,
            biomass_milligrams,
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

    pub fn len(&self) -> usize {
        self.strata.len()
    }

    pub fn strata(
        &self,
    ) -> impl Iterator<Item = (&PopulationStratumKey, PopulationStratum)> {
        self.strata.iter().map(|(key, stratum)| (key, *stratum))
    }

    pub fn stratum(&self, key: &PopulationStratumKey) -> Option<PopulationStratum> {
        self.strata.get(key).copied()
    }

    /// Recompute all aggregate invariants from sparse authority state.
    pub fn verify(&self) -> Result<(), StrataError> {
        let reconstructed = Self::new(self.strata.clone())?;
        if reconstructed.count != self.count {
            return Err(StrataError::CachedCountMismatch {
                expected: reconstructed.count,
                actual: self.count,
            });
        }
        if reconstructed.biomass_milligrams != self.biomass_milligrams {
            return Err(StrataError::CachedBiomassMismatch {
                expected: reconstructed.biomass_milligrams,
                actual: self.biomass_milligrams,
            });
        }
        Ok(())
    }

    pub fn age_distribution(
        &self,
    ) -> Result<CountDistribution<PopulationAgeBand>, StrataError> {
        self.count_distribution_by(|key| key.age)
    }

    pub fn condition_distribution(
        &self,
    ) -> Result<CountDistribution<PopulationConditionBand>, StrataError> {
        self.count_distribution_by(|key| key.condition)
    }

    pub fn occupancy_distribution(
        &self,
    ) -> Result<CountDistribution<PopulationCell>, StrataError> {
        self.count_distribution_by(|key| key.cell)
    }

    /// Exact derived biomass by age band.
    pub fn biomass_by_age(&self) -> Result<BTreeMap<PopulationAgeBand, u64>, StrataError> {
        self.biomass_distribution_by(|key| key.age)
    }

    /// Exact derived biomass by condition band.
    pub fn biomass_by_condition(
        &self,
    ) -> Result<BTreeMap<PopulationConditionBand, u64>, StrataError> {
        self.biomass_distribution_by(|key| key.condition)
    }

    /// Exact derived biomass by occupancy cell.
    pub fn biomass_by_cell(&self) -> Result<BTreeMap<PopulationCell, u64>, StrataError> {
        self.biomass_distribution_by(|key| key.cell)
    }

    /// Derive today's marginal coarse representation.
    ///
    /// This operation preserves total count, total biomass, and each v0
    /// marginal exactly, but intentionally discards joint correlations. There
    /// is no inverse constructor from [`PopulationState`] because an inverse
    /// would have to invent covariance.
    pub fn to_marginal_population(&self) -> Result<PopulationState, StrataError> {
        self.verify()?;
        PopulationState::new(
            self.count,
            self.biomass_milligrams,
            self.age_distribution()?,
            self.condition_distribution()?,
            self.occupancy_distribution()?,
        )
        .map_err(StrataError::PopulationProjection)
    }

    fn count_distribution_by<K: Ord + Copy>(
        &self,
        key_of: impl Fn(PopulationStratumKey) -> K,
    ) -> Result<CountDistribution<K>, StrataError> {
        let mut counts = BTreeMap::new();
        for (key, stratum) in self.strata() {
            let marginal_key = key_of(*key);
            let current = counts.get(&marginal_key).copied().unwrap_or(0u64);
            let next = current
                .checked_add(stratum.count)
                .ok_or(StrataError::CountOverflow)?;
            counts.insert(marginal_key, next);
        }
        CountDistribution::new(counts).map_err(StrataError::PopulationProjection)
    }

    fn biomass_distribution_by<K: Ord + Copy>(
        &self,
        key_of: impl Fn(PopulationStratumKey) -> K,
    ) -> Result<BTreeMap<K, u64>, StrataError> {
        let mut biomass = BTreeMap::new();
        for (key, stratum) in self.strata() {
            let marginal_key = key_of(*key);
            let current = biomass.get(&marginal_key).copied().unwrap_or(0u64);
            let next = current
                .checked_add(stratum.biomass_milligrams)
                .ok_or(StrataError::BiomassArithmeticOverflow)?;
            biomass.insert(marginal_key, next);
        }
        Ok(biomass)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrataError {
    ZeroCountStratum,
    CountOverflow,
    BiomassArithmeticOverflow,
    CachedCountMismatch { expected: u64, actual: u64 },
    CachedBiomassMismatch { expected: u64, actual: u64 },
    PopulationProjection(PopulationError),
}

impl fmt::Display for StrataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroCountStratum => {
                write!(formatter, "sparse population strata cannot store zero-count entries")
            }
            Self::CountOverflow => write!(formatter, "stratified population count overflow"),
            Self::BiomassArithmeticOverflow => {
                write!(formatter, "stratified population biomass arithmetic overflow")
            }
            Self::CachedCountMismatch { expected, actual } => write!(
                formatter,
                "stratified cached count {actual} does not match reconstructed count {expected}"
            ),
            Self::CachedBiomassMismatch { expected, actual } => write!(
                formatter,
                "stratified cached biomass {actual} mg does not match reconstructed biomass {expected} mg"
            ),
            Self::PopulationProjection(error) => {
                write!(formatter, "cannot derive marginal population: {error}")
            }
        }
    }
}

impl Error for StrataError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::PopulationProjection(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(
        age: PopulationAgeBand,
        condition: PopulationConditionBand,
        x: i32,
    ) -> PopulationStratumKey {
        PopulationStratumKey::new(age, condition, PopulationCell::new(x, 0, 0))
    }

    fn stratum(count: u64, biomass: u64) -> PopulationStratum {
        PopulationStratum::new(count, biomass).unwrap()
    }

    fn correlated_a() -> StratifiedPopulationState {
        StratifiedPopulationState::new(BTreeMap::from([
            (
                key(
                    PopulationAgeBand::Juvenile,
                    PopulationConditionBand::Stressed,
                    0,
                ),
                stratum(5, 500_000),
            ),
            (
                key(
                    PopulationAgeBand::Mature,
                    PopulationConditionBand::Stable,
                    1,
                ),
                stratum(5, 2_500_000),
            ),
        ]))
        .unwrap()
    }

    fn correlated_b() -> StratifiedPopulationState {
        StratifiedPopulationState::new(BTreeMap::from([
            (
                key(
                    PopulationAgeBand::Juvenile,
                    PopulationConditionBand::Stable,
                    1,
                ),
                stratum(5, 500_000),
            ),
            (
                key(
                    PopulationAgeBand::Mature,
                    PopulationConditionBand::Stressed,
                    0,
                ),
                stratum(5, 2_500_000),
            ),
        ]))
        .unwrap()
    }

    #[test]
    fn exact_joint_state_derives_all_existing_marginals() {
        let population = correlated_a();

        assert_eq!(population.count(), 10);
        assert_eq!(population.biomass_milligrams(), 3_000_000);
        assert_eq!(population.age_distribution().unwrap().total(), 10);
        assert_eq!(
            population
                .age_distribution()
                .unwrap()
                .count(&PopulationAgeBand::Juvenile),
            5
        );
        assert_eq!(
            population
                .condition_distribution()
                .unwrap()
                .count(&PopulationConditionBand::Stressed),
            5
        );
        assert_eq!(
            population
                .occupancy_distribution()
                .unwrap()
                .count(&PopulationCell::new(0, 0, 0)),
            5
        );
        assert_eq!(population.verify(), Ok(()));
    }

    #[test]
    fn identical_marginals_can_hide_different_correlation() {
        let a = correlated_a();
        let b = correlated_b();

        assert_ne!(a, b);
        assert_eq!(a.to_marginal_population().unwrap(), b.to_marginal_population().unwrap());

        // The marginal view cannot tell whether stressed organisms are the
        // juveniles or the mature members, nor where those pairings occur.
        assert_eq!(
            a.condition_distribution().unwrap(),
            b.condition_distribution().unwrap()
        );
        assert_eq!(a.age_distribution().unwrap(), b.age_distribution().unwrap());
        assert_eq!(
            a.occupancy_distribution().unwrap(),
            b.occupancy_distribution().unwrap()
        );
    }

    #[test]
    fn exact_biomass_stays_bound_to_joint_strata() {
        let population = correlated_a();
        let by_age = population.biomass_by_age().unwrap();
        let by_condition = population.biomass_by_condition().unwrap();
        let by_cell = population.biomass_by_cell().unwrap();

        assert_eq!(by_age[&PopulationAgeBand::Juvenile], 500_000);
        assert_eq!(by_age[&PopulationAgeBand::Mature], 2_500_000);
        assert_eq!(by_condition[&PopulationConditionBand::Stressed], 500_000);
        assert_eq!(by_condition[&PopulationConditionBand::Stable], 2_500_000);
        assert_eq!(by_cell[&PopulationCell::new(0, 0, 0)], 500_000);
        assert_eq!(by_cell[&PopulationCell::new(1, 0, 0)], 2_500_000);
    }

    #[test]
    fn marginal_projection_preserves_exact_total_authority() {
        let stratified = correlated_a();
        let marginal = stratified.to_marginal_population().unwrap();

        assert_eq!(marginal.count(), stratified.count());
        assert_eq!(
            marginal.biomass_milligrams(),
            stratified.biomass_milligrams()
        );
        assert_eq!(marginal.age_distribution(), &stratified.age_distribution().unwrap());
        assert_eq!(
            marginal.condition_distribution(),
            &stratified.condition_distribution().unwrap()
        );
        assert_eq!(
            marginal.occupancy_distribution(),
            &stratified.occupancy_distribution().unwrap()
        );
    }

    #[test]
    fn zero_count_strata_are_noncanonical() {
        assert_eq!(
            PopulationStratum::new(0, 0),
            Err(StrataError::ZeroCountStratum)
        );
    }

    #[test]
    fn aggregate_count_overflow_fails_closed() {
        let result = StratifiedPopulationState::new(BTreeMap::from([
            (
                key(
                    PopulationAgeBand::Mature,
                    PopulationConditionBand::Stable,
                    0,
                ),
                stratum(u64::MAX, 0),
            ),
            (
                key(
                    PopulationAgeBand::Elder,
                    PopulationConditionBand::Stable,
                    1,
                ),
                stratum(1, 0),
            ),
        ]));

        assert_eq!(result, Err(StrataError::CountOverflow));
    }

    #[test]
    fn aggregate_biomass_overflow_fails_closed() {
        let result = StratifiedPopulationState::new(BTreeMap::from([
            (
                key(
                    PopulationAgeBand::Mature,
                    PopulationConditionBand::Stable,
                    0,
                ),
                stratum(1, u64::MAX),
            ),
            (
                key(
                    PopulationAgeBand::Elder,
                    PopulationConditionBand::Stable,
                    1,
                ),
                stratum(1, 1),
            ),
        ]));

        assert_eq!(result, Err(StrataError::BiomassArithmeticOverflow));
    }

    #[test]
    fn empty_stratified_population_is_canonical() {
        let population = StratifiedPopulationState::new(BTreeMap::new()).unwrap();
        let marginal = population.to_marginal_population().unwrap();

        assert!(population.is_empty());
        assert_eq!(population.len(), 0);
        assert_eq!(population.count(), 0);
        assert_eq!(population.biomass_milligrams(), 0);
        assert_eq!(marginal.count(), 0);
        assert_eq!(marginal.biomass_milligrams(), 0);
        assert!(marginal.age_distribution().is_empty());
        assert!(marginal.condition_distribution().is_empty());
        assert!(marginal.occupancy_distribution().is_empty());
    }
}
