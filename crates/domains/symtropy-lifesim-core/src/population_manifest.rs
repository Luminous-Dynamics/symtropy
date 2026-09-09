// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact canonical content identity for [`PopulationState`].
//!
//! A semantic population revision is not enough to identify one exact canonical
//! state across rollback, replay, save/load, or alternate lineages. This module
//! derives a deterministic canonical byte manifest from the complete verified
//! coarse population state. The complete bytes are the exact identity at this
//! layer; no ad-hoc short hash is invented here.

use std::error::Error;
use std::fmt;

use crate::population::{
    PopulationAgeBand, PopulationConditionBand, PopulationError, PopulationState,
};

/// Canonical encoding version for exact coarse-population state manifests.
pub const POPULATION_STATE_MANIFEST_VERSION: u32 = 1;

const MANIFEST_DOMAIN: &[u8] = b"SYMTROPY_POPULATION_STATE_MANIFEST\0";
const SECTION_HEADCOUNT: u8 = 1;
const SECTION_BIOMASS_MILLIGRAMS: u8 = 2;
const SECTION_AGE: u8 = 3;
const SECTION_CONDITION: u8 = 4;
const SECTION_OCCUPANCY: u8 = 5;

/// Complete collision-free semantic identity of one verified `PopulationState`
/// under V1 canonical encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopulationStateManifest {
    bytes: Vec<u8>,
}

impl PopulationStateManifest {
    fn from_verified(population: &PopulationState) -> Self {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(MANIFEST_DOMAIN);
        push_u32(&mut bytes, POPULATION_STATE_MANIFEST_VERSION);

        push_u8(&mut bytes, SECTION_HEADCOUNT);
        push_u64(&mut bytes, population.count());

        push_u8(&mut bytes, SECTION_BIOMASS_MILLIGRAMS);
        push_u64(&mut bytes, population.biomass_milligrams());

        push_u8(&mut bytes, SECTION_AGE);
        let age = population.age_distribution().bins().collect::<Vec<_>>();
        push_len(&mut bytes, age.len());
        for (band, count) in age {
            push_u8(&mut bytes, encode_age_band(*band));
            push_u64(&mut bytes, count);
        }

        push_u8(&mut bytes, SECTION_CONDITION);
        let condition = population.condition_distribution().bins().collect::<Vec<_>>();
        push_len(&mut bytes, condition.len());
        for (band, count) in condition {
            push_u8(&mut bytes, encode_condition_band(*band));
            push_u64(&mut bytes, count);
        }

        push_u8(&mut bytes, SECTION_OCCUPANCY);
        let occupancy = population.occupancy_distribution().bins().collect::<Vec<_>>();
        push_len(&mut bytes, occupancy.len());
        for (cell, count) in occupancy {
            push_i32(&mut bytes, cell.x);
            push_i32(&mut bytes, cell.y);
            push_i32(&mut bytes, cell.z);
            push_u64(&mut bytes, count);
        }

        Self { bytes }
    }

    pub const fn version(&self) -> u32 {
        POPULATION_STATE_MANIFEST_VERSION
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Read-only exact-content view over one canonical coarse population.
///
/// Constructing the view verifies the population first. A later authority
/// boundary may revalidate against current state and fail closed if the exact
/// manifest has changed even when an external revision number was reused.
#[derive(Debug, Clone)]
pub struct ManifestBoundPopulationState<'a> {
    population: &'a PopulationState,
    manifest: PopulationStateManifest,
}

impl<'a> ManifestBoundPopulationState<'a> {
    pub fn new(population: &'a PopulationState) -> Result<Self, PopulationStateIdentityError> {
        population
            .verify()
            .map_err(PopulationStateIdentityError::InvalidPopulation)?;
        let manifest = PopulationStateManifest::from_verified(population);
        Ok(Self {
            population,
            manifest,
        })
    }

    pub const fn population(&self) -> &'a PopulationState {
        self.population
    }

    pub const fn manifest(&self) -> &PopulationStateManifest {
        &self.manifest
    }

    pub fn validate_current(
        &self,
        current: &PopulationState,
    ) -> Result<(), PopulationStateIdentityError> {
        current
            .verify()
            .map_err(PopulationStateIdentityError::InvalidPopulation)?;
        let current_manifest = PopulationStateManifest::from_verified(current);
        if current_manifest != self.manifest {
            return Err(PopulationStateIdentityError::ManifestMismatch);
        }
        Ok(())
    }
}

fn encode_age_band(band: PopulationAgeBand) -> u8 {
    match band {
        PopulationAgeBand::Propagule => 0,
        PopulationAgeBand::Juvenile => 1,
        PopulationAgeBand::Reproductive => 2,
        PopulationAgeBand::Mature => 3,
        PopulationAgeBand::Elder => 4,
        PopulationAgeBand::Dormant => 5,
    }
}

fn encode_condition_band(band: PopulationConditionBand) -> u8 {
    match band {
        PopulationConditionBand::Thriving => 0,
        PopulationConditionBand::Stable => 1,
        PopulationConditionBand::Stressed => 2,
        PopulationConditionBand::Critical => 3,
    }
}

fn push_len(bytes: &mut Vec<u8>, len: usize) {
    let len = u64::try_from(len).expect("population-state manifest section length fits u64");
    push_u64(bytes, len);
}

fn push_u8(bytes: &mut Vec<u8>, value: u8) {
    bytes.push(value);
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_i32(bytes: &mut Vec<u8>, value: i32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PopulationStateIdentityError {
    InvalidPopulation(PopulationError),
    ManifestMismatch,
}

impl fmt::Display for PopulationStateIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPopulation(error) => {
                write!(formatter, "invalid canonical population state: {error}")
            }
            Self::ManifestMismatch => {
                write!(formatter, "canonical population-state content identity changed")
            }
        }
    }
}

impl Error for PopulationStateIdentityError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidPopulation(error) => Some(error),
            Self::ManifestMismatch => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::population::{CountDistribution, PopulationCell};

    fn distribution<K: Ord>(pairs: impl IntoIterator<Item = (K, u64)>) -> CountDistribution<K> {
        CountDistribution::new(pairs.into_iter().collect()).unwrap()
    }

    fn population(
        biomass_milligrams: u64,
        age_pairs: impl IntoIterator<Item = (PopulationAgeBand, u64)>,
        condition_pairs: impl IntoIterator<Item = (PopulationConditionBand, u64)>,
        occupancy_pairs: impl IntoIterator<Item = (PopulationCell, u64)>,
    ) -> PopulationState {
        PopulationState::new(
            10,
            biomass_milligrams,
            distribution(age_pairs),
            distribution(condition_pairs),
            distribution(occupancy_pairs),
        )
        .unwrap()
    }

    fn baseline() -> PopulationState {
        population(
            2_500_000,
            [
                (PopulationAgeBand::Juvenile, 3),
                (PopulationAgeBand::Mature, 7),
            ],
            [
                (PopulationConditionBand::Stable, 8),
                (PopulationConditionBand::Stressed, 2),
            ],
            [
                (PopulationCell::new(0, 0, 0), 4),
                (PopulationCell::new(1, -2, 3), 6),
            ],
        )
    }

    #[test]
    fn identical_semantic_state_has_identical_exact_manifest() {
        let first = baseline();

        let mut ages = BTreeMap::new();
        ages.insert(PopulationAgeBand::Mature, 7);
        ages.insert(PopulationAgeBand::Juvenile, 3);
        let mut conditions = BTreeMap::new();
        conditions.insert(PopulationConditionBand::Stressed, 2);
        conditions.insert(PopulationConditionBand::Stable, 8);
        let mut occupancy = BTreeMap::new();
        occupancy.insert(PopulationCell::new(1, -2, 3), 6);
        occupancy.insert(PopulationCell::new(0, 0, 0), 4);

        let second = PopulationState::new(
            10,
            2_500_000,
            CountDistribution::new(ages).unwrap(),
            CountDistribution::new(conditions).unwrap(),
            CountDistribution::new(occupancy).unwrap(),
        )
        .unwrap();

        let first = ManifestBoundPopulationState::new(&first).unwrap();
        let second = ManifestBoundPopulationState::new(&second).unwrap();
        assert_eq!(first.manifest(), second.manifest());
    }

    #[test]
    fn biomass_only_change_changes_exact_identity() {
        let first = baseline();
        let second = population(
            2_500_001,
            [
                (PopulationAgeBand::Juvenile, 3),
                (PopulationAgeBand::Mature, 7),
            ],
            [
                (PopulationConditionBand::Stable, 8),
                (PopulationConditionBand::Stressed, 2),
            ],
            [
                (PopulationCell::new(0, 0, 0), 4),
                (PopulationCell::new(1, -2, 3), 6),
            ],
        );

        assert_ne!(
            ManifestBoundPopulationState::new(&first).unwrap().manifest(),
            ManifestBoundPopulationState::new(&second).unwrap().manifest()
        );
    }

    #[test]
    fn marginal_or_spatial_change_changes_exact_identity() {
        let first = baseline();
        let age_changed = population(
            2_500_000,
            [
                (PopulationAgeBand::Juvenile, 4),
                (PopulationAgeBand::Mature, 6),
            ],
            [
                (PopulationConditionBand::Stable, 8),
                (PopulationConditionBand::Stressed, 2),
            ],
            [
                (PopulationCell::new(0, 0, 0), 4),
                (PopulationCell::new(1, -2, 3), 6),
            ],
        );
        let spatial_changed = population(
            2_500_000,
            [
                (PopulationAgeBand::Juvenile, 3),
                (PopulationAgeBand::Mature, 7),
            ],
            [
                (PopulationConditionBand::Stable, 8),
                (PopulationConditionBand::Stressed, 2),
            ],
            [
                (PopulationCell::new(0, 0, 0), 4),
                (PopulationCell::new(1, -2, 4), 6),
            ],
        );

        let first = ManifestBoundPopulationState::new(&first).unwrap();
        let age_changed = ManifestBoundPopulationState::new(&age_changed).unwrap();
        let spatial_changed = ManifestBoundPopulationState::new(&spatial_changed).unwrap();
        assert_ne!(first.manifest(), age_changed.manifest());
        assert_ne!(first.manifest(), spatial_changed.manifest());
    }

    #[test]
    fn prepared_state_identity_rejects_changed_current_content() {
        let original = baseline();
        let changed = population(
            2_500_001,
            [
                (PopulationAgeBand::Juvenile, 3),
                (PopulationAgeBand::Mature, 7),
            ],
            [
                (PopulationConditionBand::Stable, 8),
                (PopulationConditionBand::Stressed, 2),
            ],
            [
                (PopulationCell::new(0, 0, 0), 4),
                (PopulationCell::new(1, -2, 3), 6),
            ],
        );
        let prepared = ManifestBoundPopulationState::new(&original).unwrap();

        assert_eq!(prepared.validate_current(&original), Ok(()));
        assert_eq!(
            prepared.validate_current(&changed),
            Err(PopulationStateIdentityError::ManifestMismatch)
        );
    }

    #[test]
    fn manifest_domain_and_version_are_explicit() {
        let population = baseline();
        let manifest = ManifestBoundPopulationState::new(&population)
            .unwrap()
            .manifest()
            .clone();

        assert_eq!(manifest.version(), POPULATION_STATE_MANIFEST_VERSION);
        assert!(manifest.as_bytes().starts_with(MANIFEST_DOMAIN));
        let version_offset = MANIFEST_DOMAIN.len();
        assert_eq!(
            &manifest.as_bytes()[version_offset..version_offset + 4],
            &POPULATION_STATE_MANIFEST_VERSION.to_le_bytes()
        );
    }
}
