# CivBench v0 — Civilizational Simulation Evaluation Specification

**Status:** Proposed benchmark contract; no benchmark result is claimed.  
**Version:** 0.1.0  
**Date:** 2026-11-02  
**Implementation target:** Symtropy simulation core, with evidence manifests from Mycelix and spatial inputs from Sol-Atlas; optional cognitive mechanisms from Symthaea.

## 1. Why this benchmark exists

Civilizational simulations are easy to make visually compelling and difficult to validate. CivBench evaluates whether a model answers a bounded question, conserves what should be conserved, reproduces known patterns, generalizes beyond calibration cases, and reports uncertainty honestly.

CivBench must not reward narrative plausibility instead of empirical fit; large agent counts without mechanism validation; historical leakage; arbitrary reward functions that encode the conclusion; a single aggregate civilization score; or consciousness proxies used as moral authority.

## 2. Required benchmark manifest

Every scored task run records:
- benchmark ID and semantic version;
- task definition, outcome, unit of analysis, and intended use;
- dataset IDs, versions, retrieval dates, content hashes, and licence;
- training/calibration cutoff and evaluation period;
- geographical and cultural scope;
- model repository, commit SHA, binary/build digest, and dependency lock;
- configuration digest, random seeds, replications, and environment;
- metric definitions, aggregation method, missing-data rules, and baselines;
- uncertainty representation and interval coverage;
- exclusions, failed runs, and deviations from pre-registration.

A run without a complete manifest is exploratory and cannot appear on the scored leaderboard.

### Relationship to the existing Humanity Atlas contracts

CivBench is the **scientific evaluation layer**, not a replacement for the existing scenario/run artifact contract.

Compose the existing open draft work:
- [Symtropy PR #1571](https://github.com/Luminous-Dynamics/symtropy/pull/1571) owns the generic Humanity Atlas scenario/run schema, run classification, evidence-gated model-credibility procedure, and exact-artifact qualification harness.
- [Luminous Dynamics PR #3760](https://github.com/Luminous-Dynamics/luminous-dynamics/pull/3760) supplies the model-specific multiworld simulator output and technology-tree producer.
- [Sol-Atlas PR #79](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/79) supplies the run-envelope/source-artifact consumer.
- [Symtropy PR #369](https://github.com/Luminous-Dynamics/symtropy/pull/369) defines civilization authority boundaries; [PR #399](https://github.com/Luminous-Dynamics/symtropy/pull/399) defines astrobiology authority and uncertainty boundaries.
- [Symtropy PR #378](https://github.com/Luminous-Dynamics/symtropy/pull/378) defines causal resource/inventory/shipment primitives; [PR #391](https://github.com/Luminous-Dynamics/symtropy/pull/391) defines a deterministic Three Worlds composition lab.

These are open draft PRs at the time of this specification. Their authored tests and schemas are not merged-code or exact-head execution evidence. Humanity Atlas run artifacts should be reused for model identity, source bytes, configuration, output digests, and qualification receipts. CivBench adds the *task-specific* evaluation design: locked forecast origins and holdouts, outcome definitions, comparison baselines, metrics, calibration, transfer tests, physical invariants, causal-path checks, and failure analysis.

Repository audit and specific validation gaps are recorded in [CIVILIZATION_BASELINE_AUDIT_2026-11-02.md](CIVILIZATION_BASELINE_AUDIT_2026-11-02.md). In particular, the existing multiworld simulator is the engine to compose and audit; this specification does not propose a parallel general-purpose simulator.


## 3. Evaluation tracks

### CIV-1 — Historical information and forecast integrity

**Question:** Can a model make a calibrated forecast using only information available at a historical forecast origin?

Protocol:
1. Choose a documented society, region, or event series.
2. Freeze a forecast origin and evidence cutoff.
3. Exclude sources published or observations available after that cutoff.
4. Train and calibrate on earlier information only.
5. Predict a pre-registered outcome at a defined horizon.
6. Evaluate against a sealed holdout.

Required baselines: persistence, historical mean, linear trend, simple regression, and appropriate ARIMA/state-space alternatives.

Metrics: Brier/log score for categorical probabilities; CRPS or log score for continuous distributions; calibration curves; nominal interval coverage; skill relative to the best baseline.

Splits: time-forward holdout, plus leave-one-region/culture-out where sample size permits. Random row splits are prohibited where they leak temporal or spatial dependence.

### CIV-2 — Mechanism and counterfactual validity

**Question:** Does a proposed mechanism explain a pattern and predict response to an intervention?

State the causal graph and assumptions before the experiment. Identify observed, latent, and unmeasured variables. Use quasi-experimental or natural-experiment evidence where appropriate. Compare alternative causal structures and non-causal predictive models. Run negative controls, placebo interventions, sensitivity analysis, and parameter identifiability checks.

Predictive improvement is not causal evidence by itself. Counterfactual scenarios must identify which assumptions produce the result.

### CIV-3 — Crisis response and resilience

**Question:** How do interacting social, ecological, and infrastructure systems respond to shocks?

Shock families: drought, heat, flood, crop failure, epidemic, conflict, migration surge, energy interruption, water-system failure, transport disruption, financial shock, and information failure.

Report service availability, time to impact, defensibly modelled welfare loss, time to recovery, recovery overshoot, cascading failures, network criticality, unequal exposure, adaptation, institutional change, ecological recovery, displacement, and loss of autonomy.

Never define resilience solely as continuity of a state, market, or institution.

### CIV-4 — Resource and physical closure

Test energy and material stock-flow balance; water inputs, storage, recycling, and losses; population births, deaths, and migration; land and food production capacity; infrastructure capacity and degradation; unit consistency; and non-negative stocks unless signed values are explicitly defined.

**Gate:** Hard invariant violations fail the run. Numerical tolerances are documented before benchmarking. Unexplained residuals may not be patched after results are observed.

### CIV-5 — Cooperation and governance

Test repeated public-goods provision, common-pool resource management, bargaining, dispute resolution, coalition and federation formation, emergency allocation, treaty compliance, misinformation, strategic manipulation, and Byzantine or Sybil-like participation.

Compare centralized, decentralized, federated, and baseline rule systems under the same conditions.

Report efficiency and stability alongside participation, distribution, privacy, appeal, autonomy, rights violations, and power concentration. No one metric stands in for all of these.

### CIV-6 — Learning, memory, and technological diffusion

Test local versus network-wide diffusion, adoption under variable cost/benefit/risk, intergenerational skill transmission, memory loss under migration/catastrophe, knowledge concentration, single points of failure, and innovation under diversity and constrained resources.

Compare simple contagion, threshold diffusion, network learning, and agent-cognition mechanisms. More expressive agents must demonstrate added predictive or explanatory value.

### CIV-7 — Cross-context transfer

Test whether a mechanism calibrated in one case explains another without hidden re-tuning. Use held-out contexts with explicit cultural, ecological, institutional, and measurement differences. Report where transfer succeeds, degrades, or fails. Do not force common categories where historical sources describe different social structures.

### CIV-8 — ExoCiv observation and detection

Test whether an analysis pipeline distinguishes simulated artificial signals from natural phenomena and human interference.

Use blind injection datasets with controlled signal strength, duty cycle, drift, bandwidth, distance assumptions, observing geometry, and confounders. Report detection completeness, false-positive rates, sensitivity curves, and follow-up requirements.

A synthetic detection benchmark validates analysis logic only. It does not establish that extraterrestrial life or civilization exists.

## 4. Baseline ladder

Each task reports the strongest applicable result from:
1. no-change/persistence baseline;
2. historical mean or climatology baseline;
3. linear trend and simple regression;
4. appropriate established statistical/time-series model;
5. parsimonious system-dynamics model;
6. conventional agent-based model with fixed behavioural rules;
7. agent model with learned or cognitive components;
8. full multi-layer model.

A more complex tier is an improvement only if it beats relevant simpler baselines on the pre-registered task without concealing worse calibration, larger uncertainty, or unacceptable invariant violations.

## 5. Scoring and provisional acceptance targets

CivBench v0 first establishes reproducible baselines. These are engineering goals, not current performance claims.

| Dimension | Initial acceptance target |
|---|---|
| Provenance | Every scored run has a complete manifest and content-addressed inputs |
| Temporal leakage | Zero known post-origin leakage in the scored forecast set |
| Calibration | Nominal 90% and 95% intervals show empirical coverage consistent with uncertainty and sample size |
| Predictive skill | Aim for at least 10% lower proper scoring loss than the strongest simple baseline on a pre-registered task; publish confidence intervals and per-case results |
| Physical closure | Zero unexplained hard-invariant violations beyond declared solver tolerance |
| Reproducibility | Independent rerun reproduces deterministic outputs or stochastic distributions within specified tolerances |
| Transfer | Held-out context results are reported separately; no hidden per-case tuning |
| Ablations | Every major added mechanism has a measured effect and failure analysis |
| Negative controls | Placebo and impossible-cause controls behave as specified before scoring |
| Ethics | No civilization ranking or consciousness-based allocation of rights or authority |

The 10% predictive-skill target is aspirational and must not justify changing the benchmark after seeing the holdout. If a baseline is near the noise floor, improve the task definition rather than inventing an improvement.

## 6. Model validation requirements

Use multiple validation forms:
- verification of code and numerical implementation;
- empirical calibration against relevant observations;
- pattern-oriented validation across independent patterns;
- parameter sensitivity and identifiability;
- posterior predictive checks where applicable;
- uncertainty propagation from data through parameters to conclusions;
- docking against simpler or independently implemented models;
- expert and community review of whether the conceptual model represents its context.

For agent-based models, keep conceptual, behavioural, and computational assumptions visible. A language model's plausible dialogue is not evidence that its simulated agents reproduce actual human decision processes.

Method references:
- Collins, Koehler & Lynch (2024), *Methods that support the validation of agent-based models*: https://doi.org/10.18564/jasss.5258
- *How to keep it adequate: A protocol for ensuring validity in agent-based simulation* (2023): https://doi.org/10.1016/j.envsoft.2022.105559
- *Insights into the accuracy of social scientists' forecasts of societal change* (2023): https://doi.org/10.1038/s41562-022-01517-1
- 2025 review on validation of generative-agent social simulations: https://pubmed.ncbi.nlm.nih.gov/41268053/

## 7. First executable reference model

Start with a deliberately small model before adding high-level cognitive agents.

**State variables**
- population by age/skill cohort;
- household food and water demand;
- agricultural and ecological production;
- energy supply and infrastructure capacity;
- public-goods stock and maintenance backlog;
- institution/coordination-rule state;
- trade, migration, and resource-network edges.

**Processes**
- births, deaths, migration, and skill transitions;
- production, consumption, storage, and spoilage;
- infrastructure depreciation and repair;
- resource exchange and allocation;
- shock arrival and recovery;
- rule changes and collective decisions;
- local learning and bounded information transmission.

**First scenario:** A small set of interacting settlements experiences variable rainfall and a multi-period water shortage. Compare independent local extraction, fixed centralized allocation, negotiated federation with transparent shared measurements, and negotiated federation with delayed or manipulated information.

Keep resource budgets identical. Report service, crop, health/welfare, distributional, and ecological outcomes. Add no consciousness or coherence gate to decision rights.

**Minimum tests:** closed-system conservation; deterministic replay with fixed seed; monotonicity and boundary conditions; no-negative-stock; independent accounting implementation; shock/no-shock control; adversarial measurement and strategic-reporting controls; baseline comparison.

This is a reference experiment, not a complete civilization model.

## 8. Runtime and scale benchmarks

Do not target agent counts before the model is valid. Once the reference model passes correctness tests, report performance at fixed state and interaction complexity for 1k, 10k, and 100k agents. Include wall-clock time, memory, allocation rate, replay cost, and scaling curves. Separate CPU, GPU, and renderer measurements. Report hardware, compiler, build profile, and feature flags.

The simulation engine should be separable from Bevy rendering so headless batch experiments and CI are first-class.

## 9. Release policy

- Version benchmark datasets, task definitions, and scoring scripts.
- Publish test/evaluation split hashes and keep blind holdouts private until a release is scored.
- Freeze the benchmark before evaluation.
- Record all seeds and failed runs.
- Publish negative and inconclusive results.
- Do not call queued CI a pass.
- Do not call a result SOTA without a named baseline, matched task, matched data, matched compute budget, and independent reproduction.

**CivBench v0 succeeds when a researcher can reproduce a result, inspect its evidence, run the baselines, and identify the conditions under which the model fails.**
