// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Self-contained HDC primitives.

use std::fmt;

pub const HDC_DIM: usize = 16_384;
const ZERO_NORM_EPSILON: f32 = 1.0e-8;
const SPARSE_VALUE_EPSILON: f32 = 1.0e-4;

/// Errors raised when an HDC operation would violate dimensional or numeric
/// invariants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HdcError {
    DimensionMismatch { left: usize, right: usize },
    IndexOutOfBounds { index: usize, dim: usize },
    NonFiniteValue,
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
        }
    }
}

impl std::error::Error for HdcError {}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ContinuousHV {
    pub values: Vec<f32>,
}

impl ContinuousHV {
    pub fn zero(dim: usize) -> Self {
        Self {
            values: vec![0.0; dim],
        }
    }

    /// Generate a deterministic dense hypervector.
    ///
    /// The vector is intentionally not normalized during construction; callers
    /// that need unit length may call [`Self::l2_normalize`]. Similarity uses
    /// cosine normalization internally, so its result is scale-independent.
    pub fn random(dim: usize, seed: u64) -> Self {
        use rand::{Rng, SeedableRng, rngs::StdRng};
        let mut rng = StdRng::seed_from_u64(seed);
        let values = (0..dim).map(|_| rng.gen_range(-1.0..1.0)).collect();
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
        if self.values.iter().any(|value| !value.is_finite()) {
            return Err(HdcError::NonFiniteValue);
        }
        let norm = self.l2_norm_f64();
        if norm > f64::from(ZERO_NORM_EPSILON) {
            for value in &mut self.values {
                *value = (f64::from(*value) / norm) as f32;
            }
        }
        Ok(())
    }

    /// Replace `self` with `self_weight * self + other_weight * other`.
    pub fn lerp_in_place(
        &mut self,
        other: &Self,
        self_weight: f32,
        other_weight: f32,
    ) -> Result<(), HdcError> {
        ensure_same_dimension(self.len(), other.len())?;
        if !self_weight.is_finite()
            || !other_weight.is_finite()
            || self.values.iter().any(|value| !value.is_finite())
            || other.values.iter().any(|value| !value.is_finite())
        {
            return Err(HdcError::NonFiniteValue);
        }

        for (value, other_value) in self.values.iter_mut().zip(&other.values) {
            *value = self_weight * *value + other_weight * *other_value;
        }
        Ok(())
    }

    /// Cosine similarity in `[-1, 1]` for finite, equal-dimensional vectors.
    /// Zero vectors have zero similarity with every vector.
    pub fn similarity(&self, other: &Self) -> Result<f32, HdcError> {
        ensure_same_dimension(self.len(), other.len())?;
        if self.values.iter().any(|value| !value.is_finite())
            || other.values.iter().any(|value| !value.is_finite())
        {
            return Err(HdcError::NonFiniteValue);
        }

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

/// Sparse-HDC representation for memory-efficient visual fields.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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

    /// Accumulate a value at `index`, keeping at most one entry per index.
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

    /// Cosine similarity between this sparse vector and a dense vector.
    pub fn similarity_with_continuous(&self, continuous: &ContinuousHV) -> Result<f32, HdcError> {
        ensure_same_dimension(self.dim, continuous.len())?;
        if continuous.values.iter().any(|value| !value.is_finite())
            || self
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

fn ensure_same_dimension(left: usize, right: usize) -> Result<(), HdcError> {
    if left == right {
        Ok(())
    } else {
        Err(HdcError::DimensionMismatch { left, right })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_is_deterministic() {
        assert_eq!(ContinuousHV::random(32, 7), ContinuousHV::random(32, 7));
        assert_ne!(ContinuousHV::random(32, 7), ContinuousHV::random(32, 8));
    }

    #[test]
    fn normalize_produces_unit_length() {
        let mut vector = ContinuousHV {
            values: vec![3.0, 4.0],
        };
        vector.l2_normalize().unwrap();
        assert!((vector.l2_norm() - 1.0).abs() < 1.0e-6);
    }

    #[test]
    fn normalize_rejects_non_finite_values() {
        let mut vector = ContinuousHV {
            values: vec![f32::NAN],
        };
        assert_eq!(vector.l2_normalize(), Err(HdcError::NonFiniteValue));
    }

    #[test]
    fn lerp_rejects_dimension_mismatch_without_mutating() {
        let mut left = ContinuousHV {
            values: vec![1.0, 2.0],
        };
        let original = left.clone();
        let right = ContinuousHV { values: vec![3.0] };
        assert_eq!(
            left.lerp_in_place(&right, 0.5, 0.5),
            Err(HdcError::DimensionMismatch { left: 2, right: 1 })
        );
        assert_eq!(left, original);
    }

    #[test]
    fn similarity_is_cosine_normalized() {
        let left = ContinuousHV {
            values: vec![2.0, 0.0],
        };
        let right = ContinuousHV {
            values: vec![100.0, 0.0],
        };
        assert!((left.similarity(&right).unwrap() - 1.0).abs() < 1.0e-6);
    }

    #[test]
    fn zero_vector_similarity_is_zero() {
        let zero = ContinuousHV::zero(2);
        let other = ContinuousHV {
            values: vec![1.0, 0.0],
        };
        assert_eq!(zero.similarity(&other).unwrap(), 0.0);
    }

    #[test]
    fn sparse_add_bounds_checks_and_coalesces() {
        let mut sparse = SparseHV::new(4);
        sparse.add(2, 0.75).unwrap();
        sparse.add(2, 0.25).unwrap();
        assert_eq!(sparse.active_indices, vec![(2, 1.0)]);
        assert_eq!(
            sparse.add(4, 1.0),
            Err(HdcError::IndexOutOfBounds { index: 4, dim: 4 })
        );
    }

    #[test]
    fn sparse_similarity_is_normalized() {
        let mut sparse = SparseHV::new(3);
        sparse.add(1, 2.0).unwrap();
        let dense = ContinuousHV {
            values: vec![0.0, 9.0, 0.0],
        };
        assert!((sparse.similarity_with_continuous(&dense).unwrap() - 1.0).abs() < 1.0e-6);
    }
}
