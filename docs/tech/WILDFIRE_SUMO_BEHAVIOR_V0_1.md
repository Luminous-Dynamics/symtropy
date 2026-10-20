# SUMO behavioral metrics contract v0.1

The behavioral benchmark should preserve SUMO-native observations without presenting them as an apples-to-apples comparison against Symtropy before the semantics are calibrated.

## Metrics

Pin SUMO and netconvert versions. Capture per-vehicle tripinfo with unfinished and undeparted records enabled, plus SUMO summary output. Reconcile every exported vehicle ID exactly once; reject missing, duplicate or unknown records. Report demand, departed, arrived, not-arrived-at-horizon and not-inserted counts. Compute nearest-rank p50/p90/p95 and mean duration over arrived vehicles only, plus waiting time and time loss where present. Preserve raw XML, stdout/stderr, fixture/source digests and output hashes.

## Comparability gates

Current synthetic edges derive length from target travel time and reference speed, while abstract per-step capacities, finite storage, shelter constraints and accessibility constraints are not enforced equivalently in SUMO. Closure rerouting also differs from fixed-route entry blocking. Therefore current results are external SUMO baselines, not matched comparisons, accuracy claims or SOTA wins.

Proposed targets (not achieved):
1. 100% vehicle-ID reconciliation and zero unknown/duplicate output records.
2. Zero hard-constraint and mass-conservation violations within Symtropy's model.
3. For paired simulator calibration, only label a scenario comparable after preregistered fixed-route checks pass: <=5% per-link entry/exit flow error and <=10% queue/storage-onset error on at least 100 held-out configurations. Otherwise report not comparable and publish the mismatch.
4. For field-data validation, report both localized bottleneck dynamics and system/path travel times; pre-register metrics, field-day variation envelopes, tolerances and confidence levels. FHWA-style flow/travel-time measures may inform the protocol, but criteria must be justified for wildfire conditions and cannot be substituted for evacuation-specific validation.
5. Compare static shortest-path, exact tiny-instance joint assignment, a scalable planner and SUMO Dynamic User Assignment under paired scenarios and comparable compute budgets where semantics permit. Aim for >=10% improvement in p95 clearance or predeclared evacuation regret, with no increase in unserved demand and zero hard-constraint violations; require confidence intervals and held-out cases.
6. Publish runtime/memory/quality curves by network size and demand before setting performance budgets.

## Industry and research baseline

FHWA microsimulation guidance says calibration should include at least one localized bottleneck measure (for example, queue/throughput) and one system-level measure (for example, path travel time); dynamic conditions and a representative-day approach matter. An operational validation programme should use relevant field observations, variation envelopes and confidence/tolerance criteria rather than tune to one aggregate score. See https://ops.fhwa.dot.gov/publications/fhwahop18036/chapter5.htm and https://ops.fhwa.dot.gov/publications/fhwahop18036/chapter2.htm.

For traffic-flow field calibration, compare the chosen local and system measures using a documented public protocol; one FHWA traffic-calibration reference describes link-flow GEH and journey-time tolerances. Treat it as context to tailor to wildfire evacuation, not as a claim that generic roadway criteria automatically validate evacuation safety: https://ops.fhwa.dot.gov/trafficanalysistools/tat_vol4/sec5.htm.

A relevant current research comparator is Moreno et al. (Transportation Research Part E, September 2026): a multi-period Cell Transmission Model with Branch-and-Benders-Cut jointly considers evacuee flows, shelter locations and contraflow, and includes a case study grounded in the 2023 Chibougamau wildfire evacuation: https://www.sciencedirect.com/science/article/pii/S1366554526003042. Our current work is not comparable to that full optimization system; it presently has a small fixed-route flow simulator and exact joint-assignment oracle only for tiny instances. The credible path to competition is to extend the benchmark across coupled shelter/contraflow decisions, scalable assignment, uncertain closures/demand, and evidence-bounded online replanning, then publish held-out quality/runtime curves.

SUMO references:
- TripInfo output and unfinished/undeparted flags: https://eclipse.dev/sumo/docs/Simulation/Output/TripInfo.html
- Summary output counts: https://sumo.dlr.de/docs/Simulation/Output/Summary.html
- Dynamic user assignment: https://sumo.dlr.de/docs/Demand/Dynamic_User_Assignment.html
