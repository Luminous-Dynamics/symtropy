# Settlement Timeline Semantics v0.1

**Status:** Stacked implementation prototype for [Symtropy settlement core PR #1730](https://github.com/Luminous-Dynamics/symtropy/pull/1730), tracked by [SETTLEMENT-TIME-001 #1739](https://github.com/Luminous-Dynamics/symtropy/issues/1739). This is synthetic discrete-event logic, not physical infrastructure dynamics, urban recovery evidence or habitat qualification.

## Why a temporal extension is necessary

A snapshot of dependency state cannot answer when a service fails, how long backup lasts, whether resources deplete, when repairs finish, or whether recovery persists. This extension advances model time in deterministic integer ticks and accounts for resource stock at each tick.

The shared physical model boundary does not change: component events and resource inflows are declared synthetic scenario facts. No electrical, hydraulic, thermal, atmospheric or transport equation is implied.

## Input contract

`SettlementTimelineInput` composes the existing v1 `ScenarioInput` with:
- an inclusive finite horizon: ticks 0 through `horizon_ticks`;
- a stability window in consecutive ticks;
- one maximum stock declaration for every resource supply;
- timestamped events ordered by unique `(tick, sequence)`, with a stable event ID.

Supported event types in this tranche:
- `ComponentStateChanged`: changes the declared observed state of one exact component generation.
- `ResourceInflow`: adds a positive number of synthetic units to one exact resource supply, within its maximum stock bound.

Unknown fields, duplicate event IDs/sequences, stale component generations, unknown supplies, out-of-horizon events and over-capacity inflows fail closed.

## Tick semantics

For each tick:
1. Capture opening stock from the previous tick's closing ledger.
2. Apply same-tick events in explicit sequence order.
3. Evaluate the existing static settlement contract with the updated component states and current stock.
4. Deduct only reported, named known-resource allocations; unknown potential is never treated as allocated stock.
5. Emit a per-supply transition and check exact integer conservation.
6. Preserve closing stock for the next tick.

For every supply:

`opening + inflow - allocated_consumption = closing`

The protected reserve is included in total stock and cannot be allocated to routine demands. Stock inflows and component-state events are different facts: an inflow does not declare a failed source operational. No unmodeled production, natural loss or unit conversion occurs in this version.

The existing supply-selection priority, demand priority, allocation-pool, unknown-state and service-floor semantics remain in force. Supply source selection uses explicit numeric priority, with stable supply ID as a tie-break—not alphabetic IDs as hidden policy.

## Recovery semantics

Each service floor gets its own summary:
- `first_non_met_tick`: first tick when the floor is not Met, including Blocked.
- `first_restoration_tick`: first later tick when it is Met again.
- `stable_recovery_tick`: earliest later tick beginning a full, predeclared window where that floor remains Met.
- `non_met_tick_count`: all non-Met ticks.
- `later_non_met_ticks`: any subsequent loss after the first completed stable window.

First restoration and stable recovery are separate. A later recurrence does not erase the earlier window; it is explicitly listed as a later non-Met tick. All times are synthetic ticks, not real-world minutes or hours.

## Current fixture

The authored deterministic fixture models a water pump dependent on grid or backup supply:
- normal grid use consumes grid stock first due to explicit source priority;
- a grid outage leaves only the bounded backup stock available;
- a delayed grid restoration and explicit stock inflow permit service to recover;
- a two-tick stability window distinguishes restoration from stable recovery;
- every per-supply stock transition must reconcile exactly.

Current authored temporal tests cover stable recovery after an outage, stock conservation across inflow and demand, input-order independence, duplicate event-sequence rejection, over-capacity inflow rejection, stale generation rejection and unknown-field rejection. Tests are authored but have not been executed locally because the current authoring environment does not expose Cargo/rustc; exact-head CI remains the required verification source.

## Limits / next steps

This tranche does not model:
- repair crews, dispatch or repair duration as a native state machine;
- stochastic event probability or distributional forecasts;
- multi-step physical production/conversion of resources;
- stock losses/leaks unless represented by an explicit future event type;
- calibrated grid, water, thermal, transport, ECLSS, radiation or planetary physics;
- real service floors authorized by an actual community or safety authority.

The event model is deterministic; it does not imply that real systems are deterministic. A passing test establishes only replay and accounting semantics for the declared synthetic corpus.

## Research references

- NIST, Community Resilience Planning Guide: systems dependencies, cascading consequences, community-defined performance goals and recovery planning. https://www.nist.gov/community-resilience/planning-guide
- NIST, Community Resilience Planning Guide Volume II: dependencies and cascading effects. https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.1190v2.pdf
- NASA, Environmental Control and Life Support Systems: multiple coupled life-support functions and explicit resource recovery boundaries. https://www.nasa.gov/reference/environmental-control-and-life-support-systems-eclss/
- Azadi et al. (2025), urban digital twins and socio-technical integration. https://doi.org/10.3390/smartcities8010032


### Same-tick event boundary

Events at one tick are applied in their unique sequence order and the settlement is evaluated once after the tick's entire event batch. If a component is marked Failed and then Available again at the same tick, the snapshot sees the final state only; the transient failure duration is not represented by this version. A scenario that needs the failure to affect service must place it on a separate tick or wait for a finer-grained event clock. Resource inflows within the same tick are accumulated and capacity-checked in sequence before demand allocation.


## Independent report verification

The branch now includes `src/verifier.rs`, an independent checker for the emitted timeline receipt. It verifies:
- report and per-tick snapshot digest consistency and claim ceilings;
- contiguous tick identities, stable scenario/component/demand/service-floor identities and unique applied events;
- exact source-allocation totals and consistency between per-demand source entries and per-supply consumed totals;
- synthetic resource-kind/unit agreement, stock conservation, reserve limits and cross-tick stock continuity;
- component state/output consistency and service-floor dispositions against underlying component/output/demand evidence;
- independently re-derived first restoration, stable-recovery window and later recurrence summaries.

Regression tests include an unchanged report, mutated stock with the outer digest recomputed, and a modified recovery claim with the digest recomputed. The point is to prove that a valid hash alone is insufficient: a self-consistent integrity hash can still describe a semantically false report, so independent semantic verification is needed.

The checker verifies report structure and internal arithmetic, not source truth, model calibration, event authenticity, legal authority or physical validity. The temporal PR currently has 10 authored unit tests; exact-head CI has not established a pass.


### Exact event receipts and inflow reconciliation

Each tick receipt now carries the complete applied event record: event ID, tick, sequence, typed event payload, and a SHA-256 digest of that payload. The independent verifier checks that event IDs are globally unique, receipt ticks match, event sequences are strictly increasing within a tick, and payload digests match the serialized event content.

Resource-inflow event units are independently summed by supply and compared against the ledger's per-tick inflow units. This check remains meaningful even if someone edits an event payload and recomputes the event and outer report digests: the ledger must still reconcile to the recorded event stream. Event digests are integrity links, not digital signatures or proof of source truth.
