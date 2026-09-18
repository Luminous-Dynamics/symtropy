# ANIMA Persistence V0

Status: normative Living World integration contract. Documentation only.

## Purpose

Bind ANIMA future-bearing biography to Symtropy's existing snapshot, event-journal, deterministic-time, and stable-identity substrate so save/reload, crash recovery, unload/reload, and representation changes cannot silently turn one individual into another.

## Existing authority

This contract composes with the existing persistence/game-state layer, which already provides:

- fixed-step authoritative simulation ticks;
- stable serializable identities;
- typed causal event envelopes;
- append-only hash-chained journals;
- atomic snapshots;
- snapshot anchoring to an event-chain head;
- crash-tail recovery and schema checks.

ANIMA does not define a second save engine.

## Core invariant

> A persisted ANIMA state is sufficient only if continuing from it preserves every declared future-bearing semantic required by enabled processes.

Deserializing successfully is not evidence of semantic sufficiency.

## Uninterrupted-equivalence theorem

For an exact product/profile/policy subject and identical subsequent inputs:

```text
uninterrupted execution
```

and

```text
execution -> snapshot -> process restart -> verified restore -> continuation
```

must produce the same declared canonical ANIMA future wherever exact equivalence is claimed.

Toleranced equivalence requires an explicit narrower theorem.

## Future-bearing state census

Depending on enabled features, a persistent individual may require:

- stable agent identity;
- exact body/species/policy/profile identities;
- realized temperament/development state;
- physiology, injury and capability state;
- current attention/orientation if it changes the next canonical observation/action;
- memory and sufficient consolidation state;
- belief/expectation state including uncertainty/freshness;
- relationship state and learned communication conventions;
- current committed intent;
- action lifecycle/interruption state;
- unsettled exactly-once effect/reservation identity;
- canonical due-tick schedule state;
- future-relevant group/social identity;
- stochastic stream/counter state when stateless keyed draws are insufficient;
- fidelity/persistent-identity state required by Living World authority;
- backend/policy migration identity where optional HDC/FEP paths affect semantics.

Pure presentation state is not persisted merely because it exists.

## Snapshot identity

An ANIMA snapshot must bind enough identity to reject silent reinterpretation, including as applicable:

- persistence schema;
- simulation tick;
- scenario/content identity;
- policy/profile composition identity;
- biography/event head;
- stochastic-policy identity;
- memory/backend identity;
- authority epoch where networked.

Human-readable labels alone are not sufficient when content may drift.

## Event journaling

Journal canonical future-bearing transitions at a granularity sufficient for crash recovery, replay, and evidence without logging every high-rate sensor/controller sample.

Candidate event families include:

- experience formation/revision;
- relationship update;
- injury/capability change;
- learned association update;
- intent commit/interruption/completion/failure;
- exactly-once ecological/resource settlement;
- fidelity transition;
- development transition;
- explicit profile/policy migration;
- multiplayer authority handoff.

Event payloads retain causal/source identity appropriate to their theorem.

## Snapshot + journal recovery

Recovery follows this shape:

1. load/schema-check a complete snapshot;
2. verify its event-head anchor against the recovered journal;
3. verify product/profile/policy compatibility;
4. replay committed post-snapshot ANIMA transitions as required;
5. preserve crash-tail handling only for an incomplete final record under the existing persistence theorem;
6. reject malformed complete records or missing anchors;
7. resume canonical due-tick/action state exactly once.

Current world truth is never used to fabricate historical observations missed before the snapshot.

## Active-action persistence

Saving may occur while an agent is:

- testing footing;
- tracking/casting for scent;
- fleeing;
- jumping;
- communicating;
- inspecting/manipulating;
- waiting for controller confirmation.

The implementation must either persist enough action/skill state for exact continuation or define a deterministic safe restart boundary whose semantic effect is separately qualified.

It must not duplicate or lose already-settled effects.

## Scheduler continuity

A snapshot persists enough due-tick state to distinguish saving immediately before, at, or after a sensor/attention/cognition/action deadline.

Restore cannot execute a canonical event twice or skip one because wall-clock time passed while the process was stopped.

## Memory persistence

A transparent reference memory backend establishes the clearest baseline first.

Optional HDC/vector memory may persist exact encoded state with a backend fingerprint or rebuild from admissible source experiences, but the choice is versioned and differentially tested.

A backend migration cannot create, erase, or strengthen future-bearing memories silently.

## Profile/content migration

Loading biography under incompatible policy/profile semantics requires one of:

- exact compatible interpretation;
- explicit deterministic migration with provenance;
- refusal.

Never silently reroll temperament, discard relationships, reinterpret confidence scales, change learning policy, or invent missing biography.

## Fidelity continuity

Representation demotion is not forgetting.

If a memory, relationship, injury, active action, learned convention, or fault history changes future behavior, the destination representation must preserve an information-sufficient closure or the transition is rejected/enriched under Living World authority.

## Multiplayer continuity

Authority handoff uses a verified biography/snapshot/event head rather than two peers merging independent minds.

A receiving authority must verify schema/profile/authority identity before committing later ANIMA events.

## Qualification direction

At minimum test:

1. save/reload at many canonical ticks matches uninterrupted control;
2. save before/during/after an interruptible action produces no duplicate/missing settlement;
3. save adjacent to a cognition/sensor due tick executes it exactly once;
4. memories, relationships, injuries and learned cues remain behaviorally effective after load;
5. unknown/stale state remains unknown/stale after serialization;
6. truncated final journal record follows existing crash-tail semantics without half-applying an ANIMA transition;
7. missing/tampered snapshot event anchor fails closed;
8. incompatible profile/policy identity refuses or explicitly migrates;
9. save while coarse/persistent identity is active preserves later promotion behavior;
10. stochastic continuation matches the uninterrupted path;
11. optional HDC persistence is differentially qualified against the reference corpus;
12. headless save/recovery requires no presentation resources.

## Non-goals

No second persistence engine, animation-pose archive, save/load-as-forgetting, silent profile migration, hidden-world history reconstruction, or unbounded journal of every raw percept/controller sample is introduced here.

Relates to #1186, #1188, #1193, #1194, #1195, #1197, #1198, #1199 and #170.
