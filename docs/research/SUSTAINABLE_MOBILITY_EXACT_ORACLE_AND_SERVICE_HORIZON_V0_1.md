# Sustainable Mobility Exact Assignment and Service-Horizon Benchmark v0.1

**Parent:** [SUSTAINABLE-MOBILITY-SERVICE-QUALITY-001 #6921](https://github.com/Luminous-Dynamics/symtropy/issues/6921)  
**Capacity source:** [SUSTAINABLE-MOBILITY-SERVICE-CAPACITY-001 #6909](https://github.com/Luminous-Dynamics/symtropy/issues/6909)  
**Branch:** research/mobility-service-quality-exact-oracle-v0-1  
**Status:** bitmask dynamic-programming assignment oracle, horizon accounting and offline upper-bound gaps implemented; exact DP verified; the additional information-model metadata revision is pending its own exact-head run.

## 1. Why this increment matters

The linked service-quality evaluator already consumes event-derived aircraft opportunity receipts and compares FCFS with EDF. It does not, however, quantify the distance between either policy and the best assignment available under the *same* request stream, capacity receipts, weather windows, permission state and fallback resources.

The other important distinction is the evaluation cut-off. A job that has started but is not complete when the horizon closes cannot be counted as a completed late delivery. It must remain a separate in-progress outcome.

This increment adds:
1. an explicitly bounded exhaustive assignment reference for tiny instances;
2. policy-gap components against that reference;
3. an in-progress-at-horizon outcome with separate accounting from completed, late and unserved requests.

It does not introduce a new routing algorithm, battery model, flight controller, market or service authority.

## 2. Exact-oracle contract

The oracle consumes the same validated ServiceQualityInput produced by the linked receipt path. It considers each aerial/fallback opportunity at most once and each request at most once. For aerial opportunities it calls the same complete service-window, weather, permission and blocked-interval gate used by the policy evaluator. No unsafe or ineligible opportunity enters the exact comparison.

The admitted exact family is bounded to:
- at most 12 requests;
- at most 16 combined aerial and fallback opportunities;
- at most 1,000,000 dynamic-programming transitions.

The solver uses bitmask dynamic programming over the set of already assigned requests. For each ordered opportunity it considers skipping the opportunity or assigning one eligible, not-yet-assigned request. At each layer it retains the lexicographically best prefix for each request mask; because future feasibility depends only on the remaining request set and remaining opportunities, dominated prefixes for the same mask can be discarded. This expands the exactly solved family without enumerating every complete assignment permutation. It has three explicit outcomes:
- evaluated: the bounded search completed and returns an exact result for the supplied finite instance;
- not_run_bounds_exceeded: the request/opportunity count exceeds the declared family;
- transition_limit_reached: the explicit DP transition cap was reached before the exact layers were completed.

Only evaluated carries an optimum and policy-gap values. The other two states do not carry a best score and must never be presented as an optimality result.

### Lexicographic objective

1. Maximize completed on-time requests.
2. Maximize total completed-by-horizon requests (on-time + late).
3. Minimize cumulative lateness seconds across late completions.
4. Minimize the number of services still in progress at the horizon.
5. Minimize total request-to-completion latency among services actually completed within the horizon.
6. Resolve exact ties using a canonical opportunity-ID/request-ID assignment order.

This intentionally avoids a weighted composite score: a reduction in average latency cannot compensate for losing an on-time delivery, and an unfinished job cannot masquerade as a completed one.

The regret vector keeps components separate:
- on-time completion gap;
- completed-by-horizon gap;
- excess cumulative lateness;
- excess completed-request latency;
- excess in-progress assignments at horizon.

The regret components are diagnostics, not a single blended percentage.

## 2A. Information model: offline upper bound, not an online policy

The exact DP sees the complete frozen request stream and every event-derived opportunity in the horizon while selecting assignments. It enforces that a request cannot be assigned before its arrival and that each opportunity/request is used at most once, but it can still reserve earlier opportunities based on knowledge of later arrivals. Its report therefore carries information_model = `clairvoyant_full_horizon_offline_upper_bound` and deployable_online_policy = false.

The FCFS/EDF comparisons are component-wise distances to this offline upper bound. They must not be described as regret to an achievable online optimum. The value of perfect future information is part of the comparison, not hidden inside a policy-performance claim.

Follow-on [#7133 — nonanticipative scenario-tree oracle](https://github.com/Luminous-Dynamics/symtropy/issues/7133) will enforce identical decisions across scenarios sharing the same revealed history. Online FCFS/EDF, rolling-horizon MPC and ADP must then be compared against that information-constrained oracle; the clairvoyant DP remains a separate upper bound.

## 3. Frozen small-instance known answer

Fixture: tools/ev-passenger-cost-frontier/fixtures/sustainable_mobility_service_quality_exact_oracle_v0_1.json.

It contains two requests and two aerial opportunities derived from the fixed Dock-shaped capacity event simulation. The urgent request has a 400-second deadline; the standard request has a 4,000-second deadline. Both require 240 seconds of service. The first two event-derived service opportunities start at 0 and 345 seconds.

| Method | Opportunity 0 | Opportunity 1 | On time | Late | Cumulative lateness |
| --- | --- | --- | ---: | ---: | ---: |
| FCFS | Standard | Urgent | 1 | 1 | 185 s |
| EDF | Urgent | Standard | 2 | 0 | 0 s |
| Exact oracle | Urgent | Standard | 2 | 0 | 0 s |

The total completed-request latency for the exact/EDF outcome is 825 seconds. FCFS has the same total latency on this small instance, but misses one additional on-time deadline and accumulates 185 seconds of lateness. This illustrates why service latency, deadline success and service completion must remain separate.

The full linked benchmark fixture has 12 requests and 12 combined qualified service opportunities (7 aerial + 5 fallback). The DP completed in 768 transitions against its 1,000,000 cap. Its clairvoyant offline upper-bound score is 7 on-time + 1 late (8 completed), zero in-progress, 170 seconds cumulative lateness and 6,715 seconds total completed-request latency. The distances to that offline bound are:

| Policy | On-time gap | Completed gap | Lateness excess | Latency excess | In-progress excess |
| --- | ---: | ---: | ---: | ---: | ---: |
| FCFS | 1 | 0 | 185 s | 0 s | 0 |
| EDF | 0 | 0 | 0 s | 0 s | 0 |

Instances beyond 12 requests or 16 opportunities return not_run_bounds_exceeded; instances crossing the transition ceiling return transition_limit_reached and carry no optimum/regret.

## 4. Horizon semantics

A request that starts within the horizon but whose projected completion lies beyond the evaluation cut-off is:
- in_progress_at_horizon;
- not counted as completed on time;
- not counted as completed late;
- not counted as unserved;
- not included in completed-request latency percentiles or completed-requests/hour;
- reported with service start and projected completion, but no actual service-completion timestamp.

Fallback service windows may extend beyond the horizon so that the evaluator can represent an active job at cut-off. The evaluator must never infer that a projected completion actually occurred after the observation boundary.

The synthetic negative control starts a 600-second ground-service job at second 28,700 in a 28,800-second horizon. It reports a projected completion at second 29,300, one in-progress request, zero completed requests and zero completed-request latency.

## 5. Research and industry comparisons

### Online dynamic truck–drone service

Zhao, Zhang and Coelho (2026), Transportation Research Part E 214, Article 105074, “Dynamic truck-drone collaborative routing with stochastic requests under changing weather conditions,” formulate an MDP and approximate dynamic programming approach for dynamic request acceptance/assignment under changing wind. The paper reports up to 18.58% more accepted requests than weather-unaware baselines on its own instances and decision times below 0.2 seconds. It studies a single truck and one drone; its numbers are not direct results for this battery-depot benchmark.

The fair Luminous comparison is:
- reproduce the paper's native benchmark/data and request acceptance semantics;
- use the exact same request information-release timestamps for every policy;
- compare wind-aware ADP, FCFS, EDF and the bounded exact reference on tiny matched streams;
- report accepted, on-time, late, in-progress and unserved outcomes independently;
- enforce the same upstream source-derived service windows and hard permission/weather gates.

Source: https://doi.org/10.1016/j.tre.2026.105074

### Mobile battery-service network

Xu et al. (2026), Transportation Research Part D 158, Article 105458, model mobile battery-swapping vans, battery inventory, charging racks and closed-loop replenishment under demand uncertainty. The research direction is directly relevant to multi-site capacity and battery-service logistics, but is a road-EV architecture rather than a UAV service benchmark.

Source: https://doi.org/10.1016/j.trd.2026.105458

### Distributionally robust battery-swap infrastructure

The November 2026 commercial-EV facility-location/inventory study in Transportation Research Part C 192, Article 105855 combines fixed/mobile swapping, demand ambiguity, worst-case mean-CVaR and joint chance constraints with a MILP reformulation. It is a strong risk/inventory comparator. The appropriate Luminous extension is to pair its native planning objective with exact serialized-resource commitments and repeated-shock reserve recovery—not to compare paper-specific reliability percentages to the synthetic UAV fixture.

Source: https://doi.org/10.1016/j.trc.2026.105855

### Commercial UAV docks

DJI Dock 4 publishes <25-second cover-closed-to-swap-complete and <45-second landing-to-next-takeoff reference times, up to three stored batteries, two charging channels, and a 27-minute 15–95% charge at 25°C. The swap time, aircraft turnaround and recharge are distinct quantities. They should never be collapsed into “missions per hour” without a matched mission duration, inventory pool, charge curve, grid limit and operating horizon.

Source: https://enterprise.dji.com/dock-4/specs

## 6. Benchmark target ladder

Targets, not achievements:
- 100% agreement between exact oracle and independent reference enumeration for all admitted tiny instances;
- zero completed-service counts for jobs still in progress at the horizon;
- zero false aerial admission under unknown/stale permission or weather;
- zero future-information leakage;
- zero aircraft/opportunity double allocation;
- zero use of unqualified or invented service receipts;
- report FCFS/EDF regret vectors separately for on-time, completion, lateness, latency and in-progress outcomes;
- target mean relative regret <=1% and P95 <=5% against exact references on a held-out small-instance family, reported only where the exact oracle actually completed;
- separately report the percentage of large cases where exact evaluation is skipped or reaches the node cap;
- report evaluator runtime p50/p95/p99 and maximum search nodes on declared hardware.

The full matched tournament should add nonanticipative rolling-horizon MPC, scenario-tree recourse, stochastic/robust ADP and learned proposals. All methods must consume the same frozen information-release traces and identical source receipts. Symthaea-generated forecasts/actions remain proposals behind the same deterministic feasibility and evidence gates.

## 7. Claim boundary

A passing exact-oracle test demonstrates exhaustive optimality only for the declared finite synthetic input and the stated lexicographic objective. It does not prove field demand, aircraft reliability, real weather calibration, customer outcomes, flight legality, battery safety, grid interconnection, commercial economics or superiority over published ADP/RL methods. Exact-head Actions run [#45210669154](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45210669154) at `d70c8a0536bf0ae8219a2c664403126f84865e40` passed schema/fixture parsing, workspace formatting, Cargo tests, deterministic legacy/linked/full-exact/tiny-exact assertions and artifact upload for the 12×16 DP implementation. Test binaries reported 223 passed, 0 failed; the service-quality module had 30 focused Rust test definitions at that head. The later information-model metadata and explicit no-online-policy assertions are additional changes pending their own exact-head run. Clippy is not included, and physical/field evidence remains a separate qualification.
