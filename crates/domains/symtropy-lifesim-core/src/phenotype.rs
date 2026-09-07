// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Keyed deterministic phenotype variation and developmental response.
//!
//! Canonical biological variation must not depend on sequential RNG call order.
//! Each trait is therefore derived independently from stable seed material and
//! a portable trait key. Adding a new trait leaves every existing trait value
//! unchanged.
//!
//! Developmental plasticity is modeled separately through versioned reaction
//! norms. A reaction norm maps one authored environmental/developmental stimulus
//! coordinate to a deterministic response coordinate without introducing new
//! randomness. This lets identical hereditary seeds develop differently under
//! different histories while keeping the mapping reproducible and explicit.

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

/// Version of one deterministic developmental response grammar.
///
/// Changing interpolation, clamping, rounding, or knot semantics requires a
/// new version rather than silently changing historical phenotype development.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReactionNormSchemeVersion(pub u16);

/// Frozen piecewise-linear developmental response scheme.
pub const PIECEWISE_LINEAR_REACTION_NORM_V1: ReactionNormSchemeVersion =
    ReactionNormSchemeVersion(1);

/// One authored knot in a developmental reaction norm.
///
/// Both axes use caller-defined fixed-point quanta. The reaction norm is unit
/// agnostic: species/content schemas own the meaning and scale of those quanta.
/// Keeping interpolation integer-only prevents platform floating-point details
/// from becoming hidden developmental authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReactionNormKnot {
    pub stimulus_q: i32,
    pub response_q: i32,
}

impl ReactionNormKnot {
    pub const fn new(stimulus_q: i32, response_q: i32) -> Self {
        Self {
            stimulus_q,
            response_q,
        }
    }
}

/// Versioned deterministic reaction norm for developmental plasticity.
///
/// The v1 scheme clamps outside the authored stimulus range and performs
/// piecewise-linear interpolation inside it using widened integer arithmetic.
/// Fractional interpolation is rounded to nearest, with exact half ties away
/// from zero. This behavior is part of the scheme contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReactionNorm {
    scheme: ReactionNormSchemeVersion,
    knots: Vec<ReactionNormKnot>,
}

impl ReactionNorm {
    /// Construct a v1 piecewise-linear reaction norm.
    ///
    /// Knot order is canonical input, not something repaired by sorting. The
    /// stimulus coordinate must therefore be strictly increasing so duplicate
    /// or ambiguous authored states fail closed.
    pub fn piecewise_linear_v1(
        knots: impl Into<Vec<ReactionNormKnot>>,
    ) -> Result<Self, PhenotypeError> {
        let knots = knots.into();
        if knots.len() < 2 {
            return Err(PhenotypeError::TooFewReactionNormKnots {
                found: knots.len(),
            });
        }

        for (index, pair) in knots.windows(2).enumerate() {
            let previous = pair[0].stimulus_q;
            let next = pair[1].stimulus_q;
            if next <= previous {
                return Err(PhenotypeError::NonIncreasingReactionNormStimulus {
                    index: index + 1,
                    previous,
                    next,
                });
            }
        }

        Ok(Self {
            scheme: PIECEWISE_LINEAR_REACTION_NORM_V1,
            knots,
        })
    }

    pub const fn scheme(&self) -> ReactionNormSchemeVersion {
        self.scheme
    }

    pub fn knots(&self) -> &[ReactionNormKnot] {
        &self.knots
    }

    /// Evaluate one stimulus coordinate using the frozen v1 interpolation.
    pub fn response_q(&self, stimulus_q: i32) -> i32 {
        debug_assert_eq!(self.scheme, PIECEWISE_LINEAR_REACTION_NORM_V1);
        let first = self.knots[0];
        let last = self.knots[self.knots.len() - 1];

        if stimulus_q <= first.stimulus_q {
            return first.response_q;
        }
        if stimulus_q >= last.stimulus_q {
            return last.response_q;
        }

        let pair = self
            .knots
            .windows(2)
            .find(|pair| stimulus_q <= pair[1].stimulus_q)
            .expect("interior reaction-norm stimulus must belong to one segment");
        interpolate_response_q(pair[0], pair[1], stimulus_q)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PhenotypeError {
    EmptyTraitKey,
    NonPortableTraitKey(String),
    NonFiniteRange { min: f64, max: f64 },
    InvertedRange { min: f64, max: f64 },
    TooFewReactionNormKnots {
        found: usize,
    },
    NonIncreasingReactionNormStimulus {
        index: usize,
        previous: i32,
        next: i32,
    },
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
            Self::TooFewReactionNormKnots { found } => write!(
                formatter,
                "developmental reaction norm requires at least two knots, found {found}"
            ),
            Self::NonIncreasingReactionNormStimulus {
                index,
                previous,
                next,
            } => write!(
                formatter,
                "developmental reaction-norm stimulus is not strictly increasing at knot {index}: {previous} -> {next}"
            ),
        }
    }
}

impl Error for PhenotypeError {}

fn validate_trait_key(trait_key: &str) -> Result<(), PhenotypeError> {
    if trait_key.is_empty() {
        return Err(PhenotypeError::EmptyTraitKey);
    }
    let portable = trait_key.len() <= 96
        && trait_key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b':'));
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

fn interpolate_response_q(
    lower: ReactionNormKnot,
    upper: ReactionNormKnot,
    stimulus_q: i32,
) -> i32 {
    debug_assert!(upper.stimulus_q > lower.stimulus_q);
    debug_assert!(stimulus_q >= lower.stimulus_q && stimulus_q <= upper.stimulus_q);

    let stimulus_span = i128::from(upper.stimulus_q) - i128::from(lower.stimulus_q);
    let stimulus_offset = i128::from(stimulus_q) - i128::from(lower.stimulus_q);
    let response_delta = i128::from(upper.response_q) - i128::from(lower.response_q);
    let scaled_delta = response_delta * stimulus_offset;
    let interpolated_delta = round_ratio_ties_away_from_zero(scaled_delta, stimulus_span);
    let response = i128::from(lower.response_q) + interpolated_delta;

    i32::try_from(response).expect("reaction-norm interpolation remains between i32 endpoints")
}

fn round_ratio_ties_away_from_zero(numerator: i128, positive_denominator: i128) -> i128 {
    debug_assert!(positive_denominator > 0);
    let half = positive_denominator / 2;
    if numerator >= 0 {
        (numerator + half) / positive_denominator
    } else {
        -((-numerator + half) / positive_denominator)
    }
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
        assert!(matches!(seed.unit(""), Err(PhenotypeError::EmptyTraitKey)));
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
        assert_eq!(value.to_bits(), 0x3fe5_5985_ca9a_78c9);
    }

    #[test]
    fn reaction_norm_clamps_and_interpolates_without_randomness() {
        let norm = ReactionNorm::piecewise_linear_v1([
            ReactionNormKnot::new(0, 0),
            ReactionNormKnot::new(1_000, 100),
            ReactionNormKnot::new(2_000, -100),
        ])
        .unwrap();

        assert_eq!(norm.scheme(), PIECEWISE_LINEAR_REACTION_NORM_V1);
        assert_eq!(norm.response_q(-1), 0);
        assert_eq!(norm.response_q(0), 0);
        assert_eq!(norm.response_q(500), 50);
        assert_eq!(norm.response_q(1_000), 100);
        assert_eq!(norm.response_q(1_500), 0);
        assert_eq!(norm.response_q(2_000), -100);
        assert_eq!(norm.response_q(2_001), -100);
    }

    #[test]
    fn reaction_norm_rounding_is_symmetric_for_positive_and_negative_response() {
        let positive = ReactionNorm::piecewise_linear_v1([
            ReactionNormKnot::new(0, 0),
            ReactionNormKnot::new(3, 2),
        ])
        .unwrap();
        let negative = ReactionNorm::piecewise_linear_v1([
            ReactionNormKnot::new(0, 0),
            ReactionNormKnot::new(3, -2),
        ])
        .unwrap();

        assert_eq!(positive.response_q(1), 1);
        assert_eq!(positive.response_q(2), 1);
        assert_eq!(negative.response_q(1), -1);
        assert_eq!(negative.response_q(2), -1);
    }

    #[test]
    fn reaction_norm_rejects_ambiguous_or_unsorted_knots() {
        assert_eq!(
            ReactionNorm::piecewise_linear_v1([ReactionNormKnot::new(0, 1)]),
            Err(PhenotypeError::TooFewReactionNormKnots { found: 1 })
        );
        assert!(matches!(
            ReactionNorm::piecewise_linear_v1([
                ReactionNormKnot::new(0, 0),
                ReactionNormKnot::new(0, 10),
            ]),
            Err(PhenotypeError::NonIncreasingReactionNormStimulus { .. })
        ));
        assert!(matches!(
            ReactionNorm::piecewise_linear_v1([
                ReactionNormKnot::new(10, 0),
                ReactionNormKnot::new(5, 10),
            ]),
            Err(PhenotypeError::NonIncreasingReactionNormStimulus { .. })
        ));
    }

    #[test]
    fn reaction_norm_frozen_vector_detects_unversioned_arithmetic_drift() {
        let norm = ReactionNorm::piecewise_linear_v1([
            ReactionNormKnot::new(0, -1_000),
            ReactionNormKnot::new(1_000, 1_000),
        ])
        .unwrap();

        assert_eq!(norm.response_q(333), -334);
        assert_eq!(norm.response_q(500), 0);
        assert_eq!(norm.response_q(667), 334);
    }
}
