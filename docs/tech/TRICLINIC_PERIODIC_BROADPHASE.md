# Triclinic Periodic AABB Image Broadphase

**Status:** standalone conservative candidate generator; not wired into PhysicsWorld.  
**Implementation:** crates/core/symtropy-physics/src/periodic_lattice_broadphase.rs  
**Parent geometry:** PHYS-TOPOLOGY-003 #2924 and draft PR #3010.

## Geometric bound

For Cartesian AABBs A and B, an image B + Bn overlaps A only if the lattice translation Bn lies in the Minkowski difference A - B:

A_min - B_max <= Bn <= A_max - B_min.

Applying the inverse lattice basis B^-1 maps this Cartesian difference box into a parallelepiped in fractional coordinates. Each fractional coordinate is a linear functional, so its minimum and maximum over the Cartesian AABB difference are attained at a corner. The 2^D corners therefore provide conservative independent integer intervals for each lattice coordinate n_i.

The Cartesian product of those intervals is a conservative integer-image candidate box. It may contain offsets whose translated B AABB does not intersect A because the transformed difference region is generally a skewed parallelepiped rather than the enclosing axis-aligned fractional-coordinate box. Those false positives remain in the returned candidate superset. This deliberately avoids a second floating-point overlap filter that could reject a true touching image due to translation rounding. The downstream narrow phase performs the definitive geometry test.

The key safety requirement is **no missed image**. Exact candidate-set equality is not required when conservative projection padding is used. Extra candidate count is broadphase overhead and must be reported.

## Current implementation envelope

- Full-rank Euclidean PeriodicLattice in D=2 or D=3, with an explicit active periodic-axis mask. Inactive fractional coefficients are constrained to zero.
- Cartesian axis-aligned AABBs; lattice cell may be skewed.
- Reciprocal/fractional coordinate projection using the PeriodicLattice QR solve.
- Explicit per-query maximum candidate-box product; inactive axes are fixed to image coefficient zero, budget exhaustion is a typed error and never silently truncates.
- Returns the complete Cartesian product of conservative reciprocal-coordinate intervals; no exact Cartesian overlap filtering is performed here.
- Conservative projection padding grows with the lattice QR pivot-quality proxy. It is a rejection/overgeneration safeguard, not a proof of a formal interval-arithmetic bound for all IEEE-754 inputs.

This is a separate function from the orthogonal image_offsets_for_aabb_pair(PeriodicBox, ...); it does not change existing orthogonal semantics.

## Independent tests

- Fixed 2D skewed-cell image-set comparison against direct integer-image translation.
- 1,000 seeded random 2D AABB-pair cases; production candidates must be a superset of all true brute-force image overlaps.
- 80 seeded random 3D AABB-pair cases with the same no-miss oracle.
- A mixed periodic-rank fixture confirms that inactive image coefficients stay zero and a displacement along the non-periodic complement is not wrapped.
- Touching-face inclusion.
- Explicit candidate-budget exhaustion.
- Invalid/inverted AABB rejection.

The tests are authored, not run results. The fixed brute-force windows must be demonstrated to contain every possible true overlap for their frozen fixture ranges. Any failing seed should be preserved with the basis and both AABBs.

## Benchmark protocol

Compare this projection-based enumerator with direct brute-force lattice-image enumeration on identical fixed corpora. Run 2D and 3D separately. Sweep pair counts, cell skew/QR pivot ratio, AABB extent relative to lattice lengths, seam/interior location, sparse and high-overlap cases, and candidate budget.

Report:
- p50/p95/p99 query latency;
- true overlapping image count and returned candidate count;
- overgeneration ratio = (returned candidates - true overlaps) / max(true overlaps, 1);
- conservative candidates above the true-overlap count (overgeneration);
- candidate-box product, budget exhaustion and numeric failure counts;
- source SHA, scenario and basis digests, seeds, compiler/toolchain, CPU and optimization flags.

Do not turn an overgeneration improvement into a correctness claim. The hard gate is no missed image against the independent oracle. Conservative extra candidates are expected and counted; the narrow phase rejects them.

## Industry context

GROMACS and LAMMPS already support triclinic periodic cells in scientific simulation; their cell semantics are mature prior art. CGAL's periodic geometry also explicitly tracks images. These systems are conceptual comparators, not drop-in benchmarks for this AABB candidate query. The Symtropy-specific claim to test is the bounded no-miss image candidate API and its integration with convex collision/CCD.

- GROMACS: https://manual.gromacs.org/documentation/current/reference-manual/algorithms/periodic-boundary-conditions.html
- LAMMPS: https://docs.lammps.org/stable/Howto_triclinic.html
- CGAL: https://doc.cgal.org/latest/Periodic_3_triangulation_3/index.html

## Explicit non-goals / gates

This code does not implement triclinic GJK/EPA query aggregation, swept AABB image generation, rotating convex CCD, periodic contact keys, impulse response, contact-cache identity, or PhysicsWorld integration. Those follow only after the lattice nearest-image and AABB image-set gates execute successfully on exact heads with retained receipts.
