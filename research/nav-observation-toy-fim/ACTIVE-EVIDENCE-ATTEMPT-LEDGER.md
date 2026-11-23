# Active evidence acquisition attempt ledger

**Status:** standalone consume-once software kernel with deterministic synthetic tests. It is not connected to a physical executor or durable evidence store.

**Tracker:** [#7080 — outcome-complete active acquisition and reconciliation](https://github.com/Luminous-Dynamics/symtropy/issues/7080).

## Purpose

A sensing request is not an observation. A payload is not necessarily valid evidence. An executor timeout may also leave the physical result unknown.

This ledger makes the attempt lifecycle explicit:

1. Freeze the planned action, attempt ID, domain, acquisition protocol and pre-attempt navigation-state commitment.
2. Admit one attempt and append a monotonic start event.
3. Consume exactly one executor outcome.
4. Preserve the exact pre-attempt navigation-state commitment for every outcome.
5. On an unknown result, block all subsequent attempt starts until an explicit reconciliation receipt is recorded.
6. Allow a retry only as a new attempt ID and charge it against the action's frozen attempt budget.
7. Treat a qualified payload as available for a later independent posterior audit; never apply it to the navigation state inside this ledger.

## Outcome semantics

| Outcome | Ledger effect | Retry / mutation rule |
|---|---|---|
| Qualified evidence available | Record payload and qualification receipt commitments; mark the action as qualified | No retry under the same action ID; downstream audit still required |
| Failed or timed out | Record failure reason; leave navigation-state commitment unchanged | New attempt ID allowed within the frozen retry budget |
| Acquired but invalid | Record payload and invalidity receipt; leave navigation-state commitment unchanged | New attempt ID allowed within the frozen retry budget |
| Unknown / unreconciled | Record the unknown reason and block every later attempt start | No new mutation or retry until reconciliation |

Unknown reconciliation must be backed by a nonzero reconciliation receipt and resolve to exactly one of qualified, failed/timed-out or acquired-but-invalid. It cannot resolve to another unknown state. The reconciliation operation also requires the unchanged pre-attempt navigation-state commitment.

## Implemented invariants

- Attempt IDs are globally consume-once.
- Only one attempt may be in flight at a time.
- Event sequence numbers are monotonic and contiguous within one ledger instance.
- Each action has a frozen action-contract, domain, protocol and pre-state identity across retries.
- Domain and protocol must match the policy pinned at ledger creation.
- A failed or invalid outcome cannot change the navigation-state commitment.
- A qualified outcome does not itself update the posterior, fault inventory, protection level or navigation authority.
- An unknown attempt blocks attempts for every action until it is reconciled.
- The initial attempt and all retries count against one frozen per-action maximum.

## Tests

The synthetic suite covers:
- monotonic start/outcome sequences and one terminal outcome;
- explicit retry after failure;
- invalid payload as non-success;
- global mutation blocking while an outcome is unknown;
- reconciliation to failure releasing the block while charging the original attempt;
- reconciliation to qualified evidence making the action terminal;
- state-digest mismatch blocking outcome recording and reconciliation;
- frozen action/domain/protocol/pre-state across retries;
- domain and protocol mismatch rejection;
- globally duplicate attempt IDs and exhausted retry budgets;
- one in-flight attempt at a time;
- missing policy, outcome and reconciliation commitments.

## Boundaries and production follow-up

This increment is in-memory and single-writer. It does not survive process restart, provide durable exactly-once execution, authenticate hashes or receipts, execute the physical sensor action, prove independence, validate the physical measurement, or authorize navigation NORMAL.

Before deployment, the event stream must be appended durably before the physical side effect is issued. On restart, the executor and durable ledger must be reconciled before another mutation is allowed. The durable store must enforce unique attempt IDs and compare-and-swap / monotonic sequence semantics across concurrent writers. The resulting qualification receipt must then be consumed by the existing independent posterior and integrity auditors.

SHA-256 fields in this kernel are caller-supplied commitments. Nonzero/equality checks do not authenticate source bytes or prove that the declared event occurred.
