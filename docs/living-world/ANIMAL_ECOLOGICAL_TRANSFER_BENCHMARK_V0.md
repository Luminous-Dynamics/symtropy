# Animal Ecological Transfer and Shock-Recovery Benchmark V0

Status: draft synthetic contract; not an empirical animal-validation result  
Research snapshot: 2026-10-20  
Tracking issue: [ANIMAL-ECO-TRANSFER-001 #2214](https://github.com/Luminous-Dynamics/symtropy/issues/2214)

## Purpose

Test whether animal agents remain competent, calibrated, and ecologically coherent when the world changes beyond familiar training scenes. The central question is not "does this rollout look plausible?" It is "does the model preserve the right invariants, adapt when adaptation is possible, recover from disturbance, and admit uncertainty when evidence or model support runs out?"

This benchmark complements—not duplicates—the animal replay/evaluation contract, active-sensing benchmark, and causal-identifiability benchmark. Those focus respectively on claim-specific replay metrics, sensing under finite budgets, and what controlled interventions identify. This contract adds **ecological regime transfer and response over time**, including the effects of individual policies on populations and resource systems.

It defines a test contract and synthetic oracle. It does not introduce a simulator implementation or claim that Symtropy currently passes.

## Paradigm shifts

### 1. From familiar-scene realism to ecological transfer

Randomly splitting adjacent frames lets a model see almost the same animal, environment, lighting, and event in training and testing. Instead, hold out complete episodes and ecological regimes: combinations of habitat topology, resource dynamics, climate, threat pressure, social composition, sensor conditions, and life stage. Report in-distribution and held-out results separately. A held-out regime must not be revealed through filenames, scene IDs, hidden state, intervention labels, or preprocessing artifacts.

### 2. From static plausibility to response curves and recovery

A transient resource pulse, a sustained drought, and a return to baseline are different experiments. Score pre-shift competence, immediate degradation, time-to-recovery, integrated deficit, overshoot, and hysteresis separately. Some shifts should change policy; negative-control shifts should not. Do not force recovery when the new regime makes the old behavior infeasible.

### 3. From isolated agents to coupled ecology

Individual reward alone can conceal resource depletion, unstable population feedback, crowding, or a group strategy that succeeds only by externalizing costs to other agents. Report individual competence beside shared-resource accounting, interactions, population trajectory, and adverse outcomes. Use conservation/accounting tests only where the declared world model actually specifies a conserved quantity or closed boundary.

### 4. From one "animal realism" number to a portfolio of falsifiable claims

Pose accuracy, action selection, sensory grounding, adaptation, uncertainty calibration, population stability, rights compliance, and computational efficiency are not interchangeable. A high score on one axis cannot compensate for a safety-critical or accounting invariant failure. Missing evidence is incomplete, never a score of zero and never an implicit pass.

### 5. From hidden omniscience to bounded, calibrated agents

The world may know exact resource locations, hazards, and future events; the agent must not receive them unless the sensor/task contract grants them. Under unsupported or out-of-distribution conditions, require a calibrated prediction, safe fallback, or explicit abstention. "Adaptation" that reads a hidden intervention flag is leakage, not intelligence.

### 6. From headline speed to matched-cost capability

Compare candidates and baselines with the same initial states, exogenous random streams, observation/action limits, simulated horizon, interaction budget, and compute budget. If compute or sensing differs, report it as a separate cost-tradeoff experiment. Always retain a simple heuristic baseline; add a frozen policy and a declared task oracle/upper bound where scientifically defensible.

## Related public research and what it measures

- [MABe22](https://proceedings.mlr.press/v202/sun23g.html) defines 72 learned-behavior-representation tasks across mouse, beetle, and fly datasets. It is useful for representation transfer and behavior-label tasks, but it is not itself an ecology-shock benchmark.
- [NeuroMechFly v2](https://www.nature.com/articles/s41592-024-02497-y) studies embodied Drosophila sensorimotor control, including vision, olfaction, feedback, and complex terrain. The current [FlyGym project](https://github.com/NeLy-EPFL/flygym/) describes major 2026 performance changes; its reported throughput is useful engineering context, not a direct apples-to-apples benchmark for multi-species ecology.
- [VastMAT](https://arxiv.org/abs/2609.34390) is a September 2026 preprint describing multi-animal tracking across 337 animal categories and seen/unseen-category splits. Its authors report top baseline HOTA scores of 66.37% seen and 52.90% category-disjoint unseen in the stated setup. Treat these as preprint-reported tracking results, not ecological-policy scores, and verify actual data release and licensing before adopting assets.

These reference different layers. Do not rank an embodied fly simulator, a behavior representation benchmark, a tracking benchmark, and a multi-species ecology simulator using one shared scalar.

## Experimental protocol

### Frozen run identity

Every candidate and baseline receipt must pin:
- source commit and dirty-tree state;
- simulator, policy, controller, ecology, and evaluator versions;
- scenario/configuration digest, sensor/action contract, and world-model assumptions;
- initial independent-unit manifest and exogenous-noise/seed family;
- training, validation, and test regime manifests with leakage audit;
- time horizon, physics/simulation step budget, interactions, wall/compute budget, and hardware/runtime;
- metric definitions, uncertainty method, confidence level, practical margins, and preregistration timestamp.

A receipt without these fields is incomplete. Failed or timed-out episodes remain in the accounting with an explicit disposition; silently dropping them biases robustness upward.

### Splits that matter

Group by independent episode, animal/individual where identity persists, and ecological regime. Do not split correlated frames from one episode across train and test. Include:
1. **Reference**: familiar regimes used only for sanity and regression.
2. **Single-factor holdouts**: one known axis changed to measure sensitivity.
3. **Compositional holdouts**: unfamiliar combinations of individually seen factors.
4. **Regime holdouts**: withheld ecological regimes or species/taxa where the claim permits the comparison.
5. **Stress/negative-control suite**: sensor faults, noncausal proxy changes, resource accounting attacks, and hidden-state queries.

All split membership and scenario hashes are frozen before candidate comparison. A claimed species-level transfer result must use an actually held-out species/taxon and appropriate rights; synthetic archetype coverage is not real-species evidence.

### Baselines

Use a ladder, with the exact method selected per task:
- static/frozen policy;
- reactive heuristic with transparent rules;
- current candidate policy;
- budget-matched learned or planning baseline, if available;
- task oracle / privileged planner as an explicitly labelled upper bound only when the oracle assumptions are documented.

The oracle must not become a production controller baseline while retaining hidden-state privileges. Compare candidate and baseline on paired independent units and identical exogenous noise/seed families where feasible. Include several independent seeds; publish uncertainty and per-regime results, not just the mean.

## Metrics: report a dashboard, not a composite "realism" score

| Axis | Primary measures | Failure signal |
|---|---|---|
| Task competence | task-native reward/success, constraint violations, time/energy to outcome | competence collapses on regime holdout or violates hard constraints |
| Transfer gap | normalized held-out regret and degradation versus reference | hidden regimes are worse than matched baseline beyond preregistered margin |
| Shock resilience | peak deficit, integrated performance deficit, recovery time, overshoot, hysteresis | silent failure, recovery claimed without a recovery window, or unstable overshoot |
| Calibration / uncertainty | proper scoring rule for probabilistic forecasts, coverage at declared nominal levels, calibration by regime, abstention quality | overconfident extrapolation, miscoverage hidden by averaging, or missing predictions treated as correct |
| Sensor grounding | performance under sensor dropout/noise/latency; invariance when only rendering changes | policy reads hidden world truth or rendering artifacts |
| Social/ecological interaction | interaction effects, per-agent outcomes, shared-resource use, population trajectories where applicable | aggregate reward masks systematic harms, depletion, or instability |
| Accounting invariants | energy/resource budgets and model-declared conservation residuals | unexplained creation/destruction beyond declared tolerances |
| Reproducibility | exact replay, digest verification, outcome stability under declared deterministic mode | same pinned run changes without explanation or receipt mismatch |
| Efficiency | wall time, CPU/GPU time, memory, energy estimate, samples/interactions to competence | claimed quality gain is purchased with unreported extra budget |
| Fairness / scope | per-taxon, life-stage, sensor-budget and regime stratification; claim coverage | aggregate gains conceal a subgroup regression or exceed available evidence |

For stochastic forecasts, report calibration with uncertainty and by regime. For recovery, define the target band, evaluation window, and "not recoverable under this regime" outcome before running the experiment. Avoid inventing a universal biological recovery threshold.

## Proposed qualification gates

These are proposed engineering gates to agree and preregister per task—not current results.

**Gate A — contract integrity**
- 100% of oracle cases have unique IDs and valid expected-contract records.
- Zero hidden-state/intervention-label leaks, train/test group overlaps, missing-receipt passes, or unaccounted failed episodes.
- Zero violations of declared hard accounting invariants.

**Gate B — reproducibility**
- Exact deterministic replays reproduce the required output/receipt digest in the declared deterministic mode.
- In stochastic mode, repeated-run variance is estimated; it is not incorrectly required to be bit-identical.
- Paired candidate/baseline units, seeds, budgets, and evaluator identity match or the comparison is marked `not_assessed`.

**Gate C — transfer**
- Publish all per-regime values and paired uncertainty intervals.
- Claim improvement only when the preregistered practical margin is exceeded under the chosen confidence procedure and no protected critical stratum crosses its regression bound.
- Compare separately against the heuristic and strongest available budget-matched baseline.
- Report compositional/unseen-regime gaps; never infer general transfer from random-frame splits.

**Gate D — resilience**
- Publish degradation, recovery time, integral deficit, overshoot, and uncertainty.
- Pre-register acceptable recovery bounds only for a specified disturbance and post-disturbance regime.
- Persistent regimes need adaptation/new equilibrium evaluation, not a forced "return to baseline" criterion.

**Gate E — population/accounting**
- No unmodeled energy/resource creation; open-system fluxes, births/deaths, and transfers are explicitly accounted for.
- Report individual distribution tails and adverse outcomes beside population means.
- A closed-box assertion is only valid if the fixture truly defines a closed box.

**Gate F — biological claim discipline**
- Synthetic oracle success proves only that the evaluator/contract rejects the specified invalid behaviors.
- Empirical validity requires independently sourced measurements, data-rights review, appropriate holdouts, uncertainty, and biological expert review.
- A tracking or pose benchmark score does not establish the correctness of a simulated causal mechanism or ecology.

## Roadmap: fastest path to real evidence

1. Land the synthetic oracle plus deterministic structural validator.
2. Bind the oracle to actual simulator scenarios and emit receipt-bearing runs.
3. Integrate an existing public benchmark only after its dataset, annotations, and derivative-use terms are separately reviewed.
4. Run a preregistered, budget-matched baseline sweep across single-factor and compositional holdouts.
5. Publish negative results and per-regime failures as first-class outputs; don't tune on the frozen test suite.

## Non-goals and limitations

- No assertion that the current agent is biologically faithful or state of the art.
- No replacement for MABe22, FlyGym/NeuroMechFly, pose/tracking datasets, or direct empirical validation.
- No pooled score that trades catastrophic ecological/accounting failures against visual or motion accuracy.
- No use of synthetic case counts as evidence of scientific generalization.
