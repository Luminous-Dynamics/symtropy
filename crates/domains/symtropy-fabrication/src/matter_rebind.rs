// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Reference algebra for comparing opaque Fabrication matter bindings with an
//! external matter authority's current allocation observation.
//!
//! This module deliberately does **not** mint physical authority. The public
//! values below are reference/test vocabulary: callers can construct them, so a
//! successful comparison is not proof that the observation came from a trusted
//! matter authority. A later authority adapter must own point-of-use lookup and
//! may wrap a successful comparison in a non-forgeable authority-bound type.

use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};
use symtropy_game_state::StableId;

use crate::{MatterBinding, MatterIntegrityError};

/// Schema for the dependency-light reference comparison contract.
pub const MATTER_REBIND_REFERENCE_SCHEMA_VERSION: u32 = 1;

/// Read-only allocation observation supplied to the reference evaluator.
///
/// Every digest remains opaque to Fabrication. Their meaning belongs to the
/// selected external authority/profile; this type only preserves exact bytes
/// for equality/currentness comparison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatterAllocationObservationV1 {
    pub authority_id: StableId,
    pub allocation_id: StableId,
    pub revision: u64,
    pub binding_digest: String,
    pub authority_state_digest: String,
    pub continuation_identity: Option<String>,
}

impl MatterAllocationObservationV1 {
    pub fn new(
        authority_id: StableId,
        allocation_id: StableId,
        revision: u64,
        binding_digest: impl Into<String>,
        authority_state_digest: impl Into<String>,
        continuation_identity: Option<String>,
    ) -> Result<Self, MatterRebindReferenceError> {
        let observation = Self {
            authority_id,
            allocation_id,
            revision,
            binding_digest: binding_digest.into(),
            authority_state_digest: authority_state_digest.into(),
            continuation_identity,
        };
        observation.validate()?;
        Ok(observation)
    }

    pub fn validate(&self) -> Result<(), MatterRebindReferenceError> {
        validate_opaque_digest("binding_digest", &self.binding_digest)?;
        validate_opaque_digest("authority_state_digest", &self.authority_state_digest)?;
        if let Some(identity) = &self.continuation_identity {
            validate_opaque_digest("continuation_identity", identity)?;
        }
        Ok(())
    }
}

/// Result of a read-only lookup attempted by an external matter resolver.
///
/// This is still caller-constructible reference vocabulary. It must not be used
/// as evidence that a real owning authority was consulted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum MatterAuthorityResolutionV1 {
    Found(MatterAllocationObservationV1),
    Missing {
        authority_id: StableId,
        allocation_id: StableId,
    },
    Unavailable {
        authority_id: StableId,
    },
    UnsupportedProfile {
        authority_id: StableId,
        profile_id: StableId,
    },
}

/// Exact reference-only comparison result.
///
/// `Current` means only that the supplied observation matches the supplied
/// binding under this algebra. It does not authenticate the observation and it
/// carries no geometry, mass, placement, installation, or permission claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatterRebindReferenceV1 {
    pub schema_version: u32,
    pub profile_id: StableId,
    pub authority_id: StableId,
    pub allocation_id: StableId,
    pub decision: MatterRebindReferenceDecisionV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum MatterRebindReferenceDecisionV1 {
    Current {
        revision: u64,
        binding_digest: String,
        authority_state_digest: String,
        continuation_identity: Option<String>,
    },
    StaleRevision {
        recorded_revision: u64,
        current_revision: u64,
        current_authority_state_digest: String,
        current_continuation_identity: Option<String>,
    },
    StaleDigest {
        revision: u64,
        recorded_binding_digest: String,
        current_binding_digest: String,
        current_authority_state_digest: String,
        current_continuation_identity: Option<String>,
    },
    Missing,
    Unavailable,
    UnsupportedProfile,
}

/// Compares a structurally valid Fabrication binding with a caller-supplied
/// external-authority resolution.
///
/// This function is intentionally pure and non-authoritative. Production code
/// must obtain `resolution` from a qualified owning-authority adapter and confer
/// stronger type-state only after point-of-use currentness succeeds.
pub fn evaluate_matter_rebind_reference(
    binding: &MatterBinding,
    profile_id: &StableId,
    resolution: &MatterAuthorityResolutionV1,
) -> Result<MatterRebindReferenceV1, MatterRebindReferenceError> {
    binding
        .validate_current()
        .map_err(MatterRebindReferenceError::InvalidBinding)?;

    let decision = match resolution {
        MatterAuthorityResolutionV1::Found(observation) => {
            observation.validate()?;
            ensure_subject_matches(
                binding,
                &observation.authority_id,
                Some(&observation.allocation_id),
            )?;

            if observation.revision != binding.revision {
                MatterRebindReferenceDecisionV1::StaleRevision {
                    recorded_revision: binding.revision,
                    current_revision: observation.revision,
                    current_authority_state_digest: observation.authority_state_digest.clone(),
                    current_continuation_identity: observation.continuation_identity.clone(),
                }
            } else if observation.binding_digest != binding.binding_digest {
                MatterRebindReferenceDecisionV1::StaleDigest {
                    revision: binding.revision,
                    recorded_binding_digest: binding.binding_digest.clone(),
                    current_binding_digest: observation.binding_digest.clone(),
                    current_authority_state_digest: observation.authority_state_digest.clone(),
                    current_continuation_identity: observation.continuation_identity.clone(),
                }
            } else {
                MatterRebindReferenceDecisionV1::Current {
                    revision: observation.revision,
                    binding_digest: observation.binding_digest.clone(),
                    authority_state_digest: observation.authority_state_digest.clone(),
                    continuation_identity: observation.continuation_identity.clone(),
                }
            }
        }
        MatterAuthorityResolutionV1::Missing {
            authority_id,
            allocation_id,
        } => {
            ensure_subject_matches(binding, authority_id, Some(allocation_id))?;
            MatterRebindReferenceDecisionV1::Missing
        }
        MatterAuthorityResolutionV1::Unavailable { authority_id } => {
            ensure_subject_matches(binding, authority_id, None)?;
            MatterRebindReferenceDecisionV1::Unavailable
        }
        MatterAuthorityResolutionV1::UnsupportedProfile {
            authority_id,
            profile_id: observed_profile,
        } => {
            ensure_subject_matches(binding, authority_id, None)?;
            if observed_profile != profile_id {
                return Err(MatterRebindReferenceError::ResolutionProfileMismatch {
                    requested: profile_id.clone(),
                    observed: observed_profile.clone(),
                });
            }
            MatterRebindReferenceDecisionV1::UnsupportedProfile
        }
    };

    Ok(MatterRebindReferenceV1 {
        schema_version: MATTER_REBIND_REFERENCE_SCHEMA_VERSION,
        profile_id: profile_id.clone(),
        authority_id: binding.authority_id.clone(),
        allocation_id: binding.allocation_id.clone(),
        decision,
    })
}

fn ensure_subject_matches(
    binding: &MatterBinding,
    authority_id: &StableId,
    allocation_id: Option<&StableId>,
) -> Result<(), MatterRebindReferenceError> {
    if authority_id != &binding.authority_id
        || allocation_id.is_some_and(|observed| observed != &binding.allocation_id)
    {
        return Err(MatterRebindReferenceError::ResolutionSubjectMismatch {
            expected_authority: binding.authority_id.clone(),
            expected_allocation: binding.allocation_id.clone(),
            observed_authority: authority_id.clone(),
            observed_allocation: allocation_id.cloned(),
        });
    }
    Ok(())
}

fn validate_opaque_digest(
    field: &'static str,
    value: &str,
) -> Result<(), MatterRebindReferenceError> {
    if value.is_empty() || value.len() > 256 {
        return Err(MatterRebindReferenceError::InvalidObservationDigest {
            field,
            length: value.len(),
        });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatterRebindReferenceError {
    InvalidBinding(MatterIntegrityError),
    InvalidObservationDigest {
        field: &'static str,
        length: usize,
    },
    ResolutionSubjectMismatch {
        expected_authority: StableId,
        expected_allocation: StableId,
        observed_authority: StableId,
        observed_allocation: Option<StableId>,
    },
    ResolutionProfileMismatch {
        requested: StableId,
        observed: StableId,
    },
}

impl fmt::Display for MatterRebindReferenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBinding(error) => write!(formatter, "invalid matter binding: {error}"),
            Self::InvalidObservationDigest { field, length } => write!(
                formatter,
                "matter rebind observation field {field} must contain 1..=256 bytes, got {length}"
            ),
            Self::ResolutionSubjectMismatch {
                expected_authority,
                expected_allocation,
                observed_authority,
                observed_allocation,
            } => write!(
                formatter,
                "matter rebind resolution subject mismatch: expected {expected_authority}/{expected_allocation}, observed {observed_authority}/{observed_allocation:?}"
            ),
            Self::ResolutionProfileMismatch {
                requested,
                observed,
            } => write!(
                formatter,
                "matter rebind resolution profile mismatch: requested {requested}, observed {observed}"
            ),
        }
    }
}

impl Error for MatterRebindReferenceError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn binding(revision: u64, digest: &str) -> MatterBinding {
        MatterBinding::new(
            id("matter:universal"),
            id("allocation:beam"),
            revision,
            digest,
        )
        .unwrap()
    }

    fn observation(revision: u64, digest: &str) -> MatterAllocationObservationV1 {
        MatterAllocationObservationV1::new(
            id("matter:universal"),
            id("allocation:beam"),
            revision,
            digest,
            format!("state:{revision}:{digest}"),
            Some(format!("continuation:{revision}")),
        )
        .unwrap()
    }

    fn profile() -> StableId {
        id("matter-rebind-profile:reference-v1")
    }

    #[test]
    fn exact_match_is_reference_current_without_claiming_authority() {
        let result = evaluate_matter_rebind_reference(
            &binding(7, "digest:beam:7"),
            &profile(),
            &MatterAuthorityResolutionV1::Found(observation(7, "digest:beam:7")),
        )
        .unwrap();

        assert!(matches!(
            result.decision,
            MatterRebindReferenceDecisionV1::Current { revision: 7, .. }
        ));
    }

    #[test]
    fn revision_drift_is_stale_even_when_digest_matches() {
        let result = evaluate_matter_rebind_reference(
            &binding(7, "digest:beam"),
            &profile(),
            &MatterAuthorityResolutionV1::Found(observation(8, "digest:beam")),
        )
        .unwrap();

        assert!(matches!(
            result.decision,
            MatterRebindReferenceDecisionV1::StaleRevision {
                recorded_revision: 7,
                current_revision: 8,
                ..
            }
        ));
    }

    #[test]
    fn same_revision_changed_digest_is_stale() {
        let result = evaluate_matter_rebind_reference(
            &binding(7, "digest:old"),
            &profile(),
            &MatterAuthorityResolutionV1::Found(observation(7, "digest:new")),
        )
        .unwrap();

        assert!(matches!(
            result.decision,
            MatterRebindReferenceDecisionV1::StaleDigest { revision: 7, .. }
        ));
    }

    #[test]
    fn missing_exact_subject_is_not_current() {
        let result = evaluate_matter_rebind_reference(
            &binding(7, "digest:beam"),
            &profile(),
            &MatterAuthorityResolutionV1::Missing {
                authority_id: id("matter:universal"),
                allocation_id: id("allocation:beam"),
            },
        )
        .unwrap();

        assert_eq!(result.decision, MatterRebindReferenceDecisionV1::Missing);
    }

    #[test]
    fn unavailable_authority_fails_closed() {
        let result = evaluate_matter_rebind_reference(
            &binding(7, "digest:beam"),
            &profile(),
            &MatterAuthorityResolutionV1::Unavailable {
                authority_id: id("matter:universal"),
            },
        )
        .unwrap();

        assert_eq!(
            result.decision,
            MatterRebindReferenceDecisionV1::Unavailable
        );
    }

    #[test]
    fn cross_authority_resolution_is_rejected() {
        let result = evaluate_matter_rebind_reference(
            &binding(7, "digest:beam"),
            &profile(),
            &MatterAuthorityResolutionV1::Found(
                MatterAllocationObservationV1::new(
                    id("matter:other"),
                    id("allocation:beam"),
                    7,
                    "digest:beam",
                    "state:other",
                    None,
                )
                .unwrap(),
            ),
        );

        assert!(matches!(
            result,
            Err(MatterRebindReferenceError::ResolutionSubjectMismatch { .. })
        ));
    }

    #[test]
    fn different_allocation_resolution_is_rejected() {
        let result = evaluate_matter_rebind_reference(
            &binding(7, "digest:beam"),
            &profile(),
            &MatterAuthorityResolutionV1::Missing {
                authority_id: id("matter:universal"),
                allocation_id: id("allocation:other"),
            },
        );

        assert!(matches!(
            result,
            Err(MatterRebindReferenceError::ResolutionSubjectMismatch { .. })
        ));
    }

    #[test]
    fn unsupported_profile_is_bound_to_the_requested_profile() {
        let profile = profile();
        let result = evaluate_matter_rebind_reference(
            &binding(7, "digest:beam"),
            &profile,
            &MatterAuthorityResolutionV1::UnsupportedProfile {
                authority_id: id("matter:universal"),
                profile_id: profile.clone(),
            },
        )
        .unwrap();

        assert_eq!(
            result.decision,
            MatterRebindReferenceDecisionV1::UnsupportedProfile
        );
    }

    #[test]
    fn malformed_binding_is_rejected_before_external_comparison() {
        let mut malformed = binding(7, "digest:beam");
        malformed.binding_digest.clear();

        let result = evaluate_matter_rebind_reference(
            &malformed,
            &profile(),
            &MatterAuthorityResolutionV1::Found(observation(7, "digest:beam")),
        );

        assert!(matches!(
            result,
            Err(MatterRebindReferenceError::InvalidBinding(_))
        ));
    }

    #[test]
    fn empty_authority_state_digest_is_rejected() {
        let result = MatterAllocationObservationV1::new(
            id("matter:universal"),
            id("allocation:beam"),
            7,
            "digest:beam",
            "",
            None,
        );

        assert!(matches!(
            result,
            Err(MatterRebindReferenceError::InvalidObservationDigest {
                field: "authority_state_digest",
                ..
            })
        ));
    }
}
