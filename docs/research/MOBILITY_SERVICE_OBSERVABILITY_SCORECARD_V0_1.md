# Mobility Service Observability Scorecard v0.1

**Issue:** [MOBILITY-SERVICE-OBSERVABILITY-001 #6850](https://github.com/Luminous-Dynamics/symtropy/issues/6850)  
**Industry baseline:** [#6226](https://github.com/Luminous-Dynamics/symtropy/issues/6226)  
**Matched tournament:** [#6247](https://github.com/Luminous-Dynamics/symtropy/issues/6247)  
**Branch:** `research/mobility-service-observability-v0-1`  
**Status:** bounded Rust reducer, strict schema and synthetic fixture; exact-head runner verification pending.

## Purpose

The scorecard computes descriptive operational metrics without treating a fast local operation as a proxy for network-level service. It requires timestamped, configuration-bound observations and preserves event denominators.

It is deliberately not a simulator, dispatch optimizer, vendor-audit tool or physical controller. A synthetic fixture verifies arithmetic and rejection semantics only.

## Independent score families

| Family | Denominator and output |
| --- | --- |
| Missions | All committed mission requests; completed, on-time, late, failed, unserved and cancelled counts |
| Mechanical swap | Every swap attempt; successful, failed and interrupted counts; completed duration p50/p95/p99 |
| Operational turnaround | Every landing/turnaround observation; completed duration p50/p95/p99 |
| Charge restoration | Every charge session; completed charge-time distribution; charger-input Wh and pack-terminal delta Wh with separate evidence/missing counts |
| Asset availability | Per-asset scheduled, available, unavailable, unknown and not-scheduled seconds |
| Energy | Independent Wh totals for vehicle battery terminal, aircraft battery terminal, charger input, stationary ESS terminal, site-meter import and export |
| Grid service | Requests, delivered/failed/cancelled counts, requested Wh, metered delivered Wh, unmeasured deliveries, response latency and sustain duration |
| Economics | Cost total by one explicit ISO currency and exact cost-per-on-time-mission numerator/denominator |

Percentiles use nearest-rank p50/p95/p99. A sample count accompanies each percentile; an empty sample emits null, not zero. Ratios are represented by their numerators and denominators.

## Evidence contract

Every event carries an evidence envelope bound to the relevant site, asset and configuration. Evidence must have non-empty producer/method/reference, a known evidence class, observation time no later than the scorecard cutoff, and validity covering its measurement interval.

The reducer rejects:
- duplicate event IDs across event families;
- event intervals outside the frozen measurement window;
- completed missions with no completion time;
- failed/unserved/cancelled missions with a false completion timestamp;
- overlapping resource availability intervals;
- mismatched asset kinds or configuration digests;
- site-meter energy relabelled as aircraft-terminal energy;
- mixed cost currencies;
- grid delivery declared without metered evidence;
- future-dated evidence;
- overflow and malformed observations.

Missing data is not silently changed into zero. Gaps in asset-availability segmentation are reported as unknown time, and explicit not-scheduled time is removed from scheduled availability denominator. Grid request energy is separate from metered delivery. Failed grid requests may carry a separately metered partial delivery without being reclassified as successful service.

## Frozen synthetic known answer

The fixture contains:
- 5 committed missions: 2 completed, 1 on time, 1 late, 1 failed, 1 unserved and 1 cancelled;
- 3 swap attempts: 2 completed (20 s and 30 s), 1 failed;
- 3 turnaround observations: 2 completed (40 s and 50 s), 1 failed;
- 2 charge sessions: 1 completed in 1,620,000 ms and 1 failed;
- 800 Wh charger input and 700 Wh measured pack-terminal delta in the completed charge session, plus a further 50 Wh input measurement on the failed session;
- distinct energy observations by six boundaries;
- 2 grid requests totaling 1,500 Wh requested, of which 800 Wh is metered delivered and one request is unmeasured/failed;
- ZAR 12,000 total operating cost, with one on-time mission as the denominator for the cost ratio.

These values are synthetic. They do not compare DJI, Zipline, Amazon, Wing or Luminous field performance.

## How this improves the benchmark field

The scorecard makes it difficult to hide denominator loss:
- a failed swap cannot disappear from attempted swaps;
- a late delivery is not an on-time mission;
- a charge session cannot be credited with pack-terminal energy based only on charger input;
- a scheduled resource with unknown telemetry is not counted as available;
- a grid request cannot be counted as delivered service without metering;
- lifecycle costs cannot be summed across currencies without conversion and declared exchange-rate evidence.

The scorecard can ingest observed event bundles from a reproduced paper/vendor trial or an independently monitored Luminous deployment. The benchmark protocol remains in [SUSTAINABLE_MOBILITY_SERVICE_CAPACITY_BENCHMARK_V0_1.md](https://github.com/Luminous-Dynamics/symtropy/blob/research/sustainable-mobility-service-capacity-v0-1/docs/research/SUSTAINABLE_MOBILITY_SERVICE_CAPACITY_BENCHMARK_V0_1.md).

## Claim boundary

The reducer does not prove sensor calibration, causal attribution, data completeness, cryptographic authenticity, airworthiness, grid permission, market settlement or physical operation. Those require independent evidence and system-specific qualification. A green synthetic scorecard is not an operational performance result.
