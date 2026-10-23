# Shipyard polygon geometry (experimental V0)

This isolated crate is a fixed-point geometry prototype intended to support later irregular ship-block footprint work. It is not yet integrated with the spatial verifier or the planner.

## Numeric contract

- Every vertex is a signed `i64` tick. The benchmark manifest must declare the physical unit and coordinate scale.
- Authoritative predicates use checked `i128` integer arithmetic, with no floating-point epsilon.
- If an exact predicate overflows its checked intermediate range, it returns `ArithmeticOverflow`. The caller must reject or route the case to a separately qualified wider-exact implementation; it must not treat overflow as no collision.
- A polygon is passed as a cycle of vertices without repeating the first vertex at the end.
- Degenerate, duplicate-vertex, zero-area, adjacent-edge overlap and non-adjacent self-intersecting rings are rejected.

## V0 API

## Frozen oracle targets

The unit suite includes 100 grid-aligned rectangle footprints and compares all 10,000 ordered rectangle pairs against a separate unit-cell oracle. The concave corpus is now 150 translated polygon placements: three independently authored orthogonal shapes, 25 integer translations each, and both winding directions. It compares all 22,500 ordered polygon pairs against an independent cell-set plus inclusive axis-aligned-segment reference.

Pinned expected ablation counts for that synthetic corpus are 12,980 positive-area overlaps, 6,216 boundary contacts, and 3,304 disjoint pairs. A conservative AABB-only conflict predicate flags 20,180 pairs, including 984 pairs whose exact polygon relation is disjoint, with zero AABB false negatives. These are authored count expectations derived from the deterministic corpus; the exact-head Rust tests must confirm them before they are treated as validated results. They are not industrial yard measurements. The result motivates a safe architecture: use AABBs to reject definitely disjoint broadphase pairs, but use exact polygon predicates for every candidate pair whose boxes overlap rather than treating whole envelopes as authoritative geometry.

A frozen two-block counterexample uses two identical L-shaped footprints in a 6×6 yard. With the first fixed at the origin, the independent exact oracle finds four non-overlapping translations for the second block, while the conservative AABB-only conflict policy rejects all 16 translations because the rectangular envelopes overlap or touch. This is a tiny synthetic completeness example, not a yard-wide optimizer benchmark; its acceptance target is four exact-feasible translations and zero AABB-accepted translations.

A separate temporal control uses a concave L-shaped footprint to test endpoint reuse at the exact clearance tick, a one-tick delayed-clearance conflict, and boundary-contact policy under overlapping time intervals.

- Exact integer orientation and segment-relation predicates.
- Validated simple polygons, point location and canonical vertex cycles.
- Polygon relation classification: disjoint, boundary contact or positive-area overlap.
- Conservative conflict semantics: boundary contact counts as a conflict.
- Half-open integer time windows; endpoint-touching windows may reuse the same footprint, while a delayed-clearance interval conflicts.
- An independent unit-cell rectangle oracle checks every ordered pair of small axis-aligned rectangle cases.


## Published geometry-quality benchmark

The 2026 irregular-block shipyard study reports representation-induced footprint-area inflation in Table 4: 6.39%, 6.37%, and 6.21% for simplified R2 shapes on its 40-, 50-, and 60-block cases; regularized R3 shapes inflate area by 14.89%, 17.76%, and 17.75%, respectively. Source: [Wang, Yin & Khan (2026), DOI 10.1049/cim2.70066](https://doi.org/10.1049/cim2.70066).

These are shape-representation deltas, not the paper's yard-utilization scores. The exact-polygon milestone should report (a) representation area inflation, (b) actual placed source-profile area / usable yard area, and (c) conservative false conflicts independently. The intended geometry target is no simplification-induced area inflation beyond a declared fixed-point quantization tolerance, while retaining explicit clearance policy; this is not yet demonstrated against the paper's source profiles.

The appendix does not supply a complete machine-readable Case9_50_4 corpus in the article download; the associated simulation data are available from the corresponding author upon request. Do not substitute the current synthetic orthogonal shapes for a paper reproduction. The source-authorized profile set and exact aisle/yard assumptions must be pinned first.

## Limits

This is only a planar footprint predicate layer. It does not establish swept-path clearance, access routes, rotation during movement, load support, stacking, floor bearing, vertical clearance, crane envelopes, lift physics, human safety or operational authorization. It has not yet been connected to the existing finite-yard verifier and has not been CI-confirmed at this commit.

The algorithm is O(n²) for ring validation and O(nm) for two-footprint comparison. Production-sized profiles will need a benchmarked broadphase, but that broadphase may only eliminate definitely disjoint bounding boxes; every possible contact/overlap still goes through authoritative predicates.

## Research basis

The exact-predicate direction follows the general lessons of Jonathan Shewchuk's robust geometric predicates: orientation and intersection decisions must not depend on arbitrary floating-point epsilons. This V0 instead accepts integer-tick input and uses checked integer determinants. See the original [robust predicates page](https://www.cs.cmu.edu/~quake/robust.html) and [paper](https://doi.org/10.1145/237218.237337).

The next gate is independent differential testing over concave polygons, collinear edges, containment, touching and arithmetic-boundary controls, followed by a separate integration adapter.
