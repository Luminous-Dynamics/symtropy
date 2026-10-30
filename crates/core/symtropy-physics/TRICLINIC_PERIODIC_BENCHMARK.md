# Triclinic Periodic Geometry Benchmark

**Benchmark target:** cargo bench -p symtropy-physics --bench triclinic_periodic_geometry  
**Implementation:** LatticePeriodicDomain and lattice-periodic broadphase candidate enumeration.  
**Tracking:** PHYS-TOPOLOGY-003 #2924.

## Purpose

Compare the bounded inverse-basis closest-image solver and conservative skew-cell AABB image generator with direct bounded integer-image enumeration on identical deterministic workloads.

This is a geometry microbenchmark. It does not measure full PhysicsWorld, GJK/EPA, periodic CCD, molecular dynamics, or fluid solver throughput.

## Benchmark suites

### Closest image

- 2D: orthogonal, moderate-skew and high-skew cells.
- 3D: orthogonal and triclinic cells.
- 96 deterministic displacement queries per case.
- Production: bounded inverse-row interval enumeration.
- Reference: direct integer offset enumeration over [-8,8] on each periodic basis coordinate.

The untimed preflight compares squared nearest-image distances within a declared floating-point tolerance and reports the number of candidate offsets examined by the production solver.

### AABB image candidates

- 2D: orthogonal, moderate-skew and high-skew cells.
- 96 deterministic AABB pairs per case.
- Production: inverse-basis interval candidate superset.
- Reference: direct integer-image enumeration over [-8,8] on each periodic basis coordinate.

The untimed preflight requires every oracle overlap image to be present in the production candidate set. Extra candidates are legal but count as broadphase over-generation. The benchmark reports oracle image count, production candidate count and extra candidates before registering the timing comparisons.

## Output contract

Before Criterion timings, the harness emits versioned key-value records:

- TRICLINIC_MIN_IMAGE_PREFLIGHT,v1
- TRICLINIC_AABB_PREFLIGHT,v1

A failed preflight panics before benchmark results are collected. Criterion timings are not correctness evidence.

Preserve the complete output together with:

- exact source commit SHA and working-tree status;
- Cargo.lock digest;
- Rust and Cargo version;
- compiler target and optimization profile;
- CPU model, core count and operating system;
- benchmark command and Criterion configuration;
- fixed corpus seeds and benchmark source revision.

## Metrics

For closest image:

- p50/p95/p99 latency per query;
- candidate offsets examined (total, mean, p95 and maximum);
- agreement with the bounded brute-force oracle;
- ambiguous-image count and search-budget exhaustion;
- allocations and peak memory where measurable.

For AABB image candidates:

- p50/p95/p99 latency per pair;
- oracle overlap images;
- production candidates and extra candidates;
- candidate over-generation ratio;
- candidate-budget exhaustion;
- downstream narrow-phase calls implied by the candidate count.

## Fair comparison rules

- Production and brute-force paths use the same input corpus and same box geometry.
- Brute-force integer ranges are reported as a bounded reference domain; do not imply they represent an unlimited proof for arbitrary displacements.
- Correctness preflight runs before timing and is reported separately from Criterion timing.
- Do not report a speedup if any closest-image distance disagrees or any oracle AABB image is missing.
- Do not compare this microbenchmark directly with GROMACS/LAMMPS full-step throughput or CGAL triangulation throughput. They are semantic/reference sources, not equivalent workloads.

## Claim ceiling

The benchmark can establish a measured cost/over-generation tradeoff for the named 2D/3D cell families and bounded query corpora. It does not establish universal triclinic performance or full periodic rigid-body correctness.
