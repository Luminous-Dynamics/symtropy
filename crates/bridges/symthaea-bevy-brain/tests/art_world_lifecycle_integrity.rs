#![cfg(feature = "reality-ledger-adapter")]

use symthaea_bevy_brain::{
    InhabitedWorldEpisode, SymtropyRealityBinding, archive_snapshot, open_lifecycle_timeline,
    plan_ephemeral_counterfactual_fork, plan_persisted_committed_fork,
    reopen_snapshot_presence, resume_snapshot, snapshot_closed_episode_from_bytes,
    suspend_snapshot,
};
use symthaea_reality_ledger::{
    DeterminismClass, DigestAlgorithm, RealityLayer, TypedDigest, WorldLifecycleState,
    WorldRelation,
};

fn d(domain: &str) -> TypedDigest {
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

fn closed() -> symthaea_bevy_brain::InhabitedWorldEpisodeReceipt {
    InhabitedWorldEpisode::open(
        "episode-a",
        binding(),
        "symthaea",
        "camera-body",
        d("sensors.v1"),
        d("actions.v1"),
        d("kernel.v1"),
        d("physics.v1"),
        d("assets.v1"),
        "1111111111111111",
        DeterminismClass::Deterministic,
        None,
        "studio-frame",
        10,
    )
    .unwrap()
    .close("2222222222222222", 20)
    .unwrap()
}

#[test]
fn snapshot_binds_actual_persisted_bytes_separately_from_semantic_state() {
    let receipt = closed();
    let a = snapshot_closed_episode_from_bytes("snap-a", &receipt, b"artifact-a").unwrap();
    let b = snapshot_closed_episode_from_bytes("snap-a", &receipt, b"artifact-b").unwrap();
    assert_eq!(
        a.state_digest.algorithm,
        DigestAlgorithm::Other("fnv1a64".into())
    );
    assert_eq!(a.host_artifact_digest.algorithm, DigestAlgorithm::Blake3);
    assert_ne!(a.host_artifact_digest, b.host_artifact_digest);
    assert_ne!(a.digest().unwrap(), b.digest().unwrap());
}

#[test]
fn resume_before_suspend_fails_closed() {
    let snapshot = snapshot_closed_episode_from_bytes("snap-a", &closed(), b"artifact").unwrap();
    let mut timeline = open_lifecycle_timeline(&snapshot).unwrap();
    assert!(resume_snapshot(
        &mut timeline,
        "resume-too-early",
        &snapshot,
        snapshot.state_digest.clone(),
        d("authority.v1"),
        Some(21),
    )
    .is_err());
    assert_eq!(timeline.current_state, WorldLifecycleState::Active);
}

#[test]
fn same_scene_value_in_wrong_domain_cannot_resume_world() {
    let snapshot = snapshot_closed_episode_from_bytes("snap-a", &closed(), b"artifact").unwrap();
    let mut timeline = open_lifecycle_timeline(&snapshot).unwrap();
    suspend_snapshot(&mut timeline, "suspend", &snapshot, d("authority.v1"), Some(20)).unwrap();
    let wrong = TypedDigest::new(
        "other.scene-state.v1",
        snapshot.state_digest.algorithm.clone(),
        snapshot.state_digest.value.clone(),
    )
    .unwrap();
    assert!(resume_snapshot(
        &mut timeline,
        "resume",
        &snapshot,
        wrong,
        d("authority.v1"),
        Some(21),
    )
    .is_err());
    assert_eq!(timeline.current_state, WorldLifecycleState::Suspended);
}

#[test]
fn suspended_and_archived_worlds_cannot_be_revisited() {
    let receipt = closed();
    let snapshot = snapshot_closed_episode_from_bytes("snap-a", &receipt, b"artifact").unwrap();
    let mut timeline = open_lifecycle_timeline(&snapshot).unwrap();
    suspend_snapshot(&mut timeline, "suspend", &snapshot, d("authority.v1"), Some(20)).unwrap();

    let attempt = reopen_snapshot_presence(
        &binding(),
        &timeline,
        &snapshot,
        &receipt.presence,
        "revisit-suspended",
        "presence-b",
        "symthaea",
        "camera-body",
        d("sensors.v1"),
        d("actions.v1"),
        21,
    );
    assert!(attempt.is_err());

    archive_snapshot(&mut timeline, "archive", &snapshot, d("authority.v1"), Some(20)).unwrap();
    assert_eq!(timeline.current_state, WorldLifecycleState::Archived);
    assert!(resume_snapshot(
        &mut timeline,
        "resume-after-archive",
        &snapshot,
        snapshot.state_digest.clone(),
        d("authority.v1"),
        Some(21),
    )
    .is_err());
}

#[test]
fn resumed_presence_is_distinct_but_state_continuous() {
    let receipt = closed();
    let snapshot = snapshot_closed_episode_from_bytes("snap-a", &receipt, b"artifact").unwrap();
    let mut timeline = open_lifecycle_timeline(&snapshot).unwrap();
    suspend_snapshot(&mut timeline, "suspend", &snapshot, d("authority.v1"), Some(20)).unwrap();
    resume_snapshot(
        &mut timeline,
        "resume",
        &snapshot,
        snapshot.state_digest.clone(),
        d("authority.v1"),
        Some(21),
    )
    .unwrap();
    let (resumed, revisit) = reopen_snapshot_presence(
        &binding(),
        &timeline,
        &snapshot,
        &receipt.presence,
        "revisit",
        "presence-b",
        "symthaea",
        "camera-body",
        d("sensors.v1"),
        d("actions.v1"),
        21,
    )
    .unwrap();
    assert_ne!(resumed.session_id, receipt.presence.session_id);
    assert!(resumed.entry_state_digest.same_typed_value(&snapshot.state_digest));
    assert_eq!(revisit.prior_session_id, receipt.presence.session_id);
    assert_eq!(revisit.resumed_session_id, resumed.session_id);
}

#[test]
fn fork_classes_remain_distinct() {
    let snapshot = snapshot_closed_episode_from_bytes("snap-a", &closed(), b"artifact").unwrap();
    let ghost = plan_ephemeral_counterfactual_fork(
        "ghost-fork",
        &snapshot,
        "ghost-a",
        "ghost-a-lineage",
        "symthaea",
        d("ghost-genesis.v1"),
    )
    .unwrap();
    assert_eq!(ghost.child_world.layer, RealityLayer::Counterfactual);
    assert_eq!(
        ghost.child_world.parent.as_ref().unwrap().relation,
        WorldRelation::CounterfactualOf
    );
    assert!(!ghost.persisted);

    let committed = plan_persisted_committed_fork(
        "committed-fork",
        &snapshot,
        "garden-copy",
        "garden-copy-lineage",
        "symthaea",
        d("garden-copy-genesis.v1"),
        d("persist-authority.v1"),
    )
    .unwrap();
    assert_eq!(committed.child_world.layer, RealityLayer::DigitalCommitted);
    assert_eq!(
        committed.child_world.parent.as_ref().unwrap().relation,
        WorldRelation::SpawnedFrom
    );
    assert!(committed.persisted);
    assert!(committed.persist_authority_receipt_digest.is_some());
}
