# Orbital Interface Event Replay Benchmark v0.1

**Status:** synthetic reference implementation; hosted Rust validation pending.  
**Owner:** [ORBITAL-INTERFACE-EVENT-REPLAY-001 #6515](https://github.com/Luminous-Dynamics/symtropy/issues/6515).  
**Parent:** [ORBITAL-INTERFACE-LIFECYCLE-001 #6484](https://github.com/Luminous-Dynamics/symtropy/issues/6484).  
**Claim ceiling:** synthetic event identity, ordering, idempotency and reducer semantics only.

## Why this exists

The first lifecycle evaluator consumes normalized synthetic lifecycle state fields. That tests state-decision semantics but does not prove the state was reconstructed from immutable original events. This module adds exact-byte event admission and deterministic event-stream replay.

It does not replace #6484's snapshot qualification semantics or #6384's receiver-accepted service receipt admission.

## Event model

Each event binds an exact pair, stream, component, configuration, procedure, interface and service scope, plus:
- stable event ID, sequence and predecessor;
- event, observation and ingestion ticks;
- source producer and authority scope;
- event kind and typed transition evidence;
- exact raw event bytes and a SHA-256 digest supplied in the outer record.

Identical event IDs with identical bytes are idempotent and counted once. Conflicting bytes under one event ID are Disputed. Sequence forks are Disputed. Missing sequence/predecessor and time-order ambiguity are Indeterminate. Counter reset without explicit component lineage is Disputed.

The digest is recomputed over the exact `raw_event` string bytes. It is not a producer signature, instrument authentication or proof of physical truth. Inspection/environment events require scoped measurement-quality fields before they can withdraw or restrict a qualification; an uncalibrated failure assertion is Indeterminate.

## Reducer

The reducer handles initial qualification, service cycles, inspections, maintenance, component replacement, configuration/procedure/interface changes, revocation, counter reset and environmental-envelope observations.

**Counter boundary:** the reported `cycle_count` is the count in the current qualified component/qualification segment. Requalification begins a new qualification segment; component replacement begins a new component segment. The immutable event stream preserves the prior history, but v0.1 does not emit a separate cumulative provider/client-pair lifetime total or per-component lifetime roll-up. Never interpret the reported field as a lifetime cycle count.

The stream must begin with an initial qualification. Each subsequent event must carry the exact previous event ID. Event-time uncertainty intervals must prove the required order; ingestion order is not used as event order. Events not fully known by the evaluation cutoff are indeterminate. Service cycles outside qualification or inspection validity cannot count as qualified cycles, and an overdue inspection cannot renew the prior qualification without an explicit requalification event.

A configuration change, maintenance event, component replacement, environmental exposure outside the current envelope, expired qualification or expired inspection interval requires requalification. Revocation and known wear-limit violation withdraw the qualification. A withdrawn qualification is terminal in v0.1; reinstatement policy is deliberately not inferred.

## Frozen corpus and oracle

The frozen corpus contains 63 synthetic end-to-end cases. The event schema is published at [event-replay-event-v0.1.schema.json](schema/event-replay-event-v0.1.schema.json). A separately written value-based integration oracle reconstructs outcomes without calling the production reducer. It checks status, reason, cycle count and duplicate count.

Run:

```bash
cargo fmt --manifest-path tools/orbital-interface-lifecycle-bench/Cargo.toml -- --check
cargo test --manifest-path tools/orbital-interface-lifecycle-bench/Cargo.toml --all-targets
cargo clippy --manifest-path tools/orbital-interface-lifecycle-bench/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path tools/orbital-interface-lifecycle-bench/Cargo.toml --bin orbital-interface-event-replay -- \
  tools/orbital-interface-lifecycle-bench/fixtures/v0.1/cases.json \
  --source-commit <40-hex-commit>
```

## Boundaries

- No physical durability or remaining-useful-life model.
- No statistical reliability inference.
- No signature verification or authenticated producer identity.
- No validation of a physical clock or calibration certificate.
- No fluid transfer, receiver-mass re-accounting or new service composition engine.
- No actuation, mission or operational authority.
- No cargo compile/test/Clippy pass is claimed until the exact-head workflow completes.

Composes with #6484/#6485, #6372, #6384/#6221 and #6256/#6303.
