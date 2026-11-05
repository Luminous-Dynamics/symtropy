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

Do not collapse the physical state and sensor state. The current adapter has coarse whole-animal physical archetypes for walking stance/swing, grooming supported/unloaded, and other supported/unloaded contexts, with contact/load/carryover. It now conditions active-movement presynaptic modulation on Walking/Grooming versus Other. A per-leg physical/contact/kinematic state vector is a follow-on extension, not yet implemented. Sensor state keeps position-, movement-, vibration- and contact-feedback channels distinct, acquisition/instrument state separate, and movement-axon presynaptic inhibition separately represented for each of the six legs. Do not imply that per-leg physical dynamics are already represented merely because per-leg presynaptic state is.

The published FeCO distinctions should stay explicit: position-encoding claw-like afferents, movement-encoding hook-like afferents, and vibration-sensitive club-like afferents. Dallmann et al. report that position-encoding claw axons remain active across behaviours while movement-encoding hook axons are suppressed during walking and grooming by GABAergic presynaptic inhibition with descending input. Their data also distinguish self-generated from passive movements. The adapter's current transitions are synthetic and do not reproduce the measured calcium-imaging distributions.

Source anchor: Dallmann et al., Nature 647, 445–453 (2025), DOI https://doi.org/10.1038/s41586-025-09554-2.

## 1A. Published connectome-CPG reference: reproduce first, then extend

The relevant biological baseline is no longer a hypothetical CPG. Pugliese et al. (Nature, version of record 28 October 2026; DOI 10.1038/s41586-026-11195-y) report that DNg100 is the strongest descending driver of rhythmic motor-neuron activity in their connectome activation screen. Simulation pruning identifies a minimal motif with one inhibitory and two excitatory interneurons that is necessary and sufficient for motor rhythms across six legs in four VNC connectome datasets. A separate descending pathway, DNb08, was predicted to drive rhythmic leg movement and was confirmed with optogenetics in behaving flies. The paper provides phase-offset analyses for the six E1 neurons and explicitly distinguishes connectome simulations from experimental confirmation.

This work is an external biological comparator and should be reproduced under the source-lock in [LOCO-PUGLIESE-001 / #3511](https://github.com/Luminous-Dynamics/symtropy/issues/3511). Keep the four connectome materials distinct: MANC v1.2.1, FANC CAVE v1178, Male CNS v0.9 and BANC CAVE v829 are not interchangeable samples. The paper's Data Availability cites Zenodo DOI 10.5281/zenodo.22260923; the authors' code/data link also points to record 22260924. Preserve this discrepancy until independently resolved; never silently substitute one ID for the other.

The differentiating research question is therefore not "can we identify a CPG?" It is:

**Can a source-qualified model predict how the published DNg100/E1-E2-I1 rhythm-generating motif interacts with state-dependent FeCO movement-afferent presynaptic inhibition, position feedback, local reflexes, mechanical contact/load and sparse inter-leg coupling under interventions that were not used to fit it?**

### Mandatory staged comparison

1. **Published-result reproduction:** match the source-locked paper's DNg100/DNb08 stimulation and silencing outputs, neuron identities, parameter distributions, pruning procedure, phase/rhythm outputs and per-connectome results before adding new mechanisms. Do not use the synthetic factor model to claim this reproduction.
2. **CPG-only baseline:** implement the published motif and descending inputs as a source-qualified mechanism family. Score its motor-rhythm prediction separately from six-leg behavioural prediction.
3. **Sensory/mechanical extension:** add distinct position-, movement- and vibration-channel state, active/passive behavioural context, acquisition noise, leg-local joint/contact/load variables, and explicit descending presynaptic-gating factors.
4. **Coupling alternatives:** compare no inter-leg coupling, published CPG cross-leg structure, pairwise contralateral/ipsilateral coupling, tripod-factor coupling, and a sparse learned coupling graph. Anatomical presence, simulated influence and measured intervention effects must remain separate.
5. **Held-out interventions:** freeze forecasts before revealing DNg100/DNb08 stimulation/silencing, movement-vs-position pathway manipulation, active-vs-passive probe, leg-specific descending modulation, joint-limit/contact perturbation, load/terrain transitions and combinations. Hold out at least one connectome materialization, one targeted leg and one behavioural context where data permit.

### Metrics specific to this reference

- Motor-rhythm frequency and phase prediction, including pairwise phase-offset distributions across homologous and non-homologous legs.
- Per-leg motor-neuron population activity log score, calibration and interval coverage.
- Correct response to DNg100/DNb08 stimulation and silencing, including false rhythmicity under negative controls.
- Held-out prediction of behavioural oscillation, contact transitions, joint trajectory and recovery—not rhythm alone.
- Incremental predictive value of sensory gating and coupling over the source-matched CPG-only baseline, measured by paired held-out log-score difference and uncertainty interval.
- Leave-one-connectome-out and leave-one-leg-out generalization; report source/population boundaries and missing endpoints.

The factorized transition prototype in [LOCO-FACT-001 / #3677](https://github.com/Luminous-Dynamics/symtropy/issues/3677) is a candidate implementation substrate for repeated leg-local factors and explicit coupling. It is currently a generic synthetic categorical model, not a Pugliese reproduction or source-fitted Drosophila model.

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
