# ANIMA Decision and Action V0

Status: normative Living World integration contract. Documentation only.

## Purpose

Define how embodied agents transform regulation state, bounded belief, body capability, relationship history, perceived affordances, and external requests into canonical intents and interruptible actions.

## Parent contracts

This contract refines `ANIMAL_HOMEOSTATIC_BEHAVIOR_V0.md` and `ANIMAL_LOCOMOTION_CONTACT_V0.md`. It does not replace species policy, physics/contact authority, ecological settlement, or robotics safety/control.

## Core invariant

> Requests are inputs to appraisal, not motor authority; animation is expression, not decision authority.

Conceptually:

`regulation + belief + body + relationship + perceived affordances -> appraisal -> candidate intents -> arbitration -> committed action -> skill/control -> physical/ecological outcome`

## Appraisal

Appraisal may convert authoritative biological or synthetic regulation state into bounded dimensions such as urgency, risk, controllability, novelty, expected effort, information need, and social relevance.

Persistent temperament/prior parameters remain distinct from temporary appraisal state.

No single scalar is claimed to be a biologically complete emotion or utility measure.

## Body-relative affordances

An affordance is a relation among:

- this body/capability state;
- the environment the agent has evidence for;
- current goals/regulation;
- uncertainty/freshness.

The same observed object may provide different actions to horse, dog, deer, humanoid, quadruped robot, or manipulator.

Affordances must not be generated from unrestricted hidden world truth.

A candidate may expose bounded feasibility, confidence, risk, cost, duration, information gain, required capabilities, evidence references, and alternatives.

## Relationship expectations

Partner-specific history may influence interpretation/appraisal through learned expectations. A request from a familiar reliable partner need not be equivalent to the same observable request from an unfamiliar partner.

Relationship state does not grant mind-reading.

## Request disposition

Where external requests exist, the decision boundary should distinguish outcomes equivalent to:

- accept;
- hesitate;
- seek information;
- choose alternative;
- refuse;
- unable.

`Unable` represents capability/constraint failure rather than motivational refusal.

## Deterministic arbitration

Canonical intent selection is deterministic for equivalent authoritative state and policy version, or uses an explicit keyed stochastic scheme.

Candidate container/thread order cannot become implicit policy.

Tie-breaking and policy identity are versioned when they change future behavior.

A committed intent references enough evidence/appraisal/affordance state for qualification builds to explain the decision.

Arbitration does not itself settle physical/ecological effects.

## Epistemic action

Information-seeking is a first-class action family when uncertainty changes behavior.

Examples include orienting, sniffing/sampling, testing footing, moving viewpoint, inspecting, listening/waiting, probing, or requesting another agent's observation.

Information seeking consumes canonical time and may consume energy or incur risk. It can fail.

## Self-generated activity

Agents may generate low-stakes intentions without the player, including feeding, drinking, resting, grooming, seeking shelter/shade, maintaining group proximity, exploring, playing/socializing, patrolling, recharging, or self-diagnosing as appropriate.

These are behavior-policy outputs, not random presentation idles.

## Multi-rate domains

Physics/contact, receptor sampling, reactive control, cognition, ecology, and presentation may run at different rates.

Every canonical sample/decision carries authoritative simulation time provenance and explicit freshness semantics.

Wall-clock time, renderer FPS, and animation frame count are not behavior authority unless a separately declared real-time mode assigns that role.

## Action lifecycle

Canonical action state distinguishes at least the semantics of:

- proposed;
- committed;
- executing;
- suspended/waiting where applicable;
- interrupted;
- completed;
- failed.

Transitions carry reason/evidence references. Animation completion cannot independently complete a canonical action.

## Fast control vs slow cognition

High-level cognition chooses meaningful goals/actions. Fast embodied control handles bounded balance, collision response, foot placement, grasp stabilization, and comparable reflex/controller work.

For synthetic agents, high-level reasoning does not directly own raw actuator torque. Existing platform-specific planners and safety authorities remain downstream control boundaries.

Controller constraints/vetoes must be observable upstream when they affect future planning; they must not silently masquerade as successful action.

## Interruption

New evidence, dominant regulation pressure, lost affordance, or controller safety outcome may request an explicit action interruption.

Interruption cannot duplicate already-settled resources/effects. Save/reload and fidelity transitions preserve exactly-once semantics for committed actions.

## Qualification direction

At minimum test:

1. same request + different qualified body/terrain/history -> explainable disposition difference;
2. missing capability produces inability rather than random refusal;
3. reduced evidence confidence can select information-seeking rather than hidden-world refresh;
4. identical perceived geometry + different bodies -> different affordance sets;
5. partner-specific history can alter response without mind-reading;
6. candidate reordering cannot change deterministic winner;
7. same state/evidence/policy -> replay-identical decision trace where exact semantics are claimed;
8. player absence still yields coherent self-generated behavior;
9. renderer/FPS changes cannot alter canonical arbitration;
10. interrupting an executing action preserves exactly-once effects;
11. stale sensor input remains stale rather than being silently refreshed;
12. robotics safety/controller veto can trigger explicit replan.

## Non-goals

This contract does not prescribe one behavior tree, utility system, active-inference formulation, reinforcement-learning architecture, animation system, or universal psychology.
