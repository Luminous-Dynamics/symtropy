# Construction Space-Time Occupancy Core — v0.1

**Status:** synthetic, conservative AABB overlap oracle. Not a collision-avoidance system, safety controller, site clearance authority, lift plan or physical qualification.

Parent: [CONSTRUCTION-SPACETIME-OCCUPANCY-001 #4974](https://github.com/Luminous-Dynamics/symtropy/issues/4974).

## Purpose

The existing construction interop core owns task lifecycle and named-resource time reservations. This crate adds one narrow layer: conservative three-dimensional occupancy overlap over the time windows already declared by those reservations.

It asks whether two work envelopes, access regions, staging footprints or transit/swept outer bounds may overlap in the same site-frame volume at the same time.

## V0.1 semantics

- Exact floor, site epoch, design revision digest and target frame binding.
- Integer millimetre coordinates and fixed axis-aligned 3D outer bounds.
- Separate geometry-uncertainty and project-clearance margins.
- Half-open time windows and clock uncertainty taken from the existing #3644 `ReservationRequest`.
- Closed spatial-boundary policy: touching after expansion is a potential conflict.
- Idempotent repeated occupancy IDs; conflicting reuse is rejected.
- A non-mutating `conflicts_for` query returns the complete conservative conflict set for a candidate, so planners can compare multiple blockers before choosing a schedule repair.
- `insert` remains fail-closed and rejects admission if any conflict exists.
- A combined admission helper evaluates the #3644 resource reservation and spatial occupancy against cloned in-memory owners, committing both only when both accept.
- If spatial occupancy rejects, the tentative reservation is not committed; if the resource calendar rejects, the tentative occupancy is not committed.
- Typed stale-context, malformed-geometry, overflow, clock and task-binding failures.
- Conflict records include both occupancy/task IDs, occupancy classes, expanded overlap bounds, overlap time and clock domain.

## Local commands

```sh
cargo fmt --manifest-path research/construction-spacetime-occupancy-v0-1/Cargo.toml -- --check
cargo test --manifest-path research/construction-spacetime-occupancy-v0-1/Cargo.toml --locked
cargo clippy --manifest-path research/construction-spacetime-occupancy-v0-1/Cargo.toml --locked --all-targets -- -D warnings
cargo bench --manifest-path research/construction-spacetime-occupancy-v0-1/Cargo.toml --locked --bench scale
```

The scale harness reports insertion and full-conflict-query timings for 10, 100, 1,000 and 10,000 synthetic sparse occupancy records. These are measurements of this implementation and runner only; they are not deployment latency thresholds or field performance claims.

## Required caller contract

The caller must first use the existing #3644 `ReservationBook` to admit the corresponding time/resource reservation and pass the same `ReservationRequest` into this occupancy index. This crate checks its task, epoch, clock and interval fields but cannot prove a reservation was granted or cryptographically authenticate its issuer.

The geometry producer must supply a conservative outer bound in the exact frame. This crate does not parse IFC, transform coordinates, infer a frame, construct a crane/robot swept volume or validate that an outer bound contains the physical system.

## Claim boundary

A reported overlap is a potential conflict under the declared conservative bounds, not proof of physical collision or unsafe operation. No reported overlap is not proof of physical safety or clearance. Unknown, stale or unobserved geometry must be held/not evaluated by the upstream observation and authority owners; it is never treated as an empty envelope or a clear zone.

The combined admission helper is a single-process in-memory transaction pattern. It is not a distributed transaction, durable commit or multi-controller serialization protocol.

The v0.1 AABB model is a deterministic broad-phase research reference only. False positives are allowed and must remain visible. A later narrow-phase geometry implementation needs an independent reference oracle and independently measured/as-built envelopes before any physical claim.
