# FRESH-PROCESS-DRIVING-OBSERVATION-REPLAY-001

**Status:** executable deterministic non-learning fixture; no trained policy or vehicle-dynamics result  
**Issue:** https://github.com/Luminous-Dynamics/symtropy/issues/4618  
**Renderer:** https://github.com/Luminous-Dynamics/symtropy/pull/4616  
**Symthaea bridge:** https://github.com/Luminous-Dynamics/symthaea/pull/11480

## Purpose

This harness links immutable action-referenced world snapshots, offscreen RGB pixels, separate evaluator audits, vehicle-state records and the actual Symthaea camera-driving bridge.

It demonstrates one continuous pipeline and one process boundary:
1. render deterministic policy-visible RGB frames from immutable snapshots;
2. persist frame, policy receipt and evaluator audit as separate artifacts;
3. retain a missing-frame event and a delayed-frame disposition;
4. build the versioned Symthaea episode request using the renderer's exact policy receipt and matching vehicle-state record;
5. invoke the real `camera-driving-bridge` binary to process the sequence through one persistent Vision Manifold instance;
6. verify every bridge receipt points back to the exact upstream observation, action and frame;
7. verify each binary fused-HDC artifact against the receipt digest;
8. invoke the Symthaea bridge a second time in a new OS process and compare complete receipt JSON plus every HDC artifact byte-for-byte;
9. invoke the existing #4556 branch evidence auditor on a valid recomputed-actor campaign and on an intentionally stale actor-replay negative control;
10. spawn a fresh OS process to replay immutable snapshots and compare frame, observation, policy-receipt, world-audit, missing-event and disposition-ledger digests.

The alternate-action branch fixture changes action identity and actor/world snapshot; the branch auditor checks that changed actor input requires a recomputed response.

## Fixture contract

- 20 expected ticks, with one lead car and one pedestrian.
- Straight-to-curved road geometry, pinned camera calibration and renderer revision.
- Tick 7 is rendered with a delayed-but-not-stale delivery profile.
- Tick 11 is an explicit camera-drop event and is not replaced by an image. The delivered stream therefore contains 19 frames. The missing event is included in the fresh-process ledger commitment and remains in the expected 20-tick denominator.
- Tick 12 has a synthetic alternate-action branch with a changed action receipt and modified actor poses. It tests branch/input identity and digest changes; it does not execute an actor policy or a causal traffic simulator.
- Action receipts are deterministic fixture records. They are SHA-256 integrity pointers, not authenticated signatures or evidence of an external actuator application.
- Vehicle-state records are explicitly declared fixture observations and carry the same monotonic clock origin as the camera capture timestamps.
- The bridge stays in frozen-learning mode. The harness invokes no `VehicleController` and no actuator.

This is an interface/replay demonstration, not a policy-training run or a driving-performance benchmark.

## Prerequisites

1. From the Symthaea repository, generate the non-secret fixture config files using its helper:

```sh
cargo run --locked \
  --manifest-path crates/bridges/symthaea-camera-driving-bridge/Cargo.toml \
  --example write_fixture_configs -- bridge-config.json vision-config.json
```

2. Build the actual Symthaea bridge binary on a Rust 1.96 toolchain:

```sh
cargo build --locked \
  --manifest-path crates/bridges/symthaea-camera-driving-bridge/Cargo.toml \
  --bin camera-driving-bridge
```

3. Fetch and build the existing recoverability auditor CLI from its PR branch (`research/recoverable-driving-evidence-rust-v0-1`) in a separate worktree:

```sh
git fetch origin research/recoverable-driving-evidence-rust-v0-1
git worktree add --detach ../symtropy-auditor FETCH_HEAD
cargo build --locked \
  --manifest-path ../symtropy-auditor/research/recoverable-driving-evidence-001/Cargo.toml \
  --bin recoverable-driving-evidence
```

This is the CLI adapter owned by [Symtropy #4556](https://github.com/Luminous-Dynamics/symtropy/pull/4556). The harness calls the auditor twice: once with a valid recomputed-actor branch and once with the stale-actor replay negative control, and requires the finding `ReplayedActorInputChanged` in the rejected report.

4. Use the generated `bridge-config.json`, `vision-config.json` and both built binary paths. The generator pins the fixture episode, scenario, agent and camera identities. The helper pins the fixture identity:
- `episode_id = fresh-process-driving-episode-v0-1`;
- `scenario_id = straight-to-curved-road-two-actors-v0-1`;
- `agent_id = ego`;
- `camera_id = front-rgb`.

It also pins 640×360 RGB geometry, an initial 33,333 µs frame interval, a 66,666 µs maximum gap (to cover the explicit missing tick), and the default 16,384D no-depth VisionConfig. Its seed phrase is public fixture data, not a production secret.

5. Create a dedicated empty output directory. The harness rejects non-empty directories to avoid stale artifacts.

## Run

From the Symtropy repository root:

```sh
cargo run --release --locked \
  --manifest-path research/fresh-process-driving-observation-replay-001/Cargo.toml -- \
  ./out/fresh-driving-episode \
  ./bridge-config.json \
  ./vision-config.json \
  ../symthaea/target/debug/camera-driving-bridge \
  ../symtropy-auditor/target/debug/recoverable-driving-evidence
```

The output directory must be empty or new. Reusing a non-empty directory is rejected to prevent stale frames or receipts being mistaken for the current run. The final argument is the #4556 branch-auditor executable.

## Artifacts

- `frames/`: delivered RGB8 pixel buffers.
- `policy-receipts/`: policy-visible renderer receipts only.
- `evaluator-audits/`: separate world snapshot / actor / road provenance.
- `missing-observation-events.json`: explicit tick-11 camera drop.
- `observation-dispositions.json`: fresh and delayed outcomes.
- `applied-actions.json`: fixture action command and receipt identities.
- `world-transition-records.json`: immutable snapshot-to-action links (not physical dynamics execution).
- `episode-request.json`: request consumed by Symthaea's bridge CLI.
- `symthaea-bridge/bridge-receipts.json`: first-process 19 fused-input receipts with a predecessor chain.
- `symthaea-bridge/policy-inputs/`: first-process 16,384D dimension-prefixed little-endian f32 vectors; each binary SHA-256 equals its receipt's `fused_policy_input_sha256`.
- `symthaea-bridge/fresh-process-replay/`: second independent bridge process receipts and HDC vectors; the harness requires receipt JSON and every HDC artifact byte to match the first process exactly.
- `symthaea-bridge/fresh-process-replay-comparison.json`: exact bridge receipt/vector replay result;
- `fresh-process-replay-comparison.json`: exact renderer frame, audit, missing-event and disposition-ledger replay result.
- `fresh-process-replay-input.json`, `fresh-process-replay-output.json`: serialized snapshots and OS-process replay results.
- `counterfactual-branch/` and `counterfactual-branch-record.json`: changed-action/changed-actor-pose branch fixture.
- `branch-evidence-auditor/`: valid campaign audit plus stale actor replay negative-control report; the episode report requires the first to pass and the second to be rejected for `ReplayedActorInputChanged`.
- `episode-report.json`: expected/observed counts and conformance result.

## Build and tests

```sh
cargo fmt --manifest-path research/fresh-process-driving-observation-replay-001/Cargo.toml -- --check
cargo test --locked --manifest-path research/fresh-process-driving-observation-replay-001/Cargo.toml
cargo clippy --locked --manifest-path research/fresh-process-driving-observation-replay-001/Cargo.toml --all-targets --all-features -- -D warnings
```

## Claim ceiling

A successful fixture establishes that the declared renderer, receipt contract and Symthaea bridge can be composed and replayed across operating-system process boundaries for this exact finite dataset, including deterministic reconstruction of the bridge's HDC vectors and explicit replay of the missing/delayed observation ledger. It does not establish a physically accurate simulator, live actor-policy response, camera semantic perception, learned visual driving policy, model superiority, real-road transfer, roadworthiness or vehicle safety.
