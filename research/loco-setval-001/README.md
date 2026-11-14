# LOCO-SETVAL-001 — Set-valued hybrid locomotion abstraction

Status: research specification, not an implemented solver or physical safety certificate.

Issue: https://github.com/Luminous-Dynamics/symtropy/issues/4083

Axiswise uncertainty reserve definition: [#4130](https://github.com/Luminous-Dynamics/symtropy/issues/4130); finite ladder bracket auditor: [#4132 / PR #4133](https://github.com/Luminous-Dynamics/symtropy/pull/4133). Matched protocol: `VIABILITY_RESERVE_PROTOCOL.md`.

Industry locomotion-performance crosswalk: `ISO18646-5-2026-CROSSWALK.md` (performance comparison only; ISO 18646-5 does not validate safety requirements). Graded multi-channel contact reliability reference: [#4143](https://github.com/Luminous-Dynamics/symtropy/issues/4143).

Exact rational interval/event-order fixtures are tracked separately in [LOCO-HYBRID-MATH-001 / #4113](https://github.com/Luminous-Dynamics/symtropy/issues/4113), with implementation in [PR #4107](https://github.com/Luminous-Dynamics/symtropy/pull/4107).

## Purpose

Bridge the finite nondeterministic robust-viability oracle in `loco-robust-viability-v0` to continuous and hybrid locomotor models without silently omitting admissible physical outcomes.

The central assurance obligation is **sound abstraction**: every admissible concrete successor represented by the declared hybrid model must be contained in at least one abstract successor. A small-state oracle can be exact while its physical interpretation is unsound if the transition graph is incomplete.

## Ownership boundaries

- #4058/#4061 own finite-state viability and recovery semantics.
- #4083 owns set-valued hybrid model semantics and abstraction soundness.
- #3988 supplies individual-dynamics hypotheses and parameter uncertainty.
- #4004 owns contact information and active sensing.
- #3183 owns contact-law assurance.
- #3403/#3393 and PHYS work own numerical/event convergence.
- #4044 owns energy measurement and accounting.

Do not duplicate contact solvers, energy ledgers, controllers, or the finite-state oracle.

## Hybrid model

A declared model contains:

- continuous state domain (X);
- discrete contact/gait mode (q);
- parameter uncertainty set \(\Theta\);
- disturbance set \(W\);
- mode-specific flow relation \(\dot{x}\in F_q(x,\theta,w)\);
- guard sets \(G_{q\rightarrow q'}\);
- reset relations \(R_{q\rightarrow q'}\);
- separate observation-delay, actuation-delay, dropout and stale-data sets.

The reachable successor must include all admissible continuous flow segments, every guard intersection, all reset images, and all consistent event orders when guard-time intervals overlap. Unresolved event ordering is branching uncertainty, not permission to select a nominal order.

## Abstraction obligation

For an abstract cell \(C\), every admissible concrete post-event state reachable from any \(x\in C\), under the declared parameter, disturbance, delay and event-order sets, must be enclosed by the emitted abstract successor cells.

Soundness is mandatory. Precision is measured separately through enclosure inflation and lost viable-state coverage.

The result state is one of:

- `Certified`: all declared enclosure and coverage obligations discharged for this model/horizon.
- `Unknown`: resource budget, numerical convergence, event coverage or model evidence is insufficient.
- `Invalid`: malformed or contradictory model input.

No partial result may be labelled `Certified`.

## Required evidence attached to every result

- model and parameter-set digests;
- state-space and mode schema versions;
- numerical method and implementation version;
- outward-rounding/interval policy and tolerances;
- horizon and event-order policy;
- guard and reset coverage;
- explored cells/transitions and resource consumption;
- enclosure inflation and unresolved-boundary counts;
- deterministic canonical output digest;
- explicit claim ceiling and assumptions.

## V0/V1 staged implementation

1. Exact-rational affine-flow, guard and reset fixtures with independently computed reachable sets.
2. Typed interval/box and hybrid-mode representation.
3. Independently qualified outward-rounded interval arithmetic (or a separately qualified dependency); ordinary floating-point min/max is not a proof.
4. Guard intersections and reset-image enclosures.
5. Conservative finite transition export to the #4061 oracle.
6. Phase-conditioned cycle tubes and individual parameter sets.
7. Observation and actuation delay branches from #4004, distinct from physical disturbances.

## Frozen adversarial tests

- exact affine flow and reset maps;
- uncertain parameter extrema;
- guard crossing at interior, endpoint and tangency;
- overlapping guard-time intervals retaining both event orders;
- event-time uncertainty straddling a control sampling boundary;
- contact loss versus delayed/missing contact observation;
- bounded actuator tracking error and saturation;
- parameter drift across cycles;
- malformed/non-finite/reversed intervals;
- overflow and resource exhaustion;
- permutation-invariant canonical digests;
- monotonic reachable enclosure under parameter/disturbance widening;
- no certified result when any guard or reset branch is unresolved;
- independent exact-rational reference oracle;
- abstraction refinement reduces inflation without losing concrete successors.

## Qualification scorecard

Keep separate:

1. sound enclosure containment;
2. false-safe count;
3. guard/event coverage;
4. reset-image enclosure error;
5. unresolved/unknown rate and reason;
6. reachable-set inflation;
7. viable-state coverage;
8. abstraction cells, runtime, memory and deadline misses;
9. deterministic digest/replay agreement.

Target zero false-safe results on the frozen corpus and 100% containment on exact/reference fixtures. These are proposed acceptance targets, not achieved results.

## Matched baselines

Compare at matched model, horizon, uncertainty set, state dimension and compute budget:

- nominal hybrid simulation;
- covariance/saltation stochastic NMPC;
- CBF/QP and STEP-CBF+-style screening;
- robust min-max / impact-aware convex MPC;
- interval reachable tubes and hybrid invariant-set methods;
- finite oracle alone versus sound abstraction plus oracle.

Primary research metric: sound viable-state coverage at fixed compute budget, reported with false-safe count, unknown fraction and enclosure inflation. Never collapse these into one reward.

## Claim ceiling

A passing exact fixture qualifies only that fixture. A sound abstraction establishes a conditional result for the declared mathematical model. It does not establish that the model bounds a physical robot. Hardware claims require independently measured bounds for contact, terrain, actuators, sensors, timing, mass and compute deadlines.

## Frontier references

- Madabushi et al., differentiable invariant sets for hybrid limit cycles (CDC 2026, to appear): https://arxiv.org/abs/2604.05108
- Katayama et al., stochastic/robust NMPC with guard saltation matrices (IROS 2024): https://www.sony.com/en/SonyInfo/technology/publications/robustifying-model-based-locomotion-by-zero-order-stochastic-nonlinear-model-predictive-control-with-guard-saltation-matrix/
- Impact-aware robust convex MPC (IEEE RA-L 2026): https://doi.org/10.1109/LRA.2026.3692086
- Contact sensing and active haptic exploration survey (2026): https://doi.org/10.1007/s10462-026-11644-0
- Actuator-grounded sim-to-real and energy modeling (IJRR 2026): https://doi.org/10.1177/02783649261459628
