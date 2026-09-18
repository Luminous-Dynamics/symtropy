# ANIMA Learning V0

Status: normative Living World integration contract. Documentation only.

## Purpose

Define what ANIMA may learn online during canonical execution so biological and synthetic agents can adapt from experience without unrestricted runtime policy drift, hidden-world learning, or bypass of safety/authority boundaries.

## Core distinctions

ANIMA separates:

1. memory formation/revision;
2. bounded associative learning;
3. bounded parameter adaptation;
4. policy/controller learning.

These are not equivalent authorities.

## V0 rule

ANIMA V0 permits transparent bounded learning and does not permit unconstrained online policy optimization as canonical gameplay authority.

Prefer:

```text
qualified experience
    -> versioned update rule
    -> bounded state delta
    -> auditable learned state
```

over opaque online adaptation whose causal inputs and limits cannot be reconstructed.

## Admissible learning sources

A learned update may derive only from experience already admissible under ANIMA epistemic authority, such as:

- qualified percepts;
- bodily/action outcomes;
- social observations received through legitimate channels;
- communication;
- explicit inference with retained evidence provenance.

Hidden simulator truth is not a learning source.

## Update provenance

Every future-bearing learned update retains enough information to identify:

- triggering experience/evidence;
- canonical tick/event;
- update-policy identity;
- prior state/value;
- resulting state/value;
- partner/object/place identity where required;
- authority epoch where networked;
- consolidation/decay class where relevant.

Learning cannot be represented solely as an unexplained mutation such as `trust += delta`.

## Bounded adaptive variables

Each online-adaptive variable declares as appropriate:

- semantic type;
- valid range;
- maximum per-update delta;
- maximum update rate over canonical time;
- saturation behavior;
- decay/recovery semantics;
- contradiction/reversal policy;
- evidence/confidence threshold;
- generalization policy;
- profile-dependent bounds;
- migration/version identity.

Bounded dimensionless values follow ANIMA quantitative semantics.

## Habituation and sensitization

Habituation is not free fear decay.

Repeated benign exposure can lower novelty/threat expectation only when the agent actually perceived the cue and qualified outcomes support the update.

A strong adverse outcome may reverse or strengthen the learned association according to versioned policy.

Unperceived repeated world events do not teach the agent.

## Partner-specific learned communication

A learned convention may retain:

- producer/receiver or partner identity;
- cue representation;
- candidate meaning/action association;
- confidence/strength;
- supporting experience references;
- decay/generalization semantics.

A cue learned with one person/animal does not become species-wide knowledge automatically.

## Generalization

Learning may generalize from one cue/place/object to similar cases only through an explicit policy.

Similarity, representation, radius/threshold, confidence transfer, and source provenance are versioned semantics.

An HDC nearest neighbor or vector similarity result is a retrieval candidate, not automatic semantic truth.

## Memory vs learned policy

Memory can affect future appraisal without modifying the underlying policy algorithm.

Examples:

- remembered slip -> changed bridge expectation;
- repeated safe handling -> changed partner predictability;
- learned whistle -> communication association.

Do not classify every memory-dependent decision as policy learning.

## Offline learned controllers/policies

Complex locomotion or manipulation may eventually use weights trained offline through RL, imitation, motion priors, or other learning.

For canonical runtime use:

- freeze model/weight identity for the qualified subject;
- constrain proposals through body capability and safety/controller authority;
- keep semantic cognition separate from opaque motor internals where possible;
- version any residual runtime adaptation;
- do not let a learned model silently gain hidden simulator inputs.

## Synthetic safety boundary

No learned component may bypass:

- body/capability checks;
- affordance feasibility;
- hard physical safety constraints;
- platform controller veto;
- authorization/authority constraints;
- stale/missing telemetry semantics;
- canonical action settlement.

A learned output remains a proposal wherever architecture says it is a proposal.

## Idempotency

The same committed experience/update event identity cannot apply the same learned delta twice through:

- journal replay;
- save/reload;
- network retransmission;
- rollback;
- authority handoff.

Exactly-once semantics apply to future-bearing learning updates.

## Catastrophic drift protection

Repeated/adversarial interactions cannot drive adaptive state outside declared bounds or erase unrelated capabilities unless an explicit modeled process owns that effect.

Examples include whistle spam, repeated harmless novelty, duplicate network events, and save/replay loops.

## Forgetting and consolidation

Learning composes with ANIMA memory rules.

Learned state may decay, consolidate, merge, or remain episodic according to explicit policy.

Canonical forgetting is not representation demotion and cannot be triggered merely because an entity went offscreen.

## Fidelity and persistence

Any learned variable that changes future behavior is future-bearing process information.

Save/reload, coarse simulation, fidelity transitions, and authority transfer preserve it or reject an insufficient transition.

## Qualification direction

At minimum test:

1. perceived qualified exposure changes declared learned state while hidden/unperceived control does not;
2. updates respect per-event/rate/range bounds;
3. benign and adverse outcomes revise associations in policy-appropriate directions;
4. partner-specific cues remain partner-specific absent explicit generalization;
5. similar/dissimilar cue matrices obey the declared generalization policy;
6. same experience ledger + policy identity -> exact learned state;
7. duplicate event identity cannot apply an update twice;
8. save/reload preserves learning exactly where claimed;
9. authority handoff accepts exactly one learning commit;
10. coarse/offscreen representation preserves future-relevant learning;
11. optional HDC/FEP backends cannot create a semantic update unsupported by admissible experience/policy;
12. learned proposals cannot violate hard body/safety/authority constraints;
13. repeated benign exposure can produce bounded habituation and adverse experience can restore/increase sensitivity according to policy;
14. Observatory counterfactual probes cannot mutate canonical biography.

## Non-goals

No unrestricted online RL in V0, self-modifying code, hidden-world learning, global species learning from one individual's experience, HDC similarity as automatic truth, or learned bypass of controller/safety/authority constraints is introduced here.

Relates to #1186, #1187, #1189, #1194, #1195, #1197, #1198, #1199, #1200, #1183 and #170.
