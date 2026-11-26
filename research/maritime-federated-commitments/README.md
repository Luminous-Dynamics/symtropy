# Maritime Federated Commitments v0.1

**Status:** synthetic state-machine prototype; no external actions.  
**Tracking:** [#7528 — MARITIME-FEDERATED-COMMITMENTS-001](https://github.com/Luminous-Dynamics/symtropy/issues/7528).  
**Related:** #3683, #7488, #7504, #7512, #7519, #7510.

## Why this workstream exists

The port network is not one organization's resource pool. Berths, vessels, tug/pilot windows, terminal equipment, yards, power, rail, truck appointments and receiver capacity are controlled by different participants. A central optimizer can propose a plan; it cannot create consent or authority for every participant.

Existing products and standards already address collaboration:
- DCSA defines interoperable port-call and track-and-trace event semantics.
- Portchain Connect supports berth-alignment information sharing and counteroffers.
- Terminal operating systems such as Kaleris N4 own operational execution.
- Mycelix remains the authority owner for participant permissions, revocation and custody.

The research target is not another Port Call standard or berth-alignment portal. It is a cross-system evidence and commitment qualification layer.

## v0.1 contract

An offer binds:
- commitment ID and monotonic revision;
- resource-controlling participant and required accepting participant;
- resource ID, time slot, quantity and explicit unit;
- expiry and immutable terms digest.

Events are append-only: offer, counteroffer, acceptance, rejection and revocation. A counteroffer supersedes the prior terms and resets acceptance. Only the named accepting participant may accept or reject. Only the resource controller may revoke an accepted commitment. Expired, rejected and revoked offers are not eligible for a decision bundle.

The projection filters events by receiver knowledge cutoff. An event recorded after the cutoff cannot rewrite the earlier projection. Effective revocation time remains separate from when the revocation was recorded.

## Unknown external outcome guard

MutationGuard is an explicit state machine, not an execution client. It has no network access and never performs a booking or dispatch.

After an external action is attempted but the outcome is unknown:
- retry with the same action and idempotency key is classified as idempotent replay only;
- a different mutation is blocked until reconciliation;
- reconciliation must match the original action ID and key;
- after confirmed execution, another mutation is blocked;
- after confirmed non-execution, a new attempt may be proposed.

This encodes the invariant: unknown outcome means no new conflicting mutation until reconciliation.

## Run locally

```sh
cargo fmt --manifest-path research/maritime-federated-commitments/Cargo.toml -- --check
cargo test --locked --manifest-path research/maritime-federated-commitments/Cargo.toml
cargo clippy --locked --manifest-path research/maritime-federated-commitments/Cargo.toml --all-targets -- -D warnings
```

## Limitations and claim ceiling

- Synthetic event reducer only; no external booking, TOS, dispatch or custody API.
- Scalar timestamps only; no timezone/clock uncertainty or full tri-temporal adapter.
- No signature verification, key management, legal contract enforcement or identity authentication.
- No multi-party bundle optimizer or market/mechanism comparison yet.
- The resource feasibility oracle remains #7519 / PR #7521; this crate does not duplicate it.
- Permission and revocation authority remains Mycelix; this crate does not issue grants.
- Passing tests do not establish participant consent, real resource availability or field performance.

The first future adapter should consume a typed resource-plan proposal from #7519, verify that every required participant has an accepted, unexpired, unrevoked commitment, then pass typed commitment receipts to #7488. No automatic operation is authorized here.


## Contract correction: accepted offers are immutable

A counteroffer is allowed only while the current offer remains open. Once accepted, its terms are committed; changing resource, slot, quantity or conditions requires a separate amendment/revocation protocol rather than silently mutating the accepted revision.

The tests distinguish:
- counteroffer before acceptance → new revision → fresh acceptance of that revision;
- counteroffer after acceptance → rejected as `OfferNotOpen`.

This prevents the event reducer from rewriting an already accepted obligation.
