# Human observation-process benchmark protocol v0.1

**Status:** research protocol; not an implemented sensor simulator or human-validation result.  
**Programme:** [HUMAN-SYSTEMS-SIM-001 / #5625](https://github.com/Luminous-Dynamics/symtropy/issues/5625)  
**Implementation track:** [HUMAN-OBSERVATION-SIM-001 / #5664](https://github.com/Luminous-Dynamics/symtropy/issues/5664)

## 1. The research shift

A useful human-systems simulator must model two different things:

1. **The person/world:** latent state trajectories produced by explicitly versioned physiological, behavioural and environmental models.
2. **The evidence about that world:** what a real or simulated sensor sampled, when it sampled it, what it measured, what it missed, how it was corrupted, and when the result became available.

A model that receives clean, synchronized, noise-free state variables is solving a different problem from one that receives delayed wrist PPG, intermittent actigraphy, incomplete sleep diaries and asynchronous environmental measurements. A synthetic world that skips the observation-generating process risks validating the estimator against information no deployed system would possess.

The target architecture is therefore:

```
latent world / known truth
        |
        v
model-specific state trajectories
        |
        v
sensor response + sampling policy
        |
        v
noise, calibration, drift, missingness
        |
        v
transport, timestamps, delay and packet faults
        |
        v
observation events actually available
        |
        v
estimator / Symthaea adapter
        |
        v
state estimate + uncertainty + abstention
        |
        v
scoring against isolated hidden truth
```

The truth generator and observation interface must be separate. The estimator must not be able to read ground-truth state, latent parameters, or future events. This creates a testable boundary for leakage, observability and identifiability.

## 2. Ownership and composition

This protocol intentionally does not define another belief-state or generic sensing framework.

| Concern | Existing owner | Human-systems responsibility |
|---|---|---|
| Human-system fidelity ladder and subsystem scope | Symtropy #5625 | Use the declared model and evidence labels |
| Typed observation semantics and FEP integration | Symthaea #12407 | Produce realistic synthetic events and test the adapter after the shared contract is agreed |
| Observation cadence and availability latency | Symtropy #5492 | Reuse the cadence/latency concepts; add human sensor and common-cause fault regimes |
| Multi-fidelity admission and escalation | Symtropy #4487 | Compare sleep/circadian model alternatives under matched context and cost |
| Benchmark adequacy, calibration, transfer and compute accounting | Symtropy #4392 | Supply human-specific tasks, splits and negative controls |
| Irregular-series baselines and active sensing | Symthaea #9700 / #9817 (referenced by #12407) | Use the existing benchmark/policy owners |

Do not implement the Symthaea observation envelope in Symtropy. Do not add a second FEP system. The Symtropy side owns synthetic latent-world generation, sensor simulation, event delivery and a hidden-truth scoring oracle. The Symthaea side owns interpretation and belief-update semantics.

## 3. Four clocks, not one timestamp

Keep these clocks separate:

- **Physical/event time:** when a state transition occurs in the simulated world.
- **Capture time:** when a sensor samples the physical world, in a declared device clock domain.
- **Availability time:** when the sample becomes available to the estimator after buffering, QA or transport delay.
- **Processing time:** when the estimator consumes the sample.

A packet delivered late must not advance physiology again. A duplicate packet must not become a second physical event. Capture-clock drift must not be mistaken for a real change in sleep or circadian phase. A future sample must not cross the benchmark's information cutoff.

For each event, preserve raw clock value, clock-domain identity, any declared clock transform and uncertainty in that transform. Do not silently rewrite event time when correcting a clock.

## 4. Sensor model contract

Every synthetic sensor profile must declare:

- sensor ID, modality, quantity, unit, placement and version;
- calibration profile and valid operating range;
- transfer function / dynamic response where relevant;
- nominal and actual sampling policy;
- noise model and parameter provenance;
- bias, gain error, drift, quantisation, clipping and censoring;
- missingness mechanism and its explicit dependency;
- device state, battery/power state and common-cause dependencies;
- clock domain, offset/drift model and timestamp uncertainty;
- transport delay, packet loss, duplication and reordering model;
- deterministic scenario and seed identity.

Do not assume all measurement errors are Gaussian, all sensors are independent, all sampling is periodic, or all missingness is random. These can be controlled baselines, not universal defaults.

### Missingness semantics

Test the standard missingness distinctions explicitly:

- **MCAR:** missingness independent of observed and latent values under the declared generator.
- **MAR:** missingness depends on recorded/observed history or covariates.
- **MNAR:** missingness depends on an unobserved value or latent process (for example, a sensor loses contact under a movement regime that is itself related to the state being estimated).

The generator must preserve the mechanism label and dependency. Do not allow an estimator to infer a physical state from a missing-value sentinel. An all-missing observation should contribute no measurement likelihood; prior dynamics may still advance according to the model's declared physical time.

## 5. Canonical synthetic scenarios

The initial catalogue is available as a machine-readable fixture at [HUMAN_OBSERVATION_SCENARIOS_V0_1.json](HUMAN_OBSERVATION_SCENARIOS_V0_1.json). It contains 12 arbitrary-unit scenarios, a scorer-only truth boundary and explicit expected invariants. It is a specification fixture, not a generator implementation. Each executed run must freeze a config digest and separate truth oracle:

| ID | Scenario | Required discriminator |
|---|---|---|
| HOBS-00 | Clean complete regular stream | Exact observation and legacy-adapter parity |
| HOBS-01 | Irregular capture cadence with jitter | Physical-time result must not depend on packet count |
| HOBS-02 | MCAR iid dropout plus independent scheduled sensor outage | MCAR and exogenous outage have separate causal labels; missing values provide no likelihood |
| HOBS-03 | MAR activity-linked motion artefact | Sampling/quality metadata must be separated from latent physiology |
| HOBS-04 | MNAR state-dependent dropout | Clean-data calibration must not be assumed to survive |
| HOBS-05 | Sensor bias and slow calibration drift | Detect or expose systematic residual; do not absorb it silently into physiology |
| HOBS-06 | Quantisation, clipping and saturation | Censored/range-limited readings must not be treated as exact values |
| HOBS-07 | Clock offset and drift | Clock error must not appear as biological phase shift |
| HOBS-08 | Delayed delivery and stale observations | Apply the information cutoff; record age at use |
| HOBS-09 | Duplicated, missing and out-of-order packets | No double update and explicit late-data disposition |
| HOBS-10 | Common-cause outage / shared calibration failure | Correlated sensor errors must not be treated as independent confirmation |
| HOBS-11 | Observationally indistinguishable latent worlds | Estimator must report non-identifiability or abstain |

Extend later with modality conflict, device/placement transfer, policy-dependent active sampling, intervention-delivery mismatch and regime change. Keep each extension as a separately versioned scenario family.

### Indistinguishable-world test

Construct two different latent histories that produce the same admissible observation stream under the declared sensor model. No estimator can uniquely recover which history occurred from those observations alone. A calibrated system should return a set, interval, ambiguity flag or abstention consistent with the information available. A confident point estimate is a failure even if it happens to match one hidden world.

## 6. Baselines and scorecard

Run all candidate estimators on identical event histories, train/test partitions, calibration opportunities, modalities and compute budgets.

### Baseline ladder

1. Full-observation oracle (upper bound only; not a deployable comparator).
2. Persistence / last observation.
3. Linear regression or autoregression, plus a transparent mixed-effects baseline where repeated subjects support it.
4. Kalman/state-space methods when their assumptions match the simulated process.
5. Missingness-aware probabilistic state-space filtering.
6. Irregular-time methods such as ODE-RNN and Neural CDE, using their correct event-time semantics.
7. Symthaea variants with predeclared ablations: no HDC, no temporal memory, no active sensing, no physiological coupling, and full candidate.
8. Model portfolio / disagreement baseline with calibrated abstention.

The interpolation/forward-fill baseline may be included as a deliberately weak diagnostic; it must not be the only classical baseline.

### Sampling-frequency invariance and equivariance (new 2026 comparison lane)

Sampling-frequency robustness is now an explicit SOTA dimension, not a generic missing-data stress. A November 10, 2026 preprint evaluates MOMENT-1-large, Chronos-2, Moirai-MoE and TimesFM under anti-aliased bandwidth reduction with fixed input-token length. It proposes three complementary measures: embedding Frequency Sensitivity Index (FSI), Forecast Degradation Index (FDI), and Classification Degradation Index (CDI). The authors report that embeddings can drift before task performance degrades, and that downstream degradation can exceed the loss attributable to removed signal information. This is an arXiv preprint, not yet a settled community standard; reproduce its protocol and label our additions as independent metrics. Source: https://arxiv.org/abs/2611.14425.

The direct model portfolio must include:
- **FlowState (ICML 2026):** sampling-rate-equivariant forecasting using a state-space encoder and functional-basis decoder. This is the strongest direct architectural comparator for variable-rate forecasting; include it rather than assuming every transformer is the SOTA. Source: https://proceedings.mlr.press/v306/graf26a.html.
- **MOMENT, Chronos-2, Moirai-MoE, TimesFM:** general time-series foundation-model baselines, with frozen release/checkpoint and software version.
- **TiRex / TiRex-2:** xLSTM-based zero-shot forecasting baseline, where the multivariate/covariate interface matches the task. Source: https://nx-ai.github.io/tirex/.
- **NormWear:** a wearable-specific multimodal representation baseline reported across 11 public datasets and 18 applications; compare only for supported sensor modalities and identical data splits. Source: https://doi.org/10.1145/3803808.
- **ODE-RNN / Neural CDE / continuous-time state-space baselines:** use true timestamps and irregular event semantics, not a resampled approximation unless resampling is the explicit test condition.

Use two distinct perturbation tracks:
1. **Rate change with preserved task bandwidth:** anti-alias before decimation, keep elapsed context and prediction horizon fixed in physical time, and test downsampling while the task-critical frequency band remains observable.
2. **Rate change that removes task-critical bandwidth:** continue below the information-theoretic limit; do not penalize a model for irrecoverable signal loss. Compare its degradation to a declared information-loss oracle/upper bound.

For each track report:
- embedding drift (cosine distance or a frozen representation metric);
- normalized forecast error at identical physical horizons;
- downstream macro-F1 / balanced accuracy and calibration;
- retained spectral information / task-band energy;
- sensor rate, context duration, number of tokens, and compute;
- recovery after returning to the original rate.

For the paper-faithful TSFM comparison, reproduce the fixed-token, anti-aliased bandwidth-reduction design and FSI/FDI/CDI definitions from the preprint. Separately add an event-native challenge with genuinely irregular capture times and explicit timestamps. Do not conflate bandwidth reduction, random dropout, irregular sampling and changed context duration: they are different perturbations.

### Cross-device and cross-dataset activity-recognition SOTA

AICOS-HAR's October 2026 data descriptor highlights fragmentation across protocols, devices, placements and labels, and releases 21 parsers for integrating public HAR datasets. Its own data include 106 participants, 87 smartphones and 17 wearable devices. This enables participant-held-out, leave-one-device-out and cross-dataset tests, but the AICOS-HAR dataset itself is listed as CC BY-NC-ND 4.0 in the NLM Dataset Catalog: do not assume commercial use or redistribution of derivatives is permitted. Sources: https://www.nature.com/articles/s41597-026-08470-y and https://datasetcatalog.nlm.nih.gov/dataset?q=0003181855.

A November 2026 IEEE Transactions on Mobile Computing paper, *Learning From Decoupled Sensor Axes*, reports a decouple-and-ensemble adaptation method evaluated across 20 source-target dataset pairs, with best accuracy in 18 scenarios and reported improvements of 9.15–10.63% over the compared SOTA methods across four source datasets. Treat this as a published comparator to reproduce—not as a transferable guaranteed gain. Match label sets, axes, placement, device domains, adaptation data access and target-label budget. Source: https://doi.org/10.1109/TMC.2026.3723993.

For our own cross-device lane:
- use participant-, device-, placement- and source-dataset-held-out partitions;
- include source-only, unsupervised adaptation, few-shot adaptation and fully supervised target-domain ceilings as separate tracks;
- compare joint-axis encoders with decoupled-axis and confidence-weighted ensemble baselines;
- report balanced accuracy, macro-F1, per-class calibration, worst-domain and lower-quantile performance, abstention, and adaptation-data/compute budgets;
- keep activity recognition as a sensor-transfer task, not evidence of physiological or cognitive fidelity.

### Metrics (report independently)

| Capability | Metrics |
|---|---|
| State prediction | MAE/RMSE in source units; horizon-specific error; event/phase timing error |
| Probabilistic forecasts | CRPS or proper log score; 50/80/90/95% interval coverage and width |
| Calibration | Reliability curves, Brier/log score for classes, calibration error and subgroup/domain coverage |
| Parameter recovery | Bias, variance, coverage, profile likelihood or equivalent practical-identifiability diagnostics |
| Missingness robustness | Error and calibration by MCAR/MAR/MNAR regime; false posterior movement when no measurement arrives |
| Time semantics | Physical-time error, capture/availability age, stale-event handling, duplicate suppression |
| Sensor robustness | Error under drift, saturation, clock error, placement and common-cause failure |
| Selective prediction | Coverage-risk curve, selective risk, abstention correctness on indistinguishable worlds |
| Active sensing | Decision quality, value of information, sample count, sensing burden and total compute cost |
| Runtime | p50/p95/p99 end-to-end latency, peak memory and energy-to-compute where measurable |

Do not use a composite "human realism" score. Do not trade away hard invariants or calibrated uncertainty for a small gain in average accuracy.

## 7. Data and benchmark crosswalk

### Public data should test one declared capability at a time

- **MMASH / PhysioNet:** 22 healthy young adult men with 24-hour heart-rate and activity measurements, sleep/activity logs and selected biomarker observations. Useful for narrow observational time-series checks. Too small and narrowly sampled to establish broad population validity or intervention effects.
- **Sleep-EDF Expanded / PhysioNet:** overnight polysomnography and expert sleep labels. Useful for sleep-state measurement and signal baselines; not a whole-day physiology or cognitive-behaviour benchmark.
- **AICOS-HAR (2026):** 5,231 acquisitions (101.4 hours) across 106 participants, 87 smartphones and 17 wearable devices. Useful for device/placement transfer in activity recognition. The Zenodo record lists CC BY-NC-ND 4.0; treat as non-commercial and no-derivatives unless the rights holder clarifies otherwise. It does not become a human physiology dataset merely because it contains wearable sensors.
- **WESAD:** a small laboratory baseline/stress/amusement protocol. Its provider states scientific non-commercial use; exclude from commercial workflows unless rights are resolved.
- **BEHAVIOR-1K:** an embodied household-task benchmark with 1,000 activities and interactive scenes. Useful as a physical task/environment reference for robot or synthetic-actor interaction, not as evidence that a synthetic agent reproduces human cognition or physiology.

Keep public datasets in separate source-specific evaluation tracks. Pin versions, checksums, rights and preprocessing; split by participant and, where supported, device/site. Keep raw records and non-redistributable derivatives out of Git.

### Published numbers are not automatically a leaderboard

A recent 2026 Sleep-EDF sleep-staging paper reports 89.72% accuracy / 85.46% macro-F1 on its SleepEDF-20 setup and 86.41% / 81.97% on SleepEDF-78. Treat these as paper-reported reference points only. They are not a direct Symtropy/Symthaea comparison until channel, epoching, label mapping, subject split, preprocessing, tuning budget and test protocol are reproduced exactly.

## 8. Circadian model portfolio: preserve a baseline, add a dynamic comparator

The dedicated source-audit and implementation track is [HUMAN-CIRCADIAN-LIGHT-001 / #5666](https://github.com/Luminous-Dynamics/symtropy/issues/5666).

The existing fixed-period five-harmonic Process C is useful as a transparent periodic baseline. It does not model light entrainment.

A separate next candidate is the human circadian pacemaker family that separates a dynamic light-stimulus processor (Process L) from a self-sustaining pacemaker oscillator (Process P). Kronauer, Forger and Jewett (1999) describe this structure and its response to brief, extended and repeated light exposures. Their paper reports Process-L timescales and also explicitly notes that further experimental data were needed to refine predictions at lower light intensities and brief stimuli.

Implementation rule:

- retain the current fixed-period waveform as a versioned baseline;
- implement the published dynamic candidate as a separate adapter/version only after equation-level transcription and parameter provenance review;
- keep photopic illuminance, spectral/retinal assumptions, exposure timing, adaptation and clock basis explicit;
- test phase response, entrainment, transient response and sensitivity against the source's declared domain;
- compare predictions with the baseline and a pinned independent implementation where available;
- do not interpret the model's output as an individual's melatonin level, sleepiness, alertness or safe-to-work status.

Do not simply add lux as a scalar offset to the current Fourier waveform and call that circadian entrainment.

## 9. Validation stages and go/no-go gates

| Stage | Evidence produced | What it does not establish |
|---|---|---|
| V0 — source/equation audit | Pinned equations, units, parameter provenance and validity domain | Correct implementation |
| V1 — code verification | Analytic, property, boundary and deterministic replay tests | Human validity |
| V2 — synthetic known-truth qualification | Recovery, leakage, missingness and identifiability results in declared worlds | Real sensor or population validity |
| V3 — public observational evaluation | Held-out participant/device/task performance and calibration | Causal intervention effects |
| V4 — external/temporal transfer | Locked evaluation on another source, device, period or protocol | Individual prospective utility |
| V5 — prospective silent evaluation | Prospective prediction against independently observed events without influencing action | Clinical effectiveness |
| V6 — impact evaluation | Decision and outcome benefit in an appropriately governed study | Generalization beyond that context |

Go forward only when a stage's preregistered acceptance criteria pass. A model can be useful at V2 or V3 without being called a digital twin. Downgrade the claim or abstain when data do not identify the parameter, uncertainty is miscalibrated, external transfer fails, or a requested intervention lies outside the validated domain.

## 10. Industry and state-of-the-art comparison

The relevant competitors are not one product category:

- **Wearables / consumer platforms:** Apple, Fitbit, Garmin, Oura and WHOOP provide proprietary sensor fusion, sleep/activity estimates and large-scale product experience. Their internal labels, model versions, training sets and calibration policies are generally not exposed in a way that supports a reproducible apples-to-apples scientific comparison. Compare only where independent ground truth and matched inputs are available.
- **Sleep staging research:** compare with reproduced EEG/PSG baselines on fixed subject-held-out Sleep-EDF and external-dataset tests. Do not compare an overnight EEG classifier score with a human-day simulator.
- **Physiology engines:** BioGears and individual CellML/Physiome models are reference implementations for selected mechanisms, not proof of an integrated human twin. OpenSim is a musculoskeletal comparator only when the task requires muscle/joint dynamics.
- **Embodied activity / task simulation:** BEHAVIOR-1K is a strong environmental-task reference, but it benchmarks embodied task capability, not the realism of human internal states.

### Published wearable comparisons: a defensible industry baseline

A 2025 PSG study evaluated six commercial wrist-worn devices (Fitbit Charge 5/Sense, Withings Scanwatch, Garmin Vivosmart 4, Whoop 4.0 and Apple Watch Series 8) in 62 adults. Reported sleep/wake sensitivity exceeded 90%, but specificity ranged from 29.39% to 52.15%, with Cohen's kappa from 0.21 to 0.53. The study also reports stage-specific misclassification and parameter-dependent bias. This is a useful warning against reporting sensitivity or overall accuracy alone: a device can identify most sleep epochs while still confusing wake and particular sleep stages. Source: https://doi.org/10.1093/sleepadvances/zpaf021.

A 2026 home comparison of Apple Watch Series 7, Fitbit Charge 5 and Polar Vantage M2 against home PSG and seven-day actigraphy reported generally weak agreement with wide limits of agreement, with device-specific stage-duration biases. The authors framed these as suitable for longitudinal self-monitoring rather than clinical-grade sleep-architecture assessment. Source: https://doi.org/10.1016/j.sleep.2026.109059.

These studies are not universal rankings: sample sizes, firmware, populations, algorithms and study protocols matter. Use them to define an independent PSG comparison lane, not to claim that a synthetic model beats a commercial product. For a credible direct comparison, test the same cohort, same PSG labels, same night boundaries and equivalent sensor inputs; report per-stage confusion, sensitivity, specificity, macro-F1, kappa, event timing, calibration and limits of agreement. A Sleep-EDF EEG model and a wrist-wearable PPG/accelerometry system are different input tasks and must be reported as separate tracks.

The differentiation should be **evidence-aware compositional simulation**: explicit latent truth, explicit observation generation, causal/time semantics, model disagreement, calibrated abstention, multi-fidelity escalation, reproducible receipts and claim-specific validation. The ambition is not to beat every specialist engine at its own task. It is to outperform monolithic or opaque approaches on cross-subsystem accountability, robustness to observation failure, and the ability to say when the evidence cannot support a prediction.

## 11. First implementation increments

1. Freeze 12 scenario configs and hidden-truth oracles (synthetic only).
2. Implement deterministic sampling, noise, missingness and transport in the isolated human-systems Rust crate or an isolated child crate.
3. Keep the generator's truth API inaccessible to estimator code; test leakage by mutating hidden truth while keeping observations fixed.
4. Add an observation adapter only after Symthaea #12407 publishes/agrees the shared event contract.
5. Add matched baseline evaluators and machine-readable receipts.
6. Run participant/device-held-out public-data adapters locally where licences permit.
7. Integrate a separate dynamic circadian light model after equation and source audit.

No CI pass is implied by this protocol. Software test evidence must be tied to an exact source revision and exact-head run.

## References

- Tudor et al. (2025), *A scoping review of human digital twins in healthcare applications and usage patterns*, npj Digital Medicine. DOI: https://doi.org/10.1038/s41746-025-01910-w. The review found 18 of 149 included studies met its definition of a healthcare digital twin; only two mentioned VVUQ.
- Kronauer, Forger & Jewett (1999), *Quantifying human circadian pacemaker response to brief, extended, and repeated light stimuli over the photopic range*, Journal of Biological Rhythms. DOI: https://doi.org/10.1177/074873099129001073.
- Forger, Jewett & Kronauer (1999), *A simpler model of the human circadian pacemaker*. DOI: https://doi.org/10.1177/074873099129000867.
- AICOS-HAR (2026), Zenodo record 19452049. DOI: https://doi.org/10.5281/zenodo.19452049. Rights: CC BY-NC-ND 4.0 as listed in the NLM Dataset Catalog.
- BEHAVIOR-1K official benchmark: https://behavior.stanford.edu/.
- TempoSleep (2026) Sleep-EDF results cited as a paper-specific comparison point: https://pmc.ncbi.nlm.nih.gov/articles/PMC13626375/.
- Sleep-EDF Expanded: https://physionet.org/content/sleep-edfx/1.0.0/.
- MMASH: https://physionet.org/content/mmash/1.0.0/.
- WESAD: https://ubi29.informatik.uni-siegen.de/usi/data_wesad.html.
