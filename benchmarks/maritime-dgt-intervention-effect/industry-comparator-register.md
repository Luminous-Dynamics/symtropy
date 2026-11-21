# Industry comparator register — Maritime DGT Intervention Effect

This register defines what later comparisons must reproduce. Reference values are descriptive and context-specific; they are not universal targets.

## Public quantitative anchors

The World Bank's 30 October 2026 *Container Terminal Performance Improvement Guide* reports the following 2025 CPPI-derived descriptive averages:

| Measure | Reported average | Correct interpretation |
|---|---:|---|
| Moves per port hour | 44.8 | Port-wide service rate; includes a wider actor boundary than terminal equipment alone |
| Moves per berth hour | 56.3 | Berth productivity indicator, not equivalent to per-crane productivity |
| Gross crane hours per vessel operating hour | 3.0 | Crane intensity / deployment rate |
| Moves per gross crane hour | 23.5 | Crane productivity denominator must match the gross-hour definition |
| Total vessel arrival time | 16.4 h | Includes 12.3 h waiting + 4.1 h steam-in |
| Total port time outside cargo operations | 46.5% | Demonstrates why optimizing cargo work alone can miss major waiting / non-operating intervals |
| Terminal capacity utilization | 72.2% | Global estimate for 2025; guide discusses a context-dependent 65–70% normal range and rising congestion risk above 70%, especially above 80% |

These numbers are useful for checking denominator definitions and spotting implausible claims. They must **not** be applied as a DGT target or direct performance score: vessel size, call mix, terminal type, trade profile, berth configuration, nautical services, observation definitions and source coverage differ.

Source: World Bank, *Container Terminal Performance Improvement Guide: Improving Spatial and Operating Efficiency at Container Terminals* (30 October 2026), https://documents.worldbank.org/en/publication/documents-reports/documentdetail/099103026152033883 and PDF https://documents1.worldbank.org/curated/en/099103026152033883/pdf/P509479-2ec7a9c5-9ed5-4252-ab11-a2f60747db87.pdf.

## Comparator classes

| Source / comparator | Scope and current public reference | How to use it | Limitation |
|---|---|---|---|
| World Bank, *Container Terminal Performance Improvement Guide* (2026) | Port / terminal / vessel-call KPI boundaries; explicitly treats berth, STS cranes, horizontal transport, yard, gate, rail/barge, labor, maintenance, information systems and external actors as interacting parts | Use its definitions and coupled-system framing before optimizing | Descriptive reference; metric scope and vessel/call mix matter |
| CPPI 2025 methodology | AIS/geofence-based port-call timing and event matching across its declared milestones | Compare event semantics, timestamp boundaries, one-to-one matching, missingness and source qualification | The reported validation applies to CPPI's own data and cannot be treated as our held-out score or universal ground truth |
| DCSA Port Call Standard v2.0.0 / Industry Blueprint 2026.Q1 | Explicit Estimated–Requested–Planned–Actual pattern for many negotiated services; service-specific actual-only milestones also exist; berth, pilotage, towage, mooring, cargo and feeder schedule updates are modelled as distinct processes | Keep raw ERP(A) status, event types, source actors and profile version; actual is execution, not schedule confirmation | Standards conformance does not prove that a physical event occurred or that cargo was delivered |
| S&P Global Port Performance | Licensed broad port/terminal call and productivity coverage | Candidate external benchmark provider after rights, metric definitions and sample comparability review | Proprietary data and coverage descriptions are not themselves direct ground truth |
| Robust berth allocation and quay-crane literature (2026) | Li, Xu and Luo's July 2026 review frames uncertainty-aware robust population metaheuristics for berth allocation and quay-crane assignment and gives a proposed benchmark suite | Compare robust evaluation, feasibility preservation/recovery, decoding and adaptive robustness strategies on same instances | Review benchmarks and illustrative baselines are not a universal SOTA ranking; reproduce baselines before comparison |

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

- World Bank, *Container Terminal Performance Improvement Guide* (30 October 2026): https://documents.worldbank.org/en/publication/documents-reports/documentdetail/099103026152033883
- DCSA, *Port Call Standard v2.0.0 — Purpose & Scope*: https://reference.dcsa.org/content/standards/releases/port-call/v2/pc-v2-purpose-and-scope
- DCSA, *Industry Blueprint 2026.Q1 — Vessel Journey*: https://reference.dcsa.org/content/standards/industry-blueprint/current/industry-blueprint-vessel-journey
- CPPI 2025: https://www.worldbank.org/en/topic/transport/publication/container-port-performance-index
- S&P Global Port Performance: https://www.spglobal.com/marketintelligence/en/solutions/port-performance
- Li, Xu and Luo (6 July 2026), *Robust Metaheuristics under Uncertainty for Berth Allocation and Quay Crane Assignment: A Review*: https://arxiv.org/abs/2608.19214
