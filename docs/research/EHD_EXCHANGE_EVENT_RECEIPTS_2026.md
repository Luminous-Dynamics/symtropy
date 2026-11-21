# EHD Event-Level Momentum Exchange Receipts (2026)

**Benchmark:** EHD-EXCHANGE-RECEIPT-001  
**Status:** in-memory Rust reference validator and authored tests; Rust execution pending  
**As of:** 21 November 2026  
**Machine-readable form:** [ehd_exchange_event_receipt_v0_1.json](../../data/research/ehd_exchange_event_receipt_v0_1.json)

## 1. Why aggregate conservation is insufficient

A global residual can hide local errors. For example, two exchange events can have residuals +0.1 N·s and -0.1 N·s. Their aggregate residual is zero even though neither event is balanced.

Therefore the exchange qualification layer must verify the physical interaction at event level before it performs the global sum.

For event \(e\), with charged-side reaction impulse \(\mathbf I_{c,e}\) and neutral-side impulse \(\mathbf I_{n,e}\), require:

\[
\mathbf r_e=\mathbf I_{c,e}+\mathbf I_{n,e},
\qquad
|r_{e,k}|\le \epsilon_{abs}+\epsilon_{rel}
\max(|I_{c,e,k}|,|I_{n,e,k}|).
\]

Then independently aggregate the event receipts and check the global componentwise residual.

## 2. Implemented reference validator

The pure-Rust module is:

`tools/ehd-reference-audit/src/exchange_receipt.rs`

Each event carries:
- unique event ID within the supplied batch;
- physical transfer-path ID;
- selected force-transfer method;
- charged and neutral solver identities;
- charged and neutral coordinate-frame identities;
- integrated charged reaction impulse and neutral impulse in N·s.

The validator:
1. rejects empty batches, duplicate event IDs and non-finite impulses;
2. rejects any event whose charged and neutral sides declare different frame identities;
3. rejects a transfer path that selects both explicit species-collision drag and reduced Coulomb deposition;
4. checks equal-and-opposite impulse for every event and every vector component;
5. checks the aggregate residual as a separate condition;
6. sorts by event ID, uses deterministic ordered maps and compensated accumulation;
7. requires solver-pair and coordinate-frame identity to remain stable across all events assigned to one transfer path;
8. emits a receipt with path-to-method ownership, aggregate impulses, aggregate residual and maximum event residual.

The reference validator does not perform coordinate transformations. A non-matching frame is rejected; a future transform adapter must provide an explicit transform identity, convention, source and destination frame, and transformed-vector receipt.

## 3. Why exactly-once is not yet claimed

This implementation rejects duplicate event IDs inside one in-memory batch. It does not provide durable idempotency across:
- solver restarts;
- process crashes after one side has consumed an event;
- message redelivery across independent processes;
- partial database/queue commits;
- two separate batches that reuse an event ID.

A production exactly-once exchange needs a durable transaction protocol: immutable event identity, idempotency key, staged/committed/reconciled states, atomic paired consumption or an explicit compensating/reconciliation process, append-only receipts and replay tests. Unknown commit outcome must block new mutation until reconciliation. This is a follow-on work package, not a property of the present Rust function.

## 4. Authored test matrix

The module currently contains twelve tests:
- exact equal-and-opposite events;
- duplicate event ID rejection;
- per-event errors that cancel globally;
- conflicting transfer methods on one path;
- implicit coordinate-frame mixing;
- solver-pair or frame identity changes within one transfer path;
- non-finite impulses;
- deterministic input permutation;
- multiple events on one path with a consistent method;
- empty batch rejection;
- invalid tolerance rejection;
- CLI reference self-check.

These are authored tests, not executed Rust tests.

## 5. Relationship to the impulse remapper

The event receipt validator and geometric impulse remapper solve different problems:

- `exchange.rs` distributes an already-integrated source impulse across destination cells and checks global remap conservation.
- `geometry.rs` validates static axis-aligned space-time overlap coverage.
- `gcl.rs` checks the swept-volume identity for linearly moving axis-aligned cells.
- `exchange_receipt.rs` checks per-event equal-and-opposite physical exchange, method exclusivity and frame consistency.

A qualified solver adapter must produce all relevant receipts. Passing one does not imply passing the others.

## 6. Proposed qualification gates (targets, not results)

- Zero duplicate event IDs within a transaction batch.
- Zero accepted event-level equal-and-opposite residual above its predeclared componentwise tolerance.
- Zero transfer paths with multiple active closure methods.
- Zero implicit frame mismatches.
- Deterministic receipts under input permutation.
- Aggregate residual passes independently after all event-local checks.
- Durable idempotency and unknown-outcome reconciliation before any production exactly-once claim.
- Event records bind to solver versions, mesh/time identities, physical coefficient provenance, units and a run ID before solver integration.

## 7. Claim ceiling

This is an in-memory batch validator and authored test suite. It does not implement a durable exactly-once transaction manager, perform collision physics, transform coordinate frames, run a plasma or neutral solver, validate physical thrust, or establish Rust/CI success or SOTA superiority.
