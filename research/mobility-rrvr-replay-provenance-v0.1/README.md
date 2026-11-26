# RRVR Replay Provenance Preflight v0.1

This isolated Rust crate provides an artifact-backed preflight for the existing three-attempt replay-fidelity auditor.

## What it verifies

- The raw split-manifest artifact matches the replay manifest's split-manifest digest. Candidate cluster identities and hashed leakage keys are checked; the same platform-specific leakage key or trajectory cluster cannot cross the train/test boundary.
- The raw split-manifest artifact in the provenance sidecar is read and hash-checked against the replay manifest's `split_manifest_sha256`; campaign and data-provenance metadata are required.
- Source trajectory and trajectory-cluster identities must match between the replay manifest and provenance sidecar.
- Raw instruction and action-prefix-index bytes match the replay manifest digests. The three selected attempts each carry distinct raw action-prefix artifacts and step counts.
- Raw original-state and replay-state snapshot files for all three selected attempts are read, hashed and checked for scenario, attempt ID, simulator-profile binding, finite positions and matching coordinate frames. The reset receipts' initial-state digests must match the original-state artifact bytes.
- Simulator, robot-model, controller and rendering profile artifacts exist, hash correctly, and identify the expected profile kind.
- The seed-42 selection-record file itself is read from the artifact root, SHA-256 checked, parsed and cross-checked against seed, method ID and exact selected-attempt order. The enclosing spot-check manifest digest is recomputed over the same typed selection metadata and attempt/frame-pair manifest material as the original replay auditor; the upstream RNG stream is still not regenerated.
- Each selected attempt contains exactly five ordered frame-pair records (indices 0–4). The preflight reads and hashes both raw original and replay frame assets for all 30 pairs across the three attempts. Image decoding and the SSIM calculation remain the original replay auditor's responsibility.
- A frozen policy-roster artifact contains exactly the four unique policy IDs used by the 20 screening rows. This proves equality to a supplied frozen roster, not that the roster is canonical author data.
- The reset-receipt manifest is raw-byte hash bound and scope bound to the scenario, source episode and simulator/robot profiles. Its three selected attempt IDs and policy IDs match the screening ledger.
- The action-prefix receipt manifest is raw-byte hash bound and scope bound to the scenario, source episode, instruction, action-prefix index, reset receipt and simulator/robot/controller/rendering profiles. Its three selected attempt IDs and policy IDs match the screening ledger; raw action-prefix bytes, hashes and positive action-step counts must agree.
- Sidecar scenario, source-trajectory and trajectory-cluster identities match the exact replay manifest supplied to the CLI. Hashed leakage keys must be unique within each candidate, and no key or trajectory cluster may appear in both train and test for the same platform.

The existing replay auditor continues to verify pose-error thresholds, five image pairs per attempt and the arithmetic mean of five SSIM frame scores per attempt. The provenance preflight now independently verifies raw state artifacts and their typed scope before that image/pose score is considered. The preflight requires those three selected attempt IDs to occur in the 20-row screening ledger and checks their policy assignments. The seed-42 selection record bytes, selected IDs, enclosing spot-check manifest digest and per-attempt five-frame asset references are now artifact-bound; the exact upstream RNG stream is not regenerated until the authors' algorithm/library/version is pinned. The provenance preflight does not alter the SSIM/pose checks.

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
