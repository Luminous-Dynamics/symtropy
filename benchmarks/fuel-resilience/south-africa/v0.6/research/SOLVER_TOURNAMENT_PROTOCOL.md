# SAFRB solver tournament — matched multi-carrier resilience evaluation

Protocol revision: 0.1
Research cut: 24 November 2026
Status: preregistration; no comparative solver results claimed
Applies to SAFRB v0.2–v0.6.

## 1. Competitive question

SAFRB should not attempt to replace general-purpose LP/MILP solvers, power-flow packages or industrial historians. Mature systems already occupy those categories.

The research question is whether an evidence-qualified layer can give established planning tools more defensible physical inputs, independently validate feasibility, improve resilience decisions and identify the next useful measurement.

There are two separate competitions:
1. Optimizer quality: feasibility, objective, service level, solve time, memory and optimality gap.
2. Decision-assurance quality: evidence eligibility, source traceability, physical-boundary conservation, uncertainty coverage, scenario robustness, independent replay and decision regret.

Do not combine these into one score. An optimal solution to an incomplete physical model remains an unsafe decision.

## 2. Industry baseline map (versions checked 24 November 2026)

| Baseline | Current capability | Role in tournament | Claim boundary |
|---|---|---|---|
| PyPSA 1.4.0 (8 October 2026) | Energy-system optimization with network constraints, storage, unit commitment, two-stage stochastic scenarios and CVaR risk preference. v1.4 corrected rolling-horizon unit commitment and security-constrained multi-period behavior. | Main open energy-system comparison. | No superiority claim without matched instances and independent output checking. |
| HiGHS 1.16.0 (11 November 2026) | LP/MIP/QP solver; parallel MIP enabled by default; rewritten LP reader and solver/presolve correctness updates. | Open reproducible LP/MILP reference. | Pin build, parameters, threads and solver status. |
| Gurobi 14.0 (17 November 2026) | Commercial LP/MIP/MINLP solver. Vendor reports averages vs 13.0 on its internal suite: MIP +26%, concurrent LP +23%, dual simplex +24%, barrier +22%, PDHG +74%, nonlinear barrier +84%, MINLP over 2x. | Commercial solver reference where a license is available. | Vendor-reported internal averages are not independent SAFRB results. |
| OR-Tools min-cost flow | Mature min-cost-flow implementation. | Baseline for v0.3 single-grade logistics. | Not a multi-carrier stochastic energy model. |
| AVEVA PI System | Industrial historian, data collection, contextualization and delivery. | Upstream data/context integration comparator. | Complement rather than replacement; no historian claims. |
| SLB Lumi / digital operations | Energy-sector data/AI platform and domain workflows. | Domain workflow comparator where authorized matched tasks exist. | No field-performance superiority claim without matched operator data. |
| NIST SI for Manufacturing / TIDAL | 2026 programme on industrial intelligence, human-machine teaming, contextualization, integration test benches and digital-twin VVUQ. | Measurement-science and assurance context. | Ongoing research, not a product leaderboard. |

Primary references:
- PyPSA v1.4.0: https://docs.pypsa.org/v1.4.0/release-notes/
- PyPSA stochastic / CVaR: https://docs.pypsa.org/latest/user-guide/optimization/stochastic/
- HiGHS v1.16.0: https://github.com/ERGO-Code/HiGHS/releases/tag/v1.16.0
- Gurobi 14.0: https://www.gurobi.com/company/newsroom/gurobi-releases-version-14.0-with-major-performance-advances-to-expand-what%E2%80%99s-possible-with-optimization
- OR-Tools min-cost flow: https://developers.google.com/optimization/flow/mincostflow
- AVEVA PI System: https://www.aveva.com/en/products/aveva-pi-system/
- SLB Lumi: https://www.slb.com/products-and-services/delivering-digital-at-scale/software/lumi
- NIST SI for Manufacturing: https://www.nist.gov/programs-projects/super-intelligence-si-manufacturing

## 3. Matched benchmark arms

All arms receive the same frozen case, evidence-eligible state, topology, product/energy conversion assumptions, demand frame, disruption set, recovery horizon and objective definition.

| ID | Method | Purpose |
|---|---|---|
| B0 | Do-nothing / status quo | Baseline shortage and unserved service. |
| B1 | Greedy priority dispatch | Transparent heuristic. |
| B2 | SAFRB v0.3 min-cost flow | Conventional time-expanded logistics baseline. |
| B3 | PyPSA 1.4.0 deterministic | Energy-system LP / unit-commitment baseline. |
| B4 | PyPSA two-stage stochastic | Here-and-now investment plus scenario recourse. |
| B5 | PyPSA CVaR | Risk-averse stochastic dispatch. |
| B6 | HiGHS 1.16.0 LP/MILP | Open solver reference. |
| B7 | Gurobi 14.0 LP/MILP | Commercial reference where licensed. |
| B8 | SAFRB v0.4 probability-ambiguity analysis | Worst-case distribution within declared lower/upper probability bounds. |
| B9 | Symthaea decision-support policy | Evidence contradiction detection, active measurement selection, alternatives and abstention; not a solver. |
| B10 | Human operator/planner | Same evidence snapshot and time budget. |
| ORACLE | Exact enumeration / exhaustive small-instance solve | Evaluation-only upper bound for tiny cases. |

Keep responsibilities distinct:
- v0.3 solves a single-grade logistics flow problem.
- PyPSA / HiGHS / Gurobi solve dispatch and network optimization.
- v0.4 scores scenario outcomes independently of the planner.
- v0.5 maps fuel batches to generator electricity under LHV/efficiency bounds.
- v0.6 evaluates multi-carrier electricity and battery schedules at a named service boundary.
- Symthaea recommends observations and bounded alternatives; authorized operators or providers retain dispatch authority.

## 4. Frozen input contract

Each case must freeze:
- canonical case ID, revision and content digest;
- product grade / energy carrier and canonical unit;
- location, service zone, period and validity interval;
- physical topology and capacity units;
- source qualification class and raw-source digests;
- stock state: reported, measured, committed, nominated, in-transit, arrived, discharged, quality-released, accessible;
- LHV, efficiency, auxiliaries and storage loss assumptions;
- storage state, power and energy limits;
- demand by service class, zone and time;
- scenario IDs and exact weights, or an explicit no-probability stress track;
- legal/contractual capacity rights and provider constraints;
- objective hierarchy, hard constraints and sourced cost basis.

Unknown values remain unknown. Missing capacity, probability, stock or cost must not be silently replaced with zero.

Run three separate data tracks:
1. Measured/qualified: only source observations that pass the evidence contract.
2. Reported/declared: claims retained but not promoted to measured fact.
3. Synthetic stress: controlled assumptions, never presented as actual South African operations.

Every comparator receives the same frozen inputs. A system may exclude an unqualified input, but must report the exclusion and service impact.

## 5. Objective and hard constraints

Use lexicographic priorities where practical:
1. Satisfy legal, safety, quality and equipment constraints.
2. Minimize unserved critical-service energy by priority class.
3. Minimize other essential-service shortfall.
4. Minimize non-critical shortfall.
5. Minimize operating and switching cost.
6. Minimize emissions only where factors are source-qualified.

Hard safety and product-quality limits are not monetary penalties. If the solver cannot express lexicographic objectives directly, use sequential solve/fix stages and record each stage.

## 6. Scenario protocol

Run the same named scenario IDs across all methods:
- port/berth closure;
- import cargo delay or non-arrival;
- supplier/export terminal/shipping lane common cause;
- tank quality rejection or grade mismatch;
- pipeline/pump-station outage;
- grid outage;
- generator underperformance or fuel-conversion uncertainty;
- battery degradation or unavailable state-of-charge;
- critical demand surge;
- combined import, terminal, pipeline and electricity shock;
- replenishment and recovery tail.

Keep three statistical tracks separate:
- empirically calibrated event frequency with sampling uncertainty;
- explicit weighted scenarios with weights documented as assumptions or elicitation;
- unweighted stress envelope with no probability claim.

For robust tests, retain the ambiguity set and the returned worst-case witness distribution. A witness is auditable; it does not make the supplied probability bounds calibrated.

## 7. Metrics — no composite leaderboard

Service and resilience:
- expected unserved critical Wh and litres (where fuel accounting applies);
- VaR95 and CVaR95 of unserved critical Wh;
- worst-case shortfall;
- critical service rate by zone and priority;
- first service-violation time and recovery time;
- geographic/service-class distribution of failures.

Physical and accounting assurance:
- bus energy-balance residual;
- grade-specific mass-balance residual;
- source-eligibility false-positive / false-negative rate;
- quality and grade mismatch rejection;
- capacity, SOC and power-envelope violations;
- reservation vs delivery and commitment vs receipt distinctions;
- fraction of constraints with source or engineering traceability.

Optimizer performance:
- time to first feasible solution and best incumbent;
- objective value and optimality gap;
- status, bound status and termination reason;
- peak memory, threads, node count and model-build time;
- numerical warnings, infeasibility certificates and time-limit outcomes.

Evidence and decision quality:
- unsupported-claim promotions (target zero);
- missing/stale-source detection;
- independent replay match rate;
- decision regret against exact small-instance oracle;
- net EVSI and realized measurement value;
- false isolation, unnecessary measurement and correct abstention.

Do not compare raw runtimes when formulations, constraints or tolerances differ.

## 8. Reproducibility and hardware

Publish source commit, case digest, solver build/version, parameters, license class, CPU, physical/logical core count, RAM, OS, container image, thread count, time limit, random seed, deterministic-mode settings, model file, solver log, result file and independent-verifier result.

Use the same machine and wall-clock budget for primary head-to-head tests. Use a second hardware tier only as a labelled scalability track. Report median, p95 and outliers; not only the best run.

Case tiers:
- Tiny oracle: exhaustive validation and exact solution comparison.
- Small: terminal / microgrid instances.
- Medium: multi-port, multi-zone, multi-day network.
- Stress: many scenarios, common causes, longer recovery and multiple products/carriers.

## 9. Proposed advancement targets (future gates, not achieved results)

- 100% rejection of invalid or evidence-ineligible fuel/energy inputs in the frozen negative corpus.
- Zero accepted mass/energy-balance residual on exact accounting cases.
- Zero battery capacity, SOC or power-envelope violations.
- Zero false promotion of forecast, reported or scenario data to verified stock.
- 90% empirical coverage for a preregistered 90% service interval on a sealed calibrated test set.
- At least 10% lower CVaR95 unserved critical-service energy than the best deterministic baseline on the sealed scenario suite, at equal or lower cost and with hard constraints unchanged.
- Lower decision regret than fixed/no-test policies where positive net VoI exists; abstention where the action is not identifiable.
- No loss of provider autonomy or unauthorized resource commitment.

These are future qualification gates. No SAFRB release has passed an industry head-to-head or calibrated South African field test.

## 10. Differentiation hypothesis

The wedge is not a faster simplex method or a visually realistic twin. It is an evidence and decision layer around established tools:

1. Reconcile source claims, physical measurements and operational state across organizations.
2. Detect shared failure domains and hidden evidence dependence.
3. Refuse to treat capacity rights, reservations, cargo status or crude stock as physical delivered diesel.
4. Give conventional solvers typed, traceable, time-aligned constraints.
5. Independently replay solution feasibility and mass/energy conservation.
6. Quantify tail risk by geography and service class.
7. Select the next measurement when it can change a decision.
8. Preserve human/provider authority and abstain when the model is incomplete.

This tournament is designed to falsify the hypothesis. If the layer does not improve feasibility, regret, service outcomes or auditability over simple baselines, it must not be described as an improvement.
