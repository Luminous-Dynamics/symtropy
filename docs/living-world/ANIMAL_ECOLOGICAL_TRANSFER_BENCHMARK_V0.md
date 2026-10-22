# Animal Ecological Transfer and Shock-Recovery Benchmark V0

Status: draft synthetic contract; not an empirical animal-validation result  
Research snapshot: 2026-10-20  
Tracking issue: [ANIMAL-ECO-TRANSFER-001 #2214](https://github.com/Luminous-Dynamics/symtropy/issues/2214)

## Purpose

Test whether animal agents remain competent, calibrated, and ecologically coherent when the world changes beyond familiar training scenes. The central question is not "does this rollout look plausible?" It is "does the model preserve the right invariants, adapt when adaptation is possible, recover from disturbance, and admit uncertainty when evidence or model support runs out?"

This benchmark complements—not duplicates—the animal replay/evaluation contract, active-sensing benchmark, and causal-identifiability benchmark. Those focus respectively on claim-specific replay metrics, sensing under finite budgets, and what controlled interventions identify. This contract adds **ecological regime transfer and response over time**, including the effects of individual policies on populations and resource systems.

Related internal qualification lanes keep the guarantees composable:
- [Causal identifiability #2186 / PR #2187](https://github.com/Luminous-Dynamics/symtropy/pull/2187): does a controlled intervention identify the predicted mechanism/effect?
- [Ecological transfer #2214 / PR #2215](https://github.com/Luminous-Dynamics/symtropy/pull/2215): does competence and recovery transfer to new regimes?
- [Model-structure uncertainty #2244 / PR #2247](https://github.com/Luminous-Dynamics/symtropy/pull/2247): how do plausible alternative mechanisms affect forecasts and choices?
- [Model-set adequacy #2334 / PR #2335](https://github.com/Luminous-Dynamics/symtropy/pull/2335): can a blinded challenger expose a mechanism missing from the registered model set?
- [Decision-scope qualification #2350 / PR #2352](https://github.com/Luminous-Dynamics/symtropy/pull/2352): are downstream risk calculations correctly marked conditional on the evidence about model-set adequacy?
- [Aquatic process closure #2206 / PR #2207](https://github.com/Luminous-Dynamics/symtropy/pull/2207): do the underlying nutrient/biomass/oxygen transfers close before ecological predictions are layered on top?

A green contract at one layer does not promote a downstream layer automatically. For example, valid process accounting is not biological calibration, and a model ensemble's internal agreement is not evidence that it contains the world-generating mechanism.

It defines a test contract, a synthetic oracle, and a frozen JSON Schema for its structure. The schema fixes V1 at 51 cases and documents that unique-ID and receipt/manifest invariants still require semantic checks outside ordinary JSON Schema validation. It does not introduce a simulator implementation or claim that Symtropy currently passes.

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

- The [Bio-logger Ethogram Benchmark (BEBE)](https://doi.org/10.1186/s40462-024-00511-8) reports 1,654 hours from 149 individuals across nine taxa for behavior classification from animal-borne tags. It supplies useful sensor-to-ethogram comparisons, not a direct test of an autonomous agent's closed-loop response to ecology changes.
- [DISK](https://doi.org/10.1038/s41592-025-02893-y), published in *Nature Methods* on 4 December 2025 (2026 volume), evaluates missing-skeleton imputation across seven animal skeleton datasets. It reports RMSE, MPJPE, and PCK@0.01 on missing keypoints; those metrics should be used for the pose-data layer, not substituted for policy or ecosystem validity.
- [Movebank's data policy](https://www.movebank.org/cms/movebank-content/data-policy) makes access and reuse study-specific, with owner-controlled permissions/licenses; it notes that animals and tags should not be presumed unique across studies and calls for extra protections for threatened or at-risk populations. Public-data status or an upstream code license is not a substitute for an asset-by-asset permission and location-sensitivity audit.

- The 2026 [*Global Change Biology* review of multi-model ecosystem ensembles](https://doi.org/10.1111/gcb.70987) argues that ensemble agreement needs diagnostic intercomparison and observation-grounded benchmarking before it can support a decision. The [2026 *Ecology Letters* review of calibration, sensitivity, and uncertainty](https://doi.org/10.1111/ele.70375) treats input, parameter, and model-structure uncertainty as related but distinct concerns; this motivates reporting transfer and uncertainty by regime instead of just a mean score.
- The June 2026 [agent-based ecosystem model using deep reinforcement learning](https://doi.org/10.1016/j.ecoinf.2026.103819) reports emergent coexistence, predator–prey cycles, and resilience to moderate perturbations in an Alpine wolf–chamois–vegetation system. Treat this as a candidate process/behavior comparator after implementation parity is established, not a direct score comparison with a general-purpose multi-species simulator.
- The July 2026 [ODD-based agent-based-model replication study](https://doi.org/10.1016/j.ecolmodel.2026.111624) reports that executable implementations can still differ in behavioral fidelity, dynamics, runtime, and maintainability. Accordingly, a green compile/test run is necessary software evidence, not proof that the intended ecology has been reproduced.

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

### Aquatic and hydrological regime slice

Where the world model includes water, include explicit tests for current-vector changes, flood timing, water depth, habitat/corridor connectivity, and delayed trophic/resource effects. Keep the fluid/hydrological field, what a given sensor can observe locally, and the rendered image as separate layers. Flooding can change traversability even if a surface looks similar; a rendered current is not a substitute for the simulated flow field. Enforce declared habitat/movement capabilities by evidence class (measured, modeled, or synthetic); do not infer aquatic competence from generic movement or claim real-species validity from synthetic profiles. For open water/resource boundaries, account for declared fluxes rather than imposing a closed-box conservation rule.

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

## Competitive targets for the first pilot

The figures below are **proposed stretch targets**, not published SOTA claims, current results, or qualification rules that should be retrofitted after seeing test outcomes. First reproduce the reference baseline on the exact official protocol; preregister the final margins, samples, uncertainty method, and exclusions before unsealing test results. A target is applicable only to the corresponding perception/task module—not as one global animal-realism score.

| Layer | First-pilot stretch target | Guardrail / fair comparison |
|---|---|---|
| Animal tracking / unseen categories | On the [VastMAT](https://arxiv.org/abs/2609.34390) category-disjoint protocol, aim for **+2 absolute HOTA points** over the strongest baseline we successfully reproduce. | Use the same protocol, detections/input assumptions, and compute declaration; no more than 1 absolute HOTA point regression on seen categories without an explicit trade-off claim. The source is a preprint; first verify benchmark release, implementation, and data permissions. |
| Skeleton-gap imputation | Aim for **≥5% relative normalized-RMSE reduction** over the strongest reproduced non-oracle baseline on held-out gap patterns. | Also report MPJPE and PCK@0.01; do not accept an RMSE win that materially damages PCK, uncertainty coverage, or downstream behavior/event statistics. Use the [DISK](https://doi.org/10.1038/s41592-025-02893-y) metric definitions and dataset splits where compatible. |
| Bio-logger behavior classification | On [BEBE](https://doi.org/10.1186/s40462-024-00511-8), target a win over the strongest reproduced official baseline in the preregistered low-data setting, with non-inferiority on at least **7 of 9 taxa/datasets**. | Use BEBE's official evaluation and per-individual reporting; dataset licenses are independently reviewed. This is a behavior-inference target, not ecological policy validation. |
| Multi-species learned representations | On [MABe22](https://proceedings.mlr.press/v202/sun23g.html), aim for a statistically supported lift on at least **10 of 72 task-native evaluations**, while publishing the full task-by-task results. | Do not pool incompatible metrics or tune on test tasks. Count a task as a win only under preregistered paired uncertainty/margin rules. |
| Ecological transfer (new benchmark) | Aim for **≥10% lower normalized regret** across compositional holdouts and **≥15% lower integrated performance deficit** after preregistered transient shocks than the strongest budget-matched baseline. | Require multiple independent seed families and at least two held-out regime families; lower confidence bound must clear the preregistered practical margin. No hard accounting, hidden-state, rights, or receipt violations. |
| Probabilistic forecasting | For a nominal 90% predictive interval, target **87–93% empirical coverage** in each adequately sampled major test stratum, not only in aggregate. | Report interval width/proper scoring rule and sample uncertainty; widen or abstain under distribution shift rather than hiding a severe subgroup miscalibration. |
| Runtime / scale | After correctness is stable, target **2× throughput** over the same Symtropy baseline at fixed workload and fidelity. | Same hardware and workload; report p50/p95 latency, memory, compute and energy where measurable. Require zero new invariant violations and ≤1% drift on preregistered validated outputs; do not compare against FlyGym's published throughput without workload parity. |

The first honest win is not necessarily beating every published number. It is reproducing the strongest applicable baselines, then demonstrating a robust, uncertainty-qualified improvement on genuinely held-out cases while preserving invariants. If baseline reproduction fails, report the replication failure; do not quietly substitute a weaker comparator.

## First integrated pilot: flood–forage–risk corridor

The first integration test should be a small, fully synthetic river-corridor world, not a visually elaborate open world. It is intentionally designed to compose contracts from the separate workstreams and create opportunities for cross-layer failures.

This is a **benchmark tranche inside existing convergence work**, not a competing scene implementation: it should be aligned with [ANIMA Cell #1185](https://github.com/Luminous-Dynamics/symtropy/issues/1185), [Living Valley #1247](https://github.com/Luminous-Dynamics/symtropy/issues/1247), and the [Living World Cell #252](https://github.com/Luminous-Dynamics/symtropy/issues/252). It does not waive those programmes' prerequisite gates. Initially use deterministic synthetic fixtures to exercise the contracts; only connect qualified hydrology, fauna/perception, habitat/terrain and persistence adapters after their authority and receipt requirements are satisfied. Where the existing owners already provide a canonical water/process or habitat state, consume those qualified outputs rather than maintaining parallel truth in the animal benchmark.

### Frozen scenario

The machine-readable pilot is [`ANIMAL_FLOOD_FORAGE_RISK_PILOT_V0.json`](ANIMAL_FLOOD_FORAGE_RISK_PILOT_V0.json), with a paired [JSON Schema](ANIMAL_FLOOD_FORAGE_RISK_PILOT_V0.schema.json). A Rust semantic validator checks its time intervals, map bounds, corridor/flood connection, policy-budget parity, denied hidden inputs, model roles, and hard invariants. This manifest is still a contract specification; it does not execute the scenario.

**Blinding boundary:** the public fixture exposes the two candidate mechanisms so they can be reproduced, but intentionally leaves the blind challenger's mechanism parameter as `null`. A real blinded evaluation must obtain the challenger's contents from a separately controlled evaluator-only manifest, pin that artifact's digest in each result receipt, and lock candidate predictions before revealing it. If no such protected challenge artifact and independent custodian exist, report the challenger portion as `not_assessed`; do not treat a public placeholder as an actual blind test.

- The world contains two refuges connected by habitat corridors, a finite forage/water budget, an explicit current/depth field, and a declared predator-encounter process.
- The focal animal receives local observations with a specified detection/latency/noise contract. Exact refuge coordinates, future flood state, causal intervention labels, and the simulator's hidden model ID remain evaluator-only.
- At a frozen time, a flood changes current, depth, and corridor connectivity. A concurrent resource shortage creates a genuinely held-out combination, not seen as a full combination in tuning.
- The animal can spend a bounded sensing budget to inspect a cue, choose a route/action, or abstain/re-plan. Movement cost and risk are charged. If social agents are enabled, their state and interactions are separate factors rather than an untracked reward multiplier.
- Two registered candidate ecological mechanisms fit the reference history but differ on the delayed post-flood forage response. A blinded challenger, generated from a held-out mechanism family, can expose a shared candidate-model mistake.
- The renderer is free to show the same appearance for two hidden flow/depth states; policy behavior must be driven by authorized sensory and process inputs, not camera pixels or hidden values.

### Evaluation outputs

Do not collapse this to a winner-takes-all scalar. Each run should emit:
1. **Process closure:** water/resource fluxes and nutrient-equivalent state are reconciled; rejected process updates have zero mutation.
2. **Perception and active sensing:** local evidence provenance, cue latency, sensing cost and the selected sensor action.
3. **Mechanism uncertainty:** per-model predictions, explicit disagreement by horizon/regime, and the difference between model-specific and robust actions.
4. **Behavior:** route feasibility, action costs, success/failure, calibrated uncertainty or abstention, and adaptation under the held-out combination.
5. **Ecological consequences:** per-individual outcomes, refuge/forage use, population/resource trajectories if multi-agent behavior is enabled, and any adverse outcome tails.
6. **Evidence receipt:** exact source, config, scenario, split, seed-family, evaluator and budget identity; failed selected episodes remain recorded.

### Comparator ladder and ablations

Use a transparent reactive heuristic, the unchanged candidate baseline, the adaptive candidate, and a privileged planner only as a separately labelled upper bound. Match candidate/baseline initial states, process inputs, observation/action budgets and exogenous disturbance streams wherever a paired pathwise comparison is meaningful. Because genuinely different ecological structures can diverge pathwise, compare distributions and decision regret as well as paired deltas; do not force identical trajectories across different models.

Run at least these ablations:
- disable active sensing while keeping the world identical;
- restore full sensing but hide future/intervention labels from all candidate agents (they should never have them);
- remove the flood while preserving all other random streams;
- remove the resource shortage while preserving flood timing;
- swap the two observationally equivalent model structures;
- reveal one held-out environmental factor only to the evaluator and verify no policy-access change;
- alter rendering/camera/FPS with identical state, process and authorized sensor observations;
- compare matched budgets against an explicitly larger-budget cost-tradeoff run.

### What would count as a result

A synthetic pass means only that the pipeline respects the frozen contract under the selected known-answer cases. A candidate improvement requires a preregistered task metric, a budget-matched comparator, independent seeds/units, uncertainty intervals and no hard invariant/authority violation. Before any ecological or species claim, calibrate the relevant process and behavior layers against licensed, appropriate measurements and report the model's domain of validity.

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
