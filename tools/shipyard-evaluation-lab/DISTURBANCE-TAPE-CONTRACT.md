# Shipyard Disturbance Tape Contract v0

Status: research prototype contract; not an industrial data format or a validated runtime.

## Purpose

A benchmark run must replay the same immutable disturbances for every policy while preventing policies from seeing facts before they are observed. This contract separates (1) when a condition is physically effective, (2) when the simulator/evaluator knows it, and (3) when a policy is allowed to observe it.

The tape is a test input, not an optimizer, safety case, or claim of industrial performance.

## Required invariants

1. **Stable identity:** every event has a unique `event_id`; duplicate IDs reject the tape.
2. **Explicit time semantics:** `effective_tick`, `observed_tick`, and `delivery_tick` are non-negative integer ticks. They are not wall-clock timestamps.
3. **No impossible observation:** `delivery_tick < observed_tick` is invalid. A late observation is allowed.
4. **Truth/observation separation:** `truth` is evaluator-owned; `observation` is policy-visible only at or after `delivery_tick`. The policy API must not receive the truth object.
5. **Deterministic ordering:** events are sorted by `(delivery_tick, event_id)` for policy delivery and by `(effective_tick, event_id)` for evaluator simulation. Input file order has no semantic meaning.
6. **No silent defaults:** unknown event types, missing required fields, invalid enum values, duplicate JSON keys, and unsupported contract versions reject rather than coerce.
7. **Immutable bytes:** corpus entries are hashed from their exact UTF-8 bytes; canonical semantic digests, if added, must be a separate field and algorithm version.
8. **Same prefix, same information:** two episodes with the same policy-visible observation prefix and same declared initial state must expose identical policy information at that decision epoch, regardless of future truth.
9. **No sample deletion:** invalid, incomplete, timed-out, and budget-exhausted runs remain in the result ledger.
10. **No clairvoyant ranking:** hindsight/oracle tapes are diagnostic upper bounds and are not ranked as online policies.

## v0 event envelope

Each event contains:
- `event_id`: stable unique string.
- `event_type`: one of the explicitly supported event types.
- `effective_tick`: tick when the physical/evaluator-side condition begins.
- `observed_tick`: tick when source evidence first establishes the event.
- `delivery_tick`: tick when the policy can receive the observation.
- `visibility`: `policy_visible` or `evaluator_only`.
- `truth`: evaluator-only structured fact.
- `observation`: policy-facing payload or `null` for evaluator-only events.

For `evaluator_only`, `observation` must be null. For `policy_visible`, observation must be non-null. A policy must never receive evaluator-only events, future events, the full tape, or hidden truth via debug metadata.

Supported v0 event types in the sample are illustrative: `vehicle_outage`, `workstation_outage`, `yard_closure`, and `clearance_delayed`. An implementation must define typed payload schemas for each before claiming full conformance.

## Episode-level manifest

A frozen episode manifest should bind:
- contract version and episode ID;
- initial-state artifact digest;
- event-tape exact-byte SHA-256;
- policy-visible event stream digest (derived using a declared algorithm);
- expected operation/scenario IDs;
- policy executable/source revision and configuration digest;
- per-decision action and compute budgets;
- termination rule and tick duration;
- evaluator revision and independent-checker versions;
- seed and RNG algorithm where randomness is used;
- hardware/toolchain identity for performance claims.

Never use a digest as a substitute for validating the underlying schema and semantics.

## Required negative controls

The corpus test suite must prove rejection or isolation for:
- duplicate event IDs and duplicate JSON object keys;
- observation delivered before observed time;
- hidden future event altered while the current visible prefix stays fixed;
- evaluator-only truth inserted into a policy observation;
- same event ID with changed observation payload;
- missing, duplicated, delayed, and reordered delivery;
- unknown event type / unsupported version;
- malformed ticks, negative ticks, and integer overflow;
- event tape changed after its declared digest is recorded;
- policy trace missing a decision or containing extra decisions;
- run timeout or budget exhaustion incorrectly reported as infeasibility.

A changed future-only event must not change any earlier policy input or action. The event may affect decisions only after its observation is delivered.

## Deterministic replay procedure

At each decision tick:
1. Advance evaluator state using events whose `effective_tick` has arrived.
2. Deliver only `policy_visible` observations whose `delivery_tick` has arrived and have not already been delivered.
3. Construct the policy input from the initial-state projection, committed/executed state, and delivered observation prefix.
4. Invoke the pinned policy under the declared action/compute budget.
5. Append the action and receipt; do not overwrite prior actions or prior run artifacts.
6. Independently verify the action and resulting state.
7. Record timeout, invalid action, budget exhaustion, or evaluator failure as explicit terminal outcomes.

The precise ordering between state advance, delivery, and decision is part of the manifest. This document chooses the sequence above for v0; implementations must not silently change it.

## Receipt requirements

Each episode receipt records the exact input digests, policy and evaluator identities, decision count, delivered observation IDs per decision, action-trace digest, outcome status, hard-constraint violations, missing coverage, wall time, peak memory when measured, and per-metric values with units. Metrics must be recomputed from event/state history by an evaluator independent of the candidate policy.

Allowed statuses include `feasible`, `infeasible` (only when backed by the declared proof procedure), `budget_exceeded`, `invalid_input`, `incomplete`, `timed_out`, and `evaluator_error`. Do not collapse these statuses into a boolean.

## Claim boundary

The sample fixture is synthetic. Passing tape-contract tests demonstrates replay-contract conformance only. It does not establish geometric feasibility, physical realism, optimality, industrial uplift, or production readiness.
