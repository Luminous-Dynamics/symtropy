# Grid Flexible-Response Reliability and Capacity-Accreditation Protocol

**Protocol:** GRID-FIRM-CAPACITY-001  
**Version:** 0.1  
**Status:** event-response reliability screen; not system resource-adequacy accreditation

## 1. What this layer does—and does not do

This protocol follows the time-resolved `GridResponseReport` from `grid_response.rs`. It groups repeated events only when they share one pinned service product, PCC boundary, baseline method, response configuration, commitment kW and operating-condition stratum.

The first output is a conservative lower confidence bound on the mean success probability of the frozen set of event opportunities. A dispatch succeeds only when every response-envelope gate passes. An unavailable, missing-controller, no-response or telemetry-failure opportunity is not silently dropped; scheduled events must match the exact caller-pinned opportunity manifest.

**This is not ELCC, EFC, LOLE, EUE, ISO/TSO accreditation, or a system-wide capacity credit.** It does not establish resource adequacy. The report deliberately calls its outcome an event-response reliability qualification and returns only the single committed kW tested.

## 2. Research and industry context

- **LBNL, Resource Accreditation in U.S. Resource Adequacy Mechanisms (July 2026).** Reviews evolving accreditation methods and current/forthcoming approaches in California, MISO and PJM amid extreme weather, energy-limited resources, retirements and data-centre load growth. This demonstrates why event-response success must not be conflated with a system capacity credit. [LBNL report](https://eta-publications.lbl.gov/publications/resource-accreditation-us-resource).
- **EPRI resource accreditation methods.** Distinguishes ELCC, EFC and equivalent conventional power. ELCC is tied to the amount of additional system load that can be served while preserving the reference adequacy criterion, commonly expressed through LOLE. A response-event pass rate alone cannot calculate this. [EPRI methods](https://msites.epri.com/resource-adequacy/resource-acredidation/resource-accreditation-methods).
- **ESIG Large Loads Task Force (April 2026).** Recommends integrating flexible large loads into resource-adequacy planning rather than treating curtailment or bring-your-own-generation as a short-term fallback. [ESIG report](https://www.esig.energy/reports-briefs/large-loads-resource-adequacy/).
- **NLR review of resource-adequacy assessment best practices (2026).** Reviews probabilistic adequacy assessment, portfolio procurement and remaining industry gaps. [DOI 10.1109/TEMPR.2025.3640116](https://doi.org/10.1109/TEMPR.2025.3640116).
- **EPRI model selection guidance.** Explicit dynamic models should include availability, weather dependence, response degradation and customer fatigue; these remain open modeling gaps rather than settled standards. [EPRI model-selection guidance](https://msites.epri.com/resource-adequacy/methodologies-and-data/model-selection-guidelines).
- **Dunlap (Research Square, 28 July 2026).** Uses four years of day-ahead prices and 49.4 million production inference requests across five U.S. grid regions to model spatial migration, temporal deferral and DVFS. It estimates 24.6% commitment depth for mixed-use and 40.0% for inference-dominant facilities under its assumptions. These are preliminary model estimates—not field-verified or accredited firm capacity. [EPIC working-paper page](https://epic.uchicago.edu/working-papers/quantifying-ai-data-center-flexibility-as-a-resource-adequacy-asset/).
- **LAD-Flex, May 2026.** Its Alibaba-trace study reports up to 22% load shedding during studied flexibility windows. This is workload-trace/model-based, not a universal firm-capacity guarantee. [DOI 10.1016/j.ijepes.2026.111940](https://doi.org/10.1016/j.ijepes.2026.111940).
- **ACM inter-data-centre field demonstration (22 June 2026).** Shows that response feasibility depends on workload data semantics, pre-launch state and infrastructure; the paper presents measured field feasibility for selected products, not full market qualification. [DOI 10.1145/3765611.3815063](https://doi.org/10.1145/3765611.3815063).

## 3. V0.1 method: conservative event-response reliability

Every test pins an exact event-opportunity manifest before evaluation:

- portfolio and opportunity-set identity/digest;
- service product, PCC measurement boundary, baseline method and response-configuration digest;
- one condition stratum (for example high ambient temperature + high workload + grid scarcity);
- one committed reduction kW;
- every scheduled dispatch event ID, including events that may later be unavailable;
- minimum number of trials, required average event-success probability and confidence failure probability delta.

The exact sorted opportunity IDs are included in the input contract and returned report. The opportunity-set digest is supplied by the caller and must be independently recomputed from the canonical manifest; this library does not claim to authenticate or recompute cryptographic evidence bytes.

Let X_i = 1 when event i's complete response envelope passes and 0 otherwise. An unavailable event is X_i=0. The empirical success fraction is:

    p_hat = sum_i X_i / n

V0.1 computes a one-sided Hoeffding lower confidence bound:

    LCB = max(0, p_hat - sqrt(ln(1/delta) / (2n)))

Under independent bounded Bernoulli event outcomes, this is a conservative lower confidence bound on the mean success probability over the pinned opportunity set. The independence-basis ID is a caller declaration, not proof that event outcomes are independent. The result does not establish a common success probability under drift, and it does not extrapolate to another operating-condition stratum.

The single committed reduction is called event-response-reliability-qualified only when the lower bound meets the contract's required mean success probability and the minimum trial count and exact-manifest checks pass. No larger kW level is inferred.

## 4. Known-truth confidence examples

At one-sided confidence 95% (delta = 0.05):

| Trials | Successes | Empirical rate | Hoeffding lower bound |
|---:|---:|---:|---:|
| 20 | 20 | 100.00% | 72.63% |
| 100 | 100 | 100.00% | 87.76% |
| 101 | 100 | 99.01% | 86.83% |

These figures show why “all observed events passed” is not a 100% reliability claim. The exact committed capacity is only returned when the lower bound meets the caller's threshold.

## 5. Mandatory adverse-event and manifest rules

- The scheduled opportunity manifest is frozen before evaluating the event outcomes. The trial IDs must match it exactly: no missing, duplicate or unexpected event IDs.
- Every scheduled opportunity must have either a validated response report or an explicit unavailable record with reason and evidence ID.
- Evaluated reports must match service product, boundary, baseline method, response digest, condition stratum and exact committed kW.
- Failed event gates count as failure, including latency, sustained delivery, under-delivery, overshoot and ramp.
- Do not pool different products, sites, baselines, configurations, committed capacities or operating-condition strata.
- Report weather, grid-stress episode, ambient, workload and recovery conditions for every event. Stress episodes that are not plausibly independent must be clustered or separately analysed.

## 6. Dependence, drift and fatigue: required V0.2 research

The Hoeffding result is not robust to arbitrary serial dependence. Consecutive dispatches within one heatwave, grid scarcity incident, maintenance window or controller outage can share common causes. Re-labelling those events with unique IDs does not make them independent.

Before using this result in an adequacy submission, compare at least:

1. Independent event-block sampling by weather/scarcity episode.
2. Cluster bootstrap over independent stress episodes, with the cluster definition preregistered.
3. Dependence-aware concentration bounds or martingale methods when assumptions can be defended.
4. A non-stationary reliability model with explicit degradation/fatigue and recovery state.
5. Chronological adequacy simulation measuring LOLE and EUE, including correlated workload, ambient, renewables, storage state, grid scarcity and outage events.

Only the last system-level stage can start to support ELCC/EFC-style resource-adequacy credit. It must compare a reference system with the same system plus the flexible data-centre resource, then quantify the added load or capacity while restoring the same adequacy criterion. Event response probability is necessary evidence, not an ELCC substitute.

## 7. Competitive benchmarks and proposed targets

These are research targets, not results or market rules:

- 100% match between the frozen opportunity manifest and trial receipts; zero silent event omissions.
- 100% of unavailable and failed dispatch opportunities retained as failures with evidence lineage.
- Zero cross-product, cross-boundary, cross-baseline, cross-configuration or cross-stratum pooling.
- Compare the same commitment on all event opportunities with minimum event count and one-sided lower confidence reporting.
- On held-out chronological stress episodes, compare event reliability and availability against workload-only, storage-only, facility-only and combined response strategies.
- For the next system-level phase, report ELCC/EFC, LOLE and EUE as a function of committed kW and duration; include weather-year sensitivity, long stress episodes, outage correlations, workload growth and fatigue.
- Publish both event-level delivery measures and the system-level adequacy measure; do not reduce them to one weighted score.

## 8. Claim ceiling

A successful v0.1 report means that supplied event reports were internally consistent, all manifest IDs were accounted for, the specified response gates were evaluated, and the declared independent-trial Hoeffding bound met the caller's threshold. It does not prove source authenticity, independent event generation, future stationarity, grid-system adequacy, ELCC/EFC, ISO/TSO acceptance or firm capacity outside the pinned condition stratum.

System adequacy and market accreditation remain a separate engineering and governance step.
