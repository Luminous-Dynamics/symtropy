# CCD Query Recall v0.1

A dependency-free evaluator over imported NYU CCD query records. It keeps query-level geometric ground truth separate from broadphase box-pair and rigid-body collider-pair metrics.

## Why this is a separate denominator

The NYU sample dataset provides one oracle Boolean per eight-row edge-edge or vertex-face query, plus a separate array of broadphase box-ID intersections. These are different populations and cannot be joined by row order. Symtropy body/collider pair IDs are a third identity space. Any mapping among them must be explicit and versioned.

## Evaluated layers

Candidate layer:
- Forwarded and fallback queries reach narrowphase.
- A certified-pruned query must carry certificate digest, frozen-input digest, verifier identity and explicit accepted-and-replayed status.
- Candidate recall, precision, false-positive rate, false-negative count and fallback rate are computed only over query labels. Scores are stratified by dataset, scene, time step and query class; aggregate metrics never replace per-stratum results. Results are stratified by dataset, scene, time step and query class; the aggregate score never replaces the per-stratum rows.

Narrowphase layer:
- Each forwarded/fallback query must record collision, no collision or indeterminate.
- Candidate omissions and narrowphase misses are counted separately.
- An indeterminate positive is not silently credited as a true positive.
- Zero-positive datasets return unavailable recall, never 100%.

## Integration

The library depends on nyu-ccd-query-import and constructs query identity from dataset, scene, time step, query class and query index. Ground truth must already be attached to the imported query. It does not infer oracle labels from the tested algorithm.

The evaluator does not count broadphase box pairs, map query indices to collider pairs, or prove a pruning certificate. The verifier receipt is a declared input and must come from the independently qualified checker (#7307). The score is not a cryptographic attestation. Safe negative certificate-backed prunes can qualify even though their narrowphase is correctly not run; oracle-positive prunes remain explicit candidate and narrowphase false negatives. Safe negative certificate-backed prunes are counted as certified prunes and are allowed to qualify even though their narrowphase is correctly not run; oracle-positive prunes remain explicit candidate and narrowphase false negatives.

## Run

```sh
cargo fmt --manifest-path research/ccd-query-recall/v0.1/Cargo.toml -- --check
cargo test --manifest-path research/ccd-query-recall/v0.1/Cargo.toml --locked
cargo clippy --manifest-path research/ccd-query-recall/v0.1/Cargo.toml --locked --all-targets -- -D warnings
```

## Promotion targets (not achieved results)

- 100% candidate recall and 100% narrowphase recall on oracle-positive queries within each separately named motion/precision stratum.
- Zero invalid, stale or unreplayed certificate-backed prunes.
- Candidate false-positive and fallback rates reported separately; no false-negative tradeoff.
- Query IDs survive filtering, sorting, batching and parallel scheduling.
- Join to broadphase/body-collider pair counts only through a reviewed explicit mapping with its own completeness denominator.
