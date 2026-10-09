// Copyright (c) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Request and response types flowing between Bevy systems and the tokio
//! background task.
//!
//! Milestone 1 covered `GetActiveProposals`. Milestone 2 adds proposal
//! submission, voting, and TEND balance queries, plus correlation IDs
//! (a `u64` minted per request and echoed in the matching response).

use bevy::prelude::*;

/// A zome call requested by a Bevy system.
///
/// Each variant carries the requesting [`Entity`] so the matching
/// [`MycelixResponse`] can be routed back. Use [`Entity::PLACEHOLDER`] for
/// requests that aren't associated with a specific entity.
#[derive(Debug, Clone)]
pub enum MycelixRequest {
    /// Fetch all currently active governance proposals.
    GetActiveProposals { requester: Entity },
    /// Submit a new proposal to the `proposals` coordinator zome.
    SubmitProposal {
        requester: Entity,
        proposal_id: String,
        title: String,
        description: String,
        author_did: String,
    },
    /// Cast a vote on a proposal.
    CastVote {
        requester: Entity,
        proposal_id: String,
        voter_did: String,
        approve: bool,
        rationale: String,
    },
    /// Query a member's TEND (time-exchange) balance.
    QueryTendBalance {
        requester: Entity,
        member_did: String,
    },
    /// Fetch a single proposal by its ID. Returns `ProposalFound`
    /// (with the full Record as JSON) or `ProposalNotFound` via the
    /// `found` field.
    GetProposal {
        requester: Entity,
        proposal_id: String,
    },
}

impl MycelixRequest {
    /// The entity that originated this request.
    pub fn requester(&self) -> Entity {
        match self {
            MycelixRequest::GetActiveProposals { requester }
            | MycelixRequest::SubmitProposal { requester, .. }
            | MycelixRequest::CastVote { requester, .. }
            | MycelixRequest::QueryTendBalance { requester, .. }
            | MycelixRequest::GetProposal { requester, .. } => *requester,
        }
    }
}

/// A mutating operation whose remote outcome may become ambiguous when IPC
/// fails after dispatch starts. This identity is descriptive; it is not a
/// target-side idempotency key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MycelixMutationKind {
    SubmitProposal { proposal_id: String },
    CastVote { proposal_id: String },
}

/// A zome call response, delivered as a Bevy [`Message`] (formerly `Event`
/// in pre-0.18 Bevy).
///
/// Each accepted request produces a response or a documented inbox-closed
/// delivery failure. `NotDispatched` means this adapter did not begin writing
/// the request; `IndeterminateMutation` means a mutation may have reached the
/// child but no authoritative response was accepted. Neither variant asserts
/// anything about Holochain commit or DHT publication.
#[derive(Debug, Clone, Message)]
pub enum MycelixResponse {
    /// Success response to [`MycelixRequest::GetActiveProposals`]. Proposals
    /// are returned as raw JSON values (typed wrappers land in M3).
    ActiveProposals {
        requester: Entity,
        proposals: Vec<serde_json::Value>,
    },
    /// Success response to [`MycelixRequest::SubmitProposal`]. Preserves both
    /// the command identity and the action hash returned by Holochain so
    /// concurrent submissions cannot be assigned to the wrong agent.
    ProposalSubmitted {
        requester: Entity,
        proposal_id: String,
        action_hash: String,
    },
    /// Success response to [`MycelixRequest::CastVote`].
    VoteCast {
        requester: Entity,
        proposal_id: String,
    },
    /// Success response to [`MycelixRequest::QueryTendBalance`]. The member
    /// identity is preserved to disambiguate concurrent balance queries.
    TendBalance {
        requester: Entity,
        member_did: String,
        balance: serde_json::Value,
    },
    /// Success response to [`MycelixRequest::GetProposal`]. `record` is
    /// `Some(raw Record JSON)` when the proposal exists on chain, `None`
    /// when `get_proposal` returned `None`.
    Proposal {
        requester: Entity,
        proposal_id: String,
        record: Option<serde_json::Value>,
    },
    /// Failure before dispatch began; the bridge did not write request bytes.
    NotDispatched { requester: Entity, reason: String },
    /// A mutation may have reached the child but no response was accepted.
    /// Never interpret this as a definitive rejection or retry with a new ID.
    IndeterminateMutation {
        requester: Entity,
        operation: MycelixMutationKind,
        reason: String,
    },
    /// A response received from the child reported an error or invalid data.
    /// This variant alone does not prove a failed mutation was rolled back.
    Error { requester: Entity, reason: String },
}

impl MycelixResponse {
    /// The entity that originated the request this response answers.
    pub fn requester(&self) -> Entity {
        match self {
            MycelixResponse::ActiveProposals { requester, .. }
            | MycelixResponse::ProposalSubmitted { requester, .. }
            | MycelixResponse::VoteCast { requester, .. }
            | MycelixResponse::TendBalance { requester, .. }
            | MycelixResponse::Proposal { requester, .. }
            | MycelixResponse::NotDispatched { requester, .. }
            | MycelixResponse::IndeterminateMutation { requester, .. }
            | MycelixResponse::Error { requester, .. } => *requester,
        }
    }
}
