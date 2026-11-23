# GAUGE slope identifiability and experiment design v0.1

**Parent:** [GAUGE rigid-body qualification #6446](https://github.com/Luminous-Dynamics/symtropy/issues/6446)  
**Domain child:** [#6584](https://github.com/Luminous-Dynamics/symtropy/issues/6584)  
**Generic calibration owner:** [#3784](https://github.com/Luminous-Dynamics/symtropy/issues/3784)

## Why this is a separate benchmark slice

A simulator can match one measured trajectory while its fitted physical parameters remain non-identifiable. This fixture isolates one such ambiguity: sliding friction versus incline-angle registration bias.

It does not implement a second calibration engine. It contributes one GAUGE-specific analytic model and equal-budget excitation comparison to the generic sensitivity/Fisher/SVD/experiment-design work owned by #3784.

## Analytic model

For an ideal rigid block sliding down a fixed incline with Coulomb friction:

`a = g (sin(phi) - mu cos(phi))`, `phi = theta + delta`

where `mu` is friction coefficient and `delta` is an unknown angle-registration bias.

The local physical sensitivity is:

`da/dmu = -g cos(phi)`

`da/ddelta = g (cos(phi) + mu sin(phi))`

A single angle produces one sensitivity direction for two unknowns. Repeating that angle adds information in the same direction but cannot raise the local Jacobian rank above one. Distinct inclinations can make the local information full rank.

This v0.1 fixture specifically models **sustained downhill sliding driven by gravity alone**. It rejects inclinations at or below the model's friction angle; externally driven sliding or initial-velocity coast-down requires a different declared model and is not silently mixed into this one.

## Implemented reference

- Explicit characteristic scales for friction and angle-bias coordinates.
- Local scaled Jacobian for each candidate excitation.
- Fisher information for independent acceleration measurements with declared standard deviations.
- Eigenvalues, singular values, effective rank, condition number and D-optimal log determinant.
- Exhaustive D-optimal two-trial selection from a declared candidate set, with deterministic tie handling.
- Reduced **IDTD-style per-parameter Schur-decoupling coverage**: each parameter is credited its best segment score, `Schur(F,{i}) / F_ii^alpha`, then combined through a negative-log cost.
- Explicit alpha information-gate behavior and ridge/log epsilon validation.
- Rank-one, near-duplicate-angle, invalid-matrix and sustained-sliding-regime negative controls.
- Equal-budget comparison: every candidate pair contains exactly two trials.

The Schur-decoupling scorer is a small two-parameter benchmark analogue of the objective in [IDTD (arXiv:2610.10905)](https://arxiv.org/abs/2610.10905). It does **not** reproduce IDTD's robot simulator, segmented command trajectories, policy optimization, CMA-ES, parameter estimation or real-robot experiment.

### Pinned external baseline

The external IDTD comparator is pinned in `external_baseline_manifest.idtd.v0.1.json` to code commit `b47eed10adb4ea3d1987fc4211a9251f5de18ea6`, with blob identities for the README, MIT LICENSE, FIM evaluator and active optimizer.

The paper's v2 (revised 2026-10-27) reports a 39.6% average reduction in parameter identification error over its strongest prior active-exploration baseline across linear dynamics, Go2, G1 and Crazyflie simulation, plus a real K1 humanoid validation. This is the authors' reported result; Symtropy has **not** independently reproduced the paper's experiment or verified the 39.6% claim.

The public implementation uses finite-difference state sensitivities, hardware-measurable state-channel selection, segment-wise per-parameter Schur coverage and CMA-ES policy-conditioned Bezier command search. It depends on an external SimForge/JaxRLWorld runtime and a Python stack. Our current comparator validates only the reduced two-parameter score and counterexamples.

A fair follow-on should compare repeated-angle, fixed-grid, random, D-optimal and IDTD-style designs with the same physical trial count, observation channels, duration, policy/actuator limits, compute budget and blind physical holdouts. Report parameter recovery, held-out trajectory error, per-parameter conditional Fisher coverage, uncertainty/calibration, failure/abstention, physical trial count and p50/p95 optimization time separately.

## Synthetic example

The analytical fixture uses `mu=0.25`, `delta=0.02 rad`, `g=9.81 m/s^2`, and acceleration standard deviation `0.05 m/s^2`.

Twelve repeated measurements at 30 degrees remain rank one. A two-trial design at 20 and 75 degrees stays within the sustained-downhill-sliding regime and becomes locally full rank with a well-conditioned information matrix. Five degrees is rejected for this model because gravity cannot sustain downhill sliding against the declared friction. These values are deterministic analytic outputs, not GAUGE physical results.

A separate synthetic Fisher-matrix counterexample shows why D-optimality and parameter decoupling must both be reported. The correlated matrix `[[100,99],[99,100]]` has determinant 199 and therefore beats the independent `[[10,0],[0,10]]` matrix's determinant 100 under D-optimality. But the reduced IDTD-style Schur score strongly prefers the independent matrix because each parameter is separately identifiable. Neither objective is universally superior: one rewards total information volume; the other rewards per-parameter separability.

## Important limitations

- The ideal Coulomb block model omits compliance, contact transients, velocity-dependent friction, rolling, surface variability and solver effects.
- Fisher information is a local-linear diagnostic under an independent Gaussian observation-noise model. It is not a posterior, global-identifiability proof or confidence interval.
- Parameter scale choices affect condition numbers and are explicit in every model input.
- The public GAUGE slope-slider metadata contains orientation declarations, but the trial data's Euler units/frame semantics still need source verification. The synthetic angle values here are **not** inferred from those records.
- Real-data use requires source admission, actual geometry/units, measurement uncertainty and blind recording-level holdouts. No raw data is imported or score reproduced.
