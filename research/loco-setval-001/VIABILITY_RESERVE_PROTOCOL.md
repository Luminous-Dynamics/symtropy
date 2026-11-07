# LOCO Viability Reserve — axiswise benchmark protocol

Status: proposed finite and simulation protocol; no physical critical thresholds or Symtropy robot margins are claimed.

Parent: [LOCO-VIABILITY-RESERVE-001 / #4130](https://github.com/Luminous-Dynamics/symtropy/issues/4130).
Reference bracket auditor: [#4132 / PR #4133](https://github.com/Luminous-Dynamics/symtropy/pull/4133).

## 1. Research question

For a declared model, state/phase, safe set, action semantics and recovery target, how much additional uncertainty can be admitted before:

1. membership in the maximal robust controlled-invariant set is lost; or
2. guaranteed reachability of the declared recovery target is lost?

These are distinct thresholds. Do not substitute static support-polygon distance for either one.

## 2. One-axis uncertainty ladder

For each axis i, define a family of admissible sets U_i(rho) with strictly increasing levels rho_0 < rho_1 < ... < rho_n.

The level orientation must be explicit: **larger rho always means at least as much adverse uncertainty**, not simply a larger raw parameter value. Examples:

- Friction: lower admissible friction bound decreases as rho increases.
- Contact compliance: upper admissible compliance increases.
- Push/impulse: the admissible magnitude/direction set expands.
- Actuator loss: admissible torque/speed/bandwidth loss expands.
- Sensor: delay/dropout/staleness bounds expand.
- Individual dynamics: parameter uncertainty sets expand.

For each level, record the exact parameter/disturbance set digest and prove or explicitly declare its nesting relationship. A scalar label is not evidence of set inclusion.

### Freeze these quantities during a one-axis sweep

- model and hybrid-mode semantics;
- state partition / abstraction resolution;
- safe set and recovery target;
- action set and action interpretation;
- other uncertainty axes and disturbance bounds;
- contact/reset laws and event-order policy;
- oracle/verifier version and resource budgets.

If any of these change, it is a different experiment, not a continuation of the same one-axis ladder.

## 3. Expected monotonicity

For fixed safe set and actions, if the admissible successor relation is enlarged pointwise, the robust predecessor cannot grow. Consequently, the maximal robust controlled-invariant set and the robust reach-to-recovery set cannot grow under a truly nested adverse uncertainty family.

The bracket auditor checks the reported outcome sequence for the expected pattern:

    viable, viable, ..., viable, nonviable, ..., nonviable

A later viable outcome after a nonviable outcome is a **monotonicity violation**. Preserve it, investigate whether the family was actually nested or some model/action semantics changed, and do not smooth or repair it.

This check is conditional on genuine nesting. The reference utility consumes externally computed statuses and cannot prove that U_i(rho) was correctly constructed.

## 4. Threshold reporting

For a qualified nested monotone ladder, report:

- largest tested level that preserves invariance;
- smallest tested level that loses invariance;
- largest tested level that preserves guaranteed recovery;
- smallest tested level that loses guaranteed recovery;
- whether either threshold is left- or right-censored by the tested range.

Never interpolate a formal critical threshold between tested levels. A result of 0.4 viable and 0.5 nonviable is a bracket [0.4, 0.5], not proof of an exact threshold such as 0.437.

If all levels pass, report right-censored above the maximum tested level. If all fail, report left-censored below the minimum tested level. If nestedness is undeclared or monotonicity fails, no qualified threshold bracket is emitted.

## 5. Coupled-axis experiments

One-axis reserve values must not be added, multiplied or treated as a single scalar robustness score.

After one-axis ladders, run predeclared 2D/3D slices, such as:
- friction × contact delay;
- actuator torque loss × thermal derating;
- push impulse × gait phase;
- compliance × contact-sensor false-negative rate;
- model-parameter uncertainty × actuator tracking error.

Report the sampled frontier surface, unresolved cells and interaction effects. HORIZON (arXiv:2606.05143v1) reports non-monotonic domain-composition effects in curriculum training; that is a reason to test coupled combinations, not proof that physical robust-viability nesting itself can be non-monotone when the transition relation is genuinely nested.

## 6. Keep comparison margins separate

For each state/phase/individual, report independent vectors and curves:

- planar support-polygon / ZMP margin;
- 3D force-tilt margin;
- 3D moment-tilt margin;
- robust invariant-set reserve by uncertainty axis;
- guaranteed recovery reserve by axis and recovery target;
- number/identity of robustly admissible first actions;
- worst-case recovery horizon;
- viable-state coverage and abstraction inflation;
- Unknown/unresolved fraction.

A positive static margin is not a future viability proof. A brief negative static margin is not necessarily a fall: SPINDER reports brief negative tripod-exchange static margins that are dynamically stabilized. Dynamic geometric margins and the set-based viability calculation should explain such cases independently.

## 7. Matched baselines

Under matched model, state, contacts, disturbances, horizons and compute budgets, compare:

1. planar support-polygon/ZMP margins;
2. Ashtekar & Dutta (2026) 3D force-tilt and moment-tilt margins;
3. STEP-CBF+ dynamic margin / barrier screening;
4. robust MPC / impact-aware robust convex MPC;
5. hybrid invariant tubes;
6. finite oracle without the reserve ladder;
7. proposed axiswise viability-reserve curves.

Sources:
- Ashtekar & Dutta (Mechanism and Machine Theory, 15 Oct 2026): https://doi.org/10.1016/j.mechmachtheory.2026.106551
- Djeffal et al., STEP-CBF+ (Scientific Reports, Sept 2026): https://doi.org/10.1038/s41598-026-72507-w
- Impact-aware robust convex MPC (IEEE RA-L 2026): https://doi.org/10.1109/LRA.2026.3692086
- Hybrid invariant tubes (2026): https://arxiv.org/abs/2604.05108
- SPINDER hexapod baseline (Scientific Reports 2026): https://doi.org/10.1038/s41598-026-56856-0

The STEP-CBF+ figures (4.0% mean fall rate, 77.3% stable-step fraction, ~13.6 ms selection) are from a reduced-order randomized simulation, not a whole-body invariant proof or hardware validation. Preserve that boundary.

## 8. Frozen finite corpus

- all-viable and all-nonviable ladders;
- single monotone first-loss bracket;
- multiple false-to-true rebounds;
- strict level ordering, duplicate levels and malformed identities;
- separate invariance and recovery outcomes;
- model/action/safe-set digest mismatch across purportedly same ladder;
- one-axis nested successor sets;
- deliberate non-nested family as a negative control;
- two-axis slices with non-monotonic composition effects;
- static support margin and dynamic viability disagreement at tripod exchange.

## 9. Scorecard

Report separately:
- independent oracle agreement;
- nested-set containment checks;
- number and location of monotonicity violations;
- bracket width and left/right censoring;
- invariant reserve vs recovery reserve;
- first rejected action and violating successor;
- worst-case recovery horizon;
- robust admissible action count;
- viable-state coverage and abstraction inflation;
- runtime, memory and unresolved/Unknown fraction;
- geometric margins by method and phase.

Do not collapse these into a weighted score.

## 10. Claim boundary

The reference auditor qualifies only bracket calculations over supplied finite outcomes. It cannot prove the physical uncertainty family is nested, compute the viability kernel, validate the abstraction, or establish real-robot safety. Every hardware claim still requires validated model bounds and platform-specific sensor, actuator, timing and contact evidence.
