# Triclinic AABB Image Candidate Benchmark

**Status:** benchmark authored; timings not yet collected.  
**Command:** cargo bench -p symtropy-physics --bench triclinic_aabb_candidates

## Compared algorithms

- Projection/range path: maps the eight (D=3) or four (D=2) corners of the Cartesian AABB Minkowski difference into lattice-fractional coordinates, forms conservative integer coordinate intervals, enumerates their Cartesian product, and filters with exact translated Cartesian AABB overlap.
- Brute-force reference: explicitly translates the second AABB by every lattice offset in a fixed integer cube and tests overlap directly.

The brute-force cube is only a reference for the frozen query corpus. It is not a general production algorithm.

## Workloads

- 2D moderate-skew and stronger-skew cells, at 64, 256 and 1,024 AABB pairs.
- 3D moderate-skew cell, at 32, 64 and 128 AABB pairs.
- Fixed deterministic LCG seeds and generated AABB extents.
- An untimed oracle-inclusion preflight aborts before timing if any brute-force overlapping image is absent from the projection candidate set.

The preflight emits TRICLINIC_AABB_PREFLIGHT,v1 records with dimension, basis label, pair count, oracle overlap count, production candidate count and conservative extra count.

## Metrics

Record:
- p50/median Criterion estimate and confidence interval;
- p95/p99 latency from a separate per-query harness if needed;
- true overlaps, returned candidates, extra candidates and overgeneration ratio;
- projection/candidate-box sizes, candidate-budget and numeric failures;
- exact source SHA, Cargo.lock digest, Rust/Cargo versions, compiler flags, CPU, OS, seed and workload generator.

The timed projection and brute-force paths both include query result allocation/enumeration. Preflight is untimed.

## Acceptance

- Every true image in the independent brute-force window is present before timing.
- Conservative extras are allowed and counted; no missed image is allowed.
- Candidate budget exhaustion is visible and never silently truncates.
- Any claimed speedup is limited to the named lattice/basis/size envelope and is not presented as full PhysicsWorld speedup.
- No timing result is claimed until the benchmark has run on a recorded machine/toolchain.

## Scope

This is an orthogonal-AABB / skew-lattice candidate-generation microbenchmark only. It does not benchmark convex GJK/EPA, swept AABB/CCD, contact solving, friction/angular response, LBVH integration or end-to-end simulation.

## Industry context

Triclinic cells and periodic images are established in scientific simulation systems. GROMACS and LAMMPS are useful comparators for cell semantics; CGAL is a useful comparator for periodic image geometry. They are not direct like-for-like baselines for this AABB query.

- GROMACS: https://manual.gromacs.org/documentation/current/reference-manual/algorithms/periodic-boundary-conditions.html
- LAMMPS: https://docs.lammps.org/stable/Howto_triclinic.html
- CGAL: https://doc.cgal.org/latest/Periodic_3_triangulation_3/index.html
