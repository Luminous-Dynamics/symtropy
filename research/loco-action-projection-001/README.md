# LOCO-ACTION-PROJECTION-001 — Robust policy-to-dynamics projection with viability-gated execution

**Status:** protocol and synthetic known-answer fixtures. No real robot superiority or safety claim.

Issue: [#4382](https://github.com/Luminous-Dynamics/symtropy/issues/4382).

## Objective and ownership

A learned controller or high-level planner proposes an action. The proposal is not yet a motor command. A separate whole-body dynamics projection maps it to contact-consistent, actuator-executable variables. An independent robust viability/recovery authority then decides whether that exact projected command may be applied.

This workstream owns projection contract semantics, typed projection outcomes, residual/constraint evidence, identity binding and execution-gate composition. It does not replace:
- #4004/#4017/#4230 contact estimation and correlated evidence;
- #4058/#4061 robust viability/recovery;
- #4083 sound set-valued hybrid abstraction;
- #4119 state/model/actuator/timing certificate validity;
- #4044/#4179/#4206 energy and actuator response;
- #4304/#4308 coupled uncertainty/frontier;
- #4310/#4313 viability-conditioned information value;
- #4324/#4326 adverse event taxonomy/audit.

## Literature boundary: what already exists

**Policy Action Dynamics Projection (PADP).** Yan et al., IEEE Robotics and Automation Letters 11(11), November 2026; first published 16 September 2026. PADP maps joint-space policy outputs to generalized accelerations, contact wrenches and joint torques while satisfying rigid-body dynamics and contact conditions. A subsequent whole-body QP integrates manipulation objectives without retraining the reused locomotion policy. The authors report simulation and hardware experiments on point-foot and wheeled-foot quadruped manipulators. This establishes that policy-to-dynamics projection itself is not our novelty. DOI: https://doi.org/10.1109/LRA.2026.3734869

**Feasibility-aware whole-body control (FAWBC).** Zhang, Wu & Bräunl, *Autonomous Robots*, 28 September 2026. It estimates terrain geometry from joint encoders and an IMU, adapts NMPC friction constraints and modulates whole-body QP costs by a friction-cone feasibility margin. It reports physical Unitree G1 trials on flat ground, grassland, wood chips and a 15° ramp. The paper explicitly states that proprioceptive terrain inference is retrospective and cannot see discontinuous steps before contact. This is a strong baseline for contact feasibility adaptation, but not a general uncertainty-set or robust viability certificate. DOI: https://doi.org/10.1007/s10514-026-10263-6

**Kinetic Minimalism.** An 8-DoF wheeled quadruped with four torque-controlled hips and four velocity-driven wheels uses torque proprioception as terrain-interaction evidence and whole-body pitching/differential drive to compensate for missing steering and knee joints. The paper separates Isaac Sim Barkour evaluation from real hardware demonstrations of jumping, stairs, bracing and environment-aided interaction. This motivates actuator-as-sensor and morphology-aware tests. DOI: https://doi.org/10.1109/CASE69030.2026.11704153

The Symtropy contribution is the composition of **projection + model/actuator uncertainty + complete contact-mode coverage + independent post-projection viability + time-of-application assurance**. A nominally feasible QP must not certify itself.

## Paradigm shift

| Conventional architecture | Symtropy direction |
|---|---|
| Policy output becomes a command | Policy output is a typed proposal with provenance |
| Nominal inverse dynamics/WBC | Dynamics projection with set-valued contact/actuator uncertainty |
| QP solver says feasible | Solver status, residuals, per-constraint margin and coverage are separate |
| Nominal contact mode | Complete possible contact-mode set / explicit Unknown |
| Fixed friction coefficient | Calibrated interval or set-valued friction/contact model |
| Weighted reward trades task against safety | Lexicographic hard feasibility, independent viability, task retention, proposal proximity |
| Command torque is assumed delivered | Command-to-motion and delivered-torque evidence from #4206 |
| One optimization result certifies itself | Projection and independent physical admission are separate authorities |

## V0 contract

### 1. Proposal identity

Bind policy/model digest, exact input state digest, action-space convention, intended command, action timestamp, deadline, contact proposal and task intent. Keep the original proposal immutable. Any projected command must have a new digest and an explicit proposal-to-command mapping.

### 2. Projection result

A projection result binds:
- exact proposal digest and exact projected-command digest;
- a separately frozen, independently supplied physical/contact constraint inventory digest (the optimizer cannot define its own completeness);
- dynamics/abstraction/domain and actuator-model digests;
- solver implementation/version and termination reason;
- primal, dual and KKT residuals, explicit units and frozen tolerances;
- complete constraint-family inventory and per-constraint residual/margin;
- contact mode(s), friction and contact-kinematic assumptions;
- actuator position/velocity/torque/slew/current/voltage/temperature limits and derating;
- start/end/deadline timestamps and resource usage.

The solver status is one of Feasible, Infeasible (with witness), Unknown, DeadlineMiss or Invalid. A partial solution is never a feasible command.

### 3. Lexicographic priority

The normative optimization order is:
1. exact model/dynamics equalities and hard physical/actuator constraints;
2. robust contact feasibility across the complete declared physical possibility set;
3. preservation of independently admitted robust viability/recovery;
4. task intent/trajectory retention;
5. minimum deviation from the policy proposal;
6. energy/thermal efficiency only among solutions satisfying every previous tier.

No single weighted penalty may let task reward buy through a hard physical constraint. A mathematically feasible point estimate is not robustly feasible if a declared contact, friction or actuator possibility violates the constraints.

### 4. Independent execution gate

After projection:
1. validate proposal, state, model, abstraction, actuator, domain and clock identities;
2. require Feasible solver disposition, exact equality to the independently frozen constraint inventory, complete residual/margin unit coverage and all residuals within frozen tolerances;
3. require every hard physical and robust-contact constraint to pass with declared margins;
4. pass the exact projected-command digest and successor/uncertainty set to #4061/#4083;
5. require a fresh #4119 assurance result at the intended application time;
6. require independent viability/recovery admission and an available fallback for Unknown/deadline failure;
7. execute only the exact projected-command digest that was admitted.

The action-projection layer must not compute or sign its own viability result. Delivered actuator response after application is separately measured and fed into the next cycle; it cannot be inferred from the commanded torque.

## V0 reference scope

The Rust reference validates finite typed contracts and gate composition around supplied optimizer results. It is not a nonlinear QP/SQP/conic solver.

It will check:
- immutable proposal and projected-command identities;
- model/abstraction/domain/actuator/clock binding;
- solver disposition and exact residual/tolerance units;
- exact equality between optimizer-declared constraints and the independently frozen hard/contact constraint inventory;
- residuals, margins and per-constraint pass status;
- independent viability receipt bound to the exact projected-command digest;
- independent assurance receipt freshness and action/state identity;
- explicit Unknown/Infeasible/DeadlineMiss/Invalid handling;
- deterministic fail-closed disposition.

A continuous optimizer remains a replaceable implementation behind this contract and is compared against it.

The complete physical possibility inventory must be source-qualified and complete at the application horizon. Its receipt binds the exact stable possibility IDs, initial state, model, abstraction, domain, actuator, horizon and validity interval. The downstream projection gate must show one required hard/contact constraint evidence row for every constraint × possibility pair. A caller-supplied list or a single aggregate margin is not evidence that the possibility enumeration is complete. Numerical residual failure and physical margin failure remain separate dispositions.

After execution, the exact authorized command must be reconciled against measured actuator realization through #4206. A mismatched, delayed or Unknown realization blocks the next state-changing command until the physical outcome is reconciled.

### Physical possibility and margin completeness

The exact #4083 possibility-set identity is bound by a source-issued digest and stable possibility IDs. Every required physical constraint (HardPhysical and RobustContact) must carry one residual/margin evidence row for **every** possibility ID. A single aggregate friction or torque margin is insufficient. The independent frozen constraint inventory and the solver-declared inventory must match exactly, including units, tolerances and minimum required margins.

Residual tolerance and physical margin have different semantics:
- residual beyond numerical tolerance makes the projection numerically unqualified (Unknown/fallback);
- a qualified residual with a physical/contact margin below its declared minimum is a known rejection (or an independently admitted fallback);
- task/proximity/efficiency residuals remain visible but cannot weaken hard physical gates.

### Command realization is a second, post-execution decision

Projection authorization and actuator realization are distinct. After application, #4206 supplies a source-qualified realization receipt bound to the exact authorized command digest, pre-action state, model/domain and actuator identity, plus measured response and resulting-state evidence where available. A matched receipt closes the current execution transaction and supplies a new post-action state; it does **not** authorize the next motor command. The next action must be a new proposal with fresh state, viability, assurance and application-time evidence. A delivery mismatch is retained as a known deviation and must not be silently folded into the nominal state transition. Unknown/Invalid delivery supplies no trusted post-state, so the loop abstains pending reconciliation. This reference checks the receipt contract; it does not compute actuator physics or assert that a motor delivered torque from the command alone.

## Matched baselines

1. Direct unprojected policy action (sandbox-only negative control).
2. Fixed-weight whole-body QP / inverse dynamics.
3. PADP-style nominal dynamics projection.
4. FAWBC-style proprioceptive friction-feasibility adaptation.
5. Nominal CBF/QP shield.
6. Robust MPC with fixed WBC.
7. Set-valued contact projection without post-projection viability.
8. Full Symtropy: set-valued projection + independent viability/recovery + application-time assurance + actuator realization evidence.

All methods receive identical policy proposals, task/command sequence, robot, terrain, contact/actuator uncertainty, sensor timing, compute deadlines and test exposure. Solver failures and deadline misses are outcomes, not discarded samples.

## Frozen known-answer / falsification corpus

Fixtures are in [fixtures/known_answers_v0_1.json](fixtures/known_answers_v0_1.json):
1. nominal friction-feasible but one calibrated friction possibility infeasible;
2. multiple possible contact modes with no robust common projection;
3. QP says Feasible but primal/KKT residual exceeds tolerance;
4. high task reward cannot override a hard actuator limit;
5. projected command has no downstream robust recovery;
6. exact projected command digest differs from viability receipt;
7. deadline-missed projection cannot execute a stale command;
8. stale/mismatched application-time assurance receipt;
9. contact evidence is asynchronous, stale or replayed;
10. actuator delivery deviates from the command and invalidates subsequent model/assurance evidence;
11. missing contact mode from enumeration suppresses a complete result;
12. command proposal and model/configuration digest mismatch;
13. axis/contact ordering permutation preserves canonical result;
14. Unknown projection selects only a separately admitted fallback, not the partial solver output.

## Scorecard

Report independently:
- proposal-to-projection deviation by action dimension;
- primal/dual/KKT residuals and frozen tolerance status;
- hard-constraint pass/fail and minimum margin by family;
- contact/dynamics/actuator possibility-set completeness;
- robust feasibility fraction and downstream viability/recovery witness;
- application-time age, end-to-end latency and deadline misses;
- fallback receipt identity, freshness and exact fallback-command binding;
- command-to-delivered torque/motion residual and actuator derating;
- task tracking, recovery and held-out capability;
- energy at battery, DC bus, actuator and mechanical boundaries;
- all adverse events in #4326.

No aggregate weighted score can hide a hard constraint, adverse event or deadline miss.

## Proposed targets (not achieved)

- 100% exact agreement on finite projection/gating fixtures.
- Zero commands applied after Infeasible, Unknown, Invalid or deadline-missed projection.
- Zero hard physical constraints accepted outside frozen tolerance.
- Zero nominal-feasible / robust-infeasible commands admitted.
- Zero unqualified contact modes or actuator derating states silently omitted.
- Zero stale or mismatched viability/assurance receipts accepted.
- At matched task tracking, energy and compute budgets, lower projection deviation without worsening any adverse category or hard physical constraint.
- Stretch: demonstrate robust feasibility and task retention on held-out contact, terrain and actuator uncertainty across at least two morphologies.

## Claim ceiling

V0 qualifies evidence composition and execution-gate semantics over supplied optimizer outcomes. It does not validate a continuous optimizer, physical contact abstraction, actuator model, receipt authentication or real-world safety. PADP and FAWBC are external baselines; Symtropy superiority requires matched source-pinned simulation and hardware trials.
