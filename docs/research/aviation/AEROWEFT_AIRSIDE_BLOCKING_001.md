# AeroWeft — Finite-Buffer Vertiport Blocking and Queue Spillback

**Tracking issue:** [AEROWEFT-AIRSIDE-BLOCKING-001 #5261](https://github.com/Luminous-Dynamics/symtropy/issues/5261)  
**Consumes:** Symtropy #5186/#5248/#5207/#5224/#5163 and Symthaea #6249  
**Status:** synthetic-first research specification  
**Claim ceiling:** finite discrete event/resource semantics only; no real vertiport capacity, aircraft energy model, operational flight procedure or safety claim.

## 1. Research thesis

A vertiport's nominal number of landing/take-off pads does not determine its usable capacity when aircraft must move through a sequence of finite downstream resources.

An aircraft that completes landing service but cannot transfer to a stand may continue occupying the TLOF. That blocks the next arrival. The next arrival may occupy a finite terminal holding slot, accrue additional energy consumption, and then require a longer recharge/turnaround. The resulting delay propagates back into later landing, departure and airspace reservations.

The missing measurement is not another independent queue. It is **finite-buffer blocking across the complete aircraft resource sequence**.

The core invariant is:

> An aircraft that cannot acquire its next required finite-capacity resource continues to occupy its current resource until a causally valid release or an explicit alternate disposition occurs.

A queue model that silently releases the TLOF when landing service duration ends, despite no available downstream transfer, is optimistic.

## 2. 2026 literature frontier

The comparison must acknowledge current work:

- **Wille, Reichmuth & Borille (June 2026), "Determining vertiport airside capacity using discrete event simulation", Journal of the Air Transport Research Society 6, 100109, DOI 10.1016/j.jatrs.2026.100109.** Their discrete-event study examines throughput and practical capacity, vertiport topology, stand/TLOF ratios, taxi processes and queueing policies other than FIFO. It explicitly argues that capacity assessment needs practical delay and taxi effects, not only movement counts. This is the primary airside DES baseline class.
- **Nagrare & Lieb (22 January 2026), "Throughput and Capacity Analysis of a Vertiport with Taxing and Parking Levels", Aerospace 13(1), 109, DOI 10.3390/aerospace13010109.** It studies separate taxi/parking levels, resource allocation, hourly demand, charging duration and parking capacity. Charging changes total operational time, while parking positions increase capacity and schedule flexibility.
- **Sengupta et al. (2025), "Urban Air Mobility Research Challenges and Opportunities", Annual Review of Control, Robotics, and Autonomous Systems 8:407–431, DOI 10.1146/annurev-control-022823-031353.** The review identifies limited parking as a blocking mechanism not represented by ordinary unlimited-buffer queue models. It also discusses a cross-layer cascade: airborne holding absorbs delays but reduces battery state, potentially lengthening charge time and consuming parking capacity, which then feeds back into subsequent traffic schedules.
- **Lyu & Feng (July 2026), DOI 10.1016/j.tra.2026.105027.** Their MNL/multi-server queue model is an important passenger-service baseline, but the present track focuses on aircraft-side blocking between TLOF, taxi, stands and chargers.
- **Qin et al. (November 2026), DOI 10.1016/j.tre.2026.105103.** Their buffered vertiport queue and commuter departure-time model are broader demand/operation baselines. This task should export airside blocking outcomes to that service-quality layer, not recreate the full demand model.

The contribution to test is an independently replayable composition of:
- exact time-indexed resource occupancy;
- blocking-after-service and finite upstream buffers;
- terminal holding and declared energy deltas;
- turnaround/charging and follow-on schedule shifts;
- practical capacity and per-aircraft recovery consequences.

It does not claim to invent discrete-event simulation, finite-buffer queues, blocking-after-service, vertiport capacity analysis or priority sequencing.

## 3. Ownership and non-duplication

| Responsibility | Existing owner | This benchmark consumes |
|---|---|---|
| Candidate aircraft schedules and airspace/vertiport reservations | Symtropy #5186 | Exact schedule proposals and resource-event receipts |
| Passenger mode choice, service quality and demand feedback | Symtropy #5248 | Airside delay/capacity outcomes; passenger choice remains separate |
| Reciprocal aircraft future-option measurement | Symtropy #5207 | Blocking/holding/recovery outcomes where option sets change |
| Strategic recovery-site and energy/access portfolio | Symtropy #5224 | Admitted resource/site state and outage cases |
| Aircraft reachable occupancy and conflict references | Symtropy #5163/#2279/#2297 | Declared occupancy constraints and verifier receipts |
| Aircraft dynamics, battery/performance applicability | Symthaea #6249 | Profile-qualified energy/reserve traces when available |
| Electrical charging/power system | Symtropy #3418 | Qualified charging service modes; no electrical solver duplication |
| Spatial source context | Sol-Atlas #189/#825 | Read-only admitted site and datum/status evidence |
| Provenance and reservations | Mycelix #5023 | Event/identity receipts, not physical truth |

This work owns the aviation-specific finite-buffer event contract, tiny blocking/cascade fixture, practical-capacity metrics and independent small reference oracle. It does not implement a new flight scheduler, passenger queue, aircraft model, charger physics or site-selection optimizer.

## 4. Semantic contract

### 4.1 Resource sequence

Model the following as distinct, finite, time-indexed resources:
- TLOF/FATO or declared landing/take-off service area;
- taxiway/taxi link / transfer mechanism;
- stand/parking bay;
- charger/energy service bay;
- terminal-area holding slot or other declared pre-landing buffer;
- upstream arrival/departure/airspace time-slot resource from #5186.

The physical aircraft is a resource-consuming token. It occupies a resource from acquisition until an explicit release event. A nominal service-duration expiry is not itself a release if downstream capacity is unavailable.

### 4.2 Blocking-after-service

Represent a movement as:

`acquire(resource A) -> perform service -> acquire compatible resource B -> release(resource A)`

If resource B is not available after service completes, resource A remains occupied. Record:
- service completion time;
- requested downstream resource;
- actual downstream acquisition time;
- blocking interval;
- exact resource/aircraft witness.

Do not allow the simulation to "teleport" an aircraft from the TLOF into an unlimited stand queue unless the scenario explicitly declares and qualifies that buffer.

### 4.3 Finite upstream holding

When a TLOF is unavailable, an inbound aircraft may occupy only a declared holding slot/resource. If holding capacity is exhausted, the scenario must record an explicit defer, alternate, divert, cancel, or unsupported disposition. Never create invisible unlimited terminal airspace.

Any energy consumed during holding must be supplied by a synthetic fixture or qualified aircraft model. No universal energy-per-minute constant is permitted. If the supplied energy/reserve boundary is crossed, continue only if the scenario includes a separately admitted recovery alternative; otherwise mark not-assessable/unsupported.

### 4.4 Stand and charging coupling

A stand may remain occupied throughout charging and turnaround. If the same physical bay is required during charge, charging is not an independent parallel server that can be used without stand occupancy.

Charger occupancy and stand occupancy are separate resource claims with their own capacity and intervals. The scenario must declare if charging can occur off-stand, whether it blocks a bay, and whether an aircraft can leave the stand before charging is complete.

### 4.5 Event identity and determinism

Each event binds:
- aircraft ID and profile revision;
- event ID and predecessor/causal parent;
- resource ID and half-open occupancy interval;
- source schedule and reservation IDs;
- service/queue policy revision;
- time basis and event ordering;
- exact energy/reserve delta source;
- outcome disposition and explicit cause.

Simultaneous event ordering is fixed and versioned. Unknown resource capacity, missing downstream compatibility, missing events or incomplete manifests return incomplete/not-assessable, never unlimited capacity.

## 5. Golden fixture: three-aircraft blocking cascade

Machine-readable fixture: `fixtures/aeroweift-airside-blocking/tiny-blocking-cascade-v0.json`.

All values are fictional integer ticks and synthetic energy units.

### Declared resources

- one TLOF;
- one stand;
- one taxi link;
- one charger;
- one terminal holding slot.

### Causal timeline

1. Aircraft B holds the only stand over ticks [0,10) and the only charger over [0,8).
2. Aircraft A acquires the TLOF at tick 2; its nominal landing service completes at tick 5.
3. Because B still occupies the stand until tick 10, A cannot transfer. A therefore remains on the TLOF over [5,10), creating five ticks of blocking-after-service. Its TLOF occupancy is [2,10).
4. Aircraft C's scheduled arrival is tick 6. The TLOF is blocked, so C uses the only terminal holding slot over [6,10), creating four ticks of hold. The fixture declares a synthetic energy decrement of one unit per tick: C's energy falls from 40 to 36.
5. B releases the stand at tick 10. A uses the taxi link over [10,11), then occupies the stand over [11,19).
6. C acquires the TLOF at tick 10, with landing service completing at tick 13. A still occupies the only stand until tick 19, so C remains on the TLOF over [13,19), creating six more ticks of blocking-after-service. C's TLOF occupancy is [10,19).
7. C uses the taxi link over [19,20), then occupies the stand and charger over [20,28). The extra holding energy deficit requires the declared eight-tick top-up before the next sortie. The next-mission energy requirement is 38 units; C has 36 after holding, so it cannot use the next mission slot without the declared charge.

Expected fixture outcomes:
- A TLOF blocking: 5 ticks.
- C terminal holding: 4 ticks.
- C energy delta: -4 synthetic units.
- C second TLOF blocking: 6 ticks.
- C actual landing tick: 10 versus scheduled tick 6.
- C next-mission release: tick 28 under the fixture's charge rule.
- TLOF-only nominal slot arithmetic would not expose the complete cascade.

The matched decoupled/infinite-buffer comparison is explicitly optimistic and not a physically admissible policy. It may be used only as a negative control to show how ignoring finite downstream capacity understates delay.

No part of the fixture represents a real vertiport, real aircraft, real battery or operational procedure.

## 6. Required outputs

**Per aircraft**
- every resource acquisition/release interval and identity;
- nominal versus actual arrival/departure/turnaround times;
- blocking-after-service duration and blocking cause;
- terminal holding interval and occupancy;
- synthetic/profile-supplied energy and reserve delta;
- next-mission eligibility and any top-up/charge delay;
- delayed/deferred/alternate/diverted/cancelled/incomplete/not-assessable disposition;
- future resource windows and recovery options retained/lost.

**Per vertiport**
- nominal throughput versus practical capacity under the declared delay envelope;
- TLOF service occupancy versus blocking occupancy;
- stand/parking/charger/taxi/holding occupancy and queue lengths;
- resource-by-resource spillback and cascade delay attribution;
- p50/p95/p99 and maximum delay when an ensemble exists;
- movements displaced, deferred or unsupported;
- exact event trace and resource-contention witnesses.

A single throughput or mean-delay scalar may not conceal an aircraft that loses every eligible continuation or a holding energy boundary that is crossed.

## 7. Matched baselines

Use identical arrivals, aircraft profiles, service times, resource capacities, schedule, energy traces, demand and compute budget:

1. TLOF-only or pad/stand ratio capacity calculation.
2. Independent infinite-buffer M/M/c-style approximation, labelled idealized and non-equivalent.
3. FIFO discrete-event queue without blocking-after-service.
4. Priority/non-FIFO event queue of the algorithm class evaluated by Wille et al. (2026).
5. Coupled finite-buffer model with blocking-after-service and finite terminal holding.
6. Candidate schedules from #5186 evaluated by the independent finite-buffer oracle.
7. Recovery-aware candidate schedules from existing owners, independently scored by the same event oracle.

A transparent local implementation is a model-class baseline, not a faithful reproduction of Wille et al. unless exact source code, topology, service distributions, calibration and compute settings are pinned.

## 8. Falsification and metamorphic controls

- A free compatible stand prevents the A blocking interval in the corresponding fixture variant.
- Occupying the only stand produces the expected TLOF blocking-after-service interval.
- Releasing B's stand earlier cannot increase A's blocking duration under the same event trace.
- Adding one genuinely compatible independent stand cannot shrink the finite-buffer feasible event set.
- Extending charging duration cannot shorten stand occupancy when the stand is held through charging.
- Increasing C's holding duration cannot increase its supplied energy reserve when the energy decrement is non-negative.
- Zero holding capacity cannot silently become unlimited terminal holding.
- Unknown downstream capacity cannot be assumed available.
- Resource release without prior acquisition and matching aircraft/resource identity is rejected.
- Two aircraft may not overlap on a capacity-one resource.
- Event input-order permutation preserving causal predecessors yields the same semantic timeline.
- The decoupled/infinite-buffer negative control may be optimistic, but its results cannot be admitted as finite-buffer feasibility.
- Changed resource/event/model digests invalidate the prior trace receipt.

## 9. Acceptance targets

- 100% exact agreement with an independent hand-enumerated oracle on the tiny corpus.
- Zero overlapping capacity-one claims accepted.
- Zero resource releases without a matching acquisition.
- Zero implicit infinite buffers or missing downstream resources promoted to available.
- 100% explicit blocking/cause/holding/delay accounting for declared events.
- Exact deterministic replay and input-order/ID relabelling invariance.
- Every finite-buffer failure contains the blocked aircraft, resource, interval, downstream dependency and successor witness.
- Practical-capacity reports include a preregistered delay/service envelope and do not equate nominal TLOF throughput with complete network capacity.

These are synthetic event-semantics targets, not real vertiport capacity or aircraft safety claims.

## 10. Execution order

1. Freeze the three-aircraft hand-solvable cascade and the decoupled negative control.
2. Implement an independent small finite-resource/event oracle.
3. Define adapters to #5186 schedule/resource receipts and #5248 service-delay outputs.
4. Consume aircraft energy/reserve inputs from Symthaea #6249 only when profile applicability and source evidence exist.
5. Feed practical-capacity, delay and blocking outcomes back to #5186/#5248; preserve their scheduler and choice ownership.
6. Expand to frozen randomized arrival/service traces only after the tiny oracle and metamorphic controls pass.

**Core invariant:** an aircraft that cannot move into the next required finite-capacity resource continues to occupy its current resource. A model that drops that occupancy is optimistic, not a harmless simplification.
