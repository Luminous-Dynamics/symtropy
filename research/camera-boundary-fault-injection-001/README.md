# CAMERA-BOUNDARY-FAULT-INJECTION-001

**Status:** nine-case real-bridge boundary fault harness authored; exact-head Rust validation pending  
**Issue:** https://github.com/Luminous-Dynamics/symtropy/issues/4794  
**Source episode owner:** #4618 — fresh-process renderer-to-Symthaea replay  
**Bridge owner:** https://github.com/Luminous-Dynamics/symthaea/pull/11480  
**Fault protocol:** #4657 — degradation and recovery envelope  
**Renderer:** #4595 / #4616

## Purpose

Inject faults into actual renderer-to-policy request artifacts and prove that the current Symthaea camera bridge rejects them before it publishes fused HDC policy vectors.

This is intentionally narrower than the #4657 degradation/recovery benchmark. It tests the *input integrity boundary*. It does not run a controller or claim that a learned policy degrades or recovers safely.

## Paradigm shift: mutate boundary evidence, not labels

Each event begins with a real clean observation emitted by the #4618 renderer. The harness then changes one boundary field or pixel buffer and sends the mutated episode to the real Symthaea `camera-driving-bridge` executable.

For each case:
- the source clean episode remains immutable;
- all frame inputs are copied into a case-specific workspace;
- the altered request is written independently;
- a separate operating-system process starts a new bridge and temporal Vision Manifold;
- the exact expected `BridgeError` is asserted from the child process result;
- a rejected case must produce neither the receipt JSON file nor any policy-input artifact.

The harness does not imitate the bridge's validation logic. It consumes the actual executable.

## Required source bundle

Generate a complete episode directory using Symtropy #4618. It must contain:
- `episode-request.json`;
- the relative RGB frame artifacts referenced by `observations[].pixels_file`.

The base request's camera/vehicle timestamps, renderer receipt, action reference, RGB frame digests and HDC/VisionConfig must first pass the nominal bridge contract. The #4618 source episode can be generated with its documented prerequisites: a built Symthaea bridge binary, the #4556 branch auditor binary, and generated BridgeConfig/VisionConfig.

## Fault campaign

<badge>9 cases</badge> <badge color="success">1 clean positive control</badge> <badge color="danger">8 negative controls</badge>

| Case | Injected boundary mutation | Expected bridge result |
|---|---|---|
| clean-control | Unchanged renderer request, with expected calibration pinned | Accept; emit one receipt and HDC artifact per frame |
| pixel-corruption | Flip one pixel byte, keep the original renderer receipt | `UpstreamFrameMismatch` |
| calibration-shift | Change calibration digest to a different valid SHA-256 while retaining the clean calibration pin in BridgeConfig | `UpstreamCalibrationMismatch` |
| stale-sequence-replay | Replay an earlier upstream sequence in a later outer observation | `UpstreamSequenceMismatch` |
| clock-order-violation | Delivery timestamp precedes render completion | `UpstreamClockOrderInvalid` |
| sensor-clock-skew | Vehicle-state timestamp exceeds configured skew | `SensorClockSkewExceeded` |
| delivery-age-exceeded | Delivered timestamp exceeds the configured maximum observation age | `UpstreamDeliveryAgeExceeded` |
| camera-identity-mismatch | Replace the pinned camera ID | `UpstreamIdentityMismatch` |
| missing-action-receipt | Remove the required applied-action receipt/command/tick references | `UpstreamActionReferenceInvalid` |

The harness reads the clean renderer calibration digest and pins it into `BridgeConfig.expected_camera_calibration_sha256` before any fault mutation. This makes the calibration shift a real pin mismatch rather than a malformed digest. All other source request fields remain unchanged except those required by the named mutation.

## Run

Build the Symthaea bridge from its current camera-input PR branch in a Symthaea working tree:

```sh
git fetch origin research/camera-conditioned-driving-input-v0-1
git worktree add --detach ../symthaea-camera-bridge FETCH_HEAD
cargo build --locked \
  --manifest-path ../symthaea-camera-bridge/crates/bridges/symthaea-camera-driving-bridge/Cargo.toml \
  --bin camera-driving-bridge
```

From the Symtropy repository, run the harness against an existing #4618 episode bundle:

```sh
cargo run --release --locked \
  --manifest-path research/camera-boundary-fault-injection-001/Cargo.toml -- \
  ./out/fresh-driving-episode \
  ../symthaea-camera-bridge/target/debug/camera-driving-bridge \
  ./out/camera-boundary-fault-campaign
```

The campaign output directory must be new or empty. Each case writes its own copied `input/` request/frame workspace, receipt path and policy-input path. The harness launches the bridge exactly once per case; there are nine fresh child processes in the declared campaign.

## Acceptance

The report is `all_cases_passed` only when:
- the clean control exits successfully;
- clean output contains exactly one receipt and one HDC artifact per observation in the source episode;
- every negative control exits non-zero and stderr contains its exact expected BridgeError variant;
- every negative control has no receipts-output file;
- every negative control has no policy-input artifacts (the directory may be absent or empty);
- exactly nine independent bridge processes were spawned.

The report schema is `schemas/fault-campaign-report-v1.schema.json`.

## Tests and validation

Six authored Rust tests cover:
- calibration pinning from the clean renderer receipt;
- valid-but-different calibration mutation;
- delayed delivery versus invalid clock ordering;
- clock-skew versus stale-sequence mutation separation;
- camera identity and missing action receipt mutations;
- unique expected BridgeError attribution for every negative control.

Check the crate on Rust 1.96.0:

```sh
cargo fmt --manifest-path research/camera-boundary-fault-injection-001/Cargo.toml -- --check
cargo test --locked --manifest-path research/camera-boundary-fault-injection-001/Cargo.toml
cargo clippy --locked --manifest-path research/camera-boundary-fault-injection-001/Cargo.toml --all-targets --all-features -- -D warnings
```

The Rust toolchain is not installed in the authoring environment, so no local compilation/test result is claimed. The exact-head GitHub Actions workflow is the compiler authority. Queued or skipped is not a pass.

## Claim ceiling

A successful campaign establishes that the exact built Symthaea bridge binary rejects these eight malformed/stale/corrupt/pin/clock/action-reference conditions before emitting policy representations, while accepting the clean control. It does not show that a trained driving policy detects a fault, selects a safe degraded policy, executes a safe stop, or recovers. Those require a separate controller and supervisor, closed-loop simulator experiments, physical fault fidelity, and later independent HIL/closed-course governance.
