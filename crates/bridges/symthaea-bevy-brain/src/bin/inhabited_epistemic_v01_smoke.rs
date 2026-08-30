#[cfg(not(feature = "reality-ledger-adapter"))]
fn main() {
    eprintln!("inhabited_epistemic_v01_smoke requires --features reality-ledger-adapter");
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
    use std::collections::BTreeSet;

    use symthaea_bevy_brain::{InhabitedWorldEpisode, SymtropyRealityBinding};
    use symthaea_bevy_brain::art_reality_adapter::SYMTROPY_SCENE_STATE_DIGEST_ALGORITHM;
    use symthaea_reality_ledger::{
        DeterminismClass, DigestAlgorithm, PresenceCapability, TypedDigest,
    };

    fn digest(domain: &str) -> TypedDigest {
        TypedDigest::blake3(domain, domain.as_bytes()).expect("static digest domain is valid")
    }

    let binding = SymtropyRealityBinding::new(
        "smoke-studio",
        "smoke-studio-lineage",
        "symthaea",
        "symtropy",
        "symtropy.scene-state.v1",
        "symtropy.capture-artifact.v1",
        DigestAlgorithm::Blake3,
    )?;

    let scene = binding.scene_state_digest("0123456789abcdef")?;
    if scene.algorithm
        != DigestAlgorithm::Other(SYMTROPY_SCENE_STATE_DIGEST_ALGORITHM.into())
    {
        return Err("semantic scene state is not truthfully typed as fnv1a64".into());
    }

    let episode = InhabitedWorldEpisode::open(
        "smoke-episode",
        binding,
        "symthaea",
        "camera-body",
        digest("smoke.sensors.v1"),
        digest("smoke.actions.v1"),
        digest("smoke.kernel.v1"),
        digest("smoke.physics.v1"),
        digest("smoke.assets.v1"),
        "0123456789abcdef",
        DeterminismClass::Deterministic,
        None,
        "studio-frame",
        1,
    )?;

    let expected: BTreeSet<_> = [
        PresenceCapability::Observe,
        PresenceCapability::Enter,
        PresenceCapability::Fork,
        PresenceCapability::Propose,
    ]
    .into_iter()
    .collect();
    let actual: BTreeSet<_> = episode.presence.capabilities.iter().cloned().collect();
    if actual != expected || episode.presence.authority_receipt_digest.is_some() {
        return Err("passive presence capability/authority boundary changed".into());
    }
    episode.graph.verify()?;
    let preclose_head = episode.ledger.verify()?;
    if preclose_head.trim().is_empty() {
        return Err("pre-close ledger head is empty".into());
    }

    let receipt = episode.close("0123456789abcdef", 2)?;
    receipt.presence.validate()?;
    if receipt.presence.is_open() {
        return Err("presence remained open after close".into());
    }
    if receipt.world_count != 1 || receipt.ledger_records != 3 {
        return Err(format!(
            "unexpected root-only smoke counts: worlds={}, records={}",
            receipt.world_count, receipt.ledger_records
        )
        .into());
    }
    if receipt.final_ledger_head.trim().is_empty() {
        return Err("final ledger head is empty".into());
    }

    println!("PASS: inhabited epistemic v0.1 structural smoke");
    println!("worlds={}", receipt.world_count);
    println!("ledger_records={}", receipt.ledger_records);
    println!("final_ledger_head={}", receipt.final_ledger_head);
    println!("scene_state_algorithm={SYMTROPY_SCENE_STATE_DIGEST_ALGORITHM}");
    Ok(())
}
