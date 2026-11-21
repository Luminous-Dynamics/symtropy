# Maritime Backlog Ledger — Benchmark Protocol v0.1

**Status:** synthetic known-answer accounting/replay contract. No real operational feed or measured port outcome is included.

## 1. Research question

Can a port operations system reconstruct vessel queue and container inventory from corrected, duplicated, late, conflicting and multi-source observations without double-counting movements, leaking future information or violating stock conservation?

This v0.1 qualifies accounting/replay before any queue or recovery optimizer is compared.

## 2. Paradigm shift: queues are stocks, throughput is flow

For a vessel-call population scoped to one terminal and observation window:

`closing_queue = opening_queue + eligible_arrivals + rebooked_returns - first_berth_entries - diversions - cancellations - other_declared_exits`

For a terminal's physical container-box stock:

`closing_boxes = opening_boxes + discharge_in + gate_in + rail_in + adjustments_in - vessel_load - gate_out - rail_out - adjustments_out`

A rehandle is recorded as a movement but has zero net stock delta. It must not become additional throughput or additional inventory merely because it was moved again.

The two ledgers are independent. Vessel calls, physical boxes, TEU-equivalents, container moves, tonnes and train paths are never interchangeable.

## 3. Time and correction semantics

Every source observation contains:
- canonical event ID;
- immutable source-record ID;
- source/issuer ID;
- revision number;
- observation tick;
- event tick;
- classification (ACTUAL, ESTIMATED, PLANNED, REQUESTED);
- state (ACTIVE or VOIDED);
- event kind;
- quantity unit and quantity;
- terminal ID.

A case declares its terminal, event-time window, historical as-of cutoff, opening stock and reported closing stock.

### Historical cutoff

Only source records with `observed_tick <= as_of_tick` are visible to a replay. A higher revision observed after the cutoff cannot change an earlier decision snapshot. Filtering by the cutoff occurs before identity/conflict checks so future records cannot contaminate historical replay.

### Idempotency and corrections

- An exact duplicate `source_record_id` with identical content is replay-idempotent.
- Reuse of a source-record ID with different content is a conflict.
- Within one canonical event ID, revisions are ordered and must be contiguous from 1.
- The largest visible revision is current state; earlier revisions are superseded, never added to it.
- A VOIDED latest revision contributes no stock effect and must carry `event_kind=NONE`, quantity zero.
- A canonical event ID may not silently switch source, terminal or stock unit across its revisions. Competing source assertions require an upstream reconciliation decision; this evaluator does not pick the freshest assertion.

### Actual versus non-actual

Only ACTIVE ACTUAL events can mutate stock. ESTIMATED, PLANNED and REQUESTED rows are retained as observations but must carry NONE/zero in this stock ledger. This prevents a forecast or request from being mistaken for a completed physical movement.

## 4. Unit and identity constraints

- Vessel ledger: `VESSEL_CALL`.
- Container ledger: `PHYSICAL_BOX`.

No TEU conversion is inferred. A physical 40-foot box counts as one box here. If a later profile needs TEU-equivalent capacity or move counts, those belong in separate fields and separately qualified source mappings.

Each active event must match the case terminal, stock unit, ledger event-kind vocabulary and time window. Actual event tick must not exceed the as-of cutoff; observation tick must not precede event tick.

## 5. Frozen synthetic cases

The fixture contains 31 cases: 15 accepted and 16 rejected.

Coverage includes:
- ordinary vessel arrival / berth / diversion / exit flows;
- ordinary container discharge, loading, gate and rail flows;
- additive correction replaced by a later revision;
- exact duplicate source-record replay;
- planned observation beside actual arrival;
- future-observed revision excluded from an earlier snapshot;
- voided event revision;
- rehandle without additional stock;
- vessel/container underflow;
- cross-ledger event kind;
- unknown event kind;
- future event time, pre-window event and observation-before-event;
- conflicting cross-source same-event assertion;
- source-record ID reused with changed content;
- revision gap;
- changed terminal;
- planned event attempting to mutate actual stock;
- NONE/zero and zero-quantity actual events;
- end-of-window stock mismatch;
- explicit unit mismatch (TEU-equivalent presented to a physical-box ledger).

Expected dispositions are frozen in the case CSV. The Rust verifier has no access to a hidden oracle or external service.

## 6. Reproduction and integrity

Run:

```bash
sha256sum --check data/SHA256SUMS
cargo fmt --all --check
cargo test --offline --locked
cargo run --offline --locked
cargo clippy --offline --locked --all-targets -- -D warnings
```

The evaluator canonicalizes active stock-changing events by `(event_tick, event_id)` before checking the intermediate stock trajectory. Input row order must not affect the outcome. The case report includes expected and actual dispositions plus the rejection reason.

## 7. Industry and standards context

- **World Bank Container Terminal Performance Improvement Guide (30 October 2026):** distinguishes terminal productivity and port-call indicators, and lays out port-call stages from arrival at port limits through pilotage, berth, cargo operations and departure. Some port-call metrics involve actors outside the terminal boundary.
- **World Bank / S&P Global CPPI 2025:** measures vessel time in port using event-level port-call data; it is an important external benchmark but does not measure end-to-end inland delivery or queue-stock conservation.
- **DCSA Port Call 2.0.0:** provides common timestamp semantics (estimated, requested, planned, actual), port-call services and event/facility distinctions including berth and pilot boarding place. This verifier deliberately consumes already-admitted typed event records; it does not replace the DCSA adapter or conformance suite.
- **S&P Global Port Performance data:** proprietary call-level benchmarking coverage. Do not scrape, copy or redistribute; seek lawful access where needed.

References:
- https://documents.worldbank.org/en/publication/documents-reports/documentdetail/099103026152033883
- https://www.worldbank.org/en/topic/transport/publication/cppi
- https://reference.dcsa.org/content/standards/releases/port-call/v2/pc-v2-purpose-and-scope
- https://reference.dcsa.org/content/standards/releases/port-call/v2/pc-v2-use-cases

## 8. What must happen before algorithmic SOTA claims

1. Admit a rights-qualified event cohort with source artifacts, event dictionaries, coverage and corrections.
2. Independently reconcile vessel-call and terminal-inventory populations against operator source totals.
3. Publish censoring, missingness and cohort rules. A ship still waiting at window end is not an ordinary completed-call observation.
4. Compose queue stocks with vessel/berth/tug/yard/gate/rail service owners.
5. Compare FCFS, published operating policy, exact tiny-instance oracle, MILP/CP-SAT, robust/rolling horizon and appropriate evolutionary or learning-based methods using identical event cutoffs, capabilities, objective definitions, disruption tapes and compute budgets.
6. Report p50/p90/p95 wait, vessel-hours in queue, box-hours of inventory, backlog area under the curve, delivered-service reliability, recovery time, cancellations/diversions and equity by operator/service/cargo class.
7. Compare candidate plans on held-out weeks, terminals, services, vessel classes and disruption episodes. Preserve all infeasible, timed-out, abstained and incomplete cases.

A lower mean wait or higher number of moves is not enough to establish improvement if queue stock, p90 wait, delivery commitments or recovery become worse.

## 9. Proposed advancement gates (not measured results)

- 31/31 synthetic expected dispositions match.
- Zero stock-flow residual in every accepted fixture.
- Exact replay remains idempotent.
- No future-observation leakage.
- No unit, source, terminal, ledger or revision ambiguity is silently resolved.
- Exact agreement with an independent tiny-instance oracle on subsequent scheduling workloads.
- Any operational improvement is reproduced against the strongest matched baseline on an independently held-out real cohort, with no hard-gate regression.

## 10. Non-claims

Synthetic conformance does not authenticate source records, establish real Durban queue performance, validate physical capacity, estimate a vessel waiting-time distribution, prove causality or grant operational/contractual authority.
