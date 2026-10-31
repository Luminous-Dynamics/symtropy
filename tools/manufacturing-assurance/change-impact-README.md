# Manufacturing assurance: lifecycle change impact (v0.1)

This is a dependency-free Rust reference that determines which downstream manufacturing evidence and claims require requalification after a declared item revision change.

It complements the composition verifier and causal benchmark. It does not create a new registry, digital-twin runtime, VVUQ engine, measurement authority, quality authority or production control path.

## Build and run

```sh
rustc --edition=2021 -D warnings --test tools/manufacturing-assurance/change_impact.rs -o /tmp/mfg-change-tests
/tmp/mfg-change-tests
rustc --edition=2021 -D warnings tools/manufacturing-assurance/change_impact.rs -o /tmp/mfg-change
/tmp/mfg-change tools/manufacturing-assurance/fixtures/valid-change-impact.tsv
```

Or run `bash tools/manufacturing-assurance/run-tests.sh`.

## Manifest format

- `ITEM|id|kind|scope|revision|sha256|valid_from_utc|valid_until_utc`
- `CLAIM|id|revision|scope|valid_from_utc|valid_until_utc|claim_class|current_status`
- `DEP|upstream_id|upstream_revision|downstream_id`
- `CHANGE|change_id|target_id|old_revision|new_revision|old_sha256|new_sha256|change_class|scope|effective_from_utc|effective_until_utc|semantic_change|receipt_ref`

The format is pipe-delimited and does not support embedded pipes or escape sequences. UTC intervals are checked as real calendar timestamps, not merely lexical strings. Every dependency binds an exact upstream revision. Cycles, unknown references, duplicate identities, ambiguous revisions and unsupported change classes fail closed.

## Change semantics

Supported change classes: `SUBSTANTIVE`, `SCHEMA`, `CALIBRATION`, `MATERIAL`, `ENVIRONMENT`, `PROCESS`, `METADATA_ONLY`.

A substantive change requires a new revision, new digest and an explicit semantic-change declaration. Change class is checked against the target item's declared kind. A change to a GLOBAL item must be declared GLOBAL; a local change must match the target item's scope.

The sole no-op is an explicit `METADATA_ONLY` revision change with identical content digest and `semantic_change=FALSE`. Identical hashes do not excuse an unexplained semantic version change; such a change is rejected rather than silently treated as a no-op.

A change propagates only through explicit, revision-bound dependency edges. Downstream items and claims must be in the affected scope and have validity intervals overlapping the change's effective interval. A GLOBAL change affects every scope; a local change affects its matching scope and explicitly GLOBAL claims/items. Historical claims whose validity ended before the change took effect remain unchanged.

The report emits direct change identity, affected downstream evidence item IDs, affected claim IDs, a deterministic shortest witness dependency path and the number of dependency paths. Multiple changes produce a deduplicated union. Requalification is required; this tool does not perform requalification or restore qualification status.

## Frozen synthetic fixture and tests

`fixtures/valid-change-impact.tsv` contains:
- a shared physics model used on two production lines;
- line-specific simulation and conformity receipts;
- a line-A calibration → measurement → uncertainty → conformity path;
- a current line-A and line-B claim plus an expired historical line-A claim;
- a GLOBAL physics change and its source receipt reference.

The 21 unit tests cover global and scoped propagation, historical preservation, exact old-revision binding, metadata-only no-op semantics, malformed metadata changes, unknown targets/dependencies, cycles, missing claim dependencies, change-class/kind compatibility, validity overlap, multiple dependency paths, reused receipts, claim scope, determinism, multi-change deduplication and a 10,000-item dependency chain.

## Initial scale benchmark\n\nThe in-test 10,000-item chain checks exact affected-set size and deterministic path propagation. The qualification programme should benchmark 100 / 1,000 / 10,000 / 100,000 items and multiple-change workloads, reporting wall time, memory, path count saturation and independent-oracle agreement. No runtime or memory performance result is claimed until the Rust test and benchmark are executed.\n\n## Claim ceiling

This reports declared dependency impact only. It does not authenticate change receipts, hash artifact bytes, detect physical drift, establish that a change was actually applied, authorize production changes, verify the correctness of a manufacturing model or perform VVUQ. An impacted claim returns to a qualified state only through the existing owners and a new exact-revision qualification receipt.

The initial fixture is synthetic-only; no external factory evidence is admitted.
