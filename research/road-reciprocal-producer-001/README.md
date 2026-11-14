# Synthetic reciprocal policy producer v0.1

**Status:** implementation candidate; exact-head CI pending.  
**Composition:** `symtropy-road-actors` + `symtropy-road-actor-adapter`.  
**Handoff target:** ROAD-EGCI v0.2 event-order receipt in Symtropy #3886.

## What this does

The producer runs two separate actor-local policy instances. Each receives one allow-listed TTC cue from the other actor's source identity. The two cues are generated from one frozen pre-decision boundary, delivered separately, and admitted through one shared input cutoff. Both policies decide before either target pose is projected or latched. The producer then emits two path-to-pose receipts and a joint latch boundary.

A physics consumer executes those exact pose commands, captures the post-step snapshot, and calls `record_physics_commit` with that snapshot digest. The producer refuses to start another policy tick until that commit has been acknowledged. The next tick's observation-generation events therefore follow the prior physics event ordinal.

## What this does not do

- It does not own or duplicate the physics engine.
- It does not invent sensor observations; each step receives TTC values from the calling scenario/world model.
- The reciprocal-coupling formula used in tests is deliberately stylized and is not a validated TTC or human-actor model.
- It does not make a public-road, collision-risk, legal-compliance or SAE L4/L5 claim.
- A 64-hex digest is a format-checked content reference, not authenticated evidence. The physics producer and evidence anchor must authenticate it independently.

## Conformance tests

- Two actor policies have distinct cue recipients and source identities.
- A shared per-tick input cutoff precedes both decisions.
- Both decisions and path-to-pose projections precede both command latches.
- An acknowledged physics event must follow all latches.
- The next tick's observation generation must follow that physics event.
- The other actor's preceding motion can change the next delivered TTC and alter the ego/other policy responses.
- Asymmetric cues produce distinct joint actions.
- A late same-tick delivery is rejected.

The symmetric fixture also exposes a useful research finding: identical cautious reciprocal policies can alternate between continue and pause under a simple feedback law. This is not necessarily a bug in the implementation; it is an interaction mode that ROAD-DIR should measure (liveness, oscillation and deadlock) and that future priority/negotiation policies must address without hidden omniscient state.
