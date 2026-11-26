# RRVR Replay Provenance Preflight v0.1

This isolated Rust crate provides an artifact-backed preflight for the existing three-attempt replay-fidelity auditor.

## What it verifies

- Raw source episode bytes match the source-episode digest in the replay manifest.
- Raw instruction and action-prefix bytes match the replay manifest digests.
- Simulator, robot-model, controller and rendering profile artifacts exist, hash correctly, and identify the expected profile kind.
- A frozen policy-roster artifact contains exactly the four unique policy IDs used by the 20 screening rows. This proves equality to a supplied frozen roster, not that the roster is canonical author data.
- The reset-receipt manifest is raw-byte hash bound and scope bound to the scenario, source episode and simulator/robot profiles. Its three selected attempt IDs and policy IDs match the screening ledger.
- The action-prefix receipt manifest is raw-byte hash bound and scope bound to the scenario, source episode, instruction, action prefix, reset receipt and simulator/robot/controller/rendering profiles. Its three selected attempt IDs and policy IDs match the screening ledger; action-step counts must be positive.
- Sidecar scenario identities and artifact digests match the exact replay manifest supplied to the CLI.

The existing replay auditor continues to verify the three independent replay attempts, state artifacts, pose errors, five image pairs per attempt and the arithmetic mean of five SSIM frame scores per attempt. The preflight requires those three selected attempt IDs to occur in the 20-row screening ledger and checks their policy assignments. The seed-42 selection receipt remains verified, but the exact upstream RNG stream is not regenerated until the authors' algorithm/library/version is pinned. The provenance preflight does not alter the SSIM/pose checks.

## What it does not verify

- It does not authenticate signatures or establish that an issuer is trusted. It validates raw artifact bytes and declared scope. The authoritative evidence adapter must perform cryptographic attestation verification.
- The policy roster is a frozen supplied roster. It does not prove the roster is the authors' canonical four-policy list.
- A seed-42 selection receipt is validated but the authors' RNG stream is not regenerated. Exact algorithm/library/version parity remains unproven.
- No RoboRecover archive reproduction, simulator, HIL or physical robot result is implied.

## Run

```bash
cargo fmt --manifest-path research/mobility-rrvr-replay-provenance-v0.1/Cargo.toml -- --check
cargo test --manifest-path research/mobility-rrvr-replay-provenance-v0.1/Cargo.toml --locked
cargo clippy --manifest-path research/mobility-rrvr-replay-provenance-v0.1/Cargo.toml --locked --all-targets -- -D warnings
cargo run --manifest-path research/mobility-rrvr-replay-provenance-v0.1/Cargo.toml --locked --bin rrvr_replay_provenance -- REPLAY_MANIFEST.json PROVENANCE_MANIFEST.json ARTIFACT_ROOT
```

This v0.1 sidecar interprets the legacy candidate-level `action_prefix_sha256` as the digest of a scenario-level action-prefix index, because the selected three trajectories require distinct action-prefix artifacts. If the original producer intended that legacy field to identify one raw action sequence instead, the source manifest needs a versioned schema migration to an explicitly named `action_prefix_index_sha256`; do not silently reinterpret it in a host reproduction. This is software/process-model qualification only. A passing preflight means the referenced raw artifacts and typed identities are internally bound; it does not establish upstream signature authenticity or physical truth.
