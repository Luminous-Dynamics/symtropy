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

## Existing terminal and berth-planning platforms

This benchmark should be an **assurance and interoperability layer around existing operating systems**, not a claim that it replaces a TOS.

| Platform / category | Publicly described role | Implication for our design |
|---|---|---|
| Kaleris N4 / Navis TOS | Kaleris describes N4 as coordinating vessel, yard, rail, gate and control-room work, with integrated execution and separate RTG/truck/yard optimization capabilities. | Consume existing operational records and decision outputs with lineage; do not reproduce TOS dispatch or equipment control. |
| Transnet's documented SPARCS N4 use | Transnet Port Terminals describes a relay integration between TFR's City Deep inland facility and South African terminals, allowing container tracking across the inland/port interface. | The South African opportunity is not merely “connect rail to port”; reconcile and independently qualify the events exchanged across an existing integration. |
| Portchain Quay / Connect | Portchain describes berth planning and berth-window alignment across terminals and carriers, with source integrations, predictive analytics and infeasible-arrival forecasts. | Treat berth allocation as an incumbent capability. Our differentiator should be auditable source admission, uncertainty, evidence-backed realization and network-level outcome evaluation across operators. |

Vendor-described capabilities and customer stories are not matched independent benchmarks. In particular, we should not assume an incumbent TOS lacks functionality because our prototype adds an evidence layer.

**Competitive boundary:** Kaleris/N4 and Portchain are planning/execution products. Symtropy's proposed wedge is independent verification of what was knowable, committed, activated, physically realized and finally delivered; conserved cross-mode stocks; explicit uncertainty/censoring; spillover-aware counterfactual evaluation; and deterministic replay. Integrate through approved APIs, data exports and standards rather than competing to become the new system of record.

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
- Kaleris, *N4 Terminal Operating System*: https://kaleris.com/terminal-operating-system/kaleris-n4-tos/
- Transnet Port Terminals, *Navis / SPARCS N4 and inland relay*: https://www.transnetportterminals.net/About/Pages/Navis.aspx
- Portchain, berth-alignment platform: https://portchain.com/home
- DCSA, *Industry Blueprint 2026.Q1 — Vessel Journey*: https://reference.dcsa.org/content/standards/industry-blueprint/current/industry-blueprint-vessel-journey
- CPPI 2025: https://www.worldbank.org/en/topic/transport/publication/container-port-performance-index
- S&P Global Port Performance: https://www.spglobal.com/marketintelligence/en/solutions/port-performance
- Li, Xu and Luo (6 July 2026), *Robust Metaheuristics under Uncertainty for Berth Allocation and Quay Crane Assignment: A Review*: https://arxiv.org/abs/2608.19214

## Southern African public visibility comparator: Dues

[Dues' South African port board](https://dues.co.za/ports) and [DGT terminal page](https://dues.co.za/ports/durban-gateway-terminal) are a useful adjacent product comparator. On the 22 November 2026 research snapshot, Dues stated it refreshes its port board about every 30 minutes from terminals' published files and showed for DGT: four vessels working, two at anchor/waiting, 3,361 boxes still to unload and one berthing within 48 hours. Its DGT page also exposes vessel status, berthing plans, export-stack dates, and a 21-day evaluation of its own plan accuracy (including start-of-work and time-at-quay measures).

**Interpretation:** this demonstrates an existing local visibility/aggregation product and should be included in any interface/coverage comparison. It is not an independent source of terminal truth: it says it reads the terminal's published files, and its derived status/forecast metrics are its own product outputs. The displayed state is a time-stamped secondary observation, not an immutable terminal snapshot or independently adjudicated ground truth. The benchmark should compare the same dated terminal files and evaluate each parser against separate adjudicated event/cargo outcomes.

Potential matched comparison:
- time from terminal publication to public display;
- source-document revision detection and immutable as-of replay;
- preservation of raw status fields and source timestamp conflicts;
- vessel/visit identity resolution and unresolved rate;
- plan-versus-realized start-of-work error by horizon;
- stack-window revision / cancellation detection;
- exact container reconciliation and custody continuity (not currently established by the Dues page);
- coverage, latency, uncertainty and explicit unknowns.

Do not infer that Dues has or lacks internal capabilities beyond those publicly described. No Dues data feed, contract, API, or ground-truth artifact has been obtained for this benchmark.
