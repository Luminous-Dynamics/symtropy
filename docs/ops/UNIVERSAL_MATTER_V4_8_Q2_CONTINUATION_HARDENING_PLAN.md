# Universal Matter v4.8 Q2 Continuation Hardening Plan

**Status:** implementation plan; execute only after exact retained v4.8 replay/Q1 evidence  
**Date:** 2026-09-01  
**Prerequisites:** Q0/Q1 replay lane from PR #74, authority audits from PR #80

## Goal

Produce a repaired descendant of the retained Universal Matter v4.8 authored tree that can truthfully satisfy **Q2 continuation/replay qualification**.

The Q2 series must preserve existing physical-state digest semantics while adding the continuation identities and lifecycle state required for exact deterministic resume/replay claims.

Do not squash Q2 repairs into the retained authored replay while qualification lineage is being established.

## Branching rule

### If pristine Q1 passes

Parent the Q2 branch directly from the exact Q1-qualified v4.8 replay commit.

### If pristine Q1 fails

1. preserve the exact retained replay as an explicitly unqualified commit;
2. land build/compatibility repairs as focused child commits;
3. obtain a cumulative Q1 build-qualified repair head;
4. parent Q2 from that exact head.

In either case, Q2 evidence must name the exact Q1 ancestor.

## Q2.0 — contract/test scaffolding

### Purpose

Land test helpers and type names without changing existing digest bytes.

### Changes

- introduce explicit native naming/documentation for physical-state versus continuation identity;
- add private/canonical helpers for hashing ordered `TerrainCellAddress` sets;
- add reusable test fixtures for equal physical state / unequal continuation state;
- add no behavior changes.

### Invariant

Every pre-existing `*StateDigest` test vector and snapshot round-trip remains unchanged.

## Q2.1 — Hydrology continuation identity

Tracking: #76 / #79.

### Add

Candidate type:

`HydrologyAuthorityContinuationDigest`

Canonical inputs:

1. domain separator/schema;
2. existing `HydrologyStateDigest`;
3. active-frontier count;
4. every active `TerrainCellAddress` in canonical BTree order.

### API

Prefer additive API such as:

- `HydrologyAuthority::state_digest()` or retain existing `digest()` semantics;
- `HydrologyAuthority::continuation_digest()`.

Avoid silently changing existing `digest()` bytes.

### Tests

- water equal / active equal => continuation equal;
- water equal / active unequal => state digest equal, continuation digest unequal;
- activation insertion order does not alter continuation digest;
- checkpoint round-trip preserves continuation digest;
- controlled bounded step fixture demonstrates active-frontier behavioral relevance;
- equal continuation + equal Matter/dt/max_cells => equal report, state digest, continuation digest.

## Q2.2 — Thermal continuation identity

Tracking: #79.

### Add

`ThermalAuthorityContinuationDigest` binding:

1. existing `ThermalStateDigest`;
2. canonical active thermal frontier.

### Tests

Mirror Hydrology hidden-state sensitivity and continuation sufficiency.

The behavioral fixture should create identical sparse thermal deltas with intentionally different active sets, then demonstrate that bounded `step(...)` visits/advances different cells until continuation identity is added.

### Compatibility

`ThermalStateDigest` remains disturbed thermal-field identity.

## Q2.3 — Active lava continuation identity

Tracking: #79.

### Add

`ActiveLavaAuthorityContinuationDigest` binding:

1. existing `ActiveLavaStateDigest`;
2. `next_step`.

### Behavioral fixture

Create a symmetric/tied lateral-routing geometry in which at least two candidates have equal gravity/morphology rank. Prove:

- same lava store digest;
- different `next_step`;
- different continuation digest;
- `lava_tie(cell, step)` may choose a different flow target;
- resulting physical state can diverge after one equal external step.

Then prove equal continuation identity produces equal routing.

## Q2.4 — Active eruption nested continuation/checkpoint identity

Tracking: #79.

### Problem

`ActiveEruptionSession::digest()` currently embeds the lava physical-state digest while `ActiveEruptionCheckpoint` stores the nested lava checkpoint including `next_step`.

Changing only nested `next_step` can therefore evade the session physical-state digest check while changing later eruption routing.

### Add

Prefer a separate `ActiveEruptionContinuationDigest` or checkpoint-integrity digest that binds:

- existing `ActiveEruptionStateDigest`;
- nested `ActiveLavaAuthorityContinuationDigest`;
- any other session-owned continuation fields discovered by implementation review.

### Tests

- change only nested lava `next_step`: physical eruption state digest may remain equal; continuation/checkpoint digest must differ;
- tampered checkpoint continuation field fails exact checkpoint integrity validation;
- checkpoint/reload preserves continuation identity;
- equal continuation + equal Matter/Hydrology/Thermal inputs produces equal next eruption result.

## Q2.5 — landscape environment residual continuation

Tracking: #81.

### Preferred model

Move environmental accumulation away from untracked `f32` residuals.

Preferred order:

1. use fixed-point integer residual units;
2. place residuals in canonical continuation state associated with each promoted physical object;
3. persist and digest them;
4. ensure object retirement removes associated residuals.

If residuals remain in a separate resource, that resource needs an explicit snapshot/continuation digest and lifecycle registration.

### Tests

- interrupted versus uninterrupted exposure gives exactly equal canonical results;
- sub-threshold residual survives checkpoint/reload;
- residual state cannot be NaN/non-finite;
- retired objects do not leak residual entries;
- equal continuation + equal environmental inputs yields equal next physical state.

## Q2.6 — derived cache/invalidation lifecycle proof

### Scope

- `DirtyMatterRegions`;
- entity/runtime bindings;
- render/collision derived consumers where lifecycle reconstruction is required.

### Principle

Do not persist runtime entity IDs or derived caches as authority truth.

Instead prove that an authority restore causes an equivalent conservative rebuild.

### Required proof

A fixture should:

1. create Matter state with a derived collision/render representation;
2. mutate Matter so at least one region is dirty;
3. checkpoint/reload at the lifecycle boundary;
4. reconstruct runtime/derived consumers from restored authority state;
5. prove no stale pre-mutation representation survives;
6. prove physical authority digest is unchanged by reconstruction.

If exact pending dirty work must be preserved for performance, it may have runtime continuation identity, but it must remain downstream of Matter truth.

## Q2.7 — continuation-manifest composition

Do not create one mega-authority.

Add a world/domain continuation manifest that references typed continuation identities rather than copying mutable state.

Candidate contents, only when active/relevant:

- Matter physical state + replay lineage as required;
- Hydrology continuation digest;
- Thermal continuation digest;
- ActiveLava continuation digest / ActiveEruption continuation digest;
- SurfaceWater physical state digest;
- Ecosystem/geomorph/sediment/cryosphere physical identities where continuation is self-contained;
- landscape physical + environmental residual continuation identity;
- explicit simulation instant;
- native scope/reference frame;
- schema/policy identity.

Absence must be explicit: an inactive subsystem is not the same thing as an omitted unknown subsystem.

This manifest should connect later to world lifecycle suspend/resume/revisit receipts without moving domain ownership into CUF.

## Q2.8 — qualification gate

Add a separate gate; do not redefine Q1.

Candidate:

`bash scripts/qualify-universal-matter-v4.8-q2-continuation.sh`

Minimum sequence:

1. verify exact Q1 ancestor/receipt;
2. `cargo fmt --all -- --check`;
3. Terrain tests;
4. Terrain clippy `-D warnings`;
5. explicit Hydrology continuation tests;
6. explicit Thermal continuation tests;
7. explicit ActiveLava continuation/tied-flow tests;
8. explicit ActiveEruption checkpoint-continuation tests;
9. landscape residual interrupted/reload test;
10. derived cache reconstruction lifecycle proof;
11. existing v4.8 proof/handoff gates;
12. CUF v0.10.1 regression;
13. repository workspace/license/diff hygiene;
14. evidence capsule + exact qualified tree identity.

## Q2.9 — evidence capsule

Record at minimum:

- Q1 ancestor commit/tree and evidence identifier;
- Q2 head/staged tree;
- all Q2 repair commits in order;
- Rust/Cargo/Nix/platform;
- Cargo.lock identity;
- explicit named continuation tests and results;
- complete gate logs;
- repository status before/after;
- PASS/FAIL;
- checksum manifest.

A Q2 PASS is attached to an exact Git tree, not a branch name.

## Q2 exit gate

Q2 is green only when all are true:

- no known continuation-significant authority field remains outside the identity required for its claimed resume semantics;
- physical-state digests retain documented compatibility;
- hidden-state sensitivity tests pass;
- continuation-sufficiency tests pass;
- lifecycle cache reconstruction is proven;
- interrupted/reloaded landscape environmental evolution matches uninterrupted evolution under the bounded fixture;
- full Terrain/CUF regressions remain green;
- exact evidence capsule identifies the qualified tree.

## After Q2

Only then begin/qualify CUF v0.11 as Q3 production integration:

1. typed native spatial bindings;
2. Matter observation;
3. SurfaceWater V2 observation;
4. Ecosystem V2 observation;
5. Matter-backed watershed/hydrogeology derived views;
6. complete multi-source groundwater observation;
7. deterministic weather forcing evidence;
8. LivingWatershedPolicyV2;
9. native authority-driven A→B→C proof.

Conserved surface infiltration into groundwater (#77) can follow as a dedicated physical-coupling tranche and later enter the Q4 watershed scenario.