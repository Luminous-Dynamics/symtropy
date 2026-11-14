# Topology-aware geometry for Symtropy

**Status:** research design proposal; no implementation or performance claim is established by this document.  
**Tracking issue:** [PHYS-TOPOLOGY-001](https://github.com/Luminous-Dynamics/symtropy/issues/2876).

## Decision in one sentence

Treat coordinate dimension, topology, metric, and region connectivity as separate concepts; validate an orthogonal periodic box first, before introducing a general topology trait or pocket-region simulation.

## Why this is not just N-dimensional support

A coordinate vector in R^D only describes coordinates. It does not say whether two boundary points are identified, what the shortest displacement is, or whether collision candidates must include translated copies of a shape. A flat periodic domain with periodic axis set P is a quotient by a lattice:

\[ M = \mathbb{R}^{D}/\Lambda, \qquad \Lambda = \{ n \odot L : n_i \in \mathbb{Z} \text{ for } i \in P,\ n_i = 0 \text{ otherwise} \}. \]

For the first implementation, restrict L to an orthogonal axis-aligned box. This represents a product of ordinary and circular axes, commonly written \(\mathbb{R}^{D-k}\times\mathbb{T}^k\). It is not a general curved manifold.

Scientific simulation systems make these semantics explicit. LAMMPS defines which axes are periodic and accounts for cross-boundary interactions and image copies. HOOMD-blue distinguishes minimum-image displacements from unwrapped positions and periodic image vectors. CGAL gives periodic topology its own data structure for flat-torus triangulation. These are design references, not directly comparable rigid-body benchmark results:

- [LAMMPS boundary settings](https://docs.lammps.org/boundary.html)
- [HOOMD-blue box and periodic boundary conditions](https://hoomd-blue.readthedocs.io/en/v7.2.0/hoomd/box.html)
- [CGAL periodic 3D triangulations](https://doc.cgal.org/latest/Periodic_3_triangulation_3/)

## Proposed Phase A API contract

Names below are placeholders; do not freeze public API names until the small pure-domain prototype and call-site audit are complete.

```rust
// Illustrative shape only; not implemented by this document.
struct PeriodicBox<const D: usize> {
    origin: SVector<f64, D>,
    lengths: SVector<f64, D>,
    periodic_axes: [bool; D],
}
```

Required operations:

- `validate`: enabled axes must have finite, strictly positive periods; reject non-finite origins, zero/negative lengths on active axes, and unsupported numeric ranges.
- `wrap_point`: map a coordinate to a canonical half-open representative for each active periodic axis; leave inactive axes unchanged.
- `minimum_image_displacement`: map point displacement to a nearest lattice representative. Specify exact half-period ties explicitly and preserve oddness (`d(a,b) = -d(b,a)`) where practical; tests must lock down the chosen tie convention.
- `image_offset`: represent a lattice image as integer offsets, rather than silently mutating a body's physical position.
- `unwrapped_position` or equivalent bookkeeping: preserve continuous trajectories across wraps for velocity estimates, replay, and diagnostics.

Do not add a universal `Topology` trait yet. A concrete flat-box type is easier to test and provides evidence about the right abstraction boundary. A trait should be considered only when at least two materially different implementations need the same contract.

## Proposed Phase B collision integration

### Narrow phase

Reuse the current support-map and GJK implementation as the narrow-phase routine for candidate body images. A point-pair minimum-image formula is not sufficient for arbitrary extended convex shapes: the candidate generator must first account for shape extents.

For an orthogonal box, conservative AABB overlap determines possible image offsets. Along periodic axis i, translated interval B+nL_i can overlap interval A only when:

\[ \frac{A_{\min,i}-B_{\max,i}}{L_i} \le n \le \frac{A_{\max,i}-B_{\min,i}}{L_i}. \]

Use outward-conservative integer rounding with a documented tolerance. Combine the per-axis integer ranges to enumerate candidate images. For CCD, derive the range from swept bounds, not just instantaneous bounds. The implementation must cap image enumeration and report an explicit unsupported/indeterminate result if its budget is exhausted; it must never turn an incomplete search into a definitive separation.

### First supported envelope

Start with small convex bodies whose extent and swept extent are below the configured per-period limit. Explicitly reject or return unresolved for multi-image cases outside that envelope until they have their own complete oracle campaign. Do not silently assume the nearest image is always sufficient.

### Broadphase and response

- Add periodic image candidates without changing the existing Euclidean broadphase path when no periodic axes are active.
- Establish a canonical pair ordering and explicit image-offset identity so neighboring cells do not apply the same contact twice.
- Test CCD seam crossings, contact caching, constraint anchors, and deterministic replay before integrating the domain into `PhysicsWorld`.
- Preserve the distinction among GJK classification, EPA penetration quality, and solver response. Periodic wrapping does not make the existing N-D EPA path exact.

## Phase C: pocket-like regions are a separate feature

A finite graph of local regions can be useful as an application model: each region owns coordinates and a metric; each interface owns a declared map between boundary charts and a policy for position, velocity, momentum, and energy. The interface must specify directionality, inverse/round-trip requirements, failure behavior, and its conservation contract.

This is not an implementation of pocket universes, general relativity, or physically validated spacetime topology. It is an explicit simulation graph with local coordinate systems. It must not introduce implicit teleportation or silent collision across disconnected regions. Keep Phase C out of the initial periodic-domain change.

## Correctness-first experimental design

### Independent references

1. Analytic displacement and distance fixtures on a circle and a flat torus.
2. A slow brute-force integer-lattice image enumerator for bounded domains.
3. Existing GJK evaluated on explicitly translated support shapes.
4. Independent SAT checks for axis-aligned hyperboxes where applicable.

No optimized implementation may be used as its own oracle.

### Properties and adversarial cases

- Wrapping is idempotent.
- Translating an input by any integer lattice vector preserves canonical location and collision classification.
- The displacement convention is documented and symmetric/antisymmetric away from exact nearest-image ties.
- Mixed periodic/non-periodic axes wrap only where configured.
- Seam-crossing fixtures match their translated non-seam equivalents.
- CCD detects seam-crossing contacts over the validated velocity/time-step envelope.
- Shape swap, common translation, and scale metamorphism are tested within documented numeric bounds.
- Include half-period ties, body extents near the limit, multi-image overlaps, repeated supports, degenerate shapes, NaN/infinity, large coordinates, and exhausted image budgets.
- Equivalent periodic images do not create duplicate contact events or impulses.
- Closed-system momentum/energy drift and deterministic replay hashes are measured; do not expect exact conservation from an integrator unless its contract guarantees it.
- Ambiguous numeric outcomes remain unresolved rather than being converted into false contact or separation.

Use separate development, validation, and confirmation seeds. Freeze tolerances and thresholds before scoring confirmation cases.

## Benchmark matrix

| Dimension | Geometry | Periodic axes | Primary stressors |
|---|---|---|---|
| D=2 | circles, axis-aligned boxes, rotated convexes | 1 and 2 | corner/edge seams, half-period ties |
| D=3 | spheres, boxes, convex support maps | 1, 2, and 3 | face seams, CCD, multiple candidates |
| D=4 | analytic spheres and boxes | 1 and 4 | N-D GJK interaction, image count |
| D=5 | analytic/reference shapes where supported | 1 and 5 | generic simplex solver boundary, unresolved tail |

Extend to further dimensions only where the active solver path has a declared validation envelope; do not infer high-D support from a generic type signature.

### Report per case and as p50/p95/p99 by strata

- false definitive classifications against the independent oracle;
- unresolved count and typed reason;
- candidate images per pair, broadphase candidate count, GJK support calls/iterations, duplicate contacts and allocations;
- p50/p95/p99 latency and peak memory;
- seam-crossing CCD miss rate;
- Euclidean-path regression versus the same build and machine;
- work and timing versus brute-force image enumeration;
- replay hash agreement and closed-system momentum/energy drift.

### Provisional gates

- Zero false definitive classifications in the bounded oracle corpus and untouched confirmation seeds.
- Zero seam-crossing CCD misses in the stated supported envelope.
- Zero duplicate contact application for equivalent periodic images.
- No Euclidean-path regression above 3% p95 without a documented and measured dispatch/opt-in decision.
- Demonstrate a statistically supported work or throughput improvement versus brute-force image enumeration on at least one representative multi-body workload while preserving correctness.

The 3% threshold is a proposed engineering gate, not a measured result. If noise prevents resolving it, publish the confidence interval and classify it as unresolved rather than as a pass.

## Fair comparison to existing systems

Use:

- LAMMPS and HOOMD-blue to compare boundary, image, and trajectory semantics;
- CGAL to study formal periodic-topology representation;
- brute-force translated-image enumeration as a small-domain correctness oracle;
- Symtropy's unchanged Euclidean path as a regression baseline;
- Rapier only for matched non-periodic rigid-body baselines unless a periodic reference implementation is explicitly added.

Do not compare raw timings across unlike APIs, hardware, or physical workloads. A real external benchmark should publish exact versions, build flags, shape distributions, seeds, CPU/GPU details, and the same correctness corpus. The advantage to seek is **more explicit topology semantics with bounded, measurable overhead and independently demonstrated correctness**, not a blanket claim of superiority over scientific engines.

## Evidence and claim limits

At the time of this proposal, this file defines work and gates only. It does not claim periodic rigid-body collision has been implemented or benchmarked. Passing Phase A validates point-domain semantics only. Passing Phase B validates collision only for the named dimensions, shape families, period/extent ratios, CCD bounds, and numeric ranges. Neither phase validates arbitrary curved-manifold collision, general relativistic simulation, full-dimensional EPA penetration, or the physical existence of pocket universes.

## References

- LAMMPS, [boundary command](https://docs.lammps.org/boundary.html).
- HOOMD-blue, [Box and periodic boundary conditions](https://hoomd-blue.readthedocs.io/en/v7.2.0/hoomd/box.html).
- CGAL, [Periodic 3D triangulations](https://doc.cgal.org/latest/Periodic_3_triangulation_3/).
- Rapier, [project overview](https://rapier.rs/docs/).
- Symtropy, [PHYS-TOPOLOGY-001 issue](https://github.com/Luminous-Dynamics/symtropy/issues/2876).