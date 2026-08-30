#![cfg(feature = "reality-ledger-adapter")]

use std::collections::BTreeSet;

use symthaea_bevy_brain::{
    InhabitedWorldEpisode, PresenceCapability, SymtropyRealityBinding,
    SYMTROPY_SCENE_STATE_DIGEST_ALGORITHM,
};
use symthaea_reality_ledger::{
    DeterminismClass, DigestAlgorithm, RealityLayer, TypedDigest, WorldDescriptor, WorldId,
    WorldKey, WorldLineageId, WorldOrigin, WorldParentRef, WorldRelation,
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

fn assert_passive_episode_integrity(episode: &InhabitedWorldEpisode) -> Result<(), String> {
    if episode.binding.committed_world.layer != RealityLayer::DigitalCommitted {
        return Err("root is not DigitalCommitted".into());
    }
    if episode.genesis.world != episode.binding.committed_world
        || episode.presence.world != episode.binding.committed_world
    {
        return Err("binding/genesis/presence root mismatch".into());
    }
    if !episode
        .genesis
        .initial_state_digest
        .same_typed_value(&episode.presence.entry_state_digest)
    {
        return Err("genesis/presence state mismatch".into());
    }
    episode.presence.validate().map_err(|error| error.to_string())?;
    if !episode.presence.is_open() {
        return Err("presence unexpectedly closed".into());
    }
    if episode.presence.authority_receipt_digest.is_some() {
        return Err("passive episode unexpectedly carries authority".into());
    }

    let actual: BTreeSet<_> = episode.presence.capabilities.iter().cloned().collect();
    let expected: BTreeSet<_> = [
        PresenceCapability::Observe,
        PresenceCapability::Enter,
        PresenceCapability::Fork,
        PresenceCapability::Propose,
    ]
    .into_iter()
    .collect();
    if actual != expected || actual.len() != episode.presence.capabilities.len() {
        return Err("passive capability surface mismatch".into());
    }

    episode.graph.verify().map_err(|error| error.to_string())?;
    let root_key = WorldKey::from(&episode.binding.committed_world);
    if episode.graph.get(&root_key) != Some(&episode.binding.committed_world) {
        return Err("committed root descriptor mismatch".into());
    }

    let mut ghosts = 0usize;
    for world in episode.graph.worlds() {
        if world == &episode.binding.committed_world {
            continue;
        }
        if world.layer != RealityLayer::Counterfactual {
            return Err("derived world is not Counterfactual".into());
        }
        let parent = world.parent.as_ref().ok_or("counterfactual missing parent")?;
        if parent.world_id != episode.binding.committed_world.world_id
            || parent.lineage_id != episode.binding.committed_world.lineage_id
            || parent.relation != WorldRelation::CounterfactualOf
            || world.generation_depth != episode.binding.committed_world.generation_depth + 1
        {
            return Err("counterfactual parent mismatch".into());
        }
        ghosts += 1;
    }
    if !matches!(ghosts, 0 | 3) {
        return Err(format!("partial ghost population: {ghosts}"));
    }

    episode.ledger.verify().map_err(|error| error.to_string())?;
    if episode.ledger.records().len() < 2 {
        return Err("missing episode preamble".into());
    }
    for record in episode.ledger.records() {
        let key = WorldKey::from(&record.world);
        if episode.graph.get(&key) != Some(&record.world) {
            return Err("ledger world descriptor not exactly registered".into());
        }
    }
    Ok(())
}

#[test]
fn scene_state_digest_truthfully_reports_fnv1a64() {
    let state = binding().scene_state_digest("0123456789abcdef").unwrap();
    assert_eq!(state.domain, "symtropy.scene-state.v1");
    assert_eq!(
        state.algorithm,
        DigestAlgorithm::Other(SYMTROPY_SCENE_STATE_DIGEST_ALGORITHM.into())
    );
    assert_eq!(SYMTROPY_SCENE_STATE_DIGEST_ALGORITHM, "fnv1a64");
}

#[test]
fn freshly_opened_episode_satisfies_passive_integrity_gate() {
    let episode = episode();
    assert_passive_episode_integrity(&episode).unwrap();
    assert_eq!(episode.graph.len(), 1);
    assert_eq!(episode.ledger.len(), 2);
}

#[test]
fn root_descriptor_spoof_fails_passive_integrity_gate() {
    let mut episode = episode();
    episode.binding.committed_world.creator_id = "spoofed".into();
    assert!(assert_passive_episode_integrity(&episode).is_err());
}

#[test]
fn authority_bearing_capability_cannot_hide_in_passive_episode() {
    let mut episode = episode();
    episode.presence.capabilities.push(PresenceCapability::Mutate);
    episode.presence.authority_receipt_digest = Some(digest("authority.v1"));
    assert!(assert_passive_episode_integrity(&episode).is_err());
}

#[test]
fn partial_ghost_population_fails_closed() {
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
    assert!(assert_passive_episode_integrity(&episode).is_err());
}
