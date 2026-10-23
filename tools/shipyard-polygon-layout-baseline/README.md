# Shipyard polygon layout baseline (experimental V0)

This crate consumes the fixed-point polygon prototype and compares two deterministic policies on the same static finite-yard instance:

- ConservativeAabb treats any closed intersection between two footprint bounding boxes as a conflict.
- ExactPolygon calls the polygon relation kernel; boundary touch remains a conflict, while disjoint concave footprints can share overlapping bounding boxes.

Both policies use the same caller order and the same y-major, then x-major integer translation candidates. The output counts placed blocks, unplaced requests and candidate checks. An unplaced request is not a global infeasibility proof: it only means this first-fit policy exhausted its finite translation candidates after prior choices.

## Frozen 6x6 counterexample

Two identical L-shaped footprints have 3x3 envelopes. The first is placed at (0,0). The second has four exact-feasible offsets: (2,2), (3,2), (2,3) and (3,3). Every candidate's bounding box intersects or touches the first bounding box, so the conservative AABB policy rejects all 16 legal offsets.

Pinned targets:
- AABB policy: one block placed, one remains unplaced, 17 candidate checks.
- Exact polygon policy: both blocks placed, zero unplaced, 12 candidate checks; second origin (2,2).
- An independent raster + boundary-contact oracle must confirm the accepted pair is physically disjoint in the declared orthogonal integer-grid model.

These are small synthetic semantic/coverage targets, not industrial performance outcomes or evidence that arbitrary real-yard shapes are represented correctly.

## Scope boundaries

No rotation, arbitrary rigid transforms, time windows, transfer corridors, cranes, resource capacity, stacking, vertical clearance, floor bearing or motion paths. It is not a global optimizer. Geometry arithmetic overflow and malformed polygons remain typed errors from the dependency; neither is treated as free space.

## Intended next benchmark

Scale from tiny known-answer examples to a frozen orthogonal-polygon corpus, then add temporal reuse and exclusions, then integrate robust recovery. Compare exact-polygon and AABB baselines on the same candidate budget, counting false-feasible results separately from conservative false rejection, and validate all returned layouts through an independent oracle. A SOTA comparison requires matched published input geometry and objective definitions.
