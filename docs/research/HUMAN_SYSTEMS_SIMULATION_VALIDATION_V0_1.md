# Human-Systems Simulation Validation Protocol — HMP-001 v0.1

**Status:** Research protocol; no social-simulation runner, empirical dataset or validated model is delivered by this document.  
**Date:** 2026-11-14  
**Parent:** [Symthaea HMP-001](https://github.com/Luminous-Dynamics/symthaea/blob/research/human-meaning-pluralism-v0-1/docs/research/HUMAN_MEANING_PLURALISM_V0_1.md)

## 1. Thesis

Symtropy should help investigate human systems by testing explicit models of mechanisms and consequences—not by generating realistic-looking societies and treating their outputs as truth.

The central shift is from **persona plausibility** to **operational validity**:

> A model is useful only to the extent that its stated mechanism and target outcomes are supported by calibration, held-out empirical tests, robustness analysis and (when causal effects are claimed) credible intervention evidence.

This addresses a documented weakness in generative agent-based modelling. A 2025 systematic review describes empirical grounding, calibration, interpretability and alignment between intended mechanisms and validation targets as unresolved challenges. [Larooij & Törnberg, 2026 issue](https://doi.org/10.1007/s10462-025-11412-6)

A June 2026 replicated forum study illustrates why multiple levels matter: it ran 30 independent 30-day simulations against 30 matched empirical windows. Some aggregate activity measures aligned, while comments, thread length, toxicity distribution and network structure still diverged. [EPJ Data Science](https://link.springer.com/article/10.1140/epjds/s13688-026-00674-x)

## 2. Model classes and baselines

Each benchmark scenario must compare, where relevant:

1. a transparent rule-based or classical agent-based model;
2. a calibrated statistical baseline;
3. a stateless LLM-agent baseline;
4. a persistent-state LLM-agent baseline;
5. a Symthaea-driven agent using documented state and decision interfaces;
6. a hybrid model in which explicit mechanisms constrain generative components.

Use matched budgets, information access, agent count, simulation horizon and observation schedule. Record exact model/version, prompts, tools, sampling parameters, memory policy, environment version and random seeds.

A simulation with more information, more compute or more tuning does not win a fair comparison merely because it fits better.

## 3. Validation layers

| Layer | Question | Evidence |
|---|---|---|
| Input validity | Do inputs represent the target population, period and setting? | Sampling frame, missingness analysis, provenance, consent and measurement review |
| Mechanism validity | Does the proposed process exist and behave as specified? | Mechanism tests, ablations, causal diagrams, sensitivity and negative controls |
| Outcome validity | Does the model reproduce target distributions on held-out data? | Predictive intervals, distributional and temporal metrics |
| Interaction validity | Do network and group dynamics match? | Degree, clustering, assortativity, reciprocity, component and core-periphery comparisons |
| Intervention validity | Does the model correctly predict changes after interventions? | Randomized/credible quasi-experimental evidence; explicit identification assumptions |
| Transfer validity | Does performance survive changes of place, time, language or institution? | External validation and documented domain-shift tests |

Text style, persona consistency and human-rated plausibility may be useful diagnostics. They are not substitutes for the layers above.

## 4. Candidate benchmark scenarios

Begin with bounded, well-instrumented scenarios for which lawful empirical or experimental references can be obtained:

- **Public-goods cooperation:** contribution, free-riding, reciprocity, punishment and recovery after defection.
- **Commons governance:** rule formation, monitoring, sanctions, participation and resource sustainability.
- **Information diffusion:** exposure, correction, rumor propagation, trust and network topology.
- **Fair allocation:** scarce resources, competing needs, procedural legitimacy and distributional consequences.
- **Community crisis coordination:** information delays, local knowledge, resource sharing and recovery.
- **Institutional participation:** voice, abstention, perceived legitimacy, appeal and minority protections.

Do not create stereotyped agent profiles from religion, ethnicity or nationality. A group label is not an individual behavioral parameter. Agent heterogeneity should be grounded in validated measurements or explicitly synthetic, and its assumptions must be exposed.

## 5. Experimental protocol

- Freeze a benchmark card before runs: target population/system, mechanism, outcomes, intended use, inclusion/exclusion rules, data provenance, consent and pre-registered metrics.
- Split by time, community/site or experimental cohort—not random rows alone—so near-duplicate observations cannot leak across calibration and test partitions.
- Use a calibration split, validation split and locked external test split. Never tune on the final test set.
- Run at least 30 independent stochastic seeds per configuration for exploratory replication; calculate the required count from a pre-registered precision/power analysis for any confirmatory claim. Thirty is an exploratory floor, not a universal adequacy guarantee.
- Report distributions and intervals, not only best runs or averages. Include failures, missing outputs and outlier runs.
- Use matched negative controls and ablations: remove memory, remove network structure, shuffle group attributes, break a causal edge, or replace the mechanism with a base-rate model.
- Preserve an immutable experiment manifest: code revision, dataset digests, transformations, parameters, seeds, agent configuration, output digests and analysis revision.
- Distinguish observational prediction from causal inference. A good fit to historical correlations does not by itself justify an intervention recommendation.

## 6. Metrics

**Individual/behavioral:** event probability calibration (Brier/log loss), transition errors, sequence likelihood where defensible, abstention and subgroup performance.

**Collective/network:** degree and clustering distributions, reciprocity, assortativity, community structure, component sizes, repeated interactions, network turnover and concentration of influence.

**Temporal:** event timing, survival/hazard behavior, autocorrelation, volatility, recovery time and forecast interval coverage.

**Distributional:** Wasserstein distance or Jensen–Shannon divergence where suitable; report metric assumptions and sensitivity to binning/support.

**Intervention:** absolute and relative effect error, interval coverage, treatment-effect heterogeneity and direction accuracy against identified experimental evidence.

**Robustness:** variation across seeds, prompt/model versions, populations, missingness, shocks and plausible alternative assumptions.

No single metric proves a society model valid. The target must match the claim.

## 7. Proposed acceptance gates

1. Positive predictive skill against a declared base-rate/statistical baseline on locked data.
2. Empirical calibration reported with uncertainty; no claim of calibration from one run.
3. No subgroup or site is hidden by a population-wide average; report worst-group performance and sample sizes.
4. Mechanism ablations produce predicted changes; if they do not, weaken or reject the mechanism claim.
5. Intervention forecasts are scored only against credible intervention evidence and stated identification assumptions.
6. A model that matches aggregate totals but misses network structure or distributional harms cannot pass a collective-behavior qualification.
7. No real-world intervention is authorized by a simulation result alone; affected communities and responsible institutions retain decision authority.

Numerical thresholds should be fixed per scenario after a calibration-only pilot and before the blind test. Do not select thresholds after observing test results.

## 8. Symtropy / Symthaea / Mycelix / Sol-Atlas interface

- **Symtropy:** deterministic replay where possible, typed scenario state, seeded stochasticity, event logs, parameter sweeps, metrics and reproducible receipts.
- **Symthaea:** propose competing mechanisms, state uncertainty, identify confounders and design counterfactual questions. It does not silently alter the frozen benchmark.
- **Mycelix:** record authority, purpose, consent, data permissions and audit receipts. Unknown or revoked rights block use.
- **Sol-Atlas:** attach historical/geographic context, source provenance, temporal boundaries and the limits of generalization.

An experiment receipt should identify its target mechanism, model class, exact inputs and digests, run seeds, software revisions, evaluation code, metrics, confidence intervals, negative controls and claim ceiling.

## 9. Research gap and competitive objective

Industry and academic systems often optimize interaction quality, agent believability or output similarity. This programme should compete on a harder standard: **reproducible prediction of held-out collective outcomes, explicit mechanism tests, valid intervention forecasts and transparent failure characterization.**

A defensible SOTA claim requires a matched public tournament against classical ABMs, statistical models, stateless and persistent LLM agents, and the best available frontier/open-weight systems. It also requires independent reproduction. Until then, this is a proposed methodology—not a performance result.
