# ANIMA Network Authority V0

Status: normative Living World integration contract. Documentation only.

## Purpose

Define how one persistent embodied agent remains one coherent canonical individual across multiplayer/P2P replication and authority transfer.

## Existing networking substrate

This contract composes with Symtropy networking rather than replacing it. Existing infrastructure already provides spatial physics authority, replicated-state classifications, explicit network authority markers, and Lightyear prediction/rollback/interpolation for networked ECS state.

ANIMA adds a distinct theorem for cognition and biography write authority.

## Core invariant

> At every canonical tick, exactly one authority may commit future-bearing ANIMA cognition/biography state for a persistent individual.

Replication does not imply multi-writer authority.

## Authority domains

Different systems may have distinct owners:

- physical-body simulation authority;
- ANIMA cognition/biography authority;
- high-rate robot controller authority;
- hard safety/emergency authority;
- unrelated consensus/governance authority.

V0 should prefer body and cognition authority co-location for ordinary animals where practical, but the relationship is explicit rather than assumed.

## Committed-state replication

Peers may receive selected committed state needed for gameplay/presentation, including as required:

- stable agent/profile identity;
- canonical physical state;
- public action/expression state;
- injury/capability state;
- public emissions/signals;
- authority epoch/head identity;
- selected ledger/event commitments;
- compact future-relevant biography summary where needed.

Do not broadcast every private memory, HDC vector, raw percept, candidate intent, attention sample, or belief intermediate by default.

Full biography transfers only where required for persistence or a new canonical authority.

## Authority epoch and fencing

Authority transfer needs a monotonic fencing identity equivalent to:

```text
agent identity
+ authority epoch
+ authority peer/process
+ effective canonical tick
+ prior biography/event head
```

Every future-bearing commit must be valid under the active epoch.

Events from a superseded authority epoch are rejected even if they arrive late.

## Handoff sequence

A canonical handoff should establish approximately:

1. old authority reaches a declared action/scheduler boundary;
2. old authority commits a biography/event head and active-action state;
3. exact required state, profile/policy identity, due-tick state and post-snapshot events transfer;
4. receiving authority verifies lineage/schema/epoch;
5. one declared tick activates the new authority;
6. prior authority is fenced from subsequent commits;
7. replication advertises the new authority mapping.

Do not merge competing biographies using last-writer-wins.

## In-flight actions

Authority transfer during a jump, scent cast, escape, manipulation, footing test, or other interruptible action must preserve exactly-once action settlement.

Either transfer at a qualified quiescent boundary or serialize sufficient active action/skill state for continuation.

## Perception boundary

A receiving authority inherits prior memory/belief as prior state. It cannot inspect its current local simulator truth and convert that into historical evidence the agent supposedly perceived earlier.

New cognition receives only legitimate current observations under the ANIMA sensory boundary.

## Prediction boundary

Client prediction may predict physical state for responsiveness where the network layer permits it.

Predicted/interpolated client state cannot independently create canonical:

- memories;
- learned associations;
- relationship updates;
- injuries;
- stochastic decisions;
- action settlement;
- biological/social evidence.

Any such future-bearing effect is recorded only after the appropriate canonical authority confirms/settles it.

## Remote social observation

Another agent may learn from a networked agent only through replicated public expression/emissions that legitimately reach the observer's receptors.

Private remote intent/belief is not directly available as social evidence.

## Fidelity

Coarse/offscreen simulation remains single-authority.

Authority placement does not relax Living World information-sufficiency rules. Promotion cannot reconstruct biography omitted by an insufficient coarse network representation.

## Synthetic hardware

For a Symthaea robot, cognition authority, simulator authority, hardware controller authority, and emergency safety authority may differ.

The high-rate platform safety/controller remains able to constrain or veto motor requests. Network loss or stale telemetry surfaces as missing/stale evidence and controller outcome, not assumed success.

## Qualification direction

At minimum test:

1. simultaneous two-peer biography commit -> exactly one accepted authority;
2. stale-epoch commits fail deterministically;
3. handoff adjacent to sensor/cognition due ticks preserves the no-transfer control future where exact equivalence is claimed;
4. handoff during an active action produces no duplicate/missing settlement;
5. stochastic continuation survives handoff;
6. learned cues, relationships and memories survive with exact provenance;
7. changing hidden non-authority peer state cannot affect canonical cognition;
8. client prediction/rollback cannot duplicate learning or experience formation;
9. remote social learning requires a legitimate replicated emission/percept path;
10. reconnect/takeover contains no two-active-writer interval;
11. fidelity/Level-I identity survives authority transfer;
12. ordinary replication does not require broadcasting complete internal cognition each tick.

## Relationship to generic state authority

A state being replicated does not mean all replicas may author it.

ANIMA write authority is an additional constraint layered over existing transport/state classifications.

## Non-goals

No CRDT merge of independently evolved minds, no lockstep broadcast of every cognitive intermediate, no client-predicted learning authority, no direct remote mind-reading, and no replacement of Lightyear/Iroh/net-core are introduced here.

Relates to #1183, #1185, #1186, #1188, #1193, #1197, #1198, #1199 and #170.
