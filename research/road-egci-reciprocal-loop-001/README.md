# ROAD-RECIPROCAL-LOOP-001 — two-sided online-policy producer v0.1

**Status:** implementation candidate; exact-head CI pending.  
**Verified substrate:** actor policy core #3489, path adapter #3748, kinematic-pose replay #3892, phase-aware handoff #3886.

## Executed protocol

A bounded three-tick synthetic crossing with one ego kinematic body and one synthetic crossing actor. The actor uses the existing `CautiousPausePolicy`; its route projection uses `symtropy-road-actor-adapter`; both agents move through the existing `symtropy-physics` `SetKinematicPose` replay command.

Each tick follows:

`independent delivered views -> independent policy transitions -> all decisions -> command latches -> physics -> realized state -> later observations`

The ego reference policy is a transparent zone-yield rule. It sees only the actor-zone cue and its own speed-limit cue. The actor policy sees only its delivered ego-TTC cue. Neither policy receives the other policy's hidden state or same-tick command.

## Reciprocal feedback experiment

Two branches share the exact initial world and policy states. The intervention changes only the ego-visible speed-limit cue (1 m/s versus 2 m/s).

- Tick 0: both policies make the same actor decision before the ego motion is realized; ego continues under its own speed-limit cue.
- Tick 1: the actor receives a TTC derived from the post-physics ego state. The 1 m/s branch continues; the 2 m/s branch pauses.
- Tick 2: ego receives an actor-zone cue derived from the post-physics actor position. The actor's different realized motion causes ego to continue in one branch and yield in the other.

The tests assert both feedback directions, separate policy-state chains, no same-tick cue visibility, exact event ordering, and rejection of a cue delivered at the already-closed tick cutoff.

## Evidence and limits

The producer constructs the v0.2 episode export, computes policy-history and joint-action commitments, binds each transition to a physics replay receipt, verifies snapshots in a fresh world, and exports the unchanged four-field SAM #10514 query shape.

The declared candidate universe is intentionally marked incomplete. Consequently, the SAM query remains `complete=false`: this is protocol conformance, not an empirical support claim or a complete counterfactual candidate campaign. The correspondence is a scenario-authored identity fixture, not an externally authenticated mapping.

## Claim ceiling

This demonstrates a finite, deterministic, synthetic two-sided policy loop for the named policies, route, observations and physics build. It does not validate human pedestrian behavior, road realism, sensor fidelity, empirical traffic response, collision-risk reduction, safety, production driving or SAE L4/L5 capability. Fresh-world same-runtime replay is tested; fresh-process reconstruction and portable cross-machine equivalence remain later gates.
