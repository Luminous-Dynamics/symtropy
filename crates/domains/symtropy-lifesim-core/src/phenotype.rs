// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Keyed deterministic phenotype variation.
//!
//! Canonical biological variation must not depend on sequential RNG call order.
//! Each trait is therefore derived independently from stable seed material and
//! a portable trait key. Adding a new trait leaves every existing trait value
//! unchanged.

use std::error::Error;
use std::fmt;

/// Stable seed material for deterministic phenotype derivation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PhenotypeSeed {
    /// Stable species or lineage seed.
    pub lineage: u64,
    /// Stable individual/genome seed.
    pub individual: u64,
}

impl PhenotypeSeed {
    pub const fn new(lineage: u64, individual: u64) -> Self {
        Self {
            lineage,
            individual,
        }
    }

    /// Derive a deterministic unit value in `[0, 1)` for one named trait.
    pub fn unit(self, trait_key: &str) -> Result<f64, PhenotypeError> {
        validate_trait_key(trait_key)?;
        let value = keyed_u64(self, trait_key);
        let mantissa = value >> 11;
        Ok(mantissa as f64 / ((1_u64 << 53) as f64))
    }

    /// Sample a deterministic trait within an inclusive authored range.
    pub fn sample(self, trait_key: &str, range: TraitRange) -> Result<f64, PhenotypeError> {
        range.validate()?;
        let unit = self.unit(trait_key)?;
        Ok(range.min + (range.max - range.min) * unit)
    }
}

/// Authored bounds for one continuously varying trait.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TraitRange {
    pub min: f64,
    pub max: f64,
}

impl TraitRange {
    pub const fn new(min: f64, max: f64) -> Self {
        Self { min, max }
    }

    pub fn validate(self) -> Result<(), PhenotypeError> {
        if !self.min.is_finite() || !self.max.is_finite() {
            return Err(PhenotypeError::NonFiniteRange {
                min: self.min,
                max: self.max,
            });
        }
        if self.max < self.min {
            return Err(PhenotypeError::InvertedRange {
                min: self.min,
                max: self.max,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PhenotypeError {
    EmptyTraitKey,
    NonPortableTraitKey(String),
    NonFiniteRange { min: f64, max: f64 },
    InvertedRange { min: f64, max: f64 },
}

impl fmt::Display for PhenotypeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTraitKey => write!(formatter, "phenotype trait key must not be empty"),
            Self::NonPortableTraitKey(key) => {
                write!(formatter, "phenotype trait key is not portable: {key:?}")
            }
            Self::NonFiniteRange { min, max } => {
                write!(formatter, "phenotype range must be finite: {min}..={max}")
            }
            Self::InvertedRange { min, max } => {
                write!(formatter, "phenotype range is inverted: {min}..={max}")
            }
        }
    }
}

impl Error for PhenotypeError {}

fn validate_trait_key(trait_key: &str) -> Result<(), PhenotypeError> {
    if trait_key.is_empty() {
        return Err(PhenotypeError::EmptyTraitKey);
    }
    let portable = trait_key.len() <= 96
        && trait_key.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b':')
        });
    if portable {
        Ok(())
    } else {
        Err(PhenotypeError::NonPortableTraitKey(trait_key.to_owned()))
    }
}

/// Stable key derivation frozen for Living World v0.
///
/// The function deliberately avoids `DefaultHasher`, `rand`, and platform
/// hashing because their byte grammar or algorithm is not a portable canonical
/// contract. FNV-1a provides deterministic byte accumulation; SplitMix64 then
/// avalanches the result before floating-point extraction.
fn keyed_u64(seed: PhenotypeSeed, trait_key: &str) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

    let mut hash = FNV_OFFSET;
    for byte in seed
        .lineage
        .to_le_bytes()
        .into_iter()
        .chain(seed.individual.to_le_bytes())
        .chain([0_u8])
        .chain(trait_key.bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    splitmix64(hash)
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_and_key_are_exactly_deterministic() {
        let seed = PhenotypeSeed::new(17, 91);
        let a = seed.unit("leaf.aspect_ratio").unwrap();
        let b = seed.unit("leaf.aspect_ratio").unwrap();
        assert_eq!(a.to_bits(), b.to_bits());
    }

    #[test]
    fn unrelated_trait_queries_do_not_shift_existing_values() {
        let seed = PhenotypeSeed::new(17, 91);
        let before = seed.unit("branch.internode_length").unwrap();

        let _new_trait = seed.unit("leaf.serration").unwrap();
        let after = seed.unit("branch.internode_length").unwrap();

        assert_eq!(before.to_bits(), after.to_bits());
    }

    #[test]
    fn distinct_trait_keys_are_independently_derived() {
        let seed = PhenotypeSeed::new(17, 91);
        assert_ne!(
            seed.unit("leaf.aspect_ratio").unwrap().to_bits(),
            seed.unit("branch.angle").unwrap().to_bits()
        );
    }

    #[test]
    fn sampled_value_stays_within_authored_bounds() {
        let seed = PhenotypeSeed::new(3, 8);
        let range = TraitRange::new(0.8, 1.2);
        let value = seed.sample("body.scale", range).unwrap();
        assert!((0.8..=1.2).contains(&value));
    }

    #[test]
    fn invalid_keys_and_ranges_fail_closed() {
        let seed = PhenotypeSeed::new(1, 2);
        assert!(matches!(
            seed.unit(""),
            Err(PhenotypeError::EmptyTraitKey)
        ));
        assert!(matches!(
            seed.unit("leaf aspect"),
            Err(PhenotypeError::NonPortableTraitKey(_))
        ));
        assert!(matches!(
            seed.sample("leaf.size", TraitRange::new(2.0, 1.0)),
            Err(PhenotypeError::InvertedRange { .. })
        ));
        assert!(matches!(
            seed.sample("leaf.size", TraitRange::new(0.0, f64::NAN)),
            Err(PhenotypeError::NonFiniteRange { .. })
        ));
    }

    #[test]
    fn frozen_derivation_vector_detects_algorithm_drift() {
        let seed = PhenotypeSeed::new(17, 91);
        let value = seed.unit("leaf.aspect_ratio").unwrap();
        assert_eq!(value.to_bits(), 0x3fdf_b436_a3c9_9d1c);
    }
}
