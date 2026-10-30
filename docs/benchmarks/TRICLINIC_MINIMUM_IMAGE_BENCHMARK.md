# Triclinic Minimum-Image Benchmark

**Status:** authored Criterion benchmark; no measurements collected.  
**Command:** cargo bench -p symtropy-physics --bench triclinic_minimum_image  
**Implementation:** [triclinic periodic lattice PR #2957](https://github.com/Luminous-Dynamics/symtropy/pull/2957)

## Comparison

Compare the certified shell-enumeration minimum-image solver with a separate bounded brute-force integer-offset oracle. The brute-force oracle directly visits a fixed integer cube around rounded fractional coordinates, evaluates Cartesian residuals, and takes the smallest norm. It does not call the production shell enumerator.

The oracle radius is ±12 per periodic coordinate, enough for the frozen 2D and 3D fixture envelope but not a general correctness proof. Every fixture is checked before timing. If the certified result differs from the oracle by more than 1e-9, or its unsearched lower-bound certificate is not greater than its returned distance, the preflight aborts.

## Workloads

- 2D basis: columns (10,0) and (8.5,3), 64/256/1,024 queries.
- 3D basis: columns (10,0,0), (2,9,0), (-1,1,8), 16/64/128 queries.
- One quarter of queries are placed near fractional half-cell boundaries; the remainder are deterministic seeded samples spanning ±10 fractional cells.
- Candidate budget: 729.

Each preflight emits TRICLINIC_CVP_PREFLIGHT,v3 with fixture, query count, mean/max candidate count, maximum brute-force distance error, maximum selected-distance roundoff allowance, inverse-residual upper bound, count of numerically ambiguous queries, maximum ambiguity-set size, and oracle search radius. A preflight fails before timing if the independent oracle finds a strictly closer image outside the returned representative/ambiguity set or if the unsearched-image lower bound does not exceed the best candidate-distance upper bound.

## Reporting

Record exact source SHA, Cargo.lock digest, Rust/Cargo version, CPU/platform, target triple, optimization flags, Criterion raw outputs, and all TRICLINIC_CVP_PREFLIGHT,v3 records. Report median/intervals from Criterion; p95/p99 require a separate per-query latency harness. Always report certification candidate counts, inverse residual, distance error allowance, ambiguity rate/set size, and refusal rate beside time.

The result is a microbenchmark of this particular basis/displacement distribution. It does not establish arbitrary-cell performance or full collision-engine speedup.

## Acceptance

- The independent preflight matches the bounded oracle on every case.
- Every returned nearest image carries a successful lower-bound certificate.
- Ill-conditioned cases outside the candidate budget are reported as explicit failures, not excluded from aggregate success rates.
- Do not compare only with component-wise rounding: it is an invalid nearest-image algorithm for general skew cells and is retained only as a diagnostic if separately labelled.
