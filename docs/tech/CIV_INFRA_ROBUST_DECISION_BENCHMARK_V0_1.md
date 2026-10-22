# CIV-INFRA-ROBUST-001 — Robust Design and Decision-Focused Survey Planning

**Status:** Proposed benchmark contract, v0.1  
**Related implementation:** [CIV-INFRA synthetic corridor verifier #1756](https://github.com/Luminous-Dynamics/symtropy/pull/1756)  
**Related solver-evidence tranche:** [CIV-INFRA-HYDRO-001 #1764](https://github.com/Luminous-Dynamics/symtropy/issues/1764)  
**Tracking issue:** [#1769](https://github.com/Luminous-Dynamics/symtropy/issues/1769)

## 1. Thesis

A conventional design comparison asks: *which candidate has the best nominal metrics?*

A decision-grade system should also ask:

1. Which decisions remain valid across the uncertainty we have actually bounded?
2. Which unknowns can reverse feasibility, safety margin, or the Pareto ordering?
3. What is the cheapest/highest-impact next observation, survey, calibration or solver run?
4. What evidence would change the decision, and what evidence would not?

The paradigm shift is from **nominal optimization to auditable decisions under uncertainty**. This is not permission to invent probability distributions. Version 0.1 uses explicit intervals and/or finite enumerated states; probabilistic Value of Information (VoI) is a later capability with additional evidence requirements.

This contract extends current synthetic CIV-INFRA corridor work. It does not silently change the v0.1 fixture's meaning, upgrade synthetic results into engineering approval, or make solver claims before solver evidence is independently admitted.

## 2. Scope and non-goals

### In scope

- A typed, versioned uncertainty register.
- Explicit scenario/state coverage and dependencies.
- Robust, conditional, infeasible and unknown classifications.
- Uncertainty-aware multi-objective comparison.
- A decision-focused planner for proposed measurements/model runs.
- An exhaustive oracle for small finite benchmark cases.
- Adversarial tests for stale, missing, forged or unsupported evidence.

### Out of scope for R0

- A real corridor, survey, terrain model or permit decision.
- Assumed probability distributions or independence.
- A probabilistic risk number presented without calibration and provenance.
- Replacing specialist CAD/BIM, hydraulic, geotechnical or surveying tools.
- A single opaque score that hides trade-offs.
- Treating successful process exit as proof of a numerically credible simulation.
- Claiming global robustness outside the declared uncertainty set.

## 3. Typed uncertainty contract

Every uncertain quantity must have a stable identifier and, at minimum:

- **uncertainty_id** — stable, unique identifier.
- **quantity and unit** — explicit dimensional meaning.
- **support** — finite lower/upper bounds, a finite set of values/states, or explicit unbounded/unknown.
- **uncertainty_class** — measurement uncertainty, natural/aleatory variability, model-form uncertainty, scenario uncertainty, or unknown.
- **source_refs** — exact source artifact IDs and digests, not only human-readable filenames.
- **observed_at and valid_for** — acquisition time/version and applicable spatial, temporal or model scope.
- **calibration_state** — uncalibrated, calibrated-with-evidence, or not-applicable with rationale.
- **authority_state** — authoritative, provisional, or contextual.
- **dependency_group** — optional explicit joint-dependence group.

Invalid bounds (non-finite numbers, reversed intervals, incompatible units, impossible domain values), missing units, duplicate IDs, stale source bindings and out-of-domain evidence must be rejected or remain unknown. They must never be repaired silently.

**Unknown is not zero.** Unknown is not a midpoint, a convenient percentile, or a uniform distribution. A finite interval is only a bound if its evidence supports that claim. Unbounded unknowns block robust-feasibility claims whenever they can affect a required constraint or objective.

For R0, probability fields are disallowed unless paired with a versioned distribution specification, calibration evidence and an explicitly reviewed applicability domain. A probability attached to an uncalibrated interval must not be accepted as evidence.

## 4. State space and dependency semantics

The verifier operates on an explicitly identified uncertainty set W.

**R0 truth boundary:** the first implementation classifies supplied state-level constraint and objective records; those records are synthetic assertions, not outputs derived from primitive engineering variables. Its independent test oracle checks the finite classification semantics, not physical truth. Before interpreting robust labels as more than robust classification of supplied assertions, R1 must derive those records from primitive, typed inputs and a versioned ruleset with an independent reference implementation. This limitation is tracked in [CIV-INFRA-ORACLE-002 #1777](https://github.com/Luminous-Dynamics/symtropy/issues/1777).

R0 should support two forms:

1. **Finite state set:** each state has a stable ID, provenance, and values for all required uncertain inputs.
2. **Bounded intervals:** permitted only where the evaluator can soundly bound each constraint/objective over the full interval. Sampling endpoints or random points is not by itself proof that the interior is safe.

If correlations/dependencies are known, preserve the joint structure. Do not form all combinations of marginal intervals if that creates physically impossible combinations, and do not assume independence just because separate input distributions are available. Each excluded combination or constraint on combinations must be explicit and testable.

If the system cannot demonstrate that its scenario set covers the stated uncertainty contract, it must report **coverage_incomplete** and cannot label the result robust-feasible.

**R0 coverage rule:** for finite discrete supports with no modeled dependencies, the verifier must independently enumerate the Cartesian product, confirm that every joint tuple is present exactly once, and fail closed if the state product exceeds a documented resource cap. A Boolean claim such as `exhaustive_joint_state_set: true` is not evidence of completeness. R0 does not certify correlated/joint scenario sets yet; those require a future explicit dependency model and evidence-backed coverage contract. Bounded intervals remain unknown until a sound interval evaluator exists.

## 5. Candidate dispositions

For each candidate, evaluate every required constraint in every admitted state, preserving state-level findings and input provenance.

- **robust_feasible_for_declared_set:** every required hard constraint passes in every admitted state; all required evidence exists, is current and is within its applicability domain.
- **robust_infeasible_for_declared_set:** every admitted state violates at least one required hard constraint. Report each violation and state; do not overstate that a candidate violates the same constraint in every state unless that is shown separately.
- **conditionally_feasible:** at least one admitted state passes and at least one fails, or the result changes across admitted states. The decision depends on uncertainty resolution, mitigation or a policy choice.
- **evidence_inconsistent:** internal contradiction, digest mismatch, unsupported units, invalid bounds, or disagreement between declared and independently derived evidence.
- **unknown_review_required:** absent/stale/out-of-domain evidence, unbounded relevant uncertainty, incomplete scenario coverage, unsupported solver diagnostics or any state the evaluator cannot classify.

These are claims **about the admitted uncertainty set**, not about every possible real-world state. The report must state that claim ceiling. A large scenario count does not rescue a poorly justified scenario set.

## 6. Objective and Pareto semantics

Preserve each objective independently (for example route length, grade, cut/fill, minimum hydraulic margin, lifecycle cost, embodied carbon, maintainability and disruption). Each quantity declares whether lower or higher is preferred, units, evidence grade and uncertainty representation.

Report best/worst values or sound interval bounds, plus which states and evidence produce them. Do not compare unlike units without an explicit normalization contract.

For finite states, candidate A robustly dominates B only if, in every admitted comparable state, A is no worse on every declared objective and strictly better on at least one objective under the declared comparison semantics. If the ordering changes with state, report candidates as incomparable or state-dependent. Use shared state IDs for candidate comparisons; do not mix unrelated worst cases as though they were simultaneous.

No weighted scalar score is emitted by default. A scalar utility may be used only when the decision owner explicitly supplies objective priorities, normalization, hard constraints and utility function as versioned inputs. Preserve the Pareto set alongside any authorized scalar decision.

For minimax regret:
- R0 uses an exhaustive oracle in finite toy worlds.
- Scalar regret requires an explicit authorized decision function and declared scales.
- Otherwise emit a Pareto-regret vector or a set of non-dominated regret outcomes, not an invented scalar.
- State how ties, infeasible candidates and changing feasible sets are handled.

## 7. Decision-focused survey/model planning

A proposed action (utility locating, borehole, ground survey, calibration observation, rainfall sensitivity run, SWMM simulation, or independent review) must declare:

- stable action ID, cost/time units and source for the estimate;
- exactly which uncertainty IDs it can resolve or narrow;
- possible outcome branches and how each branch updates the uncertainty set;
- evidence quality and the scope where the result is valid;
- whether the measurement may introduce new unknowns or constraints.

R2 ranks actions by decision impact, not curiosity or raw information volume. For each action, compare the current decision/Pareto set with the possible post-observation sets. Report:
- whether robust feasibility changes;
- whether the nondominated set or candidate ordering changes;
- worst-case regret reduction, when regret is defined;
- best/worst case outcome and cost/time;
- compute and evidence assumptions.

Until probability distributions and outcome likelihoods are supported by evidence, name this **decision-impact ranking** or **worst-case decision-regret reduction per cost/time**. Do not call it expected VoI.

R3 may introduce probabilistic expected VoI only when there are versioned, evidence-backed priors, observation models/likelihoods, action costs, calibration evidence, and sensitivity analysis. Every probabilistic output must identify those inputs and their applicability domain. If they are absent, the probabilistic pathway must reject rather than fabricate a number.

## 8. Solver evidence is part of uncertainty and model credibility

For CIV-INFRA-HYDRO-001, solver success status is not a sufficient quality criterion. Bind exact model and scenario bytes to solver identity/build, options, command, logs, raw outputs and diagnostics. Parse and preserve status-report warnings, continuity error, relevant instability indicators, flooding/ponding and link surcharge. Separate:
- process execution success;
- artifact integrity/currentness;
- numerical adequacy for the intended question;
- calibration/validation status;
- engineering acceptance by a qualified reviewer.

EPA SWMM's User Manual describes continuity errors and warns that numerical instabilities may not be automatically identified; reported completion is therefore not equivalent to model credibility. See the [EPA SWMM page](https://www.epa.gov/water-research/storm-water-management-model-swmm) and [SWMM 5.2 User's Manual, including run status, continuity and instability discussion](https://nepis.epa.gov/Exe/ZyPURL.cgi?Dockey=P10145M6.TXT).

## 9. Benchmark design and measurable targets

### Frozen oracle corpus

The first public benchmark should include a small finite world with hand-checkable exhaustive outcomes, followed by generated but deterministically seeded cases. Include:
- one candidate robustly feasible across all admitted states;
- one robustly infeasible across every state;
- a candidate whose feasibility flips by state;
- shared dependencies that rule out impossible combinations;
- an unbounded utility whose location can change clearance;
- point estimates that appear to dominate but lose under uncertainty;
- a survey action that changes the feasible set;
- a survey action that adds data without changing the decision;
- unsupported probability inputs and uncalibrated distributions;
- stale digests, missing states, invalid bounds, non-finite values, duplicate IDs and forged reports;
- solver success paired with excessive continuity error/instability diagnostic, and a missing severe-storm case.

The test oracle must be a separate, straightforward exhaustive enumerator for the finite domain. It must not call the candidate classification, dominance or regret implementation as its own expected-value calculator.

### Targets for initial qualification

| Measure | Initial target |
|---|---|
| False robust-feasible labels against exhaustive oracle | **0** across frozen and generated finite cases |
| Robust classification, Pareto/dominance and configured regret agreement with oracle | **100%** on the declared finite benchmark corpus |
| Invalid bounds, stale/missing required artifacts, unsupported probability assumptions promoted to pass | **0** |
| Probability or expected-VoI number without versioned distribution/calibration evidence | **0** |
| Exact-byte replay for identical canonical inputs/evaluator/config/solver evidence | **100%** |
| Semantic results under candidate/state input permutation | **100% invariant** |
| Adversarial required-evidence omission / contradictory-artifact rejection | **100%** of frozen negative controls |
| Survey-action ranking vs exhaustive best-next-action oracle | **Exact agreement** on the finite corpus whenever objectives and costs are fully specified |
| Reports that preserve unresolved states, claim ceilings and evidence links | **100%** |

Before using a speed target, freeze a baseline and measure state count, input size, runtime and peak memory on documented hardware. Report scale curves and worst-case behavior; do not select a speedup number after seeing the result.

A minimum first acceptance corpus should include at least **100 deterministic bounded fixtures and 10,000 total finite state evaluations**, with each scenario/action identity and oracle result retained as a machine-readable artifact. Those counts are test-volume targets, not evidence of real-world representativeness. The R0 test now generates 625 deterministic fixture variants / 12,500 candidate-state evaluations and the dedicated CI workflow exports per-case input digests, oracle/evaluated classifications and Pareto sets as a 14-day artifact. This is an in-memory generated corpus with replay metadata, not yet 625 checked-in source fixtures.

### Scorecard outputs

Every benchmark run must record evaluator version, exact input/evidence hashes, uncertainty-set ID and digest, objective/constraint contract, state count, per-state findings, candidate classifications, Pareto results, regret method, survey-action ranking, runtime/memory, rejected evidence and claim ceiling.

Report false positives and false negatives separately. Never allow a performance optimization to reduce uncertainty coverage, erase negative controls or convert unknown into pass.

## 10. Staged implementation

- **R0 — specification and finite oracle:** freeze fixtures and semantics; no hydraulic solver needed. The current implementation supports only finite discrete supports and verifies the complete Cartesian product (up to a 100,000-state cap); correlated/joint supports and intervals fail closed until separately specified and soundly checked.
- **R1 — robust classification:** interval/state-set evaluation, state-level evidence, sound fail-closed statuses.
- **R2 — robust Pareto and decision-impact planner:** preserve objectives; exhaustive small-case action oracle; no assumed probabilities.
- **R3 — calibrated probabilistic VoI:** only after distribution, likelihood, cost and validation contracts exist.
- **R4 — join to solver evidence:** consume admitted SWMM runs under #1764; preserve numerical diagnostics and claim ceilings.
- **R5 — field pilot:** only with surveyed reference data, qualified domain review, calibration/validation evidence and explicitly bounded operational claims.

The harness must remain able to run R0/R1 tests quickly without waiting for a full hydraulic solver or GPU runner. A solver-backed result supplements the deterministic core; it does not weaken core evidence gates.

## 11. Research basis

NIST's 5 October 2026 report, [On Methods of Uncertainty Quantification in Computational Modeling and Simulation: Harmonizing Modeling and Metrology Practices](https://www.nist.gov/publications/methods-uncertainty-quantification-computational-modeling-and-simulation-harmonizing), discusses risk-based credibility assessment and scaling verification, validation and uncertainty quantification to model influence and decision consequence. This specification adopts that framing as a design principle, not as a claim of conformance.

EPA's [SWMM model and manuals](https://www.epa.gov/water-research/storm-water-management-model-swmm) provide the established hydraulic simulator. The proposed contribution is not to replace SWMM; it is to preserve evidence, uncertain inputs, quality gates and decision consequences across engineering alternatives and tool boundaries.

## 12. Acceptance gate

R0 is ready for review when:
- the state-space and disposition semantics are frozen;
- the exhaustive oracle is independently implemented;
- every benchmark case has an authored expected outcome and evidence digest;
- unknown, stale, contradictory and unsupported-probability negative controls are present;
- the report format exposes per-state findings and the complete claim ceiling;
- the above initial targets are executable in CI.

Until then, this is a proposed contract—not an implementation or engineering qualification.