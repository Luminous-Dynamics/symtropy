# AV Camera Producer Adapter v0

**Issue:** #6301  
**Camera producer:** #4595 / #4616  
**AV replay contract:** #6183 / #6201 (pinned source commit 311f65c)  
**Camera producer:** #4616 (pinned source commit 58fe24f)  
**Temporal eligibility:** #6253

## Purpose

This package adapts the qualified software camera's policy-visible RGB frame and receipt into the existing AV replay observation type. It preserves the complete camera policy receipt and explicit capture/render/delivery milestones as a sidecar.

It does not create a renderer, sensor model, clock transform, transport scheduler, memory engine or evaluator authority.

## Important time distinction

The camera producer supplies acquisition, render-completion and delivery timestamps in microseconds on its declared clock. The adapter converts each value exactly to nanoseconds using checked arithmetic and preserves the original receipt unchanged.

The AV replay availability tick is independently supplied by the scenario/replay owner. It is never inferred from camera microseconds.

The #4616 camera producer currently exposes one capture timestamp, not a physical exposure interval. The AV observation therefore uses a point capture interval. No exposure duration or physical camera calibration is invented.

## Output boundary

- The generic AV bundle contains the existing VideoFrameObservationV1 and TransportEventV1.
- CameraFrameEnvelopeV1 retains the complete policy-visible PolicyObservationReceipt, clock generation/time-scale identity and all three converted clock milestones.
- EvaluatorObservationAudit is not accepted by this package and is never serialized.
- The raw RGB bytes must already be persisted at the caller-supplied root-relative artifact path. The existing #6201 loader remains the authority for root-confined loading and exact artifact verification. The Cargo manifest pins both producer packages to immutable PR-head commits until those PRs are merged; do not replace these with floating branch references.

The camera receipt uses unkeyed SHA-256 commitments. The producer verifier checks internal consistency and byte agreement, not producer authentication or signature authority.

## Run

From this directory:

```sh
cargo fmt -- --check
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
```

## Claim ceiling

Passing qualifies only the synthetic software-camera-to-AV-artifact mapping and receipt preservation. It does not establish physical calibration, acoustic fidelity, cross-clock transform correctness, operating-system hidden-truth isolation, AV model accuracy or sim-to-real transfer.
