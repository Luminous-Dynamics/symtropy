# Exact-rational linear CCD known-answer fixture

This is an original four-query by-construction fixture for scorecard and adapter tests. Coordinates are exact decimal-integer rational pairs. All motion is linear over normalized time t in [0,1]. The Boolean ground truth is derived from the geometry below, not from a CCD implementation.

## Edge-edge

Query 0 uses a stationary segment A from (0,0,0) to (1,0,0). Segment B is vertical at x=1/2, spans y in [-1,1], and moves linearly from z=1 at t=0 to z=-1 at t=1. The segments intersect at exactly t=1/2, at (1/2,0,0). Ground truth: collision, exact TOI 1/2.

Query 1 has the same movement but places B at x=3/2, outside A's x-range [0,1]. The segments never intersect. Ground truth: no collision.

## Vertex-face

The face is stationary in the plane z=0, with triangle vertices (0,0,0), (1,0,0), and (0,1,0).

Query 0 moves the vertex along (1/4,1/4,1-2t). It crosses the plane at exactly t=1/2 at (1/4,1/4,0), which lies strictly inside the triangle because 1/4+1/4<1. Ground truth: collision, exact TOI 1/2.

Query 1 moves the vertex along (3/4,3/4,1-2t). It crosses the plane at t=1/2 outside the triangle because 3/4+3/4>1. Ground truth: no vertex-face collision.

## Scope

This fixture provides both positive and negative edge-edge and vertex-face examples, including exact rational TOIs. It is a unit-test benchmark and a regression oracle, not a representative real-world dataset, a full exact CCD implementation, or an externally measured performance result. It intentionally does not stand in for the NYU/Scalable-CCD benchmark, which must be run separately.

The SHA-256 values and known-TOI records are in manifest.json.
