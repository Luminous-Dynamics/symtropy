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


## 10. Existing Symtropy substrate reconnaissance — 2026-11-14

The public Symtropy source already contains adjacent systems. HMP should qualify these before creating a second simulation stack. This is source inspection only; it is not a test or validation result.

| Existing surface | Source-level observation | HMP implication |
|---|---|---|
| src/systems/psychology.rs | A Bevy NPC system updates approximately every 3 seconds. Its PsychologicalNeeds state contains allostatic_load, social_satiation, engagement and burnout_ticks. Fixed constants govern baseline stress, isolation load, danger load, load decay, proximity bonus, burnout and collapse. The player counts as a nearby agent. | Treat this as a game-NPC heuristic, not a validated human psychological model, clinical construct, diagnostic tool or evidence about real people. Every constant and proxy needs an explicit assumption register and calibration target. |
| Social-proximity rule | Nearby-agent count influences social satiation; a code comment describes cooperation as “thermodynamically mandatory.” | This is a normative/game-design assumption, not an established law of psychology or thermodynamics. The model must represent chosen solitude, different social needs, safe boundaries, non-proximity relationships and the possibility that isolation is harmful in one context but voluntary and restorative in another. |
| src/systems/ai_player.rs | An optional FEP ActiveInferenceAgent controls one game player from an 8-dimensional observation. | This is an embodied control demonstration, not a population-level social simulation and not evidence that an agent's choices model human preferences. |
| crates/domains/symtropy-world | The bridge exposes a seeded simulation thread, monthly ticks, snapshots, governance/economy/faction summaries and player actions. | This is a natural future adapter for HMP experiment receipts if its dependency and replay contract are qualified. |
| symtropy-world dependency | Its Cargo.toml references mycelix-multiworld-sim at a relative path four directory levels above the crate. That companion source was not present at the public Symtropy repository root during this inspection. | Standalone reproducibility/buildability of this bridge is not established by the public repository snapshot. Resolve and pin the dependency before claiming the social-simulation stack is runnable from this repository alone. |
| src/systems/settlement.rs | The settlement demo includes fixed option effects and local fallback behavior when a governance port call fails. | Treat as a gameplay scenario, not an empirical governance model. Separate illustrative mechanics from validated causal mechanisms. |

### 10.1 Qualification target: the existing NPC psychology heuristic

The first target is not to make the model more biologically elaborate. It is to determine whether its constructs and causal assumptions are defensible for the intended game scenario, and to prevent the code from being misrepresented as real human psychology.

| Current design element | Qualification question | Required negative control / limitation |
|---|---|---|
| Allostatic load [0,1] | What observable game-state outcome is this variable intended to predict? | Do not call it a clinical allostatic-load measurement or infer a player's health. |
| Social satiation | Does proximity predict the intended outcome better than voluntary contact, relationship quality, communication and perceived support? | A nearby NPC is not automatically a trusted or meaningful relationship. |
| Isolation load | Is the effect conditional on the NPC's preferences, safety, context and chosen activity? | A voluntarily solitary NPC must not accrue automatic pathology merely because it is alone. |
| Fixed stress / relief constants | Are parameters calibrated to a declared game telemetry target or independently collected research data? | Arbitrary constants must be labeled illustrative and sensitivity-tested, not cited as empirical coefficients. |
| Burnout and collapse thresholds | Do the thresholds create intended gameplay behavior without encoding stereotypes or coercive social norms? | Game state thresholds are not diagnostic cutoffs for humans. |
| Player counts as nearby agent | Does the player actually interact, communicate or provide support? | Mere geometric proximity must not be treated as consent, trust or support. |
| “Cooperation is thermodynamically mandatory” | Is this a fictional gameplay law, or a testable empirical statement? | It must remain explicitly a fictional mechanic unless independently demonstrated; thermodynamic language must not be used to legitimize a moral obligation to socialize. |

### 10.2 First bounded experiment: social contact is not one variable

Build a controlled scenario with otherwise matched NPCs and vary the social condition independently:

1. voluntary solitude with meaningful self-directed activity;
2. unwanted isolation with no available support;
3. frequent proximity without meaningful interaction;
4. trusted relationship with low physical proximity;
5. social contact that is coercive, unsafe or unwanted;
6. reciprocal group activity with explicit choice and exit.

Pre-register the outcome for the game mechanic (for example, engagement or task completion), the population of simulated NPCs, the observation horizon and the parameter range. Keep all six conditions synthetic until a lawful, ethically reviewed empirical dataset is available.

Compare:
- current proximity-only rule;
- a richer relationship/choice state model;
- a simple statistical baseline;
- an ablation with the social effect removed.

Report sensitivity and worst-case behavior. Do not tune the model to force cooperation to win. The purpose is to discover when, for whom and under what assumptions each mechanism changes the declared game outcome.

### 10.3 Architecture and evidence requirements

Before integrating this with HMP:
- separate game-mechanic state from any human-facing psychological statement;
- give each parameter a source, rationale, range, units and uncertainty;
- record the exact seed, code revision, config digest, tick horizon and output digest;
- make runs replayable, and distinguish deterministic logic from stochastic events;
- expose per-agent and distributional outcomes, not only population averages;
- run ablations and negative controls for proximity, consent, agency, safety and chosen solitude;
- do not use personal mental-health data, infer users' diagnoses or recommend psychological interventions;
- require independent human-factors and domain review before using any result to make claims about people.

### 10.4 Existing integration gap

The HMP-001 research branch does not add a new social-simulation engine to Symtropy. The next implementation should first identify the actual source and version of mycelix-multiworld-sim, then define an adapter that emits the experiment receipt in Section 8. If that dependency cannot be pinned and built in the public repo, keep the benchmark scenario runner independent and label the bridge integration as blocked rather than silently relying on a private workspace path.

## 9. Research gap and competitive objective

Industry and academic systems often optimize interaction quality, agent believability or output similarity. This programme should compete on a harder standard: **reproducible prediction of held-out collective outcomes, explicit mechanism tests, valid intervention forecasts and transparent failure characterization.**

A defensible SOTA claim requires a matched public tournament against classical ABMs, statistical models, stateless and persistent LLM agents, and the best available frontier/open-weight systems. It also requires independent reproduction. Until then, this is a proposed methodology—not a performance result.
