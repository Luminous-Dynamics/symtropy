# Industry comparator register — Maritime DGT Intervention Effect

This register defines what later comparisons must reproduce. Reference values are not universal targets.

| Source / comparator | Scope and current public reference | How to use it | Limitation |
|---|---|---|---|
| World Bank, *Container Terminal Performance Improvement Guide* (2026) | Port / terminal / vessel-call KPI boundaries; warns that berth, crane, yard, gate and inland bottlenecks interact | Use its metric definitions and service-chain boundaries before optimizing | Descriptive reference; metric scope and vessel/call mix matter |
| CPPI 2025 methodology | AIS/geofence event validation; six port-call milestones; reported high event-matching quality in its own data | Compare event semantics, timestamp boundaries and one-to-one matching protocol | Dataset-specific; cannot be treated as our held-out score or a universal event oracle |
| DCSA Port Call Standard v2.0.0 / 2026.Q1 Vessel Journey | Planned, requested, estimated and actual times; berth-window and vessel-journey updates | Preserve standard status semantics and profile/version identifiers | Standards conformance does not prove physical events |
| S&P Global Port Performance | Licensed broad port/terminal call and productivity coverage | Candidate external benchmark provider after rights, definitions and sample comparability review | Proprietary and paid data; coverage claims are not direct ground truth |
| Current berth allocation / quay-crane literature (2026) | Robust optimization and metaheuristic scheduling under uncertain arrival, handling and resource conditions | Compare integrated resource plans against strong same-instance baselines | Model and benchmark instance differences prevent headline-result comparison |

## Fair comparison rules

- Match vessel/call size, trade mix, terminal configuration, arrival process, cargo demand and disruption regime.
- Match physical resource constraints, working calendars, crane/berth compatibility, yard and gate limits, rail paths and safety envelopes.
- Freeze source data, event cutoffs, weather/incident chronology and forecast availability.
- Compare the same compute budget, hardware, solver tolerances, time limits and random seed/event sets.
- Publish exact-oracle gaps on enumerable instances and paired out-of-sample regret on held-out cases.
- Report service distributions, backlog stock/area, cargo delivery and externalized effects—not only moves/hour or berth utilization.
- Report Pareto fronts for cost, delay, service reliability, energy/emissions and distributional performance.
- No claim of beating SOTA is valid until the comparator implementation, data, constraints, metric, hardware and evaluation protocol are matched and independently reproducible.

## Public references

- World Bank, *Container Terminal Performance Improvement Guide* (2026): https://www.worldbank.org/en/topic/transport/publication/container-terminal-performance-improvement-guide
- DCSA, Port Call Standard / Vessel Journey: https://dcsa.org/standards/port-call
- CPPI 2025: https://www.worldbank.org/en/topic/transport/publication/container-port-performance-index
- S&P Global Port Performance: https://www.spglobal.com/marketintelligence/en/solutions/port-performance
- Li, Xu and Luo (6 July 2026), *Robust Metaheuristics under Uncertainty for Berth Allocation and Quay Crane Assignment* (scholarly review): https://doi.org/10.1007/s10479-026-06355-6
