# HarmonyField Utility and Sensitivity Protocol v0.1

**Status:** protocol proposal; not an experiment result  
**Related tracking:** [#1582 — qualify HarmonyField utility](https://github.com/Luminous-Dynamics/symtropy/issues/1582), [#1580 — numerical input and callback safety](https://github.com/Luminous-Dynamics/symtropy/pull/1580)

## 1. Decision question and claim boundary

The experiment asks whether the implemented HarmonyField mechanisms improve a **predeclared simulation task** relative to otherwise identical runs without those mechanisms, and whether any measured effect is robust across held-out seeds and declared parameter/model variants.

It does not ask whether HarmonyField is consciousness, an electromagnetic field, a fundamental physical law, or a validated account of human affect. A stable computation is not evidence of utility; a task improvement in a simulator is not, by itself, empirical validation of the simulator.

Before confirmatory runs, record:
- the task, intended use, and the population/scenarios to which conclusions are meant to apply;
- one primary utility endpoint, its sign/direction, units, and a decision-relevant effect threshold;
- secondary endpoints and the rule for interpreting conflicting outcomes;
- candidate commit SHA, full configuration digest, toolchain/solver settings, and this protocol's version;
- the parameter grid, development seed list, and sealed/held-out seed list.

Do not select the task metric, parameter ranges, or final claim after inspecting held-out outcomes. If no defensible task-specific utility metric exists yet, the work is exploratory and must not be presented as a utility demonstration.

## 2. Mechanism treatments: complete 2³ factorial

Use three independently switchable factors:

- **F — spatial friction:** HarmonyField friction modulation enabled/disabled.
- **R — collision surprise:** resonance-based scaling of collision prediction error enabled/disabled.
- **E — emotional contagion:** proximity-weighted emotional-contagion update enabled/disabled.

Run every combination. This is eight arms, not just a baseline plus selected examples:

| Arm | F | R | E | Interpretation |
|---|---:|---:|---:|---|
| A000 | 0 | 0 | 0 | No-field/no-contagion baseline |
| A001 | 0 | 0 | 1 | Contagion only |
| A010 | 0 | 1 | 0 | Resonance/prediction-error only |
| A011 | 0 | 1 | 1 | R + E |
| A100 | 1 | 0 | 0 | Friction only |
| A101 | 1 | 0 | 1 | F + E |
| A110 | 1 | 1 | 0 | Both HarmonyField couplings |
| A111 | 1 | 1 | 1 | All three mechanisms |

This complete design is required to estimate the three main effects and the F×R, F×E, R×E, and F×R×E interactions without confounding them with omitted combinations. If compute requires a reduced design, label it exploratory and state which interactions become unidentified.

**Implementation prerequisite:** the mechanisms must be switchable independently without silently changing other simulation settings. The current kernel exponent is calculated as p_D = max(D-1,1) inside HarmonyField::try_sample_internal; it is not presently a runtime tuning parameter. Do not claim an exponent sweep using a nonexistent configuration option.

## 3. Matched-seed execution

Each seed is a matched block: every treatment for that seed starts from identical initial state, source placements/profiles, task inputs, time step, solver configuration, and non-treatment random streams. Create a fresh world for every seed × treatment × parameter/model variant tuple; never carry state between arms.

**A shared seed is not enough to claim common random numbers.** Different treatment arms may take different branches and consume a stateful PRNG stream in different orders, so later draws may no longer represent the same stochastic events. The scenario contract must state the actual random-stream strategy and cross-treatment alignment:
- Prefer counter-based/event-keyed randomness where the variates can be keyed to stable logical events (scenario, subsystem, entity, tick, event identity) that remain comparable across treatment arms.
- Independent subsystem streams can isolate unrelated sources of randomness, but do not automatically align draws within a subsystem when a treatment changes branch behavior.
- If the implementation only uses the same initial seed with a stateful stream, record `same_seed_only`; use seeds as blocks, but do not claim common-random-number variance reduction.
- Use `common_random_numbers_verified` only when the implementation and an audit demonstrate that the intended stochastic inputs align across arms. Preserve the alignment evidence and disclose any events with no counterpart in another arm.

Record the RNG algorithm/version, stream derivation policy, and alignment evidence in the frozen scenario configuration. Do not retrofit a favorable alignment claim after inspecting outcomes.

Partition seeds before tuning:
1. **Development seeds:** debug the harness and explore candidate parameter settings.
2. **Held-out seeds:** remain unopened until the model/parameter choice and analysis are frozen.

Choose seed counts using a documented pilot-based precision or power rationale. A convenient round number is not a justification. If held-out results lead to a model or parameter change, those seeds become development data and a new untouched validation set is needed.

Replay a predeclared subset of identical seed/arm/config tuples to test determinism. Record exact equality when it holds. If the platform/solver produces bounded nondeterminism, specify the tolerance and the reason in advance; do not call a run deterministic solely because its seed is fixed.

## 4. Parameter and model-structure sensitivity

Predeclare domain-justified low/nominal/high levels (or another justified design) for source strength and radius. Report the actual numeric values, units, and rationale before confirmatory execution. Include multiple spatial dimensions where the code supports them.

Separate these uncertainty types:
- **Seed variability:** variation under a fixed model and configuration.
- **Parameter sensitivity:** variation across predeclared strength/radius values.
- **Model-structure sensitivity:** variation across explicitly versioned kernel rules, cutoffs, or other mechanisms.

The current implementation fixes the radial exponent to p_D = max(D-1,1). To investigate alternative exponents, implement each as an explicitly named and hashed model variant in the experimental harness, or add a separately reviewed configurable-kernel feature. Do not mutate the production default mid-experiment or label changing spatial dimension as equivalent to a free exponent sweep.

Interactions between F/R/E and parameters should be retained where feasible. A factorial design is preferred for detecting interaction effects; one-factor-at-a-time sweeps alone cannot establish that mechanisms behave independently.

## 5. Measurements

### Primary endpoint

Report the predeclared task-specific utility for each run. The utility formula must be specified before held-out execution, including penalties and termination conditions. Avoid combining incomparable endpoints into an opaque score after seeing the data.

### Secondary endpoints

Record, as applicable to the selected task:
- task success/failure and termination reason;
- completion time, survival/progress, and goal distance;
- collision count, collision severity, and accumulated collision impulse;
- friction multipliers and resonance/prediction-error trajectories;
- energy ledger residual and relevant momentum/invariant drift;
- non-finite state/result count, panic/solver failure, and dropped/missing measurements;
- HarmonyField validation-counter total and whether the interval is qualified.

A nonzero HarmonyField validation-failure counter means the run/interval is **unqualified**, even if its fallback kept the integrator finite. Preserve and report the result for diagnosis, but exclude it from qualified-effect claims and disclose the exclusion. Never treat an invalid run as a normal low-utility observation.

## 6. Artifact contract

Publish one machine-readable record per seed × arm × parameter/model variant, plus the aggregate report. Per-run records should contain at least the following fields:

    schema_version
    protocol_version
    candidate_commit
    configuration_sha256
    task_id
    model_variant_id
    dimension
    seed
    seed_partition
    arm_id
    friction_enabled
    resonance_error_enabled
    emotional_contagion_enabled
    parameter_values
    solver_and_timestep
    platform_and_toolchain
    random_stream_contract
    run_status
    termination_reason
    primary_endpoint
    secondary_metrics
    validation_failure_count
    nonfinite_count
    invariant_drift
    replay_match_status
    artifact_sha256

Use canonicalized serialization before hashing. Store run inputs and raw traces (or an explicitly documented trace summary with content hashes); aggregate tables must link back to the per-run evidence. A list of intended/queued jobs is not evidence that a run completed. A missing record is missing, never zero. Retain failed runs and reasons in the manifest.

## 7. Analysis plan

For each arm/configuration, provide per-seed observations and summarize paired differences against A000 using the matched seed blocks. Report effect estimates and uncertainty intervals, not only p-values or a single mean.

For the complete factorial, estimate the F, R, and E main effects and all interactions. For continuous endpoints, state the estimator and interval method. For binary task success, report the risk difference (and interval) alongside raw counts. Use seed-level resampling or a justified model that respects matched runs; do not treat frames/ticks from one run as independent experimental replicates.

Predeclare treatment contrasts, multiplicity handling for secondary metrics, exclusion/qualification rules, and how missing/failed runs will be described. Report sensitivity both across seeds and across parameter/model variants. Distinguish practical effect size from statistical detectability. If intervals span both meaningful benefit and meaningful harm, call the result inconclusive rather than robust.

For each arm-versus-A000 contrast, also report the held-out sample variance of paired differences, the sum of the separate sample variances (the independent-difference variance estimate), and their ratio when defined. Label this as an empirical pairing diagnostic only. A ratio below 1 indicates lower observed variance for the paired-difference estimator in that sample; above 1 indicates higher variance. Do not infer actual common-random-number alignment from this statistic, and do not assume matching seeds reduce variance. If fewer than two held-out blocks exist, the sample variance is undefined and the confirmatory analysis must remain unqualified.

## 8. Qualification gates

A result is eligible for a utility claim only when all applicable gates are met:

- [ ] Protocol, primary endpoint, parameter grid, and analysis are frozen before held-out evaluation.
- [ ] Exact candidate/configuration identities and complete per-run evidence are available.
- [ ] All eight F/R/E arms have matched-seed coverage for every declared confirmatory condition.
- [ ] Replay/determinism requirement is demonstrated or the predeclared nondeterminism bound is met.
- [ ] Invalid-input validation counters are zero for qualified runs; all unqualified runs remain visible in the manifest.
- [ ] Missing/failed runs are never imputed as successes, zero cost, or zero effect.
- [ ] Main effects, interactions, effect sizes, uncertainty intervals, and sensitivity results are reported.
- [ ] Conclusions are restricted to the tested simulated task and specified model/configuration.
- [ ] Any real-world emotional-contagion claim is separately compared to appropriate external observations with data provenance and limitations documented.

An unrun test, queued job, skipped suite, absent artifact, or invalid interval does not pass a gate.

## 9. Research basis

This protocol adapts general methodological guidance; it is not a claim that any experiment has already been run:

1. Borgonovo, E., Pangallo, M., Rivkin, J., Rizzo, L., & Siggelkow, N. (2022). *Sensitivity analysis of agent-based models: a new protocol*. Computational and Mathematical Organization Theory, 28, 52–94. https://doi.org/10.1007/s10588-021-09358-5 — describes matching sensitivity methods to questions and using factorial designs to estimate main and interaction effects.
2. Troost, C. et al. (2023). *How to keep it adequate: A protocol for ensuring validity in agent-based simulation*. Environmental Modelling & Software, 159, 105559. https://doi.org/10.1016/j.envsoft.2022.105559 — frames validity as context-adequacy across model choice, inference, uncertainty analysis, and interpretation, with uncertainty traced throughout.
3. Heikes, R. G., Montgomery, D. C., & Rardin, R. L. (1976). *Using common random numbers in simulation experiments — an approach to statistical analysis*. Simulation, 27(3). https://doi.org/10.1177/003754977602700301 — discusses using the same pseudo-random streams across alternatives as a simulation-comparison design. For this experiment, a fixed seed alone does not prove that treatment arms consumed equivalent draws, so the RNG alignment policy and evidence must be recorded.
4. Wright, R. D., & Ramsay, T. E. (1979). *On the effectiveness of common random numbers*. Management Science, 25(7), 649–656. https://doi.org/10.1287/mnsc.25.7.649 — reports cases where common-random-number coupling produced negative output correlation and increased variance. Therefore, a CRN strategy must be verified and its observed pairing effect assessed, not treated as automatic variance reduction.

## 10. Current implementation notes

At the time of writing:
- PR #1580 hardens numeric inputs to HarmonyField field/resonance/friction/impulse/energy APIs and contains errors at a non-fallible physics callback boundary; its exact-head CI is the qualification gate for that code.
- HarmonyField's impulse multiplier API is not currently wired into the physics callback; don't enable it experimentally by assuming it is active.
- Emotional contagion is a separate agent update and requires its own checked-input/failure-reporting contract; see issue #1584.
- The optional curvature operators need checked scale/epsilon/denominator contracts; see issue #1583.

These notes must be updated against the exact commit under evaluation before any experiment report is published.
