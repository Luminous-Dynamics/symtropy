# Maritime qualification impact engine v0.1

**Programme:** Symtropy / Luminous Dynamics  
**Owner:** MARITIME-OBSERVABILITY-001 / #4609  
**Status:** synthetic executable contract; no real vessel qualification graph has been admitted  
**Version:** 0.1.0

## Purpose

The evidence validator admits a measurement/configuration report and exposes which inputs are eligible for independent engineering review. This companion engine answers a different question:

> When a qualified evidence artifact, configuration, calibration, timebase, model or software dependency changes, which prior qualifications are affected, and which remain unchanged?

The engine performs read-only dependency impact analysis. It does not mutate the manifest, revoke a qualification in an external system, issue work orders, authorize maintenance, change operating modes, clear restrictions or actuate machinery.

## Graph semantics

Each node is either:

- **evidence** — a versioned sensor topology, calibration, timebase, fault model, response model, software artifact or other source;
- **qualification** — a scoped engineering conclusion based on one or more evidence or qualification prerequisites.

An edge is oriented `dependent_id -> prerequisite_id`. The engine constructs the reverse adjacency map and propagates each changed prerequisite to all transitive dependents.

The graph must be acyclic. Duplicate node IDs, duplicate edges, unknown endpoints, invalid digests and change events whose previous digest does not match the manifest snapshot are rejected.

## Change-set semantics

A change event must bind:

- a unique change ID;
- the exact evidence node;
- the previous SHA-256 digest in the snapshot;
- a different new SHA-256 digest;
- an enumerated change class;
- an effective timestamp.

The engine accepts at most one change event per node in a change set. Concurrent changes to the same artifact must be reconciled into one authoritative event before evaluation. A digest mismatch is a stale precondition and fails closed; it is not guessed or overwritten.

## Output dispositions

- `REQUALIFICATION_REQUIRED` — a previously qualified node is transitively dependent on changed evidence.
- `ALREADY_NOT_QUALIFIED` — an impacted qualification was already expired, revoked or not evaluated.
- `UNCHANGED_BY_THIS_CHANGE_SET` — no path exists from any changed evidence node to the qualification.

A changed artifact's dependent qualifications are listed once, even when multiple changed paths converge. Existing qualification records are not overwritten. The receipt is an impact plan, not a revocation or authorization command.

## Synthetic fixture

The frozen fixture exercises two changes:

1. Sensor topology revision.
2. Timebase qualification revision.

Expected closure: five qualifications require requalification (timebase, observability, control-response, engine-health and RUL). Pressure calibration and an unrelated auxiliary alarm qualification remain unaffected. This proves the transitive graph behavior only; the IDs and digests are synthetic.

## Reproduction

```sh
cargo test --locked -p maritime-observability-evidence-validator
cargo clippy --locked -p maritime-observability-evidence-validator --all-targets -- -D warnings

source_commit="$(git rev-parse HEAD)"
manifest="benchmarks/maritime-observability/qualification-dependency-manifest.v0.1.json"
digest="$(sha256sum "$manifest" | awk '{print $1}')"
cargo run --quiet --locked -p maritime-observability-evidence-validator --bin qualification_impact -- \
  "$manifest" --source-commit "$source_commit" --change-set-sha256 "$digest"
```

The JSON receipt binds the exact source commit and change-set SHA-256. CI checks the expected five affected and two unaffected qualifications and uploads the receipt.

## Claim ceiling

The fixture demonstrates deterministic software dependency propagation for one synthetic graph. It does not show that any physical vessel, sensor topology, engine, calibration, clock or operational qualification is current or valid. Real use requires an independently reviewed configuration/dependency graph, authoritative artifact digests, change identity and owner-approved requalification rules.


## Path-explained impact receipts

Reachability counts alone do not explain why a qualification is affected. The evaluator now emits a deterministic impact_paths array. Each path binds the exact change ID and changed evidence node, target qualification ID, ordered node path from changed evidence to qualification, and every traversed edge including its dependency type (requires, uses, calibrated_against, qualified_against or derived_from).

All simple paths through the validated acyclic dependency graph are retained, so converging evidence changes and alternative dependency routes are not collapsed into one arbitrary explanation. Each qualification summary carries its own impact_path_count and sorted triggering_change_ids; unaffected qualifications must have zero paths. The synthetic fixture now has 12 source-to-qualification paths across five affected qualifications, while preserving five requalification-required and two unaffected qualification counts.

Path enumeration is bounded at 100,000 qualification paths, 250,000 traversal states and 512 dependency edges per explanation path. Exceeding any limit fails closed rather than silently truncating an explanation. The receipt reports the actual traversal-state count and maximum path depth. This is a deterministic explanation of the declared dependency graph, not proof that the graph itself is complete or physically correct.
