# Energy Architect Benchmark Protocol v0.1

**Status:** proposed protocol; no benchmark results claimed.  
**Scope:** deterministic, synthetic-first evaluation of architecture search and its supported evaluation pipeline.  
**Authority boundary:** a benchmark pass is not physical qualification, safety certification, or permission to actuate equipment.

Related work:
- [Energy System Design Studio issue #2721](https://github.com/Luminous-Dynamics/symtropy/issues/2721)
- [Candidate contract PR #2722](https://github.com/Luminous-Dynamics/symtropy/pull/2722)
- [Symthaea candidate generator PR #8498](https://github.com/Luminous-Dynamics/symthaea/pull/8498)
- [Sol-Atlas candidate-to-dispatch adapter PR #314](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/314)
- [Sol-Atlas bounded exact dispatch oracle PR #309](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/309)
- [Sol-Atlas dispatch regret benchmark PR #311](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/311)

## 1. Research thesis

Do not benchmark an architect by how convincing its designs look or by the number of candidates it emits. Benchmark whether it emits **valid, useful, diverse and robust hypotheses** that survive evaluation better than transparent alternatives under equal information and compute budgets.

The target loop is:

1. **Propose:** Symthaea emits candidates, assumptions, and the experiments needed to discriminate among them.
2. **Reject cheaply:** structural, units, carrier, chronology, evidence and hard-constraint checks run before expensive simulation.
3. **Solve bounded subproblems:** Sol-Atlas compiles only what it can faithfully represent and uses exact known-answer/reference cases where complete enumeration is tractable.
4. **Simulate in scope:** Symtropy runs only the versioned energy/thermal models that actually implement the candidate's required physics.
5. **Try to falsify:** a separate evaluator re-computes invariants from raw candidate bytes and per-step outputs; adversarial cases try to expose false-feasible conclusions.
6. **Compare:** retain rejected candidates, counterexamples, objective vectors, uncertainty, and every baseline result.
7. **Learn without self-certification:** failure feedback can improve generation, but cannot change the independent evaluator's rules or erase failure records.

A candidate's plausibility, an optimizer's feasibility result, a digital-twin trajectory, and physical qualification are different claims and must remain different statuses.

## 2. Benchmark tracks

### Track A — Contract and falsification

Run positive and negative controls against the shared candidate contract and adapters. Include at least: incompatible carrier/unit; invalid port direction; disconnected source-to-storage path; chronology gap/overlap; missing evidence source; unsupported evidence promotion; missing-as-zero cost or emissions; impossible capacity/reserve; contradictory outage availability; unknown component; unsupported thermal/process path submitted to the electrical-only adapter; digest mismatch; and an unexecuted evaluation mislabeled as executed.

**Primary metric:** false-feasible rate. A known invalid case must never be reported as feasible. Report every rejected case and reason; rejection is not a successful result if the diagnostic is missing or materially misleading.

### Track B — Small exact dispatch

Freeze small integer-quantized electrical cases whose optimum can be exhaustively enumerated or independently proved. Include trivial known answers, ties, infeasible instances, boundary capacities, opening/terminal storage constraints, outage windows, and deliberate mapping failures.

Compare the Sol-Atlas result with a separately implemented enumerator or an independently checked reference result. Record the objective, bound, feasibility status, explored-state count and whether optimality was proved. A truncated search must report a bound/gap or **unknown**; it must not claim exact optimality.

**Acceptance gate:** all frozen small cases with complete enumeration recover the known optimum and satisfy the same invariants in the independent checker.

### Track C — Architecture-search quality

Compare, on the same case corpus and resource budget:

- **B0 — Current deterministic templates:** grid-only, PV + grid, PV + battery + grid.
- **B1 — Transparent engineering rules:** documented sizing and dispatch heuristics with fixed tie-breaking.
- **B2 — Seeded random valid search:** random/topology-constrained candidates, preserving the invalid-candidate denominator.
- **B3 — Search ablations:** remove one proposed Symthaea mechanism at a time only when that mechanism is implemented and measurable.
- **B4 — Full Symthaea search:** the same candidate/evaluation budget as baselines.
- **B5 — External optimizers:** REopt, PyPSA or GenX only on precisely matched formulations, inputs, constraints, objective boundaries and available information.

Do not compare B4 with an external tool that has a different model boundary and interpret the raw objective values as a win. External packages are problem-specific comparators, not interchangeable universal scorekeepers.

### Track D — Chronological service and resilience

For candidate classes supported by an appropriate dynamic twin, test ordinary operation plus predeclared events: grid loss, low renewable supply, demand peaks, component outage, storage derating, forecast error and delayed recovery. Preserve initial state, event timing, control policy, all raw time steps and terminal state.

Report at minimum:
- essential and flexible unserved energy (kWh) by scenario and interval;
- longest continuous essential-service deficit;
- time to restore the declared service floor;
- reserve/state-of-charge shortfall and terminal-state discrepancy;
- energy-balance residuals by interval and over the full horizon;
- scenario-wise cost/emissions only when the conversion factors and accounting boundary are explicitly supplied.

Do not run these claims through a generic rigid-body simulation and imply it is AC load flow or a qualified thermal/process model. If no suitable model exists, return **unsupported** / **not evaluated**.

### Track E — Uncertainty and held-out generalization

Separate the data into fixed development, calibration/validation, and held-out test sets. Split by scenario family or time period when possible, not merely by randomly shuffling correlated hourly samples. Keep hidden test seeds and scenario perturbations away from tuning. Include seasonal shifts, correlated low-renewable periods, demand changes, outages near peak demand, and explicitly out-of-domain cases.

Where a model emits intervals or probabilities, measure empirical coverage/calibration. Where the objective is operational loss, report worst-case and tail-risk metrics (including CVaR when defined) alongside expectation. Do not manufacture scenario probabilities when they are unknown; run unweighted stress cases and label them as such.

### Track F — Component/model cross-validation

Only activate this track for components with qualified model boundaries. For the supported battery subset, compare the simplified energy-state model to published/reference trajectories from NREL SAM where the input and model detail can be matched. Report error by time step, state of charge, charge/discharge losses, temperature and degradation only where both models actually represent those quantities.

Model agreement is evidence about the matched model boundary, not proof that the candidate is physically safe or representative of an operating site.

## 3. Metrics — never hide trade-offs in one score

| Dimension | Report |
|---|---|
| Structural correctness | valid / invalid counts; reason-coded rejection counts; false-feasible rate |
| Exactness | known-answer recovery; proof status; optimality gap/bound; independent-check agreement |
| Search utility | best feasible objective vector; regret vs exact reference where defined; Pareto coverage/hypervolume with a frozen reference point |
| Diversity | unique normalized topologies; structural distance; component-family and policy coverage; duplicate rate |
| Service | essential and flexible unserved kWh; deficit duration; outage survival and recovery trajectory |
| Physical consistency | per-step and horizon energy-balance residuals; storage state violations; conservation-audit disagreements |
| Uncertainty | scenario-wise distribution; worst case; CVaR when probability weights are defensible; sensitivity to uncertain inputs |
| Evidence quality | sourced vs assumed parameters; missing/stale evidence; unsupported objectives; claim ceiling |
| Efficiency | wall time, CPU time, peak memory, candidate count, simulator calls, search expansions, and measured energy-to-compute when available |
| Reproducibility | exact input/output hashes, source commit, schema/model/solver versions, seed, environment and command line |

Keep hard constraints as gates, not as weighted penalties that a low cost can compensate for. Publish Pareto sets and per-objective values. Freeze hypervolume reference points before evaluating held-out candidates; never choose them after seeing which method wins.

Do not present structural diversity as useful innovation unless the distinct candidates are feasible or lead to distinct, informative trade-offs. Do not omit failures from the denominator.

## 4. Initial acceptance gates and scale-up

These are proposed gates, not measured results:

1. **Contract gate:** the independent test harness rejects every seeded negative control and never labels an unexecuted evaluation as executed.
2. **Exact gate:** 100% known-answer recovery on the frozen complete-enumeration suite; any unresolved case is explicitly **unknown**, not a pass.
3. **Receipt gate:** identical inputs, pinned versions, and seed reproduce the declared deterministic outputs; all reports bind hashes to the exact bytes consumed.
4. **Baseline gate:** every search comparison includes B0, B1 and B2 under identical generation/evaluation budgets before a claim is made about B4.
5. **Held-out gate:** report development and held-out results separately; no benchmark tuning on the held-out set.
6. **Claim gate:** no superiority claim unless the predeclared, matched comparison shows improvement on its named metric(s), retains hard-constraint correctness, includes uncertainty/confidence intervals, and reports regressions on other objectives.
7. **Scale gate:** begin with hand-checkable cases; then add a deterministic 1,000-case regression/fuzz corpus; grow to 10,000 or more generated cases only once case generation, oracle semantics, runtime budget and failure triage are stable. Persist minimized counterexamples as fixed regression cases.

The zero-false-feasible target applies to the finite synthetic and qualified-model suite. It is not a statistical guarantee of zero failures in the real world.

## 5. Paradigm shift: counterexample-guided, multi-fidelity design

The most promising research differentiator is a **counterexample-guided energy architect** rather than an unconstrained generative search:

- The architect proposes multiple structurally distinct candidates.
- Cheap validators find contract-level counterexamples before expensive work.
- The exact oracle challenges representable subproblems.
- Symtropy explores time-coupled operation and fault recovery only where model scope supports it.
- The independent auditor extracts a minimal counterexample: violated invariant, time interval, component, input digest and evaluator version.
- Symthaea may revise its next proposal based on that failure, but must retain the old candidate and failure receipt.
- Experiment selection prioritizes the test expected to change the decision most per unit of evaluation cost, while exposing uncertainty and the basis for that estimate.

Use multi-fidelity evaluation: schema checks → analytical bounds → exact small-case oracle → supported twin → externally validated model → hardware-in-the-loop / field evidence. Escalation is earned by evidence, not inferred from a candidate's confidence. Keep each fidelity level's claim ceiling explicit.

Ablate HDC, Phi, free-energy, or other Symthaea-specific mechanisms individually. They earn a role only if controlled, matched experiments demonstrate improvements in valid-candidate yield, Pareto utility, robustness, information gained, or compute efficiency. They are not substitutes for conservation laws, evidence quality, or engineering constraints.

## 6. Comparison references and their limits

- [NREL REopt API](https://developer.nlr.gov/docs/energy-optimization/reopt/) — building, campus, community and microgrid technology sizing/dispatch for cost, resilience and energy performance.
- [PyPSA documentation](https://docs.pypsa.org/latest/) — power and multi-energy systems, storage, network approximations and long time series. Its [stochastic optimization example](https://docs.pypsa.org/latest/user-guide/optimization/stochastic/) is relevant to scenario-based planning and risk preferences.
- [GenX model introduction](https://genxproject.github.io/GenX.jl/stable/Model_Concept_Overview/model_introduction/) — co-optimization of capacity expansion, dispatch, unit commitment, reserves, transmission and policy constraints.
- [NREL SAM battery storage models](https://samrepo.nlr.gov/help/battery_storage.html) — detailed battery behavior including voltage, temperature and degradation within the supported configurations.
- [HELICS documentation](https://docs.helics.org/en/latest/) — scalable multi-domain co-simulation; it is an integration reference, not a design optimizer.
- [ARPA-E Grid Optimization Competition Challenge 1 dataset](https://catalog.data.gov/dataset/arpa-e-grid-optimization-go-competition-challenge-1) — an example of explicit problem formats, operating scenarios, strict solver budgets and separately checked feasibility for security-constrained AC optimal power flow. Its objective and physics are not directly comparable to the synthetic site-microgrid dispatch benchmark.

Record reference-tool version, formulation, solver, inputs, options and execution budget. Unsupported features or unmatched boundaries are documented as non-comparable instead of silently simplified.

## 7. Required report artifact

Every run should emit a machine-readable report containing:
- benchmark protocol version and case-suite digest;
- exact generator, verifier, schema, twin, optimizer and reference-tool versions;
- input/candidate/config/output SHA-256 digests;
- seed and compute budget;
- status per case: **pass**, **fail**, **unsupported**, **not evaluated**, or **unknown**;
- raw result references plus independently recomputed metrics;
- infeasibility/counterexample reasons;
- Pareto set and objective-coverage table;
- claim ceiling and explicit list of evaluations not executed.

A summary status must fail closed: any missing required case, digest mismatch, unsupported hard constraint, or absent independent audit prevents an overall **pass**. Queued CI is not a result. A successful test on a PR head qualifies only that exact head SHA.

## 8. Exit criteria for v0.1

The protocol is ready to move from design to implementation when:
- the shared candidate contract is pinned;
- the current exact-dispatch fixture and its expected result are frozen;
- B0/B1/B2 baselines are documented;
- an independent checker and machine-readable report contract exist;
- the first negative-control corpus is executable offline with pinned fixtures;
- the report distinguishes unsupported from failed and unexecuted from passed.

This document is a protocol proposal. It adds no simulator, does not claim a benchmark has run, and makes no claim that Symthaea beats current industry tools.
