# CCD Query Recall v0.1

A dependency-light evaluator for query-level continuous-collision-detection results. It keeps geometric query ground truth separate from broadphase box-pair and rigid-body collider-pair accounting.

## Benchmark populations and identities

The NYU scalable corpus has at least two distinct populations: eight-row geometric queries with one independent Boolean label per query, and a separate list of broadphase box-ID intersections. Symtropy body/collider pairs form a third identity space. These populations must never share a denominator or be joined by array position.

A query identity is the tuple:

`(dataset, scene, time_step, query_class, query_index)`

The algorithm adapter emits only that identity and its result. The oracle label is attached from the independently imported dataset. The join rejects duplicate, missing and orphan algorithm rows. Query IDs therefore survive sorting, batching and parallel execution.

## Evaluated correctness layers

- **Candidate recall** counts oracle-positive queries forwarded or sent through reason-coded fallback. Certified-pruned positives are candidate false negatives.
- **Conditional narrowphase recall** uses only oracle-positive queries that actually reached narrowphase. A candidate-pruned positive is not double-counted as a narrowphase miss.
- **End-to-end recall** uses every oracle-positive query, exposing the combined result of candidate culling and narrowphase.
- Candidate precision, false-positive rate, false-negative counts, fallback counts and certified-prune counts are reported separately.
- Narrowphase outcomes are collision, no collision, indeterminate or not run. An indeterminate positive is not credited as detected.
- Zero oracle-positive denominators produce unavailable recall and cannot qualify as a benchmark success.
- Results are reported in per-dataset, scene, time-step and query-kind strata as well as an aggregate. The aggregate never replaces the stratum rows.

A certified prune must carry a certificate SHA-256, frozen-input SHA-256, verifier identity and accepted-and-replayed status. This evaluator validates the presence and shape of that receipt; it does not execute the verifier or cryptographically attest that the digest came from the correct checker. Receipt production and independent replay belong to the qualified checker (#7307).

## Reproducible run provenance

Every benchmark-qualified scorecard is bound to all of the following:

- Algorithm name, version and exact source commit.
- Dataset-manifest SHA-256.
- Canonical configuration SHA-256, covering method parameters and effective runtime options.
- Environment SHA-256.
- Measured timer-resolution nanoseconds.
- Motion profile and arithmetic profile.

Scorecards may be pooled only if every provenance field is explicit and identical. Different algorithms, commits, datasets, configurations, environments, motion models or arithmetic models remain separate. The convenience evaluator returns an unspecified profile for unit tests and is not benchmark-qualified.

## Per-query result interchange

Adapters write UTF-8 tab-separated values, with an optional first line `#ccd-query-run-results-v1`. Every data row has exactly thirteen fields:

| # | Field | Allowed values |
| ---: | --- | --- |
| 1 | Dataset ID | lowercase letters, digits, `-`, `_`, `.` |
| 2 | Scene ID | same token grammar |
| 3 | Time step | unsigned integer |
| 4 | Query class | `edge_edge`, `vertex_face` |
| 5 | Query index | unsigned integer |
| 6 | Candidate disposition | `forwarded`, `fallback`, `certified_pruned` |
| 7 | Fallback reason | blank except for fallback rows; one typed reason code |
| 8 | Certificate SHA-256 | required for certified-pruned rows |
| 9 | Frozen-input SHA-256 | required for certified-pruned rows |
| 10 | Verifier ID | required for certified-pruned rows |
| 11 | Verification status | `accepted_and_replayed`, `rejected`, `not_run` |
| 12 | Narrowphase outcome | `collision`, `no_collision`, `indeterminate`, `not_run` |
| 13 | Narrowphase elapsed time | integer nanoseconds if run; blank if not run |

Forwarded rows leave fields 7-11 blank. Fallback rows require field 7 and leave fields 8-11 blank. Certified-pruned rows leave field 7 blank and require fields 8-11. Oracle labels are never included in adapter output.

Forwarded and fallback rows must include an elapsed-time observation. Zero is permitted and means the observation fell below the timer's resolution; it is not replaced with a fabricated 1 ns value. A certified-pruned row must have `not_run` and a blank elapsed time. The scorecard reports nearest-rank p50/p95/p99 over actual narrowphase durations, globally and per stratum, plus the count of samples below the declared timer resolution. Correctness qualification and latency comparability are separate: a run can qualify on correctness while its clock is too coarse to support a latency ranking.

The official CCD-Wrapper reports elapsed microseconds, but its underlying timer is platform-specific (Linux uses `gettimeofday`; macOS and Windows use their high-resolution counters). Convert the reported value to integer nanoseconds by multiplying by 1,000, and record the actual measured timer resolution per environment; do not assume all operating systems provide 1 μs resolution.

Example rows (tabs separate fields):

```text
#ccd-query-run-results-v1
nyu-fixture-v1	armadillo-rollers	3	edge_edge	0	forwarded						collision	125000
nyu-fixture-v1	armadillo-rollers	3	edge_edge	1	fallback	unsupported_motion					no_collision	300000
```

## Included fixtures

### NYU upstream import smoke test

The imported `armadillo-rollers/3ee` sample checks two edge-edge queries, both labelled negative. It verifies that source coordinates remain exact rational strings and that the separate Boolean and broadphase-box files are parsed with distinct denominators. It is an import smoke test, not a recall benchmark.

### Original exact-rational known-answer corpus

`fixtures/analytic-linear-v0.1/` contains two edge-edge and two vertex-face queries. Each kind has one collision and one miss. The two positive queries have exact rational time of impact (t=1/2); the negative cases are separated by segment range or triangle barycentric bounds. The construction and proofs are described in `CONSTRUCTION.md`, and the manifest binds every fixture with SHA-256.

This is a small, original unit-test corpus for testing positive recall, negative pruning, query identity and timing. It is not an external benchmark and must not be represented as SOTA evidence.

## Current scope and claim boundary

This crate evaluates independent Boolean ground truth, candidate dispositions, narrowphase classification, stable identity joins and narrowphase latency. It does not compute collision geometry, derive TOI roots, score TOI interval quality, count broadphase box pairs or prove prune certificates. Exact TOI and broader broadphase metrics require their own evidence-carrying scorecards and denominators.

## Run

```sh
cargo fmt --manifest-path research/ccd-query-recall/v0.1/Cargo.toml -- --check
cargo test --manifest-path research/ccd-query-recall/v0.1/Cargo.toml --locked
cargo clippy --manifest-path research/ccd-query-recall/v0.1/Cargo.toml --locked --all-targets -- -D warnings
```

## Promotion targets (aspirational, not achieved)

- 100% candidate recall and 100% conditional narrowphase recall on oracle-positive queries in every supported motion/precision stratum.
- Zero invalid, stale or unreplayed certificate-backed prunes.
- Complete per-query timing coverage for every forwarded/fallback query.
- Candidate false-positive and fallback rates minimized only after correctness is preserved.
- Pareto improvement in narrowphase work and p50/p95/p99 runtime on held-out scenes, with no false-negative tradeoff.
- Query-to-broadphase-pair mapping only through an independently reviewed identity adapter with its own completeness denominator.
