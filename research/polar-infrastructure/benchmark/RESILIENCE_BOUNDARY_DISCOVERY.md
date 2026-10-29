# Resilience Boundary Discovery and Counterexample Protocol

**Status:** proposed experiment design; not an implemented search engine or validated physical model. **Current evidence ceiling remains the scorer's synthetic fixtures.** No site, controller, or physical asset has been validated by this protocol.

## 1. Research shift: from a fixed test list to a measured failure boundary

A fixed stress suite can demonstrate how a system responds to its listed cases, but it cannot show how close an untested system is to failure. A stronger question is:

> Under a declared design envelope and owner-approved service contract, what is the least severe observed disturbance trajectory that breaches each critical-service floor, and how does that boundary change across policies, model uncertainty, and recovery state?

This is a **resilience-boundary** experiment. It complements the trace scorer: a suitable simulator or lab harness produces trajectories; the scorer measures delivered services; this protocol organizes search, comparison, and evidence. The scorer alone is not a simulator or search engine.

The idea is informed by established cyber-physical-system stress testing, which seeks counterexamples near the limits of design assumptions, and by outage-duration research that asks how long critical loads remain supportable. The proposed direction adapts those ideas to **multi-service, stateful, compound microgrid hazards**, preserving service-specific hard gates and avoiding one universal severity score. This is a research hypothesis, not a claim of methodological novelty or superiority. See [Stress Testing Control Loops in Cyber-physical Systems (ACM TOSEM, DOI 10.1145/3624742)](https://doi.org/10.1145/3624742) and [Quantifying Microgrid Resilience by Determining Survivable Durations of Random Outages (IEEE Access, 2025)](https://doi.org/10.1109/ACCESS.2025.3626579).

## 2. Define the search object without inventing a universal risk score

Represent each test as a versioned trajectory with:

- **Initial state:** storage state of charge and temperature, fuel and spare inventory, thermal mass, component condition, active service demand, and other modeled carry-over state.
- **Environmental trajectory:** sourced or explicitly synthetic wind/resource availability, ambient temperature, icing/derating, load, and forecast error.
- **Disturbance sequence:** event identities, start times, duration, overlap/order, restoration time, and shared dependencies. Examples include low renewable production plus cold battery derating plus delayed resupply; or stale telemetry plus actuator mismatch plus a second fault during recovery.
- **Controller information:** observations and forecasts actually available at each decision time. Hidden ground truth is for evaluation only and must not leak into policy inputs.
- **Model and evidence basis:** simulator/testbed version, parameter set, calibration/validation split, source digests, clock and unit conventions, and known omissions.

Every disturbance parameter needs a bound and provenance. A parameter without a defensible field/model bound may be explored as an explicitly synthetic stress dimension, but its result cannot be relabeled as site risk.

### Use a partial order, not a magic severity scalar

Within one hazard family and fixed comparison boundary, declare which parameter vectors are comparable. A longer outage may be more severe when other conditions and event timing are fixed. But a longer outage with milder cold, an earlier outage with lower storage, and a shorter outage that disrupts black start may be **incomparable**. Do not force them onto a single weighted severity score.

Report the Pareto-minimal **observed failing trajectories**. A failure is dominated only if another observed failing trajectory is no more severe on every declared comparable dimension and strictly less severe on at least one. Keep distinct hazard families or incompatible initial states separate. A minimal observed counterexample is not proof that no milder undiscovered failure exists.

For each service, retain the owner-approved hard gate and exact unit/time basis. The failure predicate is a vector: which floors failed, when, for how long, by how much, and whether an invalid command or another separately defined constraint was violated. A system-wide pass/fail may summarize the vector but must not hide it.

## 3. Search design

Preregister the following phases before examining candidate results.

### Phase A — model and contract readiness

1. Approve service floors and engineering limits with the accountable owner/reviewer; never tune them to make a method pass.
2. Verify units, conservation/balance residuals, time alignment, initial-state rules, controller-visible versus ground-truth signals, and state carry-over.
3. Separate calibration, tuning/search, validation, and final holdout by independent episode, weather block, or site as appropriate. Adjacent windows from one storm or outage must not be scattered across splits.
4. Record model uncertainty and run sensitivity over supported parameter sets. If relevant dynamics or a failure mechanism are absent, mark the test dimension **blocked by model** or **not covered**, not passed.

### Phase B — common baseline suite

Run every eligible method on the same preregistered nominal and fault episodes. Include a documented incumbent/operator policy, transparent rule-based policy, LP/MILP or MPC baseline where its assumptions support a fair comparison, and robust/scenario method only where its uncertainty representation is justified. Include candidate ablations and deliberate negative controls.

Freeze each implementation/configuration, allowed information, compute and decision deadline, tuning budget, retry policy, and initial-state conditions. A candidate must not get more search calls or better ground-truth access than its comparators.

### Phase C — counterexample search on the search partition

Use two complementary paths:

1. **Coverage-first exploration:** a reproducible space-filling design over justified parameter bounds plus pairwise or higher-order interaction coverage for declared fault families. Preserve event order and system state through the episode.
2. **Adaptive falsification:** allocate additional evaluations toward trajectories that approach a service-floor boundary, expose an assumption violation, or produce a new failure mode. Search objectives may include time-to-first-breach, critical-service shortfall, longest breach, and recovery-state depletion, but outputs remain a vector.

For a one-dimensional parameter where monotonicity is physically justified and validated, bracket the failure transition and refine it. Otherwise, do not assume monotonicity or use binary search as if it were valid. Retain search algorithm, bounds, random seeds, budget, failed runs, and selection history. Minimize a discovered counterexample only while preserving its causal/event semantics; never remove a required precursor merely to make the test look simpler.

All adaptive searching and controller tuning stay outside the locked holdout. A search algorithm is allowed to be good at finding failure. Its best-found case is a discovery artifact, not an unbiased estimate of event probability or average field performance.

### Phase D — locked boundary validation

Freeze candidates and search procedure, then test previously unseen episodes and independently selected disturbance trajectories. Include near-boundary cases on both sides of discovered transitions, newly drawn compound sequences, and cases selected independently of candidate search results. Preserve unfavorable and invalid executions; do not silently retry or delete failed runs.

Repeat important boundaries across model parameter sets and, where possible, independent model implementations or physical references. If a claimed advantage disappears under plausible parameter variation, report that fragility rather than selecting the favorable model.

### Phase E — SIL/PIL/CHIL/HIL progression

After calibrated model evaluation, move selected tests into an appropriate isolated controller/lab testbed. Capture command-to-actuator behavior, control-mode transitions, timing, alarms/fallback, measured state, and delivered-service outcomes. HIL results cover the tested boundary and configuration only. IEC TS 62898-3-5:2026 explicitly covers HIL, commissioning, and periodic testing for microgrid monitoring, control, and energy-management systems; use applicable licensed requirements and qualified reviewers to create a standards gap map, not a compliance claim ([IEC scope](https://webstore.iec.ch/en/publication/84175)). IEEE 2030.8-2018 is another controller-testing reference ([IEEE SA](https://standards.ieee.org/ieee/2030.8/6169/)).

## 4. What to report

For every trajectory and policy, preserve at least:

- Per-service demanded/delivered quantities, units, approved floor, deficit trajectory, gate result, first breach time, longest consecutive breach, and total breach duration.
- Service-specific and system recovery time, plus recovery lag after the initiating disturbance ends.
- Initial/final storage, fuel, spares and thermal state, replenishment events, and state debt carried into the next event.
- Disturbance-family IDs, severity parameter vector, event ordering/timing, seed, model/config digests, controller commit, runtime/peak memory, deadline misses, and failure/exit status.
- Search algorithm/version, total evaluations and compute budget, counterexamples found and independently replayed, invalid simulations, and coverage gaps.
- Model calibration residuals, conservation residuals, uncertainty ranges, applicability restrictions, and whether each result is observed, simulated, synthetic, or HIL.

Use the existing service scorer for trace-level metrics where applicable. Keep actual physical units per service; do not add heat energy, water volume, and communications coverage into a fictitious physical total. Do not count an inactive service as a demonstrated pass or infer unbounded survivability from a trace that ends before a failure.

### Boundary-specific outputs

Publish:

1. **Failure frontier:** Pareto-minimal observed violating parameter vectors, with the failed service/gate and first-failure point.
2. **Survival envelope:** tested parameter regions where contracts passed, with explicit model bounds and coverage. Label it empirical and sampled; do not call it a certified safe region unless a separate proof supports that language.
3. **Recovery envelope:** outcomes under repeated events at different inter-event gaps, carrying state forward rather than resetting the model after each fault.
4. **Counterexample set:** stable case IDs and replayable trajectories, including cases that were shrunk and those that could not be shrunk.
5. **Unknown/blocked map:** hazard and model dimensions not evaluated, with reasons.

Instead of only saying “policy A scored better,” this can show that its observed service-failure boundary shifted outward on particular dimensions under stated assumptions, while exposing dimensions that remain unknown.

## 5. Benchmarking against the strongest eligible method

A boundary-discovery method must not generate harder cases for its competitors than for itself—or tune only itself against a hidden fixed set. Use identical initial states, ground-truth trajectories, policy-visible information, model fidelity, scenario budgets, decision deadlines, and comparison rules. If each controller has an adaptive search loop, freeze evaluation budgets and search access; independently validate discovered counterexamples against all compared controllers.

Use lexicographic interpretation:

1. **Admissibility first:** satisfy every required hard service/safety gate. No weighted average, fuel saving, or Pareto status excuses a new critical-floor breach.
2. **Robust outcome vector second:** compare per-service shortfall, breach duration, recovery, fuel/resupply burden, lifecycle cost, and computational cost separately.
3. **Boundary improvement third:** count a gain only where the observed frontier improves on comparable axes without a disqualifying regression elsewhere. Publish the full non-dominated set and trade-offs.

Report absolute outcomes alongside relative changes. With defensible scenario probabilities, separate probability-weighted risk from boundary search; expected risk or CVaR needs probability provenance and suitable uncertainty analysis. An adaptively selected set of worst cases is not a probability sample. Without defensible probabilities, report enumerated outcomes and worst-case cases directly.

Use episode/site/weather-block-level uncertainty methods that preserve dependence. Do not treat timesteps as independent observations. Preregister the primary comparison, multiplicity handling, interval method, and power/sample-size rationale from pilot variability; do not claim a universal minimum sample size before the event/site structure is known.

### Useful experimental metrics (not yet targets or results)

- **Validated counterexample yield:** distinct, independently replayed failure modes per fixed evaluation budget.
- **Boundary refinement cost:** evaluations needed to refine a declared one-dimensional transition to a preregistered tolerance, when monotonicity is justified.
- **Counterexample reproducibility:** fraction of discovered failures that replay with the same gate/failure classification under the pinned setup.
- **Frontier movement:** service-specific change in minimal observed failure vectors on locked tests.
- **Search overfitting:** difference between discoveries on the search partition and independent holdout discoveries.
- **Controller overhead:** latency, memory, energy, invalid outputs, and deadline misses on named target hardware.

These are research instrumentation choices, not sufficient evidence by themselves that a controller is superior.

## 6. Negative controls and falsification

At minimum, prove the experiment catches: a floor-violating controller that uses less fuel; stale data treated as fresh; nominally redundant components sharing a failed dependency; a state reset that replenishes storage/inventory without a valid event; a forecast/ground-truth leak; a candidate that wins only when failed runs are omitted; a non-monotonic boundary incorrectly certified by binary search; and a search process that leaks holdout cases into tuning.

The central hypothesis is falsified if the apparent boundary advantage fails independent replay, depends on unsupported monotonicity, disappears under plausible calibrated-model uncertainty, introduces a new critical-service gate failure, or cannot be reproduced from the published manifest.

## 7. Reproducibility manifest

Every run bundle should pin:

- Protocol and scenario-generator commits; controller and simulator/testbed commits or versions.
- Raw source/data/model/configuration digests, units, provenance and licensed-use basis.
- Scenario ID, initial state, full timestamped disturbance trajectory, generator version, seed, split assignment, and policy-visible input trace.
- Decision/runtime environment, budget, command line, start/end timestamps, exit code, retries, and invalid-run reasons.
- Raw controller outputs, delivered-service traces, gate vector, failure-frontier output, and reviewer decisions.

A hash proves byte identity, not source truth, physical validity, or an independent signature. Keep the report's evidence class explicit. Queued, cancelled, missing, invalid, or skipped runs are not passes.

## 8. Immediate implementation sequence

1. **Specification (this document):** approve event/parameter contracts and the no-scalar, hard-gate-first reporting rule.
2. **Synthetic protocol fixtures:** add hand-authored one-dimensional monotonic boundary, non-monotonic counterexample, incomparable two-hazard, repeated-event carry-over, and search/holdout-leak negative controls.
3. **Rust case/manifest validation:** validate case IDs, bounds, event ordering, initial state, split isolation, hashes, and evidence class. Keep search generation separate from scoring.
4. **Deterministic replay adapter:** connect a suitably validated simulator through an explicit versioned I/O contract; no simulator or site claim until one is actually integrated.
5. **Independent boundary-report verifier:** recompute gate outcomes and frontier minimality over supplied enumerated cases; ensure missing/invalid cases cannot silently improve results.
6. **Calibrated evaluation, then HIL:** only after a defensible model/data basis and review.

The first implementation should target contract correctness and counterexample replay—not a sophisticated optimizer. That builds an inspectable, falsifiable foundation before adaptive search adds complexity.

### Current bounded prototype

The initial Rust CLI, `src/bin/resilience_boundary.rs`, analyzes a supplied set of already-evaluated cases and reports both Pareto-minimal observed failures and Pareto-maximal observed passing points for each gate and comparison group. It reports per-gate valid coverage, invalid/unevaluated counts, and explicit gate status, including groups with no observed breach. It also records **monotonicity-conflict witnesses**: a passing case that is componentwise more severe than a failing case under the declared axis directions. Such a witness warns against assuming a simple monotonic boundary; it is not automatically a defective test or controller. The analyzer does not generate cases or replay a controller. Its fixture is intentionally synthetic, includes incomparable failures, and includes invalid/not-evaluated cases to exercise incomplete-report handling.

From the repository root, run:

```sh
cargo test --locked --manifest-path research/polar-infrastructure/benchmark/Cargo.toml
cargo run --locked --quiet --manifest-path research/polar-infrastructure/benchmark/Cargo.toml --bin resilience_boundary -- research/polar-infrastructure/benchmark/fixtures/toy-boundary-search.json
```

The report preserves axis direction, keeps different initial-state groups separate, and labels its scope as observed enumerated cases only—not a safety certificate or probability estimate. A gate with no observed breach is listed explicitly; this is not proof the gate cannot fail elsewhere. Per-gate outputs show coverage and both observed sides of the boundary. Monotonicity-conflict witnesses expose cases where a more-severe passing vector exists alongside a less-severe failing vector, without assuming that physical systems are monotone. Invalid or unevaluated cases make the relevant frontier incomplete. The exact-head workflow checks this CLI and its negative controls before these mechanics can be treated as verified software behavior.
