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
8. spawn a fresh OS process to replay the immutable snapshots and compare frame, observation, policy-receipt and world-audit digests;
9. produce an alternate-action branch fixture whose action receipt and actor/world snapshot differ from the base fixture.

## Fixture contract

- 20 expected ticks, with one lead car and one pedestrian.
- Straight-to-curved road geometry, pinned camera calibration and renderer revision.
- Tick 7 is rendered with a delayed-but-not-stale delivery profile.
- Tick 11 is an explicit camera-drop event and is not replaced by an image. The delivered stream therefore contains 19 frames.
- Tick 12 has a synthetic alternate-action branch with a changed action receipt and modified actor poses. It tests branch/input identity and digest changes; it does not execute an actor policy or a causal traffic simulator.
- Action receipts are deterministic fixture records. They are SHA-256 integrity pointers, not authenticated signatures or evidence of an external actuator application.
- Vehicle-state records are explicitly declared fixture observations and carry the same monotonic clock origin as the camera capture timestamps.
- The bridge stays in frozen-learning mode. The harness invokes no `VehicleController` and no actuator.

This is an interface/replay demonstration, not a policy-training run or a driving-performance benchmark.

## Prerequisites

1. Build Symthaea's bridge binary on a Rust 1.96 toolchain:

```sh
cargo build --locked \
  --manifest-path crates/bridges/symthaea-camera-driving-bridge/Cargo.toml \
  --bin camera-driving-bridge
```

2. Prepare:
   - a full serde-serialized `BridgeConfig` JSON matching the fixture identity, with `max_width >= 640`, `max_height >= 360`, `max_frame_bytes >= 691200`, `initial_dt_us = 33333`, and `maximum_frame_gap_us >= 66666`;
   - a full serde-serialized `VisionConfig` JSON with `hdc_dim = 16384` and `enable_depth = false`;
   - a dedicated empty output directory.

The bridge config identity must use:
- `episode_id = fresh-process-driving-episode-v0-1`;
- `agent_id = ego`;
- `camera_id = front-rgb`.

The bridge's seed phrase is used only to deterministically instantiate the encoder; do not place a real credential or password in the fixture file.

## Run

From the Symtropy repository root:

```sh
cargo run --release --locked \
  --manifest-path research/fresh-process-driving-observation-replay-001/Cargo.toml -- \
  ./out/fresh-driving-episode \
  ./bridge-config.json \
  ./vision-config.json \
  ../symthaea/target/debug/camera-driving-bridge
```

The output directory must be empty or new. Reusing a non-empty directory is rejected to prevent stale frames or receipts being mistaken for the current run.

## Artifacts

- `frames/`: delivered RGB8 pixel buffers.
- `policy-receipts/`: policy-visible renderer receipts only.
- `evaluator-audits/`: separate world snapshot / actor / road provenance.
- `missing-observation-events.json`: explicit tick-11 camera drop.
- `observation-dispositions.json`: fresh and delayed outcomes.
- `applied-actions.json`: fixture action command and receipt identities.
- `world-transition-records.json`: immutable snapshot-to-action links (not physical dynamics execution).
- `episode-request.json`: request consumed by Symthaea's bridge CLI.
- `symthaea-bridge/bridge-receipts.json`: 19 fused-input receipts with a predecessor chain.
- `symthaea-bridge/policy-inputs/`: 16,384D dimension-prefixed little-endian f32 vectors; each binary SHA-256 equals its receipt's `fused_policy_input_sha256`.
- `fresh-process-replay-input.json`, `fresh-process-replay-output.json`, `fresh-process-replay-comparison.json`: serialized snapshots and OS-process replay results.
- `counterfactual-branch/` and `counterfactual-branch-record.json`: changed-action/changed-actor-pose branch fixture.
- `episode-report.json`: expected/observed counts and conformance result.

## Build and tests

```sh
cargo fmt --manifest-path research/fresh-process-driving-observation-replay-001/Cargo.toml -- --check
cargo test --locked --manifest-path research/fresh-process-driving-observation-replay-001/Cargo.toml
cargo clippy --locked --manifest-path research/fresh-process-driving-observation-replay-001/Cargo.toml --all-targets --all-features -- -D warnings
```

## Claim ceiling

A successful fixture establishes that the declared renderer, receipt contract and Symthaea bridge can be composed and replayed across an operating-system process boundary for this exact finite dataset. It does not establish a physically accurate simulator, live actor-policy response, camera semantic perception, learned visual driving policy, model superiority, real-road transfer, roadworthiness or vehicle safety.
