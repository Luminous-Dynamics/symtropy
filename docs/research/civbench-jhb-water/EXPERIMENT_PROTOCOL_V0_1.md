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
