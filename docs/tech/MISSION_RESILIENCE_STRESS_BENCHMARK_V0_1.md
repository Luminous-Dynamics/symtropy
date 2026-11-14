# Mission Resilience and Adaptive Replanning Benchmark v0.1

**Status:** synthetic-only regression harness for candidate-plan stress testing. It is not a route planner, dispatch controller, safety case, permit, route authorization, clinical protocol, autonomous driving approval or real-world transport validation.

## Paradigm shift: design a resilient service, not a winning vehicle

A single "best vehicle" score hides the failure modes that matter most. A fast carrier can be unavailable after a corridor closure; an accessible platform can lose its transfer capability; a backup can miss the delivery window after a bounded storm delay; and a cold-chain route can become inadmissible even when the vehicle still moves. Ranking everything on one weighted number lets a strong speed or cost score obscure an unacceptable safety or access failure.

This benchmark treats transport as a **portfolio of candidate end-to-end plans** and asks:

1. Which plans meet the mission's hard constraints now?
2. Which feasible plans remain after declared disturbances?
3. Does the set retain a non-dominated trade-off frontier, without inventing a universal winner?
4. Does the correct answer become *no admissible plan* when every candidate is unsafe, over deadline or below reserves?
5. Are route, hazard, recovery, staffing and condition-control unknowns preserved as unknown rather than silently converted to permission?

The resilience unit is the mission's **plan set under a stated disturbance**, not a vehicle in isolation. The upstream [multimodal mission-chain contract](https://github.com/Luminous-Dynamics/symtropy/pull/1887) still defines how ordered legs and transfers should be represented; this layer stresses candidate-level outcomes against bounded changes. It does not yet perform route-graph search or generate live contingency routes.

## Contract and disposition rules

The fixture spans five explicitly labeled environments (Amazon Basin, Congo Basin, Antarctica, Sahara and Afghanistan), five synthetic missions, fourteen candidate plans and ten stress cases. Every plan includes a mode chain, duration bound, residual-energy percentage, condition-margin value, capabilities and six evidence gates. Each stress case names only the plans it changes; other candidates retain their baseline values. A declared impact may make a plan unavailable, add duration, reduce reserve or condition margin, override a capability, or change a gate state.

Gate values are explicitly **passed**, **unknown** or **failed**.

- **blocked_unavailable**: the candidate cannot be selected for this scenario.
- **blocked_hard_gate**: one or more required evidence gates are failed.
- **blocked_constraints**: deadline, energy reserve, condition margin or a required capability is violated.
- **needs_gate_evidence**: no known gate failure or known constraint violation exists, but required evidence is unknown.
- **eligible_for_human_review**: every modeled gate and mission constraint passes; it remains only a synthetic candidate for expert review.

Precedence is fail-closed: unavailability first; failed gates before other findings; known constraint violations before unresolved evidence; eligibility only when everything modeled passes. The report still preserves both failed and unknown gate fields even when a different disposition takes precedence. A stress perturbation cannot improve energy reserve beyond 100%, and duration deltas are non-negative. Missing fields, duplicate IDs, cross-mission impacts, unknown fields and malformed statuses reject the fixture rather than create a pass.

## Synthetic baseline and negative controls

Expected outcome oracle for the committed fixture:

| Benchmark dimension | Expected result |
|---|---:|
| Environments | 5 |
| Missions | 5 |
| Candidate plans | 14 |
| Baseline-feasible plans | 8 |
| Stress cases | 10 |
| Declared recoverable stress cases | 6 |
| Recoverable cases retaining at least one feasible plan | 6/6 (100%) |
| Intentionally unrecoverable / evidence-blocked negative controls | 4 |
| Negative controls correctly producing no admissible plan | 4/4 |
| Stress-case outcomes matching the declared oracle | 10/10 (100%) |

These are deterministic regression expectations, not empirical resilience rates. Environment-stratified summaries report mission count, candidate count, baseline feasible count, stress coverage, recoverable-case coverage, and correct negative-control refusal for each environment. The Afghanistan fixtures intentionally have no eligible baseline plan: unknown route authorization and hazard evidence remain unresolved; a failed hazard gate is a hard block. The fixture is designed to exercise failure handling, not to estimate how often real weather, energy loss, closure or equipment faults occur. The "100%" values only mean that all five declared recoverable synthetic disruptions preserve a feasible candidate and the single deliberately impossible control is correctly refused.

The Pareto frontier minimizes duration while maximizing residual energy and condition margin. A candidate is removed only when another **eligible** candidate is no worse on all three measures and strictly better on at least one. Hard gates and mission constraints filter candidates before dominance analysis. Results are a set of trade-offs; the tool does not assign weights, manufacture probabilities, or select a universal winner.

## Benchmarks to aim for

Use staged, evidence-qualified gates rather than one blended score.

### Stage 0 — deterministic contract correctness

- **100% hard-gate exclusion:** no plan with a failed required gate reaches **eligible_for_human_review**.
- **100% fail-closed unknown handling:** unresolved evidence never becomes permission.
- **100% constraint enforcement:** zero plans admitted below required energy reserve, condition margin, accessibility/support, or deadline bounds.
- **100% scenario/oracle agreement:** all committed stress cases, negative controls, malformed fixtures and generated perturbation matrices match an independently implemented reference.
- **Exhaustive gate matrix:** all 3^6 = 729 combinations of six gate fields return the expected fail-closed disposition.
- **Capability mask matrix:** all 2^6 = 64 required-capability combinations enforce every declared mission requirement.
- **100% scenario accounting:** every declared stress case and every candidate in its mission has an explicit disposition.
- **100% deterministic report:** identical normalized inputs produce byte-identical serialized report content regardless of ordering of independent input collections.
- **Zero scalar winners:** never hide trade-offs behind a universal utility score.

### Stage 1 — synthetic portfolio resilience

- **At least 95%** retained-plan coverage over *predeclared recoverable* disturbances in a larger held-out scenario suite, with every scenario tagged as recoverable or intentionally unrecoverable before test execution.
- **100%** correct refusal on declared impossible cases and all failed/unknown hard-gate cases.
- **100%** preserved human accessibility and cargo condition constraints after every perturbation.
- Track coverage by environment, mission class, payload class, disruption type and severity; do not only report a global average.
- Report worst-case regret and frontier stability only where an independent reference optimum exists and the input assumptions are justified. Do not call invented probabilities "risk".

### Stage 2 — model-based and hardware-in-the-loop evidence

- Calibrate delay, energy, heat transfer, vibration, payload movement and recovery margins against instrumented bench tests and environmental profiles.
- Measure false-feasible rate and missed-feasible rate against an independently reviewed oracle. Target **zero false-feasible outcomes** for required safety gates; disclose every unresolved mismatch.
- Evaluate single-fault and compound-fault cases, correlated disruptions, sensor faults, delayed or stale telemetry, communications loss, uncertain bounds and cascading handoff failures.
- Test replan time separately from execution safety; the planner proposes only from a known-safe, pre-authorized candidate set and must fail closed when current conditions escape the validated envelope.
- Keep human factors, accessibility, clinical suitability and environmental disturbance as requirements and disqualifiers—not soft trade-offs.

### Stage 3 — field evidence, only after domain-specific approval

- Run controlled non-human payload pilots and instrumented environmental trials under relevant permits and safety cases before considering people or clinical loads.
- Publish sample sizes, test conditions, confidence intervals, calibration error, failure taxonomy and the exact tested software/configuration hashes.
- Validate real-world recovery, abort behavior, communications loss and safe stop in every proposed operational design domain.
- Never infer authorization for Afghanistan, suspected explosive hazards or any restricted/unsafe route from this synthetic harness.

The 95% portfolio-coverage target is a **research goal for held-out benchmark suites**, not a safe-operations threshold and not evidence that a 5% failure rate is acceptable for people, medicine or hazardous terrain.

## What the current model omits

- No graph search, online replanning, route geometry, terrain map, weather service, travel-time distribution or vehicle dynamics.
- No probabilistic reliability estimate; stress cases are declarative, not sampled from observed distributions.
- A plan's durations, energy reserves and condition margin are fixture values. They are not measurements, engineering predictions or specifications.
- No thermal transient, vibration/shock propagation, crashworthiness, medical-care or ergonomic model.
- No authenticated sensor receipt, custody ledger, operator acknowledgement, proof of maintenance or external route authorization.
- No claim that an alternative plan is physically interchangeable or immediately deployable; all feasible plans require a separate human review and independent operational approval.

The next research phase should add bounded interval uncertainty and paired single/compound faults, then link the audited candidates to route-graph search. Optimize a **Pareto set** across deadline, energy margin, condition margin, cost, ecological disturbance and recovery—not a single score. Reproducible telemetry receipts and provenance are prerequisites before converting synthetic perturbations into calibrated distributions.

## Research anchors

- NIST, [Modeling and Simulation for Improving Ambulance Patient Compartment Design Standards](https://www.nist.gov/publications/modeling-and-simulation-improving-ambulance-patient-compartment-design-standards): models how workers reach patients and equipment while remaining seated and restrained. This supports treating human-factors requirements as explicit constraints; it does not directly validate off-road transport or robotics.
- NIST, [Modeling for Optimal Ambulance Patient Compartment Layout](https://www.nist.gov/publications/modeling-optimal-ambulance-patient-compartment-layout): simulation can test design options before committing to physical builds.
- [Multimodal mission-chain audit](https://github.com/Luminous-Dynamics/symtropy/pull/1887): companion ordered-leg/handoff contract and explicit treatment of uncertain route/hazard evidence.
- [Payload-first cargo and human mission contracts](https://github.com/Luminous-Dynamics/symtropy/pull/1879): companion mass, geometry, payload and human-transport requirements.

## Reproduction

From the repository root, run:

    cargo fmt --manifest-path tools/mission-resilience-audit/Cargo.toml -- --check
    cargo test --manifest-path tools/mission-resilience-audit/Cargo.toml --all-targets
    cargo run --manifest-path tools/mission-resilience-audit/Cargo.toml -- fixtures/transport/mission_resilience_scenarios_v0_1.json > mission-resilience-report.json
    jq -e '.benchmark_id == "mission-resilience-v0.1" and .mission_count == 3 and .candidate_plan_count == 9 and .stress_case_count == 6 and .baseline_eligible_plan_count == 6 and .expected_recoverable_case_count == 5 and .expected_recoverable_case_coverage_percent == 100 and .expected_no_plan_control_correct_count == 1 and .expected_viability_oracle_match_count == 6 and .weighted_utility_score_produced == false and .universal_winner_produced == false and .route_or_deployment_authorized == false and .real_world_validated == false' mission-resilience-report.json
    sha256sum mission-resilience-report.json

Passing this audit means the software matches this synthetic contract and fixture only. It does not validate route plans or authorize transport.

