// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Cross-manifest ancestry validation for world continuation.
//!
//! A continuation manifest can prove its own local shape, but ancestry claims
//! require the actual parent manifest. This module validates that relationship
//! without changing either manifest's canonical binary identity.

use std::{error::Error, fmt};

use crate::{ContinuationError, LifecycleMode, WorldContinuationManifest};

/// Validate one child manifest against the exact parent manifest it claims.
///
/// This is deliberately separate from `WorldContinuationManifest::validate()`:
/// self-validation can prove local structure, while ancestry validation requires
/// possession of the actual parent root.
pub fn validate_manifest_lineage(
    child: &WorldContinuationManifest,
    parent: &WorldContinuationManifest,
) -> Result<(), LineageError> {
    child.validate()?;
    parent.validate()?;

    if child.lifecycle_mode == LifecycleMode::Genesis {
        return Err(LineageError::GenesisHasNoParent);
    }

    let claimed_parent = child
        .parent_manifest
        .as_ref()
        .ok_or(LineageError::MissingParentManifest)?;
    let actual_parent = parent.digest()?;
    if !claimed_parent.same_typed_value(&actual_parent) {
        return Err(LineageError::ParentManifestDigestMismatch);
    }

    if child.at < parent.at {
        return Err(LineageError::ContinuationTimeRegression);
    }

    match child.lifecycle_mode {
        LifecycleMode::Genesis => unreachable!("genesis rejected above"),
        LifecycleMode::ContinueSameWorld => {
            if child.world_instance != parent.world_instance {
                return Err(LineageError::SameWorldIdentityMismatch);
            }

            let expected = parent
                .continuation_sequence
                .checked_add(1)
                .ok_or(LineageError::ContinuationSequenceOverflow)?;
            if child.continuation_sequence != expected {
                return Err(LineageError::ContinuationSequenceMismatch {
                    expected,
                    actual: child.continuation_sequence,
                });
            }
        }
        LifecycleMode::ForkNewWorld => {
            if child.world_instance == parent.world_instance {
                return Err(LineageError::ForkWorldIdentityMustDiffer);
            }
            // Local manifest validation already requires a fork sequence of zero.
        }
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineageError {
    Continuation(ContinuationError),
    GenesisHasNoParent,
    MissingParentManifest,
    ParentManifestDigestMismatch,
    SameWorldIdentityMismatch,
    ForkWorldIdentityMustDiffer,
    ContinuationSequenceOverflow,
    ContinuationSequenceMismatch { expected: u64, actual: u64 },
    ContinuationTimeRegression,
}

impl From<ContinuationError> for LineageError {
    fn from(value: ContinuationError) -> Self {
        Self::Continuation(value)
    }
}

impl fmt::Display for LineageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Continuation(error) => {
                write!(formatter, "continuation manifest validation failed: {error}")
            }
            Self::GenesisHasNoParent => {
                formatter.write_str("genesis manifest cannot be validated against a parent")
            }
            Self::MissingParentManifest => formatter.write_str(
                "non-genesis continuation manifest is missing its claimed parent digest",
            ),
            Self::ParentManifestDigestMismatch => formatter.write_str(
                "claimed parent manifest digest does not match the supplied parent manifest",
            ),
            Self::SameWorldIdentityMismatch => formatter.write_str(
                "same-world continuation must preserve the parent world instance identity",
            ),
            Self::ForkWorldIdentityMustDiffer => formatter.write_str(
                "forked world must mint a world instance identity distinct from its parent",
            ),
            Self::ContinuationSequenceOverflow => formatter.write_str(
                "parent continuation sequence cannot be incremented without overflow",
            ),
            Self::ContinuationSequenceMismatch { expected, actual } => write!(
                formatter,
                "same-world continuation sequence mismatch: expected {expected}, got {actual}",
            ),
            Self::ContinuationTimeRegression => formatter.write_str(
                "child continuation instant precedes the selected parent continuation instant",
            ),
        }
    }
}

impl Error for LineageError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Continuation(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DigestAlgorithm, ReferenceFrameId, SimInstant, TypedDigest32, WorldInstanceId,
    };

    fn d(domain: &str, bytes: &[u8]) -> TypedDigest32 {
        TypedDigest32::sha256(domain, 1, bytes).unwrap()
    }

    fn manifest(
        world: &str,
        sequence: u64,
        mode: LifecycleMode,
        parent: Option<TypedDigest32>,
        seconds: i64,
    ) -> WorldContinuationManifest {
        WorldContinuationManifest::new(
            WorldInstanceId::parse(world).unwrap(),
            sequence,
            mode,
            parent,
            SimInstant::new(seconds, 0).unwrap(),
            TypedDigest32::new(
                "symtropy.fixed-timebase.identity.v1",
                DigestAlgorithm::Sha256,
                1,
                [7; 32],
            )
            .unwrap(),
            ReferenceFrameId::parse("sol:earth:surface-fixed").unwrap(),
            d("symtropy.inactive-time-policy.v1", b"paused"),
            None,
            None,
            None,
            vec![],
            vec![],
        )
        .unwrap()
    }

    fn genesis(world: &str, seconds: i64) -> WorldContinuationManifest {
        manifest(world, 0, LifecycleMode::Genesis, None, seconds)
    }

    #[test]
    fn exact_same_world_parent_passes() {
        let parent = genesis("world:a", 10);
        let child = manifest(
            "world:a",
            1,
            LifecycleMode::ContinueSameWorld,
            Some(parent.digest().unwrap()),
            20,
        );
        assert_eq!(validate_manifest_lineage(&child, &parent), Ok(()));
    }

    #[test]
    fn wrong_parent_digest_fails() {
        let parent = genesis("world:a", 10);
        let child = manifest(
            "world:a",
            1,
            LifecycleMode::ContinueSameWorld,
            Some(TypedDigest32::new(
                "symtropy.world-continuation-manifest.identity.v1",
                DigestAlgorithm::Sha256,
                1,
                [9; 32],
            )
            .unwrap()),
            20,
        );
        assert_eq!(
            validate_manifest_lineage(&child, &parent),
            Err(LineageError::ParentManifestDigestMismatch)
        );
    }

    #[test]
    fn same_world_must_preserve_world_identity() {
        let parent = genesis("world:a", 10);
        let child = manifest(
            "world:b",
            1,
            LifecycleMode::ContinueSameWorld,
            Some(parent.digest().unwrap()),
            20,
        );
        assert_eq!(
            validate_manifest_lineage(&child, &parent),
            Err(LineageError::SameWorldIdentityMismatch)
        );
    }

    #[test]
    fn same_world_sequence_must_increment_exactly_once() {
        let parent = genesis("world:a", 10);
        let child = manifest(
            "world:a",
            2,
            LifecycleMode::ContinueSameWorld,
            Some(parent.digest().unwrap()),
            20,
        );
        assert_eq!(
            validate_manifest_lineage(&child, &parent),
            Err(LineageError::ContinuationSequenceMismatch {
                expected: 1,
                actual: 2,
            })
        );
    }

    #[test]
    fn continuation_time_must_not_regress() {
        let parent = genesis("world:a", 10);
        let child = manifest(
            "world:a",
            1,
            LifecycleMode::ContinueSameWorld,
            Some(parent.digest().unwrap()),
            9,
        );
        assert_eq!(
            validate_manifest_lineage(&child, &parent),
            Err(LineageError::ContinuationTimeRegression)
        );
    }

    #[test]
    fn fork_with_new_world_identity_passes() {
        let parent = genesis("world:a", 10);
        let child = manifest(
            "world:b",
            0,
            LifecycleMode::ForkNewWorld,
            Some(parent.digest().unwrap()),
            10,
        );
        assert_eq!(validate_manifest_lineage(&child, &parent), Ok(()));
    }

    #[test]
    fn fork_cannot_reuse_parent_world_identity() {
        let parent = genesis("world:a", 10);
        let child = manifest(
            "world:a",
            0,
            LifecycleMode::ForkNewWorld,
            Some(parent.digest().unwrap()),
            10,
        );
        assert_eq!(
            validate_manifest_lineage(&child, &parent),
            Err(LineageError::ForkWorldIdentityMustDiffer)
        );
    }

    #[test]
    fn genesis_cannot_be_validated_against_parent() {
        let parent = genesis("world:a", 10);
        let child = genesis("world:b", 20);
        assert_eq!(
            validate_manifest_lineage(&child, &parent),
            Err(LineageError::GenesisHasNoParent)
        );
    }
}
