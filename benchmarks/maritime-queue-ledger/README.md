# Maritime queue and cargo ledger v0

Dependency-free Rust accounting contract for deterministic, evidence-bounded snapshots of a port's vessel-call queue and cargo inventory.

This is a **read-only stock/flow auditor**, not an AIS connector, port digital twin, queue simulator, berth optimizer, vessel scheduler, source authenticator or operational authority.

## Paradigm shift: measure the stock, not just the flow

Weekly TEU throughput is a flow. Vessel queue size, yard inventory and cargo dwell are stocks. A period can report higher throughput while a vessel backlog remains large or grows. This crate keeps the stocks separate and calculates their area-under-curve:
- vessel queue area in vessel-ticks;
- cargo inventory area in explicitly declared unit-ticks (physical containers, TEU or tonnes).

It never converts physical boxes to TEUs, rehandles to new cargo, tonnes to containers, or train paths to TEUs. Different units require different ledgers and an explicit, separately reviewed conversion if one is ever needed.

## Event and observation semantics

Each observation preserves:
- event ID and stable entity ID (vessel-call ID or cargo-movement record ID);
- event class: planned, requested, estimated or actual;
- event time and observation/ingestion time as separate integer ticks;
- explicit correction target (supersedes_event_id);
- source ID/revision, evidence ID, raw artifact digest, rights reference/status and claim ceiling.

Only actual observations change the stock. Planned/requested/estimated events are counted as excluded observations, not silently promoted to actual arrivals or cargo movements.

An actual event must not be dated after it was observed. A late event (event_tick < observed_tick) is admitted only to snapshots whose information cut-off includes the observation. A correction changes a later snapshot only after the correction itself was observed. Future corrections do not rewrite historical decision snapshots.

Conflicting active observations of the same entity, event meaning and event class reject. The resolver never chooses whichever source arrived last. A correction must explicitly supersede an earlier event for the same entity, same event meaning and same classifier. Multiple visible corrections forked from one prior event are rejected.

## Vessel queue

The reporting interval is half-open [start_tick, end_tick). A call's eligible arrival adds one to the queue. Berth entry, diversion, cancellation and another declared terminal exit subtract one. The input carries an opening queue stock at the start of the window.

The audit checks:
- terminal and time-basis identity;
- event/observation time order and source evidence rights status;
- unique event IDs and explicit correction chains;
- no simultaneous unresolved duplicate event meanings;
- at most one terminal exit per vessel-call ID;
- a call's in-window exit cannot precede its in-window eligible arrival;
- queue stock never becomes negative and arithmetic does not overflow.

It reports opening/closing stock, arrivals, exits, peak queue, queue area and actual/non-actual observation counts. An exit without a corresponding in-window arrival can be valid when the opening queue includes that call.

## Cargo inventory

A cargo ledger fixes one terminal, inventory scope, reporting window and unit. Its event types distinguish vessel discharge/load, gate-in/out, rail-in/out and other declared movements. Inflows and outflows change the stock; actual events only.

The audit reports opening/closing inventory, inbound/outbound quantity, peak inventory and inventory area. It does not equate throughput with completed end-to-end delivery. Rehandles, transhipment, loaded/empty, import/export and reefer/OOG subdivisions need their own source-supported scopes and definitions before being composed.

## Rights and cryptographic boundary

Every admitted source record requires a reviewed-allowed rights state, a rights-reference, and a syntactically valid SHA-256 digest. The digest is not recomputed here: the crate does not receive or hash the raw source bytes. A digest proves no publisher identity, authenticity, lawful reuse or measurement truth by itself.

Unknown or restricted rights block the snapshot. Source rights and authenticity require external review.

## Synthetic tests and independent reference

The unit test corpus contains **13 adversarial/known-answer controls**, including:
- queue and cargo stock conservation;
- peak stock and area-under-curve arithmetic;
- plan/request/estimate exclusion;
- late observations and correction cut-offs;
- conflicting duplicate event semantics;
- negative queue/inventory and duplicate exits;
- call chronology, half-open window boundary and correction-fork behavior;
- unit mismatch and rights blocking.

A separate test-only oracle recomputes queue stock one integer tick at a time over **768 frozen scenarios**. It does not call the production resolver, event bucketing or area functions.

These are synthetic accounting checks only. They do not validate real vessel traffic, terminal throughput, physical capacity, source authenticity or queue forecasts.

## Run

~~~sh
cargo fmt --manifest-path benchmarks/maritime-queue-ledger/Cargo.toml -- --check
cargo test --locked --manifest-path benchmarks/maritime-queue-ledger/Cargo.toml
cargo clippy --locked --manifest-path benchmarks/maritime-queue-ledger/Cargo.toml --all-targets -- -D warnings
~~~

## Intended composition

The ledger output is a read-only evidence-qualified snapshot for [MARITIME-QUEUE-RECOVERY-001 #4007](https://github.com/Luminous-Dynamics/symtropy/issues/4007). Only after the event/stock accounting is qualified should a queue-recovery experiment compose it with:
- Symtropy's existing fleet/resource oracle;
- the named tug/pilot feasibility verifier;
- port-call and PCO/DCSA timestamp adapters;
- Symthaea vessel and engineering model applicability receipts;
- Mycelix participant/operator commitments and authority;
- Sol-Atlas route, terminal and source-coverage geography.

A ledger PASS is not a queue stability, delivery reliability, causal impact, physical model or operational-authority claim.
