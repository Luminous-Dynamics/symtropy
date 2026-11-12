# CAMERA-BOUNDARY-FAULT-INJECTION-001

**Status:** 14-case real-bridge boundary fault harness authored, including source-episode provenance gates, per-artifact HDC reconciliation and temporal-state non-advance evidence; exact-head Rust validation pending  
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
- the #4618 source report must attest to passing fresh-process renderer replay, fresh-process bridge replay and branch-auditor positive/negative controls;
- every source RGB frame is rehashed and compared to its renderer receipt; persisted policy-receipt JSON must exactly equal the corresponding receipt embedded in the episode request;
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

<badge>14 cases</badge> <badge color="success">1 clean positive control</badge> <badge color="danger">13 negative controls</badge>

| Case | Injected boundary mutation | Expected bridge result |
|---|---|---|
| clean-control | Unchanged renderer request, with expected calibration pinned | Accept; emit one receipt and HDC artifact per frame |
| stale-frame-reuse | Use the earlier RGB path under the later observation's receipt | `UpstreamFrameMismatch` |
| pixel-corruption | Flip one copied pixel byte, keep the original renderer receipt | `UpstreamFrameMismatch` |
| calibration-shift | Change calibration digest to a different valid SHA-256 and recompute the unkeyed receipt; keep the clean calibration pin in BridgeConfig | `UpstreamCalibrationMismatch` |
| capture-timestamp-mismatch | Change outer capture time without changing the upstream capture time | `UpstreamTimestampMismatch` |
| zero-sequence | Set both outer and upstream sequence to zero | `UpstreamSequenceMismatch` |
| receipt-commitment-tamper | Replace the receipt commitment with another valid-length SHA-256 | `UpstreamReceiptCommitmentMismatch` |
| stale-sequence-replay | Replay the entire previous accepted observation—same RGB frame, upstream receipt, action, sequence and timestamps—at the next position | `SequenceRegression` |
| capture-clock-regression | Move the later capture timestamp behind the previous capture timestamp and recompute the receipt | `TimestampRegression` |
| camera-identity-mismatch | Change the camera ID and recompute the receipt commitment | `UpstreamIdentityMismatch` |
| clock-order-violation | Make delivery precede render completion and recompute the receipt commitment | `UpstreamClockOrderInvalid` |
| sensor-clock-skew | Move vehicle-state time one microsecond beyond configured skew | `SensorClockSkewExceeded` |
| delivery-age-exceeded | Exceed maximum observation age while retaining valid ordering and recomputing the receipt | `UpstreamDeliveryAgeExceeded` |
| missing-action-receipt | Remove the required applied-action receipt/command/tick references | `UpstreamActionReferenceInvalid` |

The harness reads the clean renderer calibration digest and pins it into `BridgeConfig.expected_camera_calibration_sha256` before any fault mutation. The receipt adapter reproduces the renderer's unkeyed SHA-256 commitment for deliberate metadata changes. This establishes internal consistency only; it does not authenticate the renderer. The exact sequence replay and capture-clock regression remain separate cases: the former tests the persistent sequence guard, the latter tests monotonic capture time. Stale-frame reuse and pixel corruption both produce a frame-digest mismatch but alter different parts of the request. The bridge CLI now includes `frames_processed_before_rejection=N` in its error diagnostic. The harness expects 0 for first-observation faults and 1 for second-observation stale/reuse/regression controls, proving that the rejected observation itself did not advance the temporal model while preserving expected prior processing.


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

The campaign output directory must be new or empty. Each case writes its own copied `input/` request/frame workspace, receipt path and policy-input path. The harness launches the bridge exactly once per case; there are 14 fresh child processes in the declared campaign.

## Acceptance

The report is `all_cases_passed` only when:
- the clean control exits successfully;
- clean output contains exactly one receipt and one HDC artifact per observation in the source episode;
- every negative control exits non-zero and stderr contains its exact expected BridgeError variant;
- every negative control has no receipts-output file;
- every negative control has no policy-input artifacts (the directory may be absent or empty);
- every negative control reports `frames_processed_before_rejection` equal to the expected count (0 for first-observation faults; 1 for the cases whose second observation triggers the rejection), demonstrating that the rejected observation itself did not advance temporal state;
- exactly 14 independent bridge processes were spawned; every rejected observation reports the bridge frame count immediately before rejection, which must equal the number of preceding valid observations in that case.

The report schema is `schemas/fault-campaign-report-v1.schema.json`. It binds the exact #4618 `episode-report.json`, `episode-request.json` and bridge executable bytes by SHA-256.

## Tests and validation

Nineteen authored Rust tests cover:
- calibration pinning from the clean renderer receipt;
- source #4618 report pass flags, expected/observed denominator reconciliation, frame digest verification and persisted receipt equality;
- recomputed missing-action receipt commitment and synchronized skewed vehicle timestamp metadata;
- valid-but-different calibration mutation;
- delayed delivery versus invalid clock ordering;
- clock-skew versus exact prior-observation replay;
- pixel corruption that preserves the original frame and receipt digests;
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

A successful campaign establishes that the exact built Symthaea bridge binary rejects these 13 malformed/stale/corrupt/pin/clock/action-reference conditions before emitting policy representations, while accepting the clean control. The `frames_processed_before_rejection` diagnostic additionally checks that the rejected observation did not advance the Vision Manifold; earlier valid observations in replay/regression cases may have advanced it. It does not show that a trained driving policy detects a fault, selects a safe degraded policy, executes a safe stop, or recovers. Those require a separate controller and supervisor, closed-loop simulator experiments, physical fault fidelity, and later independent HIL/closed-course governance.
