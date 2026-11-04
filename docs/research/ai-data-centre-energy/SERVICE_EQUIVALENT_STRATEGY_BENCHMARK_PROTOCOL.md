# Service-Equivalent Strategy Benchmark Protocol

**Protocol:** AI-SERVICE-EQUIVALENT-BENCHMARK-001  
**Version:** 0.1  
**Status:** research protocol; synthetic known-truth fixtures only

## 1. Purpose

Compare energy-management strategies only when they deliver the same useful service under the same workload, forecast vintage, joint scenario distribution and initial resource state. A policy that reduces electricity by dropping, degrading or delaying work must not be reported as an efficiency improvement.

The Rust evaluator is `crates/domains/symtropy-energy-systems/src/strategy_benchmark.rs`. It compares supplied, evidence-bearing outcomes; it does not create schedules or simulate the physical plant.

## 2. Paradigm shift: measure service-equivalent resource intensity

Define offered work under a frozen service contract. A work unit is accepted only when it passes the caller-pinned correctness/quality oracle. On-time useful work is the accepted subset completed within the declared deadline.

For scenario i, let p_i be the normalized scenario weight, E_i facility energy, W_i accepted useful work, and W_i^ontime on-time accepted useful work.

Use **ratios of weighted totals**:

    facility kWh per accepted useful unit = sum_i(p_i * E_i) / sum_i(p_i * W_i)
    facility kWh per on-time useful unit  = sum_i(p_i * E_i) / sum_i(p_i * W_i^ontime)

Do not use the mean of per-scenario energy/work percentages. It answers a different question and can overweight low-work or near-zero-work scenarios.

Report the same denominator convention for grid-import energy, operating cost, water and carbon. A policy must pass service and hard-constraint gates before it can enter the qualified Pareto frontier.

## 3. Matched counterfactual design

Every policy must be evaluated on:

- identical offered workload, release/deadline profile and service-contract digest;
- identical joint scenario IDs and scenario digests;
- identical raw scenario probabilities and a single common normalization;
- identical forecast/model vintages, initial storage state, thermal state and site/grid limits;
- identical measurement boundaries, runtime/quality oracle and evidence acceptance rules;
- identical number of evaluation windows and the same held-out scenario set.

The only intended difference is the candidate strategy. A common scenario seed or digest improves pairing but does not prove simulator validity or real-world counterfactual truth.

Input matrices must be complete. If any strategy lacks a scenario, changes a scenario digest, alters offered work, or silently changes probability mass, the paired comparison is rejected.

## 4. Metrics and non-compensable gates

Report each strategy separately:

| Dimension | Metrics |
|---|---|
| Service | offered, accepted useful and on-time accepted work; expected and worst-case on-time fraction |
| Energy | facility, IT, cooling and grid-import kWh; facility/grid kWh per accepted and on-time useful unit |
| Cost | operating cost and currency; cost per on-time useful unit |
| Water and carbon | litres and kg CO2e; each per on-time useful unit |
| Performance | scenario P95 latency, worst scenario P95 latency, peak temperature |
| Reliability | deadline, thermal, protected reserve and grid-shortfall probabilities |

Hard gates remain separate: expected on-time work, worst-case on-time work, deadline-violation probability, worst-case scenario P95 latency, thermal-violation probability, protected-reserve breach probability and grid-shortfall probability.

The summary reports a facility-to-IT energy ratio using the ratio of weighted totals. This is a PUE-like diagnostic only. It is **not** claimed as ISO/IEC PUE unless the facility/IT measurement boundaries, reporting interval and required rules match the standard. A stable PUE-like ratio can coexist with significant improvement in accepted useful work per kWh.

## 5. Pareto analysis, not a blended score

Only strategies that pass all pinned service and reliability gates are eligible for the qualified Pareto frontier. The v0.1 frontier considers:

- minimize facility kWh per on-time useful work unit;
- minimize grid-import kWh per on-time useful work unit;
- minimize operating cost per on-time useful work unit;
- minimize water litres and carbon kg CO2e per on-time useful work unit;
- maximize expected on-time useful-work fraction.

Report all objectives and dominated-by strategy IDs. Do not create a weighted score where better carbon or lower cost can compensate for a reserve, deadline or thermal failure. A strategy can be a valid Pareto trade-off without being universally better.

## 6. Industry and SOTA comparators

- **MLPerf Power** measures energy at the wall for the paired benchmark and reports system power or energy-per-stream outputs depending on scenario; its measurement scope is the full system and only the published benchmark. Compare with exact MLPerf task, quality and latency requirements, not TDP estimates. See [MLCommons Inference Datacenter and Power methodology](https://mlcommons.org/benchmarks/inference-datacenter/).
- **MLPerf Inference v6.1** (September 2026) adds agentic-inference patterns and publishes architecture-neutral, reproducible performance results; use these workloads as a compute-level comparator where task/quality/latency semantics can be matched. See [MLCommons v6.1 announcement](https://mlcommons.org/2026/09/mlperf-inference-v6-1-results/).
- **IEA 4E Data Centres and Flexibility** (1 July 2026) distinguishes market-serving, grid-serving and system-serving flexibility and describes operational barriers to deploying it. Match dispatch duration, service contract and grid need before comparing flexibility percentages. See [IEA 4E report](https://www.iea-4e.org/edna/publications/data-centres-and-flexibility/).
- **EU data-centre sustainability reporting** uses PUE, WUE, Energy Reuse Factor and Renewable Energy Factor for facility reporting. Retain these facility metrics, but add accepted useful work and on-time goodput to compare workload-service efficiency. See [European Commission report, COM(2026) 500](https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=CELEX%3A52026DC0500).
- **Nature Energy 256-GPU demand-response field demonstration** and the **2026 Alibaba-cluster trace study** remain important flexibility comparators. Reproduce only under the same or transparently matched dispatch/service definition; do not transplant their percentage savings into a different site.

MLPerf Power's measured energy-per-stream or tokens-per-joule is not interchangeable with a scheduler's facility kWh per on-time accepted work unit. A complete comparison reports both compute-level benchmark performance and facility/system-level service-equivalent outcomes.

## 7. Synthetic known-truth example

The fixture uses two equally weighted scenarios, with 100 and 90 on-time accepted work units, for both baseline and candidate. The expected on-time work is therefore 95 units under both strategies.

| Metric | Baseline | Candidate |
|---|---:|---:|
| Expected on-time useful work | 95 | 95 |
| Facility energy | 115 kWh | 103.5 kWh |
| IT energy | 63.25 kWh | 56.925 kWh |
| Facility kWh per on-time work unit | 1.210526 | 1.089474 |
| Service-equivalent facility-energy reduction | — | 10.0% |
| Facility-to-IT ratio (PUE-like only) | 1.818182 | 1.818182 |

The facility-to-IT ratio is unchanged while the candidate uses 10% less facility energy for the same accepted on-time work. The example demonstrates why a facility-only ratio cannot replace work-normalized goodput. All values are synthetic arithmetic, not a site-performance result.

## 8. Proposed targets

These are research targets, not achieved results or standards thresholds:

- at matched offered work, quality and on-time service, target at least 10% lower facility kWh per on-time accepted work unit than the strongest simple baseline; stretch target 20% after site-specific calibration;
- no reported energy win when expected or worst-case on-time useful work is below the pinned threshold;
- zero false-negative thermal, protected-reserve or grid violations in the frozen hostile suite;
- target 10% lower operating cost per on-time useful work unit without worsening water or carbon intensity; otherwise report a Pareto trade-off;
- reproduce MLPerf Power on standard MLPerf tasks separately and retain measured wall-energy boundaries;
- reproduce published field/trace flexibility outcomes only with matched dispatch windows, workload QoS and site boundaries.

## 9. Interpretation and limitations

A successful result establishes only that the supplied strategy receipts form a complete matched matrix, that the declared probabilities were normalized under the pinned tolerance, and that the reported ratios, paired differences and gates were computed under the declared metric convention.

It does not prove the submitted outcomes are physically true, that the scenario ensemble represents future operations, that a simulator is validated, that a strategy is causally better in production, or that the system is certified or authorized for dispatch. The useful-work oracle and measurement evidence must be independently reviewed.
