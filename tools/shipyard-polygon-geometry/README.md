# Shipyard polygon geometry (experimental V0)

This isolated crate is a fixed-point geometry prototype intended to support later irregular ship-block footprint work. It is not yet integrated with the spatial verifier or the planner.

## Numeric contract

- Every vertex is a signed `i64` tick. The benchmark manifest must declare the physical unit and coordinate scale.
- Authoritative predicates use checked `i128` integer arithmetic, with no floating-point epsilon.
- If an exact predicate overflows its checked intermediate range, it returns `ArithmeticOverflow`. The caller must reject or route the case to a separately qualified wider-exact implementation; it must not treat overflow as no collision.
- A polygon is passed as a cycle of vertices without repeating the first vertex at the end.
- Degenerate, duplicate-vertex, zero-area, adjacent-edge overlap and non-adjacent self-intersecting rings are rejected.

## V0 API

- Exact integer orientation and segment-relation predicates.
- Validated simple polygons, point location and canonical vertex cycles.
- Polygon relation classification: disjoint, boundary contact or positive-area overlap.
- Conservative conflict semantics: boundary contact counts as a conflict.
- Half-open integer time windows; endpoint-touching windows may reuse the same footprint, while a delayed-clearance interval conflicts.
- An independent unit-cell rectangle oracle checks every ordered pair of small axis-aligned rectangle cases.

## Limits

This is only a planar footprint predicate layer. It does not establish swept-path clearance, access routes, rotation during movement, load support, stacking, floor bearing, vertical clearance, crane envelopes, lift physics, human safety or operational authorization. It has not yet been connected to the existing finite-yard verifier and has not been CI-confirmed at this commit.

The algorithm is O(n²) for ring validation and O(nm) for two-footprint comparison. Production-sized profiles will need a benchmarked broadphase, but that broadphase may only eliminate definitely disjoint bounding boxes; every possible contact/overlap still goes through authoritative predicates.

## Research basis

The exact-predicate direction follows the general lessons of Jonathan Shewchuk's robust geometric predicates: orientation and intersection decisions must not depend on arbitrary floating-point epsilons. This V0 instead accepts integer-tick input and uses checked integer determinants. See the original [robust predicates page](https://www.cs.cmu.edu/~quake/robust.html) and [paper](https://doi.org/10.1145/237218.237337).

The next gate is independent differential testing over concave polygons, collinear edges, containment, touching and arithmetic-boundary controls, followed by a separate integration adapter.
