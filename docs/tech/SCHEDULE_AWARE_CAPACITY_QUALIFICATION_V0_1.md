# Schedule-Aware Capacity Qualification V0.1

Status: research/reference implementation, draft PR stacked on the multimodal transport optimizer. The code is deterministic and effects-disabled. It does not authorize dispatch, hold a reservation, or prove that a reported capacity exists physically.

## Problem this slice closes

The route optimizer's earlier capacity evidence validator answers whether a snapshot is usable at a supplied evaluation instant. It does not by itself establish that the snapshot remains valid for the whole future operation or prevent two overlapping plans from spending the same crane/track-slot/transfer resource capacity independently.

This slice adds a schedule-aware admission boundary for capacity resources. It is deliberately narrower than a complete railway timetable or multi-commodity flow solver.

## Contract

Each assignment names an exact evidence record, resource identity and capacity scope, quantity/unit, and an operation window. The window has an explicit time-profile ID and half-open interval `[planned_start, planned_end)`. Timestamps are caller-supplied Unix epoch seconds under that profile; the validator reads no ambient clock.

A capacity evidence record binds:
- resource kind and physical resource ID (transfer edge or handling equipment);
- capacity quantity and unit, plus an opaque scope ID;
- source/authority identity, schema profile, revision lineage and source digest;
- observation time, repository-recorded time, validity interval and time-profile ID;
- source-reported lifecycle status.

The validator checks identity/provenance completeness, revision ordering, status, unit consistency, clock-profile compatibility, future-clock tolerance, freshness at evaluation time, freshness by planned operation start, and complete validity-interval coverage. It preserves event time separately but does not use it as observation time. Repository-recorded time is bookkeeping, not observation time.

A validity interval is also half-open. Evidence `[100, 200)` covers an operation `[100, 200)`; it does not cover `[100, 201)`. Future schedules may be published before their `valid_from` time, but still must be fresh at evaluation and remain within the configured observation-age limit at planned start.

## Shared-capacity accounting

The validator creates start/end events for every assignment and performs a deterministic sweep per physical resource:
- end events sort before start events at the same timestamp;
- adjacent windows therefore do not overlap;
- simultaneously active assignments are summed with checked integer arithmetic;
- capacity is rejected if any overlap exceeds the evidence-backed limit;
- overlapping current evidence records for the same physical resource are ambiguous and fail closed, even if their declared limits happen to match;
- different resource scopes do not let a caller double-spend the same physical resource at overlapping times.

**Capacity semantics are bounded:** this module treats the declared quantity as a concurrent-use limit over operation intervals. It is not a tonnes-per-shift, passenger-throughput, or flow-rate allocator. Those require a time-bucketed flow/quota model with explicit rate units and aggregation semantics. Never adapt a rate or period quota into a concurrent-use limit without a reviewed conversion.

The returned report sorts assignments and resource summaries deterministically and retains the evaluation time, exact policy ID, time profile, planned windows, evidence IDs/revisions, observation/record times, validity bounds, authority/source profiles, and source digest.

## Deliberate authority boundary

A successful report says only that the supplied proposal is internally consistent with the supplied evidence and declared policy. It does not:
- authenticate an authority or independently recompute the source digest;
- establish source completeness, sensor accuracy or physical truth;
- reserve capacity, consume a budget, or guarantee live availability;
- certify railway safety, cargo securing, legal load limits, or a terminal's operation;
- replace railway signaling/interlocking, operator scheduling, or owner-authoritative compare-and-consume/lease logic.

Exact retry determinism is not execution idempotency. A live reservation needs its own authority, lease/compare-and-consume protocol, durable receipt, retry semantics, cancellation/expiry, and reconciliation.

## South African pilot fit

Use this reference implementation as a qualification primitive for a future, evidence-bounded freight pilot. Do not hard-code a map or import dashboard values as though they were a live API.

1. The [National Freight Databank](https://freight.transport.gov.za/) describes five freight modes and 16 national freight corridors, including road, rail, pipeline and facilities such as ports, border posts, weighbridges and intermodal terminals. It is an authoritative discovery lead; public visibility does not by itself grant bulk-download rights or establish an API contract.
2. The [Draft National Rail Master Plan consultation](https://www.gov.za/news/speeches/minister-barbara-creecy-launch-gauteng-stakeholder-engagement-draft-national-rail) describes a planned Rail Planning Component for maintaining passenger/freight flows, capacity, asset condition and rolling-stock data. Treat this as the government's stated plan, not as proof that an operational feed is available to this project.
3. The [Western Cape intermodal-network proposal](https://www.westerncape.gov.za/mobility/intermodalism-building-more-connected-freight-network) reports a modelled opportunity for about 10.4 million tonnes of road freight to use intermodal terminals and explicitly describes the terminal network as proposed. This is a candidate for scenario selection, not a measured realizable shift, a confirmed terminal inventory, or verified current capacity.

Before using real data, freeze a source manifest containing the exact publisher, artifact URL or acquisition record, access/licence terms, dataset/schema version, coverage interval, CRS or time profile as applicable, retrieval time, byte count, SHA-256, source units, missingness and transformations. Keep `not_acquired`, `access_review_required`, `acquired_unreviewed`, `qualified_for_scenario` and `rejected` states distinct. If the source cannot provide a sufficiently precise time window or resource identity, the adapter must not invent one.

## Qualification gates

- **H0 — deterministic contract:** tests cover invalid/empty windows, incompatible time profiles, future validity, expired evidence, evidence that goes stale before use, half-open endpoint behavior, duplicate/ambiguous evidence, adjacent/non-overlapping bookings, overlapping overcommitment, unit mismatch, conflicts, replay determinism and exact report lineage.
- **H1 — frozen source snapshot:** acquire only rights-reviewed data; preserve exact source bytes and transformations. No operational capacity should be inferred from aggregate national throughput.
- **H2 — offline corridor replay:** compare simulated schedules with a separately computed verifier and historical outcomes where compatible evidence exists. Preserve unknown/stale/conflicted states.
- **H3 — operator and safety review:** infrastructure managers and qualified operators validate resource identities, capacity units, operating windows and constraints.
- **H4 — shadow mode:** recommendations are compared with actual outcomes before any operational integration. Any live reservation/control remains in the operator's authoritative system.

Passing H0 is only a code-contract result. It cannot be promoted to a real-world performance or safety claim without the later evidence gates.

## Reference tests in this slice

The Rust tests are synthetic and offline. They intentionally cover:
- evidence expiring before the planned end;
- evidence fresh at evaluation but stale at planned start;
- equal half-open boundaries;
- zero/reversed windows and incompatible time profiles;
- non-overlapping windows reusing capacity;
- intersecting assignments exceeding capacity;
- conflicting current evidence for the same resource;
- preserved observation/record timestamps, policy, source digest and window;
- deterministic replay and changed-window receipts;
- rejection of conflicted evidence.

No real rail, road or terminal records are embedded in the tests.
