# Sensor-Aware Active Junction Experiment Design T0 v0.1

**Programme:** WATER-MIXING-ACTIVE-EXPERIMENT-DESIGN-001 / [#5309](https://github.com/Luminous-Dynamics/symtropy/issues/5309)  
**Finite experiment search:** [#5259](https://github.com/Luminous-Dynamics/symtropy/issues/5259), [#5271](https://github.com/Luminous-Dynamics/symtropy/pull/5271)  
**Transient operator:** [#5274](https://github.com/Luminous-Dynamics/symtropy/issues/5274)  
**Constrained identification:** [#5298](https://github.com/Luminous-Dynamics/symtropy/issues/5298)  
**Evidence class:** synthetic sensor-aware Fisher-information design arithmetic  
**Evidence ceiling:** no physical experiment has been performed or authorized

## 1. Objective

Select among candidate inlet excitation sequences using the Fisher information of the parameter sensitivities **after** the calibrated instrument response and measurement covariance have been applied.

For a parameterized transient model with predicted sensor observations mu(theta,u), measured input waveform u and covariance Sigma:

\[
J_w = \Sigma^{-1/2}\frac{\partial \mu(\theta,u)}{\partial\theta},
\quad
F(\theta,u)=J_w^T J_w
\]

The information matrix is conditional on the supplied model, waveform and sensor model. It is not a certificate that the model is physically correct.

## 2. T0 selection rule

The evaluator computes F from a supplied sensor-filtered sensitivity matrix and diagonal measurement variance. It reports the eigenvalue/singular-value spectrum, effective rank, condition number, Fisher log-determinant, trace, dose, duration and information per cost.

Among candidates that pass every physical and measurement preflight, it selects:
1. maximum Fisher log-determinant (D-optimality);
2. minimum total dose plus duration as a deterministic tie-break.

This v0.1 is a single-candidate selector, not yet a combinatorial multi-run design optimizer. Its covariance contract is independent diagonal variance only. Correlated outlet/time noise requires a validated covariance factor and a whitening transform in a later increment.

## 3. Non-negotiable preflight

A candidate is not scored if:
- dose or duration exceeds declared design limits;
- any outlet is unobserved;
- the sensor response is uncalibrated or has not been applied to the sensitivities;
- the sensor cannot resolve the minimum delay of interest;
- a claimed temporal resolution is finer than the sample interval;
- the experiment crosses a hydraulic regime transition when fitting a fixed-regime operator;
- flow reversal exists without signed direction observation;
- the candidate is outside the declared operating domain;
- the Fisher matrix is rank deficient or exceeds the condition-number ceiling;
- observation variance is nonpositive, missing or dimensionally inconsistent.

A high nominal information score does not override any of these gates. Rank-deficient designs return UNIDENTIFIABLE_DESIGN; there is no "regularize until identifiable" path.

## 4. Frozen synthetic cases

The eight cases establish the following controls:

1. **Orthogonal versus collinear excitation:** a low-dose orthogonal inlet sequence is selected; a more dose-intensive collinear sequence has Fisher rank 1 and is rejected.
2. **Rank-deficient only:** identical or repeated collinear sequences produce UNIDENTIFIABLE_DESIGN and no selected waveform.
3. **Sensor bandwidth:** a fast excitation whose minimum delay is below the instrument's resolvable limit is rejected; a slower resolvable sequence is selected.
4. **Missing outlet channel:** no Fisher score is accepted when an outlet boundary is incomplete.
5. **Dose budget:** a high-dose excitation is rejected; the feasible lower-dose candidate is selected.
6. **Regime transition:** a hydraulic transition is rejected for fixed-regime identification.
7. **Unobserved flow reversal:** the evaluator abstains without signed port directions.
8. **Invalid covariance:** zero observation variance is rejected.

An independent JavaScript calculation generated the expected values for the 2-parameter fixtures. This mirror is not a native Rust test and does not model a real sensor or rig.

## 5. Next increments

- Derive sensitivity matrices from the actual positive kernel/compartment model and calibrated sensor convolution, rather than accept them as externally supplied fixtures.
- Add model-discrimination objectives for complete mixing, steady P, positive causal kernel and alternate compartment structures.
- Add robust design over a preregistered family of plausible parameter values and model mismatch.
- Add batch/run selection with a total dose and time budget, using D-, E- and A-optimal objectives.
- Add covariance whitening for correlated sensor/time errors only after independent covariance validation.
- Compare information gained per dose, run time, sensor burden and uncertainty reduction against fixed pulse/step baselines.

## 6. Evidence status

Authored: Rust candidate evaluator, eight frozen candidate sets, oracle, manifest and targeted CI workflow. The T0 score is synthetic and based on supplied sensitivity matrices. It is not a facility instruction or approved injection schedule. The non-potable T1 protocol and facility/operator approval remain prerequisites.

No physical operator, sensor, tracer, junction, potable system, contamination detector, toxicology, SANS 241 compliance or water-release authority is established.
