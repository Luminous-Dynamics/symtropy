// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic hyperdimensional-computing primitives.
//!
//! This crate is intentionally independent of Symthaea and the Symtropy
//! physics engine. Research projects can use the same encoder contracts without
//! importing an agent runtime, game framework, or policy layer.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Symthaea-compatible default hypervector dimension.
pub const HDC_DIM: usize = 16_384;
const ZERO_NORM_EPSILON: f32 = 1.0e-8;
const SPARSE_VALUE_EPSILON: f32 = 1.0e-4;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

/// Errors raised when an HDC operation violates a dimensional, numeric, or
/// encoder-contract invariant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HdcError {
    DimensionMismatch { left: usize, right: usize },
    IndexOutOfBounds { index: usize, dim: usize },
    NonFiniteValue,
    InvalidBipolarValue { index: usize, value: i8 },
    InvalidEncoderSpec(String),
}

impl fmt::Display for HdcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DimensionMismatch { left, right } => {
                write!(f, "HDC dimension mismatch: left={left}, right={right}")
            }
            Self::IndexOutOfBounds { index, dim } => {
                write!(f, "HDC index {index} is outside dimension {dim}")
            }
            Self::NonFiniteValue => write!(f, "HDC values and weights must be finite"),
            Self::InvalidBipolarValue { index, value } => {
                write!(
                    f,
                    "bipolar value at index {index} must be -1 or +1, got {value}"
                )
            }
            Self::InvalidEncoderSpec(message) => write!(f, "invalid HDC encoder spec: {message}"),
        }
    }
}

impl std::error::Error for HdcError {}

/// Versioned encoder contract. Any change that alters emitted vectors must
/// increment `schema_version`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncoderSpec {
    pub schema_version: u32,
    pub dimension: usize,
    pub seed: u64,
    pub scalar_levels: usize,
}

impl Default for EncoderSpec {
    fn default() -> Self {
        Self {
            schema_version: 1,
            dimension: HDC_DIM,
            seed: 0x5359_4d54_524f_5059,
            scalar_levels: 257,
        }
    }
}

impl EncoderSpec {
    pub fn validate(&self) -> Result<(), HdcError> {
        if self.schema_version == 0 {
            return Err(HdcError::InvalidEncoderSpec(
                "schema_version must be non-zero".to_owned(),
            ));
        }
        if self.dimension == 0 {
            return Err(HdcError::InvalidEncoderSpec(
                "dimension must be non-zero".to_owned(),
            ));
        }
        if self.scalar_levels < 2 {
            return Err(HdcError::InvalidEncoderSpec(
                "scalar_levels must be at least two".to_owned(),
            ));
        }
        Ok(())
    }

    /// Stable, non-cryptographic identifier for compatibility checks and
    /// research provenance.
    pub fn fingerprint(&self) -> u64 {
        let mut hash = StableHash64::new();
        hash.write_u32(self.schema_version);
        hash.write_u64(self.dimension as u64);
        hash.write_u64(self.seed);
        hash.write_u64(self.scalar_levels as u64);
        hash.finish()
    }
}

/// Dense floating-point hypervector retained for compatibility with existing
/// Symtropy and Symthaea integrations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContinuousHV {
    pub values: Vec<f32>,
}

impl ContinuousHV {
    pub fn zero(dim: usize) -> Self {
        Self {
            values: vec![0.0; dim],
        }
    }

    pub fn from_vec(values: Vec<f32>) -> Self {
        Self { values }
    }

    /// Generate a deterministic dense vector using the crate's version-stable
    /// SplitMix64 stream rather than a dependency-defined RNG algorithm.
    pub fn random(dim: usize, seed: u64) -> Self {
        let mut rng = SplitMix64::new(seed);
        let values = (0..dim)
            .map(|_| {
                let unit = (rng.next_u64() >> 40) as f32 / ((1_u32 << 24) - 1) as f32;
                unit.mul_add(2.0, -1.0)
            })
            .collect();
        Self { values }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn l2_norm(&self) -> f32 {
        self.l2_norm_f64() as f32
    }

    fn l2_norm_f64(&self) -> f64 {
        self.values
            .iter()
            .map(|&value| {
                let value = f64::from(value);
                value * value
            })
            .sum::<f64>()
            .sqrt()
    }

    pub fn l2_normalize(&mut self) -> Result<(), HdcError> {
        ensure_finite(&self.values)?;
        let norm = self.l2_norm_f64();
        if norm > f64::from(ZERO_NORM_EPSILON) {
            for value in &mut self.values {
                *value = (f64::from(*value) / norm) as f32;
            }
        }
        Ok(())
    }

    pub fn scale_in_place(&mut self, weight: f32) -> Result<(), HdcError> {
        if !weight.is_finite() {
            return Err(HdcError::NonFiniteValue);
        }
        ensure_finite(&self.values)?;
        for value in &mut self.values {
            *value *= weight;
        }
        Ok(())
    }

    /// Element-wise multiplication binding for dense vectors.
    pub fn bind(&self, other: &Self) -> Result<Self, HdcError> {
        ensure_same_dimension(self.len(), other.len())?;
        ensure_finite(&self.values)?;
        ensure_finite(&other.values)?;
        Ok(Self {
            values: self
                .values
                .iter()
                .zip(&other.values)
                .map(|(left, right)| left * right)
                .collect(),
        })
    }

    /// Circularly permute dimensions. Permutation is invertible by applying
    /// the negative offset.
    pub fn permute(&self, offset: i64) -> Self {
        if self.is_empty() {
            return self.clone();
        }
        let len = self.len() as i64;
        let offset = offset.rem_euclid(len) as usize;
        let mut values = vec![0.0; self.len()];
        for (index, value) in self.values.iter().copied().enumerate() {
            values[(index + offset) % self.len()] = value;
        }
        Self { values }
    }

    /// Sum-bundle vectors. Empty input returns a zero-dimensional vector.
    pub fn bundle(vectors: &[&Self]) -> Self {
        let Some(first) = vectors.first() else {
            return Self::zero(0);
        };
        let mut output = Self::zero(first.len());
        for vector in vectors {
            assert_eq!(
                vector.len(),
                output.len(),
                "ContinuousHV::bundle dimension mismatch"
            );
            for (sum, value) in output.values.iter_mut().zip(&vector.values) {
                *sum += *value;
            }
        }
        output
    }

    /// Replace `self` with `self_weight * self + other_weight * other`.
    pub fn lerp_in_place(
        &mut self,
        other: &Self,
        self_weight: f32,
        other_weight: f32,
    ) -> Result<(), HdcError> {
        ensure_same_dimension(self.len(), other.len())?;
        if !self_weight.is_finite() || !other_weight.is_finite() {
            return Err(HdcError::NonFiniteValue);
        }
        ensure_finite(&self.values)?;
        ensure_finite(&other.values)?;
        for (value, other_value) in self.values.iter_mut().zip(&other.values) {
            *value = self_weight * *value + other_weight * *other_value;
        }
        Ok(())
    }

    /// Cosine similarity in `[-1, 1]`. Zero vectors have zero similarity.
    pub fn similarity(&self, other: &Self) -> Result<f32, HdcError> {
        ensure_same_dimension(self.len(), other.len())?;
        ensure_finite(&self.values)?;
        ensure_finite(&other.values)?;
        let dot: f64 = self
            .values
            .iter()
            .zip(&other.values)
            .map(|(left, right)| f64::from(*left) * f64::from(*right))
            .sum();
        let denominator = self.l2_norm_f64() * other.l2_norm_f64();
        if denominator <= f64::from(ZERO_NORM_EPSILON) {
            return Ok(0.0);
        }
        Ok((dot / denominator).clamp(-1.0, 1.0) as f32)
    }
}

/// Dense bipolar vector. Every component is exactly `-1` or `+1`, making
/// binding, bundling, and similarity deterministic and inexpensive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BipolarHV {
    pub values: Vec<i8>,
}

impl BipolarHV {
    pub fn new(values: Vec<i8>) -> Result<Self, HdcError> {
        for (index, value) in values.iter().copied().enumerate() {
            if value != -1 && value != 1 {
                return Err(HdcError::InvalidBipolarValue { index, value });
            }
        }
        Ok(Self { values })
    }

    pub fn random(dim: usize, seed: u64) -> Self {
        let mut rng = SplitMix64::new(seed);
        let values = (0..dim)
            .map(|_| if rng.next_u64() & 1 == 0 { -1 } else { 1 })
            .collect();
        Self { values }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Bipolar binding is element-wise multiplication and is self-inverse.
    pub fn bind(&self, other: &Self) -> Result<Self, HdcError> {
        ensure_same_dimension(self.len(), other.len())?;
        Ok(Self {
            values: self
                .values
                .iter()
                .zip(&other.values)
                .map(|(left, right)| left * right)
                .collect(),
        })
    }

    pub fn permute(&self, offset: i64) -> Self {
        if self.is_empty() {
            return self.clone();
        }
        let len = self.len() as i64;
        let offset = offset.rem_euclid(len) as usize;
        let mut values = vec![1; self.len()];
        for (index, value) in self.values.iter().copied().enumerate() {
            values[(index + offset) % self.len()] = value;
        }
        Self { values }
    }

    /// Normalized bipolar dot product, equivalent to one minus twice the
    /// normalized Hamming distance.
    pub fn similarity(&self, other: &Self) -> Result<f32, HdcError> {
        ensure_same_dimension(self.len(), other.len())?;
        if self.is_empty() {
            return Ok(0.0);
        }
        let dot: i64 = self
            .values
            .iter()
            .zip(&other.values)
            .map(|(left, right)| i64::from(*left) * i64::from(*right))
            .sum();
        Ok(dot as f32 / self.len() as f32)
    }

    pub fn to_continuous(&self) -> ContinuousHV {
        ContinuousHV::from_vec(self.values.iter().map(|value| f32::from(*value)).collect())
    }
}

/// Majority-vote bundler. Ties are resolved deterministically using a supplied
/// tie vector, avoiding platform- or insertion-order-dependent results.
#[derive(Debug, Clone)]
pub struct BipolarBundle {
    sums: Vec<i64>,
    count: usize,
}

impl BipolarBundle {
    pub fn new(dim: usize) -> Self {
        Self {
            sums: vec![0; dim],
            count: 0,
        }
    }

    pub fn add(&mut self, vector: &BipolarHV) -> Result<(), HdcError> {
        self.add_weighted(vector, 1)
    }

    pub fn add_weighted(&mut self, vector: &BipolarHV, weight: i32) -> Result<(), HdcError> {
        ensure_same_dimension(self.sums.len(), vector.len())?;
        for (sum, value) in self.sums.iter_mut().zip(&vector.values) {
            *sum += i64::from(weight) * i64::from(*value);
        }
        self.count += 1;
        Ok(())
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn finish(&self, tie_breaker: &BipolarHV) -> Result<BipolarHV, HdcError> {
        ensure_same_dimension(self.sums.len(), tie_breaker.len())?;
        Ok(BipolarHV {
            values: self
                .sums
                .iter()
                .zip(&tie_breaker.values)
                .map(|(sum, tie)| {
                    if *sum > 0 {
                        1
                    } else if *sum < 0 {
                        -1
                    } else {
                        *tie
                    }
                })
                .collect(),
        })
    }
}

/// Sparse-HDC representation for memory-efficient visual or spatial fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SparseHV {
    pub active_indices: Vec<(u32, f32)>,
    pub dim: usize,
}

impl SparseHV {
    pub fn new(dim: usize) -> Self {
        Self {
            active_indices: Vec::new(),
            dim,
        }
    }

    pub fn add(&mut self, index: u32, value: f32) -> Result<(), HdcError> {
        let index_usize = index as usize;
        if index_usize >= self.dim {
            return Err(HdcError::IndexOutOfBounds {
                index: index_usize,
                dim: self.dim,
            });
        }
        if !value.is_finite() {
            return Err(HdcError::NonFiniteValue);
        }
        if let Some(position) = self
            .active_indices
            .iter()
            .position(|(active_index, _)| *active_index == index)
        {
            let updated = self.active_indices[position].1 + value;
            if updated.abs() <= SPARSE_VALUE_EPSILON {
                self.active_indices.swap_remove(position);
            } else {
                self.active_indices[position].1 = updated;
            }
        } else if value.abs() > SPARSE_VALUE_EPSILON {
            self.active_indices.push((index, value));
        }
        Ok(())
    }

    pub fn similarity_with_continuous(&self, continuous: &ContinuousHV) -> Result<f32, HdcError> {
        ensure_same_dimension(self.dim, continuous.len())?;
        ensure_finite(&continuous.values)?;
        if self
            .active_indices
            .iter()
            .any(|(_, value)| !value.is_finite())
        {
            return Err(HdcError::NonFiniteValue);
        }
        let mut dot = 0.0_f64;
        let mut sparse_norm_sq = 0.0_f64;
        for &(index, value) in &self.active_indices {
            let index = index as usize;
            if index >= self.dim {
                return Err(HdcError::IndexOutOfBounds {
                    index,
                    dim: self.dim,
                });
            }
            dot += f64::from(value) * f64::from(continuous.values[index]);
            sparse_norm_sq += f64::from(value) * f64::from(value);
        }
        let denominator = sparse_norm_sq.sqrt() * continuous.l2_norm_f64();
        if denominator <= f64::from(ZERO_NORM_EPSILON) {
            return Ok(0.0);
        }
        Ok((dot / denominator).clamp(-1.0, 1.0) as f32)
    }
}

/// Deterministic item memory. The same `(spec, namespace, value)` tuple always
/// produces the same vector across supported platforms.
#[derive(Debug, Clone)]
pub struct ItemMemory {
    spec: EncoderSpec,
}

impl ItemMemory {
    pub fn new(spec: EncoderSpec) -> Result<Self, HdcError> {
        spec.validate()?;
        Ok(Self { spec })
    }

    pub fn spec(&self) -> &EncoderSpec {
        &self.spec
    }

    pub fn item(&self, namespace: &str, value: &str) -> BipolarHV {
        let seed = stable_seed(self.spec.seed, namespace, value);
        BipolarHV::random(self.spec.dimension, seed)
    }

    pub fn role(&self, role: &str) -> BipolarHV {
        self.item("role", role)
    }

    pub fn bind_role_value(&self, role: &str, value_namespace: &str, value: &str) -> BipolarHV {
        self.role(role)
            .bind(&self.item(value_namespace, value))
            .expect("item memory always emits a fixed dimension")
    }

    pub fn scalar_encoder(&self, namespace: &str) -> LevelEncoder {
        LevelEncoder::new(
            self.spec.dimension,
            self.spec.scalar_levels,
            stable_seed(self.spec.seed, "scalar", namespace),
        )
        .expect("validated encoder spec produces a valid level encoder")
    }

    pub fn tie_breaker(&self, namespace: &str) -> BipolarHV {
        self.item("tie-breaker", namespace)
    }
}

/// Locality-preserving scalar encoder. Adjacent levels differ in only a small
/// deterministic subset of dimensions, while the two extremes are opposites.
#[derive(Debug, Clone)]
pub struct LevelEncoder {
    dimension: usize,
    levels: usize,
    base: BipolarHV,
    flip_order: Vec<usize>,
}

impl LevelEncoder {
    pub fn new(dimension: usize, levels: usize, seed: u64) -> Result<Self, HdcError> {
        if dimension == 0 {
            return Err(HdcError::InvalidEncoderSpec(
                "level encoder dimension must be non-zero".to_owned(),
            ));
        }
        if levels < 2 {
            return Err(HdcError::InvalidEncoderSpec(
                "level encoder requires at least two levels".to_owned(),
            ));
        }
        let base = BipolarHV::random(dimension, seed ^ 0xa5a5_a5a5_a5a5_a5a5);
        let mut flip_order: Vec<usize> = (0..dimension).collect();
        let mut rng = SplitMix64::new(seed ^ 0x9e37_79b9_7f4a_7c15);
        for index in (1..dimension).rev() {
            let swap_with = (rng.next_u64() % (index as u64 + 1)) as usize;
            flip_order.swap(index, swap_with);
        }
        Ok(Self {
            dimension,
            levels,
            base,
            flip_order,
        })
    }

    pub fn quantize(&self, value: f64, min: f64, max: f64) -> Result<usize, HdcError> {
        if !value.is_finite() || !min.is_finite() || !max.is_finite() {
            return Err(HdcError::NonFiniteValue);
        }
        if max <= min {
            return Err(HdcError::InvalidEncoderSpec(
                "scalar range requires max > min".to_owned(),
            ));
        }
        let normalized = ((value - min) / (max - min)).clamp(0.0, 1.0);
        Ok((normalized * (self.levels - 1) as f64).round() as usize)
    }

    pub fn encode(&self, value: f64, min: f64, max: f64) -> Result<BipolarHV, HdcError> {
        let level = self.quantize(value, min, max)?;
        Ok(self.encode_level(level))
    }

    pub fn encode_level(&self, level: usize) -> BipolarHV {
        let level = level.min(self.levels - 1);
        let flips = level * self.dimension / (self.levels - 1);
        let mut output = self.base.clone();
        for &index in self.flip_order.iter().take(flips) {
            output.values[index] = -output.values[index];
        }
        output
    }
}

/// Backend abstraction used by game and research encoders.
pub trait HypervectorBackend {
    type Vector: Clone;

    fn dimension(&self) -> usize;
    fn item(&self, namespace: &str, value: &str) -> Self::Vector;
    fn bind(&self, left: &Self::Vector, right: &Self::Vector) -> Result<Self::Vector, HdcError>;
    fn bundle(&self, vectors: &[Self::Vector]) -> Result<Self::Vector, HdcError>;
    fn permute(&self, vector: &Self::Vector, offset: i64) -> Self::Vector;
    fn similarity(&self, left: &Self::Vector, right: &Self::Vector) -> Result<f32, HdcError>;
}

#[derive(Debug, Clone)]
pub struct BipolarBackend {
    memory: ItemMemory,
}

impl BipolarBackend {
    pub fn new(spec: EncoderSpec) -> Result<Self, HdcError> {
        Ok(Self {
            memory: ItemMemory::new(spec)?,
        })
    }

    pub fn memory(&self) -> &ItemMemory {
        &self.memory
    }
}

impl HypervectorBackend for BipolarBackend {
    type Vector = BipolarHV;

    fn dimension(&self) -> usize {
        self.memory.spec.dimension
    }

    fn item(&self, namespace: &str, value: &str) -> Self::Vector {
        self.memory.item(namespace, value)
    }

    fn bind(&self, left: &Self::Vector, right: &Self::Vector) -> Result<Self::Vector, HdcError> {
        left.bind(right)
    }

    fn bundle(&self, vectors: &[Self::Vector]) -> Result<Self::Vector, HdcError> {
        let mut accumulator = BipolarBundle::new(self.dimension());
        for vector in vectors {
            accumulator.add(vector)?;
        }
        accumulator.finish(&self.memory.tie_breaker("backend-bundle"))
    }

    fn permute(&self, vector: &Self::Vector, offset: i64) -> Self::Vector {
        vector.permute(offset)
    }

    fn similarity(&self, left: &Self::Vector, right: &Self::Vector) -> Result<f32, HdcError> {
        left.similarity(right)
    }
}

/// Stable non-cryptographic hash used only for deterministic encoders and
/// compatibility fingerprints. Do not use it for security decisions.
#[derive(Debug, Clone)]
pub struct StableHash64 {
    state: u64,
}

impl StableHash64 {
    pub fn new() -> Self {
        Self { state: FNV_OFFSET }
    }

    pub fn with_seed(seed: u64) -> Self {
        let mut hash = Self::new();
        hash.write_u64(seed);
        hash
    }

    pub fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.state ^= u64::from(*byte);
            self.state = self.state.wrapping_mul(FNV_PRIME);
        }
    }

    pub fn write_u32(&mut self, value: u32) {
        self.write(&value.to_le_bytes());
    }

    pub fn write_u64(&mut self, value: u64) {
        self.write(&value.to_le_bytes());
    }

    pub fn finish(&self) -> u64 {
        self.state
    }
}

impl Default for StableHash64 {
    fn default() -> Self {
        Self::new()
    }
}

pub fn stable_seed(seed: u64, namespace: &str, value: &str) -> u64 {
    let mut hash = StableHash64::with_seed(seed);
    hash.write_u64(namespace.len() as u64);
    hash.write(namespace.as_bytes());
    hash.write_u64(value.len() as u64);
    hash.write(value.as_bytes());
    hash.finish()
}

#[derive(Debug, Clone)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }
}

fn ensure_same_dimension(left: usize, right: usize) -> Result<(), HdcError> {
    if left == right {
        Ok(())
    } else {
        Err(HdcError::DimensionMismatch { left, right })
    }
}

fn ensure_finite(values: &[f32]) -> Result<(), HdcError> {
    if values.iter().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(HdcError::NonFiniteValue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small_spec() -> EncoderSpec {
        EncoderSpec {
            schema_version: 1,
            dimension: 1_024,
            seed: 7,
            scalar_levels: 65,
        }
    }

    #[test]
    fn deterministic_item_memory_is_stable_and_namespaced() {
        let memory = ItemMemory::new(small_spec()).unwrap();
        assert_eq!(
            memory.item("shape", "sphere"),
            memory.item("shape", "sphere")
        );
        assert_ne!(
            memory.item("shape", "sphere"),
            memory.item("material", "sphere")
        );
        assert_ne!(memory.item("shape", "sphere"), memory.item("shape", "box"));
    }

    #[test]
    fn bipolar_binding_is_self_inverse() {
        let memory = ItemMemory::new(small_spec()).unwrap();
        let role = memory.role("velocity");
        let value = memory.item("speed", "fast");
        let bound = role.bind(&value).unwrap();
        assert_eq!(bound.bind(&role).unwrap(), value);
    }

    #[test]
    fn permutation_round_trip_is_exact() {
        let vector = BipolarHV::random(257, 11);
        assert_eq!(vector.permute(37).permute(-37), vector);
    }

    #[test]
    fn majority_bundle_recovers_shared_signal_under_sparse_noise() {
        let memory = ItemMemory::new(small_spec()).unwrap();
        let shared = memory.item("event", "collision");
        let mut variants = [shared.clone(), shared.clone(), shared.clone()];
        for (variant_index, variant) in variants.iter_mut().enumerate() {
            for offset in 0..64 {
                let index = (variant_index * 211 + offset * 17) % variant.len();
                variant.values[index] = -variant.values[index];
            }
        }
        let mut bundle = BipolarBundle::new(small_spec().dimension);
        for variant in &variants {
            bundle.add(variant).unwrap();
        }
        let result = bundle.finish(&memory.tie_breaker("test")).unwrap();
        assert!(result.similarity(&shared).unwrap() > 0.95);
    }

    #[test]
    fn adjacent_scalar_levels_are_more_similar_than_extremes() {
        let encoder = LevelEncoder::new(4_096, 257, 17).unwrap();
        let near_a = encoder.encode_level(100);
        let near_b = encoder.encode_level(101);
        let far = encoder.encode_level(256);
        assert!(near_a.similarity(&near_b).unwrap() > 0.98);
        assert!(near_a.similarity(&far).unwrap() < near_a.similarity(&near_b).unwrap());
    }

    #[test]
    fn scalar_extremes_are_opposites() {
        let encoder = LevelEncoder::new(1_024, 65, 19).unwrap();
        assert!(
            (encoder
                .encode_level(0)
                .similarity(&encoder.encode_level(64))
                .unwrap()
                + 1.0)
                .abs()
                < 1.0e-6
        );
    }

    #[test]
    fn encoder_fingerprint_changes_with_contract() {
        let base = small_spec();
        let mut changed = base.clone();
        changed.scalar_levels += 1;
        assert_ne!(base.fingerprint(), changed.fingerprint());
    }

    #[test]
    fn continuous_compatibility_operations_are_deterministic() {
        let mut vector = ContinuousHV::random(64, 5);
        let copy = ContinuousHV::random(64, 5);
        assert_eq!(vector, copy);
        vector.l2_normalize().unwrap();
        assert!((vector.l2_norm() - 1.0).abs() < 1.0e-5);
    }

    #[test]
    fn sparse_add_coalesces_and_checks_bounds() {
        let mut sparse = SparseHV::new(4);
        sparse.add(2, 0.75).unwrap();
        sparse.add(2, 0.25).unwrap();
        assert_eq!(sparse.active_indices, vec![(2, 1.0)]);
        assert_eq!(
            sparse.add(4, 1.0),
            Err(HdcError::IndexOutOfBounds { index: 4, dim: 4 })
        );
    }
}
