// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Sparse canonical joint population strata.
//!
//! Independent population marginals are intentionally insufficient whenever
//! future dynamics depend on correlations such as age × condition × location.
//! This module stores only occupied joint strata and binds exact headcount plus
//! exact living biomass to each stratum.
//!
//! Marginals may be derived from known joint state, but joint state cannot be
//! reconstructed from marginals without inventing correlations that the source
//! never retained. For that reason this module exposes a distinct read-only
//! [`PopulationMarginalSummary`] rather than converting lossy output back into
//! the canonical [`crate::population::PopulationState`] authority type.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use crate::population::{
    CountDistribution, PopulationAgeBand, PopulationCell, PopulationConditionBand, PopulationError,
};

/// Canonical v0 joint stratum key.
///
/// These are exactly the axes already represented by today's qualified coarse
/// population model. Disease, genotype, development, social group, spatial
/// structure, and other axes can be added only when their authority/versioning
/// contract is explicit.
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
/// Zero-count strata are absent from the sparse map so one semantic population
/// has one canonical sparse representation.
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

/// Derived non-authoritative marginal summary of stratified state.
///
/// This type is intentionally distinct from `PopulationState`: losing joint
/// correlation is an information projection, not an authority-preserving state
/// transition. A later downgrade boundary may consume a stratified state only
/// after checking which ecological processes remain enabled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopulationMarginalSummary {
    count: u64,
    biomass_milligrams: u64,
    age: CountDistribution<PopulationAgeBand>,
    condition: CountDistribution<PopulationConditionBand>,
    occupancy: CountDistribution<PopulationCell>,
}

impl PopulationMarginalSummary {
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
    /// Frozen structural schema version for the current age × condition × cell
    /// key plus exact count/biomass extensive state.
    pub const SCHEMA_VERSION: u16 = 1;

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

    pub fn strata(&self) -> impl Iterator<Item = (&PopulationStratumKey, PopulationStratum)> {
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

    pub fn age_distribution(&self) -> Result<CountDistribution<PopulationAgeBand>, StrataError> {
        self.count_distribution_by(|key| key.age)
    }

    pub fn condition_distribution(
        &self,
    ) -> Result<CountDistribution<PopulationConditionBand>, StrataError> {
        self.count_distribution_by(|key| key.condition)
    }

    pub fn occupancy_distribution(&self) -> Result<CountDistribution<PopulationCell>, StrataError> {
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

    /// Derive a read-only marginal summary without transferring authority.
    ///
    /// Total count, total biomass, and every v0 marginal are preserved exactly.
    /// Joint correlations are intentionally discarded. No inverse constructor
    /// from marginal state exists because an inverse would fabricate covariance.
    pub fn marginal_summary(&self) -> Result<PopulationMarginalSummary, StrataError> {
        self.verify()?;
        Ok(PopulationMarginalSummary {
            count: self.count,
            biomass_milligrams: self.biomass_milligrams,
            age: self.age_distribution()?,
            condition: self.condition_distribution()?,
            occupancy: self.occupancy_distribution()?,
        })
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
        CountDistribution::new(counts).map_err(StrataError::MarginalDistribution)
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
    MarginalDistribution(PopulationError),
}

impl fmt::Display for StrataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroCountStratum => {
                write!(
                    formatter,
                    "sparse population strata cannot store zero-count entries"
                )
            }
            Self::CountOverflow => write!(formatter, "stratified population count overflow"),
            Self::BiomassArithmeticOverflow => {
                write!(
                    formatter,
                    "stratified population biomass arithmetic overflow"
                )
            }
            Self::CachedCountMismatch { expected, actual } => write!(
                formatter,
                "stratified cached count {actual} does not match reconstructed count {expected}"
            ),
            Self::CachedBiomassMismatch { expected, actual } => write!(
                formatter,
                "stratified cached biomass {actual} mg does not match reconstructed biomass {expected} mg"
            ),
            Self::MarginalDistribution(error) => {
                write!(
                    formatter,
                    "cannot derive stratified marginal summary: {error}"
                )
            }
        }
    }
}

impl Error for StrataError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::MarginalDistribution(error) => Some(error),
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

        assert_eq!(StratifiedPopulationState::SCHEMA_VERSION, 1);
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
        assert_eq!(a.marginal_summary().unwrap(), b.marginal_summary().unwrap());

        // The marginal view cannot tell whether stressed organisms are the
        // juveniles or mature members, nor where those pairings occur.
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
    fn marginal_summary_preserves_exact_totals_without_claiming_authority() {
        let stratified = correlated_a();
        let summary = stratified.marginal_summary().unwrap();

        assert_eq!(summary.count(), stratified.count());
        assert_eq!(
            summary.biomass_milligrams(),
            stratified.biomass_milligrams()
        );
        assert_eq!(
            summary.age_distribution(),
            &stratified.age_distribution().unwrap()
        );
        assert_eq!(
            summary.condition_distribution(),
            &stratified.condition_distribution().unwrap()
        );
        assert_eq!(
            summary.occupancy_distribution(),
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
                key(PopulationAgeBand::Elder, PopulationConditionBand::Stable, 1),
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
                key(PopulationAgeBand::Elder, PopulationConditionBand::Stable, 1),
                stratum(1, 1),
            ),
        ]));

        assert_eq!(result, Err(StrataError::BiomassArithmeticOverflow));
    }

    #[test]
    fn empty_stratified_population_is_canonical() {
        let population = StratifiedPopulationState::new(BTreeMap::new()).unwrap();
        let summary = population.marginal_summary().unwrap();

        assert!(population.is_empty());
        assert_eq!(population.len(), 0);
        assert_eq!(population.count(), 0);
        assert_eq!(population.biomass_milligrams(), 0);
        assert!(summary.is_empty());
        assert_eq!(summary.count(), 0);
        assert_eq!(summary.biomass_milligrams(), 0);
        assert!(summary.age_distribution().is_empty());
        assert!(summary.condition_distribution().is_empty());
        assert!(summary.occupancy_distribution().is_empty());
    }
}
