# Period-Quota Accounting V0.1

Status: research/reference implementation. This is a separate accounting primitive from capacity_window.rs, which qualifies concurrent physical use. It does not reserve capacity, dispatch transport, or prove source claims reflect physical reality.

## Why a second model exists

Capacity is not one interchangeable number. A crane may allow two simultaneous lifts, a rail section may have a limit of train paths in a timetable period, and a terminal may process a bounded mass of cargo per day. These constraints have different units and aggregation rules.

The schedule-aware capacity-window validator interprets a value as a concurrent-use limit over operation intervals. This module interprets a source-backed value as a **total quantity available over one explicit half-open period** [period_start, period_end). A value of 800 tonnes for one day cannot be silently interpreted as 800 tonnes/hour, 800 concurrent shipments, or 800 train paths.

## Contract

Each PeriodCapacityQuotaV1 wraps one exact ScheduledCapacityEvidenceV1 record and defines the period to which its capacity_units value applies. It preserves resource identity, scope, unit, source/authority identity, source schema and digest, revision lineage, source event time, observation and repository-recorded times, validity interval, time profile and source-reported status.

The validator:

- accepts only source-reported ReportedCurrent evidence and explicit nonempty provenance;
- requires a caller-supplied policy and time profile; reads no ambient clock;
- requires evidence to be fresh both at planning evaluation and at the future period start, and its half-open validity interval to cover the entire period;
- rejects started periods, reversed/empty periods, future timestamps beyond policy skew, stale observations and invalid revision lineage;
- rejects overlapping quota periods for the same physical resource even if the caller assigns different scope labels, and rejects unit changes for the same resource in one plan;
- binds each proposed use to one exact quota ID, sums with checked integer arithmetic, rejects unit mismatch/overcommit/unknown and unused quotas, and emits sorted provenance-preserving receipts.

Two adjacent periods such as [200, 300) and [300, 400) do not overlap. The same resource may have a distinct quota in each adjacent period when the evidence and units are consistent.

## Consumption semantics

Each PeriodCapacityUseV1 is an explicit amount charged to exactly one period quota. If a shipment or operation consumes capacity in multiple periods, the caller must provide reviewed consumption slices and bind each slice to its corresponding quota. This module does not split an operation over time, infer a linear production rate, or prorate consumption by interval overlap.

When the source reports throughput such as tonnes/hour or units/shift, its period, operating calendar, outages, resource scope and unit conversion must be established by an adapter before a total-period quota can be supplied. A rate must not be converted to a period total by guessing how many hours the resource operates.

## Failure and authority boundaries

The report preserves the supplied policy ID, evidence ID/revision, period, unit, capacity limit, amount used, allocation IDs and source lineage. A receipt is deterministic planning evidence only. It is not a reservation, a consume-once ledger entry, a guarantee of live availability, a safety certificate or dispatch authorization. Source authentication, independent digest recomputation, authoritative quota issuance, prior-use reconciliation and durable compare-and-consume remain outside this reference model.

## Synthetic test coverage

Offline Rust tests cover sorted receipts and input-order determinism, adjacent periods, distinct-resource accounting, quota overcommitment, arithmetic overflow, stale evidence at period start, period validity coverage, conflicted evidence, already-started periods, duplicate or unknown allocations, unit mismatch and unreferenced quotas. No real railway or terminal data are embedded in the tests.

## Example: a shipment crossing the period boundary

Suppose a shipment consumes 3 tonnes before midnight and 5 tonnes after midnight. The caller supplies one use of 3 tonnes against the first day's quota and a separate use of 5 tonnes against the next day's quota, each with its own evidence ID and period. The validator checks each day's total independently and preserves both lineages. It does not infer the 3/5 split from a shipment duration or route estimate; those amounts must come from an explicit, reviewed demand schedule.