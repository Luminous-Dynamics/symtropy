# ROAD-RECIPROCAL-LOOP-001 — two-sided online-policy producer v0.1

**Status:** implementation candidate; exact-head CI is required before any protocol-conformance claim. The workflow is loaded from the stack base branch.  
**Verified substrate:** actor policy core #3489, path adapter #3748, kinematic-pose replay #3892, phase-aware handoff #3886.

## Executed protocol

A bounded four-tick synthetic crossing with one ego kinematic body and one synthetic crossing actor. The actor uses the existing `CautiousPausePolicy`; its route projection uses `symtropy-road-actor-adapter`; both agents move through the existing `symtropy-physics` `SetKinematicPose` replay command. The producer carries the v0.2 SAM handoff crate as a local path dependency and calls its canonical branch-root function rather than maintaining a duplicate implementation.

Each tick follows:

`independent delivered views -> independent policy transitions -> all decisions -> command latches -> physics -> realized state -> later observations`

The ego reference policy is a transparent zone-yield rule. It sees only the actor-zone cue and its own speed-limit cue. The actor policy sees only its delivered ego-TTC cue. Neither policy receives the other policy's hidden state or same-tick command. Actor cue provenance is keyed to the event ordinal that generated the observation.

## Reciprocal feedback experiment

Two branches share the exact initial world and policy states. The intervention changes only the ego-visible speed-limit cue (1 m/s versus 2 m/s).

- Tick 0: both policies choose the same actions before the ego intervention's physical consequence is observable.
- Tick 1: the actor receives TTC from the post-physics ego state. The 1 m/s branch continues; the 2 m/s policy pauses, but its body decelerates rather than teleporting to rest.
- Tick 2: the ego still sees both actors outside the declared 8 cm conflict zone. The actor continues in the 1 m/s branch and remains in its bounded deceleration profile in the 2 m/s branch. Their realized positions now differ across the zone boundary.
- Tick 3: ego receives actor-zone observations generated after tick-2 physics. It yields in the 1 m/s branch and continues in the 2 m/s branch.

This four-tick setup makes actor-to-ego feedback observable only after the actor's policy response has changed its realized position. It avoids the earlier fixture error where the ego could see the actor's common initial movement before the branches had actually separated.

The tests assert both feedback directions, separate policy-state chains, no same-tick cue visibility, exact event ordering, fresh-world replay, and rejection of a cue delivered at the already-closed tick cutoff.

## Evidence and limits

The producer constructs the v0.2 episode export, computes policy-history and joint-action commitments, binds each transition to a derived per-step commitment that includes the global replay receipt, input-tape digest, tick index and child snapshot, verifies snapshots in fresh worlds, and exports the unchanged four-field SAM #10514 query shape.

The declared candidate universe is intentionally marked incomplete. Consequently, the SAM query remains `complete=false`: this is protocol conformance, not an empirical support claim or a complete counterfactual candidate campaign. The correspondence is a scenario-authored identity fixture, not an externally authenticated mapping.

## Claim ceiling

This demonstrates a finite, deterministic, synthetic two-sided policy loop for the named policies, route, observations and physics build. It does not validate human pedestrian behavior, road realism, sensor fidelity, empirical traffic response, collision-risk reduction, safety, production driving or SAE L4/L5 capability. Fresh-world same-runtime replay is tested; fresh-process reconstruction and portable cross-machine equivalence remain later gates.
