# LOCO-SIM-REL-001 — locomotion-specific benchmark acceptance

This document specializes the SIM-REL-014 paired/sealed benchmark for the source-aware locomotion adapter. It does not create a second generic optimizer or benchmark harness. Reuse the exact joint-state oracle, independent policy replay, one-step joint EIG baseline, and SIM-REL-014 evaluation protocol.

All targets below are preregistration proposals, not achieved results.

## 1. Biological variables and causal distinctions

The mechanism hypothesis axis should distinguish, at minimum:
- central rhythm / CPG-dominant explanations;
- distributed local sensorimotor-loop explanations;
- hybrid central-plus-local explanations;
- reflex-dominant alternatives;
- mechanics-dominant alternatives.

Do not collapse the physical state and sensor state. The current adapter retains whole-body behavioral context and coarse scenario labels, but now also carries six per-leg mechanical records. Each leg records gait phase, contact state, load, joint-position bin, joint-velocity bin, and actuator-force bin. A targeted contact/load intervention must change only its declared leg; washout must preserve all six leg records. These are bounded synthetic bins rather than calibrated physical units. The current fixture has 18 physical states, 2,048 sensory states, 12 actions and two mechanism hypotheses (884,736 exact joint transition rows). Sensor state keeps position-, movement-, vibration- and contact-feedback channels distinct, acquisition/instrument state separate, and movement-axon presynaptic inhibition separately represented for each of the six legs.

The published FeCO distinctions should stay explicit: position-encoding claw-like afferents, movement-encoding hook-like afferents, and vibration-sensitive club-like afferents. Dallmann et al. report that position-encoding claw axons remain active across behaviours while movement-encoding hook axons are suppressed during walking and grooming by GABAergic presynaptic inhibition with descending input. Their data also distinguish self-generated from passive movements. The adapter's current transitions are synthetic and do not reproduce the measured calcium-imaging distributions.

Source anchor: Dallmann et al., Nature 647, 445–453 (2025), DOI https://doi.org/10.1038/s41586-025-09554-2.

## 2. Challenge families

### A. FeCO modality specificity
- Position perturbation changes only the position channel.
- Movement-afferent channel gating changes only the movement channel.
- Vibration perturbation changes only the vibration channel.
- None of these channel manipulations silently changes presynaptic inhibition or acquisition state.
- Position-encoding feedback remains available in the active movement inhibition fixture.

### B. Active/passive context dependence
- Self-generated movement and passive externally imposed movement are separate action classes.
- Active movement may engage movement-axon presynaptic inhibition in the declared behavioral context.
- Passive movement must not silently inherit the active-state modulation.
- The movement channel remains anatomically present; suppression is represented as presynaptic state, not axon loss or instrument dropout.
- Include negative controls where the active/passive distinction is not identifiable from the available observations. The correct policy must abstain rather than infer the hidden context from a label leak.

### C. Per-leg descending modulation
- Model presynaptic movement inhibition as a six-leg state vector.
- Current limitation: the adapter's mechanical state remains coarse whole-animal phase/contact/load/carryover. Per-leg joint angles, contact forces, segment identity, gait phase and mechanical carryover need an explicit physical-state extension before claiming six-leg embodied dynamics.
- A descending intervention targeting one leg must leave the other five leg states unchanged.
- Compare left/right homologous limbs and front/middle/hind limbs under held-out target-leg tests.
- Test both engagement and release; do not assume the same state is optimal across behaviours.
- A global one-bit controller is a deliberately misspecified baseline.

### D. Physical carryover versus sensor persistence
- Persistent acquisition offset can mimic physical gait/contact state.
- A targeted contact/load intervention changes only its declared per-leg mechanical record, preserving the other five legs and global scenario labels.
- Washout clears carryover while preserving per-leg kinematics/contact/load/force state.
- Joint-position, joint-velocity and actuator-force bins remain synthetic abstractions until calibrated against simulator output.
- Physical load/carryover can persist after an acquisition calibration.
- Calibration may change acquisition without changing physical state.
- Washout may clear physical carryover while leaving sensor drift untouched.
- Repeated samples from one animal/session require history-conditioned likelihoods; an IID updater must not gain false confidence.

### E. Distributed loops and central rhythm
- Local VNC/body-part feedback, intersegmental coordination, descending modulation and central rhythm are separate candidate mechanisms.
- A mechanism can be anatomically plausible without being dynamically sufficient.
- Compare local-loop, central-rhythm, hybrid and mechanics-only alternatives under matched joint-state kernels.
- Challenge the model with interventions that target local feedback while holding central rhythm inputs fixed, and the converse.

### F. Source/materialization and missingness
- Connectome release/materialization and digest changes invalidate the affected source pin.
- Unknown endpoints remain explicit missingness, never silent anatomical absence.
- Connectome-derived edges cannot be used as measured causal-effect labels.
- Every transition-likelihood row must have its own source manifest and claim ceiling.
- Trial assignment, delivered protocol, delivery fidelity, sample ID, animal/preparation ID, session ID, and pre-outcome history digest are independently represented.

## 3. Matched algorithmic baselines

Run every method on the same frozen priors, joint likelihoods, source manifests, action sets, observation histories, costs, hazard budgets, stopping rules and seeds:

1. Seeded random feasible selection.
2. Fixed canonical schedule.
3. Static EIG ranking.
4. SIM-REL-014 one-step joint-state EIG and EIG/cost (baseline::rank_by_one_step_eig).
5. Exact finite-horizon joint stochastic oracle (SIM-REL-013) on tractable cases.
6. COPEx-style constrained multi-step planning when a paper-matched reproduction is feasible. COPEx combines amortized policy/posterior networks with scenario-tree lookahead; the published task suite is generic constrained BED, not fly locomotion.
7. Online partially observed state-space BED with nested particle filters (Perez-Vieites et al., ICML 2026) where its likelihood, particle and compute assumptions can be matched.
8. Deliberately misspecified ablations: collapse all FeCO channels; remove sensor persistence; remove physical carryover; collapse per-leg modulation to a single global bit; treat repeated samples as IID; omit the acquisition reference action.

Do not treat a FlyGym controller as a BED algorithm. FlyGym/NeuroMechFly is the embodied physics/controller baseline, not a substitute for an experimental-design method. The algorithmic and embodied tracks must report separate results.

A contact/load condition is not a generic mechanical reset. Under the LOCO-SIM-REL-001 action contract, it may alter only the declared target leg's contact and/or load variables. Gait phase, joint position, joint velocity and actuator force on that leg are held fixed; all other legs, global context, carryover and sensor/acquisition state are held fixed. A separate actuator, kinematic or perturbation action is required to change those variables.

## 4. Embodied locomotion baseline

Use a pinned FlyGym/NeuroMechFly version (initial candidate: 2.1.1) and compare under matched morphology, contact solver, timestep, sensor streams, actuation limits, control rate, observation budget, physics fidelity, compute hardware and training/calibration budget.

Include at least:
- PID / rule-based control;
- independent and coupled CPG controllers;
- hybrid reflex + CPG;
- learned recurrent/RL policy;
- connectome-constrained controller;
- Symtropy joint-state/controller interface.

Measure gait stability, joint/contact force tracking, disturbance recovery, energy/work, worst-decile performance, active/passive transition robustness, per-leg perturbation response, sensor dropout/drift robustness, and cross-terrain/cross-load generalization. Preserve the simulator's recorded trajectories and exact intervention manifest as provenance.

FlyGym already supports mechanosensory feedback and exposes joint/contact/actuator information and trajectory recording. Do not claim those capabilities as novel. The differentiation target is the source-qualified, intervention-predictive and per-leg causal model integrated with them.

## 5. Primary outcomes and metrics

### Scientific and decision quality
- Held-out intervention predictive log score (primary proper score).
- Brier score and calibration error by intervention, leg, behavioural context, source/evidence tier and animal/session.
- Mechanism identification, false-identification and abstention rates.
- Posterior/interval coverage and calibration under per-leg and active/passive shift.
- Cost and number of independent experimental units to reach a preregistered decision threshold.
- Regret relative to the exact finite-horizon oracle on tiny enumerable kernels.
- Correct response to non-identifiability and omitted-mechanism controls.

### Causal and source integrity
- Cross-modal transition violations accepted: target 0.
- Non-target leg presynaptic changes accepted: target 0.
- Unpinned, missing-source, wrong-materialization, claim-ceiling and source/evidence mismatch accepted: target 0.
- Duplicate sample IDs accepted: target 0.
- Assignment/delivery mismatch accepted as a valid causal trial: target 0.
- Non-target physical or acquisition changes under a modality-specific control: target 0.
- Contact/load actions that also alter target-leg gait phase, joint position, joint velocity or actuator force: target 0.
- Contact/load actions that alter any non-target leg or global behavioural context: target 0.
- Non-target leg mechanical-state changes under a one-leg intervention: target 0.
- Per-leg joint/contact/force prediction error and calibration, reported separately from whole-body gait outcome.

### Systems
- Experiment count and cost-to-target.
- Oracle states expanded, frontier size and replay cost.
- Wall-clock decision latency, memory, throughput and total runtime.
- Particle ESS/depletion and resampling metrics for approximate SSM baselines.
- Missing/failed/delayed intervention rate and operator intervention burden for real systems.

Expected EIG alone is never a sufficient success metric. A policy can maximize information about a misspecified model set and still be confidently wrong.

## 6. Statistical protocol and target thresholds

Use the SIM-REL-014 preregistration protocol:
- freeze source manifests, model families, priors, policies, action costs, risk/abstention budgets, hyperparameter budgets and metrics before holdout evaluation;
- pair methods on identical scenario/seed combinations;
- use at least 30 independent seeds per synthetic family where compute permits;
- report paired effect sizes and 95% paired bootstrap intervals, per-family performance and worst regressions;
- separate development fixtures from sealed holdouts;
- publish all negative controls and failures;
- bind every run to exact source SHAs, kernel digest, seed, configuration and policy/version receipt.

Proposed ambition gates:
- 100% exact agreement with independent policy-tree enumeration for all tractable known-answer kernels.
- Zero false-identification claims on deliberately non-identifiable fixtures under a zero-false-claim policy configuration.
- Zero accepted modality, non-target-leg, source-integrity, assignment/delivery or sample-identity violations.
- At least 10% lower median cost-to-target than matched one-step EIG on the designated per-leg/context-confounding families, with paired 95% intervals excluding zero and no worse risk/abstention.
- No material held-out log-score or calibration degradation compared with the best matched baseline; information gain gains do not count if predictive validity worsens.
- Demonstrated leave-one-leg-out and leave-one-context-out generalization before any claim of biological-domain generalization.

## 7. Claim ceiling

Passing synthetic adapter and oracle tests proves correctness only for the supplied finite kernel, source manifest, prior, policy horizon and action set. It does not establish measured neural dynamics, reproduce the Dallmann circuit, demonstrate animal-level efficacy, certify experimental safety, or beat SOTA.

A SOTA claim requires:
- a paper-matched reproduction of relevant adaptive-BED baselines;
- sealed, source-locked intervention data;
- held-out animal/preparation and session generalization;
- calibrated conditional likelihoods with serial dependence and missingness handled;
- exact-oracle agreement on tractable cases;
- paired-seed and biological-replicate uncertainty intervals;
- independent evaluation of all failures and abstentions.


## 8. External state-of-the-art and industry comparison

The benchmark must compare the right layer with the right competitor. These are not interchangeable systems:

| External reference | What it already does | Correct Symtropy comparison |
|---|---|---|
| COPEx, ICML 2026 | Amortized policy/posterior networks with online scenario-tree lookahead for constrained, multi-step Bayesian experiment design | Experiment count/cost, realized information, regret, hazard feasibility and wall-clock overhead on identical locomotion kernels |
| Perez-Vieites et al., ICML 2026 | Nested-particle-filter online Bayesian design for partially observed nonlinear state-space models, with EIG/gradient estimators | Sequential parameter/state inference, calibration, predictive log score, particle degeneracy and compute on matched partial-observation locomotion models |
| improv, Nature Communications 2025 | Modular orchestration of streaming analysis, data collection, live control and adaptive neuroscience pipelines | Integration reliability, data-loss rate, fault recovery, latency, operator burden and reproducibility; do not mislabel it as a particular EIG planner |
| EmbodiedLab, October 2026 | End-to-end "act to know" evaluation across 30 simulated experiments, five disciplines and 14 apparatus platforms; evaluates knowledge grounded in recorded instrument readings and required physical state | End-to-end task success, experiment-procedure quality, instrument-grounded knowledge, interaction failures and transfer; treat it as an outer benchmark pattern, not a locomotion simulator |
| FlyGym / NeuroMechFly 2.1.1 | Embodied Drosophila simulation, mechanosensory/contact/actuator access, per-segment contact-force queries, trajectory recording and CPG/rule/hybrid controller examples | Matched-physics controller and observation baseline; compare causal prediction and evidence provenance separately from raw physics performance |

Primary references:
- COPEx: https://proceedings.mlr.press/v306/guo26g.html
- Online Bayesian experimental design for partially observed dynamical systems: https://proceedings.mlr.press/v306/perez-vieites26a.html
- improv: https://www.nature.com/articles/s41467-025-64856-3
- EmbodiedLab: https://arxiv.org/abs/2610.35385
- FlyGym 2.1.1 changelog: https://neuromechfly.org/2.1.1/changelog/

The EmbodiedLab authors report that even the strongest evaluated native-control agent succeeds on fewer than half of the tasks, and that apparatus interaction accounts for most failures. This motivates an explicit multi-level scorecard for our future live/simulated instrument interface:
1. **Interface correctness:** did the agent operate the simulator/apparatus and record the intended sample?
2. **Scientific correctness:** is the claimed neural/mechanical result supported by the actual recorded evidence?
3. **Diagnostic correctness:** can the system identify why a run failed, became non-identifiable, or contradicted its model?

A policy must not receive scientific credit for an intended intervention when the delivered intervention or the measurement receipt is invalid.

### Connectome-derived rhythm generation versus closed-loop locomotion

Pugliese et al. (Nature, 28 October 2026; DOI 10.1038/s41586-026-11195-y) establish a strong neural rhythm-generation comparator: their DNg100-driven three-neuron inhibitory/excitatory motif is reported necessary and sufficient for simulated motor rhythms across four connectome datasets, while DNb08 is a distinct pathway with optogenetic confirmation. Treat that as a source-reproduction and neural-dynamics baseline, not as a complete embodied walking controller or proof of tripod coordination.

The benchmark must separate three scorecards:

1. **Neural rhythm generation:** reproduce DNg100/DNb08 source outputs, neuron-level rate trajectories, rhythm frequency, phase and per-dataset circuit identities using the pinned archived data and configs in #3511.
2. **Inter-leg coordination:** evaluate phase relationships and coordinated multi-leg patterns, including the effect of local FeCO/9A gating and distributed brain-and-cord pathways in #3518/#3519.
3. **Embodied gait:** evaluate contact timing, load transfer, slip, stability, disturbance recovery and physical trajectories in pinned FlyGym 2.1.1 using #3357.

A model that succeeds on scorecard 1 must not inherit credit on scorecards 2 or 3. Compare at least central-rhythm-only, local-feedback-only, hybrid CPG+local feedback, distributed brain-and-cord, and mechanics/reflex alternatives with matched interventions and compute budgets. Keep the four Pugliese connectome datasets, female BANC brain-and-cord source, and measured FeCO datasets as separately identified source families. Do not pool them into one synthetic "fly" without an explicit population model.

The paper's Nature data-availability text cites Zenodo DOI 10.5281/zenodo.22260923, while the author repository points to record 22260924. Preserve the discrepancy and the issue #3511 custody requirements; do not silently choose one identifier or claim a reproduction before the source materialization and exact run outputs are independently pinned.

### What a defensible "better than SOTA" claim requires

A superiority claim must be conditional and lane-specific:
- **Experiment design:** beat paper-matched COPEx and online-SSM baselines on paired, sealed locomotion tasks in cost-to-target or regret, while preserving calibration, abstention and safety/risk budgets.
- **Inference:** beat the best matched state-space baseline in held-out intervention log score or calibration, not only in EIG or in-sample fit.
- **Embodied control:** improve pre-registered gait/contact/disturbance metrics under identical morphology, physics solver, sensors, controller compute, training budget and hardware.
- **Integration:** improve end-to-end task completion and evidence integrity without increasing missing samples, failed interventions or operator burden.
- **Scientific validity:** generalize to held-out legs, contexts, sessions and animals, with measured-data claims supported by source-locked biological evidence.

An exact finite-state solution is a correctness oracle for its supplied kernel, not proof of model adequacy. A high-throughput simulator is not evidence of neural mechanism recovery. An improved gait score is not evidence that the connectome explanation is correct. A strong result must pass all applicable lanes without trading away any of these boundaries.

### Proposed matched evaluation report

For every held-out scenario, publish one row per method with:
- exact source/code/model/asset/solver digests and seed;
- observation and intervention history, assigned versus delivered protocol, and sample receipt IDs;
- correct/false identification, abstention and non-identifiability outcome;
- held-out neural/mechanical predictive score and calibration;
- experiment cost, regret, hazard and safety status;
- gait phase/duty factor, per-leg contact/load/force error and disturbance recovery;
- runtime, memory, simulator steps, data-loss/retry count and operator interventions;
- paired effect size, 95% confidence interval and worst-stratum regression.

Use paired seeds for synthetic cases and independent animal/preparation/session units for measured cases. Report per-context and per-leg strata, not only pooled means.