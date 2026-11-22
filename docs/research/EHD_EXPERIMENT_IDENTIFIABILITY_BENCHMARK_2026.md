# EHD experiment identifiability and robust design benchmark (2026)

**Status:** research specification v0.1  
**Related work:** [EHD-OED-001 #6660](https://github.com/Luminous-Dynamics/symtropy/issues/6660), [#6462](https://github.com/Luminous-Dynamics/symtropy/issues/6462), [#6641](https://github.com/Luminous-Dynamics/symtropy/issues/6641), [#6642](https://github.com/Luminous-Dynamics/symtropy/pull/6642), [#6650](https://github.com/Luminous-Dynamics/symtropy/pull/6650)  
**Claim boundary:** specification and manufactured mathematical example only. No optimizer, parameter estimator, EHD physical measurement, or propulsion validation is claimed.

## 1. Purpose

The EHD programme already has a multi-observable thrust protocol and a broad control/falsification matrix. The missing layer is a quantitative method for deciding whether a proposed experiment can answer the scientific question it is intended to answer.

The experiment-design layer must answer three different questions:

1. **Parameter identifiability:** can the selected observations distinguish the physical parameters of the declared model?
2. **Model discrimination:** can the observations distinguish competing physical closures?
3. **Artifact discrimination:** can the observations distinguish physical response from force-stand, electrical, thermal, mechanical, optical, or environmental nuisance effects?

These are separate qualification outputs. Low prediction error does not imply correct parameter recovery. A precise fit can be produced by non-identifiable parameter combinations or by a nuisance model that is incorrectly absorbed into physical coefficients.

This specification reuses the model, measurement, uncertainty, scenario and evidence infrastructure owned by the other EHD workstreams. It does not define a new plasma solver or replace the force-balance / PIV / control-volume protocol.

## 2. Research grounding

The design methods are established research and must be treated as comparators, not as novel Luminous algorithms:

- Liu, Maini & Baker (2026), [Optimal experiment design for practical parameter identifiability and model discrimination](https://doi.org/10.1016/j.mbs.2026.109710), *Mathematical Biosciences* 399, 109710. The paper uses profile likelihood for practical identifiability and formulates model-discriminating experiment design as an optimal-control problem.
- Lam, Docherty & Murray (2022), [Practical identifiability of parametrised models: A review of benefits and limitations of various approaches](https://doi.org/10.1016/j.matcom.2022.03.020), *Mathematics and Computers in Simulation* 199, 202–216.
- Overstall (2022), [Properties of Fisher information gain for Bayesian design of experiments](https://doi.org/10.1016/j.jspi.2021.10.006), *Journal of Statistical Planning and Inference*. Fisher-information-only criteria can have parameter-redundancy and non-identifiability failure modes.
- Yeo (2020), [NISTIR 8298: A Summary of Industrial Verification, Validation, and Uncertainty Quantification Procedures in Computational Fluid Dynamics](https://doi.org/10.6028/NIST.IR.8298).

The intended contribution is an EHD-specific, falsification-first composition: physical parameters, artifact hypotheses, independently measured observables, robust design, held-out validation and replayable evidence.

## 3. Declared model boundary

### 3.1 Physical parameters

The parameter vector is supplied by a version-pinned discharge/plasma/neutral-flow equation set. Depending on the selected regime, candidate parameters may include:

- ion mobility and charge-transport coefficients;
- ion-neutral momentum-transfer coefficients;
- species densities and reaction rates;
- neutral transport and viscosity parameters;
- thermal equilibration and heat-transfer coefficients;
- electrode and discharge boundary parameters.

This list is not a universal EHD parameterization. A parameter is included only when the selected model defines it and the available evidence can constrain it. Unsupported parameters are reported as unsupported, not estimated by proxy.

Positive parameters use declared transformations, such as logarithmic coordinates. The transformation, reference scale and parameter bounds are part of the frozen design input.

### 3.2 Nuisance parameters and artifact models

Physical parameters must be distinguished from nuisance variables. Potential nuisance effects include:

- force-balance zero, drift, hysteresis and cross-axis response;
- cable/feedthrough electrostatic or mechanical forces;
- frame/support drag and ambient flow;
- thermal drift and convection;
- vibration and acoustic coupling;
- magnetic/electromagnetic interactions;
- PIV/LDV calibration, timing, missing-vector and reconstruction errors.

A nuisance term may depend on voltage, geometry, orientation or time. The benchmark must not assume every artifact is a constant offset.

If physical and nuisance effects cannot be distinguished with the available observations, the correct output is **NotIdentifiable** or **NotDiscriminable**, not a forced parameter estimate.

## 4. Observation and design contract

The observation vector may combine independently acquired channels:

- calibrated force-balance raw time series;
- PIV/LDV or equivalent velocity fields;
- control-volume momentum terms;
- synchronous electrode-terminal voltage/current and real power;
- pressure, gas density/composition and temperature;
- vibration, magnetic and other declared nuisance channels.

Candidate design variables may include waveform family, geometry family, gas/pressure cohort, polarity/orientation controls, inlet conditions, duration, spatial diagnostic locations and sampling cadence. These are candidate dimensions, not instructions to operate high-voltage equipment. Any physical design remains subject to independent laboratory safety and metrology review.

Every candidate design must freeze:

- physical and nuisance model versions;
- operating-condition cohort identity;
- observation channels and their expected availability;
- measurement covariance/correlation model and calibration identities;
- resource budget and admissibility constraints;
- holdout family definitions;
- random seeds or sampling design;
- analysis and decision-rule version.

No design may gain an advantage by using a sensor channel that is unavailable in the declared budget or holdout cohort.

## 5. Mathematical screening and acceptance

Let the expected observation be:

\[
g(\theta,\eta;d)
\]

where \(\theta\) denotes physical parameters, \(\eta\) nuisance parameters, and \(d\) the candidate design. Let the declared observation covariance be \(\Sigma_d\).

The local sensitivity matrix is:

\[
J_d = \frac{\partial g}{\partial(\theta,\eta)}
\]

and the local information matrix is:

\[
F_d = J_d^{\mathsf T}\Sigma_d^{-1}J_d.
\]

The benchmark reports the singular spectrum, numerical rank, parameter scaling, condition diagnostics and weakest identifiable directions. These diagnostics are screening results, not proof of practical identifiability.

Where the nuisance block is nonsingular, a nuisance-adjusted local information matrix for the physical parameters can be formed by the Schur complement:

\[
F_{\theta\mid\eta}
=
F_{\theta\theta}
-
F_{\theta\eta}F_{\eta\eta}^{-1}F_{\eta\theta}.
\]

If the nuisance block is singular or poorly conditioned, the implementation must apply a versioned generalized-inverse/rank policy and report unresolved directions. It must not silently regularize away an identifiability failure.

Practical identifiability is checked with profile likelihood or another explicitly justified global method. Report parameter-wise confidence intervals, flat/unbounded profiles, boundary effects and parameter trade-offs. A local Fisher-information score alone cannot issue an identified-parameter disposition.

## 6. Manufactured negative control: repeated measurements do not repair rank deficiency

Consider the deliberately simplified manufactured observation model:

\[
y_j = a x_j + b + \epsilon_j,
\qquad
\epsilon_j\sim\mathcal N(0,1).
\]

Here \(a\) and \(b\) are abstract unknown coefficients, and \(x\) is an abstract controlled coordinate. This is a linear-algebra test fixture, **not an EHD constitutive law**, not a claim that EHD thrust is odd in current, and not a proposed physical drive schedule.

For three repeated observations at \(x=1\), the design matrix is:

\[
X_{\mathrm{repeat}}
=
\begin{bmatrix}
1&1\\
1&1\\
1&1
\end{bmatrix}.
\]

Its information matrix is:

\[
X_{\mathrm{repeat}}^{\mathsf T}X_{\mathrm{repeat}}
=
\begin{bmatrix}
3&3\\
3&3
\end{bmatrix},
\]

with eigenvalues \(0\) and \(6\). The rank is one. The data identify only the combination \(a+b\). Repeating this identical condition reduces noise in the observable combination but cannot separately identify \(a\) and \(b\).

For a balanced manufactured design with \(x\in\{-1,0,1\}\):

\[
X_{\mathrm{balanced}}
=
\begin{bmatrix}
-1&1\\
0&1\\
1&1
\end{bmatrix},
\qquad
X_{\mathrm{balanced}}^{\mathsf T}X_{\mathrm{balanced}}
=
\begin{bmatrix}
2&0\\
0&3
\end{bmatrix}.
\]

The eigenvalues are \(2\) and \(3\), and the rank is two. The two abstract coefficients are now locally identifiable under the manufactured model and declared unit-noise covariance.

This tiny example must be included as an exact negative/positive control in the eventual implementation. It demonstrates why a larger sample count at one operating point is not a substitute for independent excitation or independent measurement channels.

### Near-collinear numerical-conditioning control

The exact-rank pair is not sufficient by itself. Add a deliberately near-collinear full-rank design:

\[
X_{\mathrm{near}}=
\begin{bmatrix}
1&0\
1&10^{-9}
\end{bmatrix}.
\]

Its information matrix has approximate eigenvalues \(2\) and \(5\times10^{-19}\), giving singular values approximately \(1.4142\) and \(7.0711\times10^{-10}\). At a relative singular-value threshold of \(10^{-12}\), the design remains numerically rank two, but its condition number is approximately \(2\times10^9\).

This is deliberately not a claim of strong practical identifiability. It verifies that the numerical rank procedure does not lose a resolvable small singular direction through cancellation in \(X^\mathsf{T}X\), and it demonstrates why rank alone is insufficient. The downstream disposition must retain the condition number/weakest direction and require profile-likelihood or other practical-identifiability checks before treating the parameters as well determined.

The reference kernel uses a two-column one-sided Jacobi rotation to obtain the singular spectrum from rotated design columns rather than subtracting nearly equal Gram-matrix eigenvalues.

## 7. Design tournament

At identical admissible cost, model set, measurement channels and holdout families, compare:

1. random/balanced baseline design;
2. D-optimal design;
3. A-optimal design;
4. E-optimal design;
5. maximin nuisance-adjusted design;
6. profile-likelihood practical-identifiability design;
7. pairwise model-discrimination design;
8. robust joint design.

D-optimality maximizes information volume but is not sufficient on its own: it can improve the determinant while leaving the weakest direction poorly identified. The benchmark must report both the full spectrum and practical interval behaviour.

Robust designs are evaluated over declared parameter ranges/prior draws, noise/covariance envelopes, and held-out condition families. The optimizer must not tune on the confirmatory holdout.

## 8. Model and artifact discrimination

For competing models \(M_i\) and \(M_j\), compare their predictive distributions under a declared distance or decision criterion. Candidate approaches include expected log Bayes factor, symmetric KL divergence where defined, or Jensen–Shannon divergence.

Report pairwise separability rather than a single winner score. Include:

- at least two explicitly specified physical closure hypotheses;
- at least one explicit measurement-artifact hypothesis;
- cases where the models are separable;
- cases where models are indistinguishable under the chosen observations;
- a noise/resolution floor below which the result must be NotDiscriminable.

The artifact model is not a competing physical theory. It is a falsification control. A model-selection algorithm must not choose a physical closure merely because nuisance explanations were omitted from its candidate set.

## 9. Sequential experiment selection

A sequential selector proposes a next candidate condition based on:

- unresolved physical parameter directions;
- unresolved physical/nuisance trade-offs;
- the closest competing model pair;
- expected improvement under the declared resource budget;
- diagnostic availability and calibration uncertainty.

Each recommendation records which ambiguity it is expected to resolve and which ambiguities it cannot resolve.

The confirmatory holdout cohort is frozen before the adaptive selection sequence. Adaptive experiment selection does not permit retrospective changes to the final decision rule or holdout assignment.

## 10. Adversarial benchmark corpus

Include at minimum:

- exactly rank-deficient physical/nuisance sensitivities;
- near-singular but technically full-rank sensitivities;
- repeated observations at one operating point;
- correlated measurement noise and misspecified covariance;
- a local Fisher-information optimum with poor profile-likelihood coverage;
- voltage-/geometry-dependent nuisance forces;
- model pairs that are indistinguishable on one cohort but separable on another;
- parameter boundary effects and unbounded profiles;
- a design that improves determinant while worsening the weakest information direction;
- train/holdout leakage across samples from one experimental session;
- missing sensors, sensor dropout, timestamp skew and calibration drift;
- optimizer use of an inadmissible condition or unavailable channel;
- structured model discrepancy that cannot be repaired by shifting one physical parameter.

## 11. Separate qualification outputs and prospective targets

Do not combine the following into one leaderboard score:

| Output | Proposed qualification |
|---|---|
| Structural identifiability | Correct rank and abstention on manufactured exact cases |
| Practical identifiability | Profile-likelihood intervals and unresolved-parameter reports |
| Nuisance separability | No physical estimate when physical/nuisance directions remain confounded |
| Model discrimination | Pairwise separability, confusion matrix and NotDiscriminable disposition |
| Artifact rejection | False-positive rate and detection power on matched negative controls |
| Predictive validation | Held-out prediction error and interval coverage |
| Design efficiency | Information/interval improvement at matched admissible cost |
| Robustness | Performance over frozen parameter/noise/condition envelopes |

Prospective targets, not achieved results:

- Exact recovery on identifiable manufactured cases within declared floating-point tolerance; explicit abstention on rank-deficient cases.
- Nominal 95% interval empirical coverage target of 90–98% over a preregistered simulation campaign, with Monte Carlo uncertainty and sample count reported.
- At equal budget, at least 2x improvement in the minimum normalized information eigenvalue versus random design on designated identifiable cases, without worsening held-out prediction.
- At least 30% reduction in designated profile-likelihood interval widths versus a frozen naive design where practical identifiability is achievable.
- Artifact false-positive-rate target <=1% on a preregistered negative-control corpus, with detection power reported separately.
- Model-discrimination error target <=5% only for model pairs whose predicted separation is above the declared noise/resolution floor; otherwise require NotDiscriminable.

Targets must be versioned. They may be revised only on the basis of documented statistical/physical evidence, not to relabel earlier outcomes.

## 12. Evidence receipt

A future design receipt should contain:

- exact model and parameter schema identities;
- exact candidate cohort and design variables;
- physical and nuisance parameter ranges;
- sensor/channel set and measurement-covariance identity;
- sensitivity matrix identity, singular values, rank threshold and scaling;
- profile-likelihood configuration and interval outputs;
- competing-model set and pairwise discrimination outputs;
- design algorithm, version, objective and resource budget;
- training/holdout identities and data digests;
- failed, unsupported and abstained candidates;
- reproducible output digest and explicit claim ceiling.

The receipt proves what was evaluated and what the design method returned. It does not prove that the physical model is correct or authorize an experiment.

## 13. Integration and ownership

- [#6462](https://github.com/Luminous-Dynamics/symtropy/issues/6462): source-pinned physical model and falsification/control matrix.
- [#6641](https://github.com/Luminous-Dynamics/symtropy/issues/6641) / [#6642](https://github.com/Luminous-Dynamics/symtropy/pull/6642): independent thrust observability and measurement protocol.
- [#6650](https://github.com/Luminous-Dynamics/symtropy/pull/6650): control-volume reducer; consumes reduced measurements and is not a raw PIV/LDV estimator.
- [#6640](https://github.com/Luminous-Dynamics/symtropy/issues/6640): species-resolved collisional energy channels.
- [#6627](https://github.com/Luminous-Dynamics/symtropy/issues/6627): transport polynomial exactness and moving-mesh qualification.
- **Symthaea:** hypothesis extraction, uncertainty-aware design proposals and independent claim critique.
- **Symtropy:** frozen design cohorts, deterministic scoring/replay, provenance and machine-readable design receipts.
- **Specialist solvers and measurement pipelines:** authoritative physical equations and raw observation reduction.

## 14. Exit criteria

1. The manufactured corpus detects exact and near rank deficiency correctly.
2. Physical/nuisance confounding is exposed rather than absorbed into physical parameters.
3. All design baselines are compared at matched budget and held-out cohorts.
4. The tournament is reproducible from frozen inputs and reports every failure and abstention.
5. A source-pinned EHD dataset is reconstructed with complete operating and measurement boundaries before any physical OED claim.
6. A held-out cohort demonstrates improved practical identifiability or model discrimination at matched cost.
7. No physical parameter estimate or model winner is emitted when the available evidence is non-identifying.
8. Design recommendations carry provenance and remain separate from experimental authorization.

**Current state:** the analytic matrix example above is a mathematical reference in this document. The optimizer, profile-likelihood pipeline, model tournament, EHD parameter adapter and physical experiment remain unimplemented/unvalidated.

## Executable D-optimal versus E-optimal tournament — 22 November 2026

The reference kernel now includes a bounded exhaustive design tournament over four abstract candidate sensitivity rows, with the same three-observation budget for every design.

Candidate rows:
- C0 = [-3, -3]
- C1 = [-3, -2]
- C2 = [-3, -1]
- C3 = [-2, 0]

All manufactured observation variances are 1. These rows are abstract sensitivity vectors, **not EHD voltages, geometries, operating conditions or physical experiment recommendations**.

There are 20 combinations with replacement at a three-observation budget. The two criteria choose different allocations:

| Criterion | Selected candidates | det(F) | Minimum eigenvalue |
|---|---|---:|---:|
| D-optimal | [0, 2, 3] | 76 | 2.583592 |
| E-optimal | [0, 3, 3] | 72 | 3.151142 |

D-optimal maximizes log(det(F)), the information volume. E-optimal maximizes the weakest information direction. In this reference example, E-optimal increases the minimum information eigenvalue by approximately **21.97%**, while accepting a lower information determinant. Both designs use exactly three observations from the same candidate set.

The exhaustive reference selector:
- evaluates the same candidate set and observation budget for both criteria;
- rejects rank-deficient candidate allocations;
- validates positive observation variances;
- imposes a maximum observation budget of 32 and an explicit maximum design-evaluation count;
- uses deterministic lexicographic tie-breaking;
- ranks E-optimal designs by the smallest singular value (monotonically equivalent to the smallest information eigenvalue), avoiding underflow from squaring very small singular values.

This is a concrete manufactured counterexample to treating D-optimality as the sole design objective. It does not demonstrate an EHD physical experiment-design improvement. The next steps remain robust parameter/noise envelopes, nuisance-adjusted criteria, model discrimination, practical profile-likelihood checks and a source-pinned EHD design adapter.
