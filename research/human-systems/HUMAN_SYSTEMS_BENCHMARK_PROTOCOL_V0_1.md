# Human-systems simulation benchmark protocol v0.1

**Status:** proposed protocol; not yet executed.  
**Programme:** [HUMAN-SYSTEMS-SIM-001 / #5625](https://github.com/Luminous-Dynamics/symtropy/issues/5625).

## 1. Evaluation questions

The benchmark must answer separate questions:

1. Does each subsystem implement its declared equations correctly?
2. Does the composed simulation preserve numerical and temporal semantics?
3. Does the model predict relevant human observations on held-out people or recordings?
4. Are predicted uncertainties calibrated?
5. Does it generalize across tasks, devices, datasets and populations?
6. Are intervention/counterfactual outputs valid under the evidence actually available?

A single task-success score or "human realism" score cannot answer all six.

## 2. Tracks

### Track A — verification and numerical behavior

No human data are needed. Test:
- analytic known-answer solutions where possible;
- dimensional and unit consistency;
- state bounds, positivity and conservation laws where applicable;
- solver convergence as integration step shrinks;
- stable replay from identical initial conditions and seeds;
- time-unit invariance after proper rescaling;
- deterministic behavior under event batching / replay;
- explicit rejection of unsupported inputs and parameter ranges.

### Track B — known-truth synthetic worlds

Generate populations with known latent parameters, heterogeneous baselines, repeated measures, missing-not-at-random observations, device noise, event-time delays and known interventions.

Measure:
- parameter recovery and identifiability;
- state-estimation error;
- interval coverage;
- error under sensor dropout and sampling-policy shift;
- false certainty under model mismatch;
- recovery after perturbation;
- counterfactual correctness when the simulator's structural equations define the oracle.

Synthetic qualification is evidence that software behaves as designed in its synthetic world. It is **not** evidence of human validity.

### Track C — public observational physiology

Use dataset-specific adapters and splits. Never pool unrelated source populations into one nominal cohort.

Initial candidates:

- MMASH: 22 healthy volunteers, 24-hour heart/activity/sleep/psychological data with cortisol and melatonin. Can test circadian/temporal associations and observational prediction; cannot establish intervention causality.
- Sleep-EDF Expanded: 197 annotated overnight polysomnographic recordings. Can test signal/sleep-state measurement and selected sleep-model outputs; it does not measure an entire day or cognitive/social behavior.
- OpenNeuro ds003478: 122 participants, resting EEG, single session. Can test a declared resting EEG feature model; it cannot validate day-to-day MDD dynamics or antidepressant selection.
- WESAD: 15 participants, lab baseline/stress/amusement. Scientific non-commercial use only. Can test narrow task-domain physiological/affective classification; do not generalize the results to free-living stress or commercial use.

### Track D — external-domain transfer

Freeze the model before the test domain. Evaluate transfer across:
- participant;
- device/location;
- task/context;
- recording protocol;
- dataset/site where a suitable independent dataset exists.

Report failure and abstention as first-class outcomes. A pooled score may not hide a failed domain.

### Track E — intervention/counterfactual qualification

Only use interventions with a defensible causal design or a mechanistic subsystem whose validity domain includes the intervention. Keep distinct:

- observed association;
- mechanistic what-if inside a model;
- effect estimated under a randomized intervention;
- prediction of an intervention outcome in a new person.

A simulator may show "what this model predicts under action X". It must not say "action X will cause this human outcome" unless independent intervention evidence supports the claim.

## 3. Baselines

Choose baselines by task and compare on identical data/splits/compute:

- population mean and persistence/last-value;
- autoregressive or linear mixed-effects models;
- Kalman/state-space baselines where model assumptions fit;
- a published reference model for each physiological subsystem;
- existing task-specific signal-processing/ML baselines;
- the same Symthaea agent with physiology, memory, HDC, or policy components ablated.

Do not compare a rich multimodal model with a weak baseline that lacks the same observation history, input modalities or subject-specific calibration.

## 4. Metrics

### Physical / physiological trajectories
- MAE and RMSE in the original physical units;
- normalized error only when the normalization basis is predeclared;
- trajectory shape / timing errors for event responses;
- conservation and boundedness violations;
- time-step convergence error;
- predictive log score or CRPS for probabilistic forecasts;
- 50/80/90/95% interval coverage and interval width.

### Sleep and EEG
- Sleep-stage macro-F1 / balanced accuracy / Cohen's kappa against expert labels where appropriate;
- sleep onset/offset and stage-transition timing error;
- subject-held-out performance and calibrated class probabilities;
- EEG feature uncertainty and measurement-quality rejection.

### Behavioral policy
- action frequency and transitions;
- task completion and recovery time under matched scenarios;
- energy/cost and opportunity cost;
- calibration of action probabilities where measured choice labels exist;
- preference/goal consistency only where the labels were actually observed.

### Transfer / robustness
- per-person and per-domain score distributions;
- worst-domain and lower-quantile performance;
- OOD detection and abstention coverage;
- error under missingness, time-shift, sensor drift and intervention shift;
- subgroup reporting only where lawful and adequately powered.

No leaderboard score alone establishes a human digital twin.

## 5. Data splitting and leakage controls

- Split by person, never random rows/windows from the same person across train and test.
- Keep longitudinal records grouped by participant.
- Freeze preprocessing, parameter fitting and model selection before the test fold.
- Do not use test outcomes to set thresholds or tune the simulator.
- Treat sensor sampling/missingness as a process that may itself shift.
- Preserve raw source data locally; public repository fixtures must be synthetic.
- Record dataset version, license, checksums, preprocessing version, split manifest, seeds and model revision in every receipt.

## 6. Suggested first benchmark

**Human Day v0.1: sleep, activity and recovery under workload**

Scope:
- one virtual participant, then a heterogeneous synthetic cohort;
- explicit event-time clock and local-day schedule;
- a published sleep/circadian submodel selected and documented before coding;
- activity/workload as a separate input process;
- measured signals and latent states remain distinct;
- a small set of transparent behavioral policies (continue / pause / rest / change task);
- no psychiatric diagnosis or medication intervention.

First public-data checks:
1. Sleep-EDF: qualify sleep-state observation handling and selected sleep-related predictions.
2. MMASH: test limited 24-hour heart/activity/sleep associations on participant-held-out splits.
3. WESAD only as a non-commercial, narrow stress-task evaluation if its licence is compatible with the intended research.

A successful first benchmark would show reproducible subsystem behavior, bounded prediction error and calibrated uncertainty on these narrow tasks. It would not establish a universal human simulator.

## 7. Go / no-go gates

**Go to the next subsystem** only if:
- exact model equations, units and parameters are versioned;
- all numerical and replay tests pass;
- subject-held-out evaluation improves on matched baselines;
- uncertainty is reported and calibrated;
- model failure is exposed under out-of-domain conditions;
- the result is reproducible from a public protocol.

**No-go / downgrade claim** when:
- the model is non-identifiable from available observations;
- uncertainty is miscalibrated or unsupported;
- external data reveal substantial negative transfer;
- the intervention is outside the model's validity domain;
- a sensor or data license does not permit the intended use.

## 8. Reporting

Every report must state:
- context of use and claim level;
- model, data and code revisions;
- source data and licence;
- participant-level split procedure;
- assumptions and unsupported states;
- uncertainty and sensitivity results;
- model failures and negative controls;
- whether results are synthetic, observational, or intervention-based.

