# SUMO Outcome Metrics for Wildfire Network Fixtures v0.1

**Benchmark:** `WILDFIRE-SUMO-OUTCOMES-001`  
**Implementation:** `verify_sumo_export` in the Rust-first wildfire benchmark  
**Classification:** synthetic-only, SUMO-side outcome diagnostics

## Change in evaluation method

The structural SUMO adapter now captures quantitative outcomes instead of treating a successful simulator exit as sufficient evidence. The runner uses pinned SUMO 1.27.1 and enables both `--tripinfo-output.write-unfinished true` and `--tripinfo-output.write-undeparted true`. SUMO's default tripinfo output only emits records for trips that have arrived; these options make not-yet-arrived and not-yet-inserted planned vehicles visible at the simulation horizon. See the official [TripInfo documentation](https://sumo.dlr.de/docs/Simulation/Output/TripInfo.html).

The Rust verifier independently checks the expected vehicle IDs against tripinfo IDs, rejecting duplicate, missing, or unplanned vehicles. A receipt cannot pass if even one planned vehicle is absent from the outcome records.

## Metrics emitted

For each of the six frozen synthetic cases, the receipt records:
- planned, recorded, arrived-by-horizon and not-arrived-by-horizon vehicle counts;
- arrival fraction (this does **not** mean shelter-served fraction);
- nearest-rank p50, p90, p95 and maximum plus mean for completed-trip duration, waiting time, time loss, departure delay and route length;
- metric sample sizes, source revision, fixture digest and pinned SUMO version.

Quantiles are calculated only over trips with non-negative arrival times. Vehicles with negative arrival times remain in the denominator for arrival fraction but do not contaminate completed-trip duration quantiles. An absent completed-trip distribution is represented as null rather than zero.

## What this proves—and what it does not

This improves *observability* and output accounting. It does not establish parity with Symtropy's discrete network queue model. Existing non-equivalences remain: source capacities/storage are not calibrated to lane/speed physics, sink ingress and accessibility constraints are sidecar metadata, and SUMO closure rerouting differs from fixed-route entry blocking. SUMO arrival is also not the same as admissible shelter service.

Do not label these metrics "unserved evacuees", "clearance time", or "safety" without mapping them to the relevant real-world entities and checking the semantic contract. At present, label them exactly as SUMO-side outcomes on synthetic test networks.

## Benchmark gates

Gates that can be required now:
1. 100% of planned synthetic vehicle IDs have exactly one tripinfo record.
2. No duplicate or unexpected vehicle IDs.
3. Arrival and non-arrival counts partition the full planned demand.
4. Completed-trip numeric measurements are finite and non-negative.
5. All reported metrics pin the source commit, fixture digest, and SUMO version.

Targets for a future *matched* model comparison (not yet demonstrated):
- 0 hard-invariant violations in both systems.
- Exact vehicle inventory and closure/departure interpretation for the frozen cases, or a recorded non-equivalence result.
- After flow/storage calibration on held-out test networks, absolute relative error targets of <=5% for total completed throughput and mean travel time, and <=10% for p95 travel time.
- Exact outcome agreement on finite tiny instances where both implementations can be checked against exhaustive enumeration.
- No superiority claim unless both systems receive the same graph, demand, horizon, closure policy, sink constraints, tuning budget and evaluation corpus, with cases and thresholds preregistered before measuring.

Targets are acceptance criteria for future work, not results from this PR. Keep this PR draft until a reviewer confirms actual SUMO outputs and their semantics.

## Comparability gate: machine-readable and fail-closed

The verifier now emits `sumo-comparability-receipt.json` with a per-case `comparable` or `non_comparable` decision and blocking reason codes. In v0.1 all six cases must remain `non_comparable` because the adapter has not calibrated edge capacity/storage, does not enforce sink service constraints, and represents accessibility as a label. The closure case additionally reports the mismatch between SUMO rerouting and fixed-route entry blocking.

The workflow asserts that no case is admitted to aggregate cross-model scorecards. This gate is not meant to remain permanently pessimistic: a future implementation can close each gap and update the frozen contract only with tests that demonstrate the mapping. Until then, matching hashes and successful execution do not authorize a behavioral comparison.
