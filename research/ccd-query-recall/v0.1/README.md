# CCD Query Recall v0.1

A dependency-free evaluator over imported NYU CCD query records. It keeps query-level geometric ground truth separate from broadphase box-pair and rigid-body collider-pair metrics.

## Why this is a separate denominator

The NYU sample dataset provides one oracle Boolean per eight-row edge-edge or vertex-face query, plus a separate array of broadphase box-ID intersections. These are different populations and cannot be joined by row order. Symtropy body/collider pair IDs are a third identity space. Any mapping among them must be explicit and versioned.

## Evaluated layers

Candidate layer:
- Forwarded and fallback queries reach narrowphase.
- A certified-pruned query must carry certificate digest, frozen-input digest, verifier identity and explicit accepted-and-replayed status.
- Candidate recall, precision, false-positive rate, false-negative count and fallback rate are computed only over query labels. Narrowphase recall is conditional on oracle-positive queries actually forwarded/fallen back to narrowphase; a candidate-pruned positive is not double-counted as a narrowphase miss. End-to-end recall uses all oracle-positive queries. Scores are stratified by dataset, scene, time step and query class; aggregate metrics never replace per-stratum results. Results are stratified by dataset, scene, time step and query class; the aggregate score never replaces the per-stratum rows.

Narrowphase layer:
- Each forwarded/fallback query must record collision, no collision or indeterminate.
- Candidate omissions and narrowphase misses are counted separately; end-to-end recall exposes their combined effect.
- An indeterminate positive is not silently credited as a true positive.
- Zero-positive datasets return unavailable recall, never 100%.

## Integration

The library depends on nyu-ccd-query-import and constructs query identity from dataset, scene, time step, query class and query index. The import adapter joins algorithm results by that identity, rejects duplicates, missing results and orphan rows, and never joins by array position. Ground truth must already be attached to the imported query; it is never accepted from the tested algorithm output.

The evaluator does not count broadphase box pairs, map query indices to collider pairs, or prove a pruning certificate. The verifier receipt is a declared input and must come from the independently qualified checker (#7307). The score is not a cryptographic attestation.

Each scorecard carries immutable run provenance: algorithm name/version/commit, dataset-manifest SHA-256, configuration SHA-256, motion profile and arithmetic profile (for example, linearized/binary64 versus Symtropy semi-implicit/Q16.16). The convenience evaluators return an unspecified profile and are not benchmark-qualified. Scorecards may be pooled only when all provenance and model fields are explicit and identical; runs from different algorithms, commits, datasets, configurations or precision/motion models stay separate. A zero-oracle-positive population also cannot claim benchmark recall. Safe negative certificate-backed prunes can qualify even though their narrowphase is correctly not run; oracle-positive prunes remain explicit candidate and narrowphase false negatives. Safe negative certificate-backed prunes are counted as certified prunes and are allowed to qualify even though their narrowphase is correctly not run; oracle-positive prunes remain explicit candidate and narrowphase false negatives.

## Per-query result interchange

Adapters write UTF-8 TSV with an optional first line `#ccd-query-run-results-v1`. Every data row has exactly twelve fields:

| Index | Field | Allowed values |
| ---: | --- | --- |
| 1 | Dataset ID | lowercase letters, digits, `-`, `_`, `.` |
| 2 | Scene ID | same token grammar |
| 3 | Time step | unsigned integer |
| 4 | Query class | `edge_edge`, `vertex_face` |
| 5 | Query index | unsigned integer |
| 6 | Candidate disposition | `forwarded`, `fallback`, `certified_pruned` |
| 7 | Fallback reason | blank except for fallback rows; one of the typed reason codes |
| 8 | Certificate SHA-256 | required for certified prune rows |
| 9 | Frozen-input SHA-256 | required for certified prune rows |
| 10 | Verifier ID | required for certified prune rows |
| 11 | Verification status | `accepted_and_replayed`, `rejected`, `not_run` |
| 12 | Narrowphase outcome | `collision`, `no_collision`, `indeterminate`, `not_run` |

Forwarded rows must leave fields 7-11 blank. Fallback rows require field 7 and leave fields 8-11 blank. Certified-pruned rows require fields 8-11 and leave field 7 blank. Oracle labels are never included in adapter output. The evaluator joins TSV rows to the independent imported query set by dataset/scene/time-step/query-kind/query-index, then rejects duplicate, missing and orphan rows.

Example rows (tabs separate fields):

```text
#ccd-query-run-results-v1
nyu-fixture-v1	armadillo-rollers	3	edge_edge	0	forwarded						no_collision
nyu-fixture-v1	armadillo-rollers	3	edge_edge	1	fallback	unsupported_motion					no_collision
```

A production benchmark run must provide an explicit `RunProfile` with algorithm name/version/commit, dataset-manifest SHA-256, canonical configuration SHA-256, motion profile and arithmetic profile. The convenience evaluator returns an unspecified profile and cannot qualify or be pooled. Only scorecards with identical explicit run provenance may be pooled; different algorithms and numeric models remain separate.

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
