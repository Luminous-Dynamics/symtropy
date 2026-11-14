// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Canonical identity for a contact between a body and a periodic image.
//!
//! This is a key primitive only; the physics world does not yet use it to
//! deduplicate impulses. The offset is defined as the image translation of B
//! relative to A before canonicalization.

use std::error::Error;
use std::fmt;

use crate::body::BodyHandle;
use symtropy_math::LatticeOffset;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeriodicContactKeyError {
    SameBody,
    OffsetNegationOverflow { axis: usize },
}

impl fmt::Display for PeriodicContactKeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SameBody => write!(f, "periodic contact key cannot reference the same body twice"),
            Self::OffsetNegationOverflow { axis } => {
                write!(f, "cannot reverse minimum i64 image offset on axis {axis}")
            }
        }
    }
}

impl Error for PeriodicContactKeyError {}

/// Contact identity invariant under swapping the input bodies and reversing
/// the relative lattice offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PeriodicContactKey<const D: usize> {
    pub body_a: BodyHandle,
    pub body_b: BodyHandle,
    /// Image translation of canonical body_b relative to canonical body_a.
    pub image_offset: LatticeOffset<D>,
}

impl<const D: usize> PeriodicContactKey<D> {
    pub fn canonical(
        body_a: BodyHandle,
        body_b: BodyHandle,
        image_offset_b_relative_to_a: LatticeOffset<D>,
    ) -> Result<Self, PeriodicContactKeyError> {
        if body_a == body_b {
            return Err(PeriodicContactKeyError::SameBody);
        }

        if body_a < body_b {
            Ok(Self {
                body_a,
                body_b,
                image_offset: image_offset_b_relative_to_a,
            })
        } else {
            let mut reversed = image_offset_b_relative_to_a.periods;
            for (axis, value) in reversed.iter_mut().enumerate() {
                *value = value
                    .checked_neg()
                    .ok_or(PeriodicContactKeyError::OffsetNegationOverflow { axis })?;
            }
            Ok(Self {
                body_a: body_b,
                body_b: body_a,
                image_offset: LatticeOffset::new(reversed),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PeriodicContactKey, PeriodicContactKeyError};
    use crate::body::BodyHandle;
    use symtropy_math::LatticeOffset;

    #[test]
    fn swapping_bodies_and_reversing_offset_keeps_identity() {
        let forward =
            PeriodicContactKey::canonical(BodyHandle(2), BodyHandle(7), LatticeOffset::new([1, -2]))
                .unwrap();
        let reverse =
            PeriodicContactKey::canonical(BodyHandle(7), BodyHandle(2), LatticeOffset::new([-1, 2]))
                .unwrap();
        assert_eq!(forward, reverse);
    }

    #[test]
    fn same_body_is_not_a_rigid_body_contact_key() {
        assert_eq!(
            PeriodicContactKey::canonical(BodyHandle(4), BodyHandle(4), LatticeOffset::zero()),
            Err(PeriodicContactKeyError::SameBody)
        );
    }

    #[test]
    fn offset_negation_overflow_fails_closed() {
        assert_eq!(
            PeriodicContactKey::canonical(
                BodyHandle(7),
                BodyHandle(2),
                LatticeOffset::new([i64::MIN, 0])
            ),
            Err(PeriodicContactKeyError::OffsetNegationOverflow { axis: 0 })
        );
    }
}
