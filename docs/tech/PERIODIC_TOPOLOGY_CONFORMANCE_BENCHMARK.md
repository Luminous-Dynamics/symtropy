# Periodic Topology: Independent Conformance and Benchmark Protocol

**Status:** authored test corpus; execution required on the exact PR head.  
**Implementation under test:** branch test/periodic-independent-conformance-v0.1  
**Parent:** [PHYS-TOPOLOGY-001 #2876](https://github.com/Luminous-Dynamics/symtropy/issues/2876).

## Purpose

The first paradigm shift is to make periodicity a topology-aware contract and verify it against independent image enumeration, not to add another geometric code path and trust its own tests.

The new integration corpus at crates/core/symtropy-physics/tests/periodic_differential_conformance.rs deliberately avoids using production image translation to generate the expected candidate set. Required AABB candidates come from direct integer-image enumeration (extra candidates are permitted because the range solver rounds outward); expected sphere CCD hits come from trying each shifted Euclidean image with the existing analytic sphere solver. Random cases are deterministic and carry fixed seeds.

The current test corpus contains:

| Test | Deterministic cases | Independent oracle |
|---|---:|---|
| Wrapping / explicit lattice offsets | 1,000 | Reconstruct the original point from canonical position + recorded lattice offset |
| Minimum-image displacement | 2,000 | Brute-force integer period shifts and choose the shortest representative |
| Periodic AABB candidate sets | 2,000 | Enumerate image offsets in a fixed wider envelope and recompute overlap directly |
| Periodic sphere CCD | 500 | Directly invoke Euclidean sphere CCD on every shifted image in a wider envelope |
| Minimum-image lattice-shift invariance | 1,000 | Add random integer multiples of each period and confirm the minimum image is unchanged |
| Uniform scaling covariance | 1,000 | Scale origins, periods, points and displacements together at four scale factors; compare offsets and scaled outputs |
| AABB global lattice translation | 500 | Translate both boxes by the same integer number of cells and require the image-offset set to remain unchanged |

Together these are more than 8,000 deterministic scenario assertions, **not passing measurements**. Until the tests run, do not report observed zero mismatches.

## Geometry contract

For each periodic axis i, the canonical coordinate interval is [origin_i, origin_i + period_i). A wrapped point returns an integer lattice offset n satisfying the reconstruction identity x ≈ x_canonical + n_i * period_i, within a documented floating-point envelope.

Minimum-image displacement maps a displacement to a shortest periodic representative. At exact half-period ties, the input sign selects the result sign so reversing the input reverses the output. Tie handling is deterministic but it does not make the nearest image unique.

The AABB image enumerator returns every integer offset for which the translated second AABB may overlap the first, including boundary contact and multi-image cases. It must never silently truncate. Candidate-budget exhaustion is an explicit error, not a no-contact result.

The sphere CCD API must return every analytic per-image hit in increasing TOI order with deterministic offset tie-breaking. This all-images contract is required for event-driven callers that may have more than one image interaction during a long step.

## Required execution gate

Run on the exact PR head:

- cargo fmt --check
- cargo test -p symtropy-physics --test periodic_differential_conformance
- cargo test -p symtropy-physics periodic_broadphase
- cargo test -p symtropy-physics periodic_collision
- cargo test -p symtropy-physics periodic_sphere_response
- cargo test -p symtropy-physics periodic_sphere_world
- cargo test -p symtropy-math periodic
- cargo test -p symtropy-physics
- cargo test -p symtropy-math

Record the exact HEAD commit SHA; Cargo.lock SHA and Rust/Cargo version; commands, exit statuses and durations; per-corpus attempted/passed/failed counts; deterministic seed and full minimized counterexample for every failure; raw stdout/stderr and CI workflow run IDs, not only a green badge.

A queued or missing workflow is not a passing result. Any failure must be retained as a regression fixture before the fix is accepted.

## Differential correctness before performance

Correctness gates are strict:
- zero false definitive separations and zero false definitive intersections on the frozen bounded corpus;
- no missing candidate relative to the independent bounded AABB oracle; conservative extra candidates are permitted and their rate must be reported;
- exact per-image hit-set equality and TOI agreement within a stated tolerance for the analytic sphere CCD oracle;
- no non-finite canonical coordinates, offsets, times, velocities or energy/momentum diagnostics;
- event-budget and image-budget exhaustion must be typed failure, never a successful partial step;
- one authoritative state per body, no duplicate impulse for an already-resolved image contact, deterministic event ordering, and no discontinuous change to unwrapped coordinates at a periodic seam.

An indeterminate result is not a false definitive result, but it counts against completeness and must be reported by reason.

## Performance campaign after correctness passes

Benchmark the same immutable scenarios against:
1. a direct brute-force lattice-image oracle;
2. the production range-formula AABB enumerator;
3. one-image/minimum-image optimization only for scenarios where the size/cutoff precondition is proven;
4. the event-driven sphere reference world and the Euclidean non-periodic baseline.

Sweep dimensions 2 and 3 for the initial sphere world, and separate geometry-only tests if a higher dimension is supported; 0, 1, 2 or 3 periodic axes; body count 16, 64, 256, 1,024 and 4,096; radius/period or support/period ratios including values close to the strict half-period envelope; bodies near seams versus interior-only bodies; short sweeps versus many-box-length sweeps; low/high density; mostly separated, mostly colliding and adversarial stacked cases; image budget including intentionally exhausted configurations.

Record p50/p95/p99 latency, image candidates per pair, narrow-phase calls, event count, allocated bytes, peak memory, failure/indeterminate counts, and momentum/energy/replay diagnostics. Report hardware, optimization flags, toolchain, seeds and workload-generation algorithm.

Compare scaling curves and cost at equivalent correctness, not just raw nanoseconds. A brute-force reference can be slower and still be more trustworthy; it is the oracle, not a competitor to optimize away.

## Industry comparison: scope and fairness

This project should not claim that periodic boundary conditions are new. Mature scientific systems already use them:

- **GROMACS** documents periodic boundary conditions, triclinic simulation cells, the minimum-image convention, and restrictions needed to make one-image short-range interactions valid: [official periodic boundary-condition manual](https://manual.gromacs.org/2024.4/reference-manual/algorithms/periodic-boundary-conditions.html).
- **LAMMPS** documents periodic and non-periodic boundaries per axis and the behaviour of particles crossing periodic faces: [official boundary command reference](https://docs.lammps.org/boundary.html).
- **CGAL** provides explicit periodic triangulation types with image offsets and a geometry/data-structure design split: [official periodic 3D triangulation manual](https://doc.cgal.org/latest/Periodic_3_triangulation_3/).

These systems are useful conceptual comparators but not drop-in collision-engine baselines: GROMACS/LAMMPS focus on particle/interaction mechanics; CGAL's periodic triangulation is a different geometric primitive. They do not establish that Symtropy is better or worse at rigid convex collision detection.

The fair Symtropy claim to test is narrower and measurable: an image-aware convex collision/CCD pipeline with explicit result completeness, stable contact identity, unwrapped trajectory state, deterministic event policy, and bounded fail-closed behavior. Demonstrate those properties with the independent oracle corpus first, then compare throughput and tail latency against a correct brute-force reference and any compatible external engine on identical scenarios.

## Follow-on gates

The tests in this PR do not close the entire periodic programme. Still required:
- integrate image-aware local contact anchors and lever arms into the production solver;
- ensure contact caches, constraints and events include the canonical periodic image key;
- verify angular response, friction, simultaneous impacts, resting contacts and duplicate impulse control;
- prove Euclidean-path regression bounds;
- preserve exact-head test/CI receipts and minimized failures.
