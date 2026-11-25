# ErgoStream continuous-time and hybrid dynamics transfer protocol v0.2

**Status:** proposed research/qualification contract; not implemented or experimentally validated  
**Related:** [#7171 structural/dynamics transfer witnesses](https://github.com/Luminous-Dynamics/symtropy/issues/7171), [#7193 Transfer Atlas](https://github.com/Luminous-Dynamics/symtropy/issues/7193), [#7149 hidden stream-world contract](https://github.com/Luminous-Dynamics/symtropy/issues/7149)

## 1. Why finite transition commutation is not enough

The v0.1 dynamics witness validates exact commutation between two finite deterministic transition systems. That is useful for conformance tests, but it does not cover:

- continuous evolution between observations;
- irregular or asynchronous sensor clocks;
- delayed, missing or duplicated measurements;
- stochastic process and observation noise;
- hybrid systems with discrete mode changes and continuous state;
- impacts, contact, switching surfaces and other non-smooth events;
- state/action mappings with bounded approximation error;
- uncertainty propagation over long rollouts.

A frame-indexed comparison can therefore report false transfer when sampling schedules differ, or miss a real equivalence when the domains use different time units. The continuous extension must compare trajectories and events under an explicit time map, not just align arrays by index.

## 2. Core dynamical correspondence

For deterministic continuous systems, let the source flow be:

[
dot{x}_s = f_s(x_s,u_s,t_s)
]

and target flow:

[
dot{x}_t = f_t(x_t,u_t,t_t).
]

A candidate transfer declares:
- state map (phi: X_s ightarrow X_t);
- action map (psi: U_s ightarrow U_t);
- time-coordinate map (	au: T_s ightarrow T_t);
- optional mode map (mu: Q_s ightarrow Q_t);
- admissible state, input, mode and time support;
- a norm, tolerance and uncertainty semantics.

For a differentiable invertible state map and time transform, the ideal local equivariance condition is:

[
Dphi(x_s) f_s(x_s,u_s,t_s),rac{d t_s}{d t_t}
=
f_t!left(phi(x_s),psi(u_s),	au(t_s)ight)
]

with the precise Jacobian / clock convention fixed by the protocol. The implementation should not assume this differential form when (phi) is non-differentiable, the time map is not smooth, or the system contains impacts and hybrid events.

A more general trajectory-level condition is:

[
d!left(
phi(x_s(t_s)),
x_t(	au(t_s))
ight)
leq
arepsilon(t_s)
]

over a declared holdout interval, under corresponding actions and initial-state support. The metric (d), tolerance (arepsilon), clock transformation and uncertainty bound must be declared before held-out target trajectories are released.

For sampled or hybrid dynamics, use event-aware trajectory semantics and do not force the smooth ODE condition across discontinuities.

## 3. Observation time is not physical time

Each observation needs separate fields for:

- measurement support interval in the source clock;
- arrival time in the receiver clock;
- clock identity and calibration/version;
- clock uncertainty / synchronization bound;
- sample duration or integration window;
- sequence number and known loss / discontinuity;
- whether the value is measured, inferred, predicted or replayed.

Never silently replace arrival time with measurement time. Never interpolate a missing sample without recording the interpolation model, its support and uncertainty.

The candidate may propose a clock transformation only from public calibration evidence. Hidden evaluator clock truth and future observations remain evaluator-owned. If the available evidence does not identify a unique alignment, the correct result is an interval/set of admissible alignments or abstention—not a fabricated precise offset.

## 4. Hybrid dynamics and event correspondence

Many important systems are hybrid:

[
dot{x}=f_q(x,u,t), qquad
x^+ = R_{qightarrow q'}(x^-,u,t)
]

where (q) is a discrete mode and (R) is a reset / jump map.

A transferable structure must separately identify:
- continuous-flow relations within a mode;
- guard/event conditions that trigger mode changes;
- event ordering and simultaneous-event semantics;
- reset maps and state discontinuities;
- contact / collision / saturation / switching conditions;
- which events are invariant, equivariant or non-transferable.

The evaluator must include boundary cases near guards and event surfaces. Averaging through a contact discontinuity or matching only pre/post snapshots is insufficient. A valid transfer can preserve event topology while changing time-to-event, impact magnitude or coordinate representation; those quantities must be scored separately.

## 5. Uncertainty and model discrepancy

Keep uncertainty sources separate:

1. initial-state uncertainty;
2. parameter uncertainty;
3. structural / model-form uncertainty;
4. process noise;
5. measurement noise and calibration uncertainty;
6. clock / synchronization uncertainty;
7. unobserved-input uncertainty;
8. residual model discrepancy / out-of-support uncertainty.

A candidate may not collapse these into one confidence score. Transfer qualification should test calibration on held-out trajectories, event times and intervention responses. Report interval coverage, negative log-likelihood / proper scoring rules where a predictive distribution is supplied, and selective risk versus coverage when the system can abstain.

A deterministic trajectory match on one seed is not evidence of calibrated predictive uncertainty.

## 6. Required evaluation metrics

Report every metric per generator family, mode, trajectory horizon and observation regime.

### Local dynamics
- derivative / vector-field error on admissible support;
- one-step and multi-step transition error;
- event-time and event-order error;
- reset-map error;
- action-conditioned prediction error.

### Trajectory transfer
- state error under declared (phi) and (	au);
- horizon to tolerance violation;
- cumulative and worst-case error;
- distributional calibration and interval coverage;
- invariant / equivariant relation recovery;
- errors near discontinuities and switching surfaces.

### Operational and epistemic
- intervention prediction error;
- downstream decision utility versus direct reactive policy;
- constraint / safety violations;
- false dynamics-transfer rate;
- false clock-alignment rate;
- false unique-map rate on non-identifiable systems;
- abstention calibration;
- negative-transfer cost;
- observations / interventions / compute to first validated transfer;
- p50 / p95 / p99 latency and stale-observation sensitivity.

Do not pool across trajectory horizon or event type to hide compounding error. A model that matches one-step prediction but diverges under a longer rollout must fail the corresponding long-horizon claim.

## 7. Generator and negative-control ladder

Start from deterministic synthetic systems with independently implemented exact or high-precision reference trajectories. Increase difficulty in separate strata:

1. **Clock scaling:** same flow under unit conversion and affine time dilation.
2. **Irregular sampling:** same flow under different sample times and sample durations.
3. **Delayed observations:** same physical trajectory, different arrival schedules.
4. **Input hold semantics:** zero-order hold, piecewise-constant control, and event-triggered control.
5. **Smooth nonlinear flow:** shared mechanism under state-coordinate transformations.
6. **Hybrid switching:** same guard/reset topology with transformed coordinates.
7. **Contact / impact:** event-time and momentum/energy discontinuities.
8. **Stochastic dynamics:** matched process-noise law or declared distributional transformation.
9. **Cross-domain false analogue:** same snapshots or local statistics, different continuous vector field or event/reset law.
10. **Partially observed system:** multiple latent dynamics compatible with current sensor streams; preserve set-valued hypotheses.

Negative controls must include:
- time scaling applied only to observations, not to the physical dynamics;
- clock drift / offset inconsistent with calibration data;
- time-varying sampling aliases that reverse apparent event order;
- similar trajectories generated by different mechanisms;
- a correct flow map but wrong reset map;
- same graph topology but different action-conditioned transition;
- out-of-support action or state values;
- missing events / censored observations that make the transfer non-identifiable.

## 8. Baselines and architecture ablations

Use the same underlying observations, action schedule, calibration data and compute budget where possible.

- exact analytic / numerical reference oracle for small worlds;
- parameter-fitted source/target model with no transfer;
- direct target-domain fitting from scratch;
- discrete-time recurrent model;
- neural ODE / continuous-time recurrent baseline when the task supports it;
- CfC-only and LTC-only;
- explicit state-space / typed transition representation only;
- graph/hypergraph representation only;
- HDC + CfC/LTC + explicit relation / dynamics witness hybrid;
- action-conditioned transformer / sequence-model baseline;
- strongest domain-appropriate simulator or system-identification baseline.

No architectural family is the default winner. Every added representation must justify its cost via held-out transfer, uncertainty calibration, robustness or decision utility.

## 9. Proposed acceptance gates

Prospective internal gates—not achieved results or external SOTA standards:

- exact commutation for all in-scope finite deterministic reference cases;
- zero hidden truth, future observations or evaluator seed in learner-facing serialized views;
- correct abstention / set-valued alignment on non-identifiable clock and dynamics pairs;
- no false transfer on wrong-vector-field, wrong-reset-map and wrong-clock controls;
- declared trajectory-error tolerance met through the full confirmatory horizon, not just one step;
- proper uncertainty coverage and non-compensable false-transfer guardrails;
- at least six independent dynamics families before a confirmatory cross-domain claim;
- matched comparison against the strongest compatible no-transfer and system-identification baselines.

Thresholds for approximate error, calibration and efficiency must be set after pilot variance / power analysis and before unblinding. No numeric universal tolerance is prescribed in v0.2 because units, scales and safety impact differ across physical systems.

## 10. Existing architecture composition

- Use Symthaea #11060's public episode/action interface and experiment-selection baselines where applicable.
- Use Symtropy #5990 for owner-side hidden-world custody.
- Use Symtropy #6027 / #6065's bounded typed model DSL for candidate model representation; do not create a second AST.
- Reuse Symthaea #12837 AV-STREAM for asynchronous multimodal observations and native clock provenance.
- Reuse #9094 / #13389 and Symtropy's existing physics/dynamics evaluators for downstream decision and robotics comparison.
- Keep the finite #7173 transition witness as a reference conformance test, not a production continuous-time evaluator.

## 11. Claim boundary

A continuous-time transfer result is scoped to the declared state, action, time, mode and observation support. It does not imply general physical understanding, causality outside the tested interventions, safety certification, biological equivalence, consciousness or SOTA performance. Physical deployment requires domain-specific safety engineering, calibration, instrumentation, independent review and operational monitoring.

## References

- Springer, *A survey of world models for physical AI with uncertainty representation and control* (3 September 2026): https://link.springer.com/article/10.1007/s44163-026-02122-1
- RoboDyna: https://arxiv.org/abs/2611.26969
- LIBERO-Dynamic: https://arxiv.org/abs/2610.24579
- World Models for Robotic Manipulation survey: https://arxiv.org/abs/2609.29669
