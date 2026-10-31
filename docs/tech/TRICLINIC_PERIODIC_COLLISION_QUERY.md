# Triclinic Periodic Convex Queries and Sphere CCD

**Status:** query layer drafted; no PhysicsWorld integration.  
**Implementation:** crates/core/symtropy-physics/src/periodic_lattice_collision.rs  
**Tracking:** [PHYS-TRICLINIC-002 #2964](https://github.com/Luminous-Dynamics/symtropy/issues/2964).  
**Dependencies:** [CVP geometry PR #3010](https://github.com/Luminous-Dynamics/symtropy/pull/3010) and [skew AABB candidates PR #3013](https://github.com/Luminous-Dynamics/symtropy/pull/3013).

## Why the query layer is separate

A triclinic periodic cell identifies points under integer lattice translations B*n. For body B, each image is a distinct translated convex query candidate. The broadphase returns a conservative superset of integer image offsets; the narrow phase must examine the complete returned set.

The broadphase does not apply a translated-AABB floating-point post-filter. This query layer also does not add one before GJK. That choice deliberately trades extra GJK work for a simpler no-miss path at touching boundaries. A future interval-certified filter may be introduced only with an independent proof/test campaign.

## Convex query contract

query_triclinic_convex:

1. obtains world-axis support AABBs from the two complete rigid transforms;
2. asks lattice_image_offsets_for_aabb_pair for the conservative set of all potentially overlapping lattice images of B;
3. computes each exact Cartesian lattice translation B*n;
4. applies that translation to B's complete transform;
5. invokes the existing transformed Euclidean GJK for each image;
6. reports confirmed image hits separately from unresolved GJK attempts.

GJK now exposes a typed `GjkTermination` result: `Intersecting`, `Separated`, or `IterationLimit`. The production cap is named `MAX_GJK_ITERATIONS`; periodic adapters consume the enum and do not duplicate the private numeric limit. `IterationLimit` is always unresolved, never separation. The existing `intersecting` and `iterations` result fields remain available for compatibility.

A result with at least one confirmed hit is Intersecting, even if other images remain unresolved. Callers must also check contact_set_complete() before assuming the entire set of possible image contacts is known. With no confirmed hit, any unresolved image produces Indeterminate; only a complete all-separated query yields Separated.

## Sphere CCD contract

sphere_sphere_ccd_triclinic_all is limited to fixed-radius spheres undergoing linear translation over a finite dt.

1. Build start/end sphere AABBs and union them to produce conservative swept AABBs.
2. Enumerate every potentially overlapping triclinic lattice image using the same conservative candidate generator and caller-supplied image budget.
3. Apply each exact image translation to B's initial position.
4. Run the existing analytic Euclidean sphere-sphere CCD for that image.
5. Validate all TOI/contact outputs and return every hit, sorted by TOI and then lexicographic lattice offset.

This handles multiple different images of one pair during a long sweep. It is not rotating convex CCD and does not integrate impulses or advance body state.

## Independent corpus

The new unit tests include:

- a fixed skew-cell convex sphere seam hit with expected image offset;
- a mixed periodic-mask negative control proving that a nonperiodic basis direction cannot create an image collision;
- a fixed skew-cell sphere seam impact with analytic TOI;
- a fixed all-image CCD comparison against direct integer-shifted Euclidean CCD;
- 160 seeded randomized convex sphere queries compared against direct per-image transformed GJK over a wider integer cube, checking both hit and unresolved image sets;
- 300 seeded randomized sphere CCD queries compared against direct per-image analytic Euclidean CCD over a wider integer cube, checking complete hit sets, TOIs, and deterministic result ordering;
- invalid-radius, non-finite and candidate-budget negative controls.

The randomized tests use deterministic generator state in the source. Their counts are authored cases, not observed pass results.

## Query performance benchmark

The Criterion benchmark at crates/core/symtropy-physics/benches/triclinic_periodic_queries.rs compares:
- reciprocal-projection candidate enumeration followed by transformed GJK; and
- direct transformed GJK over a fixed wider 13x13 lattice-image cube.

It uses deterministic pair corpora of 32, 128 and 512 sphere pairs, split into interior, first-basis seam, skew-basis seam and separated fixtures. Before timing, every pair's hit and unresolved image sets must agree exactly between both paths. The benchmark emits TRICLINIC_QUERY_PREFLIGHT,v1 records with fixture, pair count, candidate-image count, production GJK calls and oracle GJK calls.

Run:
- cargo bench -p symtropy-physics --bench triclinic_periodic_queries

The brute-force cube is sufficient for the fixed benchmark corpus only; it is not a universal lattice enumeration. Preserve benchmark output, exact commit/toolchain/hardware identity and all preflight rows. No performance claim is made until the benchmark executes.

## Exact-head qualification

Run on the exact branch head:

- cargo fmt --check
- cargo test -p symtropy-physics periodic_lattice_collision
- cargo bench -p symtropy-physics --bench triclinic_periodic_queries
- cargo test -p symtropy-physics periodic_lattice_broadphase
- cargo test -p symtropy-math periodic_lattice
- cargo test -p symtropy-math
- cargo test -p symtropy-physics

Preserve the exact commit SHA, Cargo.lock digest, Rust/Cargo version, command exit statuses, raw logs, random seed, minimized failure basis/geometry, and CI run IDs. Queued or absent workflows are not passing evidence.

## Remaining engineering gates

This query layer is not full periodic dynamics. Remaining work includes:

- propagate GJK typed termination into every periodic adapter and downstream diagnostics, while preserving unresolved outcomes through contact/event receipts;
- add independent high-precision CVP and reciprocal-projection interval checks for nearly degenerate accepted cells;
- integrate image-aware contact anchors and lever arms into manifold generation;
- key contact cache, warm starts and collision events by canonical body pair plus lattice image offset;
- verify duplicate impulse suppression, simultaneous contacts, angular response and friction;
- qualify periodic CCD for rotating convex shapes;
- integrate into PhysicsWorld only after all geometry/query gates pass.

## Comparison and claim ceiling

GROMACS and LAMMPS demonstrate that periodic triclinic cells are established simulation practice; their algorithms are not drop-in rigid convex collision baselines. The Symtropy-specific claim to test is narrower: bounded lattice-offset identity, no-miss conservative image candidates, transformed convex narrow phase, complete per-image fixed-sphere CCD, and explicit unresolved/budget outcomes.

This is bounded D=2/3 query infrastructure over the supported Euclidean lattice envelope. It does not establish general periodic rigid-body physics, arbitrary Riemannian manifolds, general relativity, or physical pocket dimensions.
