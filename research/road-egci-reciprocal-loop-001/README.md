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

## Observation and deadline fault controls

The producer's conformance tests now inject faults into the exported policy-observation boundary and verify them through the real #3886 SAM adapter:

- **Occluded / dropped / stale:** the source record has no delivery tick or delivery event sequence. If a decision still claims that cue in its visible history, export rejects with `CueNotDelivered`.
- **Delayed:** a cue recorded as arriving in a later tick cannot remain visible to an earlier decision. Export rejects with `CueNotDelivered`.
- **Late command:** a command latched after the policy's declared deadline cannot retain `Completed`; export rejects with `InvalidDecisionReceipt`.

These are fail-closed fault-injection controls, not yet a simulated stochastic dropout/delay campaign. The next producer tranche should execute explicit observation schedules (0/1/2-tick delay, dropout bursts, occlusion windows, stale-threshold sweeps) and deadline-miss outcomes as distinct branch conditions, with every changed input reflected in the policy history and branch receipt. A missing or timed-out decision must censor the downstream joint path; it must never be replaced by a fresh-state or inferred action.

## Evidence and limits

The producer constructs the v0.2 episode export, computes policy-history and joint-action commitments, binds each transition to a derived per-step commitment that includes the global replay receipt, input-tape digest, tick index and child snapshot, verifies snapshots in fresh worlds, and exports the unchanged four-field SAM #10514 query shape.

The declared candidate universe is intentionally marked incomplete. Consequently, the SAM query remains `complete=false`: this is protocol conformance, not an empirical support claim or a complete counterfactual candidate campaign. The correspondence is a scenario-authored identity fixture, not an externally authenticated mapping.

## Claim ceiling

This demonstrates a finite, deterministic, synthetic two-sided policy loop for the named policies, route, observations and physics build. It does not validate human pedestrian behavior, road realism, sensor fidelity, empirical traffic response, collision-risk reduction, safety, production driving or SAE L4/L5 capability. Fresh-world same-runtime replay is tested; fresh-process reconstruction and portable cross-machine equivalence remain later gates.


## Observation delivery and reaction maturation reference

The standalone \`observation_buffer\` module is the first executable slice for ROAD-RECIPROCAL-DELAY-001 (#4085). It keeps three clocks separate:

- source observation age (\`decision_tick - observed_tick\`);
- transport/delivery age (\`decision_tick - delivered_tick\`);
- policy reaction maturity (\`decision_tick - delivered_tick >= reaction_delay_ticks\`).

A bounded recipient-local queue retains cue identity, source event, observation and generation ticks, generation and delivery ordinals, availability status and optional payload. Selection is constrained by the decision's closed input-event cutoff. A newer immature cue does not overwrite an older mature cue. Source-age and delivery-age expiry are independent. Queue overflow rejects without evicting an existing cue. Missing, occluded, unsupported and dropped records remain unavailable rather than becoming zero or fresh observations.

This component is intentionally a pure timing/evidence primitive. It does **not** change the green #3929 v0.1 actor policy or physics loop yet. The first qualification step is to prove its zero-delay behavior and hostile queue/timing cases; a subsequent composition will bind the selected matured cue into the actual actor policy input and branch receipts.

The finite module tests cover zero-delay immediate selection, exact reaction boundary, no starvation under a newer immature cue, independent age limits, unavailable statuses, same-tick cutoff, duplicate/non-monotonic events, overflow/no eviction, and delivery timing rejection. Passing these fixtures does not establish human reaction-time distributions, sensor performance or road safety.
