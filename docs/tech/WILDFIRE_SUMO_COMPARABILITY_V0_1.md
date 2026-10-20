# SUMO Semantic Comparability Gate v0.1

**Benchmark:** `WILDFIRE-SUMO-COMPARABILITY-001`  
**Classification:** semantic eligibility gate; not an accuracy score

## Policy

Never calculate a combined cross-model error, rank or superiority claim for a case until the same modeled quantities and rules are enforced in both systems. The verifier emits one of two statuses:
- `comparable`: no applicable unclosed semantic gap remains and comparison is explicitly permitted.
- `non_comparable`: one or more blocking semantic differences remain; exclude the case from aggregate cross-model scorecards.

Each case includes machine-readable blocking reason codes and the matching description from the SUMO export manifest.

## Blocking reasons

- `EDGE_CAPACITY_STORAGE_UNCALIBRATED`: the source network model specifies vehicles-per-step edge capacities and finite storage, but the exporter has not validated a mapping to SUMO lane/speed flow and jam storage.
- `SINK_SERVICE_GATES_NOT_ENFORCED`: source sink ingress/total/accessibility constraints remain sidecar metadata, not simulation-enforced destination gates.
- `ACCESSIBILITY_LABEL_ONLY`: accessible vehicle type is metadata and does not constrain service/accessibility in SUMO.
- `CLOSURE_POLICY_DIFFERS`: used when the fixture contains a closure; `closingReroute` can redirect traffic, unlike the source benchmark's fixed-route entry-block semantics.

A required manifest gap declaration missing from the report causes the verifier to fail closed. It does not downgrade the issue to a warning.

## Current frozen-corpus result

The six v0.1 cases are all `non_comparable`. All six carry capacity/storage, sink-gate and accessibility blockers. The closure fixture additionally reports closure-policy divergence. The CI gate asserts these statuses so a future edit cannot silently turn a case into comparable by deleting or renaming a reason string.

## What will unlock comparison

1. Calibrate physical/queue mapping on separate, small single-link and two-link networks.
2. Implement or explicitly transform sink ingress, total service and accessibility gates so measured service has the same meaning in each model.
3. Reproduce fixed-route entry-block semantics for the fixed-route baseline; evaluate SUMO DUA separately as a different routing policy.
4. Test held-out fixtures with identical demand/horizon and exact vehicle inventory.
5. Only then report paired errors. Proposed target gates (not current achievements): 0 hard-invariant violations; ≤5% absolute relative error for throughput and mean travel time; ≤10% p95 trip-time error; exact agreement on exhaustive tiny cases.

A `non_comparable` result is a successful semantic-gate outcome, not a failed simulation. It is evidence that the comparison is not yet valid.
