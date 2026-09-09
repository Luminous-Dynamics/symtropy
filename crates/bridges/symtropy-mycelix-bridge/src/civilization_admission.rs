// Copyright (c) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Narrow conversion from Mycelix bridge responses into provider-neutral
//! civilization admission evidence.
//!
//! A bridge response is not institutional truth. This adapter only emits an
//! [`ExternalOrganizationRecord`] when the response carries a durable provider
//! record identifier. The resulting record must still be observed and admitted
//! through `symtropy-player-org-core` policy before any owning civilization
//! domain may consume it.

use std::{error::Error, fmt};

use symtropy_game_state::StableId;
use symtropy_player_org_core::{ExternalOrganizationRecord, ExternalOrganizationRef};

use crate::MycelixResponse;

pub const MYCELIX_PROVIDER_NAMESPACE: &str = "mycelix-holochain";
pub const PROPOSAL_SUBMITTED_RECORD_NAMESPACE: &str = "governance.proposal-submitted";

/// Explicit context supplied by product identity/persistence policy.
///
/// Bevy `Entity` values from the transport response are intentionally not used as
/// durable civilization identities. Provider-native actors/subjects must be
/// supplied explicitly when known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MycelixAdmissionContext {
    pub record_id: StableId,
    pub organization: ExternalOrganizationRef,
    pub observed_tick: u64,
    pub source_event_id: StableId,
    pub external_actor_id: Option<String>,
    pub external_subject_id: Option<String>,
}

/// Convert one response into an immutable external record when provenance is
/// strong enough for later admission review.
pub fn external_record_from_response(
    response: &MycelixResponse,
    context: MycelixAdmissionContext,
) -> Result<ExternalOrganizationRecord, MycelixEvidenceError> {
    match response {
        MycelixResponse::ProposalSubmitted { action_hash, .. } => {
            if action_hash.trim().is_empty() {
                return Err(MycelixEvidenceError::EmptyProviderRecordId);
            }
            Ok(ExternalOrganizationRecord {
                id: context.record_id,
                organization: context.organization,
                record_namespace: PROPOSAL_SUBMITTED_RECORD_NAMESPACE.into(),
                provider_record_id: action_hash.clone(),
                external_actor_id: context.external_actor_id,
                external_subject_id: context.external_subject_id,
                observed_tick: context.observed_tick,
                source_event_id: context.source_event_id,
            })
        }
        MycelixResponse::ActiveProposals { .. } => Err(
            MycelixEvidenceError::InsufficientStableProvenance("active-proposals-query"),
        ),
        MycelixResponse::VoteCast { .. } => Err(
            MycelixEvidenceError::InsufficientStableProvenance("vote-cast-acknowledgement"),
        ),
        MycelixResponse::TendBalance { .. } => Err(
            MycelixEvidenceError::InsufficientStableProvenance("tend-balance-query"),
        ),
        MycelixResponse::Proposal { .. } => Err(
            MycelixEvidenceError::InsufficientStableProvenance("raw-proposal-query"),
        ),
        MycelixResponse::Error { .. } => Err(MycelixEvidenceError::ProviderErrorResponse),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MycelixEvidenceError {
    EmptyProviderRecordId,
    InsufficientStableProvenance(&'static str),
    ProviderErrorResponse,
}

impl fmt::Display for MycelixEvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProviderRecordId => {
                f.write_str("Mycelix response contains an empty durable provider record id")
            }
            Self::InsufficientStableProvenance(kind) => write!(
                f,
                "Mycelix {kind} response does not carry enough durable provenance for civilization admission"
            ),
            Self::ProviderErrorResponse => {
                f.write_str("Mycelix error responses cannot become civilization admission evidence")
            }
        }
    }
}

impl Error for MycelixEvidenceError {}

#[cfg(test)]
mod tests {
    use bevy::prelude::Entity;

    use super::*;

    fn id(value: &str) -> StableId {
        StableId::parse(value).expect("test id")
    }

    fn context() -> MycelixAdmissionContext {
        MycelixAdmissionContext {
            record_id: id("external:proposal:17"),
            organization: ExternalOrganizationRef {
                provider_id: id("provider:mycelix"),
                organization_id: "org:helix".into(),
            },
            observed_tick: 55,
            source_event_id: id("event:mycelix-proposal-observed"),
            external_actor_id: Some("did:mycelix:alice".into()),
            external_subject_id: Some("proposal:17".into()),
        }
    }

    #[test]
    fn proposal_submission_action_hash_becomes_external_record_only() {
        let response = MycelixResponse::ProposalSubmitted {
            requester: Entity::PLACEHOLDER,
            action_hash: "uhCkk-real-action-hash".into(),
        };
        let record = external_record_from_response(&response, context()).expect("record");
        assert_eq!(record.provider_record_id, "uhCkk-real-action-hash");
        assert_eq!(record.record_namespace, PROPOSAL_SUBMITTED_RECORD_NAMESPACE);
    }

    #[test]
    fn vote_acknowledgement_is_not_promoted_without_durable_receipt() {
        let response = MycelixResponse::VoteCast {
            requester: Entity::PLACEHOLDER,
            proposal_id: "proposal:17".into(),
        };
        assert!(matches!(
            external_record_from_response(&response, context()),
            Err(MycelixEvidenceError::InsufficientStableProvenance(
                "vote-cast-acknowledgement"
            ))
        ));
    }

    #[test]
    fn raw_query_result_is_not_silently_admitted() {
        let response = MycelixResponse::Proposal {
            requester: Entity::PLACEHOLDER,
            proposal_id: "proposal:17".into(),
            record: Some(serde_json::json!({"arbitrary": "raw"})),
        };
        assert!(external_record_from_response(&response, context()).is_err());
    }
}
