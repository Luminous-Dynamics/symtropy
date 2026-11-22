# Observation-Surface Identifiability Benchmark v0.3

**Status:** research specification plus initial synthetic detector-kernel implementation in development.  
**Owner:** [#6752 — marked, stateful, multi-sensor observation identifiability](https://github.com/Luminous-Dynamics/symtropy/issues/6752).  
**Builds on:** #6722, #6659, #6715, #6731 and #6751.  
**Claim ceiling:** deterministic/synthetic model behavior only; no physical POD, reliability, qualification or mission-authority claim.

## 1. Why v0.3 is a model-class change

v0.2 formalizes one useful but restricted model:

\[
p(x)=p_0x^\gamma,\qquad x=t/T,
\]

with independent Bernoulli thinning conditional on event time. That model makes the exact confounding visible, and its two-anchor confidence bounds are useful only when the event class, configuration, non-time covariates and trial design really match.

It does not describe detectors whose probability of admitting an event depends on event morphology, event rate, refractory state, buffer occupancy, false alarms, configuration changes or another sensor's failure state. Those effects are not small corrections to a scalar POD curve. They change the observation process itself.

v0.3 therefore treats an observation system as a causal process with distinct latent-event, detector-state, channel, reference and false-positive mechanisms.

## 2. Generative model family

Let the latent physical process be a marked point process:

\[
N^*(dt,dz),\qquad
\lambda_\theta(t,z\mid H_t^*),
\]

where marks z can include amplitude, duration, waveform, polarity, morphology, location, energy, operating mode and environmental conditions.

For sensor/channel c, define:

\[
p_c(t,z,H_{c,t},q_c)
=
\Pr(\text{event admitted by channel }c\mid t,z,H_{c,t},q_c),
\]

where H(c,t) carries detector history and state (dead time, pile-up, queue occupancy, saturation, threshold adaptation, recovery), and q(c) is a versioned hardware/firmware/acquisition configuration.

The reported stream also has a distinct false-positive process:

\[
F_c(dt,dz),\qquad
\lambda_{FP,c}(t,z\mid H_{c,t}).
\]

Cross-channel misses can share a latent common-cause state G(t) (power, timing, mount, environment, shared firmware, reference path). Distinct channel IDs do not imply conditional independence. Event association has its own uncertainty: timing jitter, duplicate detections, ambiguous matches and unpaired reports.

The first executable v0.3 kernel implements a deliberately restricted subset: mark-dependent logistic detection, fixed non-paralyzable dead time, declared common-cause outage intervals, channel-specific outage intervals and a separate Poisson false-positive process. This kernel is a simulator and counterfactual test fixture, not an estimator of physical truth.

## 3. Causal structure

~~~mermaid
flowchart LR
  P[Latent physical event process] --> Z[Event marks and history]
  Z --> D1[Sensor 1 detector state]
  Z --> D2[Sensor 2 detector state]
  G[Shared common-cause state] --> D1
  G --> D2
  Q1[Sensor 1 configuration] --> D1
  Q2[Sensor 2 configuration] --> D2
  D1 --> O1[Observed channel 1 stream]
  D2 --> O2[Observed channel 2 stream]
  FP1[False-positive process 1] --> O1
  FP2[False-positive process 2] --> O2
  R[Independent reference/metrology] --> E[External calibration evidence]
  E --> A[Identifiability and decision gate]
  O1 --> A
  O2 --> A
~~~

The reference path is not presumed independent merely because it is called a reference. Its own calibration, common-cause dependencies and measurement uncertainty must be evidenced.

## 4. Identifiability before estimation

The key output should not always be a point estimate. For an observation model M, define the observational equivalence set:

\[
\mathcal E(\mathcal O)
=
\{(\theta,\psi): P_{\theta,\psi}(\mathcal O)
\text{ is compatible with the observed evidence}\},
\]

where theta parameterizes physical events and psi parameterizes detection, false positives and sensor state.

Report the projection onto physical parameters:

\[
\Theta_{phys}(\mathcal O)
=
\{\theta:\exists\psi,(\theta,\psi)\in\mathcal E(\mathcal O)\}.
\]

Only report a point estimate when the relevant projection is sufficiently identified under a justified model and the uncertainty set is decision-compatible. If several physically distinct explanations remain observationally equivalent, return a set or an abstention status. A local Fisher-information rank check is not a proof of global identifiability.

The first kernel does not compute E(O). It creates reproducible counterexamples and controlled ground truth for future inference methods.

## 5. Comparator tournament

Every method receives the same synthetic latent events, marks, sensor states, independent seeds, calibration budget and held-out configurations. Required comparators:

| ID | Method | Purpose / failure mode to expose |
|---|---|---|
| A | Timestamp-only PLP MLE and GOF | Baseline that sees only captured events |
| B | v0.2 two-anchor POD bounds | Calibrated scalar time-only reference |
| C1 | Inverse-probability weighting with oracle POD | Best-case correction when the observation surface is known |
| C2 | Inverse-probability weighting with estimated POD | Propagates calibration error and low-POD variance |
| D | Joint latent-process / observation-process likelihood | Tests whether model coupling recovers the physical process |
| E | Marked hierarchical/partial-pooling POD surface | Tests calibration transfer across repeated strata and time |
| F | Multi-sensor capture-recapture / multiple-systems model | Tests dependence assumptions and common-cause sensitivity |
| G | Dead-time / refractory-state point-process model | Tests history-dependent missed-event mechanism |
| H | Selective set-valued / abstaining policy | Tests whether unsupported physical claims are prevented |

No method is called SOTA merely because it is present. Use published baselines or independently implemented comparators, matched experimental conditions and held-out data.

## 6. Frozen scenario programme

The machine-readable scenario matrix is tools/orbital-observation-surface-bench/fixtures/v0.1/scenario-matrix.json. It defines the first kernel scenarios and the broader tournament grid:

- constant and smoothly drifting capture;
- step changes, sensor replacement and firmware/configuration change;
- mark dependence by amplitude, duration, waveform and polarity;
- event-rate dependence, fixed/random dead time, pile-up and queue saturation;
- false positives, false negatives and ambiguous event association;
- clustered/self-exciting latent processes;
- correlated channel failure and shared reference-path failure;
- low POD, sparse strata, calibration drift and covariate shift;
- deliberately misspecified model families.

The first synthetic kernel currently covers mark-dependent POD, fixed dead time, common-cause outages, channel outages and false-positive arrivals. Remaining cases are explicit future gates, not represented as implemented.

## 7. Metrics and preregistered targets

Report per-stratum and aggregate results with uncertainty:

1. 90%/95% POD and physical-parameter set coverage, width and calibration.
2. Family-wise coverage across channels/strata.
3. False physical-trend attribution when only the detector changes.
4. False-positive and false-negative decision rates.
5. Correct abstention under structural non-identifiability and misspecification; unnecessary abstention when identified.
6. Independent reference trials and calibration cost needed to cross an engineering decision boundary.
7. Runtime, peak memory, sample efficiency and byte-reproducible receipts.

Research targets (not achieved results):
- Nominal 95% uncertainty sets should achieve coverage consistent with 95% within predeclared Monte Carlo uncertainty on every in-scope stratum.
- Under detector-only drift with a structurally equivalent physical explanation, target a one-sided 95% upper confidence bound below 1% for false physical-trend commits.
- No silent inverse-probability correction under misspecified mark/rate/history dependence; return a sensitivity set or abstain.
- At equal decision error/confidence, target at least 25% fewer independent reference trials than a uniform calibration grid on held-out configurations.
- Synthetic, replay-only or unauthenticated evidence must never grant qualification, physical reliability or mission authority.

Use exact binomial bounds for rare false commits and coverage failures. Choose Monte Carlo replication counts before seeing results; do not interpret zero observed failures as zero risk.

## 8. Active calibration as decision optimization

Do not maximize entropy or global parameter information by default. Let a denote a candidate physical stimulus/configuration and C(a) its cost. Select the next experiment to shrink the uncertainty set where it changes the engineering decision:

\[
a^*
=
\arg\min_a
\left[
\mathbb E\{L(\Theta_{phys}\mid Y_a)\}
+\lambda C(a)
\right].
\]

The loss L should penalize incorrect engineering decisions, unsupported point claims and needless abstention differently. Optimize on training strata and score on preregistered held-out event classes/configurations. Preserve a random exploration allocation so the policy does not blind itself to unanticipated failure modes.

## 9. Industry and research boundary

- **NIST TN 2119** provides statistical recipes for POD, probability of identification and false-alarm estimates, including confidence intervals/bounds from binary data: https://doi.org/10.6028/NIST.TN.2119
- **ASTM E2862-23** defines hit/miss POD analysis for a specified NDE examination configuration, with validation and interpretation. Its scope is not a generic model of every sensor/event stream: https://doi.org/10.1520/E2862-23
- **NASA-STD-5009C** addresses NDE requirements where quantitative POD is required for fracture-critical metallic components: https://standards.nasa.gov/node/249
- Mottier et al. (2026) study temporal replication and partial pooling for thinned point-process practical identifiability; the paper reports persistent difficulty when detection is low: https://arxiv.org/abs/2610.29365
- A 2024 Biometrics treatment emphasizes that multiple-systems/capture-recapture estimation depends on an identifying assumption; where it cannot be justified, a population estimate should not be produced: https://doi.org/10.1093/biomtc/ujad028
- Dead-time point-process models are necessary when detection suppresses subsequent events; Peterson (2021) derives interval-distribution calculations for inhomogeneous point processes with random dead time: https://doi.org/10.1007/s00422-021-00868-8

The aim is to outperform by **coverage under misspecification, lower decision error, auditable abstention and lower calibration cost at equal assurance**, not by cherry-picked RMSE or an unmatched synthetic leaderboard.

## 10. Evidence ceiling

This specification and initial simulator are synthetic research artifacts. The simulator has no calibration authority, it cannot authenticate independence or reference metrology, and it does not establish any real sensor's probability of detection. Electronic injection qualifies only downstream acquisition behavior; an end-to-end claim requires a physical stimulus at the actual sensor/mount and an independently qualified reference measurement.