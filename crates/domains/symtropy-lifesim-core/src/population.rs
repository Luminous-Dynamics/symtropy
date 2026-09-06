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
            return Err(PopulationError::NonZeroBiomassForEmptyPopulation {
                biomass_milligrams,
            });
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
            Self::ZeroCountBin => write!(formatter, "population distributions cannot store zero-count bins"),
            Self::CountOverflow => write!(formatter, "population distribution count overflow"),
            Self::DistributionTotalMismatch {
                dimension,
                expected,
                actual,
            } => write!(
                formatter,
                "population {dimension} distribution totals {actual}, expected {expected}"
            ),
            Self::NonZeroBiomassForEmptyPopulation {
                biomass_milligrams,
            } => write!(
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
            2_500_000,
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
        assert_eq!(population.biomass_milligrams(), 2_500_000);
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
        let result = PopulationState::new(
            0,
            1,
            empty_age,
            empty_condition,
            empty_occupancy,
        );
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
        assert_eq!(population.biomass_milligrams(), 2_500_000);
        assert_eq!(population.mean_biomass_milligrams(), Some(250_000.0));
    }
}
