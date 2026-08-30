// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fail-closed integrity gates for bounded inhabited-world episodes.
//!
//! These checks deliberately sit above the individual Reality Ledger types.
//! They verify that a composed Symtropy episode still preserves the stronger
//! experiment-level contract: one committed root, either zero or exactly three
//! counterfactual children, authority-poor presence, graph/ledger agreement,
//! and an unbroken append-only ledger.

use std::collections::BTreeSet;

use symthaea_reality_ledger::{
    PresenceCapability, RealityLayer, RealityRecordKind, WorldKey, WorldRelation,
};

use crate::art_reality_episode::{InhabitedWorldEpisode, InhabitedWorldEpisodeReceipt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InhabitedEpisodeIntegrityReport {
    pub world_count: usize,
    pub counterfactual_world_count: usize,
    pub ledger_records: usize,
    pub ledger_head: String,
    pub presence_open: bool,
}

/// Verify an episode before empirical interpretation or closeout.
///
/// The gate accepts the two supported passive phases:
/// - immediately after opening: one committed root and no ghost children;
/// - after one valid four-ghost registration: one committed root plus exactly
///   three counterfactual children.
///
/// Any partial ghost population fails closed.
pub fn verify_open_inhabited_episode(
    episode: &InhabitedWorldEpisode,
) -> Result<InhabitedEpisodeIntegrityReport, InhabitedEpisodeIntegrityError> {
    if episode.binding.committed_world.layer != RealityLayer::DigitalCommitted {
        return Err(InhabitedEpisodeIntegrityError::RootNotDigitalCommitted);
    }
    if episode.genesis.world != episode.binding.committed_world
        || episode.presence.world != episode.binding.committed_world
    {
        return Err(InhabitedEpisodeIntegrityError::RootBindingMismatch);
    }
    if !episode
        .genesis
        .initial_state_digest
        .same_typed_value(&episode.presence.entry_state_digest)
    {
        return Err(InhabitedEpisodeIntegrityError::GenesisPresenceStateMismatch);
    }

    episode
        .presence
        .validate()
        .map_err(|error| InhabitedEpisodeIntegrityError::InvalidPresence(error.to_string()))?;
    if !episode.presence.is_open() {
        return Err(InhabitedEpisodeIntegrityError::PresenceAlreadyClosed);
    }
    verify_passive_capabilities(&episode.presence.capabilities)?;
    if episode.presence.authority_receipt_digest.is_some() {
        return Err(InhabitedEpisodeIntegrityError::UnexpectedAuthorityReceipt);
    }

    episode
        .graph
        .verify()
        .map_err(|error| InhabitedEpisodeIntegrityError::InvalidWorldGraph(error.to_string()))?;
    let root_key = WorldKey::from(&episode.binding.committed_world);
    let root = episode
        .graph
        .get(&root_key)
        .ok_or(InhabitedEpisodeIntegrityError::MissingCommittedRoot)?;
    if root != &episode.binding.committed_world {
        return Err(InhabitedEpisodeIntegrityError::RootBindingMismatch);
    }

    let mut counterfactual_world_count = 0usize;
    for world in episode.graph.worlds() {
        if world == &episode.binding.committed_world {
            continue;
        }
        if world.layer != RealityLayer::Counterfactual {
            return Err(InhabitedEpisodeIntegrityError::UnexpectedDerivedWorldLayer(
                world.layer,
            ));
        }
        let parent = world
            .parent
            .as_ref()
            .ok_or(InhabitedEpisodeIntegrityError::CounterfactualMissingParent)?;
        if parent.world_id != episode.binding.committed_world.world_id
            || parent.lineage_id != episode.binding.committed_world.lineage_id
            || parent.relation != WorldRelation::CounterfactualOf
            || world.generation_depth != episode.binding.committed_world.generation_depth + 1
        {
            return Err(InhabitedEpisodeIntegrityError::CounterfactualParentMismatch);
        }
        counterfactual_world_count += 1;
    }
    if !matches!(counterfactual_world_count, 0 | 3) {
        return Err(InhabitedEpisodeIntegrityError::PartialGhostPopulation(
            counterfactual_world_count,
        ));
    }

    let ledger_head = episode
        .ledger
        .verify()
        .map_err(|error| InhabitedEpisodeIntegrityError::InvalidLedger(error.to_string()))?;
    let records = episode.ledger.records();
    if records.len() < 2 {
        return Err(InhabitedEpisodeIntegrityError::MissingEpisodePreamble);
    }
    if records[0].kind != RealityRecordKind::Creation
        || records[0].world != episode.binding.committed_world
        || records[1].kind != RealityRecordKind::WorldTransition
        || records[1].world != episode.binding.committed_world
    {
        return Err(InhabitedEpisodeIntegrityError::InvalidEpisodePreamble);
    }

    for record in records {
        let key = WorldKey::from(&record.world);
        let registered = episode
            .graph
            .get(&key)
            .ok_or(InhabitedEpisodeIntegrityError::LedgerWorldNotRegistered)?;
        if registered != &record.world {
            return Err(InhabitedEpisodeIntegrityError::LedgerWorldDescriptorMismatch);
        }
    }

    Ok(InhabitedEpisodeIntegrityReport {
        world_count: episode.graph.len(),
        counterfactual_world_count,
        ledger_records: episode.ledger.len(),
        ledger_head,
        presence_open: true,
    })
}

/// Verify the compact receipt emitted after a full four-ghost inhabited episode.
///
/// This does not re-run the ledger because the closed receipt intentionally
/// carries only the verified final head. It instead checks closeout invariants
/// that must remain true in the serialized evidence boundary.
pub fn verify_closed_four_ghost_receipt(
    receipt: &InhabitedWorldEpisodeReceipt,
) -> Result<(), InhabitedEpisodeIntegrityError> {
    receipt
        .presence
        .validate()
        .map_err(|error| InhabitedEpisodeIntegrityError::InvalidPresence(error.to_string()))?;
    if receipt.presence.is_open() {
        return Err(InhabitedEpisodeIntegrityError::PresenceStillOpen);
    }
    verify_passive_capabilities(&receipt.presence.capabilities)?;
    if receipt.presence.authority_receipt_digest.is_some() {
        return Err(InhabitedEpisodeIntegrityError::UnexpectedAuthorityReceipt);
    }
    if receipt.world_count != 4 {
        return Err(InhabitedEpisodeIntegrityError::ClosedWorldCountMismatch(
            receipt.world_count,
        ));
    }
    // Genesis + entry + three ghost creations + exit is the minimum closed
    // passive episode even before any observation records are admitted.
    if receipt.ledger_records < 6 {
        return Err(InhabitedEpisodeIntegrityError::ClosedLedgerTooShort(
            receipt.ledger_records,
        ));
    }
    if receipt.final_ledger_head.trim().is_empty() {
        return Err(InhabitedEpisodeIntegrityError::MissingFinalLedgerHead);
    }
    Ok(())
}

fn verify_passive_capabilities(
    capabilities: &[PresenceCapability],
) -> Result<(), InhabitedEpisodeIntegrityError> {
    let actual: BTreeSet<_> = capabilities.iter().cloned().collect();
    let expected: BTreeSet<_> = [
        PresenceCapability::Observe,
        PresenceCapability::Enter,
        PresenceCapability::Fork,
        PresenceCapability::Propose,
    ]
    .into_iter()
    .collect();
    if actual != expected || actual.len() != capabilities.len() {
        return Err(InhabitedEpisodeIntegrityError::CapabilitySurfaceMismatch);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InhabitedEpisodeIntegrityError {
    #[error("inhabited root is not DigitalCommitted")]
    RootNotDigitalCommitted,
    #[error("genesis/presence/graph root does not exactly match the committed binding")]
    RootBindingMismatch,
    #[error("genesis initial state and presence entry state differ")]
    GenesisPresenceStateMismatch,
    #[error("presence contract is invalid: {0}")]
    InvalidPresence(String),
    #[error("integrity gate expected an open presence session")]
    PresenceAlreadyClosed,
    #[error("closed receipt still contains an open presence session")]
    PresenceStillOpen,
    #[error("passive inhabited episode capability surface differs from Observe/Enter/Fork/Propose")]
    CapabilitySurfaceMismatch,
    #[error("passive inhabited episode unexpectedly carries an authority receipt")]
    UnexpectedAuthorityReceipt,
    #[error("world graph is invalid: {0}")]
    InvalidWorldGraph(String),
    #[error("world graph does not contain the committed root")]
    MissingCommittedRoot,
    #[error("derived world has unsupported layer {0:?}")]
    UnexpectedDerivedWorldLayer(RealityLayer),
    #[error("counterfactual world has no parent")]
    CounterfactualMissingParent,
    #[error("counterfactual world does not have the exact committed root as CounterfactualOf parent")]
    CounterfactualParentMismatch,
    #[error("episode contains a partial ghost population: expected 0 or 3, got {0}")]
    PartialGhostPopulation(usize),
    #[error("reality ledger is invalid: {0}")]
    InvalidLedger(String),
    #[error("episode ledger is missing genesis/presence-entry preamble")]
    MissingEpisodePreamble,
    #[error("episode ledger genesis/presence-entry preamble is malformed")]
    InvalidEpisodePreamble,
    #[error("ledger record references a world not present in the episode graph")]
    LedgerWorldNotRegistered,
    #[error("ledger world descriptor differs from the graph descriptor with the same key")]
    LedgerWorldDescriptorMismatch,
    #[error("closed four-ghost receipt must contain exactly four worlds, got {0}")]
    ClosedWorldCountMismatch(usize),
    #[error("closed passive episode ledger is too short: {0} records")]
    ClosedLedgerTooShort(usize),
    #[error("closed episode receipt is missing its final ledger head")]
    MissingFinalLedgerHead,
}

#[cfg(test)]
mod tests {
    use super::*;
    use symthaea_reality_ledger::{
        DeterminismClass, DigestAlgorithm, TypedDigest, WorldDescriptor, WorldId,
        WorldLineageId, WorldOrigin, WorldParentRef,
    };

    use crate::{
        art_reality_adapter::SymtropyRealityBinding,
        art_reality_episode::InhabitedWorldEpisode,
    };

    fn digest(domain: &str) -> TypedDigest {
        TypedDigest::blake3(domain, domain.as_bytes()).unwrap()
    }

    fn binding() -> SymtropyRealityBinding {
        SymtropyRealityBinding::new(
            "studio",
            "studio-lineage",
            "symthaea",
            "symtropy",
            "symtropy.scene-state.v1",
            "symtropy.capture-artifact.v1",
            DigestAlgorithm::Blake3,
        )
        .unwrap()
    }

    fn episode() -> InhabitedWorldEpisode {
        InhabitedWorldEpisode::open(
            "episode",
            binding(),
            "symthaea",
            "camera-body",
            digest("sensors.v1"),
            digest("actions.v1"),
            digest("kernel.v1"),
            digest("physics.v1"),
            digest("assets.v1"),
            "scene-0",
            DeterminismClass::Deterministic,
            None,
            "studio-frame",
            10,
        )
        .unwrap()
    }

    #[test]
    fn freshly_opened_episode_passes_integrity_gate() {
        let report = verify_open_inhabited_episode(&episode()).unwrap();
        assert_eq!(report.world_count, 1);
        assert_eq!(report.counterfactual_world_count, 0);
        assert!(report.presence_open);
        assert_eq!(report.ledger_records, 2);
    }

    #[test]
    fn binding_descriptor_spoof_fails_closed() {
        let mut episode = episode();
        episode.binding.committed_world.creator_id = "spoofed".into();
        assert_eq!(
            verify_open_inhabited_episode(&episode),
            Err(InhabitedEpisodeIntegrityError::RootBindingMismatch)
        );
    }

    #[test]
    fn authority_bearing_capability_is_rejected_in_passive_episode() {
        let mut episode = episode();
        episode
            .presence
            .capabilities
            .push(PresenceCapability::Mutate);
        assert!(matches!(
            verify_open_inhabited_episode(&episode),
            Err(InhabitedEpisodeIntegrityError::InvalidPresence(_))
                | Err(InhabitedEpisodeIntegrityError::CapabilitySurfaceMismatch)
        ));
    }

    #[test]
    fn partial_counterfactual_population_fails_closed() {
        let mut episode = episode();
        let root = episode.binding.committed_world.clone();
        episode
            .graph
            .insert(WorldDescriptor {
                world_id: WorldId("ghost-a".into()),
                lineage_id: WorldLineageId("ghost-a-lineage".into()),
                layer: RealityLayer::Counterfactual,
                origin: WorldOrigin::CounterfactualBranch,
                parent: Some(WorldParentRef {
                    world_id: root.world_id.clone(),
                    lineage_id: root.lineage_id.clone(),
                    relation: WorldRelation::CounterfactualOf,
                }),
                generation_depth: root.generation_depth + 1,
                creator_id: "symtropy-four-ghost".into(),
            })
            .unwrap();
        assert_eq!(
            verify_open_inhabited_episode(&episode),
            Err(InhabitedEpisodeIntegrityError::PartialGhostPopulation(1))
        );
    }
}
