# J3 Robust Co-Design Protocol v0.1

**Status:** research protocol / M0–M1 definition. This document contains no new optimization result, hardware measurement, or completed physical qualification.

**Parent:** [J3-POSE-ROBUST-HAND-001 #6138](https://github.com/Luminous-Dynamics/symtropy/issues/6138)  
**Robust extension:** [ROBUST-JOINT-CODESIGN-001 #6145](https://github.com/Luminous-Dynamics/symtropy/issues/6145)  
**Common evidence contract:** [#6140](https://github.com/Luminous-Dynamics/symtropy/issues/6140)  
**Analytic directional-force reference:** [PR #6139](https://github.com/Luminous-Dynamics/symtropy/pull/6139)

## 1. Research question

Can one physically realizable tendon-routing and actuator design preserve the required fingertip force region across:

- all required poses and force directions;
- measured, direction-dependent transmission loss;
- manufacturing variation and assembly/remount variation;
- actuator torque-speed and thermal limits;
- tendon stiffness, pretension, creep, and service state;

while retaining the source-matched bandwidth and packaging constraints?

The optimization target is a **robust feasible design set**, not a single peak-force number.

## 2. Source baseline: what must be reproduced

Kumar et al., *PACT: Pose-Robust Actuation Co-Design for Tendon-Driven Robotic Hands*, arXiv:2611.05302 (3 November 2026), defines the feasible fingertip-force set by the full equilibrium condition

```text
F(q) = { f in R^3 | J(q)^T f is in T_tau }
```

where the actuator/tendon torque set retains pull-only tendon bounds and a separate bidirectional abduction/adduction actuator. Singular or lower-dimensional force sets are invalid; a pseudoinverse projection is not an acceptable substitute.

The required-force coverage at pose p is

```text
C_p(x) = volume(R_p intersect F(q_p; x)) / volume(R_p)
```

where R_p is the pose-specific required force region. The nominal design objective is

```text
maximize_x  min_p C_p(x)
```

subject to the source-matched isolated flexion/extension bandwidth constraints.

PACT reports a nominal shared-design minimum coverage of 0.88 across eight poses, versus 0.24 for its task-agnostic baseline, with a 5 Hz minimum isolated flexion/extension bandwidth. Its search enumerates routing/actuator topologies, samples continuous variables using scrambled Sobol points, then refines candidates with bounded adaptive Nelder–Mead. The authors explicitly identify the search as heuristic rather than a global-optimality certificate.

The prototype's force-set experiment is not an eight-pose physical validation: the reported setup uses one prescribed pose, selected feasible-set vertices, current limits at 50% of peak torque, five-second steady-state samples, five repetitions per tested actuator combination, and a six-axis fingertip load cell. The measured/source-model discrepancy is therefore important but does not yet establish whole-hand force-region coverage under all operating conditions.

Primary source: https://arxiv.org/abs/2611.05302

## 3. Robust extension

Let:

- x: design variables (routing topology/signs, free moment arms, capstan dimensions, actuator assignment);
- p: task pose;
- d: force/wrench direction within the task region;
- ξ: uncertain physical parameters;
- F(q_p; x, ξ): feasible force set under exact equilibrium and actuator limits;
- R_p: preregistered required force region.

Two distinct robustness measures must be retained.

### 3.1 Bounded stress-set coverage

```text
C_stress(x) = min_(p in P, xi in U) C_p(x, xi)
```

U is a declared, physically defensible uncertainty set built from source bounds and/or calibration data. It must preserve correlated effects where evidence supports them. Unknown bounds remain unknown; they must not be replaced by convenient Gaussian assumptions or arbitrary percentages.

### 3.2 Population lower-tail coverage

Where sufficient representative measurements support a distribution, report the empirical distribution of C_p(x, ξ) and a predeclared lower-tail statistic (for example, a lower quantile and lower-tail CVaR). The distribution, sampling unit, weighting, and confidence method must be stated.

The stress-set minimum and population lower-tail result answer different questions. Neither may be hidden behind a mean.

### 3.3 Uncertainty classes

Keep at least these categories separate:

| Class | Examples | Evidence needed |
| --- | --- | --- |
| Transmission | pulley/capstan friction, guide contact, hysteresis, direction-dependent efficiency | measured tendon input/output force or calibrated independent wrench/tension channels |
| Compliance | tendon stiffness, elongation, creep, pretension | calibrated load/elongation curves over load and service state |
| Geometry | routing-anchor position, capstan radius, moment-arm tolerance | dimensional metrology and as-built configuration |
| Actuator | torque constant, speed-torque map, current limit, thermal derating | calibrated output torque/speed and electrical/thermal observation |
| Sensing | force/tension bias, encoder offset, clock alignment | traceable calibration, uncertainty, synchronization receipt |
| Lifecycle | wear, fray, remount, lubrication, recalibration | identified articles, cycle/service history, repeated calibration |

Separate epistemic uncertainty (what the model/parameters are not known to be) from aleatoric variation (run-to-run physical variation). Do not reduce both to one noise parameter.

## 4. Fair algorithm comparison

The comparison baseline is the PACT search procedure, reproduced under the same design bounds, topology set, task requirements, seed policy, and evaluation budget.

Compare that baseline with at least one additional search method. Candidate methods may include deterministic global search with valid bounds, interval branch-and-bound on reduced parameter spaces, or another documented optimizer. The exact selection is part of the experiment plan.

Every method must report:

- evaluations and wall-clock/compute budget;
- deterministic seed and configuration digest;
- feasible, infeasible, rejected, and failed candidate counts;
- best-known objective and per-pose/per-condition values;
- convergence/restart behaviour;
- lower and upper bounds where a valid certificate exists.

If no valid upper bound or certificate exists, report the optimum and optimality gap as **unknown**. A better incumbent is not proof of global optimality.

## 5. Data separation and held-out evaluation

Use three explicit partitions:

1. **Design/calibration:** source-matched data and measurements used to identify parameters and optimize candidate designs.
2. **Validation:** a separate set used for model selection, residual diagnosis, and uncertainty-set checks.
3. **Held-out qualification:** frozen poses, directions, temperature/load/service states, and/or physical articles not used for design selection or model tuning.

Bind each partition to immutable source files, calibration records, configuration digest, article identity, and deterministic seed. Do not move a condition from held-out to design after observing a poor result without publishing a new protocol version and preserving the original result.

Use the eight PACT poses for source reproduction; add held-out poses and trajectories only as separately labelled generalization tests. Keep per-pose results visible.

## 6. Required scorecard (no aggregate rank)

For each candidate, report:

- nominal minimum pose coverage;
- bounded stress-set minimum coverage;
- held-out lower-tail coverage and uncertainty interval;
- full force-set geometry and exact equilibrium residual;
- force-direction and magnitude residuals against independent measurement;
- flexion and extension bandwidth separately for MCP, PIP, DIP;
- actuator peak/continuous torque-speed and thermal margins;
- tendon tension, elongation, creep, slack, hysteresis, and transmission loss by joint/direction;
- reflected inertia, backdrive impedance, distal mass, installed mass and volume;
- electrical input energy, mechanical output work, and separately observed regenerative electrical energy;
- failure, exclusion, censoring, calibration invalidation, repair and requalification counts.

Do not compare a directional force maximum with volumetric force-region coverage. Do not call estimated tendon tension an independent measurement. Do not infer battery regeneration from negative mechanical work.

## 7. Prospective gates and targets

These are targets, not achievements.

| Gate | Target |
| --- | --- |
| Nominal reproduction | Reproduce the source's 0.88–1.00 reported coverage range and source-matched constraints before claiming PACT reproduction |
| Source-matched bandwidth | At least 5 Hz isolated flexion/extension for MCP, PIP and DIP in the matched model reproduction |
| Loss-aware stretch | At least 0.88 worst-pose coverage after measured losses on held-out conditions |
| Search comparison | At equal evaluation budget, improve at least one declared robust Pareto dimension over the PACT heuristic without relaxing constraints |
| Analytic validity | Zero false-feasible force witnesses in deterministic/adversarial fixtures |
| Physical model fidelity | Publish residual and uncertainty stratified by pose, direction, load, temperature, article and service state |

The 0.88 loss-aware target is deliberately harder than reproducing the nominal 0.88 result. The current 0.75 loss-aware rescoring recorded in #6138 is a model-based comparison baseline, not an all-pose physical result.

Thresholds for statistical confidence and lifecycle acceptance must be preregistered after instrument repeatability, sample size, and application requirements are known. Do not choose them after seeing the outcomes.

## 8. M0–M5 promotion

- **M0 — traceability:** source equations, units, design bounds, task definitions, and evidence classes.
- **M1 — analytic correctness:** full equilibrium, force-region geometry, known-answer fixtures, negative controls, and independent witness reconstruction.
- **M2 — numerical reproduction:** PACT baseline, independent implementation, search-budget comparison, convergence checks, and held-out split integrity.
- **M3 — instrumented component:** independent fingertip wrench and tendon tension/elongation, joint-angle reference, current/voltage, speed, and thermal channels on a guarded non-human-contact fixture.
- **M4 — integrated task:** all source poses plus held-out tasks, directions, trajectories, and disturbance conditions.
- **M5 — lifecycle:** aged/remounted/rerouted articles, service history, calibration invalidation, repair time, and post-service requalification.

Passing M0/M1 does not establish physical capability. Passing M2 does not qualify hardware. No human-contact, prosthetic, rehabilitation, or safety claim follows without separate application-specific engineering and review.

## 9. Known limitations

- The existing J3 analytic oracle begins with direction-constrained feasibility; it does not yet implement full 3D force-region coverage.
- PACT's physical validation must not be represented as a physical measurement of the full eight-pose coverage objective.
- The current loss-aware 0.75 result is a rescore, not a new independent physical campaign.
- A heuristic optimizer cannot support a global-optimality claim without an independently valid bound.
- Physical work requires approved fixtures/procedures, calibration, and separate hazard analysis.

**Promotion rule:** every result must name its exact source revision, fixture/article identity, task/pose/condition, metric definition, uncertainty, denominator, verification state, and claim ceiling.