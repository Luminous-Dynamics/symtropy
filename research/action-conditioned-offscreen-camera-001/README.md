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
- Straight/curved road polylines and wireframe actor cuboids.
- Bounded 1280 × 720 RGB frames and bounded world snapshot records.
- SHA-256 implemented locally in the dependency-free crate; checked against standard empty-string and "abc" test vectors.

This renderer is a software interface/test backend. It is not photorealistic, CARLA-equivalent, physically calibrated or a validated camera sensor model. It does not implement vehicle dynamics.

## Causality and privilege separation

The input snapshot is immutable and carries a world tick plus the action receipt and applied-action tick. A non-initial observation must be linked to the action applied at that tick. The renderer never advances the world or infers a control command.

The output deliberately separates two records. The world clock ID and action receipt IDs are runtime-owned inputs, not compile-time test literals:

1. **Policy observation receipt:** frame digest, camera/calibration, renderer, acquisition/render/delivery clocks, episode/agent/tick, and preceding action identity.
2. **Evaluator audit record:** source world-snapshot digest, simulator revision, road IDs, actor IDs and actor policy revisions.

The evaluator audit record must never be passed as policy input. The pixels and policy receipt are the only visual observation surface from this crate.

Capture/acquisition, render completion and delivery timestamps are separate. Missing frames are explicit sequenced events; admitting a missing event advances the observation stream so a later duplicate cannot be relabelled as fresh. A delayed frame is delayed; a stale frame is stale; a missing frame requires an explicit missing-event record. Render completion must never replace capture time.

## Coordinate contract

- World positions and road/actor geometry are millimetres.
- Camera-space axes: X right, Y down, Z forward.
- The world-to-camera 3×3 matrix is signed Q16.16.
- The matrix must be near orthonormal and have positive determinant; malformed coordinate transforms fail closed.
- Focal lengths and principal point are integer milli-pixels.
- The projection is pinhole perspective.
- Clipping is bounded to the declared camera near/far planes and image capacity.

## Run

From this directory:

\`\`\`sh
cargo fmt -- --check
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
\`\`\`

The crate has no third-party dependencies and pins Rust 1.96.0.

## Claim ceiling

Passing establishes deterministic pixel rendering and observation/audit receipt construction from finite supplied snapshots under the exact coordinate, calibration and renderer contract. It does not establish visual semantic quality, physical rendering fidelity, a valid traffic simulation, camera-conditioned driving competence, CARLA/Bench2Drive compatibility, sim-to-real transfer or vehicle safety.
