#[cfg(not(feature = "reality-ledger-adapter"))]
fn main() {
    eprintln!("inhabited_epistemic_v02_lifecycle_smoke requires --features reality-ledger-adapter");
    std::process::exit(2);
}

#[cfg(feature = "reality-ledger-adapter")]
fn main() {
    if let Err(error) = run() {
        eprintln!("FAIL: {error}");
        std::process::exit(1);
    }
}

#[cfg(feature = "reality-ledger-adapter")]
fn run() -> Result<(), Box<dyn std::error::Error>> {
    use symthaea_bevy_brain::{
        InhabitedWorldEpisode, SymtropyRealityBinding, open_lifecycle_timeline,
        plan_ephemeral_counterfactual_fork, reopen_snapshot_presence, resume_snapshot,
        snapshot_closed_episode_from_bytes, suspend_snapshot,
    };
    use symthaea_reality_ledger::{DeterminismClass, DigestAlgorithm, TypedDigest};

    fn d(domain: &str) -> TypedDigest {
        TypedDigest::blake3(domain, domain.as_bytes()).expect("static digest is valid")
    }

    let binding = SymtropyRealityBinding::new(
        "lifecycle-studio",
        "lifecycle-studio-lineage",
        "symthaea",
        "symtropy",
        "symtropy.scene-state.v1",
        "symtropy.capture-artifact.v1",
        DigestAlgorithm::Blake3,
    )?;

    let episode = InhabitedWorldEpisode::open(
        "lifecycle-episode-a",
        binding.clone(),
        "symthaea",
        "camera-body",
        d("smoke.sensors.v1"),
        d("smoke.actions.v1"),
        d("smoke.kernel.v1"),
        d("smoke.physics.v1"),
        d("smoke.assets.v1"),
        "1111111111111111",
        DeterminismClass::Deterministic,
        None,
        "studio-frame",
        10,
    )?;
    let closed = episode.close("2222222222222222", 20)?;

    let snapshot = snapshot_closed_episode_from_bytes(
        "snapshot-a",
        &closed,
        b"deterministic-persisted-world-fixture-v1",
    )?;
    if !snapshot
        .state_digest
        .same_typed_value(closed.presence.exit_state_digest.as_ref().unwrap())
    {
        return Err("snapshot state differs from prior presence exit".into());
    }

    let mut timeline = open_lifecycle_timeline(&snapshot)?;
    suspend_snapshot(
        &mut timeline,
        "suspend-a",
        &snapshot,
        d("smoke.lifecycle-authority.v1"),
        Some(20),
    )?;
    resume_snapshot(
        &mut timeline,
        "resume-a",
        &snapshot,
        snapshot.state_digest.clone(),
        d("smoke.lifecycle-authority.v1"),
        Some(21),
    )?;

    let (resumed, revisit) = reopen_snapshot_presence(
        &binding,
        &timeline,
        &snapshot,
        &closed.presence,
        "revisit-a",
        "lifecycle-presence-b",
        "symthaea",
        "camera-body",
        d("smoke.sensors.v1"),
        d("smoke.actions.v1"),
        21,
    )?;
    if !resumed
        .entry_state_digest
        .same_typed_value(&snapshot.state_digest)
    {
        return Err("revisit entry state differs from snapshot".into());
    }

    let fork = plan_ephemeral_counterfactual_fork(
        "fork-a",
        &snapshot,
        "lifecycle-studio::ghost-a",
        "lifecycle-studio-lineage::ghost-a",
        "symthaea",
        d("smoke.child-genesis.v1"),
    )?;
    if fork.persisted || fork.persist_authority_receipt_digest.is_some() {
        return Err("ephemeral counterfactual fork unexpectedly gained persistence authority".into());
    }

    let timeline_digest = timeline.digest(&snapshot)?;
    let revisit_digest = revisit.digest()?;

    println!("PASS: inhabited epistemic v0.2 lifecycle structural smoke");
    println!("snapshot_digest={}", snapshot.digest()?.value);
    println!("lifecycle_timeline_digest={}", timeline_digest.value);
    println!("revisit_digest={}", revisit_digest.value);
    println!("fork_child={}", fork.child_world.world_id.0);
    Ok(())
}
