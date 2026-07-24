// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Holographic spacetime physics field.

use crate::hdc::{ContinuousHV, HDC_DIM, HdcError};
use nalgebra::Vector3;
use serde::{Deserialize, Serialize};
use std::{fmt, path::Path};

/// Spatial resolution used when deriving a deterministic HDC seed.
pub const POSITION_QUANTUM: f64 = 1.0e-6;
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

#[derive(Debug)]
pub enum SpacetimeError {
    InvalidPosition,
    InvalidMass(f64),
    InvalidSlope(f64),
    PositionOutOfRange,
    InvalidFieldDimension { expected: usize, actual: usize },
    Hdc(HdcError),
    Io(std::io::Error),
    Serialization(serde_json::Error),
}

impl fmt::Display for SpacetimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPosition => write!(f, "position coordinates must be finite"),
            Self::InvalidMass(mass) => write!(f, "mass must be finite and positive, got {mass}"),
            Self::InvalidSlope(slope) => {
                write!(f, "field slope must be finite and positive, got {slope}")
            }
            Self::PositionOutOfRange => write!(f, "quantized position is outside i64 range"),
            Self::InvalidFieldDimension { expected, actual } => write!(
                f,
                "field dimension mismatch: expected {expected}, got {actual}"
            ),
            Self::Hdc(error) => error.fmt(f),
            Self::Io(error) => error.fmt(f),
            Self::Serialization(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for SpacetimeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Hdc(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::Serialization(error) => Some(error),
            _ => None,
        }
    }
}

impl From<HdcError> for SpacetimeError {
    fn from(value: HdcError) -> Self {
        Self::Hdc(value)
    }
}

impl From<std::io::Error> for SpacetimeError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for SpacetimeError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serialization(value)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpacetimeCrystalField {
    pub global_cosmic_vector: ContinuousHV,
    pub slope: f64,
}

impl Default for SpacetimeCrystalField {
    fn default() -> Self {
        Self {
            global_cosmic_vector: ContinuousHV::zero(HDC_DIM),
            slope: 1.618_033_988_75,
        }
    }
}

impl SpacetimeCrystalField {
    pub fn validate(&self) -> Result<(), SpacetimeError> {
        if !self.slope.is_finite() || self.slope <= 0.0 {
            return Err(SpacetimeError::InvalidSlope(self.slope));
        }
        if self.global_cosmic_vector.len() != HDC_DIM {
            return Err(SpacetimeError::InvalidFieldDimension {
                expected: HDC_DIM,
                actual: self.global_cosmic_vector.len(),
            });
        }
        if self
            .global_cosmic_vector
            .values
            .iter()
            .any(|value| !value.is_finite())
        {
            return Err(SpacetimeError::Hdc(HdcError::NonFiniteValue));
        }
        Ok(())
    }

    pub fn inject_mass(&mut self, position: Vector3<f64>, mass: f64) -> Result<(), SpacetimeError> {
        self.validate()?;
        validate_position(position)?;
        if !mass.is_finite() || mass <= 0.0 {
            return Err(SpacetimeError::InvalidMass(mass));
        }

        let seed = self.position_seed(position)?;
        let mut signature = ContinuousHV::random(HDC_DIM, seed);
        signature.l2_normalize()?;

        // Preserve the original astronomical log-mass scale while rejecting
        // values for which log10 is undefined. Sub-kilogram masses have zero
        // coupling at this scale; 1e30 reaches full coupling.
        let weight = (mass.log10() / 30.0).clamp(0.0, 1.0) as f32;
        self.global_cosmic_vector
            .lerp_in_place(&signature, 1.0 - weight, weight)?;
        self.global_cosmic_vector.l2_normalize()?;
        Ok(())
    }

    pub fn probe(&self, position: Vector3<f64>) -> Result<f32, SpacetimeError> {
        self.validate()?;
        validate_position(position)?;
        let seed = self.position_seed(position)?;
        let signature = ContinuousHV::random(HDC_DIM, seed);
        Ok(self.global_cosmic_vector.similarity(&signature)?)
    }

    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<(), SpacetimeError> {
        self.validate()?;
        let encoded = serde_json::to_string_pretty(self)?;
        std::fs::write(path, encoded)?;
        Ok(())
    }

    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, SpacetimeError> {
        let content = std::fs::read_to_string(path)?;
        let decoded: Self = serde_json::from_str(&content)?;
        decoded.validate()?;
        Ok(decoded)
    }

    fn position_seed(&self, position: Vector3<f64>) -> Result<u64, SpacetimeError> {
        let quantized = [
            quantize_coordinate(position.x)?,
            quantize_coordinate(position.y)?,
            quantize_coordinate(position.z)?,
        ];

        // A tiny explicit FNV-1a encoder is used instead of DefaultHasher so
        // persisted fields remain deterministic across Rust toolchains.
        let mut hash = FNV_OFFSET_BASIS;
        for byte in b"symtropy-spacetime-position-v1" {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        for coordinate in quantized {
            for byte in coordinate.to_le_bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(FNV_PRIME);
            }
        }
        for byte in self.slope.to_bits().to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        Ok(hash)
    }
}

fn validate_position(position: Vector3<f64>) -> Result<(), SpacetimeError> {
    if position.iter().all(|coordinate| coordinate.is_finite()) {
        Ok(())
    } else {
        Err(SpacetimeError::InvalidPosition)
    }
}

fn quantize_coordinate(coordinate: f64) -> Result<i64, SpacetimeError> {
    let quantized = (coordinate / POSITION_QUANTUM).round();
    if quantized < i64::MIN as f64 || quantized > i64::MAX as f64 {
        return Err(SpacetimeError::PositionOutOfRange);
    }
    Ok(quantized as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mirrored_positions_do_not_alias() {
        let field = SpacetimeCrystalField::default();
        let positive = field.position_seed(Vector3::new(1.0, 2.0, 3.0)).unwrap();
        let negative = field.position_seed(Vector3::new(-1.0, -2.0, -3.0)).unwrap();
        assert_ne!(positive, negative);
    }

    #[test]
    fn coordinate_order_does_not_alias() {
        let field = SpacetimeCrystalField::default();
        assert_ne!(
            field.position_seed(Vector3::new(1.0, 2.0, 3.0)).unwrap(),
            field.position_seed(Vector3::new(3.0, 2.0, 1.0)).unwrap()
        );
    }

    #[test]
    fn positions_inside_one_quantum_share_a_seed() {
        let field = SpacetimeCrystalField::default();
        assert_eq!(
            field.position_seed(Vector3::zeros()).unwrap(),
            field
                .position_seed(Vector3::new(POSITION_QUANTUM * 0.49, 0.0, 0.0))
                .unwrap()
        );
    }

    #[test]
    fn probe_of_empty_field_is_zero() {
        let field = SpacetimeCrystalField::default();
        assert_eq!(field.probe(Vector3::zeros()).unwrap(), 0.0);
    }

    #[test]
    fn full_weight_mass_resonates_at_injected_position() {
        let mut field = SpacetimeCrystalField::default();
        let position = Vector3::new(1.0, -2.0, 3.0);
        field.inject_mass(position, 1.0e30).unwrap();
        let resonance = field.probe(position).unwrap();
        assert!((resonance - 1.0).abs() < 1.0e-5);
    }

    #[test]
    fn invalid_mass_is_rejected_without_mutation() {
        let mut field = SpacetimeCrystalField::default();
        let original = field.clone();
        assert!(matches!(
            field.inject_mass(Vector3::zeros(), -1.0),
            Err(SpacetimeError::InvalidMass(value)) if value == -1.0
        ));
        assert_eq!(field, original);
    }

    #[test]
    fn invalid_position_is_rejected() {
        let field = SpacetimeCrystalField::default();
        assert!(matches!(
            field.probe(Vector3::new(f64::NAN, 0.0, 0.0)),
            Err(SpacetimeError::InvalidPosition)
        ));
    }

    #[test]
    fn malformed_serialized_dimension_is_rejected() {
        let field = SpacetimeCrystalField {
            global_cosmic_vector: ContinuousHV::zero(4),
            slope: 1.0,
        };
        assert!(matches!(
            field.validate(),
            Err(SpacetimeError::InvalidFieldDimension {
                expected: HDC_DIM,
                actual: 4
            })
        ));
    }

    #[test]
    fn serialization_round_trip_preserves_field() {
        let mut field = SpacetimeCrystalField::default();
        field
            .inject_mass(Vector3::new(4.0, 5.0, 6.0), 1.0e20)
            .unwrap();
        let json = serde_json::to_string(&field).unwrap();
        let decoded: SpacetimeCrystalField = serde_json::from_str(&json).unwrap();
        decoded.validate().unwrap();
        assert_eq!(field, decoded);
    }
}
