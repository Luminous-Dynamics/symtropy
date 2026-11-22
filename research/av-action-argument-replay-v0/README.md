# AV Action Argument Replay v0

**Issue:** Symtropy #6897  
**Stacks on:** #6895 / draft PR #6895  
**Composes:** #6753 all-action EVSI, #6564 event-sourced executor, #6859 evidence certificates, #6861 minimum-FPS curve, #6895 evidence-gain trajectory, Symthaea #13179 candidate trace.

## Objective

Bind the exact acquisition arguments from the candidate-visible action forecast through executor acceptance and result delivery. An action ID such as crop-video is insufficient: two calls may request different intervals, FPS, ROI, resolution, preprocessing or source clock generations.

This crate is a protocol/validation layer. It does not schedule, select, capture, decode, or score actions.

## Rust crate

- src/lib.rs: typed, deny-unknown-fields manifest and lifecycle validator.
- Cargo.toml / Cargo.lock: pinned dependencies.
- schemas/av-action-argument-replay-v1.schema.json: cross-language wire contract.
- tools/av-evidence-gain-trajectory/evidence_gain_ledger.py and its tests: stacked #6895 receipt integration.

Run from repository root:

    cargo fmt --manifest-path research/av-action-argument-replay-v0/Cargo.toml -- --check
    cargo test --manifest-path research/av-action-argument-replay-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/av-action-argument-replay-v0/Cargo.toml --all-targets --locked -- -D warnings

## Canonical manifest identity

The action manifest digest is computed over compact UTF-8 JSON after conversion to a sorted-key JSON value. Schema v1 deliberately avoids floating-point values in the manifest:
- normalized ROI values use integer basis points (0..=10,000);
- source and frame timestamps use integer nanoseconds;
- fixed FPS uses an integer numerator/denominator;
- audio windows use integer sample indices and sample rate;
- transforms and source artifacts use SHA-256 digests.

The digest changes when source ID/content, clock domain/generation, time window, FPS/timestamp schedule, ROI, resolution, crop policy, preprocessing, audio sample range/rate/format, or feedback request type changes.

The all-action parameter-catalogue digest sorts action IDs and manifest digests and binds the selected action, its manifest digest, episode/turn identity and transcript head. The #6753 EVSI values are not re-scored here; the linked #6753 scorecard remains their independent authority. The explicit __stop__ forecast must be present with zero gross EVSI, but it has no sensor manifest and causes no executor request.

## Action parameter types

### Video crop / frame acquisition

Binds:
- source media content digest, source identity and clock generation;
- requested source-time interval;
- fixed-rate schedule using rational FPS and frame count, or an explicit timestamp list;
- normalized ROI in integer basis points or pixel-space ROI with source dimensions;
- output resolution;
- crop/tiling policy ID and digest;
- preprocessing/resize transform digest;
- timestamp tolerance.

Delivered frames must have unique observation refs, monotonic frame indexes/timestamps, nonzero bytes, and must match the requested schedule within the declared tolerance. A complete delivery must contain every requested frame. A partial delivery must retain a non-empty strict subset. The executor must attest the actual ROI, resolution, crop-policy digest and preprocessing digest; these cannot silently differ from the forecasted request.

### Audio sample window

Binds:
- start/end sample indices (end exclusive);
- sample rate, channels, PCM format;
- chunk size and overlap;
- filtering/resampling transform digest.

The sample range must map to the requested source-time interval within one sample period. Delivered audio must preserve the declared sample rate, channel count and sample format. A partial result must be a strict subrange; a complete result must match the full request. The result receipt also carries the executor-attested digest of the transform actually applied; it must match the requested transform digest. A transform receipt without delivered audio is invalid.

#### Raw-audio versus HDC-derived representation boundary

The audio-window receipt identifies the source PCM sample range. It is not an HDC embedding and must not be replaced by one.

Symthaea's acoustic HDC work is a **derived perceptual representation**, not a waveform codec: HDC vectors can support similarity, event/prosody features and cross-modal binding, but do not by themselves provide a decodable reconstruction of the source waveform. Therefore:

- preserve the source audio (or a separately qualified conventional codec representation) as the fidelity/transport record;
- identify every HDC-derived artifact separately, binding it to the exact source sample interval and to the encoder schema/version, frontend configuration, basis/seed identity, dimensions, numeric or quantized format, and artifact digest;
- keep transform identity and source provenance separate from the semantic embedding; a cryptographic digest establishes identity, not perceptual similarity;
- never count a delivered HDC vector as an audio sample range, and never infer lossless or bitrate compression from a smaller embedding without a defined decoder and reconstruction test.

This boundary allows HDC to be evaluated as a compact cognitive input while preserving replayable access to the underlying sound. It composes with Symthaea #10910/#10912 (live/offline HDC feature-space parity) and #10890 (acoustic-faithfulness evaluation); it does not create a second audio encoder or ASR path.

### External feedback request

Binds the expected feedback source role, feedback kind and earliest delivery tick. The feedback response has its own ID, source, kind, delivery tick and content digest. It is not a sensor observation and cannot be cited as audio/video evidence. This remains aligned with #6895's external-feedback event plane.

## Lifecycle

1. Every eligible acquisition in the all-action forecast has one complete action manifest and digest.
2. The forecast includes one #6753 EVSI row for each eligible action plus __stop__. The selected action and selected manifest digest are committed before outcome access.
3. The executor acceptance receipt binds episode, turn, request ID, action ID, manifest digest, parent transcript head, acceptance tick, expected arrival and decision deadline.
4. The action result retains those exact identities and digest, its result tick/status, actual delivered frames/audio/feedback, actual resource digest and event digest.
5. validate_action_lifecycle rejects any mismatch between forecast, acceptance and result.
6. The #6895 trajectory ledger carries the selected action manifest digest and request ID in the selected-action receipt; the next turn's action-result event must repeat that exact request ID and digest. The action-argument PR also runs the #6895 Python trajectory tests on its exact head.

The reserved stop action is present in the forecast but has no manifest, no request acceptance and no action-result payload.

## Failure and deadline semantics

- Failed, unavailable and cancelled actions retain the original parameter digest and result receipt but cannot mint sensor observations or feedback events.
- Partial results preserve exactly the delivered subset/range.
- A post-deadline payload must be marked late; it remains auditable but is not equivalent to an on-time observation.
- A result cannot be marked complete if scheduled frames or the requested audio range are missing.
- Actual resource usage is represented by a separate digest. This crate does not claim that physical metrology is correct.

## Tests

The Rust unit suite covers:
- same request → same digest; changed ROI/FPS/clock → changed digest;
- exact rational fixed-rate schedule and explicit timestamp validation;
- normalized/pixel ROI boundary checks;
- audio sample/time alignment and chunk-overlap checks;
- explicit stop plus complete action forecast coverage;
- selected manifest mismatch;
- request/result digest continuity;
- actual delivered frame timestamp/index/schedule and crop transform checks;
- audio delivery format/range;
- late-vs-on-time result status;
- failed action receipts with no fabricated observation;
- feedback request kept separate from sensor observations.

The stacked Python integration suite additionally verifies:
- every eligible #6753 action carries a manifest digest;
- selected action digest matches its catalogue entry and the linked scorecard receipt;
- request ID + parameter digest flow into the next turn's action-result receipt;
- stop has no request ID or manifest digest;
- a changed request or result manifest digest fails closed.

These are deterministic protocol checks, not proof that hardware executed an action correctly. The executor must attest actual capture and delivery; independent instrumentation must qualify resource readings.

## External comparators

- Open-o3-Video-v2 requests iterative temporal crops and object/box grounding; its own V-STaR mAM/mLGM must be reproduced under its native protocol, not compared directly with this manifest validator: https://arxiv.org/abs/2610.17656 and https://github.com/Multimedia-Semantic-Analytics-Lab/Open-o3-Video-v2
- VideoACE (18 Nov 2026) separates single-pass, autonomous inspection and user-feedback conditions; action arguments and feedback provenance must remain distinct: https://arxiv.org/abs/2611.24184
- FPS-Bench measures high-frame-rate question requirements; the actual delivered frame schedule—not nominal FPS—is needed for valid comparisons: https://openaccess.thecvf.com/content/CVPR2026/html/Choudhury_FPS-Bench_A_Benchmark_for_High_Frame-Rate_Video_Understanding_CVPR_2026_paper.html

## Claim boundary

A parameter digest proves the identity of the declared request. It does not prove the executor physically performed that request. The result receipt and delivered media must be independently attested. No model performance, energy saving, official benchmark score or SOTA improvement is claimed by schema/test completion.
