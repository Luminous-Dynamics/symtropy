# Furnishing Functional Envelopes v0.2

**Status:** synthetic specification plus independent Rust oracle; not an accessibility, building-code, biomechanics, or physical-safety certification.

## Paradigm shift

A non-overlapping furniture footprint is not a usable furnishing. This v0.2 slice models a separate, item-local functional envelope for front-side tasks such as opening a drawer, pulling a chair back, or preserving service access. The envelope moves with the item through the four supported quarter-turn orientations and is independently checked against other placed footprints and the room boundary.

The initial model deliberately uses axis-aligned 2D rectangles and synthetic known-truth dimensions. It does not approximate a human body, door-sweep arc, reach biomechanics, a prescribed accessibility clearance, or regulatory compliance. Those require task- and standards-specific models and qualified input evidence.

## Wire contract

- `schema_version = 0.2.0`; strict JSON Schema at `fixtures/interiors/furnishing_functional_envelopes_v0_2.schema.json`.
- Functional envelopes are defined in the unrotated item's local frame, starting at the local front edge (`y_m = item.depth_m)).
- A placement's x/y is the lower-left corner of the quarter-turn-aligned physical footprint. Its envelope is transformed with the same rotation.
- `synthetic_known_truth` permits geometric evaluation only within this synthetic oracle; `unknown` yields `indeterminate`.
- A conflicting, failed, or unknown mandatory task remains `blocked`. Human preference scores must not override the hard gate.

## Initial independent tests

The frozen corpus covers a clear drawer, a blocked drawer, a front/service envelope rotated 90 degrees, unknown mandatory envelope evidence, and a known envelope that extends beyond the room. Adversarial mutations try to forge a passing collision, ignore a rotation transform, promote unknown evidence, hide an outside-room conflict, duplicate envelope identity, and use an unknown schema version.

## v0.3 scaling work

The stacked implementation in [FURNISHING_FUNCTIONAL_ENVELOPES_V0_3.md](FURNISHING_FUNCTIONAL_ENVELOPES_V0_3.md) adds a balanced BVH broad phase for sparse collision queries, indexed task/envelope lookups, brute-force equivalence tests, and a dependency-free 25/100/1,000/10,000 task benchmark. The BVH narrows candidates only; exact rectangle overlap remains authoritative. No performance claim is made until the exact-head workflow emits p50/p95 results.

## Acceptance gates (proposed, not measured)

- Zero false-safe outcomes on the frozen synthetic functional-envelope cases.
- 100% of missing/unknown/invalid mandatory envelope evidence remains indeterminate or blocked.
- Exact deterministic replay for identical pinned inputs.
- Benchmark physical footprint overlaps separately from functional envelope conflicts.
- Scale diagnostic suites to 25, 100, 1,000 and eventually 10,000 item/task checks; report p50/p95 latency and memory. Never infer code compliance, accessibility, or user benefit from these synthetic tests.
