# ACTION-CONDITIONED-OFFSCREEN-CAMERA-001

**Status:** bounded deterministic software camera reference; no simulator dynamics, driver or trained policy  
**Issue:** https://github.com/Luminous-Dynamics/symtropy/issues/4595  
**Environment parent:** https://github.com/Luminous-Dynamics/symtropy/issues/4536  
**Symthaea input bridge:** https://github.com/Luminous-Dynamics/symthaea/pull/11480

## Purpose

This crate renders a policy-visible RGB camera frame from an immutable, action-linked world snapshot. It owns pixel rendering and observation/audit record boundaries only. Vehicle dynamics, actor-policy execution, scenario scheduling and safety assurance remain owned by their respective systems.

## Reference renderer

- Deterministic software RGB renderer, fixed 3D camera transform in signed Q16.16.
- Intrinsics use integer milli-pixels; world coordinates use millimetres.
- Integer perspective projection, integer line rasterization and an integer z-buffer.
- Straight/curved road polylines and wireframe actor cuboids (the fixture includes a lead vehicle and a pedestrian).
- Bounded 1280 × 720 RGB frames, 256 road markings, 8,192 points per marking, 32,768 total road points and 128 actors.
- Typed Rust records serialize through Serde; JSON Schema v1 files are included under `schemas/`.
- SHA-256 reference implementation is checked against empty-string, `abc` and a multi-block NIST known-answer vector.
- A 20-tick serialized episode fixture reloads each immutable snapshot and requires exact pixel, policy-receipt and evaluator-audit replay; this is serialization/reconstruction coverage, not yet a separate OS-process integration test.

This renderer is a software interface/test backend. It is not photorealistic, CARLA-equivalent, physically calibrated or a validated camera sensor model. It does not implement vehicle dynamics.

## Causality and privilege separation

The input snapshot is immutable and carries a world tick plus the action receipt and applied-action tick. A non-initial observation must be linked to the action applied at that tick. The renderer never advances the world or infers a control command.

The public API deliberately uses separate calls and types:

1. `SoftwareCameraRenderer::render()` returns only `RenderedObservation { pixels_rgb, policy_receipt }`. The policy receipt contains the frame digest, camera/calibration digest, renderer revision, acquisition/render/delivery clocks, episode/agent/tick and preceding action receipt identity. It does not contain scenario labels, simulator revision, road IDs, actor IDs or evaluator audit digests.
2. `SoftwareCameraRenderer::audit_snapshot()` separately returns `EvaluatorObservationAudit`, containing source world-snapshot digest, simulator revision, road IDs, actor IDs and actor-policy revisions.

The normal policy render result cannot accidentally carry the evaluator-audit struct. The caller must invoke the explicit evaluator path to obtain it. The world clock ID and action receipt IDs are runtime-owned inputs, not compile-time test literals.

Capture/acquisition, render completion and delivery timestamps are separate. Missing frames are explicit sequenced events; admitting a missing event advances the observation stream so a later duplicate cannot be relabelled as fresh. A delayed frame is delayed; a stale frame is stale; a missing frame requires an explicit missing-event record. Render completion must never replace capture time.

The action receipt SHA-256 in the input snapshot is validated for format and bound into observation identity. This renderer does not authenticate the upstream action record or verify a signature; the separate branch/episode evidence auditor must reconcile the pointer against the actual action receipt.

## Coordinate contract

- World positions and road/actor geometry are millimetres.
- Camera-space axes: X right, Y down, Z forward.
- The world-to-camera 3×3 matrix is signed Q16.16.
- The matrix must be near orthonormal and have positive determinant; malformed coordinate transforms fail closed.
- Focal lengths and principal point are integer milli-pixels.
- The projection is pinhole perspective.
- Clipping is bounded to the declared camera near/far planes and image capacity.

## Versioned schemas

- `schemas/policy-observation-receipt-v1.schema.json` — policy-visible frame/action/camera/clock receipt.
- `schemas/evaluator-observation-audit-v1.schema.json` — separate evaluator-only snapshot and actor/road audit.
- `schemas/missing-observation-event-v1.schema.json` — explicit missing-frame stream event.

Typed input, timing, policy-receipt, evaluator-audit and missing-event metadata records are Serde serializable. The raw RGB pixel buffer intentionally remains a binary artifact addressed by its SHA-256 digest; it is not expanded into a JSON array of byte values. The JSON schemas document field bounds and cross-field invariants that JSON Schema cannot express natively. SHA-256 commitments are unkeyed integrity digests, not signatures or authentication.

## Run

From this directory:

\`\`\`sh
cargo fmt -- --check
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
\`\`\`

The crate pins Rust 1.96.0. It uses `serde` for typed receipt serialization and `serde_json` in tests for JSON round-trip conformance.

## Claim ceiling

Passing establishes deterministic pixel rendering and observation/audit receipt construction from finite supplied snapshots under the exact coordinate, calibration and renderer contract. It does not establish visual semantic quality, physical rendering fidelity, a valid traffic simulation, camera-conditioned driving competence, CARLA/Bench2Drive compatibility, sim-to-real transfer or vehicle safety.
