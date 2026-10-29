# Periodic Image Enumeration Benchmark

**Status:** benchmark authored; no timings have been collected.  
**Command:** cargo bench -p symtropy-physics --bench periodic_image_enumeration

## What is measured

The Criterion benchmark compares:
- the production range-formula AABB image enumerator; and
- an independently coded brute-force lattice-offset enumeration over a deliberately wider fixed image range.

Both output candidate image vectors, so the timed work includes allocation and output construction. The brute-force range is sufficient for the frozen benchmark corpus but is not a general-purpose algorithm.

The deterministic corpus sizes are 64, 256 and 1,024 AABB pairs. Three fixture families run at each size:
1. interior-separated pairs;
2. pairs arranged to cross the x-periodic seam;
3. extended AABBs with multiple overlapping periodic images.

The periodic domain is orthogonal with a 10-unit x period and a non-periodic y axis. This is intentionally narrow: the result is an algorithm microbenchmark, not a full periodic physics or multidimensional topology benchmark.

## Preflight and output

Before timing, each corpus is checked against the independent oracle: every image the brute-force oracle says can overlap must be present in the production candidate list. Conservative overgeneration is allowed, but must be measured. The executable emits lines with the stable prefix PERIODIC_IMAGE_PREFLIGHT,v1, including fixture name, pair count, oracle candidate count, production candidate count and extra candidate count.

A missing required candidate aborts the benchmark before timing. An extra candidate does not prove a collision; it is broadphase overhead and must be resolved by narrow phase or direct AABB disjointness.

## Reproduction and reporting

Record:
- exact Git commit and Cargo.lock digest;
- Rust/Cargo versions, OS, CPU, target triple, optimization flags and power policy;
- the full Criterion output and PERIODIC_IMAGE_PREFLIGHT records;
- p50/median estimate plus Criterion confidence intervals, and p95/p99 from a separate workload-level latency harness if needed;
- oracle and production candidate counts, extra candidate ratio, and any budget/error outcomes.

Run with the same toolchain and machine for both methods. Do not report a ratio without the fixture, pair count, candidate count and measurement interval. Do not infer end-to-end world speedup from this microbenchmark.

## Acceptance before optimization claims

- The independent inclusion preflight passes on the exact head.
- Candidate-budget exhaustion and arithmetic errors remain visible, not omitted from measurements.
- The performance comparison includes the image-set/candidate-count outputs as a correctness side channel.
- Any later optimization preserves oracle inclusion and records overgeneration.
- A measured speedup is only claimed for this benchmark envelope. It does not generalize to triclinic lattices, rotating convex shapes, the full PhysicsWorld contact solver, or all periodic physical simulations.

## Industry context

Periodic-boundary behavior is mature in molecular dynamics and computational geometry. GROMACS documents the minimum-image convention and restrictions on interaction cutoff relative to the shortest box vector; LAMMPS documents per-axis periodic boundaries and image flags; CGAL's periodic triangulation uses explicit point-image offsets. These are conceptual references, not direct benchmarks for Symtropy's rigid convex AABB candidate function.

- GROMACS: https://manual.gromacs.org/2024.4/reference-manual/algorithms/periodic-boundary-conditions.html
- LAMMPS: https://docs.lammps.org/boundary.html
- CGAL: https://doc.cgal.org/latest/Periodic_3_triangulation_3/

The longer-term differentiation remains correctness under image-aware convex collision, CCD and impulse response, followed by fair performance comparisons on identical scenarios.
