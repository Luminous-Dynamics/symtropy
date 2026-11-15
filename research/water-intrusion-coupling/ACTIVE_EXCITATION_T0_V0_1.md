# Sensor-Aware Active Excitation Design T0 v0.1

**Programme:** WATER-MIXING-ACTIVE-EXPERIMENT-DESIGN-001 / [#5309](https://github.com/Luminous-Dynamics/symtropy/issues/5309)  
**Parent experiment selection:** [#5259](https://github.com/Luminous-Dynamics/symtropy/issues/5259), [#5271](https://github.com/Luminous-Dynamics/symtropy/pull/5271)  
**Robust information design:** [#5226](https://github.com/Luminous-Dynamics/symtropy/issues/5226), [#5239](https://github.com/Luminous-Dynamics/symtropy/pull/5239)  
**Transient forward operator:** [#5274](https://github.com/Luminous-Dynamics/symtropy/issues/5274), [#5288](https://github.com/Luminous-Dynamics/symtropy/pull/5288)  
**Constrained identification:** [#5298](https://github.com/Luminous-Dynamics/symtropy/issues/5298), [#5306](https://github.com/Luminous-Dynamics/symtropy/pull/5306)  
**Evidence class:** synthetic actuator/sensor-filtered excitation and identifiability arithmetic  
**Evidence ceiling:** no physical mixing qualification

## 1. Why this increment is distinct

The existing design optimizer accepts supplied sensitivity matrices and compares information, model separation and cost. The exact subset search chooses finite experiment/sensor blocks. This increment addresses the missing forward bridge:

1. candidate inlet commands are propagated through a calibrated injector/actuator response;
2. the resulting predicted delivered inlet mass-flux histories drive the outlet-resolved dynamic kernels;
3. parameter-direction kernels are propagated through the physical operator;
4. each outlet response is filtered through its own calibrated sensor impulse response;
5. the complete filtered sensitivity matrix is evaluated under the declared full measurement covariance;
6. nuisance information is eliminated by a Schur complement, and competing physical-model kernels are compared after the same injector/sensor operators;
7. candidates violating flow, dose, slew, sampling, bandwidth or boundary constraints are refused before they can be selected.

This is not a reimplementation of D-optimality or a command-waveform optimizer that assumes the actuator follows its set point exactly. It scores the experiment as the measurement chain will see it.

## 2. Mathematical contract

For command sequence u_c, calibrated injector response H_a, kernel K(theta), and calibrated sensor response H_s:

\[
u_m = H_a * u_c,\qquad
\mu(\theta,u_c)=H_s * (K(\theta)*u_m).
\]

The local parameter Jacobian is generated from the parameter-direction kernels after those same convolutions:

\[
J_m = H_s * (K_\theta * u_m),\qquad
F=J_m^T\Sigma^{-1}J_m.
\]

The covariance Sigma is full, symmetric and required to be positive definite at the declared numerical tolerance. The evaluator uses Cholesky solves; it does not diagonalize correlated samples or assume equal sensor variance.

Target parameters are separated from nuisance parameters. When the nuisance information block is estimable, target information is the Schur complement:

\[
F_{t\mid n}=F_{tt}-F_{tn}F_{nn}^{-1}F_{nt}.
\]

If the nuisance block is singular, the candidate is NUISANCE_UNRESOLVED. If the target information is rank-deficient after nuisance adjustment, the design is UNIDENTIFIABLE_DESIGN. Full rank is not enough: the minimum singular value and condition ceiling must also pass.

Competing models are propagated through the same measured-input prediction and sensor filters. Pairwise separation is the noise-whitened Mahalanobis distance:

\[
d_{ab}=\sqrt{(\mu_a-\mu_b)^T\Sigma^{-1}(\mu_a-\mu_b)}.
\]

This is a deterministic separation proxy, not expected information gain or a posterior probability.

## 3. Candidate selection

The T0 compares candidates on a Pareto frontier over:
- nuisance-adjusted minimum singular value;
- minimum pairwise model separation;
- delivered tracer mass (dose proxy);
- total duration.

The selected design is the eligible Pareto candidate with the largest minimum singular value; ties are broken by model separation and then lower injected mass. This selection policy is explicit and deterministic. A production design campaign should also include sensor count, calibration burden, cost, energy/flow disturbance, repeatability and multi-scenario worst cases in the Pareto dimensions.

A large model-separation score cannot rescue a rank-deficient design. Likewise, excellent parameter information cannot rescue a candidate that cannot discriminate the pre-registered competing models.

## 4. Frozen nine-candidate controls

1. **Orthogonal inlet pulses:** independently time-coded inlet excitation; eligible after actuator and sensor filtering.
2. **Synchronous identical inlet pulses:** same two models may be well separated, but inlet sensitivities are collinear; must be UNIDENTIFIABLE_DESIGN.
3. **Above-sensor-bandwidth excitation:** feature period is faster than calibrated sensor resolution; refused.
4. **Partial outlet boundary:** missing outlet channel; refused before information scoring.
5. **Uncalibrated injector:** command cannot stand in for delivered inlet mass flux; refused.
6. **Excessive dose and flow rate:** candidate is rejected by physical/operational limits before information scoring.
7. **Low-dose orthogonal excitation:** full rank but minimum singular value below the information threshold; refused as insufficient information.
8. **Command-only waveform:** no calibrated actuator transfer; refused.
9. **Truncated temporal boundary:** missing physical/sensor tail; refused.

The frozen covariance includes temporal correlation. The injector and sensor impulse responses are both nontrivial, so the oracle is not an unfiltered design-matrix calculation.

## 5. Numerical mirror result

The separate JavaScript numerical mirror for the frozen synthetic input reports:

| Candidate | Result | Nuisance-adjusted rank | Minimum singular value | Model separation |
|---|---|---:|---:|---:|
| Orthogonal pulses | Eligible | 2 | 2.5971 | 2.9714 |
| Synchronous pulses | Unidentifiable | 1 | 0 | 6.1519 |
| Low-dose orthogonal | Insufficient information | 2 | 0.1623 | 0.1857 |

All other candidates are refused before scoring for the exact reason recorded in the oracle. The selected fixture candidate is AE-T0-001-orthogonal-pulses.

These are synthetic calculations only. They do not establish achievable performance on a water rig.

## 6. Evidence and provenance

The input records the calibrated actuator and sensor response kernels and full measurement covariance. Candidate provenance must be command_plus_calibrated_actuator_model; command-only candidates cannot be scored. The evaluator reports the actuator-filtered inlet mass-flux history and checks its delivered mass, maximum rate and slew.

For a physical campaign, actuator-predicted inlet histories must be replaced by independently measured inlet mass-flux histories before fitting or qualification. A calibration file, calibration revision, clock map, sensor response, sample-line transfer, geometry, control volume, raw traces and run identity must then be bound to the experiment receipt. T0 does not claim that those physical records exist.

## 7. Comparators and benchmark targets

### Current field comparators

- EPANET complete junction mixing and standard tank mixing options are the low-cost network baseline.
- Geometry-aware CFD/regression methods provide a current incomplete-junction comparator; their operating envelope and generalization must be tested rather than assumed.
- CNN-based water-network sensor placement often optimizes contamination-source classification accuracy; this T0 instead asks whether the actual transient parameters and competing junction models are observable through the actual sensor chain.
- Existing D-, A-, E-optimal and robust design methods remain comparators. We should not claim novelty for Fisher information, Schur complements, sensor filtering or Pareto optimization individually.

### Proposed targets (not achieved)

- 100% frozen-oracle agreement and 0 false-identifiable outcomes on the critical negative controls.
- Correct refusal for rank-deficient excitation even when model separation is high.
- Zero information credit for excitation beyond calibrated sensor bandwidth.
- At equal delivered dose, duration and sensor set, beat random, greedy, D-optimal-only and fixed pulse/step baselines on minimum singular value and model separation.
- Demonstrate robust ranking across declared geometry, flow-ratio, temperature, injector and sensor-response strata.
- Report condition-number sensitivity to the rank tolerance; publish raw and nuisance-adjusted spectra.
- For later blind physical T1 data, evaluate interval coverage, prediction error, abstention rate and uncertainty width by geometry and flow stratum; preregister 90% interval coverage as a target, not a result.
- Publish information per delivered tracer mass, per minute, per sensor and per calibration effort; never optimize those denominators independently of safety constraints.

## 8. Known limitations / next increments

- Injector response is currently a shared, calibrated, nonnegative unit-area FIR per inlet. Real actuator uncertainty and calibration uncertainty are not propagated yet.
- The current T0 uses one output sensor channel and one fixed covariance/time grid in the frozen corpus. Extend to multiple outlet sensors, sensor dropout and sensor-specific calibration uncertainty.
- Parameter directions are local derivatives at a reference kernel. Nonlinear parameter dependence, robust Bayesian design over broad priors and expected information gain are not claimed.
- Candidate selection has a transparent lexicographic rule over a Pareto set; it is not a global mixed-integer optimal-control proof.
- No experiment can be declared safe from this synthetic evaluator. Facility approval, chemical compatibility, non-potable closed-loop rig controls and disposal approval are prerequisites for T1.

## 9. Safety / evidence boundary

Do not inject tracer into a live drinking-water distribution system under this research item. Use a facility-approved closed-loop non-potable rig, an approved tracer, competent operator review and pre-approved handling/disposal. A missing sensor, missing outlet, uncalibrated response, unresolved clock or truncated tail causes abstention; missing mass is never normalized away.

No T0 output qualifies a physical junction, tracer, sensor, chemical, drinking-water network, contamination-warning system, potability, toxicology, SANS 241 compliance or release authority.
