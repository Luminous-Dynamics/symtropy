# Spatial Resilience Trajectory Benchmark v0.1

**Status:** research contract and synthetic evaluator fixtures; not a building-performance model.  
**Owner issue:** [SPATIAL-RESILIENCE-001 #1848](https://github.com/Luminous-Dynamics/symtropy/issues/1848)  
**Parent:** [SPATIAL-DESIGN-001 #1766](https://github.com/Luminous-Dynamics/symtropy/issues/1766)  
**Manifest:** `SPATIAL_RESILIENCE_TRAJECTORY_FIXTURES_V0_1.json`  
**Schema:** `SPATIAL_RESILIENCE_TRAJECTORY_FIXTURES_V0_1.schema.json`
**Timeline oracle corpus:** `SPATIAL_RESILIENCE_TIMELINE_ORACLE_V0_1.json` (8 deterministic event sequences).
**Timeline schema:** `SPATIAL_RESILIENCE_TIMELINE_ORACLE_V0_1.schema.json`.
**Paired outcome corpus:** `SPATIAL_RESILIENCE_PAIRED_OUTCOMES_V0_1.json` (5 known-answer comparisons).
**Paired outcome schema:** `SPATIAL_RESILIENCE_PAIRED_OUTCOMES_V0_1.schema.json`.
**Decision-impact corpus:** `SPATIAL_RESILIENCE_DECISION_IMPACT_V0_1.json` (8 finite-state worlds).
**Decision-impact schema:** `SPATIAL_RESILIENCE_DECISION_IMPACT_V0_1.schema.json`.

## 1. Research question

Compare design alternatives as **time-varying habitability and resident-defined service trajectories**, not just static geometry, annual average energy, or rendered appearance. A design is a candidate to evaluate; it is never itself evidence that a service or physical condition will be delivered.

This first tranche freezes the evaluator's expected handling of outages, recurrence, repair delays, changing resident tasks, uncertainty, missing solvers, and provenance. It does not yet simulate heat transfer, airflow, thermal comfort, building energy, water systems, structural behavior, or life safety.

## 2. Compose existing authority

This contract connects rather than replaces existing work:

- Spatial candidates and residents' stated task requirements: [SPATIAL-DESIGN-001 #1766](https://github.com/Luminous-Dynamics/symtropy/issues/1766) and [ACCESS-HOME-001 #1687](https://github.com/Luminous-Dynamics/symtropy/issues/1687).
- Deterministic event timelines, resource conservation and service trajectories: [SETTLEMENT-TIME-001 #1739](https://github.com/Luminous-Dynamics/symtropy/issues/1739).
- Common exogenous scenarios and fair paired policy comparison: [SETTLEMENT-POLICY-001 #1752](https://github.com/Luminous-Dynamics/symtropy/issues/1752).
- Solver input/result contract and independent reference validation: [built-environment adapters #1790](https://github.com/Luminous-Dynamics/symtropy/issues/1790).
- Evidence validity envelopes and dependency-aware invalidation: [HABITAT-EVIDENCE-003 #1846](https://github.com/Luminous-Dynamics/symtropy/issues/1846).
- Exact geometry is not admitted until the provider gate in [PB-04d0 #683](https://github.com/Luminous-Dynamics/symtropy/issues/683) is satisfied.

The initial fixture checker should not create a competing timeline engine, geometry authority, resident accessibility workflow, validity engine, or building-physics solver.

## 3. Benchmark unit

A benchmark run consists of:

1. immutable candidate and requirement revisions;
2. an exact shared exogenous-trace identity for all candidates in a paired comparison;
3. an observation contract specifying what was available, when, and with what uncertainty;
4. a model/solver receipt with declared scope, inputs, units, version, convergence, warnings and exclusions;
5. a time-indexed outcome vector and case-level disposition;
6. an independent checker result and content hashes.

Keep **exogenous events**, **observations**, **policy actions**, and **realized outcomes** separate. A candidate or policy cannot know future disturbances by default. A missing solver result is unknown/blocked, not zero consumption, zero discomfort, or successful service.

## 4. Required outcome vector

Where valid model outputs exist, report separately:

- resident-defined essential-task and service-floor hours preserved, retaining per-task and per-person breakdowns;
- thermal exceedance degree-hours/duration under a declared applicable profile;
- time to first degradation, reserve exhaustion, first restoration and stable recovery;
- energy/water per delivered service unit and reserve autonomy under the declared demand trace;
- operating cost and embodied/operational carbon only when the data and calculation methods are declared;
- fault detection, service access, part delay, repair and re-verification duration;
- design-change cost, disruption, reversibility and preservation of resident-stated requirements;
- uncertainty/scenario spread, calibration domain, solver status and unmodeled factors.

Thermal outputs are not medical or life-safety determinations. Cost, carbon, comfort, aesthetics or nominal efficiency cannot compensate for a hard-floor violation. Do not collapse individual resident task outcomes into an average that hides a blocked person. Preserve Pareto alternatives and explain trade-offs instead of selecting a universal scalar winner.

## 5. Synthetic fixtures in v0.1

The JSON manifest contains 15 contract cases. These are evaluator fixtures, not evidence of model accuracy:

- static ranking reversal under a declared outage trace;
- missing solver and incomplete model domain;
- stale weather or geometry invalidation;
- unit mismatch rejection;
- recurrence between first restoration and stable-recovery window;
- missing repair part;
- unequal exogenous traces in a paired comparison;
- future-event leakage to a policy;
- forbidden hard-floor compensation by a soft objective;
- uncertainty outside a preregistered stability tolerance;
- a resident-stated task change that requires reopening the design decision;
- a service panel/repair-access defect;
- duplicate event delivery and deterministic replay;
- scoped invalidation after a causal input change;
- preservation of unresolved dissent and separately held preferences.

Each fixture specifies invariant IDs, a negative-control mutation and the expected disposition. Expected dispositions assert **how the evaluator must handle a case**, not physical performance values.

## 6. Independent oracle and negative controls

The standalone Rust qualifier executes the separate eight-case timeline corpus against a hand-checkable finite-state recovery oracle. Cases freeze recurrence, boundary-time loss, duplicate delivery, conflicting event identity, ambiguous same-tick ordering, out-of-horizon events, zero-length windows and tick-overflow handling. The case corpus and its schema are separate from the spatial-design contract manifest so timeline truth tables can be reviewed directly. The oracle's claim remains limited to synthetic event semantics.

The future Rust-first verifier should:

- validate the manifest against its JSON Schema and enforce semantic uniqueness of case IDs;
- require exact candidate revisions and the same exogenous trace identity in paired comparisons;
- reject malformed units, missing required metadata, invalid event ordering and forbidden look-ahead;
- verify stable-recovery logic against a deliberately simple independent finite-state oracle;
- distinguish stale, out-of-scope, missing, contradictory and unknown evidence;
- block hard-floor failures before Pareto ranking; soft-objective scores never turn an infeasible candidate feasible;
- preserve candidate-level and resident-task-level outcomes and dissent;
- emit a structured, deterministic report with stable failure codes and exact source digests.

Mutations must be tested one at a time and in important combinations. The system under test must not author its own expected answers, oracle, held-out set, or validation tolerances.

## 6a. Paired-outcome oracle: hard floors before Pareto ranking

The standalone verifier also executes five known-answer paired comparisons from the separate paired-outcome corpus. Each candidate must bind to the same exogenous trace ID **and revision** as the comparison. The candidate must report exactly one outcome for every declared resident/task service floor. No per-household aggregation is used to hide a requirement-specific service failure.

Admission order is fixed for v0.1:

1. Reject the entire paired comparison when candidate trace identities/revisions do not match the declared shared trace.
2. Block candidates whose evidence state is missing, stale, contradictory or out-of-domain.
3. Exclude candidates with a declared hard-constraint failure.
4. Exclude candidates that fail any individual resident/task service floor.
5. Exclude candidates that exceed the declared uncertainty-width tolerance.
6. Compute the Pareto frontier only among the remaining candidates: maximize each declared service's available hours and minimize synthetic energy units, repair ticks and cost units.

Dominance requires being no worse in every declared dimension and strictly better in at least one. Two distinct candidates with exactly equal metric vectors remain tied alternatives; the verifier does not invent a tie-breaking scalar. These integer values are intentionally synthetic, so passing this oracle qualifies evaluator semantics only.

The corpus includes negative controls for an energy-efficient option that fails an individual resident service floor, a candidate with attractive expected values but uncertainty beyond tolerance, a trace/revision mismatch, stale evidence, a non-passing hard constraint, a dominated candidate, and an exact Pareto tie.

## 7. Evaluation gates

### Gate A — contract mechanics

- 100% schema and semantic validation of the frozen fixtures.
- 100% deterministic replay on explicitly deterministic cases.
- 100% agreement with an independent exhaustive/finite-state oracle on tiny known-answer event sequences.
- Zero missing/out-of-domain solver results reported as passes.
- Zero hard-floor violations hidden by a weighted average.
- 100% detection of seeded stale revision, unit, trace identity, event-order, missing-input and solver-status mutations.
- 100% duplicate delivery idempotence where the contract declares it; order-sensitive cases require an explicit tie-break.

### Gate B — model adapter

Add one qualified thermal/energy adapter only after comparison with independently validated reference cases. Report model/version, exact geometry/context inputs, weather and operating schedules, calibration scope, units, numerical/convergence status, uncertainty, exclusions and held-out results. Establish model-specific acceptance tolerances before evaluation; there is no universal tolerance for every building-performance metric.

### Gate C — design-quality study

Run at least 12 candidate configurations and matched stress traces, then scale to multiple climate/use profiles. Include fixtures where the static ranking reverses under outage or heat stress, where a repairable alternative recovers faster, and where a resident-stated requirement changes the acceptable design set. Freeze a held-out set before tuning. Report the Pareto set, per-objective spread, scenario failures, uncertainty and abstention; do not use one cross-domain score.

### Gate D — field evidence

Only after Gate B: compare predictions against measured performance where valid data exists, then preregister professional and resident-led task studies with consent and accessible study design. Report outcomes and disagreement by relevant cohort. No synthetic benchmark score establishes occupant benefit.

## 8. Research basis

- U.S. DOE, [About Building Energy Modeling](https://www.energy.gov/cmei/buildings/about-building-energy-modeling): whole-building energy modeling requires geometry, construction, systems and controls, occupancy/use schedules and local weather.
- Amiri, Jemtrud & Chung (2025), [thermal resilience under power outage and varying climates](https://doi.org/10.1016/j.enconman.2025.121013): studies the divergence among passive survivability, comfort and energy objectives during outages.
- Caieiro da Costa et al. (2026), [multi-objective thermal resilience in multifamily housing](https://doi.org/10.1016/j.enbuild.2025.116886): evaluates resistance, robustness and recovery separately under heatwave scenarios.
- Martin-Goñi & Paris-Viviana (2026), [adaptability assessment for residential renovation](https://doi.org/10.1016/j.enbuild.2026.117654): motivates explicit functional/technical adaptability and preserving the value of existing buildings.

These sources motivate the benchmark but do not validate any Symthaea/Symtropy implementation.

## 9. Claim ceiling

Passing Gate A proves only that the checker handles the declared synthetic contracts. It does not establish actual building energy use, thermal comfort, health outcomes, water reliability, structural adequacy, accessibility, safety, code compliance, construction feasibility or measured occupant benefit. Those claims require the relevant qualified model, evidence, independent validation and professional authority.


## Research update — time-dependent resilience and passive survivability (2026-10)

Two recent sources reinforce the benchmark's choice to model trajectories, not an isolated comfort/efficiency scalar:

- Gortych, Kuczyński & Bocheńska-Skałecka (2026), *Thermal Resilience of Residential Buildings Under Climatic Extremes and Power Outages: An Integrated Review of Metrics, Passive Mechanisms, Energy Systems, and Design Frameworks*, `Energies` 19(13), 3006, published 24 June 2026: https://doi.org/10.3390/en19133006. The review describes resilience evaluation as time-dependent and notes that extreme-event definitions, metrics, passive mechanisms and energy-system effects are often handled separately.
- Pulkkinen & Ramesh (2026), *Passive resilience solutions and their impact on a cold climate building during a power outage*, `Energy and Buildings`, published 1 September 2026: https://doi.org/10.1016/j.enbuild.2026.117628. Its study of cold-climate residential outage conditions emphasizes the role of passive renovation and says recovery depends on initial/developing indoor and outdoor conditions and heating-system size.

### Benchmark implication

The first real-model adapter should produce a **time series with declared boundary conditions**, rather than a single pass/fail or annual average. Qualification should measure (1) time to a declared service/condition threshold, (2) duration and depth of excursions, (3) resource/reserve exhaustion, (4) response after service restoration and (5) sensitivity to initial conditions. Report these separately for each qualified scenario. This is a research direction, not a claim that the present synthetic oracle calculates temperatures or models human thermal physiology.

The initial Rust oracle intentionally proves only finite-state event semantics around continuous service and stable recovery. It is a small contract check to prevent one class of reporting error; it is not an approximation of a heat-transfer solver.


## Decision-impact measurement selection — information must change a decision

The paired-outcome evaluator is followed by a measurement-planning layer based on [SPATIAL-VOI-001 #1892](https://github.com/Luminous-Dynamics/symtropy/issues/1892) and [CIV-INFRA-DECISION-001 #1789](https://github.com/Luminous-Dynamics/symtropy/issues/1789). The principle is **not** “maximize available data.” It is “identify the lowest-cost admissible observation that resolves decision-relevant uncertainty, and stop when additional measurement cannot change the defensible decision.”

Three research findings motivate this distinction:

- Langtry & Choudhary (2025), *Quantifying the benefit of load uncertainty reduction for the design of district energy systems under grid constraints using the value of information*, *Applied Energy* 400, 126549, https://doi.org/10.1016/j.apenergy.2025.126549. In their district-energy case study, monitoring-informed design reduced overall costs by less than 1.5% on average, below the measurement cost; reduction of mean-load uncertainty captured most of the available decision-support benefit.
- Quesada-Allerhand et al. (2025), *A combined approach to Measurement-System-Design and system identification in building energy retrofits*, *Energy and Buildings* 338, 115711, https://doi.org/10.1016/j.enbuild.2025.115711. Their case study reports that measurement selection can reduce parameter uncertainty with a subset of available measurements, supporting decision-aware sensor placement rather than “measure everything.”
- NIST AMS 100-85 (5 October 2026), *On Methods of Uncertainty Quantification in Computational Modeling and Simulation*, https://doi.org/10.6028/NIST.AMS.100-85, motivates scaling credibility and uncertainty work according to model influence and decision consequence.

These results are case-specific, not universal cost thresholds. They motivate an explicit benchmark for when a measurement changes a decision.

### v0.1 finite-state rules

The machine-readable decision-impact corpus freezes eight hostile cases. For each world, it defines current possible states, unresolved critical unknowns, candidates with explicit state-conditional feasibility and objective vectors, and proposed measurement actions with source revision, evidence state, cost, duration, unknowns resolved and outcome partition.

- Every action outcome must be non-empty, mutually exclusive, and collectively cover the current possible state set exactly. An unknown state, overlapping/missing membership, stale evidence revision or mis-scoped claimed unknown is rejected. Each action also carries explicit source IDs for its estimated cost and duration; v0.1 basis IDs are clearly synthetic.
- Candidate admission is recomputed for each outcome. A candidate is robust-feasible only if it passes every declared hard constraint across every state remaining possible **and** no critical unknown remains unresolved.
- Pareto objectives use worst-case state envelopes: service hours are minimized across the surviving states; energy, repair time and cost are maximized. The evaluator does not average away a failing state.
- The action result reports whether the hard-feasible set changes, whether the Pareto set changes, the worst-case residual-state count, newly robust-feasible candidates, unresolved unknowns and cost/time. It does not emit probabilities or expected value of information.
- A transparent optional lexicographic order first prioritizes actions that resolve declared critical unknowns, then actions that change hard feasibility, then those that change the Pareto set, then lower worst-case residual-state count, cost, duration and stable action ID. **Only actions that change the robust-feasible or Pareto set enter the recommendation order**; valid but non-decision-relevant actions remain visible in the report but are not recommended. The vector remains visible so the ordering policy can be reviewed or changed.
- A measurement that divides the finite state set but leaves all robust/Pareto decisions unchanged is explicitly *not decision-relevant*. A survey that clears one unknown cannot promote a candidate while a separate critical blocker remains unresolved.

The first targets are exact agreement with an independent exhaustive oracle on the finite cases; 100% rejection of invalid partitions and stale/mis-scoped actions; zero robust-feasible promotions while critical unknowns remain; deterministic results under input ordering changes; cost/time basis IDs preserved in the result; no recommendations for actions that cannot change the robust-feasible or Pareto set; and no probabilistic VoI without a separately versioned prior/likelihood/calibration scope.

All v0.1 quantities and states are synthetic. This does not prove a real survey is accurate, that a measurement is economically worthwhile, or that a candidate building is feasible. Real measurement planning requires qualified observation models, evidence provenance, justified cost/time estimates, and—if expected VoI is claimed—validated probabilities and applicability scope.

## Qualification direction — established VVUQ tests before new optimisation claims

Prefer a mature, externally tested building simulation engine over a fresh home-grown heat-transfer model. For whole-building simulation tools where applicable, the first adapter plan should reference relevant cases from **ANSI/ASHRAE Standard 140**, which ASHRAE describes as a method for testing the technical capabilities and applicability of building thermal/HVAC simulation software, including weather-file interpretation tests in its 2023 edition: https://www.ashrae.org/technical-resources/bookstore/ashrae-building-energy-simulation-and-performance-modeling-resources

Passing Standard 140 cases is not equivalent to validating a specific building, site weather file, control sequence or outage scenario. Keep those separate: (a) software capability tests, (b) independent analytical/reference-case comparison, (c) input/model calibration and validation against measured behaviour where available, and (d) decision-specific uncertainty and extrapolation limits.

NIST AMS 100-85 (published 5 October 2026) recommends risk-proportionate credibility assessment based on model influence and decision consequence, and distinguishes variance propagation from distributional propagation: https://doi.org/10.6028/NIST.AMS.100-85. The design pipeline should therefore require stronger evidence when a prediction can change a consequential admission decision, and must disclose when a result lies outside the demonstrated model domain.

This is why the v0.1 qualifier keeps synthetic known-answer mechanics, specialist solver outputs, professional review and real occupant studies as separate evidence layers.


Decision-impact receipts bind each observation outcome to its exact compatible state IDs, rather than reporting only the number of remaining states. Action outputs retain action type, evidence status, source revision and the declared cost/duration basis identifiers. A frozen negative control changes a one-state outcome to another one-state outcome; the validator must detect the wrong membership even though the cardinality stays unchanged.


The Rust decision-impact test suite includes a deterministic generated differential check over 100 finite worlds with 100 candidates each. For each world it compares robust-feasibility and Pareto sets over the complete current state set and each one-state-excluded subset against a separately written brute-force reference, including periodic critical-unknown blockers. The generator covers 35,000 candidate/state result rows before partition replays and asserts a minimum threshold of 10,000. This is a finite-domain correctness stress test; it is not a runtime-performance, physical-accuracy or probability-calibration claim.


Additional decision-impact negative controls inject duplicate state membership within one outcome and an undeclared state ID. Duplicate membership is rejected by the schema; undeclared membership is rejected semantically and causes the full partition to fail closed. These cases guard the distinction between a partition's cardinality and its exact, valid state membership.


The planner reports three separate forms of decision impact: a change to the hard-feasible candidate set, a change to the Pareto set, and a change to the set of unresolved critical blockers. An action that clears one blocker may be the correct next step in a staged survey plan even when another blocker keeps the robust-feasible set empty. The output marks that blocker-set change explicitly; it must not mislabel the candidate as feasible or Pareto-admitted until all critical blockers are resolved.


## Adaptive measurement policies — conditional follow-up and explicit stopping

The next extension, [SPATIAL-ADAPTIVE-001 #1922](https://github.com/Luminous-Dynamics/symtropy/issues/1922), upgrades a ranked list of possible measurements to an auditable policy tree: **observe → recompute the decision → select a conditional next action or stop/escalate**. It composes the finite-state evaluator above and does not introduce a second generic value-of-information engine.

This is supported as a research direction by two recent strands of literature:

- Quesada-Allerhand et al. (2025), *A combined approach to Measurement-System-Design and system identification in building energy retrofits*, *Energy and Buildings* 338, 115711, https://doi.org/10.1016/j.enbuild.2025.115711, jointly considers measurement type, location and timing; its case study reports useful system identification with half of the available measurements.
- A July 2026 *Energy and Buildings* study, *Informative yet compact monitoring for building retrofits: Joint information entropy–based measurement system design applied to a real office building*, https://doi.org/10.1016/j.enbuild.2026.117546, reports comparable identification with 45% of available measurement devices for its particular office-building case.
- Cheng & Huan (2025, revised June 2026), *Optimal stopping for sequential Bayesian experimental design*, https://arxiv.org/abs/2509.21734, treats next-experiment selection and stopping as coupled decisions. Its probabilistic framing motivates the policy-tree shape; this v0.1 implementation deliberately does not use probabilities.

These findings are case-specific, not universal savings estimates. The benchmark's own synthetic cost ratios are mechanics fixtures only.

### v0.1 tree contract

Every bounded, acyclic policy has stable nodes. A measurement node references a declared action with valid source revision and cost/duration basis IDs. For every outcome that remains possible after the parent branch, the tree must provide exactly one child. A child cannot branch on an outcome whose compatible states were already eliminated. Every node must be reachable, and the declared depth/action/cost/duration budgets are hard constraints.

Terminal dispositions are explicit:

- **stop_decision_stable**: no critical blocker remains, at least one robust-feasible candidate exists, and each remaining possible state yields the same robust-feasible and Pareto sets;
- **reopen_design**: no critical blocker remains and no robust-feasible candidate exists;
- **escalate_blocker**: a declared critical blocker remains unresolved.

A failed stop condition is not converted into a probability or a confidence score. It is a rejected policy. Worst-case cumulative cost and duration are evaluated over all reachable branches; no expected or average costs are invented.

### Initial known-answer policy benchmark

The frozen adaptive corpus includes a two-step policy that pays for a cheap clearance survey first and requests a structure probe only on the branch where it can change the robust/Pareto decision. On identical synthetic state/candidate obligations, its worst-case cost is 5 arbitrary cost units and duration is 3 ticks, versus 10 units and 4 ticks for a universal scan. It also includes (1) a policy that tries to stop while a critical fire-separation blocker remains, and (2) a child branch that references an outcome excluded by its parent. The latter two must be rejected or escalated, never silently accepted.

Acceptance targets are 100% agreement with an independent exhaustive tree oracle across frozen worlds, 100% rejection of malformed or unreachable branches, zero decision-stable leaves with unresolved critical blockers, deterministic outputs under input-order permutations, and exact accounting against per-path budgets. Next scale target: at least 100 adaptive worlds and 10,000 policy-node visits before any performance/generalization claim. Probabilistic expected VoI remains a separate future contract that needs justified priors, observation likelihoods, calibration and applicability scope.
