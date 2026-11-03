# CivBench Johannesburg Water Experiment Protocol v0.1

**Protocol ID:** `CIVBENCH-JHB-WATER-EXPERIMENT-001`  
**Status:** design specification; *not yet a locked confirmatory pre-registration*  
**Date:** 2026-11-02  
**Scope:** synthetic and source-bounded experiments for the City of Johannesburg water-resilience vertical

## 1. The proposed paradigm shift

Do not build another monolithic “digital twin” and do not optimize a single NRW score. Build a **causal, evidence-qualified infrastructure laboratory** that connects:

1. physical state and conservation;
2. asset ageing, maintenance, repair and capacity;
3. sensing and information quality;
4. allocation and institutional coordination;
5. essential-service continuity, equity and recovery;
6. the provenance and uncertainty of every claim.

The novelty to test is not a new water-balance definition or a replacement for EPANET, AWWA or utility engineering software. It is whether matched counterfactual experiments can identify *which intervention changes which physical and human service outcomes, under what conditions, at what whole-life cost, and with what uncertainty*.

A better policy is not the policy with the prettiest average. It must preserve essential-service and safety floors, improve or explain distributional outcomes, and remain robust when the information or model is wrong.

## 2. Claim and evidence boundary

This protocol has four non-interchangeable evidence classes:

| Class | Meaning | Permitted use |
|---|---|---|
| Observed / reported | A dated value or statement from a named source, with scope and assurance recorded | Baseline envelope and external context |
| Derived | Reproducible arithmetic from cited observations | Explicitly labelled derived checks; never upgraded to observed |
| Assumption | A declared model parameter, range, or structural choice not identified by the source | Synthetic scenario construction and sensitivity analysis |
| Synthetic truth | Evaluator-known state created for a test | Oracle tests, hidden-state evaluation and controls; never presented as Johannesburg fact |

A source-derived aggregate does not identify a pipe-level age, material, roughness, failure hazard, demand trace, repair productivity or pressure regime. Do not infer those values. Unknown means unknown. A scenario may use a declared assumption or bounded distribution, but its output remains a conditional model result.

The FY2024/25 Johannesburg Water NRW figure is reported as unaudited in the service-delivery section. The annual-report PDF text was inspected, but source-byte snapshot and page-render verification remain incomplete. The two differently worded asset-backlog values must remain separate until reconciled. See the source register and caveats in [evidence PR #3371](https://github.com/Luminous-Dynamics/symtropy/pull/3371).

The public baseline motivates this work; it is not a calibrated hydraulic model, a representative household panel, or evidence of causal impact.

## 3. Factor model: vary causes independently

Use four dimensions. **G, M and I are interventions. H is the physical condition/blocking structure**, not a policy benefit.

### G — allocation and governance (four conditions)

1. Independent local allocation.
2. Fixed central allocation.
3. Negotiated federation with transparent, current observations.
4. Negotiated federation with delayed, conflicting or missing observations.

Reuse the existing allocation conditions in Symtropy #3319 / CIV-ECON-009. Keep the physical resource envelope fixed. Governance cannot create water, energy, parts or crew-hours.

### M — maintenance and renewal (five conditions)

1. Reactive-only maintenance.
2. Uniform renewal.
3. Risk-prioritised renewal.
4. Leak detection followed by independently verified repair.
5. Hybrid maintenance, renewal and metering investment.

Reuse asset and repair identities and workflow ownership already assigned in Symtropy #1645 and #3372. Do not create a second asset registry or work-order schema.

### I — information quality and observability (four conditions)

1. Complete/current information (positive control).
2. Realistic missing, stale or delayed evidence.
3. Biased or adversarial reporting, with evaluator-only ground truth.
4. Metering/telemetry investment as an intervention. Installation alone must **not** improve measured information quality: commissioning, data transmission, uptime, calibration, maintenance and coverage must be separately represented.

The candidate policy must not see evaluator-only truth. Report quality, freshness, coverage, missingness, bias, calibration and provenance as separate dimensions.

### H — physical and network conditions (blocking strata)

- Frozen topology, units, boundary conditions and reference-solver version.
- Declared pressure and demand regimes.
- Water/energy common-cause events composed through Sol-Atlas #144 and #147.
- Selected water-accounting event-order variants owned by Symtropy #3361.
- Held-out topology and hydraulic-model families where available.

H is not silently tuned separately for a favoured policy. All paired arms receive the same exogenous H scenario and time grid. Allow policy-dependent endogenous failures only after the common exogenous schedule has been applied.

## 4. Staged experiment, not a full factorial explosion

### Stage 0 — protocol lock and source qualification

Before confirmatory runs, freeze:
- protocol and schema digests;
- source inventory, definitions, periods, units, denominators and assurance labels;
- model/solver versions and configuration;
- topology, parameter ranges and unknowns;
- scenario-generation code, seed policy and time grid;
- policy implementations and resource budgets;
- primary estimands, outcome definitions, analysis plan and holdout boundary.

A commit timestamp alone is not sufficient proof of pre-registration. The locked digest must be recorded in an immutable, independently timestamped registry or signed release/receipt before confirmatory evaluation begins.

### Stage 1 — physical and accounting qualification

Run known-truth fixtures before policy comparisons:
- water stock/flow conservation and unit conversion;
- storage capacity, demand allocation and unmet-demand semantics;
- pump-energy and hydraulic reference-boundary checks where supported;
- no duplicate asset, repair, procurement or installation events;
- no double-spending of crew-hours, parts, vehicles, budget or contractor capacity;
- a repair claim cannot become verified restoration without the independent verifier boundary;
- no meter installation without telemetry/quality evidence can improve observability;
- missing parameters fail closed or produce an explicit indeterminate result.

EPANET is the hydraulic reference for aligned cases; Symtropy's discrete water kernel is an accounting/allocation oracle, not a hydraulic replacement. Use Sol-Atlas #139 for differential hydraulic checks.

### Stage 2 — isolated main effects

Use a common reference setting and vary only one factor at a time:
- G arms with M and I fixed;
- M arms with G and I fixed;
- I arms with G and M fixed.

Block/stratify by H. Match initial state, scenario, exogenous shocks, horizon, time grid, compute budget and resource envelope. The treatment factor is the only intended difference.

### Stage 3 — pre-registered interactions

Select a small number of mechanism-based interactions before looking at confirmatory outcomes:
- G×M: governance may change maintenance prioritisation or procurement/repair throughput;
- G×I: information quality may change coordination performance;
- M×I: detection investment may help only when telemetry and repair capacity are operational.

Add G×M×I only if pilot-derived power and compute budgets support it. Do not search all combinations and report only the winner.

### Stage 4 — robustness and adversarial stress

- Use common exogenous shock schedules for paired comparisons.
- Then permit policy-dependent endogenous failures and feedback.
- Hold out entire topology, asset-hazard and behavioural model families from tuning.
- Stress missingness, sensor bias, drift, wrong asset maps, delayed repairs, procurement failure, contractor scarcity, energy interruption and demand shocks.
- Include cases where reactive repair, local allocation or central bulk coordination should win.
- Preserve failures, cancellations, non-convergence, unsupported cases and abstentions in the denominator.

### Stage 5 — transfer

Keep synthetic, EPANET, laboratory and field evidence as separate evaluation domains. Calibrate on training/validation data only. Freeze the field holdout before model selection. A synthetic or EPANET win is not field validation, and a field correlation is not causal impact without a suitable identification design.

## 5. Estimands and causal comparisons

For outcome (Y), factor (F), treatment level (f), reference level (f_0), and paired scenario block (b), estimate the paired effect:

[
\hat{\tau}_{F,f} = \frac{1}{B}\sum_{b=1}^{B}
\left(Y_b(F=f)-Y_b(F=f_0)\right).
]

Define the sign convention per outcome. Report the full paired distribution, confidence/uncertainty interval, scenario coverage and sensitivity—not just a mean.

Pre-register, at minimum:

- **Essential water shortfall:** total unmet essential volume and duration, with exact demand/service-unit denominator.
- **Continuity:** interruption-hours per declared service unit; frequency and duration distributions.
- **Distribution:** worst-group and group-specific service floors and the gap between groups; do not publish sensitive household identifiers.
- **Physical losses:** real/physical loss volume or rate only where the water balance supports it; keep NRW, commercial loss and unbilled authorised consumption separate.
- **Assets:** failure rate per declared exposure, cohort survival, backlog age/risk and repeat failure.
- **Maintenance:** inspection/repair/verification cycle time, crew/parts queues and independently verified completion.
- **Economics:** capex, opex and whole-life cost; cost per verified restoration and per unit of verified physical loss reduced.
- **Resilience:** recovery time, service deficit area, peak deficit, overshoot and post-shock hysteresis under a declared shock.
- **Institutional agency:** provider concentration, viable alternatives, portability, switching/exit cost, procurement-to-completion lineage and appeal/correction availability.

For each metric define formula, unit, boundary, numerator, denominator, missing-data behavior, desired direction and whether lower/worst-tail statistics are primary. Do not create one weighted “resilience score.”

## 6. Hard constraints, not tradeable penalties

These are pass/fail constraints and cannot be offset by lower cost or better average NRW:

- essential water-service floor declared before evaluation;
- water-quality and public-health constraints wherever a qualified quality model/evidence exists;
- physical conservation and feasible capacity;
- no negative inventories or double-spent resources;
- no silent coercive denial, social-credit/deservingness score, or automated operational authority;
- privacy and critical-infrastructure information boundary;
- independent repair/commissioning verification before outcome promotion.

When the physical model cannot establish a floor, report **not evaluated / indeterminate** rather than “passed.” An aggregate improvement with a worst-group floor violation is a failure.

## 7. Statistical and reproducibility contract

- Use paired scenario blocks and common random numbers for exogenous shocks where meaningful.
- Randomize execution order where order effects or shared compute state could bias timing/performance measures.
- Use a pilot only to estimate variance, runtime and feasible sample size. Freeze the minimum detectable effect and sample count before confirmatory evaluation; never use confirmatory results to size the same test.
- Report effect estimates with 95% uncertainty intervals and a pre-declared family-wise correction (Holm within each declared primary-outcome family is the default).
- Publish per-stratum results, missingness, abstentions, failed runs and negative controls.
- Separate measurement, parameter, stochastic and model-structure uncertainty.
- Use sensitivity analysis and held-out model families; do not hide model disagreement in a pooled mean.
- Bind every result to exact source, model, topology, code, policy, scenario, evaluator and output digests. Deterministic replay must fail closed if any required digest or version is missing.
- The final confirmatory holdout is inaccessible to policy tuning and threshold selection.

No fixed run count is asserted here: it must follow a documented pilot variance estimate and a pre-specified minimum detectable effect. A small arbitrary seed count is not a statistical argument.

## 8. Benchmark lanes and competitive targets

### A. Accounting and physical correctness

**Gate:** all known-truth controls recover the declared result; no conservation, unit, chronology or capacity violation is silently accepted. Tolerances must be tied to numeric representation and solver behavior, declared before testing. Missing essential parameters must block/abstain, not be filled with convenient defaults.

### B. Hydraulic fidelity

Compare only supported EPANET/OpenFlows cases with identical topology, units, time steps, demand, roughness, elevations, pump curves, valves, controls and boundary conditions. Report flow/pressure/tank-level/water-quality error per quantity, solver convergence, runtime and unsupported feature coverage. Do not claim superiority from different model boundaries.

### C. Leak detection and localization

Keep separate: detection PR-AUC and false alarms per time/network; detection delay; exact pipe/node Top-1 and Top-3; graph and metric distance p50/p90; calibration; abstention; dispatch burden; and held-out topology/field-transfer.

The 2026 literature's headline figures use different tasks and datasets: Net3 Top-1, a within-300m localization threshold, transient experimental transfer and detection accuracy are not one leaderboard. Reconstruct the exact split and hardware before comparison.

**Aspirational competitive target:** on an exactly matched held-out task, seek at least a 5 percentage-point Top-1 improvement over a reproduced baseline *without worsening* p90 localization error, false-dispatch burden, calibration or worst-stratum performance. If the task/split is not exactly matched, label the result a new benchmark, not a SOTA win. For detection, target better calibrated sensitivity at a fixed operational false-alarm budget—not accuracy alone.

### D. Decision value and sensor placement

Compare random, fixed-coverage and conventional heuristic placements against value-of-information placement under identical sensor count, installation/maintenance cost, telemetry reliability and evaluation scenarios. Primary endpoint is reduction in decision loss or verified service shortfall, not entropy reduction alone. Include cases where extra sensors have zero or negative value because repair capacity, telemetry or authority is the binding constraint.

### E. Maintenance and asset renewal

Compare policies on identical life-cycle resource envelopes and held-out hazard/lifetime families. Report survival calibration, repeat-failure, verified restoration, backlog, whole-life cost and distributional service impacts. Do not make 1.5% (or any single annual replacement rate) a universal optimum.

### F. Resilience

Benchmark shock-response curves, service deficit area, recovery time and distributional tail outcomes across matched shocks. Report hysteresis and maintenance-debt accumulation. Mean time-to-recovery alone can hide severe worst-group outcomes.

### G. Institutional robustness

Test provider concentration, single points of failure, portability and exit/switching costs, data correction/appeals and procurement-to-completion lineage. A low-cost solution that creates an unreplaceable vendor or opaque decision bottleneck does not pass the institutional lane.

## 9. Industry comparison: complement rather than misrepresent

| Comparator | What it already does | CivBench's legitimate extension |
|---|---|---|
| DWS / IWA water balance; AWWA M36 / Water Audit Software | Standardized water audit, loss components and utility comparison | Typed provenance, explicit uncertainty and causal counterfactuals; consume rather than replace the audit |
| World Bank / IWA NewIBNET | Utility performance indicators including continuity, NRW, metering, quality and complaints | Matched policy experiments, worst-group outcomes, asset feedback and recovery |
| EPA EPANET 2.2 | Pressurised hydraulic and selected water-quality simulation | Independent accounting checks and composed service/maintenance/institutional experiments |
| Bentley OpenFlows Water | Commercial hydraulic, water-quality, transient and engineering workflows | Reproducible, open protocol and independent evaluation of broader causal and institutional outcomes—not a claim to match all product features |
| 2026 leak-detection studies | Protocol-specific detection/localization/transfer results | Strict split contracts, uncertainty, abstention, value-of-information and field transfer |
| Utility maintenance practice | Work management, asset planning, repair and renewal | Explicit matched resources, independent verification and causal service outcomes |

A result is only “better than industry” when the exact product/model version, input data, task, hardware, operational constraints and outputs are matched. Otherwise describe complementarity or a different evaluation domain.

## 10. New research hypotheses worth testing

1. **Observability is a constrained resource.** The best sensor is not necessarily the one that detects the most events; it is the one that changes a feasible, beneficial decision given telemetry uptime, repair capacity and service floors.
2. **Maintenance debt is path-dependent.** Deferred renewal can alter failure clustering, repair queues and future budget flexibility; a snapshot score will miss this.
3. **Robust policy is a set, not a winner.** Report Pareto-efficient policies and minimax regret across held-out structural models; do not hide normative trade-offs in weights.
4. **Institutional resilience is operational.** Portability, alternative providers, repair skills, spare-parts substitutability and evidence correction affect actual recovery time.
5. **The best digital twin knows when not to answer.** Calibrated abstention and explicit indeterminate states can outperform overconfident point recommendations under model shift.
6. **Service is the outcome; infrastructure is the mechanism.** A repaired pipe, installed sensor or spent budget matters only insofar as it demonstrably changes safe, reliable and equitable service.

## 11. Promotion gates and honest claim ladder

- **L0 — specification:** protocol, schema and assumptions are documented.
- **L1 — deterministic correctness:** fixtures, controls, invariants and replay pass.
- **L2 — synthetic comparative evidence:** preregistered paired results across declared synthetic model families.
- **L3 — reference-model validation:** differential agreement on supported EPANET cases.
- **L4 — independent empirical transfer:** frozen laboratory/field holdout and measurement-quality audit.
- **L5 — causal pilot evidence:** governed field intervention with counterfactual design, safety review and independent outcome verification.

Do not skip levels. A queued workflow is not a pass. A synthetic win is not real-world impact. A model comparison is not causal field evidence.

## 12. Related ownership

- Symtropy #3372 — asset ageing, maintenance and renewal experiment.
- Symtropy #3319 / #3361 / #1645 — resource accounting, water event semantics and repair workflow.
- Symtropy #3363 / PR #3371 — public aggregate evidence and source provenance.
- Sol-Atlas #139 / #144 / #147 / #174 / #326 — hydraulic reference, pump-energy coupling, joint stress, localization and WFE composition.
- Mycelix #5699 / #2006 — institutional feedback, evidence authority, privacy, contestability and appeal.

**This document is a protocol proposal, not an executed result or an externally timestamped pre-registration.**


## 13. 2025–2026 literature update: sensor-to-action value, not sensor accuracy alone

Recent work strengthens the need for a decision-aware sensor benchmark:

- Farah and Shahrour's 2026 systematic review includes 90 studies and finds substantial heterogeneity in dataset generation, splits, metrics and scenario definitions. It specifically identifies random-split scenario leakage, joint sensor-placement/detection optimization, multi-utility transfer, few-shot/limited-label testing and deployment latency/bandwidth as research needs. Reference: [Water 2026, 18(18), 2267](https://www.mdpi.com/2073-4441/18/18/2267), published 11 September 2026.
- Di Menna et al. (1 July 2026) combine normalized hydraulic sensitivity with topological shortest-path coverage for sensor placement and evaluate a grid, an urban district and an irrigation network. This is a useful placement baseline, not evidence that the chosen sensors improve repair throughput or household service. Reference: [Environ. Earth Sci. Proc. 2026, 44(1), 44](https://doi.org/10.3390/eesp2026044044).
- A 2026 cascading-aware placement study integrates graph-signal structure, hydraulic state and risk propagation. This motivates stress cases where a sensor's usefulness depends on cascading failures and changing risk, not only static distance or sensitivity. Reference: [Reliability Engineering & System Safety, DOI 10.1016/j.ress.2026.112709](https://www.sciencedirect.com/science/article/pii/S095183202600520X).
- A 2026 LeakAgent study reports its own EXA7 evaluation with 21 sensors across 381 nodes, reporting an 82.9% reduction in mean detection time (48 h to 8.2 h), 0.974 network coverage and 0.940 leak-detection ratio. Treat these as source-reported results on that study's network and setup—not a transferable SOTA result or a direct comparison with Net3/field-transfer tasks. Reference: [Communications AI & Computing (2026)](https://www.nature.com/articles/s44488-026-00024-w).
- A 2026 review of 72 studies reports a material simulated-versus-real validation gap, with reported real-world accuracy ranges of 70–79.7% across the reviewed studies and synthetic-to-real declines of roughly 3–30 percentage points. These are heterogeneous literature summaries, not a single pooled estimate. Reference: [IIC 2026 systematic review](https://doi.org/10.3390/iic2030010).

### The decision-aware sensor benchmark

Add a coupled but separately reported chain:

`placement → sensing → telemetry → inference → dispatch → inspection → repair → independent verification → service outcome`.

Measure each transition. In particular:
- sensor placement must include installation, calibration, connectivity, power, data freshness, failure and maintenance cost;
- detection must include calibration, false alarms, abstention, time-to-detect and localization uncertainty;
- dispatch must include workload, prioritization, travel/crew hours, spare parts, contractor availability and the queue created by false positives;
- repair must be verified independently and linked to a measured physical/service response;
- service outcomes must include shortfall, interruption duration, worst-group effects and recovery—not merely detection accuracy.

Define decision value for a declared loss function (L) and feasible policy class (Pi):

`VOI = E[L(Y, π_{no\ info})] - E[L(Y, π_{info})]
       - C_{sensor} - C_{telemetry} - C_{operation}`.

This is evaluated over the same held-out scenario blocks and service floors. The policy (π_{info}) must obey the same physical, budget, crew, parts and safety constraints as its comparator. The loss vector is published by component; do not hide trade-offs in one opaque scalar. Where a single decision value is used for a registered decision, publish its full component vector and sensitivity to the declared weights.

### Required sensor-placement baselines and falsifiers

Compare, with the same sensor count and whole-life operating budget:
1. existing/reference placement where known;
2. seeded random placement;
3. topology-only coverage / shortest-path placement;
4. hydraulic-sensitivity or observability placement;
5. published hybrid topology-plus-sensitivity placement where the method can be reconstructed;
6. decision-aware placement that accounts for telemetry uptime, dispatch/repair capacity and service impact.

Pre-register at least these falsifiers:
- More sensors improve observability but do not improve service because repair capacity is the binding constraint.
- Better detection increases false dispatch and worsens the repair queue under a constrained crew budget.
- A sensor is installed but telemetry is unavailable, stale or unmaintained; no observability gain is credited.
- The highest-risk/highest-sensitivity locations are not the best places for equitable service recovery.
- Risk propagation or common-cause energy failure makes static placement brittle.
- A lower-accuracy model may yield higher verified service value if it is better calibrated and abstains on ambiguous cases.

**Primary competitive claim:** outperform alternative placement methods on verified decision value and service outcomes at matched lifecycle cost, while reporting detection/localization performance separately. A sensor count, coverage score, entropy reduction or raw accuracy improvement alone is not a service-resilience win.


## 14. Node-level service performance and safe-fail capacity

Crozier and Weijs (13 June 2026) present a performance-based WARATA quantification approach for urban water supply. It recommends defining expanded system boundaries that include piped infrastructure, emergency supply options and human resources; evaluating water supplied at individual user nodes; and tracking time-based withstanding, absorptive, restorative, adaptive and transformative capabilities. The paper is theoretical and explicitly calls for future real-world case-study validation, so CivBench treats it as a framework to operationalize and test—not as a validated utility benchmark. Reference: [Water 2026, 18(12), 1458](https://doi.org/10.3390/w18121458).

This aligns with the 2026 NIST lifeline-infrastructure review, which describes water, wastewater, electricity, fuel, communications and transportation as interdependent and identifies a lack of consistent system-level guidance for continuity and recovery after hazards. Reference: [NIST, 29 September 2026](https://www.nist.gov/publications/advancing-resilience-across-lifeline-infrastructure-systems-need-consistent-guidance).

### Benchmark service as a time series

For each declared service node or privacy-preserving group (g), report:
- required/target service trajectory (r_g(t)), sourced or explicitly declared as a scenario assumption;
- piped safe water delivered (q^{pipe}_g(t));
- qualified fallback/emergency water delivered (q^{fallback}_g(t));
- total qualified service (q_g(t)), without double-counting water or treating unsafe/unknown-quality supply as potable;
- the first fail-safe threshold crossing (t^{impact}_g);
- the first safe-fail / minimum-service threshold crossing (t^{collapse}_g);
- time spent below each threshold, deficit volume, recovery time and any recurrence;
- worst-group and per-node trajectories, not only citywide means.

Where a fallback service is modeled, make its finite stock, source, quality, vehicle/pipe capacity, travel time, staff, fuel/energy, distribution hubs and replenishment constraints explicit. It is not an unlimited emergency buffer. A tanker dispatched is not water delivered; delivered volume is not safe potable service unless quality is qualified.

Do not invent household minimums or infer them from the aggregate utility report. Thresholds must come from an applicable standard, approved service obligation, or explicit stakeholder/rights-based decision with source and time basis recorded. If no defensible threshold exists, publish the raw service trajectory and mark threshold analysis indeterminate.

### Available, remaining and realized response capacity

Keep three states separate:
- **Available:** capacity potentially deployable at the start of the scenario (crew hours, qualified staff, vehicles, parts, emergency water, generator/fuel).
- **Remaining:** capacity still usable after existing commitments, travel, maintenance, fatigue, lead time and failures during the scenario.
- **Realized:** capacity actually mobilized and delivered in the recorded event chain.

This avoids counting the same crew, vehicle, part or water reserve as available to multiple simultaneous incidents. It also prevents plans and procurement from being mistaken for operational capability.

### Falsifiable service-resilience cases

- A network can lose piped service while safe-fail service prevents collapse; compare the time buffer and whole-life cost of the fallback.
- An apparent fallback capacity can fail because staff, vehicles, fuel, quality assurance or access—not water stock—are binding.
- Power restoration can precede water restoration, or vice versa; common-cause and cross-lifeline dependency must be explicit.
- A policy can improve average service while moving a vulnerable group's threshold crossing earlier; worst-group gates must catch it.
- A service trajectory can recover briefly and fail again because the maintenance backlog or energy constraint persists.

The benchmark therefore reports performance curves and threshold-crossing times alongside physical, asset, financial and institutional measures. The WARATA capabilities are explanatory dimensions, not one new weighted resilience score.


## 15. Hierarchical hydroinformatics: match model complexity to sensing scale

A 2026 Journal of Hydroinformatics review of 86 studies argues that operational readiness is limited less by algorithmic sophistication than by isolated models that ignore physical and operational constraints. It proposes sensor-aware, context-dependent hierarchy: network-level risk assessment and budget prioritization where instrumentation is sparse; DMA-level early warning from existing SCADA under stochastic demand; and in-situ confirmation / last-mile localization where high-resolution sensing is justified by excavation or repair costs. It also emphasizes operational false-alarm rates, detection time, computational feasibility, SCADA/GIS integration and leakage-safe temporal partitioning. Reference: [AI-based leak detection in water distribution networks: a decadal review across sensing and monitoring scales, Journal of Hydroinformatics (2026), DOI 10.2166/hydro.2026.069](https://doi.org/10.2166/hydro.2026.069).

This suggests a stronger architecture comparison than “one larger model beats one smaller model”:

| Tier | Sensing and context | Correct task | Output contract |
|---|---|---|---|
| Network-level | Sparse/heterogeneous observations, asset map and broad network context | Risk ranking, sensor/inspection budget allocation, uncertainty and data-gap identification | Ranked areas/assets with confidence, rationale, abstention and value-of-information estimate; not a pipe fault declaration |
| DMA-level | Existing SCADA / district flow-pressure telemetry and demand context | Robust anomaly/early warning and event triage | Calibrated alert with onset, affected DMA, false-alarm budget, evidence freshness and escalation/abstention |
| In-situ | Acoustic/transient/pressure measurements collected for a selected event | Confirmation and last-mile pipe/node localization | Candidate pipe/node, graph/metric uncertainty, confidence, evidence provenance and whether excavation threshold is met |

Compare at least:
1. monolithic model attempting all tiers;
2. conventional per-tier baselines;
3. hierarchical cascade that escalates only when uncertainty / operational value warrants;
4. hierarchical cascade with a calibrated abstention and explicit human confirmation step.

The tiers have distinct labels, costs, time resolutions and error consequences. Do not score a network-level risk ranking as a pipe localization. The hierarchy must be evaluated end-to-end as well as per tier: missed events, false escalation, dispatch volume, time-to-confirmation, unnecessary field visits, excavation avoidance, crew queues, energy/bandwidth, and independently verified service outcomes.

### Falsifiable architectural hypotheses

- A hierarchical system reduces high-resolution sensor and compute demand without materially worsening missed-event rate or time-to-restoration.
- A DMA-level detector can lower false dispatch compared with applying an in-situ localizer to every network alert.
- The cascade does not improve outcome when data hand-off is stale, GIS identity is wrong, or human/operator response capacity is unavailable.
- A monolithic high-accuracy model may be inferior in lifecycle cost and operational utility to a calibrated tiered system.
- The architecture fails closed when the event cannot be mapped to a current network asset identity or the downstream tier lacks sufficient measurement evidence.

Primary claim: improve **end-to-end operational utility** at matched total lifecycle cost and safety/service constraints, while reporting per-tier statistical performance separately. Do not claim an autonomous replacement for SCADA, GIS, hydraulic engineers or operator authorization.


## 16. Digital-twin maturity: prediction is not closure

A 2026 systematic review of explainable AI and digital twins for sustainable/resilient water management screened 48 records. Within its corpus, 43 remained at conceptual, offline, retrospective or connected-pilot stages (M0–M2), five reached operational decision support (M3), and none was identified as an audited M4 closed-loop implementation. The review also found that explanation faithfulness/stability, calibrated uncertainty and practitioner workflow were rarely assessed together. These counts describe that review's corpus and classification; they are not a census of every utility or commercial deployment. Reference: [Explainable AI and Digital Twins for Sustainable and Resilient Water Management, Water (2026)](https://www.mdpi.com/2073-4441/18/19/2443).

An AWWA survey analysis published 15 April 2026 reports utility interest in AI, digital twins and predictive analytics, while many respondents lacked well-defined implementation plans. The survey was based on a 2024 utility survey, so it is evidence about adoption patterns and planning—not the current installed base in 2026. Reference: [Model or Digital Twin? AWWA Survey on Water Distribution Technology](https://doi.org/10.1002/awwa.70075).

### CivBench operational maturity ladder

This is a working benchmark adaptation, not a claim that every sector uses the same M0–M4 definition:

- **M0 — conceptual:** proposed architecture and intended use documented.
- **M1 — offline/retrospective:** historical or simulated data evaluation, with frozen splits and leakage controls.
- **M2 — connected pilot:** live or near-live data connection in a bounded non-authoritative pilot; uptime, freshness, missingness and operator workflow measured.
- **M3 — operational decision support:** named users receive calibrated outputs in a defined workflow; recommendations, overrides, time-to-decision, failure handling and action handoff are logged.
- **M4 — audited decision-to-service closure:** source observations, state estimate, uncertainty, recommendation, human authorization, action/work order, independent completion verification and measured service outcome are linked in a reproducible audit trail, with safety gates, rollback and post-event model discrepancy review.

M4 does **not** mean autonomous infrastructure actuation. It means the evidence chain is closed and independently auditable. Any operational control remains subject to utility authority, safety engineering and human authorization.

### M4 closure metrics

Report separate values:
- observation-to-state-estimate latency and freshness;
- state-estimate calibration and model discrepancy;
- recommendation-to-human-decision latency and override rate;
- decision-to-work-order and work-order-to-field-completion time;
- proportion of actions independently verified;
- proportion of verified actions linked to a measured service outcome;
- service outcome persistence and recurrence;
- failed, cancelled, deferred, unsupported and unverified cases in the denominator;
- post-event residual review, correction and model rollback time.

Do not call a system M4 merely because it has a live dashboard, an API connection, an automated ticket or a digital twin label. A prediction that does not change a feasible decision is not an operational outcome; a work order that is not independently verified is not a completed intervention; and a completed intervention without service evidence is not measured impact.

## 17. New probabilistic sensor-placement comparator

A newly indexed 2026 Journal of Hydrology article proposes probabilistic sensor placement using Dynamic Residual Masking and game-theoretic pseudo-Jacobians derived from Random Forest / SHAP contributions. Its article highlights report 91.4% detection accuracy with five sensors and a 4.2% false-positive rate on its own evaluation. Those figures are **paper-specific reported results**, not a direct comparison to the Net3 benchmark, the within-300m localization benchmark, or field-transfer studies. Reference: [Probabilistic sensor placement in water distribution networks](https://doi.org/10.1016/j.jhydrol.2026.136514).

Add this method as a reconstruction target in the sensor-placement lane. Before any numeric comparison, recover the exact network, topology, labels, train/test split, sensor assumptions, input variables, cost model, hardware and false-positive definition. If those details or artifacts are unavailable, compare at the method-family level and label the result protocol-matched rather than reproduced.

The baseline set therefore includes: current/reference placement; seeded random; topology-only/shortest-path; hydraulic-sensitivity/observability; topology-plus-sensitivity hybrid; probabilistic Dynamic Residual Masking / pseudo-Jacobian method; cascading-aware placement; and decision-aware placement tied to telemetry, dispatch/repair capacity and service outcomes.


## 18. Outcome missingness is part of the result

Every metric in an outcome vector must carry both a numeric value and an evidence status:

- `observed`: measured directly from a declared source;
- `derived`: reproducible arithmetic from source-qualified inputs;
- `modelled`: produced by a declared model or simulation;
- `not_measured`: no measurement exists for this scenario;
- `unsupported`: the evaluator/model cannot qualify the metric;
- `withheld`: a value exists but is unavailable under a data-governance constraint;
- `not_applicable`: the metric does not apply to the scenario.

The first three statuses require a numeric value; the latter four require a missing value. A valid observed zero is not interchangeable with not-measured, unsupported, withheld or not-applicable. No missing value is imputed as zero.

A paired treatment-minus-control difference is defined only when both arms have valid numeric values. If either side is missing, the paired delta is indeterminate for that metric; retain each arm's value/status and report missingness by scenario, metric, stratum and policy. Do not fill, drop or average away missing results without a separately pre-registered missing-data analysis.

This contract is necessary for service and water-balance comparisons: "no measured interruption" is not the same claim as "interruption not measured"; "zero physical loss" is not the same claim as "physical loss model unsupported"; and a withheld group outcome must not silently improve the reported worst-group result.


## 19. Evidence-class parity for paired effects

A numeric value is not automatically comparable merely because both arms contain numbers. In v0.1, a paired treatment-minus-control delta is numeric only when **both arms have the same evidence status**: observed with observed, derived with derived, or modelled with modelled.

Observed-vs-modelled, observed-vs-derived and other cross-status comparisons are explicitly marked `evidence_status_mismatch` and produce no numeric delta. Missing control, missing treatment, both missing and evidence-status mismatch are distinct states. Do not silently promote a modelled result to an observation, demote an observation to a model output, or create a mixed-evidence delta that hides the distinction.

A cross-status comparison may be added only through a separately named, versioned, preregistered bridge that defines the relationship, validation evidence, uncertainty propagation and decision purpose. A general conversion factor or post-hoc weighting is not sufficient.

This makes the default paired estimate conservative: same scenario block, same exogenous shocks, same initial state, same topology/model family/time grid/horizon, permitted treatment-only differences, and same metric evidence class. If the two arms do not meet those conditions, the outcome is indeterminate for that comparison rather than a misleading numeric improvement.


## 20. LeakDB reproduction and sealed-holdout contract

### Benchmark inventory and paper-specific baselines

The KIOS LeakDB repository and WaterBenchmarkHub describe a public corpus with **1,000 scenarios for Net1 and 1,000 scenarios for Hanoi**, including node/link pressure, demand and flow time series, scenario configuration, leak location/type (including abrupt and incipient), and 30-minute labels. The original repository also provides the older CCWI-WDSA2018 example, a MATLAB scoring tool and a Zenodo link for the complete data. Sources: [KIOS LeakDB repository](https://github.com/KIOS-Research/LeakDB), [WaterBenchmarkHub LeakDB record](https://waterfutures.github.io/WaterBenchmarkHub/benchmarks/KIOS-LeakDB.html).

Several current papers use a 500-scenario Hanoi subset, which should be preserved as a **paper-reproduction lane**, not mistaken for the full public benchmark:
- Damonti & Bernasconi, 2026 uncertainty-aware incipient detector: on its 500-scenario subset, reports 380 leak / 120 non-leak scenarios, 93.60% scenario accuracy, 91.58% sensitivity, 0/120 false positives, 0.975 ROC AUC and 0.993 PR AUC. It describes a threshold of (1.45\times10^{-6}), selected just below a difficult scenario's score, and says this enforces 0 FPs across all 120 leak-free scenarios. Its accessible software-availability statement says the specific Python implementation is not publicly hosted. References: [paper](https://doi.org/10.1007/s00477-026-03239-6); [dataset](https://github.com/KIOS-Research/LeakDB).
- Damonti & Bernasconi, November 2026 Dynamic Residual Masking sensor placement: on its own 500-scenario Hanoi evaluation, the abstract reports 91.60% detection accuracy with five sensors versus 93.80% with 31 sensors, and a 2.5% false-positive reduction versus its explicit-Jacobian baseline. Article highlights round the five-sensor result to 91.4% and state 4.2% false positives. Preserve these as publisher-reported, paper-specific values pending reproduction; do not splice the highlight and abstract numbers into a synthetic “best” result. Reference: [paper](https://doi.org/10.1016/j.jhydrol.2026.136514).
- A 2026 systematic review of 90 studies reports validation practices as 32 random-split, 9 temporal/independent holdout, 7 cross-validation, and 42 absent/insufficiently reported. It warns that pooled random scenario splits can leak highly similar scenarios across train/test. Reference: [review](https://doi.org/10.3390/w18182267).
- A second 2026 review of 244 records / 71 pure deep-learning studies makes a similar case for standardizing splits, metrics, scenario definitions, uncertainty, and operational evaluation. Reference: [review](https://doi.org/10.2166/wpt.2026.485).

**Important caveat on the 93.60% / zero-FP claim:** the uncertainty-aware paper's accessible methods/results discuss selecting the operational cutoff against a difficult scenario and say it was chosen to guarantee zero false positives across the 120 negative scenarios; the text we inspected does not establish an independent, sealed threshold-calibration set. Treat the reported metrics as an internal 500-scenario benchmark result until the authors' split and threshold-calibration procedure are independently reconstructed. This is a reproducibility caveat, not an allegation of misconduct.

Even with 0/120 false positives observed, the one-sided 95% exact binomial upper confidence limit for the underlying FPR is approximately **2.47%**, not zero. A finite test set cannot establish a population FPR of exactly zero.

### New integrated digital-twin comparator (separate from a universal SOTA claim)

Li & Gao (Scientific Reports, published 24 October 2026) connect hydraulic-state assimilation, leak detection, graph-based localization and pressure regulation through one digital twin. The paper reports pooled in-house F1 0.916 and Top-5 hit rate 0.816, compared with 0.836 and 0.573 for its strongest baseline. On the independent BattLeDIM 2019 validation set, F1 is 0.818 and Top-5 hit rate falls by about twenty percentage points. Its C-Town pressure-regulation case reports leakage reduction of 37.5% versus 24.6% for uniform depressurization while retaining 99.4% service compliance. These are important paper-specific reference values, not direct CivBench wins: exact topology, sensors, labels, splits, operating threshold and service constraints must be reconstructed before numeric comparison. The paper also identifies state assimilation as a load-bearing ablation and notes limits around network scale, transient hydraulics and dense instrumentation. Reference: [Li & Gao, 2026](https://doi.org/10.1038/s41598-026-73010-y).

CivBench should reproduce this architecture as a *coupled baseline*: (1) state estimation/recalibration, (2) residual and change detection, (3) calibrated node/pipe posterior ranking, and (4) constrained pressure-control decisions. Ablate state assimilation independently. Score the whole loop by verified leakage reduction, service compliance, false-dispatch burden, detection/localization delay and transfer degradation—not by pooled F1 alone. Do not infer that more sensing or lower pressure is always better.

### External real-utility temporal-anomaly dataset (not LeakDB localization)

Babela, Munk & Munková (Scientific Data, 17 April 2026) publish an anonymized one-zone Slovak utility dataset covering 2022: 8,737 hourly records with 18 precomputed anomaly-score features derived from hydraulic, energy and environmental streams. Maintenance-system fault records are projected into a binary `fault_d7` label spanning a ±7-day window around the recorded fault. The published release provides derived scores rather than raw SCADA/energy/environmental streams, and the paper states that no custom source code is included. Conventional flow and pressure variables were not retained among the 18 features because they did not meet that dataset's association screen. Reference: [Babela et al., 2026](https://doi.org/10.1038/s41597-026-07203-5); [public dataset](https://doi.org/10.5281/zenodo.15096167).

Treat this as a **single-zone, weakly labelled temporal-anomaly transfer dataset**, not as LeakDB, a pipe-localization benchmark, a hydraulic calibration corpus or representative multi-utility field validation. The absence of `fault_d7 = 1` means no confirmed record fell in that window; it does not prove the network was leak-free. A ±7-day label mixes pre-report and post-report signatures and is not an exact leak-onset timestamp. Therefore:

- Separate pre-report early-warning evaluation from post-report event detection; never let post-report records contribute to a predictive lead-time claim.
- Use event-level temporal partitions. Group all records/windows linked to one maintenance event, and purge at least the full ±7-day label span plus any model lookback/recovery window at split boundaries.
- Treat unconfirmed periods as unlabelled/unknown or use a defensible positive-unlabelled design; do not silently score them as true negatives.
- Refit every learned transform, feature screen, normalizer, calibration and threshold on training/calibration partitions only. Because the public release contains precomputed anomaly scores and not the raw data/code, split-safe reconstruction of the published preprocessing cannot currently be verified; this limitation must accompany any result.
- Report event-level sensitivity, false alert episodes per zone-day, lead-time distribution, calibration/abstention, and confirmed-work-order verification. Do not use pointwise accuracy as the principal outcome.

This dataset is a valuable external reality check for time-series anomaly handling and weak-label discipline, but it cannot establish pipe-level location or the causal service benefit of an intervention.

### Three non-interchangeable evaluation lanes

1. **L0 — paper reproduction:** reproduce each paper's own 500-scenario Hanoi subset, labels, sensor configuration, preprocessing, threshold and baseline as closely as available artifacts permit. Record unavailable artifacts and every protocol difference. Numeric comparisons are “reproduction” only when split, labels, sensor placement and operating threshold are matched.
2. **L1 — full LeakDB, scenario-held-out:** use the full 1,000-scenario Hanoi and 1,000-scenario Net1 corpora where data/version integrity permits. Split by whole scenario ID before window generation or feature extraction. No timestep, window, augmentation or derived series from one scenario may cross train/calibration/test.
3. **L2 — cross-topology transfer:** train/calibrate on one topology and test on the other, then reverse. Report both directions. Because this is only two synthetic topologies, call it cross-topology LeakDB transfer—not multi-utility or field generalization.

Add separate locked challenge subsets for incipient-only leaks, leak-size/onset/duration groups, unseen leak locations, concurrent events where present, demand/noise regimes, sensor failure/missingness and topology/asset-map perturbation. A model may not tune on a challenge subset and later present it as held out.

### Split and threshold governance

- Group by complete scenario identity. If scenario IDs have parent/configuration, random-seed or near-duplicate relationships, group those related cases together.
- Split before any rolling windows, filtering fit, feature selection, normalization parameter fit, sensor-location optimization, hyperparameter search or threshold selection.
- Fit learned transforms only on training data. Fit model selection/hyperparameters inside training folds. Use a calibration partition for operating thresholds, probability calibration or abstention. The final test is evaluated once with all choices frozen.
- Keep the 500-case paper-reproduction subset separate from the complete 1,000-case topology corpus; do not silently mix or replace data versions.
- Publish SHA-256 digests of source archive, topology/configuration, scenario manifest, label files, split membership, feature pipeline, sensor placement, model, calibration procedure and evaluator.
- Use a blind evaluator process with test labels inaccessible to candidate code. Repeated leaderboard submissions require a fixed query budget, audit log and a final non-public holdout to control adaptive overfitting.
- The final split manifest must record scenario IDs and grouping rationale, not merely a random seed. A seed is not a substitute for a frozen split artifact.

### Metrics: event, time, location and operational burden

Keep these separate:
- **Scenario/event detection:** sensitivity, specificity, precision, F1, balanced accuracy, MCC, PR-AUC, ROC-AUC and calibrated probability score (Brier/log loss). Report prevalence and confidence intervals.
- **Time-step/event timing:** detection delay from leak onset, time to sustained alert, false alarm episodes per node-day / network-day, event fragmentation and missed-event duration. Pointwise accuracy alone is insufficient.
- **Localization:** exact pipe/node Top-1 and Top-3, graph and metric distance p50/p90, localization abstention/coverage and calibration. Do not treat a binary detector as a localizer.
- **Operations:** false dispatches at a declared threshold, unnecessary visits, detection latency, compute, sensor/telemetry uptime, bandwidth, energy and cost per independently verified event.
- **Distribution and robustness:** report incipient versus abrupt, severity, location, demand regime, sensor density, missingness, noise, topology and model family strata. Preserve failed runs and abstentions in denominators.

### Sensor-placement fairness

For every compared placement algorithm:
- the sensor budget (e.g. 5 versus 31) and sensor capabilities are fixed before the test;
- placement is optimized on training data only, then frozen before calibration/test;
- all methods see the same scenario universe, noise/missingness, sensor failure model, telemetry constraints and whole-life sensor/maintenance cost;
- include random, coverage/shortest-path, explicit hydraulic Jacobian, published topology+sensitivity, Dynamic Residual Masking, and decision-aware variants where the method can be reconstructed;
- report both diagnostic performance and decision/service value. More sensors or higher detection accuracy alone is not a service outcome.

### CivBench competitive claim rule

First reproduce the 2026 paper-specific baselines on their own compatible task. Then report new L1/L2 results independently. A +5 percentage-point gain can be claimed only on a genuinely identical frozen test protocol and only if false dispatch, p90 localization, calibration and worst-stratum performance do not regress. Otherwise report a new benchmark result, not “SOTA superiority”.
