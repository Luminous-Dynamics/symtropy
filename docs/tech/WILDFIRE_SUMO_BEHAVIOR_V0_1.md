# SUMO behavioral metrics contract v0.1

The behavioral benchmark should preserve SUMO-native observations without presenting them as an apples-to-apples comparison against Symtropy before the semantics are calibrated.

## Metrics

Pin SUMO and netconvert versions. Capture per-vehicle tripinfo with unfinished and undeparted records enabled, plus SUMO summary output. Reconcile every exported vehicle ID exactly once; reject missing, duplicate or unknown records. Report demand, departed, arrived, not-arrived-at-horizon and not-inserted counts. Compute nearest-rank p50/p90/p95 and mean duration over arrived vehicles only, plus waiting time and time loss where present. Preserve raw XML, stdout/stderr, fixture/source digests and output hashes.

## Comparability gates

Current synthetic edges derive length from target travel time and reference speed, while abstract per-step capacities, finite storage, shelter constraints and accessibility constraints are not enforced equivalently in SUMO. Closure rerouting also differs from fixed-route entry blocking. Therefore current results are external SUMO baselines, not matched comparisons, accuracy claims or SOTA wins.

Proposed targets (not achieved):
1. 100% vehicle-ID reconciliation and zero unknown/duplicate output records.
2. Zero hard-constraint and mass-conservation violations within Symtropy's model.
3. Only label a scenario comparable after preregistered fixed-route calibration passes: <=5% per-link flow error and <=10% queue/storage-onset error on at least 100 held-out configurations. Otherwise report not comparable with the mismatch.
4. Compare static shortest-path, exact tiny-instance joint assignment, scalable planner and SUMO Dynamic User Assignment under paired scenarios and comparable compute budgets where semantics permit. Aim for >=10% improvement in p95 clearance or predeclared evacuation regret, with no increase in unserved demand and zero hard-constraint violations; require confidence intervals and held-out cases.
5. Publish runtime/memory/quality curves by network size and demand before setting performance budgets.

SUMO references:
- TripInfo output and unfinished/undeparted flags: https://eclipse.dev/sumo/docs/Simulation/Output/TripInfo.html
- Summary output counts: https://sumo.dlr.de/docs/Simulation/Output/Summary.html
- Dynamic user assignment: https://sumo.dlr.de/docs/Demand/Dynamic_User_Assignment.html
